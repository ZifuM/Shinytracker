//! Dashboard widgets for the Card Collector.

use super::cards_app::{mission_row, xp_bar, xp_rules, CARD_ASPECT};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{self as wk, head, note};
use super::{App, Tab};
use crate::cards::{self, Rarity, XP_PER_PACK};
use crate::theme::GOLD;
use eframe::egui::{self, pos2, vec2, Rect, RichText, Sense};

impl App {
    fn open_cards(&mut self, view: super::cards_app::View) {
        self.setup.cards.view = view;
        self.go_to(Tab::Cards);
    }

    pub(super) fn w_cards_xp(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        let pack_at = self.setup.cards.pack_at;
        let gain = self.setup.cards.gains.back().map(|g| g.0.clone());
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "⭐", GOLD, "Booster progress", Some("Open ›"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(f.xp.to_string()).font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
                ui.label(RichText::new(format!("/ {XP_PER_PACK} XP")).size(kit::fs(12.5)).color(MUTED()));
            });
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 12.0), Sense::hover());
            xp_bar(ui.painter(), r, f.xp as f32 / XP_PER_PACK as f32, ui.input(|i| i.time) as f32, pack_at);
            resp.on_hover_text(xp_rules());
            ui.add_space(6.0);
            match gain {
                Some(g) => note(ui, format!("Last: +{} XP · {}", g.xp, g.why)),
                None => note(ui, format!("{} XP to the next pack", XP_PER_PACK - f.xp)),
            }
            if f.packs > 0 {
                ui.label(RichText::new(format!("🎁  {} pack{} to open", f.packs, if f.packs == 1 { "" } else { "s" })).size(kit::fs(12.5)).color(GOLD));
            }
        });
        if go {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    pub(super) fn w_cards_packs(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        let mut open: Option<&'static str> = None;
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🎁", kit::C_PINK, "Packs to open", Some("All ›"));
            ui.add_space(6.0);
            ui.label(RichText::new(if f.packs > 0 { format!("{} waiting", f.packs) } else { "None right now".to_string() }).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(8.0);
            let w = ui.available_width();
            let pw = ((w - 12.0) / 2.0).min(120.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for s in cards::SETS.iter() {
                    let (r, resp) = ui.allocate_exact_size(vec2(pw, pw * 1.45), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && f.packs > 0, kit::QUICK);
                    self.draw_booster(ui, r.translate(vec2(0.0, -4.0 * ht)), s, f.packs > 0);
                    if resp.on_hover_text(if f.packs > 0 { format!("Open a {} pack", s.name) } else { "No packs yet".to_string() }).clicked() && f.packs > 0 {
                        open = Some(s.id);
                    }
                }
            });
            if f.packs == 0 {
                ui.add_space(6.0);
                note(ui, format!("{} XP to the next one.", XP_PER_PACK - f.xp));
            }
        });
        if let Some(s) = open {
            self.go_to(Tab::Cards);
            self.setup.cards.choose = cards::set(s).map(|x| x.id);
        } else if go {
            self.open_cards(super::cards_app::View::Packs);
        }
    }

    pub(super) fn w_cards_pulls(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        let mut look: Option<String> = None;
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🃏", kit::C_GOLD, "Latest pulls", Some("Binder ›"));
            ui.add_space(8.0);
            let mut best: Vec<&cards::Card> = f.pulls.iter().take(3).flat_map(|p| p.cards.iter()).filter_map(|id| cards::by_id(id)).filter(|c| c.rarity >= Rarity::Uncommon).collect();
            best.sort_by_key(|c| std::cmp::Reverse(c.rarity));
            best.dedup_by(|a, b| a.id == b.id);
            if best.is_empty() {
                note(ui, "Open a pack and your best cards show here.");
                return;
            }
            let w = ui.available_width();
            let cw = ((w - 8.0 * 5.0) / 6.0).clamp(56.0, 110.0);
            let fit = ((w + 8.0) / (cw + 8.0)).floor() as usize;
            let (rr, _) = ui.allocate_exact_size(vec2(w, cw * CARD_ASPECT), Sense::hover());
            for (k, c) in best.iter().take(fit).enumerate() {
                let r = Rect::from_min_size(pos2(rr.min.x + k as f32 * (cw + 8.0), rr.min.y), vec2(cw, cw * CARD_ASPECT));
                self.draw_card(ui, r, c, false, true);
                if ui.interact(r, ui.id().with(("w-pull", k)), Sense::click()).on_hover_text(&c.name).clicked() {
                    look = Some(c.id.clone());
                }
            }
        });
        if look.is_some() {
            self.setup.cards.open = look;
            self.open_cards(super::cards_app::View::Binder);
        } else if go {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    pub(super) fn w_cards_sets(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "📚", kit::C_CYAN, "Set completion", Some("Binder ›"));
            ui.add_space(8.0);
            for s in cards::SETS.iter() {
                let n = cards::of_set(s.id).len();
                let h = f.unique(Some(s.id));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(s.name).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{h} / {n}")).size(kit::fs(12.0)).color(MUTED()));
                    });
                });
                kit::bar(ui, ui.available_width(), 7.0, h as f32 / n as f32);
                // holo rares: the ones everyone wants
                let holos = cards::of_set(s.id).into_iter().filter(|c| c.rarity == Rarity::Holo).collect::<Vec<_>>();
                let got = holos.iter().filter(|c| f.count(&c.id) > 0).count();
                ui.label(RichText::new(format!("★ {got} of {} holo rares", holos.len())).size(kit::fs(11.5)).color(if got > 0 { GOLD } else { MUTED() }));
                ui.add_space(8.0);
            }
        });
        if go {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    pub(super) fn w_cards_missions(&mut self, ui: &mut egui::Ui) {
        let day = crate::stats::local_day(crate::config::now_unix());
        self.setup.cards.file().roll_over(day);
        let f = self.setup.cards.file().clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = head(ui, "🎯", kit::C_CYAN, "Card missions", Some("All ›"));
            ui.add_space(6.0);
            for m in &f.day.missions {
                mission_row(ui, m, f.day.progress(m), kit::C_CYAN);
            }
            let left = f.week.missions.iter().filter(|m| !m.done).count();
            if left > 0 && wk::link(ui, &format!("{left} weekly mission{} too ›", if left == 1 { "" } else { "s" })) {
                go = true;
            }
        });
        if go {
            self.open_cards(super::cards_app::View::Missions);
        }
    }
}
