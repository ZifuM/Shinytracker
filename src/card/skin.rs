//! The share images in the overlay themes' looks (Glass, Retro DS, Neon,
//! Pokétch, Field journal, Holo card, Broadcast and Terminal). One set of
//! layouts (hunt card, trainer card, rare grass poster), each theme a
//! `Skin` that draws the parts: background, panels, the shiny's frame,
//! chips, titles, numbers, gauges, tiles and the footer.

use super::*;
use crate::poster::{PosterArea, PosterData, PosterMode};

#[path = "skin/broadcast.rs"]
mod broadcast;
#[path = "skin/glass.rs"]
mod glass;
#[path = "skin/holo.rs"]
mod holo;
#[path = "skin/journal.rs"]
mod journal;
#[path = "skin/neon.rs"]
mod neon;
#[path = "skin/poketch.rs"]
mod poketch;
#[path = "skin/retro.rs"]
mod retro;
#[path = "skin/terminal.rs"]
mod terminal;

pub(super) const WHITE: Col = Col(255, 255, 255, 255);
pub(super) const BLACK: Col = Col(0, 0, 0, 255);
pub(super) const GOOD: Col = Col(76, 205, 110, 255);
pub(super) const OK: Col = Col(255, 200, 60, 255);
pub(super) const BAD: Col = Col(255, 92, 82, 255);
pub(super) const BRAND: &str = "Poke Together Companion";

pub(super) fn of(style: CardStyle) -> Option<&'static dyn Skin> {
    Some(match style {
        CardStyle::Glass => &glass::Glass,
        CardStyle::Retro => &retro::Retro,
        CardStyle::Neon => &neon::Neon,
        CardStyle::Poketch => &poketch::Poketch,
        CardStyle::Journal => &journal::Journal,
        CardStyle::Holo => &holo::Holo,
        CardStyle::Broadcast => &broadcast::Broadcast,
        CardStyle::Terminal => &terminal::Terminal,
        CardStyle::Anime | CardStyle::Classic => return None,
    })
}

/// What a panel is for (some themes draw them differently).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Role {
    /// The shiny's frame.
    Hero,
    /// A stat tile.
    Tile,
    /// A small box in a row (showcase, achievements, poster pins).
    Slot,
    /// A big box holding a group (identity, poster cards, world clock).
    Card,
}

#[derive(Clone, Copy)]
pub(super) struct Faces {
    pub title: Face,
    pub num: Face,
    pub label: Face,
    pub body: Face,
}

/// Draws nothing: for the background, which has no text.
struct NoText;
impl TextDraw for NoText {
    fn width(&self, s: &str, size: f32, _face: Face) -> f32 {
        s.chars().count() as f32 * size * 0.55
    }
    fn draw(&self, _c: &mut Canvas, _s: &str, _x: f32, _y: f32, _size: f32, _col: Col, _face: Face) {}
}

/// A theme for the share images. Positions are in design units, `y` the
/// top of the text.
pub(super) trait Skin: Sync {
    /// The canvas colour under everything.
    fn bg(&self, o: &CardOptions) -> Col;
    /// The background: the parts that never move (no text here).
    fn backdrop(&self, pen: &mut Pen, o: &CardOptions);
    /// Text parts of the frame (title bars, engravings), drawn first.
    fn chrome(&self, _pen: &mut Pen, _o: &CardOptions, _what: &str) {}
    /// Drawn last, over everything (scan lines, foil, a moving cursor).
    fn overlay(&self, _pen: &mut Pen, _o: &CardOptions) {}
    /// Space kept round the edge, and extra at the top.
    fn inset(&self) -> f32 {
        48.0
    }
    fn top(&self) -> f32 {
        0.0
    }
    fn text(&self) -> Col;
    fn soft(&self) -> Col;
    fn accent(&self, o: &CardOptions) -> Col;
    fn faces(&self) -> Faces;
    /// Labels in capitals (or lower case, or as written).
    fn case(&self, s: &str) -> String {
        s.to_uppercase()
    }
    /// A status colour made readable on this theme.
    fn tone(&self, c: Col) -> Col {
        c
    }
    /// Room the title's decoration takes beyond the text, on the right.
    fn title_pad(&self) -> f32 {
        0.0
    }
    /// The shiny's frame is tall (put it beside the text on square images).
    fn portrait(&self) -> bool {
        false
    }
    fn panel(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role);
    /// The shiny (and the normal colours) in its frame.
    #[allow(clippy::too_many_arguments)]
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        hero_sprites(pen, d, o, x, y, w, h, self.accent(o));
    }
    /// A small label; `fill` = a coloured chip (else plain). Returns its width.
    #[allow(clippy::too_many_arguments)]
    fn chip(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let f = self.faces().label;
        let t = self.case(text);
        let w = pen.width(&t, 16.0, f) + 30.0;
        let x0 = aligned(x, w, align);
        let fill = fill.unwrap_or(self.accent(o));
        pen.rrect(x0, y, w, 34.0, 17.0, fill);
        pen.text(&t, x0 + 15.0, y + 8.0, 16.0, ink_on(fill, BLACK), f, Align::Left, false);
        w
    }
    /// The big title. Returns the size used.
    #[allow(clippy::too_many_arguments)]
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        pen.text(text, x, y, size, self.text(), self.faces().title, align, false);
    }
    #[allow(clippy::too_many_arguments)]
    fn number(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        pen.text(text, x, y, size, self.accent(o), self.faces().num, align, false);
    }
    /// Width of `number` at `size`.
    fn number_w(&self, pen: &Pen, text: &str, size: f32) -> f32 {
        pen.width(text, size, self.faces().num)
    }
    /// A small heading.
    #[allow(clippy::too_many_arguments)]
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        let t = self.case(text);
        pen.text(&t, x, y, size, col, self.faces().label, align, false)
    }
    fn gauge_h(&self) -> f32 {
        24.0
    }
    /// The luck bar: `frac` of the way to 3× the odds, `col` the status colour.
    #[allow(clippy::too_many_arguments)]
    fn gauge(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col);
    /// A stat tile.
    #[allow(clippy::too_many_arguments)]
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        let f = self.faces();
        self.label(pen, o, label, x + 18.0, y + 14.0, 15.0, self.soft(), Align::Left);
        let vs = pen.fit(value, 30.0, w - 36.0, f.num);
        pen.text(value, x + 18.0, y + h - vs - 14.0, vs, self.text(), f.num, Align::Left, false);
    }
    /// The footer: "TRAINER  Name  LV 22" on the left, the app on the right.
    #[allow(clippy::too_many_arguments)]
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        pen.line(x0, y, x1, y, 1.5, self.soft().alpha(0.4));
        let f = self.faces();
        let mut x = x0;
        if !left.is_empty() {
            x += self.label(pen, o, left, x, cy - 8.0, 14.0, self.soft(), Align::Left) + 12.0;
        }
        if !name.is_empty() {
            x += pen.text(name, x, cy - 11.0, 20.0, self.text(), f.body, Align::Left, false) + 12.0;
        }
        if let Some(lv) = level {
            self.chip(pen, o, &format!("LV {lv}"), x, cy - 13.0, None, Align::Left);
        }
        let bw = self.label(pen, o, BRAND, x1, cy - 8.0, 14.0, self.soft(), Align::Right);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 38.0, cy - 14.0, 28.0, 28.0);
        }
    }
    /// The trainer level badge.
    #[allow(clippy::too_many_arguments)]
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, metal: Col, light: Col) {
        let hex = |r: f32| -> Vec<(f32, f32)> {
            (0..6)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_6 + i as f32 * std::f32::consts::FRAC_PI_3;
                    (cx + a.cos() * r, cy + a.sin() * r)
                })
                .collect()
        };
        pen.poly(&hex(r), metal);
        pen.poly(&hex(r * 0.82), light);
        pen.poly(&hex(r * 0.74), metal.mix(BLACK, 0.2));
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.5 } else { r * 0.66 };
        self.number(pen, o, &n, cx, cy - size * 0.6, size, Align::Center);
    }
    /// A map pin bubble (poster).
    #[allow(clippy::too_many_arguments)]
    fn pin(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, col: Col) {
        pen.rrect(x - 3.0, y - 3.0, w + 6.0, h + 6.0, h / 2.0 + 3.0, self.text());
        pen.rrect(x, y, w, h, h / 2.0, col.mix(WHITE, 0.55));
        let _ = o;
    }
    /// Recolour a map pixel (poster).
    fn map_px(&self, c: Col) -> Col {
        c
    }
    /// A city name on the map (poster).
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        halo(pen, name, cx, y, size, WHITE, self.text(), Face::Bold, Align::Center, 3.0);
    }
    /// Colours for areas (poster cards and pins).
    fn area_col(&self, o: &CardOptions, i: usize) -> Col {
        let a = self.accent(o);
        [a, a.mix(WHITE, 0.3), a.mix(BLACK, 0.2), a.mix(GOOD, 0.4)][i % 4]
    }
}

// ───────────────────────────── helpers ─────────────────────────────

pub(super) fn aligned(x: f32, w: f32, align: Align) -> f32 {
    match align {
        Align::Left => x,
        Align::Center => x - w / 2.0,
        Align::Right => x - w,
    }
}

/// Shorten with "…" to fit `maxw`.
pub(super) fn clip(pen: &Pen, s: &str, size: f32, face: Face, maxw: f32) -> String {
    if pen.width(s, size, face) <= maxw {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let t: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
        if pen.width(&t, size, face) <= maxw {
            return t;
        }
    }
    "…".into()
}

/// Text with a ring of `halo` round it.
#[allow(clippy::too_many_arguments)]
pub(super) fn halo(pen: &mut Pen, s: &str, x: f32, y: f32, size: f32, halo_c: Col, col: Col, face: Face, align: Align, t: f32) -> f32 {
    for i in 0..12 {
        let a = i as f32 / 12.0 * std::f32::consts::TAU;
        pen.text(s, x + a.cos() * t, y + a.sin() * t, size, halo_c, face, align, false);
    }
    pen.text(s, x, y, size, col, face, align, false)
}

/// Text with a soft glow round it.
#[allow(clippy::too_many_arguments)]
pub(super) fn glow_text(pen: &mut Pen, s: &str, x: f32, y: f32, size: f32, col: Col, glow: Col, face: Face, align: Align) -> f32 {
    let spread = (size / 14.0).clamp(2.0, 7.0);
    for (k, a) in [(1.0, 0.08), (0.65, 0.14), (0.35, 0.22)] {
        for i in 0..10 {
            let ang = i as f32 / 10.0 * std::f32::consts::TAU;
            pen.text(s, x + ang.cos() * spread * k, y + ang.sin() * spread * k, size, glow.alpha(a), face, align, false);
        }
    }
    pen.text(s, x, y, size, col, face, align, false)
}

/// Letter-spaced text. Returns its width.
#[allow(clippy::too_many_arguments)]
pub(super) fn tracked(pen: &mut Pen, s: &str, x: f32, y: f32, size: f32, col: Col, face: Face, align: Align, gap: f32) -> f32 {
    let widths: Vec<f32> = s.chars().map(|c| pen.width(&c.to_string(), size, face)).collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    let mut cx = aligned(x, total, align);
    for (c, w) in s.chars().zip(widths) {
        pen.text(&c.to_string(), cx, y, size, col, face, Align::Left, false);
        cx += w + gap;
    }
    total
}

pub(super) fn tracked_w(pen: &Pen, s: &str, size: f32, face: Face, gap: f32) -> f32 {
    s.chars().map(|c| pen.width(&c.to_string(), size, face)).sum::<f32>() + gap * s.chars().count().saturating_sub(1) as f32
}

/// A pixel sprite (trimmed) fitted into `w`×`h` with its middle at (cx, cy).
/// Whole-number scaling keeps it crisp. Returns where it went (x, y, w, h).
#[allow(clippy::too_many_arguments)]
pub(super) fn sprite(pen: &mut Pen, img: &Img, tb: Option<(u32, u32, u32, u32)>, cx: f32, cy: f32, w: f32, h: f32, centred: bool) -> (f32, f32, f32, f32) {
    let sp = crop(img, tb.unwrap_or_else(|| Canvas::trim_box(img)));
    if sp.w == 0 || sp.h == 0 {
        return (cx, cy, 0.0, 0.0);
    }
    let k = (w / sp.w as f32).min(h / sp.h as f32);
    let k = if k * pen.s >= 1.0 { (k * pen.s).floor() / pen.s } else { k };
    let (sw, sh) = (sp.w as f32 * k, sp.h as f32 * k);
    let (x, y) = (cx - sw / 2.0, if centred { cy - sh / 2.0 } else { cy - sh });
    let s = pen.s;
    let (px, py) = ((x * s).round(), (y * s).round());
    if k * s >= 1.0 {
        pen.c.blit_nearest(&sp, px, py, k * s);
    } else {
        pen.c.blit_smooth(&sp, px, py, sw * s, sh * s);
    }
    (x, y, sw, sh)
}

/// A sprite filled with one colour (for shadows and silhouettes).
#[allow(clippy::too_many_arguments)]
pub(super) fn silhouette(pen: &mut Pen, img: &Img, tb: Option<(u32, u32, u32, u32)>, cx: f32, cy: f32, w: f32, h: f32, col: Col) {
    let sp = crop(img, tb.unwrap_or_else(|| Canvas::trim_box(img)));
    let mut px = sp.px.clone();
    for p in px.chunks_exact_mut(4) {
        let a = (p[3] as f32 * col.3 as f32 / 255.0) as u8;
        p.copy_from_slice(&[col.0, col.1, col.2, a]);
    }
    let img = Img { w: sp.w, h: sp.h, px };
    sprite(pen, &img, None, cx, cy, w, h, true);
}

/// The shiny centred in a frame, the normal colours small in a corner.
#[allow(clippy::too_many_arguments)]
pub(super) fn hero_sprites(pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, spark: Col) {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    match &d.shiny {
        Some(img) => {
            let (sx, sy, sw, sh) = sprite(pen, img, d.sprite_box, cx, cy, w * 0.78, h * 0.72, true);
            if o.sparkles {
                sparkles(pen, sx, sy, sw, sh, spark);
            }
        }
        None => {
            pen.text("?", cx, cy - 60.0, 120.0, spark, Face::Bold, Align::Center, false);
        }
    }
}

/// Twinkles round a sprite (they pulse in the animated version).
pub(super) fn sparkles(pen: &mut Pen, sx: f32, sy: f32, sw: f32, sh: f32, col: Col) {
    let tw = |base: f32, off: f32, ph: Option<f32>| -> f32 { ph.map_or(base, |p| base * (0.6 + 0.4 * ((p + off) * std::f32::consts::TAU).sin().abs())) };
    let ph = pen.phase;
    pen.sparkle(sx + sw + 8.0, sy + 16.0, tw(22.0, 0.0, ph), col);
    pen.sparkle(sx - 14.0, sy + sh * 0.4, tw(13.0, 0.33, ph), col.alpha(0.9));
    pen.sparkle(sx + sw + 4.0, sy + sh * 0.75, tw(10.0, 0.66, ph), col.alpha(0.8));
}

/// The normal-colour sprite in a small box at (x, y).
pub(super) fn normal_inset(pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, s: f32) -> bool {
    if !o.show_normal {
        return false;
    }
    let Some(img) = &d.normal else { return false };
    sprite(pen, img, None, x + s / 2.0, y + s / 2.0, s * 0.82, s * 0.82, true);
    true
}

/// Corners of a box turned by `deg` round its middle.
pub(super) fn turned(x: f32, y: f32, w: f32, h: f32, deg: f32) -> Vec<(f32, f32)> {
    let (sn, cs) = deg.to_radians().sin_cos();
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
        .iter()
        .map(|&(px, py)| {
            let (dx, dy) = (px - cx, py - cy);
            (cx + dx * cs - dy * sn, cy + dx * sn + dy * cs)
        })
        .collect()
}

/// Horizontal gradient in a rounded box.
#[allow(clippy::too_many_arguments)]
pub(super) fn hgrad(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, r: f32, a: Col, b: Col) {
    pen.rrect_with(x, y, w, h, r, |px, _| a.mix(b, (px - x) / w.max(1.0)));
}

/// Everything the hunt card says, worked out once.
pub(super) struct Info {
    pub title: String,
    pub sub: String,
    pub count: String,
    pub count_label: String,
    /// (multiple of the odds, "UNDER ODDS", status colour)
    pub luck: Option<(f64, &'static str, Col)>,
    pub tiles: Vec<(&'static str, String)>,
    pub caught: bool,
    pub note: Option<String>,
    pub phases: Option<String>,
}

impl Info {
    pub fn new(d: &CardData, o: &CardOptions) -> Self {
        let title = if o.title.trim().is_empty() { auto_title(d) } else { o.title.trim().to_uppercase() };
        let mut sub = Vec::new();
        if d.nickname.as_ref().is_some_and(|n| !n.trim().is_empty()) && o.title.trim().is_empty() {
            sub.push(d.name.clone());
        }
        if let Some(n) = d.dex {
            sub.push(format!("#{n:04}"));
        }
        if o.show_odds {
            if !d.method.is_empty() {
                sub.push(d.method.clone());
            }
            if let Some(odds) = d.odds_one_in {
                sub.push(format!("1 in {}", thousands(odds.round() as u64)));
            }
        }
        let mut count_label = if d.encounters == 1 { "ENCOUNTER".to_string() } else { "ENCOUNTERS".to_string() };
        if let Some(ts) = d.target_seen {
            count_label.push_str(&format!(" · {} {} SEEN", thousands(ts), d.name.to_uppercase()));
        }
        let luck = if o.show_meter {
            d.odds_one_in.map(|odds| {
                let n = match (d.odds_on_target, d.target_seen) {
                    (true, Some(ts)) => ts,
                    _ => d.encounters,
                };
                let mult = n as f64 / odds;
                let (word, _) = luck(mult);
                let col = if mult < 0.9 { GOOD } else if mult <= 1.1 { OK } else { BAD };
                (mult, word, col)
            })
        } else {
            None
        };
        let note = d.note.as_ref().map(|n| n.trim().to_string()).filter(|n| o.show_note && !n.is_empty()).map(|n| if n.chars().count() > 70 { format!("{}…", n.chars().take(67).collect::<String>()) } else { n });
        let phases = (!d.phases.is_empty()).then(|| {
            let mut names: Vec<&str> = d.phases.iter().map(|s| s.as_str()).take(3).collect();
            names.dedup();
            let more = d.phases.len().saturating_sub(3);
            format!("Phase{}: {}{}", if d.phases.len() == 1 { "" } else { "s" }, names.join(", "), if more > 0 { format!(" +{more}") } else { String::new() })
        });
        Info { title, sub: sub.join("  ·  "), count: thousands(d.encounters), count_label, luck, tiles: tiles(d, o), caught: d.caught, note, phases }
    }
}

// ───────────────────────────── the base ─────────────────────────────

pub(super) fn base(sk: &dyn Skin, o: &CardOptions) -> Canvas {
    let (w, h, s) = o.output_size();
    let (dw, dh) = o.layout.size();
    let mut pen = Pen { c: Canvas::new(w, h, sk.bg(o)), t: &NoText, p: o.theme.palette(), phase: None, s, w: dw as f32, h: dh as f32 };
    sk.backdrop(&mut pen, o);
    pen.c
}

// ───────────────────────────── hunt card ─────────────────────────────

pub(super) fn hunt(sk: &dyn Skin, pen: &mut Pen, d: &CardData, o: &CardOptions) {
    let info = Info::new(d, o);
    sk.chrome(pen, o, &d.name.to_lowercase().replace(' ', "-"));
    match o.layout {
        CardLayout::Wide => hunt_wide(sk, pen, d, o, &info),
        CardLayout::Square => hunt_square(sk, pen, d, o, &info),
    }
    sk.overlay(pen, o);
}

const FOOT_H: f32 = 56.0;

/// Status chips: "CAUGHT!" / "HUNTING". Returns the bottom.
fn chips(sk: &dyn Skin, pen: &mut Pen, o: &CardOptions, info: &Info, x: f32, y: f32, align: Align, cw: f32) -> f32 {
    let (status, fill) = if info.caught { ("Caught!", GOOD) } else { ("Hunting", sk.accent(o)) };
    if align == Align::Center {
        let f = sk.faces().label;
        let w1 = pen.width(&sk.case(status), 16.0, f) + 30.0;
        let w2 = info.phases.as_ref().map_or(0.0, |p| pen.width(&sk.case(p), 16.0, f) + 46.0);
        let x0 = x - (w1 + w2) / 2.0;
        sk.chip(pen, o, status, x0, y, Some(fill), Align::Left);
        if let Some(p) = &info.phases {
            sk.chip(pen, o, p, x0 + w1 + 16.0, y, None, Align::Left);
        }
    } else {
        let w1 = sk.chip(pen, o, status, x, y, Some(fill), Align::Left);
        if let Some(p) = &info.phases {
            let room = cw - w1 - 16.0;
            let t = clip(pen, p, 16.0, sk.faces().label, room - 30.0);
            if room > 120.0 {
                sk.chip(pen, o, &t, x + w1 + 16.0, y, None, Align::Left);
            }
        }
    }
    y + 34.0
}

/// Title, sub line, the count. Returns the bottom.
#[allow(clippy::too_many_arguments)]
fn headline(sk: &dyn Skin, pen: &mut Pen, o: &CardOptions, info: &Info, gender: Gender, x: f32, y: f32, w: f32, align: Align, count_max: f32, room: f32) -> f32 {
    let f = sk.faces();
    let sign = if gender != Gender::None { 1.0 } else { 0.0 };
    let mut ts = pen.fit(&info.title, 62.0, w, f.title).max(22.0);
    while ts > 22.0 && pen.width(&info.title, ts, f.title) + sk.title_pad() + sign * ts * 0.9 > w {
        ts -= 1.0;
    }
    sk.title(pen, o, &info.title, x, y, ts, align);
    if gender != Gender::None {
        let tw = pen.width(&info.title, ts, f.title);
        let right = match align {
            Align::Left => x + tw,
            Align::Center => x + tw / 2.0,
            Align::Right => x,
        };
        let pad = sk.title_pad() * if align == Align::Center { 0.5 } else { 1.0 };
        gender_sign(pen, gender, right + pad + ts * 0.45, y + ts * 0.62, ts * 0.2);
    }
    let mut y = y + ts * 1.2;
    if !info.sub.is_empty() {
        let ss = pen.fit(&info.sub, 22.0, w, f.body);
        pen.text(&info.sub, x, y, ss, sk.soft(), f.body, align, false);
        y += ss + 18.0;
    }
    // `room` is how far down the count may go
    let mut cs = count_max.min(((room - y) / 1.08 - 30.0).max(40.0));
    while cs > 30.0 && sk.number_w(pen, &info.count, cs) > w {
        cs -= 2.0;
    }
    sk.number(pen, o, &info.count, x, y, cs, align);
    y += cs * 1.08;
    let ls = pen.fit(&info.count_label, 16.0, w, f.label);
    sk.label(pen, o, &info.count_label, x, y, ls, sk.soft(), align);
    y + ls + 14.0
}

/// The luck gauge with its labels. Returns the bottom.
fn luck_gauge(sk: &dyn Skin, pen: &mut Pen, o: &CardOptions, info: &Info, x: f32, y: f32, w: f32) -> f32 {
    let Some((mult, word, col)) = info.luck else { return y };
    let col = sk.tone(col);
    sk.label(pen, o, word, x, y, 16.0, col, Align::Left);
    sk.label(pen, o, &format!("{} the odds", fmt_mult(mult)), x + w, y, 16.0, sk.text(), Align::Right);
    let gy = y + 28.0;
    sk.gauge(pen, o, x, gy, w, (mult / 3.0).clamp(0.0, 1.0) as f32, col);
    let ty = gy + sk.gauge_h() + 6.0;
    for k in 1..=3 {
        let tx = x + w * k as f32 / 3.0;
        let a = if k == 3 { Align::Right } else { Align::Center };
        sk.label(pen, o, &format!("{k}×"), if k == 3 { x + w } else { tx }, ty, 12.0, sk.soft(), a);
    }
    ty + 18.0
}

pub(super) fn gauge_block_h(sk: &dyn Skin) -> f32 {
    28.0 + sk.gauge_h() + 6.0 + 18.0
}

/// Stat tiles in one row. Returns the bottom.
#[allow(clippy::too_many_arguments)]
fn tile_row(sk: &dyn Skin, pen: &mut Pen, o: &CardOptions, list: &[(&str, String)], x: f32, y: f32, w: f32, n_max: usize, th: f32) -> f32 {
    if list.is_empty() {
        return y;
    }
    let n = list.len().min(n_max);
    let gap = 16.0;
    let tw = (w - gap * (n as f32 - 1.0)) / n as f32;
    for (i, (label, value)) in list.iter().take(n).enumerate() {
        sk.tile(pen, o, x + i as f32 * (tw + gap), y, tw, th, label, value);
    }
    y + th
}

fn note_line(sk: &dyn Skin, pen: &mut Pen, info: &Info, x: f32, y: f32, w: f32, align: Align) -> f32 {
    let Some(n) = &info.note else { return y };
    let f = sk.faces().body;
    let q = format!("“{n}”");
    let s = pen.fit(&q, 20.0, w, f);
    pen.text(&q, x, y, s, sk.soft(), f, align, false);
    y + s + 14.0
}

fn hunt_wide(sk: &dyn Skin, pen: &mut Pen, d: &CardData, o: &CardOptions, info: &Info) {
    let (w, h) = (pen.w, pen.h);
    let m = sk.inset();
    let top = m + sk.top();
    let foot_y = h - m - FOOT_H;
    let hw = 440.0;
    sk.hero(pen, d, o, m, top, hw, foot_y - top - 18.0);
    let x0 = m + hw + 40.0;
    let cw = w - m - x0;
    // from the bottom: tiles, then the gauge
    let th = 84.0;
    let tiles_y = foot_y - 18.0 - th;
    let gh = if info.luck.is_some() { gauge_block_h(sk) } else { 0.0 };
    let gauge_y = tiles_y - 20.0 - gh;
    let mut y = chips(sk, pen, o, info, x0, top, Align::Left, cw) + 22.0;
    y = headline(sk, pen, o, info, d.gender, x0, y, cw, Align::Left, 104.0, gauge_y - 60.0);
    if gauge_y - y > 40.0 {
        note_line(sk, pen, info, x0, y, cw, Align::Left);
    }
    if info.luck.is_some() {
        luck_gauge(sk, pen, o, info, x0, gauge_y, cw);
    }
    tile_row(sk, pen, o, &info.tiles, x0, tiles_y, cw, 3, th);
    sk.footer(pen, o, m, w - m, foot_y, FOOT_H, "Trainer", o.trainer.trim(), d.trainer_level, d.icon.as_ref());
}

fn hunt_square(sk: &dyn Skin, pen: &mut Pen, d: &CardData, o: &CardOptions, info: &Info) {
    let (w, h) = (pen.w, pen.h);
    let m = sk.inset();
    let top = m + sk.top();
    let foot_y = h - m - FOOT_H;
    let cw = w - 2.0 * m;
    let th = 88.0;
    let tiles_y = foot_y - 20.0 - th;
    let gh = if info.luck.is_some() { gauge_block_h(sk) } else { 0.0 };
    let gauge_y = tiles_y - 22.0 - gh;
    if sk.portrait() {
        let hw = 420.0;
        let hh = gauge_y - 26.0 - top;
        sk.hero(pen, d, o, m, top, hw, hh);
        let x0 = m + hw + 36.0;
        let rw = w - m - x0;
        let mut y = chips(sk, pen, o, info, x0, top + 10.0, Align::Left, rw) + 26.0;
        y = headline(sk, pen, o, info, d.gender, x0, y, rw, Align::Left, 110.0, top + hh - 30.0);
        note_line(sk, pen, info, x0, y, rw, Align::Left);
    } else {
        let hh = 400.0;
        sk.hero(pen, d, o, m, top, cw, hh);
        let mut y = chips(sk, pen, o, info, w / 2.0, top + hh + 26.0, Align::Center, cw) + 22.0;
        y = headline(sk, pen, o, info, d.gender, w / 2.0, y, cw, Align::Center, 96.0, gauge_y - 20.0);
        if gauge_y - y > 36.0 {
            note_line(sk, pen, info, w / 2.0, y, cw, Align::Center);
        }
    }
    if info.luck.is_some() {
        luck_gauge(sk, pen, o, info, m, gauge_y, cw);
    }
    tile_row(sk, pen, o, &info.tiles, m, tiles_y, cw, 4, th);
    sk.footer(pen, o, m, w - m, foot_y, FOOT_H, "Trainer", o.trainer.trim(), d.trainer_level, d.icon.as_ref());
}

// ───────────────────────────── trainer card ─────────────────────────────

pub(super) fn profile(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions) {
    let name = if d.name.trim().is_empty() { "trainer".to_string() } else { d.name.trim().to_lowercase() };
    sk.chrome(pen, o, &format!("trainer/{}", name.replace(' ', "-")));
    match o.layout {
        CardLayout::Wide => profile_wide(sk, pen, d, o),
        CardLayout::Square => profile_square(sk, pen, d, o),
    }
    sk.overlay(pen, o);
}

/// Badge, name, title and XP inside a panel.
#[allow(clippy::too_many_arguments)]
fn identity(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, vertical: bool) {
    sk.panel(pen, o, x, y, w, h, Role::Card);
    let f = sk.faces();
    let name = if d.name.trim().is_empty() { "TRAINER".to_string() } else { d.name.trim().to_uppercase() };
    let (metal, light) = (rgb(d.metal.1), rgb(d.metal.2));
    let (cx, cy, br, tx, ty, tw, align) = if vertical {
        (x + w / 2.0, y + 140.0, 86.0, x + w / 2.0, y + 256.0, w - 50.0, Align::Center)
    } else {
        (x + 140.0, y + h / 2.0, 92.0, x + 270.0, y + 46.0, w - 300.0, Align::Left)
    };
    sk.badge(pen, o, cx, cy, br, d.level, metal, light);
    let ns = pen.fit(&name, 46.0, tw, f.title);
    sk.title(pen, o, &name, tx, ty, ns, align);
    let mut yy = ty + ns * 1.2;
    let t = sk.case(&d.title);
    let tsz = pen.fit(&t, 20.0, tw, f.label);
    pen.text(&t, tx, yy, tsz, sk.accent(o), f.label, align, false);
    yy += tsz + 22.0;
    let gx = if vertical { x + 30.0 } else { tx };
    let gw = if vertical { w - 60.0 } else { tw };
    let (l, r) = if d.max_level { (format!("LV {} · MAX", d.level), format!("{} XP", thousands(d.total_xp))) } else { (format!("LV {}", d.level), format!("{} / {} XP", thousands(d.xp_into), thousands(d.xp_need))) };
    sk.label(pen, o, &l, gx, yy, 15.0, sk.text(), Align::Left);
    sk.label(pen, o, &r, gx + gw, yy, 15.0, sk.soft(), Align::Right);
    let frac = if d.max_level { 1.0 } else { d.xp_into as f32 / d.xp_need.max(1) as f32 };
    sk.gauge(pen, o, gx, yy + 26.0, gw, frac, sk.accent(o));
    if vertical {
        let tt = format!("{} XP in total", thousands(d.total_xp));
        sk.label(pen, o, &tt, x + w / 2.0, yy + 26.0 + sk.gauge_h() + 16.0, 13.0, sk.soft(), Align::Center);
    }
}

#[allow(clippy::too_many_arguments)]
fn stat_grid(sk: &dyn Skin, pen: &mut Pen, o: &CardOptions, stats: &[(String, String)], x: f32, y: f32, w: f32, per_row: usize, th: f32) -> f32 {
    let gap = 14.0;
    let tw = (w - gap * (per_row as f32 - 1.0)) / per_row as f32;
    let mut yy = y;
    for row in stats.chunks(per_row) {
        for (i, (l, v)) in row.iter().enumerate() {
            sk.tile(pen, o, x + i as f32 * (tw + gap), yy, tw, th, l, v);
        }
        yy += th + gap;
    }
    yy - gap
}

#[allow(clippy::too_many_arguments)]
fn showcase(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
    sk.label(pen, o, "Shiny showcase", x, y, 15.0, sk.soft(), Align::Left);
    let y = y + 26.0;
    let f = sk.faces();
    if d.shinies.is_empty() {
        sk.panel(pen, o, x, y, w, h, Role::Slot);
        pen.text("No shinies yet - the hunt continues!", x + w / 2.0, y + h / 2.0 - 11.0, 21.0, sk.soft(), f.body, Align::Center, false);
        return;
    }
    let n = d.shinies.len().clamp(4, 6);
    let gap = 12.0;
    let sw = ((w - gap * (n as f32 - 1.0)) / n as f32).min(h * 0.95);
    for (i, (img, name, enc)) in d.shinies.iter().take(n).enumerate() {
        let sx = x + i as f32 * (sw + gap);
        sk.panel(pen, o, sx, y, sw, h, Role::Slot);
        if let Some(img) = img {
            sprite(pen, img, None, sx + sw / 2.0, y + (h - 40.0) / 2.0 + 4.0, sw - 18.0, h - 58.0, true);
        }
        let ns = pen.fit(name, 15.0, sw - 12.0, f.body);
        pen.text(name, sx + sw / 2.0, y + h - 38.0, ns, sk.text(), f.body, Align::Center, false);
        let e = thousands(*enc);
        sk.label(pen, o, &e, sx + sw / 2.0, y + h - 19.0, 12.0, sk.accent(o), Align::Center);
    }
}

#[allow(clippy::too_many_arguments)]
fn achievements(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
    let head = format!("Recent achievements · {} / {}", d.ach_count.0, d.ach_count.1);
    sk.label(pen, o, &head, x, y, 15.0, sk.soft(), Align::Left);
    let y = y + 26.0;
    let f = sk.faces();
    if d.achievements.is_empty() {
        sk.panel(pen, o, x, y, w, h, Role::Slot);
        pen.text("Achievements unlock as you hunt", x + w / 2.0, y + h / 2.0 - 10.0, 19.0, sk.soft(), f.body, Align::Center, false);
        return;
    }
    let n = d.achievements.len().clamp(4, 6);
    let gap = 12.0;
    let aw = (w - gap * (n as f32 - 1.0)) / n as f32;
    for (i, (name, tier, img)) in d.achievements.iter().take(n).enumerate() {
        let ax = x + i as f32 * (aw + gap);
        sk.panel(pen, o, ax, y, aw, h, Role::Slot);
        let tc = rgb(*tier);
        match img {
            Some(img) => {
                sprite(pen, img, None, ax + aw / 2.0, y + (h - 30.0) / 2.0 + 2.0, aw - 16.0, h - 44.0, true);
                pen.circle(ax + 18.0, y + 18.0, 9.0, tc);
                pen.sparkle(ax + 18.0, y + 18.0, 5.0, WHITE);
            }
            None => {
                let r = ((h - 40.0) * 0.36).min(aw * 0.3);
                let (cx, cy) = (ax + aw / 2.0, y + (h - 30.0) / 2.0 + 2.0);
                pen.circle(cx + 2.0, cy + 3.0, r, BLACK.alpha(0.25));
                pen.circle(cx, cy, r, tc);
                pen.ring(cx, cy, r * 0.72, (r * 0.08).max(1.5), tc.mix(WHITE, 0.5));
                pen.sparkle(cx, cy, r * 0.5, WHITE);
            }
        }
        let ns = pen.fit(name, 13.0, aw - 10.0, f.body);
        pen.text(name, ax + aw / 2.0, y + h - 24.0, ns, sk.text(), f.body, Align::Center, false);
    }
}

fn profile_wide(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions) {
    let (w, h) = (pen.w, pen.h);
    let m = sk.inset();
    let top = m + sk.top();
    let foot_y = h - m - FOOT_H;
    let lw = 340.0;
    identity(sk, pen, d, o, m, top, lw, foot_y - top - 16.0, true);
    let x0 = m + lw + 30.0;
    let cw = w - m - x0;
    sk.chip(pen, o, "Trainer card", x0, top, None, Align::Left);
    let mut y = stat_grid(sk, pen, o, &d.stats, x0, top + 48.0, cw, 4, 70.0) + 18.0;
    let room = foot_y - 16.0 - y;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { (room - 52.0) * 0.55 } else { room - 26.0 };
        showcase(sk, pen, d, o, x0, y, cw, sh);
        y += 26.0 + sh + 14.0;
    }
    if d.show_achievements {
        let ah = foot_y - 16.0 - y - 26.0;
        achievements(sk, pen, d, o, x0, y, cw, ah);
    }
    sk.footer(pen, o, m, w - m, foot_y, FOOT_H, "", &d.date, None, d.icon.as_ref());
}

fn profile_square(sk: &dyn Skin, pen: &mut Pen, d: &ProfileData, o: &CardOptions) {
    let (w, h) = (pen.w, pen.h);
    let m = sk.inset();
    let top = m + sk.top();
    let foot_y = h - m - FOOT_H;
    let cw = w - 2.0 * m;
    identity(sk, pen, d, o, m, top, cw, 270.0, false);
    let mut y = stat_grid(sk, pen, o, &d.stats, m, top + 270.0 + 22.0, cw, 4, 80.0) + 22.0;
    let room = foot_y - 18.0 - y;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { (room - 52.0) * 0.55 } else { room - 26.0 };
        showcase(sk, pen, d, o, m, y, cw, sh);
        y += 26.0 + sh + 16.0;
    }
    if d.show_achievements {
        let ah = foot_y - 18.0 - y - 26.0;
        achievements(sk, pen, d, o, m, y, cw, ah);
    }
    sk.footer(pen, o, m, w - m, foot_y, FOOT_H, "", &d.date, None, d.icon.as_ref());
}

// ───────────────────────────── rare grass poster ─────────────────────────────

/// Coverage of pixel centre (fx, fy) inside a rounded box (pixels).
fn round_cov(fx: f32, fy: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let cx = fx.clamp(x0 + r, x1 - r);
    let cy = fy.clamp(y0 + r, y1 - r);
    let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
    (r - d + 0.5).clamp(0.0, 1.0)
}

pub(super) fn poster(sk: &dyn Skin, data: &PosterData, mode: PosterMode, theme: CardTheme, t: &dyn TextDraw) -> Canvas {
    let o = CardOptions { style: CardStyle::Glass, theme, layout: CardLayout::Wide, ..CardOptions::default() };
    let map_only = mode == PosterMode::MapOnly;
    let lists_only = mode == PosterMode::Lists;
    const W: f32 = 1920.0;
    let m = sk.inset().max(56.0);
    let (areas, tally, top3) = crate::poster::prepare(data);
    let name_of = |key: &str| -> String { areas.iter().flat_map(|a| a.mons.iter()).find(|mm| mm.key == key).map(|mm| mm.name.clone()).unwrap_or_default() };
    let sprite_of = |key: &str| -> Option<Img> { areas.iter().flat_map(|a| a.mons.iter()).find(|mm| mm.key == key).and_then(|mm| mm.sprite.clone()) };

    // ── measure
    let top0 = m + sk.top();
    let hero_h = if lists_only { 250.0 } else { 1130.0 };
    let band_h = if top3.is_empty() || map_only { 0.0 } else { 400.0 };
    let rows_total: usize = areas.iter().map(|a| a.mons.len().max(1)).sum();
    let (row, sbox, fname): (f32, f32, f32) = if rows_total <= 10 { (66.0, 54.0, 21.0) } else if rows_total <= 28 { (58.0, 46.0, 19.0) } else { (50.0, 40.0, 17.0) };
    const COLS: usize = 4;
    const GAP: f32 = 34.0;
    const HEAD: f32 = 64.0;
    const PAD: f32 = 12.0;
    let cw = (W - 2.0 * m - (COLS as f32 - 1.0) * GAP) / COLS as f32;
    let mut col_h = [0.0f32; COLS];
    let mut placed: Vec<(usize, usize, f32)> = Vec::new();
    for (ai, a) in areas.iter().enumerate() {
        let hh = HEAD + a.mons.len().max(1) as f32 * row + PAD * 2.0;
        let ci = (0..COLS).min_by(|&i, &j| col_h[i].total_cmp(&col_h[j])).unwrap_or(0);
        placed.push((ai, ci, col_h[ci]));
        col_h[ci] += hh + GAP;
    }
    let band_y = top0 + hero_h + 30.0;
    let index_y = band_y + band_h + if band_h > 0.0 { 40.0 } else { 0.0 };
    const INDEX_HEAD: f32 = 130.0;
    let tallest = if areas.is_empty() { 90.0 } else { col_h.iter().cloned().fold(0.0, f32::max) };
    let foot_h = 64.0;
    let h = if map_only { top0 + hero_h + 30.0 + foot_h + m } else { index_y + INDEX_HEAD + tallest + foot_h + m };

    let max_px = 40_000_000.0;
    let r = if W * h * 4.0 > max_px { (max_px / (W * h)).sqrt().max(1.0) } else { 2.0 };
    let mut pen = Pen { c: Canvas::new((W * r).round() as u32, (h * r).round() as u32, sk.bg(&o)), t: &NoText, p: o.theme.palette(), phase: None, s: r, w: W, h };
    sk.backdrop(&mut pen, &o);
    pen.t = t;
    let pen = &mut pen;
    let zone = crate::worldclock::server_zone_name(data.next);
    sk.chrome(pen, &o, "rare-grass/today");
    let f = sk.faces();

    // ── top: title, date and reset times; the map on the right
    let lx = m;
    let lw = if lists_only { W - 2.0 * m } else { 600.0 };
    sk.label(pen, &o, "Pokémon Platinum Together", lx, top0 + 6.0, 18.0, sk.soft(), Align::Left);
    let big = if lists_only { 92.0 } else { 120.0 };
    let date = crate::poster::date_label(data.reset);
    if lists_only {
        sk.title(pen, &o, &sk.case("Rare grass"), lx, top0 + 44.0, big, Align::Left);
        sk.number(pen, &o, &date, W - m, top0 + 50.0, 52.0, Align::Right);
        let span = format!("{}, 12:00 am → {}, 12:00 am ({zone})", crate::poster::short_date(data.reset), crate::poster::short_date(data.next));
        pen.text(&span, W - m, top0 + 130.0, 20.0, sk.soft(), f.body, Align::Right, false);
    } else {
        let bs = pen.fit(&sk.case("Grass"), big, lw, f.title);
        sk.title(pen, &o, &sk.case("Rare"), lx, top0 + 44.0, bs, Align::Left);
        sk.title(pen, &o, &sk.case("Grass"), lx, top0 + 44.0 + bs * 1.05, bs, Align::Left);
        let mut y = top0 + 44.0 + bs * 2.2;
        let ds = pen.fit(&date, 44.0, lw, f.num);
        sk.number(pen, &o, &date, lx, y, ds, Align::Left);
        y += ds * 1.2 + 6.0;
        let span = format!("{}, 12:00 am → {}, 12:00 am", crate::poster::short_date(data.reset), crate::poster::short_date(data.next));
        let ss = pen.fit(&span, 19.0, lw, f.body);
        pen.text(&span, lx, y, ss, sk.text(), f.body, Align::Left, false);
        sk.label(pen, &o, &format!("Server time ({zone})"), lx, y + 28.0, 14.0, sk.soft(), Align::Left);
        y += 70.0;
        // reset round the world
        let rows = crate::worldclock::rows(data.next);
        let card_h = top0 + hero_h - y;
        sk.panel(pen, &o, lx, y, lw, card_h, Role::Card);
        sk.label(pen, &o, "Reset around the world", lx + 22.0, y + 18.0, 16.0, sk.accent(&o), Align::Left);
        let rh = ((card_h - 70.0) / rows.len().max(1) as f32).min(48.0);
        for (i, rw) in rows.iter().enumerate() {
            let cy = y + 56.0 + i as f32 * rh;
            if rw.server {
                pen.rrect(lx + 10.0, cy - 2.0, lw - 20.0, rh - 2.0, 8.0, sk.accent(&o).alpha(0.22));
            }
            let ls = pen.fit(rw.label, 17.0, lw * 0.55, f.body);
            pen.text(rw.label, lx + 22.0, cy + (rh - ls) / 2.0 - 3.0, ls, sk.text(), f.body, Align::Left, false);
            let t12 = format!("{} {}", rw.time12(), rw.weekday);
            pen.text(&t12, lx + lw - 22.0, cy + (rh - 18.0) / 2.0 - 3.0, 18.0, sk.text(), f.label, Align::Right, false);
        }
        // the map
        let (fx, fy, fh) = (lx + lw + 40.0, top0, hero_h);
        let fw = W - m - fx;
        sk.panel(pen, &o, fx - 8.0, fy - 8.0, fw + 16.0, fh + 16.0, Role::Hero);
        if let Some(mp) = &data.map {
            let (_, _, tw, th) = mp.fit;
            let scale = (fh / th).min(fw / tw);
            let (mw, mh) = (tw * scale, th * scale);
            let (mx, my) = (fx + (fw - mw) / 2.0, fy + (fh - mh) / 2.0);
            let rr = pen.s;
            let (pw, ph) = ((fw * rr).round() as u32, (fh * rr).round() as u32);
            let sea = sk.map_px(Col::hex(0x56b8e0));
            let mut tmp = Canvas::new(pw, ph, sea);
            let (ix, iy) = (mx - fx + (mp.x0 - mp.fit.0) * scale, my - fy + (mp.y0 - mp.fit.1) * scale);
            let (iw, ih) = (mp.img.w as f32 / mp.px_per_tile * scale, mp.img.h as f32 / mp.px_per_tile * scale);
            tmp.blit_smooth(&mp.img, ix * rr, iy * rr, iw * rr, ih * rr);
            let (ox, oy) = ((fx * rr).round() as i32, (fy * rr).round() as i32);
            let rad = 18.0 * rr;
            pen.c.fill_with(ox, oy, ox + pw as i32, oy + ph as i32, |x, y| {
                let (tx, ty) = ((x - ox) as u32, (y - oy) as u32);
                if tx >= pw || ty >= ph {
                    return None;
                }
                let cov = round_cov(x as f32 + 0.5, y as f32 + 0.5, ox as f32, oy as f32, (ox + pw as i32) as f32, (oy + ph as i32) as f32, rad);
                let i = ((ty * pw + tx) * 4) as usize;
                (cov > 0.0).then(|| (sk.map_px(Col(tmp.px[i], tmp.px[i + 1], tmp.px[i + 2], 255)), cov))
            });
            let to_px = |tx: f32, ty: f32| -> (f32, f32) { (mx + (tx - mp.fit.0) * scale, my + (ty - mp.fit.1) * scale) };
            let mut taken: Vec<(f32, f32, f32, f32)> = Vec::new();
            for l in &data.labels {
                let (cx, cy) = to_px(l.x, l.y);
                let name = l.name.trim_end_matches(" City").trim_end_matches(" Town");
                let fs = if l.city { 21.0 } else { 17.0 };
                sk.map_label(pen, &o, name, cx, cy - fs - 4.0, fs);
                let tw = pen.width(name, fs, Face::Bold);
                taken.push((cx - tw / 2.0 - 4.0, cy - fs - 8.0, tw + 8.0, fs + 8.0));
            }
            let (pins, sb) = crate::poster::place_pins(&areas, &to_px, &taken, fx + 8.0, fx + fw - 8.0, fy + 8.0, fy + fh - 8.0);
            for pin in &pins {
                if ((pin.x - pin.ox).powi(2) + (pin.y - pin.oy).powi(2)).sqrt() > 6.0 {
                    pen.line(pin.ox, pin.oy, pin.x, pin.y, 3.0, sk.text());
                }
                pen.circle(pin.ox, pin.oy, 6.0, sk.text());
                pen.circle(pin.ox, pin.oy, 3.0, sk.accent(&o));
            }
            for pin in &pins {
                let a: &PosterArea = &areas[pin.area];
                let mons: Vec<&crate::poster::PosterMon> = pin.mons.iter().take(4).filter_map(|&i| a.mons.get(i)).collect();
                let b = sb * 0.72;
                let step = b * 0.7;
                let n = mons.len().max(1) as f32;
                let (bw, bh) = (b + (n - 1.0) * step + 14.0, b + 8.0);
                sk.pin(pen, &o, pin.x - bw / 2.0, pin.y - bh / 2.0, bw, bh, sk.area_col(&o, pin.area));
                for (i, mon) in mons.iter().enumerate() {
                    let ix = pin.x + (i as f32 - (n - 1.0) / 2.0) * step;
                    match &mon.sprite {
                        Some(img) => {
                            sprite(pen, img, None, ix, pin.y, b, b, true);
                        }
                        None => pen.circle(ix, pin.y, b * 0.28, sk.area_col(&o, pin.area)),
                    }
                }
            }
        } else {
            sk.title(pen, &o, &sk.case("Map unavailable"), fx + fw / 2.0, fy + fh / 2.0 - 20.0, 40.0, Align::Center);
        }
    }

    if !map_only {
        // ── most widespread: the top 3
        if !top3.is_empty() {
            let by = band_y;
            sk.title(pen, &o, &sk.case("Most widespread"), m, by, 52.0, Align::Left);
            pen.text("Pokémon found in the most areas today. Ties share a place.", m, by + 66.0, 20.0, sk.soft(), f.body, Align::Left, false);
            let pw = (W - 2.0 * m - 2.0 * 30.0) / 3.0;
            let ph = band_h - 110.0;
            for rank in 0..3 {
                let (x, y) = (m + rank as f32 * (pw + 30.0), by + 110.0);
                sk.panel(pen, &o, x, y, pw, ph, Role::Card);
                sk.number(pen, &o, &(rank + 1).to_string(), x + 60.0, y + ph / 2.0 - 60.0, 110.0, Align::Center);
                let tx = x + 130.0;
                let maxw = x + pw - 24.0 - tx;
                match top3.get(rank) {
                    Some((n, keys)) => {
                        let k = keys.len().min(4);
                        let size = match k {
                            1 => 120.0,
                            2 => 96.0,
                            _ => 70.0,
                        };
                        for (j, key) in keys.iter().take(4).enumerate() {
                            if let Some(img) = sprite_of(key) {
                                sprite(pen, &img, None, tx + size / 2.0 + j as f32 * size * 0.8, y + 26.0 + size / 2.0, size, size, true);
                            }
                        }
                        let names: Vec<String> = keys.iter().map(|k| name_of(k)).collect();
                        let label = if names.len() == 2 { names.join(" & ") } else { names.join(", ") };
                        let ls = pen.fit(&label, 34.0, maxw, f.title);
                        let label = clip(pen, &label, ls, f.title, maxw);
                        let ny = y + 40.0 + size;
                        pen.text(&label, tx, ny, ls, sk.text(), f.title, Align::Left, false);
                        let cnt = format!("{}{}", if *n == 1 { "1 area".to_string() } else { format!("{n} areas") }, if keys.len() > 1 { " each" } else { "" });
                        sk.chip(pen, &o, &cnt, tx, ny + ls + 12.0, None, Align::Left);
                    }
                    None => {
                        pen.text(&sk.case("Nobody yet"), tx, y + ph / 2.0 - 16.0, 30.0, sk.soft(), f.title, Align::Left, false);
                    }
                }
            }
        }

        // ── where to find them
        sk.title(pen, &o, &sk.case("Where to find them"), m, index_y, 60.0, Align::Left);
        pen.text("In route order, from your game log plus the shared list. Use Sweet Scent on the rare grass!", m, index_y + 76.0, 20.0, sk.soft(), f.body, Align::Left, false);
        let iy = index_y + INDEX_HEAD;
        if areas.is_empty() {
            pen.text("Nothing met on rare grass yet since the reset.", m, iy + 20.0, 22.0, sk.text(), f.body, Align::Left, false);
        }
        for &(ai, ci, yoff) in &placed {
            let a = &areas[ai];
            let (x, y) = (m + ci as f32 * (cw + GAP), iy + yoff);
            let chh = HEAD + a.mons.len().max(1) as f32 * row + PAD * 2.0;
            sk.panel(pen, &o, x, y, cw, chh, Role::Card);
            let col = sk.area_col(&o, ai);
            let badge = crate::poster::route_no(&a.name).map(|n| n.to_string()).unwrap_or_else(|| "★".into());
            let bw = sk.chip(pen, &o, &badge, x + 16.0, y + 16.0, Some(col), Align::Left);
            let tx = x + 16.0 + bw + 12.0;
            let maxw = x + cw - 16.0 - tx;
            let title = sk.case(&a.name);
            let ts = pen.fit(&title, 21.0, maxw, f.title);
            pen.text(&clip(pen, &title, ts, f.title, maxw), tx, y + 16.0 + (34.0 - ts * 1.2) / 2.0, ts, sk.text(), f.title, Align::Left, false);
            let rows_y = y + HEAD + PAD;
            for (i, mon) in a.mons.iter().enumerate() {
                let cy = rows_y + i as f32 * row + row / 2.0;
                if i > 0 {
                    pen.line(x + 16.0, cy - row / 2.0, x + cw - 16.0, cy - row / 2.0, 1.2, sk.soft().alpha(0.3));
                }
                let sx = x + 16.0 + sbox / 2.0;
                match &mon.sprite {
                    Some(img) => {
                        sprite(pen, img, None, sx, cy, sbox, sbox, true);
                    }
                    None => pen.circle(sx, cy, sbox * 0.3, col),
                }
                let nx = sx + sbox / 2.0 + 14.0;
                let tag = if mon.spots > 1 { format!("{} tiles", mon.spots) } else { "1 tile".to_string() };
                let tag = sk.case(&tag);
                let tag_w = pen.width(&tag, 14.0, f.label);
                let nmax = x + cw - 16.0 - tag_w - 14.0 - nx;
                let nm = clip(pen, &mon.name, fname, f.body, nmax);
                pen.text(&nm, nx, cy - fname * 0.62, fname, sk.text(), f.body, Align::Left, false);
                pen.text(&tag, x + cw - 16.0, cy - 9.0, 14.0, sk.soft(), f.label, Align::Right, false);
            }
        }
    }

    // ── footer
    let fy = h - m - foot_h;
    let (nm, na) = (tally.len(), areas.len());
    let mid = format!("{na} {} with rare grass · {nm} Pokémon", if na == 1 { "area" } else { "areas" });
    sk.footer(pen, &o, m, W - m, fy, foot_h, "Today", &mid, None, None);
    sk.overlay(pen, &o);
    std::mem::replace(&mut pen.c, Canvas::new(1, 1, WHITE))
}
