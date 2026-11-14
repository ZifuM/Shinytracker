//! Today's rare grass: the list beside the Map and on Home,
//! "Hunt on rare grass" in the Pokédex, and the offer when a target
//! that's on rare grass today is picked.

use super::ui_kit::{self as kit, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::config::HuntMethod;
use crate::gamemap::GameMap;
use crate::theme::GOLD;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
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

    /// `rare_home`, optionally with a heading inside the card; the heading
    /// closure draws it and returns true when its link was clicked.
    pub(super) fn rare_home_titled(&mut self, ui: &mut egui::Ui, head: Option<&dyn Fn(&mut egui::Ui, usize) -> bool>) -> bool {
        let ready = self.game_map().is_some();
        let spots = self.rare_spots_today();
        let mut clicked = false;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(h) = head {
                clicked = h(ui, spots.len());
                ui.add_space(10.0);
            }
            if !ready {
                ui.label(
                    RichText::new("Today's rare grass shows here once the Map has read your game's map files.")
                        .size(kit::fs(12.5))
                        .color(MUTED()),
                );
            } else if spots.is_empty() {
                ui.label(
                    RichText::new("Nothing met on rare grass yet today. Use Sweet Scent on a rare grass patch and it shows up here.")
                        .size(kit::fs(12.5))
                        .color(MUTED()),
                );
            } else {
                // A grid of tiles, as many rows as fit three; the Map has the rest.
                const GAP: f32 = 10.0;
                const TILE_H: f32 = 74.0;
                let avail = ui.available_width();
                let cols = ((avail + GAP) / (230.0 + GAP)).floor().max(1.0) as usize;
                let tile_w = ((avail - GAP * (cols as f32 - 1.0)) / cols as f32).floor();
                let shown = spots.len().min(cols * 3);
                let mut go: Option<RareSpot> = None;
                let mut open: Option<String> = None;
                for row in spots[..shown].chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP;
                        for s in row {
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(tile_w, TILE_H), Sense::click());
                            let p = ui.painter_at(r);
                            let hot = resp.hovered();
                            p.rect_filled(r, kit::r(10.0), if hot { CARD_HOVER() } else { CARD_HOVER().gamma_multiply(0.6) });
                            p.rect_stroke(r, kit::r(10.0), Stroke::new(1.0f32, if hot { GOLD } else { LINE() }));
                            // the Pokémon (click: its Pokédex entry)
                            let sr = Rect::from_min_size(r.min + Vec2::new(8.0, (TILE_H - 58.0) / 2.0), Vec2::splat(58.0));
                            let sp_resp = ui.interact(sr, ui.id().with(("rare-tile-sprite", s.species.slug.as_str(), s.place.as_str())), Sense::click());
                            p.circle_filled(sr.center(), 26.0, Color32::from_black_alpha(if sp_resp.hovered() { 90 } else { 50 }));
                            self.draw_sprite(&p, &s.species, false, sr.center_bottom() - Vec2::new(0.0, 4.0), Vec2::splat(54.0));
                            let tx = sr.right() + 8.0;
                            let maxw = r.right() - tx - 8.0;
                            let clip = |t: &str, size: f32, bold: bool, col: Color32| {
                                let font = if bold {
                                    kit::font_bold(kit::fs(size))
                                } else {
                                    egui::FontId::proportional(kit::fs(size))
                                };
                                let mut t = t.to_string();
                                loop {
                                    let g = p.layout_no_wrap(t.clone(), font.clone(), col);
                                    if g.size().x <= maxw || t.chars().count() <= 2 {
                                        return g;
                                    }
                                    t = t.trim_end_matches('…').to_string();
                                    t.pop();
                                    t.push('…');
                                }
                            };
                            let g1 = clip(s.species.name, 13.0, true, TEXT());
                            let g2 = clip(&s.place, 11.5, false, MUTED());
                            let spots_t = if s.tiles > 1 { format!("{} spots", s.tiles) } else { "1 spot".to_string() };
                            let g3 = clip(&spots_t, 11.0, false, GOLD);
                            let y0 = r.center().y - (g1.size().y + g2.size().y + g3.size().y + 4.0) / 2.0;
                            let h1 = g1.size().y;
                            let h2 = g2.size().y;
                            p.galley(Pos2::new(tx, y0), g1, TEXT());
                            p.galley(Pos2::new(tx, y0 + h1 + 2.0), g2, MUTED());
                            p.galley(Pos2::new(tx, y0 + h1 + h2 + 4.0), g3, GOLD);
                            if sp_resp.on_hover_text("Open in the Pokédex").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                open = s.wiki_id.clone();
                            } else if resp
                                .on_hover_text(format!("Show {} on the Map", s.place))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                go = Some(s.clone());
                            }
                        }
                    });
                    ui.add_space(GAP - ui.spacing().item_spacing.y);
                }
                if let Some(id) = open {
                    self.open_dex_entry(&id);
                } else if let Some(s) = go {
                    self.go_to_rare(&s);
                }
                if spots.len() > shown {
                    ui.add_space(4.0);
                    if ui
                        .add(egui::Button::new(RichText::new(format!("See all {} on the Map", spots.len())).size(kit::fs(12.5))).fill(kit::btn_fill()))
                        .clicked()
                    {
                        self.setup.tab = Tab::Map;
                        self.setup.scroll_top = true;
                    }
                }
            }
        });
        clicked
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
