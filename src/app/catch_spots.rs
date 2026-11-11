//! Pokédex "Where to find": cut-outs of the map around each place's grass
//! (or water, or cave floor) with the tiles you can meet the Pokémon on
//! outlined, and today's rare-grass sightings of it.

use super::map::tile_edges;
use super::pokedex::card_title;
use super::ui_kit::{self as kit, PillKind, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::gamemap::{bbox, GameMap, Spots, CHUNK, PX};
use crate::theme::{self, GOLD};
use crate::wiki::{Mon, Wiki, Wild};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::sync::Arc;

/// Tiles of a place where a Pokémon can be met, and the cut-out around them.
#[derive(Clone)]
pub(super) struct SpotSet {
    pub matrix: usize,
    pub tiles: Vec<(usize, usize)>,
    /// x0, y0, x1, y1 (tiles, end exclusive)
    pub rect: (usize, usize, usize, usize),
    /// false = the place is one of several identical rooms.
    pub exact: bool,
}

/// Room around the tiles in a cut-out.
const PAD: usize = 3;
/// Most previews shown before "Show all".
const FIRST: usize = 4;

fn spot_set(map: &GameMap, place: &str, spots: Spots) -> Option<SpotSet> {
    let link = map.place_zones(place)?;
    let first = *link.zones.first()?;
    let matrix = map.zone(first)?.matrix as usize;
    let zones: Vec<u16> = link.zones.iter().copied().filter(|z| map.zone(*z).is_some_and(|z| z.matrix as usize == matrix)).collect();
    // For a group of identical rooms, show the first one.
    let zones = if link.exact { zones } else { vec![first] };
    let mut tiles: Vec<(usize, usize)> = Vec::new();
    for z in &zones {
        tiles.extend(map.spot_tiles(*z, spots));
    }
    if tiles.is_empty() && spots == Spots::Cave {
        for z in &zones {
            tiles.extend(map.spot_tiles(*z, Spots::Land));
        }
    }
    let mut cells: Vec<(usize, usize)> = Vec::new();
    for z in &zones {
        cells.extend(map.zone_cells(*z));
    }
    let (cx0, cy0, cx1, cy1) = bbox(&cells)?;
    let (zx0, zy0, zx1, zy1) = (cx0 * CHUNK, cy0 * CHUNK, cx1 * CHUNK, cy1 * CHUNK);
    let rect = match bbox(&tiles) {
        Some((x0, y0, x1, y1)) => (x0.saturating_sub(PAD).max(zx0), y0.saturating_sub(PAD).max(zy0), (x1 + PAD).min(zx1), (y1 + PAD).min(zy1)),
        None => (zx0, zy0, zx1, zy1),
    };
    Some(SpotSet {
        matrix,
        tiles,
        rect,
        exact: link.exact,
    })
}

impl App {
    /// The game's map, if it's been read.
    fn map_ready(&mut self) -> Option<Arc<GameMap>> {
        match self.gamemap.as_ref()? {
            Ok(m) => Some(m.clone()),
            Err(_) => None,
        }
    }

    /// "Catch spots": a cut-out for each place (and way) it lives.
    pub(super) fn catch_spots(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &Mon, wild: &[&Wild]) {
        let Some(map) = self.map_ready() else {
            if let Some(Err(e)) = &self.gamemap {
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    card_title(ui, "Catch spots");
                    ui.label(RichText::new(format!("Map previews need your game's map files: {e}")).size(kit::fs(11.5)).color(MUTED()));
                });
                ui.add_space(10.0);
            }
            return;
        };
        // One preview per place and kind of tiles, in the table's order.
        let mut items: Vec<(String, Spots)> = Vec::new();
        for e in wild {
            let k = (e.area.clone(), Spots::for_method(&e.method));
            if !items.contains(&k) {
                items.push(k);
            }
        }
        if items.is_empty() {
            return;
        }
        // (catch-spot previews are of outdoor places)
        let rare: Vec<(usize, usize)> = self.rare_tiles_of(&map, &m.id).into_iter().filter(|t| t.0 == 0).map(|t| (t.1, t.2)).collect();
        let show_all = self.setup.pokedex.all_spots;
        let total = items.len();
        let shown: Vec<(String, Spots)> = if show_all { items } else { items.into_iter().take(FIRST).collect() };
        let mut go: Option<(String, Spots)> = None;
        let mut toggle = false;
        let avail = ui.available_width();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                card_title(ui, "Catch spots");
                ui.label(
                    RichText::new("The outlined tiles are where you can meet it. Click a picture to open it on the Map.")
                        .size(kit::fs(11.0))
                        .color(MUTED()),
                );
            });
            let cols = if avail > 760.0 { 2 } else { 1 };
            let card_w = ((ui.available_width() - 12.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in shown.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (place, spots) in row {
                        let title = format!(
                            "{}  ·  {}",
                            w.place(place).map(|p| p.name.clone()).unwrap_or_else(|| place.replace('_', " ")),
                            spots.label()
                        );
                        ui.allocate_ui_with_layout(Vec2::new(card_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(card_w);
                            if self.spot_preview(ui, &map, place, *spots, &title, &rare, card_w) {
                                go = Some((place.clone(), *spots));
                            }
                        });
                    }
                });
                ui.add_space(10.0);
            }
            if total > FIRST {
                let t = if show_all { "Show fewer".to_string() } else { format!("Show all {total} places") };
                if ui.link(RichText::new(t).size(kit::fs(12.0))).clicked() {
                    toggle = true;
                }
            }
        });
        ui.add_space(10.0);
        if toggle {
            self.setup.pokedex.all_spots = !show_all;
        }
        if let Some((place, spots)) = go {
            let title = w.place(&place).map(|p| p.name.clone()).unwrap_or(place.clone());
            self.show_on_map(&[place], title, Some(spots));
        }
    }

    /// One cut-out with its tiles outlined. True when clicked.
    #[allow(clippy::too_many_arguments)]
    fn spot_preview(&mut self, ui: &mut egui::Ui, map: &GameMap, place: &str, spots: Spots, title: &str, rare: &[(usize, usize)], width: f32) -> bool {
        let key = format!("{place}|{spots:?}");
        if !self.mapui.spot_sets.contains_key(&key) {
            let set = spot_set(map, place, spots);
            self.mapui.spot_sets.insert(key.clone(), set);
        }
        let Some(set) = self.mapui.spot_sets.get(&key).cloned().flatten() else {
            ui.label(RichText::new(title).size(kit::fs(12.5)).strong().color(TEXT()));
            ui.label(RichText::new("Not on the map yet.").size(kit::fs(11.5)).color(MUTED()));
            return false;
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(kit::fs(12.5)).strong().color(TEXT()));
            if !set.exact {
                kit::pill(ui, "One of several rooms", PillKind::Info).on_hover_text("This Pokémon lives in several rooms that are all alike; this is one of them.");
            }
        });
        let (x0, y0, x1, y1) = set.rect;
        let (tw, th) = ((x1 - x0) as f32, (y1 - y0) as f32);
        let max_h = 260.0;
        let scale = (width / (tw * PX as f32)).min(max_h / (th * PX as f32)).min(3.0);
        let size = Vec2::new(tw * PX as f32 * scale, th * PX as f32 * scale);
        let (r, resp) = ui.allocate_exact_size(size, Sense::click());
        let p = ui.painter_at(r);
        p.rect_filled(r, kit::r(8.0), CARD_HOVER());
        let crop_key = format!("{}:{x0},{y0},{x1},{y1}", set.matrix);
        let tex = self.mapui.crop(&crop_key, set.matrix, set.rect).map(|c| c.tex.id());
        match tex {
            Some(id) => {
                p.image(id, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            None if self.mapui.crop_failed(&crop_key) => {
                p.text(r.center(), egui::Align2::CENTER_CENTER, "Couldn't draw this part of the map", kit::font(11.0), MUTED());
            }
            None => {
                p.text(r.center(), egui::Align2::CENTER_CENTER, "Drawing…", kit::font(11.0), MUTED());
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        // outline the tiles
        let tile = PX as f32 * scale;
        let at = |t: Pos2| r.min + Vec2::new((t.x - x0 as f32) * tile, (t.y - y0 as f32) * tile);
        for (x, y) in &set.tiles {
            let tr = Rect::from_min_size(at(Pos2::new(*x as f32, *y as f32)), Vec2::splat(tile));
            p.rect_filled(tr, 0.0, Color32::from_rgba_unmultiplied(255, 220, 60, 38));
        }
        if !set.tiles.is_empty() {
            let stroke = Stroke::new((tile * 0.22).clamp(1.5, 3.0), Color32::from_rgb(255, 210, 40));
            for (a, b) in tile_edges(&set.tiles) {
                p.line_segment([at(a), at(b)], stroke);
            }
        }
        // today's rare grass sightings of it
        for (x, y) in rare {
            if (x0..x1).contains(x) && (y0..y1).contains(y) {
                let c = at(Pos2::new(*x as f32 + 0.5, *y as f32 + 0.5));
                theme::sparkle(&p, c, (tile * 0.9).clamp(5.0, 12.0), GOLD);
            }
        }
        p.rect_stroke(r, kit::r(8.0), Stroke::new(1.0f32, if resp.hovered() { theme::LINK() } else { LINE() }));
        let n = set.tiles.len();
        let what = match spots {
            Spots::Land => "grass tiles",
            Spots::Cave => "cave floor tiles",
            Spots::Water => "water tiles",
        };
        ui.label(RichText::new(format!("{n} {what}")).size(kit::fs(11.0)).color(MUTED()));
        resp.on_hover_text("Open on the Map").on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    /// Today's rare-grass tiles where this Pokémon was met.
    fn rare_tiles_of(&mut self, map: &GameMap, id: &str) -> Vec<(u16, usize, usize)> {
        self.mapui.refresh_rare();
        let rare = self.mapui.rare.clone();
        rare.iter()
            .filter_map(|r| {
                let sp = r.species?;
                (self.rare_wiki_id(map, sp).as_deref() == Some(id)).then_some((r.matrix, r.x, r.y))
            })
            .collect()
    }

    /// "Rare grass today": where it turned up on rare grass since the
    /// daily reset, from the game log.
    pub(super) fn rare_spots(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>, m: &Mon) {
        let Some(map) = self.map_ready() else { return };
        let tiles = self.rare_tiles_of(&map, &m.id);
        if tiles.is_empty() {
            return;
        }
        // group sightings that are close together
        // (each group is on one map)
        let mut groups: Vec<(u16, Vec<(usize, usize)>)> = Vec::new();
        for (m, x, y) in tiles {
            match groups
                .iter_mut()
                .find(|(gm, g)| *gm == m && g.iter().any(|o| o.0.abs_diff(x) <= 10 && o.1.abs_diff(y) <= 10))
            {
                Some((_, g)) => g.push((x, y)),
                None => groups.push((m, vec![(x, y)])),
            }
        }
        let mut go: Option<(u16, usize, usize)> = None;
        let avail = ui.available_width();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                card_title(ui, "Rare grass today");
                ui.label(RichText::new("From your game log since the daily reset.").size(kit::fs(11.0)).color(MUTED()));
            });
            let cols = if avail > 760.0 { 2 } else { 1 };
            let card_w = ((ui.available_width() - 12.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in groups.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (gm, g) in row {
                        let gm = *gm;
                        let (bx0, by0, bx1, by1) = bbox(g).unwrap_or((0, 0, 1, 1));
                        let rect = (bx0.saturating_sub(8), by0.saturating_sub(8), bx1 + 8, by1 + 8);
                        let zone = map.rare_zone(gm, g[0].0, g[0].1).map(|z| self.zone_label(&map, z)).unwrap_or_else(|| "Outdoors".into());
                        ui.allocate_ui_with_layout(Vec2::new(card_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(card_w);
                            ui.label(
                                RichText::new(format!("{zone}  ·  {} spot{}", g.len(), if g.len() == 1 { "" } else { "s" }))
                                    .size(kit::fs(12.5))
                                    .strong()
                                    .color(TEXT()),
                            );
                            let (x0, y0, x1, y1) = rect;
                            let scale = (card_w / ((x1 - x0) * PX) as f32).min(200.0 / ((y1 - y0) * PX) as f32).min(3.0);
                            let size = Vec2::new(((x1 - x0) * PX) as f32 * scale, ((y1 - y0) * PX) as f32 * scale);
                            let (r, resp) = ui.allocate_exact_size(size, Sense::click());
                            let p = ui.painter_at(r);
                            p.rect_filled(r, kit::r(8.0), CARD_HOVER());
                            let key = format!("{gm}:{x0},{y0},{x1},{y1}");
                            let tex = self.mapui.crop(&key, gm as usize, rect).map(|c| c.tex.id());
                            if let Some(id) = tex {
                                p.image(id, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                            } else {
                                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                            }
                            let tile = PX as f32 * scale;
                            for (x, y) in g {
                                let c = r.min + Vec2::new((*x - x0) as f32 * tile + tile / 2.0, (*y - y0) as f32 * tile + tile / 2.0);
                                p.rect_stroke(Rect::from_center_size(c, Vec2::splat(tile)), 0.0, Stroke::new(2.0f32, GOLD));
                                theme::sparkle(&p, c, (tile * 0.8).clamp(5.0, 12.0), GOLD);
                            }
                            p.rect_stroke(r, kit::r(8.0), Stroke::new(1.0f32, if resp.hovered() { theme::LINK() } else { LINE() }));
                            if resp.on_hover_text("Open on the Map").clicked() {
                                go = Some((gm, g[0].0, g[0].1));
                            }
                        });
                    }
                });
                ui.add_space(10.0);
            }
            ui.label(RichText::new("Rare grass moves every day; these are today's.").size(kit::fs(11.0)).color(MUTED()));
        });
        ui.add_space(10.0);
        if let Some((gm, x, y)) = go {
            self.show_rare_spot(&map, gm, x, y, format!("{} on rare grass", m.name));
        }
    }
}
