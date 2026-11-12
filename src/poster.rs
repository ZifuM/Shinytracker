//! The rare grass infographic (adapted from the Sparkle Map's share image):
//! the Sinnoh map with today's rare grass pinned on it at the exact tiles
//! from the game log, the daily reset time around the world, the most
//! widespread Pokémon and where to find each one.
//!
//! Drawing only: no egui, no font files. Text goes through `PosterText`
//! (Segoe UI and Bahnschrift in the app, see `poster_text.rs`).

use crate::canvas::{crop, Canvas, Col, Img};
use crate::worldclock;

#[path = "poster/anime.rs"]
mod anime;

/// Text styles: condensed display weights (titles, numbers) and text weights.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PFace {
    D800,
    D700,
    D600,
    T600,
    T500,
    /// Heavy italic for the anime style's lettering (Segoe UI Black Italic).
    Black,
    /// Bold text for the anime style (Segoe UI Bold).
    Bold,
}

/// The look: flat anime infographic (new) or the original sea-blue one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PosterStyle {
    #[default]
    Anime,
    Classic,
}

/// Measures and draws text in pixels, at a baseline.
pub trait PosterText {
    fn width(&self, s: &str, px: f32, face: PFace) -> f32;
    fn draw(&self, c: &mut Canvas, s: &str, x: f32, baseline: f32, px: f32, col: Col, face: PFace);
}

/// A Pokémon met on rare grass in an area.
#[derive(Clone)]
pub struct PosterMon {
    /// Same for the same Pokémon everywhere (its slug), for the top 3.
    pub key: String,
    pub name: String,
    pub sprite: Option<Img>,
    /// Rare grass tiles it was met on there.
    pub spots: usize,
}

/// An area with rare grass today.
#[derive(Clone)]
pub struct PosterArea {
    pub name: String,
    pub mons: Vec<PosterMon>,
    /// Every rare grass tile met on here: (x, y) on the Sinnoh map in
    /// tiles, and which of `mons` was met there. Pins go on these.
    pub tiles: Vec<(f32, f32, usize)>,
}

/// A city or town name on the map: the top middle of the place, in tiles.
#[derive(Clone)]
pub struct PosterLabel {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub city: bool,
}

/// The Sinnoh map picture (the Sparkle Map's own drawing): tile (x0, y0)
/// is its top-left pixel; `fit` is the part (tiles: x, y, w, h) that's fitted
/// to the height of the top, the rest (open sea) runs off the edges.
#[derive(Clone)]
pub struct PosterMap {
    pub img: Img,
    pub x0: f32,
    pub y0: f32,
    pub px_per_tile: f32,
    pub fit: (f32, f32, f32, f32),
}

#[derive(Clone)]
pub struct PosterData {
    pub areas: Vec<PosterArea>,
    pub labels: Vec<PosterLabel>,
    pub map: Option<PosterMap>,
    /// The daily reset that started today (unix seconds).
    pub reset: i64,
    /// The next one.
    pub next: i64,
}

/// What to put in the picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PosterMode {
    /// Everything.
    #[default]
    Full,
    /// The map, title and reset times.
    MapOnly,
    /// The top 3 and where to find everything (no map).
    Lists,
}

impl PosterMode {
    pub fn label(self) -> &'static str {
        match self {
            PosterMode::Full => "Full infographic",
            PosterMode::MapOnly => "Map only",
            PosterMode::Lists => "Top 3 & locations",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            PosterMode::Full => "The map, reset times, top 3 and every location",
            PosterMode::MapOnly => "Just the map, title and reset times",
            PosterMode::Lists => "Just the top 3 and where to find them",
        }
    }
    pub fn file_tag(self) -> &'static str {
        match self {
            PosterMode::Full => "rare-grass",
            PosterMode::MapOnly => "rare-grass-map",
            PosterMode::Lists => "rare-grass-list",
        }
    }
}

// ───────────────────────────── colours ─────────────────────────────

const fn hex(v: u32) -> Col {
    Col((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
}
const HERO: Col = hex(0x1b5a80);
const SEA: Col = hex(0x1c4a62);
const SEA_DEEP: Col = hex(0x143a4e);
const SEA_TEXT: Col = hex(0x9cc0d2);
const SEA_FAINT: Col = hex(0x5f8ea5);
const INK: Col = hex(0x13232c);
const INK2: Col = hex(0x55666e);
const PAPER: Col = hex(0xf1f4f2);
const RULE: Col = hex(0xd5dfda);
const GOLD: Col = hex(0xf2b33d);
const WHITE: Col = Col(255, 255, 255, 255);
const PALE: Col = hex(0xe8f1f5);

struct Metal {
    name: &'static str,
    hi: Col,
    mid: Col,
    lo: Col,
    ink: Col,
}
const METALS: [Metal; 3] = [
    Metal { name: "Gold", hi: hex(0xffe9a6), mid: hex(0xf2b33d), lo: hex(0xa8730f), ink: hex(0x6e4a06) },
    Metal { name: "Silver", hi: hex(0xffffff), mid: hex(0xc7d1da), lo: hex(0x7d8b97), ink: hex(0x48545e) },
    Metal { name: "Bronze", hi: hex(0xf6c99c), mid: hex(0xcd8a4f), lo: hex(0x8a5226), ink: hex(0x5a3315) },
];

// ───────────────────────────── pen ─────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Al {
    L,
    C,
    R,
}

/// Draws in layout units (the poster is 1920 wide) onto a canvas `r` times
/// bigger, so it stays sharp when zoomed in.
struct Pen<'a> {
    c: Canvas,
    r: f32,
    t: &'a dyn PosterText,
}

impl<'a> Pen<'a> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, col: Col) {
        let r = self.r;
        self.c.rrect(x * r, y * r, w * r, h * r, 0.0, col);
    }
    fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, rad: f32, col: Col) {
        let r = self.r;
        self.c.rrect(x * r, y * r, w * r, h * r, rad * r, col);
    }
    fn rrect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32, rad: f32, t: f32, col: Col) {
        let r = self.r;
        // centred on the edge like a canvas stroke
        self.c.rrect_stroke((x - t / 2.0) * r, (y - t / 2.0) * r, (w + t) * r, (h + t) * r, (rad + t / 2.0) * r, t * r, col);
    }
    fn circle(&mut self, cx: f32, cy: f32, rad: f32, col: Col) {
        let r = self.r;
        self.c.circle(cx * r, cy * r, rad * r, col);
    }
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, col: Col) {
        let r = self.r;
        self.c.ellipse(cx * r, cy * r, rx * r, ry * r, col);
    }
    fn ring(&mut self, cx: f32, cy: f32, rad: f32, t: f32, col: Col) {
        let r = self.r;
        self.c.ring(cx * r, cy * r, rad * r, t * r, col);
    }
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, col: Col) {
        let r = self.r;
        self.c.line(x0 * r, y0 * r, x1 * r, y1 * r, w * r, col);
    }
    fn poly(&mut self, pts: &[(f32, f32)], col: Col) {
        let r = self.r;
        let p: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x * r, y * r)).collect();
        self.c.poly(&p, col);
    }
    fn star(&mut self, cx: f32, cy: f32, size: f32, col: Col) {
        let r = self.r;
        self.c.sparkle(cx * r, cy * r, size * r, col);
    }
    /// A quadratic curve as short straight pieces.
    fn quad(&mut self, a: (f32, f32), ctl: (f32, f32), b: (f32, f32), w: f32, col: Col) {
        const N: usize = 6;
        let at = |t: f32| {
            let u = 1.0 - t;
            (u * u * a.0 + 2.0 * u * t * ctl.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * ctl.1 + t * t * b.1)
        };
        let mut p = a;
        for i in 1..=N {
            let q = at(i as f32 / N as f32);
            self.line(p.0, p.1, q.0, q.1, w, col);
            p = q;
        }
    }

    fn tw(&self, s: &str, size: f32, face: PFace) -> f32 {
        self.t.width(s, size * self.r, face) / self.r
    }
    fn text(&mut self, s: &str, x: f32, y: f32, size: f32, face: PFace, col: Col, al: Al) {
        let w = self.tw(s, size, face);
        let x = match al {
            Al::L => x,
            Al::C => x - w / 2.0,
            Al::R => x - w,
        };
        let r = self.r;
        self.t.draw(&mut self.c, s, x * r, y * r, size * r, col, face);
    }
    /// Text with a soft outline around it (readable over the map).
    fn halo_text(&mut self, s: &str, x: f32, y: f32, size: f32, face: PFace, col: Col, halo: Col, hw: f32) {
        let w = self.tw(s, size, face);
        let x0 = x - w / 2.0;
        let r = self.r;
        let d = hw / 2.0;
        for i in 0..12 {
            let a = i as f32 * std::f32::consts::TAU / 12.0;
            self.t.draw(&mut self.c, s, (x0 + a.cos() * d) * r, (y + a.sin() * d) * r, size * r, halo.alpha(0.55), face);
        }
        self.t.draw(&mut self.c, s, x0 * r, y * r, size * r, col, face);
    }
    /// Shorten with "…" to fit.
    fn fit(&self, s: &str, size: f32, face: PFace, maxw: f32) -> String {
        if self.tw(s, size, face) <= maxw {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while chars.len() > 1 {
            chars.pop();
            let t: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
            if self.tw(&t, size, face) <= maxw {
                return t;
            }
        }
        "…".into()
    }
    fn wrap(&self, s: &str, size: f32, face: PFace, maxw: f32) -> Vec<String> {
        let mut out = Vec::new();
        let mut line = String::new();
        for word in s.split(' ') {
            let t = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.tw(&t, size, face) > maxw && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = t;
            }
        }
        if !line.is_empty() {
            out.push(line);
        }
        out
    }

    /// A sprite's shape in one colour (nearest-neighbour scaled).
    fn mask(&mut self, img: &Img, x: f32, y: f32, scale: f32, col: Col) {
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let dw = (img.w as f32 * scale).round() as i32;
        let dh = (img.h as f32 * scale).round() as i32;
        for dy in 0..dh {
            let sy = (((dy as f32 + 0.5) / scale) as u32).min(img.h - 1);
            for dx in 0..dw {
                let sx = (((dx as f32 + 0.5) / scale) as u32).min(img.w - 1);
                let a = img.px[((sy * img.w + sx) * 4 + 3) as usize];
                if a > 0 {
                    self.c.blend(x0 + dx, y0 + dy, col, a as f32 / 255.0);
                }
            }
        }
    }

    /// A Pokémon like a sticker: white outline and a soft shadow, centred on
    /// (cx, cy), fitting a `box_` square. Returns the size drawn.
    fn sticker(&mut self, img: Option<&Img>, cx: f32, cy: f32, box_: f32, outline: f32) -> Option<(f32, f32)> {
        let img = img?;
        let (bx, by, bw, bh) = Canvas::trim_box(img);
        if bw == 0 || bh == 0 {
            return None;
        }
        let spr = crop(img, (bx, by, bw, bh));
        let k = fit_scale(bw as f32, bh as f32, box_);
        let kr0 = k * self.r;
        // whole-pixel zoom keeps pixel art sharp
        let kr = if kr0 >= 1.0 { kr0.round().max(1.0) } else { kr0 };
        let (pw, ph) = ((bw as f32 * kr).round().max(1.0), (bh as f32 * kr).round().max(1.0));
        let x = cx * self.r - pw / 2.0;
        let y = cy * self.r - ph / 2.0;
        let o = outline * self.r;
        let dirs = [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)];
        let shadow = Col(20, 35, 40, 255).alpha(0.05);
        for (dx, dy) in dirs {
            self.mask(&spr, x + dx * (o + self.r), y + dy * (o + self.r) + 2.0 * self.r, kr, shadow);
        }
        for (dx, dy) in dirs {
            self.mask(&spr, x + dx * o, y + dy * o, kr, WHITE);
        }
        if kr >= 1.0 {
            self.c.blit_nearest(&spr, x, y, kr);
        } else {
            self.c.blit_smooth(&spr, x, y, pw, ph);
        }
        Some((pw / self.r, ph / self.r))
    }

    /// A little trophy cup in metal tones, with its rank on it.
    fn trophy(&mut self, cx: f32, cy: f32, h: f32, m: &Metal, rank: usize) {
        let w = h * 0.78;
        let top = cy - h / 2.0;
        // handles (the cup covers their inner halves)
        self.ring(cx - w * 0.42, top + h * 0.25, h * 0.14, h * 0.07, m.mid);
        self.ring(cx + w * 0.42, top + h * 0.25, h * 0.14, h * 0.07, m.mid);
        // cup: a bezier on each side
        let bez = |t: f32, sgn: f32| {
            let (p0, p1, p2, p3) = ((0.42, 0.0), (0.42, 0.42), (0.2, 0.56), (0.08, 0.6));
            let u = 1.0 - t;
            let f = |a: f32, b: f32, c: f32, d: f32| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
            (cx + sgn * w * f(p0.0, p1.0, p2.0, p3.0), top + h * f(p0.1, p1.1, p2.1, p3.1))
        };
        let mut pts: Vec<(f32, f32)> = (0..=12).map(|i| bez(i as f32 / 12.0, 1.0)).collect();
        pts.extend((0..=12).rev().map(|i| bez(i as f32 / 12.0, -1.0)));
        let r = self.r;
        let (lx, rx) = ((cx - w * 0.42) * r, (cx + w * 0.42) * r);
        let grad = move |px: i32| {
            let t = ((px as f32 - lx) / (rx - lx)).clamp(0.0, 1.0);
            if t < 0.35 {
                m.lo.mix(m.hi, t / 0.35)
            } else if t < 0.6 {
                m.hi.mix(m.mid, (t - 0.35) / 0.25)
            } else {
                m.mid.mix(m.lo, (t - 0.6) / 0.4)
            }
        };
        let scaled: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x * r, y * r)).collect();
        poly_with(&mut self.c, &scaled, |px, _| grad(px));
        // stem and base
        self.rect(cx - w * 0.06, top + h * 0.6, w * 0.12, h * 0.18, m.mid);
        let (bx, by, bw, bh) = (cx - w * 0.3, top + h * 0.78, w * 0.6, h * 0.1);
        self.c.rrect_with(bx * r, by * r, bw * r, bh * r, h * 0.03 * r, |px, _| grad(px));
        self.rrect(cx - w * 0.36, top + h * 0.88, w * 0.72, h * 0.12, h * 0.03, m.lo);
        // shine and rank
        self.ellipse(cx - w * 0.2, top + h * 0.16, w * 0.05, h * 0.1, WHITE.alpha(0.55));
        self.text(&rank.to_string(), cx + w * 0.02, top + h * 0.38, h * 0.34, PFace::D800, m.ink, Al::C);
    }

    /// Sun (daytime) or moon (night) on a reset-time bar.
    fn sky_icon(&mut self, cx: f32, cy: f32, day: bool, bg: Col) {
        if day {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                self.line(cx + a.cos() * 6.5, cy + a.sin() * 6.5, cx + a.cos() * 9.5, cy + a.sin() * 9.5, 1.8, hex(0xffcf4a));
            }
            self.circle(cx, cy, 5.2, hex(0xffcf4a));
            self.circle(cx - 1.2, cy - 1.2, 2.0, hex(0xfff1b8));
        } else {
            // the moon minus an offset disc, so nothing dark is painted
            let _ = bg;
            let r = self.r;
            let (mx, my, mr) = (cx * r, cy * r, 6.4 * r);
            let (bx, by, br) = ((cx + 3.4) * r, (cy - 2.4) * r, 5.4 * r);
            let moon = hex(0xf3f6fb);
            self.c.fill_with((mx - mr) as i32 - 1, (my - mr) as i32 - 1, (mx + mr) as i32 + 2, (my + mr) as i32 + 2, |x, y| {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let inside = (mr + 0.5 - ((fx - mx).powi(2) + (fy - my).powi(2)).sqrt()).clamp(0.0, 1.0);
                let bite = (br + 0.5 - ((fx - bx).powi(2) + (fy - by).powi(2)).sqrt()).clamp(0.0, 1.0);
                let cov = inside * (1.0 - bite);
                (cov > 0.0).then_some((moon, cov))
            });
        }
    }
}

/// Zoom for a sprite to fit a box: whole numbers from 2× up (crisp pixels).
fn fit_scale(w: f32, h: f32, box_: f32) -> f32 {
    let k = box_ / w.max(h).max(1.0);
    if k >= 2.0 {
        k.floor()
    } else {
        k
    }
}

/// Filled convex polygon painted by `paint(x, y)` (pixel units).
fn poly_with(c: &mut Canvas, pts: &[(f32, f32)], paint: impl Fn(i32, i32) -> Col) {
    if pts.len() < 3 {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let mut area = 0.0;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        area += a.0 * b.1 - b.0 * a.1;
    }
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let edges: Vec<(f32, f32, f32)> = (0..pts.len())
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-4 {
                return None;
            }
            let (nx, ny) = (sign * dy / len, -sign * dx / len);
            Some((nx, ny, -(nx * a.0 + ny * a.1)))
        })
        .collect();
    c.fill_with(x0 as i32 - 1, y0 as i32 - 1, x1 as i32 + 2, y1 as i32 + 2, |px, py| {
        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
        let d = edges.iter().map(|(nx, ny, c0)| nx * fx + ny * fy + c0).fold(f32::MIN, f32::max);
        let cov = (0.5 - d).clamp(0.0, 1.0);
        (cov > 0.0).then(|| (paint(px, py), cov))
    });
}

/// "Route 210 (south)" → 210.
fn route_no(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("Route ")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// "Thursday 12 November" for the server day starting at `reset`.
pub fn date_label(reset: i64) -> String {
    let (_, m, d, wd) = worldclock::server_date(reset);
    let full = WEEKDAYS.iter().find(|w| w.starts_with(wd)).copied().unwrap_or(wd);
    format!("{full} {d} {}", worldclock::MONTHS[(m - 1) as usize])
}

/// "Thu 12 Nov" (server calendar).
fn short_date(t: i64) -> String {
    let (_, m, d, wd) = worldclock::server_date(t);
    format!("{wd} {d} {}", &worldclock::MONTHS[(m - 1) as usize][..3])
}

/// "2026-11-12" for file names.
pub fn date_slug(reset: i64) -> String {
    let (y, m, d, _) = worldclock::server_date(reset);
    format!("{y:04}-{m:02}-{d:02}")
}

// ───────────────────────────── the poster ─────────────────────────────

/// Pixels per layout unit for a poster this tall: 2× (3840 wide), less for
/// very long lists so the picture stays a sensible size.
fn pick_scale(h: f32) -> f32 {
    let max_px = 40_000_000.0;
    let r: f32 = 2.0;
    if 1920.0 * h * r * r > max_px {
        (max_px / (1920.0 * h)).sqrt().max(1.0)
    } else {
        r
    }
}

struct Entry<'d> {
    area: &'d PosterArea,
    col: usize,
    y: f32,
}

/// Areas in route order (Pokémon by name), how many areas each Pokémon is
/// in, and the top 3 by that (ties share a place).
#[allow(clippy::type_complexity)]
fn prepare(data: &PosterData) -> (Vec<PosterArea>, Vec<(String, usize)>, Vec<(usize, Vec<String>)>) {
    // areas in route order, Pokémon by name
    let mut areas: Vec<PosterArea> = data.areas.iter().filter(|a| !a.mons.is_empty()).cloned().collect();
    for a in &mut areas {
        // Pokémon by name (keeping each tile pointing at its Pokémon)
        let mut order: Vec<usize> = (0..a.mons.len()).collect();
        order.sort_by(|&i, &j| a.mons[i].name.cmp(&a.mons[j].name));
        let mut new_of = vec![0; order.len()];
        for (n, &o) in order.iter().enumerate() {
            new_of[o] = n;
        }
        a.mons = order.iter().map(|&i| a.mons[i].clone()).collect();
        for t in &mut a.tiles {
            t.2 = new_of.get(t.2).copied().unwrap_or(0);
        }
    }
    areas.sort_by(|a, b| {
        route_no(&a.name).unwrap_or(10_000).cmp(&route_no(&b.name).unwrap_or(10_000)).then_with(|| a.name.cmp(&b.name))
    });

    // the top 3: Pokémon in the most areas (ties share a trophy)
    let mut tally: Vec<(String, usize)> = Vec::new();
    for a in &areas {
        for m in &a.mons {
            match tally.iter_mut().find(|(k, _)| *k == m.key) {
                Some(e) => e.1 += 1,
                None => tally.push((m.key.clone(), 1)),
            }
        }
    }
    let name_of = |key: &str| -> String {
        areas.iter().flat_map(|a| a.mons.iter()).find(|m| m.key == key).map(|m| m.name.clone()).unwrap_or_default()
    };
    tally.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| name_of(&a.0).cmp(&name_of(&b.0))));
    let mut top3: Vec<(usize, Vec<String>)> = Vec::new();
    for (k, n) in &tally {
        if let Some(g) = top3.iter_mut().find(|g| g.0 == *n) {
            g.1.push(k.clone());
        } else if top3.len() < 3 {
            top3.push((*n, vec![k.clone()]));
        }
    }
    (areas, tally, top3)
}

/// Draw the infographic.
pub fn render(data: &PosterData, mode: PosterMode, style: PosterStyle, t: &dyn PosterText) -> Canvas {
    if style == PosterStyle::Anime {
        return anime::render(data, mode, t);
    }
    let map_only = mode == PosterMode::MapOnly;
    let lists_only = mode == PosterMode::Lists;
    const W: f32 = 1920.0;
    const M: f32 = 64.0;

    let (areas, tally, top3) = prepare(data);
    let name_of = |key: &str| -> String {
        areas.iter().flat_map(|a| a.mons.iter()).find(|m| m.key == key).map(|m| m.name.clone()).unwrap_or_default()
    };
    let sprite_of = |key: &str| -> Option<&Img> {
        areas.iter().flat_map(|a| a.mons.iter()).find(|m| m.key == key).and_then(|m| m.sprite.as_ref())
    };

    // ── measure
    const HEAD_H: f32 = 230.0;
    let hero_h = if lists_only { HEAD_H } else { 1180.0 };
    let band_h = if top3.is_empty() { 0.0 } else { 250.0 };
    let rows_total: usize = areas.iter().map(|a| a.mons.len().max(1)).sum();
    // row, box, name, meta, sign, num
    let (row, sbox, fname, fmeta, sign, fnum): (f32, f32, f32, f32, f32, f32) = if rows_total <= 10 {
        (66.0, 52.0, 21.0, 15.5, 66.0, 38.0)
    } else if rows_total <= 28 {
        (58.0, 46.0, 19.0, 14.5, 58.0, 34.0)
    } else {
        (50.0, 40.0, 17.0, 13.5, 50.0, 29.0)
    };
    const COLS: usize = 4;
    const GAP: f32 = 40.0;
    const PAD: f32 = 18.0;
    let cw = (W - 2.0 * M - (COLS as f32 - 1.0) * GAP) / COLS as f32;
    let title_h = fmeta + 16.0;
    let mut col_h = [0.0f32; COLS];
    let mut placed: Vec<Entry> = Vec::new();
    for a in &areas {
        let h = sign.max(title_h + a.mons.len().max(1) as f32 * row) + PAD * 2.0;
        let ci = (0..COLS).min_by(|&i, &j| col_h[i].total_cmp(&col_h[j])).unwrap_or(0);
        placed.push(Entry { area: a, col: ci, y: col_h[ci] });
        col_h[ci] += h;
    }
    let index_y = hero_h + band_h;
    const INDEX_HEAD: f32 = 150.0;
    let tallest = if areas.is_empty() { 90.0 } else { col_h.iter().cloned().fold(0.0, f32::max) };
    let index_h = INDEX_HEAD + tallest + 60.0;
    const FOOT_H: f32 = 64.0;
    let h = if map_only { hero_h + FOOT_H } else { index_y + index_h + 70.0 };

    let r = pick_scale(h);
    let mut p = Pen {
        c: Canvas::new((W * r).round() as u32, (h * r).round() as u32, PAPER),
        r,
        t,
    };

    let zone = worldclock::server_zone_name(data.next);
    let zone_long = if zone == "CEST" { "Central European Summer Time" } else { "Central European Time" };

    if lists_only {
        lists_header(&mut p, data, zone, W, M, HEAD_H);
    } else {
        hero(&mut p, data, &areas, zone, W, M, hero_h);
        if map_only {
            p.rect(0.0, hero_h, W, FOOT_H, PAPER);
            p.star(M + 8.0, h - 28.0, 9.0, SEA);
            p.text("Poke Together Companion", M + 24.0, h - 22.0, 19.0, PFace::D700, INK, Al::L);
            let nm = tally.len();
            let na = areas.len();
            let mid = format!(
                "{na} {} with rare grass · {nm} Pokémon",
                if na == 1 { "area" } else { "areas" }
            );
            p.text(&mid, W / 2.0, h - 23.0, 15.0, PFace::T500, INK2, Al::C);
            p.text(&format!("Server time is {zone} ({zone_long})"), W - M, h - 23.0, 15.0, PFace::T500, INK2, Al::R);
            return p.c;
        }
    }

    // ── most widespread: the top 3 by number of areas
    if !top3.is_empty() {
        let by = hero_h;
        p.rect(0.0, by, W, band_h, SEA_DEEP);
        p.text("Most widespread", M, by + 60.0, 38.0, PFace::D700, WHITE, Al::L);
        let tw = p.tw("Most widespread", 38.0, PFace::D700);
        p.text("Pokémon found in the most areas. Ties share a trophy.", M + tw + 20.0, by + 59.0, 18.0, PFace::T500, SEA_TEXT, Al::L);
        const GAPB: f32 = 24.0;
        let cwb = (W - 2.0 * M - 2.0 * GAPB) / 3.0;
        const CHB: f32 = 128.0;
        for i in top3.len()..3 {
            let (x, y, mt) = (M + i as f32 * (cwb + GAPB), by + 88.0, &METALS[i]);
            p.rrect(x, y, cwb, CHB, 14.0, WHITE.alpha(0.04));
            p.rrect_stroke(x, y, cwb, CHB, 14.0, 1.5, SEA_TEXT.alpha(0.3));
            // a faded trophy
            let fade = |c: Col| c.mix(SEA_DEEP, 0.55);
            let ghost = Metal { name: mt.name, hi: fade(mt.hi), mid: fade(mt.mid), lo: fade(mt.lo), ink: fade(mt.ink) };
            p.trophy(x + 52.0, y + CHB / 2.0, 82.0, &ghost, i + 1);
            p.text("Nobody yet", x + 112.0, y + 60.0, 34.0, PFace::D700, WHITE.alpha(0.55), Al::L);
            p.text(&format!("{} goes to the next most widespread", mt.name), x + 112.0, y + 88.0, 16.0, PFace::T500, SEA_FAINT, Al::L);
        }
        for (i, (n, keys)) in top3.iter().enumerate() {
            let (x, y, mt) = (M + i as f32 * (cwb + GAPB), by + 88.0, &METALS[i]);
            p.rrect(x, y, cwb, CHB, 14.0, WHITE.alpha(0.07));
            p.rrect_stroke(x, y, cwb, CHB, 14.0, 2.0, mt.mid.alpha(0.55));
            p.trophy(x + 52.0, y + CHB / 2.0, 82.0, mt, i + 1);
            let k = keys.len().min(4);
            let bx = match k {
                1 => 86.0,
                2 => 70.0,
                _ => 56.0,
            };
            let step = bx * 0.62;
            let sx0 = x + 112.0;
            for (j, key) in keys.iter().take(4).enumerate() {
                p.sticker(sprite_of(key), sx0 + bx / 2.0 + j as f32 * step, y + CHB / 2.0, bx, 2.0);
            }
            let tx = sx0 + bx + (k as f32 - 1.0) * step + 14.0;
            let maxw = x + cwb - 20.0 - tx;
            let names: Vec<String> = keys.iter().map(|k| name_of(k)).collect();
            let label = if names.len() == 2 { names.join(" & ") } else { names.join(", ") };
            let mut fs = if k == 1 { 40.0 } else { 34.0 };
            while fs > 24.0 && p.tw(&label, fs, PFace::D700) > maxw {
                fs -= 1.0;
            }
            let fitted = p.fit(&label, fs, PFace::D700, maxw);
            p.text(&fitted, tx, y + 54.0, fs, PFace::D700, WHITE, Al::L);
            let count = format!("{}{}", if *n == 1 { "1 area".to_string() } else { format!("{n} areas") }, if keys.len() > 1 { " each" } else { "" });
            p.text(&count, tx, y + 82.0, 19.0, PFace::T600, mt.mid, Al::L);
            let wher: Vec<String> = areas
                .iter()
                .filter(|a| a.mons.iter().any(|m| keys.contains(&m.key)))
                .map(|a| a.name.strip_prefix("Route ").unwrap_or(&a.name).to_string())
                .collect();
            let wt = p.fit(&wher.join(", "), 15.5, PFace::T500, maxw);
            p.text(&wt, tx, y + 106.0, 15.5, PFace::T500, SEA_TEXT, Al::L);
        }
    }

    // ── where to find them, in route order
    p.rect(0.0, index_y, W, h - index_y, PAPER);
    p.text("Where to find them", M, index_y + 92.0, 62.0, PFace::D700, INK, Al::L);
    p.text(
        "In route order: from your game log plus the shared list. Use Sweet Scent on the rare grass.",
        M,
        index_y + 126.0,
        19.0,
        PFace::T500,
        INK2,
        Al::L,
    );
    let iy = index_y + INDEX_HEAD;
    if areas.is_empty() {
        p.text("Nothing met on rare grass yet since the reset.", M, iy + 40.0, 20.0, PFace::T500, INK2, Al::L);
    }
    for e in &placed {
        let (x, y, a) = (M + e.col as f32 * (cw + GAP), iy + e.y, e.area);
        if e.y > 0.0 {
            p.line(x, y, x + cw, y, 1.2, RULE);
        }
        let sy = y + PAD;
        p.rrect(x, sy, sign, sign, 9.0, SEA);
        p.rrect_stroke(x + 4.0, sy + 4.0, sign - 8.0, sign - 8.0, 6.0, 1.3, WHITE.alpha(0.55));
        match route_no(&a.name) {
            Some(n) => {
                let t = n.to_string();
                let mut fs = fnum + 2.0;
                while fs > 12.0 && p.tw(&t, fs, PFace::D700) > sign - 12.0 {
                    fs -= 1.0;
                }
                p.text(&t, x + sign / 2.0, sy + sign / 2.0 + fs * 0.35, fs, PFace::D700, WHITE, Al::C);
            }
            None => {
                // a map pin
                let (gx, gy, gr) = (x + sign / 2.0, sy + sign / 2.0 - 2.0, sign * 0.17);
                p.poly(&[(gx - gr * 0.86, gy - gr * 0.0), (gx + gr * 0.86, gy - gr * 0.0), (gx, gy + gr * 1.45)], WHITE);
                p.circle(gx, gy - gr * 0.35, gr, WHITE);
                p.circle(gx, gy - gr * 0.35, gr * 0.42, SEA);
            }
        }
        let tx = x + sign + 16.0;
        let maxw = cw - sign - 16.0;
        let title = p.fit(&a.name, fmeta + 6.0, PFace::D700, maxw);
        p.text(&title, tx, sy + fmeta + 4.0, fmeta + 6.0, PFace::D700, SEA, Al::L);
        let rows_y = sy + title_h;
        for (i, m) in a.mons.iter().enumerate() {
            let cy = rows_y + i as f32 * row + row / 2.0;
            if p.sticker(m.sprite.as_ref(), tx + sbox / 2.0, cy, sbox, if sbox >= 46.0 { 2.0 } else { 1.6 }).is_none() {
                p.circle(tx + sbox / 2.0, cy, sbox * 0.3, GOLD);
            }
            let nx = tx + sbox + 12.0;
            let nmax = maxw - sbox - 12.0;
            let nm = p.fit(&m.name, fname, PFace::T600, nmax);
            p.text(&nm, nx, cy - 3.0, fname, PFace::T600, INK, Al::L);
            let meta = if m.spots > 1 { format!("{} rare grass tiles", m.spots) } else { "1 rare grass tile".to_string() };
            p.text(&meta, nx, cy + fmeta * 1.15, fmeta, PFace::T500, INK2, Al::L);
        }
    }
    // footer
    p.line(M, h - 56.0, W - M, h - 56.0, 1.2, RULE);
    p.star(M + 8.0, h - 28.0, 9.0, SEA);
    p.text("Poke Together Companion", M + 24.0, h - 22.0, 19.0, PFace::D700, INK, Al::L);
    p.text(&format!("Server time is {zone} ({zone_long})"), W - M, h - 23.0, 15.0, PFace::T500, INK2, Al::R);
    p.c
}

/// The slim title band of the lists-only picture.
fn lists_header(p: &mut Pen, data: &PosterData, zone: &str, w: f32, m: f32, head_h: f32) {
    p.rect(0.0, 0.0, w, head_h, HERO);
    let r = p.r;
    let (wpx, hpx) = (w * r, head_h * r);
    p.c.fill_with(0, 0, wpx as i32, hpx as i32, |x, y| {
        let t = ((x as f32 / wpx) + (y as f32 / hpx)) / 2.0;
        Some((Col(120, 200, 225, 255).alpha(0.14 * (1.0 - t)).mix(Col(4, 20, 38, 255).alpha(0.30), t), 1.0))
    });
    let mut yy = 26.0;
    let mut row = 0;
    while yy < head_h {
        let mut xx = if row % 2 == 1 { 36.0 } else { 0.0 } - 72.0;
        while xx < w {
            let c = Col(127, 211, 230, 255).alpha(0.10);
            p.quad((xx, yy), (xx + 18.0, yy - 11.0), (xx + 36.0, yy), 2.0, c);
            p.quad((xx + 36.0, yy), (xx + 54.0, yy + 11.0), (xx + 72.0, yy), 2.0, c);
            xx += 72.0;
        }
        yy += 44.0;
        row += 1;
    }
    p.text("Pokémon Platinum Together", m, 70.0, 19.0, PFace::T600, SEA_TEXT, Al::L);
    p.text("Rare grass", m - 3.0, 172.0, 104.0, PFace::D800, WHITE, Al::L);
    let tw = p.tw("Rare grass", 104.0, PFace::D800);
    p.star(m + tw + 28.0, 104.0, 18.0, GOLD);
    p.star(m + tw + 52.0, 78.0, 8.0, hex(0xffe2a3));
    p.text(&date_label(data.reset), w - m, 112.0, 44.0, PFace::D600, GOLD, Al::R);
    p.text(&format!("Server time ({zone})"), w - m, 146.0, 15.0, PFace::T600, SEA_FAINT, Al::R);
    let span = format!("{}, 12:00 am until {}, 12:00 am", short_date(data.reset), short_date(data.next));
    p.text(&span, w - m, 174.0, 19.0, PFace::T500, PALE, Al::R);
}

/// A pin on the map: one rare grass patch (an area's tiles that lie close
/// together). (ox, oy) is where the patch really is, (x, y) where the pin
/// went after being nudged clear of the others; `r` its room.
struct Pin {
    mons: Vec<usize>,
    area: usize,
    ox: f32,
    oy: f32,
    x: f32,
    y: f32,
    r: f32,
}

/// Pins for every patch, nudged apart and off the place names (`taken`),
/// kept inside left..right, top..bottom. Also returns the sprite size.
#[allow(clippy::too_many_arguments)]
fn place_pins(
    areas: &[PosterArea],
    to_px: &dyn Fn(f32, f32) -> (f32, f32),
    taken: &[(f32, f32, f32, f32)],
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
) -> (Vec<Pin>, f32) {
    // pins: one per rare grass patch (an area's tiles that lie close
    // together), at the middle of its exact tiles; then nudged apart
    // so none overlap, with a line back to where the patch really is
    const NEAR: f32 = 40.0; // tiles: closer than this is the same patch
    let mut patches: Vec<(usize, Vec<(f32, f32)>, Vec<usize>)> = Vec::new();
    for (ai, a) in areas.iter().enumerate() {
        let first = patches.len();
        for &(tx, ty, mi) in &a.tiles {
            let hit = patches[first..].iter_mut().find(|(_, pts, _)| {
                let (cx, cy) = (pts.iter().map(|p| p.0).sum::<f32>() / pts.len() as f32, pts.iter().map(|p| p.1).sum::<f32>() / pts.len() as f32);
                ((cx - tx).powi(2) + (cy - ty).powi(2)).sqrt() < NEAR
            });
            match hit {
                Some((_, pts, ms)) => {
                    pts.push((tx, ty));
                    if !ms.contains(&mi) {
                        ms.push(mi);
                    }
                }
                None => patches.push((ai, vec![(tx, ty)], vec![mi])),
            }
        }
    }
    let busy = patches.len() > 16;
    let sb = if busy { 50.0 } else { 62.0 };
    let mut pins: Vec<Pin> = patches
        .into_iter()
        .map(|(area, pts, mut mons)| {
            mons.sort();
            let n = pts.len() as f32;
            let (ox, oy) = to_px(pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
            let k = mons.len().clamp(1, 4) as f32;
            Pin { mons, area, ox, oy, x: ox, y: oy, r: if busy { 23.0 } else { 28.0 } + (k - 1.0) * sb * 0.3 }
        })
        .collect();
    for _ in 0..120 {
        for i in 0..pins.len() {
            for j in i + 1..pins.len() {
                let (dx, dy) = (pins[j].x - pins[i].x, pins[j].y - pins[i].y);
                let d = (dx * dx + dy * dy).sqrt().max(0.01);
                let min = pins[i].r + pins[j].r;
                if d < min {
                    let push = (min - d) / 2.0;
                    let (ux, uy) = if d <= 0.01 { (1.0, 0.0) } else { (dx / d, dy / d) };
                    pins[i].x -= ux * push;
                    pins[i].y -= uy * push;
                    pins[j].x += ux * push;
                    pins[j].y += uy * push;
                }
            }
        }
    }
    // ...and off the place names
    for _ in 0..40 {
        for pin in &mut pins {
            for &(lx, ly, lw, lh) in taken {
                let (nx, ny) = (pin.x.clamp(lx, lx + lw), pin.y.clamp(ly, ly + lh));
                let (dx, dy) = (pin.x - nx, pin.y - ny);
                let d = (dx * dx + dy * dy).sqrt();
                let room = pin.r * 0.8;
                if d < room {
                    let (ux, uy) = if d < 0.01 { (0.0, if pin.y < ly + lh / 2.0 { -1.0 } else { 1.0 }) } else { (dx / d, dy / d) };
                    pin.x += ux * (room - d + 0.5);
                    pin.y += uy * (room - d + 0.5);
                }
            }
        }
    }
    for pin in &mut pins {
        pin.x = pin.x.clamp(left + pin.r, right - pin.r);
        pin.y = pin.y.clamp(top + pin.r, bottom - pin.r);
    }
    (pins, sb)
}

/// The top: the map full-bleed on the right, the title and reset times on the sea.
fn hero(p: &mut Pen, data: &PosterData, areas: &[PosterArea], zone: &str, w: f32, m: f32, hero_h: f32) {
    let r = p.r;
    p.rect(0.0, 0.0, w, hero_h, HERO);

    // the map: as tall as the top, against the right edge (as the Sparkle Map's poster)
    let (mut mx, mut my, mut scale, mut mw) = (w * 0.4, 5.0, 1.0f32, w * 0.6);
    if let Some(mp) = &data.map {
        let (_, _, fw, fh) = mp.fit;
        scale = (hero_h - 10.0) / fh;
        mw = fw * scale;
        mx = w - mw - 6.0;
        my = (hero_h - fh * scale) / 2.0;
    }

    // soft depth: darker towards the edges
    let (gcx, gcy, r0, r1) = ((mx + mw * 0.55) * r, hero_h * 0.5 * r, 100.0 * r, w * 0.75 * r);
    p.c.fill_with(0, 0, (w * r) as i32, (hero_h * r) as i32, |x, y| {
        let d = ((x as f32 - gcx).powi(2) + (y as f32 - gcy).powi(2)).sqrt();
        let t = ((d - r0) / (r1 - r0)).clamp(0.0, 1.0);
        let inner = Col(120, 200, 225, 255);
        let outer = Col(4, 20, 38, 255);
        let a = 0.18 + (0.35 - 0.18) * t;
        Some((inner.mix(outer, t), a))
    });
    // gentle waves over the whole sea
    let (wv, wh) = (78.0, 50.0);
    let wave = Col(127, 211, 230, 255).alpha(0.10);
    let mut yy = wh * 0.5;
    let mut row = 0;
    while yy < hero_h {
        let mut xx = if row % 2 == 1 { wv * 0.5 } else { 0.0 } - wv;
        while xx < w {
            p.quad((xx, yy), (xx + wv * 0.25, yy - wh * 0.35), (xx + wv * 0.5, yy), 2.0, wave);
            p.quad((xx + wv * 0.5, yy), (xx + wv * 0.75, yy + wh * 0.35), (xx + wv, yy), 2.0, wave);
            xx += wv;
        }
        yy += wh;
        row += 1;
    }

    let to_px = |tx: f32, ty: f32| -> (f32, f32) {
        match &data.map {
            Some(mp) => (mx + (tx - mp.fit.0) * scale, my + (ty - mp.fit.1) * scale),
            None => (mx, my),
        }
    };

    if let Some(mp) = &data.map {
        let (ix, iy) = to_px(mp.x0, mp.y0);
        let (iw, ih) = (mp.img.w as f32 / mp.px_per_tile * scale, mp.img.h as f32 / mp.px_per_tile * scale);
        p.c.blit_smooth(&mp.img, ix * r, iy * r, iw * r, ih * r);
        // city and town names (pins keep clear of them)
        let mut taken: Vec<(f32, f32, f32, f32)> = Vec::new();
        for l in &data.labels {
            let (cx, cy) = to_px(l.x, l.y);
            let name = l.name.trim_end_matches(" City").trim_end_matches(" Town");
            let (fs, face) = if l.city { (22.0, PFace::D700) } else { (17.0, PFace::D600) };
            p.halo_text(name, cx, cy - 6.0, fs, face, INK, WHITE, 4.0);
            let tw = p.tw(name, fs, face);
            taken.push((cx - tw / 2.0 - 3.0, cy - 6.0 - fs, tw + 6.0, fs + 6.0));
        }
        let (pins, sb) = place_pins(areas, &to_px, &taken, mx, w - 4.0, 4.0, hero_h - 4.0);
        for pin in &pins {
            if ((pin.x - pin.ox).powi(2) + (pin.y - pin.oy).powi(2)).sqrt() > 6.0 {
                p.line(pin.ox, pin.oy, pin.x, pin.y, 1.6, INK.alpha(0.55));
                p.circle(pin.ox, pin.oy, 3.0, INK);
            }
        }
        for pin in &pins {
            let a = &areas[pin.area];
            let mons: Vec<&PosterMon> = pin.mons.iter().take(4).filter_map(|&i| a.mons.get(i)).collect();
            let b = sb * 0.72;
            let step = b * 0.62;
            p.ellipse(pin.x, pin.y + b * 0.44, b * 0.42 + (mons.len() as f32 - 1.0) * step / 2.0, b * 0.12, GOLD.alpha(0.55));
            for (i, mon) in mons.iter().enumerate() {
                let ix = pin.x + (i as f32 - (mons.len() as f32 - 1.0) / 2.0) * step;
                if p.sticker(mon.sprite.as_ref(), ix, pin.y, b, 1.6).is_none() {
                    p.circle(ix, pin.y, b * 0.3, GOLD);
                }
            }
        }
    }

    // ── title column over the sea
    let tx = m;
    p.text("Pokémon Platinum Together", tx, 104.0, 20.0, PFace::T600, SEA_TEXT, Al::L);
    p.text("Rare", tx - 5.0, 252.0, 164.0, PFace::D800, WHITE, Al::L);
    p.text("grass", tx - 5.0, 388.0, 164.0, PFace::D800, WHITE, Al::L);
    let gw = p.tw("grass", 164.0, PFace::D800);
    p.star(tx + gw + 30.0, 304.0, 24.0, GOLD);
    p.star(tx + gw + 62.0, 266.0, 10.0, hex(0xffe2a3));
    p.text(&date_label(data.reset), tx, 456.0, 44.0, PFace::D600, GOLD, Al::L);
    p.text(&format!("Server time ({zone})"), tx + 2.0, 484.0, 15.0, PFace::T500, SEA_TEXT, Al::L);
    p.text("Today runs from", tx, 530.0, 15.0, PFace::T600, SEA_FAINT, Al::L);
    p.text(&format!("{}, 12:00 am", short_date(data.reset)), tx, 557.0, 20.0, PFace::T500, PALE, Al::L);
    p.text(&format!("until {}, 12:00 am", short_date(data.next)), tx, 584.0, 20.0, PFace::T500, PALE, Al::L);

    // when the daily reset happens around the world
    let head = p.wrap("Rare grass 24hr reset time around the world", 31.0, PFace::D700, 440.0);
    let n = head.len().min(2);
    for (i, l) in head.iter().take(2).enumerate() {
        p.text(l, tx, 666.0 - (n - 1 - i) as f32 * 33.0, 31.0, PFace::D700, WHITE, Al::L);
    }
    let rows = worldclock::rows(data.next);
    let (rx0, timex, barx, barw, rowh, y0) = (tx, tx + 262.0, tx + 312.0, 150.0, 32.0, 708.0);
    for (l, hr) in [("12a", 0.0), ("6a", 6.0), ("12p", 12.0), ("6p", 18.0), ("12a", 24.0)] {
        p.text(l, barx + barw * hr / 24.0, y0 - 12.0, 12.5, PFace::T500, SEA_FAINT, Al::C);
    }
    let night = hex(0x0f3247);
    let dayc = hex(0x3f7f9c);
    for (i, rw) in rows.iter().enumerate() {
        let cy = y0 + i as f32 * rowh + 12.0;
        if rw.server {
            let hh = rowh - 4.0;
            p.rrect(rx0 - 10.0, cy - hh / 2.0, barx + barw + 16.0 - rx0 + 10.0, hh, 8.0, GOLD.alpha(0.14));
        }
        let col = if rw.server { GOLD } else { WHITE };
        let t12 = rw.time12();
        let tw12 = p.tw(&t12, 22.0, PFace::D600);
        let room = timex - tw12 - 14.0 - rx0;
        let mut cfs = 17.0;
        while cfs > 13.0 && p.tw(rw.label, cfs, PFace::T600) > room {
            cfs -= 0.5;
        }
        let label = p.fit(rw.label, cfs, PFace::T600, room);
        p.text(&label, rx0, cy + 6.0, cfs, PFace::T600, col, Al::L);
        p.text(&t12, timex, cy + 7.0, 22.0, PFace::D600, col, Al::R);
        p.text(rw.weekday, timex + 8.0, cy + 6.0, 13.5, PFace::T500, SEA_TEXT, Al::L);
        // 24 hour bar: darker at night
        let (bx0, bx1) = (barx * r, (barx + barw) * r);
        p.c.rrect_with(barx * r, (cy - 4.0) * r, barw * r, 8.0 * r, 4.0 * r, |x, _| {
            let t = (x as f32 - bx0) / (bx1 - bx0);
            let k = if t < 0.22 || t > 0.78 {
                0.0
            } else if t < 0.32 {
                (t - 0.22) / 0.1
            } else if t > 0.68 {
                (0.78 - t) / 0.1
            } else {
                1.0
            };
            night.mix(dayc, k)
        });
        let frac = (rw.hour as f32 + rw.minute as f32 / 60.0) / 24.0;
        let day = (6..18).contains(&rw.hour);
        let icon_x = barx + barw * frac;
        let bg_here = night.mix(dayc, if day { 1.0 } else { 0.0 });
        p.sky_icon(icon_x, cy, day, bg_here);
    }
    p.text(
        &format!("Local time of the daily 12am {zone} reset"),
        tx,
        y0 + rows.len() as f32 * rowh + 8.0,
        13.5,
        PFace::T500,
        SEA_FAINT,
        Al::L,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_numbers() {
        assert_eq!(route_no("Route 210 (south)"), Some(210));
        assert_eq!(route_no("Eterna Forest"), None);
    }

    #[test]
    fn labels() {
        assert_eq!(date_label(1794438000), "Thursday 12 November");
        assert_eq!(date_slug(1794438000), "2026-11-12");
        assert_eq!(short_date(1794438000), "Thu 12 Nov");
    }
}
