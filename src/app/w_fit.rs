//! Fluid Dashboard widgets. Every widget fills the box it has on the grid:
//! it knows its size in squares and in pixels, picks a layout for its shape
//! (a small square, a flat strip, a tall column, a block or a big one) and
//! scales its text and pictures with the squares, so a 4×2 looks the same
//! on any screen, only bigger or smaller, and nothing is cut off or left
//! empty.

use super::store::pill_button;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Galley, Painter, Pos2, Rect, Sense, Stroke};
use std::sync::Arc;

/// The box a widget fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WBox {
    pub r: Rect,
    pub cols: i32,
    pub rows: i32,
    /// One grid square's side, in points.
    pub cell: f32,
}

/// What a box is like, by its squares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    /// 2×2 and smaller: one thing, big.
    Mini,
    /// At least twice as wide as tall (one square tall included).
    Strip,
    /// At least twice as tall as wide.
    Column,
    /// Squarish.
    Block,
    /// Squarish and roomy (20 squares or more).
    Big,
}

thread_local! {
    static BOX: std::cell::Cell<Option<WBox>> = const { std::cell::Cell::new(None) };
}

/// The next widget drawn fills this box (the Dashboard sets it).
pub(super) fn set_box(b: Option<WBox>) {
    BOX.with(|c| c.set(b));
}

/// The box this widget fills, taken up in `ui`: the Dashboard's, or (in the
/// gallery, or a window too narrow for the grid) one made from the room
/// there is, at the widget's start size.
pub(super) fn take_box(ui: &mut egui::Ui, start: (i32, i32)) -> WBox {
    let _ = kit::take_card_fill();
    if let Some(b) = BOX.with(|c| c.take()) {
        ui.allocate_rect(b.r, Sense::hover());
        return b;
    }
    let w = ui.available_width().max(60.0);
    let cell = ((w - 12.0 * (start.0 as f32 - 1.0)) / start.0 as f32).clamp(40.0, 150.0);
    let h = start.1 as f32 * (cell + 12.0) - 12.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    // (a plain card behind it, as the Dashboard draws one)
    super::store::card_bg(ui.painter(), r, false);
    WBox { r, cols: start.0, rows: start.1, cell }
}

/// The shape of the box the next widget fills (None off the grid).
pub(super) fn peek_shape() -> Option<Shape> {
    BOX.with(|c| c.get()).map(|b| b.shape())
}

/// The box for a widget of `kind` (its start size when there's no grid).
pub(super) fn wbox(ui: &mut egui::Ui, kind: &str) -> WBox {
    let start = super::widgets::def(kind).map_or((4, 2), super::widgets::start_size);
    take_box(ui, start)
}

impl WBox {
    /// How big things are drawn: 1 for a square of 120 points.
    pub fn s(&self) -> f32 {
        (self.cell / 120.0).clamp(0.6, 1.5)
    }

    pub fn shape(&self) -> Shape {
        let (c, r) = (self.cols, self.rows);
        if c <= 2 && r <= 2 {
            Shape::Mini
        } else if c >= r * 2 {
            Shape::Strip
        } else if r >= c * 2 {
            Shape::Column
        } else if c * r >= 20 {
            Shape::Big
        } else {
            Shape::Block
        }
    }

    /// One square tall.
    pub fn flat(&self) -> bool {
        self.rows <= 1
    }

    /// Space inside the card's edge.
    pub fn pad(&self) -> f32 {
        (14.0 * self.s()).round().max(8.0)
    }

    pub fn inner(&self) -> Rect {
        self.r.shrink(self.pad())
    }

    /// A size scaled with the squares.
    pub fn px(&self, v: f32) -> f32 {
        v * self.s()
    }

    pub fn font(&self, size: f32) -> FontId {
        crate::fonts::sans_font((size * self.s()).max(8.5))
    }

    pub fn bold(&self, size: f32) -> FontId {
        crate::fonts::sans_bold_font((size * self.s()).max(8.5))
    }

    /// The room left in the inner box under `y`.
    pub fn below(&self, y: f32) -> Rect {
        let i = self.inner();
        Rect::from_min_max(pos2(i.min.x, y.min(i.max.y)), i.max)
    }
}

/// The heading across the top of a widget (icon, title, a link on the
/// right). Returns the room under it and whether the link was clicked.
/// One square tall, or a small box: a slim one.
pub(super) fn head(ui: &mut egui::Ui, b: &WBox, icon: &str, col: Color32, title: &str, link: Option<&str>) -> (Rect, bool) {
    let inner = b.inner();
    let slim = b.flat() || b.shape() == Shape::Mini;
    let hh = if slim { b.px(16.0) } else { b.px(30.0) };
    let hr = Rect::from_min_size(inner.min, vec2(inner.width(), hh));
    let p = ui.painter_at(b.r);
    let mut x = hr.min.x;
    if slim {
        let g = p.layout_no_wrap(icon.to_string(), b.font(11.5), col);
        p.galley(pos2(x, hr.center().y - g.size().y / 2.0), g.clone(), col);
        x += g.size().x + b.px(6.0);
    } else {
        let chip = Rect::from_min_size(hr.min, vec2(hh, hh));
        kit::icon_chip(&p, chip, icon, col, false);
        x = chip.max.x + b.px(10.0);
    }
    // the link first (it gives way to the title when there isn't room)
    let mut clicked = false;
    let mut title_room = hr.max.x - x;
    if let Some(l) = link {
        let lg = p.layout_no_wrap(l.to_string(), b.font(if slim { 11.0 } else { 12.0 }), MUTED());
        let tg = p.layout_no_wrap(title.to_string(), if slim { b.font(11.5) } else { b.bold(14.5) }, TEXT());
        if tg.size().x + lg.size().x + b.px(16.0) <= hr.max.x - x {
            let lr = Rect::from_min_size(pos2(hr.max.x - lg.size().x, hr.center().y - lg.size().y / 2.0), lg.size());
            let resp = ui.interact(lr.expand(4.0), ui.id().with(("w-link", title, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            p.galley(lr.min, lg, if resp.hovered() { TEXT() } else { MUTED() });
            clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            title_room = lr.min.x - x - b.px(10.0);
        } else {
            // no room for the words: the heading itself is the link
            let resp = ui.interact(hr, ui.id().with(("w-link", title, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            clicked = resp.on_hover_text(l.trim_end_matches(" ›")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        }
    }
    let tg = fit(&p, title, if slim { b.font(11.5) } else { b.bold(14.5) }, 0.85, title_room, if slim { MUTED() } else { TEXT() });
    p.galley(pos2(x, hr.center().y - tg.size().y / 2.0), tg, TEXT());
    let gap = if slim { b.px(4.0) } else { b.px(10.0) };
    (b.below(hr.max.y + gap), clicked)
}

/// `text` in `font`, made smaller (down to `min` times its size) and then
/// cut with "…" until it fits `max_w`.
pub(super) fn fit(p: &Painter, text: &str, font: FontId, min: f32, max_w: f32, col: Color32) -> Arc<Galley> {
    let max_w = max_w.max(8.0);
    let mut f = font.clone();
    let g = p.layout_no_wrap(text.to_string(), f.clone(), col);
    if g.size().x <= max_w {
        return g;
    }
    // smaller first
    let shrink = (max_w / g.size().x).max(min);
    f.size = (font.size * shrink).max(8.0);
    let g = p.layout_no_wrap(text.to_string(), f.clone(), col);
    if g.size().x <= max_w {
        return g;
    }
    // then cut
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let t: String = chars[..mid].iter().collect::<String>() + "…";
        if p.layout_no_wrap(t, f.clone(), col).size().x <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let t: String = chars[..lo].iter().collect::<String>().trim_end().to_string() + "…";
    p.layout_no_wrap(t, f, col)
}

/// The biggest `text` (bold, from `max` points scaled, down to 9) that fits
/// `r`, placed by `align` in it. Returns where it went.
pub(super) fn hero(p: &Painter, r: Rect, text: &str, max: f32, col: Color32, align: Align2) -> Rect {
    let mut size = max;
    let mut g = p.layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(size), col);
    while (g.size().x > r.width() || g.size().y > r.height() * 1.05) && size > 9.0 {
        size = (size * 0.9).max(9.0);
        g = p.layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(size), col);
    }
    let at = align.align_size_within_rect(g.size(), r);
    p.galley(at.min, g, col);
    at
}

/// Text placed by `align` in `r`, fitted to its width.
pub(super) fn label(p: &Painter, r: Rect, text: &str, font: FontId, col: Color32, align: Align2) -> Rect {
    let g = fit(p, text, font, 0.8, r.width(), col);
    let at = align.align_size_within_rect(g.size(), r);
    p.galley(at.min, g, col);
    at
}

/// Cut `r` into a top part `h` tall (and a gap) and the rest.
pub(super) fn split_top(r: Rect, h: f32, gap: f32) -> (Rect, Rect) {
    let h = h.min(r.height());
    (Rect::from_min_size(r.min, vec2(r.width(), h)), Rect::from_min_max(pos2(r.min.x, (r.min.y + h + gap).min(r.max.y)), r.max))
}

/// Cut `r` into a bottom part `h` tall (and a gap above it) and the rest.
pub(super) fn split_bottom(r: Rect, h: f32, gap: f32) -> (Rect, Rect) {
    let h = h.min(r.height());
    (Rect::from_min_max(r.min, pos2(r.max.x, (r.max.y - h - gap).max(r.min.y))), Rect::from_min_max(pos2(r.min.x, r.max.y - h), r.max))
}

/// Cut `r` into a left part `w` wide (and a gap) and the rest.
pub(super) fn split_left(r: Rect, w: f32, gap: f32) -> (Rect, Rect) {
    let w = w.min(r.width());
    (Rect::from_min_size(r.min, vec2(w, r.height())), Rect::from_min_max(pos2((r.min.x + w + gap).min(r.max.x), r.min.y), r.max))
}

/// Cut `r` into a right part `w` wide (and a gap before it) and the rest.
pub(super) fn split_right(r: Rect, w: f32, gap: f32) -> (Rect, Rect) {
    let w = w.min(r.width());
    (Rect::from_min_max(r.min, pos2((r.max.x - w - gap).max(r.min.x), r.max.y)), Rect::from_min_max(pos2(r.max.x - w, r.min.y), r.max))
}

/// `n` boxes in a row across `r`, `gap` apart.
pub(super) fn cols(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    let n = n.max(1);
    let w = (r.width() - gap * (n as f32 - 1.0)) / n as f32;
    (0..n).map(|i| Rect::from_min_size(pos2(r.min.x + i as f32 * (w + gap), r.min.y), vec2(w, r.height()))).collect()
}

/// `n` boxes one under another down `r`, `gap` apart.
pub(super) fn stack(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    let n = n.max(1);
    let h = (r.height() - gap * (n as f32 - 1.0)) / n as f32;
    (0..n).map(|i| Rect::from_min_size(pos2(r.min.x, r.min.y + i as f32 * (h + gap)), vec2(r.width(), h))).collect()
}

/// Up to `n` tiles filling `r` in a grid, each at least `min_w` × `min_h`,
/// with the columns picked so the tiles come closest to `aspect` (width
/// over height). As many as fit when all `n` won't.
pub(super) fn tiles(r: Rect, n: usize, aspect: f32, min_w: f32, min_h: f32, gap: f32) -> Vec<Rect> {
    if n == 0 || r.width() < 1.0 || r.height() < 1.0 {
        return Vec::new();
    }
    let max_cols = (((r.width() + gap) / (min_w + gap)).floor() as usize).max(1);
    let max_rows = (((r.height() + gap) / (min_h + gap)).floor() as usize).max(1);
    let k = n.min(max_cols * max_rows);
    let mut best: Option<(f32, usize, usize)> = None;
    for c in 1..=max_cols.min(k) {
        let rws = k.div_ceil(c);
        if rws > max_rows {
            continue;
        }
        let w = (r.width() - gap * (c as f32 - 1.0)) / c as f32;
        let h = (r.height() - gap * (rws as f32 - 1.0)) / rws as f32;
        // close to the shape wanted, and few empty slots
        let empty = (c * rws - k) as f32;
        let score = ((w / h) / aspect).ln().abs() + empty * 0.35;
        if best.is_none_or(|b| score < b.0) {
            best = Some((score, c, rws));
        }
    }
    let (_, c, rws) = best.unwrap_or((0.0, max_cols.min(k).max(1), 1));
    let w = (r.width() - gap * (c as f32 - 1.0)) / c as f32;
    let h = (r.height() - gap * (rws as f32 - 1.0)) / rws as f32;
    (0..k.min(c * rws)).map(|i| Rect::from_min_size(pos2(r.min.x + (i % c) as f32 * (w + gap), r.min.y + (i / c) as f32 * (h + gap)), vec2(w, h))).collect()
}

/// Rows down `r` for up to `n` things, each `row_h` tall (at least): as
/// many as fit, sharing out what's left so the last one ends at the bottom
/// (but never more than `grow` times taller).
pub(super) fn rows(r: Rect, n: usize, row_h: f32, gap: f32, grow: f32) -> Vec<Rect> {
    if n == 0 || r.height() < row_h * 0.6 {
        return Vec::new();
    }
    let fit = (((r.height() + gap) / (row_h + gap)).floor() as usize).max(1);
    let k = n.min(fit);
    let h = ((r.height() - gap * (k as f32 - 1.0)) / k as f32).min(row_h * grow);
    (0..k).map(|i| Rect::from_min_size(pos2(r.min.x, r.min.y + i as f32 * (h + gap)), vec2(r.width(), h))).collect()
}

/// A clickable area that lights up on hover (draw it before what's in it).
pub(super) fn hit(ui: &mut egui::Ui, r: Rect, id: impl std::hash::Hash, rounding: f32) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(id), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    if ht > 0.0 {
        ui.painter().rect_filled(r, rounding, CARD_HOVER().gamma_multiply(ht));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A soft tile (a slot in a grid of things).
pub(super) fn tile(p: &Painter, r: Rect, tint: Color32, rounding: f32) {
    p.rect_filled(r, rounding, theme::blend(CARD(), tint, 0.07));
    p.rect_stroke(r, rounding, Stroke::new(1.0f32, theme::blend(LINE(), tint, 0.12)));
}

/// Bars for `vals` across `r` (the biggest reaching the top), `hi` in full
/// colour; labels under them when given and there's room.
pub(super) fn bars(p: &Painter, r: Rect, vals: &[u64], hi: Option<usize>, col: Color32, labels: Option<&[String]>, font: FontId) {
    if vals.is_empty() {
        return;
    }
    let lab_h = if labels.is_some() && r.height() > 50.0 { font.size + 6.0 } else { 0.0 };
    let plot = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - lab_h));
    let max = vals.iter().copied().max().unwrap_or(0).max(1) as f32;
    let slot = plot.width() / vals.len() as f32;
    let bw = (slot * 0.58).clamp(2.0, 40.0);
    for (k, v) in vals.iter().enumerate() {
        let cx = plot.min.x + slot * (k as f32 + 0.5);
        let h = (plot.height() * (*v as f32 / max)).max(3.0);
        let c = if Some(k) == hi { col } else { theme::blend(CARD(), col, if *v > 0 { 0.45 } else { 0.15 }) };
        p.rect_filled(Rect::from_min_max(pos2(cx - bw / 2.0, plot.max.y - h), pos2(cx + bw / 2.0, plot.max.y)), (bw / 3.0).min(6.0), c);
        if lab_h > 0.0 {
            if let Some(l) = labels.and_then(|l| l.get(k)) {
                p.text(pos2(cx, r.max.y - lab_h / 2.0 + 1.0), Align2::CENTER_CENTER, l, font.clone(), if Some(k) == hi { TEXT() } else { MUTED() });
            }
        }
    }
}

/// A rounded progress bar filling `r` (`f` from 0 to 1).
pub(super) fn meter(p: &Painter, r: Rect, f: f32, col: Color32) {
    let rr = r.height() / 2.0;
    p.rect_filled(r, rr, theme::blend(CARD(), TEXT(), 0.08));
    let f = f.clamp(0.0, 1.0);
    if f > 0.0 {
        p.rect_filled(Rect::from_min_size(r.min, vec2((r.width() * f).max(r.height()), r.height())), rr, col);
    }
}

/// A ring showing `f` (from 0 to 1) around `c`, `w` thick.
pub(super) fn ring(p: &Painter, c: Pos2, rad: f32, w: f32, f: f32, col: Color32) {
    p.circle_stroke(c, rad, Stroke::new(w, theme::blend(CARD(), TEXT(), 0.1)));
    let f = f.clamp(0.0, 1.0);
    if f > 0.0 {
        let n = (f * 64.0).ceil().max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|k| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * f * k as f32 / n as f32;
                c + vec2(a.cos(), a.sin()) * rad
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(w, col)));
    }
}

/// A rounded button filling `r` (its words sized to it).
pub(super) fn button(ui: &mut egui::Ui, r: Rect, label: &str, primary: bool, id: impl std::hash::Hash) -> bool {
    pill_button(ui, r, label, primary, id).clicked()
}

/// A muted line of help text that wraps in `r` (cut when it runs out).
pub(super) fn note(p: &Painter, r: Rect, text: &str, font: FontId) {
    let g = p.layout(text.to_string(), font, MUTED(), r.width().max(20.0));
    p.with_clip_rect(r).galley(r.min, g, MUTED());
}

/// A widget with nothing to show yet: its icon and a line saying how to
/// fill it, centred in the room it has.
pub(super) fn empty(p: &Painter, b: &WBox, r: Rect, icon: &str, text: &str) {
    if r.height() < b.px(20.0) {
        return;
    }
    let big = r.height() > b.px(90.0) && r.width() > b.px(140.0);
    if big {
        p.text(pos2(r.center().x, r.min.y + r.height() * 0.32), Align2::CENTER_CENTER, icon, egui::FontId::proportional(b.px(26.0)), MUTED().gamma_multiply(0.6));
        let g = p.layout(text.to_string(), b.font(12.0), MUTED(), (r.width() - b.px(12.0)).max(40.0));
        let at = pos2(r.center().x - g.size().x / 2.0, r.min.y + r.height() * 0.32 + b.px(24.0));
        p.with_clip_rect(r).galley(at, g, MUTED());
    } else {
        note(p, r, text, b.font(11.5));
    }
}

/// Places for things stacked down `r`, each `(min, max)` tall: everything
/// gets its least, the room left goes to the ones that can grow (evenly,
/// up to their most), and the lot is centred in `r`. Too little room:
/// everything shrinks alike.
pub(super) fn vflow(r: Rect, items: &[(f32, f32)], gap: f32) -> Vec<Rect> {
    let n = items.len();
    if n == 0 {
        return Vec::new();
    }
    let gaps = gap * (n as f32 - 1.0);
    let mut h: Vec<f32> = items.iter().map(|i| i.0).collect();
    let min_sum: f32 = h.iter().sum();
    let room = (r.height() - gaps).max(0.0);
    if min_sum > room {
        let k = room / min_sum.max(1.0);
        for v in h.iter_mut() {
            *v *= k;
        }
    } else {
        let mut extra = room - min_sum;
        for _ in 0..n {
            let open: Vec<usize> = (0..n).filter(|i| h[*i] < items[*i].1 - 0.01).collect();
            if open.is_empty() || extra <= 0.01 {
                break;
            }
            let share = extra / open.len() as f32;
            for i in open {
                let add = share.min(items[i].1 - h[i]);
                h[i] += add;
                extra -= add;
            }
        }
    }
    let total: f32 = h.iter().sum::<f32>() + gaps;
    let mut y = r.min.y + ((r.height() - total) / 2.0).max(0.0);
    h.iter()
        .map(|v| {
            let rr = Rect::from_min_size(pos2(r.min.x, y), vec2(r.width(), *v));
            y += v + gap;
            rr
        })
        .collect()
}
