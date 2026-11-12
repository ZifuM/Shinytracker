//! The map of Sinnoh, read from the player's own Platinum Together install
//! (nothing from the game is copied into the app):
//!
//! - `fielddata/mapmatrix/map_matrix.narc`: how 32×32-tile map squares are
//!   laid out, and which area (map header) each square belongs to.
//! - `fielddata/land_data/land_data.narc`: what every tile is (tall grass,
//!   water, cave floor, wall…).
//! - `minimap/<x>_<y>.png`: the game's own overhead picture of each outdoor
//!   square (8 pixels a tile).
//!
//! The `patch-built` copies are used first: they're the ones the game runs.
//! Area names and which wiki place is which area come from `zones.json`,
//! worked out from the same files.

use eframe::egui::{Color32, ColorImage};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// Tiles in a map square, each way.
pub const CHUNK: usize = 32;
/// Pixels a tile in the game's minimap pictures.
pub const PX: usize = 8;
/// Pixels a tile in the zoomed-out overview picture.
pub const OVERVIEW_PX: usize = 2;

const ZONES_JSON: &str = include_str!("zones.json");

// ───────────────────────────── zone table ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Zone {
    pub id: u16,
    pub code: String,
    pub name: String,
    pub matrix: u16,
    #[allow(dead_code)]
    pub enc: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaceLink {
    pub zones: Vec<u16>,
    /// false = one of several rooms with the same Pokémon (not sure which).
    pub exact: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ZoneFile {
    zones: Vec<Zone>,
    places: HashMap<String, PlaceLink>,
    #[serde(rename = "gameSpecies", default)]
    game_species: HashMap<String, String>,
    #[serde(default)]
    overlays: Vec<Overlay>,
    /// Doors of the caves and side areas:
    /// [map, x, y, x2, y2, to area, to map, to x, to y].
    #[serde(default)]
    doors: Vec<[u16; 9]>,
    #[serde(default)]
    layers: HashMap<String, LayerPlan>,
    /// Items on the ground and hidden: map, x, y, hidden (0/1), name, note.
    #[serde(default)]
    items: Vec<(u16, u16, u16, u8, String, String)>,
}

/// An item ball or hidden item, where it lies (a tile of its map), as the
/// Platinum Together Wiki names it.
#[derive(Debug, Clone)]
pub struct MapItem {
    pub matrix: u16,
    pub x: u16,
    pub y: u16,
    pub hidden: bool,
    pub name: String,
    /// What it takes to reach it ("Needs Surf"), if anything.
    pub note: String,
}

/// A layer laid out ahead of time (so the lines between doors never cross):
/// where each part goes, the places' boxes and the lines' corners.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayerPlan {
    /// map, area, x, y on the layer of the part's tile (cx, cy), then the
    /// part (cx, cy, w, h) in tiles of its own map.
    pub parts: Vec<(u16, u16, i32, i32, i32, i32, i32, i32)>,
    /// name, box (x0, y0, x1, y1), and whether its doors are joined by lines
    /// (false where they couldn't be drawn without crossing).
    pub places: Vec<(String, i32, i32, i32, i32, bool)>,
    /// Each line: x0, y0, x1, y1, … (tile corners, through the tile's middle).
    pub links: Vec<Vec<i32>>,
}

/// A door, stairs or exit of a cave or side area: the tiles you step on
/// (x..=x2, y..=y2 on its map) and where it takes you (a tile of another
/// map, or the outdoors when `to_matrix` is 0).
#[derive(Debug, Clone, Copy)]
pub struct Door {
    pub matrix: u16,
    pub x: u16,
    pub y: u16,
    pub x2: u16,
    pub y2: u16,
    pub to_zone: u16,
    pub to_matrix: u16,
    pub tx: u16,
    pub ty: u16,
}

impl Door {
    /// The door's tiles (in tiles of its own map).
    pub fn tiles(&self) -> (f32, f32, f32, f32) {
        (self.x as f32, self.y as f32, (self.x2 + 1) as f32, (self.y2 + 1) as f32)
    }
}

/// A part of a cave, ruin or side area (one of its maps) for the Map's
/// layers: which layer, which place it belongs to, and where it sits on the
/// Sinnoh map (tile (0, 0) of its own map lands on tile (x, y) outdoors,
/// worked out from the doors between them; unknown for some deep floors).
#[derive(Debug, Clone, Deserialize)]
pub struct Overlay {
    pub zone: u16,
    pub matrix: u16,
    /// "cave", "ruins" or "side".
    pub kind: String,
    /// The place it's part of ("Mt. Coronet").
    #[serde(default)]
    pub group: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    /// One of the parts you walk into from outside (sets where the place is).
    #[serde(default)]
    pub entry: bool,
}

/// A cave/side-area picture with see-through walls, cut down to its floor.
pub struct OverlayPic {
    pub img: ColorImage,
    /// Left, top, width, height in tiles, within the area's own map.
    pub rect: [f32; 4],
}

// ───────────────────────────── tiles ─────────────────────────────

/// What a tile is, as far as the map cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Blocked,
    Floor,
    Grass,
    Snow,
    Marsh,
    Cave,
    Water,
}

impl Ground {
    pub fn from_perm(v: u16) -> Ground {
        let b = (v & 0xFF) as u8;
        match b {
            0x02 | 0x03 => Ground::Grass,
            0xA1..=0xA3 => Ground::Snow,
            0xA4..=0xA7 => Ground::Marsh,
            0x08 => Ground::Cave,
            0x10 | 0x15 => Ground::Water,
            _ if v & 0x8000 != 0 => Ground::Blocked,
            _ => Ground::Floor,
        }
    }
}

/// Which tiles a way of encountering uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spots {
    /// Tall grass, deep snow and marsh mud.
    Land,
    /// Cave floor.
    Cave,
    /// Water you can surf or fish in.
    Water,
}

impl Spots {
    /// From a wiki encounter method ("grass", "surf", "old_rod"…).
    pub fn for_method(method: &str) -> Spots {
        match method {
            "surf" | "old_rod" | "good_rod" | "super_rod" => Spots::Water,
            "cave" => Spots::Cave,
            _ => Spots::Land,
        }
    }
    pub fn has(self, g: Ground) -> bool {
        match self {
            Spots::Land => matches!(g, Ground::Grass | Ground::Snow | Ground::Marsh),
            Spots::Cave => g == Ground::Cave,
            Spots::Water => g == Ground::Water,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Spots::Land => "Grass",
            Spots::Cave => "Cave floor",
            Spots::Water => "Water",
        }
    }
}

// ───────────────────────────── files ─────────────────────────────

fn u16le(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn u32le(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

/// The files inside a Nintendo archive (NARC).
fn narc_files(b: &[u8]) -> Option<Vec<&[u8]>> {
    if b.len() < 16 || &b[0..4] != b"NARC" {
        return None;
    }
    let mut p = u16le(b, 0x0C) as usize;
    if b.get(p..p + 4)? != b"BTAF" {
        return None;
    }
    let fat_size = u32le(b, p + 4) as usize;
    let n = u16le(b, p + 8) as usize;
    let fat: Vec<(usize, usize)> = (0..n).map(|i| (u32le(b, p + 12 + i * 8) as usize, u32le(b, p + 16 + i * 8) as usize)).collect();
    p += fat_size;
    if b.get(p..p + 4)? != b"BTNF" {
        return None;
    }
    p += u32le(b, p + 4) as usize;
    if b.get(p..p + 4)? != b"GMIF" {
        return None;
    }
    let base = p + 8;
    fat.into_iter().map(|(s, e)| b.get(base + s..base + e)).collect()
}

/// How map squares are laid out in one map.
#[derive(Debug, Clone, Default)]
pub struct Matrix {
    pub w: usize,
    pub h: usize,
    /// Area of each square (only in maps that have more than one area).
    pub headers: Option<Vec<u16>>,
    /// land_data file of each square (0xFFFF = nothing there).
    pub land: Vec<u16>,
}

impl Matrix {
    fn parse(b: &[u8]) -> Option<Matrix> {
        let (w, h, has_headers, has_heights, name_len) = (*b.first()? as usize, *b.get(1)? as usize, *b.get(2)?, *b.get(3)?, *b.get(4)? as usize);
        let n = w * h;
        let mut p = 5 + name_len;
        let headers = if has_headers != 0 {
            let v = (0..n).map(|i| b.get(p + i * 2..p + i * 2 + 2).map(|s| u16le(s, 0))).collect::<Option<Vec<_>>>()?;
            p += n * 2;
            Some(v)
        } else {
            None
        };
        if has_heights != 0 {
            p += n;
        }
        let land = (0..n).map(|i| b.get(p + i * 2..p + i * 2 + 2).map(|s| u16le(s, 0))).collect::<Option<Vec<_>>>()?;
        Some(Matrix { w, h, headers, land })
    }

    pub fn land_at(&self, cx: usize, cy: usize) -> Option<u16> {
        if cx >= self.w || cy >= self.h {
            return None;
        }
        let l = self.land[cy * self.w + cx];
        (l != 0xFFFF).then_some(l)
    }

    pub fn header_at(&self, cx: usize, cy: usize) -> Option<u16> {
        self.headers.as_ref().and_then(|h| h.get(cy * self.w + cx).copied())
    }
}

/// Where the game is installed.
pub fn game_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_default().join("PlatinumTogether").join("Game")
}

/// The patched copy of a game file if there is one, otherwise the original.
fn game_file(dir: &Path, rel: &str) -> PathBuf {
    let patched = dir.join("patch-built").join(rel);
    if patched.exists() {
        patched
    } else {
        dir.join(rel)
    }
}

// ───────────────────────────── the map ─────────────────────────────

/// Everything the Map and the Pokédex need about Sinnoh's layout.
pub struct GameMap {
    pub dir: PathBuf,
    pub zones: Vec<Zone>,
    zone_index: HashMap<u16, usize>,
    pub places: HashMap<String, PlaceLink>,
    /// Game species number → wiki id, for the newer Pokémon (the rest are
    /// their National Dex number).
    pub game_species: HashMap<u32, String>,
    pub matrices: Vec<Matrix>,
    /// Tile info (32×32) for each land_data file.
    perms: Vec<Vec<u16>>,
    /// Zoomed-out picture of the outdoors (OVERVIEW_PX a tile), if the game's
    /// minimap pictures were there.
    pub overview: Option<ColorImage>,
    /// Caves and side areas with their place on the Sinnoh map.
    pub overlays: Vec<Overlay>,
    /// Their doors and exits.
    pub doors: Vec<Door>,
    /// Layouts of the layers ("cave", "ruins", "side").
    pub layers: HashMap<String, LayerPlan>,
    /// Item balls and hidden items.
    pub items: Vec<MapItem>,
}

impl GameMap {
    /// Picture of a cave or side area for laying over the Sinnoh map: only
    /// the ground you can stand on plus a rim of wall, the rest see-through.
    pub fn draw_overlay(&self, matrix: usize) -> Option<OverlayPic> {
        const OPX: usize = 4;
        let m = self.matrices.get(matrix)?;
        let (tw, th) = (m.w * CHUNK, m.h * CHUNK);
        if tw == 0 || th == 0 || tw * th > 512 * 512 {
            return None;
        }
        let free: Vec<bool> = (0..tw * th).map(|i| self.ground(matrix, i % tw, i / tw) != Ground::Blocked).collect();
        // Only what you can walk to from its doors (the rest of a map is
        // often unused floor around the real room).
        let mut seeds: Vec<(usize, usize)> = Vec::new();
        for d in &self.doors {
            if d.matrix as usize == matrix {
                for y in d.y..=d.y2 {
                    for x in d.x..=d.x2 {
                        seeds.push((x as usize, y as usize));
                    }
                }
            }
            if d.to_matrix as usize == matrix {
                seeds.push((d.tx as usize, d.ty as usize));
            }
        }
        seeds.retain(|&(x, y)| x < tw && y < th);
        let open = if seeds.is_empty() {
            free
        } else {
            let mut open = vec![false; tw * th];
            for &(x, y) in &seeds {
                open[y * tw + x] = true;
            }
            while let Some((x, y)) = seeds.pop() {
                let mut next = |nx: usize, ny: usize| {
                    let i = ny * tw + nx;
                    if free[i] && !open[i] {
                        open[i] = true;
                        seeds.push((nx, ny));
                    }
                };
                if x + 1 < tw {
                    next(x + 1, y);
                }
                if x > 0 {
                    next(x - 1, y);
                }
                if y + 1 < th {
                    next(x, y + 1);
                }
                if y > 0 {
                    next(x, y - 1);
                }
            }
            open
        };
        // Bounds of the walkable part, with a tile of wall around it.
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
        for ty in 0..th {
            for tx in 0..tw {
                if open[ty * tw + tx] {
                    x0 = x0.min(tx);
                    y0 = y0.min(ty);
                    x1 = x1.max(tx);
                    y1 = y1.max(ty);
                }
            }
        }
        if x0 == usize::MAX {
            return None;
        }
        let (x0, y0) = (x0.saturating_sub(1), y0.saturating_sub(1));
        let (x1, y1) = ((x1 + 1).min(tw - 1), (y1 + 1).min(th - 1));
        let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
        let (w, h) = (cw * OPX, ch * OPX);
        let mut px = vec![Color32::TRANSPARENT; w * h];
        let wall = Color32::from_rgb(52, 46, 44);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let c = if open[ty * tw + tx] {
                    Some(ground_colour(self.ground(matrix, tx, ty)))
                } else {
                    // wall only where it touches floor
                    let mut near = false;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let (nx, ny) = (tx as i32 + dx, ty as i32 + dy);
                            if nx >= 0 && ny >= 0 && (nx as usize) < tw && (ny as usize) < th && open[ny as usize * tw + nx as usize] {
                                near = true;
                            }
                        }
                    }
                    near.then_some(wall)
                };
                if let Some(c) = c {
                    for yy in 0..OPX {
                        for xx in 0..OPX {
                            let col = if (xx == 0 || yy == 0) && c != wall { shade(c, 0.93) } else { c };
                            px[((ty - y0) * OPX + yy) * w + (tx - x0) * OPX + xx] = col;
                        }
                    }
                }
            }
        }
        Some(OverlayPic {
            img: ColorImage { size: [w, h], pixels: px },
            rect: [x0 as f32, y0 as f32, cw as f32, ch as f32],
        })
    }

    pub fn zone(&self, id: u16) -> Option<&Zone> {
        self.zone_index.get(&id).map(|&i| &self.zones[i])
    }

    /// The outdoors.
    pub fn world(&self) -> &Matrix {
        &self.matrices[0]
    }

    /// Tile info at a tile of a map.
    pub fn perm(&self, matrix: usize, x: usize, y: usize) -> Option<u16> {
        let m = self.matrices.get(matrix)?;
        let land = m.land_at(x / CHUNK, y / CHUNK)?;
        self.perms.get(land as usize)?.get((y % CHUNK) * CHUNK + x % CHUNK).copied()
    }

    pub fn ground(&self, matrix: usize, x: usize, y: usize) -> Ground {
        self.perm(matrix, x, y).map(Ground::from_perm).unwrap_or(Ground::Blocked)
    }

    /// The area at a tile of the outdoors.
    pub fn world_zone_at(&self, x: usize, y: usize) -> Option<u16> {
        let m = self.world();
        let (cx, cy) = (x / CHUNK, y / CHUNK);
        m.land_at(cx, cy)?;
        m.header_at(cx, cy).filter(|&h| h != 0 && self.zone_index.contains_key(&h))
    }

    /// Whether an outdoor map square belongs to a real place (a route, city,
    /// lake…) as opposed to the filler scenery between them.
    pub fn is_place_square(&self, cx: usize, cy: usize) -> bool {
        self.world_zone_at(cx * CHUNK, cy * CHUNK).is_some()
    }

    /// The map squares an area covers, in its map.
    pub fn zone_cells(&self, id: u16) -> Vec<(usize, usize)> {
        let Some(z) = self.zone(id) else { return Vec::new() };
        let Some(m) = self.matrices.get(z.matrix as usize) else { return Vec::new() };
        let mut out = Vec::new();
        for cy in 0..m.h {
            for cx in 0..m.w {
                if m.land_at(cx, cy).is_none() {
                    continue;
                }
                let mine = match m.header_at(cx, cy) {
                    Some(h) => h == id,
                    None => true,
                };
                if mine {
                    out.push((cx, cy));
                }
            }
        }
        out
    }

    /// The tiles of an area where `spots` encounters happen.
    pub fn spot_tiles(&self, id: u16, spots: Spots) -> Vec<(usize, usize)> {
        let Some(z) = self.zone(id) else { return Vec::new() };
        let mi = z.matrix as usize;
        let mut out = Vec::new();
        for (cx, cy) in self.zone_cells(id) {
            for ty in 0..CHUNK {
                for tx in 0..CHUNK {
                    let (x, y) = (cx * CHUNK + tx, cy * CHUNK + ty);
                    if spots.has(self.ground(mi, x, y)) {
                        out.push((x, y));
                    }
                }
            }
        }
        out
    }

    /// Areas a wiki place covers.
    pub fn place_zones(&self, place: &str) -> Option<&PlaceLink> {
        self.places.get(place)
    }

    /// Wiki places that are this area.
    pub fn zone_places(&self, id: u16) -> Vec<&str> {
        let mut v: Vec<&str> = self.places.iter().filter(|(_, p)| p.zones.contains(&id)).map(|(k, _)| k.as_str()).collect();
        v.sort();
        v
    }

    /// A drawn picture of an indoor map (PX a tile), from its tile info.
    pub fn draw_matrix(&self, matrix: usize) -> Option<ColorImage> {
        let m = self.matrices.get(matrix)?;
        let (tw, th) = (m.w * CHUNK, m.h * CHUNK);
        let (w, h) = (tw * PX, th * PX);
        if w * h > 2048 * 2048 * 2 {
            return None;
        }
        let mut px = vec![Color32::from_rgb(24, 26, 30); w * h];
        for ty in 0..th {
            for tx in 0..tw {
                let g = self.ground(matrix, tx, ty);
                let c = ground_colour(g);
                let blocked = g == Ground::Blocked;
                for yy in 0..PX {
                    for xx in 0..PX {
                        let mut col = c;
                        if !blocked && (xx == 0 || yy == 0) {
                            col = shade(c, 0.93);
                        }
                        px[(ty * PX + yy) * w + tx * PX + xx] = col;
                    }
                }
                // Walls: a darker rim where floor meets wall.
                if !blocked {
                    for (dx, dy) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                        let (nx, ny) = (tx as i32 + dx, ty as i32 + dy);
                        let wall = nx < 0 || ny < 0 || nx >= tw as i32 || ny >= th as i32 || self.ground(matrix, nx as usize, ny as usize) == Ground::Blocked;
                        if wall {
                            for k in 0..PX {
                                let (xx, yy) = match (dx, dy) {
                                    (0, -1) => (k, 0),
                                    (0, 1) => (k, PX - 1),
                                    (-1, 0) => (0, k),
                                    _ => (PX - 1, k),
                                };
                                px[(ty * PX + yy) * w + tx * PX + xx] = Color32::from_rgb(70, 66, 60);
                            }
                        }
                    }
                }
            }
        }
        Some(ColorImage { size: [w, h], pixels: px })
    }
}

fn shade(c: Color32, k: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8)
}

/// Colours for drawn (indoor) maps.
pub fn ground_colour(g: Ground) -> Color32 {
    match g {
        Ground::Blocked => Color32::from_rgb(38, 40, 46),
        Ground::Floor => Color32::from_rgb(176, 168, 150),
        Ground::Cave => Color32::from_rgb(156, 132, 104),
        Ground::Grass => Color32::from_rgb(92, 168, 84),
        Ground::Snow => Color32::from_rgb(222, 230, 240),
        Ground::Marsh => Color32::from_rgb(120, 100, 70),
        Ground::Water => Color32::from_rgb(84, 136, 214),
    }
}

/// The game's picture of one outdoor map square (256×256), if it's there.
pub fn load_minimap(dir: &Path, cx: usize, cy: usize) -> Option<ColorImage> {
    let bytes = std::fs::read(dir.join("minimap").join(format!("{cx}_{cy}.png"))).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// A cut-out of a map: the game's pictures outdoors, a drawn one indoors.
#[allow(dead_code)]
/// `x0, y0` is the top-left tile; the picture is PX pixels a tile.
pub struct Crop {
    pub image: ColorImage,
    pub x0: usize,
    pub y0: usize,
    pub tiles_w: usize,
    pub tiles_h: usize,
}

impl GameMap {
    /// Cut out tiles `x0..x1, y0..y1` of a map (end exclusive).
    pub fn crop(&self, matrix: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> Option<Crop> {
        let (tw, th) = (x1.checked_sub(x0)?, y1.checked_sub(y0)?);
        if tw == 0 || th == 0 || tw * th > 400 * 400 {
            return None;
        }
        let (w, h) = (tw * PX, th * PX);
        let mut px = vec![Color32::from_rgb(24, 26, 30); w * h];
        if matrix == 0 {
            // Stitch the game's pictures of each square the crop touches.
            for cy in y0 / CHUNK..=(y1 - 1) / CHUNK {
                for cx in x0 / CHUNK..=(x1 - 1) / CHUNK {
                    let Some(img) = load_minimap(&self.dir, cx, cy) else { continue };
                    if img.size != [CHUNK * PX, CHUNK * PX] {
                        continue;
                    }
                    for py in 0..CHUNK * PX {
                        let wy = cy * CHUNK * PX + py;
                        if wy < y0 * PX || wy >= y1 * PX {
                            continue;
                        }
                        for pxx in 0..CHUNK * PX {
                            let wx = cx * CHUNK * PX + pxx;
                            if wx < x0 * PX || wx >= x1 * PX {
                                continue;
                            }
                            px[(wy - y0 * PX) * w + (wx - x0 * PX)] = img.pixels[py * CHUNK * PX + pxx];
                        }
                    }
                }
            }
        } else {
            let full = self.draw_matrix(matrix)?;
            for y in 0..h {
                for x in 0..w {
                    let (sx, sy) = (x0 * PX + x, y0 * PX + y);
                    if sx < full.size[0] && sy < full.size[1] {
                        px[y * w + x] = full.pixels[sy * full.size[0] + sx];
                    }
                }
            }
        }
        Some(Crop {
            image: ColorImage { size: [w, h], pixels: px },
            x0,
            y0,
            tiles_w: tw,
            tiles_h: th,
        })
    }
}

/// Bounding box (x0, y0, x1, y1 exclusive) of some tiles.
pub fn bbox(tiles: &[(usize, usize)]) -> Option<(usize, usize, usize, usize)> {
    let x0 = tiles.iter().map(|t| t.0).min()?;
    let y0 = tiles.iter().map(|t| t.1).min()?;
    let x1 = tiles.iter().map(|t| t.0).max()? + 1;
    let y1 = tiles.iter().map(|t| t.1).max()? + 1;
    Some((x0, y0, x1, y1))
}

// ───────────────────────────── loading ─────────────────────────────

/// Why the map couldn't be read.
#[derive(Debug, Clone)]
pub enum LoadError {
    NotInstalled(PathBuf),
    Broken(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotInstalled(p) => write!(f, "Platinum Together isn't installed where expected ({})", p.display()),
            LoadError::Broken(e) => write!(f, "{e}"),
        }
    }
}

pub fn load(dir: &Path, with_overview: bool) -> Result<GameMap, LoadError> {
    let zf: ZoneFile = serde_json::from_str(ZONES_JSON).map_err(|e| LoadError::Broken(format!("zones.json: {e}")))?;
    let matrix_path = game_file(dir, "fielddata/mapmatrix/map_matrix.narc");
    let land_path = game_file(dir, "fielddata/land_data/land_data.narc");
    if !matrix_path.exists() || !land_path.exists() {
        return Err(LoadError::NotInstalled(dir.to_path_buf()));
    }
    let mb = std::fs::read(&matrix_path).map_err(|e| LoadError::Broken(e.to_string()))?;
    let matrices: Vec<Matrix> = narc_files(&mb)
        .ok_or_else(|| LoadError::Broken("map_matrix.narc isn't readable".into()))?
        .into_iter()
        .map(|f| Matrix::parse(f).unwrap_or_default())
        .collect();
    if matrices.first().map(|m| m.w * m.h) != Some(900) {
        return Err(LoadError::Broken("the outdoor map isn't the expected size".into()));
    }
    let lb = std::fs::read(&land_path).map_err(|e| LoadError::Broken(e.to_string()))?;
    let perms: Vec<Vec<u16>> = narc_files(&lb)
        .ok_or_else(|| LoadError::Broken("land_data.narc isn't readable".into()))?
        .into_iter()
        .map(|f| {
            if f.len() >= 16 + CHUNK * CHUNK * 2 {
                (0..CHUNK * CHUNK).map(|i| u16le(f, 16 + i * 2)).collect()
            } else {
                vec![0x8000; CHUNK * CHUNK]
            }
        })
        .collect();
    drop(lb);
    let zone_index = zf.zones.iter().enumerate().map(|(i, z)| (z.id, i)).collect();
    let game_species = zf.game_species.into_iter().filter_map(|(k, v)| Some((k.parse().ok()?, v))).collect();
    let mut map = GameMap {
        dir: dir.to_path_buf(),
        zones: zf.zones,
        zone_index,
        places: zf.places,
        game_species,
        matrices,
        perms,
        overview: None,
        overlays: zf.overlays,
        layers: zf.layers,
        items: zf
            .items
            .into_iter()
            .map(|(matrix, x, y, h, name, note)| MapItem {
                matrix,
                x,
                y,
                hidden: h != 0,
                name,
                note,
            })
            .collect(),
        doors: zf
            .doors
            .iter()
            .map(|d| Door {
                matrix: d[0],
                x: d[1],
                y: d[2],
                x2: d[3],
                y2: d[4],
                to_zone: d[5],
                to_matrix: d[6],
                tx: d[7],
                ty: d[8],
            })
            .collect(),
    };
    if with_overview {
        map.overview = build_overview(&map);
    }
    Ok(map)
}

/// The whole outdoors at OVERVIEW_PX a tile, from the game's pictures
/// (or drawn from the tile info where a picture is missing).
fn build_overview(map: &GameMap) -> Option<ColorImage> {
    let m = map.world();
    let side = m.w * CHUNK * OVERVIEW_PX;
    let mut px = vec![Color32::TRANSPARENT; side * side];
    let step = PX / OVERVIEW_PX;
    let mut any = false;
    for cy in 0..m.h {
        for cx in 0..m.w {
            // Only real places are drawn; filler scenery is left out.
            if !map.is_place_square(cx, cy) {
                continue;
            }
            let img = load_minimap(&map.dir, cx, cy);
            for ty in 0..CHUNK * OVERVIEW_PX {
                for tx in 0..CHUNK * OVERVIEW_PX {
                    let c = match &img {
                        Some(img) if img.size == [CHUNK * PX, CHUNK * PX] => {
                            // average a step×step block
                            let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
                            for yy in 0..step {
                                for xx in 0..step {
                                    let p = img.pixels[(ty * step + yy) * CHUNK * PX + tx * step + xx];
                                    r += p.r() as u32;
                                    g += p.g() as u32;
                                    b += p.b() as u32;
                                }
                            }
                            let n = (step * step) as u32;
                            any = true;
                            Color32::from_rgb((r / n) as u8, (g / n) as u8, (b / n) as u8)
                        }
                        _ => ground_colour(map.ground(0, cx * CHUNK + tx / OVERVIEW_PX, cy * CHUNK + ty / OVERVIEW_PX)),
                    };
                    px[(cy * CHUNK * OVERVIEW_PX + ty) * side + cx * CHUNK * OVERVIEW_PX + tx] = c;
                }
            }
        }
    }
    let _ = any;
    fill_sea_and_land(m, &|cx, cy| map.is_place_square(cx, cy), &mut px, side);
    Some(ColorImage { size: [side, side], pixels: px })
}

/// Open sea around Sinnoh (also the Map's background colour).
pub const OCEAN: Color32 = Color32::from_rgb(34, 96, 148);
const SHALLOWS: Color32 = Color32::from_rgb(64, 142, 190);

/// Squares that aren't a real place (no picture, or just filler scenery)
/// left the map looking patchy. Fill them in: such squares that reach the edge of
/// the world are sea (lighter along the coast), and empty squares shut in
/// by land take the colour of the land around them.
fn fill_sea_and_land(m: &Matrix, mapped: &dyn Fn(usize, usize) -> bool, px: &mut [Color32], side: usize) {
    let cs = CHUNK * OVERVIEW_PX;
    let (w, h) = (m.w, m.h);
    if w == 0 || h == 0 || side != w * cs {
        return;
    }
    // 0 = has a map, 1 = sea, 2 = shut in by land
    let mut kind: Vec<u8> = // (Everything that isn't a real place is sea now, inland gaps included;
    // the shut-in handling below only applies if a square is marked 2.)
    (0..w * h).map(|i| if mapped(i % w, i / w) { 0 } else { 1 }).collect();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for cy in 0..h {
        for cx in 0..w {
            if (cx == 0 || cy == 0 || cx == w - 1 || cy == h - 1) && kind[cy * w + cx] == 2 {
                kind[cy * w + cx] = 1;
                stack.push((cx, cy));
            }
        }
    }
    while let Some((cx, cy)) = stack.pop() {
        let around = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
        for (nx, ny) in around {
            if nx < w && ny < h && kind[ny * w + nx] == 2 {
                kind[ny * w + nx] = 1;
                stack.push((nx, ny));
            }
        }
    }
    // Average colour of every mapped square, then spread it into the
    // shut-in squares from their neighbours.
    let mut avg: Vec<Option<[f32; 3]>> = vec![None; w * h];
    for cy in 0..h {
        for cx in 0..w {
            if kind[cy * w + cx] != 0 {
                continue;
            }
            let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
            for y in 0..cs {
                for x in 0..cs {
                    let c = px[(cy * cs + y) * side + cx * cs + x];
                    r += c.r() as u64;
                    g += c.g() as u64;
                    b += c.b() as u64;
                }
            }
            let n = (cs * cs) as f32;
            avg[cy * w + cx] = Some([r as f32 / n, g as f32 / n, b as f32 / n]);
        }
    }
    for _ in 0..(w + h) {
        let mut next = avg.clone();
        let mut changed = false;
        for cy in 0..h {
            for cx in 0..w {
                if kind[cy * w + cx] != 2 || avg[cy * w + cx].is_some() {
                    continue;
                }
                let around = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];
                let (mut sum, mut n) = ([0.0f32; 3], 0.0f32);
                for (nx, ny) in around {
                    if nx < w && ny < h {
                        if let Some(c) = avg[ny * w + nx] {
                            for k in 0..3 {
                                sum[k] += c[k];
                            }
                            n += 1.0;
                        }
                    }
                }
                if n > 0.0 {
                    next[cy * w + cx] = Some([sum[0] / n, sum[1] / n, sum[2] / n]);
                    changed = true;
                }
            }
        }
        avg = next;
        if !changed {
            break;
        }
    }
    let band = 16.0f32; // how far the lighter coastal water reaches, in pixels
    for cy in 0..h {
        for cx in 0..w {
            let k = kind[cy * w + cx];
            if k == 0 {
                continue;
            }
            for y in 0..cs {
                for x in 0..cs {
                    let (gx, gy) = (cx * cs + x, cy * cs + y);
                    let c = if k == 2 {
                        // Land nobody can reach: the colour around it, a touch
                        // darker and grainy so it reads as rough ground.
                        let a = avg[cy * w + cx].unwrap_or([70.0, 110.0, 70.0]);
                        let grain = (((gx.wrapping_mul(73_856_093)) ^ (gy.wrapping_mul(19_349_663))) >> 7) % 9;
                        let f = 0.86 + grain as f32 * 0.012;
                        Color32::from_rgb((a[0] * f) as u8, (a[1] * f) as u8, (a[2] * f) as u8)
                    } else {
                        // Sea: lighter the nearer it is to a coast.
                        let mut d = band;
                        for dy in -1i32..=1 {
                            for dx in -1i32..=1 {
                                let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                                if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                                    continue;
                                }
                                if kind[ny as usize * w + nx as usize] == 1 {
                                    continue;
                                }
                                let ddx = match dx {
                                    -1 => x as f32 + 1.0,
                                    1 => (cs - x) as f32,
                                    _ => 0.0,
                                };
                                let ddy = match dy {
                                    -1 => y as f32 + 1.0,
                                    1 => (cs - y) as f32,
                                    _ => 0.0,
                                };
                                d = d.min((ddx * ddx + ddy * ddy).sqrt());
                            }
                        }
                        let t = (1.0 - d / band).clamp(0.0, 1.0);
                        let t = t * t;
                        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
                        Color32::from_rgb(mix(OCEAN.r(), SHALLOWS.r()), mix(OCEAN.g(), SHALLOWS.g()), mix(OCEAN.b(), SHALLOWS.b()))
                    };
                    px[gy * side + gx] = c;
                }
            }
        }
    }
}

/// Read the map in the background.
pub fn start() -> Receiver<Result<Arc<GameMap>, LoadError>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().name("gamemap".into()).spawn(move || {
        let _ = tx.send(load(&game_dir(), true).map(Arc::new));
    });
    rx
}

// ───────────────────────────── rare grass ─────────────────────────────

/// A rare grass record from today's game logs.
#[derive(Debug, Clone)]
pub struct Rare {
    /// The map it's on: 0 outdoors, otherwise a side area's (a lake,
    /// Eterna Forest…). The game's log only gives the tile, so the map is
    /// worked out from which one has grass there.
    pub matrix: u16,
    pub x: usize,
    pub y: usize,
    /// Game species number (None for a "patch found here" scan).
    pub species: Option<u32>,
    /// How many rare grass tiles a scan found around here.
    #[allow(dead_code)]
    pub tiles: u32,
}

/// Days since 1970-01-01 for a calendar date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Unix time of midnight, Central European time, at the start of a day
/// (days since 1970 on the server's calendar). Summer time never changes
/// near midnight, so the offset an hour before is the one at midnight.
fn server_midnight(day: i64) -> i64 {
    day * 86400 - crate::worldclock::SERVER.offset_at(day * 86400 - 3600)
}

/// Unix time of the last daily reset: midnight, Central European time.
pub fn last_reset(now_unix: i64) -> i64 {
    let off = crate::worldclock::SERVER.offset_at(now_unix);
    server_midnight((now_unix + off).div_euclid(86400))
}

/// Unix time of the next daily reset.
pub fn next_reset(now_unix: i64) -> i64 {
    let off = crate::worldclock::SERVER.offset_at(now_unix);
    server_midnight((now_unix + off).div_euclid(86400) + 1)
}

/// "session-20261111-110446.log" → unix time as if that local time were UTC.
fn session_local(name: &str) -> Option<i64> {
    let s = name.strip_prefix("session-")?.strip_suffix(".log")?;
    let (d, t) = s.split_once('-')?;
    let n = |a: &str| a.parse::<i64>().ok();
    let (y, mo, da) = (n(&d[0..4])?, n(&d[4..6])?, n(&d[6..8])?);
    let (h, mi, se) = (n(&t[0..2])?, n(&t[2..4])?, n(&t[4..6])?);
    Some(days_from_civil(y, mo, da) * 86400 + h * 3600 + mi * 60 + se)
}

fn unix(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
}

/// Rare grass seen since the last daily reset, from the game's logs.
/// Only spots outdoors (in the outdoor map) are returned.
pub fn read_rare_today(dir: &Path, map: &GameMap) -> Vec<Rare> {
    let logs = dir.join("logs");
    let Ok(rd) = std::fs::read_dir(&logs) else { return Vec::new() };
    let now = unix(std::time::SystemTime::now()).unwrap_or(0);
    let reset = last_reset(now);
    let mut files: Vec<(String, PathBuf, i64)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let modified = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(unix)?;
            (name.starts_with("session-") && name.ends_with(".log") && modified >= reset).then(|| (name, e.path(), modified))
        })
        .collect();
    files.sort();
    // The player's UTC offset: a log is created when its name's local time was now.
    let offset = files
        .iter()
        .rev()
        .find_map(|(name, path, _)| {
            let local = session_local(name)?;
            let created = std::fs::metadata(path).ok()?.created().ok().and_then(unix)?;
            let off = local - created;
            Some(((off as f64 / 900.0).round() as i64) * 900)
        })
        .unwrap_or(0);
    let mut out: Vec<Rare> = Vec::new();
    for (name, path, _) in &files {
        let Some(start_local) = session_local(name) else { continue };
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        let day0 = start_local.div_euclid(86400) * 86400;
        let mut prev = start_local - day0;
        let mut days = 0i64;
        for line in text.lines() {
            if !line.contains(" RARE ") {
                continue;
            }
            let Some(secs) = line.get(0..8).and_then(|t| {
                let p: Vec<i64> = t.split(':').filter_map(|v| v.parse().ok()).collect();
                (p.len() == 3).then(|| p[0] * 3600 + p[1] * 60 + p[2])
            }) else {
                continue;
            };
            if secs + 3600 < prev {
                days += 1;
            }
            prev = secs;
            let when = day0 + days * 86400 + secs - offset;
            if when < reset {
                continue;
            }
            if let Some(r) = parse_rare(line) {
                out.push(r);
            }
        }
    }
    // Today's spots shared by the dev (bundled with the app, or the newer
    // copy on GitHub), so nobody has to find every patch themselves.
    for (x, y, sp) in shared_rare(reset) {
        out.push(Rare {
            matrix: 0,
            x,
            y,
            species: Some(sp),
            tiles: 0,
        });
    }
    let out = map.place_rare(out);
    // Only spots where a Pokémon was actually met count: the "N rare grass
    // tiles around x,y" scan lines also fire where there's no rare grass.
    let mut seen = std::collections::HashSet::new();
    out.into_iter().filter(|r| r.species.is_some() && seen.insert((r.matrix, r.x, r.y, r.species))).collect()
}

impl GameMap {
    /// The area a rare grass spot is in.
    pub fn rare_zone(&self, matrix: u16, x: usize, y: usize) -> Option<u16> {
        if matrix == 0 {
            self.world_zone_at(x, y)
        } else {
            let m = self.matrices.get(matrix as usize)?;
            m.header_at(x / CHUNK, y / CHUNK)
                .filter(|h| self.overlays.iter().any(|o| o.zone == *h))
                .or_else(|| self.overlays.iter().find(|o| o.matrix == matrix).map(|o| o.zone))
        }
    }

    /// Tall grass (or snowy grass) at a tile of a map.
    fn grassy(&self, matrix: usize, x: usize, y: usize) -> bool {
        matches!(self.ground(matrix, x, y), Ground::Grass | Ground::Snow)
    }

    /// Work out which map each rare grass spot is on: outdoors when
    /// there's grass there, else the side area (lake, forest…) that has
    /// grass on that tile, preferring places other spots are already on.
    /// Spots on no map at all are dropped.
    fn place_rare(&self, spots: Vec<Rare>) -> Vec<Rare> {
        let mut out: Vec<Rare> = Vec::new();
        let mut open: Vec<(Rare, Vec<u16>)> = Vec::new();
        for mut r in spots {
            let land = self.world().land_at(r.x / CHUNK, r.y / CHUNK).is_some();
            if r.species.is_none() || (land && self.grassy(0, r.x, r.y)) {
                if land {
                    r.matrix = 0;
                    out.push(r);
                }
                continue;
            }
            let mut seen = HashSet::new();
            let cands: Vec<u16> = self
                .overlays
                .iter()
                .filter(|o| seen.insert(o.matrix) && self.grassy(o.matrix as usize, r.x, r.y))
                .map(|o| o.matrix)
                .collect();
            match cands.len() {
                0 if land => out.push(r),
                0 => {}
                1 => {
                    r.matrix = cands[0];
                    out.push(r);
                }
                _ => open.push((r, cands)),
            }
        }
        // Several maps fit: take the one whose place has the most other spots.
        let group = |m: u16| self.overlays.iter().find(|o| o.matrix == m).map(|o| o.group.clone()).unwrap_or_default();
        let mut count: HashMap<String, usize> = HashMap::new();
        for r in &out {
            if r.matrix != 0 {
                *count.entry(group(r.matrix)).or_default() += 1;
            }
        }
        for (mut r, cands) in open {
            let best = cands
                .iter()
                .copied()
                .max_by_key(|m| (count.get(&group(*m)).copied().unwrap_or(0), std::cmp::Reverse(*m)))
                .unwrap_or(cands[0]);
            r.matrix = best;
            out.push(r);
        }
        out
    }
}

/// Rare grass shipped with this version (for the day it was made).
const SHARED_RARE: &str = include_str!("rare_today.json");
/// The latest shared list, updated on GitHub without a new release.
const SHARED_RARE_URL: &str = "https://raw.githubusercontent.com/ZifuM/Shinytracker/main/data/rare-today.json";

#[derive(Deserialize)]
struct SharedRare {
    reset: i64,
    spots: Vec<(usize, usize, u32)>,
}

/// Shared rare grass for the day that started at `reset` (empty when the
/// shared lists are for another day). GitHub is asked at most every 20 minutes.
fn shared_rare(reset: i64) -> Vec<(usize, usize, u32)> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static REMOTE: Mutex<Option<(Instant, Option<SharedRare>)>> = Mutex::new(None);
    let mut out = Vec::new();
    if let Ok(b) = serde_json::from_str::<SharedRare>(SHARED_RARE) {
        if b.reset == reset {
            out.extend(b.spots);
        }
    }
    let Ok(mut g) = REMOTE.lock() else { return out };
    let stale = g.as_ref().map_or(true, |(t, _)| t.elapsed() > Duration::from_secs(20 * 60));
    if stale {
        // Fetch on its own thread (this one also loads the map pictures);
        // the result is used from the next look onwards.
        let keep = g.take().and_then(|(_, r)| r);
        *g = Some((Instant::now(), keep));
        std::thread::spawn(|| {
            if let Some(got) = crate::sprites::fetch(SHARED_RARE_URL).ok().and_then(|b| serde_json::from_slice::<SharedRare>(&b).ok()) {
                if let Ok(mut g) = REMOTE.lock() {
                    *g = Some((Instant::now(), Some(got)));
                }
            }
        });
    }
    if let Some((_, Some(r))) = g.as_ref() {
        if r.reset == reset {
            out.extend(r.spots.iter().copied());
        }
    }
    out
}

fn nums(s: &str) -> Vec<u64> {
    s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect()
}

/// "RARE 5 rare grass tiles around 120,840 on 11-11-2026" or
/// "RARE rare grass at 121,842: species 371 from 6 choices (route level 30)".
fn parse_rare(line: &str) -> Option<Rare> {
    if let Some(i) = line.find("rare grass tiles around ") {
        let head = nums(&line[..i]);
        let n = *head.last()? as u32;
        let rest = &line[i + 24..];
        let xy = nums(rest.split(" on ").next()?);
        return Some(Rare {
            matrix: 0,
            x: *xy.first()? as usize,
            y: *xy.get(1)? as usize,
            species: None,
            tiles: n,
        });
    }
    if let Some(i) = line.find("rare grass at ") {
        let rest = &line[i + 14..];
        let (xy, sp) = rest.split_once(": species ")?;
        let xy = nums(xy);
        let species = *nums(sp).first()? as u32;
        return Some(Rare {
            matrix: 0,
            x: *xy.first()? as usize,
            y: *xy.get(1)? as usize,
            species: Some(species),
            tiles: 1,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resets_at_cet_midnight() {
        // 2026-11-11 10:00 UTC → reset was 2026-11-10 23:00 UTC (CET, +1)
        let t = days_from_civil(2026, 11, 11) * 86400 + 10 * 3600;
        assert_eq!(last_reset(t), days_from_civil(2026, 11, 10) * 86400 + 23 * 3600);
        // summer: 2026-07-01 12:00 UTC → 2026-06-30 22:00 UTC (CEST, +2)
        let t = days_from_civil(2026, 7, 1) * 86400 + 12 * 3600;
        assert_eq!(last_reset(t), days_from_civil(2026, 6, 30) * 86400 + 22 * 3600);
        // the day summer time starts: 2026-03-29 12:00 UTC → midnight was
        // still CET (2026-03-28 23:00 UTC), and the next one is CEST
        let t = days_from_civil(2026, 3, 29) * 86400 + 12 * 3600;
        assert_eq!(last_reset(t), days_from_civil(2026, 3, 28) * 86400 + 23 * 3600);
        assert_eq!(next_reset(t), days_from_civil(2026, 3, 29) * 86400 + 22 * 3600);
        // the day before it ends: next midnight is still CEST
        let t = days_from_civil(2026, 10, 24) * 86400 + 12 * 3600;
        assert_eq!(next_reset(t), days_from_civil(2026, 10, 24) * 86400 + 22 * 3600);
    }

    #[test]
    fn parses_rare_lines() {
        let a = parse_rare("10:12:01 INFO RARE 5 rare grass tiles around 120,840 on 11-11-2026").unwrap();
        assert_eq!((a.x, a.y, a.species, a.tiles), (120, 840, None, 5));
        let b = parse_rare("10:12:09 INFO RARE rare grass at 121,842: species 371 from 6 choices (route level 30)").unwrap();
        assert_eq!((b.x, b.y, b.species), (121, 842, Some(371)));
        assert!(parse_rare("10:12:09 INFO RARE sweet scent: evolve roll 1 of 4, species 371 becomes 372").is_none());
        assert_eq!(session_local("session-19700102-000001.log"), Some(86401));
    }

    #[test]
    fn grounds() {
        assert_eq!(Ground::from_perm(0x0002), Ground::Grass);
        assert_eq!(Ground::from_perm(0x8000), Ground::Blocked);
        assert_eq!(Ground::from_perm(0x0000), Ground::Floor);
        assert_eq!(Ground::from_perm(0x0015), Ground::Water);
        assert!(Spots::Land.has(Ground::Snow));
        assert!(!Spots::Land.has(Ground::Water));
    }

    #[test]
    fn zones_json_reads() {
        let zf: ZoneFile = serde_json::from_str(ZONES_JSON).unwrap();
        assert!(zf.zones.iter().any(|z| z.code == "R201" && z.name == "Route 201"));
        assert!(zf.places.contains_key("route_201"));
    }
}
