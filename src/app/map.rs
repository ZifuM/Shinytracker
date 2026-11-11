//! The Map app: Sinnoh from the game's own pictures, zoomable, with every
//! area outlined, today's sparkly grass from the game log, and a slow
//! flashing highlight for "Show on map". Also makes the cut-out previews
//! the Pokédex shows under "Where to find".

use super::ui_kit::{self as kit, PillKind, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::gamemap::{self, bbox, Crop, GameMap, Rare, Spots, CHUNK};
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Below this many pixels a tile, the zoomed-out picture is used.
const DETAIL_ZOOM: f32 = 3.0;
const MIN_ZOOM: f32 = 0.35;
const MAX_ZOOM: f32 = 24.0;
/// Most full-size map squares kept on the graphics card.
const MAX_CHUNKS: usize = 160;
/// One slow flash, in seconds.
const PULSE: f32 = 1.8;

/// Which map is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Area {
    World,
    /// Inside: the area (map header) whose map is showing.
    Inside(u16),
}

pub(super) struct Highlight {
    pub zones: Vec<u16>,
    pub title: String,
    pub spots: Option<Spots>,
    pub since: Instant,
}

enum Job {
    Chunk(usize, usize),
    Crop { key: String, matrix: usize, rect: (usize, usize, usize, usize) },
    Rare,
}

enum Done {
    Chunk(usize, usize, Option<egui::ColorImage>),
    Crop(String, Option<Crop>),
    Rare(Vec<Rare>),
}

/// A finished cut-out, ready to draw.
pub(super) struct CropTex {
    pub tex: TextureHandle,
}

#[derive(Default)]
pub(super) struct MapState {
    pub area: Option<Area>,
    pub center: Pos2,
    pub zoom: f32,
    fitted: bool,
    pub selected: Option<u16>,
    pub highlight: Option<Highlight>,
    pub search: String,
    pub show_rare: bool,
    pub labels: bool,
    /// Size of the map on screen last frame (for fitting areas into it).
    canvas: Vec2,
    flying: Option<(Pos2, f32, Pos2, f32, Instant)>,
    overview: Option<TextureHandle>,
    chunks: HashMap<(usize, usize), TextureHandle>,
    chunk_order: VecDeque<(usize, usize)>,
    pending: HashSet<(usize, usize)>,
    inside: HashMap<usize, TextureHandle>,
    pub crops: HashMap<String, Option<CropTex>>,
    /// Catch-spot tiles worked out for the Pokédex, by place and kind.
    pub spot_sets: HashMap<String, Option<super::catch_spots::SpotSet>>,
    crop_pending: HashSet<String>,
    pub rare: Vec<Rare>,
    rare_asked: Option<Instant>,
    /// Outline pieces of each outdoor area (tile coordinates).
    edges: HashMap<u16, Vec<(Pos2, Pos2)>>,
    tx: Option<Sender<Job>>,
    rx: Option<Receiver<Done>>,
    started: bool,
}

/// Outline pieces between map squares that belong to different areas.
fn world_edges(map: &GameMap) -> HashMap<u16, Vec<(Pos2, Pos2)>> {
    let m = map.world();
    let zone = |cx: i32, cy: i32| -> Option<u16> {
        if cx < 0 || cy < 0 {
            return None;
        }
        let (cx, cy) = (cx as usize, cy as usize);
        m.land_at(cx, cy)?;
        m.header_at(cx, cy).filter(|&h| h != 0 && map.zone(h).is_some())
    };
    let mut out: HashMap<u16, Vec<(Pos2, Pos2)>> = HashMap::new();
    let c = CHUNK as f32;
    for cy in 0..m.h as i32 {
        for cx in 0..m.w as i32 {
            let Some(z) = zone(cx, cy) else { continue };
            let (x, y) = (cx as f32 * c, cy as f32 * c);
            let sides = [
                ((0, -1), Pos2::new(x, y), Pos2::new(x + c, y)),
                ((0, 1), Pos2::new(x, y + c), Pos2::new(x + c, y + c)),
                ((-1, 0), Pos2::new(x, y), Pos2::new(x, y + c)),
                ((1, 0), Pos2::new(x + c, y), Pos2::new(x + c, y + c)),
            ];
            for ((dx, dy), a, b) in sides {
                if zone(cx + dx, cy + dy) != Some(z) {
                    out.entry(z).or_default().push((a, b));
                }
            }
        }
    }
    out
}

/// Outline pieces around a set of tiles (in tile units).
pub(super) fn tile_edges(tiles: &[(usize, usize)]) -> Vec<(Pos2, Pos2)> {
    let set: HashSet<(usize, usize)> = tiles.iter().copied().collect();
    let has = |x: i64, y: i64| x >= 0 && y >= 0 && set.contains(&(x as usize, y as usize));
    let mut out = Vec::new();
    for &(x, y) in tiles {
        let (xi, yi) = (x as i64, y as i64);
        let (fx, fy) = (x as f32, y as f32);
        if !has(xi, yi - 1) {
            out.push((Pos2::new(fx, fy), Pos2::new(fx + 1.0, fy)));
        }
        if !has(xi, yi + 1) {
            out.push((Pos2::new(fx, fy + 1.0), Pos2::new(fx + 1.0, fy + 1.0)));
        }
        if !has(xi - 1, yi) {
            out.push((Pos2::new(fx, fy), Pos2::new(fx, fy + 1.0)));
        }
        if !has(xi + 1, yi) {
            out.push((Pos2::new(fx + 1.0, fy), Pos2::new(fx + 1.0, fy + 1.0)));
        }
    }
    out
}

/// Text with a dark halo, readable on any part of the map.
fn halo_text(p: &egui::Painter, pos: Pos2, text: &str, size: f32, color: Color32) {
    let font = kit::font_bold(size);
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (1.0, 1.0), (-1.0, -1.0)] {
        p.text(pos + Vec2::new(dx, dy), Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha(200));
    }
    p.text(pos, Align2::CENTER_CENTER, text, font, color);
}

fn pulse(since: Instant) -> f32 {
    let t = since.elapsed().as_secs_f32();
    0.5 + 0.5 * (t * std::f32::consts::TAU / PULSE).sin()
}

impl MapState {
    /// Start the background worker for a loaded map.
    fn attach(&mut self, map: &Arc<GameMap>) {
        if self.started {
            return;
        }
        self.started = true;
        self.show_rare = true;
        self.labels = true;
        if self.zoom <= 0.0 {
            self.zoom = 1.0;
        }
        self.edges = world_edges(map);
        let (tx, jobs) = std::sync::mpsc::channel::<Job>();
        let (done_tx, rx) = std::sync::mpsc::channel::<Done>();
        let map = map.clone();
        let _ = std::thread::Builder::new().name("map-worker".into()).spawn(move || {
            for job in jobs {
                let done = match job {
                    Job::Chunk(cx, cy) => Done::Chunk(cx, cy, gamemap::load_minimap(&map.dir, cx, cy)),
                    Job::Crop { key, matrix, rect } => Done::Crop(key, map.crop(matrix, rect.0, rect.1, rect.2, rect.3)),
                    Job::Rare => Done::Rare(gamemap::read_rare_today(&map.dir, &map)),
                };
                if done_tx.send(done).is_err() {
                    return;
                }
            }
        });
        self.tx = Some(tx);
        self.rx = Some(rx);
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let mut got = false;
        while let Ok(d) = rx.try_recv() {
            got = true;
            match d {
                Done::Chunk(cx, cy, img) => {
                    self.pending.remove(&(cx, cy));
                    if let Some(img) = img {
                        let tex = ctx.load_texture(format!("map-{cx}-{cy}"), img, TextureOptions::NEAREST);
                        self.chunks.insert((cx, cy), tex);
                        self.chunk_order.push_back((cx, cy));
                        while self.chunk_order.len() > MAX_CHUNKS {
                            if let Some(old) = self.chunk_order.pop_front() {
                                self.chunks.remove(&old);
                            }
                        }
                    }
                }
                Done::Crop(key, crop) => {
                    self.crop_pending.remove(&key);
                    let tex = crop.map(|c| CropTex { tex: ctx.load_texture(format!("crop-{key}"), c.image, TextureOptions::NEAREST) });
                    self.crops.insert(key, tex);
                }
                Done::Rare(r) => self.rare = r,
            }
        }
        if got {
            ctx.request_repaint();
        }
    }

    /// Today's sparkly grass, re-read from the logs every minute.
    pub(super) fn refresh_rare(&mut self) {
        if self.rare_asked.is_some_and(|t| t.elapsed() < Duration::from_secs(60)) {
            return;
        }
        if let Some(tx) = &self.tx {
            self.rare_asked = Some(Instant::now());
            let _ = tx.send(Job::Rare);
        }
    }

    fn want_chunk(&mut self, cx: usize, cy: usize) -> Option<&TextureHandle> {
        if !self.chunks.contains_key(&(cx, cy)) && self.pending.insert((cx, cy)) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(Job::Chunk(cx, cy));
            }
        }
        self.chunks.get(&(cx, cy))
    }

    /// A cut-out (starts making it the first time it's asked for).
    pub(super) fn crop(&mut self, key: &str, matrix: usize, rect: (usize, usize, usize, usize)) -> Option<&CropTex> {
        if !self.crops.contains_key(key) && self.crop_pending.insert(key.to_string()) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(Job::Crop { key: key.to_string(), matrix, rect });
            }
        }
        self.crops.get(key).and_then(|c| c.as_ref())
    }

    pub(super) fn crop_failed(&self, key: &str) -> bool {
        matches!(self.crops.get(key), Some(None))
    }

    pub(super) fn fly_to(&mut self, center: Pos2, zoom: f32) {
        if !self.fitted {
            self.center = center;
            self.zoom = zoom;
            self.fitted = true;
            return;
        }
        self.flying = Some((self.center, self.zoom, center, zoom, Instant::now()));
    }
}

impl App {
    fn map_data(&mut self) -> Option<Arc<GameMap>> {
        while let Ok(r) = self.gamemap_rx.try_recv() {
            self.gamemap = Some(r.map_err(|e| e.to_string()));
        }
        let map = self.gamemap.as_ref()?.as_ref().ok()?.clone();
        self.mapui.attach(&map);
        Some(map)
    }

    /// Keep the map's background work flowing (called every frame).
    pub(super) fn map_tick(&mut self, ctx: &egui::Context) {
        if self.map_data().is_some() {
            self.mapui.poll(ctx);
            // keep today's sparkly grass fresh (re-read at most once a minute)
            self.mapui.refresh_rare();
        }
    }

    /// Open the Map with these wiki places flashing.
    pub(super) fn show_on_map(&mut self, places: &[String], title: String, spots: Option<Spots>) {
        let Some(map) = self.map_data() else {
            self.setup.tab = Tab::Map;
            return;
        };
        let mut zones: Vec<u16> = Vec::new();
        for p in places {
            if let Some(link) = map.place_zones(p) {
                for z in &link.zones {
                    if !zones.contains(z) {
                        zones.push(*z);
                    }
                }
            }
        }
        self.setup.tab = Tab::Map;
        if zones.is_empty() {
            return;
        }
        // Outdoors if any of them are, otherwise the first indoor one.
        let outdoor: Vec<u16> = zones.iter().copied().filter(|z| map.zone(*z).is_some_and(|z| z.matrix == 0)).collect();
        let (area, shown) = if !outdoor.is_empty() { (Area::World, outdoor) } else { (Area::Inside(zones[0]), vec![zones[0]]) };
        self.mapui.area = Some(area);
        self.mapui.selected = shown.first().copied();
        self.mapui.highlight = Some(Highlight { zones: shown.clone(), title, spots, since: Instant::now() });
        self.map_fit(&map, &shown);
    }

    /// Open the Map with one area flashing.
    pub(super) fn show_zone_on_map(&mut self, map: &GameMap, z: u16, title: String) {
        let Some(zone) = map.zone(z) else { return };
        self.setup.tab = Tab::Map;
        self.mapui.area = Some(if zone.matrix == 0 { Area::World } else { Area::Inside(z) });
        self.mapui.selected = Some(z);
        self.mapui.highlight = Some(Highlight { zones: vec![z], title, spots: None, since: Instant::now() });
        self.map_fit(map, &[z]);
    }

    fn map_fit(&mut self, map: &GameMap, zones: &[u16]) {
        let mut cells: Vec<(usize, usize)> = Vec::new();
        for z in zones {
            cells.extend(map.zone_cells(*z));
        }
        let Some((x0, y0, x1, y1)) = bbox(&cells) else { return };
        let (w, h) = (((x1 - x0) * CHUNK) as f32, ((y1 - y0) * CHUNK) as f32);
        let center = Pos2::new((x0 + x1) as f32 * CHUNK as f32 / 2.0, (y0 + y1) as f32 * CHUNK as f32 / 2.0);
        // fit in the map on screen, with room around
        let canvas = if self.mapui.canvas.x > 100.0 { self.mapui.canvas } else { Vec2::new(820.0, 480.0) };
        let zoom = (canvas.x / (w + 24.0)).min(canvas.y / (h + 24.0)).clamp(MIN_ZOOM, 8.0);
        self.mapui.fly_to(center, zoom);
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_map(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "MAP", "Sinnoh, drawn from your game's own map files. Scroll to zoom, drag to move, click an area.");
        kit::experimental(ui, "The Map");
        let map = match self.gamemap.as_ref() {
            None => {
                let _ = self.map_data();
                kit::pill(ui, "Reading the map from your game…", PillKind::Info);
                ui.ctx().request_repaint_after(Duration::from_millis(200));
                return;
            }
            Some(Err(e)) => {
                let e = e.clone();
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Couldn't read the map").size(kit::fs(13.5)).strong().color(TEXT()));
                    ui.label(RichText::new(e).size(kit::fs(12.0)).color(MUTED()));
                    ui.label(
                        RichText::new("The Map uses the map files of your Platinum Together install, so the game needs to be installed on this PC.")
                            .size(kit::fs(12.0))
                            .color(MUTED()),
                    );
                });
                return;
            }
            Some(Ok(_)) => match self.map_data() {
                Some(m) => m,
                None => return,
            },
        };
        let area = *self.mapui.area.get_or_insert(Area::World);
        if self.mapui.show_rare {
            self.mapui.refresh_rare();
        }

        // ── toolbar
        let mut go_zone: Option<u16> = None;
        let mut leave_inside = false;
        let mut clear_hl = false;
        let mut zoom_by: Option<f32> = None;
        let mut fit_all = false;
        let floors: Vec<(u16, String)> = match area {
            Area::Inside(z) => {
                let name = map.zone(z).map(|z| z.name.clone()).unwrap_or_default();
                map.zones.iter().filter(|o| o.name == name && o.matrix != 0).map(|o| (o.id, self.zone_label(&map, o.id))).collect()
            }
            Area::World => Vec::new(),
        };
        let floor_now = match area {
            Area::Inside(z) => self.zone_label(&map, z),
            Area::World => String::new(),
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Area::Inside(z) = area {
                if ui.button("‹  Outdoors").clicked() {
                    leave_inside = true;
                }
                // Other floors / rooms of the same place.
                if floors.len() > 1 {
                    egui::ComboBox::from_id_salt("map-floor").selected_text(floor_now.as_str()).width(240.0).show_ui(ui, |ui| {
                        for (f, label) in &floors {
                            if ui.selectable_label(*f == z, label.as_str()).clicked() {
                                go_zone = Some(*f);
                            }
                        }
                    });
                }
            }
            kit::search_box(ui, &mut self.mapui.search, "Find a route, town, cave…", 220.0);
            if ui.button(" − ").on_hover_text("Zoom out").clicked() {
                zoom_by = Some(1.0 / 1.4);
            }
            if ui.button(" + ").on_hover_text("Zoom in").clicked() {
                zoom_by = Some(1.4);
            }
            if ui.button("Fit").on_hover_text("Show the whole map").clicked() {
                fit_all = true;
            }
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            let left = (gamemap::next_reset(now) - now).max(0);
            kit::checkbox(ui, &mut self.mapui.show_rare, "Sparkly grass today")
                .on_hover_text(format!("From your game log. New patches in {}h {:02}m (midnight, Central European time).", left / 3600, left % 3600 / 60));
            kit::checkbox(ui, &mut self.mapui.labels, "Area names");
            if let Some(h) = &self.mapui.highlight {
                let t = format!("Showing: {}  ✕", h.title);
                if ui.add(egui::Button::new(RichText::new(t).size(kit::fs(12.0)).color(theme::LINK())).frame(false)).on_hover_text("Stop the highlight").clicked() {
                    clear_hl = true;
                }
            }
        });
        // search results
        let q = self.mapui.search.trim().to_lowercase();
        if !q.is_empty() {
            let mut hits: Vec<(String, u16)> = Vec::new();
            let mut seen = HashSet::new();
            for z in &map.zones {
                let label = self.zone_label(&map, z.id);
                if label.to_lowercase().contains(&q) && seen.insert(label.clone()) {
                    hits.push((label, z.id));
                }
            }
            hits.sort_by(|a, b| a.0.cmp(&b.0));
            ui.horizontal_wrapped(|ui| {
                for (label, id) in hits.iter().take(14) {
                    if ui.add(egui::Button::new(RichText::new(label).size(kit::fs(12.0))).small()).clicked() {
                        go_zone = Some(*id);
                    }
                }
                if hits.is_empty() {
                    ui.label(RichText::new("No area by that name.").size(kit::fs(12.0)).color(MUTED()));
                }
            });
        }
        if clear_hl {
            self.mapui.highlight = None;
        }
        if leave_inside {
            let from = if let Area::Inside(z) = area { Some(z) } else { None };
            self.mapui.area = Some(Area::World);
            self.mapui.fitted = true;
            // Jump to the outdoor area with the same name, if any.
            if let Some(name) = from.and_then(|z| map.zone(z)).map(|z| z.name.clone()) {
                if let Some(o) = map.zones.iter().find(|o| o.matrix == 0 && o.name == name) {
                    let id = o.id;
                    self.mapui.selected = Some(id);
                    self.map_fit(&map, &[id]);
                }
            }
        }
        if let Some(z) = go_zone {
            self.mapui.search.clear();
            self.open_zone(&map, z);
        }
        ui.add_space(6.0);

        // ── canvas
        let area = self.mapui.area.unwrap_or(Area::World);
        let h = (ui.ctx().screen_rect().height() - 250.0).max(380.0);
        // Today's sparkly grass beside the map (below it on narrow windows).
        let spots = if self.mapui.show_rare { self.rare_spots_today() } else { Vec::new() };
        let side_w = if !spots.is_empty() && ui.available_width() > 720.0 { 280.0 } else { 0.0 };
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
        let rect = Rect::from_min_max(row.min, Pos2::new(row.max.x - if side_w > 0.0 { side_w + 10.0 } else { 0.0 }, row.max.y));
        let resp = ui.interact(rect, ui.id().with("map-canvas"), Sense::click_and_drag());
        self.mapui.canvas = rect.size();
        if side_w > 0.0 {
            let side = Rect::from_min_max(Pos2::new(rect.max.x + 10.0, row.min.y), row.max);
            #[allow(deprecated)]
            ui.allocate_ui_at_rect(side, |ui| self.rare_panel(ui, &spots, side.height()));
        }
        let (mi, tiles_w, tiles_h) = match area {
            Area::World => (0usize, (map.world().w * CHUNK) as f32, (map.world().h * CHUNK) as f32),
            Area::Inside(z) => {
                let mi = map.zone(z).map(|z| z.matrix as usize).unwrap_or(0);
                let m = &map.matrices[mi];
                (mi, (m.w * CHUNK) as f32, (m.h * CHUNK) as f32)
            }
        };
        if !self.mapui.fitted || fit_all {
            let zoom = (rect.width() / tiles_w).min(rect.height() / tiles_h) * 0.95;
            let (c, z) = if area == Area::World && !fit_all {
                // the explored part of Sinnoh, roughly
                (Pos2::new(480.0, 560.0), (rect.width() / 920.0).min(rect.height() / 760.0))
            } else {
                (Pos2::new(tiles_w / 2.0, tiles_h / 2.0), zoom)
            };
            if fit_all {
                self.mapui.fly_to(c, z.clamp(MIN_ZOOM, MAX_ZOOM));
            } else {
                self.mapui.center = c;
                self.mapui.zoom = z.clamp(MIN_ZOOM, MAX_ZOOM);
                self.mapui.fitted = true;
            }
        }
        // fly animation
        if let Some((c0, z0, c1, z1, t0)) = self.mapui.flying {
            let k = (t0.elapsed().as_secs_f32() / 0.6).min(1.0);
            let e = 1.0 - (1.0 - k).powi(3);
            self.mapui.center = c0 + (c1 - c0) * e;
            self.mapui.zoom = z0 * (z1 / z0).powf(e);
            if k >= 1.0 {
                self.mapui.flying = None;
            }
            ui.ctx().request_repaint();
        }
        // input: drag, scroll / pinch zoom at the pointer
        if resp.dragged() {
            self.mapui.center -= resp.drag_delta() / self.mapui.zoom;
            self.mapui.flying = None;
        }
        let hover_pos = resp.hover_pos();
        if let Some(hp) = hover_pos {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let mut f = pinch;
            if scroll != 0.0 {
                f *= (scroll / 300.0).exp();
            }
            if f != 1.0 {
                ui.ctx().input_mut(|i| {
                    i.smooth_scroll_delta = Vec2::ZERO;
                    i.raw_scroll_delta = Vec2::ZERO;
                });
                let before = self.mapui.center + (hp - rect.center()) / self.mapui.zoom;
                self.mapui.zoom = (self.mapui.zoom * f).clamp(MIN_ZOOM, MAX_ZOOM);
                self.mapui.center = before - (hp - rect.center()) / self.mapui.zoom;
                self.mapui.flying = None;
            }
        }
        if let Some(f) = zoom_by {
            let z = (self.mapui.zoom * f).clamp(MIN_ZOOM, MAX_ZOOM);
            self.mapui.fly_to(self.mapui.center, z);
        }
        let zoom = self.mapui.zoom;
        let center = self.mapui.center;
        let to_screen = |t: Pos2| rect.center() + (t - center) * zoom;
        let to_tile = |s: Pos2| center + (s - rect.center()) / zoom;

        let p = ui.painter_at(rect);
        p.rect_filled(rect, kit::r(10.0), Color32::from_rgb(14, 22, 34));

        // ── the picture
        match area {
            Area::World => {
                if self.mapui.overview.is_none() {
                    if let Some(img) = map.overview.clone() {
                        self.mapui.overview = Some(ui.ctx().load_texture("map-overview", img, TextureOptions::LINEAR));
                    }
                }
                if let Some(ov) = &self.mapui.overview {
                    let full = Rect::from_min_max(to_screen(Pos2::ZERO), to_screen(Pos2::new(tiles_w, tiles_h)));
                    p.image(ov.id(), full, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                if zoom >= DETAIL_ZOOM {
                    let a = to_tile(rect.min);
                    let b = to_tile(rect.max);
                    let c0 = (a.x / CHUNK as f32).floor().max(0.0) as usize;
                    let r0 = (a.y / CHUNK as f32).floor().max(0.0) as usize;
                    let c1 = ((b.x / CHUNK as f32).ceil().max(0.0) as usize).min(map.world().w);
                    let r1 = ((b.y / CHUNK as f32).ceil().max(0.0) as usize).min(map.world().h);
                    for cy in r0..r1 {
                        for cx in c0..c1 {
                            if map.world().land_at(cx, cy).is_none() {
                                continue;
                            }
                            if let Some(t) = self.mapui.want_chunk(cx, cy) {
                                let tl = to_screen(Pos2::new((cx * CHUNK) as f32, (cy * CHUNK) as f32));
                                let r = Rect::from_min_size(tl, Vec2::splat(CHUNK as f32 * zoom));
                                p.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                            }
                        }
                    }
                }
            }
            Area::Inside(_) => {
                if !self.mapui.inside.contains_key(&mi) {
                    if let Some(img) = map.draw_matrix(mi) {
                        let t = ui.ctx().load_texture(format!("inside-{mi}"), img, TextureOptions::NEAREST);
                        self.mapui.inside.insert(mi, t);
                    }
                }
                if let Some(t) = self.mapui.inside.get(&mi) {
                    let full = Rect::from_min_max(to_screen(Pos2::ZERO), to_screen(Pos2::new(tiles_w, tiles_h)));
                    p.image(t.id(), full, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            }
        }

        // ── what's under the pointer
        let hover_tile = hover_pos.map(to_tile);
        let hover_zone = hover_tile.and_then(|t| {
            if t.x < 0.0 || t.y < 0.0 {
                return None;
            }
            let (x, y) = (t.x as usize, t.y as usize);
            match area {
                Area::World => map.world_zone_at(x, y),
                Area::Inside(z) => {
                    let m = &map.matrices[mi];
                    let (cx, cy) = (x / CHUNK, y / CHUNK);
                    m.land_at(cx, cy)?;
                    Some(m.header_at(cx, cy).filter(|h| map.zone(*h).is_some()).unwrap_or(z))
                }
            }
        });

        // ── area outlines and names
        if area == Area::World {
            let faint = Stroke::new(1.0f32, Color32::from_white_alpha(if zoom < 1.0 { 40 } else { 70 }));
            for segs in self.mapui.edges.values() {
                for (a, b) in segs {
                    p.line_segment([to_screen(*a), to_screen(*b)], faint);
                }
            }
        }
        let outline = |z: u16, stroke: Stroke| {
            if area == Area::World {
                if let Some(segs) = self.mapui.edges.get(&z) {
                    for (a, b) in segs {
                        p.line_segment([to_screen(*a), to_screen(*b)], stroke);
                    }
                }
            } else {
                let tiles: Vec<(usize, usize)> = map
                    .zone_cells(z)
                    .into_iter()
                    .map(|(cx, cy)| (cx, cy))
                    .collect();
                for (cx, cy) in tiles {
                    let r = Rect::from_min_size(to_screen(Pos2::new((cx * CHUNK) as f32, (cy * CHUNK) as f32)), Vec2::splat(CHUNK as f32 * zoom));
                    p.rect_stroke(r, 0.0, stroke);
                }
            }
        };
        if let Some(z) = hover_zone {
            outline(z, Stroke::new(1.5f32, Color32::from_white_alpha(170)));
        }
        if let Some(z) = self.mapui.selected {
            outline(z, Stroke::new(2.5f32, theme::LINK()));
        }

        // ── the flashing highlight
        if let Some(hl) = &self.mapui.highlight {
            let k = pulse(hl.since);
            let fill = Color32::from_rgba_unmultiplied(255, 214, 64, (30.0 + 70.0 * k) as u8);
            let line = Color32::from_rgba_unmultiplied(255, 214, 64, (140.0 + 115.0 * k) as u8);
            for &z in &hl.zones {
                if map.zone(z).map(|z| z.matrix as usize) != Some(mi) {
                    continue;
                }
                match hl.spots {
                    Some(spots) if zoom >= 1.2 => {
                        for (x, y) in map.spot_tiles(z, spots) {
                            let r = Rect::from_min_size(to_screen(Pos2::new(x as f32, y as f32)), Vec2::splat(zoom));
                            if rect.intersects(r) {
                                p.rect_filled(r, 0.0, fill);
                            }
                        }
                    }
                    _ => {
                        for (cx, cy) in map.zone_cells(z) {
                            let r = Rect::from_min_size(to_screen(Pos2::new((cx * CHUNK) as f32, (cy * CHUNK) as f32)), Vec2::splat(CHUNK as f32 * zoom));
                            p.rect_filled(r, 0.0, fill);
                        }
                    }
                }
                outline(z, Stroke::new(3.0f32, line));
            }
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }

        // ── names
        if self.mapui.labels && area == Area::World && zoom >= 0.45 {
            let size = (9.0 + zoom * 1.6).clamp(9.5, 15.0);
            let mut done: HashSet<String> = HashSet::new();
            let mut placed: Vec<Rect> = Vec::new();
            // towns and cities first, they matter most
            let mut order: Vec<&gamemap::Zone> = map.zones.iter().collect();
            order.sort_by_key(|z| !(z.code.starts_with('C') || z.code.starts_with('T')));
            for z in order {
                if z.matrix != 0 {
                    continue;
                }
                let cells = map.zone_cells(z.id);
                if cells.is_empty() {
                    continue;
                }
                let label = self.zone_label(&map, z.id);
                // routes split in two get one name each half only when zoomed in
                if zoom < 1.2 && !done.insert(z.name.clone()) {
                    continue;
                }
                let n = cells.len() as f32;
                let cxm = cells.iter().map(|c| c.0 as f32 + 0.5).sum::<f32>() / n;
                let cym = cells.iter().map(|c| c.1 as f32 + 0.5).sum::<f32>() / n;
                let pos = to_screen(Pos2::new(cxm * CHUNK as f32, cym * CHUNK as f32));
                if rect.contains(pos) {
                    let text = if zoom < 1.2 { z.name.clone() } else { label };
                    // skip names that would sit on top of one already drawn
                    let w = p.layout_no_wrap(text.clone(), kit::font_bold(size), Color32::WHITE).size();
                    let r = Rect::from_center_size(pos, w + Vec2::new(6.0, 2.0));
                    if placed.iter().any(|o: &Rect| o.intersects(r)) {
                        continue;
                    }
                    placed.push(r);
                    halo_text(&p, pos, &text, size, Color32::WHITE);
                }
            }
        }

        // ── today's sparkly grass
        let mut rare_tip: Option<String> = None;
        let mut rare_hit: Option<String> = None;
        if self.mapui.show_rare && area == Area::World {
            let rare = self.mapui.rare.clone();
            for r in &rare {
                let c = to_screen(Pos2::new(r.x as f32 + 0.5, r.y as f32 + 0.5));
                if !rect.contains(c) {
                    continue;
                }
                match r.species {
                    None => continue,
                    Some(sp) => {
                        let name = self.rare_name(&map, sp);
                        if zoom >= 2.5 {
                            if let Some(s) = self.rare_species(&map, sp) {
                                let size = (zoom * 3.0).clamp(22.0, 48.0);
                                p.circle_filled(c, size * 0.42, Color32::from_black_alpha(120));
                                p.circle_stroke(c, size * 0.42, Stroke::new(1.5f32, GOLD));
                                self.draw_sprite(&p, &s, false, c + Vec2::new(0.0, size * 0.4), Vec2::splat(size));
                            }
                        } else {
                            continue;
                        }
                        if hover_pos.is_some_and(|h| h.distance(c) < (zoom * 1.5).max(12.0)) {
                            rare_tip = Some(format!("{name} on sparkly grass today\nClick to open it in the Pokédex"));
                            rare_hit = self.rare_wiki_id(&map, sp);
                        }
                    }
                }
            }
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }

        // ── hover / click
        if let Some(tip) = rare_tip {
            resp.clone().on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
        } else if let Some(z) = hover_zone {
            let label = self.zone_label(&map, z);
            let inside = map.zone(z).is_some_and(|z| z.matrix != 0);
            let extra = if area == Area::World && map.zones.iter().any(|o| o.matrix != 0 && map.zone(z).is_some_and(|zz| zz.name == o.name)) {
                "\nDouble-click to go inside"
            } else {
                ""
            };
            let _ = inside;
            resp.clone().on_hover_text(format!("{label}{extra}"));
        }
        if resp.clicked() {
            match rare_hit.take() {
                Some(id) => self.open_dex_entry(&id),
                None => self.mapui.selected = hover_zone,
            }
        }
        if resp.double_clicked() && area == Area::World {
            if let Some(name) = hover_zone.and_then(|z| map.zone(z)).map(|z| z.name.clone()) {
                if let Some(first) = map.zones.iter().find(|o| o.matrix != 0 && o.name == name).map(|o| o.id) {
                    self.open_zone(&map, first);
                }
            }
        }
        p.rect_stroke(rect, kit::r(10.0), Stroke::new(1.0f32, LINE()));

        // ── about the selected area
        ui.add_space(10.0);
        if side_w == 0.0 && !spots.is_empty() {
            self.rare_panel(ui, &spots, 360.0);
            ui.add_space(10.0);
        }
        if let Some(z) = self.mapui.selected {
            self.map_zone_card(ui, &map, z);
        } else {
            ui.label(
                RichText::new("Click an area to see what lives there. Indoor places (caves, forests, towers) are drawn from the game's tile data.")
                    .size(kit::fs(12.0))
                    .color(MUTED()),
            );
        }
    }

    /// Go to an area (inside if it's indoors).
    fn open_zone(&mut self, map: &GameMap, z: u16) {
        let Some(zone) = map.zone(z) else { return };
        self.mapui.selected = Some(z);
        if zone.matrix == 0 {
            self.mapui.area = Some(Area::World);
        } else {
            let same_map = matches!(self.mapui.area, Some(Area::Inside(o)) if map.zone(o).map(|o| o.matrix) == Some(zone.matrix));
            self.mapui.area = Some(Area::Inside(z));
            if !same_map {
                self.mapui.fitted = false;
                return;
            }
        }
        self.map_fit(map, &[z]);
    }

    /// The name to show for an area: the wiki's name when it's one place
    /// (e.g. "Route 210 (north)"), otherwise the game's.
    pub(super) fn zone_label(&self, map: &GameMap, z: u16) -> String {
        let places = map.zone_places(z);
        let exact: Vec<&str> = places.iter().copied().filter(|p| map.places.get(*p).is_some_and(|l| l.exact && l.zones.len() == 1)).collect();
        if let (1, Some(w)) = (exact.len(), self.wiki.as_ref()) {
            if let Some(pl) = w.place(exact[0]) {
                return pl.name.clone();
            }
        }
        map.zone(z).map(|z| z.name.clone()).unwrap_or_else(|| format!("Area {z}"))
    }

    pub(super) fn rare_species(&self, map: &GameMap, sp: u32) -> Option<crate::species::Species> {
        let dex = if sp <= 493 {
            sp
        } else {
            let id = map.game_species.get(&sp)?;
            self.wiki.as_ref()?.mon(id)?.dex
        };
        self.species.get((dex as usize).wrapping_sub(1)).cloned()
    }

    fn rare_name(&self, map: &GameMap, sp: u32) -> String {
        self.rare_species(map, sp).map(|s| s.name.to_string()).unwrap_or_else(|| format!("Pokémon #{sp}"))
    }

    /// The wiki species id for a game species number.
    pub(super) fn rare_wiki_id(&self, map: &GameMap, sp: u32) -> Option<String> {
        if sp <= 493 {
            Some(self.wiki.as_ref()?.by_dex(sp)?.id.clone())
        } else {
            map.game_species.get(&sp).cloned()
        }
    }

    /// Card under the map: the selected area's wild Pokémon.
    fn map_zone_card(&mut self, ui: &mut egui::Ui, map: &GameMap, z: u16) {
        let Some(w) = self.wiki.clone() else { return };
        let label = self.zone_label(map, z);
        let places = map.zone_places(z);
        let mut open: Option<String> = None;
        let mut go_inside: Option<u16> = None;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&label).size(kit::fs(15.0)).strong().color(TEXT()));
                if let Some(zone) = map.zone(z) {
                    if zone.matrix == 0 {
                        if let Some(first) = map.zones.iter().find(|o| o.matrix != 0 && o.name == zone.name).map(|o| o.id) {
                            if ui.button("Go inside").clicked() {
                                go_inside = Some(first);
                            }
                        }
                    }
                }
            });
            if let Some(pl) = places.first().and_then(|p| w.place(p)) {
                if !pl.stage.is_empty() {
                    ui.label(RichText::new(format!("Can get there: {}", pl.stage)).size(kit::fs(11.5)).color(MUTED()));
                }
            }
            ui.add_space(6.0);
            // wild Pokémon living here, by how
            let mut by_method: Vec<(String, Vec<(String, String)>)> = Vec::new();
            for e in &w.data.wild {
                if !places.contains(&e.area.as_str()) || !e.setting.is_empty() {
                    continue;
                }
                let how = super::dex_tabs::method_label(&e.method).to_string();
                let name = w.mon(&e.species).map(|m| m.name.clone()).unwrap_or_else(|| e.species.clone());
                let entry = match by_method.iter_mut().find(|(m, _)| *m == how) {
                    Some(x) => x,
                    None => {
                        by_method.push((how.clone(), Vec::new()));
                        by_method.last_mut().unwrap()
                    }
                };
                if !entry.1.iter().any(|(id, _)| *id == e.species) {
                    entry.1.push((e.species.clone(), name));
                }
            }
            if by_method.is_empty() {
                ui.label(RichText::new("No wild Pokémon here.").size(kit::fs(12.0)).color(MUTED()));
            }
            for (how, mons) in &by_method {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{how}:")).size(kit::fs(12.0)).strong().color(MUTED()));
                    for (id, name) in mons {
                        if ui.link(RichText::new(name).size(kit::fs(12.0))).on_hover_text("Open in the Pokédex").clicked() {
                            open = Some(id.clone());
                        }
                    }
                });
            }
            ui.add_space(2.0);
            ui.label(
                RichText::new("Pokémon only added by MORE POKéMON or MODERN POKéMON aren't listed; see the Pokédex for those.")
                    .size(kit::fs(11.0))
                    .color(MUTED()),
            );
        });
        if let Some(id) = open {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.open = Some(id);
        }
        if let Some(f) = go_inside {
            self.open_zone(map, f);
        }
    }
}
