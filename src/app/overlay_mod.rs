//! The modular overlay: the same look as the original, built from sections
//! that can each be turned on and off (Settings > Overlay), in three sizes
//! (Full, Compact, Bar). The Legacy overlay (`full_ui`) is left exactly as
//! it was and is drawn instead when Legacy is picked.

use super::{fmt_one_in, App, FULL_SIZE};
use crate::config::{now_unix, OverlayModules, OverlaySize};
use crate::hotkeys::HotAction;
use crate::stats::local_day;
use crate::theme::{self, outlined_text, GOLD, MUTED, OUTLINE, RED};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub(super) const COMPACT_W: f32 = 236.0;
pub(super) const BAR_H: f32 = 50.0;
/// Narrowest and widest the liquid layout goes.
pub(super) const LIQUID_MIN: f32 = 236.0;
pub(super) const LIQUID_MAX: f32 = 640.0;
/// Room under the overlay for a notice or pop-up card.
pub(super) const BELOW_H: f32 = 96.0;
/// Width of a pop-up under the bar layout.
pub(super) const BAR_POPUP_W: f32 = 250.0;
const GAP: f32 = 8.0;
pub(super) const XP_BLUE: Color32 = Color32::from_rgb(90, 170, 255);
pub(super) const FLAME: Color32 = Color32::from_rgb(255, 140, 40);

/// Slow-changing numbers for the overlay, worked out about once a second.
#[derive(Clone, Default)]
pub(super) struct OvStats {
    pub level: u32,
    pub xp_into: u64,
    pub xp_need: u64,
    pub streak: u32,
    pub today: u64,
    /// Best single day before today (all runs).
    pub best_day: u64,
    pub phases: usize,
}

/// Odds bar contents (shared by every size).
pub(super) struct Odds {
    pub left: String,
    pub color: Color32,
    pub frac: f32,
    pub right: String,
    pub tip: String,
    pub mult: Option<f64>,
}

fn tiny(p: &egui::Painter, pos: Pos2, anchor: Align2, text: &str, color: Color32) -> Rect {
    outlined_text(p, pos, anchor, text, 6.5, color)
}

fn body_text(p: &egui::Painter, pos: Pos2, anchor: Align2, text: impl ToString, size: f32, color: Color32) -> Rect {
    p.text(pos, anchor, text.to_string(), egui::FontId::proportional(size), color)
}

/// Small panel behind a stat (same look as the sprite boxes).
fn cell(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 6.0, theme::PANEL().gamma_multiply(0.9));
    p.rect_stroke(r, 6.0, Stroke::new(1.0f32, theme::BORDER()));
}

/// Little flame for the streak chip.
fn flame(p: &egui::Painter, c: Pos2, s: f32, color: Color32) {
    p.circle_filled(c + egui::vec2(0.0, s * 0.25), s * 0.45, color);
    p.add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(-s * 0.42, s * 0.12),
            c + egui::vec2(0.0, -s * 0.7),
            c + egui::vec2(s * 0.42, s * 0.12),
        ],
        color,
        Stroke::NONE,
    ));
    p.circle_filled(c + egui::vec2(0.0, s * 0.35), s * 0.2, Color32::from_rgb(255, 225, 120));
}

impl App {
    /// Click-through is only for the modular overlay, and never while
    /// something on it needs a click (a shiny, the Caught! question, Undo).
    pub(super) fn click_through_active(&self) -> bool {
        let o = &self.cfg.overlay;
        o.click_through
            && !o.legacy()
            && self.shiny_alert.is_none()
            && !self.confirm_caught
            && self.undo_caught_left().is_none()
    }

    pub(super) fn ov_stats(&mut self) -> OvStats {
        if let Some((at, s)) = &self.ov_cache {
            if at.elapsed() < Duration::from_secs(1) {
                return s.clone();
            }
        }
        let m = self.cfg.overlay.clone();
        let mut s = OvStats::default();
        if m.xp {
            let l = self.lifetime();
            let xp = self.total_xp(&l).total();
            s.level = crate::progress::level_for(xp);
            let (a, b) = crate::progress::level_progress(xp);
            s.xp_into = a;
            s.xp_need = b;
        }
        if m.streak {
            s.streak = self.daily_streak();
        }
        if m.phases {
            s.phases = self.phases_of(self.cfg.run_id).len();
        }
        if m.daily_goal || m.best {
            let (today, best) = self.day_totals();
            s.today = today;
            s.best_day = best;
        }
        self.ov_cache = Some((Instant::now(), s.clone()));
        s
    }

    /// Encounters today across every run, and the best single day before today.
    pub(super) fn day_totals(&self) -> (u64, u64) {
        let today = local_day(now_unix());
        let mut days: BTreeMap<i64, u64> = BTreeMap::new();
        let mut add = |st: &crate::stats::RunStats| {
            for d in &st.days {
                *days.entry(d.day).or_default() += d.encounters;
            }
        };
        add(&self.cfg.run_stats);
        for r in &self.cfg.saved_runs {
            if r.id != self.cfg.run_id {
                add(&r.stats);
            }
        }
        for h in &self.cfg.history {
            if let Some(st) = &h.stats {
                add(st);
            }
        }
        let t = days.get(&today).copied().unwrap_or(0);
        let best = days.iter().filter(|(d, _)| **d != today).map(|(_, n)| *n).max().unwrap_or(0);
        (t, best)
    }

    pub(super) fn ov_odds(&self) -> Odds {
        match self.cfg.odds_one_in() {
            None => Odds {
                left: "ODDS: AFTER 1ST BATTLE".to_string(),
                color: MUTED(),
                frac: 0.0,
                right: String::new(),
                tip: "Waiting for the game log to report the odds (start a battle).".to_string(),
                mult: None,
            },
            Some(one_in) => {
                let (odds_n, odds_what) = self.odds_count();
                let mult = odds_n as f64 / one_in;
                let m = if mult < 0.1 { format!("{mult:.2}×") } else { format!("{mult:.1}×") };
                let (left, color) = if mult < 0.9 {
                    (format!("UNDER ODDS · {m}"), theme::GREEN)
                } else if mult <= 1.1 {
                    (format!("AT ODDS · {m}"), theme::HP_YELLOW)
                } else if mult < 2.0 {
                    (format!("OVER ODDS · {m}"), theme::HP_RED)
                } else {
                    (format!("{m} OVER ODDS"), theme::HP_RED)
                };
                let p = 1.0 / one_in;
                let per_battle = self.cfg.per_battle_one_in();
                let wild = self.cfg.hunt_method.is_wild_battles();
                let right = match per_battle {
                    Some(b) if wild => format!("1/{} a battle", fmt_one_in(b)),
                    _ if wild => format!("1/{} each", fmt_one_in(one_in)),
                    _ => format!("1/{}", fmt_one_in(one_in)),
                };
                let found = 1.0 - (1.0 - p).powf(odds_n as f64);
                let mut tip = format!(
                    "{}\n{} of {} {odds_what} ({m}).\n{:.0}% of hunters would have found one by now.",
                    self.cfg.odds_caption(),
                    theme::thousands(odds_n),
                    fmt_one_in(one_in),
                    found * 100.0,
                );
                tip.push_str(&format!("\nEach Pokémon: 1 in {}.", fmt_one_in(one_in)));
                if wild {
                    if let (Some(b), Some(n)) = (per_battle, self.cfg.pokemon_per_battle()) {
                        if n > 1.0 {
                            tip.push_str(&format!(
                                "\nEach battle ({} Pokémon): 1 in {}.",
                                if n == 2.0 { "2".to_string() } else { format!("~{n:.1}") },
                                fmt_one_in(b)
                            ));
                        }
                    }
                }
                tip.push_str("\nThe chance never builds up between encounters.");
                Odds { left, color, frac: mult.min(1.0) as f32, right, tip, mult: Some(mult) }
            }
        }
    }

    /// (share of hunters who'd have found one by now, colour, tooltip).
    pub(super) fn ov_luck(&self) -> Option<(f64, Color32, String)> {
        let one_in = self.cfg.odds_one_in()?;
        let (n, what) = self.odds_count();
        let found = 1.0 - (1.0 - 1.0 / one_in).powf(n as f64);
        let pct = found * 100.0;
        let color = if pct < 50.0 {
            theme::GREEN
        } else if pct < 80.0 {
            theme::HP_YELLOW
        } else {
            theme::HP_RED
        };
        let tip = format!(
            "After {} {what}, {pct:.0}% of hunters would have found their shiny.\nUnder 50% you're doing fine; over 80% you've been unlucky. Every encounter is still a fresh 1 in {}.",
            theme::thousands(n),
            fmt_one_in(one_in)
        );
        Some((pct, color, tip))
    }

    /// Next milestone: (headline, "in N", tooltip). The nearer of the next
    /// thousand and the next whole multiple of the odds.
    pub(super) fn ov_next_milestone(&self) -> (String, String, String) {
        let count = self.cfg.count;
        let next_k = (count / 1000 + 1) * 1000;
        let mut best = (
            theme::thousands(next_k),
            next_k - count,
            format!("{} encounters is the next thousand.", theme::thousands(next_k)),
        );
        if let Some(one_in) = self.cfg.odds_one_in() {
            let (n, what) = self.odds_count();
            let k = (n as f64 / one_in).floor() as u64 + 1;
            let at = (k as f64 * one_in).ceil() as u64;
            let left = at.saturating_sub(n);
            if left < best.1 {
                best = (format!("{k}× odds"), left, format!("{} {what} reaches {k}× the odds.", theme::thousands(at)));
            }
        }
        (best.0, format!("in {}", theme::thousands(best.1)), best.2)
    }

    /// Encounters per minute for the last `mins` minutes (oldest first).
    pub(super) fn ov_pace(&self, mins: usize) -> Vec<u64> {
        let now = now_unix();
        let mut b = vec![0u64; mins];
        for &(t, n) in &self.recent_enc {
            let age = (now.saturating_sub(t) / 60) as usize;
            if age < mins {
                b[mins - 1 - age] += n;
            }
        }
        b
    }

    pub(super) fn ov_pace_per_hour(&self) -> u64 {
        let now = now_unix();
        let last10: u64 = self.recent_enc.iter().filter(|(t, _)| now.saturating_sub(*t) < 600).map(|(_, n)| n).sum();
        last10 * 6
    }

    fn sparkline(&self, p: &egui::Painter, r: Rect, bars: &[u64]) {
        let max = bars.iter().copied().max().unwrap_or(0).max(1) as f32;
        let n = bars.len().max(1) as f32;
        let w = r.width() / n;
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0f32, theme::BORDER()));
        for (i, &v) in bars.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let h = (v as f32 / max * r.height()).max(2.0);
            let x = r.min.x + i as f32 * w;
            let col = if i + 1 == bars.len() { GOLD } else { theme::BUTTON().gamma_multiply(0.85) };
            p.rect_filled(
                Rect::from_min_max(egui::pos2(x + 1.0, r.max.y - h), egui::pos2(x + w - 1.0, r.max.y)),
                1.5,
                col,
            );
        }
    }

    /// The pin / settings / hide / close cluster, the status light and the
    /// title. Same as the original header.
    fn ov_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: &egui::Painter, header: Rect, m: &OverlayModules) {
        theme::header_bar(p, header, 12.0);
        p.line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(2.0f32, theme::RED_DARK()));
        p.image(
            self.icon_tex.id(),
            Rect::from_center_size(header.left_center() + egui::vec2(15.0, 0.0), egui::vec2(18.0, 18.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let ghost = self.click_through_active();
        let title_r = outlined_text(
            p,
            header.left_center() + egui::vec2(28.0, 1.0),
            Align2::LEFT_CENTER,
            if ghost { "CLICK-THROUGH" } else { "SHINY HUNT" },
            8.5,
            if ghost { GOLD } else { Color32::WHITE },
        );
        if self.cfg.level_on_overlay && !ghost {
            let lvl = self.cfg.level_seen.max(1);
            theme::level_badge(p, egui::pos2(title_r.max.x + 11.0, header.center().y), 8.0, lvl);
        }

        let btn_w = 21.0;
        let drag_rect = Rect::from_min_max(header.min, egui::pos2(header.max.x - btn_w * 4.0 - 18.0, header.max.y));
        let hr = ui.interact(drag_rect, ui.id().with("drag"), Sense::click_and_drag());
        if hr.drag_started() && !self.cfg.pinned {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        hr.on_hover_text(if self.cfg.pinned { "Pinned: unpin to move" } else { "Drag to move" });

        if m.status_dot {
            let (dot, status_text) = self.status_line();
            let dot_pos = egui::pos2(header.max.x - btn_w * 4.0 - 11.0, header.center().y);
            p.circle_filled(dot_pos, 3.5, dot);
            p.circle_stroke(dot_pos, 3.5, Stroke::new(1.0f32, OUTLINE));
            ui.interact(Rect::from_center_size(dot_pos, egui::vec2(16.0, 16.0)), ui.id().with("status-dot"), Sense::hover())
                .on_hover_text(status_text);
        }

        let icon_btn = |ui: &mut egui::Ui, i: usize, icon: &str, on: bool, tip: &str| {
            let r = Rect::from_min_size(
                egui::pos2(header.max.x - 5.0 - btn_w * (4 - i) as f32, header.min.y + 5.0),
                egui::vec2(btn_w - 1.0, 22.0),
            );
            let color = if on { GOLD } else { Color32::WHITE };
            ui.put(r, egui::Button::new(RichText::new(icon).size(12.5).color(color)).frame(false))
                .on_hover_text(tip)
                .clicked()
        };
        if icon_btn(ui, 0, if self.cfg.pinned { "🔒" } else { "🔓" }, self.cfg.pinned, "Pin in place") {
            self.cfg.pinned = !self.cfg.pinned;
            self.dirty = true;
        }
        if icon_btn(ui, 1, "⚙", self.show_setup, "Setup: target, screen boxes, odds, overlay sections") {
            self.show_setup = !self.show_setup;
        }
        let hide_tip = format!("Hide to badge{}", self.cfg.keybinds.hint(HotAction::ToggleHide));
        if icon_btn(ui, 2, "—", false, &hide_tip) {
            self.cfg.mini = true;
            self.dirty = true;
        }
        let close_r = Rect::from_min_size(egui::pos2(header.max.x - 5.0 - btn_w, header.min.y + 5.0), egui::vec2(btn_w - 1.0, 22.0));
        self.close_button(ui, ctx, p, close_r);
    }

    pub(super) fn close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: &egui::Painter, close_r: Rect) {
        let close = ui.interact(close_r, ui.id().with("close-app"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if close.hovered() {
            p.rect_filled(close_r, 5.0, Color32::from_rgb(196, 43, 28));
        }
        let cc = close_r.center();
        let xs = Stroke::new(1.8f32, Color32::WHITE);
        p.line_segment([cc - egui::vec2(3.5, 3.5), cc + egui::vec2(3.5, 3.5)], xs);
        p.line_segment([cc + egui::vec2(-3.5, 3.5), cc + egui::vec2(3.5, -3.5)], xs);
        if close.on_hover_text("Close the overlay (the Companion keeps running)").clicked() {
            self.save_and_quit(ctx);
        }
    }

    /// Shiny alert banner over `area` (the sprite boxes).
    fn ov_shiny_banner(&mut self, ui: &mut egui::Ui, p: &egui::Painter, area: Rect, plate_idx: usize, blink: bool) {
        let banner = Rect::from_min_max(area.min + egui::vec2(0.0, 20.0), egui::pos2(area.max.x, area.max.y - 30.0));
        p.rect_filled(banner, 8.0, Color32::from_rgba_unmultiplied(20, 16, 4, 235));
        p.rect_stroke(banner, 8.0, Stroke::new(2.0f32, GOLD));
        let c = banner.center();
        if blink {
            let dx = (banner.width() / 2.0 - 30.0).min(86.0);
            theme::sparkle(p, c - egui::vec2(dx, 6.0), 9.0, GOLD);
            theme::sparkle(p, c + egui::vec2(dx, -6.0), 9.0, GOLD);
        }
        outlined_text(p, c - egui::vec2(0.0, 8.0), Align2::CENTER_CENTER, "SHINY!", 16.0, GOLD);
        body_text(p, c + egui::vec2(0.0, 14.0), Align2::CENTER_CENTER, super::shiny_alert_text(plate_idx), 11.0, Color32::WHITE);
        let dr = Rect::from_center_size(egui::pos2(c.x, banner.max.y + 14.0), egui::vec2(80.0, 20.0));
        if ui.put(dr, egui::Button::new("Dismiss").fill(theme::PLATE())).clicked() {
            self.shiny_alert = None;
            crate::sound::stop_song();
        }
    }

    /// Milestone confetti over `area` (about 5 seconds).
    fn ov_celebrate(&mut self, ctx: &egui::Context, p: &egui::Painter, area: Rect, small: bool) {
        let Some((at, text)) = self.celebrate.clone() else { return };
        let el = at.elapsed().as_secs_f32();
        if el > 5.0 || self.shiny_alert.is_some() {
            self.celebrate = None;
            return;
        }
        let fade = (el / 0.25).min(1.0) * ((5.0 - el) / 0.6).min(1.0);
        let cols = [GOLD, RED(), theme::GREEN, Color32::from_rgb(90, 170, 255), Color32::WHITE];
        for i in 0..36u32 {
            let seed = i.wrapping_mul(2654435761);
            let fx = (seed % 1000) as f32 / 1000.0;
            let speed = 40.0 + (seed / 1000 % 60) as f32;
            let fy = ((el * speed + (seed / 7 % 140) as f32) % (area.height() + 20.0)) - 10.0;
            let wob = (el * 3.0 + i as f32).sin() * 6.0;
            let pos = egui::pos2(area.min.x + fx * area.width() + wob, area.min.y + fy);
            let c = cols[(i % 5) as usize].gamma_multiply(fade);
            let sz = 2.0 + (i % 3) as f32;
            p.rect_filled(Rect::from_center_size(pos, egui::vec2(sz * 1.6, sz)), 1.0, c);
        }
        let h = if small { area.height() - 8.0 } else { 54.0 };
        let banner = Rect::from_center_size(area.center(), egui::vec2(area.width() - 8.0, h));
        let pulse = 1.0 + 0.04 * (el * 6.0).sin();
        let banner = Rect::from_center_size(banner.center(), banner.size() * pulse);
        p.rect_filled(banner, 10.0, Color32::from_rgba_unmultiplied(20, 16, 4, (230.0 * fade) as u8));
        p.rect_stroke(banner, 10.0, Stroke::new(2.0f32, GOLD.gamma_multiply(fade)));
        theme::sparkle(p, banner.left_center() + egui::vec2(14.0, if small { 0.0 } else { -12.0 }), 6.0 + 2.0 * (el * 5.0).sin().abs(), GOLD.gamma_multiply(fade));
        theme::sparkle(p, banner.right_center() + egui::vec2(-14.0, if small { 0.0 } else { 12.0 }), 6.0 + 2.0 * (el * 5.0).cos().abs(), GOLD.gamma_multiply(fade));
        let size = if text.chars().count() > 16 { 9.0 } else { 11.0 };
        if small {
            outlined_text(p, banner.center(), Align2::CENTER_CENTER, &text, size, GOLD.gamma_multiply(fade));
        } else {
            outlined_text(p, banner.center() - egui::vec2(0.0, 6.0), Align2::CENTER_CENTER, &text, size, GOLD.gamma_multiply(fade));
            body_text(p, banner.center() + egui::vec2(0.0, 13.0), Align2::CENTER_CENTER, "Milestone! Keep going", 11.0, Color32::WHITE.gamma_multiply(fade));
        }
        ctx.request_repaint();
    }

    /// −1 / +1 / pause / Caught!, or the Caught! question, or Undo.
    /// Same as the original overlay's bottom row.
    fn ov_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: &egui::Painter, row: Rect) {
        let undo_left = if self.confirm_caught { None } else { self.undo_caught_left() };
        let w = row.width();
        let at = |a: f32, b: f32| Rect::from_min_max(egui::pos2(row.min.x + w * a, row.min.y), egui::pos2(row.min.x + w * b, row.max.y));
        if let Some(left) = undo_left {
            body_text(p, egui::pos2(row.min.x, row.center().y), Align2::LEFT_CENTER, "✔ Logged as caught", 11.5, theme::pal().text);
            if ui
                .put(at(0.52, 0.84), egui::Button::new(RichText::new(format!("Undo ({left})")).strong().color(Color32::WHITE)).fill(theme::PLATE()))
                .on_hover_text("Pressed Caught! by mistake? Put everything back")
                .clicked()
            {
                self.undo_caught_now();
            }
            if ui.put(at(0.86, 1.0), egui::Button::new("OK").fill(theme::PLATE())).clicked() {
                self.undo_caught = None;
            }
            ctx.request_repaint_after(Duration::from_millis(250));
        } else if self.confirm_caught {
            let many = self.all_target_idx().len() >= 3;
                    let two = if many { None } else { self.target2_idx().map(|i| self.species[i].name.to_string()) };
            let (phase_a, phase_b, x_a) = if two.is_some() || many { (0.76, 0.90, 0.92) } else { (0.64, 0.86, 0.88) };
            body_text(
                p,
                egui::pos2(row.min.x, row.center().y),
                Align2::LEFT_CENTER,
                if two.is_some() || many { "Which?" } else { "Shiny! Was it…" },
                11.5,
                theme::pal().text,
            );
            if many {
                        // Three or more targets: pick in the Companion.
                        if ui
                            .put(at(0.22, 0.74), egui::Button::new(RichText::new("Choose…").strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                            .on_hover_text("Pick which of your targets it was")
                            .clicked()
                        {
                            self.setup.caught_pick = true;
                            self.show_setup = true;
                        }
                    } else if let Some(name2) = two {
                let short = |n: &str| if n.chars().count() > 8 { format!("{}…", n.chars().take(7).collect::<String>()) } else { n.to_string() };
                let name1 = self.target_idx().map(|i| self.species[i].name.to_string()).unwrap_or_default();
                if ui
                    .put(at(0.22, 0.47), egui::Button::new(RichText::new(short(&name1)).strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                    .on_hover_text(format!("Shiny {name1}! Logs this hunt and starts fresh"))
                    .clicked()
                {
                    self.log_hunt_as(self.cfg.target.clone());
                    self.confirm_caught = false;
                }
                if ui
                    .put(at(0.49, 0.74), egui::Button::new(RichText::new(short(&name2)).strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                    .on_hover_text(format!("Shiny {name2}! Logs this hunt and starts fresh"))
                    .clicked()
                {
                    self.log_hunt_as(self.cfg.target2.clone());
                    self.confirm_caught = false;
                }
            } else if ui
                .put(at(0.36, 0.62), egui::Button::new(RichText::new("Target").strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                .on_hover_text("Your target! Logs this hunt and starts fresh")
                .clicked()
            {
                self.log_hunt();
                self.confirm_caught = false;
            }
            if ui
                .put(at(phase_a, phase_b), egui::Button::new(RichText::new("Phase").strong().color(Color32::WHITE)).fill(GOLD.gamma_multiply(0.55)))
                .on_hover_text("A shiny of a different Pokémon: pick which, and this hunt keeps going")
                .clicked()
            {
                self.confirm_caught = false;
                self.setup.phase_pick = true;
                self.setup.search.clear();
                self.show_setup = true;
            }
            if ui.put(at(x_a, 1.0), egui::Button::new("✖").fill(theme::PLATE())).on_hover_text("Not a shiny after all").clicked() {
                self.confirm_caught = false;
            }
        } else {
            let gap = 6.0;
            let widths = [0.18, 0.18, 0.22, 0.42];
            let mut bx = row.min.x;
            let total = row.width() - gap * 3.0;
            for (i, wf) in widths.iter().enumerate() {
                let r = Rect::from_min_size(egui::pos2(bx, row.min.y), egui::vec2(total * wf, row.height()));
                bx += total * wf + gap;
                self.count_button(ui, r, i);
            }
        }
    }

    /// One of −1 / +1 / pause / Caught! (index 0..=3).
    fn count_button(&mut self, ui: &mut egui::Ui, r: Rect, i: usize) {
        let (label, tip) = match i {
            0 => ("−1".to_string(), format!("Remove one{}", self.cfg.keybinds.hint(HotAction::Minus))),
            1 => ("+1".to_string(), format!("Add one{}", self.cfg.keybinds.hint(HotAction::Plus))),
            2 => (
                if self.cfg.paused { "▶" } else { "⏸" }.to_string(),
                format!("Pause / resume auto count{}", self.cfg.keybinds.hint(HotAction::Pause)),
            ),
            _ => ("Caught!".to_string(), "Found your shiny? Log this hunt and start fresh".to_string()),
        };
        let caught_btn = i == 3;
        let ink = if caught_btn { theme::button_ink() } else { Color32::WHITE };
        let slot = ui.painter().add(egui::Shape::Noop);
        let btn = egui::Button::new(RichText::new(label).strong().color(ink))
            .fill(if caught_btn { Color32::TRANSPARENT } else { theme::PLATE() })
            .rounding(6.0);
        let resp = ui.put(r, btn);
        if caught_btn {
            ui.painter().set(slot, theme::button_shape(resp.rect, 6.0, resp.hovered()));
        }
        if resp.on_hover_text(tip).clicked() {
            match i {
                0 => self.add(-1),
                1 => self.add(1),
                2 => self.set_paused(!self.cfg.paused),
                _ => self.confirm_caught = true,
            }
        }
    }

    /// Today's hunting time with its clock (right-aligned at `right`).
    fn ov_timer(&self, ui: &mut egui::Ui, p: &egui::Painter, right: Pos2) {
        let secs = self.today_hunt_secs();
        let idle = self.timer_idle();
        let r = body_text(
            p,
            right,
            Align2::RIGHT_CENTER,
            crate::stats::fmt_duration(secs),
            10.5,
            if idle { MUTED().gamma_multiply(0.55) } else { MUTED() },
        );
        let c = egui::pos2(r.min.x - 8.0, right.y);
        if idle {
            for dx in [-1.8f32, 1.8] {
                p.rect_filled(Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(2.0, 7.0)), 0.5, MUTED().gamma_multiply(0.7));
            }
        } else {
            p.circle_stroke(c, 4.0, Stroke::new(1.2f32, MUTED()));
            p.line_segment([c, c + egui::vec2(0.0, -2.6)], Stroke::new(1.2f32, MUTED()));
            p.line_segment([c, c + egui::vec2(2.0, 0.0)], Stroke::new(1.2f32, MUTED()));
        }
        ui.interact(
            Rect::from_min_max(egui::pos2(r.min.x - 14.0, r.min.y - 2.0), r.max + egui::vec2(0.0, 2.0)),
            ui.id().with("today-timer"),
            Sense::hover(),
        )
        .on_hover_text(if idle {
            format!("Timer paused: no encounter for over {} minutes. It carries on with the next one.", self.cfg.idle_gap_secs() / 60)
        } else if self.cfg.idle_pause {
            format!("Time hunted today. Pauses after {} minutes with no encounter (Settings > Hunt).", self.cfg.idle_minutes)
        } else {
            "Time hunted today".to_string()
        });
    }

    pub(super) fn counter_label(&self) -> (&'static str, Color32) {
        if self.status.error.is_some() {
            ("CHECK SETUP ⚙", theme::HP_RED)
        } else if self.cfg.paused {
            ("PAUSED", MUTED())
        } else if self.game_closed() {
            ("GAME CLOSED", MUTED())
        } else {
            ("ENCOUNTERS", MUTED())
        }
    }

    /// Small pills: phases this run, daily streak, personal best.
    pub(super) fn ov_chips(&self, m: &OverlayModules, st: &OvStats) -> Vec<(String, Color32, bool, String)> {
        // (text, colour, flame icon, tooltip)
        let mut chips = Vec::new();
        if m.phases {
            let n = st.phases;
            chips.push((
                format!("{n} PHASE{}", if n == 1 { "" } else { "S" }),
                if n > 0 { GOLD } else { MUTED() },
                false,
                "Shinies of other Pokémon found during this hunt".to_string(),
            ));
        }
        if m.streak {
            let n = st.streak;
            chips.push((
                format!("{n}-DAY STREAK"),
                if n > 0 { FLAME } else { MUTED() },
                true,
                "Days in a row you've reached your daily goal".to_string(),
            ));
        }
        if m.best {
            if st.today > st.best_day && st.best_day > 0 {
                chips.push(("NEW BEST DAY!".to_string(), GOLD, false, format!("{} today beats your old best of {}", theme::thousands(st.today), theme::thousands(st.best_day))));
            } else {
                chips.push((
                    format!("BEST DAY {}", theme::thousands(st.best_day)),
                    MUTED(),
                    false,
                    format!("Your best single day. Today so far: {}", theme::thousands(st.today)),
                ));
            }
        }
        chips
    }

    /// The overlay as its own always-on-top window (while it's started).
    /// Where the overlay draws: filling its own window, or (for the Overlay
    /// page's live preview) in a box in the main window that ignores clicks.
    pub(super) fn ov_panel(&mut self, ctx: &egui::Context, add: impl FnOnce(&mut Self, &mut egui::Ui)) {
        match self.ov_preview {
            Some((r, clip)) => {
                egui::Area::new(egui::Id::new(super::overlay_page::PREVIEW_AREA))
                    .order(egui::Order::Middle)
                    .fixed_pos(r.min)
                    .constrain(false)
                    .interactable(false)
                    .show(ctx, |ui| {
                        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r));
                        c.set_clip_rect(clip);
                        add(self, &mut c);
                        // nothing in the preview can be clicked or dragged
                        let _ = ui.interact(r, egui::Id::new("overlay-preview-block"), Sense::click_and_drag());
                    });
            }
            None => {
                egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| add(self, ui));
            }
        }
    }

    pub(super) fn overlay_viewport(&mut self, ctx: &egui::Context) {
        let want = self.overlay_window_size();
        if self.ov_geom.is_none() {
            let key = self.overlay_layout_key();
            // Older settings kept one spot for every layout.
            if self.cfg.overlay_positions.is_empty() {
                if let Some(p) = self.cfg.window_pos {
                    self.cfg.overlay_positions.insert(key.to_string(), p);
                }
            }
            let saved = self.cfg.overlay_positions.get(key).copied();
            let pos = crate::topmost::fit_on_screen(saved, want, ctx.pixels_per_point(), true);
            self.ov_layout = Some(key);
            self.ov_geom = Some((pos, want));
            self.applied_size = Some(want);
            self.applied_passthrough = None;
            self.ov_opened = Some(Instant::now());
        }
        let Some((pos, size)) = self.ov_geom else { return };
        let builder = egui::ViewportBuilder::default()
            .with_title(crate::topmost::OVERLAY_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            // Fixed for this open session (size changes go through commands),
            // so egui never fights the window being dragged.
            .with_position(pos)
            .with_inner_size(size);
        // Medium / Large: everything the overlay draws is scaled up as one
        // picture (layout, hit-testing and pointer positions all follow), so
        // every theme keeps its exact proportions. The scale is only set while
        // the overlay window draws, so the main window never sees it.
        let scale = self.ov_scale();
        let layers = [egui::LayerId::background(), egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts"))];
        if scale != 1.0 {
            for l in layers {
                ctx.set_transform_layer(l, egui::emath::TSTransform::from_scaling(scale));
            }
        }
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("shiny-overlay"), builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                self.overlay_on = false;
            }
            self.sync_window(ctx);
            if self.cfg.mini {
                self.mini_ui(ctx);
            } else if self.cfg.overlay.legacy() {
                self.full_ui(ctx);
            } else {
                self.modular_ui(ctx);
            }
            self.clip_overlay(ctx);
        });
        if scale != 1.0 {
            for l in layers {
                ctx.set_transform_layer(l, egui::emath::TSTransform::IDENTITY);
            }
        }
    }

    /// The window shape a notice or pop-up card under the overlay needs:
    /// just the card itself, so the space around it stays see-through.
    pub(super) fn popup_shape(&self, area: Rect, theme: crate::config::OverlayTheme) -> [f32; 5] {
        let rad = super::ov_toast::skin(theme).map_or(10.0, |s| s.radius);
        let h = match self.notices.first() {
            Some(n) if n.progress.is_some() || n.action.is_some() => 84.0,
            Some(_) => 64.0,
            None => {
                // A pop-up opening or closing: just the part that's showing.
                if let Some(r) = self.toast_now(area) {
                    return [r.min.x, r.min.y, r.width(), r.height() + 1.0, rad.min(r.height() / 2.0)];
                }
                64.0
            }
        };
        [area.min.x, area.min.y, area.width(), h + 1.0, rad]
    }

    /// Cut the overlay window to the shape that was just drawn, so nothing of
    /// the window shows past the rounded corners (on some PCs the see-through
    /// corners come out black).
    fn clip_overlay(&mut self, ctx: &egui::Context) {
        let Some(hwnd) = self.overlay_keeper else { return };
        // shapes are in the overlay's own (unscaled) units
        let ppp = ctx.pixels_per_point() * self.ov_scale();
        let px = |v: f32| (v * ppp).round() as i32;
        // One pixel proud of the drawn edge, so the outline is never shaved.
        let key: Vec<[i32; 5]> = self
            .ov_shape
            .iter()
            .map(|r| [px(r[0]) - 1, px(r[1]) - 1, px(r[2]) + 2, px(r[3]) + 2, px(r[4]) + 1])
            .collect();
        if self.ov_clip.as_ref().is_some_and(|(h, k)| *h == hwnd && *k == key) {
            return;
        }
        crate::topmost::clip_window(hwnd, &key);
        self.ov_clip = Some((hwnd, key));
    }

    // ───────────────────────────── full / compact ─────────────────────────────

    pub(super) fn modular_ui(&mut self, ctx: &egui::Context) {
        let theme = self.cfg.overlay.theme;
        if theme != crate::config::OverlayTheme::Classic {
            self.themed_ui(ctx, theme);
            return;
        }
        if self.cfg.overlay.size == OverlaySize::Bar {
            self.bar_ui(ctx);
            return;
        }
        let mut m = self.cfg.overlay.clone();
        let compact = m.size == OverlaySize::Compact;
        if compact {
            // Compact: just the sprites and the counter (their own toggles
            // still apply). Caught! / Undo and alerts still show when needed.
            m = OverlayModules {
                style: m.style,
                theme: m.theme,
                size: m.size,
                scale: m.scale,
                click_through: m.click_through,
                status_dot: m.status_dot,
                sprites: m.sprites,
                single_sprite: m.single_sprite,
                counter: m.counter,
                target: false,
                buttons: false,
                odds_bar: false,
                method_line: false,
                luck: false,
                milestone: false,
                session: false,
                sparkline: false,
                last_seen: false,
                phases: false,
                xp: false,
                daily_goal: false,
                streak: false,
                best: false,
            };
        }
        // Liquid layout: the full overlay is as wide as the user dragged it.
        let liquid = self.cfg.liquid && !compact;
        let width = if compact {
            COMPACT_W
        } else if liquid {
            self.cfg.liquid_width.clamp(LIQUID_MIN, LIQUID_MAX)
        } else {
            FULL_SIZE[0]
        };
        let st = self.ov_stats();
        self.ov_panel(ctx, |this, ui| {
            let win = ui.max_rect();
            let full_w = Rect::from_min_size(win.min, egui::vec2(width, win.height().max(60.0))).shrink(1.0);
            let p = ui.painter().clone();
            let t = this.t_ms();
            let blink = (t / 350) % 2 == 0;
            let target_flash = this.target_flash.is_some_and(|f| f.elapsed() < Duration::from_millis(1600));

            let bg_slot = p.add(egui::Shape::Noop);
            let header = Rect::from_min_size(full_w.min, egui::vec2(full_w.width(), 32.0));
            this.ov_header(ui, ctx, &p, header, &m);
            let frame_slot = p.add(egui::Shape::Noop);

            let mut y = header.max.y + 10.0;
            let x0 = full_w.min.x + 12.0;
            let x1 = full_w.max.x - 12.0;
            let cx = (x0 + x1) / 2.0;
            let mut shown_any = false;

            let target = this.target_idx().map(|i| this.species[i].clone());
            // Every target (main first); two or more = a multi-target hunt.
            let all: Vec<crate::species::Species> = this.all_target_idx().into_iter().map(|i| this.species[i].clone()).collect();
            let multi = all.len() > 1;

            // ── Target plate
            if m.target {
                shown_any = true;
                let plate = Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 26.0));
                theme::plate(&p, plate, theme::PLATE());
                match &target {
                    Some(sp) => {
                        // What sits on the right of the plate, so the name can
                        // be fitted into what's left.
                        let watch = if !target_flash && !compact { this.target_watch() } else { None };
                        let right_txt = match (&watch, target_flash) {
                            (_, true) => Some(("SPOTTED!".to_string(), 7.0)),
                            (Some(w), _) if w.dry => Some((format!("NOT SEEN IN {}", theme::thousands(w.since.unwrap_or(0))), 6.5)),
                            (Some(w), _) => Some((format!("SEEN {}", theme::thousands(w.seen)), 6.5)),
                            _ => None,
                        };
                        let text_w = |t: &str, size: f32| p.layout_no_wrap(t.to_string(), crate::fonts::pixel_font(size), Color32::WHITE).size().x;
                        let right_w = right_txt.as_ref().map_or(0.0, |(t, sz)| text_w(t, *sz) + 18.0);
                        let room = plate.width() - 16.0 - 12.0 - right_w;
                        // Longest wording that fits: every name, then the
                        // first name + "N more", then smaller text.
                        let (name, size) = if multi {
                            let names: Vec<&str> = all.iter().map(|s| s.name).collect();
                            let n = names.len();
                            let mut options: Vec<String> = vec![names.join(" + ")];
                            if n > 2 {
                                options.push(format!("{} + {} + {} more", names[0], names[1], n - 2));
                            }
                            options.push(format!("{} + {} more", names[0], n - 1));
                            let mut pick = None;
                            'fit: for size in [8.0, 7.0, 6.0] {
                                for o in &options {
                                    let o = o.to_uppercase();
                                    if text_w(&o, size) <= room {
                                        pick = Some((o, size));
                                        break 'fit;
                                    }
                                }
                            }
                            pick.unwrap_or_else(|| (format!("{} +{}", names[0], n - 1).to_uppercase(), 6.0))
                        } else {
                            let nm = sp.name.to_uppercase();
                            let size = if text_w(&nm, 10.0) <= room { 10.0 } else { 8.0 };
                            (nm, size)
                        };
                        let name_clip = p.with_clip_rect(Rect::from_min_max(plate.min, egui::pos2(plate.max.x - right_w, plate.max.y)));
                        outlined_text(&name_clip, plate.left_center() + egui::vec2(16.0, 1.0), Align2::LEFT_CENTER, &name, size, Color32::WHITE);
                        if multi {
                            let full = all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ");
                            ui.interact(plate, ui.id().with("target-plate-names"), Sense::hover()).on_hover_text(format!("Hunting {full}"));
                        }
                        if target_flash && blink {
                            outlined_text(&p, plate.right_center() - egui::vec2(16.0, -1.0), Align2::RIGHT_CENTER, "SPOTTED!", 7.0, GOLD);
                        } else if !target_flash && !compact {
                            if let Some(w) = watch {
                                let (txt, col) = if w.dry {
                                    (format!("NOT SEEN IN {}", theme::thousands(w.since.unwrap_or(0))), theme::HP_YELLOW)
                                } else {
                                    (format!("SEEN {}", theme::thousands(w.seen)), MUTED())
                                };
                                outlined_text(&p, plate.right_center() - egui::vec2(12.0, -1.0), Align2::RIGHT_CENTER, &txt, 6.5, col);
                                let tip = w.tip(sp.name);
                                ui.interact(plate, ui.id().with("target-plate"), Sense::hover()).on_hover_text(tip);
                            }
                        }
                    }
                    None => {
                        outlined_text(&p, plate.center(), Align2::CENTER_CENTER, "PICK A TARGET ⚙", 8.0, MUTED());
                    }
                }
                y = plate.max.y + GAP;
            }

            // ── Sprites (the shiny alert / celebration use this space; it's
            // made for them when the sprites are hidden)
            let alerting = this.shiny_alert.is_some();
            let celebrating = this.celebrate.is_some();
            let mut fx_area: Option<Rect> = None;
            if m.sprites || alerting || celebrating {
                shown_any = true;
                let mut box_h = if compact { 84.0 } else { 118.0 };
                if alerting {
                    box_h = f32::max(box_h, 118.0);
                }
                if !m.sprites {
                    box_h = if alerting { 118.0 } else { 70.0 };
                }
                let area = Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + box_h));
                if m.sprites {
                    // One target: normal + shiny. Several: their shinies, a box
                    // each for up to three, all in one box beyond that (or when
                    // "shiny only" is on).
                    let shinies: Vec<(Option<crate::species::Species>, bool)> = all.iter().map(|sp| (Some(sp.clone()), true)).collect();
                    let boxes: Vec<(Rect, Vec<(Option<crate::species::Species>, bool)>)> = if m.single_sprite || all.len() > 3 {
                        let who = if multi { shinies } else { vec![(target.clone(), true)] };
                        vec![(area, who)]
                    } else {
                        let shows: Vec<(Option<crate::species::Species>, bool)> =
                            if multi { shinies } else { vec![(target.clone(), false), (target.clone(), true)] };
                        let n = shows.len() as f32;
                        let box_w = (area.width() - 8.0 * (n - 1.0)) / n;
                        shows
                            .into_iter()
                            .enumerate()
                            .map(|(k, s)| (Rect::from_min_size(egui::pos2(x0 + k as f32 * (box_w + 8.0), y), egui::vec2(box_w, box_h)), vec![s]))
                            .collect()
                    };
                    for (b, who) in boxes {
                        let shiny = who.iter().any(|w| w.1);
                        p.rect_filled(b, 8.0, theme::PANEL().gamma_multiply(0.9));
                        p.rect_stroke(b, 8.0, Stroke::new(1.5f32, if shiny { GOLD.gamma_multiply(0.8) } else { theme::BORDER() }));
                        if shiny {
                            theme::sparkle(&p, b.right_top() + egui::vec2(-12.0, 12.0), 6.0, GOLD);
                        }
                        let n = who.len() as f32;
                        let slot_w = b.width() / n;
                        let max_w = if n > 1.0 { slot_w - 8.0 } else { (b.width() / 2.0 - 12.0).max(box_h - 24.0).min(b.width() - 12.0) };
                        for (k, (sp, sh)) in who.into_iter().enumerate() {
                            let foot = egui::pos2(b.min.x + slot_w * (k as f32 + 0.5), b.max.y - 14.0);
                            theme::platform(&p, foot + egui::vec2(0.0, 2.0), (slot_w * 0.38).min(52.0), if compact { 6.0 } else { 8.0 });
                            if let Some(sp) = &sp {
                                this.draw_sprite(&p, sp, sh, foot + egui::vec2(0.0, 4.0), egui::vec2(max_w, box_h - 24.0));
                            }
                        }
                    }
                }
                fx_area = Some(area);
                y += box_h + 10.0;
            }
            if let (Some(area), Some(plate_idx)) = (fx_area, this.shiny_alert) {
                this.ov_shiny_banner(ui, &p, area, plate_idx, blink);
            }
            if let Some(area) = fx_area {
                this.ov_celebrate(ctx, &p, area, false);
            }

            // ── Counter (+ today's time)
            if m.counter {
                shown_any = true;
                let big = if compact { 20.0 } else { 24.0 };
                outlined_text(&p, egui::pos2(cx, y + big * 0.58), Align2::CENTER_CENTER, &theme::thousands(this.cfg.count), big, Color32::WHITE);
                y += if compact { 27.0 } else { 32.0 };
                let (label, label_color) = this.counter_label();
                outlined_text(&p, egui::pos2(cx, y + 4.0), Align2::CENTER_CENTER, label, 8.0, label_color);
                if this.cfg.show_session_timer {
                    this.ov_timer(ui, &p, egui::pos2(x1, y + 4.0));
                }
                y += 20.0;
            }

            // ── Odds bar + method line
            if m.odds_bar {
                shown_any = true;
                let o = this.ov_odds();
                outlined_text(&p, egui::pos2(x0, y + 5.0), Align2::LEFT_CENTER, &o.left, 7.0, o.color);
                body_text(&p, egui::pos2(x1, y + 5.0), Align2::RIGHT_CENTER, &o.right, 11.0, MUTED());
                let top = y;
                y += 14.0;
                let bar = Rect::from_min_max(egui::pos2(x0 + 2.0, y), egui::pos2(x1 - 2.0, y + 8.0));
                theme::gauge(&p, bar, o.frac, o.color);
                y += 15.0;
                ui.interact(Rect::from_min_max(egui::pos2(x0 - 4.0, top - 2.0), egui::pos2(x1 + 4.0, y - 4.0)), ui.id().with("odds"), Sense::hover())
                    .on_hover_text(o.tip);
            }
            if m.method_line {
                shown_any = true;
                let mut caption = this.cfg.odds_caption();
                if let Some(one_in) = this.cfg.odds_one_in() {
                    if this.cfg.pokemon_per_battle().map_or(true, |n| n > 1.0) {
                        caption.push_str(&format!(" · 1/{} each", fmt_one_in(one_in)));
                    }
                }
                let cy = if m.odds_bar { y } else { y + 6.0 };
                body_text(&p, egui::pos2(cx, cy), Align2::CENTER_CENTER, caption, 10.5, MUTED().gamma_multiply(0.85));
                y = cy + 15.0;
            } else if m.odds_bar {
                y += 1.0;
            }

            // ── Luck · next milestone
            let mut cells: Vec<(&str, String, Color32, String)> = Vec::new();
            if m.luck {
                match this.ov_luck() {
                    Some((pct, col, tip)) => cells.push(("LUCK", format!("{pct:.0}% would have it"), col, tip)),
                    None => cells.push(("LUCK", "after 1st battle".to_string(), MUTED(), "Needs the odds first (start a battle).".to_string())),
                }
            }
            if m.milestone {
                let (what, when, tip) = this.ov_next_milestone();
                cells.push(("NEXT MILESTONE", format!("{what} {when}"), Color32::WHITE, tip));
            }
            if !cells.is_empty() {
                shown_any = true;
                y = this.ov_cells(ui, &p, x0, x1, y, 32.0, cells, "luck");
            }

            // ── Session strip
            if m.session {
                shown_any = true;
                let now = now_unix();
                let (enc, secs) = this.cfg.run_stats.current_session(now).map_or((0, 0), |s| (s.encounters, s.secs));
                let rate = if secs >= 60 { format!("{}/h", theme::thousands(enc * 3600 / secs)) } else { "—".to_string() };
                let cells = vec![
                    ("SESSION", theme::thousands(enc), Color32::WHITE, "Encounters this session (a session ends after a long break)".to_string()),
                    ("TIME", if secs > 0 { crate::stats::fmt_duration(secs) } else { "—".to_string() }, Color32::WHITE, "Hunting time this session".to_string()),
                    ("PACE", rate, Color32::WHITE, "Encounters per hour this session".to_string()),
                ];
                y = this.ov_cells(ui, &p, x0, x1, y, 32.0, cells, "session");
            }

            // ── Speed sparkline
            if m.sparkline {
                shown_any = true;
                let r = Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 40.0));
                cell(&p, r);
                tiny(&p, egui::pos2(r.min.x + 8.0, r.min.y + 9.0), Align2::LEFT_CENTER, "PACE · LAST 20 MIN", MUTED());
                body_text(&p, egui::pos2(r.max.x - 8.0, r.min.y + 9.0), Align2::RIGHT_CENTER, format!("{}/h", theme::thousands(this.ov_pace_per_hour())), 10.5, Color32::WHITE);
                let g = Rect::from_min_max(egui::pos2(r.min.x + 8.0, r.min.y + 17.0), egui::pos2(r.max.x - 8.0, r.max.y - 6.0));
                this.sparkline(&p, g, &this.ov_pace(20));
                ui.interact(r, ui.id().with("pace"), Sense::hover())
                    .on_hover_text("Encounters each minute for the last 20 minutes (gold is this minute). The number is your pace over the last 10 minutes.");
                y = r.max.y + GAP;
            }

            // ── Last encounter
            if m.last_seen {
                shown_any = true;
                let names: Vec<&str> = this.last_battle.iter().filter_map(|&i| this.species.get(i)).map(|s| s.name).collect();
                let text = if names.is_empty() { "—".to_string() } else { names.join(" · ") };
                let ago = this.last_enc_at.map(|t| {
                    let s = t.elapsed().as_secs();
                    if s < 60 { format!("{s}s ago") } else if s < 3600 { format!("{}m ago", s / 60) } else { format!("{}h ago", s / 3600) }
                });
                let l = tiny(&p, egui::pos2(x0, y + 7.0), Align2::LEFT_CENTER, "LAST", MUTED());
                let right = ago.map(|a| body_text(&p, egui::pos2(x1, y + 7.0), Align2::RIGHT_CENTER, a, 10.5, MUTED()).min.x).unwrap_or(x1);
                let clip = Rect::from_min_max(egui::pos2(l.max.x + 6.0, y - 2.0), egui::pos2(right - 6.0, y + 16.0));
                p.with_clip_rect(clip).text(egui::pos2(clip.min.x, y + 7.0), Align2::LEFT_CENTER, text, egui::FontId::proportional(11.0), Color32::WHITE);
                y += 14.0 + GAP;
            }

            // ── Chips: phases · streak · personal best
            let chips = this.ov_chips(&m, &st);
            if !chips.is_empty() {
                shown_any = true;
                let mut x = x0;
                let h = 18.0;
                for (text, col, fire, tip) in chips {
                    let galley = p.layout_no_wrap(text.clone(), crate::fonts::pixel_font(6.5), col);
                    let w = galley.size().x + 16.0 + if fire { 10.0 } else { 0.0 };
                    if x + w > x1 + 0.5 && x > x0 {
                        x = x0;
                        y += h + 5.0;
                    }
                    let r = Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
                    p.rect_filled(r, 9.0, theme::PLATE());
                    p.rect_stroke(r, 9.0, Stroke::new(1.0f32, if col == MUTED() { theme::BORDER() } else { col.gamma_multiply(0.7) }));
                    let mut tx = r.min.x + 8.0;
                    if fire {
                        flame(&p, egui::pos2(tx + 3.0, r.center().y), 8.0, col);
                        tx += 10.0;
                    }
                    tiny(&p, egui::pos2(tx, r.center().y + 0.5), Align2::LEFT_CENTER, &text, col);
                    ui.interact(r, ui.id().with(("chip", text)), Sense::hover()).on_hover_text(tip);
                    x += w + 6.0;
                }
                y += h + GAP;
            }

            // ── Progress bars: trainer XP · daily goal
            if m.xp {
                shown_any = true;
                tiny(&p, egui::pos2(x0, y + 5.0), Align2::LEFT_CENTER, &format!("TRAINER LV {}", st.level.max(1)), XP_BLUE);
                body_text(&p, egui::pos2(x1, y + 5.0), Align2::RIGHT_CENTER, format!("{} / {} XP", theme::thousands(st.xp_into), theme::thousands(st.xp_need)), 10.5, MUTED());
                let bar = Rect::from_min_max(egui::pos2(x0 + 2.0, y + 14.0), egui::pos2(x1 - 2.0, y + 19.0));
                theme::gauge(&p, bar, st.xp_into as f32 / st.xp_need.max(1) as f32, XP_BLUE);
                ui.interact(Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 22.0)), ui.id().with("xp"), Sense::hover())
                    .on_hover_text("Trainer level progress (Profile has the details)");
                y += 21.0 + GAP;
            }
            if m.daily_goal {
                shown_any = true;
                let goal = this.cfg.daily_goal;
                let done = goal > 0 && st.today >= goal;
                let col = if done { GOLD } else { theme::GREEN };
                tiny(&p, egui::pos2(x0, y + 5.0), Align2::LEFT_CENTER, if done { "DAILY GOAL ✔" } else { "DAILY GOAL" }, col);
                let right = if goal > 0 { format!("{} / {}", theme::thousands(st.today), theme::thousands(goal)) } else { "off".to_string() };
                body_text(&p, egui::pos2(x1, y + 5.0), Align2::RIGHT_CENTER, right, 10.5, MUTED());
                let bar = Rect::from_min_max(egui::pos2(x0 + 2.0, y + 14.0), egui::pos2(x1 - 2.0, y + 19.0));
                theme::gauge(&p, bar, if goal > 0 { st.today as f32 / goal as f32 } else { 0.0 }, col);
                ui.interact(Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 22.0)), ui.id().with("daily"), Sense::hover())
                    .on_hover_text("Encounters today across every run, towards your daily goal (Profile > Goals)");
                y += 21.0 + GAP;
            }

            if !shown_any && !m.buttons {
                body_text(&p, egui::pos2(cx, y + 6.0), Align2::CENTER_CENTER, "Every section is off (⚙ > Overlay)", 11.0, MUTED());
                y += 20.0;
            }

            // ── Buttons (the Caught! question / Undo show even when hidden)
            let need_row = m.buttons || this.confirm_caught || this.undo_caught_left().is_some();
            if need_row {
                let h = if compact { 28.0 } else { 30.0 };
                let row = Rect::from_min_max(egui::pos2(x0, y + 2.0), egui::pos2(x1, y + 2.0 + h));
                this.ov_buttons(ui, ctx, &p, row);
                y = row.max.y + 10.0;
            } else {
                y += 2.0;
            }

            let full = Rect::from_min_max(full_w.min, egui::pos2(full_w.max.x, y.max(header.max.y + 24.0)));

            let border = if (this.shiny_alert.is_some() || target_flash) && blink { GOLD } else { OUTLINE };
            let body = Rect::from_min_max(egui::pos2(full.min.x, header.max.y - 1.0), full.max);
            p.set(
                bg_slot,
                egui::Shape::rect_filled(
                    body,
                    egui::Rounding { nw: 0.0, ne: 0.0, sw: 12.0, se: 12.0 },
                    theme::BG().gamma_multiply(this.cfg.opacity),
                ),
            );
            p.set(
                frame_slot,
                egui::Shape::rect_stroke(full, 12.0, Stroke::new(if border == GOLD { 2.5f32 } else { 1.5f32 }, border)),
            );

            // Notices and pop-ups hover just under the overlay, never over it.
            let below = !this.notices.is_empty() || this.toasts_pending();
            this.ov_shape.clear();
            this.ov_shape.push([full.min.x, full.min.y, full.width(), full.height(), 12.0]);
            if below {
                let area = Rect::from_min_max(egui::pos2(full.min.x, full.max.y + 6.0), egui::pos2(full.max.x, full.max.y + 6.0 + BELOW_H - 8.0));
                if !this.notices.is_empty() {
                    this.draw_notice(ui, ctx, area, crate::config::OverlayTheme::Classic);
                }
                if !this.toasts.is_empty() {
                    let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts")));
                    this.draw_toast(ctx, &fg, area, crate::config::OverlayTheme::Classic);
                }
                let shape = this.popup_shape(area, crate::config::OverlayTheme::Classic);
                this.ov_shape.push(shape);
            }

            // Liquid layout: drag the right edge to change the width.
            if liquid {
                let grip = Rect::from_min_max(egui::pos2(full.max.x - 7.0, header.max.y + 2.0), egui::pos2(full.max.x, full.max.y - 12.0));
                let gr = ui.interact(grip, ui.id().with("liquid-edge"), Sense::drag());
                if gr.hovered() || gr.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    let x = full.max.x - 3.0;
                    p.line_segment([egui::pos2(x, grip.center().y - 16.0), egui::pos2(x, grip.center().y + 16.0)], Stroke::new(3.0f32, GOLD));
                }
                if gr.dragged() {
                    this.cfg.liquid_width = (width + gr.drag_delta().x).clamp(LIQUID_MIN, LIQUID_MAX);
                }
                if gr.drag_stopped() {
                    this.dirty = true;
                }
                gr.on_hover_text("Drag to resize the overlay");
            }

            this.ov_size = Some([width, (full.max.y - win.min.y + 1.0 + if below { BELOW_H + 6.0 } else { 0.0 }).ceil()]);
        });
    }

    /// A row of equal stat cells (tiny label over a value). Returns the next y.
    #[allow(clippy::too_many_arguments)]
    fn ov_cells(&self, ui: &mut egui::Ui, p: &egui::Painter, x0: f32, x1: f32, y: f32, h: f32, cells: Vec<(&str, String, Color32, String)>, id: &str) -> f32 {
        let n = cells.len().max(1) as f32;
        let gap = 6.0;
        let w = (x1 - x0 - gap * (n - 1.0)) / n;
        for (k, (label, value, col, tip)) in cells.into_iter().enumerate() {
            let r = Rect::from_min_size(egui::pos2(x0 + k as f32 * (w + gap), y), egui::vec2(w, h));
            cell(p, r);
            tiny(p, egui::pos2(r.min.x + 8.0, r.min.y + 9.0), Align2::LEFT_CENTER, label, MUTED());
            let clip = r.shrink2(egui::vec2(6.0, 0.0));
            p.with_clip_rect(clip).text(egui::pos2(r.min.x + 8.0, r.max.y - 9.0), Align2::LEFT_CENTER, value, egui::FontId::proportional(12.0), col);
            ui.interact(r, ui.id().with((id, k)), Sense::hover()).on_hover_text(tip);
        }
        y + h + GAP
    }

    // ───────────────────────────── bar ─────────────────────────────

    /// One slim strip: sprite, name + count, odds, luck, next, session,
    /// pace, streak, buttons, then settings / hide / close.
    fn bar_ui(&mut self, ctx: &egui::Context) {
        let m = self.cfg.overlay.clone();
        let st = self.ov_stats();
        self.ov_panel(ctx, |this, ui| {
            let win = ui.max_rect();
            let p = ui.painter().clone();
            let t = this.t_ms();
            let blink = (t / 350) % 2 == 0;
            let target_flash = this.target_flash.is_some_and(|f| f.elapsed() < Duration::from_millis(1600));
            let top = win.min.y + 1.0;
            let left = win.min.x + 1.0;
            let mid = top + BAR_H / 2.0;
            let bg_slot = p.add(egui::Shape::Noop);
            let accent_slot = p.add(egui::Shape::Noop);

            // Drag anywhere that isn't a button.
            let drag_w = this.ov_size.map_or(400.0, |s| s[0]);
            let drag = ui.interact(
                Rect::from_min_size(egui::pos2(left, top), egui::vec2(drag_w, BAR_H)),
                ui.id().with("bar-drag"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() && !this.cfg.pinned {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            drag.on_hover_text(if this.cfg.pinned { "Pinned: unpin to move" } else { "Drag to move" });

            let mut x = left + 10.0;
            let divider = |p: &egui::Painter, x: &mut f32| {
                p.line_segment([egui::pos2(*x, top + 12.0), egui::pos2(*x, top + BAR_H - 12.0)], Stroke::new(1.0f32, theme::BORDER()));
                *x += 10.0;
            };

            // several targets take turns, one at a time
            let turn = this.bar_rotation(ctx);
            // Sprite
            if m.sprites {
                let foot = egui::pos2(x + 22.0, top + BAR_H - 6.0);
                theme::platform(&p, foot, 18.0, 4.5);
                match turn.as_ref().map(|t| t.0).or_else(|| this.target_idx()).map(|i| this.species[i].clone()) {
                    Some(sp) => {
                        this.draw_sprite(&p, &sp, true, foot + egui::vec2(0.0, 2.0), egui::vec2(44.0, 40.0));
                    }
                    None => {
                        p.image(
                            this.icon_tex.id(),
                            Rect::from_center_size(egui::pos2(x + 22.0, mid), egui::vec2(28.0, 28.0)),
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
                x += 50.0;
            }

            // Name + count (or SHINY!)
            if let Some(plate_idx) = this.shiny_alert {
                let r1 = outlined_text(&p, egui::pos2(x, top + 17.0), Align2::LEFT_CENTER, "SHINY!", 13.0, if blink { GOLD } else { Color32::WHITE });
                let r2 = body_text(&p, egui::pos2(x, top + 36.0), Align2::LEFT_CENTER, if plate_idx == super::GIFT_PLATE { "Shiny starter!".to_string() } else { format!("Star on plate {}", plate_idx + 1) }, 10.5, Color32::WHITE);
                x = r1.max.x.max(r2.max.x) + 10.0;
                let dr = Rect::from_min_size(egui::pos2(x, mid - 12.0), egui::vec2(64.0, 24.0));
                if ui.put(dr, egui::Button::new("Dismiss").fill(theme::PLATE())).clicked() {
                    this.shiny_alert = None;
                    crate::sound::stop_song();
                }
                x = dr.max.x + 10.0;
            } else if m.target || m.counter {
                let mut w: f32 = 0.0;
                if m.target {
                    let col = if target_flash && blink { GOLD } else if this.target_idx().is_some() { Color32::WHITE } else { MUTED() };
                    let y = if m.counter { top + 15.0 } else { mid };
                    match &turn {
                        Some((_, name, k, n)) => {
                            // the current one's name, "1/3" after it; room for the longest
                            let r = outlined_text(&p, egui::pos2(x, y), Align2::LEFT_CENTER, &name.to_uppercase(), 7.5, col);
                            let widest = this.bar_names().iter().map(|nm| p.layout_no_wrap(nm.clone(), crate::fonts::pixel_font(7.5), Color32::WHITE).size().x).fold(r.width(), f32::max);
                            let tag = tiny(&p, egui::pos2(x + widest + 6.0, y), Align2::LEFT_CENTER, &format!("{}/{}", k + 1, n), MUTED());
                            w = w.max(widest + 6.0 + tag.width());
                        }
                        None => {
                            let name = this.targets_title().map(|t| t.to_uppercase()).unwrap_or_else(|| "PICK A TARGET".to_string());
                            w = w.max(outlined_text(&p, egui::pos2(x, y), Align2::LEFT_CENTER, &name, 7.5, col).width());
                        }
                    }
                }
                if m.counter {
                    let y = if m.target { top + 34.0 } else { mid };
                    let r = outlined_text(&p, egui::pos2(x, y), Align2::LEFT_CENTER, &theme::thousands(this.cfg.count), 16.0, Color32::WHITE);
                    let (label, col) = this.counter_label();
                    if label != "ENCOUNTERS" {
                        tiny(&p, egui::pos2(r.max.x + 6.0, y + 2.0), Align2::LEFT_CENTER, label, col);
                    }
                    w = w.max(r.width() + if label != "ENCOUNTERS" { 70.0 } else { 0.0 });
                }
                x += w + 10.0;
            }

            // Odds
            if m.odds_bar {
                divider(&p, &mut x);
                let o = this.ov_odds();
                let label = o.mult.map_or("ODDS —".to_string(), |v| if v < 0.1 { format!("{v:.2}× ODDS") } else { format!("{v:.1}× ODDS") });
                tiny(&p, egui::pos2(x, top + 17.0), Align2::LEFT_CENTER, &label, o.color);
                let bar = Rect::from_min_size(egui::pos2(x + 1.0, top + 28.0), egui::vec2(64.0, 6.0));
                theme::gauge(&p, bar, o.frac, o.color);
                ui.interact(Rect::from_min_size(egui::pos2(x, top + 6.0), egui::vec2(68.0, BAR_H - 12.0)), ui.id().with("bar-odds"), Sense::hover())
                    .on_hover_text(o.tip);
                x += 74.0;
            }

            // Little stat columns: tiny label over a value.
            let stat = |ui: &mut egui::Ui, x: &mut f32, label: &str, value: String, col: Color32, tip: String| {
                divider(&p, x);
                let a = tiny(&p, egui::pos2(*x, top + 17.0), Align2::LEFT_CENTER, label, MUTED());
                let b = body_text(&p, egui::pos2(*x, top + 33.0), Align2::LEFT_CENTER, value, 12.0, col);
                let w = a.width().max(b.width());
                ui.interact(Rect::from_min_size(egui::pos2(*x, top + 6.0), egui::vec2(w, BAR_H - 12.0)), ui.id().with(("bar-stat", label)), Sense::hover())
                    .on_hover_text(tip);
                *x += w + 10.0;
            };
            if m.luck {
                if let Some((pct, col, tip)) = this.ov_luck() {
                    stat(ui, &mut x, "LUCK", format!("{pct:.0}%"), col, tip);
                }
            }
            if m.milestone {
                let (what, when, tip) = this.ov_next_milestone();
                stat(ui, &mut x, "NEXT", format!("{what} {when}"), Color32::WHITE, tip);
            }
            if m.session {
                let enc = this.cfg.run_stats.current_session(now_unix()).map_or(0, |s| s.encounters);
                stat(ui, &mut x, "SESSION", theme::thousands(enc), Color32::WHITE, "Encounters this session".to_string());
            }
            if m.streak && st.streak > 0 {
                stat(ui, &mut x, "STREAK", format!("{} day{}", st.streak, if st.streak == 1 { "" } else { "s" }), FLAME, "Days in a row you've reached your daily goal".to_string());
            }
            if m.sparkline {
                divider(&p, &mut x);
                tiny(&p, egui::pos2(x, top + 14.0), Align2::LEFT_CENTER, &format!("{}/H", theme::thousands(this.ov_pace_per_hour())), MUTED());
                let g = Rect::from_min_size(egui::pos2(x, top + 21.0), egui::vec2(60.0, 18.0));
                this.sparkline(&p, g, &this.ov_pace(15));
                ui.interact(g.expand(4.0), ui.id().with("bar-pace"), Sense::hover()).on_hover_text("Encounters each minute, last 15 minutes");
                x += 70.0;
            }

            // Buttons / Caught! question / Undo
            let undo_left = if this.confirm_caught { None } else { this.undo_caught_left() };
            let bh = 26.0;
            let btn_at = |x: f32, w: f32| Rect::from_min_size(egui::pos2(x, mid - bh / 2.0), egui::vec2(w, bh));
            if this.shiny_alert.is_none() {
                if let Some(left_s) = undo_left {
                    divider(&p, &mut x);
                    if ui.put(btn_at(x, 70.0), egui::Button::new(RichText::new(format!("Undo ({left_s})")).strong().color(Color32::WHITE)).fill(theme::PLATE()))
                        .on_hover_text("Logged as caught. Pressed it by mistake? Put everything back")
                        .clicked()
                    {
                        this.undo_caught_now();
                    }
                    x += 74.0;
                    if ui.put(btn_at(x, 34.0), egui::Button::new("OK").fill(theme::PLATE())).clicked() {
                        this.undo_caught = None;
                    }
                    x += 44.0;
                    ctx.request_repaint_after(Duration::from_millis(250));
                } else if this.confirm_caught {
                    divider(&p, &mut x);
                    let all = this.all_target_idx();
                    if all.len() > 3 {
                        if ui.put(btn_at(x, 76.0), egui::Button::new(RichText::new("Choose…").strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                            .on_hover_text("Pick which of your targets it was")
                            .clicked()
                        {
                            this.setup.caught_pick = true;
                            this.show_setup = true;
                        }
                        x += 80.0;
                    } else {
                        let targets: Vec<(Option<String>, String)> = if all.len() <= 1 {
                            vec![(this.cfg.target.clone(), "Target".to_string())]
                        } else {
                            all.iter().map(|&i| (Some(this.species[i].slug.clone()), this.species[i].name.to_string())).collect()
                        };
                        for (slug, name) in targets {
                            let short = if name.chars().count() > 8 { format!("{}…", name.chars().take(7).collect::<String>()) } else { name.clone() };
                            if ui.put(btn_at(x, 66.0), egui::Button::new(RichText::new(short).strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                                .on_hover_text(format!("Shiny {name}! Logs this hunt and starts fresh"))
                                .clicked()
                            {
                                this.log_hunt_as(slug);
                                this.confirm_caught = false;
                            }
                            x += 70.0;
                        }
                    }
                    if ui.put(btn_at(x, 54.0), egui::Button::new(RichText::new("Phase").strong().color(Color32::WHITE)).fill(GOLD.gamma_multiply(0.55)))
                        .on_hover_text("A shiny of a different Pokémon: pick which, and this hunt keeps going")
                        .clicked()
                    {
                        this.confirm_caught = false;
                        this.setup.phase_pick = true;
                        this.setup.search.clear();
                        this.show_setup = true;
                    }
                    x += 58.0;
                    if ui.put(btn_at(x, 28.0), egui::Button::new("✖").fill(theme::PLATE())).on_hover_text("Not a shiny after all").clicked() {
                        this.confirm_caught = false;
                    }
                    x += 38.0;
                } else if m.buttons {
                    divider(&p, &mut x);
                    for (i, w) in [(0usize, 30.0f32), (1, 30.0), (2, 30.0), (3, 64.0)] {
                        this.count_button(ui, btn_at(x, w), i);
                        x += w + 5.0;
                    }
                    x += 5.0;
                }
            }

            // Status light, settings, hide, close.
            if m.status_dot {
                let (dot, status_text) = this.status_line();
                let c = egui::pos2(x + 2.0, mid);
                p.circle_filled(c, 3.5, dot);
                p.circle_stroke(c, 3.5, Stroke::new(1.0f32, OUTLINE));
                ui.interact(Rect::from_center_size(c, egui::vec2(14.0, 14.0)), ui.id().with("bar-dot"), Sense::hover()).on_hover_text(status_text);
                x += 12.0;
            }
            let small_btn = |ui: &mut egui::Ui, x: f32, icon: &str, on: bool, tip: &str| {
                let r = Rect::from_min_size(egui::pos2(x, mid - 11.0), egui::vec2(20.0, 22.0));
                ui.put(r, egui::Button::new(RichText::new(icon).size(12.5).color(if on { GOLD } else { Color32::WHITE })).frame(false))
                    .on_hover_text(tip)
                    .clicked()
            };
            if small_btn(ui, x, if this.cfg.pinned { "🔒" } else { "🔓" }, this.cfg.pinned, "Pin in place") {
                this.cfg.pinned = !this.cfg.pinned;
                this.dirty = true;
            }
            x += 21.0;
            if small_btn(ui, x, "⚙", this.show_setup, "Setup: target, screen boxes, odds, overlay sections") {
                this.show_setup = !this.show_setup;
            }
            x += 21.0;
            let hide_tip = format!("Hide to badge{}", this.cfg.keybinds.hint(HotAction::ToggleHide));
            if small_btn(ui, x, "—", false, &hide_tip) {
                this.cfg.mini = true;
                this.dirty = true;
            }
            x += 21.0;
            this.close_button(ui, ctx, &p, Rect::from_min_size(egui::pos2(x, mid - 11.0), egui::vec2(20.0, 22.0)));
            x += 26.0;

            let width = (x - win.min.x).ceil().max(160.0);
            let bar = Rect::from_min_size(egui::pos2(left, top), egui::vec2(width - 2.0, BAR_H));
            let border = if (this.shiny_alert.is_some() || target_flash) && blink { GOLD } else { OUTLINE };
            p.set(bg_slot, egui::Shape::rect_filled(bar, 12.0, theme::BG().gamma_multiply(this.cfg.opacity)));
            // Thin red accent along the top, like the full overlay's header.
            p.set(
                accent_slot,
                egui::Shape::rect_filled(
                    Rect::from_min_size(bar.min + egui::vec2(10.0, 0.0), egui::vec2(bar.width() - 20.0, 3.0)),
                    egui::Rounding { nw: 0.0, ne: 0.0, sw: 2.0, se: 2.0 },
                    theme::RED(),
                ),
            );
            p.rect_stroke(bar, 12.0, Stroke::new(if border == GOLD { 2.5f32 } else { 1.5f32 }, border));
            if this.click_through_active() {
                tiny(&p, egui::pos2(bar.max.x - 12.0, bar.max.y - 6.0), Align2::RIGHT_CENTER, "CLICK-THROUGH", GOLD);
            }
            if this.celebrate.is_some() {
                this.ov_celebrate(ctx, &p, bar.shrink(2.0), true);
            }

            // Notices and pop-ups get a card under the bar while they show.
            let mut h = BAR_H + 2.0;
            this.ov_shape.clear();
            this.ov_shape.push([bar.min.x, bar.min.y, bar.width(), bar.height(), 12.0]);
            if !this.notices.is_empty() || this.toasts_pending() {
                // A compact card pinned under the bar's right end.
                let right = bar.max.x.max(left + BAR_POPUP_W);
                let area = Rect::from_min_max(egui::pos2(right - BAR_POPUP_W, bar.max.y + 6.0), egui::pos2(right, bar.max.y + 100.0));
                let shape = this.popup_shape(area, crate::config::OverlayTheme::Classic);
                this.ov_shape.push(shape);
                if !this.notices.is_empty() {
                    this.draw_notice(ui, ctx, area, crate::config::OverlayTheme::Classic);
                }
                if !this.toasts.is_empty() {
                    let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts")));
                    this.draw_toast(ctx, &fg, area, crate::config::OverlayTheme::Classic);
                }
                h = BAR_H + 104.0;
            }
            let w = if h > BAR_H + 2.0 { width.max(BAR_POPUP_W + 2.0) } else { width };
            this.ov_size = Some([w, h]);
        });
    }
}
