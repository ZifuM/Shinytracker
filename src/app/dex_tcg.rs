//! The Pokédex entry's TCG tab: the trading cards of this Pokémon (and of
//! its family) from the sets the Card Collector knows, one big with its
//! details and the rest as a strip to pick from. With the Card Collector
//! installed, the ones you don't have yet show in grey.

use super::cards_app::{rarity_col, CARD_ASPECT};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::cards::{self, Card, Kind};
use crate::theme::{self, GOLD};
use crate::wiki::{Mon, Wiki};
use eframe::egui::{self, pos2, vec2, Align2, Rect, RichText, Sense, Stroke};
use std::sync::Arc;

/// The cards of a Pokémon (by National Dex number), in set order.
pub(super) fn cards_of(dex: u32) -> Vec<&'static Card> {
    cards::all().iter().filter(|c| c.kind == Kind::Pokemon && c.dex as u32 == dex).collect()
}

impl App {
    /// How many cards a Pokémon has (for the tab's label).
    pub(super) fn tcg_count(&self, m: &Mon) -> usize {
        cards_of(m.dex).len()
    }

    pub(super) fn entry_tcg(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &Mon) {
        let mine = cards_of(m.dex);
        let collecting = self.has_app(AppId::Cards);
        let owned = |this: &mut App, id: &str| if collecting { this.setup.cards.file().count(id) } else { 1 };
        // its family's other cards, for below
        let family: Vec<u32> = w.family(&m.id).into_iter().flatten().filter_map(|id| w.mon(&id).map(|x| x.dex)).filter(|d| *d != m.dex).collect();
        let kin: Vec<&'static Card> = family.iter().flat_map(|d| cards_of(*d)).collect();

        if mine.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("No {} cards yet", m.name)).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("{} isn't in Base Set or Base Set 2. More sets can come later.", m.name)).size(kit::fs(12.5)).color(MUTED()));
            });
            if !kin.is_empty() {
                ui.add_space(14.0);
                self.tcg_strip(ui, "Its family's cards", &kin, None, collecting);
            }
            return;
        }

        // which one is big (kept per Pokémon while the app is open)
        let key = egui::Id::new(("tcg-pick", m.dex));
        let pick_id = ui.ctx().data(|d| d.get_temp::<String>(key)).filter(|id| mine.iter().any(|c| c.id == *id)).unwrap_or_else(|| mine[0].id.clone());
        let c = mine.iter().find(|c| c.id == pick_id).copied().unwrap_or(mine[0]);
        let have = owned(self, &c.id);

        let avail = ui.available_width();
        let wide = avail >= 720.0;
        let big_w = if wide { 300.0f32.min(avail * 0.4) } else { (avail * 0.7).min(300.0) };
        let mut open_collector = false;
        let mut get_app = false;
        let draw_big = |this: &mut App, ui: &mut egui::Ui| {
            let (r, _) = ui.allocate_exact_size(vec2(big_w, big_w * CARD_ASPECT), Sense::hover());
            ui.painter().add(kit::shadow_tier(3).as_shape(r, 12.0));
            if have > 0 {
                this.draw_card(ui, r, c, true, true);
            } else {
                this.draw_card_missing(ui, r, c, true);
            }
        };
        let mut draw_facts = |this: &mut App, ui: &mut egui::Ui| {
            ui.label(RichText::new(c.set_name().to_uppercase()).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.label(RichText::new(&c.name).font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
            ui.add_space(8.0);
            let row = |ui: &mut egui::Ui, a: &str, b: String, col: egui::Color32| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(96.0, 20.0), Sense::hover());
                    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, a, crate::fonts::sans_font(12.5), MUTED());
                    ui.label(RichText::new(b).size(kit::fs(13.0)).color(col));
                });
            };
            row(ui, "Number", format!("{} of {}", c.num, cards::of_set(c.set).len()), TEXT());
            row(ui, "Rarity", format!("{} {}", c.rarity.symbol(), c.rarity.name()), rarity_col(c.rarity));
            row(ui, "Stage", c.stage.clone(), TEXT());
            row(ui, "Type", c.ty.clone(), super::cards_app::type_col(&c.ty));
            row(ui, "HP", c.hp.to_string(), TEXT());
            if let Some(s) = cards::set(c.set) {
                row(ui, "Released", s.year.to_string(), TEXT());
            }
            ui.add_space(10.0);
            if collecting {
                if have > 0 {
                    ui.label(RichText::new(format!("✔  In your binder (×{have})")).size(kit::fs(13.0)).color(theme::GREEN));
                } else {
                    ui.label(RichText::new("Not in your binder yet").size(kit::fs(13.0)).color(MUTED()));
                }
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new("🃏  Open in the Card Collector  ›").size(kit::fs(12.5))).fill(kit::btn_fill()).rounding(14.0)).clicked() {
                    open_collector = true;
                }
            } else {
                ui.label(RichText::new("Collect these with the Card Collector app: hunting earns booster packs.").size(kit::fs(12.5)).color(MUTED()));
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new("🏪  Get the Card Collector  ›").size(kit::fs(12.5))).fill(kit::btn_fill()).rounding(14.0)).clicked() {
                    get_app = true;
                }
            }
            let _ = this;
        };
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 28.0;
                    draw_big(self, ui);
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        draw_facts(self, ui);
                    });
                });
            } else {
                ui.vertical_centered(|ui| draw_big(self, ui));
                ui.add_space(12.0);
                draw_facts(self, ui);
            }
        });
        if mine.len() > 1 {
            ui.add_space(14.0);
            let picked = self.tcg_strip(ui, &format!("All {} cards of {}", mine.len(), m.name), &mine, Some(&c.id), collecting);
            if let Some(id) = picked {
                ui.ctx().data_mut(|d| d.insert_temp(key, id));
            }
        }
        if !kin.is_empty() {
            ui.add_space(14.0);
            let _ = self.tcg_strip(ui, "Its family's cards", &kin, None, collecting);
        }
        if open_collector {
            self.setup.cards.open = Some(c.id.clone());
            self.setup.cards.view = super::cards_app::View::Binder;
            self.go_to(super::Tab::Cards);
        }
        if get_app {
            self.show_store_page(AppId::Cards);
        }
    }

    /// A row of cards to pick from. Returns the one clicked (when `sel`
    /// is given; otherwise a click opens it in the Pokédex).
    fn tcg_strip(&mut self, ui: &mut egui::Ui, title: &str, list: &[&'static Card], sel: Option<&str>, collecting: bool) -> Option<String> {
        let mut out: Option<String> = None;
        let mut goto: Option<u32> = None;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
            ui.add_space(10.0);
            let cw = 112.0;
            let gap = 14.0;
            let w = ui.available_width();
            let cols = ((w + gap) / (cw + gap)).floor().max(1.0) as usize;
            for row in list.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for c in row {
                        let (r, resp) = ui.allocate_exact_size(vec2(cw, cw * CARD_ASPECT + 34.0), Sense::click());
                        let cr = Rect::from_min_size(r.min, vec2(cw, cw * CARD_ASPECT));
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let lift = cr.translate(vec2(0.0, -3.0 * ht));
                        let have = if collecting { self.setup.cards.file().count(&c.id) } else { 1 };
                        ui.painter().add(kit::shadow_tier(if ht > 0.0 { 2 } else { 1 }).as_shape(lift, 6.0));
                        if have > 0 {
                            self.draw_card(ui, lift, c, false, ht > 0.0);
                        } else {
                            self.draw_card_missing(ui, lift, c, false);
                        }
                        if sel == Some(c.id.as_str()) {
                            ui.painter().rect_stroke(lift.expand(3.0), 8.0, Stroke::new(2.0f32, GOLD));
                        }
                        let p = ui.painter();
                        p.text(pos2(r.min.x + 2.0, cr.max.y + 10.0), Align2::LEFT_CENTER, &c.name, crate::fonts::sans_bold_font(11.5), if have > 0 { TEXT() } else { MUTED() });
                        p.text(pos2(r.min.x + 2.0, cr.max.y + 25.0), Align2::LEFT_CENTER, format!("{} · #{}", c.set_name(), c.num), crate::fonts::sans_font(10.5), MUTED());
                        p.text(pos2(r.max.x - 2.0, cr.max.y + 25.0), Align2::RIGHT_CENTER, c.rarity.symbol(), egui::FontId::proportional(10.5), rarity_col(c.rarity));
                        if resp.on_hover_text(if sel.is_some() { "Show this one big" } else { "Its Pokédex page" }).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            if sel.is_some() {
                                out = Some(c.id.clone());
                            } else {
                                goto = Some(c.dex as u32);
                            }
                        }
                    }
                });
                ui.add_space(10.0);
            }
        });
        if let Some(d) = goto {
            let id = self.wiki.as_ref().and_then(|w| w.by_dex(d)).map(|x| x.id.clone());
            if let Some(id) = id {
                self.open_dex_entry(&id);
                self.setup.pokedex.tab = super::pokedex::EntryTab::Tcg;
            }
        }
        out
    }
}
