//! Dashboard widgets for Pokégotchi.

use super::gotchi_app::{act_icon, meter, mood_col, need_col, need_tint, paint_device, Act};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{head, note};
use super::widgets::empty_card;
use super::{App, Tab};
use crate::gotchi::{self, Need};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, RichText, Sense, Stroke};
use std::time::Instant;

const PINK: egui::Color32 = egui::Color32::from_rgb(255, 156, 199);

impl App {
    /// No pet yet: a placeholder card. True when there is one.
    fn w_gotchi_has_pet(&mut self, ui: &mut egui::Ui, title: &str) -> bool {
        if self.setup.gotchi.file().partner().is_some() {
            return true;
        }
        empty_card(ui, "🐾", title, "Adopt a shiny you caught in Pokégotchi and it shows here.");
        false
    }

    pub(super) fn w_gotchi_pet(&mut self, ui: &mut egui::Ui) {
        if !self.w_gotchi_has_pet(ui, "Your Pokégotchi") {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return };
        let mood = pet.mood(f.asleep);
        let name = self.gotchi_name(&pet);
        let t = ui.input(|i| i.time) as f32;
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🐾", PINK, "Your Pokégotchi", Some("Open ›"));
            ui.add_space(8.0);
            let w = ui.available_width();
            let (r, resp) = ui.allocate_exact_size(vec2(w, 150.0), Sense::click());
            let p = ui.painter_at(r.expand(4.0));
            let dev = Rect::from_min_size(r.min, vec2(122.0, 150.0));
            let parts = paint_device(&p, dev, f.frame, t, None, None, false);
            self.gotchi_screen(&p, parts.screen, f.frame, t, true);
            let x = dev.max.x + 14.0;
            p.text(pos2(x, r.min.y + 30.0), Align2::LEFT_CENTER, &name, crate::fonts::sans_bold_font(16.0), TEXT());
            p.text(pos2(x, r.min.y + 52.0), Align2::LEFT_CENTER, format!("Friendship Lv {}", pet.level()), crate::fonts::sans_font(12.0), kit::C_GOLD);
            p.circle_filled(pos2(x + 5.0, r.min.y + 82.0), 4.5, mood_col(mood));
            p.text(pos2(x + 16.0, r.min.y + 82.0), Align2::LEFT_CENTER, mood.name(), crate::fonts::sans_bold_font(13.0), TEXT());
            let g = p.layout(mood.hint().to_string(), crate::fonts::sans_font(11.5), MUTED(), (r.max.x - x).max(60.0));
            p.galley(pos2(x, r.min.y + 96.0), g, MUTED());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go = true;
            }
        });
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    pub(super) fn w_gotchi_needs(&mut self, ui: &mut egui::Ui) {
        if !self.w_gotchi_has_pet(ui, "Pet needs") {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return };
        let name = self.gotchi_name(&pet);
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "💗", PINK, "Pet needs", Some("Open ›"));
            ui.add_space(4.0);
            note(ui, format!("{name} · {}", pet.mood(f.asleep).name()));
            ui.add_space(8.0);
            for n in Need::ALL {
                let w = ui.available_width();
                let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                let p = ui.painter();
                let v = pet.need(n);
                p.text(pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, n.name(), crate::fonts::sans_bold_font(12.0), TEXT());
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, format!("{v:.0}"), crate::fonts::sans_font(12.0), MUTED());
                meter(p, Rect::from_min_max(pos2(r.min.x + 60.0, r.center().y - 4.0), pos2(r.max.x - 32.0, r.center().y + 4.0)), v / 100.0, need_col(n), kit::sunken());
            }
        });
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    pub(super) fn w_gotchi_care(&mut self, ui: &mut egui::Ui) {
        if !self.w_gotchi_has_pet(ui, "Quick care") {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let said = self.setup.gotchi.said.as_ref().filter(|s| s.1.elapsed().as_secs_f32() < 2.6).map(|s| s.0.clone());
        let mut act = None;
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🍓", PINK, "Quick care", Some("Open ›"));
            ui.add_space(8.0);
            let w = ui.available_width();
            let acts = [Act::Feed, Act::Treat, Act::Clean, Act::Pat];
            let bw = (w - 8.0 * 3.0) / 4.0;
            let (row, _) = ui.allocate_exact_size(vec2(w, 58.0), Sense::hover());
            for (i, a) in acts.iter().enumerate() {
                let br = Rect::from_min_size(pos2(row.min.x + i as f32 * (bw + 8.0), row.min.y), vec2(bw, 58.0));
                let resp = ui.interact(br, ui.id().with(("w-gotchi-care", i)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter();
                p.rect_filled(br, 10.0, theme::blend(kit::sunken(), need_tint(*a), 0.08 + 0.14 * ht));
                p.rect_stroke(br, 10.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), need_tint(*a), 0.3 * ht)));
                act_icon(p, pos2(br.center().x, br.min.y + 20.0), 16.0, *a, need_tint(*a));
                let label = match a {
                    Act::Feed => format!("Feed {}", f.berries),
                    Act::Treat => format!("Treat {}", f.treats),
                    _ => a.name().to_string(),
                };
                p.text(pos2(br.center().x, br.max.y - 12.0), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(11.5), TEXT());
                let tip = match a {
                    Act::Feed => "Feed a berry",
                    Act::Treat => "Give a treat",
                    Act::Clean => "Clean up",
                    _ => "Pat",
                };
                if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    act = Some(*a);
                }
            }
            ui.add_space(8.0);
            note(ui, said.unwrap_or_else(|| format!("{} berries · {} treats", f.berries, f.treats)));
        });
        if let Some(a) = act {
            self.gotchi_act(a);
        } else if go {
            self.go_to(Tab::Gotchi);
        }
    }

    pub(super) fn w_gotchi_pantry(&mut self, ui: &mut egui::Ui) {
        if !self.w_gotchi_has_pet(ui, "Pantry") {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let today = crate::stats::local_day(crate::config::now_unix());
        let td = if f.today.0 == today { f.today } else { (0, 0, 0, 0) };
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🎒", kit::C_GREEN, "Pantry", Some("Open ›"));
            ui.add_space(8.0);
            let w = ui.available_width();
            let (row, _) = ui.allocate_exact_size(vec2(w, 46.0), Sense::hover());
            let half = (w - 10.0) / 2.0;
            for (i, (a, n, what)) in [(Act::Feed, f.berries, "berries"), (Act::Treat, f.treats, "treats")].iter().enumerate() {
                let tr = Rect::from_min_size(pos2(row.min.x + i as f32 * (half + 10.0), row.min.y), vec2(half, 46.0));
                let p = ui.painter();
                p.rect_filled(tr, 10.0, kit::sunken());
                act_icon(p, pos2(tr.min.x + 20.0, tr.center().y), 16.0, *a, need_tint(*a));
                p.text(pos2(tr.min.x + 38.0, tr.center().y - 7.0), Align2::LEFT_CENTER, n.to_string(), crate::fonts::sans_bold_font(15.0), TEXT());
                p.text(pos2(tr.min.x + 38.0, tr.center().y + 10.0), Align2::LEFT_CENTER, *what, crate::fonts::sans_font(11.0), MUTED());
            }
            ui.add_space(8.0);
            let left = gotchi::ENC_PER_BERRY - f.enc_part;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Next berry").size(kit::fs(12.0)).color(TEXT()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{left} to go")).size(kit::fs(12.0)).color(MUTED()));
                });
            });
            kit::bar(ui, ui.available_width(), 7.0, f.enc_part as f32 / gotchi::ENC_PER_BERRY as f32);
            ui.add_space(6.0);
            note(ui, format!("Today: +{} berr{} · +{} treat{}", td.1, if td.1 == 1 { "y" } else { "ies" }, td.2, if td.2 == 1 { "" } else { "s" }));
        });
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    pub(super) fn w_gotchi_box(&mut self, ui: &mut egui::Ui) {
        if !self.w_gotchi_has_pet(ui, "The Box") {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let mut swap = None;
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "📦", PINK, "The Box", Some("Open ›"));
            ui.add_space(8.0);
            let w = ui.available_width();
            let n = gotchi::MAX_BOX as f32;
            let cw = ((w - 8.0 * (n - 1.0)) / n).min(130.0);
            let (row, _) = ui.allocate_exact_size(vec2(w, 108.0), Sense::hover());
            for i in 0..gotchi::MAX_BOX {
                let r = Rect::from_min_size(pos2(row.min.x + i as f32 * (cw + 8.0), row.min.y), vec2(cw, 108.0));
                let p = ui.painter_at(r);
                let Some(pet) = f.pets.get(i) else {
                    p.rect_stroke(r.shrink(0.5), 12.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), PINK, 0.12)));
                    continue;
                };
                let out = i == f.partner;
                let resp = ui.interact(r, ui.id().with(("w-gotchi-box", i)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                p.rect_filled(r, 12.0, theme::blend(kit::sunken(), PINK, if out { 0.16 } else { 0.04 * ht }));
                if out {
                    p.rect_stroke(r.shrink(0.5), 12.0, Stroke::new(1.0f32, PINK.gamma_multiply(0.6)));
                }
                if let Some(si) = self.gotchi_species(&pet.slug) {
                    let sp = self.species[si].clone();
                    self.draw_sprite_centered(&p, &sp, true, pos2(r.center().x, r.min.y + 42.0), vec2(cw - 14.0, 62.0));
                }
                let name = self.gotchi_name(pet);
                p.text(pos2(r.center().x, r.max.y - 26.0), Align2::CENTER_CENTER, &name, crate::fonts::sans_bold_font(12.0), TEXT());
                p.text(pos2(r.center().x, r.max.y - 11.0), Align2::CENTER_CENTER, if out { "Out".to_string() } else { format!("Lv {}", pet.level()) }, crate::fonts::sans_font(10.5), if out { PINK } else { MUTED() });
                if resp.on_hover_text(if out { "Out with you now" } else { "Bring it out" }).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !out {
                    swap = Some(i);
                }
            }
        });
        if let Some(i) = swap {
            self.setup.gotchi.file().swap(i);
            self.setup.gotchi.game = None;
            self.setup.gotchi.say("Hi again!");
            self.setup.gotchi.hop = Some(Instant::now());
            self.setup.gotchi.touch();
        } else if go {
            self.go_to(Tab::Gotchi);
        }
    }
}
