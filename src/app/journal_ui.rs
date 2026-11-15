//! The Journal's look, after Obsidian: thin line icons, flat icon buttons
//! and the panel colours.

use super::ui_kit::{self as kit, CARD, TEXT};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Sense, Shape, Stroke};

/// The side panels, the ribbon and the tab bar.
pub(super) fn side_bg() -> Color32 {
    theme::blend(kit::PAGE(), CARD(), 0.62)
}

/// Where notes and canvases are.
pub(super) fn main_bg() -> Color32 {
    kit::PAGE()
}

/// A row that's picked (the open note in the list).
pub(super) fn sel_bg() -> Color32 {
    theme::blend(side_bg(), TEXT(), 0.10)
}

pub(super) fn hover_bg() -> Color32 {
    theme::blend(side_bg(), TEXT(), 0.06)
}

/// Icon colour at rest.
pub(super) fn icon_col() -> Color32 {
    theme::blend(side_bg(), TEXT(), 0.62)
}

/// A flat icon button: a picture, a soft square behind it when hovered (or on).
pub(super) fn icon_btn(ui: &mut egui::Ui, r: Rect, id: impl std::hash::Hash, icon: &str, tip: &str, on: bool) -> egui::Response {
    icon_btn_en(ui, r, id, icon, tip, on, true)
}

pub(super) fn icon_btn_en(ui: &mut egui::Ui, r: Rect, id: impl std::hash::Hash, icon: &str, tip: &str, on: bool, enabled: bool) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(("jx-icon", id)), if enabled { Sense::click() } else { Sense::hover() });
    let hov = resp.hovered() && enabled;
    if on || hov {
        ui.painter().rect_filled(r, 6.0, if on { sel_bg() } else { hover_bg() });
    }
    let col = if !enabled {
        icon_col().gamma_multiply(0.45)
    } else if on || hov {
        TEXT()
    } else {
        icon_col()
    };
    draw(ui.painter(), icon, r.center(), col);
    let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn poly(p: &Painter, pts: &[Pos2], st: Stroke) {
    p.add(Shape::line(pts.to_vec(), st));
}

fn closed(p: &Painter, pts: &[Pos2], st: Stroke) {
    p.add(Shape::closed_line(pts.to_vec(), st));
}

/// Draw an icon (about 16 points) centred on `c`.
pub(super) fn draw(p: &Painter, icon: &str, c: Pos2, col: Color32) {
    let st = Stroke::new(1.4f32, col);
    let v = |x: f32, y: f32| c + vec2(x, y);
    match icon {
        "sidebar-left" | "sidebar-right" => {
            let r = Rect::from_center_size(c, vec2(16.0, 14.0));
            p.rect_stroke(r, 3.0, st);
            let x = if icon == "sidebar-left" { r.min.x + 5.5 } else { r.max.x - 5.5 };
            p.line_segment([pos2(x, r.min.y), pos2(x, r.max.y)], st);
        }
        "folder" | "folder-open" | "folder-plus" => {
            closed(p, &[v(-7.5, -5.0), v(-2.5, -5.0), v(-1.0, -3.0), v(7.5, -3.0), v(7.5, 6.0), v(-7.5, 6.0)], st);
            if icon == "folder-open" {
                p.line_segment([v(-7.5, 0.0), v(7.5, 0.0)], st);
            }
            if icon == "folder-plus" {
                p.line_segment([v(0.0, 0.0), v(0.0, 4.5)], st);
                p.line_segment([v(-2.25, 2.25), v(2.25, 2.25)], st);
            }
        }
        "search" => {
            p.circle_stroke(v(-1.5, -1.5), 5.0, st);
            p.line_segment([v(2.2, 2.2), v(6.5, 6.5)], st);
        }
        "bookmark" => closed(p, &[v(-5.0, -7.0), v(5.0, -7.0), v(5.0, 7.0), v(0.0, 3.5), v(-5.0, 7.0)], st),
        "hash" => {
            p.line_segment([v(-2.0, -7.0), v(-4.0, 7.0)], st);
            p.line_segment([v(4.0, -7.0), v(2.0, 7.0)], st);
            p.line_segment([v(-6.5, -2.5), v(6.5, -2.5)], st);
            p.line_segment([v(-7.0, 2.5), v(6.0, 2.5)], st);
        }
        // a square with a pen across its corner: a new note
        "note-plus" => {
            poly(p, &[v(1.0, -6.5), v(-6.5, -6.5), v(-6.5, 6.5), v(6.5, 6.5), v(6.5, -1.0)], st);
            closed(p, &[v(4.5, -7.5), v(7.5, -4.5), v(0.5, 2.5), v(-2.5, 2.5), v(-2.5, -0.5)], st);
        }
        "canvas" => {
            p.rect_stroke(Rect::from_min_max(v(-7.0, -7.0), v(-1.0, 1.0)), 1.5, st);
            p.rect_stroke(Rect::from_min_max(v(1.0, -7.0), v(7.0, -3.0)), 1.5, st);
            p.rect_stroke(Rect::from_min_max(v(1.0, -1.0), v(7.0, 7.0)), 1.5, st);
            p.rect_stroke(Rect::from_min_max(v(-7.0, 3.0), v(-1.0, 7.0)), 1.5, st);
        }
        "sort" => {
            p.line_segment([v(-4.0, -6.5), v(-4.0, 6.5)], st);
            poly(p, &[v(-7.0, -3.5), v(-4.0, -6.5), v(-1.0, -3.5)], st);
            p.line_segment([v(1.5, -4.0), v(7.0, -4.0)], st);
            p.line_segment([v(1.5, 0.0), v(5.5, 0.0)], st);
            p.line_segment([v(1.5, 4.0), v(4.0, 4.0)], st);
        }
        "chevrons" | "vault" => {
            poly(p, &[v(-4.0, -2.0), v(0.0, -6.0), v(4.0, -2.0)], st);
            poly(p, &[v(-4.0, 2.0), v(0.0, 6.0), v(4.0, 2.0)], st);
        }
        "collapse" => {
            poly(p, &[v(-4.0, -6.5), v(0.0, -2.5), v(4.0, -6.5)], st);
            poly(p, &[v(-4.0, 6.5), v(0.0, 2.5), v(4.0, 6.5)], st);
        }
        // a page with a magnifier: the quick switcher
        "switcher" => {
            poly(p, &[v(1.5, 7.0), v(-6.0, 7.0), v(-6.0, -7.0), v(2.0, -7.0), v(6.0, -3.0), v(6.0, 0.0)], st);
            p.circle_stroke(v(1.5, 2.5), 2.6, st);
            p.line_segment([v(3.4, 4.4), v(6.0, 7.0)], st);
        }
        "calendar" => {
            let r = Rect::from_center_size(v(0.0, 1.0), vec2(15.0, 13.0));
            p.rect_stroke(r, 2.0, st);
            p.line_segment([pos2(r.min.x, r.min.y + 4.0), pos2(r.max.x, r.min.y + 4.0)], st);
            p.line_segment([v(-3.5, -7.5), v(-3.5, -4.0)], st);
            p.line_segment([v(3.5, -7.5), v(3.5, -4.0)], st);
        }
        "settings" => {
            p.circle_stroke(c, 2.6, st);
            p.circle_stroke(c, 5.6, st);
            for k in 0..8 {
                let a = std::f32::consts::TAU * k as f32 / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 5.6, c + d * 7.6], Stroke::new(2.0f32, col));
            }
        }
        "help" => {
            p.circle_stroke(c, 7.0, st);
            p.text(c + vec2(0.0, 0.5), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(10.5), col);
        }
        "back" | "forward" => {
            let s = if icon == "back" { 1.0 } else { -1.0 };
            p.line_segment([v(6.0 * s, 0.0), v(-6.0 * s, 0.0)], st);
            poly(p, &[v(-1.5 * s, -4.5), v(-6.0 * s, 0.0), v(-1.5 * s, 4.5)], st);
        }
        // an open book: Reading view
        "book" => {
            poly(p, &[v(0.0, -4.5), v(-3.0, -6.0), v(-7.5, -6.0), v(-7.5, 5.0), v(-3.0, 5.0), v(0.0, 6.5)], st);
            poly(p, &[v(0.0, -4.5), v(3.0, -6.0), v(7.5, -6.0), v(7.5, 5.0), v(3.0, 5.0), v(0.0, 6.5)], st);
            p.line_segment([v(0.0, -4.5), v(0.0, 6.5)], st);
        }
        "pencil" => {
            closed(p, &[v(3.5, -7.0), v(7.0, -3.5), v(-3.5, 7.0), v(-7.0, 7.0), v(-7.0, 3.5)], st);
            p.line_segment([v(1.0, -4.5), v(4.5, -1.0)], st);
        }
        "more" => {
            for y in [-5.0, 0.0, 5.0] {
                p.circle_filled(v(0.0, y), 1.5, col);
            }
        }
        "chevron-down" => poly(p, &[v(-5.0, -2.5), v(0.0, 2.5), v(5.0, -2.5)], st),
        "plus" => {
            p.line_segment([v(-6.0, 0.0), v(6.0, 0.0)], st);
            p.line_segment([v(0.0, -6.0), v(0.0, 6.0)], st);
        }
        "minus" => {
            p.line_segment([v(-6.0, 0.0), v(6.0, 0.0)], st);
        }
        "x" => {
            p.line_segment([v(-4.5, -4.5), v(4.5, 4.5)], st);
            p.line_segment([v(-4.5, 4.5), v(4.5, -4.5)], st);
        }
        "link" => {
            p.circle_stroke(v(-3.0, 3.0), 3.5, st);
            p.circle_stroke(v(3.0, -3.0), 3.5, st);
            p.line_segment([v(-1.5, 1.5), v(1.5, -1.5)], st);
        }
        "arrow-out" => {
            p.line_segment([v(-5.0, 5.0), v(5.0, -5.0)], st);
            poly(p, &[v(-1.0, -5.0), v(5.0, -5.0), v(5.0, 1.0)], st);
        }
        "list" => {
            for (k, y) in [-5.0f32, 0.0, 5.0].into_iter().enumerate() {
                let x0 = if k == 0 { -7.0 } else { -4.0 };
                p.line_segment([v(x0, y), v(7.0, y)], st);
            }
        }
        "reset" => {
            let pts: Vec<Pos2> = (0..=14).map(|k| {
                let a = std::f32::consts::PI * (0.35 + 1.55 * k as f32 / 14.0);
                c + vec2(a.cos(), a.sin()) * 6.0
            }).collect();
            poly(p, &pts, st);
            let tip = pts[0];
            poly(p, &[tip + vec2(-3.5, -1.0), tip, tip + vec2(-0.5, -3.5)], st);
        }
        "frame" => {
            let r = Rect::from_center_size(c, vec2(14.0, 14.0));
            for (a, dx, dy) in [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.right_bottom(), -1.0, -1.0), (r.left_bottom(), 1.0, -1.0)] {
                p.line_segment([a, a + vec2(dx * 4.5, 0.0)], st);
                p.line_segment([a, a + vec2(0.0, dy * 4.5)], st);
            }
        }
        "undo" | "redo" => {
            let s = if icon == "undo" { 1.0 } else { -1.0 };
            let pts: Vec<Pos2> = (0..=10).map(|k| {
                let a = std::f32::consts::PI * (0.15 + 0.85 * k as f32 / 10.0);
                c + vec2(-s * 5.0 * a.cos(), -5.0 * a.sin() + 2.5)
            }).collect();
            poly(p, &pts, st);
            let tip = pts[0];
            poly(p, &[tip + vec2(s * 3.5, -3.5), tip, tip + vec2(s * 3.5, 3.0)], st);
        }
        // a blank page: a card
        "card" => closed(p, &[v(-6.0, -7.5), v(2.0, -7.5), v(6.0, -3.5), v(6.0, 7.5), v(-6.0, 7.5)], st),
        // a page with lines: a note from the Journal
        "file-text" => {
            closed(p, &[v(-6.0, -7.5), v(2.0, -7.5), v(6.0, -3.5), v(6.0, 7.5), v(-6.0, 7.5)], st);
            for y in [-1.0, 2.0, 5.0] {
                p.line_segment([v(-3.0, y), v(3.0, y)], Stroke::new(1.2f32, col));
            }
        }
        // a page with a hill: a picture
        "image-file" => {
            closed(p, &[v(-6.0, -7.5), v(2.0, -7.5), v(6.0, -3.5), v(6.0, 7.5), v(-6.0, 7.5)], st);
            poly(p, &[v(-4.0, 5.0), v(-1.0, 1.0), v(1.5, 3.5), v(3.0, 2.0), v(4.0, 5.0)], Stroke::new(1.2f32, col));
            p.circle_stroke(v(-2.0, -2.5), 1.4, Stroke::new(1.2f32, col));
        }
        "image" => {
            let r = Rect::from_center_size(c, vec2(16.0, 13.0));
            p.rect_stroke(r, 2.0, st);
            poly(p, &[v(-6.0, 5.0), v(-2.0, 0.0), v(1.0, 3.0), v(3.0, 1.0), v(6.0, 5.0)], st);
            p.circle_stroke(v(3.0, -2.5), 1.5, st);
        }
        "widget" => {
            for (dx, dy) in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
                p.rect_stroke(Rect::from_center_size(c + vec2(dx, dy), vec2(5.5, 5.5)), 1.0, Stroke::new(1.3f32, col));
            }
        }
        "snippet" => {
            p.circle_stroke(c, 6.5, st);
            p.circle_filled(c, 2.5, col);
        }
        "group" => {
            let r = Rect::from_center_size(c, vec2(15.0, 13.0));
            for w in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()].windows(2) {
                p.extend(Shape::dashed_line(&[w[0], w[1]], Stroke::new(1.3f32, col), 2.5, 2.0));
            }
        }
        "sparkle" => {
            let s = 6.5;
            closed(p, &[v(0.0, -s), v(1.6, -1.6), v(s, 0.0), v(1.6, 1.6), v(0.0, s), v(-1.6, 1.6), v(-s, 0.0), v(-1.6, -1.6)], st);
        }
        "trash" => {
            p.line_segment([v(-7.0, -4.5), v(7.0, -4.5)], st);
            poly(p, &[v(-2.5, -4.5), v(-2.5, -7.0), v(2.5, -7.0), v(2.5, -4.5)], st);
            poly(p, &[v(-5.5, -4.5), v(-4.5, 7.0), v(4.5, 7.0), v(5.5, -4.5)], st);
        }
        _ => {}
    }
}
