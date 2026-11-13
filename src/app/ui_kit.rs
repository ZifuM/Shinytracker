//! Small styled widgets for the settings window: cards, toggles, segmented
//! buttons, choice tiles, status pills, steppers and key caps.

use crate::fonts::pixel_font;
use crate::theme::{self, GOLD, OUTLINE, RED};
use eframe::egui::{self, Align2, Color32, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};

#[allow(non_snake_case)]
pub fn SIDEBAR() -> Color32 {
    if clean() { theme::surf().sidebar } else { theme::pal().sidebar }
}
#[allow(non_snake_case)]
pub fn PAGE() -> Color32 {
    if clean() { theme::surf().page } else { theme::pal().page }
}
#[allow(non_snake_case)]
pub fn CARD() -> Color32 {
    if clean() { theme::surf().card } else { theme::pal().card }
}
#[allow(non_snake_case)]
pub fn CARD_HOVER() -> Color32 {
    if clean() { theme::surf().card_hover } else { theme::pal().card_hover }
}
#[allow(non_snake_case)]
pub fn LINE() -> Color32 {
    if clean() { theme::surf().line } else { theme::pal().line }
}
#[allow(non_snake_case)]
pub fn SELECTED_BG() -> Color32 {
    if clean() { theme::surf().selected } else { theme::pal().selected_bg }
}
#[allow(non_snake_case)]
pub fn TEXT() -> Color32 {
    if clean() { theme::surf().text } else { theme::pal().text }
}
/// Secondary text. (Settings pages use this one; the overlay keeps theme::MUTED.)
#[allow(non_snake_case)]
pub fn MUTED() -> Color32 {
    if clean() { theme::surf().muted } else { theme::MUTED() }
}
/// Outline of buttons, text boxes and segmented groups.
#[allow(non_snake_case)]
pub fn CONTROL() -> Color32 {
    if clean() { theme::surf().control } else { theme::pal().line }
}

// ───────────────────────── clean style helpers ─────────────────────────
//
// The clean style (the default; Appearance → Style → Clean / Classic) follows a few fixed rules so
// every page matches:
//   · type: Segoe UI; page title 22 semibold, section 15 semibold, body 13.5,
//     secondary 12.5, nothing under 12
//   · controls are 30px tall; buttons are pills, inputs 6px, cards 8px
//   · the accent (button colour) only on primary buttons, switches, checked
//     boxes, selected-tile borders and focus; everything else is grey

/// Is the clean settings style on? (Also true in the modern style, which
/// builds on it.)
pub fn clean() -> bool {
    theme::clean()
}

/// Is the modern style on (the default)?
pub fn modern() -> bool {
    theme::modern()
}

// ───────────────────────── Cirrus controls ─────────────────────────
// The modern style's controls follow the Cirrus design system: every
// interactive control is a full pill with a 1px Edge hairline, primary
// actions and the open tab use the user's button colour, hovers wash to
// Sunken, focus is a soft Signal-blue ring, and shadows are whisper-soft.
/// Cirrus Signal Blue: focus rings and text selection only.
pub const SIGNAL: Color32 = Color32::from_rgb(46, 125, 239);

/// Fill of primary buttons, the open tab, switches that are on and ticked
/// boxes: the user's button colour (Appearance), which is their accent
/// unless they picked a separate one.
pub fn primary() -> Color32 {
    theme::BUTTON()
}

/// Text and ticks on `primary()`: black or white, whichever reads.
pub fn on_primary() -> Color32 {
    theme::button_ink()
}

/// `primary()` under the pointer.
pub fn primary_hover() -> Color32 {
    theme::blend(theme::BUTTON(), Color32::WHITE, 0.14)
}

/// Paint a primary pill in the user's button colour or gradient.
pub fn primary_shape(rect: Rect, hovered: bool) -> egui::Shape {
    theme::button_shape(rect, rect.height() / 2.0, hovered)
}

/// `primary_shape` part-way into its hover (`t` 0..1).
pub fn primary_shape_t(rect: Rect, t: f32) -> egui::Shape {
    theme::button_shape_t(rect, rect.height() / 2.0, t)
}

// ───────────────────────── motion ─────────────────────────
// Cirrus motion: quick (140ms) for hovers and presses, base (220ms) for
// things that move, a little longer for a page coming in; all on a soft
// ease-out (fast start, gentle landing). Appearance → Animations turns it
// all off: every helper then jumps straight to where it's going.

/// Hovers and presses.
pub const QUICK: f32 = 0.14;
/// Things that move or grow.
pub const BASE: f32 = 0.24;
/// A page or step coming in.
pub const ENTER: f32 = 0.32;

pub fn motion() -> bool {
    theme::motion()
}

/// Soft ease-out (close to Cirrus's cubic-bezier(0.22, 1, 0.36, 1)).
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(4)
}

/// 0..1 easing towards `on` (hover fades, switches).
pub fn anim(ctx: &egui::Context, id: egui::Id, on: bool, secs: f32) -> f32 {
    if !motion() {
        return if on { 1.0 } else { 0.0 };
    }
    let t = ctx.animate_bool_with_time(id, on, secs);
    if on { ease(t) } else { 1.0 - ease(1.0 - t) }
}

#[derive(Clone, Copy)]
struct Glide {
    from: [f32; 4],
    to: [f32; 4],
    start: f64,
}

fn glide_at(g: &Glide, now: f64, secs: f32) -> ([f32; 4], bool) {
    let t = ((now - g.start) as f32 / secs.max(0.001)).clamp(0.0, 1.0);
    let e = ease(t);
    let mut v = [0.0; 4];
    for i in 0..4 {
        v[i] = g.from[i] + (g.to[i] - g.from[i]) * e;
    }
    (v, t >= 1.0)
}

/// Up to four numbers gliding to `target` whenever it changes (a sliding
/// tab pill, a bar growing). `from` = where it starts the first time it's
/// seen (None: already there).
pub fn glide4(ctx: &egui::Context, id: egui::Id, target: [f32; 4], from: Option<[f32; 4]>, secs: f32) -> [f32; 4] {
    if !motion() {
        return target;
    }
    let now = ctx.input(|i| i.time);
    let g = ctx.data_mut(|d| {
        let g = d.get_temp_mut_or_insert_with(id, || Glide { from: from.unwrap_or(target), to: target, start: now });
        if g.to.iter().zip(target.iter()).any(|(a, b)| (a - b).abs() > 0.05) {
            let (cur, _) = glide_at(g, now, secs);
            *g = Glide { from: cur, to: target, start: now };
        }
        *g
    });
    let (v, done) = glide_at(&g, now, secs);
    if !done {
        ctx.request_repaint();
    }
    v
}

pub fn glide(ctx: &egui::Context, id: egui::Id, target: f32, from: Option<f32>, secs: f32) -> f32 {
    glide4(ctx, id, [target, 0.0, 0.0, 0.0], from.map(|f| [f, 0.0, 0.0, 0.0]), secs)[0]
}

/// A rect gliding to `target`. The glide runs relative to `anchor` (the
/// track's corner), so scrolling or moving the window carries the pill
/// along instead of making it chase its track.
pub fn glide_rect(ctx: &egui::Context, id: egui::Id, target: Rect, anchor: egui::Pos2, secs: f32) -> Rect {
    let t = target.translate(-anchor.to_vec2());
    let v = glide4(ctx, id, [t.min.x, t.min.y, t.max.x, t.max.y], None, secs);
    Rect::from_min_max(egui::pos2(v[0], v[1]), egui::pos2(v[2], v[3])).translate(anchor.to_vec2())
}

/// How far (0..1, eased) something keyed by `key` has come in since the key
/// last changed, over `secs`. Used for pages and tour steps fading in.
pub fn entering(ctx: &egui::Context, id: egui::Id, key: u64, secs: f32) -> f32 {
    if !motion() {
        return 1.0;
    }
    let now = ctx.input(|i| i.time);
    let (k, at) = ctx.data_mut(|d| {
        let e = d.get_temp_mut_or_insert_with(id, || (key, now - secs as f64));
        if e.0 != key {
            *e = (key, now);
        }
        *e
    });
    let _ = k;
    let t = ((now - at) as f32 / secs).clamp(0.0, 1.0);
    if t < 1.0 {
        ctx.request_repaint();
    }
    ease(t)
}

/// Fade a ui in and lift it into place, `t` 0..1 from `entering`.
pub fn enter_ui(ui: &mut Ui, t: f32, rise: f32) {
    if t >= 1.0 {
        return;
    }
    ui.set_opacity(t.max(0.0));
    ui.add_space(rise * (1.0 - t));
}

/// A number that changes each time a page is opened (so bars on it can
/// grow in again). Bumped by the settings window when the page changes.
pub fn page_visit(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<u64>(egui::Id::new("page-visit"))).unwrap_or(0)
}

pub fn bump_page_visit(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_default::<u64>(egui::Id::new("page-visit"));
        *v = v.wrapping_add(1);
    });
}

/// A bar's filled fraction growing in from 0 each time its page opens.
pub fn grow(ctx: &egui::Context, id: egui::Id, frac: f32) -> f32 {
    let id = id.with(page_visit(ctx));
    glide(ctx, id, frac, Some(0.0), 0.7)
}

/// Cirrus Sunken: the hover wash on Cloud surfaces.
pub fn sunken() -> Color32 {
    if theme::pal().light {
        Color32::from_rgb(243, 246, 250)
    } else {
        theme::blend(CARD(), TEXT(), 0.07)
    }
}

/// The Signal-blue focus ring round a control.
pub fn focus_ring(p: &egui::Painter, rect: Rect, rounding: f32) {
    p.rect_stroke(rect.expand(2.5), rounding + 2.5, Stroke::new(3.0f32, SIGNAL.gamma_multiply(0.3)));
    p.rect_stroke(rect, rounding, Stroke::new(1.0f32, SIGNAL));
}

/// Cirrus elevation tier 1 (cards, inputs, the tab track) and tier 2 (the
/// open tab, popovers).
pub fn shadow_tier(tier: u8) -> egui::epaint::Shadow {
    let light = theme::pal().light;
    let a = match (tier, light) {
        (1, true) => 14,
        (_, true) => 24,
        (1, false) => 60,
        (_, false) => 90,
    };
    egui::epaint::Shadow {
        offset: egui::vec2(0.0, if tier == 1 { 6.0 } else { 4.0 }),
        blur: if tier == 1 { 16.0 } else { 10.0 },
        spread: 0.0,
        color: Color32::from_black_alpha(a),
    }
}

// ───────────────────────── modern style palette ─────────────────────────
// Colour-coded icon chips (sidebar, page headers, stat tiles). Each page
// keeps its colour everywhere it appears, so it's recognisable at a glance.

pub const C_BLUE: Color32 = Color32::from_rgb(86, 140, 255);
pub const C_VIOLET: Color32 = Color32::from_rgb(150, 104, 255);
pub const C_PINK: Color32 = Color32::from_rgb(240, 92, 150);
pub const C_ORANGE: Color32 = Color32::from_rgb(255, 142, 66);
pub const C_TEAL: Color32 = Color32::from_rgb(38, 190, 170);
pub const C_GREEN: Color32 = Color32::from_rgb(70, 200, 120);
pub const C_GOLD: Color32 = Color32::from_rgb(240, 188, 60);
pub const C_RED: Color32 = Color32::from_rgb(236, 84, 84);
pub const C_CYAN: Color32 = Color32::from_rgb(56, 182, 240);
pub const C_INDIGO: Color32 = Color32::from_rgb(112, 120, 255);
pub const C_SLATE: Color32 = Color32::from_rgb(132, 146, 176);

/// `c` laid over a card at strength `a` (0..1): soft tinted backgrounds.
pub fn tint(c: Color32, a: f32) -> Color32 {
    theme::blend(CARD(), c, a)
}

/// The page's icon and colour, shown in its header (set by the window
/// before each page is drawn).
static PAGE_ICON: std::sync::RwLock<Option<(&'static str, Color32)>> = std::sync::RwLock::new(None);

pub fn set_page_icon(icon: Option<(&'static str, Color32)>) {
    if let Ok(mut w) = PAGE_ICON.write() {
        *w = icon;
    }
}

fn page_icon() -> Option<(&'static str, Color32)> {
    PAGE_ICON.read().ok().and_then(|g| *g)
}

/// A rounded square with an icon: solid in its colour, or a soft tint with
/// the icon in the colour.
pub fn icon_chip(p: &egui::Painter, rect: Rect, icon: &str, color: Color32, solid: bool) {
    let r = (rect.height() * 0.3).round();
    if solid {
        p.rect_filled(rect, r, color);
        p.text(rect.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(rect.height() * 0.56), Color32::WHITE);
    } else {
        p.rect_filled(rect, r, theme::blend(CARD(), color, if theme::pal().light { 0.14 } else { 0.2 }));
        p.text(rect.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(rect.height() * 0.56), color);
    }
}

/// The soft drop shadow under modern cards.
pub fn card_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if theme::pal().light { 16 } else { 70 }),
    }
}

/// Dashboard tile: icon chip, small label, big value and an optional
/// coloured note under it. Clickable (hover lifts the border).
pub fn stat_tile(ui: &mut Ui, width: f32, icon: &str, color: Color32, label: &str, value: &str, note: Option<(&str, Color32)>) -> Response {
    let h = if note.is_some() { 104.0 } else { 88.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, h), Sense::click());
    let p = ui.painter_at(rect.expand(12.0));
    let hovered = resp.hovered();
    // Lifts a little on hover.
    let lift = anim(ui.ctx(), resp.id.with("lift"), hovered, QUICK);
    let rect = rect.translate(egui::vec2(0.0, -2.0 * lift));
    p.add(card_shadow().as_shape(rect, 14.0));
    if lift > 0.0 {
        p.add(shadow_tier(2).as_shape(rect, 14.0));
    }
    p.rect_filled(rect, 14.0, CARD());
    p.rect_stroke(rect, 14.0, Stroke::new(1.0f32, if hovered { theme::blend(LINE(), color, 0.6) } else { LINE() }));
    let chip = Rect::from_min_size(rect.min + egui::vec2(14.0, 14.0), egui::vec2(30.0, 30.0));
    icon_chip(&p, chip, icon, color, false);
    let tx = chip.max.x + 10.0;
    let lab = p.layout_no_wrap(label.to_string(), crate::fonts::sans_font(12.5), MUTED());
    let clip = Rect::from_min_max(egui::pos2(tx, rect.min.y), egui::pos2(rect.max.x - 10.0, rect.max.y));
    p.with_clip_rect(clip).galley(egui::pos2(tx, chip.center().y - lab.size().y / 2.0), lab, MUTED());
    let val = p.layout_no_wrap(value.to_string(), crate::fonts::sans_bold_font(22.0), TEXT());
    p.with_clip_rect(rect.shrink(8.0)).galley(egui::pos2(rect.min.x + 16.0, chip.max.y + 8.0), val, TEXT());
    if let Some((t, c)) = note {
        p.text(egui::pos2(rect.min.x + 16.0, rect.max.y - 16.0), Align2::LEFT_CENTER, t, crate::fonts::sans_font(12.0), c);
    }
    p.text(rect.right_bottom() + egui::vec2(-14.0, -16.0), Align2::RIGHT_CENTER, "›", egui::FontId::proportional(16.0), if hovered { TEXT() } else { theme::blend(CARD(), MUTED(), 0.5) });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A thin progress bar in the accent (gradient if the user has one).
pub fn bar(ui: &mut Ui, width: f32, height: f32, frac: f32) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    // Grows in from empty each time its page opens.
    let frac = grow(ui.ctx(), resp.id, frac);
    paint_bar(ui.painter(), rect, frac);
}

pub fn paint_bar(p: &egui::Painter, rect: Rect, frac: f32) {
    let r = rect.height() / 2.0;
    p.rect_filled(rect, r, theme::blend(CARD(), TEXT(), 0.1));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = Rect::from_min_size(rect.min, egui::vec2((rect.width() * f).max(rect.height()), rect.height()));
        let pl = theme::pal();
        p.add(theme::gradient_rect(fill, [r; 4], pl.button, pl.button2));
    }
}

/// Height of every control (buttons, text boxes, segmented groups).
pub const CONTROL_H: f32 = 30.0;

/// Font size for settings text: unchanged in the classic look; in the clean
/// look nothing goes below 12 so it stays readable.
pub fn fs(size: f32) -> f32 {
    if clean() { size.max(12.0) } else { size }
}

/// The family for bold text in a RichText (`.family(strong_family(13.5)).strong()` alone only changes
/// the colour): Inter SemiBold, or Inter Tight for titles (size ≥ 14.5).
pub fn strong_family(size: f32) -> egui::FontFamily {
    if size >= crate::fonts::TITLE_MIN { crate::fonts::title() } else { crate::fonts::sans_bold() }
}

/// Regular font for drawn text in the settings window.
pub fn font(size: f32) -> egui::FontId {
    if clean() { crate::fonts::sans_font(fs(size)) } else { egui::FontId::proportional(size) }
}

/// Small drawn labels in tight spots (chart axes, chips): minimum 11.
pub fn font_small(size: f32) -> egui::FontId {
    if clean() { crate::fonts::sans_font(size.max(11.0)) } else { egui::FontId::proportional(size) }
}

/// Semibold font (headings, values) in the clean look.
pub fn font_bold(size: f32) -> egui::FontId {
    if clean() { crate::fonts::sans_bold_font(fs(size)) } else { egui::FontId::proportional(size) }
}

/// Big display text (counts, names): pixel font in the classic look,
/// semibold sans in the clean look (sized up a little to match).
pub fn display_font(size: f32) -> egui::FontId {
    if clean() { crate::fonts::sans_bold_font((size * 1.3).round()) } else { pixel_font(size) }
}

/// Corner radius for cards and tiles: 8 at most in the clean look.
pub fn r(radius: f32) -> f32 {
    if modern() {
        radius.min(14.0)
    } else if clean() {
        radius.min(8.0)
    } else {
        radius
    }
}

/// Colour for "this one is selected" borders: the button colour in the
/// clean look (matching switches and ticks), the accent in the classic one.
#[allow(non_snake_case)]
pub fn ACCENT() -> Color32 {
    if clean() { theme::BUTTON() } else { RED() }
}

/// Rounding for the solid button-colour buttons in the counting wizard:
/// pills in the clean look, egui's default otherwise.
pub fn primary_round() -> f32 {
    if modern() {
        CONTROL_H
    } else if clean() {
        CONTROL_H / 2.0
    } else {
        2.0
    }
}

/// Fill for ordinary (secondary) buttons: grey in the classic look,
/// see-through outlined pill in the clean look.
pub fn btn_fill() -> Color32 {
    if modern() {
        CARD()
    } else if clean() {
        Color32::TRANSPARENT
    } else {
        CARD_HOVER()
    }
}

/// Button corner radius: pills in the clean look.
pub fn btn_round(r: f32) -> f32 {
    if modern() {
        CONTROL_H
    } else if clean() {
        CONTROL_H / 2.0
    } else {
        r
    }
}

/// Apply the clean style to a settings panel (no-op in the classic look).
/// Sets the font, control outlines, rounding and neutral widget colours.
pub fn settings_style(ui: &mut Ui) {
    if !clean() {
        return;
    }
    let sf = theme::surf();
    let style = ui.style_mut();
    for (ts, f) in style.text_styles.iter_mut() {
        match ts {
            egui::TextStyle::Monospace => {}
            egui::TextStyle::Heading => {
                f.family = crate::fonts::title();
                f.size = 20.0;
            }
            egui::TextStyle::Button => {
                f.family = crate::fonts::sans_medium();
                f.size = 13.5;
            }
            egui::TextStyle::Small => {
                f.family = crate::fonts::sans();
                f.size = 12.0;
            }
            _ => {
                f.family = crate::fonts::sans();
                f.size = 13.5;
            }
        }
    }
    style.spacing.button_padding = egui::vec2(14.0, 5.0);
    style.spacing.interact_size.y = CONTROL_H;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    let v = &mut style.visuals;
    v.override_text_color = Some(sf.text);
    v.extreme_bg_color = sf.field;
    v.faint_bg_color = sf.card_hover;
    v.window_fill = sf.card;
    v.panel_fill = sf.page;
    v.selection.bg_fill = theme::BUTTON().gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0f32, theme::BUTTON());
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, sf.line);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, sf.text);
    let hover_line = theme::blend(sf.control, sf.text, 0.45);
    let w = &mut v.widgets;
    w.inactive.bg_fill = sf.card_hover;
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_stroke = Stroke::new(1.0f32, sf.control);
    w.inactive.fg_stroke = Stroke::new(1.0f32, sf.text);
    w.hovered.bg_fill = sf.selected;
    w.hovered.weak_bg_fill = sf.card_hover;
    w.hovered.bg_stroke = Stroke::new(1.0f32, hover_line);
    w.hovered.fg_stroke = Stroke::new(1.5f32, sf.text);
    w.active.bg_fill = sf.selected;
    w.active.weak_bg_fill = sf.selected;
    w.active.bg_stroke = Stroke::new(1.0f32, sf.text);
    w.active.fg_stroke = Stroke::new(1.5f32, sf.text);
    w.open.bg_fill = sf.selected;
    w.open.weak_bg_fill = sf.card_hover;
    w.open.bg_stroke = Stroke::new(1.0f32, hover_line);
    for x in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        x.rounding = egui::Rounding::same(6.0);
        x.expansion = 0.0;
    }
    if modern() {
        // Cirrus: Cloud pills with an Edge hairline; hover washes to
        // Sunken; text boxes are Cloud pills that focus with a Signal ring.
        let sunk = sunken();
        w.inactive.weak_bg_fill = sf.card;
        w.inactive.bg_fill = sunk;
        w.inactive.bg_stroke = Stroke::new(1.0f32, sf.line);
        w.hovered.weak_bg_fill = sunk;
        w.hovered.bg_fill = sunk;
        w.hovered.bg_stroke = Stroke::new(1.0f32, sf.control);
        w.active.weak_bg_fill = theme::blend(sunk, sf.text, 0.06);
        w.active.bg_fill = theme::blend(sunk, sf.text, 0.06);
        w.active.bg_stroke = Stroke::new(1.0f32, sf.control);
        w.open.weak_bg_fill = sunk;
        w.open.bg_fill = sunk;
        w.open.bg_stroke = Stroke::new(1.0f32, sf.control);
        for x in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            x.rounding = egui::Rounding::same(CONTROL_H);
        }
        v.extreme_bg_color = if theme::pal().light { sf.card } else { sf.field };
        v.window_rounding = egui::Rounding::same(20.0);
        v.menu_rounding = egui::Rounding::same(14.0);
        v.window_shadow = shadow_tier(2);
        v.popup_shadow = shadow_tier(2);
        v.window_stroke = Stroke::new(1.0f32, sf.line);
        v.selection.bg_fill = SIGNAL.gamma_multiply(if theme::pal().light { 0.42 } else { 0.5 });
        v.selection.stroke = Stroke::new(1.0f32, SIGNAL);
        v.slider_trailing_fill = true;
        v.hyperlink_color = if theme::pal().light { SIGNAL } else { theme::blend(SIGNAL, Color32::WHITE, 0.3) };
        v.handle_shape = egui::style::HandleShape::Circle;
        style.spacing.button_padding = egui::vec2(16.0, 6.0);
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.slider_rail_height = 6.0;
    }
}

/// Search box. In the clean look it gets a magnifier icon and the standard
/// 30px control height; `width` = f32::INFINITY fills the row.
pub fn search_box(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    search_box_keyed(ui, None, text, hint, width)
}

fn keyed(key: Option<egui::Id>, te: egui::TextEdit<'_>) -> egui::TextEdit<'_> {
    match key {
        Some(id) => te.id(id),
        None => te,
    }
}

/// `search_box` with a fixed id, so it keeps focus when things around it change.
pub fn search_box_keyed(ui: &mut Ui, key: Option<egui::Id>, text: &mut String, hint: &str, width: f32) -> Response {
    if !clean() {
        return ui.add(keyed(
            key,
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .desired_width(width)
                .min_size(egui::vec2(0.0, CONTROL_H))
                .vertical_align(egui::Align::Center)
                .margin(egui::Margin::symmetric(8.0, 5.0)),
        ));
    }
    if modern() {
        return modern::search_box(ui, key, text, hint, width);
    }
    // Where the box's left edge will be. (The response rect is the text area
    // inside the margin, so the icon is placed from here instead.)
    let left = ui.cursor().min.x;
    let resp = ui.add(keyed(
        key,
        egui::TextEdit::singleline(text)
            .hint_text(RichText::new(hint).color(MUTED()))
            .desired_width(width)
            .min_size(egui::vec2(0.0, CONTROL_H))
            .vertical_align(egui::Align::Center)
            .margin(egui::Margin { left: 32.0, right: 8.0, top: 5.0, bottom: 5.0 }),
    ));
    // magnifier: a ring and a short handle
    let c = egui::pos2(left + 14.0, resp.rect.center().y - 1.0);
    let col = if resp.has_focus() { TEXT() } else { MUTED() };
    let p = ui.painter();
    p.circle_stroke(c, 5.0, Stroke::new(1.6f32, col));
    p.line_segment([c + egui::vec2(3.6, 3.6), c + egui::vec2(7.0, 7.0)], Stroke::new(1.8f32, col));
    resp
}

/// Full-width tabs for switching between views of one page. Clean look:
/// outlined bar, grey highlight on the open tab. Classic look: the open tab
/// filled with the button colour. Returns true when the tab changed.
pub fn tab_bar<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, tabs: &[(T, &str)]) -> bool {
    if modern() {
        return modern::tab_bar(ui, value, tabs);
    }
    let mut changed = false;
    let c = clean();
    let h = if c { 36.0 } else { 42.0 };
    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    if c {
        ui.painter().rect_stroke(bar.shrink(0.5), 8.0, Stroke::new(1.0f32, CONTROL()));
    } else {
        ui.painter().rect_filled(bar, 12.0, CARD());
    }
    let n = tabs.len().max(1) as f32;
    let (pad, gap) = if c { (3.0, 2.0) } else { (4.0, 0.0) };
    let cw = (bar.width() - 2.0 * pad - gap * (n - 1.0)) / n;
    for (i, (v, label)) in tabs.iter().enumerate() {
        let r = Rect::from_min_size(
            egui::pos2(bar.min.x + pad + (cw + gap) * i as f32, bar.min.y + pad),
            egui::vec2(cw, h - 2.0 * pad),
        );
        let resp = ui
            .interact(r, ui.id().with(("tab-bar", i, *label)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let sel = *value == *v;
        let hovered = resp.hovered();
        let p = ui.painter();
        let ink = if c {
            if sel {
                p.rect_filled(r, 6.0, SELECTED_BG());
            } else if hovered {
                p.rect_filled(r, 6.0, CARD_HOVER());
            }
            if sel || hovered { TEXT() } else { MUTED() }
        } else {
            if sel {
                p.add(theme::button_shape(r, 9.0, hovered));
            } else if hovered {
                p.rect_filled(r, 9.0, CARD_HOVER());
            }
            if sel { theme::button_ink() } else if hovered { TEXT() } else { MUTED() }
        };
        let font = if c && sel { crate::fonts::sans_bold_font(13.5) } else { font(13.5) };
        p.text(r.center(), Align2::CENTER_CENTER, *label, font, ink);
        if resp.clicked() && !sel {
            *value = *v;
            changed = true;
        }
    }
    changed
}

/// Small all-caps group label in the sidebar (clean look only).
pub fn nav_group(ui: &mut Ui, text: &str) {
    if !clean() {
        return;
    }
    if modern() {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(SIDEBAR(), MUTED(), 0.75)));
        });
        ui.add_space(4.0);
        return;
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(12.0)).color(MUTED()));
    });
    ui.add_space(2.0);
}

/// "TRAINER PROFILE" → "Trainer Profile".
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Checkbox: the standard one in the classic look; in the clean look a
/// rounded box that fills with the button colour when ticked.
pub fn checkbox(ui: &mut Ui, on: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
    if !clean() {
        return ui.checkbox(on, text);
    }
    if modern() {
        return modern::checkbox(ui, on, text);
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (rect, bresp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
        let lresp = ui.add(egui::Label::new(text).sense(Sense::click()));
        let mut resp = bresp | lresp;
        if resp.clicked() {
            *on = !*on;
            resp.mark_changed();
        }
        let hovered = resp.hovered();
        let p = ui.painter();
        let b = Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0));
        if *on {
            p.rect_filled(b, 4.0, theme::BUTTON());
            let ink = theme::button_ink();
            let s = Stroke::new(2.0f32, ink);
            let c = b.center();
            p.line_segment([c + egui::vec2(-4.0, 0.5), c + egui::vec2(-1.0, 3.5)], s);
            p.line_segment([c + egui::vec2(-1.0, 3.5), c + egui::vec2(4.5, -3.0)], s);
        } else {
            p.rect_filled(b, 4.0, theme::surf().field);
            p.rect_stroke(
                b,
                4.0,
                Stroke::new(1.0f32, if hovered { TEXT() } else { CONTROL() }),
            );
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

/// Page title (pixel font) with a one-line subtitle.
pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
    if modern() {
        return modern::page_header(ui, title, subtitle);
    }
    if clean() {
        return clean::page_header(ui, title, subtitle);
    }
    ui.label(RichText::new(title).font(pixel_font(12.0)).color(TEXT()));
    ui.add_space(4.0);
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(12.0).color(MUTED()));
    }
    ui.add_space(14.0);
}

/// "Experimental" notice under the header of an app that's still being built.
pub fn experimental(ui: &mut Ui, what: &str) {
    ui.horizontal_wrapped(|ui| {
        pill(ui, "Experimental", PillKind::Warn);
        ui.label(
            RichText::new(format!("{what} is still being built: some things may be missing or wrong. Tell the dev if you spot something."))
                .size(fs(11.5))
                .color(MUTED()),
        );
    });
    ui.add_space(10.0);
}

/// Small grey label above a group of controls.
pub fn section(ui: &mut Ui, text: &str) {
    if modern() {
        return modern::section(ui, text);
    }
    if clean() {
        return clean::section(ui, text);
    }
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(11.5).color(MUTED()));
    ui.add_space(4.0);
}

/// `id`, or (if another control already used it this frame) `id` with a
/// count added, so look-alike controls never share animation state.
pub(crate) fn unique_this_frame(ui: &Ui, id: egui::Id) -> egui::Id {
    // read the frame number first: asking inside data_mut would deadlock
    let frame = ui.ctx().cumulative_pass_nr();
    // one list per window (the overlay draws in the middle of the main frame)
    let key = egui::Id::new("kit-ids-this-frame").with(ui.ctx().viewport_id());
    ui.ctx().data_mut(|d| {
        let (f, seen) = d.get_temp_mut_or_default::<(u64, std::collections::HashMap<egui::Id, u32>)>(key);
        if *f != frame {
            *f = frame;
            seen.clear();
        }
        let n = seen.entry(id).or_insert(0);
        let out = if *n == 0 { id } else { id.with(*n) };
        *n += 1;
        out
    })
}

/// Remember where the last card was drawn (its frame, without the space
/// after it), for highlights that outline a card.
pub(crate) fn note_card(ui: &Ui, rect: egui::Rect) {
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("kit-last-card"), rect));
}

/// The frame of the card drawn most recently.
pub fn last_card_rect(ui: &Ui) -> Option<egui::Rect> {
    ui.ctx().data(|d| d.get_temp::<egui::Rect>(egui::Id::new("kit-last-card")))
}

/// The pages still being worked on: they carry a ⚠ warning triangle.
pub const EXPERIMENTAL_TIP: &str = "Experimental: still being built and tested, so things may change or not work yet";

/// The yellow warning triangle with a black "!", centred on `c`, `size`
/// points wide. Drawn from shapes so it looks the same in every font.
/// `ring` draws a thin outline in that colour so it stands out on an icon.
pub fn warn_triangle(p: &egui::Painter, c: egui::Pos2, size: f32, ring: Option<Color32>) -> Rect {
    let yellow = Color32::from_rgb(250, 204, 21);
    let ink = Color32::from_rgb(24, 22, 18);
    let h = size * 0.88;
    let top = c + egui::vec2(0.0, -h / 2.0);
    let (bl, br) = (c + egui::vec2(-size / 2.0, h / 2.0), c + egui::vec2(size / 2.0, h / 2.0));
    // a darker edge keeps it readable on light backgrounds
    let ring = ring.or_else(|| theme::pal().light.then(|| Color32::from_rgb(176, 128, 8)));
    if let Some(r) = ring {
        let k = (size * 0.14).max(1.5);
        p.add(egui::Shape::convex_polygon(
            vec![top + egui::vec2(0.0, -k * 1.3), br + egui::vec2(k * 1.15, k * 0.7), bl + egui::vec2(-k * 1.15, k * 0.7)],
            r,
            Stroke::NONE,
        ));
    }
    // fill, then a rounded outline in the same yellow softens the corners
    p.add(egui::Shape::convex_polygon(vec![top, br, bl], yellow, Stroke::NONE));
    let soft = Stroke::new((size * 0.14).max(1.2), yellow);
    p.add(egui::Shape::closed_line(vec![top, br, bl], soft));
    // the "!": a bar and a dot
    let w = (size * 0.13).max(1.4);
    let bar_top = c + egui::vec2(0.0, -h * 0.16);
    let bar_bot = c + egui::vec2(0.0, h * 0.17);
    p.line_segment([bar_top, bar_bot], Stroke::new(w, ink));
    p.circle_filled(c + egui::vec2(0.0, h * 0.34), w * 0.62, ink);
    Rect::from_center_size(c, egui::vec2(size + 2.0, h + 2.0))
}

/// The experimental mark beside a name: the warning triangle with its right
/// edge on `right`; returns its rect.
pub fn exp_badge(p: &egui::Painter, right: egui::Pos2) -> Rect {
    let size = 15.0;
    warn_triangle(p, egui::pos2(right.x - size / 2.0, right.y), size, None)
}

/// Rounded card container.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    if modern() {
        return modern::card(ui, add);
    }
    if clean() {
        return clean::card(ui, add);
    }
    let shown = egui::Frame::none()
        .fill(CARD())
        .rounding(10.0)
        .stroke(Stroke::new(1.0f32, LINE()))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    note_card(ui, shown.response.rect);
    let r = shown.inner;
    ui.add_space(10.0);
    r
}

/// A small drawn "i" in a circle; hover it for the longer explanation.
/// (Drawn rather than a font glyph, so it never shows as a missing-character box.)
pub fn info(ui: &mut Ui, text: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
    let color = if resp.hovered() { TEXT() } else { MUTED() };
    let p = ui.painter();
    let c = rect.center();
    p.circle_stroke(c, 7.0, Stroke::new(1.3f32, color));
    p.text(
        c + egui::vec2(0.0, 0.5),
        Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(10.0),
        color,
    );
    resp.on_hover_text(text)
}

/// iOS-style on/off switch.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    if modern() {
        return modern::toggle(ui, on);
    }
    if clean() {
        return clean::toggle(ui, on);
    }
    let size = egui::vec2(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let bg = if *on {
        theme::GREEN.gamma_multiply(0.85)
    } else {
        LINE()
    };
    let p = ui.painter();
    p.rect_filled(rect, rect.height() / 2.0, bg);
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    p.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp
}

/// Label (+ optional info icon) on the left, switch on the right. Returns true if changed.
pub fn toggle_row(ui: &mut Ui, label: &str, help: &str, on: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(fs(13.0)).color(TEXT()));
        if !help.is_empty() {
            info(ui, help);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed = toggle(ui, on).changed();
        });
    });
    changed
}

/// Joined buttons where exactly one option is selected.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    if modern() {
        return modern::segmented(ui, value, options);
    }
    if clean() {
        return clean::segmented(ui, value, options);
    }
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (v, label) in options {
            let selected = *value == *v;
            let text = RichText::new(*label).size(12.5).color(if selected {
                theme::button_ink()
            } else {
                MUTED()
            });
            let btn = egui::Button::new(text)
                .fill(CARD_HOVER())
                .stroke(Stroke::NONE)
                .rounding(6.0)
                .min_size(egui::vec2(0.0, 30.0));
            let resp = if selected { accent_button(ui, btn, 6.0) } else { ui.add(btn) };
            if resp.clicked() && !selected {
                *value = *v;
                changed = true;
            }
        }
    });
    changed
}

#[derive(Clone, Copy)]
pub enum PillKind {
    Ok,
    Warn,
    Bad,
    Info,
}

/// Rounded status pill, e.g. "✔ Reading game log".
pub fn pill(ui: &mut Ui, text: &str, kind: PillKind) -> Response {
    if modern() {
        return modern::pill(ui, text, kind);
    }
    let (bg, fg) = match kind {
        PillKind::Ok => (
            Color32::from_rgb(22, 64, 38),
            Color32::from_rgb(140, 232, 160),
        ),
        PillKind::Warn => (
            Color32::from_rgb(72, 56, 14),
            Color32::from_rgb(250, 214, 110),
        ),
        PillKind::Bad => (
            Color32::from_rgb(78, 24, 26),
            Color32::from_rgb(255, 150, 140),
        ),
        PillKind::Info => (CARD_HOVER(), MUTED()),
    };
    egui::Frame::none()
        .fill(bg)
        .rounding(11.0)
        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
        .show(ui, |ui| ui.label(RichText::new(text).size(fs(11.5)).color(fg)))
        .response
}

/// Small odds badge.
pub fn badge(ui: &mut Ui, text: &str, gold: bool) {
    if modern() {
        let c = if gold { C_GOLD } else { MUTED() };
        egui::Frame::none()
            .fill(tint(c, 0.16))
            .rounding(8.0)
            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            .show(ui, |ui| ui.label(RichText::new(text).size(fs(11.5)).color(if gold { C_GOLD } else { MUTED() })));
        return;
    }
    let (bg, fg) = if gold {
        (Color32::from_rgb(72, 56, 14), GOLD)
    } else {
        (CARD_HOVER(), MUTED())
    };
    egui::Frame::none()
        .fill(bg)
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| ui.label(RichText::new(text).size(fs(11.0)).color(fg)));
}

/// Clickable choice tile: icon, title, subtitle, badge. Highlighted when selected.
pub fn choice_tile(
    ui: &mut Ui,
    size: Vec2,
    selected: bool,
    icon: &str,
    title: &str,
    subtitle: &str,
    badge_text: &str,
) -> Response {
    if modern() {
        return modern::choice_tile(ui, size, selected, icon, title, subtitle, badge_text);
    }
    if clean() {
        return clean::choice_tile(ui, size, selected, icon, title, subtitle, badge_text);
    }
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter_at(rect.expand(2.0));
    let fill = if selected {
        SELECTED_BG()
    } else if resp.hovered() {
        CARD_HOVER()
    } else {
        CARD()
    };
    p.rect_filled(rect, 10.0, fill);
    p.rect_stroke(
        rect,
        10.0,
        if selected {
            Stroke::new(2.0f32, RED())
        } else {
            Stroke::new(1.0f32, LINE())
        },
    );
    let x = rect.left() + 12.0;
    p.text(
        egui::pos2(x, rect.top() + 18.0),
        Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(16.5),
        if selected {
            if theme::pal().light {
                RED()
            } else {
                GOLD
            }
        } else {
            MUTED()
        },
    );
    p.text(
        egui::pos2(x, rect.top() + 42.0),
        Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        TEXT(),
    );
    p.text(
        egui::pos2(x, rect.top() + 60.0),
        Align2::LEFT_CENTER,
        subtitle,
        egui::FontId::proportional(11.0),
        MUTED(),
    );
    if !badge_text.is_empty() {
        let font = egui::FontId::proportional(11.0);
        let galley = p.layout_no_wrap(
            badge_text.to_string(),
            font,
            if selected {
            if theme::pal().light {
                RED()
            } else {
                GOLD
            }
        } else {
            MUTED()
        },
        );
        let b = Rect::from_min_size(
            egui::pos2(x - 2.0, rect.bottom() - 26.0),
            galley.size() + egui::vec2(12.0, 4.0),
        );
        p.rect_filled(
            b,
            6.0,
            if selected {
                Color32::from_rgb(72, 56, 14)
            } else {
                CARD_HOVER()
            },
        );
        p.galley(b.min + egui::vec2(6.0, 2.0), galley, Color32::WHITE);
    }
    if selected {
        p.text(
            rect.right_top() + egui::vec2(-14.0, 14.0),
            Align2::CENTER_CENTER,
            "✔",
            egui::FontId::proportional(13.0),
            RED(),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Metric tile: small label above a big value.
pub fn stat(ui: &mut Ui, width: f32, label: &str, value: &str) {
    stat_inner(ui, width, label, value);
}

/// A stat tile you can click (Home's tiles open the page behind the number).
pub fn stat_button(ui: &mut Ui, width: f32, label: &str, value: &str, tip: &str) -> bool {
    let rect = stat_inner(ui, width, label, value).rect;
    let resp = ui.interact(rect, ui.id().with(("stat-button", label)), Sense::click());
    if resp.hovered() {
        ui.painter().rect_stroke(rect, r(12.0), Stroke::new(1.0f32, if modern() { CONTROL() } else { MUTED() }));
    }
    // A small arrow says the tile goes somewhere.
    ui.painter().text(
        rect.right_top() + egui::vec2(-10.0, 9.0),
        Align2::RIGHT_TOP,
        "›",
        egui::FontId::proportional(14.0),
        if resp.hovered() { TEXT() } else { MUTED() },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked()
}

fn stat_inner(ui: &mut Ui, width: f32, label: &str, value: &str) -> Response {
    if modern() {
        return modern::stat(ui, width, label, value);
    }
    if clean() {
        return clean::stat(ui, width, label, value);
    }
    // Stacked label/value at a fixed width; long values are cut with "…"
    // instead of pushing the page wider than the window.
    let w = (width - 22.0).max(10.0);
    egui::Frame::none()
        .fill(CARD_HOVER())
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(11.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(w);
            ui.vertical(|ui| {
                ui.set_max_width(w);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add(
                    egui::Label::new(RichText::new(label).size(11.0).color(MUTED()))
                        .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(label);
                ui.add(
                    egui::Label::new(RichText::new(value).size(16.0).family(strong_family(16.0)).strong().color(TEXT()))
                        .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(value);
            });
        })
        .response
}

/// "−  value  +" stepper. Returns true if changed.
pub fn stepper(ui: &mut Ui, value: &mut u32, min: u32, max: u32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let small = |t: &str| {
            egui::Button::new(RichText::new(t).size(14.0))
                .fill(btn_fill())
                .rounding(btn_round(6.0))
                .min_size(if clean() { egui::vec2(CONTROL_H, CONTROL_H) } else { egui::vec2(28.0, 26.0) })
        };
        // Minus always sits on the left, plus on the right, even inside a
        // right-aligned row (which lays widgets out in reverse).
        let rtl = ui.layout().prefer_right_to_left();
        let (first, second) = if rtl { ("+", "−") } else { ("−", "+") };
        let mut press = |ui: &mut Ui, sign: &str, value: &mut u32| {
            let can = if sign == "+" { *value < max } else { *value > min };
            if ui.add_enabled(can, small(sign)).clicked() {
                if sign == "+" {
                    *value += 1;
                } else {
                    *value -= 1;
                }
                changed = true;
            }
        };
        press(ui, first, &mut *value);
        ui.add_sized(
            [34.0, 26.0],
            egui::Label::new(
                RichText::new(value.to_string())
                    .size(14.0)
                    .family(strong_family(14.0)).strong()
                    .color(TEXT()),
            ),
        );
        press(ui, second, &mut *value);
    });
    changed
}

/// Keyboard key cap, e.g. [Ctrl].
pub fn keycap(ui: &mut Ui, text: &str) {
    egui::Frame::none()
        .fill(CARD_HOVER())
        .rounding(5.0)
        .stroke(Stroke::new(1.0f32, LINE()))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).monospace().color(TEXT()))
        });
}

/// Sidebar navigation item. Returns true when clicked.
pub fn nav_item(ui: &mut Ui, icon: &str, label: &str, selected: bool) -> bool {
    if modern() {
        return modern::nav_item(ui, icon, label, selected, theme::BUTTON(), false, true).clicked();
    }
    if clean() {
        return clean::nav_item(ui, icon, label, selected);
    }
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 8.0, SELECTED_BG());
        p.rect_filled(
            Rect::from_min_size(rect.min + egui::vec2(0.0, 7.0), egui::vec2(3.0, 20.0)),
            2.0,
            RED(),
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 8.0, CARD());
    }
    let color = if selected { TEXT() } else { MUTED() };
    p.text(
        rect.left_center() + egui::vec2(14.0, 0.0),
        Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(14.0),
        if selected {
            if theme::pal().light {
                RED()
            } else {
                GOLD
            }
        } else {
            MUTED()
        },
    );
    p.text(
        rect.left_center() + egui::vec2(40.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Thin progress bar for the welcome steps.
pub fn progress(ui: &mut Ui, frac: f32) {
    if modern() {
        let w = ui.available_width();
        return bar(ui, w, 6.0, frac);
    }
    if clean() {
        return clean::progress(ui, frac);
    }
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, LINE());
    p.rect_filled(
        Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * frac.clamp(0.0, 1.0), rect.height()),
        ),
        3.0,
        RED(),
    );
    let _ = OUTLINE;
}

/// A full-width "show more / show fewer" bar with a chevron, so it reads
/// clearly as something that opens and closes.
pub fn expander(ui: &mut Ui, label: &str, detail: &str, open: bool) -> Response {
    if modern() {
        return modern::expander(ui, label, detail, open);
    }
    if clean() {
        return clean::expander(ui, label, detail, open);
    }
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), Sense::click());
    let p = ui.painter();
    let hovered = resp.hovered();
    p.rect_filled(rect, 8.0, if hovered { CARD_HOVER() } else { CARD() });
    p.rect_stroke(rect, 8.0, Stroke::new(1.0f32, if hovered { RED() } else { LINE() }));
    let mut x = rect.min.x + 14.0;
    let g = p.text(
        egui::pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        TEXT(),
    );
    x = g.max.x + 10.0;
    if !detail.is_empty() {
        let avail = rect.max.x - 50.0 - x;
        if avail > 40.0 {
            let galley = ui.painter().layout(
                detail.to_string(),
                egui::FontId::proportional(12.0),
                MUTED(),
                f32::INFINITY,
            );
            let clip = Rect::from_min_max(egui::pos2(x, rect.min.y), egui::pos2(x + avail, rect.max.y));
            ui.painter()
                .with_clip_rect(clip)
                .galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, MUTED());
        }
    }
    // chevron in a small circle on the right
    let c = egui::pos2(rect.max.x - 22.0, rect.center().y);
    p.circle_filled(c, 11.0, if hovered { RED() } else { CARD_HOVER() });
    let col = if hovered { Color32::WHITE } else { TEXT() };
    let s = Stroke::new(2.0f32, col);
    let (dy, tip) = if open { (2.5, -2.5) } else { (-2.5, 2.5) };
    p.line_segment([c + egui::vec2(-5.0, dy), c + egui::vec2(0.0, tip)], s);
    p.line_segment([c + egui::vec2(0.0, tip), c + egui::vec2(5.0, dy)], s);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Add a button filled with the button colour (or gradient). Pass the
/// button's own rounding.
pub fn accent_button(ui: &mut Ui, btn: egui::Button<'_>, rounding: f32) -> Response {
    accent_button_w(ui, btn, rounding, 0.0)
}

/// `accent_button` at least `min_w` wide.
pub fn accent_button_w(ui: &mut Ui, btn: egui::Button<'_>, rounding: f32, min_w: f32) -> Response {
    if modern() {
        return modern::accent_button(ui, btn, min_w);
    }
    if clean() {
        return clean::accent_button(ui, btn, min_w);
    }
    let slot = ui.painter().add(egui::Shape::Noop);
    let btn = if min_w > 0.0 { btn.min_size(egui::vec2(min_w, 0.0)) } else { btn };
    let resp = ui.add(btn.fill(Color32::TRANSPARENT).rounding(rounding));
    let shape = theme::button_shape(resp.rect, rounding, resp.hovered());
    ui.painter().set(slot, shape);
    resp
}

// ─────────────────── clean-style versions of the widgets above ───────────────────

mod clean {
    use super::*;

    pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
        ui.label(RichText::new(title_case(title)).font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
        if !subtitle.is_empty() {
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(13.5).color(MUTED()));
        }
        ui.add_space(20.0);
    }

    pub fn section(ui: &mut Ui, text: &str) {
        ui.add_space(10.0);
        ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
        ui.add_space(4.0);
    }

    pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let shown = egui::Frame::none()
            .fill(CARD())
            .rounding(8.0)
            .stroke(Stroke::new(1.0f32, LINE()))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add(ui)
            });
        super::note_card(ui, shown.response.rect);
        let r = shown.inner;
        ui.add_space(12.0);
        r
    }

    /// Switch in the button colour when on, grey when off.
    pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
        let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(38.0, 22.0), Sense::click());
        if resp.clicked() {
            *on = !*on;
            resp.mark_changed();
        }
        let t = ui.ctx().animate_bool(resp.id, *on);
        let off = if resp.hovered() { theme::blend(CONTROL(), TEXT(), 0.2) } else { CONTROL() };
        let bg = theme::blend(off, theme::BUTTON(), t);
        let p = ui.painter();
        p.rect_filled(rect, rect.height() / 2.0, bg);
        let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
        p.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Outlined group; the chosen option sits on a grey highlight.
    pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
        let mut changed = false;
        egui::Frame::none()
            .stroke(Stroke::new(1.0f32, CONTROL()))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(3.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.spacing_mut().button_padding = egui::vec2(12.0, 2.0);
                    for (v, label) in options {
                        let selected = *value == *v;
                        let slot = ui.painter().add(egui::Shape::Noop);
                        let text = RichText::new(label.trim()).size(13.0).color(if selected { TEXT() } else { MUTED() });
                        let resp = ui.add(
                            egui::Button::new(text)
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .min_size(egui::vec2(0.0, CONTROL_H - 6.0)),
                        );
                        let fill = if selected {
                            SELECTED_BG()
                        } else if resp.hovered() {
                            CARD_HOVER()
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().set(slot, egui::Shape::rect_filled(resp.rect, 5.0, fill));
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
                            *value = *v;
                            changed = true;
                        }
                    }
                });
            });
        changed
    }

    /// Plain card; the selected one gets a button-colour border and a tick.
    pub fn choice_tile(
        ui: &mut Ui,
        size: Vec2,
        selected: bool,
        icon: &str,
        title: &str,
        subtitle: &str,
        badge_text: &str,
    ) -> Response {
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let p = ui.painter_at(rect.expand(2.0));
        let accent = theme::BUTTON();
        p.rect_filled(rect, 8.0, if resp.hovered() && !selected { CARD_HOVER() } else { CARD() });
        p.rect_stroke(
            rect.shrink(if selected { 1.0 } else { 0.5 }),
            8.0,
            if selected {
                Stroke::new(2.0f32, accent)
            } else if resp.hovered() {
                Stroke::new(1.0f32, theme::blend(CONTROL(), TEXT(), 0.3))
            } else {
                Stroke::new(1.0f32, LINE())
            },
        );
        let x = rect.left() + 14.0;
        p.text(
            egui::pos2(x, rect.top() + 20.0),
            Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(17.0),
            if selected { TEXT() } else { MUTED() },
        );
        p.text(egui::pos2(x, rect.top() + 44.0), Align2::LEFT_CENTER, title, crate::fonts::title_font(14.0), TEXT());
        let sub = p.layout(subtitle.to_string(), crate::fonts::sans_font(12.0), MUTED(), rect.width() - 28.0);
        p.galley(egui::pos2(x, rect.top() + 55.0), sub, MUTED());
        if !badge_text.is_empty() {
            let galley = p.layout_no_wrap(badge_text.to_string(), crate::fonts::sans_font(12.0), MUTED());
            let b = Rect::from_min_size(
                egui::pos2(x - 2.0, rect.bottom() - 28.0),
                galley.size() + egui::vec2(14.0, 4.0),
            );
            p.rect_filled(b, b.height() / 2.0, CARD_HOVER());
            p.galley(b.min + egui::vec2(7.0, 2.0), galley, MUTED());
        }
        if selected {
            let c = rect.right_top() + egui::vec2(-16.0, 16.0);
            p.circle_filled(c, 8.0, accent);
            let s = Stroke::new(1.8f32, theme::button_ink());
            p.line_segment([c + egui::vec2(-3.5, 0.3), c + egui::vec2(-1.0, 2.8)], s);
            p.line_segment([c + egui::vec2(-1.0, 2.8), c + egui::vec2(3.8, -2.5)], s);
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    pub fn stat(ui: &mut Ui, width: f32, label: &str, value: &str) -> Response {
        let w = (width - 26.0).max(10.0);
        egui::Frame::none()
            .fill(CARD_HOVER())
            .rounding(8.0)
            .inner_margin(egui::Margin::symmetric(13.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(w);
                ui.vertical(|ui| {
                    ui.set_max_width(w);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.add(
                        egui::Label::new(RichText::new(label).size(12.0).color(MUTED()))
                            .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(label);
                    ui.add(
                        egui::Label::new(RichText::new(value).font(crate::fonts::sans_bold_font(18.0)).color(TEXT()))
                            .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(value);
                });
            })
            .response
    }

    /// Sidebar row: grey highlight when selected, no accent stripe.
    pub fn nav_item(ui: &mut Ui, icon: &str, label: &str, selected: bool) -> bool {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
        let p = ui.painter();
        let hovered = resp.hovered();
        if selected {
            p.rect_filled(rect, 6.0, SELECTED_BG());
        } else if hovered {
            p.rect_filled(rect, 6.0, theme::blend(SIDEBAR(), SELECTED_BG(), 0.6));
        }
        let color = if selected || hovered { TEXT() } else { MUTED() };
        p.text(
            rect.left_center() + egui::vec2(12.0, 0.0),
            Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            color,
        );
        p.text(
            rect.left_center() + egui::vec2(38.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            if selected { crate::fonts::sans_bold_font(13.5) } else { crate::fonts::sans_font(13.5) },
            color,
        );
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    pub fn progress(ui: &mut Ui, frac: f32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, LINE());
        p.rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac.clamp(0.0, 1.0), rect.height())),
            2.0,
            theme::BUTTON(),
        );
    }

    /// Flat "show more" row with a plain chevron.
    pub fn expander(ui: &mut Ui, label: &str, detail: &str, open: bool) -> Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), Sense::click());
        let p = ui.painter();
        let hovered = resp.hovered();
        p.rect_filled(rect, 8.0, if hovered { CARD_HOVER() } else { CARD() });
        p.rect_stroke(rect, 8.0, Stroke::new(1.0f32, if hovered { CONTROL() } else { LINE() }));
        let g = p.text(
            egui::pos2(rect.min.x + 16.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            crate::fonts::sans_bold_font(13.5),
            TEXT(),
        );
        let x = g.max.x + 10.0;
        if !detail.is_empty() {
            let avail = rect.max.x - 44.0 - x;
            if avail > 40.0 {
                let galley = ui.painter().layout(detail.to_string(), crate::fonts::sans_font(12.5), MUTED(), f32::INFINITY);
                let clip = Rect::from_min_max(egui::pos2(x, rect.min.y), egui::pos2(x + avail, rect.max.y));
                ui.painter()
                    .with_clip_rect(clip)
                    .galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, MUTED());
            }
        }
        let c = egui::pos2(rect.max.x - 22.0, rect.center().y);
        let s = Stroke::new(1.8f32, if hovered { TEXT() } else { MUTED() });
        let (dy, tip) = if open { (2.5, -2.5) } else { (-2.5, 2.5) };
        p.line_segment([c + egui::vec2(-5.0, dy), c + egui::vec2(0.0, tip)], s);
        p.line_segment([c + egui::vec2(0.0, tip), c + egui::vec2(5.0, dy)], s);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Primary button: a flat pill in the button colour, the same size and
    /// shape as the outlined buttons. (The gradient stays on the header bar.)
    pub fn accent_button(ui: &mut Ui, btn: egui::Button<'_>, min_w: f32) -> Response {
        let slot = ui.painter().add(egui::Shape::Noop);
        let resp = ui.add(
            btn.fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(egui::vec2(min_w, CONTROL_H)),
        );
        let base = theme::BUTTON();
        let fill = if resp.is_pointer_button_down_on() {
            theme::blend(base, Color32::BLACK, 0.12)
        } else if resp.hovered() {
            theme::blend(base, Color32::WHITE, 0.12)
        } else {
            base
        };
        ui.painter().set(slot, egui::Shape::rect_filled(resp.rect, resp.rect.height() / 2.0, fill));
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

// ─────────────────── modern-style versions of the widgets above ───────────────────

pub(crate) mod modern {
    use super::*;

    pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
        ui.horizontal(|ui| {
            if let Some((icon, color)) = page_icon() {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
                icon_chip(ui.painter(), rect, icon, color, true);
                ui.add_space(6.0);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(title_case(title)).font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
                if !subtitle.is_empty() {
                    ui.label(RichText::new(subtitle).size(13.5).color(MUTED()));
                }
            });
        });
        ui.add_space(22.0);
    }

    pub fn section(ui: &mut Ui, text: &str) {
        ui.add_space(12.0);
        ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
        ui.add_space(6.0);
    }

    pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let shown = egui::Frame::none()
            .fill(CARD())
            .rounding(14.0)
            .stroke(Stroke::new(1.0f32, LINE()))
            .shadow(card_shadow())
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add(ui)
            });
        super::note_card(ui, shown.response.rect);
        let r = shown.inner;
        ui.add_space(14.0);
        r
    }

    /// Cirrus tabs: a Cloud pill track with a hairline; the open tab is a
    /// pill in the button colour, lifted by the tier-2 shadow, that slides
    /// to the tab you pick; the others wash softly on hover.
    pub fn tab_bar<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, tabs: &[(T, &str)]) -> bool {
        let mut changed = false;
        let h = 44.0;
        let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
        let p = ui.painter().clone();
        p.add(shadow_tier(1).as_shape(bar, h / 2.0));
        p.rect_filled(bar, h / 2.0, CARD());
        p.rect_stroke(bar, h / 2.0, Stroke::new(1.0f32, LINE()));
        let n = tabs.len().max(1) as f32;
        let (pad, gap) = (5.0, 4.0);
        let cw = (bar.width() - 2.0 * pad - gap * (n - 1.0)) / n;
        let rect_of = |i: usize| Rect::from_min_size(egui::pos2(bar.min.x + pad + (cw + gap) * i as f32, bar.min.y + pad), egui::vec2(cw, h - 2.0 * pad));
        let bar_id = super::unique_this_frame(ui, ui.id().with(("tab-bar", tabs.iter().map(|t| t.1).collect::<Vec<_>>().join("|"))));
        let sel = tabs.iter().position(|(v, _)| *v == *value);
        let hover_slot = p.add(egui::Shape::Noop);
        let pill = sel.map(|i| glide_rect(ui.ctx(), bar_id.with("pill"), rect_of(i), bar.min, BASE));
        if let Some(pr) = pill {
            p.add(shadow_tier(2).as_shape(pr, pr.height() / 2.0));
            p.add(primary_shape(pr, false));
        }
        let mut hovers = vec![];
        for (i, (v, label)) in tabs.iter().enumerate() {
            let r = rect_of(i);
            let resp = ui.interact(r, bar_id.with(i), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let is_sel = *value == *v;
            let ht = anim(ui.ctx(), bar_id.with(("hover", i)), resp.hovered() && !is_sel, QUICK);
            if ht > 0.0 {
                hovers.push(egui::Shape::rect_filled(r, r.height() / 2.0, sunken().gamma_multiply(ht)));
            }
            // The label turns to the pill's ink as the pill slides under it.
            let cover = pill.map_or(0.0, |pr| (pr.intersect(r).width().max(0.0) / r.width()).clamp(0.0, 1.0));
            let base = theme::blend(MUTED(), TEXT(), ht);
            let ink = theme::blend(base, on_primary(), cover);
            let font = crate::fonts::sans_bold_font(13.5);
            let label = clip_label(&p, label, &font, r.width() - 16.0);
            p.text(r.center(), Align2::CENTER_CENTER, label, font, ink);
            if resp.clicked() && !is_sel {
                *value = *v;
                changed = true;
            }
        }
        p.set(hover_slot, egui::Shape::Vec(hovers));
        changed
    }

    /// Shorten a label with "…" to fit `w`.
    fn clip_label(p: &egui::Painter, s: &str, font: &egui::FontId, w: f32) -> String {
        let fits = |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x <= w;
        if fits(s) {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while chars.len() > 1 {
            chars.pop();
            let t = chars.iter().collect::<String>().trim_end().to_string() + "…";
            if fits(&t) {
                return t;
            }
        }
        "…".into()
    }

    /// Cirrus segmented control: a Sunken pill track; the chosen option is
    /// a pill in the button colour that slides to the one you pick.
    pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
        let mut changed = false;
        // Keyed by every label, not just the first: two switches in one row
        // that both start with "All" used to share (and fight over) one pill.
        let seg_id = super::unique_this_frame(ui, ui.id().with(("segmented", options.iter().map(|o| o.1).collect::<Vec<_>>().join("|"))));
        egui::Frame::none()
            .fill(sunken())
            .stroke(Stroke::new(1.0f32, LINE()))
            .rounding(CONTROL_H)
            .inner_margin(egui::Margin::same(3.0))
            .show(ui, |ui| {
                let anchor = ui.cursor().min;
                let pill_slot = ui.painter().add(egui::Shape::Noop);
                let mut sel_rect = None;
                let mut rects = vec![];
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.spacing_mut().button_padding = egui::vec2(14.0, 2.0);
                    for (k, (v, label)) in options.iter().enumerate() {
                        let selected = *value == *v;
                        let slot = ui.painter().add(egui::Shape::Noop);
                        let text = RichText::new(label.trim()).font(crate::fonts::sans_bold_font(13.0)).color(Color32::TRANSPARENT);
                        let resp = ui.add(
                            egui::Button::new(text)
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .rounding(CONTROL_H)
                                .min_size(egui::vec2(0.0, CONTROL_H - 4.0)),
                        );
                        let ht = anim(ui.ctx(), seg_id.with(("hover", k)), resp.hovered() && !selected, QUICK);
                        if ht > 0.0 {
                            ui.painter().set(slot, egui::Shape::rect_filled(resp.rect, resp.rect.height() / 2.0, theme::blend(sunken(), TEXT(), 0.06).gamma_multiply(ht)));
                        }
                        if selected {
                            sel_rect = Some(resp.rect);
                        }
                        rects.push((resp.rect, label.trim().to_string(), ht));
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
                            *value = *v;
                            changed = true;
                        }
                    }
                });
                let pill = sel_rect.map(|r| glide_rect(ui.ctx(), seg_id.with("pill"), r, anchor, BASE));
                if let Some(pr) = pill {
                    let rad = pr.height() / 2.0;
                    ui.painter().set(pill_slot, egui::Shape::Vec(vec![shadow_tier(2).as_shape(pr, rad).into(), primary_shape(pr, false)]));
                }
                // labels on top, turning to the pill's ink as it slides under
                for (r, label, ht) in rects {
                    let cover = pill.map_or(0.0, |pr| (pr.intersect(r).width().max(0.0) / r.width()).clamp(0.0, 1.0));
                    let ink = theme::blend(theme::blend(MUTED(), TEXT(), ht), on_primary(), cover);
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(13.0), ink);
                }
            });
        changed
    }

    /// Cirrus search: a Cloud pill with a hairline, a Mist magnifier, and
    /// the Signal ring while typing.
    pub fn search_box(ui: &mut Ui, key: Option<egui::Id>, text: &mut String, hint: &str, width: f32) -> Response {
        let h = CONTROL_H + 4.0;
        let left = ui.cursor().min.x;
        let resp = ui
            .scope(|ui| {
                let v = &mut ui.visuals_mut().widgets;
                for x in [&mut v.inactive, &mut v.hovered, &mut v.active] {
                    x.rounding = egui::Rounding::same(h / 2.0);
                }
                ui.visuals_mut().selection.stroke = Stroke::NONE;
                let te = egui::TextEdit::singleline(text)
                    .hint_text(RichText::new(hint).color(theme::blend(MUTED(), CARD(), 0.25)))
                    .desired_width(width)
                    .min_size(egui::vec2(0.0, h))
                    .vertical_align(egui::Align::Center)
                    .margin(egui::Margin { left: 38.0, right: 14.0, top: 6.0, bottom: 6.0 });
                ui.add(match key {
                    Some(id) => te.id(id),
                    None => te,
                })
            })
            .inner;
        let frame = Rect::from_min_max(egui::pos2(left, resp.rect.min.y), egui::pos2(resp.rect.max.x + 14.0, resp.rect.max.y));
        let frame = Rect::from_min_max(egui::pos2(left, frame.center().y - h / 2.0), egui::pos2(frame.max.x, frame.center().y + h / 2.0));
        let p = ui.painter();
        if resp.has_focus() {
            focus_ring(p, frame, h / 2.0);
        }
        let c = egui::pos2(left + 19.0, frame.center().y - 1.0);
        let col = if resp.has_focus() { TEXT() } else { MUTED() };
        p.circle_stroke(c, 5.0, Stroke::new(1.6f32, col));
        p.line_segment([c + egui::vec2(3.6, 3.6), c + egui::vec2(7.0, 7.0)], Stroke::new(1.8f32, col));
        resp
    }

    /// Cirrus checkbox: an 18px squircle; ticked fills with the button colour.
    pub fn checkbox(ui: &mut Ui, on: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let (rect, bresp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
            let lresp = ui.add(egui::Label::new(text).sense(Sense::click()));
            let mut resp = bresp | lresp;
            if resp.clicked() {
                *on = !*on;
                resp.mark_changed();
            }
            let t = anim(ui.ctx(), resp.id, *on, QUICK);
            let hovered = resp.hovered();
            let p = ui.painter();
            let b = rect;
            let rad = 6.0;
            p.rect_filled(b, rad, CARD());
            p.rect_stroke(b.shrink(0.75), rad, Stroke::new(1.5f32, if hovered { CONTROL() } else { theme::blend(LINE(), CONTROL(), 0.5) }));
            if t > 0.0 {
                p.rect_filled(b, rad, primary().gamma_multiply(t));
                let s = Stroke::new(2.0f32, on_primary().gamma_multiply(t));
                let c = b.center();
                p.line_segment([c + egui::vec2(-4.0, 0.3), c + egui::vec2(-1.2, 3.2)], s);
                p.line_segment([c + egui::vec2(-1.2, 3.2), c + egui::vec2(4.2, -2.8)], s);
            }
            resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        })
        .inner
    }

    /// Cirrus switch: a pill track (Sunken with a hairline when off, the
    /// button colour when on) and a Cloud knob with a soft shadow.
    pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
        let size = egui::vec2(44.0, 24.0);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
        if resp.clicked() {
            *on = !*on;
            resp.mark_changed();
        }
        let t = anim(ui.ctx(), resp.id, *on, BASE);
        let hovered = resp.hovered();
        let p = ui.painter();
        let rad = rect.height() / 2.0;
        p.rect_filled(rect, rad, sunken());
        p.rect_stroke(rect.shrink(0.5), rad, Stroke::new(1.0f32, if hovered { CONTROL() } else { LINE() }));
        if t > 0.0 {
            p.rect_filled(rect, rad, (if hovered { primary_hover() } else { primary() }).gamma_multiply(t));
        }
        let x = egui::lerp((rect.left() + rad)..=(rect.right() - rad), t);
        let c = egui::pos2(x, rect.center().y);
        let knob = rad - 3.0;
        let knob_col = Color32::WHITE;
        p.circle_filled(c + egui::vec2(0.0, 1.0), knob + 0.5, Color32::from_black_alpha(if theme::pal().light { 30 } else { 70 }));
        p.circle_filled(c, knob, knob_col);
        if !*on {
            p.circle_stroke(c, knob, Stroke::new(1.0f32, LINE()));
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Cirrus chip: Cloud pill with a hairline, a status dot and Ink text.
    pub fn pill(ui: &mut Ui, text: &str, kind: PillKind) -> Response {
        let c = match kind {
            PillKind::Ok => Color32::from_rgb(43, 196, 138),
            PillKind::Warn => Color32::from_rgb(255, 122, 61),
            PillKind::Bad => theme::HP_RED,
            PillKind::Info => MUTED(),
        };
        egui::Frame::none()
            .fill(CARD())
            .stroke(Stroke::new(1.0f32, LINE()))
            .rounding(14.0)
            .inner_margin(egui::Margin { left: 10.0, right: 12.0, top: 4.0, bottom: 4.0 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 7.0;
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, c);
                    // Pills often start with their own ✔ / ⚠: the dot says it now.
                    let t = text.trim_start_matches(['✔', '⚠', '✖', '●', '•', ' ']);
                    ui.label(RichText::new(if t.is_empty() { text } else { t }).size(fs(12.0)).color(TEXT()));
                })
            })
            .response
    }

    pub fn choice_tile(ui: &mut Ui, size: Vec2, selected: bool, icon: &str, title: &str, subtitle: &str, badge_text: &str) -> Response {
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let p = ui.painter_at(rect.expand(14.0));
        let accent = primary();
        let hovered = resp.hovered();
        let lift = anim(ui.ctx(), resp.id.with("lift"), hovered && !selected, QUICK);
        let rect = rect.translate(egui::vec2(0.0, -2.0 * lift));
        if lift > 0.0 {
            p.add(shadow_tier(2).as_shape(rect, 20.0));
        }
        p.add(card_shadow().as_shape(rect, 20.0));
        p.rect_filled(rect, 20.0, if selected { CARD() } else if hovered { CARD_HOVER() } else { CARD() });
        p.rect_stroke(
            rect.shrink(if selected { 1.0 } else { 0.5 }),
            20.0,
            if selected {
                Stroke::new(2.0f32, accent)
            } else if hovered {
                Stroke::new(1.0f32, CONTROL())
            } else {
                Stroke::new(1.0f32, LINE())
            },
        );
        let x = rect.left() + 14.0;
        if size.y < 112.0 {
            // Short tile: chip beside the title.
            let chip = Rect::from_min_size(egui::pos2(x, rect.top() + 12.0), egui::vec2(28.0, 28.0));
            icon_chip(&p, chip, icon, if selected { theme::BUTTON() } else { MUTED() }, selected);
            let tx = chip.max.x + 10.0;
            let right = rect.right() - if selected { 34.0 } else { 12.0 };
            let title_g = p.layout_no_wrap(title.to_string(), crate::fonts::title_font(14.0), TEXT());
            p.with_clip_rect(Rect::from_min_max(rect.min, egui::pos2(right, rect.max.y))).galley(egui::pos2(tx, chip.min.y - 1.0), title_g, TEXT());
            let sub = p.layout(subtitle.to_string(), crate::fonts::sans_font(12.0), MUTED(), (right - tx).max(40.0));
            p.with_clip_rect(rect.shrink(2.0)).galley(egui::pos2(tx, chip.min.y + 18.0), sub, MUTED());
        } else {
            let chip = Rect::from_min_size(egui::pos2(x, rect.top() + 12.0), egui::vec2(28.0, 28.0));
            icon_chip(&p, chip, icon, if selected { theme::BUTTON() } else { MUTED() }, selected);
            p.text(egui::pos2(x, chip.max.y + 14.0), Align2::LEFT_CENTER, title, crate::fonts::title_font(14.0), TEXT());
            let sub = p.layout(subtitle.to_string(), crate::fonts::sans_font(12.0), MUTED(), rect.width() - 28.0);
            p.galley(egui::pos2(x, chip.max.y + 24.0), sub, MUTED());
        }
        if !badge_text.is_empty() {
            let galley = p.layout_no_wrap(badge_text.to_string(), crate::fonts::sans_font(12.0), MUTED());
            let bx = if size.y < 112.0 { x + 36.0 } else { x - 2.0 };
            let b = Rect::from_min_size(egui::pos2(bx, rect.bottom() - 26.0), galley.size() + egui::vec2(14.0, 4.0));
            p.rect_filled(b, b.height() / 2.0, sunken());
            p.galley(b.min + egui::vec2(7.0, 2.0), galley, MUTED());
        }
        if selected {
            let c = rect.right_top() + egui::vec2(-18.0, 18.0);
            p.circle_filled(c, 9.0, accent);
            let s = Stroke::new(1.8f32, on_primary());
            p.line_segment([c + egui::vec2(-3.5, 0.3), c + egui::vec2(-1.0, 2.8)], s);
            p.line_segment([c + egui::vec2(-1.0, 2.8), c + egui::vec2(3.8, -2.5)], s);
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    pub fn stat(ui: &mut Ui, width: f32, label: &str, value: &str) -> Response {
        let w = (width - 30.0).max(10.0);
        egui::Frame::none()
            .fill(CARD())
            .stroke(Stroke::new(1.0f32, LINE()))
            .rounding(12.0)
            .inner_margin(egui::Margin::symmetric(15.0, 12.0))
            .show(ui, |ui| {
                ui.set_width(w);
                ui.vertical(|ui| {
                    ui.set_max_width(w);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.add(egui::Label::new(RichText::new(label).size(12.5).color(MUTED())).wrap_mode(egui::TextWrapMode::Truncate)).on_hover_text(label);
                    ui.add(egui::Label::new(RichText::new(value).font(crate::fonts::sans_bold_font(20.0)).color(TEXT())).wrap_mode(egui::TextWrapMode::Truncate))
                        .on_hover_text(value);
                });
            })
            .response
    }

    /// Sidebar row: an icon chip in the page's colour; the open page sits on
    /// a raised highlight with its chip filled in. `compact`: icon only.
    /// The open page's pill in the sidebar.
    pub fn nav_selected_fill() -> Color32 {
        theme::blend(SIDEBAR(), TEXT(), if theme::pal().light { 0.06 } else { 0.09 })
    }

    /// A sidebar row. `draw_selected`: paint the open page's pill here
    /// (the modern sidebar paints one pill itself, sliding between rows).
    pub fn nav_item(ui: &mut Ui, icon: &str, label: &str, selected: bool, color: Color32, compact: bool, draw_selected: bool) -> Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), Sense::click());
        let p = ui.painter();
        let hovered = resp.hovered();
        // Cirrus nav: the open page on a Sunken pill, hovers a fainter wash.
        let rad = rect.height() / 2.0;
        let ht = anim(ui.ctx(), resp.id.with("hover"), hovered && !selected, QUICK);
        if selected && draw_selected {
            p.rect_filled(rect, rad, nav_selected_fill());
        } else if ht > 0.0 {
            p.rect_filled(rect, rad, theme::blend(SIDEBAR(), TEXT(), 0.04).gamma_multiply(ht));
        }
        let chip = if compact {
            Rect::from_center_size(rect.center(), egui::vec2(28.0, 28.0))
        } else {
            Rect::from_center_size(rect.left_center() + egui::vec2(22.0, 0.0), egui::vec2(28.0, 28.0))
        };
        icon_chip(p, chip, icon, color, selected);
        if !compact {
            p.text(
                chip.right_center() + egui::vec2(11.0, 0.0),
                Align2::LEFT_CENTER,
                label,
                if selected { crate::fonts::sans_bold_font(13.5) } else { crate::fonts::sans_font(13.5) },
                if selected || hovered { TEXT() } else { theme::blend(SIDEBAR(), TEXT(), 0.72) },
            );
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if compact {
            resp.on_hover_text(label)
        } else {
            resp
        }
    }

    pub fn expander(ui: &mut Ui, label: &str, detail: &str, open: bool) -> Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
        let p = ui.painter();
        let hovered = resp.hovered();
        p.rect_filled(rect, 22.0, if hovered { sunken() } else { CARD() });
        p.rect_stroke(rect, 22.0, Stroke::new(1.0f32, if hovered { CONTROL() } else { LINE() }));
        let g = p.text(egui::pos2(rect.min.x + 20.0, rect.center().y), Align2::LEFT_CENTER, label, crate::fonts::title_font(14.0), TEXT());
        let x = g.max.x + 10.0;
        if !detail.is_empty() {
            let avail = rect.max.x - 52.0 - x;
            if avail > 40.0 {
                let galley = ui.painter().layout(detail.to_string(), crate::fonts::sans_font(12.5), MUTED(), f32::INFINITY);
                let clip = Rect::from_min_max(egui::pos2(x, rect.min.y), egui::pos2(x + avail, rect.max.y));
                ui.painter().with_clip_rect(clip).galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, MUTED());
            }
        }
        let c = egui::pos2(rect.max.x - 24.0, rect.center().y);
        p.circle_filled(c, 13.0, if hovered { primary() } else { sunken() });
        p.circle_stroke(c, 13.0, Stroke::new(1.0f32, LINE()));
        let s = Stroke::new(1.8f32, if hovered { on_primary() } else { TEXT() });
        let (dy, tip) = if open { (2.5, -2.5) } else { (-2.5, 2.5) };
        p.line_segment([c + egui::vec2(-5.0, dy), c + egui::vec2(0.0, tip)], s);
        p.line_segment([c + egui::vec2(0.0, tip), c + egui::vec2(5.0, dy)], s);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Primary button: a pill in the user's button colour (or gradient),
    /// lifted by the tier-1 shadow; it sinks a pixel on press.
    pub fn accent_button(ui: &mut Ui, btn: egui::Button<'_>, min_w: f32) -> Response {
        let slot = ui.painter().add(egui::Shape::Noop);
        let resp = ui.add(btn.fill(Color32::TRANSPARENT).stroke(Stroke::NONE).rounding(CONTROL_H).min_size(egui::vec2(min_w, CONTROL_H + 2.0)));
        let down = resp.is_pointer_button_down_on();
        let press = anim(ui.ctx(), resp.id.with("press"), down, 0.08);
        let ht = anim(ui.ctx(), resp.id.with("hover"), resp.hovered() && !down, QUICK);
        let r = resp.rect.translate(egui::vec2(0.0, press));
        let mut shapes = vec![];
        if resp.enabled() {
            shapes.push(shadow_tier(1).as_shape(r, r.height() / 2.0).into());
            if ht > 0.0 {
                shapes.push(egui::epaint::Shadow { offset: egui::vec2(0.0, 6.0), blur: 16.0, spread: 0.0, color: theme::BUTTON().gamma_multiply(0.3 * ht) }.as_shape(r, r.height() / 2.0).into());
            }
        }
        let mut shape = primary_shape_t(r, ht);
        if !resp.enabled() {
            shape = egui::Shape::rect_filled(r, r.height() / 2.0, primary().gamma_multiply(0.35));
        }
        shapes.push(shape);
        if resp.has_focus() {
            ui.painter().rect_stroke(r.expand(2.5), r.height() / 2.0 + 2.5, Stroke::new(3.0f32, SIGNAL.gamma_multiply(0.3)));
        }
        ui.painter().set(slot, egui::Shape::Vec(shapes));
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// The Cirrus slider: a soft pill rail, a fill in the button colour that ends
/// exactly at the knob, and a white "cloud" knob with a hairline edge and a
/// soft shadow that grows a little under the pointer. Small whole-number
/// ranges get a dot for each stop. The value sits in a fixed-width pill on the
/// right, so rows of sliders line up. Drag or click anywhere on the rail;
/// with the pointer over it, the arrow keys nudge it one step.
/// `step` 0 means smooth. Width comes from `ui.spacing().slider_width`.
pub fn slider_f64(ui: &mut Ui, v: &mut f64, range: std::ops::RangeInclusive<f64>, step: f64, value: &str) -> Response {
    ui.horizontal(|ui| {
        let (lo, hi) = (*range.start(), *range.end());
        let span = (hi - lo).max(f64::EPSILON);
        let w = ui.spacing().slider_width.max(60.0);
        let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(w, CONTROL_H), Sense::click_and_drag());
        const R: f32 = 10.0;
        let rail = Rect::from_center_size(rect.center(), egui::vec2(rect.width() - 2.0 * (R + 2.0), 8.0));
        let snap = |x: f64| {
            let x = if step > 0.0 { lo + ((x - lo) / step).round() * step } else { x };
            x.clamp(lo, hi)
        };
        let before = *v;
        if let Some(pos) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on() {
                let t = ((pos.x - rail.min.x) / rail.width()).clamp(0.0, 1.0) as f64;
                *v = snap(lo + t * span);
            }
        }
        if resp.hovered() {
            let nudge = if step > 0.0 { step } else { span / 100.0 };
            let (l, r) = ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowUp)));
            if l {
                *v = snap(*v - nudge);
            }
            if r {
                *v = snap(*v + nudge);
            }
        }
        if *v != before {
            resp.mark_changed();
        }
        let active = resp.dragged() || resp.is_pointer_button_down_on();
        let hover = anim(ui.ctx(), resp.id.with("hover"), resp.hovered() || active, QUICK);
        let press = anim(ui.ctx(), resp.id.with("press"), active, QUICK);
        let t = (((*v).clamp(lo, hi) - lo) / span) as f32;
        let kx = rail.min.x + rail.width() * t;
        let p = ui.painter();
        // rail
        let track = theme::blend(LINE(), TEXT(), if theme::pal().light { 0.04 } else { 0.10 });
        p.rect_filled(rail, 4.0, track);
        // fill, in the button colour (or gradient), ending under the knob
        if kx > rail.min.x + 0.5 {
            let fill = Rect::from_min_max(rail.min, egui::pos2(kx, rail.max.y));
            let shape = if fill.width() >= rail.height() {
                primary_shape(fill, false)
            } else {
                egui::Shape::rect_filled(fill, 4.0, primary())
            };
            p.add(shape);
        }
        // a dot for each stop on small whole-number ranges
        if step > 0.0 {
            let stops = (span / step).round() as i32;
            if (2..=12).contains(&stops) {
                for k in 1..stops {
                    let x = rail.min.x + rail.width() * (k as f32 / stops as f32);
                    let c = if x < kx { theme::blend(primary(), on_primary(), 0.55) } else { theme::blend(track, TEXT(), 0.35) };
                    p.circle_filled(egui::pos2(x, rail.center().y), 1.6, c);
                }
            }
        }
        // knob: a white cloud with a hairline edge and a soft shadow
        let kr = R + 1.5 * hover + 0.5 * press;
        let kc = egui::pos2(kx, rail.center().y);
        let shadow = shadow_tier(2);
        let kbox = Rect::from_center_size(kc, egui::Vec2::splat(kr * 2.0));
        p.add(shadow.as_shape(kbox.translate(egui::vec2(0.0, 1.0)), kr));
        p.circle_filled(kc, kr, Color32::WHITE);
        p.circle_stroke(kc, kr, Stroke::new(1.0f32, Color32::from_black_alpha(if theme::pal().light { 28 } else { 60 })));
        // under the pointer, a dot in the button colour shows it's grabbed
        if hover > 0.01 {
            p.circle_filled(kc, 3.5 * hover + 1.0 * press, theme::blend(Color32::WHITE, primary(), hover));
        }
        let resp = resp.on_hover_cursor(if active { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        if !value.is_empty() {
            let (vr, _) = ui.allocate_exact_size(egui::vec2(SLIDER_VALUE_W, CONTROL_H), Sense::hover());
            let p = ui.painter();
            p.rect_filled(vr, CONTROL_H / 2.0, ui.visuals().extreme_bg_color);
            let edge = theme::blend(ui.visuals().widgets.inactive.bg_stroke.color, primary(), press);
            p.rect_stroke(vr, CONTROL_H / 2.0, Stroke::new(1.0 + press, edge));
            p.text(vr.center(), egui::Align2::CENTER_CENTER, value, egui::FontId::proportional(13.0), ui.visuals().text_color());
        }
        resp
    })
    .inner
}

/// `slider_f64` for whole numbers.
pub fn slider_u32(ui: &mut Ui, v: &mut u32, range: std::ops::RangeInclusive<u32>, value: &str) -> Response {
    let mut f = *v as f64;
    let r = slider_f64(ui, &mut f, *range.start() as f64..=*range.end() as f64, 1.0, value);
    *v = f.round() as u32;
    r
}

/// `slider_f64` for a smooth 0..1-style value.
pub fn slider_f32(ui: &mut Ui, v: &mut f32, range: std::ops::RangeInclusive<f32>, value: &str) -> Response {
    let mut f = *v as f64;
    let r = slider_f64(ui, &mut f, *range.start() as f64..=*range.end() as f64, 0.01, value);
    *v = f as f32;
    r
}

/// Width of a slider's value pill.
pub const SLIDER_VALUE_W: f32 = 72.0;

// ───────────────────────── Cirrus dropdown ─────────────────────────
// Cirrus has no dropdown of its own, so this one is built from its rules:
// the closed box is an input (a Cloud pill, Edge hairline, Ink text, a Mist
// chevron); hovering washes it to Sunken; open, it gets the Signal focus
// ring and the chevron turns up. The list is a popover: a Cloud surface with
// a hairline, tier-2 shadow and a 14px radius, 6px in from its edge. Rows are
// full-width pills that wash to Sunken on hover; the picked one sits on a
// slightly deeper Sunken pill in Ink. Signal blue is kept for the focus ring
// only.

/// A Cirrus dropdown, used like `egui::ComboBox`:
/// `kit::Dropdown::from_id_salt("x").selected_text(..).width(..).show_ui(ui, |ui| ..)`.
/// Inside, `selectable_value` / `selectable_label` / `button` come out as
/// Cirrus rows; the list closes when one is clicked (or with `keep_open`, only
/// when `close_dropdown` is called, for lists with a search box).
pub struct Dropdown {
    id: egui::Id,
    text: egui::WidgetText,
    width: f32,
    height: f32,
    keep_open: bool,
    menu: bool,
}

impl Dropdown {
    pub fn from_id_salt(salt: impl std::hash::Hash) -> Self {
        Dropdown { id: egui::Id::new(("cirrus-dd", egui::Id::new(salt))), text: "".into(), width: 0.0, height: 320.0, keep_open: false, menu: false }
    }
    pub fn selected_text(mut self, t: impl Into<egui::WidgetText>) -> Self {
        self.text = t.into();
        self
    }
    /// The closed box's width (0 = fit the text).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// The list's most height before it scrolls.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Stay open on clicks inside (a list with a search box).
    pub fn keep_open(mut self) -> Self {
        self.keep_open = true;
        self
    }

    pub fn show_ui<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<Option<R>> {
        let popup = self.id.with("list");
        let open = ui.memory(|m| m.is_popup_open(popup));
        // the closed box
        let font = egui::TextStyle::Button.resolve(ui.style());
        let galley = self.text.clone().into_galley(ui, Some(egui::TextWrapMode::Truncate), (self.width - 40.0).max(0.0).max(if self.width > 0.0 { 0.0 } else { f32::INFINITY }), font);
        let h = CONTROL_H.max(galley.size().y + 10.0);
        let w = if self.width > 0.0 { self.width } else { galley.size().x + 44.0 };
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let t = anim(ui.ctx(), self.id.with("hov"), resp.hovered() || open, QUICK);
            let r = h / 2.0;
            p.rect_filled(rect, r, theme::blend(CARD(), sunken(), t));
            p.rect_stroke(rect, r, Stroke::new(1.0f32, LINE()));
            if open || resp.has_focus() {
                focus_ring(p, rect, r);
            }
            let tc = if ui.is_enabled() { TEXT() } else { MUTED() };
            p.galley(egui::pos2(rect.min.x + 14.0, rect.center().y - galley.size().y / 2.0), galley, tc);
            chevron(p, egui::pos2(rect.max.x - 17.0, rect.center().y), open, if self.menu { TEXT() } else { MUTED() });
        }
        if resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup));
        }
        let mut inner = None;
        if ui.memory(|m| m.is_popup_open(popup)) {
            let list_w = rect.width().max(if self.menu { 220.0 } else { 0.0 });
            let area = egui::Area::new(popup).kind(egui::UiKind::Popup).order(egui::Order::Foreground).fixed_pos(rect.left_bottom() + egui::vec2(0.0, 6.0)).constrain(true);
            let shown = area.show(ui.ctx(), |ui| {
                egui::Frame::none()
                    .fill(CARD())
                    .stroke(Stroke::new(1.0f32, LINE()))
                    .rounding(14.0)
                    .shadow(shadow_tier(2))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.set_min_width(list_w - 12.0);
                        list_style(ui);
                        egui::ScrollArea::vertical().max_height(self.height).show(ui, |ui| {
                            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| add(ui)).inner
                        })
                        .inner
                    })
                    .inner
            });
            inner = Some(shown.inner);
            let area_rect = shown.response.rect;
            let (pressed_at, clicked_at, esc) = ui.input(|i| {
                (
                    if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None },
                    if i.pointer.primary_clicked() { i.pointer.interact_pos() } else { None },
                    i.key_pressed(egui::Key::Escape),
                )
            });
            let outside = pressed_at.is_some_and(|p| !area_rect.contains(p) && !rect.contains(p));
            // a click (not a scroll-bar drag) on a row picks it and closes the list
            let picked = !self.keep_open && clicked_at.is_some_and(|p| area_rect.contains(p));
            if outside || esc || picked {
                ui.memory_mut(|m| m.close_popup());
            }
        }
        egui::InnerResponse::new(inner, resp)
    }
}

/// A Cirrus menu: a pill button (label and chevron) that opens a popover of
/// actions. Rows close it when clicked.
pub fn menu<R>(ui: &mut Ui, label: impl Into<egui::WidgetText>, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<Option<R>> {
    let label = label.into();
    let id = ui.id().with(("cirrus-menu", label.text().to_string()));
    let mut d = Dropdown::from_id_salt(id).selected_text(label).height(420.0);
    d.menu = true;
    d.show_ui(ui, add)
}

/// Close the open dropdown (for a `keep_open` list, after a pick).
pub fn close_dropdown(ui: &Ui) {
    ui.memory_mut(|m| m.close_popup());
}

/// The Lucide chevron: down when closed, up when open.
fn chevron(p: &egui::Painter, c: egui::Pos2, up: bool, col: Color32) {
    let (w, h) = (4.5, 2.6);
    let d = if up { -1.0 } else { 1.0 };
    p.add(egui::Shape::line(vec![c + egui::vec2(-w, -h * d), c + egui::vec2(0.0, h * d), c + egui::vec2(w, -h * d)], Stroke::new(1.6f32, col)));
}

/// Rows in a dropdown list: full-width pills, Sunken on hover, the picked one
/// on a deeper Sunken pill in bold Ink.
fn list_style(ui: &mut Ui) {
    let sunk = sunken();
    let st = ui.style_mut();
    st.spacing.button_padding = egui::vec2(12.0, 7.0);
    st.spacing.item_spacing = egui::vec2(6.0, 2.0);
    st.spacing.interact_size.y = 30.0;
    let v = &mut st.visuals;
    v.selection.bg_fill = theme::blend(sunk, TEXT(), 0.08);
    v.selection.stroke = Stroke::new(1.0f32, TEXT());
    let w = &mut v.widgets;
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0f32, TEXT());
    w.hovered.weak_bg_fill = sunk;
    w.hovered.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.0f32, TEXT());
    w.active.weak_bg_fill = theme::blend(sunk, TEXT(), 0.08);
    w.active.bg_stroke = Stroke::NONE;
    w.open.weak_bg_fill = sunk;
    w.open.bg_stroke = Stroke::NONE;
    for x in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open, &mut w.noninteractive] {
        x.rounding = egui::Rounding::same(10.0);
        x.expansion = 0.0;
    }
}
