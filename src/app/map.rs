//! The Map app: Sinnoh from the game's own pictures, zoomable, with every
//! area outlined, today's rare grass from the game log, and a slow
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

/// Which layer of Sinnoh the map shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum Layer {
    #[default]
    Overworld,
    Caves,
    Ruins,
    Side,
}

/// A layer laid out: each part (zone, its map, tile rect) and each place
/// (name, tile rect around its parts).
#[derive(Clone, Default)]
pub(super) struct LayerLayout {
    /// zone, its map, tile rect, how many parts its place has
    pub parts: Vec<(u16, usize, Rect, usize)>,
    pub places: Vec<(String, Rect)>,
    /// Where tile (0, 0) of each map lands on the layer.
    pub origin: HashMap<usize, Vec2>,
    /// The lines between doors, as corner points (laid out ahead of time).
    pub links: Vec<Vec<Pos2>>,
    /// Maps whose doors get a name tag instead of a line (their place's
    /// rooms link up too much to draw lines that don't cross).
    pub tagged: HashSet<usize>,
    /// Laid out ahead of time (lines are drawn from `links`).
    pub planned: bool,
}

/// A layer from its plan in zones.json.
fn planned_layout(plan: &gamemap::LayerPlan, tex: &HashMap<usize, Option<(TextureHandle, [f32; 4])>>) -> LayerLayout {
    let mut out = LayerLayout {
        planned: true,
        ..Default::default()
    };
    for (name, x0, y0, x1, y1, routed) in &plan.places {
        let r = Rect::from_min_max(Pos2::new(*x0 as f32, *y0 as f32), Pos2::new(*x1 as f32, *y1 as f32));
        out.places.push((name.clone(), r));
        if !routed {
            for (m, _, x, y, _, _, w, h) in &plan.parts {
                let c = Pos2::new((*x + *w / 2) as f32, (*y + *h / 2) as f32);
                if r.expand(1.0).contains(c) {
                    out.tagged.insert(*m as usize);
                }
            }
        }
    }
    for (m, zone, x, y, cx, cy, w, h) in &plan.parts {
        let m = *m as usize;
        let origin = Vec2::new((*x - *cx) as f32, (*y - *cy) as f32);
        out.origin.insert(m, origin);
        let Some(Some((_, r))) = tex.get(&m) else { continue };
        // How many parts share its place (for the room names).
        let c = Pos2::new((*x + *w / 2) as f32, (*y + *h / 2) as f32);
        let n = out
            .places
            .iter()
            .find(|(_, pr)| pr.expand(1.0).contains(c))
            .map(|(_, pr)| {
                plan.parts
                    .iter()
                    .filter(|p| pr.expand(1.0).contains(Pos2::new((p.2 + p.6 / 2) as f32, (p.3 + p.7 / 2) as f32)))
                    .count()
            })
            .unwrap_or(1);
        let img = Rect::from_min_size(Pos2::new(origin.x + r[0], origin.y + r[1]), Vec2::new(r[2], r[3]));
        out.parts.push((*zone, m, img, n));
    }
    for l in &plan.links {
        let pts: Vec<Pos2> = l.chunks(2).filter(|c| c.len() == 2).map(|c| Pos2::new(c[0] as f32 + 0.5, c[1] as f32 + 0.5)).collect();
        if pts.len() >= 2 {
            out.links.push(pts);
        }
    }
    out
}

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
    /// One tile to ping (a rare grass spot) instead of lighting up the
    /// whole area.
    pub ping: Option<(u16, Pos2)>,
}

/// What to frame on a layer once it's laid out.
struct Focus {
    zones: Vec<u16>,
    /// Start from a fresh view rather than fly from where the map was.
    fresh: bool,
    /// A tile to zoom in on instead (map, tile), e.g. a rare grass spot.
    tile: Option<(u16, Pos2)>,
}

/// The door under the pointer: what to say about it and where it goes.
struct DoorHit {
    tip: String,
    door: gamemap::Door,
    /// Its other end, when that's on the map showing (tile position).
    dest: Option<Pos2>,
}

/// One area's tiles (on its own map).
pub(super) fn zones_rect_pub(map: &GameMap, z: u16) -> Option<Rect> {
    zones_rect(map, &[z])
}

pub(super) fn float_shadow_pub() -> egui::epaint::Shadow {
    float_shadow()
}

/// The tiles a set of areas covers (on their own map).
fn zones_rect(map: &GameMap, zones: &[u16]) -> Option<Rect> {
    let mut cells: Vec<(usize, usize)> = Vec::new();
    for z in zones {
        cells.extend(map.zone_cells(*z));
    }
    let (x0, y0, x1, y1) = bbox(&cells)?;
    Some(Rect::from_min_max(
        Pos2::new((x0 * CHUNK) as f32, (y0 * CHUNK) as f32),
        Pos2::new((x1 * CHUNK) as f32, (y1 * CHUNK) as f32),
    ))
}

/// The layer a cave, ruin or side area is drawn on.
fn layer_for(kind: &str) -> Layer {
    match kind {
        "cave" => Layer::Caves,
        "ruins" => Layer::Ruins,
        _ => Layer::Side,
    }
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
    /// What the key shows: ways in, doors, item balls, hidden items.
    pub show_entrances: bool,
    pub show_doors: bool,
    pub show_items: bool,
    pub show_hidden: bool,
    /// The map key popover is open.
    pub key_open: bool,
    /// The side panel is hidden (the map gets the room).
    pub panel_closed: bool,
    pub panel_tab: super::map_panel::MapTab,
    /// Filter for the rare grass list.
    pub rare_query: String,
    /// The panel asked for the infographic / the shared list file.
    pub want_poster: bool,
    pub want_export: bool,
    /// "Fit" was pressed (done on the next frame, before drawing).
    pub want_fit: bool,
    /// Caves or side areas laid over the Sinnoh map.
    pub layer: Layer,
    /// Their pictures, by map, with the part of the map each covers (tiles).
    overlay_tex: HashMap<usize, Option<(TextureHandle, [f32; 4])>>,
    /// Where each piece of a layer sits (zone, its map, tile rect), worked
    /// out once: near its real place, nudged apart so none overlap.
    layer_layout: HashMap<Layer, LayerLayout>,
    /// Frame the layer just switched to.
    layer_fit: bool,
    /// Areas to frame on the layer once it's laid out (and whether to start
    /// from a fresh view rather than fly from where the map was).
    focus: Option<Focus>,
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
    /// Result of saving the shared rare grass file.
    export_msg: Option<String>,
    /// The rare grass infographic window, while it's open.
    pub poster: Option<super::rare_poster::PosterView>,
    rare_asked: Option<Instant>,
    /// Outline pieces of each outdoor area (tile coordinates).
    edges: HashMap<u16, Vec<(Pos2, Pos2)>>,
    tx: Option<Sender<Job>>,
    rx: Option<Receiver<Done>>,
    started: bool,
}

/// The look of everything that floats over the map: a Cirrus Cloud card
/// with a hairline and a soft shadow (a touch stronger than on a page, so
/// it lifts off busy map pictures).
fn float_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(kit::CARD())
        .rounding(20.0)
        .stroke(Stroke::new(1.0f32, LINE()))
        .inner_margin(egui::Margin::same(12.0))
        .shadow(float_shadow())
}

/// A one-line float (a hint, "Showing: …", a folded panel): a full pill.
fn float_pill() -> egui::Frame {
    float_frame().rounding(40.0).inner_margin(egui::Margin::symmetric(14.0, 6.0))
}

fn float_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: Vec2::new(0.0, 8.0),
        blur: 22.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if theme::pal().light { 40 } else { 110 }),
    }
}

/// Height of the controls along the top of the map (the search box's).
const TOP_H: f32 = kit::CONTROL_H + 4.0;

/// The Map's layer tabs: a Cirrus pill track (Cloud, hairline, soft
/// shadow) as tall as the search box; the layer showing is a pill in the
/// button colour.
fn map_tabs<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    const INSET: f32 = 4.0;
    const PAD: f32 = 14.0;
    let font = kit::font_bold(13.0);
    let widths: Vec<f32> = options.iter().map(|(_, l)| ui.painter().layout_no_wrap(l.to_string(), font.clone(), TEXT()).size().x + 2.0 * PAD).collect();
    let total = widths.iter().sum::<f32>() + 2.0 * INSET + 2.0 * (options.len().saturating_sub(1)) as f32;
    let (track, _) = ui.allocate_exact_size(Vec2::new(total, TOP_H), Sense::hover());
    let rad = track.height() / 2.0;
    ui.painter().add(float_shadow().as_shape(track, rad));
    ui.painter().rect(track, rad, kit::CARD(), Stroke::new(1.0f32, LINE()));
    let mut changed = false;
    let mut x = track.left() + INSET;
    for ((v, label), w) in options.iter().zip(widths) {
        let r = Rect::from_min_size(Pos2::new(x, track.top() + INSET), Vec2::new(w, TOP_H - 2.0 * INSET));
        x += w + 2.0;
        let resp = ui.interact(r, ui.id().with(("map-layer-tab", *label)), Sense::click());
        let selected = *value == *v;
        let p = ui.painter();
        if selected {
            p.add(kit::primary_shape(r, false));
        } else if resp.hovered() {
            p.rect_filled(r, r.height() / 2.0, kit::sunken());
        }
        let col = if selected { kit::on_primary() } else if resp.hovered() { TEXT() } else { MUTED() };
        let galley = p.layout_no_wrap(label.to_string(), font.clone(), col);
        p.galley(r.center() - galley.size() / 2.0, galley, col);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
            *value = *v;
            changed = true;
        }
    }
    changed
}

/// A pill button in the command bar: an icon drawn on the left, then its
/// label (on = pressed in).
fn bar_button(ui: &mut egui::Ui, label: &str, on: bool, tip: &str, icon: impl FnOnce(&egui::Painter, Rect, Color32)) -> bool {
    let font = kit::font_bold(13.0);
    let g = ui.painter().layout_no_wrap(label.to_string(), font, TEXT());
    let w = if label.is_empty() { TOP_H } else { g.size().x + 44.0 };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, TOP_H), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    let rad = r.height() / 2.0;
    p.add(float_shadow().as_shape(r, rad));
    if on {
        p.add(kit::primary_shape(r, false));
    } else {
        p.rect_filled(r, rad, theme::blend(kit::CARD(), kit::CARD_HOVER(), t));
        p.rect_stroke(r, rad, Stroke::new(1.0f32, LINE()));
    }
    let ink = if on { kit::on_primary() } else { theme::blend(MUTED(), TEXT(), t.max(0.6)) };
    let ir = Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(18.0));
    icon(p, ir, ink);
    if !label.is_empty() {
        p.galley(Pos2::new(r.left() + 34.0, r.center().y - g.size().y / 2.0), g, ink);
    }
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// A command bar pill led by a sprite (your target).
fn bar_sprite_button(ui: &mut egui::Ui, label: &str, tip: &str, sprite: impl FnOnce(&egui::Painter, Rect)) -> bool {
    let font = kit::font_bold(13.0);
    let g = ui.painter().layout_no_wrap(label.to_string(), font, TEXT());
    let w = if label.is_empty() { TOP_H + 6.0 } else { g.size().x + 50.0 };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, TOP_H), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    let rad = r.height() / 2.0;
    p.add(float_shadow().as_shape(r, rad));
    p.rect_filled(r, rad, theme::blend(kit::CARD(), kit::CARD_HOVER(), t));
    p.rect_stroke(r, rad, Stroke::new(1.0f32, theme::blend(LINE(), GOLD, 0.5 + 0.5 * t)));
    let sr = Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(28.0));
    p.circle_filled(sr.center(), 13.0, theme::blend(kit::CARD_HOVER(), GOLD, 0.18));
    sprite(p, sr);
    if !label.is_empty() {
        p.galley(Pos2::new(r.left() + 40.0, r.center().y - g.size().y / 2.0), g, TEXT());
    }
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn result_head(ui: &mut egui::Ui, t: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(t).font(kit::font_bold(10.5)).color(MUTED()));
    ui.add_space(2.0);
}

/// A search result: an icon, its name and a note on the right.
fn result_row(ui: &mut egui::Ui, label: &str, note: &str, icon: impl FnOnce(&egui::Painter, Rect)) -> bool {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if t > 0.0 {
        p.rect_filled(r, 10.0, theme::blend(kit::CARD(), kit::CARD_HOVER(), t));
    }
    icon(p, Rect::from_center_size(Pos2::new(r.left() + 18.0, r.center().y), Vec2::splat(28.0)));
    let ng = p.layout_no_wrap(note.to_string(), kit::font(11.5), MUTED());
    let nx = r.right() - 8.0 - ng.size().x;
    p.galley(Pos2::new(nx, r.center().y - ng.size().y / 2.0), ng, MUTED());
    let g = p.layout_no_wrap(label.to_string(), kit::font_bold(12.5), TEXT());
    p.with_clip_rect(Rect::from_min_max(r.min, Pos2::new(nx - 6.0, r.max.y))).galley(Pos2::new(r.left() + 40.0, r.center().y - g.size().y / 2.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
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

/// Write today's rare grass as the shared list file. Goes into the
/// GitHub copy of the app (…\GitHub\Shinytracker\data) when there is one,
/// otherwise Downloads. Returns a line saying where it went.
fn export_shared_rare(rare: &[Rare]) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let reset = gamemap::last_reset(now);
    let mut spots: Vec<(usize, usize, u32)> = rare.iter().filter_map(|r| Some((r.x, r.y, r.species?))).collect();
    spots.sort();
    spots.dedup();
    let body = serde_json::json!({
        "about": "Rare grass Pokémon seen today, shared with everyone. 'reset' is the daily reset (midnight Central European time) these are for.",
        "reset": reset,
        "spots": spots,
    });
    let repo = dirs::document_dir().map(|d| d.join("GitHub").join("Shinytracker"));
    let dir = match repo.filter(|r| r.join(".git").exists()) {
        Some(r) => r.join("data"),
        None => dirs::download_dir().unwrap_or_default(),
    };
    let path = dir.join("rare-today.json");
    let text = serde_json::to_string_pretty(&body).unwrap_or_default();
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
        Ok(()) => format!("Saved {} spots to {}", spots.len(), path.display()),
        Err(e) => format!("Couldn't save: {e}"),
    }
}

/// Name of a cave or side area on its layer.
fn layer_name(z: &gamemap::Zone) -> &str {
    if z.code.starts_with("R209R") {
        "Lost Tower"
    } else {
        &z.name
    }
}

/// A door link between two parts of a place: which parts, and where the
/// door sits in each (tiles from the part's top left).
struct Link {
    a: usize,
    b: usize,
    pa: Vec2,
    pb: Vec2,
}

/// Do two segments cross (not just touch at an end)?
fn segments_cross(p1: Pos2, p2: Pos2, q1: Pos2, q2: Pos2) -> bool {
    let d = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let (d1, d2, d3, d4) = (d(q1, q2, p1), d(q1, q2, p2), d(p1, p2, q1), d(p1, p2, q2));
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// Does a segment pass through a rect?
fn segment_hits_rect(p: Pos2, q: Pos2, r: Rect) -> bool {
    if r.contains(p) || r.contains(q) {
        return true;
    }
    let c = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
    (0..4).any(|i| segments_cross(p, q, c[i], c[(i + 1) % 4]))
}

/// Lay out the parts (floors, rooms) of one place so the dotted lines
/// between their doors cross as little as possible: parts go in columns
/// (or rows) by how many doors in from the entrance they are, the order
/// within each column is tuned to untangle the lines, and the tidier of
/// the two directions is kept. Returns each part's rect (block top left
/// at 0, 0).
fn arrange_place(
    parts: &[(u16, usize, Vec2, Option<Rect>, bool)],
    tex: &HashMap<usize, Option<(TextureHandle, [f32; 4])>>,
    doors: &[gamemap::Door],
    label: f32,
) -> Vec<(u16, usize, Rect)> {
    let n = parts.len();
    let idx = |m: u16| parts.iter().position(|p| p.1 == m as usize);
    let crop = |m: usize| tex.get(&m).and_then(|t| t.as_ref()).map(|(_, r)| *r).unwrap_or([0.0; 4]);
    // The links between parts, each pair of doors once.
    let mut links: Vec<Link> = Vec::new();
    for d in doors {
        let (Some(a), Some(b)) = (idx(d.matrix), idx(d.to_matrix)) else { continue };
        if a == b {
            continue;
        }
        let (x0, y0, x1, y1) = d.tiles();
        let ra = crop(parts[a].1);
        let pa = Vec2::new((x0 + x1) / 2.0 - ra[0], (y0 + y1) / 2.0 - ra[1]);
        let rb = crop(parts[b].1);
        let back = doors
            .iter()
            .find(|o| o.matrix == d.to_matrix && (o.x..=o.x2).contains(&d.tx) && (o.y..=o.y2).contains(&d.ty));
        let pb = match back {
            Some(o) => {
                let (bx0, by0, bx1, by1) = o.tiles();
                Vec2::new((bx0 + bx1) / 2.0 - rb[0], (by0 + by1) / 2.0 - rb[1])
            }
            None => Vec2::new(d.tx as f32 + 0.5 - rb[0], d.ty as f32 + 0.5 - rb[1]),
        };
        let dup = links
            .iter()
            .any(|l| (l.a == b && l.b == a && (l.pa - pb).length() < 1.5 && (l.pb - pa).length() < 1.5) || (l.a == a && l.b == b && (l.pa - pa).length() < 1.5));
        if !dup {
            links.push(Link { a, b, pa, pb });
        }
    }
    if n == 1 {
        return vec![(parts[0].0, parts[0].1, Rect::from_min_size(Pos2::new(0.0, label), parts[0].2))];
    }
    // How many doors in from the way in each part is.
    let mut level = vec![usize::MAX; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..n {
        if parts[i].4 {
            level[i] = 0;
            queue.push_back(i);
        }
    }
    loop {
        while let Some(i) = queue.pop_front() {
            for l in &links {
                let j = if l.a == i {
                    l.b
                } else if l.b == i {
                    l.a
                } else {
                    continue;
                };
                if level[j] == usize::MAX {
                    level[j] = level[i] + 1;
                    queue.push_back(j);
                }
            }
        }
        // Parts no door reaches from the way in start a new chain.
        match (0..n).find(|&i| level[i] == usize::MAX) {
            Some(i) => {
                level[i] = 0;
                queue.push_back(i);
            }
            None => break,
        }
    }
    let depth = level.iter().copied().max().unwrap_or(0) + 1;
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for i in 0..n {
        cols[level[i]].push(i);
    }
    cols.retain(|c| !c.is_empty());
    // Where everything goes for an order, flowing across (columns) or down (rows).
    const ACROSS: f32 = 22.0; // room for lines between columns/rows
    const ALONG: f32 = 8.0; // between parts in one column/row
    let place = |cols: &Vec<Vec<usize>>, across: bool| -> Vec<Rect> {
        let mut out = vec![Rect::NOTHING; n];
        let breadth =
            |c: &Vec<usize>| -> f32 { c.iter().map(|&i| if across { parts[i].2.y + label } else { parts[i].2.x }).sum::<f32>() + ALONG * (c.len().saturating_sub(1)) as f32 };
        let widest = cols.iter().map(breadth).fold(0.0f32, f32::max);
        let mut pos = 0.0f32;
        for c in cols {
            let thick = c.iter().map(|&i| if across { parts[i].2.x } else { parts[i].2.y + label }).fold(0.0f32, f32::max);
            let mut at = ((widest - breadth(c)) / 2.0).round();
            for &i in c {
                let s = parts[i].2;
                out[i] = if across {
                    let x = pos + ((thick - s.x) / 2.0).round();
                    Rect::from_min_size(Pos2::new(x, at + label), s)
                } else {
                    let y = pos + ((thick - s.y - label) / 2.0).round();
                    Rect::from_min_size(Pos2::new(at, y + label), s)
                };
                at += if across { s.y + label } else { s.x } + ALONG;
            }
            pos += thick + ACROSS;
        }
        out
    };
    // Untidiness: lines crossing each other, lines over other parts, length.
    let cost = |rects: &[Rect]| -> f32 {
        let segs: Vec<(Pos2, Pos2, usize, usize)> = links.iter().map(|l| (rects[l.a].min + l.pa, rects[l.b].min + l.pb, l.a, l.b)).collect();
        let mut c = 0.0f32;
        for (i, s) in segs.iter().enumerate() {
            c += s.0.distance(s.1) * 0.02;
            for t in &segs[i + 1..] {
                if segments_cross(s.0, s.1, t.0, t.1) {
                    c += 10.0;
                }
            }
            for (k, r) in rects.iter().enumerate() {
                if k != s.2 && k != s.3 && segment_hits_rect(s.0, s.1, r.shrink(1.0)) {
                    c += 14.0;
                }
            }
        }
        // Keep the block from getting long and thin.
        let mut bb = rects[0];
        for r in rects {
            bb = bb.union(*r);
        }
        let aspect = (bb.width() / bb.height().max(1.0)).max(0.1);
        c + (aspect / 1.4).ln().abs() * 6.0
    };
    let mut best: Option<(f32, Vec<Rect>)> = None;
    for across in [true, false] {
        let mut order = cols.clone();
        // Start each column in the order of its neighbours in the one before.
        for k in 1..order.len() {
            let prev = order[k - 1].clone();
            let key = |i: usize| -> f32 {
                let ps: Vec<f32> = links
                    .iter()
                    .filter_map(|l| {
                        if l.a == i {
                            Some(l.b)
                        } else if l.b == i {
                            Some(l.a)
                        } else {
                            None
                        }
                    })
                    .filter_map(|j| prev.iter().position(|&p| p == j).map(|p| p as f32))
                    .collect();
                if ps.is_empty() {
                    f32::MAX
                } else {
                    ps.iter().sum::<f32>() / ps.len() as f32
                }
            };
            order[k].sort_by(|&x, &y| key(x).partial_cmp(&key(y)).unwrap_or(std::cmp::Ordering::Equal));
        }
        let mut now = cost(&place(&order, across));
        // Then swap parts within a column, or move one to the next column,
        // while that untangles things.
        for _ in 0..40 {
            let mut better = false;
            for k in 0..order.len() {
                for i in 0..order[k].len() {
                    for j in i + 1..order[k].len() {
                        order[k].swap(i, j);
                        let c = cost(&place(&order, across));
                        if c + 0.01 < now {
                            now = c;
                            better = true;
                        } else {
                            order[k].swap(i, j);
                        }
                    }
                }
            }
            for k in 0..order.len() {
                for i in 0..order[k].len() {
                    for to in [k.wrapping_sub(1), k + 1] {
                        if to >= order.len() || order[k].len() < 2 || i >= order[k].len() {
                            continue;
                        }
                        for at in 0..=order[to].len() {
                            let mut trial = order.clone();
                            let p = trial[k].remove(i);
                            trial[to].insert(at, p);
                            let c = cost(&place(&trial, across));
                            if c + 0.01 < now {
                                now = c;
                                order = trial;
                                better = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !better {
                break;
            }
        }
        let rects = place(&order, across);
        if best.as_ref().map_or(true, |b| now < b.0) {
            best = Some((now, rects));
        }
    }
    let rects = best.map(|b| b.1).unwrap_or_default();
    parts.iter().zip(rects).map(|(p, r)| (p.0, p.1, r)).collect()
}

/// Lay out a layer: the parts of each place (floors, rooms) packed together
/// in a tidy block, each block centred where the place really is in Sinnoh,
/// then blocks nudged apart until none overlap.
fn layer_layout(pieces: &[gamemap::Overlay], tex: &HashMap<usize, Option<(TextureHandle, [f32; 4])>>, doors: &[gamemap::Door]) -> LayerLayout {
    // place → parts (zone, map, size, real rect if known, entry)
    let mut groups: Vec<(String, Vec<(u16, usize, Vec2, Option<Rect>, bool)>)> = Vec::new();
    for o in pieces {
        let m = o.matrix as usize;
        let Some(Some((_, r))) = tex.get(&m) else { continue };
        let size = Vec2::new(r[2], r[3]);
        let real = match (o.x, o.y) {
            (Some(x), Some(y)) => Some(Rect::from_min_size(Pos2::new(x as f32 + r[0], y as f32 + r[1]), size)),
            _ => None,
        };
        let g = if o.group.is_empty() { format!("{}", o.zone) } else { o.group.clone() };
        match groups.iter_mut().find(|(n, _)| *n == g) {
            Some((_, v)) => v.push((o.zone, m, size, real, o.entry)),
            None => groups.push((g, vec![(o.zone, m, size, real, o.entry)])),
        }
    }
    const LABEL: f32 = 12.0; // room for a part's name above it
    let mut blocks: Vec<(String, Rect, Vec<(u16, usize, Rect)>)> = Vec::new();
    for (name, mut parts) in groups {
        // Where the place is: the middle of the parts you walk into.
        let entries: Vec<Pos2> = parts.iter().filter(|p| p.4).filter_map(|p| p.3.map(|r| r.center())).collect();
        let known: Vec<Pos2> = parts.iter().filter_map(|p| p.3.map(|r| r.center())).collect();
        let pts = if entries.is_empty() { known } else { entries };
        if pts.is_empty() {
            continue;
        }
        let anchor = pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32;
        parts.sort_by_key(|p| p.0);
        let label = if parts.len() > 1 { LABEL } else { 0.0 };
        let placed = arrange_place(&parts, tex, doors, label);
        let mut bb = placed[0].2;
        for p in &placed {
            bb = bb.union(p.2);
        }
        // Centre the block on the place, on whole tiles.
        let shift = (anchor - bb.center().to_vec2()).round();
        let placed: Vec<(u16, usize, Rect)> = placed.into_iter().map(|(z, m, r)| (z, m, r.translate(shift))).collect();
        let bb = Rect::from_min_max(bb.min - Vec2::new(0.0, label), bb.max).translate(shift);
        blocks.push((name, bb, placed));
    }
    // Keep blocks apart (with room for the place name above each).
    let pad = |r: Rect| Rect::from_min_max(r.min - Vec2::new(14.0, 30.0), r.max + Vec2::new(14.0, 14.0));
    let n = blocks.len();
    for _ in 0..600 {
        let mut moved = false;
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (pad(blocks[i].1), pad(blocks[j].1));
                if !a.intersects(b) {
                    continue;
                }
                let ox = a.max.x.min(b.max.x) - a.min.x.max(b.min.x);
                let oy = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);
                let d = b.center() - a.center();
                let push = if ox < oy {
                    Vec2::new(if d.x >= 0.0 { ox + 1.0 } else { -ox - 1.0 }, 0.0)
                } else {
                    Vec2::new(0.0, if d.y >= 0.0 { oy + 1.0 } else { -oy - 1.0 })
                } * 0.5;
                for (k, dir) in [(i, -1.0f32), (j, 1.0)] {
                    blocks[k].1 = blocks[k].1.translate(push * dir);
                    for p in blocks[k].2.iter_mut() {
                        p.2 = p.2.translate(push * dir);
                    }
                }
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let mut out = LayerLayout::default();
    for (name, r, parts) in blocks {
        // Back onto whole tiles after the nudging.
        let snap = r.min.round() - r.min;
        let r = r.translate(snap);
        let parts: Vec<(u16, usize, Rect)> = parts.into_iter().map(|(z, m, pr)| (z, m, pr.translate(snap))).collect();
        out.places.push((name, r));
        let n = parts.len();
        for (_, m, pr) in &parts {
            if let Some(Some((_, c))) = tex.get(m) {
                out.origin.insert(*m, pr.min.to_vec2() - Vec2::new(c[0], c[1]));
            }
        }
        out.parts.extend(parts.into_iter().map(|(z, m, r)| (z, m, r, n)));
    }
    out
}

/// The icon for a way into a cave, ruin or side area: a coloured disc with
/// a little doorway.
fn entrance_icon(p: &egui::Painter, c: Pos2, r0: f32, col: Color32, hot: bool) {
    p.circle_filled(c + Vec2::new(0.0, 1.5), r0 + 1.0, Color32::from_black_alpha(120));
    p.circle_filled(c, r0, col);
    p.circle_stroke(c, r0, Stroke::new(if hot { 2.5f32 } else { 1.5 }, Color32::WHITE));
    let w = r0 * 0.5;
    let st = Stroke::new((r0 * 0.18).max(1.3), Color32::from_rgb(30, 26, 24));
    p.line_segment([c + Vec2::new(-w, w), c + Vec2::new(-w, -w * 0.3)], st);
    p.line_segment([c + Vec2::new(w, w), c + Vec2::new(w, -w * 0.3)], st);
    p.line_segment([c + Vec2::new(-w, -w * 0.3), c + Vec2::new(0.0, -w)], st);
    p.line_segment([c + Vec2::new(w, -w * 0.3), c + Vec2::new(0.0, -w)], st);
}

/// An item ball (hidden: faded, with a dashed ring).
pub(super) fn item_icon(p: &egui::Painter, c: Pos2, r: f32, hidden: bool) {
    let a = if hidden { 0.55 } else { 1.0 };
    let red = Color32::from_rgb(226, 62, 62).gamma_multiply(a);
    let white = Color32::from_rgb(240, 240, 240).gamma_multiply(a);
    let dark = Color32::from_rgb(30, 30, 34).gamma_multiply(a.max(0.8));
    p.circle_filled(c, r + 1.0, Color32::from_black_alpha(if hidden { 70 } else { 140 }));
    p.circle_filled(c, r, white);
    // top half red
    let n = 12;
    let mut top: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = std::f32::consts::PI * (1.0 + i as f32 / n as f32);
            c + Vec2::new(t.cos(), t.sin()) * r
        })
        .collect();
    top.push(c + Vec2::new(r, 0.0));
    p.add(egui::Shape::convex_polygon(top, red, Stroke::NONE));
    p.line_segment([c - Vec2::new(r, 0.0), c + Vec2::new(r, 0.0)], Stroke::new((r * 0.22).max(1.0), dark));
    p.circle_filled(c, r * 0.32, white);
    p.circle_stroke(c, r * 0.32, Stroke::new((r * 0.16).max(1.0), dark));
    if hidden {
        // dashed ring: "it's in the ground here"
        for i in 0..8 {
            let t0 = std::f32::consts::TAU * i as f32 / 8.0;
            let t1 = t0 + std::f32::consts::TAU / 16.0;
            p.line_segment(
                [c + Vec2::new(t0.cos(), t0.sin()) * (r + 3.0), c + Vec2::new(t1.cos(), t1.sin()) * (r + 3.0)],
                Stroke::new(1.2f32, Color32::from_white_alpha(200)),
            );
        }
    }
}

/// A dotted line along screen points, the same gap between dots all the
/// way (round the corners too).
fn dotted_path(p: &egui::Painter, pts: &[Pos2], clip: Rect) {
    const GAP: f32 = 9.0;
    const LINK: Color32 = Color32::from_rgb(120, 205, 255);
    if pts.len() < 2 {
        return;
    }
    let mut bb = Rect::from_two_pos(pts[0], pts[1]);
    for q in pts {
        bb.extend_with(*q);
    }
    if !clip.intersects(bb.expand(4.0)) {
        return;
    }
    let lens: Vec<f32> = pts.windows(2).map(|w| w[0].distance(w[1])).collect();
    let total: f32 = lens.iter().sum();
    let n = (total / GAP).floor() as i32;
    if n < 2 {
        return;
    }
    let step = total / n as f32;
    let (mut seg, mut start) = (0usize, 0.0f32);
    for i in 1..n {
        let want = step * i as f32;
        // move on to the segment this dot falls on
        while seg < lens.len() && start + lens[seg] < want {
            start += lens[seg];
            seg += 1;
        }
        if seg >= lens.len() {
            break;
        }
        let (a, b) = (pts[seg], pts[seg + 1]);
        let c = a + (b - a) * ((want - start) / lens[seg].max(0.001));
        p.circle_filled(c, 2.6, Color32::from_black_alpha(140));
        p.circle_filled(c, 1.7, Color32::from_rgba_unmultiplied(LINK.r(), LINK.g(), LINK.b(), 230));
    }
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
        self.show_entrances = true;
        self.show_doors = true;
        self.show_items = true;
        self.show_hidden = true;
        // The side panel starts folded away (the pill at the top right, or
        // clicking a place, opens it).
        self.panel_closed = true;
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
                    let tex = crop.map(|c| CropTex {
                        tex: ctx.load_texture(format!("crop-{key}"), c.image, TextureOptions::NEAREST),
                    });
                    self.crops.insert(key, tex);
                }
                Done::Rare(r) => self.rare = r,
            }
        }
        if got {
            ctx.request_repaint();
        }
    }

    /// Today's rare grass, re-read from the logs every minute.
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
                let _ = tx.send(Job::Crop {
                    key: key.to_string(),
                    matrix,
                    rect,
                });
            }
        }
        self.crops.get(key).and_then(|c| c.as_ref())
    }

    pub(super) fn crop_failed(&self, key: &str) -> bool {
        matches!(self.crops.get(key), Some(None))
    }

    pub(super) fn export_msg(&self) -> Option<String> {
        self.export_msg.clone()
    }

    pub(super) fn overview_tex(&self) -> Option<egui::TextureId> {
        self.overview.as_ref().map(|t| t.id())
    }

    pub(super) fn stop_flying(&mut self) {
        self.flying = None;
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

    /// Glide in to a spot. `fresh`: the map was showing somewhere else (or
    /// nothing yet), so start zoomed out over the spot rather than flying
    /// across from an unrelated view.
    pub(super) fn fly_in(&mut self, center: Pos2, zoom: f32, fresh: bool) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if fresh || !self.fitted {
            self.center = center;
            self.zoom = (zoom / 2.4).clamp(MIN_ZOOM, MAX_ZOOM);
            self.fitted = true;
        }
        self.flying = Some((self.center, self.zoom, center, zoom, Instant::now()));
    }

    /// Glide to show a tile rect with room around it, no closer than
    /// `max_zoom`. `closer_only`: never zoom out to do it (just move).
    pub(super) fn fly_fit(&mut self, tr: Rect, max_zoom: f32, closer_only: bool, fresh: bool) {
        let canvas = if self.canvas.x > 100.0 { self.canvas } else { Vec2::new(820.0, 480.0) };
        let mut z = (canvas.x / (tr.width() + 10.0)).min(canvas.y / (tr.height() + 10.0)).clamp(MIN_ZOOM, max_zoom);
        if closer_only && self.fitted && !fresh {
            z = z.max(self.zoom);
        }
        self.fly_in(tr.center(), z, fresh);
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
            // keep today's rare grass fresh (re-read at most once a minute)
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
        let shown = self.goto_zones(&map, &zones);
        self.mapui.highlight = Some(Highlight {
            zones: shown,
            title,
            spots,
            since: Instant::now(),
            ping: None,
        });
    }

    /// Open the Map at a rare grass tile, with a ring pinging on it.
    pub(super) fn show_rare_spot(&mut self, map: &GameMap, matrix: u16, x: usize, y: usize, title: String) {
        self.setup.tab = Tab::Map;
        self.mapui.show_rare = true;
        let at = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
        let z = map.rare_zone(matrix, x, y);
        if matrix == 0 {
            let fresh = self.mapui.area != Some(Area::World) || self.mapui.layer != Layer::Overworld;
            self.to_overworld();
            self.mapui.fly_in(at, MAX_ZOOM, fresh);
        } else {
            // A lake, forest…: its layer, zoomed in on the tile.
            self.goto_zones(map, &z.into_iter().collect::<Vec<_>>());
            if let Some(f) = self.mapui.focus.as_mut() {
                f.tile = Some((matrix, at));
            }
        }
        self.mapui.selected = z;
        self.mapui.highlight = Some(Highlight {
            zones: z.into_iter().collect(),
            title,
            spots: None,
            since: Instant::now(),
            ping: Some((matrix, at)),
        });
    }

    fn to_overworld(&mut self) {
        self.mapui.area = Some(Area::World);
        self.mapui.layer = Layer::Overworld;
        self.mapui.layer_fit = false;
        self.mapui.focus = None;
    }

    /// Show these areas on the right map: outdoors on Sinnoh's overworld,
    /// caves, ruins and side areas on their layer, anything else indoors
    /// on its own map. Glides in to them. Returns the areas now showing.
    pub(super) fn goto_zones(&mut self, map: &GameMap, zones: &[u16]) -> Vec<u16> {
        self.setup.tab = Tab::Map;
        let was = (self.mapui.area, self.mapui.layer);
        let outdoor: Vec<u16> = zones.iter().copied().filter(|z| map.zone(*z).is_some_and(|z| z.matrix == 0)).collect();
        if !outdoor.is_empty() {
            self.to_overworld();
            self.mapui.selected = outdoor.first().copied();
            let fresh = was != (Some(Area::World), Layer::Overworld);
            self.center_on(map, &outdoor, fresh);
            return outdoor;
        }
        // A cave, ruin or side area: its layer (the first one's kind).
        if let Some(kind) = zones.iter().find_map(|z| map.overlays.iter().find(|o| o.zone == *z)).map(|o| o.kind.clone()) {
            let on: Vec<u16> = zones.iter().copied().filter(|z| map.overlays.iter().any(|o| o.zone == *z && o.kind == kind)).collect();
            let layer = layer_for(&kind);
            self.mapui.area = Some(Area::World);
            self.mapui.layer = layer;
            self.mapui.layer_fit = false;
            self.mapui.selected = on.first().copied();
            self.mapui.focus = Some(Focus {
                zones: on.clone(),
                fresh: was != (Some(Area::World), layer),
                tile: None,
            });
            return on;
        }
        // Some other indoor place (a building, a gym…): its own map.
        let z = zones[0];
        let fresh = match was.0 {
            Some(Area::Inside(o)) => map.zone(o).map(|o| o.matrix) != map.zone(z).map(|z| z.matrix),
            _ => true,
        };
        self.mapui.area = Some(Area::Inside(z));
        self.mapui.selected = Some(z);
        self.center_on(map, &[z], fresh);
        vec![z]
    }

    /// "Go to on map": glide in as close as the map goes, the areas in the
    /// middle of the screen.
    fn center_on(&mut self, map: &GameMap, zones: &[u16], fresh: bool) {
        // (several areas: the first one, the rest are flashing too)
        if let Some(tr) = zones_rect(map, &zones[..zones.len().min(1)]) {
            self.mapui.fly_in(tr.center(), MAX_ZOOM, fresh);
        }
    }

    /// Glide to show these areas (all on the map showing).
    fn map_fit(&mut self, map: &GameMap, zones: &[u16], fresh: bool) {
        if let Some(tr) = zones_rect(map, zones) {
            self.mapui.fly_fit(tr, 12.0, false, fresh);
        }
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_map(&mut self, ui: &mut egui::Ui) {
        // Not ready yet: an ordinary padded page. Ready: the map is the page.
        let padded = egui::Frame::none().inner_margin(egui::Margin {
            left: 32.0,
            right: 24.0,
            top: 26.0,
            bottom: 12.0,
        });
        let map = match self.gamemap.as_ref() {
            None => {
                let _ = self.map_data();
                padded.show(ui, |ui| {
                    kit::page_header(ui, "MAP", "Sinnoh, drawn from your game's own map files.");
                    kit::pill(ui, "Reading the map from your game…", PillKind::Info);
                });
                ui.ctx().request_repaint_after(Duration::from_millis(200));
                return;
            }
            Some(Err(e)) => {
                let e = e.clone();
                let mut go = false;
                padded.show(ui, |ui| {
                    kit::page_header(ui, "MAP", "Sinnoh, drawn from your game's own map files.");
                    kit::card(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Couldn't read the map").size(kit::fs(13.5)).family(kit::strong_family(kit::fs(13.5))).strong().color(TEXT()));
                        ui.label(RichText::new(e).size(kit::fs(12.0)).color(MUTED()));
                        ui.label(
                            RichText::new("The Map uses the map files of your Platinum Together install. If you moved the game somewhere else, choose its folder in the game folder settings.")
                                .size(kit::fs(12.0))
                                .color(MUTED()),
                        );
                        ui.add_space(8.0);
                        if ui
                            .add(egui::Button::new(RichText::new("Game folder settings  ›").size(kit::fs(12.5))).fill(kit::btn_fill()))
                            .on_hover_text("Choose where the game is installed (Advanced settings)")
                            .clicked()
                        {
                            go = true;
                        }
                    });
                });
                if go {
                    self.open_game_folder_settings();
                }
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

        // ── the whole pane is the map; everything else floats on top of it
        // The side panel docks on the right of wide windows (the map is the
        // rest); on narrow ones it floats over the map.
        let full = ui.max_rect();
        let pad = 12.0;
        let narrow = full.width() < 860.0;
        let panel_w = if narrow { (full.width() - 2.0 * pad).min(340.0) } else { (full.width() * 0.27).clamp(330.0, 390.0) };
        let docked = !narrow && !self.mapui.panel_closed;
        let rect = if docked { Rect::from_min_max(full.min, Pos2::new(full.max.x - panel_w, full.max.y)) } else { full };
        ui.allocate_rect(full, Sense::hover());
        let resp = ui.interact(rect, ui.id().with("map-canvas"), Sense::click_and_drag());
        self.mapui.canvas = rect.size();
        let ctx = ui.ctx().clone();
        let spots = self.rare_spots_today();

        let mut go_zone: Option<u16> = None;
        let mut go_hit: Option<u16> = None;
        let mut go_mon: Option<(String, Vec<String>)> = None;
        let mut leave_inside = false;
        let mut clear_hl = false;
        let mut zoom_by: Option<f32> = None;
        let mut fit_all = std::mem::take(&mut self.mapui.want_fit);
        let floors: Vec<(u16, String)> = match area {
            Area::Inside(z) => {
                let name = map.zone(z).map(|z| z.name.clone()).unwrap_or_default();
                map.zones
                    .iter()
                    .filter(|o| o.name == name && o.matrix != 0)
                    .map(|o| (o.id, self.zone_label(&map, o.id)))
                    .collect()
            }
            Area::World => Vec::new(),
        };
        let floor_now = match area {
            Area::Inside(z) => self.zone_label(&map, z),
            Area::World => String::new(),
        };
        // Search matches (for what's typed so far).
        let q = self.mapui.search.trim().to_lowercase();
        let mut hits: Vec<(String, u16)> = Vec::new();
        if !q.is_empty() {
            let mut seen = HashSet::new();
            for z in &map.zones {
                let label = self.zone_label(&map, z.id);
                if label.to_lowercase().contains(&q) && seen.insert(label.clone()) {
                    hits.push((label, z.id));
                }
            }
            hits.sort_by(|a, b| a.0.cmp(&b.0));
        }
        // Pokémon by name or National Dex number ("25", "#025"), those that
        // live in the wild somewhere.
        let mut mon_hits: Vec<(String, String, Vec<String>, u32)> = Vec::new();
        if let (false, Some(w)) = (q.is_empty(), self.wiki.clone()) {
            let num: Option<u32> = q.trim_start_matches('#').trim().parse().ok();
            for m in &w.data.species {
                let hit = match num {
                    Some(n) => m.dex == n,
                    None => q.len() >= 2 && m.name.to_lowercase().contains(&q),
                };
                if !hit {
                    continue;
                }
                let mut places: Vec<String> = Vec::new();
                for e in w.wild(&m.id) {
                    if e.setting.is_empty() && !places.contains(&e.area) {
                        places.push(e.area.clone());
                    }
                }
                mon_hits.push((format!("#{:03}  {}", m.dex, m.name), m.name.clone(), places, m.dex));
            }
            // Names that start with what's typed first.
            mon_hits.sort_by_key(|h| (!h.1.to_lowercase().starts_with(&q), h.0.clone()));
        }
        let showing = self.mapui.highlight.as_ref().map(|h| h.title.clone());
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let left = (gamemap::next_reset(now) - now).max(0);

        // ── keys (when nothing is being typed in)
        let sid = egui::Id::new("map-search-id");
        if ctx.memory(|m| m.focused().is_none()) {
            let (zin, zout, fk, esc, slash) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                    i.key_pressed(egui::Key::Minus),
                    i.key_pressed(egui::Key::F),
                    i.key_pressed(egui::Key::Escape),
                    i.key_pressed(egui::Key::Slash),
                )
            });
            if zin {
                zoom_by = Some(1.4);
            }
            if zout {
                zoom_by = Some(1.0 / 1.4);
            }
            if fk {
                fit_all = true;
            }
            if esc {
                self.mapui.selected = None;
                self.mapui.highlight = None;
                self.mapui.key_open = false;
            }
            if slash {
                if let Some(id) = ctx.data(|d| d.get_temp::<egui::Id>(sid)) {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
        }

        // ── floating: the command bar (top left of the map)
        let narrow_bar = rect.width() < 900.0;
        let target = self.target_places();
        let mut show_target = false;
        let mut enter = false;
        egui::Area::new(egui::Id::new("map-float-top"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min + Vec2::splat(pad))
            .show(&ctx, |ui| {
                kit::settings_style(ui);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if let Area::Inside(z) = area {
                        if bar_button(ui, "Outdoors", false, "Back to the Sinnoh map", |p, r, ink| {
                            super::map_panel::chevron(p, r.center(), Vec2::new(-1.0, 0.0), 5.0, Stroke::new(1.8f32, ink));
                        }) {
                            leave_inside = true;
                        }
                        // other floors / rooms of the same place
                        if floors.len() > 1 {
                            egui::Frame::none()
                                .fill(kit::CARD())
                                .stroke(Stroke::new(1.0f32, LINE()))
                                .rounding(TOP_H / 2.0)
                                .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                .shadow(float_shadow())
                                .show(ui, |ui| {
                                    kit::Dropdown::from_id_salt("map-floor").selected_text(floor_now.as_str()).width(190.0).show_ui(ui, |ui| {
                                        for (f, label) in &floors {
                                            if ui.selectable_label(*f == z, label.as_str()).clicked() {
                                                go_zone = Some(*f);
                                            }
                                        }
                                    });
                                });
                        }
                    }
                    // search, its results dropping down under it
                    ui.vertical(|ui| {
                        let sw = if narrow_bar { 170.0 } else { 230.0 };
                        let r = kit::search_box(ui, &mut self.mapui.search, "Find a place or Pokémon…", sw).on_hover_text("Press / to search");
                        ui.ctx().data_mut(|d| d.insert_temp(sid, r.id));
                        enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if !q.is_empty() {
                            ui.add_space(6.0);
                            float_frame().inner_margin(egui::Margin::same(8.0)).show(ui, |ui| {
                                ui.set_width(300.0);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                if hits.is_empty() && mon_hits.is_empty() {
                                    ui.add_space(6.0);
                                    ui.label(RichText::new("No place or Pokémon by that name.").size(12.0).color(MUTED()));
                                    ui.add_space(6.0);
                                }
                                if !hits.is_empty() {
                                    result_head(ui, "PLACES");
                                }
                                for (label, id) in hits.iter().take(if mon_hits.is_empty() { 8 } else { 5 }) {
                                    let (icon, col, kind) = super::map_panel::zone_kind(&map, *id);
                                    if result_row(ui, label, kind, |p, r| kit::icon_chip(p, r.shrink(2.0), icon, col, false)) {
                                        go_hit = Some(*id);
                                    }
                                }
                                if !mon_hits.is_empty() {
                                    if !hits.is_empty() {
                                        ui.add_space(4.0);
                                    }
                                    result_head(ui, "POKÉMON");
                                }
                                for (label, name, places, dex) in mon_hits.iter().take(if hits.is_empty() { 8 } else { 5 }) {
                                    let sp = self.species.get((*dex as usize).wrapping_sub(1)).cloned();
                                    let sub = match places.len() {
                                        0 => "Not in the wild".to_string(),
                                        1 => "1 place".to_string(),
                                        n => format!("{n} places"),
                                    };
                                    let mut sprite = |p: &egui::Painter, r: Rect| {
                                        if let Some(sp) = &sp {
                                            self.draw_sprite(p, sp, false, r.center_bottom(), Vec2::splat(30.0));
                                        }
                                    };
                                    if result_row(ui, label, &sub, &mut sprite) && !places.is_empty() {
                                        go_mon = Some((name.clone(), places.clone()));
                                    }
                                }
                            });
                        }
                    });
                    if area == Area::World {
                        let mut layer = self.mapui.layer;
                        if map_tabs(ui, &mut layer, &[(Layer::Overworld, "Overworld"), (Layer::Caves, "Caves"), (Layer::Ruins, "Ruins"), (Layer::Side, "Side areas")]) {
                            self.mapui.layer = layer;
                            self.mapui.layer_fit = true;
                            self.mapui.selected = None;
                        }
                    }
                    let key_open = self.mapui.key_open;
                    if bar_button(ui, "Map key", key_open, "What's shown on the map: rare grass, ways in, doors, items and names", |p, r, ink| {
                        for (i, c) in [GOLD, Color32::from_rgb(214, 150, 84), Color32::from_rgb(120, 205, 255)].into_iter().enumerate() {
                            let y = r.center().y - 5.0 + 5.0 * i as f32;
                            p.circle_filled(Pos2::new(r.center().x - 4.0, y), 2.0, c);
                            p.line_segment([Pos2::new(r.center().x, y), Pos2::new(r.center().x + 6.0, y)], Stroke::new(1.5f32, ink));
                        }
                    }) {
                        self.mapui.key_open = !key_open;
                    }
                    if let Some((sp, places)) = &target {
                        if !places.is_empty() {
                            let label = if narrow_bar { String::new() } else { sp.name.to_string() };
                            let sp2 = sp.clone();
                            let tip = format!("Show where {} (your target) lives", sp.name);
                            if bar_sprite_button(ui, &label, &tip, |p, r| {
                                self.draw_sprite(p, &sp2, true, r.center_bottom() + Vec2::new(0.0, 2.0), Vec2::splat(30.0));
                            }) {
                                show_target = true;
                            }
                        }
                    }
                });
                // under the bar: what's highlighted, and the key
                if let Some(title) = &showing {
                    ui.add_space(8.0);
                    float_pill().fill(theme::blend(kit::CARD(), GOLD, 0.16)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Showing: {title}")).size(12.5).color(TEXT()));
                            if super::map_panel::round_button(ui, 20.0, "Stop the highlight (Esc)", |p, r, ink| {
                                let c = r.center();
                                let st = Stroke::new(1.6f32, ink);
                                p.line_segment([c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)], st);
                                p.line_segment([c + Vec2::new(3.5, -3.5), c + Vec2::new(-3.5, 3.5)], st);
                            }) {
                                clear_hl = true;
                            }
                        });
                    });
                }
                if self.mapui.key_open {
                    ui.add_space(8.0);
                    self.map_key(ui, left);
                }
            });
        if enter {
            if let Some((_, id)) = hits.first() {
                go_hit = Some(*id);
            } else if let Some((_, name, places, _)) = mon_hits.iter().find(|h| !h.2.is_empty()) {
                go_mon = Some((name.clone(), places.clone()));
            }
        }
        if show_target {
            if let Some((sp, places)) = &target {
                self.show_on_map(places, format!("Where {} lives", sp.name), None);
            }
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
                    self.map_fit(&map, &[id], true);
                }
            }
        }
        if let Some(z) = go_zone {
            self.mapui.search.clear();
            self.open_zone(&map, z);
        }
        if let Some(z) = go_hit {
            self.mapui.search.clear();
            self.mapui.highlight = None;
            self.goto_zones(&map, &[z]);
        }
        if let Some((name, places)) = go_mon {
            self.mapui.search.clear();
            self.show_on_map(&places, format!("Where {name} lives"), None);
        }

        // ── canvas
        let area = self.mapui.area.unwrap_or(Area::World);
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
        // Outdoors the background is open sea; indoors it's plain and dark.
        // A layer other than the overworld shows only its own pieces.
        let layered = area == Area::World && self.mapui.layer != Layer::Overworld;
        let backdrop = match (area, self.mapui.layer) {
            (Area::World, Layer::Overworld) | (Area::World, Layer::Side) => gamemap::OCEAN,
            (Area::World, Layer::Caves) | (Area::World, Layer::Ruins) => Color32::from_rgb(24, 22, 30),
            _ => Color32::from_rgb(14, 22, 34),
        };
        p.rect_filled(rect, 0.0, backdrop);

        // ── the picture
        match area {
            Area::World if layered => {}
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
                            if !map.is_place_square(cx, cy) {
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

        // ── layers: caves or side areas on their own, each near where it
        // is in Sinnoh, grouped so none overlap
        let mut over_rects: Vec<(u16, Rect)> = Vec::new();
        let mut door_hit: Option<DoorHit> = None;
        if let (Area::Inside(_), true) = (area, self.mapui.show_doors) {
            // Doors of this map (and lines between those that link to
            // another spot on it).
            let view = |m: u16, x: f32, y: f32| (m as usize == mi).then(|| Pos2::new(x, y));
            door_hit = self.draw_doors(&p, &map, &view, &to_screen, rect, zoom, hover_pos, true, &HashSet::new());
        }
        if layered {
            let layer = self.mapui.layer;
            let kind = match layer {
                Layer::Caves => "cave",
                Layer::Ruins => "ruins",
                _ => "side",
            };
            let pieces: Vec<gamemap::Overlay> = map.overlays.iter().filter(|o| o.kind == kind).cloned().collect();
            for o in &pieces {
                let m = o.matrix as usize;
                if !self.mapui.overlay_tex.contains_key(&m) {
                    let made = map
                        .draw_overlay(m)
                        .map(|pic| (ui.ctx().load_texture(format!("map-overlay-{m}"), pic.img, TextureOptions::NEAREST), pic.rect));
                    self.mapui.overlay_tex.insert(m, made);
                }
            }
            if !self.mapui.layer_layout.contains_key(&layer) {
                let layout = match map.layers.get(kind) {
                    Some(plan) => planned_layout(plan, &self.mapui.overlay_tex),
                    None => layer_layout(&pieces, &self.mapui.overlay_tex, &map.doors),
                };
                self.mapui.layer_layout.insert(layer, layout);
            }
            let layout = self.mapui.layer_layout.get(&layer).cloned().unwrap_or_default();
            // Glide to the areas a "Show on map" asked for.
            if let Some(f) = self.mapui.focus.take() {
                let mut bb: Option<Rect> = None;
                let mut spot: Option<Pos2> = None;
                for (z, m, tr, _) in &layout.parts {
                    if f.zones.first() == Some(z) {
                        bb = Some(bb.map_or(*tr, |b| b.union(*tr)));
                    }
                    if let (Some((tm, at)), Some(o)) = (f.tile, layout.origin.get(m)) {
                        if tm as usize == *m {
                            spot = Some(at + *o);
                        }
                    }
                }
                self.mapui.layer_fit = false;
                if let Some(at) = spot {
                    self.mapui.fly_in(at, MAX_ZOOM, f.fresh);
                } else if let Some(bb) = bb {
                    self.mapui.fly_in(bb.center(), MAX_ZOOM, f.fresh);
                }
                ui.ctx().request_repaint();
            }
            // Frame the whole layer when it's switched to (or on Fit).
            if (self.mapui.layer_fit || fit_all) && !layout.places.is_empty() {
                self.mapui.layer_fit = false;
                let mut bb = layout.places[0].1;
                for (_, r) in &layout.places {
                    bb = bb.union(*r);
                }
                let bb = bb.expand(24.0);
                let z = (rect.width() / bb.width()).min(rect.height() / bb.height()).clamp(MIN_ZOOM, MAX_ZOOM);
                self.mapui.fly_to(bb.center(), z);
            }
            // Each place: a card behind its parts, its name on top.
            let mut names: Vec<(Pos2, String, f32)> = Vec::new();
            for (name, tr) in &layout.places {
                let sr = Rect::from_min_max(to_screen(tr.min), to_screen(tr.max)).expand(8.0);
                if !rect.intersects(sr.expand(40.0)) {
                    continue;
                }
                p.rect_filled(sr.translate(Vec2::new(0.0, 3.0)), 10.0, Color32::from_black_alpha(90));
                p.rect_filled(sr, 10.0, Color32::from_rgba_unmultiplied(16, 18, 26, 215));
                p.rect_stroke(sr, 10.0, Stroke::new(1.0f32, Color32::from_white_alpha(45)));
                names.push((sr.center_top() - Vec2::new(0.0, 12.0), name.clone(), (11.0 + zoom * 1.6).clamp(12.0, 16.0)));
            }
            // Parts of the place, with a floor/room name when there's more than one.
            for (zone, m, tr, group_size) in &layout.parts {
                let Some(Some((tex, _))) = self.mapui.overlay_tex.get(m) else { continue };
                let sr = Rect::from_min_max(to_screen(tr.min), to_screen(tr.max));
                if rect.intersects(sr.expand(40.0)) {
                    p.image(tex.id(), sr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    if self.mapui.labels && *group_size > 1 && zoom >= 0.9 {
                        let text = self.part_label(&map, *zone);
                        p.text(
                            sr.left_top() - Vec2::new(0.0, 3.0),
                            Align2::LEFT_BOTTOM,
                            text,
                            kit::font_bold(11.0),
                            Color32::from_white_alpha(190),
                        );
                    }
                }
                over_rects.push((*zone, sr));
            }
            // Where each map's tiles land on the layer.
            let view = |m: u16, x: f32, y: f32| -> Option<Pos2> { layout.origin.get(&(m as usize)).map(|o| Pos2::new(x, y) + *o) };
            // "Show on map": the parts asked for, pulsing (their spots lit).
            if let Some(hl) = self.mapui.highlight.as_ref().filter(|h| h.ping.is_none()) {
                let k = pulse(hl.since);
                let line = Color32::from_rgba_unmultiplied(255, 214, 64, (140.0 + 115.0 * k) as u8);
                let fill = Color32::from_rgba_unmultiplied(255, 214, 64, (40.0 + 80.0 * k) as u8);
                for (zone, m, tr, _) in &layout.parts {
                    if !hl.zones.contains(zone) {
                        continue;
                    }
                    let sr = Rect::from_min_max(to_screen(tr.min), to_screen(tr.max));
                    if let Some(spots) = hl.spots {
                        for (x, y) in map.spot_tiles(*zone, spots) {
                            if let Some(t) = view(*m as u16, x as f32, y as f32) {
                                p.rect_filled(Rect::from_min_size(to_screen(t), Vec2::splat(zoom)), 0.0, fill);
                            }
                        }
                    }
                    p.rect_stroke(sr.expand(5.0), 8.0, Stroke::new(3.0f32, line));
                }
                ui.ctx().request_repaint_after(Duration::from_millis(33));
            }
            // Lines laid out ahead of time: straight runs and right angles.
            if self.mapui.show_doors {
                for line in &layout.links {
                    dotted_path(&p, &line.iter().map(|t| to_screen(*t)).collect::<Vec<_>>(), rect);
                }
                door_hit = self.draw_doors(&p, &map, &view, &to_screen, rect, zoom, hover_pos, !layout.planned, &layout.tagged);
            }
            for (pos, name, size) in names {
                halo_text(&p, pos, &name, size, Color32::WHITE);
            }
        } else if area == Area::World && self.mapui.layer_fit {
            // Back to the overworld: show all of Sinnoh again.
            self.mapui.layer_fit = false;
            let z = (rect.width() / 920.0).min(rect.height() / 760.0).clamp(MIN_ZOOM, MAX_ZOOM);
            self.mapui.fly_to(Pos2::new(480.0, 560.0), z);
        }
        // Where a tile of any map lands on what's showing (None: not shown).
        let origin: HashMap<usize, Vec2> = if layered {
            self.mapui.layer_layout.get(&self.mapui.layer).map(|l| l.origin.clone()).unwrap_or_default()
        } else {
            HashMap::new()
        };
        let view_any = |m: u16, x: f32, y: f32| -> Option<Pos2> {
            match area {
                Area::World if layered => origin.get(&(m as usize)).map(|o| Pos2::new(x, y) + *o),
                Area::World => (m == 0).then(|| Pos2::new(x, y)),
                Area::Inside(_) => (m as usize == mi).then(|| Pos2::new(x, y)),
            }
        };
        // Topmost layer piece under the pointer, if any.
        let over_hit = hover_pos.and_then(|hp| over_rects.iter().rev().find(|(_, r)| r.contains(hp)).map(|(z, _)| *z));

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
        // A cave or side area on the active layer wins over what's beneath it.
        let hover_zone = if layered { over_hit } else { over_hit.or(hover_zone) };

        // ── area outlines and names
        if area == Area::World && !layered {
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
                let tiles: Vec<(usize, usize)> = map.zone_cells(z).into_iter().map(|(cx, cy)| (cx, cy)).collect();
                for (cx, cy) in tiles {
                    let r = Rect::from_min_size(to_screen(Pos2::new((cx * CHUNK) as f32, (cy * CHUNK) as f32)), Vec2::splat(CHUNK as f32 * zoom));
                    p.rect_stroke(r, 0.0, stroke);
                }
            }
        };
        if !layered {
            if let Some(z) = hover_zone {
                outline(z, Stroke::new(1.5f32, Color32::from_white_alpha(170)));
            }
            if let Some(z) = self.mapui.selected {
                outline(z, Stroke::new(2.5f32, theme::LINK()));
            }
        }
        // Layer pieces: a frame when pointed at or picked, and their names.
        for (z, r) in &over_rects {
            let picked = self.mapui.selected == Some(*z);
            if picked || over_hit == Some(*z) {
                let st = if picked {
                    Stroke::new(2.5f32, theme::LINK())
                } else {
                    Stroke::new(1.5f32, Color32::from_white_alpha(170))
                };
                p.rect_stroke(r.expand(6.0), 8.0, st);
            }
        }

        // ── the flashing highlight
        if let Some((at, since)) = self.mapui.highlight.as_ref().and_then(|h| h.ping.map(|p| (p, h.since))) {
            // A rare grass spot: rings rippling out from its tile.
            let c = view_any(at.0, at.1.x, at.1.y).map(to_screen).unwrap_or(Pos2::new(-1000.0, -1000.0));
            let r0 = (zoom * 3.8).clamp(30.0, 62.0) * 0.42 + 5.0;
            let t = since.elapsed().as_secs_f32() / 1.5;
            for i in 0..3 {
                let k = (t + i as f32 / 3.0).fract();
                let a = ((1.0 - k) * 230.0) as u8;
                p.circle_stroke(c, r0 + k * 34.0, Stroke::new(2.5f32 * (1.0 - k) + 0.8, Color32::from_rgba_unmultiplied(255, 214, 64, a)));
            }
            p.circle_stroke(c, r0, Stroke::new(3.5f32, Color32::from_black_alpha(120)));
            p.circle_stroke(c, r0, Stroke::new(2.0f32, GOLD));
            ui.ctx().request_repaint();
        } else if let Some(hl) = self.mapui.highlight.as_ref().filter(|_| !layered) {
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
        if self.mapui.labels && area == Area::World && !layered && zoom >= 0.45 {
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

        // ── ways into caves, ruins and side areas (overworld): click to go in
        let mut enter_hit: Option<(u16, u16, Pos2)> = None;
        let mut enter_tip: Option<String> = None;
        if self.mapui.show_entrances && area == Area::World && !layered && zoom >= 0.8 {
            let mut done: Vec<(u16, Pos2)> = Vec::new();
            let r0 = (zoom * 1.6).clamp(7.0, 13.0);
            for d in &map.doors {
                if d.to_matrix != 0 {
                    continue;
                }
                let Some(o) = map.overlays.iter().find(|o| o.matrix == d.matrix) else { continue };
                let at = Pos2::new(d.tx as f32 + 0.5, d.ty as f32 + 0.5);
                // one icon per entrance (doors side by side share one)
                if done
                    .iter()
                    .any(|(m, q)| map.overlays.iter().any(|oo| oo.matrix == *m && oo.group == o.group) && (q.x - at.x).abs() <= 2.0 && (q.y - at.y).abs() <= 2.0)
                {
                    continue;
                }
                done.push((d.matrix, at));
                let c = to_screen(at);
                if !rect.contains(c) {
                    continue;
                }
                let (col, layer) = match o.kind.as_str() {
                    "cave" => (Color32::from_rgb(214, 150, 84), "Caves"),
                    "ruins" => (Color32::from_rgb(176, 132, 230), "Ruins"),
                    _ => (Color32::from_rgb(110, 200, 140), "Side areas"),
                };
                let hot = hover_pos.is_some_and(|h| h.distance(c) <= r0 + 3.0);
                entrance_icon(&p, c, r0, col, hot);
                if hot && enter_hit.is_none() {
                    let name = if o.group.is_empty() { self.zone_label(&map, o.zone) } else { o.group.clone() };
                    enter_tip = Some(format!("Way into {name} ({layer})\nClick to go in"));
                    let (x0, y0, x1, y1) = d.tiles();
                    enter_hit = Some((o.zone, d.matrix, Pos2::new((x0 + x1) / 2.0, (y0 + y1) / 2.0)));
                }
            }
        }

        // ── items on the ground and hidden ones
        let mut item_tip: Option<String> = None;
        if self.mapui.show_items || self.mapui.show_hidden {
            let show_at = match area {
                Area::World if layered => 1.3,
                Area::World => 2.0,
                Area::Inside(_) => 1.0,
            };
            if zoom >= show_at {
                let r = (zoom * 0.45).clamp(4.5, 9.0);
                for it in &map.items {
                    if !(if it.hidden { self.mapui.show_hidden } else { self.mapui.show_items }) {
                        continue;
                    }
                    let Some(t) = view_any(it.matrix, it.x as f32 + 0.5, it.y as f32 + 0.5) else { continue };
                    let c = to_screen(t);
                    if !rect.contains(c) {
                        continue;
                    }
                    item_icon(&p, c, r, it.hidden);
                    if item_tip.is_none() && hover_pos.is_some_and(|h| h.distance(c) <= r + 3.0) {
                        let how = if it.hidden { "Hidden item" } else { "Item ball" };
                        let note = if it.note.is_empty() { String::new() } else { format!("\n{}", it.note) };
                        item_tip = Some(format!("{}\n{how}{note}", it.name));
                    }
                }
            }
        }

        // ── today's rare grass, spots a tile or two apart shown as one
        let mut rare_tip: Option<String> = None;
        let mut rare_hit: Option<String> = None;
        if self.mapui.show_rare {
            let rare = self.mapui.rare.clone();
            // Layers are seen further out, so their Pokémon show sooner.
            let show_at = if layered { 1.3 } else { 2.5 };
            // (map, tiles, species)
            let mut groups: Vec<(u16, Vec<Pos2>, Vec<u32>)> = Vec::new();
            for r in &rare {
                // Only tiles where a Pokémon we know was met: no pings for
                // "rare grass around here" scans or unknown species.
                let Some(sp) = r.species.filter(|&sp| self.rare_species(&map, sp).is_some()) else { continue };
                let t = Pos2::new(r.x as f32 + 0.5, r.y as f32 + 0.5);
                match groups
                    .iter_mut()
                    .find(|g| g.0 == r.matrix && g.1.iter().any(|q| (q.x - t.x).abs() <= 2.0 && (q.y - t.y).abs() <= 2.0))
                {
                    Some(g) => {
                        g.1.push(t);
                        g.2.push(sp);
                    }
                    None => groups.push((r.matrix, vec![t], vec![sp])),
                }
            }
            if zoom >= show_at {
                for (m, tiles, sps) in &groups {
                    let mid = tiles.iter().fold(Vec2::ZERO, |a, q| a + q.to_vec2()) / tiles.len() as f32;
                    let Some(t) = view_any(*m, mid.x, mid.y) else { continue };
                    let c = to_screen(t);
                    if !rect.contains(c) {
                        continue;
                    }
                    // the Pokémon seen most here
                    let mut kinds: Vec<(u32, usize)> = Vec::new();
                    for sp in sps {
                        match kinds.iter_mut().find(|k| k.0 == *sp) {
                            Some(k) => k.1 += 1,
                            None => kinds.push((*sp, 1)),
                        }
                    }
                    kinds.sort_by(|a, b| b.1.cmp(&a.1));
                    let top = kinds[0].0;
                    let size = (zoom * 3.8).clamp(30.0, 62.0);
                    if let Some(s) = self.rare_species(&map, top) {
                        p.circle_filled(c, size * 0.42, Color32::from_black_alpha(120));
                        p.circle_stroke(c, size * 0.42, Stroke::new(1.5f32, GOLD));
                        self.draw_sprite(&p, &s, false, c + Vec2::new(0.0, size * 0.4), Vec2::splat(size));
                    }
                    if tiles.len() > 1 {
                        let bc = c + Vec2::new(size * 0.34, -size * 0.34);
                        p.circle_filled(bc, 8.5, GOLD);
                        p.circle_stroke(bc, 8.5, Stroke::new(1.0f32, Color32::from_black_alpha(160)));
                        p.text(bc, Align2::CENTER_CENTER, tiles.len().to_string(), kit::font_bold(10.0), Color32::from_rgb(30, 24, 10));
                    }
                    if hover_pos.is_some_and(|h| h.distance(c) < size * 0.45) {
                        let names: Vec<String> = kinds
                            .iter()
                            .map(|(sp, n)| {
                                if *n > 1 {
                                    format!("{} ×{n}", self.rare_name(&map, *sp))
                                } else {
                                    self.rare_name(&map, *sp)
                                }
                            })
                            .collect();
                        let spots = if tiles.len() > 1 { format!(" ({} spots)", tiles.len()) } else { String::new() };
                        rare_tip = Some(format!(
                            "{} on rare grass today{spots}\nClick to open {} in the Pokédex",
                            names.join(", "),
                            self.rare_name(&map, top)
                        ));
                        rare_hit = self.rare_wiki_id(&map, top);
                    }
                }
            }
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }

        // ── hover / click
        if let Some(h) = &door_hit {
            resp.clone().on_hover_text(h.tip.clone()).on_hover_cursor(egui::CursorIcon::PointingHand);
        } else if let Some(tip) = enter_tip {
            resp.clone().on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
        } else if let Some(tip) = item_tip.filter(|_| rare_tip.is_none()) {
            resp.clone().on_hover_text(tip);
        } else if let Some(tip) = rare_tip {
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
            if let Some(h) = &door_hit {
                self.use_door(&map, h);
            } else if let Some((zone, m, at)) = enter_hit {
                // In through the door: its layer, zoomed in on where you come in.
                self.mapui.highlight = None;
                self.goto_zones(&map, &[zone]);
                if let Some(f) = self.mapui.focus.as_mut() {
                    f.tile = Some((m, at));
                }
            } else if let Some(id) = rare_hit.take() {
                self.open_dex_entry(&id);
            } else if let Some(z) = hover_zone {
                // Pick it and glide in on it (no further out than now); the
                // panel shows what's there.
                self.mapui.selected = Some(z);
                self.mapui.panel_tab = super::map_panel::MapTab::Area;
                self.mapui.panel_closed = false;
                let tr = if layered {
                    over_rects.iter().find(|(o, _)| *o == z).map(|(_, r)| Rect::from_min_max(to_tile(r.min), to_tile(r.max)))
                } else {
                    zones_rect(&map, &[z])
                };
                if let Some(tr) = tr {
                    self.mapui.fly_fit(tr, 8.0, true, false);
                }
            }
        }
        // Double-click on empty space: let go of the picked area and any
        // highlight or rare grass ping.
        if resp.double_clicked() && hover_zone.is_none() && door_hit.is_none() && enter_hit.is_none() {
            self.mapui.selected = None;
            self.mapui.highlight = None;
        }
        if resp.double_clicked() && area == Area::World {
            if let Some(name) = hover_zone.and_then(|z| map.zone(z)).map(|z| z.name.clone()) {
                if let Some(first) = map.zones.iter().find(|o| o.matrix != 0 && o.name == name).map(|o| o.id) {
                    self.open_zone(&map, first);
                }
            }
        }
        // ── floating: zoom buttons and, beside them, the minimap (bottom right)
        // (not under the floating panel on narrow windows)
        let covered = !docked && !self.mapui.panel_closed;
        let mini = area == Area::World && !layered && !covered && rect.width() >= 720.0 && rect.height() >= 460.0;
        let rail = egui::Area::new(egui::Id::new("map-float-zoom"))
            .order(egui::Order::Middle)
            .pivot(Align2::RIGHT_BOTTOM)
            .fixed_pos(rect.max - Vec2::new(pad, pad))
            .show(&ctx, |ui| {
                kit::settings_style(ui);
                let b = |ui: &mut egui::Ui, t: &str, tip: &str| {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
                    let p = ui.painter();
                    let down = resp.is_pointer_button_down_on();
                    let ink = if resp.hovered() {
                        p.add(kit::primary_shape(r.shrink(if down { 1.0 } else { 0.0 }), false));
                        kit::on_primary()
                    } else {
                        TEXT()
                    };
                    let size = if t.chars().count() > 1 { 12.0 } else { 17.0 };
                    p.text(r.center(), Align2::CENTER_CENTER, t, kit::font_bold(kit::fs(size)), ink);
                    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                };
                egui::Frame::none()
                    .fill(kit::CARD())
                    .stroke(Stroke::new(1.0f32, LINE()))
                    .rounding(30.0)
                    .inner_margin(egui::Margin::same(5.0))
                    .shadow(float_shadow())
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        if b(ui, "+", "Zoom in (+)") {
                            self.mapui.fly_to(self.mapui.center, (self.mapui.zoom * 1.4).clamp(MIN_ZOOM, MAX_ZOOM));
                        }
                        if b(ui, "−", "Zoom out (−)") {
                            self.mapui.fly_to(self.mapui.center, (self.mapui.zoom / 1.4).clamp(MIN_ZOOM, MAX_ZOOM));
                        }
                        if b(ui, "Fit", "Show the whole map (F)") {
                            self.mapui.want_fit = true;
                            ui.ctx().request_repaint();
                        }
                    });
            })
            .response
            .rect;
        if mini {
            egui::Area::new(egui::Id::new("map-float-mini"))
                .order(egui::Order::Middle)
                .pivot(Align2::RIGHT_BOTTOM)
                .fixed_pos(Pos2::new(rail.min.x - 10.0, rect.max.y - pad))
                .show(&ctx, |ui| {
                    kit::settings_style(ui);
                    float_frame().inner_margin(egui::Margin::same(6.0)).rounding(16.0).show(ui, |ui| {
                        self.map_minimap(ui, &map, rect, (rect.width() * 0.17).clamp(150.0, 210.0));
                    });
                });
        }

        // ── floating: where you are and help (bottom left of the map)
        egui::Area::new(egui::Id::new("map-float-status"))
            .order(egui::Order::Middle)
            .pivot(Align2::LEFT_BOTTOM)
            .fixed_pos(rect.left_bottom() + Vec2::new(pad, -pad))
            .show(&ctx, |ui| {
                kit::settings_style(ui);
                let layer = self.mapui.layer;
                self.map_status(ui, &map, area, layer);
            });

        // ── the side panel: the picked area, today's rare grass
        let mut acts = super::map_panel::PanelActs::default();
        if !self.mapui.panel_closed {
            let (pos, size) = if docked {
                (Pos2::new(full.max.x - panel_w, full.min.y), Vec2::new(panel_w, full.height()))
            } else {
                let top = rect.min.y + pad + TOP_H + 10.0;
                // ends above the zoom buttons
                (Pos2::new(full.max.x - pad - panel_w, top), Vec2::new(panel_w, (full.max.y - top - pad - 150.0).clamp(200.0, 620.0)))
            };
            egui::Area::new(egui::Id::new("map-panel"))
                .order(egui::Order::Middle)
                .fixed_pos(pos)
                .constrain(false)
                .show(&ctx, |ui| {
                    kit::settings_style(ui);
                    let (frame, margin) = if docked {
                        (egui::Frame::none().fill(kit::CARD()).inner_margin(egui::Margin { left: 16.0, right: 14.0, top: 14.0, bottom: 0.0 }), 30.0)
                    } else {
                        (float_frame().inner_margin(egui::Margin::same(12.0)), 24.0)
                    };
                    let h = size.y - if docked { 14.0 } else { 24.0 };
                    frame.show(ui, |ui| {
                        ui.set_width(size.x - margin);
                        ui.set_min_height(h);
                        ui.set_max_height(h);
                        acts = self.map_panel(ui, &map, &spots, left, h);
                    });
                    if docked {
                        ui.painter().line_segment([pos, Pos2::new(pos.x, full.max.y)], Stroke::new(1.0f32, LINE()));
                    }
                });
        } else {
            // folded away: a pill to bring it back
            let rare_n = spots.len();
            let mut open = false;
            egui::Area::new(egui::Id::new("map-panel-tab"))
                .order(egui::Order::Middle)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(Pos2::new(rect.max.x - pad, rect.min.y + pad))
                .show(&ctx, |ui| {
                    kit::settings_style(ui);
                    let label = if rare_n > 0 { format!("Rare grass {rare_n}") } else { "Panel".to_string() };
                    if bar_button(ui, &label, false, "Show the side panel: the place you picked and today's rare grass", |p, r, ink| {
                        super::map_panel::chevron(p, r.center() - Vec2::new(3.0, 0.0), Vec2::new(-1.0, 0.0), 6.0, Stroke::new(1.8f32, ink));
                        super::map_panel::chevron(p, r.center() + Vec2::new(3.0, 0.0), Vec2::new(-1.0, 0.0), 6.0, Stroke::new(1.8f32, ink));
                    }) {
                        open = true;
                    }
                });
            if open {
                self.mapui.panel_closed = false;
            }
        }
        if acts.collapse {
            self.mapui.panel_closed = true;
        }
        if acts.close {
            self.mapui.selected = None;
        }
        if let Some(z) = acts.go_inside {
            self.open_zone(&map, z);
        }
        if let Some(z) = acts.zoom_to {
            self.map_fit(&map, &[z], false);
        }
        if std::mem::take(&mut self.mapui.want_poster) {
            self.open_rare_poster(&ctx);
        }
        if std::mem::take(&mut self.mapui.want_export) {
            self.mapui.export_msg = Some(export_shared_rare(&self.mapui.rare));
        }
    }

    /// Your target and the wiki places it lives in (the command bar's
    /// "where's my target" button).
    fn target_places(&self) -> Option<(crate::species::Species, Vec<String>)> {
        let sp = self.species.get(self.target_idx()?)?.clone();
        let w = self.wiki.as_ref()?;
        let mon = w.by_dex(sp.dex)?;
        let mut places: Vec<String> = Vec::new();
        for e in w.wild(&mon.id) {
            if e.setting.is_empty() && !places.contains(&e.area) {
                places.push(e.area.clone());
            }
        }
        Some((sp, places))
    }

    /// Doors, stairs and exits on the map showing: a marker on their tiles
    /// and an evenly dotted line to the other end when that's showing too.
    /// `view` places a tile of a map on the map showing (None: not shown).
    /// Returns the door under the pointer.
    #[allow(clippy::too_many_arguments)]
    fn draw_doors(
        &self,
        p: &egui::Painter,
        map: &GameMap,
        view: &dyn Fn(u16, f32, f32) -> Option<Pos2>,
        to_screen: &dyn Fn(Pos2) -> Pos2,
        rect: Rect,
        zoom: f32,
        hover: Option<Pos2>,
        straight: bool,
        tagged: &HashSet<usize>,
    ) -> Option<DoorHit> {
        const LINK: Color32 = Color32::from_rgb(120, 205, 255);
        const OUT: Color32 = Color32::from_rgb(130, 230, 140);
        const AWAY: Color32 = Color32::from_rgb(255, 182, 92);
        let mut lines: Vec<(Pos2, Pos2)> = Vec::new();
        let mut seen: HashSet<[i32; 4]> = HashSet::new();
        let mut marks: Vec<(Rect, Color32, gamemap::Door, Option<Pos2>)> = Vec::new();
        for d in &map.doors {
            let (x0, y0, x1, y1) = d.tiles();
            let Some(a) = view(d.matrix, x0, y0) else { continue };
            let tr = Rect::from_min_size(a, Vec2::new(x1 - x0, y1 - y0));
            // Its other end: the door you come out of, if it's showing.
            let dest = if d.to_matrix == 0 {
                None
            } else {
                let back = map
                    .doors
                    .iter()
                    .find(|o| o.matrix == d.to_matrix && (o.x..=o.x2).contains(&d.tx) && (o.y..=o.y2).contains(&d.ty));
                match back {
                    Some(o) => {
                        let (bx0, by0, bx1, by1) = o.tiles();
                        view(o.matrix, bx0, by0).map(|b| b + Vec2::new(bx1 - bx0, by1 - by0) * 0.5)
                    }
                    None => view(d.to_matrix, d.tx as f32 + 0.5, d.ty as f32 + 0.5),
                }
            };
            if let Some(b) = dest {
                let c = tr.center();
                let (ka, kb) = ([(c.x * 2.0) as i32, (c.y * 2.0) as i32], [(b.x * 2.0) as i32, (b.y * 2.0) as i32]);
                let key = if ka <= kb { [ka[0], ka[1], kb[0], kb[1]] } else { [kb[0], kb[1], ka[0], ka[1]] };
                if straight && ka != kb && seen.insert(key) {
                    lines.push((c, b));
                }
            }
            let sr = Rect::from_min_max(to_screen(tr.min), to_screen(tr.max));
            let sr = Rect::from_center_size(sr.center(), sr.size().max(Vec2::splat(8.0)));
            if !rect.intersects(sr.expand(4.0)) {
                continue;
            }
            let col = if d.to_matrix == 0 {
                OUT
            } else if dest.is_some() {
                LINK
            } else {
                AWAY
            };
            marks.push((sr, col, *d, dest));
        }
        for (a, b) in lines {
            dotted_path(p, &[to_screen(a), to_screen(b)], rect);
        }
        let mut hit: Option<DoorHit> = None;
        for (sr, col, d, dest) in marks {
            let r = (zoom * 0.25).clamp(2.0, 4.0);
            p.rect_filled(sr.expand(1.5), r + 1.0, Color32::from_black_alpha(170));
            p.rect_filled(sr, r, col.gamma_multiply(0.85));
            p.rect_stroke(sr.shrink(1.0), r, Stroke::new(1.0f32, Color32::from_white_alpha(200)));
            // Rooms too linked up for lines: a name tag says where it goes.
            if dest.is_some() && tagged.contains(&(d.matrix as usize)) && zoom >= 1.6 {
                let text = format!("→ {}", self.part_label(map, d.to_zone));
                let at = sr.right_center() + Vec2::new(3.0, 0.0);
                let g = p.layout_no_wrap(text, kit::font_bold(10.0), Color32::WHITE);
                let tr = Rect::from_min_size(at - Vec2::new(0.0, g.size().y / 2.0 + 1.0), g.size() + Vec2::new(6.0, 2.0));
                p.rect_filled(tr, 3.0, Color32::from_black_alpha(190));
                p.galley(tr.min + Vec2::new(3.0, 1.0), g, Color32::WHITE);
            }
            if hit.is_none() && hover.is_some_and(|h| sr.expand(3.0).contains(h)) {
                p.rect_stroke(sr.expand(3.0), r + 2.0, Stroke::new(2.0f32, Color32::WHITE));
                let to = self.zone_label(map, d.to_zone);
                let tip = if d.to_matrix == 0 {
                    format!("Exit to {to}\nClick to go there")
                } else if dest.is_some() {
                    format!("To {to}\nClick to follow it")
                } else {
                    format!("To {to}\nClick to go there")
                };
                hit = Some(DoorHit { tip, door: d, dest });
            }
        }
        hit
    }

    /// The map key: what each kind of pin and icon is, with a switch for
    /// each (opens from the command bar).
    fn map_key(&mut self, ui: &mut egui::Ui, left: i64) {
        float_frame().inner_margin(egui::Margin::same(14.0)).show(ui, |ui| {
            ui.set_width(250.0);
            ui.label(RichText::new("Map key").font(kit::font_bold(14.0)).color(TEXT()));
            ui.label(RichText::new("Turn things on or off on the map.").size(11.5).color(MUTED()));
            ui.add_space(8.0);
            let row = |ui: &mut egui::Ui, on: &mut bool, label: &str, tip: &str, icon: &dyn Fn(&egui::Painter, Pos2)| {
                ui.horizontal(|ui| {
                    ui.set_min_height(28.0);
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                    icon(ui.painter(), r.center());
                    ui.add_space(4.0);
                    ui.label(RichText::new(label).size(13.0).color(TEXT())).on_hover_text(tip);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        kit::toggle(ui, on).on_hover_text(tip);
                    });
                });
            };
            let reset = format!(
                "Today's rare grass, from your game log plus the shared list. New patches in {}h {:02}m (midnight, Central European time).",
                left / 3600,
                left % 3600 / 60
            );
            row(ui, &mut self.mapui.show_rare, "Rare grass", &reset, &|p, c| {
                p.circle_filled(c, 7.0, Color32::from_black_alpha(120));
                p.circle_stroke(c, 7.0, Stroke::new(1.5f32, GOLD));
                theme::sparkle(p, c, 4.0, GOLD);
            });
            row(ui, &mut self.mapui.show_entrances, "Ways in", "Icons where you can go into a cave, ruin or side area (click one to go in)", &|p, c| {
                entrance_icon(p, c, 7.0, Color32::from_rgb(214, 150, 84), false);
            });
            row(ui, &mut self.mapui.show_doors, "Doors and exits", "Doors, stairs and exits on the layers and inside, with lines between linked doors", &|p, c| {
                for (i, col) in [Color32::from_rgb(120, 205, 255), Color32::from_rgb(130, 230, 140), Color32::from_rgb(255, 182, 92)].into_iter().enumerate() {
                    let r = Rect::from_center_size(c + Vec2::new(-5.0 + 5.0 * i as f32, 0.0), Vec2::splat(4.5));
                    p.rect_filled(r, 1.0, col);
                }
            });
            row(ui, &mut self.mapui.show_items, "Item balls", "Items lying on the ground", &|p, c| item_icon(p, c, 6.0, false));
            row(ui, &mut self.mapui.show_hidden, "Hidden items", "Items hidden in the ground (find them with the Dowsing Machine)", &|p, c| item_icon(p, c, 6.0, true));
            row(ui, &mut self.mapui.labels, "Names", "Area and room names", &|p, c| {
                p.text(c, Align2::CENTER_CENTER, "Aa", kit::font_bold(11.0), TEXT());
            });
        });
    }

    /// A part's name within its place ("B1F", "Room 3").
    fn part_label(&self, map: &GameMap, zone: u16) -> String {
        let label = self.zone_label(map, zone);
        let place = map.zone(zone).map(|z| layer_name(z).to_string()).unwrap_or_default();
        let short = label
            .strip_prefix(place.as_str())
            .map(|t| t.trim_start_matches([' ', '-', '(', ',']).trim_end_matches(')').trim().to_string())
            .filter(|t| !t.is_empty());
        // No floor name known: "Room N" from the area's code (…R0103 → 3).
        short.unwrap_or_else(|| {
            let code = map.zone(zone).map(|z| z.code.clone()).unwrap_or_default();
            let n: u32 = code.chars().rev().take(2).collect::<String>().chars().rev().collect::<String>().parse().unwrap_or(0);
            if n > 0 {
                format!("Room {n}")
            } else {
                label.clone()
            }
        })
    }

    /// Go through a door: follow its line, or open where it leads.
    fn use_door(&mut self, map: &GameMap, hit: &DoorHit) {
        let d = hit.door;
        if let Some(b) = hit.dest {
            let z = self.mapui.zoom.max(3.0);
            self.mapui.fly_to(b, z);
            return;
        }
        let to = Pos2::new(d.tx as f32 + 0.5, d.ty as f32 + 0.5);
        if d.to_matrix == 0 {
            self.to_overworld();
            self.mapui.selected = map.world_zone_at(d.tx as usize, d.ty as usize);
            self.mapui.fly_in(to, 5.0, true);
        } else {
            self.mapui.highlight = None;
            self.goto_zones(map, &[d.to_zone]);
            if matches!(self.mapui.area, Some(Area::Inside(_))) {
                self.mapui.fly_in(to, 5.0, true);
            }
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
        self.map_fit(map, &[z], false);
    }

    /// The name to show for an area: the wiki's name when it's one place
    /// (e.g. "Route 210 (north)"), otherwise the game's.
    pub(super) fn zone_label(&self, map: &GameMap, z: u16) -> String {
        let places = map.zone_places(z);
        let exact: Vec<&str> = places
            .iter()
            .copied()
            .filter(|p| map.places.get(*p).is_some_and(|l| l.exact && l.zones.len() == 1))
            .collect();
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

}
