//! Today's rare grass: the list beside the Map and on Home,
//! "Hunt on rare grass" in the Pokédex, and the offer when a target
//! that's on rare grass today is picked.

use super::ui_kit::{self as kit, MUTED, TEXT};
use super::{App, Tab};
use crate::config::HuntMethod;
use crate::gamemap::GameMap;
use crate::theme::GOLD;
use eframe::egui::{self, Align2, RichText, Vec2};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A Pokémon met on rare grass today, and where.
#[derive(Clone)]
pub(super) struct RareSpot {
    pub species: crate::species::Species,
    pub wiki_id: Option<String>,
    /// The map it's on (0 outdoors, else a side area's).
    pub matrix: u16,
    /// The first tile it was met on.
    pub x: usize,
    pub y: usize,
    pub zone: Option<u16>,
    pub place: String,
    /// Tiles it was met on in that area.
    pub tiles: usize,
}

/// "Do you want to hunt it on rare grass?" after picking a target.
pub(super) struct RareOffer {
    pub slug: String,
    pub name: String,
    pub spots: Vec<RareSpot>,
}

impl App {
    pub(super) fn game_map(&self) -> Option<Arc<GameMap>> {
        self.gamemap.as_ref()?.as_ref().ok().cloned()
    }

    /// Every Pokémon on rare grass today, one entry per Pokémon and area:
    /// worked out with the game's formula for the server's date (cached for
    /// the minute). Place names come from the loaded map
    /// when there is one, else the bundled area list.
    pub(super) fn rare_spots_today(&mut self) -> Vec<RareSpot> {
        let now = crate::stats::now_unix() as i64;
        let key = (crate::rare_calc::today(now), crate::rare_calc::modern(), self.wiki.is_some(), self.game_map().is_some());
        if let Some((k, at, list)) = &self.setup.rare_cache {
            if *k == key && at.elapsed() < Duration::from_secs(60) {
                return list.clone();
            }
        }
        let out = self.rare_spots_on(key.0);
        self.setup.rare_cache = Some((key, Instant::now(), out.clone()));
        out
    }

    /// Every rare grass patch on a date (days since 1970, the server's
    /// calendar), grouped by Pokémon and place.
    pub(super) fn rare_spots_on(&mut self, day: i64) -> Vec<RareSpot> {
        let map = self.game_map();
        let mut out: Vec<RareSpot> = Vec::new();
        for s in crate::rare_calc::spots_on(day, crate::rare_calc::modern()) {
            let sp = s.species as u32;
            let Some(species) = self.rare_species_any(sp) else { continue };
            let zone = s.tile.zone;
            if let Some(o) = out.iter_mut().find(|o| o.species.slug == species.slug && o.zone == zone) {
                o.tiles += 1;
                continue;
            }
            let place = match (zone, &map) {
                (Some(z), Some(m)) => self.zone_label(m, z),
                (Some(z), None) => crate::rare_calc::zone_name(z).unwrap_or("Outdoors").to_string(),
                _ => "Outdoors".into(),
            };
            out.push(RareSpot {
                wiki_id: self.rare_wiki_any(sp),
                species,
                matrix: s.tile.matrix,
                x: s.tile.x as usize,
                y: s.tile.y as usize,
                zone,
                place,
                tiles: 1,
            });
        }
        out.sort_by(|a, b| a.species.dex.cmp(&b.species.dex).then(a.place.cmp(&b.place)));
        out
    }

    /// A game species number as a Pokémon (the newer ones by the bundled list).
    fn rare_species_any(&self, sp: u32) -> Option<crate::species::Species> {
        let dex = if sp <= 493 {
            sp
        } else {
            let id = crate::rare_calc::game_species_id(sp)?;
            self.wiki.as_ref()?.mon(id)?.dex
        };
        self.species.get((dex as usize).wrapping_sub(1)).cloned()
    }

    /// The wiki id for a game species number.
    fn rare_wiki_any(&self, sp: u32) -> Option<String> {
        if sp <= 493 {
            Some(self.wiki.as_ref()?.by_dex(sp)?.id.clone())
        } else {
            crate::rare_calc::game_species_id(sp).map(|s| s.to_string())
        }
    }

    /// Where a Pokémon (by species slug) is on rare grass today.
    pub(super) fn rare_spots_for(&mut self, slug: &str) -> Vec<RareSpot> {
        self.rare_spots_today().into_iter().filter(|s| s.species.slug == slug).collect()
    }

    /// Open the Map at a rare-grass spot, a ring pinging on it.
    pub(super) fn go_to_rare(&mut self, spot: &RareSpot) {
        self.add_event("touch_grass");
        let Some(map) = self.game_map() else { return };
        self.show_rare_spot(&map, spot.matrix, spot.x, spot.y, format!("{} on rare grass", spot.species.name));
    }

    /// Open a Pokémon's Pokédex entry.
    pub(super) fn open_dex_entry(&mut self, wiki_id: &str) {
        self.add_event("researcher");
        self.setup.tab = Tab::Pokedex;
        self.setup.pokedex.open = Some(wiki_id.to_string());
        self.setup.pokedex.tab = super::pokedex::EntryTab::Overview;
    }

    /// Make this the target and hunt it with Sweet Scent on rare grass.
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
            title: format!("Hunting {name} on rare grass"),
            body: format!("It's on rare grass today at {wher}. Use Sweet Scent there."),
            progress: None,
            action: None,
            expires: Some(Instant::now() + Duration::from_secs(12)),
            accent: GOLD,
        });
    }

    /// After a new target is picked: if it's on rare grass today, offer
    /// to hunt it there.
    pub(super) fn check_rare_offer(&mut self, slug: &str) {
        let spots = self.rare_spots_for(slug);
        if spots.is_empty() {
            return;
        }
        let name = spots[0].species.name.to_string();
        self.setup.rare_offer = Some(RareOffer {
            slug: slug.to_string(),
            name,
            spots,
        });
    }

    /// The offer, as a small window over the launcher.
    pub(super) fn rare_offer_window(&mut self, ctx: &egui::Context) {
        let Some(offer) = self.setup.rare_offer.as_ref() else { return };
        let (slug, name, spots) = (offer.slug.clone(), offer.name.clone(), offer.spots.clone());
        let already = self.cfg.hunt_method == HuntMethod::RareGrassSweetScent;
        let mut close = false;
        let mut hunt = false;
        let mut show: Option<RareSpot> = None;
        egui::Window::new("✨ On rare grass today")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("{name} turned up on rare grass today at {}.", places_text(&spots)))
                        .size(kit::fs(13.0))
                        .color(TEXT()),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(if already {
                        "You're already hunting on rare grass. Head there and use Sweet Scent."
                    } else {
                        "Do you want to hunt it on the rare grass? Sweet Scent there brings it out much more often."
                    })
                    .size(kit::fs(12.0))
                    .color(MUTED()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if !already {
                        let b = egui::Button::new(RichText::new("Hunt on rare grass").family(kit::strong_family(13.5)).strong().color(kit::on_primary()));
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
