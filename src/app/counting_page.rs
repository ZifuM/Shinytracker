//! The Counting page: is it counting, and where from. A live status card
//! with the count and its controls, a feed of the last battles and the
//! pace, where encounters come from, automation, hotkeys and the name
//! reading setup.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PURPLE, RED, YELLOW};
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::{App, Tab};
use crate::config::CountSource;
use crate::hotkeys::HotAction;
use crate::stats::now_unix;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// How counting is doing right now.
struct State {
    col: Color32,
    title: &'static str,
    detail: String,
    live: bool,
}

impl App {
    fn count_state(&self) -> State {
        let s = &self.status;
        let st = |col, title, detail: String, live| State { col, title, detail, live };
        if self.game_closed() {
            return st(MUTED(), "Paused", "Platinum Together isn't running. Counting starts again when you open it.".into(), false);
        }
        if self.cfg.paused {
            return st(ORANGE, "Paused", "Auto counting is off. Resume to count encounters again.".into(), false);
        }
        if let Some(e) = &s.error {
            return st(RED, "Can't read encounters", e.clone(), false);
        }
        match self.cfg.count_source {
            CountSource::GameLog if s.text.is_empty() => st(YELLOW, "Waiting for the game", "Start Platinum Together and counting begins by itself.".into(), false),
            CountSource::GameLog if s.in_battle => st(GOLD, "In a battle", format!("Reading {}", s.text), true),
            CountSource::GameLog => st(GREEN, "Counting live", format!("Reading {}", s.text), true),
            CountSource::Screen if self.cfg.name_regions.is_empty() => st(YELLOW, "Set up name reading", "Draw a box over each enemy name (below) so the screen can be read.".into(), false),
            CountSource::Screen if s.in_battle => st(GOLD, "In a battle", "Reading the enemy names".into(), true),
            CountSource::Screen => st(GREEN, "Watching the screen", "Counting starts when a battle does.".into(), true),
        }
    }

    pub(super) fn page_counting(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "COUNTING", "Is it counting, and where from. Your encounters, live.");
        self.count_hero(ui);
        ui.add_space(18.0);
        let w = ui.available_width();
        if w >= 980.0 {
            let gap = 18.0;
            let side_w = (w * 0.4).clamp(340.0, 460.0);
            let main_w = w - side_w - gap;
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(main_w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(main_w);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.count_source_card(ui);
                    ui.add_space(18.0);
                    self.count_auto_card(ui);
                });
                ui.allocate_ui_with_layout(vec2(side_w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(side_w);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.count_feed_card(ui);
                    ui.add_space(18.0);
                    self.count_keys_card(ui);
                });
            });
        } else {
            self.count_feed_card(ui);
            ui.add_space(18.0);
            self.count_source_card(ui);
            ui.add_space(18.0);
            self.count_auto_card(ui);
            ui.add_space(18.0);
            self.count_keys_card(ui);
        }
        ui.add_space(18.0);
        self.count_names_card(ui);
        ui.add_space(30.0);
    }

    // ───────────────────────────── live status ─────────────────────────────

    fn count_hero(&mut self, ui: &mut egui::Ui) {
        let s = self.count_state();
        let w = ui.available_width();
        let wide = w >= 760.0;
        let h = if wide { 196.0 } else { 392.0 };
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter_at(rect.expand(16.0));
        p.add(kit::card_shadow().as_shape(rect, 26.0));
        p.rect_filled(rect, 26.0, kit::CARD());
        p.rect_stroke(rect, 26.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), s.col, 0.35)));

        // status orb, pulsing while it's counting
        let oc = pos2(rect.min.x + 76.0, rect.min.y + 78.0);
        p.circle_filled(oc, 50.0, ch::faint(s.col, 0.07));
        p.circle_filled(oc, 34.0, ch::faint(s.col, 0.16));
        if s.live && kit::motion() {
            let t = ui.input(|i| i.time) as f32;
            let ph = (t / 1.6).fract();
            p.circle_stroke(oc, 12.0 + 26.0 * ph, Stroke::new(2.0f32, s.col.gamma_multiply(0.7 * (1.0 - ph))));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        p.circle_filled(oc, 12.0, s.col);
        let icon = if s.live { "" } else if s.title == "Paused" { "⏸" } else { "!" };
        if !icon.is_empty() {
            p.text(oc, Align2::CENTER_CENTER, icon, ch::bold(12.0), kit::CARD());
        }

        let x = rect.min.x + 150.0;
        let right_w = if wide { 300.0 } else { 0.0 };
        let room = rect.max.x - 24.0 - right_w - x;
        p.text(pos2(x, rect.min.y + 26.0), Align2::LEFT_TOP, s.title, ch::heavy(22.0), TEXT());
        p.text(pos2(x, rect.min.y + 56.0), Align2::LEFT_TOP, ch::clip(ui, &s.detail, &ch::reg(12.5), room), ch::reg(12.5), MUTED());
        let name = self.target_name(self.cfg.target.as_deref());
        let num = theme::thousands(self.cfg.count);
        let nr = p.text(pos2(x, rect.min.y + 84.0), Align2::LEFT_TOP, &num, ch::heavy(38.0), TEXT());
        let what = if self.cfg.target.is_some() { format!("encounters · {name}") } else { "encounters this run".to_string() };
        p.text(pos2(nr.max.x + 10.0, nr.max.y - 9.0), Align2::LEFT_BOTTOM, ch::clip(ui, &what, &ch::reg(13.0), room - nr.width() - 10.0), ch::reg(13.0), MUTED());

        // quick numbers along the bottom
        let (today, _) = self.day_totals();
        let last = self.last_enc_at.map(|t| ago_secs(t.elapsed().as_secs())).unwrap_or_else(|| "—".into());
        let pace = self.pace_per_hour().map_or("—".into(), |r| format!("{}/h", theme::thousands(r)));
        let facts = [("Last battle", last), ("Pace (30 min)", pace), ("Today", theme::thousands(today)), ("Battles this run", theme::thousands(self.cfg.battles))];
        let fy = rect.max.y - 44.0;
        let fw = if wide { (rect.max.x - 24.0 - right_w - x) / 4.0 } else { (rect.max.x - 24.0 - x) / 2.0 };
        for (k, (l, v)) in facts.iter().enumerate() {
            let (cx, cy) = if wide { (x + fw * k as f32, fy) } else { (x + fw * (k % 2) as f32, rect.min.y + 142.0 + 44.0 * (k / 2) as f32) };
            p.text(pos2(cx, cy), Align2::LEFT_TOP, *l, ch::reg(11.5), MUTED());
            p.text(pos2(cx, cy + 15.0), Align2::LEFT_TOP, v, ch::bold(14.5), TEXT());
        }

        // controls
        let paused = self.cfg.paused;
        let cr = if wide {
            Rect::from_min_max(pos2(rect.max.x - 24.0 - right_w + 24.0, rect.min.y + 24.0), pos2(rect.max.x - 24.0, rect.max.y - 24.0))
        } else {
            Rect::from_min_max(pos2(rect.min.x + 24.0, rect.min.y + 240.0), pos2(rect.max.x - 24.0, rect.max.y - 16.0))
        };
        if wide {
            p.line_segment([pos2(cr.min.x - 24.0, rect.min.y + 24.0), pos2(cr.min.x - 24.0, rect.max.y - 24.0)], Stroke::new(1.0f32, kit::LINE()));
        } else {
            p.line_segment([pos2(rect.min.x + 24.0, cr.min.y - 12.0), pos2(rect.max.x - 24.0, cr.min.y - 12.0)], Stroke::new(1.0f32, kit::LINE()));
        }
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(cr).layout(egui::Layout::top_down(egui::Align::Min)));
        let ui = &mut c;
        ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
        let bw = cr.width();
        let resp = if paused {
            kit::accent_button_w(ui, egui::Button::new(RichText::new("▶  Resume counting").font(ch::bold(13.5)).color(kit::on_primary())), 4.0, bw)
        } else {
            ui.add(egui::Button::new(RichText::new("⏸  Pause counting").font(ch::bold(13.5))).fill(kit::btn_fill()).min_size(vec2(bw, kit::CONTROL_H + 2.0)))
        };
        if resp.on_hover_text("Same as the pause button on the overlay").clicked() {
            self.set_paused(!paused);
        }
        ui.label(RichText::new("FIX THE COUNT").font(ch::bold(10.5)).color(MUTED()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let third = ((bw - 16.0) / 3.0).floor();
            if ui.add(egui::Button::new(RichText::new("−1").font(ch::bold(14.0))).fill(kit::btn_fill()).min_size(vec2(third, kit::CONTROL_H))).on_hover_text("Take one off").clicked() {
                self.add(-1);
            }
            if ui.add(egui::Button::new(RichText::new("+1").font(ch::bold(14.0))).fill(kit::btn_fill()).min_size(vec2(third, kit::CONTROL_H))).on_hover_text("Add one").clicked() {
                self.add(1);
            }
            let undo_n = self.last_added.unwrap_or(1).max(1);
            if ui
                .add(egui::Button::new(RichText::new("Undo").font(ch::bold(13.0))).fill(kit::btn_fill()).min_size(vec2(third, kit::CONTROL_H)))
                .on_hover_text(format!("Take back the last battle ({undo_n})"))
                .clicked()
            {
                self.undo_last();
            }
        });
        // set an exact count
        let id = egui::Id::new("count-set");
        let mut v: u64 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(self.cfg.count);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let dv = ui.add_sized(vec2(bw - 76.0, kit::CONTROL_H), egui::DragValue::new(&mut v).speed(1.0).range(0..=9_999_999).prefix("Set to "));
            if !dv.has_focus() && !dv.dragged() && v != self.cfg.count && !ui.ctx().data(|d| d.get_temp::<bool>(id.with("edited")).unwrap_or(false)) {
                v = self.cfg.count;
            }
            if dv.changed() {
                ui.ctx().data_mut(|d| d.insert_temp(id.with("edited"), true));
            }
            let differs = v != self.cfg.count;
            if ui.add_enabled(differs, egui::Button::new(RichText::new("Apply").font(ch::bold(12.5))).fill(kit::btn_fill()).min_size(vec2(68.0, kit::CONTROL_H))).clicked() {
                let delta = v as i64 - self.cfg.count as i64;
                self.add(delta);
                self.add_event("accountant");
                ui.ctx().data_mut(|d| d.insert_temp(id.with("edited"), false));
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, v));
    }

    /// Encounters per hour over the last 30 minutes of counting.
    fn pace_per_hour(&self) -> Option<u64> {
        let now = now_unix();
        let recent: Vec<&(u64, u64)> = self.recent_enc.iter().filter(|(t, _)| now.saturating_sub(*t) <= 30 * 60).collect();
        let first = recent.first()?.0;
        let span = now.saturating_sub(first).max(60) as f64;
        let n: u64 = recent.iter().map(|x| x.1).sum();
        (n >= 4).then(|| (n as f64 / span * 3600.0).round() as u64)
    }

    // ───────────────────────────── live feed ─────────────────────────────

    fn count_feed_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "📡", CYAN, "Live feed", Some("last battles counted"));
            ui.add_space(12.0);
            // pace over the last 30 minutes, one bar a minute
            let now = now_unix();
            let mut mins = [0u64; 30];
            for (t, n) in &self.recent_enc {
                let age = now.saturating_sub(*t) / 60;
                if age < 30 {
                    mins[29 - age as usize] += n;
                }
            }
            let max = mins.iter().copied().max().unwrap_or(0).max(1) as f32;
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
            let p = ui.painter();
            let bw = (r.width() - 29.0 * 3.0) / 30.0;
            for (k, v) in mins.iter().enumerate() {
                let x0 = r.min.x + k as f32 * (bw + 3.0);
                if *v == 0 {
                    p.rect_filled(Rect::from_min_max(pos2(x0, r.max.y - 2.0), pos2(x0 + bw, r.max.y)), 1.0, theme::blend(kit::CARD(), TEXT(), 0.1));
                } else {
                    let bh = (r.height() * (*v as f32 / max)).max(4.0);
                    p.rect_filled(Rect::from_min_max(pos2(x0, r.max.y - bh), pos2(x0 + bw, r.max.y)), 2.0, if k == 29 { CYAN } else { ch::faint(CYAN, 0.45) });
                }
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("30 min ago").size(10.5).color(MUTED()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("now").size(10.5).color(MUTED()));
                });
            });
            ui.add_space(8.0);
            if self.enc_feed.is_empty() {
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No battles yet this session.").size(12.5).color(MUTED()));
                    ui.label(RichText::new("They show up here the moment they're counted.").size(12.0).color(MUTED()));
                });
                ui.add_space(6.0);
                return;
            }
            let targets = self.all_target_idx();
            let feed: Vec<(u64, Vec<usize>, u32)> = self.enc_feed.iter().rev().take(6).cloned().collect();
            for (k, (t, names, added)) in feed.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                if k > 0 {
                    ui.painter().line_segment([pos2(r.min.x, r.min.y), pos2(r.max.x, r.min.y)], Stroke::new(1.0f32, kit::LINE()));
                }
                let fresh = k == 0 && now.saturating_sub(*t) < 20;
                let mut x = r.min.x + 2.0;
                for &i in names.iter().take(3) {
                    let Some(sp) = self.species.get(i).cloned() else { continue };
                    let c = pos2(x + 17.0, r.center().y);
                    let is_target = targets.contains(&i);
                    ui.painter().circle_filled(c, 17.0, if is_target { ch::faint(GOLD, 0.2) } else { theme::blend(kit::CARD(), TEXT(), 0.06) });
                    self.draw_sprite_centered(ui.painter(), &sp, false, c, vec2(32.0, 32.0));
                    x += 38.0;
                }
                let p = ui.painter();
                let tx = r.min.x + 2.0 + 38.0 * names.len().min(3) as f32 + 6.0;
                let hit = names.iter().any(|i| targets.contains(i));
                let mut job = egui::text::LayoutJob::default();
                for (n, i) in names.iter().enumerate() {
                    let Some(sp) = self.species.get(*i) else { continue };
                    if n > 0 {
                        job.append(" & ", 0.0, egui::TextFormat::simple(ch::reg(13.0), MUTED()));
                    }
                    let c = if targets.contains(i) { ch::ink(GOLD) } else { TEXT() };
                    job.append(sp.name, 0.0, egui::TextFormat::simple(ch::bold(13.0), c));
                }
                job.wrap = egui::text::TextWrapping { max_width: r.max.x - tx - 80.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                let g = ui.fonts(|f| f.layout_job(job));
                p.galley(pos2(tx, r.center().y - 8.0 - g.size().y / 2.0), g, TEXT());
                let sub = if hit { format!("+{added} counted · ★ target seen") } else { format!("+{added} counted") };
                p.text(pos2(tx, r.center().y + 9.0), Align2::LEFT_CENTER, sub, ch::reg(11.5), MUTED());
                let when = if fresh { "just now".to_string() } else { ago_secs(now.saturating_sub(*t)) };
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, when, ch::reg(11.5), if fresh { ch::ink(GREEN) } else { MUTED() });
            }
        });
    }

    // ───────────────────────────── source ─────────────────────────────

    fn count_source_card(&mut self, ui: &mut egui::Ui) {
        let mut go_folder = false;
        let mut open_logs = false;
        kit::card(ui, |ui| {
            card_head(ui, "📥", PURPLE, "Where encounters come from", None);
            ui.add_space(12.0);
            let full = ui.available_width();
            let tw = ((full - 10.0) / 2.0).floor();
            let log = self.cfg.count_source == CountSource::GameLog;
            let mut pick = None;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if source_tile(ui, tw, log, "📄", "Game log", "Reads every wild Pokémon from the game's own log. Exact, even with the overlay hidden.", ("Recommended", GREEN)) {
                    pick = Some(CountSource::GameLog);
                }
                if source_tile(ui, tw, !log, "👁", "Screen reading", "Reads the enemy names off the screen. Needs a quick setup first.", ("Needs setup", ORANGE)) {
                    pick = Some(CountSource::Screen);
                }
            });
            if let Some(src) = pick {
                if src != self.cfg.count_source {
                    self.cfg.count_source = src;
                    self.dirty = true;
                    self.push_settings(true);
                }
            }
            ui.add_space(14.0);
            match self.cfg.count_source {
                CountSource::GameLog => {
                    let dir = self.game_folder();
                    let logs = dir.join("logs");
                    let logs_ok = logs.is_dir();
                    detail_row(ui, "Log folder", &logs.display().to_string(), if logs_ok { None } else { Some("not found") });
                    let file = if self.status.text.is_empty() { "—".to_string() } else { self.status.text.clone() };
                    detail_row(ui, "Reading", &file, None);
                    if let Some((n, d)) = self.cfg.last_log_odds {
                        detail_row(ui, "Game's odds", &format!("{n} in {d}"), None);
                    }
                    ui.add_space(8.0);
                    if !logs_ok {
                        kit::pill(ui, "No game logs there. If you moved the game, choose its folder.", PillKind::Warn);
                        ui.add_space(8.0);
                    }
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("📁  Game folder settings").fill(kit::btn_fill())).clicked() {
                            go_folder = true;
                        }
                        if logs_ok && ui.add(egui::Button::new("Open log folder").fill(kit::btn_fill())).clicked() {
                            open_logs = true;
                        }
                    });
                }
                CountSource::Screen => {
                    let n = self.cfg.name_regions.len();
                    detail_row(ui, "Name boxes", &if n == 0 { "none yet".to_string() } else { format!("{n} set up") }, (n == 0).then_some("set up below"));
                    detail_row(ui, "Checks the screen", &format!("every {} ms", self.cfg.interval_ms), None);
                    ui.add_space(8.0);
                    let label = if n == 0 { "Set up name reading" } else { "Name reading setup" };
                    if ui.add(egui::Button::new(format!("🔤  {label}  ↓")).fill(kit::btn_fill())).clicked() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("count-names-open"), true);
                            d.insert_temp(egui::Id::new("count-names-scroll"), true);
                        });
                    }
                }
            }
        });
        if go_folder {
            self.open_game_folder_settings();
        }
        if open_logs {
            crate::share::open_dir(&self.game_folder().join("logs"));
        }
    }

    // ───────────────────────────── automation ─────────────────────────────

    fn count_auto_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "⚙", ORANGE, "Automation", None);
            ui.add_space(12.0);
            if kit::toggle_row(
                ui,
                "Pause when the game is closed",
                "Stops counting (and the hunt timer) while Platinum Together isn't running, and starts again when you open it.",
                &mut self.cfg.auto_pause_game,
            ) {
                self.dirty = true;
                self.push_settings(true);
            }
            ui.add_space(12.0);
            divider(ui);
            ui.add_space(12.0);
            if kit::toggle_row(
                ui,
                "Shiny star alert",
                "Flashes the overlay when the shiny ★ appears next to an enemy name. Uses the name boxes (make each box cover the name and the space after it, up to the ♂/♀ sign).",
                &mut self.cfg.shiny_star_alert,
            ) {
                self.dirty = true;
                self.push_settings(false);
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                kit::pill(ui, "⚠ Experimental: may not work correctly yet", PillKind::Warn);
                if self.cfg.shiny_star_alert && self.cfg.name_regions.is_empty() {
                    kit::pill(ui, "Needs the name boxes (below)", PillKind::Info);
                }
            });
        });
    }

    // ───────────────────────────── hotkeys ─────────────────────────────

    fn count_keys_card(&mut self, ui: &mut egui::Ui) {
        let mut go = false;
        kit::card(ui, |ui| {
            card_head(ui, "⌨", PURPLE, "Counting hotkeys", None);
            ui.add_space(10.0);
            for a in [HotAction::Plus, HotAction::Minus, HotAction::Undo, HotAction::Pause] {
                ui.horizontal(|ui| {
                    ui.set_min_height(30.0);
                    ui.label(RichText::new(a.label()).size(13.0).color(TEXT()));
                    let key = self.cfg.keybinds.get(a).map(|b| b.text());
                    // keys flush right: their width as measured last frame
                    let wid = egui::Id::new(("count-key-w", a as u8));
                    let w = ui.ctx().data(|d| d.get_temp::<f32>(wid)).unwrap_or(120.0);
                    ui.add_space((ui.available_width() - w).max(8.0));
                    let start = ui.cursor().min.x;
                    match key {
                        Some(k) => {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for part in k.split('+') {
                                kit::keycap(ui, part);
                            }
                        }
                        None => {
                            ui.label(RichText::new("Not set").size(12.5).color(MUTED()));
                        }
                    }
                    let used = ui.min_rect().max.x - start;
                    if (used - w).abs() > 0.5 {
                        ui.ctx().data_mut(|d| d.insert_temp(wid, used));
                        ui.ctx().request_repaint();
                    }
                });
            }
            ui.add_space(6.0);
            if ui.link(RichText::new("Change hotkeys ›").size(12.5)).clicked() {
                go = true;
            }
        });
        if go {
            self.go_to(Tab::Hotkeys);
        }
    }

    // ───────────────────────────── name reading ─────────────────────────────

    fn count_names_card(&mut self, ui: &mut egui::Ui) {
        let needed = self.cfg.count_source == CountSource::Screen || self.cfg.shiny_star_alert;
        let id = egui::Id::new("count-names-open");
        let default = needed && self.cfg.name_regions.is_empty();
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(default);
        let scroll = ui.ctx().data_mut(|d| d.remove_temp::<bool>(egui::Id::new("count-names-scroll"))).unwrap_or(false);
        kit::card(ui, |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if scroll {
                ui.scroll_to_rect(r.expand(20.0), Some(egui::Align::Min));
            }
            let p = ui.painter();
            let chip = Rect::from_min_size(pos2(r.min.x, r.center().y - 15.0), vec2(30.0, 30.0));
            kit::icon_chip(p, chip, "🔤", CYAN, false);
            p.text(pos2(chip.max.x + 12.0, r.center().y - 8.0), Align2::LEFT_CENTER, "Name reading setup", ch::bold(16.0), TEXT());
            let n = self.cfg.name_regions.len();
            let sub = match (needed, n) {
                (false, _) => "Only needed for screen reading or the shiny star alert".to_string(),
                (true, 0) => "Not set up yet: draw a box over each enemy name".to_string(),
                (true, n) => format!("{n} name box{} set up", if n == 1 { "" } else { "es" }),
            };
            p.text(pos2(chip.max.x + 12.0, r.center().y + 10.0), Align2::LEFT_CENTER, sub, ch::reg(12.0), if needed && n == 0 { ch::ink(ORANGE) } else { MUTED() });
            let t = kit::anim(ui.ctx(), id.with("t"), open, kit::QUICK);
            let c = pos2(r.max.x - 14.0, r.center().y);
            let a = std::f32::consts::FRAC_PI_2 * t;
            let pts = [vec2(-4.0, -7.0), vec2(3.0, 0.0), vec2(-4.0, 7.0)].map(|v| c + egui::emath::Rot2::from_angle(a) * v);
            p.add(egui::Shape::line(pts.to_vec(), Stroke::new(2.0f32, if resp.hovered() { TEXT() } else { MUTED() })));
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = !open;
            }
            if open {
                ui.add_space(12.0);
                divider(ui);
                ui.add_space(12.0);
                self.screen_wizard(ui);
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
    }
}

/// "9s ago", "3 min ago", "2h ago"
fn ago_secs(s: u64) -> String {
    if s < 60 {
        format!("{s}s ago")
    } else if s < 3600 {
        format!("{} min ago", s / 60)
    } else {
        format!("{}h ago", s / 3600)
    }
}

fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str, right: Option<&str>) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
        if let Some(t) = right {
            ui.add_space((ui.available_width() - ui.painter().layout_no_wrap(t.to_string(), ch::reg(12.0), MUTED()).size().x - 2.0).max(8.0));
            ui.label(RichText::new(t).size(12.0).color(MUTED()));
        }
    });
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

/// "Log folder    C:\…\logs" with an optional warning after it.
fn detail_row(ui: &mut egui::Ui, label: &str, value: &str, warn: Option<&str>) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let p = ui.painter();
    p.text(pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, label, ch::reg(12.5), MUTED());
    let x = r.min.x + 130.0;
    let room = r.max.x - x - if warn.is_some() { 100.0 } else { 0.0 };
    let vr = p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, ch::clip(ui, value, &ch::reg(12.5), room), ch::reg(12.5), TEXT());
    if let Some(wt) = warn {
        p.text(pos2(vr.max.x + 8.0, r.center().y), Align2::LEFT_CENTER, wt, ch::bold(11.5), ch::ink(ORANGE));
    }
}

/// A counting source to pick: icon, name, what it does, a tag.
fn source_tile(ui: &mut egui::Ui, w: f32, sel: bool, icon: &str, title: &str, body: &str, tag: (&str, Color32)) -> bool {
    let h = 138.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("sel"), sel, kit::BASE);
    let ht = kit::anim(ui.ctx(), resp.id.with("hov"), resp.hovered() && !sel, kit::QUICK);
    let p = ui.painter_at(rect.expand(4.0));
    p.rect_filled(rect, 20.0, theme::blend(theme::blend(kit::CARD(), TEXT(), 0.035 * ht), kit::primary(), 0.10 * t));
    p.rect_stroke(rect, 20.0, Stroke::new(1.0 + t, theme::blend(theme::blend(kit::LINE(), TEXT(), 0.18 * ht), kit::primary(), t)));
    let chip = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(36.0, 36.0));
    kit::icon_chip(&p, chip, icon, if sel { kit::primary() } else { MUTED() }, false);
    // title with its tag under it; a tick on the right when chosen
    p.text(pos2(chip.max.x + 12.0, chip.center().y - 8.0), Align2::LEFT_CENTER, title, ch::bold(15.0), TEXT());
    let tg = p.text(pos2(chip.max.x + 12.0 + 10.0, chip.center().y + 10.0), Align2::LEFT_CENTER, tag.0, ch::bold(11.0), ch::ink(tag.1));
    p.circle_filled(pos2(chip.max.x + 15.0, tg.center().y), 3.0, tag.1);
    let job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping { max_width: w - 32.0, max_rows: 3, break_anywhere: false, overflow_character: Some('…') },
        ..egui::text::LayoutJob::single_section(body.to_string(), egui::TextFormat::simple(ch::reg(12.0), MUTED()))
    };
    let g = ui.fonts(|f| f.layout_job(job));
    p.galley(pos2(rect.min.x + 16.0, chip.max.y + 12.0), g, MUTED());
    if t > 0.0 {
        let c = pos2(rect.max.x - 24.0, rect.min.y + 24.0);
        p.circle_filled(c, 10.0 * t, kit::primary());
        p.text(c, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(11.0 * t.max(0.01)), kit::on_primary());
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel
}
