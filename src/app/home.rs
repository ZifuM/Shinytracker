//! Home in the modern look: a dashboard. The current hunt up top as a hero
//! card (with the one action that matters most: Start overlay), today's
//! numbers as tiles, the other apps, recent shinies and today's rare grass.

use super::shell::tab_meta;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::stats::fmt_duration;
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

impl App {
    pub(super) fn page_home_modern(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HOME", &self.greeting());
        self.home_notices(ui);
        self.hunt_hero(ui, true);
        ui.add_space(18.0);

        // Today, at a glance.
        let avail = ui.available_width();
        let (today, _) = self.day_totals();
        let goal = self.cfg.daily_goal;
        let streak = self.daily_streak();
        let level = self.cfg.level_seen.max(1);
        let cols = if avail >= 800.0 { 5 } else if avail >= 520.0 { 3 } else { 2 };
        let gap = 12.0;
        let tw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let goal_note = (goal > 0).then(|| {
            let f = today as f64 / goal as f64;
            if f >= 1.0 {
                ("Goal reached ✔".to_string(), theme::GREEN)
            } else {
                (format!("{:.0}% of today's goal", f * 100.0), MUTED())
            }
        });
        let tiles: [(&str, Color32, &str, String, Option<(String, Color32)>); 5] = [
            ("⚔", kit::C_VIOLET, "Encounters today", theme::thousands(today), None),
            ("⏱", kit::C_CYAN, "Hunted today", fmt_duration(self.today_hunt_secs()), None),
            (
                "🎯",
                kit::C_PINK,
                "Daily goal",
                if goal > 0 { format!("{} / {}", theme::thousands(today.min(goal)), theme::thousands(goal)) } else { "Off".into() },
                goal_note,
            ),
            ("🔥", kit::C_ORANGE, "Streak", format!("{streak} day{}", if streak == 1 { "" } else { "s" }), None),
            ("⭐", kit::C_GOLD, "Trainer level", level.to_string(), Some((crate::progress::title(level).to_string(), MUTED()))),
        ];
        let any_note = tiles.iter().any(|t| t.4.is_some());
        let mut open: Option<usize> = None;
        for row in tiles.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for t in row {
                    let i = tiles.iter().position(|x| x.2 == t.2).unwrap_or(0);
                    let note = t.4.as_ref().map(|(s, c)| (s.as_str(), *c));
                    let note = if any_note { note.or(Some(("", MUTED()))) } else { None };
                    let tip = match i {
                        0 | 1 => "Open this hunt's stats",
                        2 => "Open your goals",
                        _ => "Open your Trainer Profile",
                    };
                    if kit::stat_tile(ui, tw, t.0, t.1, t.2, &t.3, note).on_hover_text(tip).clicked() {
                        open = Some(i);
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(i) = open {
            self.setup.scroll_top = true;
            match i {
                0 | 1 => {
                    self.go_to(Tab::Runs);
                    self.setup.view_run = Some(super::RunRef::Current);
                    self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
                }
                2 => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Goals;
                }
                _ => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Overview;
                }
            }
        }

        // Apps and recent shinies side by side when there's room.
        let wide = avail >= 860.0;
        let left_w = if wide { (avail * 0.58).floor() } else { avail };
        let right_w = if wide { avail - left_w - 16.0 } else { avail };
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(left_w);
                kit::section(ui, "Apps");
                self.app_tiles(ui, left_w);
            });
            if wide {
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    kit::section(ui, "Recent shinies");
                    self.recent_shinies(ui);
                });
            }
        });
        if !wide {
            kit::section(ui, "Recent shinies");
            self.recent_shinies(ui);
        }

        kit::section(ui, "Rare grass today");
        self.rare_home(ui);
    }

    /// "Good evening, Zifu."
    fn greeting(&self) -> String {
        let hour = crate::stats::local_hour(crate::config::now_unix());
        let part = match hour {
            5..=11 => "Good morning",
            12..=17 => "Good afternoon",
            _ => "Good evening",
        };
        let name = self.cfg.trainer_name.trim();
        if name.is_empty() {
            format!("{part}! Here's your hunt and today at a glance.")
        } else {
            format!("{part}, {name}! Here's your hunt and today at a glance.")
        }
    }

    /// The current hunt as a big card: target, count, odds, Start overlay.
    /// `home`: with the "Open Shiny Hunter" button. The card grows to fit
    /// its contents (fonts and window sizes differ), with a minimum height.
    pub(super) fn hunt_hero(&mut self, ui: &mut egui::Ui, home: bool) {
        let w = ui.available_width();
        let pl = theme::pal();
        let show_sprite = w >= 560.0;
        let art_w = if show_sprite { (w * 0.3).clamp(220.0, 320.0) } else { 0.0 };
        let text_w = (w - 56.0 - art_w).max(200.0);
        let (n, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        let method = self.cfg.odds_caption();
        let mut open_hunter = false;
        let mut change = false;
        let mut stats = false;

        // Background goes underneath once the card's size is known.
        let slot = ui.painter().add(egui::Shape::Noop);
        let frame = egui::Frame::none()
            .inner_margin(egui::Margin { left: 28.0, right: 28.0, top: 22.0, bottom: 24.0 })
            .show(ui, |ui| {
                ui.set_width(w - 56.0);
                ui.set_min_height(186.0);
                ui.allocate_ui_with_layout(egui::vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(text_w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new("CURRENT HUNT").font(crate::fonts::sans_bold_font(11.5)).color(theme::blend(TEXT(), pl.accent, 0.35)));
                    ui.add(egui::Label::new(RichText::new(self.target_label()).font(crate::fonts::sans_bold_font(28.0)).color(TEXT())).wrap_mode(egui::TextWrapMode::Truncate));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        chip(ui, &method);
                        if let Some(o) = odds {
                            chip(ui, &format!("1 in {}", super::fmt_one_in(o)));
                        }
                        if self.overlay_on {
                            chip_col(ui, "● Counting", theme::GREEN);
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(RichText::new(theme::thousands(self.cfg.count)).font(crate::fonts::sans_bold_font(34.0)).color(TEXT()));
                        ui.label(RichText::new("encounters").size(13.5).color(MUTED()));
                    });
                    if let Some(o) = odds {
                        let mult = n as f64 / o;
                        let (label, col) = if mult < 0.9 {
                            (format!("{:.2}× the odds", mult), theme::GREEN)
                        } else if mult <= 1.1 {
                            (format!("{:.1}× the odds", mult), theme::HP_YELLOW)
                        } else {
                            (format!("{:.1}× the odds", mult), theme::HP_RED)
                        };
                        ui.horizontal(|ui| {
                            let bw = (text_w - 130.0).clamp(80.0, 300.0);
                            kit::bar(ui, bw, 6.0, (mult as f32).min(1.0));
                            ui.label(RichText::new(label).size(12.5).color(col));
                        });
                    }
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                        self.overlay_button(ui, egui::vec2(150.0, 36.0));
                        change = sec_button(ui, "🎯  Change target", "Pick a different Pokémon to hunt");
                        if home {
                            open_hunter = sec_button(ui, "Open Shiny Hunter  ›", "Hunt settings, counting, runs and the overlay");
                        } else {
                            stats = sec_button(ui, "📊  Hunt stats", "This hunt's encounters, time and pace");
                        }
                    });
                });
            });
        let rect = frame.response.rect;
        let h = rect.height();

        // Card: shadow, gradient, edge.
        let a = theme::blend(CARD(), pl.accent, if pl.light { 0.18 } else { 0.34 });
        let b = theme::blend(CARD(), if pl.gradient { pl.accent2 } else { pl.accent }, if pl.light { 0.10 } else { 0.16 });
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], a, b),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), pl.accent, 0.35))),
            ]),
        );

        // The target, big, on a soft glow at the right.
        if show_sprite {
            let pc = ui.painter_at(rect.shrink(1.0));
            let sprite_c = egui::pos2(rect.max.x - art_w / 2.0 - 12.0, rect.center().y + 4.0);
            let s = h.min(art_w);
            for (rad, alpha) in [(s * 0.6, 6u8), (s * 0.45, 8), (s * 0.31, 12)] {
                pc.circle_filled(sprite_c, rad, Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha));
            }
            match self.target_idx() {
                Some(i) => {
                    let sp = self.species[i].clone();
                    self.draw_sprite(&pc, &sp, true, sprite_c + egui::vec2(0.0, s * 0.32), egui::vec2(s * 0.78, s * 0.72));
                }
                None => {
                    pc.text(sprite_c, Align2::CENTER_CENTER, "?", kit::font_bold(56.0), MUTED());
                }
            }
            for (dx, dy, r) in [(-0.34, -0.30, 7.0), (0.30, -0.22, 5.0), (0.24, 0.30, 4.0)] {
                theme::sparkle(&pc, sprite_c + egui::vec2(s * dx, s * dy), r, GOLD);
            }
        }

        if change {
            self.open_target_picker();
        }
        if open_hunter {
            self.go_to(Tab::Shiny);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
        if stats {
            self.go_to(Tab::Runs);
            self.setup.view_run = Some(super::RunRef::Current);
        }
    }

    /// The other apps as tiles.
    fn app_tiles(&mut self, ui: &mut egui::Ui, width: f32) {
        self.page_tiles(
            ui,
            width,
            &[
                (Tab::Pokedex, "Every Pokémon: evolutions, moves and where to find them."),
                (Tab::Map, "Sinnoh from your game's map: areas, Pokémon and rare grass."),
            ],
            2,
        );
    }

    /// Pages as tiles (icon chip in the page's colour, name, what's there),
    /// up to `max_cols` across.
    pub(super) fn page_tiles(&mut self, ui: &mut egui::Ui, width: f32, apps: &[(Tab, &str)], max_cols: usize) {
        let gap = 12.0;
        let cols = ((width + gap) / (210.0 + gap)).floor().clamp(1.0, max_cols as f32) as usize;
        let tw = ((width - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut go: Option<Tab> = None;
        for row in apps.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (tab, about) in row {
                    let (icon, name, color) = tab_meta(*tab);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(tw, 104.0), Sense::click());
                    let p = ui.painter_at(rect.expand(12.0));
                    let hov = resp.hovered();
                    p.add(kit::card_shadow().as_shape(rect, 14.0));
                    p.rect_filled(rect, 14.0, if hov { kit::CARD_HOVER() } else { CARD() });
                    p.rect_stroke(rect, 14.0, Stroke::new(1.0f32, if hov { theme::blend(LINE(), color, 0.6) } else { LINE() }));
                    let chip = Rect::from_min_size(rect.min + egui::vec2(16.0, 16.0), egui::vec2(36.0, 36.0));
                    kit::icon_chip(&p, chip, icon, color, true);
                    p.text(egui::pos2(chip.max.x + 12.0, chip.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(15.0), TEXT());
                    p.text(rect.right_top() + egui::vec2(-16.0, 34.0), Align2::RIGHT_CENTER, "›", egui::FontId::proportional(18.0), if hov { TEXT() } else { MUTED() });
                    let g = p.layout(about.to_string(), crate::fonts::sans_font(12.5), MUTED(), rect.width() - 32.0);
                    p.galley(egui::pos2(rect.min.x + 16.0, chip.max.y + 10.0), g, MUTED());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        go = Some(*tab);
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(t) = go {
            self.go_to(t);
        }
    }

    /// The last few shinies caught, newest first.
    fn recent_shinies(&mut self, ui: &mut egui::Ui) {
        let recent: Vec<(Option<usize>, String, u64, u64)> = self
            .cfg
            .history
            .iter()
            .rev()
            .take(6)
            .map(|h| {
                let idx = h.slug.as_deref().and_then(|s| self.species.iter().position(|sp| sp.slug == s));
                (idx, h.target.clone(), h.encounters, h.when_unix)
            })
            .collect();
        kit::card(ui, |ui| {
            if recent.is_empty() {
                ui.label(RichText::new("No shinies yet. Your first one shows up here, with how long it took.").size(12.5).color(MUTED()));
                return;
            }
            let w = ui.available_width();
            let cols = ((w + 8.0) / (96.0 + 8.0)).floor().clamp(1.0, 6.0) as usize;
            let cw = ((w - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in recent.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (idx, name, enc, when) in row {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 118.0), Sense::hover());
                        let p = ui.painter_at(r);
                        p.rect_filled(r, 12.0, theme::blend(CARD(), GOLD, 0.06));
                        if let Some(i) = idx {
                            let sp = self.species[*i].clone();
                            self.draw_sprite(&p, &sp, true, egui::pos2(r.center().x, r.min.y + 70.0), egui::vec2(cw - 16.0, 62.0));
                        }
                        theme::sparkle(&p, r.right_top() + egui::vec2(-12.0, 12.0), 4.5, GOLD);
                        p.text(egui::pos2(r.center().x, r.max.y - 34.0), Align2::CENTER_CENTER, name, crate::fonts::sans_bold_font(12.5), TEXT());
                        p.text(
                            egui::pos2(r.center().x, r.max.y - 16.0),
                            Align2::CENTER_CENTER,
                            format!("{} enc.", theme::thousands(*enc)),
                            crate::fonts::sans_font(11.5),
                            MUTED(),
                        );
                        resp.on_hover_text(format!("{name}: {} encounters · {}", theme::thousands(*enc), ago(*when)));
                    }
                });
                ui.add_space(8.0);
            }
        });
    }
}

fn chip(ui: &mut egui::Ui, text: &str) {
    chip_col(ui, text, TEXT());
}

/// A Cirrus chip: Cloud pill with a hairline.
fn chip_col(ui: &mut egui::Ui, text: &str, col: Color32) {
    egui::Frame::none()
        .fill(kit::CARD())
        .stroke(Stroke::new(1.0f32, kit::LINE()))
        .rounding(14.0)
        .inner_margin(egui::Margin::symmetric(11.0, 4.0))
        .show(ui, |ui| ui.label(RichText::new(text).size(12.0).color(col)));
}

/// A Cirrus secondary button: Cloud pill with a hairline; Sunken on hover.
fn sec_button(ui: &mut egui::Ui, text: &str, tip: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(TEXT()))
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, kit::LINE()))
            .rounding(18.0)
            .min_size(egui::vec2(0.0, 36.0)),
    )
    .on_hover_text(tip)
    .clicked()
}

/// "5m ago", "3h ago", "2d ago" for a unix time.
fn ago(unix: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let s = now.saturating_sub(unix);
    if s < 60 {
        "just now".into()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86_400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86_400)
    }
}
