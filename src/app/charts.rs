//! Chart pieces for the stats pages: dashboard cards, activity rings,
//! bar charts with an axis, banded line charts, distribution bars, a
//! 24-hour clock, a calendar heat map, trend tiles and ranked rows.
//!
//! The look: dark cards, one bright colour per measure (as fitness apps do),
//! big numbers in that colour, hairline grids and quiet labels. Luck always
//! uses the same five colours, from blue (very lucky) to red (very unlucky).

use super::ui_kit::{self as kit, CARD, CARD_HOVER, MUTED, TEXT};
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke};

// ───────────────────────────── colours ─────────────────────────────

pub(super) const BLUE: Color32 = Color32::from_rgb(10, 132, 255);
pub(super) const GREEN: Color32 = Color32::from_rgb(48, 209, 88);
pub(super) const YELLOW: Color32 = Color32::from_rgb(255, 204, 0);
pub(super) const ORANGE: Color32 = Color32::from_rgb(255, 146, 48);
pub(super) const RED: Color32 = Color32::from_rgb(255, 69, 58);
pub(super) const PURPLE: Color32 = Color32::from_rgb(191, 90, 242);
pub(super) const PINK: Color32 = Color32::from_rgb(255, 55, 120);
pub(super) const CYAN: Color32 = Color32::from_rgb(90, 200, 250);
pub(super) const LIME: Color32 = Color32::from_rgb(170, 235, 60);
pub(super) const GOLD: Color32 = Color32::from_rgb(255, 200, 66);

/// A colour readable on the current card (darker on light themes).
pub(super) fn ink(c: Color32) -> Color32 {
    if theme::pal().light {
        theme::blend(c, Color32::BLACK, 0.25)
    } else {
        c
    }
}

pub(super) fn faint(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a)
}

fn track() -> Color32 {
    theme::blend(CARD(), TEXT(), 0.09)
}

fn grid() -> Color32 {
    theme::blend(CARD(), TEXT(), 0.12)
}

pub(super) fn bold(size: f32) -> FontId {
    crate::fonts::sans_bold_font(size)
}

pub(super) fn reg(size: f32) -> FontId {
    crate::fonts::sans_font(size)
}

/// Big numbers: the title font (Inter Tight Bold).
pub(super) fn heavy(size: f32) -> FontId {
    crate::fonts::title_font(size)
}

// ───────────────────────────── luck ─────────────────────────────

/// Luck bands by the share of hunters who'd have found the shiny within
/// that many encounters: (upper bound, name, colour).
pub(super) const LUCK_BANDS: [(f64, &str, Color32); 5] = [
    (0.20, "Very lucky", BLUE),
    (0.45, "Lucky", GREEN),
    (0.70, "Average", YELLOW),
    (0.90, "Unlucky", ORANGE),
    (1.01, "Very unlucky", RED),
];

/// The band for a share of hunters (0..1).
pub(super) fn luck_band(found: f64) -> usize {
    LUCK_BANDS.iter().position(|b| found < b.0).unwrap_or(4)
}

pub(super) fn luck_col(found: f64) -> Color32 {
    LUCK_BANDS[luck_band(found)].2
}

/// The luck bands as chart zones on a 0–100 % scale: (from, to, colour).
/// The same thresholds as `luck_band`, so a zone's colour is always the
/// colour `luck_col` gives any value inside it.
pub(super) fn luck_zones_pct() -> Vec<(f64, f64, Color32)> {
    let mut lo = 0.0;
    LUCK_BANDS
        .iter()
        .map(|(hi, _, c)| {
            let hi = hi.min(1.0);
            let r = (lo * 100.0, hi * 100.0, *c);
            lo = hi;
            r
        })
        .collect()
}

/// The multiple of the odds by which a share `f` of hunters have found it.
pub(super) fn mult_at(f: f64) -> f64 {
    -(1.0 - f.min(0.999)).ln()
}

/// The luck bands as zones on a "multiple of the odds" scale up to `hi`.
pub(super) fn luck_zones_mult(hi: f64) -> Vec<(f64, f64, Color32)> {
    let mut lo = 0.0;
    LUCK_BANDS
        .iter()
        .map(|(f, _, c)| {
            let top = if *f >= 1.0 { hi.max(lo) } else { mult_at(*f) };
            let r = (lo, top, *c);
            lo = top;
            r
        })
        .collect()
}

/// The zone a value falls in: zones start at their `from` (a value exactly
/// on a boundary belongs to the zone above, like `luck_band`), and the last
/// zone keeps its top edge.
pub(super) fn zone_at(zones: &[(f64, f64, Color32)], v: f64) -> Option<Color32> {
    if zones.is_empty() {
        return None;
    }
    let i = zones.iter().rposition(|(a, _, _)| v >= *a).unwrap_or(0);
    Some(zones[i].2)
}

/// The colour for an axis number: the zone it sits in, or for a number on a
/// boundary, the zone it is the top of (0 takes the first zone).
fn tick_zone(zones: &[(f64, f64, Color32)], v: f64) -> Option<Color32> {
    zones.iter().find(|(a, b, _)| v > *a && v <= *b).or(zones.first().filter(|z| v <= z.0)).map(|z| z.2)
}

// ───────────────────────────── cards ─────────────────────────────

/// A dashboard card: an emoji and a title, an optional coloured note on
/// the right (e.g. "↓ 12%"), then the body.
pub(super) fn card<R>(ui: &mut egui::Ui, icon: &str, title: &str, right: Option<(&str, Color32)>, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    kit::card(ui, |ui| {
        ui.horizontal(|ui| {
            if !icon.is_empty() {
                ui.label(egui::RichText::new(icon).size(kit::fs(16.0)));
            }
            ui.label(egui::RichText::new(title).font(bold(16.0)).color(TEXT()));
            if let Some((t, c)) = right {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(t).font(bold(14.0)).color(ink(c)));
                });
            }
        });
        ui.add_space(10.0);
        body(ui)
    })
}

/// "LABEL" over a big coloured number with an optional unit.
pub(super) fn big_number(ui: &mut egui::Ui, label: &str, value: &str, unit: &str, col: Color32, size: f32) -> Response {
    let p = ui.painter().clone();
    let lab = p.layout_no_wrap(label.to_uppercase(), bold(11.0), MUTED());
    let num = p.layout_no_wrap(value.to_string(), heavy(size), ink(col));
    let unit_g = p.layout_no_wrap(unit.to_string(), bold(size * 0.42), ink(col));
    let w = (num.size().x + if unit.is_empty() { 0.0 } else { unit_g.size().x + 4.0 }).max(lab.size().x);
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, lab.size().y + 2.0 + num.size().y), Sense::hover());
    let ny = r.min.y + lab.size().y + 2.0;
    let nh = num.size().y;
    let nw = num.size().x;
    p.galley(r.min, lab, MUTED());
    p.galley(egui::pos2(r.min.x, ny), num, ink(col));
    if !unit.is_empty() {
        let uh = unit_g.size().y;
        p.galley(egui::pos2(r.min.x + nw + 4.0, ny + nh - uh - nh * 0.12), unit_g, ink(col));
    }
    resp
}

// ───────────────────────────── rings ─────────────────────────────

pub(super) struct Ring {
    pub frac: f32,
    pub col: Color32,
}

fn arc(p: &egui::Painter, c: Pos2, r: f32, a0: f32, a1: f32, w: f32, col: Color32) {
    let steps = (((a1 - a0).abs() / std::f32::consts::TAU) * 96.0).ceil().max(2.0) as usize;
    let pts: Vec<Pos2> = (0..=steps).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        c + egui::vec2(a.cos(), a.sin()) * r
    }).collect();
    p.add(Shape::line(pts, Stroke::new(w, col)));
    p.circle_filled(c + egui::vec2(a0.cos(), a0.sin()) * r, w / 2.0, col);
    p.circle_filled(c + egui::vec2(a1.cos(), a1.sin()) * r, w / 2.0, col);
}

/// Concentric activity rings (outermost first). Past a full lap the ring
/// keeps going, with a shadow at its head so the overlap shows.
pub(super) fn rings(p: &egui::Painter, c: Pos2, outer: f32, w: f32, gap: f32, list: &[Ring]) {
    let start = -std::f32::consts::FRAC_PI_2;
    for (i, rg) in list.iter().enumerate() {
        let r = outer - w / 2.0 - i as f32 * (w + gap);
        if r <= w {
            break;
        }
        p.circle_stroke(c, r, Stroke::new(w, faint(rg.col, 0.22)));
        let f = rg.frac.max(0.0);
        if f <= 0.001 {
            continue;
        }
        let laps = f.floor();
        if f >= 1.0 {
            p.circle_stroke(c, r, Stroke::new(w, rg.col));
        }
        let rest = if f >= 1.0 { f - laps } else { f };
        if f >= 1.0 && rest > 0.001 {
            let a1 = start + std::f32::consts::TAU * rest;
            // shadow just behind the head
            for k in 0..4 {
                let a = a1 + 0.05 + k as f32 * 0.03;
                p.circle_filled(c + egui::vec2(a.cos(), a.sin()) * r, w / 2.0, faint(Color32::BLACK, 0.18 - k as f32 * 0.04));
            }
            arc(p, c, r, start, a1, w, theme::blend(rg.col, Color32::WHITE, 0.12));
        } else if f < 1.0 {
            arc(p, c, r, start, start + std::f32::consts::TAU * rest, w, rg.col);
        }
        // a little arrow at the start
        let ap = c + egui::vec2(0.0, -r);
        let s = w * 0.26;
        p.add(Shape::line(vec![ap + egui::vec2(-s * 0.6, -s), ap + egui::vec2(s * 0.6, 0.0), ap + egui::vec2(-s * 0.6, s)], Stroke::new(w * 0.16, faint(Color32::BLACK, 0.6))));
    }
}

// ───────────────────────────── bar chart ─────────────────────────────

pub(super) struct Bars<'a> {
    pub values: &'a [f64],
    /// x labels (empty = none for that bar).
    pub labels: &'a [String],
    pub col: Color32,
    /// Bars drawn in full colour (others dimmer); None = all full.
    pub hot: Option<&'a dyn Fn(usize) -> bool>,
    /// Text on hover.
    pub hovers: &'a [String],
    /// Values above the bars.
    pub tops: bool,
    /// Grid lines with values on the right.
    pub axis: bool,
    pub height: f32,
    /// Colour per bar (overrides `col`).
    pub cols: Option<&'a dyn Fn(usize) -> Color32>,
    /// Numbers under the labels (e.g. how many days).
    pub subs: &'a [String],
}

fn nice_max(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let e = 10f64.powf(v.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if m * e >= v {
            return m * e;
        }
    }
    10.0 * e
}

pub(super) fn short_num(v: f64) -> String {
    if v >= 10_000.0 {
        format!("{:.0}k", v / 1000.0)
    } else if v >= 1000.0 {
        let s = format!("{:.1}k", v / 1000.0);
        s.replace(".0k", "k")
    } else if v >= 10.0 || v == 0.0 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

pub(super) fn bars(ui: &mut egui::Ui, b: &Bars) -> Response {
    let n = b.values.len().max(1);
    let axis_w = if b.axis { 46.0 } else { 0.0 };
    let top_h = if b.tops { 20.0 } else { 6.0 };
    let label_h = if b.labels.iter().any(|l| !l.is_empty()) { 20.0 } else { 0.0 } + if b.subs.is_empty() { 0.0 } else { 16.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), b.height), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));
    let plot = Rect::from_min_max(rect.min + egui::vec2(0.0, top_h), rect.max - egui::vec2(axis_w, label_h));
    let max = nice_max(b.values.iter().cloned().fold(0.0, f64::max));
    if b.axis {
        for k in 0..=2 {
            let v = max * k as f64 / 2.0;
            let y = plot.max.y - plot.height() * k as f32 / 2.0;
            p.add(Shape::dashed_line(&[egui::pos2(plot.min.x, y), egui::pos2(plot.max.x, y)], Stroke::new(1.0f32, grid()), 3.0, 3.0));
            p.text(egui::pos2(rect.max.x, y), Align2::RIGHT_CENTER, short_num(v), reg(11.0), MUTED());
        }
    }
    let slot = plot.width() / n as f32;
    let bw = (slot * 0.68).clamp(2.0, 46.0);
    let hover_i = ui.input(|i| i.pointer.hover_pos()).filter(|h| plot.expand2(egui::vec2(0.0, top_h)).contains(*h)).map(|h| (((h.x - plot.min.x) / slot).floor() as usize).min(n - 1));
    // The bars rise from the baseline each time the page opens.
    let rise = super::ui_kit::grow(ui.ctx(), resp.id, 1.0);
    for (i, &v) in b.values.iter().enumerate() {
        let cx = plot.min.x + slot * (i as f32 + 0.5);
        let col = b.cols.map_or(b.col, |f| f(i));
        let hot = b.hot.is_none_or(|f| f(i)) || hover_i == Some(i);
        let full = Rect::from_min_max(egui::pos2(cx - bw / 2.0, plot.min.y), egui::pos2(cx + bw / 2.0, plot.max.y));
        if !b.axis {
            p.rect_filled(full, (bw / 2.0).min(6.0), track());
        }
        let h = (v / max) as f32 * plot.height() * rise;
        if v > 0.0 {
            let r = Rect::from_min_max(egui::pos2(full.min.x, plot.max.y - h.max(3.0)), full.max);
            p.rect_filled(r, (bw / 2.0).min(6.0), if hot { col } else { faint(col, 0.45) });
        }
        if b.tops && v > 0.0 || b.tops && b.axis == false {
            p.text(egui::pos2(cx, plot.max.y - h - 4.0), Align2::CENTER_BOTTOM, short_num(v), bold(12.0), if hot { ink(col) } else { MUTED() });
        }
        if let Some(l) = b.labels.get(i).filter(|l| !l.is_empty()) {
            p.text(egui::pos2(cx, plot.max.y + 4.0), Align2::CENTER_TOP, l, reg(12.0), if hover_i == Some(i) { TEXT() } else { MUTED() });
        }
        if let Some(s) = b.subs.get(i).filter(|s| !s.is_empty()) {
            p.text(egui::pos2(cx, plot.max.y + 22.0), Align2::CENTER_TOP, s, reg(10.5), faint(MUTED(), 0.75));
        }
    }
    if let Some(i) = hover_i {
        if let Some(t) = b.hovers.get(i).filter(|t| !t.is_empty()) {
            egui::show_tooltip_text(ui.ctx(), ui.layer_id(), ui.id().with(("bars", rect.min.x as i32, rect.min.y as i32)), t);
        }
    }
    resp
}

// ───────────────────────────── line chart ─────────────────────────────

pub(super) struct Line<'a> {
    pub values: &'a [f64],
    pub labels: &'a [String],
    /// Coloured zones behind: (from, to, colour).
    pub bands: &'a [(f64, f64, Color32)],
    /// y range.
    pub lo: f64,
    pub hi: f64,
    /// Ticks on the left: (value, text).
    pub ticks: &'a [(f64, String)],
    /// Ticks on the right, drawn in the colour of the zone they top
    /// (e.g. the luck thresholds): (value, text).
    pub right_ticks: &'a [(f64, String)],
    pub col: &'a dyn Fn(usize, f64) -> Color32,
    /// A dashed straight-line trend.
    pub trend: bool,
    pub hovers: &'a [String],
    pub height: f32,
    /// Fill under the line.
    pub area: Option<Color32>,
    /// Extra markers drawn over points (e.g. a star for a shiny month).
    pub stars: &'a [usize],
    /// "You are here": (fractional index, value, label).
    pub you: Option<(f32, f64, &'a str)>,
    /// A picture under each point (drawn centred on the given spot).
    pub pic: Option<&'a mut dyn FnMut(&egui::Painter, usize, Pos2)>,
    /// Draw dots on the points.
    pub dots: bool,
    /// One colour for the whole line (None = each segment in its point's colour).
    pub line: Option<Color32>,
}

/// Least-squares slope and intercept.
pub(super) fn fit_line(v: &[f64]) -> Option<(f64, f64)> {
    let n = v.len() as f64;
    if v.len() < 2 {
        return None;
    }
    let mx = (n - 1.0) / 2.0;
    let my = v.iter().sum::<f64>() / n;
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (i, y) in v.iter().enumerate() {
        sxy += (i as f64 - mx) * (y - my);
        sxx += (i as f64 - mx).powi(2);
    }
    let b = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    Some((b, my - b * mx))
}

pub(super) fn line_chart(ui: &mut egui::Ui, l: &mut Line) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), l.height), Sense::hover());
    let p = ui.painter_at(rect.expand(4.0));
    let pic_h = if l.pic.is_some() { 44.0 } else { 0.0 };
    let plot = Rect::from_min_max(
        rect.min + egui::vec2(if l.ticks.is_empty() { 14.0 } else { 40.0 }, 14.0),
        rect.max - egui::vec2(if l.right_ticks.is_empty() { 14.0 } else { 44.0 }, 26.0 + pic_h),
    );
    let span = (l.hi - l.lo).max(1e-9);
    let to_y = |v: f64| plot.max.y - ((v - l.lo) / span).clamp(0.0, 1.0) as f32 * plot.height();
    let n = l.values.len().max(1);
    let to_x = |i: usize| if n == 1 { plot.center().x } else { plot.min.x + plot.width() * i as f32 / (n - 1) as f32 };
    for &(a, b, c) in l.bands {
        let r = Rect::from_min_max(egui::pos2(plot.min.x, to_y(b.min(l.hi))), egui::pos2(plot.max.x, to_y(a.max(l.lo))));
        p.rect_filled(r, 0.0, faint(c, if theme::pal().light { 0.14 } else { 0.11 }));
    }
    // zone edges: a crisp line in the colour of the zone below
    for w in l.bands.windows(2) {
        let v = w[1].0;
        if v > l.lo && v < l.hi {
            let y = to_y(v);
            p.line_segment([egui::pos2(plot.min.x, y), egui::pos2(plot.max.x, y)], Stroke::new(1.0f32, faint(w[0].2, 0.55)));
        }
    }
    for (v, t) in l.ticks {
        let y = to_y(*v);
        if !l.bands.windows(2).any(|w| (w[1].0 - *v).abs() < 1e-9) {
            p.add(Shape::dashed_line(&[egui::pos2(plot.min.x, y), egui::pos2(plot.max.x, y)], Stroke::new(1.0f32, grid()), 5.0, 4.0));
        }
        let c = tick_zone(l.bands, *v).map_or(MUTED(), ink);
        p.text(egui::pos2(plot.min.x - 6.0, y), Align2::RIGHT_CENTER, t, reg(11.5), c);
    }
    for (v, t) in l.right_ticks {
        if *v < l.lo || *v > l.hi {
            continue;
        }
        let y = to_y(*v);
        let c = tick_zone(l.bands, *v).map_or(MUTED(), ink);
        p.line_segment([egui::pos2(plot.max.x, y), egui::pos2(plot.max.x + 4.0, y)], Stroke::new(1.5f32, c));
        p.text(egui::pos2(plot.max.x + 7.0, y), Align2::LEFT_CENTER, t, reg(11.0), c);
    }
    let pts: Vec<Pos2> = l.values.iter().enumerate().map(|(i, v)| egui::pos2(to_x(i), to_y(*v))).collect();
    if let (Some(ac), true) = (l.area, pts.len() > 1) {
        let mut mesh = egui::Mesh::default();
        for (i, q) in pts.iter().enumerate() {
            mesh.colored_vertex(*q, faint(ac, 0.35));
            mesh.colored_vertex(egui::pos2(q.x, plot.max.y), faint(ac, 0.0));
            if i > 0 {
                let k = (i * 2) as u32;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k, k + 1);
            }
        }
        p.add(Shape::mesh(mesh));
    }
    if l.trend {
        if let Some((b, a)) = fit_line(l.values) {
            let y0 = a;
            let y1 = a + b * (n as f64 - 1.0);
            p.add(Shape::dashed_line(&[egui::pos2(to_x(0), to_y(y0)), egui::pos2(to_x(n - 1), to_y(y1))], Stroke::new(2.0f32, faint(RED, 0.8)), 8.0, 6.0));
        }
    }
    if pts.len() > 1 {
        for i in 1..pts.len() {
            let (v0, v1) = (l.values[i - 1], l.values[i]);
            match (l.line, l.bands.is_empty()) {
                (Some(c), _) => {
                    p.line_segment([pts[i - 1], pts[i]], Stroke::new(3.0f32, c));
                }
                (None, true) => {
                    p.line_segment([pts[i - 1], pts[i]], Stroke::new(3.0f32, (l.col)(i - 1, v0)));
                }
                (None, false) => {
                    // Split the segment where it crosses a zone edge, so the
                    // line changes colour exactly on the boundary.
                    let mut cuts: Vec<f64> = vec![0.0, 1.0];
                    for z in l.bands.iter().skip(1) {
                        let e = z.0;
                        if (v0 - e) * (v1 - e) < 0.0 {
                            cuts.push((e - v0) / (v1 - v0));
                        }
                    }
                    cuts.sort_by(f64::total_cmp);
                    for w in cuts.windows(2) {
                        let at = |t: f64| pts[i - 1] + (pts[i] - pts[i - 1]) * t as f32;
                        let mid = v0 + (v1 - v0) * (w[0] + w[1]) / 2.0;
                        let c = zone_at(l.bands, mid).unwrap_or(MUTED());
                        p.line_segment([at(w[0]), at(w[1])], Stroke::new(3.0f32, c));
                    }
                }
            }
        }
    }
    let hover_i = ui.input(|i| i.pointer.hover_pos()).filter(|h| rect.contains(*h)).map(|h| {
        pts.iter().enumerate().min_by(|a, b| (a.1.x - h.x).abs().total_cmp(&(b.1.x - h.x).abs())).map_or(0, |(i, _)| i)
    });
    for (i, q) in pts.iter().enumerate() {
        // On a zoned chart a point always takes the colour of the zone it sits in.
        let c = zone_at(l.bands, l.values[i]).unwrap_or_else(|| (l.col)(i, l.values[i]));
        if l.dots || hover_i == Some(i) {
            let r = if hover_i == Some(i) { 8.0 } else { 6.5 };
            p.circle_filled(*q, r + 2.5, faint(c, 0.35));
            p.circle_filled(*q, r, c);
        }
        if l.stars.contains(&i) {
            crate::theme::sparkle(&p, *q + egui::vec2(0.0, -16.0), 7.0, GOLD);
        }
    }
    if let Some((fx, v, label)) = l.you {
        let x = if n == 1 { plot.center().x } else { plot.min.x + plot.width() * fx / (n - 1) as f32 };
        let q = egui::pos2(x.clamp(plot.min.x, plot.max.x), to_y(v));
        p.add(Shape::dashed_line(&[egui::pos2(q.x, plot.max.y), q], Stroke::new(1.5f32, TEXT()), 4.0, 3.0));
        p.circle_filled(q, 11.0, faint(TEXT(), 0.25));
        p.circle_filled(q, 7.0, TEXT());
        p.circle_filled(q, 4.0, CARD());
        let g = p.layout_no_wrap(label.to_string(), bold(12.5), CARD());
        let w = g.size().x + 16.0;
        let tr = Rect::from_min_size(egui::pos2((q.x - w / 2.0).clamp(plot.min.x, plot.max.x - w), q.y - 38.0), egui::vec2(w, 22.0));
        p.rect_filled(tr, 11.0, TEXT());
        p.galley(tr.min + egui::vec2(8.0, 3.0), g, CARD());
    }
    for (i, t) in l.labels.iter().enumerate() {
        if !t.is_empty() {
            p.text(egui::pos2(to_x(i), plot.max.y + 8.0 + pic_h), Align2::CENTER_TOP, t, reg(12.0), if hover_i == Some(i) { TEXT() } else { MUTED() });
        }
    }
    if let Some(pic) = l.pic.as_mut() {
        for i in 0..pts.len() {
            pic(&p, i, egui::pos2(to_x(i), plot.max.y + 6.0 + pic_h / 2.0));
        }
    }
    if let Some(i) = hover_i {
        if let Some(t) = l.hovers.get(i).filter(|t| !t.is_empty()) {
            egui::show_tooltip_text(ui.ctx(), ui.layer_id(), ui.id().with(("line", rect.min.x as i32, rect.min.y as i32)), t);
        }
    }
    resp
}

// ───────────────────────────── distribution bar ─────────────────────────────

/// One stacked bar split by share, with a legend underneath:
/// (name, value, colour).
pub(super) fn dist_bar(ui: &mut egui::Ui, segs: &[(String, f64, Color32)], legend_cols: usize) {
    let total: f64 = segs.iter().map(|s| s.1).sum::<f64>().max(1e-9);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::hover());
    let p = ui.painter_at(rect);
    let shown: Vec<&(String, f64, Color32)> = segs.iter().filter(|s| s.1 > 0.0).collect();
    let gap = 5.0;
    let room = rect.width() - gap * (shown.len().saturating_sub(1)) as f32;
    let mut x = rect.min.x;
    for s in &shown {
        let w = (room * (s.1 / total) as f32).max(8.0);
        p.rect_filled(Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(w.min(rect.max.x - x), rect.height())), 8.0, s.2);
        x += w + gap;
    }
    if shown.is_empty() {
        p.rect_filled(rect, 8.0, track());
    }
    ui.add_space(10.0);
    let per = legend_cols.max(1);
    let cw = ui.available_width() / per as f32;
    for row in segs.chunks(per) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (name, v, c) in row {
                let (r, _) = ui.allocate_exact_size(egui::vec2(cw, 24.0), Sense::hover());
                let p = ui.painter_at(r);
                p.circle_filled(egui::pos2(r.min.x + 7.0, r.center().y), 6.0, *c);
                let pct = (v / total * 100.0).round();
                p.text(egui::pos2(r.min.x + 20.0, r.center().y), Align2::LEFT_CENTER, format!("{name} {pct:.0}%"), reg(13.5), if *v > 0.0 { TEXT() } else { MUTED() });
            }
        });
    }
}

// ───────────────────────────── hour clock ─────────────────────────────

/// A 24-hour clock: a bar for each hour pointing out from the middle.
pub(super) fn clock(ui: &mut egui::Ui, vals: &[f64; 24], col: Color32, size: f32, centre: (&str, &str)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), size), Sense::hover());
    let p = ui.painter_at(rect);
    let c = rect.center();
    let r_out = size / 2.0 - 18.0;
    let r_in = r_out * 0.42;
    let max = vals.iter().cloned().fold(0.0, f64::max).max(1e-9);
    let best = vals.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i);
    p.circle_stroke(c, r_in - 4.0, Stroke::new(1.0f32, grid()));
    p.circle_stroke(c, r_out, Stroke::new(1.0f32, faint(grid(), 0.6)));
    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|h| (*h - c).length() < r_out + 10.0 && (*h - c).length() > r_in * 0.5);
    let hover_h = hover.map(|h| {
        let a = (h - c).angle() + std::f32::consts::FRAC_PI_2;
        ((a.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 24.0).floor() as usize) % 24
    });
    for (h, &v) in vals.iter().enumerate() {
        let a = (h as f32 + 0.5) / 24.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let dir = egui::vec2(a.cos(), a.sin());
        let len = (r_out - r_in) * (v / max) as f32;
        p.line_segment([c + dir * r_in, c + dir * (r_out)], Stroke::new(7.0f32, track()));
        if v > 0.0 {
            let hot = Some(h) == best || hover_h == Some(h);
            p.line_segment([c + dir * r_in, c + dir * (r_in + len.max(3.0))], Stroke::new(7.0f32, if hot { col } else { faint(col, 0.55) }));
        }
    }
    for (h, t) in [(0, "12a"), (6, "6a"), (12, "12p"), (18, "6p")] {
        let a = h as f32 / 24.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        p.text(c + egui::vec2(a.cos(), a.sin()) * (r_out + 11.0), Align2::CENTER_CENTER, t, reg(11.0), MUTED());
    }
    p.text(c - egui::vec2(0.0, 7.0), Align2::CENTER_CENTER, centre.0, bold(16.0), ink(col));
    p.text(c + egui::vec2(0.0, 11.0), Align2::CENTER_CENTER, centre.1, reg(10.5), MUTED());
    if let Some(h) = hover_h {
        let t = format!("{}–{}: {}", crate::stats::fmt_hour(h as i64), crate::stats::fmt_hour(h as i64 + 1), theme::thousands(vals[h].round() as u64));
        egui::show_tooltip_text(ui.ctx(), ui.layer_id(), ui.id().with("clock"), t);
    }
    resp
}

// ───────────────────────────── calendar heat map ─────────────────────────────

/// Days as squares, a column per week (Monday on top), shaded by `value`.
pub(super) fn heatmap(ui: &mut egui::Ui, last_day: i64, weeks: usize, value: &dyn Fn(i64) -> u64, col: Color32, hover: &dyn Fn(i64, u64) -> String) -> Response {
    heatmap_marked(ui, last_day, weeks, value, col, hover, None)
}

/// `heatmap`, with a gold dot on the days `mark` picks (e.g. a shiny caught).
pub(super) fn heatmap_marked(
    ui: &mut egui::Ui,
    last_day: i64,
    weeks: usize,
    value: &dyn Fn(i64) -> u64,
    col: Color32,
    hover: &dyn Fn(i64, u64) -> String,
    mark: Option<&dyn Fn(i64) -> bool>,
) -> Response {
    let gap = 3.0;
    let avail = ui.available_width();
    let cell = ((avail - 26.0 - gap * (weeks as f32 - 1.0)) / weeks as f32).clamp(6.0, 26.0);
    let h = cell * 7.0 + gap * 6.0 + 18.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(avail, h), Sense::hover());
    let p = ui.painter_at(rect);
    let last_w = crate::stats::weekday(last_day) as i64;
    let first = last_day - last_w - (weeks as i64 - 1) * 7;
    let mut max = 0;
    for d in first..=last_day {
        max = max.max(value(d));
    }
    let max = max.max(1) as f32;
    let x0 = rect.min.x + 26.0;
    for (k, l) in [(0, "Mon"), (2, "Wed"), (4, "Fri"), (6, "Sun")] {
        p.text(egui::pos2(rect.min.x, rect.min.y + 16.0 + k as f32 * (cell + gap) + cell / 2.0), Align2::LEFT_CENTER, l, reg(10.0), MUTED());
    }
    let mut hovered = None;
    let hp = ui.input(|i| i.pointer.hover_pos());
    let mut last_month = -1;
    for w in 0..weeks {
        for wd in 0..7 {
            let d = first + (w * 7 + wd) as i64;
            if d > last_day {
                continue;
            }
            let r = Rect::from_min_size(egui::pos2(x0 + w as f32 * (cell + gap), rect.min.y + 16.0 + wd as f32 * (cell + gap)), egui::vec2(cell, cell));
            let v = value(d);
            let c = if v == 0 { track() } else { theme::blend(faint(col, 0.35), col, (v as f32 / max).sqrt()) };
            p.rect_filled(r, 3.0, c);
            if mark.is_some_and(|m| m(d)) {
                p.circle_filled(r.center(), (cell * 0.24).max(2.0), GOLD);
            }
            if d == last_day {
                p.rect_stroke(r.expand(1.0), 3.0, Stroke::new(1.5f32, TEXT()));
            }
            if hp.is_some_and(|h| r.expand(gap / 2.0).contains(h)) {
                hovered = Some((d, v));
            }
            if wd == 0 {
                let (_, m, dd) = crate::stats::civil_from_days(d);
                if m as i32 != last_month && dd <= 7 {
                    last_month = m as i32;
                    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                    p.text(egui::pos2(r.min.x, rect.min.y), Align2::LEFT_TOP, MONTHS[(m as usize + 11) % 12], reg(10.5), MUTED());
                }
            }
        }
    }
    if let Some((d, v)) = hovered {
        egui::show_tooltip_text(ui.ctx(), ui.layer_id(), ui.id().with("heat"), hover(d, v));
    }
    resp
}

// ───────────────────────────── tiles and rows ─────────────────────────────

/// The heatmap's key: "Less ▢▢▢▢ More", plus what the dot means.
pub(super) fn heat_legend(ui: &mut egui::Ui, col: Color32, dot: Option<&str>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(egui::RichText::new("Less").size(11.0).color(MUTED()));
        for k in 0..5 {
            let (r, _) = ui.allocate_exact_size(egui::vec2(11.0, 11.0), Sense::hover());
            let c = if k == 0 { track() } else { theme::blend(faint(col, 0.35), col, (k as f32 / 4.0).sqrt()) };
            ui.painter().rect_filled(r, 2.5, c);
        }
        ui.label(egui::RichText::new("More").size(11.0).color(MUTED()));
        if let Some(t) = dot {
            ui.add_space(12.0);
            let (r, _) = ui.allocate_exact_size(egui::vec2(11.0, 11.0), Sense::hover());
            ui.painter().rect_filled(r, 2.5, track());
            ui.painter().circle_filled(r.center(), 2.8, GOLD);
            ui.label(egui::RichText::new(t).size(11.0).color(MUTED()));
        }
    });
}

/// "Keep it going" / "Worth a look" tile: a chevron (up, down or flat),
/// a label, a coloured value and a sentence.
pub(super) fn trend_tile(ui: &mut egui::Ui, w: f32, dir: Option<bool>, label: &str, value: &str, note: &str, col: Color32) {
    let galley = ui.painter().layout(note.to_string(), reg(12.0), MUTED(), w - 70.0);
    let h = 62.0 + if note.is_empty() { 0.0 } else { galley.size().y + 6.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));
    p.rect_filled(rect, 14.0, CARD_HOVER());
    let c = egui::pos2(rect.min.x + 30.0, rect.min.y + 32.0);
    let s = 10.0;
    let col = ink(col);
    match dir {
        Some(true) => {
            p.add(Shape::line(vec![c + egui::vec2(-s, s * 0.5), c + egui::vec2(0.0, -s * 0.5), c + egui::vec2(s, s * 0.5)], Stroke::new(3.5f32, col)));
        }
        Some(false) => {
            p.add(Shape::line(vec![c + egui::vec2(-s, -s * 0.5), c + egui::vec2(0.0, s * 0.5), c + egui::vec2(s, -s * 0.5)], Stroke::new(3.5f32, col)));
        }
        None => {
            p.line_segment([c - egui::vec2(s, 0.0), c + egui::vec2(s, 0.0)], Stroke::new(3.5f32, col));
        }
    }
    let x = rect.min.x + 56.0;
    p.text(egui::pos2(x, rect.min.y + 20.0), Align2::LEFT_CENTER, label, reg(13.0), TEXT());
    p.text(egui::pos2(x, rect.min.y + 42.0), Align2::LEFT_CENTER, value, heavy(18.0), col);
    if !note.is_empty() {
        p.galley(egui::pos2(x, rect.min.y + 58.0), galley, MUTED());
    }
}

/// A small fact: an emoji, a coloured value and what it is.
pub(super) fn fact_tile(ui: &mut egui::Ui, w: f32, icon: &str, value: &str, label: &str, col: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 72.0), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));
    p.rect_filled(rect, 14.0, CARD_HOVER());
    let chip = Rect::from_center_size(egui::pos2(rect.min.x + 32.0, rect.center().y), egui::vec2(38.0, 38.0));
    p.rect_filled(chip, 11.0, faint(col, 0.2));
    p.text(chip.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(18.0), TEXT());
    let x = chip.max.x + 12.0;
    let room = rect.max.x - 10.0 - x;
    let vf = fit_font(ui, value, heavy, 20.0, 12.0, room);
    p.text(egui::pos2(x, rect.center().y - 9.0), Align2::LEFT_CENTER, value, vf, ink(col));
    let lab = clip(ui, label, &reg(12.0), room);
    p.text(egui::pos2(x, rect.center().y + 14.0), Align2::LEFT_CENTER, lab, reg(12.0), MUTED());
}

pub(super) fn fit_font(ui: &egui::Ui, s: &str, f: fn(f32) -> FontId, max: f32, min: f32, w: f32) -> FontId {
    let mut size = max;
    while size > min && ui.painter().layout_no_wrap(s.to_string(), f(size), TEXT()).size().x > w {
        size -= 0.5;
    }
    f(size)
}

pub(super) fn clip(ui: &egui::Ui, s: &str, f: &FontId, w: f32) -> String {
    let fits = |t: &str| ui.painter().layout_no_wrap(t.to_string(), f.clone(), TEXT()).size().x <= w;
    if fits(s) {
        return s.to_string();
    }
    let mut c: Vec<char> = s.chars().collect();
    while c.len() > 1 {
        c.pop();
        let t: String = c.iter().collect::<String>().trim_end().to_string() + "…";
        if fits(&t) {
            return t;
        }
    }
    "…".into()
}

/// A ranked row: "#1" badge, a picture (drawn by `pic` in the given rect),
/// a title and sub line, and a big score on the right with a side stripe.
#[allow(clippy::too_many_arguments)]
pub(super) fn rank_row(ui: &mut egui::Ui, rank: usize, title: &str, sub: &str, score: &str, score_note: &str, col: Color32, pic: &mut dyn FnMut(&egui::Painter, Rect)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 76.0), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));
    p.rect_filled(rect, 16.0, if resp.hovered() { theme::blend(CARD_HOVER(), TEXT(), 0.04) } else { CARD_HOVER() });
    p.rect_filled(Rect::from_min_max(egui::pos2(rect.max.x - 5.0, rect.min.y + 10.0), egui::pos2(rect.max.x - 1.0, rect.max.y - 10.0)), 2.0, col);
    let medal = match rank {
        1 => GOLD,
        2 => Color32::from_rgb(200, 208, 220),
        3 => Color32::from_rgb(214, 140, 80),
        _ => MUTED(),
    };
    let bc = egui::pos2(rect.min.x + 36.0, rect.center().y);
    p.circle_filled(bc, 22.0, faint(medal, 0.22));
    p.text(bc, Align2::CENTER_CENTER, format!("#{rank}"), bold(15.0), ink(medal));
    let pr = Rect::from_center_size(egui::pos2(rect.min.x + 96.0, rect.center().y), egui::vec2(56.0, 56.0));
    pic(&p, pr);
    let x = pr.max.x + 14.0;
    let sw = p.layout_no_wrap(score.to_string(), heavy(26.0), col).size().x;
    let room = rect.max.x - 30.0 - sw - x - 10.0;
    p.text(egui::pos2(x, rect.center().y - 11.0), Align2::LEFT_CENTER, clip(ui, title, &bold(17.0), room), bold(17.0), TEXT());
    p.text(egui::pos2(x, rect.center().y + 13.0), Align2::LEFT_CENTER, clip(ui, sub, &reg(12.5), room), reg(12.5), MUTED());
    p.text(egui::pos2(rect.max.x - 22.0, rect.center().y - 6.0), Align2::RIGHT_CENTER, score, heavy(26.0), ink(col));
    if !score_note.is_empty() {
        p.text(egui::pos2(rect.max.x - 22.0, rect.center().y + 17.0), Align2::RIGHT_CENTER, score_note, reg(11.0), MUTED());
    }
    resp
}

/// A horizontal bar row: a picture, a name, a bar of `frac` and a value.
#[allow(clippy::too_many_arguments)]
pub(super) fn hbar_row(ui: &mut egui::Ui, name: &str, value: &str, frac: f32, col: Color32, highlight: bool, pic: &mut dyn FnMut(&egui::Painter, Rect)) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), Sense::hover());
    let frac = super::ui_kit::grow(ui.ctx(), resp.id, frac);
    let p = ui.painter_at(rect.expand(2.0));
    if resp.hovered() || highlight {
        p.rect_filled(rect, 10.0, if highlight { faint(GOLD, 0.12) } else { CARD_HOVER() });
    }
    let pr = Rect::from_center_size(egui::pos2(rect.min.x + 22.0, rect.center().y), egui::vec2(36.0, 36.0));
    pic(&p, pr);
    let name_w = 130.0f32.min(rect.width() * 0.3);
    p.text(egui::pos2(pr.max.x + 10.0, rect.center().y), Align2::LEFT_CENTER, clip(ui, name, &reg(13.5), name_w - 8.0), if highlight { bold(13.5) } else { reg(13.5) }, TEXT());
    let vx = rect.max.x - 10.0;
    let vw = 90.0;
    p.text(egui::pos2(vx, rect.center().y), Align2::RIGHT_CENTER, value, bold(13.0), if highlight { ink(GOLD) } else { TEXT() });
    let bx0 = pr.max.x + 10.0 + name_w;
    let bx1 = vx - vw;
    if bx1 > bx0 + 10.0 {
        let tr = Rect::from_min_max(egui::pos2(bx0, rect.center().y - 6.0), egui::pos2(bx1, rect.center().y + 6.0));
        p.rect_filled(tr, 6.0, track());
        p.rect_filled(Rect::from_min_size(tr.min, egui::vec2((tr.width() * frac.clamp(0.0, 1.0)).max(6.0), tr.height())), 6.0, col);
    }
}

/// The luck scale: the five bands sized by how many hunters land in each,
/// with "you" marked. `found` = share of hunters who'd have it by now.
pub(super) fn luck_scale(ui: &mut egui::Ui, found: f64, you: &str) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 90.0), Sense::hover());
    // The "you" marker slides along the scale to its place.
    let found = super::ui_kit::grow(ui.ctx(), resp.id, found.clamp(0.0, 1.0) as f32) as f64;
    let p = ui.painter_at(rect.expand(4.0));
    let bar = Rect::from_min_max(egui::pos2(rect.min.x, rect.min.y + 30.0), egui::pos2(rect.max.x, rect.min.y + 52.0));
    let mut lo = 0.0;
    let gap = 4.0;
    for (i, (hi, name, c)) in LUCK_BANDS.iter().enumerate() {
        let hi = hi.min(1.0);
        let x0 = bar.min.x + bar.width() * lo as f32 + if i > 0 { gap / 2.0 } else { 0.0 };
        let x1 = bar.min.x + bar.width() * hi as f32 - if i < 4 { gap / 2.0 } else { 0.0 };
        let r = Rect::from_min_max(egui::pos2(x0, bar.min.y), egui::pos2(x1, bar.max.y));
        p.rect_filled(r, 7.0, *c);
        p.text(egui::pos2(r.center().x, bar.max.y + 12.0), Align2::CENTER_CENTER, *name, reg(11.0), MUTED());
        lo = hi;
    }
    // the share of hunters at each boundary, to scale under the gaps
    for (i, (hi, _, c)) in LUCK_BANDS.iter().enumerate().take(4) {
        let x = bar.min.x + bar.width() * *hi as f32;
        p.line_segment([egui::pos2(x, bar.max.y + 22.0), egui::pos2(x, bar.max.y + 27.0)], Stroke::new(1.5f32, ink(*c)));
        let _ = i;
        p.text(egui::pos2(x, bar.max.y + 30.0), Align2::CENTER_TOP, format!("{:.0}%", hi * 100.0), reg(10.5), ink(*c));
    }
    let x = bar.min.x + bar.width() * found.clamp(0.0, 1.0) as f32;
    let c = luck_col(found);
    p.line_segment([egui::pos2(x, bar.min.y - 4.0), egui::pos2(x, bar.max.y + 2.0)], Stroke::new(3.0f32, TEXT()));
    let g = p.layout_no_wrap(you.to_string(), bold(12.5), Color32::WHITE);
    let w = g.size().x + 18.0;
    let tx = (x - w / 2.0).clamp(rect.min.x, rect.max.x - w);
    let tr = Rect::from_min_size(egui::pos2(tx, rect.min.y), egui::vec2(w, 22.0));
    p.rect_filled(tr, 11.0, theme::blend(c, Color32::BLACK, 0.15));
    p.galley(tr.min + egui::vec2(9.0, 3.0), g, Color32::WHITE);
    p.add(Shape::convex_polygon(vec![egui::pos2(x - 5.0, tr.max.y), egui::pos2(x + 5.0, tr.max.y), egui::pos2(x, tr.max.y + 6.0)], theme::blend(c, Color32::BLACK, 0.15), Stroke::NONE));
}

/// An empty card's message.
pub(super) fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).size(kit::fs(12.5)).color(MUTED()));
    ui.add_space(6.0);
}

pub(super) fn legend(ui: &mut egui::Ui, items: &[(Color32, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for (c, t) in items {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.0, *c);
            ui.label(egui::RichText::new(*t).size(kit::fs(12.0)).color(MUTED()));
            ui.add_space(10.0);
        }
    });
}

/// Width for `n` tiles across with 12 px gaps.
pub(super) fn tile_w(ui: &egui::Ui, n: usize) -> f32 {
    ((ui.available_width() - 12.0 * (n as f32 - 1.0)) / n as f32).floor()
}


#[cfg(test)]
mod zone_tests {
    use super::*;

    /// Every zoned chart must colour a value the same way `luck_col` does,
    /// and the zone edges must sit exactly on the luck thresholds.
    #[test]
    fn zones_match_luck_bands() {
        let z = luck_zones_pct();
        let edges: Vec<f64> = z.iter().map(|b| b.0).collect();
        assert_eq!(edges, vec![0.0, 20.0, 45.0, 70.0, 90.0]);
        assert_eq!(z.last().unwrap().1, 100.0);
        for k in 0..=1000 {
            let v = k as f64 / 10.0;
            assert_eq!(zone_at(&z, v), Some(luck_col(v / 100.0)), "pct {v}");
        }
        let hi = 4.0;
        let zm = luck_zones_mult(hi);
        for (i, (hi_f, _, _)) in LUCK_BANDS.iter().enumerate().take(4) {
            assert!((zm[i].1 - mult_at(*hi_f)).abs() < 1e-12);
            assert!((zm[i + 1].0 - zm[i].1).abs() < 1e-12);
        }
        for k in 0..=4000 {
            let m = k as f64 / 1000.0;
            // the continuous curve the multiple scale is drawn on
            let f = 1.0 - (-m).exp();
            assert_eq!(zone_at(&zm, m), Some(luck_col(f)), "mult {m}");
        }
    }

    #[test]
    fn boundary_ticks_take_the_zone_below() {
        let z = luck_zones_pct();
        assert_eq!(tick_zone(&z, 0.0), Some(BLUE));
        assert_eq!(tick_zone(&z, 20.0), Some(BLUE));
        assert_eq!(tick_zone(&z, 45.0), Some(GREEN));
        assert_eq!(tick_zone(&z, 70.0), Some(YELLOW));
        assert_eq!(tick_zone(&z, 90.0), Some(ORANGE));
        assert_eq!(tick_zone(&z, 100.0), Some(RED));
    }
}
