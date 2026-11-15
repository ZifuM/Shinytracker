//! Stats share images: an infographic of your numbers and charts, in three
//! layouts (Poster 4:5, Dashboard 16:9, Wrapped 9:16), for the Stats
//! Overview, a hunt's stats and a Month in Review. Drawn to match the
//! design page exactly: every box, size and colour below comes from it, in
//! its own units (CSS pixels), scaled up to the image's size.
//!
//! Text uses Inter (assets/fonts/share, see the README there) with Inter's
//! kerning and tabular digits, like the browser drew the designs.

use crate::canvas::{Canvas, Col, Img};
use ab_glyph::{point, Font, FontRef, Glyph, GlyphId, Outline, OutlineCurve, OutlinedGlyph, PxScale, PxScaleFactor, Rect, ScaleFont};
use std::sync::OnceLock;

// ───────────────────────────── what goes in ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    #[default]
    Poster,
    Dashboard,
    Wrapped,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Poster, Layout::Dashboard, Layout::Wrapped];
    pub fn label(self) -> &'static str {
        match self {
            Layout::Poster => "Poster",
            Layout::Dashboard => "Dashboard",
            Layout::Wrapped => "Wrapped",
        }
    }
    pub fn shape(self) -> &'static str {
        match self {
            Layout::Poster => "1080 × 1350 · 4:5",
            Layout::Dashboard => "1600 × 900 · 16:9",
            Layout::Wrapped => "1080 × 1920 · 9:16",
        }
    }
    /// The design's size (CSS pixels) and the scale up to the image.
    fn frame(self) -> (f32, f32, f32) {
        match self {
            Layout::Poster => (432.0, 540.0, 2.5),
            Layout::Dashboard => (640.0, 360.0, 2.5),
            Layout::Wrapped => (300.0, 533.333, 3.6),
        }
    }
    pub fn size(self) -> (u32, u32) {
        let (w, h, s) = self.frame();
        ((w * s).round() as u32, (h * s).round() as u32)
    }
}

/// You, at the top of the image.
#[derive(Clone, Default)]
pub struct Who {
    /// Trainer name ("" to leave it off).
    pub name: String,
    /// "Level 26 Shiny Hunter" ("" to leave it off).
    pub level: String,
    pub avatar: Option<Img>,
}

/// A shiny on the shelf.
#[derive(Clone, Default)]
pub struct Pic {
    pub name: String,
    pub sprite: Option<Img>,
}

/// The Stats Overview (or a month): everything over a stretch of days.
#[derive(Clone, Default)]
pub struct Overview {
    /// "LAST 30 DAYS" / "NOVEMBER 2026" (the chip on the Poster).
    pub chip: String,
    /// "last 30 days" / "November 2026" (in "Zifu · last 30 days").
    pub span: String,
    /// "17 Oct – 15 Nov 2026".
    pub dates: String,
    pub encounters: u64,
    pub secs: u64,
    pub shinies: u32,
    pub pace: Option<u64>,
    pub days: u32,
    /// Encounters per bar (day, week or month), oldest first.
    pub bars: Vec<u64>,
    /// Bars with shinies on them.
    pub stars: Vec<usize>,
    /// Under the chart: start, middle, end.
    pub axis: [String; 3],
    /// Your luck (× the odds) and how many hunters you're luckier than.
    pub luck: Option<f64>,
    pub luckier: Option<u32>,
    /// (encounters, "11 Nov", "Wednesday")
    pub best_day: Option<(u64, String, String)>,
    pub longest: Option<u64>,
    pub fastest: Option<u64>,
    pub streak: u32,
    /// Up to 3 shinies (newest first) and the line under them.
    pub shelf: Vec<Pic>,
    pub shelf_line: String,
    pub hours: [u64; 24],
    /// Monday first.
    pub weekdays: [u64; 7],
    pub met: Vec<(String, u64)>,
}

/// One hunt's stats.
#[derive(Clone, Default)]
pub struct Hunt {
    pub name: String,
    pub sprite: Option<Img>,
    pub shiny: bool,
    /// "Masuda + Lure"
    pub method: String,
    /// "1 in 1,024"
    pub odds: String,
    pub count: u64,
    /// "eggs" / "encounters" and "Eggs" / "Encounters".
    pub unit: String,
    pub unit_cap: String,
    pub mult: f64,
    /// 0..1: how many hunters would have found it by now.
    pub found: f64,
    pub secs: u64,
    pub pace: Option<u64>,
    /// "10 Nov"
    pub started: String,
    pub days: u32,
    pub sessions: u32,
    /// Encounters on the last days hunted: ("10 Nov", n), up to 2.
    pub per_day: Vec<(String, u64)>,
    /// More encounters for a 50, 90 and 99% chance.
    pub ahead: [u64; 3],
    /// (encounters, "11 Nov", "Wednesday")
    pub best_day: Option<(u64, String, String)>,
    pub longest: Option<u64>,
    /// Caught (the hunt's over).
    pub caught: bool,
    pub fun_fact: String,
}

#[derive(Clone)]
pub enum Data {
    Overview(Overview),
    Hunt(Hunt),
}

// ───────────────────────────── colours ─────────────────────────────

const TEXT: Col = Col::rgb(236, 238, 246);
const MUTED: Col = Col::rgb(138, 143, 166);
const DIM: Col = Col::rgb(92, 97, 120);
const BRAND: Col = Col::rgb(207, 211, 230);
const BLUE: Col = Col::rgb(47, 140, 255);
const GREEN: Col = Col::rgb(52, 209, 122);
const GOLD: Col = Col::rgb(246, 189, 58);
const ORANGE: Col = Col::rgb(240, 138, 60);
const RED: Col = Col::rgb(236, 84, 84);
const VIOLET: Col = Col::rgb(139, 92, 246);
const AQUA: Col = Col::rgb(47, 196, 196);
const PINK: Col = Col::rgb(236, 92, 154);
const BAR_OFF: Col = Col::rgb(42, 49, 80);
const CARD: Col = Col::rgb(26, 28, 39);
const LINE: Col = Col::rgb(42, 45, 61);

fn white(a: f32) -> Col {
    Col(255, 255, 255, (a * 255.0).round() as u8)
}

// ───────────────────────────── fonts ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum W {
    /// 400
    R,
    /// 600
    Sb,
    /// 700
    B,
    /// 800
    Xb,
    /// 900
    K,
    /// 900 with tabular digits (the big numbers)
    N,
}

struct Fonts {
    f: [FontRef<'static>; 6],
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let load = |b: &'static [u8]| FontRef::try_from_slice(b).expect("share font");
        Fonts {
            f: [
                load(include_bytes!("../assets/fonts/share/Inter-Regular.otf")),
                load(include_bytes!("../assets/fonts/share/Inter-SemiBold.otf")),
                load(include_bytes!("../assets/fonts/share/Inter-Bold.otf")),
                load(include_bytes!("../assets/fonts/share/Inter-ExtraBold.otf")),
                load(include_bytes!("../assets/fonts/share/Inter-Black.otf")),
                load(include_bytes!("../assets/fonts/share/Inter-Black-Tnum.otf")),
            ],
        }
    })
}

/// ab_glyph sizes a font by its height (ascent - descent); CSS by its em.
fn em(f: &FontRef<'static>, size: f32) -> PxScale {
    let upem = f.units_per_em().unwrap_or(2048.0);
    PxScale::from(size * f.height_unscaled() / upem)
}

impl Fonts {
    fn get(&self, w: W) -> &FontRef<'static> {
        &self.f[match w {
            W::R => 0,
            W::Sb => 1,
            W::B => 2,
            W::Xb => 3,
            W::K => 4,
            W::N => 5,
        }]
    }
}

/// DejaVu Sans' ✦ (U+2726), the star the designs show: quadratic curves in
/// its own units (2048 an em), y up; 1716 wide.
const STAR: [[f32; 6]; 16] = [
    [87.0, 748.0, 242.0, 748.0, 383.0, 807.0],
    [383.0, 807.0, 517.0, 863.0, 632.0, 977.0],
    [632.0, 977.0, 744.0, 1088.0, 801.5, 1226.0],
    [801.5, 1226.0, 859.0, 1364.0, 859.0, 1525.0],
    [859.0, 1525.0, 859.0, 1368.0, 914.0, 1232.0],
    [914.0, 1232.0, 969.0, 1096.0, 1086.0, 977.0],
    [1086.0, 977.0, 1194.0, 867.0, 1335.0, 807.0],
    [1335.0, 807.0, 1475.0, 748.0, 1631.0, 748.0],
    [1631.0, 748.0, 1475.0, 748.0, 1335.0, 689.0],
    [1335.0, 689.0, 1194.0, 629.0, 1086.0, 519.0],
    [1086.0, 519.0, 969.0, 400.0, 914.0, 264.0],
    [914.0, 264.0, 859.0, 128.0, 859.0, -29.0],
    [859.0, -29.0, 859.0, 132.0, 801.5, 270.0],
    [801.5, 270.0, 744.0, 408.0, 632.0, 519.0],
    [632.0, 519.0, 517.0, 633.0, 383.0, 689.0],
    [383.0, 689.0, 242.0, 748.0, 87.0, 748.0],
];
const STAR_ADV: f32 = 1716.0 / 2048.0;

fn star_outline() -> Outline {
    let p = |x: f32, y: f32| point(x, y);
    Outline {
        bounds: Rect { min: p(87.0, 1525.0), max: p(1631.0, -29.0) },
        curves: STAR.iter().map(|c| OutlineCurve::Quad(p(c[0], c[1]), p(c[2], c[3]), p(c[4], c[5]))).collect(),
    }
}

/// Where the browser puts the baseline, below the top of a line box (CSS px),
/// for font sizes 6 to 60 in half pixels, at line heights 1, 1.05 and 1.45:
/// measured from Chrome at the images' scales (2.5 and 3.6), so the text sits
/// exactly where it does on the design page.
const BASE_2_5: [[u8; 109]; 3] = [
    [4, 5, 6, 6, 7, 6, 7, 7, 8, 8, 9, 9, 10, 9, 10, 10, 11, 11, 13, 13, 14, 14, 13, 14, 14, 15, 16, 16, 17, 17, 17, 17, 18, 18, 19, 20, 20, 21, 20, 21, 21, 22, 23, 23, 24, 24, 25, 24, 25, 25, 26, 27, 27, 27, 28, 27, 28, 28, 29, 30, 31, 31, 32, 31, 32, 32, 33, 33, 34, 34, 35, 34, 35, 35, 36, 36, 38, 38, 39, 39, 39, 39, 40, 40, 41, 41, 42, 42, 42, 42, 43, 43, 44, 45, 45, 46, 45, 46, 46, 47, 48, 48, 49, 49, 50, 49, 50, 50, 51],
    [4, 5, 6, 6, 7, 6, 7, 7, 8, 9, 9, 10, 10, 10, 10, 11, 11, 12, 13, 13, 14, 14, 13, 14, 14, 15, 16, 17, 17, 18, 17, 18, 18, 19, 20, 20, 21, 21, 21, 21, 22, 22, 23, 24, 24, 25, 25, 25, 25, 26, 26, 28, 28, 28, 29, 28, 29, 29, 30, 31, 31, 32, 32, 32, 32, 33, 33, 34, 35, 35, 36, 35, 36, 36, 37, 37, 39, 39, 40, 40, 40, 40, 41, 41, 42, 42, 43, 43, 43, 44, 44, 45, 45, 46, 46, 47, 46, 47, 47, 48, 49, 50, 50, 51, 51, 51, 51, 52, 52],
    [5, 6, 7, 7, 8, 8, 9, 9, 10, 11, 11, 12, 13, 12, 13, 13, 14, 15, 16, 16, 17, 17, 17, 18, 18, 19, 20, 21, 21, 22, 21, 22, 22, 23, 24, 25, 25, 26, 26, 26, 27, 28, 29, 29, 30, 31, 31, 31, 31, 32, 32, 34, 34, 35, 35, 35, 36, 36, 37, 38, 39, 39, 40, 39, 40, 40, 41, 42, 43, 43, 44, 44, 44, 45, 46, 46, 47, 48, 49, 49, 49, 49, 50, 50, 52, 52, 53, 53, 53, 54, 54, 55, 56, 57, 57, 58, 57, 58, 58, 59, 60, 61, 61, 62, 63, 62, 63, 64, 64],
];
const BASE_3_6: [[u8; 109]; 3] = [
    [4, 5, 6, 6, 7, 6, 7, 7, 8, 8, 9, 9, 10, 9, 10, 10, 11, 11, 13, 13, 14, 14, 13, 14, 14, 15, 16, 16, 17, 17, 17, 17, 18, 18, 19, 20, 20, 21, 20, 21, 21, 22, 23, 23, 24, 24, 25, 24, 25, 25, 26, 27, 27, 27, 28, 27, 28, 28, 29, 30, 31, 31, 32, 31, 32, 32, 33, 33, 34, 34, 35, 34, 35, 35, 36, 36, 38, 38, 39, 39, 39, 39, 40, 40, 41, 41, 42, 42, 42, 42, 43, 43, 44, 45, 45, 46, 45, 46, 46, 47, 48, 48, 49, 49, 50, 49, 50, 50, 51],
    [4, 5, 6, 6, 7, 6, 7, 7, 8, 9, 9, 10, 10, 10, 10, 11, 11, 12, 13, 13, 14, 14, 13, 14, 14, 15, 16, 17, 17, 18, 17, 18, 18, 19, 20, 20, 21, 21, 21, 21, 22, 22, 23, 24, 24, 25, 25, 25, 25, 26, 26, 28, 28, 28, 29, 28, 29, 29, 30, 31, 31, 32, 32, 32, 32, 33, 33, 34, 35, 35, 36, 35, 36, 36, 37, 37, 39, 39, 40, 40, 40, 40, 41, 41, 42, 42, 43, 43, 43, 44, 44, 45, 45, 46, 46, 47, 46, 47, 47, 48, 49, 50, 50, 51, 51, 51, 51, 52, 52],
    [5, 6, 7, 7, 8, 8, 9, 9, 10, 11, 11, 12, 13, 12, 13, 13, 14, 15, 16, 16, 17, 17, 17, 18, 18, 19, 20, 21, 21, 22, 21, 22, 22, 23, 24, 25, 25, 26, 26, 26, 27, 28, 29, 29, 30, 31, 31, 31, 31, 32, 32, 34, 34, 35, 35, 35, 36, 36, 37, 38, 39, 39, 40, 39, 40, 40, 41, 42, 43, 43, 44, 44, 44, 45, 46, 46, 47, 48, 49, 49, 49, 49, 50, 50, 52, 52, 53, 53, 53, 54, 54, 55, 56, 57, 57, 58, 57, 58, 58, 59, 60, 61, 61, 62, 63, 62, 63, 64, 64],
];

/// The baseline below a line box's top, as the browser does it.
fn base_off(scale: f32, size: f32, lh: f32) -> f32 {
    let k = lh / size;
    let row = [1.0f32, 1.05, 1.45].iter().position(|m| (k - m).abs() < 0.01);
    let half = size * 2.0;
    let i = half.round() as i32 - 12;
    let table = if (scale - 3.6).abs() < 0.01 { &BASE_3_6 } else { &BASE_2_5 };
    match row {
        Some(r) if (half - half.round()).abs() < 0.01 && (0..109).contains(&i) => table[r][i as usize] as f32,
        // between the measured sizes: Inter's own metrics
        _ => (lh - size * 1.2099) / 2.0 + size * 0.96875,
    }
}

// ───────────────────────────── drawing in design units ─────────────────────────────

/// A paint for boxes and text: a colour, or a colour for each spot (in
/// design units).
enum Paint<'a> {
    C(Col),
    F(&'a dyn Fn(f32, f32) -> Col),
}

struct D {
    c: Canvas,
    /// Design units to image pixels.
    s: f32,
}

/// CSS linear-gradient(angle, stops) over a box: the colour at (x, y).
fn linear(angle: f32, bx: f32, by: f32, bw: f32, bh: f32, stops: &[(f32, Col)]) -> impl Fn(f32, f32) -> Col + '_ {
    let a = angle.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let len = (bw * dx).abs() + (bh * dy).abs();
    let (cx, cy) = (bx + bw / 2.0, by + bh / 2.0);
    move |x, y| {
        let t = (((x - cx) * dx + (y - cy) * dy) / len + 0.5).clamp(0.0, 1.0);
        at_stop(stops, t)
    }
}

fn at_stop(stops: &[(f32, Col)], t: f32) -> Col {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        if t <= w[1].0 {
            let span = (w[1].0 - w[0].0).max(1e-6);
            return w[0].1.mix(w[1].1, (t - w[0].0) / span);
        }
    }
    stops[stops.len() - 1].1
}

impl D {
    fn new(layout: Layout) -> D {
        let (w, h) = layout.size();
        D { c: Canvas::new(w, h, Col::rgb(16, 18, 28)), s: layout.frame().2 }
    }

    fn paint_at(p: &Paint, x: f32, y: f32) -> Col {
        match p {
            Paint::C(c) => *c,
            Paint::F(f) => f(x, y),
        }
    }

    /// A box (rounded `r`), filled with `p`.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, p: Paint) {
        let s = self.s;
        self.c.rrect_with(x * s, y * s, w * s, h * s, r * s, |px, py| D::paint_at(&p, (px as f32 + 0.5) / s, (py as f32 + 0.5) / s));
    }

    fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        self.rect(x, y, w, h, r, Paint::C(c));
    }

    /// A 1px (design) border inside the box.
    fn border(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, c: Col) {
        let s = self.s;
        self.c.rrect_stroke(x * s, y * s, w * s, h * s, r * s, t * s, c);
    }

    /// A box rounded only at the top (chart bars).
    fn bar(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        if h <= 0.0 || w <= 0.0 {
            return;
        }
        let r = r.min(w / 2.0).min(h);
        self.fill(x, y, w, h, r, c);
        let bottom = (h - r).max(0.0);
        if bottom > 0.0 {
            let s = self.s;
            // square the bottom corners off
            self.c.fill_with(((x * s).floor()) as i32, ((y + r) * s).floor() as i32, ((x + w) * s).ceil() as i32, ((y + h) * s).ceil() as i32, |px, py| {
                let cx = (((px as f32 + 1.0).min((x + w) * s) - (px as f32).max(x * s)).clamp(0.0, 1.0)) as f32;
                let cy = ((py as f32 + 1.0).min((y + h) * s) - (py as f32).max((y + r) * s)).clamp(0.0, 1.0);
                let cov = cx * cy;
                (cov > 0.0).then_some((c, cov))
            });
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        let s = self.s;
        self.c.circle(cx * s, cy * s, r * s, c);
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: Col) {
        let s = self.s;
        self.c.line(x0 * s, y0 * s, x1 * s, y1 * s, w * s, c);
    }

    // ── text ──

    fn glyphs(&self, w: W, size: f32, ls: f32, text: &str) -> (Vec<(char, GlyphId, f32)>, f32) {
        let f = fonts().get(w);
        let sf = f.as_scaled(em(f, size));
        let mut out = Vec::new();
        let mut pen = 0.0;
        let mut prev: Option<GlyphId> = None;
        for ch in text.chars() {
            if ch == '✦' {
                out.push((ch, GlyphId(0), pen));
                pen += STAR_ADV * size + ls;
                prev = None;
                continue;
            }
            let id = f.glyph_id(ch);
            if let Some(p) = prev {
                pen += sf.kern(p, id);
            }
            out.push((ch, id, pen));
            pen += sf.h_advance(id) + ls;
            prev = Some(id);
        }
        (out, pen)
    }

    /// How wide `text` is (design units).
    fn width(&self, w: W, size: f32, ls: f32, text: &str) -> f32 {
        self.glyphs(w, size, ls, text).1
    }

    /// Text with its line box's top at `top` (line height `lh`), starting
    /// at `x`. Returns its width.
    fn text(&mut self, x: f32, top: f32, lh: f32, size: f32, w: W, ls: f32, p: Paint, text: &str) -> f32 {
        let b = top + base_off(self.s, size, lh);
        self.text_b(x, b, size, w, ls, p, text)
    }

    /// The baseline of a line (for text that shares it, like a span).
    fn baseline(&self, top: f32, lh: f32, size: f32) -> f32 {
        top + base_off(self.s, size, lh)
    }

    /// Text sitting on the baseline `baseline` (design units).
    fn text_b(&mut self, x: f32, baseline: f32, size: f32, w: W, ls: f32, p: Paint, text: &str) -> f32 {
        let f = fonts().get(w);
        let s = self.s;
        let px = size * s;
        let scale = em(f, px);
        let base = baseline * s;
        let (gs, width) = self.glyphs(w, size, ls, text);
        for (ch, id, at) in gs {
            let gx = (x + at) * s;
            let og = if ch == '✦' {
                let g = Glyph { id: GlyphId(0), scale, position: point(gx, base) };
                let k = px / 2048.0;
                Some(OutlinedGlyph::new(g, star_outline(), PxScaleFactor { horizontal: k, vertical: k }))
            } else {
                f.outline_glyph(id.with_scale_and_position(scale, point(gx, base)))
            };
            if let Some(og) = og {
                let b = og.px_bounds();
                let c = &mut self.c;
                og.draw(|ox, oy, cov| {
                    let (ix, iy) = (b.min.x as i32 + ox as i32, b.min.y as i32 + oy as i32);
                    let col = D::paint_at(&p, (ix as f32 + 0.5) / s, (iy as f32 + 0.5) / s);
                    c.blend(ix, iy, col, cov);
                });
            }
        }
        width
    }

    fn t(&mut self, x: f32, top: f32, lh: f32, size: f32, w: W, c: Col, text: &str) -> f32 {
        self.text(x, top, lh, size, w, 0.0, Paint::C(c), text)
    }

    /// Right-aligned to `right`.
    fn tr(&mut self, right: f32, top: f32, lh: f32, size: f32, w: W, c: Col, text: &str) -> f32 {
        let wd = self.width(w, size, 0.0, text);
        self.t(right - wd, top, lh, size, w, c, text)
    }

    /// A small caps label (`.k`): 9.5px, 800, letter-spaced, upper case.
    fn k(&mut self, x: f32, top: f32, c: Col, text: &str) {
        self.text(x, top, 13.775, 9.5, W::Xb, 0.855, Paint::C(c), &text.to_uppercase());
    }

    /// A big number (`.n`): Inter Black with tabular digits, -0.02em.
    fn n(&mut self, x: f32, top: f32, lh: f32, size: f32, c: Col, text: &str) -> f32 {
        self.text(x, top, lh, size, W::N, -0.02 * size, Paint::C(c), text)
    }

    fn n_width(&self, size: f32, text: &str) -> f32 {
        self.width(W::N, size, -0.02 * size, text)
    }

    // ── pieces the designs share ──

    /// The Poké Ball and "Poke Together Companion" (`.brand`), 11px bold.
    fn brand(&mut self, x: f32, top: f32, c: Col, text: &str) {
        self.ball(x, top + 1.0);
        self.t(x + 21.0, top, 15.95, 11.0, W::B, c, text);
    }

    /// The 14px Poké Ball (`.ball`).
    fn ball(&mut self, x: f32, y: f32) {
        let (cx, cy) = (x + 7.0, y + 7.0);
        self.circle(cx, cy, 7.0, Col::rgb(17, 17, 17));
        let s = self.s;
        self.c.rrect_with((x + 2.0) * s, (y + 2.0) * s, 10.0 * s, 10.0 * s, 5.0 * s, |_, py| {
            if (py as f32 + 0.5) / s < cy {
                RED
            } else {
                Col::rgb(255, 255, 255)
            }
        });
        // the button: 4px white with a 1.5px ring
        self.circle(cx, cy, 3.5, Col::rgb(17, 17, 17));
        self.circle(cx, cy, 2.0, Col::rgb(255, 255, 255));
    }

    /// A pill (`.chip`): 10px bold on white 7%, 3px 8px padding.
    fn chip(&mut self, right: f32, y: f32, text: &str) {
        let w = self.width(W::B, 10.0, 0.0, text) + 16.0;
        self.fill(right - w, y, w, 20.5, 10.25, white(0.07));
        self.t(right - w + 8.0, y + 3.0, 14.5, 10.0, W::B, Col::rgb(217, 220, 239), text);
    }

    /// A round picture (`.sprite`): the radial background, a 1px ring, and
    /// the sprite or avatar inside (else `fallback` text).
    fn pic(&mut self, x: f32, y: f32, d: f32, ring: Col, img: Option<&Img>, round_img: bool, fallback: &str, fsize: f32) {
        let (cx, cy, r) = (x + d / 2.0, y + d / 2.0, d / 2.0);
        let far = ((d * 0.5).powi(2) + (d * 0.6).powi(2)).sqrt();
        let (a, b) = (Col::rgb(44, 51, 80), Col::rgb(26, 30, 48));
        let paint = move |px: f32, py: f32| a.mix(b, (((px - cx).powi(2) + (py - (y + d * 0.4)).powi(2)).sqrt() / far).clamp(0.0, 1.0));
        self.rect(x, y, d, d, r, Paint::F(&paint));
        match img {
            Some(im) if round_img => self.blit_round(im, cx, cy, r - 1.0),
            Some(im) => {
                // pixel art, as big as fits, crisp
                let s = self.s;
                let fit = (d * 0.92 * s) / (im.w.max(im.h) as f32);
                let scale = if fit >= 1.0 { fit.floor().max(1.0) } else { fit };
                let (w, h) = (im.w as f32 * scale, im.h as f32 * scale);
                self.c.blit_nearest(im, cx * s - w / 2.0, cy * s - h / 2.0, scale);
            }
            None => {
                let wd = self.width(W::Xb, fsize, 0.0, fallback);
                let lh = fsize * 1.45;
                self.t(cx - wd / 2.0, cy - lh / 2.0, lh, fsize, W::Xb, Col::rgb(125, 134, 168), fallback);
            }
        }
        self.border(x, y, d, d, r, 1.0, ring);
    }

    /// A photo inside a circle (the avatar), smooth.
    fn blit_round(&mut self, im: &Img, cx: f32, cy: f32, r: f32) {
        let s = self.s;
        let (pcx, pcy, pr) = (cx * s, cy * s, r * s);
        let side = im.w.min(im.h) as f32;
        let (ox, oy) = ((im.w as f32 - side) / 2.0, (im.h as f32 - side) / 2.0);
        self.c.fill_with((pcx - pr) as i32 - 1, (pcy - pr) as i32 - 1, (pcx + pr) as i32 + 2, (pcy + pr) as i32 + 2, |px, py| {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let d = ((fx - pcx).powi(2) + (fy - pcy).powi(2)).sqrt();
            let cov = (pr + 0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                return None;
            }
            let u = ox + ((fx - (pcx - pr)) / (2.0 * pr)) * side;
            let v = oy + ((fy - (pcy - pr)) / (2.0 * pr)) * side;
            let (ix, iy) = ((u as u32).min(im.w - 1), (v as u32).min(im.h - 1));
            let i = ((iy * im.w + ix) * 4) as usize;
            let c = Col(im.px[i], im.px[i + 1], im.px[i + 2], im.px[i + 3]);
            Some((c, cov))
        });
    }

    /// The luck meter (`.meter`): blue, green, gold, orange, red bands, and
    /// the white marker at `at` (0..1).
    fn meter(&mut self, x: f32, y: f32, w: f32, at: f32) {
        let stops = |t: f32| {
            if t < 0.20 {
                BLUE
            } else if t < 0.45 {
                GREEN
            } else if t < 0.70 {
                GOLD
            } else if t < 0.90 {
                ORANGE
            } else {
                RED
            }
        };
        let paint = move |px: f32, _py: f32| stops((px - x) / w);
        self.rect(x, y, w, 8.0, 4.0, Paint::F(&paint));
        let mx = x + w * at.clamp(0.0, 1.0);
        self.circle(mx, y + 4.0, 7.0, Col::rgb(17, 17, 17));
        self.circle(mx, y + 4.0, 4.0, Col::rgb(255, 255, 255));
    }

    /// A row of chart bars (`.bars`): flex bars `gap` apart, each as tall
    /// as its share of the biggest; empty ones a 3% stub. Stars (✦) over
    /// the bars in `stars`.
    fn bars(&mut self, x: f32, y: f32, w: f32, h: f32, gap: f32, vals: &[u64], stars: &[usize], on: Col) {
        if vals.is_empty() {
            return;
        }
        let n = vals.len() as f32;
        let bw = (w - gap * (n - 1.0)) / n;
        let max = vals.iter().copied().max().unwrap_or(0).max(1) as f32;
        for (i, v) in vals.iter().enumerate() {
            let bx = x + i as f32 * (bw + gap);
            let pct = if *v > 0 { (*v as f32 / max * 100.0).max(6.0) } else { 3.0 };
            let bh = h * pct / 100.0;
            let col = if *v > 0 { on } else { BAR_OFF };
            self.bar(bx, y + h - bh, bw, bh, 3.0, col);
            if stars.contains(&i) {
                // `.star:after`: 12px ✦ at the bar's top - 15px, centred
                let sw = STAR_ADV * 12.0;
                self.t(bx + bw / 2.0 - sw / 2.0, y + h - bh - 15.0, 17.4, 12.0, W::R, GOLD, "✦");
            }
        }
    }

    /// The "when hunters find this shiny" curve: P(found) = 1 - e^-x for x
    /// in 0..xmax, in a box; the gradient runs blue → red along it.
    fn curve(&mut self, bx: f32, by: f32, bw: f32, bh: f32, pad: f32, xmax: f32, stroke: f32) {
        let pts: Vec<(f32, f32)> = (0..=60)
            .map(|i| {
                let x = i as f32 / 60.0 * xmax;
                let y = 1.0 - (-x).exp();
                (bx + pad + i as f32 / 60.0 * (bw - 2.0 * pad), by + bh - pad - y * (bh - 2.0 * pad))
            })
            .collect();
        let (x0, x1) = (pts[0].0, pts[60].0);
        let stops = [(0.0, BLUE), (0.3, GREEN), (0.55, GOLD), (0.8, ORANGE), (1.0, RED)];
        for w in pts.windows(2) {
            let t = ((w[0].0 + w[1].0) / 2.0 - x0) / (x1 - x0);
            self.line(w[0].0, w[0].1, w[1].0, w[1].1, stroke, at_stop(&stops, t));
        }
    }

    /// Where `mult` sits on that curve.
    fn curve_at(bx: f32, by: f32, bw: f32, bh: f32, pad: f32, xmax: f32, mult: f64) -> (f32, f32) {
        let x = (mult as f32).clamp(0.0, xmax);
        let y = 1.0 - (-x).exp();
        (bx + pad + x / xmax * (bw - 2.0 * pad), by + bh - pad - y * (bh - 2.0 * pad))
    }

    /// The You marker: white dot with a dark ring.
    fn you(&mut self, x: f32, y: f32, r: f32, ring: f32) {
        self.circle(x, y, r + ring / 2.0, Col::rgb(17, 17, 17));
        self.circle(x, y, (r - ring / 2.0).max(0.5), Col::rgb(255, 255, 255));
    }

    /// A `.rowl`: label left (muted), value right, a hairline under it.
    fn row(&mut self, x: f32, y: f32, w: f32, h: f32, size: f32, label: &str, value: &str, vcol: Col, rule: bool, label_first_col: Col, label_w: W) {
        let lh = size * 1.45;
        let top = y + (h - lh) / 2.0 - if rule { 0.5 } else { 0.0 };
        self.t(x, top, lh, size, label_w, label_first_col, label);
        self.tr(x + w, top, lh, size, W::R, vcol, value);
        if rule {
            self.fill(x, y + h - 1.0, w, 1.0, 0.0, white(0.05));
        }
    }

    fn done(self) -> Canvas {
        self.c
    }
}

// ───────────────────────────── numbers in words ─────────────────────────────

pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// "7h 41m", "41m", "0m".
pub fn hm(secs: u64) -> String {
    let (h, m) = (secs / 3600, secs % 3600 / 60);
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn mult_text(m: f64) -> String {
    format!("{m:.2}×")
}

fn pct_text(p: f64) -> String {
    let v = p * 100.0;
    if v >= 99.95 {
        ">99.9%".into()
    } else if v >= 99.0 {
        format!("{v:.1}%")
    } else if v < 1.0 {
        "<1%".into()
    } else {
        format!("{}%", v.round() as u32)
    }
}

/// Luck's colour: lucky green, about even gold, unlucky red.
fn mult_col(m: f64) -> Col {
    if m < 1.0 {
        GREEN
    } else if m < 1.5 {
        GOLD
    } else {
        RED
    }
}

/// Where the luck marker sits on the meter: × the odds on a 0 to 3×
/// scale, as on the Stats page (kept just inside the end).
fn luck_at(mult: Option<f64>) -> f32 {
    mult.map(|m| (m as f32 / 3.0).min(0.97)).unwrap_or(0.33)
}

// ───────────────────────────── the images ─────────────────────────────

pub fn render(data: &Data, layout: Layout, who: &Who) -> Canvas {
    match (data, layout) {
        (Data::Overview(o), Layout::Poster) => poster_overview(o, who),
        (Data::Hunt(h), Layout::Poster) => poster_hunt(h, who),
        (Data::Overview(o), Layout::Dashboard) => dash_overview(o, who),
        (Data::Hunt(h), Layout::Dashboard) => dash_hunt(h, who),
        (Data::Overview(o), Layout::Wrapped) => wrapped_overview(o, who),
        (Data::Hunt(h), Layout::Wrapped) => wrapped_hunt(h, who),
    }
}

fn initial(who: &Who) -> String {
    who.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "★".into())
}

// ── Poster (4:5) ──

fn poster_bg(d: &mut D, top: Col, mid: Col) {
    let stops = [(0.0, top), (0.45, mid), (1.0, Col::rgb(16, 18, 28))];
    let g = linear(160.0, 0.0, 0.0, 432.0, 540.0, &stops);
    d.rect(0.0, 0.0, 432.0, 540.0, 0.0, Paint::F(&g));
}

/// The Poster's stat tiles (white 4%, radius 10, 8px padding).
fn tiles(d: &mut D, y: f32, tiles: &[(&str, String, Col, Option<&str>)]) {
    let n = tiles.len() as f32;
    let gap = 7.0;
    let tw = (388.0 - gap * (n - 1.0)) / n;
    for (i, (label, value, col, small)) in tiles.iter().enumerate() {
        let x = 22.0 + i as f32 * (tw + gap);
        d.fill(x, y, tw, 55.9, 10.0, white(0.04));
        d.k(x + 8.0, y + 8.0, MUTED, label);
        let w = d.n(x + 8.0, y + 21.8, 26.1, 18.0, *col, value);
        if let Some(sm) = small {
            // `<small>`: 10px on the number's baseline, muted
            let b = d.baseline(y + 21.8, 26.1, 18.0);
            d.text_b(x + 8.0 + w, b, 10.0, W::N, -0.36, Paint::C(MUTED), sm);
        }
    }
}

fn poster_overview(o: &Overview, who: &Who) -> Canvas {
    let mut d = D::new(Layout::Poster);
    poster_bg(&mut d, Col::rgb(27, 22, 64), Col::rgb(20, 22, 38));
    d.brand(22.0, 24.3, BRAND, "Poke Together Companion");
    d.chip(410.0, 22.0, &o.chip);
    // you
    d.pic(22.0, 58.5, 44.0, Col::rgb(51, 58, 88), who.avatar.as_ref(), true, &initial(who), 13.0);
    let title = if who.name.is_empty() { "My hunting".to_string() } else { format!("{}'s hunting", who.name) };
    d.t(78.0, 62.0, 21.75, 15.0, W::Xb, TEXT, &title);
    let sub = if who.level.is_empty() { o.dates.clone() } else { format!("{} · {}", o.dates, who.level) };
    d.t(78.0, 83.8, 15.225, 10.5, W::R, MUTED, &sub);
    // the hero number, in a blue gradient across its own width
    let hero = thousands(o.encounters);
    let hw = d.n_width(52.0, &hero);
    let stops = [(0.0, Col::rgb(127, 183, 255)), (1.0, BLUE)];
    let g = linear(90.0, 22.0, 0.0, hw, 1.0, &stops);
    d.text(22.0, 116.5, 52.0, 52.0, W::N, -1.04, Paint::F(&g), &hero);
    let b = d.baseline(116.5, 52.0, 52.0);
    d.text_b(22.0 + hw + 6.0, b, 13.0, W::Sb, 0.0, Paint::C(MUTED), "encounters");
    // tiles
    let pace = o.pace.map(thousands).unwrap_or_else(|| "—".into());
    tiles(
        &mut d,
        180.5,
        &[
            ("Shinies", o.shinies.to_string(), GOLD, None),
            ("Hunted", hm(o.secs), TEXT, None),
            ("Pace", pace, TEXT, o.pace.map(|_| "/h")),
            ("Days", o.days.to_string(), TEXT, None),
        ],
    );
    // activity
    d.k(22.0, 252.4, MUTED, "Activity · ✦ shinies caught");
    d.bars(22.0, 282.1, 388.0, 54.0, 2.0, &o.bars, &o.stars, BLUE);
    axis(&mut d, 22.0, 340.2, 388.0, &o.axis);
    // luck
    d.k(22.0, 367.2, MUTED, "Luck");
    let (lt, lc) = match o.luck {
        Some(m) => (mult_text(m), mult_col(m)),
        None => ("—".to_string(), MUTED),
    };
    d.n(22.0, 381.0, 37.7, 26.0, lc, &lt);
    match o.luckier {
        Some(l) => {
            let a = d.t(107.4, 377.8, 15.225, 10.5, W::R, BRAND, "Luckier than ");
            let b = d.t(107.4 + a, 377.8, 15.225, 10.5, W::B, BRAND, &format!("{l}%"));
            d.t(107.4 + a + b, 377.8, 15.225, 10.5, W::R, BRAND, " of hunters");
        }
        None => {
            d.t(107.4, 377.8, 15.225, 10.5, W::R, BRAND, "Your luck shows once you catch a shiny");
        }
    }
    d.meter(107.4, 400.0, 302.6, luck_at(o.luck));
    // highlights: two columns of rows
    let best = o.best_day.as_ref().map(|(n, day, _)| format!("{} · {day}", thousands(*n))).unwrap_or_else(|| "—".into());
    let rows = [
        ("Best day", best),
        ("Longest session", o.longest.map(hm).unwrap_or_else(|| "—".into())),
        ("Fastest", o.fastest.map(|p| format!("{}/h", thousands(p))).unwrap_or_else(|| "—".into())),
        ("Streak", format!("{} day{}", o.streak, if o.streak == 1 { "" } else { "s" })),
    ];
    for (i, (l, v)) in rows.iter().enumerate() {
        let (cx, cy) = (if i % 2 == 0 { 22.0 } else { 223.0 }, if i < 2 { 428.7 } else { 458.3 });
        d.row(cx, cy, 187.0, 29.7, 11.5, l, v, TEXT, i < 3, MUTED, W::R);
    }
    // the shelf
    shelf_row(&mut d, o, 22.0, 488.0, 30.0, 38.0, 136.0, 495.8);
    d.done()
}

/// Up to three shinies in gold rings and the line beside them.
fn shelf_row(d: &mut D, o: &Overview, x: f32, y: f32, size: f32, step: f32, tx: f32, ty: f32) {
    if o.shelf.is_empty() {
        d.t(x, ty, 14.5, 10.0, W::R, MUTED, "No shinies in this stretch yet. Keep going!");
        return;
    }
    for (i, p) in o.shelf.iter().take(3).enumerate() {
        d.pic(x + i as f32 * step, y, size, GOLD, p.sprite.as_ref(), false, "✦", 9.0);
    }
    let tx = tx - (3 - o.shelf.len().min(3)) as f32 * step;
    d.t(tx, ty, 14.5, 10.0, W::R, MUTED, &o.shelf_line);
}

fn poster_hunt(h: &Hunt, who: &Who) -> Canvas {
    let mut d = D::new(Layout::Poster);
    poster_bg(&mut d, Col::rgb(42, 21, 48), Col::rgb(23, 20, 36));
    d.brand(22.0, 24.3, BRAND, "Poke Together Companion");
    d.chip(410.0, 22.0, if h.caught { "CAUGHT" } else { "HUNT" });
    d.pic(22.0, 58.5, 78.0, Col::rgb(51, 58, 88), h.sprite.as_ref(), false, &h.name, 12.0);
    d.t(114.0, 72.1, 34.8, 24.0, W::K, TEXT, &h.name);
    let by = if who.name.is_empty() { String::new() } else { format!(" · {}", who.name) };
    d.t(114.0, 106.9, 15.95, 11.0, W::R, MUTED, &format!("{} · {}{by}", h.method, h.odds));
    let cnt = thousands(h.count);
    let cw = d.text(22.0, 148.0, 52.0, 52.0, W::N, -1.04, Paint::C(TEXT), &cnt);
    let b = d.baseline(148.0, 52.0, 52.0);
    d.text_b(22.0 + cw + 6.0, b, 13.0, W::Sb, 0.0, Paint::C(MUTED), &h.unit);
    // tiles
    let n = 3.0;
    let gap = 7.0;
    let tw = (388.0 - gap * (n - 1.0)) / n;
    let vals = [
        ("Odds", mult_text(h.mult), mult_col(h.mult)),
        ("Hunted", hm(h.secs), TEXT),
        ("Pace", h.pace.map(|p| format!("{}/h", thousands(p))).unwrap_or_else(|| "—".into()), TEXT),
    ];
    for (i, (l, v, c)) in vals.iter().enumerate() {
        let x = 22.0 + i as f32 * (tw + gap);
        d.fill(x, 212.5, tw, 55.9, 10.0, white(0.04));
        d.k(x + 8.0, 220.5, MUTED, l);
        d.n(x + 8.0, 234.3, 26.1, 18.0, *c, v);
    }
    // the curve, You on it
    d.k(22.0, 284.4, MUTED, "When hunters find this shiny");
    let (cx, cy, cw2, ch) = (22.0, 304.1, 388.0, 84.0);
    d.fill(cx, cy, cw2, ch, 8.0, white(0.02));
    let xmax = (h.mult as f32 * 1.01).max(3.0);
    d.curve(cx, cy, cw2, ch, 8.0, xmax, 3.0);
    let (yx, yy) = D::curve_at(cx, cy, cw2, ch, 8.0, xmax, h.mult);
    d.you(yx, yy, 6.0, 3.0);
    let label = format!("You, at {:.1}×", h.mult);
    let lw = d.width(W::Xb, 11.0, 0.0, &label);
    // as in the design: left of the dot and 20 below it; early on (the dot
    // low and to the left) above and right of it instead, inside the chart
    let (lx, ly) = if yx - 80.0 >= cx + 4.0 && yy + 20.0 <= cy + ch - 4.0 { (yx - 80.0, yy + 20.0) } else { ((yx + 12.0).min(cx + cw2 - lw - 4.0), (yy - 10.0).max(cy + 14.0)) };
    d.text_b(lx, ly, 11.0, W::Xb, 0.0, Paint::C(Col::rgb(255, 255, 255)), &label);
    let head = if h.caught { format!("Caught at {}", pct_text(h.found)) } else { format!("{} of hunters would have found it by now", pct_text(h.found)) };
    d.t(22.0, 398.1, 18.125, 12.5, W::Xb, TEXT, &head);
    let sub = if h.caught {
        format!("{} {} at {} odds.", thousands(h.count), h.unit, h.odds)
    } else {
        format!("Still hunting. Every {} is a fresh {}.", singular(&h.unit_cap), h.odds)
    };
    d.t(22.0, 419.3, 15.225, 10.5, W::R, MUTED, &sub);
    // the last days hunted
    d.k(22.0, 448.5, MUTED, &format!("{} each day", h.unit_cap));
    let max = h.per_day.iter().map(|p| p.1).max().unwrap_or(1).max(1) as f32;
    for (i, (day, n)) in h.per_day.iter().take(2).enumerate() {
        let y = 468.2 + i as f32 * 19.5;
        d.t(22.0, y, 14.5, 10.0, W::R, MUTED, day);
        let bw = 230.0 * (*n as f32 / max);
        d.fill(70.0, y + 2.3, bw.max(10.0), 10.0, 5.0, PINK);
        d.t(70.0 + bw.max(10.0) + 8.0, y, 14.5, 10.0, W::R, TEXT, &thousands(*n));
    }
    // footer
    d.t(22.0, 503.5, 14.5, 10.0, W::R, MUTED, &format!("Started {}", h.started));
    d.tr(410.0, 503.5, 14.5, 10.0, W::R, MUTED, &format!("{} day{} · {} session{}", h.days, plural(h.days), h.sessions, plural(h.sessions)));
    d.done()
}

/// Three labels under a chart, spread out (justify-content: space-between).
fn axis(d: &mut D, x: f32, top: f32, w: f32, labels: &[String; 3]) {
    let ws: Vec<f32> = labels.iter().map(|l| d.width(W::R, 9.0, 0.0, l)).collect();
    let gap = (w - ws.iter().sum::<f32>()) / 2.0;
    let mut at = x;
    for (l, lw) in labels.iter().zip(&ws) {
        d.t(at, top, 13.05, 9.0, W::R, DIM, l);
        at += lw + gap;
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// "Eggs" → "Egg", "Encounters" → "encounter".
fn singular(unit_cap: &str) -> String {
    match unit_cap {
        "Eggs" => "Egg".into(),
        "Encounters" => "encounter".into(),
        u => u.trim_end_matches('s').to_lowercase(),
    }
}

// ── Dashboard (16:9) ──

fn mcard(d: &mut D, x: f32, y: f32, w: f32, h: f32) {
    d.fill(x, y, w, h, 12.0, CARD);
    d.border(x, y, w, h, 12.0, 1.0, LINE);
}

fn kpis(d: &mut D, y: f32, items: &[(&str, String, Col, Col)]) {
    let n = items.len() as f32;
    let gap = 7.0;
    let w = (608.0 - gap * (n - 1.0)) / n;
    for (i, (l, v, lc, vc)) in items.iter().enumerate() {
        let x = 16.0 + i as f32 * (w + gap);
        mcard(d, x, y, w, 59.3);
        d.k(x + 9.0, y + 9.0, *lc, l);
        d.n(x + 9.0, y + 22.8, 27.55, 19.0, *vc, v);
    }
}

fn dash_overview(o: &Overview, who: &Who) -> Canvas {
    let mut d = D::new(Layout::Dashboard);
    d.fill(0.0, 0.0, 640.0, 360.0, 0.0, Col::rgb(17, 18, 27));
    d.pic(16.0, 18.3, 28.0, Col::rgb(51, 58, 88), who.avatar.as_ref(), true, &initial(who), 10.0);
    let title = if who.name.is_empty() { cap_first(&o.span) } else { format!("{} · {}", who.name, o.span) };
    d.t(53.0, 16.0, 18.85, 13.0, W::Xb, TEXT, &title);
    d.t(53.0, 34.8, 13.775, 9.5, W::R, MUTED, &o.dates);
    d.brand(460.9, 24.3, BRAND, "Poke Together Companion");
    let (lt, lcol) = match o.luck {
        Some(m) => (mult_text(m), TEXT),
        None => ("—".to_string(), MUTED),
    };
    kpis(
        &mut d,
        58.6,
        &[
            ("Encounters", thousands(o.encounters), BLUE, TEXT),
            ("Hunt time", hm(o.secs), VIOLET, TEXT),
            ("Shinies", o.shinies.to_string(), GOLD, TEXT),
            ("Pace", o.pace.map(|p| format!("{}/h", thousands(p))).unwrap_or_else(|| "—".into()), AQUA, TEXT),
            ("Luck", lt, GREEN, lcol),
        ],
    );
    mcard(&mut d, 16.0, 124.9, 608.0, 85.8);
    d.k(27.0, 133.9, MUTED, "Activity · ✦ shinies");
    d.bars(27.0, 161.7, 586.0, 40.0, 2.0, &o.bars, &o.stars, BLUE);
    // when you hunt: the 24-hour clock
    mcard(&mut d, 16.0, 217.7, 210.8, 107.8);
    d.k(27.0, 226.7, MUTED, "When you hunt");
    clock(&mut d, 84.4, 242.5, 74.0, &o.hours);
    // days of the week
    mcard(&mut d, 233.8, 217.7, 191.6, 107.8);
    d.k(244.8, 226.7, MUTED, "Days of the week");
    d.bars(244.8, 248.5, 169.6, 52.0, 5.0, &o.weekdays, &[], AQUA);
    // the day letters, spread out (justify-content: space-between)
    let letters = ["M", "T", "W", "T", "F", "S", "S"];
    let ws: Vec<f32> = letters.iter().map(|l| d.width(W::R, 8.5, 0.0, l)).collect();
    let gap = (169.6 - ws.iter().sum::<f32>()) / 6.0;
    let mut x = 244.8;
    for (l, w) in letters.iter().zip(&ws) {
        d.t(x, 303.5, 12.325, 8.5, W::R, DIM, l);
        x += w + gap;
    }
    // met most
    mcard(&mut d, 432.4, 217.7, 191.6, 107.8);
    d.k(443.4, 226.7, MUTED, "Met most");
    let met: Vec<&(String, u64)> = o.met.iter().take(3).collect();
    if met.is_empty() {
        d.t(443.4, 244.5, 14.5, 10.0, W::R, MUTED, "Nothing met yet");
    }
    for (i, (name, n)) in met.iter().enumerate() {
        let y = 240.5 + i as f32 * 23.5;
        let last = i + 1 == met.len();
        d.row(443.4, y, 169.6, if last { 22.5 } else { 23.5 }, 10.0, name, &thousands(*n), MUTED, !last, TEXT, W::R);
    }
    d.done()
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// The 24-hour clock: a spoke per hour, longer for busier hours, the
/// busiest hour in the middle. `x, y, size`: its box (a 100-unit viewBox).
fn clock(d: &mut D, x: f32, y: f32, size: f32, hours: &[u64; 24]) {
    let k = size / 100.0;
    let m = hours.iter().copied().max().unwrap_or(0).max(1) as f32;
    let (cx, cy) = (x + 50.0 * k, y + 50.0 * k);
    d.circle(cx, cy, 18.0 * k, Col::rgb(31, 34, 51));
    for (i, v) in hours.iter().enumerate() {
        let a = (i as f32 / 24.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let (r1, r2) = (22.0, 22.0 + (*v as f32 / m) * 24.0);
        let col = if *v > 0 { VIOLET } else { LINE };
        d.line(cx + r1 * k * a.cos(), cy + r1 * k * a.sin(), cx + r2 * k * a.cos(), cy + r2 * k * a.sin(), 3.0 * k, col);
    }
    let busiest = hours.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).filter(|_| m > 1.0 || hours.iter().any(|v| *v > 0));
    let label = busiest.map(hour_label).unwrap_or_else(|| "—".into());
    let fs = 10.0 * k;
    let lw = d.width(W::Xb, fs, 0.0, &label);
    // SVG text: its baseline at 54
    d.text_b(cx - lw / 2.0, y + 54.0 * k, fs, W::Xb, 0.0, Paint::C(TEXT), &label);
}

/// 21 → "9pm".
pub fn hour_label(h: usize) -> String {
    match h {
        0 => "12am".into(),
        12 => "12pm".into(),
        h if h < 12 => format!("{h}am"),
        h => format!("{}pm", h - 12),
    }
}

fn dash_hunt(h: &Hunt, _who: &Who) -> Canvas {
    let mut d = D::new(Layout::Dashboard);
    d.fill(0.0, 0.0, 640.0, 360.0, 0.0, Col::rgb(17, 18, 27));
    d.pic(16.0, 16.0, 40.0, Col::rgb(51, 58, 88), h.sprite.as_ref(), false, &h.name, 9.0);
    d.t(66.0, 17.5, 23.2, 16.0, W::K, TEXT, &format!("{} hunt", h.name));
    d.t(66.0, 40.7, 13.775, 9.5, W::R, MUTED, &format!("{} · {} · since {}", h.method, h.odds, h.started));
    d.brand(460.9, 28.0, BRAND, "Poke Together Companion");
    let n = 4.0;
    let gap = 7.0;
    let w = (608.0 - gap * (n - 1.0)) / n;
    let items = [
        (h.unit_cap.as_str(), thousands(h.count), TEXT),
        ("Odds", mult_text(h.mult), mult_col(h.mult)),
        ("Hunted", hm(h.secs), TEXT),
        ("Pace", h.pace.map(|p| format!("{}/h", thousands(p))).unwrap_or_else(|| "—".into()), TEXT),
    ];
    for (i, (l, v, c)) in items.iter().enumerate() {
        let x = 16.0 + i as f32 * (w + gap);
        mcard(&mut d, x, 66.0, w, 59.3);
        d.k(x + 9.0, 75.0, MUTED, l);
        d.n(x + 9.0, 88.8, 27.55, 19.0, *c, v);
    }
    // the curve (a 330×110 viewBox shown 96 tall, centred)
    mcard(&mut d, 16.0, 132.3, 369.8, 134.0);
    d.k(27.0, 141.3, MUTED, "When hunters find this shiny");
    let k = 96.0 / 110.0;
    let (vx, vy) = (27.0 + (347.8 - 330.0 * k) / 2.0, 155.1);
    let xmax = (h.mult as f32 * 1.01).max(3.0);
    d.curve(vx, vy, 330.0 * k, 110.0 * k, 6.0 * k, xmax, 3.0 * k);
    let (yx, yy) = D::curve_at(vx, vy, 330.0 * k, 110.0 * k, 6.0 * k, xmax, h.mult);
    d.you(yx, yy, 5.0 * k, 3.0 * k);
    let label = format!("You · {}", pct_text(h.found));
    let fs = 10.0 * k;
    let lw = d.width(W::Xb, fs, 0.0, &label);
    let bottom = vy + 110.0 * k;
    let (lx, ly) = if yx - 74.0 * k >= vx && yy + 18.0 * k <= bottom - 4.0 { (yx - 74.0 * k, yy + 18.0 * k) } else { ((yx + 10.0 * k).min(vx + 330.0 * k - lw), (yy - 9.0 * k).max(vy + 12.0)) };
    d.text_b(lx, ly, fs, W::Xb, 0.0, Paint::C(Col::rgb(255, 255, 255)), &label);
    // from here
    mcard(&mut d, 392.8, 132.3, 231.2, 134.0);
    d.k(403.8, 141.3, MUTED, if h.caught { "It took" } else { "From here" });
    let rows: Vec<(String, String, Col)> = if h.caught {
        vec![
            ("Odds".into(), h.odds.clone(), TEXT),
            ("Luck".into(), mult_text(h.mult), mult_col(h.mult)),
            ("Hunters luckier".into(), pct_text(1.0 - h.found), MUTED),
        ]
    } else {
        vec![
            ("50% within".into(), format!("{} more", thousands(h.ahead[0])), GOLD),
            ("90% within".into(), format!("{} more", thousands(h.ahead[1])), ORANGE),
            ("99% within".into(), format!("{} more", thousands(h.ahead[2])), RED),
        ]
    };
    for (i, (l, v, c)) in rows.iter().enumerate() {
        let last = i == 2;
        d.row(403.8, 155.1 + i as f32 * 25.5, 209.2, if last { 24.5 } else { 25.5 }, 10.0, l, v, *c, !last, MUTED, W::R);
    }
    // best day, sessions, fun fact
    let small = [
        ("Best day", h.best_day.as_ref().map(|b| thousands(b.0)).unwrap_or_else(|| "—".into()), h.best_day.as_ref().map(|b| b.1.clone()).unwrap_or_default()),
        ("Sessions", h.sessions.to_string(), h.longest.map(|l| format!("longest {}", hm(l))).unwrap_or_default()),
    ];
    for (i, (l, v, sub)) in small.iter().enumerate() {
        let x = 16.0 + i as f32 * 205.0;
        mcard(&mut d, x, 273.3, 198.0, 68.7);
        d.k(x + 11.0, 282.3, MUTED, l);
        d.n(x + 11.0, 296.0, 23.2, 16.0, TEXT, v);
        d.t(x + 11.0, 319.2, 13.775, 9.5, W::R, MUTED, sub);
    }
    mcard(&mut d, 426.0, 273.3, 198.0, 68.7);
    d.k(437.0, 282.3, MUTED, "Fun fact");
    wrap(&mut d, 437.0, 299.0, 176.0, 14.5, 10.0, W::R, TEXT, &h.fun_fact, 2);
    d.done()
}

/// Text wrapped to `w`, at most `lines` lines.
fn wrap(d: &mut D, x: f32, top: f32, w: f32, lh: f32, size: f32, wt: W, c: Col, text: &str, lines: usize) -> f32 {
    let mut line = String::new();
    let mut y = top;
    let mut n = 0;
    for word in text.split_whitespace() {
        let try_line = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if d.width(wt, size, 0.0, &try_line) > w && !line.is_empty() {
            d.t(x, y, lh, size, wt, c, &line);
            y += lh;
            n += 1;
            line = word.to_string();
            if n + 1 > lines {
                return y;
            }
        } else {
            line = try_line;
        }
    }
    if !line.is_empty() {
        d.t(x, y, lh, size, wt, c, &line);
        y += lh;
    }
    y
}

// ── Wrapped (9:16) ──

fn band(d: &mut D, y: f32, h: f32, c: Col) {
    d.fill(0.0, y, 300.0, h, 0.0, c);
}

fn wrapped_overview(o: &Overview, who: &Who) -> Canvas {
    let mut d = D::new(Layout::Wrapped);
    d.fill(0.0, 0.0, 300.0, 533.4, 0.0, Col::rgb(15, 16, 32));
    let stops = [(0.0, Col::rgb(107, 61, 245)), (1.0, BLUE)];
    let g = linear(135.0, 0.0, 0.0, 300.0, 144.7, &stops);
    d.rect(0.0, 0.0, 300.0, 144.7, 0.0, Paint::F(&g));
    let head = if who.name.is_empty() { format!("My {}", o.span) } else { format!("{}'s {}", who.name, o.span) };
    d.brand(18.0, 16.0, Col::rgb(255, 255, 255), &head);
    d.t(18.0, 45.9, 17.4, 12.0, W::B, TEXT.alpha(0.85), "You met");
    d.n(18.0, 63.3, 48.0, 48.0, TEXT, &thousands(o.encounters));
    d.t(18.0, 111.3, 17.4, 12.0, W::B, TEXT.alpha(0.85), &format!("Pokémon in {}", hm(o.secs)));
    // shinies
    let ink = Col::rgb(42, 29, 0);
    band(&mut d, 144.7, 106.9, GOLD);
    d.t(18.0, 157.7, 17.4, 12.0, W::Xb, ink, "and found");
    let nw = d.n(18.0, 181.1, 40.0, 40.0, ink, &o.shinies.to_string());
    let stars = "✦".repeat((o.shinies as usize).clamp(1, 3));
    let word = if o.shinies == 1 { "shiny" } else { "shinies" };
    d.t(18.0 + nw + 8.0, 192.4, 17.4, 12.0, W::Xb, ink, &format!("{word} {stars}"));
    let line = if o.shelf_line.is_empty() { "Keep hunting!".to_string() } else { o.shelf_line.clone() };
    d.t(18.0, 224.1, 14.5, 10.0, W::B, ink, &line);
    // best day
    band(&mut d, 251.6, 140.9, Col::rgb(27, 29, 44));
    match &o.best_day {
        Some((n, day, weekday)) => {
            d.t(18.0, 265.6, 17.4, 12.0, W::B, MUTED, &format!("Your best day was a {weekday}"));
            let num = format!("{} ", thousands(*n));
            let w = d.n(18.0, 285.0, 43.5, 30.0, TEXT, &num);
            // "on 11 Nov": a 12px span inside the .n (900, -0.6px)
            let b = d.baseline(285.0, 43.5, 30.0);
            d.text_b(18.0 + w, b, 12.0, W::N, -0.6, Paint::C(MUTED), &format!("on {day}"));
        }
        None => {
            d.t(18.0, 265.6, 17.4, 12.0, W::B, MUTED, "Your best day");
            d.n(18.0, 285.0, 43.5, 30.0, TEXT, "—");
        }
    }
    d.bars(18.0, 338.5, 264.0, 40.0, 2.0, &o.bars, &o.stars, Col::rgb(107, 61, 245));
    // luck
    let ink = Col::rgb(5, 48, 26);
    band(&mut d, 392.5, 101.3, GREEN);
    match o.luckier {
        Some(l) => {
            d.t(18.0, 406.5, 17.4, 12.0, W::Xb, ink, "Luckier than");
            d.n(18.0, 423.9, 40.0, 40.0, ink, &format!("{l}%"));
            d.t(18.0, 463.9, 15.95, 11.0, W::B, ink, &format!("of hunters · {} the odds", o.luck.map(mult_text).unwrap_or_default()));
        }
        None => {
            d.t(18.0, 406.5, 17.4, 12.0, W::Xb, ink, "Your luck");
            d.n(18.0, 423.9, 40.0, 40.0, ink, "—");
            d.t(18.0, 463.9, 15.95, 11.0, W::B, ink, "shows once you catch a shiny");
        }
    }
    // footer
    let mut foot = Vec::new();
    if let Some(f) = o.fastest {
        foot.push(format!("Fastest {}/h", thousands(f)));
    }
    if o.streak > 0 {
        foot.push(format!("streak {} day{}", o.streak, plural(o.streak)));
    }
    d.t(18.0, 505.8, 14.5, 10.0, W::R, MUTED, &foot.join(" · "));
    d.tr(282.0, 505.8, 14.5, 10.0, W::R, MUTED, "Poke Together");
    d.done()
}

fn wrapped_hunt(h: &Hunt, who: &Who) -> Canvas {
    let mut d = D::new(Layout::Wrapped);
    d.fill(0.0, 0.0, 300.0, 533.4, 0.0, Col::rgb(15, 16, 32));
    let stops = [(0.0, Col::rgb(58, 43, 107)), (1.0, Col::rgb(29, 43, 82))];
    let g = linear(135.0, 0.0, 0.0, 300.0, 106.8, &stops);
    d.rect(0.0, 0.0, 300.0, 106.8, 0.0, Paint::F(&g));
    d.pic(18.0, 22.4, 62.0, Col::rgb(51, 58, 88), h.sprite.as_ref(), false, &h.name, 11.0);
    let head = match (who.name.is_empty(), h.caught) {
        (true, false) => "Now hunting".to_string(),
        (true, true) => "Caught".to_string(),
        (false, false) => format!("{} is hunting", who.name),
        (false, true) => format!("{} caught", who.name),
    };
    d.brand(92.0, 18.0, Col::rgb(255, 255, 255), &head);
    d.t(92.0, 35.9, 37.7, 26.0, W::K, TEXT, &h.name);
    d.t(92.0, 73.6, 15.225, 10.5, W::R, TEXT.alpha(0.8), &format!("{} · {}", h.method, h.odds));
    // count
    band(&mut d, 106.8, 101.4, Col::rgb(27, 29, 44));
    let what = if h.unit == "eggs" { "Eggs hatched so far" } else { "Encounters so far" };
    d.t(18.0, 118.8, 17.4, 12.0, W::B, MUTED, what);
    d.n(18.0, 136.2, 44.1, 42.0, TEXT, &thousands(h.count));
    let pace = h.pace.map(|p| format!(" · {} an hour", thousands(p))).unwrap_or_default();
    d.t(18.0, 180.3, 15.95, 11.0, W::R, MUTED, &format!("in {}{pace}", hm(h.secs)));
    // luck band: red when unlucky, green when lucky, gold in between
    let (bg, ink) = if h.mult < 1.0 {
        (GREEN, Col::rgb(5, 48, 26))
    } else if h.mult < 1.5 {
        (GOLD, Col::rgb(42, 29, 0))
    } else {
        (RED, Col::rgb(46, 5, 5))
    };
    band(&mut d, 208.3, 111.3, bg);
    d.t(18.0, 220.3, 17.4, 12.0, W::Xb, ink, "That's");
    d.n(18.0, 237.7, 38.0, 38.0, ink, &mult_text(h.mult));
    let line = if h.caught {
        format!("the odds. Caught with {} of hunters still looking.", pct_text(1.0 - h.found))
    } else {
        format!("the odds. {} of hunters would have it by now.", pct_text(h.found))
    };
    wrap(&mut d, 18.0, 275.7, 264.0, 15.95, 11.0, W::Xb, ink, &line, 2);
    // coin flip
    band(&mut d, 319.5, 86.1, Col::rgb(27, 29, 44));
    if h.caught {
        d.t(18.0, 331.5, 17.4, 12.0, W::B, MUTED, "Where you landed");
        d.t(18.0, 351.9, 21.75, 15.0, W::Xb, TEXT, &format!("Luckier than {} of hunters", pct_text(1.0 - h.found)));
    } else {
        d.t(18.0, 331.5, 17.4, 12.0, W::B, MUTED, "Coin flip says");
        d.t(18.0, 351.9, 21.75, 15.0, W::Xb, TEXT, &format!("50/50 within the next {} {}", thousands(h.ahead[0]), h.unit_cap));
    }
    d.meter(18.0, 385.7, 264.0, luck_at(Some(h.mult)));
    // best day
    band(&mut d, 405.7, 88.8, AQUA);
    let ink = Col::rgb(3, 48, 47);
    d.t(18.0, 417.7, 17.4, 12.0, W::Xb, ink, "Best day");
    match &h.best_day {
        Some((n, day, weekday)) => {
            d.n(18.0, 435.1, 31.5, 30.0, ink, &format!("{} {}", thousands(*n), h.unit_cap));
            let long = h.longest.map(|l| format!(" · {} straight", hm(l))).unwrap_or_default();
            d.t(18.0, 466.6, 15.95, 11.0, W::B, ink, &format!("{weekday} {day}{long}"));
        }
        None => {
            d.n(18.0, 435.1, 31.5, 30.0, ink, "—");
        }
    }
    d.t(18.0, 506.5, 14.5, 10.0, W::R, MUTED, &format!("{} day{} · {} session{}", h.days, plural(h.days), h.sessions, plural(h.sessions)));
    d.tr(282.0, 506.5, 14.5, 10.0, W::R, MUTED, "Poke Together");
    d.done()
}

// ───────────────────────────── a sample (the design page's numbers) ─────────────────────────────

/// The design page's own numbers, for checking the images against it.
pub fn sample() -> (Overview, Hunt, Who) {
    let mut bars = vec![0u64; 30];
    bars[24] = 1800;
    bars[25] = 5994;
    bars[26] = 1545;
    let mut hours = [0u64; 24];
    for (i, v) in [(10, 120), (14, 240), (15, 380), (16, 300), (17, 520), (18, 900), (19, 1500), (20, 2100), (21, 1900), (22, 1200), (23, 159)] {
        hours[i] = v;
    }
    let o = Overview {
        chip: "LAST 30 DAYS".into(),
        span: "last 30 days".into(),
        dates: "17 Oct – 15 Nov 2026".into(),
        encounters: 9339,
        secs: 7 * 3600 + 41 * 60,
        shinies: 3,
        pace: Some(1214),
        days: 3,
        bars,
        stars: vec![24, 26],
        axis: ["17 Oct".into(), "1 Nov".into(), "15 Nov".into()],
        luck: Some(0.41),
        luckier: Some(72),
        best_day: Some((5994, "11 Nov".into(), "Wednesday".into())),
        longest: Some(2 * 3600 + 59 * 60),
        fastest: Some(1419),
        streak: 3,
        shelf: vec![Pic::default(), Pic::default(), Pic::default()],
        shelf_line: "Charmander · Chikorita (phase) · 1 more".into(),
        hours,
        weekdays: [0, 1800, 5994, 1545, 0, 0, 0],
        met: vec![("Beldum".into(), 6702), ("Chikorita".into(), 1980), ("Mudkip".into(), 220)],
    };
    let h = Hunt {
        name: "Beldum".into(),
        sprite: None,
        shiny: false,
        method: "Masuda + Lure".into(),
        odds: "1 in 1,024".into(),
        count: 6702,
        unit: "eggs".into(),
        unit_cap: "Eggs".into(),
        mult: 6.54,
        found: 0.9986,
        secs: 4 * 3600 + 58 * 60,
        pace: Some(1349),
        started: "10 Nov".into(),
        days: 2,
        sessions: 3,
        per_day: vec![("10 Nov".into(), 2452), ("11 Nov".into(), 4250)],
        ahead: [710, 2357, 4714],
        best_day: Some((4250, "11 Nov".into(), "Wednesday".into())),
        longest: Some(2 * 3600 + 59 * 60),
        caught: false,
        fun_fact: "Enough Eggs to fill 223 PC boxes.".into(),
    };
    let who = Who { name: "Zifu".into(), level: "Level 26 Shiny Hunter".into(), avatar: None };
    (o, h, who)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layout_draws_at_its_size() {
        let (o, h, who) = sample();
        for l in Layout::ALL {
            for data in [Data::Overview(o.clone()), Data::Hunt(h.clone())] {
                let c = render(&data, l, &who);
                assert_eq!((c.w, c.h), l.size());
            }
        }
        assert_eq!(Layout::Poster.size(), (1080, 1350));
        assert_eq!(Layout::Dashboard.size(), (1600, 900));
        assert_eq!(Layout::Wrapped.size(), (1080, 1920));
    }

    #[test]
    fn numbers_read_like_the_designs() {
        assert_eq!(thousands(9339), "9,339");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(hm(7 * 3600 + 41 * 60), "7h 41m");
        assert_eq!(hm(59), "0m");
        assert_eq!(pct_text(0.9986), "99.9%");
        assert_eq!(pct_text(0.72), "72%");
        assert_eq!(hour_label(21), "9pm");
        assert_eq!(hour_label(0), "12am");
    }

    #[test]
    fn a_new_hunt_draws_too() {
        let (_, mut h, who) = sample();
        h.count = 5;
        h.mult = 0.005;
        h.found = 0.005;
        h.per_day.truncate(1);
        h.best_day = None;
        h.pace = None;
        for l in Layout::ALL {
            let c = render(&Data::Hunt(h.clone()), l, &who);
            assert_eq!((c.w, c.h), l.size());
        }
        // nothing to show at all
        let o = Overview::default();
        for l in Layout::ALL {
            render(&Data::Overview(o.clone()), l, &Who::default());
        }
    }

    /// Writes the sample images (SHARE_OUT=dir) to compare with the design.
    #[test]
    fn write_samples() {
        let Some(dir) = std::env::var_os("SHARE_OUT") else { return };
        let (o, h, who) = sample();
        for (i, (data, l)) in [
            (Data::Overview(o.clone()), Layout::Poster),
            (Data::Hunt(h.clone()), Layout::Poster),
            (Data::Overview(o.clone()), Layout::Dashboard),
            (Data::Hunt(h.clone()), Layout::Dashboard),
            (Data::Overview(o.clone()), Layout::Wrapped),
            (Data::Hunt(h.clone()), Layout::Wrapped),
        ]
        .into_iter()
        .enumerate()
        {
            let c = render(&data, l, &who);
            let img = image::RgbaImage::from_raw(c.w, c.h, c.px).unwrap();
            img.save(std::path::Path::new(&dir).join(format!("out{i}.png"))).unwrap();
        }
    }
}
