//! Today's sparkly ("rare") grass: the list beside the Map and on Home,
//! "Hunt on sparkly grass" in the Pokédex, and the offer when a target
//! that's on sparkly grass today is picked.

use super::ui_kit::{self as kit, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::config::HuntMethod;
use crate::gamemap::GameMap;
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Pos2, RichText, Sense, Stroke, Vec2};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A Pokémon met on sparkly grass today, and where.
#[derive(Clone)]
pub(super) struct RareSpot {
    pub species: crate::species::Species,
    pub wiki_id: Option<String>,
    /// The first tile it was met on.
    pub x: usize,
    pub y: usize,
    pub zone: Option<u16>,
    pub place: String,
    /// Tiles it was met on in that area.
    pub tiles: usize,
}

/// "Do you want to hunt it on sparkly grass?" after picking a target.
pub(super) struct RareOffer {
    pub slug: String,
    pub name: String,
    pub spots: Vec<RareSpot>,
}

impl App {
    fn game_map(&self) -> Option<Arc<GameMap>> {
        self.gamemap.as_ref()?.as_ref().ok().cloned()
    }

    /// Everything met on sparkly grass today, one entry per Pokémon and area.
    pub(super) fn rare_spots_today(&mut self) -> Vec<RareSpot> {
        let Some(map) = self.game_map() else { return Vec::new() };
        self.mapui.refresh_rare();
        let rare = self.mapui.rare.clone();
        let mut out: Vec<RareSpot> = Vec::new();
        for r in &rare {
            let Some(sp) = r.species else { continue };
            let Some(species) = self.rare_species(&map, sp) else { continue };
            let zone = map.world_zone_at(r.x, r.y);
            if let Some(o) = out.iter_mut().find(|o| o.species.slug == species.slug && o.zone == zone) {
                o.tiles += 1;
                continue;
            }
            let place = zone.map(|z| self.zone_label(&map, z)).unwrap_or_else(|| "Outdoors".into());
            out.push(RareSpot { wiki_id: self.rare_wiki_id(&map, sp), species, x: r.x, y: r.y, zone, place, tiles: 1 });
        }
        out.sort_by(|a, b| a.species.dex.cmp(&b.species.dex).then(a.place.cmp(&b.place)));
        out
    }

    /// Where a Pokémon (by species slug) is on sparkly grass today.
    pub(super) fn rare_spots_for(&mut self, slug: &str) -> Vec<RareSpot> {
        self.rare_spots_today().into_iter().filter(|s| s.species.slug == slug).collect()
    }

    /// Open the Map at a sparkly-grass spot, flashing its area.
    pub(super) fn go_to_rare(&mut self, spot: &RareSpot) {
        let Some(map) = self.game_map() else { return };
        self.mapui.show_rare = true;
        match spot.zone {
            Some(z) => self.show_zone_on_map(&map, z, format!("{} on sparkly grass", spot.species.name)),
            None => self.setup.tab = Tab::Map,
        }
        self.mapui.zoom_to_tile(spot.x, spot.y, 6.0);
    }

    /// Open a Pokémon's Pokédex entry.
    pub(super) fn open_dex_entry(&mut self, wiki_id: &str) {
        self.setup.tab = Tab::Pokedex;
        self.setup.pokedex.open = Some(wiki_id.to_string());
        self.setup.pokedex.tab = super::pokedex::EntryTab::Overview;
    }

    /// Make this the target and hunt it with Sweet Scent on sparkly grass.
    pub(super) fn hunt_on_rare_grass(&mut self, slug: &str, name: &str, spots: &[RareSpot]) {
        self.set_target(slug.to_string());
        self.cfg.hunt_method = HuntMethod::RareGrassSweetScent;
        self.dirty = true;
        self.setup.rare_offer = None;
        self.setup.tab = Tab::Shiny;
        self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        let wher = places_text(spots);
        self.notify(super::notices::Notice {
            id: "rare-hunt",
            icon: "✨",
            title: format!("Hunting {name} on sparkly grass"),
            body: format!("It's on sparkly grass today at {wher}. Use Sweet Scent there."),
            progress: None,
            action: None,
            expires: Some(Instant::now() + Duration::from_secs(12)),
            accent: GOLD,
        });
    }

    /// After a new target is picked: if it's on sparkly grass today, offer
    /// to hunt it there.
    pub(super) fn check_rare_offer(&mut self, slug: &str) {
        let spots = self.rare_spots_for(slug);
        if spots.is_empty() {
            return;
        }
        let name = spots[0].species.name.to_string();
        self.setup.rare_offer = Some(RareOffer { slug: slug.to_string(), name, spots });
    }

    /// The offer, as a small window over the launcher.
    pub(super) fn rare_offer_window(&mut self, ctx: &egui::Context) {
        let Some(offer) = self.setup.rare_offer.as_ref() else { return };
        let (slug, name, spots) = (offer.slug.clone(), offer.name.clone(), offer.spots.clone());
        let already = self.cfg.hunt_method == HuntMethod::RareGrassSweetScent;
        let mut close = false;
        let mut hunt = false;
        let mut show: Option<RareSpot> = None;
        egui::Window::new("✨ On sparkly grass today")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("{name} turned up on sparkly grass today at {}.", places_text(&spots)))
                        .size(kit::fs(13.0))
                        .color(TEXT()),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(if already {
                        "You're already hunting on sparkly grass. Head there and use Sweet Scent."
                    } else {
                        "Do you want to hunt it on the sparkly grass? Sweet Scent there brings it out much more often."
                    })
                    .size(kit::fs(12.0))
                    .color(MUTED()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if !already {
                        let b = egui::Button::new(RichText::new("Hunt on sparkly grass").strong().color(theme::button_ink()));
                        if kit::accent_button(ui, b, 6.0).clicked() {
                            hunt = true;
                        }
                    }
                    if ui.button("Show me where").clicked() {
                        show = spots.first().cloned();
                    }
                    if ui.button(if already { "OK" } else { "No thanks" }).clicked() {
                        close = true;
                    }
                });
            });
        if hunt {
            self.hunt_on_rare_grass(&slug, &name, &spots);
        } else if let Some(s) = show {
            self.setup.rare_offer = None;
            self.go_to_rare(&s);
        } else if close {
            self.setup.rare_offer = None;
        }
    }

    /// The list of today's sparkly-grass Pokémon. `height` caps the list
    /// (it scrolls past that).
    pub(super) fn rare_list(&mut self, ui: &mut egui::Ui, spots: &[RareSpot], height: f32) {
        let mut go: Option<RareSpot> = None;
        let mut open: Option<String> = None;
        egui::ScrollArea::vertical().id_salt(("rare-list", ui.id())).max_height(height).auto_shrink([false, true]).show(ui, |ui| {
            for s in spots {
                let row_w = ui.available_width();
                ui.horizontal(|ui| {
                    ui.set_width(row_w);
                    // sprite, clickable to the Pokédex
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, kit::r(8.0), if resp.hovered() { CARD_HOVER() } else { CARD_HOVER().gamma_multiply(0.7) });
                    p.rect_stroke(r, kit::r(8.0), Stroke::new(1.0f32, if resp.hovered() { GOLD } else { LINE() }));
                    let sp = s.species.clone();
                    self.draw_sprite(&p, &sp, false, r.center_bottom() - Vec2::new(0.0, 3.0), Vec2::splat(40.0));
                    if resp.on_hover_text("Open in the Pokédex").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = s.wiki_id.clone();
                    }
                    ui.vertical(|ui| {
                        if ui
                            .add(egui::Label::new(RichText::new(s.species.name).size(kit::fs(12.5)).strong().color(TEXT())).sense(Sense::click()))
                            .on_hover_text("Open in the Pokédex")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            open = s.wiki_id.clone();
                        }
                        let tiles = if s.tiles > 1 { format!("  ·  {} spots", s.tiles) } else { String::new() };
                        ui.label(RichText::new(format!("{}{tiles}", s.place)).size(kit::fs(11.0)).color(MUTED()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Go to").on_hover_text("Show it on the Map").clicked() {
                            go = Some(s.clone());
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
        if let Some(s) = go {
            self.go_to_rare(&s);
        }
    }

    /// Side panel beside the Map.
    pub(super) fn rare_panel(&mut self, ui: &mut egui::Ui, spots: &[RareSpot], height: f32) {
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(format!("Sparkly grass today ({})", spots.len())).size(kit::fs(13.5)).strong().color(TEXT()));
            ui.label(RichText::new("Pokémon met on sparkly grass, from your game log.").size(kit::fs(11.0)).color(MUTED()));
            ui.add_space(6.0);
            self.rare_list(ui, spots, (height - 90.0).max(120.0));
        });
    }

    /// Home's "Rare grass today" section.
    pub(super) fn rare_home(&mut self, ui: &mut egui::Ui) {
        let ready = self.game_map().is_some();
        let spots = self.rare_spots_today();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            if !ready {
                ui.label(
                    RichText::new("Today's sparkly grass shows here once the Map has read your game's map files.")
                        .size(kit::fs(12.5))
                        .color(MUTED()),
                );
            } else if spots.is_empty() {
                ui.label(
                    RichText::new("Nothing met on sparkly grass yet today. Use Sweet Scent on a sparkly patch and it shows up here.")
                        .size(kit::fs(12.5))
                        .color(MUTED()),
                );
            } else {
                self.rare_list(ui, &spots, 320.0);
            }
        });
    }
}

/// "Route 204 (south)" or "Route 204 (south) and Route 201".
pub(super) fn places_text(spots: &[RareSpot]) -> String {
    let mut places: Vec<&str> = Vec::new();
    for s in spots {
        if !places.contains(&s.place.as_str()) {
            places.push(&s.place);
        }
    }
    match places.len() {
        0 => "an outdoor spot".into(),
        1 => places[0].to_string(),
        _ => format!("{} and {}", places[..places.len() - 1].join(", "), places[places.len() - 1]),
    }
}

impl super::map::MapState {
    /// Centre on a tile, zoomed in.
    pub(super) fn zoom_to_tile(&mut self, x: usize, y: usize, zoom: f32) {
        self.fly_to(Pos2::new(x as f32 + 0.5, y as f32 + 0.5), zoom);
    }
}
