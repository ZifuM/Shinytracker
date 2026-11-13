//! Dashboard widgets for the Shiny Hunter, and the Dashboard's own (clock,
//! notes).

use super::home::ago;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_kit::{self as wk, head, n, note};
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::stats::{fmt_duration, found_by};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Rect, RichText, Sense, Stroke};

impl App {
    /// The time, the date, how long the app's been open and today's hunting.
    pub(super) fn w_clock(&mut self, ui: &mut egui::Ui) {
        let now = crate::config::now_unix();
        let local = now as i64 + crate::stats::utc_offset_secs();
        let secs = local.rem_euclid(86_400);
        let (h, m) = (secs / 3600, (secs / 60) % 60);
        let (h12, ampm) = (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "am" } else { "pm" });
        let day = crate::stats::local_day(now);
        let open = self.started.elapsed().as_secs();
        let hunted = self.today_hunt_secs();
        kit::card(ui, |ui| {
            head(ui, "🕘", kit::C_CYAN, "Clock", None);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{h12}:{m:02}")).font(crate::fonts::sans_bold_font(34.0)).color(TEXT()));
                ui.label(RichText::new(ampm).size(14.0).color(MUTED()));
            });
            ui.label(RichText::new(format!("{}, {}", crate::stats::WEEKDAYS[crate::stats::weekday(day)], crate::stats::fmt_day(day))).size(kit::fs(12.5)).color(MUTED()));
            ui.add_space(10.0);
            for (label, v) in [("App open", fmt_duration(open)), ("Hunted today", fmt_duration(hunted))] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).size(kit::fs(12.5)).color(MUTED()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(v).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                    });
                });
            }
        });
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    }

    /// A notepad that keeps what you write.
    pub(super) fn w_notes(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let mut text = if preview { "Hunt Gible on Route 206 after the gym.\nAsk about the next event.".to_string() } else { w.data.first().cloned().unwrap_or_default() };
        kit::card(ui, |ui| {
            head(ui, "📝", kit::C_GOLD, "Notes", None);
            ui.add_space(6.0);
            let edit = egui::TextEdit::multiline(&mut text)
                .id(egui::Id::new(("w-notes", w.id)))
                .hint_text("Write anything here…")
                .desired_width(ui.available_width())
                .desired_rows(5)
                .font(egui::FontId::proportional(kit::fs(13.0)));
            if ui.add(edit).changed() && !preview {
                *out = Some(vec![text.clone()]);
            }
        });
    }

    /// The hunt you're on: count, odds, time, and Start overlay.
    pub(super) fn w_hunt_ctrl(&mut self, ui: &mut egui::Ui) {
        let (count, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        let method = self.cfg.odds_caption();
        let name = self.target_label();
        let secs = self.cfg.run_stats.hunt_secs;
        let idx = self.shown_target_idx(ui.ctx());
        let mut open = false;
        let mut change = false;
        kit::card(ui, |ui| {
            open = head(ui, "🎯", kit::C_VIOLET, "Current hunt", Some("Open ›"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let (sr, _) = ui.allocate_exact_size(vec2(84.0, 84.0), Sense::hover());
                let p = ui.painter().clone();
                p.circle_filled(sr.center(), 40.0, theme::blend(CARD(), kit::C_VIOLET, 0.14));
                match idx {
                    Some(i) => {
                        let sp = self.species[i].clone();
                        self.draw_sprite_centered(&p, &sp, true, sr.center(), vec2(76.0, 76.0));
                    }
                    None => {
                        p.text(sr.center(), Align2::CENTER_CENTER, "?", kit::font_bold(30.0), MUTED());
                    }
                }
                theme::sparkle(&p, sr.right_top() + vec2(-10.0, 10.0), 5.0, GOLD);
                ui.vertical(|ui| {
                    ui.add(egui::Label::new(RichText::new(&name).font(crate::fonts::sans_bold_font(17.0)).color(TEXT())).truncate());
                    ui.label(RichText::new(&method).size(kit::fs(12.0)).color(MUTED()));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(n(self.cfg.count)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                        ui.label(RichText::new("encounters").size(kit::fs(12.5)).color(MUTED()));
                    });
                });
            });
            ui.add_space(6.0);
            if let Some(o) = odds {
                let mult = count as f64 / o;
                let col = super::charts::luck_col(found_by(count, o));
                ui.horizontal(|ui| {
                    kit::bar(ui, (ui.available_width() - 150.0).max(60.0), 6.0, (mult as f32).min(1.0));
                    ui.label(RichText::new(format!("{mult:.2}× the odds (1 in {})", super::fmt_one_in(o))).size(kit::fs(12.0)).color(super::charts::ink(col)));
                });
            }
            ui.label(RichText::new(format!("{} on this hunt · {} today", fmt_duration(secs), fmt_duration(self.today_hunt_secs()))).size(kit::fs(12.0)).color(MUTED()));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                self.overlay_button(ui, vec2(150.0, 34.0));
                change = wk::go_button(ui, "🎯  Change target", false);
            });
        });
        if change {
            self.open_target_picker();
        }
        if open {
            self.go_to(Tab::Shiny);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
    }

    /// Your latest shinies, big, taking turns.
    pub(super) fn w_showcase(&mut self, ui: &mut egui::Ui) {
        let recent: Vec<(Option<String>, String, u64, u64)> = self.cfg.history.iter().rev().take(5).map(|h| (h.slug.clone(), h.target.clone(), h.encounters, h.when_unix)).collect();
        let mut trophy: Option<Option<String>> = None;
        kit::card(ui, |ui| {
            if head(ui, "🌟", GOLD, "Shiny showcase", Some("Shiny Dex ›")) {
                trophy = Some(None);
            }
            ui.add_space(6.0);
            if recent.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
                let p = ui.painter();
                p.circle_stroke(r.center() - vec2(0.0, 18.0), 34.0, Stroke::new(1.5f32, theme::blend(LINE(), GOLD, 0.3)));
                p.text(r.center() - vec2(0.0, 18.0), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(26.0), MUTED());
                p.text(r.center() + vec2(0.0, 38.0), Align2::CENTER_CENTER, "Your first shiny shows here", crate::fonts::sans_font(12.5), MUTED());
                return;
            }
            // a new one every 5 seconds
            let t = ui.input(|i| i.time);
            let k = (t / 5.0) as usize % recent.len();
            let fade = ((t % 5.0) / 0.4).min(1.0) as f32;
            let (slug, name, enc, when) = recent[k].clone();
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::click());
            let p = ui.painter().clone();
            p.rect_filled(r, 14.0, theme::blend(CARD(), GOLD, 0.08));
            let c = pos2(r.center().x, r.min.y + 58.0);
            for (rad, a) in [(54.0, 0.06), (40.0, 0.09)] {
                p.circle_filled(c, rad, GOLD.gamma_multiply(a * fade));
            }
            if let Some((_, sp)) = self.species_of(slug.as_deref()) {
                self.draw_sprite_centered(&p.with_clip_rect(r), &sp, true, c, vec2(96.0 * (0.9 + 0.1 * fade), 96.0 * (0.9 + 0.1 * fade)));
            }
            for (dx, dy, s) in [(-46.0, -30.0, 6.0), (44.0, -18.0, 4.5), (36.0, 30.0, 3.5)] {
                theme::sparkle(&p, c + vec2(dx, dy), s * fade, GOLD);
            }
            p.text(pos2(r.center().x, r.max.y - 34.0), Align2::CENTER_CENTER, &name, crate::fonts::sans_bold_font(15.0), TEXT());
            p.text(pos2(r.center().x, r.max.y - 15.0), Align2::CENTER_CENTER, format!("{} encounters · {}", n(enc), ago(when)), crate::fonts::sans_font(11.5), MUTED());
            // which one this is
            let dots = recent.len() as f32;
            for i in 0..recent.len() {
                let x = r.center().x + (i as f32 - (dots - 1.0) / 2.0) * 12.0;
                let dc = pos2(x, r.min.y + 10.0);
                if i == k {
                    p.circle_filled(dc, 3.5, GOLD);
                } else {
                    p.circle_filled(dc, 2.5, MUTED().gamma_multiply(0.5));
                }
            }
            if resp.on_hover_text("Open its trophy").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                trophy = Some(slug);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(if fade < 1.0 { 30 } else { 500 }));
        });
        if let Some(t) = trophy {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.trophy_back = t.as_ref().map(|_| Tab::Home);
            self.setup.dex.open = t;
            self.setup.scroll_top = true;
        }
    }

    /// Encounters each day this week, and your best.
    pub(super) fn w_week(&mut self, ui: &mut egui::Ui) {
        let days = self.home_days();
        let today = crate::stats::local_day(crate::config::now_unix());
        let week: Vec<(i64, u64)> = (0..7).map(|k| (today - 6 + k, days.get(&(today - 6 + k)).copied().unwrap_or(0))).collect();
        let total: u64 = week.iter().map(|d| d.1).sum();
        let best_ever = days.iter().max_by_key(|(_, v)| **v).map(|(d, v)| (*d, *v));
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "📊", kit::C_BLUE, "This week", Some("Stats ›"));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(n(total)).font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
                ui.label(RichText::new("encounters in 7 days").size(kit::fs(12.5)).color(MUTED()));
            });
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
            let p = ui.painter();
            let max = week.iter().map(|d| d.1).max().unwrap_or(0).max(1) as f32;
            let slot = r.width() / 7.0;
            let bw = (slot * 0.55).min(22.0);
            let best_k = week.iter().enumerate().max_by_key(|(_, d)| d.1).map(|(k, _)| k).filter(|k| week[*k].1 > 0);
            for (k, (d, v)) in week.iter().enumerate() {
                let cx = r.min.x + slot * (k as f32 + 0.5);
                let h = ((r.height() - 34.0) * (*v as f32 / max)).max(3.0);
                let bottom = r.max.y - 16.0;
                let col = if Some(k) == best_k { kit::C_BLUE } else { theme::blend(CARD(), kit::C_BLUE, if *v > 0 { 0.5 } else { 0.15 }) };
                let g = kit::grow(ui.ctx(), egui::Id::new(("w-week", k)), 1.0);
                p.rect_filled(Rect::from_min_max(pos2(cx - bw / 2.0, bottom - h * g), pos2(cx + bw / 2.0, bottom)), bw / 3.0, col);
                if *v > 0 && Some(k) == best_k {
                    p.text(pos2(cx, bottom - h - 9.0), Align2::CENTER_CENTER, n(*v), crate::fonts::sans_bold_font(10.5), TEXT());
                }
                let wd = crate::stats::WEEKDAYS[crate::stats::weekday(*d)];
                p.text(pos2(cx, r.max.y - 6.0), Align2::CENTER_CENTER, &wd[..1], crate::fonts::sans_font(10.5), if k == 6 { TEXT() } else { MUTED() });
            }
            if let Some((d, v)) = best_ever.filter(|b| b.1 > 0) {
                ui.add_space(4.0);
                note(ui, format!("Best day ever: {} on {}", n(v), crate::stats::fmt_day_short(d)));
            }
        });
        if go {
            self.go_to(Tab::Stats);
        }
    }

    /// How full your Shiny Dex is, and the next few to get.
    pub(super) fn w_shinydex(&mut self, ui: &mut egui::Ui) {
        let caught = self.dex_caught();
        let entries: Vec<crate::moddex::Entry> = crate::moddex::entries().into_iter().filter(|e| !e.unobtainable).collect();
        let total = entries.len();
        let have = entries.iter().filter(|e| self.species.get(e.dex as usize - 1).is_some_and(|s| caught.contains_key(&s.slug))).count();
        let next: Vec<usize> = entries.iter().map(|e| e.dex as usize - 1).filter(|i| self.species.get(*i).is_some_and(|s| !caught.contains_key(&s.slug))).take(4).collect();
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "💎", kit::C_PINK, "Shiny Dex", Some("Open ›"));
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 92.0), Sense::hover());
            let p = ui.painter().clone();
            let c = pos2(r.min.x + 46.0, r.center().y);
            let f = if total > 0 { have as f32 / total as f32 } else { 0.0 };
            p.circle_stroke(c, 38.0, Stroke::new(9.0f32, theme::blend(CARD(), TEXT(), 0.08)));
            let g = kit::grow(ui.ctx(), egui::Id::new("w-dexring"), f);
            if g > 0.0 {
                let k = (g * 64.0).ceil().max(2.0) as usize;
                let pts: Vec<egui::Pos2> = (0..=k).map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * g * i as f32 / k as f32;
                    c + vec2(a.cos(), a.sin()) * 38.0
                }).collect();
                p.add(egui::Shape::line(pts, Stroke::new(9.0f32, kit::C_PINK)));
            }
            p.text(c, Align2::CENTER_CENTER, format!("{:.0}%", f * 100.0), crate::fonts::sans_bold_font(15.0), TEXT());
            p.text(pos2(c.x + 56.0, c.y - 10.0), Align2::LEFT_CENTER, format!("{} / {}", n(have as u64), n(total as u64)), crate::fonts::sans_bold_font(18.0), TEXT());
            p.text(pos2(c.x + 56.0, c.y + 12.0), Align2::LEFT_CENTER, "shinies caught", crate::fonts::sans_font(12.0), MUTED());
            if !next.is_empty() {
                ui.add_space(4.0);
                note(ui, "Still to get:");
                ui.horizontal(|ui| {
                    for i in &next {
                        let (sr, resp) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                        let p = ui.painter().clone();
                        p.rect_filled(sr, 10.0, theme::blend(CARD(), kit::C_PINK, 0.08));
                        let sp = self.species[*i].clone();
                        self.draw_sprite_centered(&p, &sp, true, sr.center(), vec2(44.0, 44.0));
                        resp.on_hover_text(sp.name);
                    }
                });
            }
        });
        if go {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.dex.open = None;
            self.setup.scroll_top = true;
        }
    }

    /// How your hunt's luck compares: the share of hunters who'd have found
    /// it by now.
    pub(super) fn w_luck(&mut self, ui: &mut egui::Ui) {
        let (count, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        kit::card(ui, |ui| {
            head(ui, "🍀", kit::C_GREEN, "Luck meter", None);
            ui.add_space(6.0);
            let Some(o) = odds.filter(|o| *o > 1.0) else {
                note(ui, "Pick a hunting method with known odds and your luck shows here.");
                return;
            };
            let f = found_by(count, o);
            let col = super::charts::luck_col(f);
            // a half-dial with a needle
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
            let p = ui.painter().clone();
            let c = pos2(r.center().x, r.max.y - 8.0);
            let rad = (r.height() - 14.0).min(r.width() / 2.0 - 10.0);
            let steps = 40;
            for i in 0..steps {
                let a0 = std::f32::consts::PI * (1.0 - i as f32 / steps as f32);
                let a1 = std::f32::consts::PI * (1.0 - (i + 1) as f32 / steps as f32);
                let seg = super::charts::luck_col(i as f64 / steps as f64);
                p.line_segment([c + vec2(a0.cos(), -a0.sin()) * rad, c + vec2(a1.cos(), -a1.sin()) * rad], Stroke::new(10.0f32, seg.gamma_multiply(0.75)));
            }
            let g = kit::grow(ui.ctx(), egui::Id::new("w-luck"), f as f32);
            let a = std::f32::consts::PI * (1.0 - g);
            p.line_segment([c, c + vec2(a.cos(), -a.sin()) * (rad - 14.0)], Stroke::new(3.0f32, TEXT()));
            p.circle_filled(c, 5.0, TEXT());
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("{:.0}%", f * 100.0)).font(crate::fonts::sans_bold_font(20.0)).color(super::charts::ink(col)));
                let line = if count == 0 {
                    "Start hunting and the needle moves.".to_string()
                } else if f < 0.5 {
                    format!("of hunters would have found it by now. Right on track at {}.", n(count))
                } else {
                    format!("of hunters would have found it by now. Unlucky so far; it can come any time.")
                };
                ui.label(RichText::new(line).size(kit::fs(12.0)).color(MUTED()));
            });
        });
    }
}
