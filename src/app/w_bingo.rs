//! Dashboard widgets for Shiny Bingo.

use super::bingo_app::{countdown, secs_to_next_card, GOLD, VIOLET};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{head, note};
use super::{App, Tab};
use crate::bingo::{self, Goal};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

impl App {
    fn w_bingo_head(&mut self, ui: &mut egui::Ui, icon: &str, title: &str) -> bool {
        head(ui, icon, VIOLET, title, Some("Open ›"))
    }

    /// The card itself, small.
    pub(super) fn w_bingo_card(&mut self, ui: &mut egui::Ui) {
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let lines = f.lines_done(&card).len();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_bingo_head(ui, "🎯", "Shiny Bingo");
            ui.add_space(6.0);
            self.bingo_grid(ui, true, None);
            ui.add_space(6.0);
            let l = if f.rewards_waiting > 0 { "daily reward ready!".to_string() } else if lines == 0 { "no lines yet".to_string() } else { format!("{lines} line{}", if lines == 1 { "" } else { "s" }) };
            note(ui, format!("{} / 25 · {l} · new card in {}", f.count(&card), countdown(secs_to_next_card())));
        });
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// Squares, lines and the countdown.
    pub(super) fn w_bingo_week(&mut self, ui: &mut egui::Ui) {
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let lines = f.lines_done(&card).len();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_bingo_head(ui, "📅", "Bingo today");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(f.count(&card).to_string()).font(crate::fonts::sans_bold_font(28.0)).color(TEXT()));
                ui.label(RichText::new("/ 25 squares").size(13.0).color(MUTED()));
            });
            ui.add_space(6.0);
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
            let seg = (w - 11.0 * 4.0) / 12.0;
            for k in 0..12 {
                let sr = Rect::from_min_size(pos2(r.min.x + k as f32 * (seg + 4.0), r.min.y), vec2(seg, 8.0));
                ui.painter().rect_filled(sr, 4.0, if k < lines { GOLD } else { Color32::from_rgb(38, 38, 54) });
            }
            ui.add_space(8.0);
            let l = if f.blackout { "Blackout!".to_string() } else { format!("{lines} of 12 lines") };
            note(ui, format!("{l} · new card in {}", countdown(secs_to_next_card())));
        });
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// The lines nearest to done, and what each still needs.
    pub(super) fn w_bingo_next(&mut self, ui: &mut egui::Ui) {
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_bingo_head(ui, "📌", "Closest to a line");
            ui.add_space(6.0);
            let close = f.closest(&card);
            if f.blackout || close.is_empty() {
                note(ui, "Every square is done today. A new card comes at the daily reset!");
                return;
            }
            for (l, left) in close.iter().take(3) {
                let what: Vec<String> = left.iter().filter_map(|s| card[*s].map(|g| g.text())).collect();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(bingo::line_name(*l)).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} to go", left.len())).size(11.5).color(MUTED()));
                    });
                });
                ui.add(egui::Label::new(RichText::new(what.join(" · ")).size(11.5).color(MUTED())).truncate());
                ui.add_space(4.0);
            }
        });
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// This week's Pokémon to meet, with a tick on the ones you have.
    pub(super) fn w_bingo_meet(&mut self, ui: &mut egui::Ui) {
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_bingo_head(ui, "🔍", "Bingo Pokémon");
            ui.add_space(6.0);
            let list: Vec<(usize, u32)> = card.iter().enumerate().filter_map(|(i, g)| if let Some(Goal::Meet(d)) = g { Some((i, *d)) } else { None }).collect();
            let w = ui.available_width();
            let cols = 3usize;
            let cw = (w - 8.0 * (cols as f32 - 1.0)) / cols as f32;
            for row in list.chunks(cols) {
                let (r, _) = ui.allocate_exact_size(vec2(w, 62.0), Sense::hover());
                for (k, (i, d)) in row.iter().enumerate() {
                    let cr = Rect::from_min_size(pos2(r.min.x + k as f32 * (cw + 8.0), r.min.y), vec2(cw, 58.0));
                    let done = f.done(&card, *i);
                    let p = ui.painter().clone();
                    p.rect_filled(cr, 10.0, if done { crate::theme::blend(kit::sunken(), VIOLET, 0.24) } else { kit::sunken() });
                    p.rect_stroke(cr, 10.0, Stroke::new(1.0f32, if done { GOLD.gamma_multiply(0.6) } else { kit::LINE() }));
                    let sp = self.species.iter().find(|s| s.dex == *d).cloned();
                    let shown = match sp {
                        Some(sp) if self.sprites.get(&sp, false).is_some() => self.draw_sprite_centered(&p, &sp, false, pos2(cr.center().x, cr.min.y + 22.0), vec2(36.0, 36.0)),
                        _ => false,
                    };
                    if !shown {
                        let c = pos2(cr.center().x, cr.min.y + 22.0);
                        p.circle_filled(c, 14.0, crate::theme::blend(kit::C_SLATE, VIOLET, 0.25));
                        let short: String = bingo::meet_name(*d).chars().take(3).collect::<String>().to_uppercase();
                        p.text(c, Align2::CENTER_CENTER, short, crate::fonts::sans_bold_font(9.5), Color32::WHITE);
                    }
                    p.text(pos2(cr.center().x, cr.max.y - 9.0), Align2::CENTER_CENTER, bingo::meet_name(*d), crate::fonts::sans_font(11.0), if done { Color32::WHITE } else { MUTED() });
                    if done {
                        super::bingo_app::stamp(&p, pos2(cr.max.x - 11.0, cr.min.y + 11.0), 6.5);
                    }
                }
            }
            note(ui, "The Map shows where each one lives.");
        });
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// The latest squares ticked.
    pub(super) fn w_bingo_recent(&mut self, ui: &mut egui::Ui) {
        let _ = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_bingo_head(ui, "✅", "Just ticked");
            ui.add_space(6.0);
            if f.recent.is_empty() {
                note(ui, "Nothing ticked on today's card yet. Go meet some Pokémon!");
                return;
            }
            let now = crate::stats::now_unix();
            for (t, what) in f.recent.iter().take(5) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✔").size(12.0).color(GOLD));
                    ui.add(egui::Label::new(RichText::new(what).size(12.5).color(TEXT())).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let s = now.saturating_sub(*t);
                        let ago = if s < 3600 { format!("{}m", (s / 60).max(1)) } else if s < 86_400 { format!("{}h", s / 3600) } else { format!("{}d", s / 86_400) };
                        ui.label(RichText::new(ago).size(11.0).color(MUTED()));
                    });
                });
            }
        });
        if go {
            self.go_to(Tab::Bingo);
        }
    }
}
