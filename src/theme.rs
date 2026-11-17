//! Colours and drawing helpers styled after the game's battle UI:
//! dark slanted name plates, white text with black outlines, HP-style bars,
//! red command-button accents and gold "shiny" highlights.

use crate::fonts::pixel_font;
use eframe::egui::{self, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, Visuals};

pub const OUTLINE: Color32 = Color32::from_rgb(8, 8, 12);
pub const GOLD: Color32 = Color32::from_rgb(246, 200, 66);
pub const GREEN: Color32 = Color32::from_rgb(56, 214, 90);
pub const HP_YELLOW: Color32 = Color32::from_rgb(242, 194, 48);
pub const HP_RED: Color32 = Color32::from_rgb(232, 72, 63);
pub const GRASS: Color32 = Color32::from_rgb(38, 104, 108);
pub const GRASS_EDGE: Color32 = Color32::from_rgb(120, 118, 140);
pub const REGION: Color32 = Color32::from_rgb(80, 200, 255);

// ───────────────────────── live palette ─────────────────────────
// Colours that follow the user's Appearance settings (light / semi-dark /
// dark, accent colour or gradient). Read through the functions below.

#[derive(Clone, Copy)]
pub struct Pal {
    pub light: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub plate: Color32,
    pub border: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_dark: Color32,
    pub accent2: Color32,
    pub gradient: bool,
    pub button: Color32,
    pub button2: Color32,
    pub sidebar: Color32,
    pub page: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub line: Color32,
    pub selected_bg: Color32,
    pub text: Color32,
    pub widget: Color32,
    pub widget_hover: Color32,
    pub extreme: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

const DARK: Pal = Pal {
    light: false,
    bg: rgb(30, 27, 48),
    panel: rgb(18, 17, 26),
    plate: rgb(40, 37, 58),
    border: rgb(92, 88, 116),
    muted: rgb(168, 164, 192),
    accent: rgb(214, 52, 58),
    accent_dark: rgb(140, 28, 34),
    accent2: rgb(214, 52, 58),
    gradient: false,
    button: rgb(214, 52, 58),
    button2: rgb(214, 52, 58),
    sidebar: rgb(14, 13, 21),
    page: rgb(20, 19, 30),
    card: rgb(30, 28, 45),
    card_hover: rgb(38, 35, 56),
    line: rgb(58, 55, 80),
    selected_bg: rgb(64, 24, 32),
    text: rgb(236, 234, 245),
    widget: rgb(46, 43, 68),
    widget_hover: rgb(66, 61, 96),
    extreme: rgb(10, 10, 16),
};

const SEMI: Pal = Pal {
    light: false,
    bg: rgb(54, 51, 76),
    panel: rgb(42, 40, 60),
    plate: rgb(66, 62, 92),
    border: rgb(116, 112, 146),
    muted: rgb(196, 192, 216),
    sidebar: rgb(40, 38, 58),
    page: rgb(50, 48, 70),
    card: rgb(62, 59, 86),
    card_hover: rgb(72, 69, 99),
    line: rgb(92, 89, 120),
    text: rgb(244, 243, 250),
    widget: rgb(76, 72, 104),
    widget_hover: rgb(92, 88, 124),
    extreme: rgb(34, 32, 48),
    ..DARK
};

const LIGHT: Pal = Pal {
    light: true,
    bg: rgb(246, 244, 251),
    panel: rgb(232, 229, 242),
    plate: rgb(66, 62, 96),
    border: rgb(196, 192, 216),
    muted: rgb(102, 98, 128),
    sidebar: rgb(236, 233, 245),
    page: rgb(249, 248, 253),
    card: rgb(255, 255, 255),
    card_hover: rgb(241, 239, 249),
    line: rgb(216, 212, 230),
    text: rgb(30, 28, 44),
    widget: rgb(232, 229, 242),
    widget_hover: rgb(220, 216, 236),
    extreme: rgb(255, 255, 255),
    ..DARK
};

static PAL: std::sync::RwLock<Pal> = std::sync::RwLock::new(DARK);

// ───────────────────── clean settings style (the default) ─────────────────────
// A flat look for the settings window only (the overlay keeps `Pal`):
// neutral greys instead of the purple-tinted ones, the user's accent used
// sparingly. Switched by Appearance → Style (Clean / Classic).

static CLEAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// True when the settings window uses the clean style.
pub fn clean() -> bool {
    CLEAN.load(std::sync::atomic::Ordering::Relaxed)
}

/// Neutral surface colours for the clean settings window.
#[derive(Clone, Copy)]
pub struct Surf {
    /// Sidebar (darkest in dark modes).
    pub sidebar: Color32,
    /// Page background.
    pub page: Color32,
    /// Cards and tiles.
    pub card: Color32,
    /// Hover / quiet fills inside cards.
    pub card_hover: Color32,
    /// Hairline borders.
    pub line: Color32,
    /// Outline of controls (buttons, inputs, segmented groups).
    pub control: Color32,
    /// Selected row / segment fill.
    pub selected: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// Text box background.
    pub field: Color32,
}

const CLEAN_DARK: Surf = Surf {
    sidebar: rgb(28, 29, 32),
    page: rgb(36, 37, 41),
    card: rgb(43, 44, 49),
    card_hover: rgb(51, 52, 58),
    line: rgb(56, 58, 64),
    control: rgb(78, 80, 88),
    selected: rgb(58, 60, 67),
    text: rgb(232, 233, 236),
    muted: rgb(152, 155, 162),
    field: rgb(28, 29, 32),
};

const CLEAN_SEMI: Surf = Surf {
    sidebar: rgb(43, 45, 49),
    page: rgb(49, 51, 56),
    card: rgb(56, 58, 64),
    card_hover: rgb(64, 66, 73),
    line: rgb(70, 72, 79),
    control: rgb(92, 95, 104),
    selected: rgb(72, 75, 83),
    text: rgb(238, 239, 242),
    muted: rgb(168, 172, 180),
    field: rgb(38, 40, 44),
};

const CLEAN_LIGHT: Surf = Surf {
    sidebar: rgb(250, 250, 251),
    page: rgb(244, 244, 246),
    card: rgb(255, 255, 255),
    card_hover: rgb(240, 240, 243),
    line: rgb(224, 225, 229),
    control: rgb(196, 198, 204),
    selected: rgb(232, 233, 237),
    text: rgb(32, 33, 37),
    muted: rgb(104, 107, 115),
    field: rgb(255, 255, 255),
};

static SURF: std::sync::RwLock<Surf> = std::sync::RwLock::new(MODERN_DARK);

// ───────────────────── modern style (the default since 0.5) ─────────────────────
// Deep ink surfaces with a hint of blue, soft cards with shadows, colour-coded
// icon chips and the user's accent (or gradient) on primary actions. Built on
// the clean style's rules (system font, nothing under 12px), so every page
// that follows those picks the modern look up too. Appearance → Style.

static MODERN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static MOTION: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static PLUS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Modern Plus (Appearance → Look): the review's changes on top of Modern.
pub fn plus() -> bool {
    PLUS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Modern Plus's one action colour: the Brand Kit violet.
pub const PLUS_VIOLET: Color32 = Color32::from_rgb(139, 92, 246);

/// Are transitions and animations on? (Appearance → Animations.)
pub fn motion() -> bool {
    MOTION.load(std::sync::atomic::Ordering::Relaxed)
}

/// True when the Companion window uses the modern style.
pub fn modern() -> bool {
    MODERN.load(std::sync::atomic::Ordering::Relaxed)
}

const MODERN_DARK: Surf = Surf {
    sidebar: rgb(13, 15, 22),
    page: rgb(18, 20, 29),
    card: rgb(26, 29, 41),
    card_hover: rgb(33, 37, 52),
    line: rgb(39, 43, 59),
    control: rgb(60, 65, 88),
    selected: rgb(41, 45, 64),
    text: rgb(236, 238, 245),
    muted: rgb(144, 150, 172),
    field: rgb(14, 16, 24),
};

const MODERN_SEMI: Surf = Surf {
    sidebar: rgb(31, 34, 48),
    page: rgb(38, 41, 57),
    card: rgb(47, 51, 70),
    card_hover: rgb(55, 59, 80),
    line: rgb(63, 68, 91),
    control: rgb(86, 92, 118),
    selected: rgb(63, 68, 93),
    text: rgb(242, 243, 248),
    muted: rgb(172, 178, 198),
    field: rgb(31, 34, 48),
};

/// Light mode follows the Cirrus tokens: Sky page, Cloud cards and fields,
/// Edge hairlines, Sunken hovers, Ink text and Mist secondary text.
const MODERN_LIGHT: Surf = Surf {
    sidebar: rgb(255, 255, 255),
    page: rgb(237, 242, 247),
    card: rgb(255, 255, 255),
    card_hover: rgb(243, 246, 250),
    line: rgb(227, 232, 238),
    control: rgb(200, 207, 217),
    selected: rgb(236, 240, 245),
    text: rgb(14, 17, 22),
    muted: rgb(91, 100, 114),
    field: rgb(255, 255, 255),
};

/// The clean style's surface colours for the current mode.
pub fn surf() -> Surf {
    *SURF.read().unwrap_or_else(|e| e.into_inner())
}

/// Blend two colours (t = 0 → a, 1 → b).
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    mix(a, b, t)
}

pub fn pal() -> Pal {
    *PAL.read().unwrap_or_else(|e| e.into_inner())
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Apply the user's Appearance to the whole app (overlay and settings).
pub fn set_appearance(ctx: &egui::Context, a: &crate::config::Appearance) {
    use crate::config::UiMode;
    let mut p = match a.mode {
        UiMode::Dark => DARK,
        UiMode::SemiDark => SEMI,
        UiMode::Light => LIGHT,
    };
    let acc = Color32::from_rgb(a.accent[0], a.accent[1], a.accent[2]);
    p.accent = acc;
    p.accent_dark = mix(acc, Color32::BLACK, 0.35);
    p.gradient = a.gradient;
    p.accent2 = if a.gradient {
        Color32::from_rgb(a.accent2[0], a.accent2[1], a.accent2[2])
    } else {
        acc
    };
    p.selected_bg = mix(p.card, acc, if p.light { 0.16 } else { 0.32 });
    let c3 = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    if a.separate_buttons {
        p.button = c3(a.button);
        p.button2 = if a.button_gradient { c3(a.button2) } else { p.button };
    } else {
        p.button = p.accent;
        p.button2 = if a.solid_buttons { p.accent } else { p.accent2 };
    }
    // Modern Plus: every main action in one solid colour: your button
    // colour (your accent, unless you picked a separate one), no gradient
    if a.plus {
        p.button2 = p.button;
    }
    // A Modern Plus theme brings its own surfaces and colours.
    let th = active_theme(a);
    if let Some(t) = th {
        let s = t.s;
        // (Glass HUD and Broadcast keep your own colour, like their overlays)
        let (own_acc, own_btn) = (p.accent, p.button);
        p = if t.light { LIGHT } else { DARK };
        p.light = t.light;
        p.bg = s.page;
        p.panel = s.page;
        p.sidebar = s.sidebar;
        p.page = s.page;
        p.card = s.card;
        p.card_hover = s.card_hover;
        p.line = s.line;
        p.border = s.control;
        p.text = s.text;
        p.muted = s.muted;
        p.widget = s.card_hover;
        p.widget_hover = mix(s.card_hover, s.text, 0.10);
        p.extreme = s.field;
        let (acc, btn) = if t.own_accent { (own_acc, own_btn) } else { (t.accent, t.button) };
        p.accent = acc;
        p.accent_dark = mix(acc, Color32::BLACK, 0.35);
        p.accent2 = acc;
        p.gradient = false;
        p.button = btn;
        p.button2 = btn;
        p.selected_bg = if t.own_accent { mix(s.card, acc, 0.3) } else { s.selected };
    }
    if let Ok(mut w) = THEME.write() {
        *w = th;
    }
    // (your colours' hue turn, for the touches drawn outside the theme)
    let turn = match (th, theme_def(a.plus_theme)) {
        (Some(_), Some(d)) if a.plus && a.theme_colours && !d.own_accent => {
            let base = if a.theme_flip { flip(d) } else { d };
            let c = a.accent;
            let d = to_hsl(Color32::from_rgb(c[0], c[1], c[2])).0 - to_hsl(base.accent).0;
            if d == 0.0 { 0.0001 } else { d }
        }
        _ => 0.0,
    };
    HUE_TURN.store(f32::to_bits(turn), std::sync::atomic::Ordering::Relaxed);
    PARTICLES_ON.store(a.particles, std::sync::atomic::Ordering::Relaxed);
    TRANSITIONS_ON.store(a.theme_transitions, std::sync::atomic::Ordering::Relaxed);
    // the pixel themes: the pixel face on everything, for now (a look test)
    // (and Field journal's handwriting on everything, like its overlay)
    let face = match th.map(|t| t.skin) {
        Some(Skin::Pixel) if a.plus => Some("pixelify"),
        Some(Skin::Journal) if a.plus => Some("ov-hand"),
        _ => None,
    };
    crate::fonts::set_face_everywhere(ctx, face);
    PLUS.store(a.plus, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut w) = PAL.write() {
        *w = p;
    }
    // only the Modern look now
    MODERN.store(true, std::sync::atomic::Ordering::Relaxed);
    MOTION.store(a.animations, std::sync::atomic::Ordering::Relaxed);
    CLEAN.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut w) = SURF.write() {
        if let Some(t) = th {
            *w = t.s;
            drop(w);
            apply(ctx);
            return;
        }
        *w = match (true, a.mode) {
            (true, UiMode::Dark) => MODERN_DARK,
            (true, UiMode::SemiDark) => MODERN_SEMI,
            (true, UiMode::Light) => MODERN_LIGHT,
            (false, UiMode::Dark) => CLEAN_DARK,
            (false, UiMode::SemiDark) => CLEAN_SEMI,
            (false, UiMode::Light) => CLEAN_LIGHT,
        };
    }
    apply(ctx);
}

// ───────────────────── Modern Plus themes (Appearance → Theme) ─────────────────────
// Ready-made looks from the UI Lab. Each brings its own surfaces, light or
// dark, and action colour; "Your colours" (the default) keeps your own mode
// and colours. A theme only applies with Modern Plus on.

use crate::config::PlusTheme;

/// A Modern Plus theme: everything it changes.
#[derive(Clone, Copy)]
pub struct ThemeDef {
    pub name: &'static str,
    /// One line for the picker.
    pub blurb: &'static str,
    pub light: bool,
    pub s: Surf,
    /// Selected things, progress, the top hairline.
    pub accent: Color32,
    /// The theme's second colour (small touches only).
    pub accent2: Color32,
    /// Every main button.
    pub button: Color32,
    /// The title bar's own fill and ink, when it isn't the page colour.
    pub chrome: Option<(Color32, Color32)>,
    /// Soft glows behind the page (top left, top right).
    pub glow: Option<(Color32, Color32)>,
    /// See-through cards with a light rim (over a glow or backdrop).
    pub glass: bool,
    /// A top-to-bottom wash behind the page, instead of one flat colour.
    pub backdrop: Option<(Color32, Color32)>,
    /// How round corners are: 1 = Modern Plus's own, less = squarer.
    pub round: f32,
    /// Pixel style: pixel key buttons (hard edges, a bevel and a drop shadow
    /// that collapses when pressed, pixel type) and 8×8 pixel app icons.
    pub pixel: bool,
    /// Cards cast a hard, unblurred shadow in this colour (Brutal, Pocket).
    pub hard_shadow: Option<Color32>,
    /// The face for page titles and big numbers (the overlay themes' own:
    /// handwriting, a console font, Impact; the game themes' pixel face).
    pub font: Option<fn(f32) -> egui::FontId>,
    /// Buttons' labels in the pixel face too (the game themes only).
    pub pixel_font: bool,
    /// Keeps your own accent and button colour (Glass HUD, Broadcast: like
    /// their overlays, which take your colour).
    pub own_accent: bool,
    /// Little things drifting behind the page (Appearance can turn them off).
    pub particles: Option<Particles>,
    /// How every control is drawn: buttons, fields, tabs, cards, the page
    /// behind them (the overlay themes' and the pixel themes' own).
    pub skin: Skin,
    /// The face for buttons and tabs (when it reads well that small).
    pub label: Option<fn(f32) -> egui::FontId>,
}

/// A theme's way of drawing every control, to match its overlay theme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skin {
    /// Modern Plus's own controls.
    Plain,
    /// Pokédex, Pocket, Retro DS: everything square, pixel keys, the pixel face.
    Pixel,
    /// Chunky sticker buttons, ink outlines, hard shadows, comic type.
    Sticker,
    /// Handwriting, red stamps, a ruled notebook page.
    Journal,
    /// Slanted scoreboard plates, Impact, a colour stripe on each card.
    Broadcast,
    /// Console type, square boxes, green borders, scanlines.
    Terminal,
    /// See-through rounded ghost buttons and fields.
    Glass,
    // ── the ready-made themes' own personalities
    /// Soft pink pills with a glow.
    Companion,
    /// Violet-to-orange gradient pills, a violet glow.
    Studio,
    /// Teal-to-violet glass.
    Aurora,
    /// A field guide: graph paper, teal labels, a book serif.
    Field,
    /// Neon: glowing gradient buttons, a retro grid on the horizon.
    Synth,
    /// Gold leaf: gilded buttons with a fine border, glints.
    Gold,
    /// Glossy candy pills, confetti.
    Candy,
    /// Black outlines, hard shadows, square everything.
    Brutal,
    /// Glossy liquid buttons over frosted glass.
    Liquid,
    /// Ice: frosted buttons with a snow cap, drifts at the bottom.
    Winter,
    /// Pumpkin buttons that drip, a cobweb in the corner.
    Spooky,
}

/// What drifts behind the page.
#[derive(Clone, Copy, PartialEq)]
pub enum Particles {
    /// Winter: snow falling slowly, swaying.
    Snow,
    /// Spooky: orange embers rising.
    Embers,
}

/// Surfaces from the UI Lab's tokens: chrome (title bar and fields), page,
/// panel (cards), panel2 (hovers), line, text, muted.
const fn surf_of(chrome: Color32, page: Color32, panel: Color32, panel2: Color32, line: Color32, control: Color32, selected: Color32, text: Color32, muted: Color32) -> Surf {
    Surf { sidebar: chrome, page, card: panel, card_hover: panel2, line, control, selected, text, muted, field: chrome }
}

const STUDIO: ThemeDef = ThemeDef {
    name: "Studio",
    blurb: "Ink black with a violet glow and an orange spark.",
    light: false,
    s: surf_of(rgb(10, 10, 15), rgb(13, 13, 18), rgb(25, 24, 35), rgb(33, 32, 46), rgb(48, 46, 66), rgb(70, 68, 94), rgb(42, 33, 72), rgb(236, 235, 245), rgb(156, 154, 179)),
    accent: rgb(139, 92, 246),
    accent2: rgb(249, 115, 22),
    // (a touch deeper than the accent, so white text on it reads well)
    button: rgb(130, 80, 240),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::bricolage_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Studio,
    label: Some(crate::fonts::bricolage_font),
};

const POKEDEX: ThemeDef = ThemeDef {
    name: "Pokédex",
    blurb: "A red title bar, sky-blue actions and a yellow tab line.",
    light: false,
    s: surf_of(rgb(22, 23, 28), rgb(27, 28, 34), rgb(37, 38, 46), rgb(48, 49, 59), rgb(58, 59, 70), rgb(84, 86, 100), rgb(30, 56, 74), rgb(243, 245, 249), rgb(163, 168, 184)),
    accent: rgb(56, 189, 248),
    accent2: rgb(250, 204, 21),
    button: rgb(56, 189, 248),
    chrome: Some((rgb(196, 32, 44), rgb(255, 255, 255))),
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: true,
    hard_shadow: Some(rgb(8, 8, 12)),
    font: Some(crate::fonts::pixel_ui_font),
    pixel_font: true,
    own_accent: false,
    particles: None,
    skin: Skin::Pixel,
    label: None,
};

const AURORA: ThemeDef = ThemeDef {
    name: "Aurora glass",
    blurb: "Deep navy, a teal and violet glow, see-through cards.",
    light: false,
    s: surf_of(rgb(14, 18, 36), rgb(7, 11, 28), rgb(24, 28, 46), rgb(34, 38, 58), rgb(47, 51, 72), rgb(72, 78, 108), rgb(28, 54, 70), rgb(238, 242, 255), rgb(167, 176, 214)),
    accent: rgb(94, 234, 212),
    accent2: rgb(167, 139, 250),
    button: rgb(94, 234, 212),
    chrome: None,
    glow: Some((rgb(94, 234, 212), rgb(167, 139, 250))),
    glass: true,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::jura_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Aurora,
    label: Some(crate::fonts::jura_font),
};

const FIELD: ThemeDef = ThemeDef {
    name: "Field guide",
    blurb: "Light paper, deep teal and a rust red pen.",
    light: true,
    s: surf_of(rgb(223, 230, 234), rgb(233, 238, 241), rgb(251, 252, 252), rgb(241, 245, 247), rgb(202, 213, 219), rgb(170, 184, 192), rgb(214, 231, 227), rgb(27, 42, 51), rgb(91, 108, 119)),
    accent: rgb(15, 118, 110),
    accent2: rgb(194, 65, 12),
    button: rgb(15, 118, 110),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::young_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Field,
    label: Some(crate::fonts::young_font),
};

const SYNTH: ThemeDef = ThemeDef {
    name: "Synthwave",
    blurb: "Neon cyan and pink on a deep purple night.",
    light: false,
    s: surf_of(rgb(10, 8, 38), rgb(6, 5, 26), rgb(22, 18, 56), rgb(31, 26, 72), rgb(52, 44, 112), rgb(76, 66, 146), rgb(20, 46, 80), rgb(230, 247, 255), rgb(150, 147, 204)),
    accent: rgb(34, 211, 238),
    accent2: rgb(244, 114, 182),
    button: rgb(34, 211, 238),
    chrome: None,
    glow: None,
    glass: true,
    backdrop: Some((rgb(6, 5, 26), rgb(42, 11, 61))),
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::tektur_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Synth,
    label: Some(crate::fonts::tektur_font),
};

const GOLD_T: ThemeDef = ThemeDef {
    name: "Gold",
    blurb: "Black and gold, like a shiny's sparkle.",
    light: false,
    s: surf_of(rgb(8, 7, 5), rgb(11, 10, 7), rgb(26, 23, 16), rgb(35, 31, 21), rgb(64, 55, 36), rgb(96, 83, 56), rgb(56, 45, 18), rgb(246, 238, 216), rgb(176, 162, 128)),
    accent: rgb(242, 193, 78),
    accent2: rgb(255, 240, 179),
    button: rgb(242, 193, 78),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::gloock_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Gold,
    label: Some(crate::fonts::gloock_font),
};

const COMPANION: ThemeDef = ThemeDef {
    name: "Companion",
    blurb: "The Companion's own look: soft pink on deep slate.",
    light: false,
    s: surf_of(rgb(25, 26, 34), rgb(28, 29, 38), rgb(38, 39, 52), rgb(46, 47, 62), rgb(56, 58, 74), rgb(82, 84, 104), rgb(70, 44, 74), rgb(238, 238, 246), rgb(160, 163, 184)),
    accent: rgb(243, 166, 232),
    accent2: rgb(139, 106, 216),
    button: rgb(243, 166, 232),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::outfit_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Companion,
    label: Some(crate::fonts::outfit_font),
};

const POCKET: ThemeDef = ThemeDef {
    name: "Pocket",
    blurb: "Four greens, pixel buttons and icons, hard shadows. Like an old handheld.",
    light: true,
    s: surf_of(rgb(139, 172, 15), rgb(139, 172, 15), rgb(155, 188, 15), rgb(169, 201, 35), rgb(48, 98, 48), rgb(48, 98, 48), rgb(184, 212, 74), rgb(15, 56, 15), rgb(20, 60, 20)),
    // (the darkest greens for text and accents: the real palette's middle
    // greens are too faint to read on the screen colour)
    accent: rgb(22, 64, 22),
    // (the open tab's line on the dark green title bar)
    accent2: rgb(155, 188, 15),
    button: rgb(48, 98, 48),
    chrome: Some((rgb(15, 56, 15), rgb(155, 188, 15))),
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: true,
    hard_shadow: Some(rgb(48, 98, 48)),
    font: Some(crate::fonts::pixel_ui_font),
    pixel_font: true,
    own_accent: false,
    particles: None,
    skin: Skin::Pixel,
    label: None,
};

const CANDY: ThemeDef = ThemeDef {
    name: "Candy",
    blurb: "Soft pink, extra round and playful.",
    light: true,
    s: surf_of(rgb(255, 232, 245), rgb(255, 243, 250), rgb(255, 255, 255), rgb(255, 246, 251), rgb(246, 207, 228), rgb(226, 170, 204), rgb(253, 221, 238), rgb(58, 29, 57), rgb(124, 90, 120)),
    accent: rgb(219, 39, 119),
    accent2: rgb(139, 92, 246),
    button: rgb(219, 39, 119),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.5,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::erica_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Candy,
    label: Some(crate::fonts::erica_font),
};

const BRUTAL: ThemeDef = ThemeDef {
    name: "Brutal",
    blurb: "Cream paper, black lines, square corners and hard shadows.",
    light: true,
    s: surf_of(rgb(255, 233, 168), rgb(255, 244, 214), rgb(255, 255, 255), rgb(255, 249, 234), rgb(17, 17, 17), rgb(17, 17, 17), rgb(255, 222, 232), rgb(17, 17, 17), rgb(74, 74, 74)),
    accent: rgb(225, 29, 92),
    accent2: rgb(61, 214, 198),
    button: rgb(255, 92, 138),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: false,
    hard_shadow: Some(rgb(17, 17, 17)),
    font: Some(crate::fonts::boldonse_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Brutal,
    label: Some(crate::fonts::boldonse_font),
};

const LIQUID: ThemeDef = ThemeDef {
    name: "Liquid",
    blurb: "Frosted glass over a periwinkle sky, very round.",
    light: true,
    s: surf_of(rgb(184, 201, 255), rgb(159, 183, 255), rgb(205, 218, 255), rgb(220, 229, 255), rgb(238, 243, 255), rgb(120, 140, 210), rgb(190, 205, 255), rgb(13, 18, 48), rgb(44, 53, 102)),
    accent: rgb(30, 64, 175),
    accent2: rgb(219, 39, 119),
    button: rgb(37, 99, 235),
    chrome: None,
    glow: None,
    glass: true,
    backdrop: Some((rgb(159, 183, 255), rgb(200, 214, 255))),
    round: 1.6,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::instrument_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Liquid,
    label: Some(crate::fonts::instrument_font),
};

const WINTER: ThemeDef = ThemeDef {
    name: "Winter",
    blurb: "Ice blue on a deep winter night, with a red berry.",
    light: false,
    s: surf_of(rgb(9, 19, 34), rgb(11, 22, 38), rgb(18, 34, 58), rgb(24, 43, 71), rgb(36, 64, 106), rgb(58, 88, 132), rgb(24, 52, 78), rgb(234, 244, 255), rgb(157, 182, 214)),
    accent: rgb(125, 211, 252),
    accent2: rgb(244, 63, 94),
    button: rgb(125, 211, 252),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.15,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::italiana_font),
    pixel_font: false,
    own_accent: false,
    particles: Some(Particles::Snow),
    skin: Skin::Winter,
    label: None,
};

const SPOOKY: ThemeDef = ThemeDef {
    name: "Spooky",
    blurb: "Pumpkin orange and slime green in a haunted purple.",
    light: false,
    s: surf_of(rgb(16, 9, 23), rgb(18, 10, 26), rgb(32, 19, 45), rgb(41, 24, 54), rgb(59, 35, 80), rgb(86, 58, 112), rgb(58, 30, 30), rgb(243, 232, 255), rgb(180, 156, 200)),
    accent: rgb(249, 115, 22),
    accent2: rgb(163, 230, 53),
    button: rgb(249, 115, 22),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::scrawl_font),
    pixel_font: false,
    own_accent: false,
    particles: Some(Particles::Embers),
    skin: Skin::Spooky,
    label: None,
};

// ── Matching the overlay themes (Appearance → Theme, "Match your overlay")

const STICKER: ThemeDef = ThemeDef {
    name: "Sticker",
    blurb: "Like the Sticker overlay: cream paper, thick ink lines, hard shadows and sticker-yellow buttons.",
    light: true,
    s: surf_of(rgb(255, 104, 154), rgb(255, 247, 232), rgb(255, 255, 255), rgb(255, 250, 240), rgb(28, 24, 48), rgb(28, 24, 48), rgb(255, 226, 236), rgb(28, 24, 48), rgb(92, 84, 104)),
    accent: rgb(214, 40, 104),
    accent2: rgb(60, 203, 232),
    button: rgb(255, 210, 63),
    chrome: Some((rgb(255, 104, 154), rgb(28, 24, 48))),
    glow: None,
    glass: false,
    backdrop: None,
    round: 1.0,
    pixel: false,
    hard_shadow: Some(rgb(28, 24, 48)),
    font: Some(crate::fonts::black_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Sticker,
    label: None,
};

const GLASS_HUD: ThemeDef = ThemeDef {
    name: "Glass HUD",
    blurb: "Like the Glass HUD overlay: frosted see-through panels, light numbers, and your own colour.",
    light: false,
    s: surf_of(rgb(16, 18, 28), rgb(14, 16, 24), rgb(34, 38, 54), rgb(44, 49, 68), rgb(62, 68, 92), rgb(90, 98, 126), rgb(44, 50, 74), rgb(255, 255, 255), rgb(170, 176, 196)),
    accent: rgb(139, 92, 246),
    accent2: rgb(255, 255, 255),
    button: rgb(139, 92, 246),
    chrome: None,
    glow: None,
    glass: true,
    backdrop: Some((rgb(40, 45, 62), rgb(10, 12, 18))),
    round: 1.4,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::light_font),
    pixel_font: false,
    own_accent: true,
    particles: None,
    skin: Skin::Glass,
    label: None,
};

const RETRO_DS: ThemeDef = ThemeDef {
    name: "Retro DS",
    blurb: "Like the Retro DS overlay: a grey handheld shell, cream battle boxes, pixel type and icons.",
    light: true,
    s: surf_of(rgb(214, 214, 198), rgb(226, 226, 212), rgb(248, 248, 232), rgb(238, 238, 220), rgb(72, 72, 80), rgb(72, 72, 80), rgb(214, 232, 248), rgb(48, 48, 56), rgb(96, 96, 108)),
    accent: rgb(178, 46, 32),
    accent2: rgb(248, 216, 56),
    button: rgb(28, 104, 192),
    chrome: Some((rgb(52, 55, 66), rgb(248, 248, 232))),
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: true,
    hard_shadow: Some(rgb(72, 72, 80)),
    font: Some(crate::fonts::pixel_ui_font),
    pixel_font: true,
    own_accent: false,
    particles: None,
    skin: Skin::Pixel,
    label: None,
};

const JOURNAL_T: ThemeDef = ThemeDef {
    name: "Field journal",
    blurb: "Like the Field journal overlay: notebook paper, blue pen, a red stamp, handwritten headings.",
    light: true,
    s: surf_of(rgb(240, 233, 214), rgb(251, 246, 233), rgb(255, 252, 243), rgb(246, 240, 224), rgb(196, 214, 236), rgb(150, 176, 210), rgb(255, 238, 168), rgb(32, 44, 96), rgb(90, 90, 100)),
    accent: rgb(190, 44, 44),
    accent2: rgb(232, 128, 128),
    button: rgb(196, 46, 46),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.45,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::hand_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Journal,
    label: None,
};

const BROADCAST_T: ThemeDef = ThemeDef {
    name: "Broadcast",
    blurb: "Like the Broadcast overlay: a scoreboard's dark plates, big Impact numbers, your colour and a LIVE red.",
    light: false,
    s: surf_of(rgb(8, 10, 18), rgb(12, 14, 24), rgb(28, 32, 50), rgb(36, 41, 62), rgb(50, 56, 84), rgb(76, 84, 116), rgb(48, 26, 38), rgb(255, 255, 255), rgb(164, 172, 200)),
    accent: rgb(232, 36, 52),
    accent2: rgb(232, 36, 52),
    button: rgb(232, 36, 52),
    chrome: None,
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::impact_font),
    pixel_font: false,
    own_accent: true,
    particles: None,
    skin: Skin::Broadcast,
    label: None,
};

const TERMINAL_T: ThemeDef = ThemeDef {
    name: "Terminal",
    blurb: "Like the Terminal overlay: a green console on black, with an amber highlight and console type.",
    light: false,
    s: surf_of(rgb(24, 30, 28), rgb(8, 12, 10), rgb(14, 20, 17), rgb(20, 30, 24), rgb(34, 84, 52), rgb(46, 112, 70), rgb(20, 48, 32), rgb(150, 255, 180), rgb(84, 196, 120)),
    accent: rgb(96, 255, 146),
    accent2: rgb(255, 196, 76),
    button: rgb(96, 255, 146),
    chrome: Some((rgb(24, 30, 28), rgb(96, 255, 146))),
    glow: None,
    glass: false,
    backdrop: None,
    round: 0.0,
    pixel: false,
    hard_shadow: None,
    font: Some(crate::fonts::mono_bold_font),
    pixel_font: false,
    own_accent: false,
    particles: None,
    skin: Skin::Terminal,
    label: None,
};

/// A theme's colours (None for "Your colours").
pub fn theme_def(t: PlusTheme) -> Option<ThemeDef> {
    match t {
        PlusTheme::Yours => None,
        PlusTheme::Studio => Some(STUDIO),
        PlusTheme::Pokedex => Some(POKEDEX),
        PlusTheme::Aurora => Some(AURORA),
        PlusTheme::Field => Some(FIELD),
        PlusTheme::Synthwave => Some(SYNTH),
        PlusTheme::Gold => Some(GOLD_T),
        PlusTheme::Companion => Some(COMPANION),
        PlusTheme::Pocket => Some(POCKET),
        PlusTheme::Candy => Some(CANDY),
        PlusTheme::Brutal => Some(BRUTAL),
        PlusTheme::Liquid => Some(LIQUID),
        PlusTheme::Winter => Some(WINTER),
        PlusTheme::Spooky => Some(SPOOKY),
        PlusTheme::Sticker => Some(STICKER),
        PlusTheme::GlassHud => Some(GLASS_HUD),
        PlusTheme::RetroDs => Some(RETRO_DS),
        PlusTheme::Journal => Some(JOURNAL_T),
        PlusTheme::Broadcast => Some(BROADCAST_T),
        PlusTheme::Terminal => Some(TERMINAL_T),
    }
}

/// The themes in the picker, in order ("Your colours" first). Themes still
/// being built are left out until they're done.
pub const PLUS_THEMES: [PlusTheme; 20] = [
    PlusTheme::Yours,
    PlusTheme::Companion,
    PlusTheme::Studio,
    PlusTheme::Pokedex,
    PlusTheme::Pocket,
    PlusTheme::Aurora,
    PlusTheme::Field,
    PlusTheme::Synthwave,
    PlusTheme::Gold,
    PlusTheme::Candy,
    PlusTheme::Brutal,
    PlusTheme::Liquid,
    PlusTheme::Winter,
    PlusTheme::Spooky,
    PlusTheme::Sticker,
    PlusTheme::GlassHud,
    PlusTheme::RetroDs,
    PlusTheme::Journal,
    PlusTheme::Broadcast,
    PlusTheme::Terminal,
];

/// The themes that match an overlay theme, shown as their own group.
pub const OVERLAY_MATCH: [PlusTheme; 6] = [PlusTheme::Sticker, PlusTheme::GlassHud, PlusTheme::RetroDs, PlusTheme::Journal, PlusTheme::Broadcast, PlusTheme::Terminal];

/// The app theme that matches an overlay theme, if there is one (Pokétch,
/// Holo card, Stream neon and Classic have none).
pub fn theme_for_overlay(o: crate::config::OverlayTheme) -> Option<PlusTheme> {
    use crate::config::OverlayTheme as O;
    match o {
        O::Anime => Some(PlusTheme::Sticker),
        O::Glass => Some(PlusTheme::GlassHud),
        O::Retro => Some(PlusTheme::RetroDs),
        O::Journal => Some(PlusTheme::Journal),
        O::Broadcast => Some(PlusTheme::Broadcast),
        O::Terminal => Some(PlusTheme::Terminal),
        // (the overlays made for the app themes)
        O::Studio => Some(PlusTheme::Studio),
        O::Pokedex => Some(PlusTheme::Pokedex),
        O::Aurora => Some(PlusTheme::Aurora),
        O::FieldGuide => Some(PlusTheme::Field),
        O::Synthwave => Some(PlusTheme::Synthwave),
        O::Gold => Some(PlusTheme::Gold),
        O::Companion => Some(PlusTheme::Companion),
        O::Pocket => Some(PlusTheme::Pocket),
        O::Candy => Some(PlusTheme::Candy),
        O::Brutal => Some(PlusTheme::Brutal),
        O::Liquid => Some(PlusTheme::Liquid),
        O::Winter => Some(PlusTheme::Winter),
        O::Spooky => Some(PlusTheme::Spooky),
        _ => None,
    }
}

/// Can this theme be picked yet?
pub fn theme_ready(t: PlusTheme) -> bool {
    PLUS_THEMES.contains(&t)
}

/// The theme in use right now (only with Modern Plus on), in its own mode
/// or flipped to the other one (Appearance → Theme → Light / Dark).
fn active_theme(a: &crate::config::Appearance) -> Option<ThemeDef> {
    if a.plus && theme_ready(a.plus_theme) {
        theme_def(a.plus_theme).map(|t| if a.theme_flip { flip(t) } else { t }).map(|t| {
            // your own colours through the theme (Appearance → Theme → Use my colours)
            if a.theme_colours && !t.own_accent {
                let c3 = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
                recolour(t, c3(a.accent), a.gradient.then(|| c3(a.accent2)), a.separate_buttons.then(|| c3(a.button)))
            } else {
                t
            }
        })
    } else {
        None
    }
}

/// Hue (0..360), saturation and lightness (0..1).
fn to_hsl(c: Color32) -> (f32, f32, f32) {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let l = (mx + mn) / 2.0;
    if mx - mn < 1e-5 {
        return (0.0, 0.0, l);
    }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32, a: u8) -> Color32 {
    let h = h.rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let ch = |t: f32| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color32::from_rgba_unmultiplied(ch(h + 1.0 / 3.0), ch(h), ch(h - 1.0 / 3.0), a)
}

/// The hue turn your colours put on the theme in use (0 = none), for the
/// few colours drawn outside the theme's own (one-colour charts, a skin's
/// fixed touches).
static HUE_TURN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A colour turned by the theme's hue turn (your colours, when used).
pub fn hue_turned(c: Color32) -> Color32 {
    let turn = f32::from_bits(HUE_TURN.load(std::sync::atomic::Ordering::Relaxed));
    if turn == 0.0 {
        return c;
    }
    let (h, s, l) = to_hsl(c);
    from_hsl(h + turn, s, l, c.a())
}

/// The theme in your colours: every surface turned to your colour's hue
/// (so it keeps the theme's light, dark and softness), your colour as its
/// accent, your second colour and button colour if you set them, all kept
/// readable. A grey pick makes the theme grey.
pub fn recolour(t: ThemeDef, acc: Color32, acc2: Option<Color32>, btn: Option<Color32>) -> ThemeDef {
    let (h0, _, _) = to_hsl(t.accent);
    let (h1, s1, _) = to_hsl(acc);
    let grey = s1 < 0.08;
    let turn = h1 - h0;
    let rot = |c: Color32| {
        let (h, s, l) = to_hsl(c);
        from_hsl(h + turn, if grey { s * 0.15 } else { s }, l, c.a())
    };
    let mut f = t;
    let s = t.s;
    f.s = Surf {
        sidebar: rot(s.sidebar),
        page: rot(s.page),
        card: rot(s.card),
        card_hover: rot(s.card_hover),
        line: rot(s.line),
        control: rot(s.control),
        selected: rot(s.selected),
        text: rot(s.text),
        muted: rot(s.muted),
        field: rot(s.field),
    };
    f.accent2 = acc2.unwrap_or_else(|| rot(t.accent2));
    f.glow = t.glow.map(|(a, b)| (rot(a), rot(b)));
    f.backdrop = t.backdrop.map(|(a, b)| (rot(a), rot(b)));
    f.chrome = t.chrome.map(|(a, b)| (rot(a), rot(b)));
    f.hard_shadow = t.hard_shadow.map(rot);
    let to = if f.light { Color32::BLACK } else { Color32::WHITE };
    f.accent = reach(acc, f.s.card, 3.0, to);
    f.accent2 = reach(f.accent2, f.s.card, 2.6, to);
    // the button: yours, with its own text readable on it
    let b0 = btn.unwrap_or(acc);
    let best = |c: Color32| contrast(c, Color32::WHITE).max(contrast(c, rgb(30, 28, 44)));
    f.button = b0;
    let mut k = 0.0;
    while best(f.button) < 4.5 && k < 1.0 {
        k += 0.04;
        f.button = mix(b0, to, k);
    }
    f.s.selected = mix(f.s.card, f.accent, if f.light { 0.16 } else { 0.28 });
    // (the writing stays readable whatever the hue did to the surfaces)
    f.s.text = reach(f.s.text, f.s.card, 7.0, to);
    f.s.muted = reach(f.s.muted, f.s.card, 4.5, to);
    if let Some((bar, ink)) = f.chrome {
        f.chrome = Some((bar, reach(ink, bar, 4.5, if contrast(Color32::WHITE, bar) > contrast(Color32::BLACK, bar) { Color32::WHITE } else { Color32::BLACK })));
    }
    f
}

/// WCAG contrast ratio between two colours.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let lum = |c: Color32| 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c` moved towards `toward` (black or white) just until it reaches
/// `min` contrast on `bg`.
fn reach(c: Color32, bg: Color32, min: f32, toward: Color32) -> Color32 {
    let mut k = 0.0;
    let mut x = c;
    while contrast(x, bg) < min && k < 1.0 {
        k += 0.04;
        x = mix(c, toward, k);
    }
    x
}

/// A theme in the other mode: a dark theme made light, a light one dark.
/// The surfaces are rebuilt around the theme's own colour, and its accents
/// darkened (on light) or lifted (on dark) until they read; its title bar,
/// glows, particles and skin stay as they are.
pub fn flip(t: ThemeDef) -> ThemeDef {
    let mut f = t;
    let tint = t.accent;
    if t.light {
        // light → dark
        let base = |k: f32| mix(rgb(10, 10, 14), tint, k);
        let page = base(0.06);
        let card = mix(base(0.10), Color32::WHITE, 0.04);
        f.s = Surf {
            sidebar: base(0.04),
            page,
            card,
            card_hover: mix(card, Color32::WHITE, 0.05),
            line: mix(card, Color32::WHITE, 0.12),
            control: mix(card, Color32::WHITE, 0.22),
            selected: card,
            text: mix(rgb(240, 240, 246), tint, 0.06),
            muted: mix(rgb(165, 165, 180), tint, 0.15),
            field: base(0.04),
        };
        f.light = false;
        f.accent = reach(t.accent, card, 3.5, Color32::WHITE);
        f.accent2 = reach(t.accent2, card, 3.0, Color32::WHITE);
        f.button = reach(t.button, page, 3.0, Color32::WHITE);
        f.s.selected = mix(card, f.accent, 0.28);
        f.backdrop = t.backdrop.map(|(a, b)| (mix(page, a, 0.18), mix(page, b, 0.22)));
        f.hard_shadow = t.hard_shadow.map(|_| mix(f.s.text, page, 0.45));
    } else {
        // dark → light
        let page = mix(Color32::WHITE, tint, 0.06);
        let card = mix(Color32::WHITE, tint, 0.015);
        f.s = Surf {
            sidebar: mix(Color32::WHITE, tint, 0.10),
            page,
            card,
            card_hover: mix(Color32::WHITE, tint, 0.05),
            line: mix(rgb(218, 218, 226), tint, 0.12),
            control: mix(rgb(180, 180, 192), tint, 0.15),
            selected: card,
            text: mix(rgb(20, 20, 28), tint, 0.12),
            muted: mix(rgb(92, 92, 108), tint, 0.12),
            field: card,
        };
        f.light = true;
        f.accent = reach(t.accent, card, 3.5, Color32::BLACK);
        f.accent2 = reach(t.accent2, card, 3.0, Color32::BLACK);
        f.button = reach(t.button, page, 3.0, Color32::BLACK);
        f.s.selected = mix(card, f.accent, 0.16);
        f.backdrop = t.backdrop.map(|(a, b)| (mix(page, a, 0.16), mix(page, b, 0.2)));
        f.hard_shadow = t.hard_shadow.map(|_| f.s.text);
    }
    // a few themes' dark versions are made by hand: built from their red,
    // they came out red-brown
    if t.light {
        let hand = |sidebar, page, card, hover, line, control, text, muted| Surf {
            sidebar,
            page,
            card,
            card_hover: hover,
            line,
            control,
            selected: card,
            text,
            muted,
            field: sidebar,
        };
        match t.name {
            // a night notebook: navy paper, cream writing, blue-ink pen
            "Field journal" => {
                f.s = hand(rgb(16, 21, 36), rgb(20, 26, 44), rgb(27, 34, 56), rgb(34, 42, 68), rgb(60, 74, 110), rgb(90, 106, 150), rgb(236, 228, 206), rgb(172, 172, 188));
                f.accent = rgb(122, 172, 240);
                f.accent2 = rgb(240, 204, 120);
                f.button = rgb(64, 110, 200);
                f.s.selected = mix(f.s.card, f.accent, 0.25);
            }
            // the black DS Lite: graphite, a blue screen glow and its yellow
            "Retro DS" => {
                f.s = hand(rgb(22, 23, 27), rgb(28, 29, 34), rgb(38, 40, 46), rgb(46, 48, 56), rgb(70, 72, 82), rgb(100, 102, 115), rgb(232, 232, 220), rgb(165, 165, 176));
                f.accent = rgb(96, 164, 240);
                f.accent2 = rgb(248, 216, 56);
                f.button = rgb(28, 104, 192);
                f.s.selected = mix(f.s.card, f.accent, 0.25);
                f.hard_shadow = Some(rgb(10, 10, 12));
            }
            _ => {}
        }
    }
    // (the button's own text, light or dark, must read on it)
    let to = if f.light { Color32::BLACK } else { Color32::WHITE };
    let best = |c: Color32| contrast(c, Color32::WHITE).max(contrast(c, rgb(30, 28, 44)));
    let b0 = f.button;
    let mut k = 0.0;
    while best(f.button) < 4.5 && k < 1.0 {
        k += 0.04;
        f.button = mix(b0, to, k);
    }
    // (muted text must still read on cards)
    f.s.muted = reach(f.s.muted, f.s.card, 4.5, if f.light { Color32::BLACK } else { Color32::WHITE });
    f
}

static THEME: std::sync::RwLock<Option<ThemeDef>> = std::sync::RwLock::new(None);

/// The Modern Plus theme in use, if any.
pub fn theme_now() -> Option<ThemeDef> {
    *THEME.read().unwrap_or_else(|e| e.into_inner())
}

/// The title bar's own fill and ink, when the theme has one.
pub fn chrome() -> Option<(Color32, Color32)> {
    theme_now().and_then(|t| t.chrome)
}

/// The theme's background glows, if any.
pub fn glow() -> Option<(Color32, Color32)> {
    theme_now().and_then(|t| t.glow)
}

/// Glass: see-through cards with a light rim (Aurora glass, Synthwave).
pub fn glass() -> bool {
    theme_now().is_some_and(|t| t.glass)
}

/// Aurora glass or Glass HUD in a dark mode: their glass is mixed in screen
/// space (`see`) so it stays dark enough for light text over the glows.
pub fn glass_dark() -> bool {
    plus() && matches!(skin(), Skin::Aurora | Skin::Glass) && theme_now().is_some_and(|t| !t.light)
}

/// `c` at `al` (0..255) of opacity, mixed in screen space. (egui's own
/// `from_rgba_unmultiplied` and `from_white_alpha` mix in linear light, which
/// makes a faint layer far brighter than asked on a dark page: a 6% white
/// card came out as a 27% grey one, and the glows turned a navy page teal.)
pub fn see(c: Color32, al: u8) -> Color32 {
    let k = al as u32;
    let f = |v: u8| ((v as u32 * k + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), al)
}

/// How round corners are (1 = Modern Plus's own; Field guide is squarer).
pub fn round_scale() -> f32 {
    theme_now().map_or(1.0, |t| t.round)
}

static PARTICLES_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// The theme's face for titles and big numbers, at `size`.
pub fn theme_font(size: f32) -> Option<egui::FontId> {
    // (Candy's and Brutal's display faces jam their letters together when
    // small: only for big headings, a heavy plain face below that)
    if matches!(skin(), Skin::Candy | Skin::Brutal) && size < 24.0 {
        return Some(crate::fonts::black_font(size));
    }
    theme_now().and_then(|t| t.font).map(|f| f(size))
}

/// Buttons' labels in the pixel face? (Pokédex, Pocket, Retro DS.)
pub fn pixel_type() -> bool {
    theme_now().is_some_and(|t| t.pixel_font)
}

/// The theme's particles, if it has some and they're on.
pub fn particles() -> Option<Particles> {
    if !PARTICLES_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    theme_now().and_then(|t| t.particles)
}

/// Paint the drifting particles over `area` (snow falls, embers rise). Each
/// one's path comes from its number, so nothing needs storing; with
/// Animations off they stay still.
pub fn paint_particles(p: &Painter, area: Rect, time: f64, kind: Particles) {
    let n = ((area.width() * area.height()) / 9000.0).clamp(24.0, 140.0) as u32;
    let t = if motion() { time as f32 } else { 0.0 };
    // a cheap hash: a fixed 0..1 value for particle i, slot k
    let h = |i: u32, k: u32| -> f32 {
        let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        (x % 10_000) as f32 / 10_000.0
    };
    for i in 0..n {
        let speed = 10.0 + 22.0 * h(i, 1);
        let size = 1.0 + 2.2 * h(i, 2);
        let x0 = area.min.x + area.width() * h(i, 3);
        let travel = area.height() + 20.0;
        let off = (h(i, 4) * travel + t * speed).rem_euclid(travel);
        let sway = (t * (0.4 + 0.6 * h(i, 5)) + h(i, 6) * 6.28).sin() * (6.0 + 10.0 * h(i, 7));
        let (y, col) = match kind {
            Particles::Snow => {
                // (on Winter's light version, pale blue flakes so they show)
                let al = (70.0 + 110.0 * h(i, 8)) as u8;
                let col = if theme_now().is_some_and(|t| t.light) { Color32::from_rgba_unmultiplied(110, 160, 215, al) } else { Color32::from_white_alpha(al) };
                (area.min.y - 10.0 + off, col)
            }
            Particles::Embers => {
                // orange to a deeper red, fading near the top
                let c = hue_turned(mix(rgb(255, 160, 60), rgb(240, 90, 30), h(i, 8)));
                let y = area.max.y + 10.0 - off;
                let fade = ((y - area.min.y) / area.height()).clamp(0.0, 1.0);
                (y, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (60.0 + 150.0 * h(i, 9) * fade) as u8))
            }
        };
        let c = egui::pos2(x0 + sway, y);
        if kind == Particles::Embers {
            p.circle_filled(c, size * 2.4, col.gamma_multiply(0.18));
        }
        p.circle_filled(c, size, col);
    }
}

/// The face for big numbers: the theme's own, unless it's a thin or
/// scratchy one meant for headings only (Winter, Spooky).
pub fn number_face(size: f32) -> Option<egui::FontId> {
    if matches!(skin(), Skin::Winter | Skin::Spooky) {
        return None;
    }
    theme_font(size)
}

/// Page titles in the theme's colour: pumpkin (Spooky), ice (Winter), gold
/// (Gold), neon (Synthwave). None: the text colour.
pub fn title_ink() -> Option<Color32> {
    let t = theme_now()?;
    match skin() {
        Skin::Spooky | Skin::Winter | Skin::Gold | Skin::Synth => Some(t.accent),
        _ => None,
    }
}

// ── charts and stats in each theme's own colours

/// A theme's colours for charts, stats and app icons (by hue: each chart
/// colour becomes the theme's nearest one, so green still means good and
/// red bad). Picked to stand out on the theme's cards.
fn chart_palette(t: &ThemeDef) -> &'static [Color32] {
    const COMPANION_P: [Color32; 8] = [rgb(243, 166, 232), rgb(139, 106, 216), rgb(125, 180, 250), rgb(110, 220, 170), rgb(250, 180, 120), rgb(240, 110, 150), rgb(240, 220, 110), rgb(190, 160, 240)];
    const STUDIO_P: [Color32; 8] = [rgb(139, 92, 246), rgb(249, 115, 22), rgb(217, 70, 239), rgb(245, 158, 11), rgb(56, 189, 248), rgb(16, 185, 129), rgb(244, 63, 94), rgb(99, 102, 241)];
    const DEX_P: [Color32; 8] = [rgb(239, 68, 68), rgb(56, 189, 248), rgb(250, 204, 21), rgb(74, 222, 128), rgb(251, 146, 60), rgb(168, 85, 247), rgb(45, 212, 191), rgb(244, 114, 182)];
    const POCKET_P: [Color32; 4] = [rgb(15, 56, 15), rgb(34, 80, 30), rgb(48, 98, 48), rgb(70, 110, 30)];
    const AURORA_P: [Color32; 8] = [rgb(94, 234, 212), rgb(167, 139, 250), rgb(125, 211, 252), rgb(244, 114, 182), rgb(190, 242, 100), rgb(252, 211, 77), rgb(129, 140, 248), rgb(253, 164, 175)];
    const FIELD_P: [Color32; 8] = [rgb(15, 118, 110), rgb(194, 65, 12), rgb(161, 98, 7), rgb(77, 124, 15), rgb(37, 99, 160), rgb(126, 52, 150), rgb(185, 28, 28), rgb(21, 128, 61)];
    const SYNTH_P: [Color32; 8] = [rgb(34, 211, 238), rgb(244, 114, 182), rgb(168, 85, 247), rgb(250, 204, 21), rgb(232, 121, 249), rgb(96, 165, 250), rgb(251, 146, 60), rgb(163, 230, 53)];
    const GOLD_P: [Color32; 8] = [rgb(242, 193, 78), rgb(255, 240, 179), rgb(205, 127, 50), rgb(230, 210, 160), rgb(217, 119, 6), rgb(150, 190, 120), rgb(214, 104, 84), rgb(170, 150, 220)];
    const CANDY_P: [Color32; 8] = [rgb(236, 72, 153), rgb(139, 92, 246), rgb(14, 165, 233), rgb(16, 185, 129), rgb(202, 138, 4), rgb(249, 115, 22), rgb(225, 29, 72), rgb(168, 85, 247)];
    const BRUTAL_P: [Color32; 8] = [rgb(255, 92, 138), rgb(13, 148, 136), rgb(202, 138, 4), rgb(59, 130, 246), rgb(249, 115, 22), rgb(34, 197, 94), rgb(168, 85, 247), rgb(239, 68, 68)];
    const LIQUID_P: [Color32; 8] = [rgb(37, 99, 235), rgb(219, 39, 119), rgb(8, 145, 178), rgb(124, 58, 237), rgb(5, 150, 105), rgb(217, 119, 6), rgb(220, 38, 38), rgb(2, 132, 199)];
    const WINTER_P: [Color32; 8] = [rgb(125, 211, 252), rgb(244, 63, 94), rgb(52, 211, 153), rgb(186, 230, 253), rgb(165, 180, 252), rgb(250, 204, 21), rgb(56, 189, 248), rgb(244, 114, 182)];
    const SPOOKY_P: [Color32; 8] = [rgb(249, 115, 22), rgb(163, 230, 53), rgb(168, 85, 247), rgb(239, 68, 68), rgb(250, 204, 21), rgb(74, 222, 128), rgb(217, 70, 239), rgb(231, 224, 200)];
    const STICKER_P: [Color32; 8] = [rgb(30, 170, 210), rgb(40, 180, 120), rgb(230, 170, 20), rgb(240, 120, 40), rgb(230, 70, 130), rgb(130, 90, 230), rgb(220, 60, 60), rgb(60, 120, 230)];
    const RETRO_P: [Color32; 8] = [rgb(40, 160, 90), rgb(200, 160, 20), rgb(200, 60, 44), rgb(40, 120, 210), rgb(140, 80, 200), rgb(220, 120, 30), rgb(30, 150, 140), rgb(210, 80, 140)];
    const JOURNAL_P: [Color32; 7] = [rgb(32, 44, 96), rgb(196, 46, 46), rgb(40, 130, 80), rgb(200, 150, 20), rgb(50, 100, 190), rgb(120, 60, 150), rgb(210, 110, 40)];
    const BROADCAST_P: [Color32; 8] = [rgb(232, 36, 52), rgb(250, 190, 40), rgb(59, 130, 246), rgb(34, 197, 94), rgb(34, 211, 238), rgb(249, 115, 22), rgb(168, 85, 247), rgb(236, 72, 153)];
    const TERMINAL_P: [Color32; 5] = [rgb(96, 255, 146), rgb(255, 196, 76), rgb(255, 110, 90), rgb(110, 210, 255), rgb(200, 140, 255)];
    match t.name {
        "Companion" => &COMPANION_P,
        "Studio" => &STUDIO_P,
        "Pokédex" => &DEX_P,
        "Pocket" => &POCKET_P,
        "Aurora glass" => &AURORA_P,
        "Field guide" => &FIELD_P,
        "Synthwave" => &SYNTH_P,
        "Gold" => &GOLD_P,
        "Candy" => &CANDY_P,
        "Brutal" => &BRUTAL_P,
        "Liquid" => &LIQUID_P,
        "Winter" => &WINTER_P,
        "Spooky" => &SPOOKY_P,
        "Sticker" => &STICKER_P,
        "Retro DS" => &RETRO_P,
        "Field journal" => &JOURNAL_P,
        "Broadcast" => &BROADCAST_P,
        "Terminal" => &TERMINAL_P,
        _ => &[],
    }
}

/// Hue (0..360) and saturation (0..1) of a colour.
fn hue_sat(c: Color32) -> (f32, f32) {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    if d < 1e-4 {
        return (0.0, 0.0);
    }
    let h = if mx == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 })
}

/// A chart, stat or icon colour in the current theme's palette (as it is
/// without a theme, and greys stay grey). Keeps its transparency.
pub fn chart_colour(c: Color32) -> Color32 {
    let Some(t) = theme_now().filter(|_| plus()) else { return c };
    let pal = chart_palette(&t);
    if pal.is_empty() {
        return c;
    }
    let (h, sat) = hue_sat(c);
    if sat < 0.18 {
        return c;
    }
    // one hue only (Pocket's greens): spread by hue instead
    let (h0, _) = hue_sat(pal[0]);
    let mono = pal.iter().all(|p| {
        let d = (hue_sat(*p).0 - h0).abs();
        d.min(360.0 - d) < 40.0
    });
    let pick = if mono {
        hue_turned(pal[((h / 360.0 * pal.len() as f32) as usize).min(pal.len() - 1)])
    } else {
        *pal.iter()
            .min_by(|a, b| {
                let da = (hue_sat(**a).0 - h).abs();
                let db = (hue_sat(**b).0 - h).abs();
                da.min(360.0 - da).total_cmp(&db.min(360.0 - db))
            })
            .unwrap_or(&c)
    };
    // (readable on this theme's cards, flipped ones included)
    let pick = reach(pick, t.s.card, 2.6, if t.light { Color32::BLACK } else { Color32::WHITE });
    if c.a() == 255 {
        pick
    } else {
        Color32::from_rgba_unmultiplied(pick.r(), pick.g(), pick.b(), c.a())
    }
}

/// The skin in use (Plain outside Modern Plus or without a theme).
pub fn skin() -> Skin {
    if plus() {
        theme_now().map_or(Skin::Plain, |t| t.skin)
    } else {
        Skin::Plain
    }
}

/// The skin's face for buttons, tabs and small labels, if it has one.
pub fn label_font(size: f32) -> Option<egui::FontId> {
    Some(match skin() {
        Skin::Pixel => crate::fonts::pixel_ui_font(size),
        Skin::Sticker => crate::fonts::black_font(size),
        Skin::Journal => crate::fonts::hand_font(size * 1.18),
        Skin::Broadcast => crate::fonts::impact_font(size * 1.05),
        Skin::Terminal => crate::fonts::mono_bold_font(size * 0.95),
        // (their display faces are too tight for tabs and buttons)
        Skin::Candy | Skin::Brutal => crate::fonts::black_font(size),
        _ => return theme_now().and_then(|t| t.label).map(|f| f(size)),
    })
}

/// The ready-made themes' fields, dropdowns, search boxes and plain buttons:
/// (corner, outline, outline on hover, fill, fill on hover). None keeps
/// Modern Plus's own.
pub fn skin_widgets() -> Option<(Option<f32>, Stroke, Stroke, Color32, Color32)> {
    // (fields, dropdowns and plain buttons take the theme's one corner)
    skin_widgets_raw().map(|(r, a, b, c, d)| (skin_corner(200.0).or(r), a, b, c, d))
}

fn skin_widgets_raw() -> Option<(Option<f32>, Stroke, Stroke, Color32, Color32)> {
    let t = theme_now()?;
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    let s = t.s;
    Some(match t.skin {
        Skin::Companion => (None, Stroke::new(1.0f32, a(t.accent, 70)), Stroke::new(1.0f32, t.accent), s.card, mix(s.card, t.accent, 0.12)),
        Skin::Studio => (None, Stroke::new(1.0f32, s.line), Stroke::new(1.0f32, t.accent), s.card, mix(s.card, t.accent, 0.14)),
        Skin::Aurora if t.light => (None, Stroke::new(1.0f32, a(s.text, 50)), Stroke::new(1.0f32, a(t.accent, 200)), a(s.text, 8), a(s.text, 16)),
        Skin::Aurora => (None, Stroke::new(1.0f32, Color32::from_white_alpha(60)), Stroke::new(1.0f32, a(t.accent, 200)), Color32::from_white_alpha(14), Color32::from_white_alpha(26)),
        Skin::Field => (Some(3.0), Stroke::new(1.0f32, a(t.accent, 130)), Stroke::new(1.5f32, t.accent), s.card, mix(s.card, t.accent, 0.08)),
        Skin::Synth => (Some(6.0), Stroke::new(1.0f32, a(t.accent, 130)), Stroke::new(1.0f32, t.accent2), a(s.card, 220), mix(s.card, t.accent2, 0.14)),
        Skin::Gold => (Some(4.0), Stroke::new(1.0f32, a(t.accent, 120)), Stroke::new(1.0f32, t.accent), s.card, mix(s.card, t.accent, 0.1)),
        // (white fields on the light versions; card-coloured on the dark ones)
        Skin::Candy if t.light => (None, Stroke::new(2.0f32, a(t.accent, 110)), Stroke::new(2.0f32, t.accent), Color32::WHITE, mix(Color32::WHITE, t.accent, 0.08)),
        Skin::Candy => (None, Stroke::new(2.0f32, a(t.accent, 110)), Stroke::new(2.0f32, t.accent), s.card, mix(s.card, t.accent, 0.16)),
        Skin::Brutal if t.light => (Some(0.0), Stroke::new(2.0f32, s.text), Stroke::new(2.0f32, s.text), Color32::WHITE, rgb(255, 233, 168)),
        Skin::Brutal => (Some(0.0), Stroke::new(2.0f32, s.text), Stroke::new(2.0f32, s.text), s.card, mix(s.card, t.accent2, 0.22)),
        Skin::Liquid if t.light => (None, Stroke::new(1.0f32, Color32::from_white_alpha(170)), Stroke::new(1.0f32, Color32::WHITE), Color32::from_white_alpha(96), Color32::from_white_alpha(140)),
        Skin::Liquid => (None, Stroke::new(1.0f32, Color32::from_white_alpha(60)), Stroke::new(1.0f32, a(t.accent, 220)), Color32::from_white_alpha(14), Color32::from_white_alpha(28)),
        Skin::Winter => (Some(10.0), Stroke::new(1.0f32, a(t.accent, 110)), Stroke::new(1.0f32, t.accent), s.card, mix(s.card, t.accent, 0.12)),
        Skin::Spooky => (Some(8.0), Stroke::new(1.0f32, a(t.accent, 120)), Stroke::new(1.0f32, t.accent2), s.card, mix(s.card, t.accent, 0.12)),
        _ => return None,
    })
}

/// The skin's look behind the page, drawn over the page colour: a ruled
/// notebook page with its red margin (Field journal), scanlines (Terminal),
/// halftone dots (Sticker) or scoreboard stripes (Broadcast).
pub fn paint_skin_backdrop(p: &Painter, area: Rect) {
    match skin() {
        Skin::Journal => {
            let rule = Color32::from_rgba_unmultiplied(120, 160, 214, 70);
            let mut y = area.min.y + 30.0;
            while y < area.max.y {
                p.line_segment([egui::pos2(area.min.x, y), egui::pos2(area.max.x, y)], Stroke::new(1.0f32, rule));
                y += 28.0;
            }
            // the red margin, just right of the punched holes (see paint_skin_top)
            let mx = area.min.x + 27.0;
            p.line_segment([egui::pos2(mx, area.min.y), egui::pos2(mx, area.max.y)], Stroke::new(1.5f32, if theme_now().is_some_and(|t| t.light) { Color32::from_rgba_unmultiplied(232, 128, 128, 150) } else { Color32::from_rgba_unmultiplied(240, 204, 120, 110) }));
        }
        Skin::Terminal => {
            let line = Color32::from_rgba_unmultiplied(96, 255, 146, 7);
            let mut y = area.min.y;
            while y < area.max.y {
                p.line_segment([egui::pos2(area.min.x, y), egui::pos2(area.max.x, y)], Stroke::new(1.0f32, line));
                y += 4.0;
            }
        }
        Skin::Sticker => {
            let ink = theme_now().map_or(rgb(28, 24, 48), |t| t.s.text);
            let dot = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 16);
            let mut y = area.min.y + 9.0;
            let mut row = 0;
            while y < area.max.y {
                let mut x = area.min.x + if row % 2 == 0 { 9.0 } else { 18.0 };
                while x < area.max.x {
                    p.circle_filled(egui::pos2(x, y), 1.6, dot);
                    x += 18.0;
                }
                y += 16.0;
                row += 1;
            }
        }
        Skin::Field => {
            // graph paper
            let fine = Color32::from_rgba_unmultiplied(15, 118, 110, 16);
            let bold = Color32::from_rgba_unmultiplied(15, 118, 110, 32);
            let mut k = 0;
            let mut x = area.min.x;
            while x < area.max.x {
                p.line_segment([egui::pos2(x, area.min.y), egui::pos2(x, area.max.y)], Stroke::new(1.0f32, if k % 5 == 0 { bold } else { fine }));
                x += 20.0;
                k += 1;
            }
            k = 0;
            let mut y = area.min.y;
            while y < area.max.y {
                p.line_segment([egui::pos2(area.min.x, y), egui::pos2(area.max.x, y)], Stroke::new(1.0f32, if k % 5 == 0 { bold } else { fine }));
                y += 20.0;
                k += 1;
            }
        }
        Skin::Synth => {
            // a retro grid running to the horizon along the bottom
            let pink = hue_turned(Color32::from_rgba_unmultiplied(244, 114, 182, 46));
            let horizon = area.max.y - area.height() * 0.32;
            p.line_segment([egui::pos2(area.min.x, horizon), egui::pos2(area.max.x, horizon)], Stroke::new(1.5f32, hue_turned(Color32::from_rgba_unmultiplied(244, 114, 182, 90))));
            // (its cross lines run towards you: paint_ambient)
            let cx = area.center().x;
            for i in -12..=12 {
                let xb = cx + i as f32 * area.width() / 10.0;
                let xt = cx + i as f32 * area.width() / 60.0;
                p.line_segment([egui::pos2(xt, horizon), egui::pos2(xb, area.max.y)], Stroke::new(1.0f32, pink));
            }
        }
        Skin::Gold | Skin::Candy => {
            // glints (Gold) or confetti (Candy), in fixed spots
            let gold = skin() == Skin::Gold;
            let h = |i: u32, k: u32| -> f32 {
                let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
                x ^= x >> 15;
                x = x.wrapping_mul(0x2C1B_3C6D);
                x ^= x >> 12;
                (x % 10_000) as f32 / 10_000.0
            };
            let n = ((area.width() * area.height()) / 26_000.0).clamp(12.0, 70.0) as u32;
            let confetti = [rgb(236, 72, 153), rgb(139, 92, 246), rgb(56, 189, 248), rgb(250, 204, 21), rgb(52, 211, 153)];
            for i in 0..n {
                let c = egui::pos2(area.min.x + area.width() * h(i, 1), area.min.y + area.height() * h(i, 2));
                if gold {
                    sparkle(p, c, 2.0 + 4.0 * h(i, 3), Color32::from_rgba_unmultiplied(242, 193, 78, (20.0 + 40.0 * h(i, 4)) as u8));
                } else {
                    let col = confetti[(h(i, 3) * 5.0) as usize % 5];
                    let r = Rect::from_center_size(c, egui::vec2(5.0, 9.0));
                    p.rect_filled(r, 2.5, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 40));
                }
            }
        }
        Skin::Liquid => {
            glow_spot(p, area.min + egui::vec2(area.width() * 0.2, area.height() * 0.25), area.width() * 0.45, rgb(219, 39, 119), 0.16);
            glow_spot(p, area.min + egui::vec2(area.width() * 0.8, area.height() * 0.7), area.width() * 0.5, rgb(37, 99, 235), 0.2);
        }
        Skin::Studio => {
            glow_spot(p, area.min + egui::vec2(area.width() * 0.88, area.height() * 0.05), area.width() * 0.5, rgb(139, 92, 246), 0.16);
            glow_spot(p, area.min + egui::vec2(area.width() * 0.08, area.height() * 0.95), area.width() * 0.4, rgb(249, 115, 22), 0.08);
        }
        Skin::Companion => {
            glow_spot(p, area.min + egui::vec2(area.width() * 0.5, -area.height() * 0.1), area.width() * 0.55, rgb(243, 166, 232), 0.08);
        }
        Skin::Winter => {
            // snow drifts along the bottom
            let light = theme_now().is_some_and(|t| t.light);
            let snow = if light { Color32::from_rgba_unmultiplied(120, 170, 220, 26) } else { Color32::from_rgba_unmultiplied(234, 244, 255, 22) };
            let base = area.max.y;
            let mut x = area.min.x - 40.0;
            let mut k = 0;
            while x < area.max.x + 40.0 {
                let rr = 60.0 + 30.0 * ((k * 37 % 7) as f32 / 7.0);
                p.circle_filled(egui::pos2(x, base + rr * 0.55), rr, snow);
                x += 90.0;
                k += 1;
            }
        }
        Skin::Spooky => {
            // a cobweb in the top-left corner
            let tx = theme_now().map_or(rgb(220, 210, 235), |t| t.s.text);
            let web = Color32::from_rgba_unmultiplied(tx.r(), tx.g(), tx.b(), 46);
            let o = area.min;
            let len = 150.0;
            let spokes = 6;
            for i in 0..=spokes {
                let a = std::f32::consts::FRAC_PI_2 * i as f32 / spokes as f32;
                p.line_segment([o, o + egui::vec2(a.cos(), a.sin()) * len], Stroke::new(1.0f32, web));
            }
            for ring in 1..=4 {
                let rr = len * ring as f32 / 4.6;
                let pts: Vec<egui::Pos2> = (0..=spokes)
                    .map(|i| {
                        let a = std::f32::consts::FRAC_PI_2 * i as f32 / spokes as f32;
                        // (each strand sags a little towards the corner)
                        let sag = if i % 2 == 1 { 0.86 } else { 1.0 };
                        o + egui::vec2(a.cos(), a.sin()) * rr * sag
                    })
                    .collect();
                p.add(Shape::line(pts, Stroke::new(1.0f32, web)));
            }
            // a little spider on its thread
            let sx = o.x + len * 0.72;
            p.line_segment([egui::pos2(sx, o.y), egui::pos2(sx, o.y + 64.0)], Stroke::new(1.0f32, web));
            p.circle_filled(egui::pos2(sx, o.y + 68.0), 4.0, Color32::from_rgba_unmultiplied(20, 10, 26, 200));
        }
        Skin::Broadcast => {
            let band = Color32::from_white_alpha(4);
            let mut x = area.min.x - area.height();
            while x < area.max.x {
                p.add(Shape::convex_polygon(
                    vec![egui::pos2(x, area.max.y), egui::pos2(x + 14.0, area.max.y), egui::pos2(x + 14.0 + area.height(), area.min.y), egui::pos2(x + area.height(), area.min.y)],
                    band,
                    Stroke::NONE,
                ));
                x += 52.0;
            }
        }
        _ => {}
    }
}

/// Field journal's punched holes down the left-hand edge, like a ring
/// binder's. They go on their own layer above the page (full-window pages
/// like the Map included), in the strip left of where pages start, so
/// nothing covers them.
pub fn paint_skin_top(ctx: &egui::Context, area: Rect) {
    if skin() != Skin::Journal {
        return;
    }
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("journal-holes")));
    let hx = area.min.x + 14.0;
    // (the holes show the darker page behind: in the paper's own tone)
    let paper = theme_now().map_or(rgb(251, 246, 233), |t| t.s.page);
    let hole = mix(paper, Color32::BLACK, 0.15);
    let hole_in = mix(paper, Color32::BLACK, 0.22);
    let mut y = area.min.y + 46.0;
    while y < area.max.y - 20.0 {
        let c = egui::pos2(hx, y);
        p.circle_filled(c + egui::vec2(0.0, 1.0), 7.5, Color32::from_rgba_unmultiplied(120, 104, 80, 60));
        p.circle_filled(c, 7.0, hole);
        p.circle_filled(c + egui::vec2(-1.0, -1.0), 5.0, hole_in);
        y += 64.0;
    }
}

/// The pixel themes' last pass over the Companion's window each frame: every
/// rounded box, pill, field, chip, bar and badge loses its round corners,
/// and small round things (dots, knobs, rings, the back button) become
/// squares, so the whole window reads as pixels without every page having
/// to know. Text, pictures and sprites are left as they are.
pub fn pixelize(ctx: &egui::Context) {
    // (the other skins use the same pass: Terminal squares corners too,
    // Broadcast and Field journal keep them small, Sticker inks every box)
    let sk = skin();
    let cap = match sk {
        Skin::Pixel | Skin::Terminal => 0.0,
        // (square themes square everything; Field guide too)
        Skin::Broadcast | Skin::Field => 0.0,
        Skin::Journal => 5.0,
        Skin::Sticker => f32::INFINITY,
        Skin::Brutal => 0.0,
        _ => return,
    };
    let ink = matches!(sk, Skin::Sticker | Skin::Brutal).then(|| theme_now().map_or(OUTLINE, |t| t.s.text));
    // (square themes: small circles, like toggle knobs and dots, go square too)
    let squares = matches!(sk, Skin::Pixel | Skin::Terminal | Skin::Brutal | Skin::Broadcast | Skin::Field);
    let sketch = sk == Skin::Journal;
    let mut ids: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());
    if !ids.contains(&egui::LayerId::background()) {
        ids.push(egui::LayerId::background());
    }
    ctx.graphics_mut(|g| {
        for id in ids {
            let list = g.entry(id);
            let n = list.next_idx().0;
            for i in 0..n {
                list.mutate_shape(egui::layers::ShapeIdx(i), |cs| {
                    square_off(&mut cs.shape, cap, squares, ink);
                    if sketch {
                        sketch_shape(&mut cs.shape);
                    }
                });
            }
        }
    });
}

// ── Field journal: everything drawn in pen, a little wobbly, gone over twice

/// A stroke's colour when it's one solid colour (strokes can be gradients).
trait SolidColour {
    fn solid(&self) -> Option<Color32>;
}
impl SolidColour for Color32 {
    fn solid(&self) -> Option<Color32> {
        Some(*self)
    }
}
impl SolidColour for egui::epaint::ColorMode {
    fn solid(&self) -> Option<Color32> {
        match self {
            egui::epaint::ColorMode::Solid(c) => Some(*c),
            _ => None,
        }
    }
}

/// A fixed 0..1 wobble for a point (the same every frame, so lines don't shimmer).
fn jitter(x: f32, y: f32, seed: u32) -> f32 {
    let mut h = ((x * 0.5).round() as i32 as u32).wrapping_mul(0x9E37_79B9) ^ ((y * 0.5).round() as i32 as u32).wrapping_mul(0x85EB_CA6B) ^ seed.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h % 1000) as f32 / 1000.0
}

/// A pen line from a to b: overshooting a touch at the ends, wandering a
/// little on the way.
fn wobble(a: egui::Pos2, b: egui::Pos2, seed: u32, amp: f32) -> Vec<egui::Pos2> {
    let d = b - a;
    let len = d.length();
    if len < 0.5 {
        return vec![a, b];
    }
    let dir = d / len;
    let nrm = egui::vec2(-dir.y, dir.x);
    let over = (len * 0.02).min(1.5);
    let n = ((len / 9.0) as usize).clamp(2, 60);
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let q = a + d * t;
            let edge = i == 0 || i == n;
            let off = if edge { 0.0 } else { (jitter(q.x, q.y, seed) - 0.5) * 2.0 * amp };
            let ext = if i == 0 { -over } else if i == n { over } else { 0.0 };
            q + nrm * off + dir * ext
        })
        .collect()
}

/// A shape outline as pen strokes: the line, then a lighter second go.
fn pen_lines(pts: &[egui::Pos2], closed: bool, width: f32, col: Color32) -> Vec<Shape> {
    let mut out = Vec::new();
    let n = pts.len();
    if n < 2 {
        return out;
    }
    let segs = if closed { n } else { n - 1 };
    for pass in 0..2u32 {
        let mut line: Vec<egui::Pos2> = Vec::new();
        for i in 0..segs {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let mut w = wobble(a, b, 7 + pass * 31, if pass == 0 { 0.7 } else { 1.1 });
            if !line.is_empty() {
                w.remove(0);
            }
            line.extend(w);
        }
        let (wd, c) = if pass == 0 { (width, col) } else { ((width * 0.7).max(0.6), col.gamma_multiply(0.45)) };
        out.push(Shape::line(line, Stroke::new(wd, c)));
    }
    out
}

/// How colourful a colour is (paper and greys are low, ink and markers high).
fn chroma(c: Color32) -> u8 {
    let (r, g, b) = (c.r(), c.g(), c.b());
    r.max(g).max(b) - r.min(g).min(b)
}

fn sketch_shape(s: &mut Shape) {
    match s {
        Shape::Vec(v) => v.iter_mut().for_each(sketch_shape),
        Shape::Rect(r) => {
            let rect = r.rect;
            if rect.width() < 3.0 || rect.height() < 3.0 || !rect.is_finite() {
                return;
            }
            let corners = vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
            let mut extra: Vec<Shape> = Vec::new();
            if r.stroke.width > 0.0 && r.stroke.color.a() > 0 {
                extra.extend(pen_lines(&corners, true, r.stroke.width.max(1.0), r.stroke.color));
                r.stroke = Stroke::NONE;
            } else if r.fill.a() > 40 && chroma(r.fill) > 60 && rect.width() < 700.0 && rect.height() < 320.0 {
                // a coloured box (a bar, a chip, a stamp): a pen outline in its own colour, darker
                let ink = mix(r.fill, Color32::BLACK, 0.4);
                extra.extend(pen_lines(&corners, true, 1.2, ink));
                // thin ones (progress and chart bars) get pencil hatching
                if rect.height() <= 24.0 || rect.width() <= 24.0 {
                    let hatch = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 70);
                    let step = 5.0;
                    let mut x = rect.min.x - rect.height();
                    while x < rect.max.x {
                        let a = egui::pos2(x.max(rect.min.x), rect.max.y - (x.max(rect.min.x) - x));
                        let x2 = (x + rect.height()).min(rect.max.x);
                        let b = egui::pos2(x2, rect.max.y - (x2 - x));
                        if a.y >= rect.min.y - 0.5 && b.y >= rect.min.y - 0.5 {
                            extra.push(Shape::line_segment([a, b], Stroke::new(1.0f32, hatch)));
                        }
                        x += step;
                    }
                }
            }
            if !extra.is_empty() {
                let base = Shape::Rect(*r);
                extra.insert(0, base);
                *s = Shape::Vec(extra);
            }
        }
        Shape::Circle(c) => {
            if c.radius < 2.0 {
                return;
            }
            let (w, col) = if c.stroke.width > 0.0 && c.stroke.color.a() > 0 {
                (c.stroke.width, c.stroke.color)
            } else if c.fill.a() > 40 && chroma(c.fill) > 60 {
                (1.1, mix(c.fill, Color32::BLACK, 0.4))
            } else {
                return;
            };
            let n = ((c.radius * 0.9) as usize).clamp(10, 48);
            let pts: Vec<egui::Pos2> = (0..n).map(|i| c.center + egui::Vec2::angled(i as f32 / n as f32 * std::f32::consts::TAU) * c.radius).collect();
            let mut v = vec![Shape::Circle(egui::epaint::CircleShape { stroke: Stroke::NONE, ..*c })];
            v.extend(pen_lines(&pts, true, w, col));
            *s = Shape::Vec(v);
        }
        Shape::LineSegment { points, stroke } => {
            if let Some(col) = stroke.color.solid() {
                let (a, b, w) = (points[0], points[1], stroke.width);
                if w > 0.0 {
                    *s = Shape::Vec(pen_lines(&[a, b], false, w, col));
                }
            }
        }
        Shape::Path(ps) => {
            if ps.stroke.width <= 0.0 || ps.points.len() < 2 || ps.points.len() > 600 {
                return;
            }
            if let Some(col) = ps.stroke.color.solid() {
                let lines = pen_lines(&ps.points, ps.closed, ps.stroke.width, col);
                ps.stroke.width = 0.0;
                let mut v = vec![Shape::Path(ps.clone())];
                v.extend(lines);
                *s = Shape::Vec(v);
            }
        }
        _ => {}
    }
}

fn square_off(s: &mut Shape, cap: f32, squares: bool, ink: Option<Color32>) {
    match s {
        Shape::Rect(r) => {
            let k = &mut r.rounding;
            k.nw = k.nw.min(cap);
            k.ne = k.ne.min(cap);
            k.sw = k.sw.min(cap);
            k.se = k.se.min(cap);
            // Sticker: every outlined box gets a 2px ink line
            if let Some(ink) = ink {
                if r.stroke.width > 0.0 && r.stroke.width < 2.0 && r.fill.a() > 0 {
                    r.stroke = Stroke::new(2.0f32, ink);
                }
            }
        }
        Shape::Vec(v) => v.iter_mut().for_each(|x| square_off(x, cap, squares, ink)),
        Shape::Circle(c) if squares && c.radius <= 24.0 => {
            let r = Rect::from_center_size(c.center, egui::vec2(c.radius * 2.0, c.radius * 2.0));
            *s = Shape::Rect(egui::epaint::RectShape::new(r, egui::Rounding::ZERO, c.fill, c.stroke));
        }
        _ => {}
    }
}

/// Pixel style on? (Pokédex, Pocket: pixel key buttons and pixel icons.)
pub fn pixel() -> bool {
    theme_now().is_some_and(|t| t.pixel)
}

/// The hard card shadow's colour, when the theme has one.
pub fn hard_shadow() -> Option<Color32> {
    theme_now().and_then(|t| t.hard_shadow)
}

/// Glass cards' fill and rim: faint white on dark themes, more on light
/// ones (Liquid), so they read as frosted either way.
pub fn glass_colours(light: bool) -> (Color32, Color32) {
    if glass_dark() && !light {
        // smoked glass: the theme's card, a little see-through, and a fine
        // light rim (light text stays readable over any glow behind it)
        let card = theme_now().map_or(rgb(24, 28, 46), |t| t.s.card);
        return (see(card, 222), see(Color32::WHITE, 38));
    }
    if light {
        (Color32::from_white_alpha(96), Color32::from_white_alpha(170))
    } else {
        (Color32::from_white_alpha(16), Color32::from_white_alpha(34))
    }
}

// ─────────────────────────── pixel style ───────────────────────────

/// A pixel key (the UI Lab's): square, a dark outline, a light bevel top and
/// left, a dark one bottom and right, and a hard drop shadow that collapses
/// as it's pressed (`down` 0..1). `t` is the hover (0..1).
pub fn pixel_key(rect: Rect, fill: Color32, t: f32, down: f32) -> Shape {
    let fill = mix(fill, Color32::WHITE, 0.12 * t.clamp(0.0, 1.0));
    let k = 3.0;
    let light = mix(fill, Color32::WHITE, 0.55);
    let dark = mix(fill, Color32::BLACK, 0.28);
    let line = mix(fill, Color32::BLACK, 0.82);
    let drop = 4.0 * (1.0 - down.clamp(0.0, 1.0));
    let r = rect.translate(egui::vec2(3.0 * down, 3.0 * down));
    let outer = r.expand(k);
    let mut v = Vec::new();
    if drop > 0.2 {
        v.push(Shape::rect_filled(outer.translate(egui::vec2(drop, drop)), 0.0, Color32::from_black_alpha(90)));
    }
    v.push(Shape::rect_filled(outer, 0.0, line));
    v.push(Shape::rect_filled(r, 0.0, fill));
    v.push(Shape::rect_filled(Rect::from_min_size(r.min, egui::vec2(r.width(), k)), 0.0, light));
    v.push(Shape::rect_filled(Rect::from_min_size(r.min, egui::vec2(k, r.height())), 0.0, light));
    v.push(Shape::rect_filled(Rect::from_min_max(egui::pos2(r.min.x, r.max.y - k), r.max), 0.0, dark));
    v.push(Shape::rect_filled(Rect::from_min_max(egui::pos2(r.max.x - k, r.min.y), r.max), 0.0, dark));
    Shape::Vec(v)
}

/// The UI Lab's 8×8 pixel app icons ('#' = a lit pixel).
const PX_SPARKLE: [&str; 8] = ["...#....", "...#....", "..###...", "#######.", "..###...", "...#....", "...#....", "........"];
const PX_FLAG: [&str; 8] = [".#......", ".#####..", ".#####..", ".####...", ".#......", ".#......", ".#......", "........"];
const PX_GRID: [&str; 8] = ["##.##.##", "##.##.##", "........", "##.##.##", "##.##.##", "........", "##.##.##", "##.##.##"];
const PX_EGG: [&str; 8] = ["...##...", "..####..", ".######.", ".######.", "########", "########", ".######.", "..####.."];
const PX_CARDS: [&str; 8] = ["..#####.", "..#...#.", "#####.#.", "#...#.#.", "#...#.#.", "#...###.", "#...#...", "#####..."];
const PX_SWORD: [&str; 8] = ["......##", ".....###", "....###.", "#..###..", ".####...", "..##....", ".#.##...", "#...#..."];
const PX_BOOK: [&str; 8] = [".######.", "#.....#.", "#.###.#.", "#.....#.", "#.###.#.", "#.....#.", "#######.", "........"];
const PX_LAYERS: [&str; 8] = ["...##...", ".######.", "########", ".######.", "#..##..#", ".######.", "..####..", "...##..."];
const PX_BARS: [&str; 8] = ["......##", "......##", "...##.##", "...##.##", "##.##.##", "##.##.##", "##.##.##", "........"];
const PX_GEAR: [&str; 8] = ["...##...", ".######.", ".##..##.", "###..###", "###..###", ".##..##.", ".######.", "...##..."];
// (and one for every other app and page, in the same style)
const PX_HOME: [&str; 8] = ["...##...", "..####..", ".######.", "########", ".#....#.", ".#.##.#.", ".#.##.#.", ".######."];
const PX_STORE: [&str; 8] = ["########", "#.#.#.##", "########", ".#....#.", ".#.##.#.", ".#.##.#.", ".#.##.#.", "########"];
const PX_SHELF: [&str; 8] = ["........", "##.#.##.", "##.#.##.", "##.#.##.", "##.#.##.", "##.#.##.", "########", "........"];
const PX_MEDAL: [&str; 8] = ["##...##.", ".#...#..", "..#.#...", ".#####..", "##.#.##.", "#.###.#.", "##.#.##.", ".#####.."];
const PX_TARGET: [&str; 8] = ["..####..", ".#....#.", "#..##..#", "#.#..#.#", "#.#..#.#", "#..##..#", ".#....#.", "..####.."];
const PX_EYE: [&str; 8] = ["........", "..####..", ".#....#.", "#..##..#", "#..##..#", ".#....#.", "..####..", "........"];
const PX_LIST: [&str; 8] = ["######..", "#....##.", "#.##..#.", "#.....#.", "#.###.#.", "#.....#.", "#.###.#.", "#######."];
const PX_STAR: [&str; 8] = ["...#....", "...#....", "..###...", "#######.", ".#####..", "..###...", ".##.##..", "##...##."];
const PX_TROPHY: [&str; 8] = ["########", "#.####.#", "#.####.#", ".######.", "..####..", "...##...", "..####..", ".######."];
const PX_PIN_MAP: [&str; 8] = ["..###...", ".#####..", ".##.##..", ".#####..", "..###...", "...#....", "...#....", "........"];
const PX_PAD: [&str; 8] = ["........", ".######.", "##.###.#", "#...#.##", "##.###.#", "########", "##....##", "........"];
const PX_PAW: [&str; 8] = [".#...#..", "###.###.", ".#...#..", "...#....", "..###...", ".#####..", ".#####..", "..#.#..."];
const PX_CLAP: [&str; 8] = ["#.#.#.#.", "########", "........", "########", "#......#", "#......#", "#......#", "########"];
const PX_NOTE: [&str; 8] = [".######.", "##.....#", ".#.###.#", "##.....#", ".#.###.#", "##.....#", ".#######", "........"];
const PX_SKULL: [&str; 8] = ["..####..", ".######.", "#..##..#", "#..##..#", "########", ".######.", ".#.##.#.", "........"];
const PX_PUSHPIN: [&str; 8] = ["...###..", "..####..", "..###...", "#####...", ".###....", "#.#.....", "#.......", "........"];
const PX_PALETTE: [&str; 8] = ["..####..", ".#....#.", "#.#..#.#", "#......#", "#.#...##", "#....#..", ".#..##..", "..##...."];
const PX_KEYS: [&str; 8] = ["........", "########", "#.#.#.##", "########", "##.#.#.#", "########", "#......#", "########"];

/// A theme's own icon tile (the skins: sticky notes, stickers, coins…).
/// Returns false to leave it to the usual chip.
pub fn skin_chip(p: &Painter, rect: Rect, icon: &str, color: Color32) -> bool {
    let Some(t) = theme_now().filter(|_| plus()) else { return false };
    let color = chart_colour(color);
    let s = t.s;
    let glyph = |ink: Color32| {
        p.text(rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(rect.height() * 0.54), ink);
    };
    let h = rect.height();
    match t.skin {
        Skin::Journal => {
            // a sticky note with a strip of tape
            p.rect_filled(rect.translate(egui::vec2(1.0, 1.5)), 1.0, Color32::from_black_alpha(30));
            p.rect_filled(rect, 1.0, rgb(255, 232, 128));
            let tape = Rect::from_center_size(egui::pos2(rect.center().x, rect.min.y + 1.0), egui::vec2(h * 0.5, h * 0.16));
            p.rect_filled(tape, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150));
            glyph(mix(color, rgb(32, 44, 96), 0.55));
        }
        Skin::Sticker => {
            // a round sticker: white, an ink rim, a hard shadow
            let r = h / 2.0;
            p.circle_filled(rect.center() + egui::vec2(2.5, 3.0), r, s.text);
            p.circle_filled(rect.center(), r, color);
            p.circle_stroke(rect.center(), r - 1.0, Stroke::new(2.0f32, s.text));
            glyph(Color32::WHITE);
        }
        Skin::Terminal => {
            p.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0f32, t.accent));
            p.text(egui::pos2(rect.min.x + 1.0, rect.center().y), egui::Align2::LEFT_CENTER, "[", crate::fonts::mono_font(h * 0.6), t.accent);
            p.text(egui::pos2(rect.max.x - 1.0, rect.center().y), egui::Align2::RIGHT_CENTER, "]", crate::fonts::mono_font(h * 0.6), t.accent);
            glyph(t.accent);
        }
        Skin::Broadcast => {
            let lean = h * 0.22;
            p.add(Shape::convex_polygon(vec![rect.left_bottom(), egui::pos2(rect.min.x + lean, rect.min.y), rect.right_top(), egui::pos2(rect.max.x - lean, rect.max.y)], color, Stroke::NONE));
            glyph(Color32::WHITE);
        }
        Skin::Glass | Skin::Aurora => {
            let r = (h * 0.32).round();
            p.rect_filled(rect, r, Color32::from_white_alpha(22));
            p.rect_filled(rect, r, color.gamma_multiply(0.35));
            p.rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, Color32::from_white_alpha(90)));
            glyph(Color32::WHITE);
        }
        Skin::Gold => {
            // a gold coin
            let r = h / 2.0;
            p.circle_filled(rect.center(), r, mix(t.accent, Color32::BLACK, 0.2));
            p.circle_filled(rect.center(), r - 2.0, t.accent);
            p.circle_stroke(rect.center(), r - 4.0, Stroke::new(1.0f32, mix(t.accent, Color32::WHITE, 0.5)));
            glyph(rgb(60, 44, 10));
        }
        Skin::Candy => {
            // a glossy gumdrop
            let r = h / 2.0;
            p.circle_filled(rect.center(), r, color);
            p.add(Shape::rect_filled(Rect::from_min_max(rect.min + egui::vec2(h * 0.22, h * 0.1), egui::pos2(rect.max.x - h * 0.22, rect.min.y + h * 0.42)), h * 0.16, Color32::from_white_alpha(80)));
            glyph(Color32::WHITE);
        }
        Skin::Brutal => {
            p.rect_filled(rect.translate(egui::vec2(3.0, 3.0)), 0.0, s.text);
            p.rect_filled(rect, 0.0, color);
            p.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(2.0f32, s.text));
            glyph(Color32::WHITE);
        }
        Skin::Synth => {
            // a neon outline
            let r = (h * 0.28).round();
            p.add(egui::epaint::Shadow { offset: egui::Vec2::ZERO, blur: 10.0, spread: 0.0, color: color.gamma_multiply(0.6) }.as_shape(rect, r));
            p.rect_filled(rect, r, s.card);
            p.rect_stroke(rect.shrink(1.0), r, Stroke::new(1.5f32, color));
            glyph(color);
        }
        Skin::Winter => {
            // frosted ice with a snow cap
            let r = (h * 0.3).round();
            p.rect_filled(rect, r, mix(s.card, color, 0.35));
            p.rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, Color32::from_white_alpha(120)));
            let cap = Rect::from_min_max(rect.min + egui::vec2(r * 0.5, -1.0), egui::pos2(rect.max.x - r * 0.5, rect.min.y + h * 0.14));
            p.rect_filled(cap, h * 0.07, Color32::from_rgb(246, 251, 255));
            glyph(Color32::WHITE);
        }
        Skin::Spooky => {
            // a little pumpkin
            let r = h / 2.0;
            let c = rect.center() + egui::vec2(0.0, h * 0.04);
            let orange = rgb(249, 115, 22);
            p.circle_filled(c, r * 0.96, orange);
            for dx in [-0.42f32, 0.42] {
                p.circle_stroke(c, r * 0.96, Stroke::new(1.0f32, mix(orange, Color32::BLACK, 0.3)));
                p.line_segment([c + egui::vec2(dx * r, -r * 0.8), c + egui::vec2(dx * r, r * 0.8)], Stroke::new(1.0f32, mix(orange, Color32::BLACK, 0.25)));
            }
            p.rect_filled(Rect::from_center_size(egui::pos2(c.x, rect.min.y + 1.0), egui::vec2(3.0, h * 0.2)), 1.0, rgb(101, 163, 13));
            glyph(rgb(30, 14, 4));
        }
        Skin::Liquid => {
            let r = h / 2.0;
            p.circle_filled(rect.center(), r, color);
            p.add(Shape::rect_filled(Rect::from_min_max(rect.min + egui::vec2(h * 0.2, h * 0.08), egui::pos2(rect.max.x - h * 0.2, rect.min.y + h * 0.45)), h * 0.18, Color32::from_white_alpha(90)));
            p.circle_stroke(rect.center(), r - 0.5, Stroke::new(1.0f32, Color32::from_white_alpha(170)));
            glyph(Color32::WHITE);
        }
        Skin::Field => {
            // a specimen tag with its string hole
            let r = (h * 0.12).round();
            p.rect_filled(rect, r, mix(s.card, color, 0.18));
            p.rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, mix(color, Color32::BLACK, 0.2)));
            p.circle_filled(egui::pos2(rect.max.x - h * 0.16, rect.min.y + h * 0.16), h * 0.06, s.page);
            glyph(mix(color, Color32::BLACK, 0.25));
        }
        Skin::Studio => {
            let r = (h * 0.3).round();
            p.add(gradient_rect(rect, [r; 4], color, mix(color, t.accent2, 0.5)));
            glyph(Color32::WHITE);
        }
        Skin::Companion => {
            let r = h / 2.0;
            p.rect_filled(rect, r, mix(s.card, color, 0.26));
            p.rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, color.gamma_multiply(0.6)));
            glyph(color);
        }
        Skin::Plain | Skin::Pixel => return false,
    }
    true
}

/// The pixel icon for an app's or page's icon (its emoji), if there is one.
pub fn pixel_icon(icon: &str) -> Option<&'static [&'static str; 8]> {
    Some(match icon.trim_end_matches('\u{fe0f}') {
        "✨" => &PX_SPARKLE,
        "🛡" => &PX_FLAG,
        "📦" => &PX_GRID,
        "🐣" => &PX_EGG,
        "🃏" => &PX_CARDS,
        "💥" => &PX_SWORD,
        "📖" => &PX_BOOK,
        "🖵" => &PX_LAYERS,
        "📊" => &PX_BARS,
        "⚙" => &PX_GEAR,
        "🏠" => &PX_HOME,
        "🏪" => &PX_STORE,
        "📚" => &PX_SHELF,
        "🏅" => &PX_MEDAL,
        "🎯" => &PX_TARGET,
        "👁" => &PX_EYE,
        "📜" => &PX_LIST,
        "⭐" => &PX_STAR,
        "🏆" => &PX_TROPHY,
        "🗺" => &PX_PIN_MAP,
        "🎮" => &PX_PAD,
        "🐾" => &PX_PAW,
        "🎬" => &PX_CLAP,
        "📓" => &PX_NOTE,
        "💀" => &PX_SKULL,
        "📌" => &PX_PUSHPIN,
        "🎨" => &PX_PALETTE,
        "⌨" => &PX_KEYS,
        _ => return None,
    })
}

/// A pixel icon chip: a square tile in `color` with a bevel, and the 8×8
/// picture in `ink`, kept on whole pixels so it stays crisp.
pub fn pixel_chip(p: &Painter, rect: Rect, bits: &[&str; 8], color: Color32, ink: Color32) {
    p.rect_filled(rect, 0.0, color);
    let k = (rect.height() * 0.07).round().max(2.0);
    p.rect_filled(Rect::from_min_size(rect.min, egui::vec2(rect.width(), k)), 0.0, Color32::from_white_alpha(70));
    p.rect_filled(Rect::from_min_size(rect.min, egui::vec2(k, rect.height())), 0.0, Color32::from_white_alpha(70));
    p.rect_filled(Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - k), rect.max), 0.0, Color32::from_black_alpha(70));
    p.rect_filled(Rect::from_min_max(egui::pos2(rect.max.x - k, rect.min.y), rect.max), 0.0, Color32::from_black_alpha(70));
    let cell = (rect.height() * 0.6 / 8.0).floor().max(1.0);
    let size = cell * 8.0;
    let o = (rect.center() - egui::vec2(size / 2.0, size / 2.0)).round();
    for (y, row) in bits.iter().enumerate() {
        for (x, b) in row.bytes().enumerate() {
            if b == b'#' {
                p.rect_filled(Rect::from_min_size(o + egui::vec2(x as f32 * cell, y as f32 * cell), egui::vec2(cell, cell)), 0.0, ink);
            }
        }
    }
}

/// A soft round glow: `col` at `alpha` in the middle, fading out by `radius`.
pub fn glow_spot(p: &Painter, c: Pos2, radius: f32, col: Color32, alpha: f32) {
    // (rings: how far out, and how strong there; eased so there's no hard edge)
    let rings = [(0.35f32, 0.62f32), (0.68, 0.24), (1.0, 0.0)];
    let n: u32 = 56;
    // (dark Aurora glass / Glass HUD: mixed in screen space, a touch
    // stronger to make up for it, so the page stays deep with a soft glow)
    let soft = glass_dark();
    let tint = |k: f32| {
        let al = (255.0 * alpha * k * if soft { 1.5 } else { 1.0 }).round().clamp(0.0, 255.0) as u8;
        if soft { see(col, al) } else { Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), al) }
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, tint(1.0));
    for (rf, k) in rings {
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            mesh.colored_vertex(c + egui::vec2(a.cos(), a.sin()) * radius * rf, tint(k));
        }
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    for ring in 0..(rings.len() as u32 - 1) {
        let a0 = 1 + ring * n;
        let b0 = a0 + n;
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.add_triangle(a0 + i, b0 + i, b0 + j);
            mesh.add_triangle(a0 + i, b0 + j, a0 + j);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// The theme's top-to-bottom wash (Synthwave's night sky) over `rect`.
pub fn paint_backdrop(p: &Painter, rect: Rect) {
    if let Some((top, bottom)) = theme_now().and_then(|t| t.backdrop) {
        paint_backdrop_colours(p, rect, top, bottom);
    }
}

/// A top-to-bottom wash from `top` to `bottom` over `rect` (deepening
/// quickly near the bottom, like a sky at dusk).
pub fn paint_backdrop_colours(p: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    {
        let mid = mix(top, bottom, 0.4);
        let my = rect.min.y + rect.height() * 0.7;
        let mut m = egui::Mesh::default();
        for (y, c) in [(rect.min.y, top), (my, mid), (rect.max.y, bottom)] {
            m.colored_vertex(egui::pos2(rect.min.x, y), c);
            m.colored_vertex(egui::pos2(rect.max.x, y), c);
        }
        m.add_triangle(0, 1, 3);
        m.add_triangle(0, 3, 2);
        m.add_triangle(2, 3, 5);
        m.add_triangle(2, 5, 4);
        p.add(Shape::mesh(m));
    }
}

/// The theme's glows, placed over `screen` (the whole window) and drawn with
/// `p` (clip it to the part being painted, so the title bar and the page
/// line up into one).
pub fn paint_glows(p: &Painter, screen: Rect) {
    if let Some((a, b)) = glow() {
        let r = screen.width().max(screen.height()) * 0.62;
        glow_spot(p, screen.min + egui::vec2(screen.width() * 0.15, screen.height() * 0.10), r, a, 0.17);
        glow_spot(p, screen.min + egui::vec2(screen.width() * 0.85, screen.height() * 0.30), r, b, 0.21);
    }
}

/// The colours a look uses, worked out without applying it (the
/// Appearance page previews a choice while you hover it).
#[derive(Clone, Copy)]
pub struct Look {
    pub s: Surf,
    pub light: bool,
    pub accent: Color32,
    pub accent2: Color32,
    pub button: Color32,
    pub button2: Color32,
    pub modern: bool,
    pub clean: bool,
}

pub fn look_of(a: &crate::config::Appearance) -> Look {
    use crate::config::UiMode;
    if let Some(t) = active_theme(a) {
        let c3 = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
        let (accent, button) = if t.own_accent { (c3(a.accent), if a.separate_buttons { c3(a.button) } else { c3(a.accent) }) } else { (t.accent, t.button) };
        let mut s = t.s;
        if t.own_accent {
            s.selected = mix(s.card, accent, 0.3);
        }
        return Look { s, light: t.light, accent, accent2: accent, button, button2: button, modern: true, clean: true };
    }
    let p = match a.mode {
        UiMode::Dark => DARK,
        UiMode::SemiDark => SEMI,
        UiMode::Light => LIGHT,
    };
    let c3 = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    let accent = c3(a.accent);
    let accent2 = if a.gradient { c3(a.accent2) } else { accent };
    let (button, button2) = if a.separate_buttons {
        (c3(a.button), if a.button_gradient { c3(a.button2) } else { c3(a.button) })
    } else {
        (accent, if a.solid_buttons { accent } else { accent2 })
    };
    // (Modern Plus draws every main button in one solid colour)
    let button2 = if a.plus { button } else { button2 };
    let s = match (a.modern_ui, a.clean_settings, a.mode) {
        (true, _, UiMode::Dark) => MODERN_DARK,
        (true, _, UiMode::SemiDark) => MODERN_SEMI,
        (true, _, UiMode::Light) => MODERN_LIGHT,
        (false, true, UiMode::Dark) => CLEAN_DARK,
        (false, true, UiMode::SemiDark) => CLEAN_SEMI,
        (false, true, UiMode::Light) => CLEAN_LIGHT,
        (false, false, _) => Surf {
            sidebar: p.sidebar,
            page: p.page,
            card: p.card,
            card_hover: p.card_hover,
            line: p.line,
            control: p.line,
            selected: mix(p.card, accent, if p.light { 0.16 } else { 0.32 }),
            text: p.text,
            muted: p.muted,
            field: p.widget,
        },
    };
    Look { s, light: p.light, accent, accent2, button, button2, modern: a.modern_ui, clean: a.clean_settings || a.modern_ui }
}

#[allow(non_snake_case)]
pub fn BG() -> Color32 {
    pal().bg
}
#[allow(non_snake_case)]
pub fn PANEL() -> Color32 {
    pal().panel
}
#[allow(non_snake_case)]
pub fn PLATE() -> Color32 {
    pal().plate
}
#[allow(non_snake_case)]
pub fn BORDER() -> Color32 {
    pal().border
}
#[allow(non_snake_case)]
pub fn MUTED() -> Color32 {
    pal().muted
}
/// Gold link text (darker on light backgrounds so it stays readable).
#[allow(non_snake_case)]
pub fn LINK() -> Color32 {
    if pal().light {
        mix(GOLD, Color32::BLACK, 0.4)
    } else {
        GOLD
    }
}
/// The accent colour (red by default).
#[allow(non_snake_case)]
pub fn RED() -> Color32 {
    pal().accent
}
#[allow(non_snake_case)]
pub fn RED_DARK() -> Color32 {
    pal().accent_dark
}

/// A rounded rectangle filled with a left-to-right gradient. Corner radii
/// are (top-left, top-right, bottom-right, bottom-left).
pub fn gradient_rect(rect: Rect, radii: [f32; 4], a: Color32, b: Color32) -> Shape {
    // (the pixel themes square everything off)
    let radii = if plus() && pixel() { [0.0; 4] } else { radii };
    if a == b {
        return Shape::rect_filled(
            rect,
            egui::Rounding { nw: radii[0], ne: radii[1], se: radii[2], sw: radii[3] },
            a,
        );
    }
    // Convex outline traced clockwise, filled as a fan; colour depends only on
    // x, so the linear gradient comes out exact.
    let lim = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let corners = [
        (rect.left_top(), std::f32::consts::PI, egui::vec2(1.0, 1.0), radii[0]),
        (rect.right_top(), -std::f32::consts::FRAC_PI_2, egui::vec2(-1.0, 1.0), radii[1]),
        (rect.right_bottom(), 0.0, egui::vec2(-1.0, -1.0), radii[2]),
        (rect.left_bottom(), std::f32::consts::FRAC_PI_2, egui::vec2(1.0, -1.0), radii[3]),
    ];
    let mut pts: Vec<Pos2> = Vec::new();
    for (corner, start, inward, r) in corners {
        let r = r.min(lim);
        if r <= 0.5 {
            pts.push(corner);
            continue;
        }
        let centre = corner + inward * r;
        let steps = 8;
        for i in 0..=steps {
            let ang = start + std::f32::consts::FRAC_PI_2 * i as f32 / steps as f32;
            pts.push(centre + egui::vec2(ang.cos(), ang.sin()) * r);
        }
    }
    let col = |x: f32| mix(a, b, ((x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0));
    let mut mesh = egui::Mesh::default();
    let c = rect.center();
    mesh.colored_vertex(c, col(c.x));
    for q in &pts {
        mesh.colored_vertex(*q, col(q.x));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    Shape::mesh(mesh)
}

/// The header bar (accent colour or gradient), rounded on top.
pub fn header_bar(p: &Painter, rect: Rect, r: f32) {
    let pl = pal();
    let b = if pl.gradient { pl.accent2 } else { pl.accent };
    p.add(gradient_rect(rect, [r, r, 0.0, 0.0], pl.accent, b));
}

/// The first button colour (for solid fills).
#[allow(non_snake_case)]
pub fn BUTTON() -> Color32 {
    pal().button
}

/// Fill for a button: its colour or gradient, a little brighter on hover.
pub fn button_shape(rect: Rect, r: f32, hovered: bool) -> Shape {
    button_shape_t(rect, r, if hovered { 1.0 } else { 0.0 })
}

/// `button_shape` part-way into its hover (`t` 0..1), for eased hovers.
/// Every theme is either square-cornered or rounded, all the way through:
/// the corner for a control `h` tall in this theme (None: Modern Plus's
/// own). Square: the pixel themes, Terminal, Brutal, Broadcast, Field
/// guide. Rounded: the rest, some as full pills.
pub fn skin_corner(h: f32) -> Option<f32> {
    if !plus() {
        return None;
    }
    let pill = h / 2.0;
    Some(match skin() {
        Skin::Pixel | Skin::Terminal | Skin::Brutal | Skin::Broadcast | Skin::Field => 0.0,
        Skin::Journal => 5.0f32.min(pill),
        Skin::Gold | Skin::Synth => 6.0f32.min(pill),
        Skin::Spooky => 8.0f32.min(pill),
        Skin::Winter | Skin::Studio => 10.0f32.min(pill),
        Skin::Sticker => 12.0f32.min(pill),
        Skin::Companion | Skin::Candy | Skin::Liquid | Skin::Glass | Skin::Aurora => pill,
        Skin::Plain => return None,
    })
}

/// Is this theme a square-cornered one?
pub fn square_theme() -> bool {
    skin_corner(40.0) == Some(0.0)
}

pub fn button_shape_t(rect: Rect, r: f32, t: f32) -> Shape {
    let pl = pal();
    // (one corner for every button in a theme: see skin_corner)
    let r = skin_corner(rect.height()).unwrap_or(r);
    // the overlay themes' buttons
    let tt = t.clamp(0.0, 1.0);
    match skin() {
        Skin::Sticker => {
            // a chunky sticker: ink outline, a hard ink shadow
            let ink = theme_now().map_or(OUTLINE, |th| th.s.text);
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let rr = r.min(12.0);
            let lift = if rect.height() >= 24.0 { 3.5 } else { 2.0 };
            return Shape::Vec(vec![
                Shape::rect_filled(rect.translate(egui::vec2(lift, lift)), rr, ink),
                Shape::rect_filled(rect, rr, fill),
                Shape::rect_stroke(rect.shrink(1.0), rr, Stroke::new(2.0f32, ink)),
            ]);
        }
        Skin::Journal => {
            // a rubber stamp: red, a darker double border
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let edge = mix(pl.button, Color32::BLACK, 0.35);
            let rr = r.min(5.0);
            return Shape::Vec(vec![
                Shape::rect_filled(rect, rr, fill),
                Shape::rect_stroke(rect.shrink(1.0), rr, Stroke::new(2.0f32, edge)),
                Shape::rect_stroke(rect.shrink(4.0), (rr - 2.0).max(0.0), Stroke::new(1.0f32, mix(fill, Color32::WHITE, 0.45))),
            ]);
        }
        Skin::Broadcast => {
            // a slanted scoreboard plate
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let lean = (rect.height() * 0.28).min(9.0);
            return Shape::convex_polygon(
                vec![rect.left_bottom(), egui::pos2(rect.min.x + lean, rect.min.y), rect.right_top(), egui::pos2(rect.max.x - lean, rect.max.y)],
                fill,
                Stroke::NONE,
            );
        }
        Skin::Terminal => {
            return Shape::rect_filled(rect, 0.0, mix(pl.button, Color32::WHITE, 0.2 * tt));
        }
        Skin::Glass => {
            // a ghost button: see-through, a light rim
            let fill = pl.button.gamma_multiply(0.38 + 0.18 * tt);
            return Shape::Vec(vec![Shape::rect_filled(rect, r, fill), Shape::rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, Color32::from_white_alpha(110)))]);
        }
        Skin::Companion => {
            // a soft pill with a pink glow
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let glow = egui::epaint::Shadow { offset: egui::vec2(0.0, 4.0), blur: 14.0, spread: 0.0, color: pl.button.gamma_multiply(0.35 + 0.2 * tt) };
            return Shape::Vec(vec![glow.as_shape(rect, r).into(), Shape::rect_filled(rect, r, fill)]);
        }
        Skin::Studio => {
            // violet into orange (a deeper orange, so white text reads on it)
            let k = 0.12 * tt;
            let b = theme_now().map_or(pl.button, |th| mix(th.accent2, Color32::BLACK, 0.32));
            return gradient_rect(rect, [r; 4], mix(pl.button, Color32::WHITE, k), mix(b, Color32::WHITE, k));
        }
        Skin::Aurora => {
            let b = theme_now().map_or(pl.button, |th| mix(pl.button, th.accent2, 0.45));
            let k = 0.12 * tt;
            return Shape::Vec(vec![
                gradient_rect(rect, [r; 4], mix(pl.button, Color32::WHITE, k), mix(b, Color32::WHITE, k)),
                Shape::rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, Color32::from_white_alpha(90))),
            ]);
        }
        Skin::Field => {
            // a specimen label: teal, a fine inner rule
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let rr = r.min(4.0);
            return Shape::Vec(vec![Shape::rect_filled(rect, rr, fill), Shape::rect_stroke(rect.shrink(3.0), (rr - 1.0).max(0.0), Stroke::new(1.0f32, Color32::from_white_alpha(110)))]);
        }
        Skin::Synth => {
            // neon: cyan into pink, glowing
            let b = theme_now().map_or(pl.button, |th| th.accent2);
            let glow = egui::epaint::Shadow { offset: egui::Vec2::ZERO, blur: 16.0 + 8.0 * tt, spread: 1.0, color: pl.button.gamma_multiply(0.55) };
            let k = 0.1 * tt;
            return Shape::Vec(vec![glow.as_shape(rect, r).into(), gradient_rect(rect, [r; 4], mix(pl.button, Color32::WHITE, k), mix(b, Color32::WHITE, k))]);
        }
        Skin::Gold => {
            // gold leaf: a gilded gradient, a fine dark border and an inner glint
            let b = theme_now().map_or(pl.button, |th| th.accent2);
            let k = 0.1 * tt;
            let rr = r.min(6.0);
            return Shape::Vec(vec![
                gradient_rect(rect, [rr; 4], mix(pl.button, Color32::WHITE, k), mix(mix(pl.button, b, 0.6), Color32::WHITE, k)),
                Shape::rect_stroke(rect.shrink(0.5), rr, Stroke::new(1.0f32, mix(pl.button, Color32::BLACK, 0.45))),
                Shape::rect_stroke(rect.shrink(3.0), (rr - 2.0).max(0.0), Stroke::new(1.0f32, Color32::from_rgba_unmultiplied(255, 248, 220, 120))),
            ]);
        }
        Skin::Candy => {
            // glossy candy: pink into violet with a shine on top
            let b = theme_now().map_or(pl.button, |th| th.accent2);
            let k = 0.1 * tt;
            let shine = Rect::from_min_max(rect.min + egui::vec2(rect.height() * 0.35, 2.0), egui::pos2(rect.max.x - rect.height() * 0.35, rect.center().y));
            return Shape::Vec(vec![
                gradient_rect(rect, [r; 4], mix(pl.button, Color32::WHITE, k), mix(b, Color32::WHITE, k)),
                Shape::rect_filled(shine, shine.height() / 2.0, Color32::from_white_alpha(60)),
            ]);
        }
        Skin::Brutal => {
            // flat, a black outline and a hard black shadow
            let ink = theme_now().map_or(OUTLINE, |th| th.s.text);
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let lift = if rect.height() >= 24.0 { 4.0 } else { 2.0 };
            return Shape::Vec(vec![
                Shape::rect_filled(rect.translate(egui::vec2(lift, lift)), 0.0, ink),
                Shape::rect_filled(rect, 0.0, fill),
                Shape::rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(2.0f32, ink)),
            ]);
        }
        Skin::Liquid => {
            // a glossy drop: blue deepening down, a bright shine, a white rim
            let k = 0.1 * tt;
            let shine = Rect::from_min_max(rect.min + egui::vec2(rect.height() * 0.3, 1.5), egui::pos2(rect.max.x - rect.height() * 0.3, rect.min.y + rect.height() * 0.45));
            return Shape::Vec(vec![
                Shape::rect_filled(rect, r, mix(pl.button, Color32::WHITE, k)),
                Shape::rect_filled(shine, shine.height() / 2.0, Color32::from_white_alpha(80)),
                Shape::rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0f32, Color32::from_white_alpha(170))),
            ]);
        }
        Skin::Winter => {
            // ice, with snow settled along the top
            let fill = mix(pl.button, Color32::WHITE, 0.12 * tt);
            let mut v = vec![Shape::rect_filled(rect, r, fill)];
            if rect.height() >= 20.0 && rect.width() >= 40.0 {
                let snow = Color32::from_rgb(246, 251, 255);
                let cap = Rect::from_min_max(rect.min + egui::vec2(r.min(rect.height() / 2.0) * 0.6, -1.0), egui::pos2(rect.max.x - r.min(rect.height() / 2.0) * 0.6, rect.min.y + 3.5));
                v.push(Shape::rect_filled(cap, 2.0, snow));
                let n = ((cap.width() / 16.0) as i32).max(2);
                for i in 0..n {
                    let x = cap.min.x + cap.width() * (i as f32 + 0.5) / n as f32;
                    v.push(Shape::circle_filled(egui::pos2(x, cap.min.y + 1.5), 2.6 + (i % 2) as f32 * 0.9, snow));
                }
            }
            return Shape::Vec(v);
        }
        Skin::Spooky => {
            // a pumpkin pill, dripping
            let fill = mix(pl.button, Color32::WHITE, 0.1 * tt);
            let drip = mix(pl.button, Color32::BLACK, 0.18);
            let mut v = vec![];
            if tt > 0.0 {
                // a slime-green glow on hover
                v.push(egui::epaint::Shadow { offset: egui::Vec2::ZERO, blur: 14.0, spread: 0.0, color: Color32::from_rgba_unmultiplied(163, 230, 53, (90.0 * tt) as u8) }.as_shape(rect, r).into());
            }
            v.push(Shape::rect_filled(rect, r, fill));
            if rect.height() >= 20.0 && rect.width() >= 50.0 {
                for (fx, len) in [(0.28f32, 5.0f32), (0.55, 8.0), (0.74, 4.0)] {
                    let x = rect.min.x + rect.width() * fx;
                    v.push(Shape::rect_filled(Rect::from_min_size(egui::pos2(x - 2.0, rect.max.y - 2.0), egui::vec2(4.0, len)), 2.0, drip));
                    v.push(Shape::circle_filled(egui::pos2(x, rect.max.y - 1.0 + len), 2.6, drip));
                }
            }
            return Shape::Vec(v);
        }
        Skin::Plain | Skin::Pixel => {}
    }
    // Pixel style: buttons are pixel keys; small fills (pills in tab bars,
    // switches, bars) just lose their round corners
    if pixel() {
        if rect.height() >= 30.0 && rect.width() >= 40.0 {
            return pixel_key(rect, pl.button, t, 0.0);
        }
        return Shape::rect_filled(rect, 0.0, mix(pl.button, Color32::WHITE, 0.14 * t.clamp(0.0, 1.0)));
    }
    let k = 0.14 * t.clamp(0.0, 1.0);
    gradient_rect(
        rect,
        [r; 4],
        mix(pl.button, Color32::WHITE, k),
        mix(pl.button2, Color32::WHITE, k),
    )
}

/// Text colour that reads on the button colour: whichever of dark or white
/// has the stronger contrast (WCAG relative luminance), so mid-bright colours
/// such as the default pink get dark text instead of washed-out white.
pub fn button_ink() -> Color32 {
    // (ghost buttons are see-through over dark glass: white reads)
    if skin() == Skin::Glass {
        return Color32::WHITE;
    }
    let pl = pal();
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let lum = |c: Color32| 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    // Clean style draws buttons flat in the first colour, so judge that alone.
    let l = if clean() && !modern() { lum(pl.button) } else { (lum(pl.button) + lum(pl.button2)) / 2.0 };
    let dark = Color32::from_rgb(30, 28, 44);
    let on_white = 1.05 / (l + 0.05);
    let on_dark = (l + 0.05) / (lum(dark) + 0.05);
    if on_dark >= on_white {
        dark
    } else {
        Color32::WHITE
    }
}

pub fn apply(ctx: &egui::Context) {
    let pl = pal();
    let mut v = if pl.light { Visuals::light() } else { Visuals::dark() };
    v.window_fill = pl.panel;
    v.panel_fill = pl.panel;
    v.extreme_bg_color = pl.extreme;
    v.faint_bg_color = pl.card_hover;
    v.selection.bg_fill = pl.accent;
    v.selection.stroke = Stroke::new(1.0f32, Color32::WHITE);
    v.hyperlink_color = if pl.light { mix(GOLD, Color32::BLACK, 0.35) } else { GOLD };
    v.override_text_color = Some(pl.text);
    for (w, fill) in [
        (&mut v.widgets.inactive, pl.widget),
        (&mut v.widgets.hovered, pl.widget_hover),
        (&mut v.widgets.active, pl.accent),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
    }
    ctx.set_visuals(v);
}

/// White (or coloured) text with a 1px black outline, like the game's plates.
pub fn outlined_text(
    p: &Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    size: f32,
    color: Color32,
) -> Rect {
    let font = pixel_font(size);
    let o = (size / 9.0).clamp(1.0, 2.0);
    for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o), (o, o)] {
        p.text(
            pos + egui::vec2(dx, dy),
            anchor,
            text,
            font.clone(),
            OUTLINE,
        );
    }
    p.text(pos, anchor, text, font, color)
}

/// Slanted name plate (parallelogram) with a dark outline.
pub fn plate(p: &Painter, rect: Rect, fill: Color32) {
    let s = rect.height() * 0.45;
    let pts = vec![
        egui::pos2(rect.min.x + s, rect.min.y),
        egui::pos2(rect.max.x, rect.min.y),
        egui::pos2(rect.max.x - s, rect.max.y),
        egui::pos2(rect.min.x, rect.max.y),
    ];
    p.add(Shape::convex_polygon(
        pts,
        fill,
        Stroke::new(2.0f32, OUTLINE),
    ));
}

/// Battle-platform ellipse under a sprite.
pub fn platform(p: &Painter, center: Pos2, rx: f32, ry: f32) {
    let pts: Vec<Pos2> = (0..36)
        .map(|i| {
            let a = i as f32 / 36.0 * std::f32::consts::TAU;
            center + egui::vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect();
    p.add(Shape::convex_polygon(
        pts,
        GRASS,
        Stroke::new(2.0f32, GRASS_EDGE),
    ));
}

/// Four-point sparkle (original icon, used for "shiny").
pub fn sparkle(p: &Painter, c: Pos2, r: f32, color: Color32) {
    let pts: Vec<Pos2> = (0..8)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::FRAC_PI_4;
            let rr = if i % 2 == 0 { r } else { r * 0.32 };
            c + egui::vec2(a.cos() * rr, a.sin() * rr)
        })
        .collect();
    // Star is concave: draw as 4 triangles from the centre.
    for i in 0..8 {
        let a = pts[i];
        let b = pts[(i + 1) % 8];
        p.add(Shape::convex_polygon(vec![c, a, b], color, Stroke::NONE));
    }
}

/// HP-style bar. `frac` 0..=1. Colour goes green → yellow → red like HP.
pub fn hp_bar(p: &Painter, rect: Rect, frac: f32) {
    let frac = frac.clamp(0.0, 1.0);
    p.rect_filled(rect.expand(2.0), 3.0, OUTLINE);
    p.rect_filled(rect, 2.0, Color32::from_rgb(52, 50, 66));
    let color = if frac > 0.5 {
        GREEN
    } else if frac > 0.2 {
        HP_YELLOW
    } else {
        HP_RED
    };
    if frac > 0.0 {
        let fill = Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height()));
        p.rect_filled(fill, 2.0, color);
        // highlight stripe
        let hl = Rect::from_min_size(fill.min, egui::vec2(fill.width(), rect.height() * 0.35));
        p.rect_filled(hl, 2.0, Color32::from_white_alpha(50));
    }
}

/// Progress gauge in a single colour (used for the odds bar).
pub fn gauge(p: &Painter, rect: Rect, frac: f32, color: Color32) {
    let frac = frac.clamp(0.0, 1.0);
    p.rect_filled(rect.expand(2.0), 3.0, OUTLINE);
    p.rect_filled(rect, 2.0, Color32::from_rgb(52, 50, 66));
    if frac > 0.0 {
        let fill = Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height()));
        p.rect_filled(fill, 2.0, color);
        let hl = Rect::from_min_size(fill.min, egui::vec2(fill.width(), rect.height() * 0.35));
        p.rect_filled(hl, 2.0, Color32::from_white_alpha(50));
    }
}

/// DS-style message box (black with a double grey border).
pub fn message_box(p: &Painter, rect: Rect) {
    p.rect_filled(rect, 6.0, Color32::from_rgb(14, 14, 20));
    p.rect_stroke(rect, 6.0, Stroke::new(2.0f32, BORDER()));
    p.rect_stroke(
        rect.shrink(4.0),
        4.0,
        Stroke::new(1.0f32, Color32::from_rgb(44, 42, 58)),
    );
}

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

fn hexagon(c: Pos2, r: f32) -> Vec<Pos2> {
    (0..6)
        .map(|i| {
            let a = std::f32::consts::FRAC_PI_6 + i as f32 * std::f32::consts::FRAC_PI_3;
            c + egui::vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

/// The trainer level badge: a hexagon in the level's metal with the number.
pub fn level_badge(p: &Painter, c: Pos2, r: f32, level: u32) {
    let (_, main, light) = crate::progress::badge_metal(level);
    let main = Color32::from_rgb(main[0], main[1], main[2]);
    let light = Color32::from_rgb(light[0], light[1], light[2]);
    p.add(Shape::convex_polygon(hexagon(c + egui::vec2(0.0, r * 0.08), r * 1.04), Color32::from_black_alpha(90), Stroke::NONE));
    p.add(Shape::convex_polygon(hexagon(c, r), main, Stroke::new((r * 0.08).max(1.0), OUTLINE)));
    p.add(Shape::convex_polygon(hexagon(c, r * 0.76), mix(main, Color32::BLACK, 0.25), Stroke::new((r * 0.06).max(1.0), light)));
    // shine on the top-left facet
    p.add(Shape::convex_polygon(
        vec![c + egui::vec2(-r * 0.62, -r * 0.30), c + egui::vec2(0.0, -r * 0.70), c + egui::vec2(-r * 0.12, -r * 0.48)],
        light.gamma_multiply(0.55),
        Stroke::NONE,
    ));
    let txt = level.to_string();
    let size = if txt.len() >= 3 { r * 0.55 } else { r * 0.72 };
    outlined_text(p, c + egui::vec2(0.0, r * 0.06), Align2::CENTER_CENTER, &txt, size, Color32::WHITE);
    sparkle(p, c + egui::vec2(r * 0.72, -r * 0.62), (r * 0.22).max(2.5), light);
}

/// An achievement medal: a round medal in the tier's colour with a star
/// (greyed out while locked).
pub fn medal(p: &Painter, c: Pos2, r: f32, rgb: [u8; 3], locked: bool) {
    let mut col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    if locked {
        let g = ((rgb[0] as u32 + rgb[1] as u32 + rgb[2] as u32) / 3) as u8;
        col = Color32::from_rgb(g / 2 + 30, g / 2 + 30, g / 2 + 40);
    }
    let light = mix(col, Color32::WHITE, 0.45);
    // ribbon tails
    let rib = mix(col, Color32::BLACK, 0.35);
    p.add(Shape::convex_polygon(
        vec![c + egui::vec2(-r * 0.75, r * 0.2), c + egui::vec2(-r * 0.25, r * 0.4), c + egui::vec2(-r * 0.45, r * 1.25), c + egui::vec2(-r * 0.8, r * 1.0)],
        rib,
        Stroke::NONE,
    ));
    p.add(Shape::convex_polygon(
        vec![c + egui::vec2(r * 0.25, r * 0.4), c + egui::vec2(r * 0.75, r * 0.2), c + egui::vec2(r * 0.8, r * 1.0), c + egui::vec2(r * 0.45, r * 1.25)],
        rib,
        Stroke::NONE,
    ));
    p.circle_filled(c, r, col);
    p.circle_stroke(c, r, Stroke::new((r * 0.1).max(1.0), OUTLINE));
    p.circle_stroke(c, r * 0.74, Stroke::new((r * 0.08).max(1.0), light));
    if locked {
        // a little padlock
        let body = Rect::from_center_size(c + egui::vec2(0.0, r * 0.15), egui::vec2(r * 0.7, r * 0.55));
        p.rect_filled(body, r * 0.1, light);
        p.circle_stroke(c - egui::vec2(0.0, r * 0.18), r * 0.22, Stroke::new((r * 0.1).max(1.0), light));
    } else {
        sparkle(p, c, r * 0.5, Color32::WHITE);
    }
}

// ── sprite stages (the Pokédex entry's and the Boxes' big art panel)

/// The panel a Pokémon stands on, tinted by its types (`c1`, `c2`), with
/// soft rings behind it at `at` (`big` = the outer ring's size). Each theme
/// draws it its own way: paper and pencil (Field journal), a flat block
/// (pixel themes), a sticker, a terminal window, glass, or a light wash on
/// light themes; the type's dark gradient otherwise.
pub fn art_stage(p: &Painter, rect: Rect, r: f32, c1: Color32, c2: Color32, at: Pos2, big: f32) {
    let ring = |p: &Painter, col: Color32| {
        p.circle_filled(at, big, col);
        p.circle_filled(at, big * 0.72, col);
    };
    let dark_grad = |p: &Painter| {
        p.add(gradient_rect(rect, [r; 4], mix(c1, Color32::BLACK, 0.15), mix(c2, Color32::BLACK, 0.35)));
        ring(p, Color32::from_white_alpha(10));
    };
    let Some(t) = theme_now().filter(|_| plus()) else {
        dark_grad(p);
        return;
    };
    let s = t.s;
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    let clip = p.with_clip_rect(rect);
    match t.skin {
        Skin::Journal => {
            // paper with a watercolour wash of the type, pencil hatching
            // and an inked edge (the sketch pass wobbles it)
            p.add(gradient_rect(rect, [r; 4], mix(s.card, c1, 0.20), mix(s.card, c2, 0.30)));
            let step = 9.0;
            let mut x = rect.min.x - rect.height();
            while x < rect.max.x {
                clip.line_segment([egui::pos2(x, rect.max.y), egui::pos2(x + rect.height(), rect.min.y)], Stroke::new(1.0f32, a(c1, 34)));
                x += step;
            }
            p.circle_filled(at, big * 0.72, a(s.card, 150));
            p.circle_stroke(at, big, Stroke::new(1.2f32, a(s.text, 70)));
            p.circle_stroke(at, big * 0.72, Stroke::new(1.0f32, a(c1, 110)));
            p.rect_stroke(rect, r, Stroke::new(1.4f32, a(s.text, 170)));
        }
        Skin::Pixel => {
            // a flat block, a dithered floor and square rings
            p.rect_filled(rect, 0.0, mix(c1, Color32::BLACK, if t.light { 0.05 } else { 0.25 }));
            let floor = Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - rect.height() * 0.3), rect.max);
            p.rect_filled(floor, 0.0, mix(c2, Color32::BLACK, 0.3));
            let mut y = floor.min.y - 8.0;
            let mut row = 0;
            while y < floor.min.y {
                let mut x = rect.min.x + if row % 2 == 0 { 0.0 } else { 4.0 };
                while x < rect.max.x {
                    p.rect_filled(Rect::from_min_size(egui::pos2(x, y), egui::vec2(4.0, 4.0)), 0.0, mix(c2, Color32::BLACK, 0.3));
                    x += 8.0;
                }
                y += 4.0;
                row += 1;
            }
            p.rect_filled(Rect::from_center_size(at, egui::vec2(big * 1.6, big * 1.6)), 0.0, Color32::from_white_alpha(14));
            p.rect_filled(Rect::from_center_size(at, egui::vec2(big * 1.15, big * 1.15)), 0.0, Color32::from_white_alpha(18));
            p.rect_stroke(rect, 0.0, Stroke::new(2.0f32, mix(s.text, c1, 0.2)));
        }
        Skin::Terminal => {
            // a little terminal window: dark, scanlines, a phosphor edge
            p.rect_filled(rect, r.min(3.0), mix(s.page, c1, 0.10));
            let mut y = rect.min.y;
            while y < rect.max.y {
                clip.line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], Stroke::new(1.0f32, Color32::from_black_alpha(40)));
                y += 3.0;
            }
            p.circle_stroke(at, big, Stroke::new(1.0f32, a(t.accent, 50)));
            p.circle_stroke(at, big * 0.72, Stroke::new(1.0f32, a(t.accent, 80)));
            p.rect_stroke(rect, r.min(3.0), Stroke::new(1.0f32, a(t.accent, 160)));
        }
        Skin::Sticker | Skin::Brutal => {
            // flat colour, a thick ink outline and a hard shadow
            let ink = t.hard_shadow.unwrap_or(s.text);
            p.rect_filled(rect.translate(egui::vec2(4.0, 4.0)), r, ink);
            p.rect_filled(rect, r, mix(c1, Color32::WHITE, if t.light { 0.25 } else { 0.0 }));
            p.circle_filled(at, big * 0.8, mix(c1, Color32::WHITE, if t.light { 0.55 } else { 0.25 }));
            p.circle_stroke(at, big * 0.8, Stroke::new(2.0f32, ink));
            p.rect_stroke(rect, r, Stroke::new(2.5f32, ink));
        }
        Skin::Glass | Skin::Liquid | Skin::Aurora | Skin::Winter => {
            // tinted glass with a sheen across the top
            p.add(gradient_rect(rect, [r; 4], a(mix(s.card, c1, 0.45), 230), a(mix(s.card, c2, 0.30), 230)));
            let top = Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.center().y));
            p.add(gradient_rect(top, [r, r, 0.0, 0.0], Color32::from_white_alpha(if t.light { 60 } else { 22 }), Color32::from_white_alpha(0)));
            ring(p, Color32::from_white_alpha(if t.light { 70 } else { 16 }));
            p.rect_stroke(rect, r, Stroke::new(1.0f32, Color32::from_white_alpha(if t.light { 160 } else { 50 })));
        }
        _ if t.light => {
            // light themes: a light wash of the type, not a dark slab
            p.add(gradient_rect(rect, [r; 4], mix(s.card, c1, 0.32), mix(s.card, c2, 0.45)));
            ring(p, Color32::from_white_alpha(90));
            p.rect_stroke(rect, r, Stroke::new(1.0f32, mix(s.line, c1, 0.3)));
        }
        Skin::Broadcast => {
            dark_grad(p);
            p.rect_filled(Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 5.0, rect.max.y)), 0.0, t.accent);
        }
        Skin::Gold | Skin::Synth => {
            dark_grad(p);
            p.rect_stroke(rect, r, Stroke::new(1.2f32, a(t.accent, 150)));
        }
        _ => dark_grad(p),
    }
}

/// Text drawn on an `art_stage` (white on the dark ones, the theme's ink on
/// light ones), at alpha `al`.
pub fn stage_ink(c1: Color32, al: u8) -> Color32 {
    let base = match theme_now().filter(|_| plus()) {
        None => mix(c1, Color32::BLACK, 0.2),
        Some(t) => match t.skin {
            Skin::Journal => mix(t.s.card, c1, 0.22),
            Skin::Pixel => mix(c1, Color32::BLACK, if t.light { 0.05 } else { 0.25 }),
            Skin::Terminal => mix(t.s.page, c1, 0.10),
            Skin::Sticker | Skin::Brutal => mix(c1, Color32::WHITE, if t.light { 0.25 } else { 0.0 }),
            Skin::Glass | Skin::Liquid | Skin::Aurora | Skin::Winter => mix(t.s.card, c1, 0.45),
            _ if t.light => mix(t.s.card, c1, 0.35),
            _ => mix(c1, Color32::BLACK, 0.2),
        },
    };
    let dark = theme_now().map_or(rgb(20, 20, 28), |t| if t.light { t.s.text } else { rgb(20, 20, 28) });
    let ink = if contrast(Color32::WHITE, base) >= contrast(dark, base) { Color32::WHITE } else { dark };
    Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), al)
}

// ── theme sounds and motion

/// The theme's sound pack (`sound::play_ui`): one per skin, 0 = plain.
pub fn sound_pack() -> u8 {
    pack_of(skin())
}

fn pack_of(s: Skin) -> u8 {
    match s {
        Skin::Plain => 0,
        Skin::Pixel => 1,
        Skin::Sticker => 2,
        Skin::Journal => 3,
        Skin::Broadcast => 4,
        Skin::Terminal => 5,
        Skin::Glass => 6,
        Skin::Companion => 7,
        Skin::Studio => 8,
        Skin::Aurora => 9,
        Skin::Field => 10,
        Skin::Synth => 11,
        Skin::Gold => 12,
        Skin::Candy => 13,
        Skin::Brutal => 14,
        Skin::Liquid => 15,
        Skin::Winter => 16,
        Skin::Spooky => 17,
    }
}

/// Tell the theme sounds a page was opened (its sound replaces the click's).
pub fn page_turned(ctx: &egui::Context) {
    if ctx.cumulative_pass_nr() > 10 {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("theme-page-turn"), true));
    }
}

/// The theme's sounds, once a frame at its end: a click on anything with a
/// pointing hand (played a frame late, so a page that opens plays its own
/// sound instead), a soft tick when the pointer comes onto one, and a page
/// sound. Off unless Appearance → Theme → Theme sounds is on.
pub fn ui_sounds(ctx: &egui::Context, on: bool, volume: f32, device: &str) {
    // (pointing last frame, time of the last hover tick, a click waiting)
    static ST: std::sync::Mutex<(bool, f64, bool)> = std::sync::Mutex::new((false, 0.0, false));
    let turn_id = egui::Id::new("theme-page-turn");
    let turned = ctx.data(|d| d.get_temp::<bool>(turn_id)).unwrap_or(false);
    if turned {
        ctx.data_mut(|d| d.remove::<bool>(turn_id));
    }
    let Ok(mut st) = ST.lock() else { return };
    if !on || !plus() {
        *st = (false, 0.0, false);
        return;
    }
    let pack = sound_pack();
    let pointing = ctx.output(|o| o.cursor_icon) == egui::CursorIcon::PointingHand;
    let released = ctx.input(|i| i.pointer.primary_released());
    let now = ctx.input(|i| i.time);
    use crate::sound::{play_ui, UiSnd};
    if turned {
        play_ui(pack, UiSnd::Page, volume, device);
        st.2 = false;
    } else if st.2 {
        play_ui(pack, UiSnd::Click, volume, device);
        st.2 = false;
    }
    if released && pointing {
        st.2 = true;
        ctx.request_repaint();
    } else if pointing && !st.0 && now - st.1 > 0.09 && !ctx.input(|i| i.pointer.any_down()) {
        play_ui(pack, UiSnd::Hover, volume, device);
        st.1 = now;
    }
    st.0 = pointing;
}

/// The themes' page transitions (Appearance → Theme → Page transitions).
static TRANSITIONS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// How long a page takes to come in: the themed transitions take a touch
/// longer, so they can be seen.
pub fn enter_len() -> f32 {
    if plus() && theme_now().is_some() && TRANSITIONS_ON.load(std::sync::atomic::Ordering::Relaxed) {
        0.5
    } else {
        0.32
    }
}

/// How a page comes in, by theme: its opacity and how far it still has to
/// rise, `t` 0..1 (eased) and `rise` the usual lift.
pub fn enter_style(t: f32, rise: f32) -> (f32, f32) {
    if !TRANSITIONS_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return (t, rise * (1.0 - t));
    }
    match skin() {
        // pixel themes: in four hard steps
        Skin::Pixel => {
            let q = (t * 4.0).ceil() / 4.0;
            (q, (rise * (1.0 - q) / 4.0).round() * 4.0)
        }
        // terminal: flickers on like an old screen, no lift
        Skin::Terminal => (if t < 0.4 { if ((t * 30.0) as i32) % 2 == 0 { 0.25 } else { 0.85 } } else { t.max(0.85) }, 0.0),
        // sticker, brutal, candy: dropped in with a bounce
        Skin::Sticker | Skin::Brutal | Skin::Candy => (t.min(1.0) * 1.5, rise * 1.6 * (1.0 - t) * (t * std::f32::consts::PI * 2.5).cos().abs()),
        // glass, aurora, liquid, winter: floats up slowly
        Skin::Glass | Skin::Aurora | Skin::Liquid | Skin::Winter => (t.powf(1.4), rise * 1.8 * (1.0 - t)),
        // spooky: materialises, flickering like a candle
        Skin::Spooky => (t * (0.55 + 0.45 * (t * 47.0).sin().abs()), rise * (1.0 - t)),
        // journal, broadcast, synth, gold: their wipe does the work
        Skin::Journal | Skin::Broadcast | Skin::Synth => (t.max(0.35), 0.0),
        _ => (t, rise * (1.0 - t)),
    }
}

/// The wipes some themes open a page with, painted over the page: a paper
/// edge turning (Field journal), a stripe (Broadcast), a neon scan line
/// (Synthwave), a shine (Gold), a line typing down (Terminal).
pub fn paint_enter(ctx: &egui::Context, area: Rect, t: f32) {
    if t >= 1.0 || !plus() || !motion() || !TRANSITIONS_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Some(th) = theme_now() else { return };
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("theme-enter"))).with_clip_rect(area);
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    match th.skin {
        Skin::Journal => {
            // the old page is ripped off and flung away, showing the new one
            let off = egui::vec2(t * area.width() * 0.9, -t * area.height() * 0.3);
            torn_sheet(&p, area.translate(off), &th);
        }
        Skin::Broadcast => {
            let x = area.min.x - 120.0 + (area.width() + 240.0) * t;
            p.rect_filled(Rect::from_min_max(egui::pos2(x, area.min.y), area.max), 0.0, a(th.s.page, (255.0 * (1.0 - t)) as u8));
            p.rect_filled(Rect::from_min_max(egui::pos2(x - 18.0, area.min.y), egui::pos2(x, area.max.y)), 0.0, th.accent);
            p.rect_filled(Rect::from_min_max(egui::pos2(x - 30.0, area.min.y), egui::pos2(x - 22.0, area.max.y)), 0.0, a(th.accent2, 200));
        }
        Skin::Synth => {
            let y = area.min.y + area.height() * t;
            p.rect_filled(Rect::from_min_max(egui::pos2(area.min.x, y), area.max), 0.0, a(th.s.page, 230));
            for (w, al) in [(14.0f32, 30u8), (6.0, 70), (2.0, 255)] {
                p.line_segment([egui::pos2(area.min.x, y), egui::pos2(area.max.x, y)], Stroke::new(w, a(th.accent, al)));
            }
        }
        Skin::Terminal => {
            let y = area.min.y + area.height() * t;
            p.rect_filled(Rect::from_min_max(egui::pos2(area.min.x, y), area.max), 0.0, th.s.page);
            p.rect_filled(Rect::from_min_size(egui::pos2(area.min.x + 12.0, y - 16.0), egui::vec2(9.0, 16.0)), 0.0, th.accent);
        }
        Skin::Pixel => {
            // the old screen breaks up into blocks, a few at a time
            let b = 24.0;
            let mut y = area.min.y;
            let mut j = 0u32;
            while y < area.max.y {
                let mut x = area.min.x;
                let mut i = 0u32;
                while x < area.max.x {
                    if hash01(i, 31 + j) > t {
                        p.rect_filled(Rect::from_min_size(egui::pos2(x, y), egui::vec2(b, b)), 0.0, th.s.page);
                    }
                    x += b;
                    i += 1;
                }
                y += b;
                j += 1;
            }
        }
        Skin::Sticker => {
            // comic halftone dots shrinking away
            let g = 26.0;
            let rad = g * 0.75 * (1.0 - t);
            let mut y = area.min.y;
            let mut row = 0;
            while y < area.max.y + g {
                let mut x = area.min.x + if row % 2 == 0 { 0.0 } else { g / 2.0 };
                while x < area.max.x + g {
                    p.circle_filled(egui::pos2(x, y), rad, th.s.page);
                    p.circle_stroke(egui::pos2(x, y), rad, Stroke::new(1.5f32, a(th.hard_shadow.unwrap_or(th.s.text), (200.0 * (1.0 - t)) as u8)));
                    x += g;
                }
                y += g * 0.87;
                row += 1;
            }
        }
        Skin::Brutal => {
            // slabs drop away, one after another
            let n = 5;
            let w = area.width() / n as f32;
            let ink = th.hard_shadow.unwrap_or(th.s.text);
            for i in 0..n {
                let k = ((t - i as f32 * 0.08) / 0.6).clamp(0.0, 1.0);
                let drop = k * k * (area.height() + 40.0);
                let r = Rect::from_min_size(egui::pos2(area.min.x + i as f32 * w, area.min.y + drop), egui::vec2(w + 1.0, area.height()));
                p.rect_filled(r, 0.0, if i % 2 == 0 { th.s.page } else { mix(th.s.page, th.accent, 0.12) });
                p.line_segment([r.left_top(), r.right_top()], Stroke::new(4.0f32, ink));
            }
        }
        Skin::Glass | Skin::Aurora => {
            // frost clears from the middle out, a sheen passing down
            p.rect_filled(area, 0.0, a(th.s.page, (230.0 * (1.0 - t)) as u8));
            let y = area.min.y + area.height() * t;
            p.add(gradient_rect(Rect::from_min_max(egui::pos2(area.min.x, y - 80.0), egui::pos2(area.max.x, y)), [0.0; 4], Color32::TRANSPARENT, Color32::from_white_alpha(if th.light { 70 } else { 26 })));
            if let Some((g1, _)) = th.glow {
                glow_spot(&p, area.center(), area.width() * 0.6 * (0.4 + t), g1, 0.2 * (1.0 - t));
            }
        }
        Skin::Liquid => {
            // a ripple spreads from the middle
            p.rect_filled(area, 0.0, a(th.s.page, (220.0 * (1.0 - t)) as u8));
            let c = area.center();
            let far = area.width().max(area.height()) * 0.75;
            for (k, w) in [(1.0f32, 6.0f32), (0.75, 3.0), (0.5, 2.0)] {
                let r = far * t * k;
                p.circle_stroke(c, r, Stroke::new(w, a(th.accent, (180.0 * (1.0 - t)) as u8)));
            }
        }
        Skin::Winter => {
            // the frost melts back into the corners
            p.rect_filled(area, 0.0, a(th.s.page, (200.0 * (1.0 - t)) as u8));
            let ice = if th.light { rgb(90, 150, 210) } else { rgb(225, 240, 255) };
            for corner in [area.left_top(), area.right_top(), area.left_bottom(), area.right_bottom()] {
                let to_c = (area.center() - corner).normalized();
                let base = to_c.y.atan2(to_c.x);
                for b in 0..6u32 {
                    let ang = base + (b as f32 - 2.5) * 0.25;
                    let len = (120.0 + 140.0 * hash01(b, 40)) * (1.0 - t);
                    p.line_segment([corner, corner + egui::vec2(ang.cos(), ang.sin()) * len], Stroke::new(1.4f32, a(ice, 170)));
                }
            }
        }
        Skin::Spooky => {
            // the lights flicker back on
            let flick = 0.6 + 0.4 * (t * 40.0).sin().abs();
            p.rect_filled(area, 0.0, Color32::from_black_alpha((235.0 * (1.0 - t) * flick) as u8));
        }
        Skin::Candy => {
            // a candy-striped curtain slides off to the side
            let edge = area.min.x + area.width() * t;
            let cover = p.with_clip_rect(Rect::from_min_max(egui::pos2(edge, area.min.y), area.max).intersect(area));
            let mut x = area.min.x - area.height() + t * 160.0;
            let mut i = 0;
            while x < area.max.x + area.height() {
                let pts = vec![egui::pos2(x, area.max.y), egui::pos2(x + 40.0, area.max.y), egui::pos2(x + 40.0 + area.height(), area.min.y), egui::pos2(x + area.height(), area.min.y)];
                cover.add(Shape::convex_polygon(pts, if i % 2 == 0 { th.accent } else { th.s.page }, Stroke::NONE));
                x += 40.0;
                i += 1;
            }
            p.line_segment([egui::pos2(edge, area.min.y), egui::pos2(edge, area.max.y)], Stroke::new(3.0f32, Color32::WHITE));
        }
        Skin::Companion => {
            // bubbles pop, showing the page beneath
            let g = 36.0;
            let mut y = area.min.y;
            let mut j = 0u32;
            while y < area.max.y + g {
                let mut x = area.min.x;
                let mut i = 0u32;
                while x < area.max.x + g {
                    let pop = (t * 1.4 - hash01(i, 50 + j) * 0.4).clamp(0.0, 1.0);
                    if pop < 1.0 {
                        let r = g * 0.72 * (1.0 - pop * pop);
                        p.circle_filled(egui::pos2(x, y), r, th.s.page);
                    }
                    x += g;
                    i += 1;
                }
                y += g;
                j += 1;
            }
        }
        Skin::Studio => {
            // a curtain lifts, its edge in the theme's colours
            let y = area.max.y - area.height() * t;
            p.rect_filled(Rect::from_min_max(area.min, egui::pos2(area.max.x, y)), 0.0, th.s.page);
            p.add(gradient_h(Rect::from_min_max(egui::pos2(area.min.x, y - 3.0), egui::pos2(area.max.x, y)), th.accent, th.accent2));
        }
        Skin::Field => {
            // the map unfolds: two halves part from the middle
            let half = area.width() / 2.0 * (1.0 - t);
            let l = Rect::from_min_max(area.min, egui::pos2(area.min.x + half, area.max.y));
            let r = Rect::from_min_max(egui::pos2(area.max.x - half, area.min.y), area.max);
            for side in [l, r] {
                p.rect_filled(side, 0.0, th.s.page);
                let mut y = side.min.y + 20.0;
                while y < side.max.y {
                    p.line_segment([egui::pos2(side.min.x, y), egui::pos2(side.max.x, y)], Stroke::new(1.0f32, a(th.accent, 40)));
                    y += 24.0;
                }
            }
            p.line_segment([l.right_top(), l.right_bottom()], Stroke::new(2.0f32, th.accent));
            p.line_segment([r.left_top(), r.left_bottom()], Stroke::new(2.0f32, th.accent));
        }
        Skin::Gold => {
            // a shine sweeping across
            let x = area.min.x - 200.0 + (area.width() + 400.0) * t;
            for (off, w, al) in [(0.0f32, 90.0f32, 18u8), (20.0, 30.0, 36)] {
                let pts = vec![
                    egui::pos2(x + off, area.min.y),
                    egui::pos2(x + off + w, area.min.y),
                    egui::pos2(x + off + w - 160.0, area.max.y),
                    egui::pos2(x + off - 160.0, area.max.y),
                ];
                p.add(Shape::convex_polygon(pts, Color32::from_rgba_unmultiplied(255, 236, 170, al), Stroke::NONE));
            }
        }
        _ => {}
    }
    ctx.request_repaint();
}

/// A left-to-right gradient.
#[allow(dead_code)]
fn gradient_h(rect: Rect, a: Color32, b: Color32) -> Shape {
    let mut m = egui::Mesh::default();
    m.colored_vertex(rect.left_top(), a);
    m.colored_vertex(rect.right_top(), b);
    m.colored_vertex(rect.right_bottom(), b);
    m.colored_vertex(rect.left_bottom(), a);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    Shape::mesh(m)
}

/// Does this theme have a moving background (its own, besides particles)?
pub fn has_ambient(s: Skin) -> bool {
    matches!(s, Skin::Synth | Skin::Gold | Skin::Aurora | Skin::Glass | Skin::Liquid | Skin::Terminal | Skin::Candy | Skin::Companion)
}

/// The themes' slow background motion: Synthwave's grid rolling to you,
/// Gold's glints twinkling, the glass themes' glows drifting, Terminal's
/// scan band, Candy and Companion's bubbles. Still with Animations or
/// Background effects off. True when it wants another frame soon.
pub fn paint_ambient(p: &Painter, area: Rect, time: f64) -> bool {
    let Some(th) = theme_now().filter(|_| plus()) else { return false };
    let moving = motion() && PARTICLES_ON.load(std::sync::atomic::Ordering::Relaxed);
    let t = if moving { time as f32 } else { 0.0 };
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    let h = |i: u32, k: u32| -> f32 {
        let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        (x % 10_000) as f32 / 10_000.0
    };
    match th.skin {
        Skin::Synth => {
            // the grid's cross lines, running towards you
            let horizon = area.max.y - area.height() * 0.32;
            let depth = area.max.y - horizon;
            let ph = (t * 0.5).fract();
            let n = 12;
            for k in 0..n {
                let u = (k as f32 + ph) / n as f32;
                let y = horizon + depth * u * u;
                p.line_segment([egui::pos2(area.min.x, y), egui::pos2(area.max.x, y)], Stroke::new(1.0f32, hue_turned(Color32::from_rgba_unmultiplied(244, 114, 182, (20.0 + 40.0 * u) as u8))));
            }
            moving
        }
        Skin::Gold => {
            for i in 0..14u32 {
                let c = egui::pos2(area.min.x + area.width() * h(i, 1), area.min.y + area.height() * h(i, 2));
                let k = ((t * (0.6 + 0.8 * h(i, 3)) + h(i, 4) * 6.28).sin()).max(0.0).powi(6);
                if k > 0.02 || !moving {
                    let k = if moving { k } else { 0.4 * h(i, 5) };
                    sparkle(p, c, 3.0 + 4.0 * h(i, 6), a(th.accent, (200.0 * k) as u8));
                }
            }
            moving
        }
        Skin::Aurora | Skin::Glass | Skin::Liquid => {
            if let Some((g1, g2)) = th.glow {
                let r = area.width().max(area.height()) * 0.45;
                let c1 = egui::pos2(area.min.x + area.width() * (0.5 + 0.35 * (t * 0.06).sin()), area.min.y + area.height() * (0.55 + 0.2 * (t * 0.045).cos()));
                let c2 = egui::pos2(area.min.x + area.width() * (0.3 + 0.3 * (t * 0.05 + 2.0).cos()), area.min.y + area.height() * (0.75 + 0.15 * (t * 0.07).sin()));
                glow_spot(p, c1, r, g1, 0.10);
                glow_spot(p, c2, r * 0.8, g2, 0.08);
            }
            moving
        }
        Skin::Terminal => {
            let y = area.min.y + area.height() * (t * 0.07).fract();
            let band = Rect::from_min_max(egui::pos2(area.min.x, y - 50.0), egui::pos2(area.max.x, y));
            p.add(gradient_rect(band, [0.0; 4], Color32::TRANSPARENT, a(th.accent, 12)));
            moving
        }
        Skin::Candy | Skin::Companion => {
            // bubbles drifting up
            for i in 0..18u32 {
                let travel = area.height() + 60.0;
                let off = (h(i, 1) * travel + t * (8.0 + 12.0 * h(i, 2))).rem_euclid(travel);
                let y = area.max.y + 30.0 - off;
                let x = area.min.x + area.width() * h(i, 3) + (t * 0.5 + h(i, 4) * 6.0).sin() * 12.0;
                let r = 4.0 + 10.0 * h(i, 5);
                let col = if h(i, 6) > 0.5 { th.accent } else { th.accent2 };
                p.circle_stroke(egui::pos2(x, y), r, Stroke::new(1.2f32, a(col, 46)));
            }
            moving
        }
        _ => false,
    }
}

// ── startup animations: one per theme (Appearance → Theme → Startup animation)

/// How long a theme's startup animation runs (seconds; the last half
/// second is its way out).
pub const INTRO_LEN: f32 = 2.6;
const INTRO_OUT: f32 = 0.5;

/// (when it started, by the app's clock; done or skipped; its sound
/// played; asked for again from the Theme card)
static INTRO: std::sync::Mutex<(Option<f64>, bool, bool, bool)> = std::sync::Mutex::new((None, false, false, false));

/// Today's rare grass spawns (name, where), rarest first, for Broadcast's
/// news ticker; filled in by the app while the animation plays.
static INTRO_NEWS: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// Is Broadcast's startup animation playing without its news yet?
pub fn intro_needs_news() -> bool {
    let playing = INTRO.lock().is_ok_and(|st| !st.1);
    let broadcast = INTRO_PREVIEW.lock().ok().and_then(|p| *p).or_else(theme_now).is_some_and(|t| t.skin == Skin::Broadcast);
    playing && broadcast && INTRO_NEWS.lock().is_ok_and(|n| n.is_empty())
}

pub fn set_intro_news(news: Vec<(String, String)>) {
    if let Ok(mut n) = INTRO_NEWS.lock() {
        *n = news;
    }
}

fn intro_news() -> Vec<(String, String)> {
    INTRO_NEWS.lock().map(|n| n.clone()).unwrap_or_default()
}

/// A theme whose startup animation is being previewed (not the one in use).
static INTRO_PREVIEW: std::sync::Mutex<Option<ThemeDef>> = std::sync::Mutex::new(None);

/// A theme's sound pack, for previews (the debug tools').
pub fn sound_pack_for(t: crate::config::PlusTheme) -> u8 {
    theme_def(t).map_or(0, |d| pack_of(d.skin))
}

/// Play the theme's startup animation again (the Theme card's preview).
pub fn replay_intro() {
    if let Ok(mut p) = INTRO_PREVIEW.lock() {
        *p = None;
    }
    if let Ok(mut st) = INTRO.lock() {
        *st = (None, false, false, true);
    }
}

/// Preview any theme's startup animation (the ▶ on its tile), in the light
/// or dark and colours `a` would give it.
pub fn preview_intro(a: &crate::config::Appearance) {
    let t = match active_theme(a) {
        Some(t) => t,
        None => {
            let lk = look_of(a);
            yours_def(lk.s, lk.light, lk.accent, lk.accent2, lk.button)
        }
    };
    if let Ok(mut p) = INTRO_PREVIEW.lock() {
        *p = Some(t);
    }
    if let Ok(mut st) = INTRO.lock() {
        *st = (None, false, false, true);
    }
}

fn eo(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}
fn eio(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}
/// Overshoots a little, then settles.
fn back(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    1.0 + 2.701_58 * x * x * x + 1.701_58 * x * x
}
fn al(c: Color32, k: f32) -> Color32 {
    c.gamma_multiply(k.clamp(0.0, 1.0))
}
fn rgba(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
/// A rectangle turned by `ang` radians about its centre.
fn turned(c: Pos2, w: f32, h: f32, ang: f32) -> Vec<Pos2> {
    let (s, co) = ang.sin_cos();
    [(-w, -h), (w, -h), (w, h), (-w, h)].iter().map(|(x, y)| egui::pos2(c.x + (x * co - y * s) / 2.0, c.y + (x * s + y * co) / 2.0)).collect()
}
fn hash01(i: u32, k: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x % 10_000) as f32 / 10_000.0
}

/// Text colour `fg` made readable on `bg`: nudged lighter or darker
/// (whichever reads better) until it reaches 4.5:1, or 3:1 when big.
fn ink_on(fg: Color32, bg: Color32, size: f32) -> Color32 {
    let [r, g, b, a] = fg.to_srgba_unmultiplied();
    let solid = Color32::from_rgb(r, g, b);
    let [br, bg_, bb, _] = bg.to_srgba_unmultiplied();
    let bg = Color32::from_rgb(br, bg_, bb);
    let min = if size >= 24.0 { 3.0 } else { 4.5 };
    let to = if contrast(Color32::WHITE, bg) >= contrast(Color32::BLACK, bg) { Color32::WHITE } else { Color32::BLACK };
    let c = reach(solid, bg, min, to);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

struct Fx<'a> {
    p: &'a Painter,
    r: Rect,
    c: Pos2,
    /// seconds in
    s: f32,
    /// 1 → 0 over the way out
    out: f32,
    th: ThemeDef,
}

impl Fx<'_> {
    /// 0..1 between `a` and `b` seconds.
    fn seg(&self, a: f32, b: f32) -> f32 {
        ((self.s - a) / (b - a)).clamp(0.0, 1.0)
    }
    /// The theme's own face (this theme's, even when it's only a preview).
    fn font(&self, size: f32) -> egui::FontId {
        if matches!(self.th.skin, Skin::Candy | Skin::Brutal) && size < 24.0 {
            return crate::fonts::black_font(size);
        }
        self.th.font.map(|f| f(size)).unwrap_or_else(|| egui::FontId::proportional(size))
    }
    fn fill(&self, col: Color32) {
        self.p.rect_filled(self.r, 0.0, col);
    }
    fn text(&self, at: Pos2, s: &str, size: f32, col: Color32) -> Rect {
        self.p.text(at, Align2::CENTER_CENTER, s, self.font(size), col)
    }
    fn text_f(&self, at: Pos2, s: &str, font: egui::FontId, col: Color32) -> Rect {
        self.p.text(at, Align2::CENTER_CENTER, s, font, col)
    }
    fn w(&self) -> f32 {
        self.r.width()
    }
    fn h(&self) -> f32 {
        self.r.height()
    }
}

/// The theme's startup animation, over the whole window as the app opens:
/// each theme's own little show (a Pokédex opening, a Game Boy boot, a
/// terminal booting, a sunrise over a neon grid...). A click or key skips
/// it. `snd`: the theme sounds' volume and output, if they're on.
pub fn paint_intro(ctx: &egui::Context, on: bool, snd: Option<(f32, &str)>) {
    let Ok(mut st) = INTRO.lock() else { return };
    let preview = INTRO_PREVIEW.lock().ok().and_then(|p| *p);
    // (a preview plays whatever the switches say)
    if st.1 || (preview.is_none() && (!on || !plus() || !motion())) {
        return;
    }
    // (Your colours has one too, in your colours)
    let th = match preview.or_else(theme_now) {
        Some(t) => t,
        None => {
            let p = pal();
            yours_def(surf(), p.light, p.accent, p.accent2, p.button)
        }
    };
    let now = ctx.input(|i| i.time);
    // (only as the app opens: picking a theme later doesn't play it)
    if st.0.is_none() && now > 6.0 && !st.3 {
        st.1 = true;
        return;
    }
    let start = *st.0.get_or_insert(now);
    let s = (now - start) as f32;
    if s >= INTRO_LEN {
        st.1 = true;
        if let Ok(mut p) = INTRO_PREVIEW.lock() {
            *p = None;
        }
        return;
    }
    let skip = ctx.input(|i| i.pointer.any_pressed() || i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. })));
    if s > 0.2 && skip {
        st.1 = true;
        if let Ok(mut p) = INTRO_PREVIEW.lock() {
            *p = None;
        }
        return;
    }
    // the theme's sound pack plays one sound at the big moment
    let ding = match th.skin {
        Skin::Terminal => 1.55,
        Skin::Pixel => 1.25,
        _ => 0.9,
    };
    if let Some((vol, dev)) = snd {
        if s >= ding && !st.2 {
            st.2 = true;
            crate::sound::play_ui(pack_of(th.skin), crate::sound::UiSnd::Page, vol, dev);
        }
    }
    drop(st);
    let r = ctx.screen_rect();
    let layer = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("theme-intro")));
    let p = layer.with_clip_rect(r);
    let out = ((INTRO_LEN - s) / INTRO_OUT).clamp(0.0, 1.0);
    let fx = Fx { p: &p, r, c: r.center(), s, out, th };
    match th.name {
        "Pokédex" => intro_dex(&fx),
        "Pocket" => intro_pocket(&fx),
        "Retro DS" => intro_ds(&fx),
        "Studio" => intro_studio(&fx),
        "Aurora glass" => intro_aurora(&fx),
        "Field guide" => intro_field(&fx),
        "Synthwave" => intro_synth(&fx),
        "Gold" => intro_gold(&fx),
        "Companion" => intro_companion(&fx),
        "Candy" => intro_candy(&fx),
        "Brutal" => intro_brutal(&fx),
        "Liquid" => intro_liquid(&fx),
        "Winter" => intro_winter(&fx),
        "Spooky" => intro_spooky(&fx),
        "Sticker" => intro_sticker(&fx),
        "Glass HUD" => intro_hud(&fx),
        "Field journal" => intro_journal(&fx),
        "Broadcast" => intro_broadcast(&fx),
        "Terminal" => intro_terminal(&fx),
        _ => intro_yours(&fx),
    }
    ctx.request_repaint();
}

/// Pokédex: the red lid with its big blue lens, the lights blinking on,
/// data loading, then the two halves part to open the app.
fn intro_dex(f: &Fx) {
    let open = eio(f.seg(1.55, 2.45));
    let half = f.h() / 2.0;
    let shift = open * (half + 20.0);
    let red = rgb(220, 38, 38);
    let dark = rgb(150, 20, 24);
    let top = Rect::from_min_max(egui::pos2(f.r.min.x, f.r.min.y - shift), egui::pos2(f.r.max.x, f.c.y - shift));
    let bot = Rect::from_min_max(egui::pos2(f.r.min.x, f.c.y + shift), egui::pos2(f.r.max.x, f.r.max.y + shift));
    f.p.rect_filled(top, 0.0, red);
    f.p.rect_filled(bot, 0.0, dark);
    // the hinge
    f.p.rect_filled(Rect::from_min_max(egui::pos2(top.min.x, top.max.y - 12.0), top.max), 0.0, rgb(24, 24, 28));
    f.p.rect_filled(Rect::from_min_max(bot.min, egui::pos2(bot.max.x, bot.min.y + 12.0)), 0.0, rgb(24, 24, 28));
    // the lens, pulsing, and the three lights
    let lens = egui::pos2(top.min.x + 96.0, top.min.y + 92.0);
    f.p.circle_filled(lens, 54.0, Color32::WHITE);
    f.p.circle_filled(lens, 46.0, rgb(24, 24, 28));
    let pulse = 0.6 + 0.4 * (f.s * 9.0).sin().abs();
    f.p.circle_filled(lens, 40.0, mix(rgb(30, 90, 200), rgb(110, 190, 255), pulse * eo(f.seg(0.05, 0.4))));
    f.p.circle_filled(lens + egui::vec2(-13.0, -13.0), 11.0, Color32::from_white_alpha(170));
    for (i, col) in [rgb(255, 70, 70), rgb(250, 210, 40), rgb(70, 220, 110)].iter().enumerate() {
        let lit = f.s > 0.25 + i as f32 * 0.14;
        let c = egui::pos2(lens.x + 90.0 + i as f32 * 34.0, top.min.y + 54.0);
        f.p.circle_filled(c, 11.0, rgb(24, 24, 28));
        f.p.circle_filled(c, 8.0, if lit { *col } else { mix(*col, Color32::BLACK, 0.6) });
    }
    // the name, and data loading on the lower half
    let name_k = eo(f.seg(0.5, 0.9));
    f.text(egui::pos2(f.c.x, top.max.y - 70.0 - 14.0 * (1.0 - name_k)), "POKéDEX", 46.0, al(ink_on(Color32::WHITE, rgb(220, 38, 38), 46.0), name_k));
    f.text(egui::pos2(f.c.x, top.max.y - 34.0), "POKE TOGETHER COMPANION", 15.0, al(ink_on(rgb(255, 210, 210), rgb(220, 38, 38), 15.0), name_k));
    let bar = Rect::from_center_size(egui::pos2(f.c.x, bot.min.y + 80.0), egui::vec2(320.0, 26.0));
    f.p.rect_filled(bar.expand(4.0), 0.0, rgb(24, 24, 28));
    let k = f.seg(0.55, 1.45);
    let cells = 16;
    for i in 0..cells {
        if (i as f32) < k * cells as f32 {
            let w = bar.width() / cells as f32;
            f.p.rect_filled(Rect::from_min_size(egui::pos2(bar.min.x + i as f32 * w + 1.0, bar.min.y), egui::vec2(w - 2.0, bar.height())), 0.0, rgb(70, 220, 110));
        }
    }
    let msg = if k < 1.0 { "LOADING DATA..." } else { "READY!" };
    f.text(egui::pos2(f.c.x, bar.max.y + 30.0), msg, 16.0, ink_on(rgb(255, 220, 220), rgb(150, 20, 24), 16.0));
}

/// Pocket: a Game Boy switching on: the logo scrolls down the green
/// screen, settles, a ding, then the screen fades.
fn intro_pocket(f: &Fx) {
    let k = f.out;
    let screen = rgb(155, 188, 15);
    let ink = rgb(15, 56, 15);
    f.fill(al(screen, k));
    // the screen's dot grid
    let mut y = f.r.min.y;
    while y < f.r.max.y {
        f.p.line_segment([egui::pos2(f.r.min.x, y), egui::pos2(f.r.max.x, y)], Stroke::new(1.0f32, al(rgb(139, 172, 15), k * 0.5)));
        y += 4.0;
    }
    // scrolls down (in whole pixels, like the real thing)
    let u = f.seg(0.1, 1.2);
    let ly = (f.r.min.y - 40.0 + (f.c.y - 20.0 - f.r.min.y + 40.0) * u).round();
    f.text(egui::pos2(f.c.x, ly), "Poke Together", 40.0, al(ink, k));
    if f.s > 1.25 {
        let b = eo(f.seg(1.25, 1.5));
        f.text(egui::pos2(f.c.x, f.c.y + 30.0), "COMPANION", 18.0, al(ink_on(rgb(48, 98, 48), rgb(155, 188, 15), 18.0), k * b));
        f.text(egui::pos2(f.c.x + 128.0, ly - 14.0), "®", 12.0, al(ink, k * b));
    }
    // the power light
    f.p.circle_filled(egui::pos2(f.r.min.x + 30.0, f.r.max.y - 30.0), 6.0, al(rgb(220, 40, 60), k * eo(f.seg(0.0, 0.15))));
}

/// Retro DS: the console opens: two screens light up, the title on top,
/// "Touch to continue" blinking below; then the screens fill the window.
fn intro_ds(f: &Fx) {
    let k = f.out;
    let grow = 1.0 + (1.0 - k) * 1.6;
    f.fill(al(rgb(52, 55, 66), k));
    let sw = (f.w() * 0.42).min(420.0) * grow;
    let sh = sw * 0.66;
    let hinge = 26.0 * grow;
    let open = eo(f.seg(0.0, 0.45));
    let top = Rect::from_center_size(egui::pos2(f.c.x, f.c.y - (sh / 2.0 + hinge / 2.0) * open), egui::vec2(sw, sh));
    let bot = Rect::from_center_size(egui::pos2(f.c.x, f.c.y + (sh / 2.0 + hinge / 2.0) * open), egui::vec2(sw, sh));
    let on = eo(f.seg(0.3, 0.7));
    for scr in [top, bot] {
        f.p.rect_filled(scr.expand(10.0), 0.0, al(rgb(30, 32, 40), k));
        f.p.rect_filled(scr, 0.0, al(mix(rgb(16, 16, 20), rgb(248, 248, 232), on), k));
    }
    f.p.rect_filled(Rect::from_center_size(f.c, egui::vec2(sw * 0.5, hinge * 0.5)), 0.0, al(rgb(72, 72, 80), k));
    let t = eo(f.seg(0.6, 1.0));
    f.text(egui::pos2(top.center().x, top.center().y - 10.0 + 10.0 * (1.0 - t)), "Poke Together", 34.0 * grow, al(rgb(48, 48, 56), k * t));
    f.text(egui::pos2(top.center().x, top.center().y + 28.0), "COMPANION", 15.0 * grow, al(ink_on(rgb(178, 46, 32), rgb(248, 248, 232), 15.0), k * t));
    if f.s > 1.0 && (f.s * 2.5).fract() < 0.65 {
        f.text(bot.center(), "Touch the screen to continue.", 17.0 * grow, al(ink_on(rgb(28, 104, 192), rgb(248, 248, 232), 17.0), k));
    }
    // a little star on the bottom screen and the power light
    sparkle(f.p, egui::pos2(bot.max.x - 40.0, bot.min.y + 36.0), 9.0 * t, al(rgb(248, 216, 56), k));
    f.p.circle_filled(egui::pos2(bot.min.x - 22.0, bot.max.y + 14.0), 5.0, al(rgb(80, 220, 120), k * on));
}

/// "Your colours" as a theme, just for its startup animation.
fn yours_def(s: Surf, light: bool, accent: Color32, accent2: Color32, button: Color32) -> ThemeDef {
    let mut t = theme_def(crate::config::PlusTheme::Studio).expect("Studio is a theme");
    t.name = "Your colours";
    t.s = s;
    t.light = light;
    t.accent = accent;
    t.accent2 = accent2;
    t.button = button;
    t.skin = Skin::Plain;
    t.font = None;
    t.label = None;
    t.glow = None;
    t.backdrop = None;
    t.hard_shadow = None;
    t.particles = None;
    t
}

/// Your colours: a ring in your colour draws itself round a Poké Ball
/// that drops in and settles with a wobble, its button glints, and the
/// name fades up beneath; then it all lifts away.
fn intro_yours(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    let lift = (1.0 - k) * 40.0;
    let c = egui::pos2(f.c.x, f.c.y - 40.0 - lift);
    // the ring
    let sweep = std::f32::consts::TAU * eo(f.seg(0.0, 0.8));
    let n = 90;
    let ring: Vec<Pos2> = (0..=n).map(|i| {
        let a = -std::f32::consts::FRAC_PI_2 + sweep * i as f32 / n as f32;
        c + egui::vec2(a.cos(), a.sin()) * 70.0
    }).collect();
    f.p.add(Shape::line(ring, Stroke::new(4.0f32, al(t.accent, k))));
    if t.accent2 != t.accent {
        let sweep2 = std::f32::consts::TAU * eo(f.seg(0.15, 0.95));
        let ring2: Vec<Pos2> = (0..=n).map(|i| {
            let a = std::f32::consts::FRAC_PI_2 + sweep2 * i as f32 / n as f32;
            c + egui::vec2(a.cos(), a.sin()) * 82.0
        }).collect();
        f.p.add(Shape::line(ring2, Stroke::new(2.0f32, al(t.accent2, k * 0.8))));
    }
    // the ball drops in, then wobbles
    let drop = back(f.seg(0.25, 0.75));
    let wob = (f.seg(0.75, 1.4) * 18.0).sin() * 0.25 * (1.0 - f.seg(0.75, 1.4));
    let bc = egui::pos2(c.x, c.y - 120.0 * (1.0 - drop));
    let r = 44.0;
    if f.s > 0.25 {
        let top: Vec<Pos2> = (0..=40).map(|i| {
            let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 40.0 + wob;
            bc + egui::vec2(a.cos(), a.sin()) * r
        }).collect();
        f.p.circle_filled(bc, r, al(t.s.card, k));
        f.p.add(Shape::convex_polygon(top, al(t.button, k), Stroke::NONE));
        let (s, co) = wob.sin_cos();
        let band = egui::vec2(co, s) * r;
        f.p.line_segment([bc - band, bc + band], Stroke::new(4.0f32, al(t.s.text, k)));
        f.p.circle_stroke(bc, r, Stroke::new(3.0f32, al(t.s.text, k)));
        f.p.circle_filled(bc, 11.0, al(t.s.text, k));
        f.p.circle_filled(bc, 7.0, al(t.s.card, k));
        if f.s > 1.2 {
            sparkle(f.p, bc + egui::vec2(18.0, -20.0), 8.0 * back(f.seg(1.2, 1.5)), al(t.accent2, k));
        }
    }
    let tk = eo(f.seg(0.8, 1.3));
    f.text(egui::pos2(f.c.x, c.y + 120.0 + 12.0 * (1.0 - tk)), "Poke Together", 38.0, al(ink_on(t.s.text, t.s.page, 38.0), tk * k));
    f.text(egui::pos2(f.c.x, c.y + 156.0), "Companion", 15.0, al(ink_on(t.s.muted, t.s.page, 15.0), tk * k));
}

/// Studio: a soft colour orb blooms, the title settles in with its
/// letters drawing together, then the orb swells away.
fn intro_studio(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    let bloom = eo(f.seg(0.0, 1.0)) * (1.0 + (1.0 - k) * 3.0);
    let base = f.w().min(f.h()) * 0.3 * bloom;
    for i in 0..18 {
        let u = i as f32 / 18.0;
        let col = mix(t.accent, t.accent2, u);
        f.p.circle_filled(f.c + egui::vec2(-40.0 * (1.0 - u), -20.0 * u), base * (1.0 - u * 0.8), al(rgba(col, 18), k));
    }
    let tk = eo(f.seg(0.45, 1.1));
    // letters drawing together
    let gap = 18.0 * (1.0 - tk);
    let word = "Poke Together";
    let font = f.font(44.0);
    let widths: Vec<f32> = word.chars().map(|ch| f.p.layout_no_wrap(ch.to_string(), font.clone(), Color32::WHITE).size().x).collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
    let mut x = f.c.x - total / 2.0;
    for (ch, w) in word.chars().zip(&widths) {
        f.p.text(egui::pos2(x, f.c.y), Align2::LEFT_CENTER, ch, font.clone(), al(ink_on(t.s.text, t.s.page, 44.0), tk * k));
        x += w + gap;
    }
    let lw = 180.0 * eo(f.seg(0.9, 1.4));
    f.p.line_segment([egui::pos2(f.c.x - lw / 2.0, f.c.y + 36.0), egui::pos2(f.c.x + lw / 2.0, f.c.y + 36.0)], Stroke::new(2.0f32, al(t.accent, k)));
    f.text(egui::pos2(f.c.x, f.c.y + 60.0), "C O M P A N I O N", 13.0, al(ink_on(t.s.muted, t.s.page, 13.0), k * eo(f.seg(1.0, 1.4))));
}

/// Aurora glass: ribbons of light ripple across a dark sky, then a frosted
/// pane with the title fades up through them.
fn intro_aurora(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    let reach = f.r.min.x + f.w() * eo(f.seg(0.0, 1.0));
    let cols = [t.accent, t.accent2, rgb(125, 211, 252)];
    for (j, col) in cols.iter().enumerate() {
        let mut prev: Option<Pos2> = None;
        let mut x = f.r.min.x;
        while x <= reach {
            let y = f.c.y - 60.0 + j as f32 * 55.0 + ((x * 0.006) + f.s * 1.6 + j as f32 * 1.3).sin() * 60.0 + ((x * 0.013) - f.s * 0.9).sin() * 18.0;
            let pt = egui::pos2(x, y);
            if let Some(q) = prev {
                f.p.line_segment([q, pt], Stroke::new(70.0f32, al(rgba(*col, 16), k)));
                f.p.line_segment([q, pt], Stroke::new(22.0f32, al(rgba(*col, 34), k)));
                f.p.line_segment([q, pt], Stroke::new(3.0f32, al(rgba(*col, 150), k)));
            }
            prev = Some(pt);
            x += 10.0;
        }
    }
    let pk = eo(f.seg(0.6, 1.2));
    let pane = Rect::from_center_size(f.c, egui::vec2(380.0, 140.0) * (0.94 + 0.06 * pk));
    f.p.rect_filled(pane, 22.0, al(Color32::from_white_alpha(22), pk * k));
    f.p.rect_stroke(pane, 22.0, Stroke::new(1.0f32, al(Color32::from_white_alpha(90), pk * k)));
    f.p.line_segment([pane.left_top() + egui::vec2(24.0, 1.0), pane.right_top() + egui::vec2(-24.0, 1.0)], Stroke::new(1.0f32, al(Color32::from_white_alpha(160), pk * k)));
    f.text(egui::pos2(f.c.x, f.c.y - 10.0), "Poke Together", 40.0, al(ink_on(t.s.text, t.s.page, 40.0), pk * k));
    f.text(egui::pos2(f.c.x, f.c.y + 32.0), "companion", 15.0, al(ink_on(t.accent, t.s.page, 15.0), pk * k));
}

/// Field guide: contour lines draw themselves around a compass whose
/// needle swings to north, then the guide's title stamps in.
fn intro_field(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    for i in 0..10 {
        let base = 50.0 + i as f32 * 42.0;
        let sweep = std::f32::consts::TAU * eo(f.seg(i as f32 * 0.05, 0.9 + i as f32 * 0.05));
        let n = 120;
        let pts: Vec<Pos2> = (0..=n)
            .map(|j| {
                let a = sweep * j as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                let rr = base * (1.0 + 0.07 * (5.0 * a + i as f32).sin() + 0.04 * (3.0 * a - i as f32 * 2.0).cos());
                f.c + egui::vec2(a.cos(), a.sin()) * rr
            })
            .collect();
        f.p.add(Shape::line(pts, Stroke::new(if i % 4 == 0 { 1.6f32 } else { 1.0f32 }, al(rgba(t.accent, if i % 4 == 0 { 140 } else { 80 }), k))));
    }
    // the compass needle
    let swing = (1.0 - back(f.seg(0.2, 1.1))) * 3.2;
    let up = egui::vec2(swing.sin(), -swing.cos());
    let side = egui::vec2(-up.y, up.x);
    let tip = f.c + up * 34.0;
    let tail = f.c - up * 34.0;
    f.p.add(Shape::convex_polygon(vec![tip, f.c + side * 7.0, f.c - side * 7.0], al(rgb(194, 65, 12), k), Stroke::NONE));
    f.p.add(Shape::convex_polygon(vec![tail, f.c - side * 7.0, f.c + side * 7.0], al(t.s.text, k), Stroke::NONE));
    f.p.circle_filled(f.c, 4.0, al(t.s.page, k));
    f.text(f.c + up * 50.0, "N", 14.0, al(ink_on(t.s.text, t.s.page, 14.0), k * eo(f.seg(1.0, 1.3))));
    // the stamp
    let sk = back(f.seg(1.15, 1.45));
    let stamp = Rect::from_center_size(egui::pos2(f.c.x, f.c.y + 150.0), egui::vec2(300.0, 70.0) * (1.6 - 0.6 * sk));
    if f.s > 1.15 {
        f.p.rect_stroke(stamp, 6.0, Stroke::new(2.0f32, al(t.accent, k)));
        f.text(egui::pos2(stamp.center().x, stamp.center().y - 8.0), "Poke Together", 26.0, al(ink_on(t.s.text, t.s.page, 26.0), k));
        f.text(egui::pos2(stamp.center().x, stamp.center().y + 18.0), "FIELD GUIDE · COMPANION", 11.0, al(ink_on(t.accent, t.s.page, 11.0), k));
    }
}

/// Synthwave: a striped sun rises over a neon grid racing towards you,
/// with the title in chrome and glow.
fn intro_synth(f: &Fx) {
    let k = f.out;
    let sky = Rect::from_min_max(f.r.min, egui::pos2(f.r.max.x, f.r.max.y - f.h() * 0.32));
    let horizon = sky.max.y;
    f.p.add(gradient_rect(sky, [0.0; 4], al(rgb(14, 6, 32), k), al(rgb(90, 20, 96), k)));
    f.p.rect_filled(Rect::from_min_max(egui::pos2(f.r.min.x, horizon), f.r.max), 0.0, al(rgb(10, 4, 22), k));
    // the sun, rising, in stripes
    let rad = 120.0;
    let cy = horizon + rad * 0.2 + 160.0 * (1.0 - eo(f.seg(0.0, 1.1)));
    let mut y = cy - rad;
    while y < (cy + rad).min(horizon) {
        let dy = (y - cy) / rad;
        let half = rad * (1.0 - dy * dy).max(0.0).sqrt();
        let u = (y - (cy - rad)) / (2.0 * rad);
        let gapped = dy > 0.0 && ((dy * 9.0).fract() < 0.25 + dy * 0.4);
        if !gapped {
            f.p.line_segment([egui::pos2(f.c.x - half, y), egui::pos2(f.c.x + half, y)], Stroke::new(2.0f32, al(mix(rgb(250, 204, 21), rgb(244, 64, 160), u), k)));
        }
        y += 2.0;
    }
    // the grid
    let pink = rgb(244, 114, 182);
    let ph = (f.s * 1.4).fract();
    for j in 0..14 {
        let u = (j as f32 + ph) / 14.0;
        let gy = horizon + (f.r.max.y - horizon) * u * u;
        f.p.line_segment([egui::pos2(f.r.min.x, gy), egui::pos2(f.r.max.x, gy)], Stroke::new(1.0f32, al(rgba(pink, (40.0 + 180.0 * u) as u8), k)));
    }
    for i in -14..=14 {
        let xb = f.c.x + i as f32 * f.w() / 9.0;
        let xt = f.c.x + i as f32 * f.w() / 70.0;
        f.p.line_segment([egui::pos2(xt, horizon), egui::pos2(xb, f.r.max.y)], Stroke::new(1.0f32, al(rgba(pink, 120), k)));
    }
    f.p.line_segment([egui::pos2(f.r.min.x, horizon), egui::pos2(f.r.max.x, horizon)], Stroke::new(2.0f32, al(pink, k)));
    // chrome title with a glow
    let tk = eo(f.seg(0.7, 1.2));
    let ty = horizon - 230.0 + 20.0 * (1.0 - tk);
    for (o, a) in [(6.0f32, 30u8), (3.0, 60)] {
        for d in [egui::vec2(o, 0.0), egui::vec2(-o, 0.0), egui::vec2(0.0, o), egui::vec2(0.0, -o)] {
            f.text(egui::pos2(f.c.x, ty) + d, "POKE TOGETHER", 50.0, al(rgba(rgb(34, 211, 238), a), tk * k));
        }
    }
    f.text(egui::pos2(f.c.x + 3.0, ty + 3.0), "POKE TOGETHER", 50.0, al(rgb(168, 85, 247), tk * k));
    f.text(egui::pos2(f.c.x, ty), "POKE TOGETHER", 50.0, al(rgb(240, 248, 255), tk * k));
    f.text(egui::pos2(f.c.x, ty + 44.0), "c o m p a n i o n", 18.0, al(pink, tk * k));
}

/// Gold: a gold line draws out of the dark, the title glints, sparks
/// burst, then the dark parts like a curtain.
fn intro_gold(f: &Fx) {
    let t = f.th;
    let part = (1.0 - f.out) * (f.h() / 2.0 + 10.0);
    let bg = t.s.page;
    f.p.rect_filled(Rect::from_min_max(egui::pos2(f.r.min.x, f.r.min.y - part), egui::pos2(f.r.max.x, f.c.y - part)), 0.0, bg);
    f.p.rect_filled(Rect::from_min_max(egui::pos2(f.r.min.x, f.c.y + part), egui::pos2(f.r.max.x, f.r.max.y + part)), 0.0, bg);
    let k = f.out;
    let gold = t.accent;
    let lw = f.w() * 0.32 * eo(f.seg(0.0, 0.6));
    for (dy, a) in [(0.0f32, 255u8), (5.0, 90)] {
        f.p.line_segment([egui::pos2(f.c.x - lw, f.c.y + 40.0 + dy), egui::pos2(f.c.x + lw, f.c.y + 40.0 + dy)], Stroke::new(if dy == 0.0 { 1.6f32 } else { 1.0f32 }, al(rgba(gold, a), k)));
    }
    let tk = eo(f.seg(0.35, 0.9));
    let title = f.text(egui::pos2(f.c.x, f.c.y), "Poke Together", 52.0, al(ink_on(gold, t.s.page, 52.0), tk * k));
    // the glint
    let gx = title.min.x - 80.0 + (title.width() + 160.0) * eio(f.seg(0.9, 1.5));
    let clip = f.p.with_clip_rect(title.expand(4.0));
    clip.add(Shape::convex_polygon(
        vec![egui::pos2(gx, title.min.y), egui::pos2(gx + 26.0, title.min.y), egui::pos2(gx + 6.0, title.max.y), egui::pos2(gx - 20.0, title.max.y)],
        al(Color32::from_rgba_unmultiplied(255, 250, 220, 150), k),
        Stroke::NONE,
    ));
    f.text(egui::pos2(f.c.x, f.c.y + 66.0), "COMPANION", 14.0, al(ink_on(mix(gold, t.s.text, 0.4), t.s.page, 14.0), k * eo(f.seg(0.8, 1.2))));
    // sparks
    let b = f.seg(0.95, 1.9);
    if b > 0.0 && b < 1.0 {
        for i in 0..14u32 {
            let a = hash01(i, 1) * std::f32::consts::TAU;
            let d = eo(b) * (120.0 + 120.0 * hash01(i, 2));
            sparkle(f.p, f.c + egui::vec2(a.cos(), a.sin()) * d, 3.0 + 4.0 * hash01(i, 3), al(gold, (1.0 - b) * k));
        }
    }
}

/// Companion: soft blobs bounce in, a heart pops and beats, the title
/// hops up under it.
fn intro_companion(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    for i in 0..7u32 {
        let pk = back(f.seg(i as f32 * 0.07, 0.5 + i as f32 * 0.07));
        let c = egui::pos2(f.r.min.x + f.w() * hash01(i, 1), f.r.min.y + f.h() * hash01(i, 2));
        let col = if i % 2 == 0 { t.accent } else { t.accent2 };
        f.p.circle_filled(c, (60.0 + 90.0 * hash01(i, 3)) * pk, al(rgba(col, 40), k));
    }
    let hk = back(f.seg(0.25, 0.75)) * (1.0 + 0.08 * (f.s * 9.0).sin() * (1.0 - f.seg(0.8, 1.8))) * (1.0 + (1.0 - k) * 0.6);
    let hc = egui::pos2(f.c.x, f.c.y - 50.0);
    let s = 34.0 * hk;
    let col = al(t.accent, k);
    f.p.circle_filled(hc + egui::vec2(-s * 0.5, -s * 0.15), s * 0.58, col);
    f.p.circle_filled(hc + egui::vec2(s * 0.5, -s * 0.15), s * 0.58, col);
    f.p.add(Shape::convex_polygon(vec![hc + egui::vec2(-s * 1.05, 0.05 * s), hc + egui::vec2(s * 1.05, 0.05 * s), hc + egui::vec2(0.0, s * 1.15)], col, Stroke::NONE));
    f.p.circle_filled(hc + egui::vec2(-s * 0.62, -s * 0.36), s * 0.14, al(Color32::from_white_alpha(170), k));
    let tk = back(f.seg(0.55, 0.95));
    f.text(egui::pos2(f.c.x, f.c.y + 40.0 + 30.0 * (1.0 - tk)), "Poke Together", 38.0, al(ink_on(t.s.text, t.s.page, 38.0), k * f.seg(0.55, 0.7)));
    f.text(egui::pos2(f.c.x, f.c.y + 76.0), "your hunting buddy", 14.0, al(ink_on(t.s.muted, t.s.page, 14.0), k * f.seg(0.8, 1.1)));
    for i in 0..5u32 {
        let a = f.seg(0.9 + i as f32 * 0.08, 1.6 + i as f32 * 0.08);
        if a > 0.0 && a < 1.0 {
            let c = hc + egui::vec2((hash01(i, 5) - 0.5) * 160.0, -40.0 - 80.0 * a);
            sparkle(f.p, c, 5.0, al(t.accent2, (1.0 - a) * k));
        }
    }
}

/// Candy: stripes slide, a lollipop swirl winds itself up, and the title's
/// letters drop in one by one with a bounce.
fn intro_candy(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    let off = (f.s * 70.0) % 80.0;
    let mut x = f.r.min.x - f.h() - 80.0 + off;
    while x < f.r.max.x {
        let pts = vec![egui::pos2(x, f.r.max.y), egui::pos2(x + 40.0, f.r.max.y), egui::pos2(x + 40.0 + f.h(), f.r.min.y), egui::pos2(x + f.h(), f.r.min.y)];
        f.p.add(Shape::convex_polygon(pts, al(rgba(t.accent, 26), k), Stroke::NONE));
        x += 80.0;
    }
    // the lollipop
    let lc = egui::pos2(f.c.x, f.c.y - 90.0);
    let turns = 3.0 * std::f32::consts::TAU * eo(f.seg(0.0, 0.9));
    let pts: Vec<Pos2> = (0..=160).map(|i| {
        let a = turns * i as f32 / 160.0;
        let rr = 4.0 + a * 3.6;
        lc + egui::vec2((a + f.s * 2.0).cos(), (a + f.s * 2.0).sin()) * rr
    }).collect();
    f.p.circle_filled(lc, 4.0 + turns * 3.6 + 6.0, al(Color32::WHITE, k * 0.9));
    f.p.add(Shape::line(pts, Stroke::new(9.0f32, al(t.accent, k))));
    f.p.line_segment([egui::pos2(lc.x, lc.y + 80.0), egui::pos2(lc.x, lc.y + 130.0)], Stroke::new(6.0f32, al(t.accent2, k * f.seg(0.3, 0.6))));
    // the title, letter by letter
    let word = "Poke Together";
    let font = f.font(44.0);
    let widths: Vec<f32> = word.chars().map(|ch| f.p.layout_no_wrap(ch.to_string(), font.clone(), Color32::WHITE).size().x).collect();
    let total: f32 = widths.iter().sum();
    let mut x = f.c.x - total / 2.0;
    for (i, (ch, w)) in word.chars().zip(&widths).enumerate() {
        let a = f.seg(0.45 + i as f32 * 0.05, 0.8 + i as f32 * 0.05);
        let y = f.c.y + 60.0 - 70.0 * (1.0 - back(a));
        let col = if i % 2 == 0 { t.accent } else { t.accent2 };
        f.p.text(egui::pos2(x, y), Align2::LEFT_CENTER, ch, font.clone(), al(ink_on(mix(col, Color32::BLACK, 0.1), t.s.page, 44.0), k * (a * 4.0).min(1.0)));
        x += w;
    }
}

/// Brutal: heavy blocks slam down one after another (the screen shakes),
/// the title sits on them, then they shove off to the side.
fn intro_brutal(f: &Fx) {
    let t = f.th;
    let k = f.out;
    let ink = t.hard_shadow.unwrap_or(t.s.text);
    let mut shake = 0.0;
    for i in 0..4 {
        let land = 0.15 + i as f32 * 0.16 + 0.22;
        let d = f.s - land;
        if d > 0.0 && d < 0.18 {
            shake += (d * 90.0).sin() * 6.0 * (1.0 - d / 0.18);
        }
    }
    let sh = egui::vec2(shake, shake * 0.4);
    f.fill(al(t.s.page, k));
    let blocks = [
        (egui::vec2(-210.0, -110.0), egui::vec2(260.0, 120.0), t.accent),
        (egui::vec2(120.0, -70.0), egui::vec2(220.0, 180.0), t.accent2),
        (egui::vec2(-120.0, 90.0), egui::vec2(300.0, 110.0), t.button),
        (egui::vec2(0.0, 0.0), egui::vec2(470.0, 120.0), t.s.card),
    ];
    for (i, (pos, size, col)) in blocks.iter().enumerate() {
        let a = f.seg(0.15 + i as f32 * 0.16, 0.37 + i as f32 * 0.16);
        let fall = a * a; // accelerating, like a drop
        let slide = (1.0 - k) * f.w() * (1.0 + i as f32 * 0.25);
        let target = f.c + *pos + sh + egui::vec2(slide, 0.0);
        let at = egui::pos2(target.x, target.y - (1.0 - fall) * (f.h() + 200.0));
        let rr = Rect::from_center_size(at, *size);
        if a > 0.0 {
            f.p.rect_filled(rr.translate(egui::vec2(10.0, 10.0)), 0.0, ink);
            f.p.rect_filled(rr, 0.0, *col);
            f.p.rect_stroke(rr, 0.0, Stroke::new(4.0f32, ink));
        }
        if i == 3 && a >= 1.0 {
            f.text_f(rr.center() - egui::vec2(0.0, 10.0), "POKE TOGETHER", crate::fonts::heavy_font(44.0), ink_on(ink, t.s.card, 44.0));
            f.text_f(rr.center() + egui::vec2(0.0, 30.0), "COMPANION ////", crate::fonts::heavy_font(14.0), ink_on(t.accent, t.s.card, 14.0));
        }
    }
}

/// Liquid: water rises with a rolling surface, bubbles drift up through
/// it, the title floats on it, then it all drains away.
fn intro_liquid(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k.min(1.0)));
    let fill = eo(f.seg(0.0, 0.9));
    let level = f.r.max.y + 30.0 - (f.h() + 60.0) * fill * k;
    let deep = mix(t.accent, Color32::BLACK, 0.35);
    let mut m = egui::Mesh::default();
    let step = 14.0;
    let mut x = f.r.min.x;
    let mut idx = 0u32;
    while x <= f.r.max.x + step {
        let y = level + (x * 0.018 + f.s * 4.0).sin() * 12.0 + (x * 0.007 - f.s * 2.0).sin() * 8.0;
        m.colored_vertex(egui::pos2(x, y), rgba(mix(t.accent, Color32::WHITE, 0.25), 235));
        m.colored_vertex(egui::pos2(x, f.r.max.y), rgba(deep, 245));
        if idx > 0 {
            let b = idx * 2;
            m.add_triangle(b - 2, b - 1, b);
            m.add_triangle(b - 1, b, b + 1);
        }
        idx += 1;
        x += step;
    }
    f.p.add(Shape::mesh(m));
    // bubbles
    for i in 0..24u32 {
        let travel = f.h();
        let y = f.r.max.y - ((hash01(i, 1) * travel + f.s * (60.0 + 80.0 * hash01(i, 2))) % travel);
        if y > level + 10.0 {
            let x = f.r.min.x + f.w() * hash01(i, 3) + (f.s * 3.0 + i as f32).sin() * 6.0;
            f.p.circle_stroke(egui::pos2(x, y), 2.0 + 6.0 * hash01(i, 4), Stroke::new(1.2f32, Color32::from_white_alpha(120)));
        }
    }
    let tk = f.seg(0.55, 1.0);
    let bob = (f.s * 2.2).sin() * 4.0;
    f.text(egui::pos2(f.c.x, f.c.y + bob), "Poke Together", 46.0, al(ink_on(Color32::WHITE, mix(t.accent, Color32::BLACK, 0.1), 46.0), tk * k));
    f.text(egui::pos2(f.c.x, f.c.y + 40.0 + bob), "companion", 16.0, al(ink_on(Color32::WHITE, mix(t.accent, Color32::BLACK, 0.1), 16.0), tk * k));
}

/// Winter: frost creeps in from the corners, a snowflake turns at the
/// centre, a flurry blows through, then it all melts away.
fn intro_winter(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k));
    let ice = if t.light { rgb(90, 150, 210) } else { rgb(225, 240, 255) };
    let grow = eo(f.seg(0.0, 1.2));
    let corners = [f.r.left_top(), f.r.right_top(), f.r.left_bottom(), f.r.right_bottom()];
    for (ci, corner) in corners.iter().enumerate() {
        let to_c = (f.c - *corner).normalized();
        let base = to_c.y.atan2(to_c.x);
        for b in 0..7u32 {
            let ang = base + (b as f32 - 3.0) * 0.22 + (hash01(b, ci as u32) - 0.5) * 0.1;
            let len = (160.0 + 160.0 * hash01(b, 10 + ci as u32)) * grow;
            let dir = egui::vec2(ang.cos(), ang.sin());
            let end = *corner + dir * len;
            f.p.line_segment([*corner, end], Stroke::new(1.4f32, al(rgba(ice, 150), k)));
            let mut d = 16.0;
            while d < len {
                let tw = 10.0 * (1.0 - d / (len + 1.0)) + 3.0;
                for sgn in [-1.0f32, 1.0] {
                    let a2 = ang + sgn * 1.05;
                    let p0 = *corner + dir * d;
                    f.p.line_segment([p0, p0 + egui::vec2(a2.cos(), a2.sin()) * tw], Stroke::new(1.0f32, al(rgba(ice, 110), k)));
                }
                d += 14.0;
            }
        }
    }
    // the snowflake
    let fk = back(f.seg(0.3, 0.8));
    let fc = egui::pos2(f.c.x, f.c.y - 70.0);
    for i in 0..6 {
        let a = i as f32 * std::f32::consts::TAU / 6.0 + f.s * 0.6;
        let dir = egui::vec2(a.cos(), a.sin());
        let arm = 46.0 * fk;
        f.p.line_segment([fc, fc + dir * arm], Stroke::new(3.0f32, al(ice, k)));
        for d in [0.45f32, 0.7] {
            for sgn in [-1.0f32, 1.0] {
                let b = a + sgn * 0.8;
                let p0 = fc + dir * arm * d;
                f.p.line_segment([p0, p0 + egui::vec2(b.cos(), b.sin()) * arm * 0.28], Stroke::new(2.0f32, al(ice, k)));
            }
        }
    }
    f.text(egui::pos2(f.c.x, f.c.y + 30.0), "Poke Together", 44.0, al(ink_on(t.accent, t.s.page, 44.0), k * eo(f.seg(0.6, 1.0))));
    f.text(egui::pos2(f.c.x, f.c.y + 66.0), "companion", 15.0, al(ink_on(t.s.muted, t.s.page, 15.0), k * eo(f.seg(0.8, 1.2))));
    // the flurry, blowing across
    for i in 0..90u32 {
        let x = f.r.min.x + ((hash01(i, 1) * f.w() + f.s * (220.0 + 160.0 * hash01(i, 2))) % (f.w() + 40.0)) - 20.0;
        let y = f.r.min.y + ((hash01(i, 3) * f.h() + f.s * (90.0 + 60.0 * hash01(i, 4))) % f.h());
        f.p.circle_filled(egui::pos2(x, y), 1.0 + 2.2 * hash01(i, 5), al(rgba(ice, (90.0 + 140.0 * hash01(i, 6)) as u8), k));
    }
}

/// Spooky: pitch dark, a candle flickers up to light the title, bats
/// flap past the moon, fog rolls along the floor, then the light floods out.
fn intro_spooky(f: &Fx) {
    let t = f.th;
    let k = f.out;
    let dark = if t.light { rgb(40, 30, 44) } else { rgb(8, 6, 10) };
    f.fill(al(dark, k));
    // the moon
    let moon = egui::pos2(f.r.max.x - 150.0, f.r.min.y + 120.0);
    f.p.circle_filled(moon, 46.0, al(rgb(240, 232, 200), k * eo(f.seg(0.0, 0.6))));
    f.p.circle_filled(moon + egui::vec2(18.0, -10.0), 42.0, al(dark, k));
    // candlelight
    let flick = 0.88 + 0.08 * (f.s * 23.0).sin() + 0.04 * (f.s * 37.0).sin();
    let light = 240.0 * eo(f.seg(0.2, 0.9)) * flick * (1.0 + (1.0 - k) * 6.0);
    for i in 0..16 {
        let u = i as f32 / 16.0;
        f.p.circle_filled(f.c, light * (1.0 - u * 0.85), al(Color32::from_rgba_unmultiplied(255, 150, 60, 8), k));
    }
    let tk = eo(f.seg(0.55, 1.0)) * flick;
    f.text(egui::pos2(f.c.x, f.c.y - 10.0), "Poke Together", 50.0, al(ink_on(t.accent, dark, 50.0), tk * k));
    f.text(egui::pos2(f.c.x, f.c.y + 34.0), "...companion", 16.0, al(ink_on(rgb(200, 180, 210), dark, 16.0), tk * k));
    // bats
    for i in 0..6u32 {
        let a = f.seg(0.3 + i as f32 * 0.12, 1.5 + i as f32 * 0.12);
        if a > 0.0 && a < 1.0 {
            let x = f.r.min.x - 40.0 + (f.w() + 80.0) * a;
            let y = f.r.min.y + 80.0 + 200.0 * hash01(i, 1) + (a * 12.0).sin() * 20.0;
            let flap = (f.s * 22.0 + i as f32).sin() * 9.0;
            let s = 10.0 + 8.0 * hash01(i, 2);
            let c = egui::pos2(x, y);
            let col = al(rgb(20, 14, 24), k);
            f.p.add(Shape::convex_polygon(vec![c, c + egui::vec2(-s * 1.6, -flap), c + egui::vec2(-s * 0.8, s * 0.3)], col, Stroke::NONE));
            f.p.add(Shape::convex_polygon(vec![c, c + egui::vec2(s * 0.8, s * 0.3), c + egui::vec2(s * 1.6, -flap)], col, Stroke::NONE));
            f.p.circle_filled(c, s * 0.3, col);
        }
    }
    // fog
    for i in 0..9u32 {
        let x = f.r.min.x + ((hash01(i, 1) * f.w() + f.s * 30.0 * (0.5 + hash01(i, 2))) % (f.w() + 300.0)) - 150.0;
        f.p.circle_filled(egui::pos2(x, f.r.max.y - 30.0 * hash01(i, 3)), 120.0 + 60.0 * hash01(i, 4), al(Color32::from_rgba_unmultiplied(160, 140, 180, 10), k));
    }
}

/// Sticker: stickers slap down all over with a wobble, the title sticker
/// lands last, then peels up and away.
fn intro_sticker(f: &Fx) {
    let t = f.th;
    let k = f.out;
    let ink = t.hard_shadow.unwrap_or(t.s.text);
    f.fill(al(t.s.page, k));
    let cols = [t.accent, t.accent2, t.button, rgb(110, 220, 170), rgb(125, 180, 250)];
    for i in 0..9u32 {
        let a = f.seg(0.05 + i as f32 * 0.07, 0.3 + i as f32 * 0.07);
        if a <= 0.0 {
            continue;
        }
        let sc = 1.0 + 0.6 * (1.0 - back(a));
        let c = egui::pos2(f.r.min.x + f.w() * (0.1 + 0.8 * hash01(i, 1)), f.r.min.y + f.h() * (0.1 + 0.8 * hash01(i, 2)));
        let ang = (hash01(i, 3) - 0.5) * 0.7;
        let col = cols[i as usize % cols.len()];
        if i % 3 == 0 {
            f.p.circle_filled(c + egui::vec2(5.0, 5.0), 38.0 * sc, al(ink, k));
            f.p.circle_filled(c, 38.0 * sc, al(col, k));
            f.p.circle_stroke(c, 38.0 * sc, Stroke::new(3.0f32, al(ink, k)));
            sparkle(f.p, c, 14.0 * sc, al(Color32::WHITE, k));
        } else {
            let (w, h) = (110.0 * sc, 60.0 * sc);
            f.p.add(Shape::convex_polygon(turned(c + egui::vec2(6.0, 6.0), w, h, ang), al(ink, k), Stroke::NONE));
            f.p.add(Shape::convex_polygon(turned(c, w, h, ang), al(col, k), Stroke::new(3.0f32, al(ink, k))));
        }
    }
    // the title sticker: lands, then peels off
    let a = f.seg(0.75, 1.05);
    if a > 0.0 {
        let peel = 1.0 - k;
        let sc = (1.0 + 0.7 * (1.0 - back(a))) * (1.0 - 0.2 * peel);
        let c = f.c + egui::vec2(peel * 260.0, -peel * 340.0);
        let ang = -0.06 + peel * 0.9;
        f.p.add(Shape::convex_polygon(turned(c + egui::vec2(8.0, 8.0), 440.0 * sc, 130.0 * sc, ang), al(ink, k), Stroke::NONE));
        f.p.add(Shape::convex_polygon(turned(c, 440.0 * sc, 130.0 * sc, ang), Color32::WHITE, Stroke::new(4.0f32, ink)));
        f.p.add(Shape::convex_polygon(turned(c, 410.0 * sc, 100.0 * sc, ang), t.accent, Stroke::NONE));
        f.text(c - egui::vec2(0.0, 8.0), "Poke Together", 40.0 * sc, ink_on(ink, t.accent, 40.0));
        f.text(c + egui::vec2(0.0, 28.0 * sc), "COMPANION!", 14.0 * sc, ink_on(ink, t.accent, 14.0));
    }
}

/// Glass HUD: corner brackets lock on, a radar sweeps, "SYSTEM ONLINE"
/// types itself out, then the HUD flies out to the window's edges.
fn intro_hud(f: &Fx) {
    let t = f.th;
    let k = f.out;
    f.fill(al(t.s.page, k * 0.97));
    let mut y = f.r.min.y;
    while y < f.r.max.y {
        f.p.line_segment([egui::pos2(f.r.min.x, y), egui::pos2(f.r.max.x, y)], Stroke::new(1.0f32, al(rgba(t.accent, 10), k)));
        y += 4.0;
    }
    let lock = eo(f.seg(0.0, 0.5));
    let spread = 1.0 - k;
    let frame = Rect::from_center_size(f.c, egui::vec2(380.0, 230.0));
    let hw = frame.width() / 2.0 + (f.w() / 2.0 - frame.width() / 2.0) * ((1.0 - lock) + spread);
    let hh = frame.height() / 2.0 + (f.h() / 2.0 - frame.height() / 2.0) * ((1.0 - lock) + spread);
    let st = Stroke::new(2.0f32, al(t.accent, k));
    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = f.c + egui::vec2(sx * hw, sy * hh);
        f.p.line_segment([corner, corner + egui::vec2(-sx * 34.0, 0.0)], st);
        f.p.line_segment([corner, corner + egui::vec2(0.0, -sy * 34.0)], st);
    }
    // the radar
    let rc = egui::pos2(f.c.x, f.c.y - 30.0);
    let rk = eo(f.seg(0.3, 0.7));
    f.p.circle_stroke(rc, 62.0 * rk, Stroke::new(1.0f32, al(rgba(t.accent, 140), k)));
    f.p.circle_stroke(rc, 36.0 * rk, Stroke::new(1.0f32, al(rgba(t.accent, 80), k)));
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        let d = egui::vec2(a.cos(), a.sin());
        f.p.line_segment([rc + d * 62.0 * rk, rc + d * (66.0 + if i % 6 == 0 { 6.0 } else { 0.0 }) * rk], Stroke::new(1.0f32, al(t.accent, k)));
    }
    for j in 0..14 {
        let a = f.s * 4.0 - j as f32 * 0.05;
        f.p.line_segment([rc, rc + egui::vec2(a.cos(), a.sin()) * 62.0 * rk], Stroke::new(2.0f32, al(rgba(t.accent, (160.0 * (1.0 - j as f32 / 14.0)) as u8), k)));
    }
    let blip = rc + egui::vec2(28.0, -20.0) * rk;
    f.p.circle_filled(blip, 4.0, al(t.accent2, k * (f.s * 4.0).sin().abs()));
    // typed out
    let mono = egui::FontId::monospace(15.0);
    let line = "SYSTEM ONLINE · SHINY RADAR READY";
    let n = (line.chars().count() as f32 * f.seg(0.6, 1.3)) as usize;
    let typed: String = line.chars().take(n).collect();
    let cur = if (f.s * 3.0).fract() < 0.5 { "_" } else { " " };
    f.text_f(egui::pos2(f.c.x, frame.max.y - 40.0), &format!("{typed}{cur}"), mono, al(ink_on(t.accent, t.s.page, 15.0), k));
    f.text(egui::pos2(f.c.x, frame.max.y - 74.0), "POKE TOGETHER", 26.0, al(ink_on(t.s.text, t.s.page, 26.0), k * f.seg(1.0, 1.3)));
}

/// A torn paper edge: little teeth of `paper` along `x`, pointing `dir`
/// (-1 left, 1 right), from `top` to `bottom`.
fn torn_edge(p: &Painter, x: f32, top: f32, bottom: f32, paper: Color32, dir: f32) {
    let mut y = top;
    let mut i = 0u32;
    while y < bottom {
        let step = 7.0 + 6.0 * hash01(i, 21);
        let reach = 4.0 + 8.0 * hash01(i, 22);
        let pts = vec![egui::pos2(x, y), egui::pos2(x + dir * reach, y + step * 0.5), egui::pos2(x, y + step)];
        p.add(Shape::convex_polygon(pts, paper, Stroke::NONE));
        y += step;
        i += 1;
    }
}

/// A sheet of the journal's ruled paper over `r`, torn along its left edge.
fn torn_sheet(p: &Painter, r: Rect, t: &ThemeDef) {
    let body = Rect::from_min_max(egui::pos2(r.min.x + 10.0, r.min.y), r.max);
    p.rect_filled(body.translate(egui::vec2(10.0, 14.0)), 0.0, Color32::from_black_alpha(36));
    p.rect_filled(body, 0.0, t.s.page);
    torn_edge(p, body.min.x, r.min.y, r.max.y, t.s.page, -1.0);
    let rule = Color32::from_rgba_unmultiplied(120, 160, 214, if t.light { 90 } else { 60 });
    let clip = p.with_clip_rect(body.intersect(p.clip_rect()));
    let mut y = r.min.y + 30.0;
    while y < r.max.y {
        clip.line_segment([egui::pos2(body.min.x, y), egui::pos2(r.max.x, y)], Stroke::new(1.0f32, rule));
        y += 28.0;
    }
}

/// Field journal: the title writes itself in ink across ruled paper, gets
/// underlined with a scribble and a doodled star, then the page is ripped
/// out of the notebook and flung away, leaving its torn stub on the rings.
fn intro_journal(f: &Fx) {
    let d = eio(1.0 - f.out);
    if d > 0.0 {
        // the stub left on the binding, punched holes and all
        let t = f.th;
        let stub = Rect::from_min_max(f.r.min, egui::pos2(f.r.min.x + 30.0, f.r.max.y));
        f.p.rect_filled(stub, 0.0, al(t.s.page, f.out));
        torn_edge(f.p, stub.max.x, stub.min.y, stub.max.y, al(t.s.page, f.out), 1.0);
        let mut y = f.r.min.y + 60.0;
        while y < f.r.max.y {
            f.p.circle_filled(egui::pos2(stub.min.x + 14.0, y), 5.0, al(mix(t.s.page, Color32::BLACK, 0.2), f.out));
            y += 90.0;
        }
    }
    // the page itself, pulled up and away
    let off = egui::vec2(d * f.w() * 0.9, -d * f.h() * 0.35);
    let g = Fx { p: f.p, r: f.r.translate(off), c: f.c + off, s: f.s, out: 1.0, th: f.th };
    journal_page(&g, d > 0.0);
}

fn journal_page(f: &Fx, torn: bool) {
    let t = f.th;
    let edge = if torn { f.r.min.x + 10.0 } else { f.r.min.x };
    if torn {
        f.p.rect_filled(Rect::from_min_max(egui::pos2(edge, f.r.min.y), f.r.max).translate(egui::vec2(10.0, 14.0)), 0.0, Color32::from_black_alpha(36));
        torn_edge(f.p, edge, f.r.min.y, f.r.max.y, t.s.page, -1.0);
    }
    let page = f.p.with_clip_rect(Rect::from_min_max(egui::pos2(edge, f.r.min.y), f.r.max).intersect(f.p.clip_rect()));
    page.rect_filled(f.r, 0.0, t.s.page);
    let rule = if t.light { Color32::from_rgba_unmultiplied(120, 160, 214, 90) } else { Color32::from_rgba_unmultiplied(120, 160, 214, 60) };
    let mut y = f.r.min.y + 30.0;
    while y < f.r.max.y {
        page.line_segment([egui::pos2(f.r.min.x, y), egui::pos2(f.r.max.x, y)], Stroke::new(1.0f32, rule));
        y += 28.0;
    }
    let margin = if t.light { Color32::from_rgba_unmultiplied(232, 128, 128, 160) } else { Color32::from_rgba_unmultiplied(240, 204, 120, 120) };
    page.line_segment([egui::pos2(f.r.min.x + 80.0, f.r.min.y), egui::pos2(f.r.min.x + 80.0, f.r.max.y)], Stroke::new(1.5f32, margin));
    // written in, left to right, with the pen at the edge
    let font = f.font(58.0);
    let g = f.p.layout_no_wrap("Poke Together".into(), font.clone(), t.s.text);
    let at = egui::pos2(f.c.x - g.size().x / 2.0, f.c.y - g.size().y / 2.0 - 20.0);
    let w = g.size().x * f.seg(0.15, 1.2);
    let ink = page.with_clip_rect(Rect::from_min_max(egui::pos2(at.x - 4.0, at.y - 10.0), egui::pos2(at.x + w, at.y + g.size().y + 10.0)).intersect(page.clip_rect()));
    ink.galley(at, g.clone(), t.s.text);
    if f.s < 1.25 && f.s > 0.15 {
        let nib = egui::pos2(at.x + w, at.y + g.size().y * 0.7 + (f.s * 30.0).sin() * 8.0);
        page.add(Shape::convex_polygon(vec![nib, nib + egui::vec2(10.0, -26.0), nib + egui::vec2(20.0, -20.0)], t.s.text, Stroke::NONE));
    }
    // the scribbled underline
    let u = f.seg(1.2, 1.5);
    let ul_y = at.y + g.size().y + 6.0;
    let n = (40.0 * u) as usize;
    let pts: Vec<Pos2> = (0..=n).map(|i| {
        let x = at.x + g.size().x * i as f32 / 40.0;
        egui::pos2(x, ul_y + (i as f32 * 1.7).sin() * 3.0 + (i as f32 * 0.4).sin() * 2.0)
    }).collect();
    if pts.len() > 1 {
        page.add(Shape::line(pts, Stroke::new(2.0f32, t.accent)));
    }
    if f.s > 1.35 {
        sparkle(&page, egui::pos2(at.x + g.size().x + 30.0, at.y + 6.0), 12.0 * back(f.seg(1.35, 1.6)), t.accent);
        page.text(egui::pos2(f.c.x, ul_y + 40.0), Align2::CENTER_CENTER, "my companion notebook", f.font(20.0), al(ink_on(t.s.muted, t.s.page, 20.0), f.seg(1.4, 1.7)));
    }
}

/// Broadcast: colour bars and "please stand by", stripes sweep across
/// into a lower-third with the title, LIVE blinking, a ticker running;
/// then the whole graphic slides off.
fn intro_broadcast(f: &Fx) {
    let t = f.th;
    let slide = egui::vec2(-(1.0 - f.out) * (f.w() + 100.0), 0.0);
    let p = f.p;
    // colour bars
    let bars = [rgb(192, 192, 192), rgb(192, 192, 0), rgb(0, 192, 192), rgb(0, 192, 0), rgb(192, 0, 192), rgb(192, 0, 0), rgb(0, 0, 192)];
    let bw = f.w() / bars.len() as f32;
    for (i, c) in bars.iter().enumerate() {
        p.rect_filled(Rect::from_min_size(egui::pos2(f.r.min.x + i as f32 * bw, f.r.min.y), egui::vec2(bw + 1.0, f.h())).translate(slide), 0.0, *c);
    }
    if f.s < 0.5 {
        let plate = Rect::from_center_size(f.c, egui::vec2(360.0, 70.0)).translate(slide);
        p.rect_filled(plate, 0.0, Color32::BLACK);
        p.text(plate.center(), Align2::CENTER_CENTER, "PLEASE STAND BY", crate::fonts::heavy_font(26.0), Color32::WHITE);
    }
    // stripes sweep over the bars
    let sweep = eio(f.seg(0.4, 0.95));
    let edge = f.r.min.x - 300.0 + (f.w() + 600.0) * sweep;
    let behind = Rect::from_min_max(f.r.min, egui::pos2(edge - 120.0, f.r.max.y)).translate(slide);
    p.rect_filled(behind, 0.0, t.s.page);
    for (i, col) in [t.accent, Color32::WHITE, t.accent2, t.accent].iter().enumerate() {
        let x = edge - i as f32 * 34.0;
        let pts = vec![egui::pos2(x, f.r.min.y), egui::pos2(x + 30.0, f.r.min.y), egui::pos2(x - 90.0, f.r.max.y), egui::pos2(x - 120.0, f.r.max.y)];
        p.add(Shape::convex_polygon(pts.into_iter().map(|q| q + slide).collect(), *col, Stroke::NONE));
    }
    // the lower third
    let lt = eo(f.seg(0.85, 1.2));
    if lt > 0.0 {
        let band = Rect::from_min_size(egui::pos2(f.r.min.x + 60.0 - 600.0 * (1.0 - lt), f.c.y + 30.0), egui::vec2(560.0, 84.0)).translate(slide);
        // the news ticker under it: today's rare grass spawns
        let strip = Rect::from_min_max(egui::pos2(band.min.x, band.max.y), egui::pos2((band.min.x + 860.0).min(f.r.max.x - 20.0).max(band.max.x), band.max.y + 32.0));
        p.rect_filled(strip, 0.0, rgb(20, 20, 26));
        p.rect_filled(band, 0.0, t.accent);
        p.rect_filled(Rect::from_min_size(band.min, egui::vec2(10.0, band.height())), 0.0, Color32::WHITE);
        p.text(band.left_center() + egui::vec2(28.0, -8.0), Align2::LEFT_CENTER, "POKE TOGETHER", crate::fonts::heavy_font(34.0), ink_on(Color32::WHITE, t.accent, 34.0));
        p.text(band.left_center() + egui::vec2(30.0, 24.0), Align2::LEFT_CENTER, "COMPANION · SHINY HUNT COVERAGE", egui::FontId::proportional(13.0), ink_on(Color32::WHITE, t.accent, 13.0));
        let tag = Rect::from_min_size(strip.min, egui::vec2(168.0, strip.height()));
        let news = intro_news();
        let line = if news.is_empty() {
            "No rare grass spotted yet today · use Sweet Scent on a rare grass patch · more after the break".to_string()
        } else {
            news.iter().map(|(n, at)| format!("{} at {at}", n.to_uppercase())).collect::<Vec<_>>().join("   ·   ")
        };
        let tp = p.with_clip_rect(Rect::from_min_max(egui::pos2(tag.max.x, strip.min.y), strip.max).intersect(p.clip_rect()));
        let font = egui::FontId::proportional(14.0);
        let gw = tp.layout_no_wrap(line.clone(), font.clone(), Color32::WHITE).size().x + 80.0;
        let run = (f.s - 0.85).max(0.0) * 140.0;
        let mut x = tag.max.x + 30.0 - run % gw;
        while x < strip.max.x {
            tp.text(egui::pos2(x, strip.center().y), Align2::LEFT_CENTER, &line, font.clone(), Color32::WHITE);
            x += gw;
        }
        p.rect_filled(tag, 0.0, t.accent2);
        p.text(tag.center(), Align2::CENTER_CENTER, "RARE GRASS TODAY", crate::fonts::heavy_font(13.0), ink_on(Color32::WHITE, t.accent2, 13.0));
        // LIVE
        let live = Rect::from_min_size(f.r.min + egui::vec2(28.0, 28.0), egui::vec2(78.0, 30.0)).translate(slide);
        p.rect_filled(live, 3.0, rgb(220, 30, 40));
        if (f.s * 2.0).fract() < 0.6 {
            p.circle_filled(live.left_center() + egui::vec2(14.0, 0.0), 5.0, Color32::WHITE);
        }
        p.text(live.left_center() + egui::vec2(26.0, 0.0), Align2::LEFT_CENTER, "LIVE", crate::fonts::heavy_font(16.0), Color32::WHITE);
    }
}

/// Terminal: boot log lines type themselves out with a blinking cursor;
/// then the picture collapses to a line, like an old CRT, and is gone.
fn intro_terminal(f: &Fx) {
    let t = f.th;
    // the CRT collapse on the way out
    let h = (f.h() / 2.0) * f.out.powf(1.6);
    let band = Rect::from_min_max(egui::pos2(f.r.min.x, f.c.y - h), egui::pos2(f.r.max.x, f.c.y + h));
    let p = f.p.with_clip_rect(band);
    p.rect_filled(f.r, 0.0, t.s.page);
    let mono = egui::FontId::monospace(16.0);
    let green = t.accent;
    let lines = [
        "> poke-together --boot",
        "[ ok ] reading pokedex ............ 493 entries",
        "[ ok ] opening your save (read-only)",
        "[ ok ] loading hunts and runs",
        "[ ok ] warming up the shiny radar",
        "[ ok ] companion ready",
        "> _",
    ];
    let mut y = f.r.min.y + 60.0;
    for (i, l) in lines.iter().enumerate() {
        let a = f.seg(0.05 + i as f32 * 0.2, 0.25 + i as f32 * 0.2);
        if a <= 0.0 {
            break;
        }
        let n = (l.chars().count() as f32 * a) as usize;
        let shown: String = l.chars().take(n).collect();
        let cursor = if a < 1.0 || (i == lines.len() - 1 && (f.s * 3.0).fract() < 0.5) { "█" } else { "" };
        let text = if i == lines.len() - 1 { shown.trim_end_matches('_').to_string() } else { shown };
        p.text(egui::pos2(f.r.min.x + 60.0, y), Align2::LEFT_CENTER, format!("{text}{cursor}"), mono.clone(), ink_on(if l.starts_with("[ ok ]") && a >= 1.0 { mix(green, t.s.text, 0.2) } else { green }, t.s.page, 16.0));
        y += 28.0;
    }
    // big ASCII-ish title once booted
    if f.s > 1.3 {
        p.text(egui::pos2(f.c.x, f.c.y + 120.0), Align2::CENTER_CENTER, "POKE_TOGETHER", egui::FontId::monospace(44.0), al(ink_on(green, t.s.page, 44.0), eo(f.seg(1.3, 1.6))));
    }
    // scanlines and a glow
    let mut sy = f.r.min.y;
    while sy < f.r.max.y {
        p.line_segment([egui::pos2(f.r.min.x, sy), egui::pos2(f.r.max.x, sy)], Stroke::new(1.0f32, Color32::from_black_alpha(50)));
        sy += 3.0;
    }
    if f.out < 0.35 {
        // the last bright line
        f.p.line_segment([egui::pos2(f.r.min.x, f.c.y), egui::pos2(f.r.max.x, f.c.y)], Stroke::new(2.0f32, al(Color32::WHITE, f.out / 0.35)));
    }
}

// ── scroll bars

/// The page's scroll bar in the theme's own style: `track` is its lane,
/// `handle` the thumb; `hot` while hovered or dragged.
pub fn paint_scrollbar(p: &Painter, track: Rect, handle: Rect, hot: bool, dragging: bool) {
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    let Some(t) = theme_now() else {
        // Your colours: a soft pill in your colour
        let pl = pal();
        let s = surf();
        p.rect_filled(track.shrink2(egui::vec2(3.0, 0.0)), 2.0, a(s.line, 120));
        let hw = if hot { 10.0 } else { 6.0 };
        let h = Rect::from_center_size(handle.center(), egui::vec2(hw, handle.height()));
        p.rect_filled(h, hw / 2.0, if dragging { pl.accent } else if hot { mix(s.control, pl.accent, 0.6) } else { s.control });
        return;
    };
    let s = t.s;
    let r = handle;
    match t.skin {
        Skin::Pixel => {
            // a pixel key in a sunken square lane
            p.rect_filled(track, 0.0, mix(s.page, Color32::BLACK, 0.12));
            p.rect_stroke(track, 0.0, Stroke::new(1.0f32, mix(s.line, s.text, 0.3)));
            let fill = if hot { mix(t.button, Color32::WHITE, 0.12) } else { t.button };
            p.rect_filled(r.translate(egui::vec2(0.0, 2.0)), 0.0, mix(t.button, Color32::BLACK, 0.5));
            p.rect_filled(r, 0.0, fill);
            p.rect_filled(Rect::from_min_size(r.min, egui::vec2(r.width(), 2.0)), 0.0, mix(fill, Color32::WHITE, 0.4));
            p.rect_filled(Rect::from_min_size(r.min, egui::vec2(2.0, r.height())), 0.0, mix(fill, Color32::WHITE, 0.4));
            // grip lines
            for k in -1..=1 {
                let y = (r.center().y + k as f32 * 4.0).round();
                p.rect_filled(Rect::from_min_size(egui::pos2(r.min.x + 3.0, y), egui::vec2(r.width() - 6.0, 2.0)), 0.0, mix(fill, Color32::BLACK, 0.35));
            }
        }
        Skin::Terminal => {
            // a dotted rail and a solid block cursor
            let mut y = track.min.y;
            while y < track.max.y {
                p.rect_filled(Rect::from_min_size(egui::pos2(track.center().x - 1.0, y), egui::vec2(2.0, 2.0)), 0.0, a(t.accent, 80));
                y += 6.0;
            }
            p.rect_filled(r, 0.0, a(t.accent, if hot { 230 } else { 170 }));
            p.rect_stroke(r.expand(2.0), 0.0, Stroke::new(1.0f32, a(t.accent, if hot { 120 } else { 50 })));
        }
        Skin::Journal => {
            // a pencil line, and a highlighter swipe for the thumb
            p.line_segment([egui::pos2(track.center().x, track.min.y), egui::pos2(track.center().x, track.max.y)], Stroke::new(1.0f32, a(s.text, 60)));
            p.rect_filled(r.shrink2(egui::vec2(1.0, 0.0)), 3.0, a(t.accent2, if hot { 210 } else { 150 }));
            p.rect_stroke(r.shrink2(egui::vec2(1.0, 0.0)), 3.0, Stroke::new(1.2f32, a(s.text, 150)));
        }
        Skin::Sticker | Skin::Brutal => {
            // ink outlines and a hard shadow
            let ink = t.hard_shadow.unwrap_or(s.text);
            let rr = if t.skin == Skin::Sticker { 5.0 } else { 0.0 };
            p.rect_filled(track, rr, s.card);
            p.rect_stroke(track, rr, Stroke::new(2.0f32, ink));
            // (pressed: the shadow closes up)
            let sh = if dragging { 1.0 } else { 3.0 };
            p.rect_filled(r.translate(egui::vec2(sh, sh)), rr, ink);
            p.rect_filled(r, rr, if hot { mix(t.accent, Color32::WHITE, 0.15) } else { t.accent });
            p.rect_stroke(r, rr, Stroke::new(2.0f32, ink));
        }
        Skin::Broadcast => {
            p.rect_filled(track, 0.0, a(s.text, 26));
            p.rect_filled(r, 0.0, t.accent);
            p.rect_filled(Rect::from_min_size(egui::pos2(r.min.x, r.center().y - 2.0), egui::vec2(r.width(), 4.0)), 0.0, if hot { Color32::WHITE } else { a(Color32::WHITE, 170) });
        }
        Skin::Glass | Skin::Aurora | Skin::Liquid => {
            // frosted glass
            p.rect_filled(track, 5.0, Color32::from_white_alpha(if t.light { 40 } else { 10 }));
            p.rect_filled(r, 5.0, if hot { Color32::from_white_alpha(if t.light { 200 } else { 90 }) } else { Color32::from_white_alpha(if t.light { 140 } else { 50 }) });
            p.rect_stroke(r, 5.0, Stroke::new(1.0f32, a(t.accent, if hot { 220 } else { 120 })));
            p.rect_filled(Rect::from_min_size(r.min + egui::vec2(2.0, 3.0), egui::vec2(2.0, (r.height() - 6.0).max(0.0))), 1.0, Color32::from_white_alpha(120));
        }
        Skin::Synth => {
            // neon: a thin glowing rail and a glowing bar
            p.line_segment([egui::pos2(track.center().x, track.min.y), egui::pos2(track.center().x, track.max.y)], Stroke::new(1.0f32, a(t.accent2, 70)));
            for (w, al) in [(8.0f32, 30u8), (4.0, 60)] {
                p.rect_filled(r.expand(w / 2.0), 6.0, a(t.accent, al));
            }
            p.rect_filled(r.shrink2(egui::vec2(2.0, 0.0)), 4.0, if hot { mix(t.accent, Color32::WHITE, 0.3) } else { t.accent });
        }
        Skin::Gold => {
            p.line_segment([egui::pos2(track.center().x, track.min.y), egui::pos2(track.center().x, track.max.y)], Stroke::new(1.0f32, a(t.accent, 60)));
            p.add(gradient_rect(r.shrink2(egui::vec2(2.0, 0.0)), [2.0; 4], mix(t.accent, Color32::WHITE, if hot { 0.35 } else { 0.15 }), mix(t.accent, Color32::BLACK, 0.3)));
            sparkle(p, egui::pos2(r.center().x, r.min.y + 6.0), if hot { 4.0 } else { 3.0 }, Color32::from_rgba_unmultiplied(255, 250, 220, 200));
        }
        Skin::Candy => {
            // a candy-cane stick
            p.rect_filled(track, 5.0, a(t.accent, 26));
            p.rect_filled(r, 5.0, Color32::WHITE);
            let clip = p.with_clip_rect(r.shrink(1.0).intersect(p.clip_rect()));
            let mut y = r.min.y - r.width();
            while y < r.max.y {
                clip.add(Shape::convex_polygon(vec![egui::pos2(r.min.x, y), egui::pos2(r.max.x, y - 6.0), egui::pos2(r.max.x, y), egui::pos2(r.min.x, y + 6.0)], t.accent, Stroke::NONE));
                y += 12.0;
            }
            p.rect_stroke(r, 5.0, Stroke::new(if hot { 2.0f32 } else { 1.2f32 }, mix(t.accent, Color32::BLACK, 0.2)));
        }
        Skin::Winter => {
            // an icicle
            p.rect_filled(track, 5.0, a(s.text, 14));
            p.rect_filled(r, 5.0, mix(t.accent, Color32::WHITE, if hot { 0.55 } else { 0.35 }));
            p.rect_filled(Rect::from_min_size(r.min + egui::vec2(2.0, 3.0), egui::vec2(2.0, (r.height() - 6.0).max(0.0))), 1.0, Color32::from_white_alpha(200));
        }
        Skin::Spooky => {
            // a bone-ish pumpkin bar on a dark lane
            p.rect_filled(track, 5.0, a(Color32::BLACK, 60));
            p.rect_filled(r, 5.0, if hot { mix(t.accent, Color32::WHITE, 0.15) } else { t.accent });
            p.circle_filled(egui::pos2(r.center().x, r.min.y + 4.0), 4.0, mix(t.accent, Color32::BLACK, 0.3));
            p.circle_filled(egui::pos2(r.center().x, r.max.y - 4.0), 4.0, mix(t.accent, Color32::BLACK, 0.3));
        }
        Skin::Field => {
            // a ruler
            p.rect_filled(track, 2.0, a(t.accent, 20));
            p.rect_filled(r, 2.0, if hot { mix(s.card, t.accent, 0.25) } else { s.card });
            p.rect_stroke(r, 2.0, Stroke::new(1.0f32, t.accent));
            let mut y = r.min.y + 4.0;
            let mut k = 0;
            while y < r.max.y - 2.0 {
                let w = if k % 4 == 0 { 6.0 } else { 3.0 };
                p.line_segment([egui::pos2(r.min.x + 1.0, y), egui::pos2(r.min.x + 1.0 + w, y)], Stroke::new(1.0f32, a(t.accent, 180)));
                y += 4.0;
                k += 1;
            }
        }
        Skin::Companion => {
            p.rect_filled(track, 5.0, a(t.accent, 30));
            p.rect_filled(r, 5.0, if hot { t.accent2 } else { t.accent });
            p.circle_filled(egui::pos2(r.center().x, r.min.y + 5.0), 2.0, Color32::from_white_alpha(200));
        }
        Skin::Studio => {
            p.rect_filled(track.shrink2(egui::vec2(3.0, 0.0)), 2.0, a(s.line, 140));
            let hw = if hot { 10.0 } else { 6.0 };
            let h = Rect::from_center_size(r.center(), egui::vec2(hw, r.height()));
            p.add(gradient_rect(h, [hw / 2.0; 4], t.accent, t.accent2));
        }
        Skin::Plain => {
            let hw = if hot { 10.0 } else { 6.0 };
            let h = Rect::from_center_size(r.center(), egui::vec2(hw, r.height()));
            p.rect_filled(h, hw / 2.0, if hot { t.accent } else { s.control });
        }
    }
}

// ── charts and stats drawn in each theme's own style (bars, tracks, grid
// lines, lines, dots, rings, heat-map squares)

fn chart_skin() -> Option<(ThemeDef, Skin)> {
    theme_now().filter(|_| plus()).map(|t| (t, t.skin))
}

#[allow(dead_code)]
fn vgrad_rect(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

/// One bar of a chart (or the filled part of a progress bar) in the theme's
/// style: pixel blocks, terminal slices, pencil hatching, inked stickers,
/// neon tubes, gold, candy stripes, ice… `col` is already the bar's colour
/// (faded if it isn't the one picked out); `hot`: the one picked out.
pub fn chart_bar(p: &Painter, r: Rect, col: Color32, hot: bool) {
    let rad = (r.width().min(r.height()) / 2.0).min(6.0);
    let Some((t, sk)) = chart_skin() else {
        p.rect_filled(r, rad, col);
        return;
    };
    let vertical = r.height() >= r.width();
    let ink = t.hard_shadow.unwrap_or(t.s.text);
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    match sk {
        Skin::Pixel => {
            // whole 4px steps, a light bevel and a dark edge
            let q = |v: f32| (v / 4.0).round() * 4.0;
            let r = Rect::from_min_max(egui::pos2(q(r.min.x), q(r.min.y)), egui::pos2(q(r.max.x).max(q(r.min.x) + 4.0), q(r.max.y).max(q(r.min.y) + 4.0)));
            p.rect_filled(r, 0.0, col);
            let lite = mix(col, Color32::WHITE, 0.35);
            let dark = mix(col, Color32::BLACK, 0.35);
            p.rect_filled(Rect::from_min_size(r.min, egui::vec2(r.width(), 2.0)), 0.0, lite);
            p.rect_filled(Rect::from_min_size(r.min, egui::vec2(2.0, r.height())), 0.0, lite);
            p.rect_filled(Rect::from_min_max(egui::pos2(r.min.x, r.max.y - 2.0), r.max), 0.0, dark);
            p.rect_filled(Rect::from_min_max(egui::pos2(r.max.x - 2.0, r.min.y), r.max), 0.0, dark);
        }
        Skin::Terminal => {
            // ▓ blocks with gaps, like a text progress bar
            if vertical {
                let mut y = r.max.y;
                while y > r.min.y + 0.5 {
                    let top = (y - 3.0).max(r.min.y);
                    p.rect_filled(Rect::from_min_max(egui::pos2(r.min.x, top), egui::pos2(r.max.x, y)), 0.0, col);
                    y -= 4.0;
                }
            } else {
                let mut x = r.min.x;
                while x < r.max.x - 0.5 {
                    p.rect_filled(Rect::from_min_max(egui::pos2(x, r.min.y), egui::pos2((x + 3.0).min(r.max.x), r.max.y)), 0.0, col);
                    x += 4.0;
                }
            }
        }
        Skin::Journal => {
            // pencil hatching in the colour, inside an inked box
            p.rect_filled(r, 2.0, a(col, 50));
            let clip = p.with_clip_rect(r.intersect(p.clip_rect()));
            let mut x = r.min.x - r.height();
            while x < r.max.x {
                clip.line_segment([egui::pos2(x, r.max.y), egui::pos2(x + r.height(), r.min.y)], Stroke::new(1.4f32, col));
                x += 4.5;
            }
            p.rect_stroke(r, 2.0, Stroke::new(1.3f32, a(t.s.text, 200)));
        }
        Skin::Sticker | Skin::Brutal => {
            // a hard shadow and an ink outline
            let rr = if sk == Skin::Sticker { rad.min(4.0) } else { 0.0 };
            p.rect_filled(r.translate(egui::vec2(3.0, 3.0)), rr, ink);
            p.rect_filled(r, rr, col);
            p.rect_stroke(r, rr, Stroke::new(2.0f32, ink));
        }
        Skin::Broadcast => {
            p.rect_filled(r, 0.0, col);
            let cap = if vertical { Rect::from_min_size(r.min, egui::vec2(r.width(), 3.0)) } else { Rect::from_min_max(egui::pos2(r.max.x - 3.0, r.min.y), r.max) };
            p.rect_filled(cap, 0.0, Color32::WHITE);
        }
        Skin::Glass | Skin::Aurora if !t.light => {
            // smoked glass bars: the colour itself, deepening toward the
            // base, a fine sheen and a lighter rim (a faded bar stays as
            // faded as it was asked to be)
            let fade = col.a() as f32 / 255.0;
            let k = |c: Color32, f: f32| Color32::from_rgba_premultiplied((c.r() as f32 * f) as u8, (c.g() as f32 * f) as u8, (c.b() as f32 * f) as u8, c.a());
            let lift = |c: Color32, t: f32| {
                let w = (255.0 * t * fade) as f32;
                Color32::from_rgba_premultiplied((c.r() as f32 * (1.0 - t) + w) as u8, (c.g() as f32 * (1.0 - t) + w) as u8, (c.b() as f32 * (1.0 - t) + w) as u8, c.a())
            };
            let (top, low) = if hot { (lift(col, 0.3), col) } else { (lift(col, 0.14), k(col, 0.84)) };
            p.add(gradient_rect(r, [rad; 4], top, low));
            let sheen = if vertical { Rect::from_min_size(r.min + egui::vec2(1.5, 2.0), egui::vec2((r.width() * 0.28).max(1.5), (r.height() - 4.0).max(0.0))) } else { Rect::from_min_size(r.min + egui::vec2(2.0, 1.5), egui::vec2((r.width() - 4.0).max(0.0), (r.height() * 0.28).max(1.5))) };
            p.rect_filled(sheen, rad.min(2.0), see(Color32::WHITE, (60.0 * fade) as u8));
            p.rect_stroke(r, rad, Stroke::new(1.0f32, see(Color32::WHITE, ((if hot { 110.0 } else { 55.0 }) * fade) as u8)));
        }
        Skin::Glass | Skin::Aurora | Skin::Liquid => {
            // see-through with a light rim and a sheen
            p.rect_filled(r, rad, a(col, 175));
            let sheen = if vertical { Rect::from_min_size(r.min + egui::vec2(1.5, 2.0), egui::vec2((r.width() * 0.3).max(1.5), (r.height() - 4.0).max(0.0))) } else { Rect::from_min_size(r.min + egui::vec2(2.0, 1.5), egui::vec2((r.width() - 4.0).max(0.0), (r.height() * 0.3).max(1.5))) };
            p.rect_filled(sheen, rad.min(2.0), Color32::from_white_alpha(if t.light { 120 } else { 55 }));
            p.rect_stroke(r, rad, Stroke::new(1.0f32, if t.light { a(t.s.text, 60) } else { Color32::from_white_alpha(90) }));
        }
        Skin::Synth => {
            // a neon tube
            if hot {
                p.rect_filled(r.expand(3.0), rad + 3.0, a(col, 40));
                p.rect_filled(r.expand(1.5), rad + 1.5, a(col, 70));
            }
            p.rect_filled(r, rad.min(3.0), a(col, 110));
            p.rect_stroke(r, rad.min(3.0), Stroke::new(1.6f32, mix(col, Color32::WHITE, 0.25)));
        }
        Skin::Gold => {
            // metal: darker under, a bright top half and a glint
            p.rect_filled(r, rad.min(3.0), mix(col, Color32::BLACK, 0.25));
            let top = if vertical { Rect::from_min_max(r.min, egui::pos2(r.max.x, r.center().y)) } else { Rect::from_min_max(r.min, egui::pos2(r.max.x, r.center().y)) };
            p.rect_filled(top, rad.min(3.0), mix(col, Color32::WHITE, 0.22));
            p.rect_stroke(r, rad.min(3.0), Stroke::new(1.0f32, mix(col, Color32::WHITE, 0.45)));
            if hot && vertical && r.height() > 12.0 {
                sparkle(p, egui::pos2(r.center().x, r.min.y - 6.0), 4.0, mix(col, Color32::WHITE, 0.5));
            }
        }
        Skin::Candy => {
            // a candy stick: stripes, round ends
            let rr = r.width().min(r.height()) / 2.0;
            p.rect_filled(r, rr, col);
            let clip = p.with_clip_rect(r.shrink(1.0).intersect(p.clip_rect()));
            let mut x = r.min.x - r.height();
            while x < r.max.x {
                clip.add(Shape::convex_polygon(vec![egui::pos2(x, r.max.y), egui::pos2(x + 4.0, r.max.y), egui::pos2(x + 4.0 + r.height(), r.min.y), egui::pos2(x + r.height(), r.min.y)], mix(col, Color32::WHITE, 0.42), Stroke::NONE));
                x += 11.0;
            }
            p.rect_stroke(r, rr, Stroke::new(1.0f32, mix(col, Color32::BLACK, 0.2)));
        }
        Skin::Companion => {
            // a soft pill with a little shine
            let rr = r.width().min(r.height()) / 2.0;
            p.rect_filled(r, rr, col);
            let d = if vertical { egui::pos2(r.center().x, r.min.y + rr.min(6.0)) } else { egui::pos2(r.min.x + rr.min(6.0), r.center().y) };
            p.circle_filled(d, (rr * 0.35).clamp(1.0, 3.0), Color32::from_white_alpha(180));
        }
        Skin::Winter => {
            // ice: frosted top, a snow cap
            p.rect_filled(r, rad.max(3.0), col);
            let frost = if vertical { Rect::from_min_max(r.min, egui::pos2(r.max.x, r.min.y + r.height() * 0.4)) } else { Rect::from_min_max(egui::pos2(r.min.x + r.width() * 0.6, r.min.y), r.max) };
            p.rect_filled(frost, rad.max(3.0), a(mix(col, Color32::WHITE, 0.6), 200));
            let cap = if vertical { Rect::from_min_size(r.min - egui::vec2(1.0, 1.5), egui::vec2(r.width() + 2.0, 4.0)) } else { Rect::from_min_max(egui::pos2(r.max.x - 3.0, r.min.y - 1.0), r.max + egui::vec2(1.5, 1.0)) };
            p.rect_filled(cap, 2.0, Color32::from_rgb(246, 251, 255));
        }
        Skin::Spooky => {
            // a glowing ember bar
            if hot {
                p.rect_filled(r.expand(2.5), rad + 2.5, a(col, 45));
            }
            p.rect_filled(r, rad.min(3.0), mix(col, Color32::BLACK, 0.12));
            let edge = if vertical { Rect::from_min_size(r.min, egui::vec2(r.width(), 2.5)) } else { Rect::from_min_max(egui::pos2(r.max.x - 2.5, r.min.y), r.max) };
            p.rect_filled(edge, 1.0, mix(col, Color32::WHITE, 0.4));
        }
        Skin::Field => {
            // a ruler: an edge and tick marks
            p.rect_filled(r, 2.0, col);
            p.rect_stroke(r, 2.0, Stroke::new(1.0f32, mix(col, Color32::BLACK, 0.3)));
            let tick = a(Color32::WHITE, 140);
            if vertical {
                let mut y = r.max.y - 6.0;
                let mut k = 0;
                while y > r.min.y + 2.0 {
                    let w = if k % 4 == 3 { 0.6 } else { 0.3 };
                    p.line_segment([egui::pos2(r.min.x + 1.0, y), egui::pos2(r.min.x + 1.0 + r.width() * w, y)], Stroke::new(1.0f32, tick));
                    y -= 6.0;
                    k += 1;
                }
            } else {
                let mut x = r.min.x + 6.0;
                let mut k = 0;
                while x < r.max.x - 2.0 {
                    let h = if k % 4 == 3 { 0.6 } else { 0.3 };
                    p.line_segment([egui::pos2(x, r.min.y + 1.0), egui::pos2(x, r.min.y + 1.0 + r.height() * h)], Stroke::new(1.0f32, tick));
                    x += 6.0;
                    k += 1;
                }
            }
        }
        Skin::Studio => {
            p.add(gradient_rect(r, [rad; 4], col, mix(col, t.accent2, 0.45)));
        }
        Skin::Plain => {
            p.rect_filled(r, rad, col);
        }
    }
}

/// The empty track behind a bar, in the theme's style.
pub fn chart_track(p: &Painter, r: Rect, col: Color32) {
    let rad = (r.width().min(r.height()) / 2.0).min(6.0);
    let Some((t, sk)) = chart_skin() else {
        p.rect_filled(r, rad, col);
        return;
    };
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    match sk {
        Skin::Pixel | Skin::Terminal | Skin::Broadcast => {
            p.rect_filled(r, 0.0, col);
        }
        Skin::Brutal => {
            p.rect_filled(r, 0.0, col);
            p.rect_stroke(r, 0.0, Stroke::new(1.5f32, t.hard_shadow.unwrap_or(t.s.text)));
        }
        Skin::Journal => {
            p.rect_stroke(r, 2.0, Stroke::new(1.0f32, a(t.s.text, 70)));
        }
        Skin::Synth => {
            p.rect_filled(r, rad.min(3.0), a(t.accent2, 26));
        }
        Skin::Candy | Skin::Companion => {
            p.rect_filled(r, r.width().min(r.height()) / 2.0, col);
        }
        _ => {
            p.rect_filled(r, rad, col);
        }
    }
}

/// A chart's grid line, in the theme's style (dotted, ruled, neon…).
pub fn chart_grid(p: &Painter, a0: Pos2, b0: Pos2, col: Color32) {
    let Some((t, sk)) = chart_skin() else {
        p.add(Shape::dashed_line(&[a0, b0], Stroke::new(1.0f32, col), 3.0, 3.0));
        return;
    };
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    match sk {
        Skin::Terminal => {
            p.add(Shape::dashed_line(&[a0, b0], Stroke::new(1.0f32, a(t.accent, 90)), 1.0, 3.0));
        }
        Skin::Pixel | Skin::Broadcast => {
            p.line_segment([a0, b0], Stroke::new(1.0f32, col));
        }
        Skin::Journal => {
            p.line_segment([a0, b0], Stroke::new(1.0f32, Color32::from_rgba_unmultiplied(120, 160, 214, 110)));
        }
        Skin::Synth => {
            p.line_segment([a0, b0], Stroke::new(3.0f32, a(t.accent2, 18)));
            p.line_segment([a0, b0], Stroke::new(1.0f32, a(t.accent2, 70)));
        }
        Skin::Brutal => {
            p.line_segment([a0, b0], Stroke::new(1.5f32, a(t.s.text, 110)));
        }
        Skin::Gold => {
            p.line_segment([a0, b0], Stroke::new(1.0f32, a(t.accent, 60)));
        }
        Skin::Field => {
            p.add(Shape::dashed_line(&[a0, b0], Stroke::new(1.0f32, a(t.accent, 80)), 6.0, 3.0));
        }
        _ => {
            p.add(Shape::dashed_line(&[a0, b0], Stroke::new(1.0f32, col), 3.0, 3.0));
        }
    }
}

/// A chart line through `pts`, in the theme's style: neon glow, pixel
/// steps, pencil strokes, an inked outline, a gold glint…
pub fn chart_line(p: &Painter, pts: &[Pos2], w: f32, col: Color32) {
    if pts.len() < 2 {
        return;
    }
    let plain = |p: &Painter, w: f32, c: Color32| {
        p.add(Shape::line(pts.to_vec(), Stroke::new(w, c)));
    };
    let Some((t, sk)) = chart_skin() else {
        plain(p, w, col);
        return;
    };
    let a = |c: Color32, al: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), al);
    match sk {
        Skin::Synth | Skin::Aurora | Skin::Glass => {
            plain(p, w + 7.0, a(col, 30));
            plain(p, w + 3.0, a(col, 70));
            plain(p, w, mix(col, Color32::WHITE, 0.2));
        }
        Skin::Pixel => {
            // stairs: across, then up or down
            let mut v = vec![pts[0]];
            for q in &pts[1..] {
                let last = *v.last().unwrap_or(q);
                v.push(egui::pos2(q.x, last.y));
                v.push(*q);
            }
            p.add(Shape::line(v, Stroke::new(w.max(3.0), col)));
        }
        Skin::Journal => {
            plain(p, w * 0.8, col);
            let off: Vec<Pos2> = pts.iter().map(|q| *q + egui::vec2(0.6, -0.8)).collect();
            p.add(Shape::line(off, Stroke::new(1.0f32, a(t.s.text, 120))));
        }
        Skin::Brutal | Skin::Sticker => {
            plain(p, w + 3.0, t.hard_shadow.unwrap_or(t.s.text));
            plain(p, w, col);
        }
        Skin::Gold => {
            plain(p, w, mix(col, Color32::BLACK, 0.15));
            let hi: Vec<Pos2> = pts.iter().map(|q| *q + egui::vec2(0.0, -w * 0.3)).collect();
            p.add(Shape::line(hi, Stroke::new((w * 0.35).max(1.0), a(mix(col, Color32::WHITE, 0.6), 200))));
        }
        Skin::Spooky => {
            plain(p, w + 4.0, a(col, 40));
            plain(p, w, col);
        }
        _ => plain(p, w, col),
    }
}

/// A point on a chart in the theme's shape: square (pixel, terminal,
/// brutal), inked (sticker), a little star (gold), a heart (companion),
/// a glowing dot (neon themes), else round.
pub fn chart_dot(p: &Painter, c: Pos2, r: f32, col: Color32) {
    let Some((t, sk)) = chart_skin() else {
        p.circle_filled(c, r, col);
        return;
    };
    match sk {
        Skin::Pixel | Skin::Terminal => {
            p.rect_filled(Rect::from_center_size(c, egui::vec2(r * 1.7, r * 1.7)), 0.0, col);
        }
        Skin::Brutal => {
            let q = Rect::from_center_size(c, egui::vec2(r * 1.8, r * 1.8));
            p.rect_filled(q.translate(egui::vec2(2.0, 2.0)), 0.0, t.hard_shadow.unwrap_or(t.s.text));
            p.rect_filled(q, 0.0, col);
            p.rect_stroke(q, 0.0, Stroke::new(1.5f32, t.hard_shadow.unwrap_or(t.s.text)));
        }
        Skin::Sticker => {
            p.circle_filled(c, r + 1.5, t.hard_shadow.unwrap_or(t.s.text));
            p.circle_filled(c, r, col);
        }
        Skin::Gold => sparkle(p, c, r * 1.5, col),
        Skin::Companion => {
            let s = r * 0.9;
            p.circle_filled(c + egui::vec2(-s * 0.5, -s * 0.2), s * 0.6, col);
            p.circle_filled(c + egui::vec2(s * 0.5, -s * 0.2), s * 0.6, col);
            p.add(Shape::convex_polygon(vec![c + egui::vec2(-s * 1.05, 0.0), c + egui::vec2(s * 1.05, 0.0), c + egui::vec2(0.0, s * 1.15)], col, Stroke::NONE));
        }
        Skin::Synth | Skin::Aurora | Skin::Spooky => {
            p.circle_filled(c, r + 3.0, col.gamma_multiply(0.25));
            p.circle_filled(c, r, col);
            p.circle_filled(c, r * 0.4, Color32::from_white_alpha(200));
        }
        Skin::Journal => {
            p.circle_stroke(c, r, Stroke::new(1.6f32, col));
        }
        _ => {
            p.circle_filled(c, r, col);
        }
    }
}

/// One heat-map square in the theme's shape.
pub fn chart_cell(p: &Painter, r: Rect, col: Color32) {
    let Some((t, sk)) = chart_skin() else {
        p.rect_filled(r, 3.0, col);
        return;
    };
    match sk {
        Skin::Pixel | Skin::Terminal | Skin::Brutal | Skin::Broadcast => {
            p.rect_filled(r, 0.0, col);
        }
        Skin::Companion | Skin::Candy => {
            p.circle_filled(r.center(), r.width().min(r.height()) / 2.0, col);
        }
        Skin::Journal => {
            p.rect_filled(r.shrink(0.5), 1.0, col);
            p.rect_stroke(r.shrink(0.5), 1.0, Stroke::new(0.8f32, Color32::from_rgba_unmultiplied(t.s.text.r(), t.s.text.g(), t.s.text.b(), 60)));
        }
        Skin::Synth => {
            p.rect_filled(r, 2.0, col);
            p.rect_stroke(r, 2.0, Stroke::new(1.0f32, mix(col, Color32::WHITE, 0.2)));
        }
        Skin::Winter => {
            p.rect_filled(r, r.width() * 0.3, col);
        }
        _ => {
            p.rect_filled(r, 3.0, col);
        }
    }
}

/// How rings and round gauges are drawn: (pixel squares, pencil, glow, ink).
pub fn chart_ring_style() -> u8 {
    match chart_skin().map(|x| x.1) {
        Some(Skin::Pixel | Skin::Terminal) => 1,
        Some(Skin::Journal) => 2,
        Some(Skin::Synth | Skin::Aurora | Skin::Spooky | Skin::Glass) => 3,
        Some(Skin::Brutal | Skin::Sticker) => 4,
        _ => 0,
    }
}

/// The ink colour of the theme (for inked ring edges).
pub fn chart_ink() -> Color32 {
    chart_skin().map_or(Color32::BLACK, |(t, _)| t.hard_shadow.unwrap_or(t.s.text))
}
