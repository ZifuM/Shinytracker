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

const MODERN_LIGHT: Surf = Surf {
    sidebar: rgb(255, 255, 255),
    page: rgb(243, 244, 248),
    card: rgb(255, 255, 255),
    card_hover: rgb(245, 246, 250),
    line: rgb(228, 230, 238),
    control: rgb(204, 208, 220),
    selected: rgb(236, 238, 246),
    text: rgb(24, 27, 40),
    muted: rgb(100, 106, 128),
    field: rgb(247, 248, 251),
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
        p.button2 = p.accent2;
    }
    if let Ok(mut w) = PAL.write() {
        *w = p;
    }
    MODERN.store(a.modern_ui, std::sync::atomic::Ordering::Relaxed);
    CLEAN.store(a.clean_settings || a.modern_ui, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut w) = SURF.write() {
        *w = match (a.modern_ui, a.mode) {
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
    let pl = pal();
    let k = if hovered { 0.14 } else { 0.0 };
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
