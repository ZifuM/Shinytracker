//! The shareable hunt infographic: themes, options and the layout. Drawing
//! goes through `Canvas`; text through the `TextDraw` trait (real fonts in
//! the app, a stand-in when previewing offline).

use crate::canvas::{crop, Canvas, Col, Img, Rng};

#[path = "card/anime.rs"]
mod anime;
use crate::stats::{fmt_duration, thousands};

#[cfg(not(offline_test))]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
pub enum CardTheme {
    Normal,
    Fire,
    Water,
    #[cfg_attr(not(offline_test), serde(alias = "Gold"))]
    Electric,
    #[cfg_attr(not(offline_test), serde(alias = "RareGrass"))]
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    #[cfg_attr(not(offline_test), serde(alias = "Platinum"))]
    Dragon,
    #[cfg_attr(not(offline_test), serde(alias = "NightSky"))]
    Dark,
    Steel,
    #[cfg_attr(not(offline_test), serde(alias = "Candy"))]
    Fairy,
}

/// The little things floating in the background.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motif {
    Stars,
    Embers,
    Bubbles,
    Bolts,
    Wisps,
    Snow,
    Leaves,
    Dust,
    Streaks,
}

impl CardTheme {
    pub const ALL: [CardTheme; 18] = [
        CardTheme::Normal,
        CardTheme::Fire,
        CardTheme::Water,
        CardTheme::Electric,
        CardTheme::Grass,
        CardTheme::Ice,
        CardTheme::Fighting,
        CardTheme::Poison,
        CardTheme::Ground,
        CardTheme::Flying,
        CardTheme::Psychic,
        CardTheme::Bug,
        CardTheme::Rock,
        CardTheme::Ghost,
        CardTheme::Dragon,
        CardTheme::Dark,
        CardTheme::Steel,
        CardTheme::Fairy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CardTheme::Normal => "Normal",
            CardTheme::Fire => "Fire",
            CardTheme::Water => "Water",
            CardTheme::Electric => "Electric",
            CardTheme::Grass => "Grass",
            CardTheme::Ice => "Ice",
            CardTheme::Fighting => "Fighting",
            CardTheme::Poison => "Poison",
            CardTheme::Ground => "Ground",
            CardTheme::Flying => "Flying",
            CardTheme::Psychic => "Psychic",
            CardTheme::Bug => "Bug",
            CardTheme::Rock => "Rock",
            CardTheme::Ghost => "Ghost",
            CardTheme::Dragon => "Dragon",
            CardTheme::Dark => "Dark",
            CardTheme::Steel => "Steel",
            CardTheme::Fairy => "Fairy",
        }
    }

    /// The type's signature colour.
    pub fn type_colour(self) -> Col {
        Col::hex(match self {
            CardTheme::Normal => 0xa8a77a,
            CardTheme::Fire => 0xee8130,
            CardTheme::Water => 0x6390f0,
            CardTheme::Electric => 0xf7d02c,
            CardTheme::Grass => 0x7ac74c,
            CardTheme::Ice => 0x96d9d6,
            CardTheme::Fighting => 0xc22e28,
            CardTheme::Poison => 0xa33ea1,
            CardTheme::Ground => 0xe2bf65,
            CardTheme::Flying => 0xa98ff3,
            CardTheme::Psychic => 0xf95587,
            CardTheme::Bug => 0xa6b91a,
            CardTheme::Rock => 0xb6a136,
            CardTheme::Ghost => 0x735797,
            CardTheme::Dragon => 0x6f35fc,
            CardTheme::Dark => 0x705746,
            CardTheme::Steel => 0xb7b7ce,
            CardTheme::Fairy => 0xd685ad,
        })
    }

    pub fn motif(self) -> Motif {
        match self {
            CardTheme::Fire => Motif::Embers,
            CardTheme::Water | CardTheme::Poison => Motif::Bubbles,
            CardTheme::Electric => Motif::Bolts,
            CardTheme::Ghost | CardTheme::Psychic => Motif::Wisps,
            CardTheme::Ice => Motif::Snow,
            CardTheme::Grass | CardTheme::Bug | CardTheme::Flying => Motif::Leaves,
            CardTheme::Ground | CardTheme::Rock => Motif::Dust,
            CardTheme::Fighting => Motif::Streaks,
            _ => Motif::Stars,
        }
    }

    pub fn palette(self) -> Palette {
        let t = self.type_colour();
        match self {
            CardTheme::Fire => Palette {
                bg_a: Col::hex(0x1e0703),
                bg_b: Col::hex(0x6e1a08),
                panel_a: Col::hex(0x7a2610),
                panel_b: Col::hex(0x401106),
                line: Col::hex(0xc85a26),
                accent: Col::hex(0xf4511e),
                highlight: Col::hex(0xffcb47),
                text: Col::hex(0xfff4e8),
                muted: Col::hex(0xf0bd98),
                shadow: Col(28, 4, 0, 160),
                platform: Col::hex(0x9a3a16),
                light: false,
            },
            CardTheme::Water => Palette {
                bg_a: Col::hex(0x03183a),
                bg_b: Col::hex(0x0a5687),
                panel_a: Col::hex(0x0e5b8a),
                panel_b: Col::hex(0x073457),
                line: Col::hex(0x3c9fd6),
                accent: Col::hex(0x1e88e5),
                highlight: Col::hex(0x80eaff),
                text: Col::hex(0xffffff),
                muted: Col::hex(0xa8d6f0),
                shadow: Col(0, 10, 30, 160),
                platform: Col::hex(0x136a9a),
                light: false,
            },
            CardTheme::Electric => Palette {
                bg_a: Col::hex(0x12110a),
                bg_b: Col::hex(0x37310f),
                panel_a: Col::hex(0x3d3712),
                panel_b: Col::hex(0x201d08),
                line: Col::hex(0xa3962a),
                accent: Col::hex(0xffd01f),
                highlight: Col::hex(0xfff06b),
                text: Col::hex(0xfffbe6),
                muted: Col::hex(0xe0d496),
                shadow: Col(8, 6, 0, 170),
                platform: Col::hex(0x5a5218),
                light: false,
            },
            CardTheme::Grass => Palette {
                bg_a: Col::hex(0x0a1f12),
                bg_b: Col::hex(0x16502c),
                panel_a: Col::hex(0x1f6338),
                panel_b: Col::hex(0x123d22),
                line: Col::hex(0x4fa564),
                accent: Col::hex(0x5cc23f),
                highlight: Col::hex(0xfff06a),
                text: Col::hex(0xffffff),
                muted: Col::hex(0xb9dfc1),
                shadow: Col(2, 14, 6, 160),
                platform: Col::hex(0x2f7d47),
                light: false,
            },
            CardTheme::Ghost => Palette {
                bg_a: Col::hex(0x0b0516),
                bg_b: Col::hex(0x2a0f45),
                panel_a: Col::hex(0x33154e),
                panel_b: Col::hex(0x1a0a2b),
                line: Col::hex(0x784cae),
                accent: Col::hex(0x8e4dff),
                highlight: Col::hex(0xd9adff),
                text: Col::hex(0xf7efff),
                muted: Col::hex(0xbaa4d6),
                shadow: Col(4, 0, 12, 170),
                platform: Col::hex(0x48226e),
                light: false,
            },
            CardTheme::Ice => Palette {
                bg_a: Col::hex(0xc9ebfb),
                bg_b: Col::hex(0xf3fbff),
                panel_a: Col::hex(0xffffff),
                panel_b: Col::hex(0xe6f5fd),
                line: Col::hex(0x8cc8e6),
                accent: Col::hex(0x1b86c6),
                highlight: Col::hex(0x0b66a0),
                text: Col::hex(0x0a2738),
                muted: Col::hex(0x3c6982),
                shadow: Col(20, 80, 120, 60),
                platform: Col::hex(0xcbe9f8),
                light: true,
            },
            CardTheme::Fairy => Palette {
                bg_a: Col::hex(0xffc4da),
                bg_b: Col::hex(0xffeef5),
                panel_a: Col::hex(0xfffafc),
                panel_b: Col::hex(0xffeaf2),
                line: Col::hex(0xe58db0),
                accent: Col::hex(0xd81b60),
                highlight: Col::hex(0xc2185b),
                text: Col::hex(0x2e0f26),
                muted: Col::hex(0x6e2f58),
                shadow: Col(150, 40, 90, 60),
                platform: Col::hex(0xffc9de),
                light: true,
            },
            CardTheme::Normal => light_palette(t, Col::hex(0x8a6d2f)),
            CardTheme::Fighting => dark_palette(t, Col::hex(0xffb36b)),
            CardTheme::Poison => dark_palette(t, Col::hex(0xeaa8ff)),
            CardTheme::Ground => dark_palette(t, Col::hex(0xffe3a1)),
            CardTheme::Flying => dark_palette(t, Col::hex(0xe4dcff)),
            CardTheme::Psychic => dark_palette(t, Col::hex(0xffc2d8)),
            CardTheme::Bug => dark_palette(t, Col::hex(0xecff80)),
            CardTheme::Rock => dark_palette(Col::hex(0x8f8266), Col::hex(0xf0e2b8)),
            CardTheme::Dragon => dark_palette(t, Col::hex(0xffd76b)),
            CardTheme::Dark => dark_palette(t, Col::hex(0xffd76b)),
            CardTheme::Steel => dark_palette(t, Col::hex(0xeaf2ff)),
        }
    }
}

/// A dark card tinted with the type colour.
fn dark_palette(t: Col, highlight: Col) -> Palette {
    let black = Col::hex(0x07060a);
    let white = Col::hex(0xffffff);
    Palette {
        bg_a: black.mix(t, 0.12),
        bg_b: black.mix(t, 0.36),
        panel_a: black.mix(t, 0.40),
        panel_b: black.mix(t, 0.20),
        line: black.mix(t, 0.75),
        accent: t,
        highlight,
        text: white.mix(t, 0.05),
        muted: white.mix(t, 0.42),
        shadow: Col(4, 3, 6, 165),
        platform: black.mix(t, 0.55),
        light: false,
    }
}

/// A light card tinted with the type colour (dark text).
fn light_palette(t: Col, highlight: Col) -> Palette {
    let black = Col::hex(0x15130f);
    let white = Col::hex(0xffffff);
    Palette {
        bg_a: white.mix(t, 0.38),
        bg_b: white.mix(t, 0.08),
        panel_a: white.mix(t, 0.02),
        panel_b: white.mix(t, 0.12),
        line: white.mix(t, 0.65),
        accent: black.mix(t, 0.62),
        highlight,
        text: black.mix(t, 0.12),
        muted: black.mix(t, 0.45),
        shadow: Col(60, 50, 30, 55),
        platform: white.mix(t, 0.35),
        light: true,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_a: Col,
    pub bg_b: Col,
    pub panel_a: Col,
    pub panel_b: Col,
    pub line: Col,
    pub accent: Col,
    pub highlight: Col,
    pub text: Col,
    pub muted: Col,
    pub shadow: Col,
    pub platform: Col,
    /// Light background: status colours are darkened so they stay readable.
    pub light: bool,
}

/// The look of the share images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
pub enum CardStyle {
    /// Flat anime-style infographic: bold flat colours, ink outlines, speed
    /// lines, halftone dots and sticker-style stats (the default since 0.5).
    #[default]
    Anime,
    /// The original look: dark gradient panels and the pixel font.
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
pub enum CardLayout {
    /// 1200 × 675, good for Discord and Twitter/X.
    Wide,
    /// 1080 × 1080, good for Instagram and phone screens.
    Square,
}

impl CardLayout {
    /// The size the layout is designed at (drawing is scaled from this).
    pub fn size(self) -> (u32, u32) {
        match self {
            CardLayout::Wide => (1200, 675),
            CardLayout::Square => (1080, 1080),
        }
    }
}

pub const RESOLUTIONS: [u32; 4] = [480, 720, 1080, 1440];

impl CardOptions {
    /// Output size in pixels and the scale from the design size.
    pub fn output_size(&self) -> (u32, u32, f32) {
        let res = if RESOLUTIONS.contains(&self.res) { self.res } else { 1080 };
        let (bw, bh) = self.layout.size();
        let s = res as f32 / bh as f32;
        let w = ((bw as f32 * s / 2.0).round() * 2.0) as u32;
        (w, res, s)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct CardOptions {
    pub style: CardStyle,
    pub theme: CardTheme,
    pub layout: CardLayout,
    /// Empty = automatic ("SHINY PORYGON!" / "PORYGON").
    pub title: String,
    pub trainer: String,
    pub show_normal: bool,
    pub show_odds: bool,
    pub show_meter: bool,
    pub show_time: bool,
    pub show_rate: bool,
    pub show_dates: bool,
    pub sparkles: bool,
    /// Animated GIF (moving sprite, floating stars) instead of a still PNG.
    pub animated: bool,
    /// Output height in pixels: 480, 720, 1080 or 1440 (width follows the shape).
    pub res: u32,
    /// Show the latest hunt journal note.
    pub show_note: bool,
    /// Nickname and gender of the Pokémon on the image being made. Kept per
    /// hunt in the settings (not here), so they aren't saved with the style.
    #[cfg_attr(not(offline_test), serde(skip))]
    pub nickname: String,
    #[cfg_attr(not(offline_test), serde(skip))]
    pub gender: Gender,
}

impl Default for CardOptions {
    fn default() -> Self {
        Self {
            style: CardStyle::Anime,
            theme: CardTheme::Dragon,
            layout: CardLayout::Wide,
            title: String::new(),
            trainer: String::new(),
            show_normal: true,
            show_odds: true,
            show_meter: true,
            show_time: true,
            show_rate: true,
            show_dates: true,
            sparkles: true,
            animated: false,
            res: 1080,
            show_note: true,
            nickname: String::new(),
            gender: Gender::None,
        }
    }
}

/// The Pokémon's gender on the share image (None = genderless or not set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
pub enum Gender {
    #[default]
    None,
    Male,
    Female,
}

/// Everything the card shows about a hunt.
#[derive(Clone, Default)]
pub struct CardData {
    pub name: String,
    pub dex: Option<u32>,
    pub caught: bool,
    pub encounters: u64,
    pub odds_one_in: Option<f64>,
    /// "Rare grass · Doubles"
    pub method: String,
    pub hunt_secs: u64,
    pub per_hour: Option<f64>,
    pub days_hunted: usize,
    /// "8 Nov 2026"
    pub started: Option<String>,
    pub finished: Option<String>,
    pub shiny: Option<Img>,
    pub normal: Option<Img>,
    pub icon: Option<Img>,
    /// Fixed crop box for the shiny sprite (keeps it still between animation
    /// frames); None = trim each image on its own.
    pub sprite_box: Option<(u32, u32, u32, u32)>,
    /// Trainer level, shown next to the trainer name.
    pub trainer_level: Option<u32>,
    /// Latest hunt journal note.
    pub note: Option<String>,
    /// How many of the encounters were the target (when known).
    pub target_seen: Option<u64>,
    /// The luck meter counts only target encounters.
    pub odds_on_target: bool,
    /// Shinies of other Pokémon found during this hunt.
    pub phases: Vec<String>,
    /// The player's nickname for it (shown as the title, species underneath).
    pub nickname: Option<String>,
    pub gender: Gender,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    /// Press Start 2P (falls back to the body font).
    Pixel,
    Body,
    /// Heavy sans for labels (anime style).
    Bold,
    /// Black italic display face for big titles and numbers (anime style).
    Display,
}

pub trait TextDraw {
    fn width(&self, s: &str, size: f32, face: Face) -> f32;
    /// Draw with the top of the text box at `y`.
    fn draw(&self, c: &mut Canvas, s: &str, x: f32, y: f32, size: f32, col: Col, face: Face);
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

/// Draws in design units (e.g. 1200 × 675) onto a canvas of any size.
struct Pen<'a> {
    c: Canvas,
    t: &'a dyn TextDraw,
    p: Palette,
    phase: Option<f32>,
    /// pixels per design unit
    s: f32,
    /// design width / height
    w: f32,
    h: f32,
}

impl Pen<'_> {
    fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        let s = self.s;
        self.c.rrect(x * s, y * s, w * s, h * s, r * s, c);
    }
    #[allow(clippy::too_many_arguments)]
    fn rrect_vgrad(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, a: Col, b: Col) {
        let s = self.s;
        self.c.rrect_vgrad(x * s, y * s, w * s, h * s, r * s, a, b);
    }
    #[allow(clippy::too_many_arguments)]
    fn rrect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, c: Col) {
        let s = self.s;
        self.c.rrect_stroke(x * s, y * s, w * s, h * s, r * s, (t * s).max(1.0), c);
    }
    /// `paint` gets design-unit coordinates.
    fn rrect_with(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, paint: impl Fn(f32, f32) -> Col) {
        let s = self.s;
        self.c.rrect_with(x * s, y * s, w * s, h * s, r * s, |px, py| paint(px as f32 / s, py as f32 / s));
    }
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Col) {
        let s = self.s;
        self.c.ellipse(cx * s, cy * s, rx * s, ry * s, c);
    }
    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        let s = self.s;
        self.c.circle(cx * s, cy * s, r * s, c);
    }
    fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, c: Col) {
        let s = self.s;
        self.c.ring(cx * s, cy * s, r * s, (t * s).max(0.8), c);
    }
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: Col) {
        let s = self.s;
        self.c.line(x0 * s, y0 * s, x1 * s, y1 * s, (w * s).max(0.8), c);
    }
    fn glow(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        let s = self.s;
        self.c.glow(cx * s, cy * s, r * s, c);
    }
    fn sparkle(&mut self, cx: f32, cy: f32, size: f32, c: Col) {
        let s = self.s;
        self.c.sparkle(cx * s, cy * s, size * s, c);
    }
    /// Pixel-art sprite at `k` design units per sprite pixel (crisp).
    fn blit_px(&mut self, img: &Img, x: f32, y: f32, k: f32) {
        let s = self.s;
        self.c.blit_nearest(img, x * s, y * s, k * s);
    }
    fn poly(&mut self, pts: &[(f32, f32)], c: Col) {
        let s = self.s;
        let v: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x * s, y * s)).collect();
        self.c.poly(&v, c);
    }
    fn blit_smooth(&mut self, img: &Img, x: f32, y: f32, w: f32, h: f32) {
        let s = self.s;
        self.c.blit_smooth(img, x * s, y * s, w * s, h * s);
    }
    /// Text width in design units.
    fn width(&self, text: &str, size: f32, face: Face) -> f32 {
        self.t.width(text, size * self.s, face) / self.s
    }
}

impl Pen<'_> {
    /// Text with an optional drop shadow. Returns its width.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, s: &str, x: f32, y: f32, size: f32, col: Col, face: Face, align: Align, shadow: bool) -> f32 {
        let w = self.width(s, size, face);
        let x = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let k = self.s;
        if shadow {
            let off = (size / 14.0).clamp(2.0, 5.0);
            self.t.draw(&mut self.c, s, (x + off) * k, (y + off) * k, size * k, self.p.shadow, face);
        }
        self.t.draw(&mut self.c, s, x * k, y * k, size * k, col, face);
        w
    }

    /// Largest size ≤ `size` that fits in `max_w`.
    fn fit(&self, s: &str, size: f32, max_w: f32, face: Face) -> f32 {
        let w = self.width(s, size, face);
        if w <= max_w {
            size
        } else {
            (size * max_w / w).floor().max(10.0)
        }
    }
}

/// Readable text colour on a filled chip: dark on bright fills, white otherwise.
fn ink_on(fill: Col, dark: Col) -> Col {
    let lum = (0.2126 * fill.0 as f32 + 0.7152 * fill.1 as f32 + 0.0722 * fill.2 as f32) / 255.0;
    if lum > 0.55 {
        dark
    } else {
        Col::hex(0xffffff)
    }
}

/// "Under odds" / "At odds" / "Over odds" with its colour.
pub fn luck(mult: f64) -> (&'static str, Col) {
    if mult < 0.9 {
        ("UNDER ODDS", Col::hex(0x5fd36f))
    } else if mult <= 1.1 {
        ("AT ODDS", Col::hex(0xffcf3a))
    } else {
        ("OVER ODDS", Col::hex(0xff6a55))
    }
}

fn fmt_mult(m: f64) -> String {
    if m < 0.1 {
        format!("{m:.2}x")
    } else {
        format!("{m:.1}x")
    }
}

/// Stat tiles in priority order, filtered by the options.
fn tiles(d: &CardData, o: &CardOptions) -> Vec<(&'static str, String)> {
    let mut v = Vec::new();
    if o.show_time && d.hunt_secs >= 60 {
        v.push(("Hunt time", fmt_duration(d.hunt_secs)));
    }
    if o.show_rate {
        if let Some(r) = d.per_hour {
            v.push(("Per hour", thousands(r.round() as u64)));
        }
    }
    if o.show_time && d.days_hunted > 0 {
        v.push(("Days hunted", d.days_hunted.to_string()));
    }
    if o.show_dates {
        if let Some(s) = &d.started {
            v.push(("Started", short_date(s)));
        }
        if let Some(f) = &d.finished {
            v.push((if d.caught { "Caught" } else { "Finished" }, short_date(f)));
        }
    }
    // A caught hunt's date matters more than "days hunted": move it up.
    if d.caught && v.len() > 2 {
        if let Some(i) = v.iter().position(|(l, _)| *l == "Caught") {
            let item = v.remove(i);
            v.insert(2, item);
        }
    }
    v
}

/// "10 Nov 2026" -> "10 Nov 26" (keeps tiles narrow).
fn short_date(s: &str) -> String {
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.len() == 3 && parts[2].len() == 4 {
        format!("{} {} '{}", parts[0], parts[1], &parts[2][2..])
    } else {
        s.to_string()
    }
}

/// ♂ / ♀ drawn as shapes (the pixel font has no such characters).
fn gender_sign(pen: &mut Pen, g: Gender, cx: f32, cy: f32, r: f32) {
    let t = (r * 0.32).max(2.0);
    match g {
        Gender::None => {}
        Gender::Male => {
            let c = Col::rgb(96, 168, 255);
            let (cx, cy) = (cx - r * 0.2, cy + r * 0.2);
            let k = std::f32::consts::FRAC_1_SQRT_2;
            let (sx, sy) = (cx + r * k, cy - r * k);
            let (ex, ey) = (cx + r * 1.75, cy - r * 1.75);
            pen.line(sx + 1.0, sy + 1.0, ex + 1.0, ey + 1.0, t, Col::rgb(0, 0, 0).alpha(0.35));
            pen.ring(cx + 1.5, cy + 1.5, r, t, Col::rgb(0, 0, 0).alpha(0.35));
            pen.ring(cx, cy, r, t, c);
            pen.line(sx, sy, ex, ey, t, c);
            pen.line(ex, ey, ex - r * 0.8, ey, t, c);
            pen.line(ex, ey, ex, ey + r * 0.8, t, c);
        }
        Gender::Female => {
            let c = Col::rgb(255, 120, 172);
            let cy = cy - r * 0.45;
            pen.ring(cx + 1.5, cy + 1.5, r, t, Col::rgb(0, 0, 0).alpha(0.35));
            pen.ring(cx, cy, r, t, c);
            pen.line(cx, cy + r, cx, cy + r * 2.3, t, c);
            pen.line(cx - r * 0.6, cy + r * 1.65, cx + r * 0.6, cy + r * 1.65, t, c);
        }
    }
}

fn auto_title(d: &CardData) -> String {
    if let Some(n) = d.nickname.as_ref().map(|n| n.trim()).filter(|n| !n.is_empty()) {
        return n.to_uppercase();
    }
    if d.caught {
        format!("SHINY {}!", d.name.to_uppercase())
    } else {
        d.name.to_uppercase()
    }
}

pub fn render(d: &CardData, o: &CardOptions, t: &dyn TextDraw) -> Canvas {
    render_frame(d, o, t, None)
}

/// One frame of the animated version: `phase` runs 0..1 over the loop
/// (None = still image). Everything that moves loops seamlessly.
pub fn render_frame(d: &CardData, o: &CardOptions, t: &dyn TextDraw, phase: Option<f32>) -> Canvas {
    render_on(render_base(o), d, o, t, phase)
}

/// The parts that never move (gradient and corner glows). Animations make
/// this once and reuse it for every frame.
pub fn render_base(o: &CardOptions) -> Canvas {
    if o.style == CardStyle::Anime {
        return anime::base(o);
    }
    let (w, h, s) = o.output_size();
    let (dw, dh) = o.layout.size();
    let p = o.theme.palette();
    let mut c = Canvas::new(w, h, p.bg_a);
    c.diagonal_gradient(p.bg_a, p.bg_b);
    let (dw, dh) = (dw as f32, dh as f32);
    c.glow(dw * 0.95 * s, dh * 0.05 * s, dw * 0.35 * s, p.accent.alpha(0.18));
    c.glow(dw * 0.05 * s, dh * 0.98 * s, dw * 0.30 * s, p.highlight.alpha(0.10));
    c
}

/// Draw a frame on top of a copy of `render_base`.
pub fn render_on(base: Canvas, d: &CardData, o: &CardOptions, t: &dyn TextDraw, phase: Option<f32>) -> Canvas {
    let (_, _, s) = o.output_size();
    let (dw, dh) = o.layout.size();
    let p = o.theme.palette();
    let mut pen = Pen { c: base, t, p, phase, s, w: dw as f32, h: dh as f32 };
    if o.style == CardStyle::Anime {
        anime::hunt(&mut pen, d, o);
        return pen.c;
    }
    background_decor(&mut pen, o);
    match o.layout {
        CardLayout::Wide => wide(&mut pen, d, o),
        CardLayout::Square => square(&mut pen, d, o),
    }
    pen.c
}

fn background_decor(pen: &mut Pen, o: &CardOptions) {
    let (w, h) = (pen.w, pen.h);
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let motif = o.theme.motif();
    let p = pen.p;
    // Animated: everything drifts a little (up, or down for snow), fading in
    // and out. Each path repeats exactly once per loop, so the GIF loops
    // seamlessly.
    let dir = if motif == Motif::Snow { -1.0 } else { 1.0 };
    let snow = if p.light { p.accent } else { Col::hex(0xffffff) };
    let place = |phase: Option<f32>, y: f32, rise: f32, off: f32| -> (f32, f32) {
        match phase {
            None => (y, 1.0),
            Some(ph) => {
                let t = (ph + off).fract();
                (y + dir * (rise * 0.5 - rise * t), (std::f32::consts::PI * t).sin())
            }
        }
    };
    // small particles
    let count = match motif {
        Motif::Bolts => 70,
        Motif::Wisps => 60,
        _ => 140,
    };
    for _ in 0..count {
        let (x, y) = (rng.next() * w, rng.next() * h);
        let r = 0.8 + rng.next() * 1.6;
        let a = 0.15 + rng.next() * 0.45;
        let (rise, off) = (30.0 + rng.next() * 70.0, rng.next());
        let tint = rng.next();
        let (y, fade) = place(pen.phase, y, rise, off);
        let a = a * fade;
        match motif {
            Motif::Stars | Motif::Bolts => pen.circle(x, y, r, p.text.alpha(0.5 * a)),
            Motif::Embers => {
                let c = p.accent.mix(p.highlight, tint);
                pen.circle(x, y, r * 1.6, c.alpha(0.25 * a));
                pen.circle(x, y, r, c.alpha(0.9 * a));
            }
            Motif::Bubbles => pen.ring(x, y, 2.0 + r * 2.5, 1.0, p.highlight.alpha(0.55 * a)),
            Motif::Wisps => pen.glow(x, y, 6.0 + r * 5.0, p.highlight.alpha(0.35 * a)),
            Motif::Snow => pen.circle(x, y, r * 1.2, snow.alpha(0.85 * a)),
            Motif::Leaves => {
                let ang = tint * std::f32::consts::TAU;
                let (dx, dy) = (ang.cos() * r * 1.8, ang.sin() * r * 1.8);
                let c = p.accent.mix(p.highlight, tint * 0.6);
                pen.line(x - dx, y - dy, x + dx, y + dy, r * 1.5, c.alpha(0.75 * a));
            }
            Motif::Dust => {
                let c = p.muted.mix(p.highlight, tint);
                pen.circle(x, y, r * (0.8 + tint), c.alpha(0.7 * a));
            }
            Motif::Streaks => {
                let len = 6.0 + r * 5.0;
                pen.line(x, y, x + len * 0.45, y - len, 1.2, p.highlight.alpha(0.55 * a));
            }
        }
    }
    if !o.sparkles {
        return;
    }
    // bigger accents
    for _ in 0..14 {
        let (x, y) = (rng.next() * w, rng.next() * h);
        let size = 5.0 + rng.next() * 9.0;
        let a = 0.18 + rng.next() * 0.25;
        let (rise, off) = (50.0 + rng.next() * 60.0, rng.next());
        let (y, fade) = place(pen.phase, y, rise, off);
        let a = a * fade;
        match motif {
            Motif::Stars => pen.sparkle(x, y, size, p.highlight.alpha(a)),
            Motif::Embers => {
                pen.glow(x, y, size * 2.2, p.accent.alpha(0.5 * a));
                pen.circle(x, y, size * 0.35, p.highlight.alpha((2.0 * a).min(1.0)));
            }
            Motif::Bubbles => {
                pen.ring(x, y, size, 1.6, p.highlight.alpha((1.8 * a).min(1.0)));
                pen.circle(x - size * 0.35, y - size * 0.35, size * 0.18, Col::hex(0xffffff).alpha((1.8 * a).min(1.0)));
            }
            Motif::Bolts => {
                let s = size * 1.4;
                let c = p.highlight.alpha((2.2 * a).min(1.0));
                let pts = [(x + s * 0.25, y - s), (x - s * 0.2, y - s * 0.05), (x + s * 0.2, y + s * 0.05), (x - s * 0.25, y + s)];
                for k in 0..3 {
                    pen.line(pts[k].0, pts[k].1, pts[k + 1].0, pts[k + 1].1, 2.4, c);
                }
            }
            Motif::Wisps => {
                pen.glow(x, y, size * 2.6, p.highlight.alpha(0.6 * a));
                pen.glow(x, y - size * 0.8, size * 1.6, p.accent.alpha(0.5 * a));
            }
            Motif::Snow => {
                let c = snow.alpha((2.6 * a).min(1.0));
                let s = size * 0.9;
                for k in 0..3 {
                    let ang = k as f32 * std::f32::consts::PI / 3.0;
                    let (dx, dy) = (ang.cos() * s, ang.sin() * s);
                    pen.line(x - dx, y - dy, x + dx, y + dy, 1.6, c);
                }
                pen.circle(x, y, 1.6, c);
            }
            Motif::Leaves => {
                let ang = 0.6 + size * 0.3;
                let (dx, dy) = (ang.cos() * size, ang.sin() * size * 0.6);
                let c = p.accent.mix(p.highlight, 0.35).alpha((2.2 * a).min(1.0));
                pen.line(x - dx, y - dy, x + dx, y + dy, size * 0.75, c);
                pen.line(x - dx * 0.7, y - dy * 0.7, x + dx * 0.7, y + dy * 0.7, 1.0, p.bg_a.alpha((1.4 * a).min(0.6)));
            }
            Motif::Dust => {
                pen.circle(x, y, size * 0.32, p.highlight.alpha((1.6 * a).min(1.0)));
                pen.circle(x + size * 0.5, y + size * 0.3, size * 0.18, p.muted.alpha((1.4 * a).min(1.0)));
            }
            Motif::Streaks => {
                let c = p.highlight.alpha((2.0 * a).min(1.0));
                for k in 0..3 {
                    let o = k as f32 * 5.0;
                    let len = size * (1.6 - 0.3 * k as f32);
                    pen.line(x + o, y + o * 0.4, x + o + len * 0.45, y + o * 0.4 - len, 2.0, c);
                }
            }
        }
    }
}

/// The sprite stage: panel, glow, platform, shiny sprite, normal inset, sparkles.
fn stage(pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
    let p = pen.p;
    pen.rrect(x + 6.0, y + 10.0, w, h, 30.0, p.shadow.alpha(0.6));
    pen.rrect_vgrad(x, y, w, h, 30.0, p.panel_a, p.panel_b);
    pen.rrect_stroke(x, y, w, h, 30.0, 3.0, p.highlight.alpha(0.55));
    let cx = x + w / 2.0;
    let floor = y + h - 70.0;
    pen.glow(cx, floor - h * 0.32, h * 0.55, p.highlight.alpha(0.30));
    pen.ellipse(cx, floor + 4.0, w.min(420.0) * 0.36, 30.0, p.shadow.alpha(0.55));
    pen.ellipse(cx, floor, w.min(420.0) * 0.34, 26.0, p.platform);
    pen.ellipse(cx, floor - 3.0, w.min(420.0) * 0.28, 17.0, p.platform.mix(p.text, 0.15));
    if let Some(ph) = pen.phase {
        let hl = p.highlight;
        // little stars floating up from the platform
        if o.sparkles {
            let mut rng = Rng(0x51ce_ba11_2024_0001);
            for _ in 0..9 {
                let lane = rng.next();
                let off = rng.next();
                let t = (ph + off).fract();
                let sx = cx + (lane - 0.5) * w.min(420.0) * 0.6 + (std::f32::consts::TAU * (t + lane)).sin() * 8.0;
                let sy = floor - t * h * 0.7;
                let a = (t * 4.0).min(1.0) * (1.0 - t);
                pen.sparkle(sx, sy, 4.0 + 5.0 * rng.next(), hl.alpha(0.9 * a));
            }
        }
    }

    // "SHINY" chip
    let chip_txt = "SHINY";
    let cw = pen.width(chip_txt, 14.0, Face::Pixel) + 52.0;
    let (chx, chy) = (x + w - cw - 18.0, y + 18.0);
    pen.rrect(chx, chy, cw, 34.0, 17.0, p.highlight);
    let chip_ink = ink_on(p.highlight, p.bg_a);
    pen.sparkle(chx + 20.0, chy + 17.0, 9.0, chip_ink);
    pen.text(chip_txt, chx + 34.0, chy + 10.0, 14.0, chip_ink, Face::Pixel, Align::Left, false);

    match &d.shiny {
        Some(img) => {
            let tb = d.sprite_box.unwrap_or_else(|| Canvas::trim_box(img));
            let sp = crop(img, tb);
            let max_w = (w - 80.0).min(380.0);
            let max_h = h - 130.0;
            let k = ((max_w / sp.w as f32).min(max_h / sp.h as f32)).floor().clamp(1.0, 7.0) as u32;
            let (sw, sh) = ((sp.w * k) as f32, (sp.h * k) as f32);
            let sx = (cx - sw / 2.0).round();
            let sy = (floor + 6.0 - sh).round();
            pen.blit_px(&sp, sx, sy, k as f32);
            if o.sparkles {
                let hl = p.highlight;
                pen.sparkle(sx + sw + 4.0, sy + 10.0, 22.0, hl);
                pen.sparkle(sx - 14.0, sy + sh * 0.35, 13.0, hl.alpha(0.9));
                pen.sparkle(sx + sw + 18.0, sy + sh * 0.55, 10.0, hl.alpha(0.8));
                pen.sparkle(sx + sw * 0.25, sy - 12.0, 9.0, p.text.alpha(0.85));
            }
        }
        None => {
            pen.text("?", cx, floor - 150.0, 96.0, p.muted, Face::Pixel, Align::Center, true);
        }
    }

    if o.show_normal {
        if let Some(img) = &d.normal {
            let sp = crop(img, Canvas::trim_box(img));
            let box_s = 116.0;
            let (bx, by) = (x + 18.0, y + 18.0);
            pen.rrect(bx, by, box_s, box_s + 26.0, 18.0, p.shadow.alpha(0.5));
            pen.rrect_stroke(bx, by, box_s, box_s + 26.0, 18.0, 2.0, p.line);
            let k = ((box_s - 22.0) / sp.w.max(sp.h) as f32).floor().clamp(1.0, 3.0) as u32;
            let (sw, sh) = ((sp.w * k) as f32, (sp.h * k) as f32);
            pen.blit_px(&sp, (bx + (box_s - sw) / 2.0).round(), (by + 8.0 + (box_s - 14.0 - sh)).round(), k as f32);
            pen.text("NORMAL", bx + box_s / 2.0, by + box_s + 2.0, 11.0, p.muted, Face::Pixel, Align::Center, false);
        }
    }
}

/// Big encounter number with its label. Returns the bottom y.
fn encounters(pen: &mut Pen, d: &CardData, x: f32, y: f32, max_w: f32, align: Align) -> f32 {
    let n = thousands(d.encounters);
    let size = pen.fit(&n, 84.0, max_w, Face::Pixel);
    pen.text(&n, x, y, size, pen.p.highlight, Face::Pixel, align, true);
    let mut label = if d.encounters == 1 { "ENCOUNTER" } else { "ENCOUNTERS" }.to_string();
    if let Some(ts) = d.target_seen {
        label.push_str(&format!(" · {} {} SEEN", thousands(ts), d.name.to_uppercase()));
    }
    let ls = pen.fit(&label, 16.0, max_w, Face::Pixel);
    pen.text(&label, x, y + size + 16.0, ls, pen.p.muted, Face::Pixel, align, false);
    y + size + 16.0 + 16.0
}

/// Luck meter: how far through (up to 3×) the odds the hunt is.
fn meter(pen: &mut Pen, d: &CardData, x: f32, y: f32, w: f32) -> f32 {
    let Some(odds) = d.odds_one_in else { return y };
    let p = pen.p;
    let n = match (d.odds_on_target, d.target_seen) {
        (true, Some(ts)) => ts,
        _ => d.encounters,
    };
    let mult = n as f64 / odds;
    let (label, col) = luck(mult);
    // the bright status colours vanish on light backgrounds: use deeper ones
    let col = if p.light { col.mix(Col::hex(0x000000), 0.38) } else { col };
    pen.text(label, x, y, 14.0, col, Face::Pixel, Align::Left, false);
    pen.text(&format!("{} THE ODDS", fmt_mult(mult)), x + w, y, 14.0, p.text, Face::Pixel, Align::Right, false);
    let by = y + 28.0;
    let bh = 26.0;
    pen.rrect(x, by, w, bh, bh / 2.0, p.shadow.alpha(0.7));
    pen.rrect_stroke(x, by, w, bh, bh / 2.0, 2.0, p.line);
    let frac = (mult / 3.0).clamp(0.0, 1.0) as f32;
    if frac > 0.0 {
        let fw = (w * frac).max(bh);
        let green = Col::hex(0x5fd36f);
        let gold = Col::hex(0xffcf3a);
        let red = Col::hex(0xff6a55);
        pen.rrect_with(x + 3.0, by + 3.0, fw - 6.0, bh - 6.0, (bh - 6.0) / 2.0, |px, _| {
            let t = (px as f32 - x) / w * 3.0; // 0..3 across the bar
            if t < 1.0 {
                green.mix(gold, t)
            } else {
                gold.mix(red, (t - 1.0) / 2.0)
            }
        });
        // shine
        pen.rrect(x + 10.0, by + 6.0, (fw - 20.0).max(0.0), 4.0, 2.0, p.text.alpha(0.25));
    }
    for k in 1..=3 {
        let tx = x + w * k as f32 / 3.0;
        if k < 3 {
            pen.rrect(tx - 1.5, by - 4.0, 3.0, bh + 8.0, 1.5, p.text.alpha(0.55));
        }
        let a = if k == 3 { Align::Right } else { Align::Center };
        let lx = if k == 3 { x + w } else { tx };
        pen.text(&format!("{k}x"), lx, by + bh + 8.0, 12.0, p.muted, Face::Pixel, a, false);
    }
    by + bh + 8.0 + 12.0
}

fn tile_row(pen: &mut Pen, list: &[(&str, String)], x: f32, y: f32, w: f32, per_row: usize) -> f32 {
    if list.is_empty() {
        return y;
    }
    let p = pen.p;
    let gap = 16.0;
    let n = list.len().min(per_row);
    let tw = (w - gap * (n as f32 - 1.0)) / n as f32;
    let th = 92.0;
    for (i, (label, value)) in list.iter().take(n).enumerate() {
        let tx = x + i as f32 * (tw + gap);
        pen.rrect(tx, y, tw, th, 18.0, p.panel_b.alpha(0.85));
        pen.rrect_stroke(tx, y, tw, th, 18.0, 2.0, p.line.alpha(0.8));
        pen.text(label, tx + 18.0, y + 16.0, 19.0, p.muted, Face::Body, Align::Left, false);
        let vs = pen.fit(value, 24.0, tw - 36.0, Face::Pixel);
        pen.text(value, tx + 18.0, y + 52.0, vs, p.text, Face::Pixel, Align::Left, false);
    }
    y + th
}

fn footer(pen: &mut Pen, d: &CardData, o: &CardOptions, x0: f32, x1: f32, y: f32) {
    let p = pen.p;
    if !o.trainer.trim().is_empty() {
        let w = pen.text("TRAINER", x0, y + 8.0, 12.0, p.muted, Face::Pixel, Align::Left, false);
        let name = o.trainer.trim().to_uppercase();
        let nw = pen.text(&name, x0 + w + 14.0, y + 5.0, 18.0, p.text, Face::Pixel, Align::Left, false);
        if let Some(lv) = d.trainer_level {
            let chip = format!("LV {lv}");
            let cw = pen.width(&chip, 11.0, Face::Pixel) + 16.0;
            let cx = x0 + w + 14.0 + nw + 12.0;
            pen.rrect(cx, y + 2.0, cw, 24.0, 12.0, p.accent);
            pen.text(&chip, cx + 8.0, y + 8.5, 11.0, ink_on(p.accent, p.bg_a), Face::Pixel, Align::Left, false);
        }
    }
    let brand = "SHINY TRACKER";
    let bw = pen.text(brand, x1, y + 8.0, 12.0, p.muted, Face::Pixel, Align::Right, false);
    if let Some(icon) = &d.icon {
        pen.blit_smooth(icon, x1 - bw - 38.0, y - 1.0, 28.0, 28.0);
    }
}

fn header(pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, align: Align) -> f32 {
    let p = pen.p;
    // Status chip. A hunt only counts as caught once the Caught! button
    // was pressed on it (it's then in the trophy case).
    let (chip, fill) = if d.caught { ("CAUGHT!", Col::hex(0x34b85a)) } else { ("IN PROGRESS", p.accent) };
    let icon_w = 22.0;
    let cw = pen.width(chip, 13.0, Face::Pixel) + 32.0 + icon_w;
    let cx = match align {
        Align::Center => x - cw / 2.0,
        _ => x,
    };
    let ink = ink_on(fill, p.bg_a);
    pen.rrect(cx, y, cw, 32.0, 16.0, fill);
    pen.rrect_stroke(cx, y, cw, 32.0, 16.0, 2.0, ink.alpha(0.22));
    let (ix, iy) = (cx + 23.0, y + 16.0);
    if d.caught {
        // tick
        pen.line(ix - 6.0, iy, ix - 2.0, iy + 4.5, 3.5, ink);
        pen.line(ix - 2.0, iy + 4.5, ix + 6.0, iy - 5.0, 3.5, ink);
    } else {
        // "live" dot; its halo pulses in the animated version
        let pulse = pen.phase.map_or(0.35, |ph| 0.5 + 0.5 * (ph * std::f32::consts::TAU * 2.0).sin());
        pen.circle(ix, iy, 5.0 + 4.0 * pulse, ink.alpha(0.35 * (1.0 - pulse) + 0.1));
        pen.circle(ix, iy, 4.5, ink);
    }
    pen.text(chip, cx + 16.0 + icon_w, y + 10.0, 13.0, ink, Face::Pixel, Align::Left, false);
    let mut y = y + 52.0;
    let title = if o.title.trim().is_empty() { auto_title(d) } else { o.title.trim().to_uppercase() };
    // Room for the ♂/♀ sign after the title.
    let sign = d.gender != Gender::None;
    let gap = |size: f32| if sign { size * 1.15 } else { 0.0 };
    let mut size = pen.fit(&title, 46.0, w, Face::Pixel);
    while sign && size > 10.0 && pen.width(&title, size, Face::Pixel) + gap(size) > w {
        size -= 1.0;
    }
    let tc = if d.caught { p.highlight } else { p.text };
    let tw = pen.width(&title, size, Face::Pixel);
    let tx = match align {
        Align::Center => x - gap(size) / 2.0,
        _ => x,
    };
    pen.text(&title, tx, y, size, tc, Face::Pixel, align, true);
    if sign {
        let left = match align {
            Align::Center => tx - tw / 2.0,
            Align::Right => tx - tw,
            Align::Left => tx,
        };
        gender_sign(pen, d.gender, left + tw + size * 0.6, y + size * 0.5, size * 0.42);
    }
    y += size + 20.0;
    let mut sub = Vec::new();
    // With a nickname, the species goes under it.
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
    if !sub.is_empty() {
        let s = sub.join("  ·  ");
        let fs = pen.fit(&s, 24.0, w, Face::Body);
        pen.text(&s, x, y, fs, p.muted, Face::Body, align, false);
        y += fs + 10.0;
    }
    if let (true, Some(note)) = (o.show_note, d.note.as_ref().filter(|n| !n.trim().is_empty())) {
        let mut q = format!("\u{201c}{}\u{201d}", note.trim());
        if q.chars().count() > 70 {
            q = format!("\u{201c}{}…\u{201d}", note.trim().chars().take(66).collect::<String>());
        }
        let fs = pen.fit(&q, 21.0, w, Face::Body);
        pen.text(&q, x, y, fs, p.highlight, Face::Body, align, false);
        y += fs + 8.0;
    }
    if !d.phases.is_empty() {
        let mut names: Vec<&str> = d.phases.iter().map(|s| s.as_str()).take(4).collect();
        let more = d.phases.len().saturating_sub(4);
        let extra = if more > 0 { format!(" +{more}") } else { String::new() };
        names.dedup();
        let t = format!(
            "{} {}{}",
            if d.phases.len() == 1 { "Phase:" } else { "Phases:" },
            names.join(", "),
            extra
        );
        let fs = pen.fit(&t, 20.0, w, Face::Body);
        pen.text(&t, x, y, fs, p.highlight, Face::Body, align, false);
        y += fs + 8.0;
    }
    y
}

fn wide(pen: &mut Pen, d: &CardData, o: &CardOptions) {
    let (w, h) = (pen.w, pen.h);
    let m = 44.0;
    let stage_w = 500.0;
    stage(pen, d, o, m, m, stage_w, h - 2.0 * m);
    let x0 = m + stage_w + 40.0;
    let cw = w - m - x0;
    let mut y = header(pen, d, o, x0, m + 6.0, cw, Align::Left);
    y += 14.0;
    y = encounters(pen, d, x0, y, cw, Align::Left);
    y += 26.0;
    if o.show_meter && d.odds_one_in.is_some() {
        y = meter(pen, d, x0, y, cw) + 24.0;
    }
    let t = tiles(d, o);
    let footer_y = h - m - 26.0;
    if y + 92.0 <= footer_y - 16.0 {
        tile_row(pen, &t, x0, y.max(footer_y - 16.0 - 92.0 - 6.0).min(footer_y - 16.0 - 92.0), cw, 3);
    }
    footer(pen, d, o, x0, w - m, footer_y);
}

fn square(pen: &mut Pen, d: &CardData, o: &CardOptions) {
    let (w, h) = (pen.w, pen.h);
    let m = 50.0;
    let cw = w - 2.0 * m;
    let y = header(pen, d, o, w / 2.0, m, cw, Align::Center);
    let stage_h = 400.0;
    stage(pen, d, o, m, y + 14.0, cw, stage_h);
    let mut y = y + 14.0 + stage_h + 30.0;
    y = encounters(pen, d, w / 2.0, y, cw, Align::Center);
    y += 26.0;
    if o.show_meter && d.odds_one_in.is_some() {
        y = meter(pen, d, m, y, cw) + 26.0;
    }
    let footer_y = h - m - 22.0;
    let t = tiles(d, o);
    if y + 92.0 <= footer_y - 16.0 {
        tile_row(pen, &t, m, y, cw, 4);
    }
    footer(pen, d, o, m, w - m, footer_y);
}


// ───────────────────────────── trainer profile card ─────────────────────────────

/// Everything on the trainer profile infographic.
#[derive(Clone, Default)]
pub struct ProfileData {
    pub name: String,
    pub level: u32,
    pub title: String,
    /// Badge metal: name, main colour, light colour.
    pub metal: (String, [u8; 3], [u8; 3]),
    pub xp_into: u64,
    pub xp_need: u64,
    pub total_xp: u64,
    pub max_level: bool,
    /// Eight (label, value) stat tiles.
    pub stats: Vec<(String, String)>,
    /// Caught shinies: sprite, name, encounters (newest first, up to 6).
    pub shinies: Vec<(Option<Img>, String, u64)>,
    /// Recent achievements: name, tier colour, Pokémon sprite (up to 6).
    pub achievements: Vec<(String, [u8; 3], Option<Img>)>,
    pub ach_count: (usize, usize),
    pub show_shinies: bool,
    pub show_achievements: bool,
    pub date: String,
    pub icon: Option<Img>,
}

fn rgb(c: [u8; 3]) -> Col {
    Col(c[0], c[1], c[2], 255)
}

/// The hexagon level badge.
fn badge(pen: &mut Pen, cx: f32, cy: f32, r: f32, d: &ProfileData) {
    let main = rgb(d.metal.1);
    let light = rgb(d.metal.2);
    let hex = |r: f32, dy: f32| -> Vec<(f32, f32)> {
        (0..6)
            .map(|i| {
                let a = std::f32::consts::FRAC_PI_6 + i as f32 * std::f32::consts::FRAC_PI_3;
                (cx + a.cos() * r, cy + dy + a.sin() * r)
            })
            .collect()
    };
    pen.glow(cx, cy, r * 1.9, light.alpha(0.35));
    pen.poly(&hex(r * 1.04, r * 0.08), Col(0, 0, 0, 110));
    pen.poly(&hex(r, 0.0), Col::hex(0x08080c));
    pen.poly(&hex(r * 0.93, 0.0), main);
    pen.poly(&hex(r * 0.76, 0.0), light);
    pen.poly(&hex(r * 0.70, 0.0), main.mix(Col::hex(0x000000), 0.28));
    // shine facet
    pen.poly(
        &[(cx - r * 0.62, cy - r * 0.30), (cx, cy - r * 0.70), (cx - r * 0.12, cy - r * 0.48)],
        light.alpha(0.55),
    );
    let n = d.level.to_string();
    let size = if n.len() >= 3 { r * 0.48 } else { r * 0.62 };
    let w = pen.width(&n, size, Face::Pixel);
    let ty = cy - size * 0.5 + r * 0.04;
    pen.text(&n, cx - w / 2.0, ty, size, Col::hex(0xffffff), Face::Pixel, Align::Left, true);
    pen.sparkle(cx + r * 0.72, cy - r * 0.62, r * 0.18, light);
    pen.sparkle(cx - r * 0.85, cy + r * 0.45, r * 0.10, light.alpha(0.8));
}

fn medal_c(pen: &mut Pen, cx: f32, cy: f32, r: f32, tier: [u8; 3]) {
    let c = rgb(tier);
    pen.circle(cx, cy + 2.0, r, Col(0, 0, 0, 90));
    pen.circle(cx, cy, r, Col::hex(0x08080c));
    pen.circle(cx, cy, r * 0.9, c);
    pen.ring(cx, cy, r * 0.66, r * 0.08, c.mix(Col::hex(0xffffff), 0.5));
    pen.sparkle(cx, cy, r * 0.45, Col::hex(0xffffff));
}

/// Pixel-art sprite fitted into a box, standing on its bottom edge.
fn sprite_in(pen: &mut Pen, img: &Img, x: f32, y: f32, w: f32, h: f32) {
    let sp = crop(img, Canvas::trim_box(img));
    let k = (w / sp.w as f32).min(h / sp.h as f32);
    let k = if k >= 1.0 { k.floor() } else { k };
    let (sw, sh) = (sp.w as f32 * k, sp.h as f32 * k);
    pen.blit_px(&sp, (x + (w - sw) / 2.0).round(), (y + h - sh).round(), k);
}

pub fn render_profile(d: &ProfileData, o: &CardOptions, t: &dyn TextDraw) -> Canvas {
    let (_, _, s) = o.output_size();
    let (dw, dh) = o.layout.size();
    let p = o.theme.palette();
    let mut pen = Pen { c: render_base(o), t, p, phase: None, s, w: dw as f32, h: dh as f32 };
    if o.style == CardStyle::Anime {
        anime::profile(&mut pen, d, o);
        return pen.c;
    }
    background_decor(&mut pen, o);
    match o.layout {
        CardLayout::Wide => profile_wide(&mut pen, d),
        CardLayout::Square => profile_square(&mut pen, d),
    }
    pen.c
}

/// Badge, name, title and XP bar inside a panel.
fn profile_identity(pen: &mut Pen, d: &ProfileData, x: f32, y: f32, w: f32, h: f32, vertical: bool) {
    let p = pen.p;
    pen.rrect(x + 6.0, y + 10.0, w, h, 30.0, p.shadow.alpha(0.6));
    pen.rrect_vgrad(x, y, w, h, 30.0, p.panel_a, p.panel_b);
    pen.rrect_stroke(x, y, w, h, 30.0, 3.0, rgb(d.metal.2).alpha(0.7));
    let name = if d.name.trim().is_empty() { "TRAINER".to_string() } else { d.name.trim().to_uppercase() };
    let (cx, cy, br, tx, ty, tw, align) = if vertical {
        (x + w / 2.0, y + 150.0, 96.0, x + w / 2.0, y + 290.0, w - 50.0, Align::Center)
    } else {
        (x + 150.0, y + h / 2.0, 100.0, x + 290.0, y + 60.0, w - 330.0, Align::Left)
    };
    badge(pen, cx, cy, br, d);
    // LEVEL chip under the badge
    let chip = format!("LEVEL {}", d.level);
    let cw = pen.width(&chip, 13.0, Face::Pixel) + 26.0;
    pen.rrect(cx - cw / 2.0, cy + br + 4.0, cw, 30.0, 15.0, p.accent);
    pen.text(&chip, cx, cy + br + 13.0, 13.0, ink_on(p.accent, p.bg_a), Face::Pixel, Align::Center, false);
    // name + title
    let ns = pen.fit(&name, 36.0, tw, Face::Pixel);
    pen.text(&name, tx, ty, ns, p.text, Face::Pixel, align, true);
    let mut yy = ty + ns + 18.0;
    pen.text(&d.title, tx, yy, 26.0, p.highlight, Face::Body, align, false);
    yy += 40.0;
    let metal = format!("{} badge  ·  {} / {} achievements", d.metal.0, d.ach_count.0, d.ach_count.1);
    let ms = pen.fit(&metal, 19.0, tw, Face::Body);
    pen.text(&metal, tx, yy, ms, p.muted, Face::Body, align, false);
    yy += 38.0;
    // XP bar
    let bw = tw.min(440.0);
    let bx = match align {
        Align::Center => tx - bw / 2.0,
        _ => tx,
    };
    let frac = if d.max_level { 1.0 } else { d.xp_into as f32 / d.xp_need.max(1) as f32 };
    pen.rrect(bx, yy, bw, 22.0, 11.0, p.shadow.alpha(0.7));
    pen.rrect_stroke(bx, yy, bw, 22.0, 11.0, 2.0, p.line);
    if frac > 0.0 {
        let fw = (bw * frac).max(22.0) - 6.0;
        let (a, b) = (p.accent, p.highlight);
        pen.rrect_with(bx + 3.0, yy + 3.0, fw, 16.0, 8.0, |px, _| a.mix(b, (px - bx) / bw));
    }
    yy += 32.0;
    let xp = if d.max_level {
        format!("{} XP  ·  MAX LEVEL", thousands(d.total_xp))
    } else {
        format!("{} / {} XP to level {}", thousands(d.xp_into), thousands(d.xp_need), d.level + 1)
    };
    let xs = pen.fit(&xp, 17.0, tw, Face::Body);
    pen.text(&xp, tx, yy, xs, p.muted, Face::Body, align, false);
}

fn stat_grid(pen: &mut Pen, stats: &[(String, String)], x: f32, y: f32, w: f32, per_row: usize, th: f32) -> f32 {
    let p = pen.p;
    let gap = 14.0;
    let tw = (w - gap * (per_row as f32 - 1.0)) / per_row as f32;
    let mut yy = y;
    for row in stats.chunks(per_row) {
        for (i, (label, value)) in row.iter().enumerate() {
            let tx = x + i as f32 * (tw + gap);
            pen.rrect(tx, yy, tw, th, 16.0, p.panel_b.alpha(0.85));
            pen.rrect_stroke(tx, yy, tw, th, 16.0, 2.0, p.line.alpha(0.8));
            let ls = pen.fit(label, 16.0, tw - 28.0, Face::Body);
            pen.text(label, tx + 14.0, yy + 12.0, ls, p.muted, Face::Body, Align::Left, false);
            let vs = pen.fit(value, 22.0, tw - 28.0, Face::Pixel);
            pen.text(value, tx + 14.0, yy + th - vs - 14.0, vs, p.text, Face::Pixel, Align::Left, false);
        }
        yy += th + gap;
    }
    yy - gap
}

fn section_label(pen: &mut Pen, text: &str, x: f32, y: f32) {
    let p = pen.p;
    pen.sparkle(x + 7.0, y + 7.0, 7.0, p.highlight);
    pen.text(text, x + 20.0, y, 14.0, p.muted, Face::Pixel, Align::Left, false);
}

fn showcase(pen: &mut Pen, d: &ProfileData, x: f32, y: f32, w: f32, h: f32) {
    let p = pen.p;
    section_label(pen, "SHINY SHOWCASE", x, y);
    let y = y + 26.0;
    if d.shinies.is_empty() {
        pen.rrect(x, y, w, h, 16.0, p.panel_b.alpha(0.6));
        pen.text("No shinies yet - the hunt continues!", x + w / 2.0, y + h / 2.0 - 11.0, 21.0, p.muted, Face::Body, Align::Center, false);
        return;
    }
    let n = 6;
    let gap = 12.0;
    let sw = (w - gap * (n as f32 - 1.0)) / n as f32;
    for (i, (img, name, enc)) in d.shinies.iter().take(n).enumerate() {
        let sx = x + i as f32 * (sw + gap);
        pen.rrect(sx, y, sw, h, 16.0, p.panel_a.alpha(0.9));
        pen.rrect_stroke(sx, y, sw, h, 16.0, 2.0, p.highlight.alpha(0.55));
        pen.glow(sx + sw / 2.0, y + h * 0.38, sw * 0.5, p.highlight.alpha(0.2));
        pen.ellipse(sx + sw / 2.0, y + h - 44.0, sw * 0.32, 7.0, p.platform);
        if let Some(img) = img {
            sprite_in(pen, img, sx + 8.0, y + 8.0, sw - 16.0, h - 54.0);
        }
        pen.sparkle(sx + sw - 14.0, y + 14.0, 7.0, p.highlight);
        let ns = pen.fit(name, 15.0, sw - 12.0, Face::Body);
        pen.text(name, sx + sw / 2.0, y + h - 34.0, ns, p.text, Face::Body, Align::Center, false);
        let e = thousands(*enc);
        let es = pen.fit(&e, 11.0, sw - 12.0, Face::Pixel);
        pen.text(&e, sx + sw / 2.0, y + h - 15.0, es, p.highlight, Face::Pixel, Align::Center, false);
    }
}

fn recent_achievements(pen: &mut Pen, d: &ProfileData, x: f32, y: f32, w: f32, h: f32) {
    let p = pen.p;
    section_label(pen, "RECENT ACHIEVEMENTS", x, y);
    let y = y + 26.0;
    if d.achievements.is_empty() {
        pen.rrect(x, y, w, h, 16.0, p.panel_b.alpha(0.6));
        pen.text("Achievements unlock as you hunt", x + w / 2.0, y + h / 2.0 - 10.0, 19.0, p.muted, Face::Body, Align::Center, false);
        return;
    }
    let n = 6;
    let gap = 12.0;
    let aw = (w - gap * (n as f32 - 1.0)) / n as f32;
    for (i, (name, tier, img)) in d.achievements.iter().take(n).enumerate() {
        let ax = x + i as f32 * (aw + gap);
        let tc = rgb(*tier);
        pen.rrect(ax, y, aw, h, 14.0, p.panel_b.alpha(0.85));
        pen.rrect_stroke(ax, y, aw, h, 14.0, 2.0, tc.alpha(0.8));
        if let Some(img) = img {
            sprite_in(pen, img, ax + 6.0, y + 4.0, aw - 12.0, h - 34.0);
        }
        medal_c(pen, ax + 16.0, y + 16.0, 10.0, *tier);
        let ns = pen.fit(name, 13.0, aw - 10.0, Face::Body);
        pen.text(name, ax + aw / 2.0, y + h - 22.0, ns, p.text, Face::Body, Align::Center, false);
    }
}

fn profile_footer(pen: &mut Pen, d: &ProfileData, x0: f32, x1: f32, y: f32) {
    let p = pen.p;
    pen.text(&d.date, x0, y + 8.0, 12.0, p.muted, Face::Pixel, Align::Left, false);
    let bw = pen.text("SHINY TRACKER", x1, y + 8.0, 12.0, p.muted, Face::Pixel, Align::Right, false);
    if let Some(icon) = &d.icon {
        pen.blit_smooth(icon, x1 - bw - 38.0, y - 1.0, 28.0, 28.0);
    }
}

fn profile_wide(pen: &mut Pen, d: &ProfileData) {
    let (w, h) = (pen.w, pen.h);
    let m = 40.0;
    let lw = 360.0;
    profile_identity(pen, d, m, m, lw, h - 2.0 * m, true);
    let x0 = m + lw + 32.0;
    let cw = w - m - x0;
    let p = pen.p;
    let chip = "TRAINER CARD";
    let chw = pen.width(chip, 13.0, Face::Pixel) + 32.0;
    pen.rrect(x0, m, chw, 32.0, 16.0, p.accent);
    pen.text(chip, x0 + 16.0, m + 10.0, 13.0, ink_on(p.accent, p.bg_a), Face::Pixel, Align::Left, false);
    let mut y = stat_grid(pen, &d.stats, x0, m + 46.0, cw, 4, 74.0) + 22.0;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { 136.0 } else { 220.0 };
        showcase(pen, d, x0, y, cw, sh);
        y += 26.0 + sh + 18.0;
    }
    if d.show_achievements {
        let ah = if both { 112.0 } else { 180.0 };
        recent_achievements(pen, d, x0, y, cw, ah);
    }
    profile_footer(pen, d, x0, w - m, h - m - 20.0);
}

fn profile_square(pen: &mut Pen, d: &ProfileData) {
    let (w, h) = (pen.w, pen.h);
    let m = 46.0;
    let cw = w - 2.0 * m;
    profile_identity(pen, d, m, m, cw, 290.0, false);
    let mut y = m + 290.0 + 26.0;
    y = stat_grid(pen, &d.stats, m, y, cw, 4, 84.0) + 26.0;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { 168.0 } else { 300.0 };
        showcase(pen, d, m, y, cw, sh);
        y += 26.0 + sh + 22.0;
    }
    if d.show_achievements {
        let ah = if both { 150.0 } else { 260.0 };
        recent_achievements(pen, d, m, y, cw, ah);
    }
    profile_footer(pen, d, m, w - m, h - m - 20.0);
}
