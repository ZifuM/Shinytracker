//! Ready-made fluid widgets most Dashboard widgets are built from: a list
//! (rows that fill the box, in columns when it's wide, a strip of cards
//! when it's flat), a set of numbers (boxes that fill it, like Today at a
//! glance), and a grid of pictures (tiles as big as the box allows). Each
//! takes the widget's box and lays itself out for its shape.

use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_fit::{self as fit, Shape, WBox};
use super::App;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect};

/// What goes at the front of a row or in a tile.
#[derive(Clone, Debug, Default)]
pub(super) enum Lead {
    #[default]
    None,
    /// A Pokémon by its slug (the game's sprites).
    Slug(String),
    /// A Pokémon from the species list, shiny or not.
    Species(usize, bool),
    /// A Pokémon by its wiki id.
    Wiki(String, bool),
    /// An emoji in a coloured chip.
    Icon(String, Color32),
    /// A word in a coloured pill ("Won", "Gold").
    Badge(String, Color32),
}

/// One row of a list.
#[derive(Clone, Debug, Default)]
pub(super) struct Row {
    pub lead: Lead,
    pub title: String,
    pub sub: String,
    pub right: String,
    pub right_col: Option<Color32>,
    /// A progress bar under the words (0 to 1).
    pub bar: Option<(f32, Color32)>,
    pub tip: String,
    /// A right-click menu with one button (say, "Remove").
    pub menu: Option<String>,
}

impl Row {
    pub fn new(title: impl Into<String>) -> Row {
        Row { title: title.into(), ..Default::default() }
    }
    pub fn lead(mut self, l: Lead) -> Row {
        self.lead = l;
        self
    }
    pub fn sub(mut self, s: impl Into<String>) -> Row {
        self.sub = s.into();
        self
    }
    pub fn right(mut self, s: impl Into<String>, col: Option<Color32>) -> Row {
        self.right = s.into();
        self.right_col = col;
        self
    }
    pub fn bar(mut self, f: f32, col: Color32) -> Row {
        self.bar = Some((f, col));
        self
    }
    pub fn tip(mut self, s: impl Into<String>) -> Row {
        self.tip = s.into();
        self
    }
    pub fn menu(mut self, s: impl Into<String>) -> Row {
        self.menu = Some(s.into());
        self
    }
}

/// A widget's heading.
#[derive(Clone, Copy)]
pub(super) struct Head<'a> {
    pub icon: &'a str,
    pub col: Color32,
    pub title: &'a str,
    pub link: Option<&'a str>,
}

/// What was clicked.
#[derive(Default, Debug)]
pub(super) struct Clicked {
    /// The heading's link.
    pub link: bool,
    /// A row or tile (by its place in the list).
    pub item: Option<usize>,
    /// "+N more".
    pub more: bool,
    /// A row's right-click menu button.
    pub menu: Option<usize>,
}

/// One number in a set of them.
#[derive(Clone, Debug)]
pub(super) struct Stat {
    pub value: String,
    pub label: String,
    pub col: Color32,
    /// A bar under it (0 to 1).
    pub frac: Option<f32>,
}

impl Stat {
    pub fn new(value: impl Into<String>, label: impl Into<String>, col: Color32) -> Stat {
        Stat { value: value.into(), label: label.into(), col, frac: None }
    }
    #[allow(dead_code)]
    pub fn frac(mut self, f: f32) -> Stat {
        self.frac = Some(f);
        self
    }
}

/// One picture in a grid.
#[derive(Clone, Debug, Default)]
pub(super) struct Pic {
    pub lead: Lead,
    pub name: String,
    pub sub: String,
    pub tip: String,
    /// Faded (not got yet).
    pub dim: bool,
}

impl App {
    /// Draw `lead` filling `r` (square in the middle of it).
    pub(super) fn draw_lead(&mut self, p: &egui::Painter, b: &WBox, lead: &Lead, r: Rect) {
        let side = r.width().min(r.height());
        let c = r.center();
        match lead {
            Lead::None => {}
            Lead::Slug(slug) => self.rare_sprite(p, c, side * 0.46, slug, false),
            Lead::Species(i, shiny) => {
                if let Some(sp) = self.species.get(*i).cloned() {
                    p.circle_filled(c, side * 0.48, theme::blend(kit::sunken(), Color32::WHITE, 0.05));
                    self.draw_sprite_centered(p, &sp, *shiny, c, vec2(side, side));
                }
            }
            Lead::Wiki(id, shiny) => {
                p.circle_filled(c, side * 0.48, theme::blend(kit::sunken(), Color32::WHITE, 0.05));
                if !self.team_sprite(p, id, *shiny, c, vec2(side, side)) {
                    let short: String = id.chars().filter(|c| c.is_alphabetic()).take(3).collect::<String>().to_uppercase();
                    p.text(c, Align2::CENTER_CENTER, short, crate::fonts::sans_bold_font((side * 0.3).max(8.0)), MUTED());
                }
            }
            Lead::Icon(icon, col) => {
                let s = Rect::from_center_size(c, vec2(side, side) * 0.86);
                kit::icon_chip(p, s, icon, *col, false);
            }
            Lead::Badge(text, col) => {
                let g = fit::fit(p, text, b.bold(10.5), 0.8, r.width() - b.px(10.0), Color32::WHITE);
                let pill = Rect::from_center_size(c, vec2((g.size().x + b.px(12.0)).min(r.width()), (g.size().y + b.px(6.0)).min(r.height())));
                p.rect_filled(pill, pill.height() / 2.0, *col);
                p.galley(pill.center() - g.size() / 2.0, g, Color32::WHITE);
            }
        }
    }

    /// One list row drawn in `r` (hover and click are the caller's).
    fn draw_row(&mut self, p: &egui::Painter, b: &WBox, row: &Row, r: Rect) {
        let ir = r.shrink2(vec2(b.px(6.0), b.px(3.0)));
        let has_lead = !matches!(row.lead, Lead::None);
        let lead_w = match row.lead {
            Lead::Badge(..) => b.px(58.0).min(ir.width() * 0.3),
            _ => ir.height().min(b.px(56.0)),
        };
        let (lead_r, rest) = if has_lead { fit::split_left(ir, lead_w, b.px(8.0)) } else { (Rect::NOTHING, ir) };
        if has_lead {
            self.draw_lead(p, b, &row.lead, lead_r);
        }
        // the value on the right
        let rest = if row.right.is_empty() {
            rest
        } else {
            let g = fit::fit(p, &row.right, b.bold(12.0), 0.85, rest.width() * 0.45, row.right_col.unwrap_or(TEXT()));
            let rr = Rect::from_min_max(pos2(rest.max.x - g.size().x, rest.min.y), rest.max);
            let at = Align2::RIGHT_CENTER.align_size_within_rect(g.size(), rr);
            p.galley(at.min, g, row.right_col.unwrap_or(TEXT()));
            Rect::from_min_max(rest.min, pos2(rr.min.x - b.px(8.0), rest.max.y))
        };
        // the words (and the bar under them), as one group in the middle
        let mut parts: Vec<(f32, f32)> = vec![(b.px(15.0), b.px(17.0))];
        if !row.sub.is_empty() && rest.height() > b.px(30.0) {
            parts.push((b.px(13.0), b.px(15.0)));
        }
        let with_bar = row.bar.is_some() && rest.height() > b.px(28.0);
        if with_bar {
            parts.push((b.px(5.0), b.px(6.0)));
        }
        let fl = fit::vflow(rest, &parts, b.px(2.0));
        fit::label(p, fl[0], &row.title, b.bold(12.5), TEXT(), Align2::LEFT_CENTER);
        let mut k = 1;
        if !row.sub.is_empty() && rest.height() > b.px(30.0) {
            fit::label(p, fl[k], &row.sub, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
            k += 1;
        }
        if let (true, Some((f, col))) = (with_bar, row.bar) {
            fit::meter(p, fl[k], f, col);
        }
    }

    /// A list widget: rows that fill the box (in columns when it's wide, as
    /// a strip of cards when it's flat, the first one big in a small
    /// square), and "+N more" when they don't all fit.
    pub(super) fn f_list(&mut self, ui: &mut egui::Ui, b: &WBox, h: Head, rows: &[Row], empty: &str) -> Clicked {
        let mut out = Clicked::default();
        let id = (h.title, b.r.min.x as i32, b.r.min.y as i32);
        let mini = b.shape() == Shape::Mini;
        let (body, link) = if mini { (b.inner(), false) } else { fit::head(ui, b, h.icon, h.col, h.title, h.link) };
        out.link = link;
        let p = ui.painter_at(b.r);
        if rows.is_empty() {
            fit::empty(&p, b, body, h.icon, empty);
            if mini {
                out.link = ui.interact(b.r, ui.id().with(("list-open", id)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            }
            return out;
        }
        let gap = b.px(6.0);
        if mini {
            // the first one, big
            let row = &rows[0];
            let resp = fit::hit(ui, body, ("list-mini", id), b.px(10.0));
            let has_lead = !matches!(row.lead, Lead::None);
            let (lead_r, text_r) = if has_lead { fit::split_top(body, body.height() * 0.52, b.px(4.0)) } else { (Rect::NOTHING, body) };
            if has_lead {
                self.draw_lead(&p, b, &row.lead, lead_r);
            }
            let mut parts = vec![(b.px(15.0), b.px(18.0))];
            if !row.right.is_empty() {
                parts.push((b.px(16.0), b.px(26.0)));
            } else if !row.sub.is_empty() {
                parts.push((b.px(13.0), b.px(15.0)));
            }
            let fl = fit::vflow(text_r, &parts, b.px(2.0));
            fit::label(&p, fl[0], &row.title, b.bold(12.5), TEXT(), Align2::CENTER_CENTER);
            if parts.len() > 1 {
                if !row.right.is_empty() {
                    fit::hero(&p, fl[1], &row.right, fl[1].height(), row.right_col.unwrap_or(TEXT()), Align2::CENTER_CENTER);
                } else {
                    fit::label(&p, fl[1], &row.sub, b.font(11.0), MUTED(), Align2::CENTER_CENTER);
                }
            }
            if rows.len() > 1 {
                p.text(b.r.right_top() + vec2(-b.px(8.0), b.px(8.0)), Align2::RIGHT_TOP, format!("+{}", rows.len() - 1), b.bold(10.5), MUTED());
            }
            let tip = if row.tip.is_empty() { row.title.clone() } else { row.tip.clone() };
            if resp.on_hover_text(tip).clicked() {
                out.item = Some(0);
            }
            return out;
        }
        // the places for the rows
        let slots: Vec<Rect> = if b.flat() {
            // a strip of little cards
            fit::tiles(body, rows.len(), 3.2, b.px(150.0), body.height() * 0.95, b.px(8.0))
        } else {
            let row_h = if rows.iter().any(|r| !r.sub.is_empty() || r.bar.is_some()) { b.px(42.0) } else { b.px(32.0) };
            let per_col = (((body.height() + gap) / (row_h + gap)).floor() as usize).max(1);
            let max_cols = (((body.width() + b.px(14.0)) / (b.px(240.0) + b.px(14.0))).floor() as usize).clamp(1, 3);
            let ncol = rows.len().div_ceil(per_col).clamp(1, max_cols);
            let colw = fit::cols(body, ncol, b.px(14.0));
            let mut v = Vec::new();
            let per = rows.len().div_ceil(ncol).min(per_col);
            for c in colw {
                v.extend(fit::rows(c, per, row_h, gap, 2.2));
            }
            v
        };
        // (never more places than rows)
        let mut slots = slots;
        slots.truncate(rows.len());
        let more = rows.len().saturating_sub(slots.len());
        let shown = if more > 0 { slots.len().saturating_sub(1) } else { slots.len() };
        for (k, r) in slots.iter().take(shown).enumerate() {
            let row = &rows[k];
            let resp = fit::hit(ui, *r, ("list-row", k, id), b.px(10.0));
            if b.flat() {
                fit::tile(&p, *r, h.col, b.px(10.0));
            }
            let rp = ui.painter_at(*r);
            self.draw_row(&rp, b, row, *r);
            let tip = if row.tip.is_empty() { String::new() } else { row.tip.clone() };
            let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
            if resp.clicked() {
                out.item = Some(k);
            }
            if let Some(m) = &row.menu {
                resp.context_menu(|ui| {
                    if ui.button(m).clicked() {
                        out.menu = Some(k);
                        ui.close_menu();
                    }
                });
            }
        }
        if more > 0 {
            if let Some(r) = slots.last() {
                let resp = fit::hit(ui, *r, ("list-more", id), b.px(10.0));
                fit::label(&p, r.shrink(b.px(6.0)), &format!("+{} more ›", more + 1), b.bold(12.0), MUTED(), if b.flat() { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER });
                if resp.clicked() {
                    out.more = true;
                }
            }
        }
        out
    }

    /// A set of numbers in boxes that fill the widget (the most that fit,
    /// in the layout closest to wide little boxes), and a note under them
    /// when there's room.
    pub(super) fn f_stats(&mut self, ui: &mut egui::Ui, b: &WBox, h: Head, stats: &[Stat], note: Option<&str>) -> bool {
        let id = (h.title, b.r.min.x as i32, b.r.min.y as i32);
        let mini = b.shape() == Shape::Mini;
        let (body, mut link) = if mini { (b.inner(), false) } else { fit::head(ui, b, h.icon, h.col, h.title, h.link) };
        let p = ui.painter_at(b.r);
        let gap = b.px(8.0);
        let (body, note_r) = match note {
            Some(_) if body.height() > b.px(120.0) => fit::split_bottom(body, b.px(15.0), b.px(6.0)),
            _ => (body, Rect::NOTHING),
        };
        // how many boxes, and how
        let n = stats.len().max(1);
        let (min_w, min_h) = (b.px(70.0), b.px(40.0));
        let mut pick: Option<(usize, usize, usize)> = None;
        for k in (1..=n).rev() {
            let mut best: Option<(f32, usize, usize)> = None;
            for c in 1..=k {
                let r = k.div_ceil(c);
                let bw = (body.width() - gap * (c as f32 - 1.0)) / c as f32;
                let bh = (body.height() - gap * (r as f32 - 1.0)) / r as f32;
                if bw < min_w || bh < min_h {
                    continue;
                }
                let score = ((bw / bh) / 1.8).ln().abs() + (c * r - k) as f32 * 0.5;
                if best.is_none_or(|x| score < x.0) {
                    best = Some((score, c, r));
                }
            }
            if let Some((_, c, r)) = best {
                pick = Some((k, c, r));
                break;
            }
        }
        let (k, c, r) = pick.unwrap_or((1, 1, 1));
        let bw = (body.width() - gap * (c as f32 - 1.0)) / c as f32;
        let bh = (body.height() - gap * (r as f32 - 1.0)) / r as f32;
        for (i, s) in stats.iter().take(k).enumerate() {
            let rr = Rect::from_min_size(body.min + vec2((i % c) as f32 * (bw + gap), (i / c) as f32 * (bh + gap)), vec2(bw, bh));
            fit::tile(&p, rr, s.col, b.px(10.0));
            let ir = rr.shrink2(vec2(b.px(10.0), b.px(6.0)));
            let mut parts = vec![(b.px(16.0), ir.height())];
            if ir.height() > b.px(36.0) {
                parts.push((b.px(13.0), b.px(14.0)));
            }
            if s.frac.is_some() && ir.height() > b.px(48.0) {
                parts.push((b.px(5.0), b.px(6.0)));
            }
            let fl = fit::vflow(ir, &parts, b.px(3.0));
            fit::hero(&p, fl[0], &s.value, fl[0].height().min(b.px(72.0)), TEXT(), Align2::LEFT_CENTER);
            if parts.len() > 1 {
                fit::label(&p, fl[1], &s.label, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
            }
            if let (Some(f), true) = (s.frac, parts.len() > 2) {
                fit::meter(&p, fl[2], f, s.col);
            }
        }
        if let (Some(t), true) = (note, note_r.is_positive()) {
            fit::label(&p, note_r, t, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if mini && ui.interact(b.r, ui.id().with(("stats-open", id)), egui::Sense::click()).on_hover_text(h.title).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            link = true;
        }
        link
    }

    /// A grid of pictures as big as the box allows (the most that fit, at
    /// least `min` across), each with its name (and a line under it) when
    /// there's room.
    pub(super) fn f_pics(&mut self, ui: &mut egui::Ui, b: &WBox, h: Head, pics: &[Pic], empty: &str, min: f32) -> Clicked {
        let mut out = Clicked::default();
        let id = (h.title, b.r.min.x as i32, b.r.min.y as i32);
        let mini = b.shape() == Shape::Mini && pics.len() <= 1;
        let (body, link) = if mini { (b.inner(), false) } else { fit::head(ui, b, h.icon, h.col, h.title, h.link) };
        out.link = link;
        let p = ui.painter_at(b.r);
        if pics.is_empty() {
            fit::empty(&p, b, body, h.icon, empty);
            return out;
        }
        let cells = fit::tiles(body, pics.len(), 0.85, b.px(min), b.px(min), b.px(8.0));
        for (k, r) in cells.iter().enumerate() {
            let pic = &pics[k];
            let resp = fit::hit(ui, *r, ("pic", k, id), b.px(12.0));
            let tp = ui.painter_at(*r);
            fit::tile(&tp, *r, h.col, b.px(12.0));
            let with_name = r.height() >= b.px(84.0) && !pic.name.is_empty();
            let with_sub = r.height() >= b.px(104.0) && !pic.sub.is_empty();
            let text_h = if with_sub { b.px(31.0) } else if with_name { b.px(17.0) } else { 0.0 };
            let (pic_r, text_r) = fit::split_bottom(r.shrink(b.px(6.0)), text_h, 0.0);
            self.draw_lead(&tp, b, &pic.lead, pic_r);
            if pic.dim {
                tp.rect_filled(*r, b.px(12.0), Color32::from_black_alpha(110));
            }
            if with_name {
                let (n_r, s_r) = if with_sub { fit::split_top(text_r, text_h * 0.55, 0.0) } else { (text_r, Rect::NOTHING) };
                fit::label(&tp, n_r, &pic.name, b.bold(11.5), TEXT(), Align2::CENTER_CENTER);
                if with_sub {
                    fit::label(&tp, s_r, &pic.sub, b.font(10.5), MUTED(), Align2::CENTER_CENTER);
                }
            }
            let tip = if pic.tip.is_empty() { pic.name.clone() } else { pic.tip.clone() };
            if resp.on_hover_text(tip).clicked() {
                out.item = Some(k);
            }
        }
        if cells.len() < pics.len() {
            out.more = false;
        }
        out
    }
}
