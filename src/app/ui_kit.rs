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

/// Is the clean settings style on?
pub fn clean() -> bool {
    theme::clean()
}

/// Height of every control (buttons, text boxes, segmented groups).
pub const CONTROL_H: f32 = 30.0;

/// Font size for settings text: unchanged in the classic look; in the clean
/// look nothing goes below 12 so it stays readable.
pub fn fs(size: f32) -> f32 {
    if clean() { size.max(12.0) } else { size }
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
    if clean() { radius.min(8.0) } else { radius }
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
    if clean() { CONTROL_H / 2.0 } else { 2.0 }
}

/// Fill for ordinary (secondary) buttons: grey in the classic look,
/// see-through outlined pill in the clean look.
pub fn btn_fill() -> Color32 {
    if clean() { Color32::TRANSPARENT } else { CARD_HOVER() }
}

/// Button corner radius: pills in the clean look.
pub fn btn_round(r: f32) -> f32 {
    if clean() { CONTROL_H / 2.0 } else { r }
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
                f.family = crate::fonts::sans_bold();
                f.size = 20.0;
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
}

/// Search box. In the clean look it gets a magnifier icon and the standard
/// 30px control height; `width` = f32::INFINITY fills the row.
pub fn search_box(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    if !clean() {
        return ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .desired_width(width)
                .min_size(egui::vec2(0.0, CONTROL_H))
                .vertical_align(egui::Align::Center)
                .margin(egui::Margin::symmetric(8.0, 5.0)),
        );
    }
    // Where the box's left edge will be. (The response rect is the text area
    // inside the margin, so the icon is placed from here instead.)
    let left = ui.cursor().min.x;
    let resp = ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(RichText::new(hint).color(MUTED()))
            .desired_width(width)
            .min_size(egui::vec2(0.0, CONTROL_H))
            .vertical_align(egui::Align::Center)
            .margin(egui::Margin { left: 32.0, right: 8.0, top: 5.0, bottom: 5.0 }),
    );
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
    if clean() {
        return clean::section(ui, text);
    }
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(11.5).color(MUTED()));
    ui.add_space(4.0);
}

/// Rounded card container.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    if clean() {
        return clean::card(ui, add);
    }
    let r = egui::Frame::none()
        .fill(CARD())
        .rounding(10.0)
        .stroke(Stroke::new(1.0f32, LINE()))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner;
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
        ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0f32, MUTED()));
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
                    egui::Label::new(RichText::new(value).size(16.0).strong().color(TEXT()))
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
                    .strong()
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
    if clean() {
        return clean::accent_button(ui, btn);
    }
    let slot = ui.painter().add(egui::Shape::Noop);
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
        let r = egui::Frame::none()
            .fill(CARD())
            .rounding(8.0)
            .stroke(Stroke::new(1.0f32, LINE()))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add(ui)
            })
            .inner;
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
        p.text(egui::pos2(x, rect.top() + 44.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(13.5), TEXT());
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
    pub fn accent_button(ui: &mut Ui, btn: egui::Button<'_>) -> Response {
        let slot = ui.painter().add(egui::Shape::Noop);
        let resp = ui.add(
            btn.fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(egui::vec2(0.0, CONTROL_H)),
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
