//! Game Info › Moves: every move in a sortable, filterable table, the TMs
//! and HMs with where to find each one, the three move tutors and their
//! shard prices, and a page per move (stats, who learns it and how, how to
//! teach it, what it hits hard, and moves like it).

use super::info_home::Page;
use super::pokedex::{type_chip, type_color, EntryTab};
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::wiki::{Item, Wiki};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum MovesView {
    #[default]
    All,
    Tms,
    Tutors,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Sort {
    #[default]
    Name,
    Type,
    Cat,
    Power,
    Acc,
    Pp,
    Learners,
}

/// How a Pokémon learns a move.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, PartialOrd, Ord, Hash)]
pub(super) enum How {
    #[default]
    Level,
    Tm,
    Tutor,
    Egg,
}

impl How {
    fn label(self) -> &'static str {
        match self {
            How::Level => "By level",
            How::Tm => "TM / HM",
            How::Tutor => "Tutor",
            How::Egg => "Egg move",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Teach {
    #[default]
    Any,
    Tm,
    Tutor,
}

#[derive(Default)]
pub(super) struct MovesState {
    pub view: MovesView,
    pub query: String,
    pub typ: Option<String>,
    pub cat: Option<String>,
    pub teach: Teach,
    pub sort: Sort,
    pub desc: bool,
    /// Move page: learnsets of MODERN POKéMON worlds (None: Platinum's).
    pub modern: Option<bool>,
    pub how: How,
    index: Option<Arc<MoveIndex>>,
}

/// One move, with what the lists need.
#[derive(Clone)]
struct Row {
    key: String,
    name: String,
    typ: String,
    cat: String,
    power: u32,
    acc: u32,
    pp: u32,
    tm: Option<String>,
    tutor: bool,
    learners: usize,
}

/// Someone who learns a move.
#[derive(Clone)]
struct Learner {
    mon: String,
    name: String,
    dex: u32,
    how: How,
    /// For level-up: the level (or "Evolve").
    level: String,
    classic: bool,
    modern: bool,
}

#[derive(Default)]
pub(super) struct MoveIndex {
    key: usize,
    rows: Vec<Row>,
    learners: HashMap<String, Vec<Learner>>,
    /// move key → "TM06"
    tm_of: HashMap<String, String>,
}

fn num(s: &str) -> u32 {
    s.trim().parse().unwrap_or(0)
}

fn build(w: &Wiki) -> MoveIndex {
    let key = w as *const Wiki as usize;
    let mut tm_of = HashMap::new();
    for (tm, mv) in &w.data.tms {
        tm_of.insert(mv.clone(), tm.clone());
    }
    // who learns what: Platinum's lists, and MODERN's (its level list replaces
    // Platinum's; its TM, tutor and egg lists add to them)
    let mut learners: HashMap<String, Vec<Learner>> = HashMap::new();
    for m in &w.data.species {
        let Some(ls) = w.learnset(&m.id) else { continue };
        // (move, how) → (level, classic, modern)
        let mut got: BTreeMap<(String, How), (String, bool, bool)> = BTreeMap::new();
        let classic_ok = !m.modern;
        let mut add = |k: &str, how: How, level: &str, classic: bool, modern: bool| {
            let e = got.entry((k.to_string(), how)).or_insert((level.to_string(), false, false));
            if e.0.is_empty() || (classic && !e.1) {
                if !level.is_empty() {
                    e.0 = level.to_string();
                }
            }
            e.1 |= classic;
            e.2 |= modern;
        };
        let modern_lv = if ls.mlv.is_empty() { &ls.lv } else { &ls.mlv };
        for l in &ls.lv {
            add(&l.key, How::Level, &l.level, classic_ok, std::ptr::eq(modern_lv, &ls.lv));
        }
        if !std::ptr::eq(modern_lv, &ls.lv) {
            for l in modern_lv {
                add(&l.key, How::Level, &l.level, false, true);
            }
        }
        let tm_key = |t: &String| w.data.tms.get(t).cloned().unwrap_or_else(|| t.clone());
        for t in &ls.tm {
            add(&tm_key(t), How::Tm, "", classic_ok, true);
        }
        for t in &ls.mtm {
            add(&tm_key(t), How::Tm, "", false, true);
        }
        for (list, extra, how) in [(&ls.tutor, &ls.mtutor, How::Tutor), (&ls.egg, &ls.megg, How::Egg)] {
            for k in list {
                add(k, how, "", classic_ok, true);
            }
            for k in extra {
                add(k, how, "", false, true);
            }
        }
        for ((k, how), (level, classic, modern)) in got {
            learners.entry(k).or_default().push(Learner { mon: m.id.clone(), name: m.name.clone(), dex: m.dex, how, level, classic, modern });
        }
    }
    let mut rows: Vec<Row> = w
        .data
        .moves
        .iter()
        .map(|(k, m)| {
            let n = learners.get(k).map(|l| {
                let mut ids: Vec<&str> = l.iter().map(|x| x.mon.as_str()).collect();
                ids.sort_unstable();
                ids.dedup();
                ids.len()
            });
            Row {
                key: k.clone(),
                name: m.n.clone(),
                typ: m.t.clone(),
                cat: m.c.clone(),
                power: num(&m.p),
                acc: num(&m.a),
                pp: num(&m.pp),
                tm: tm_of.get(k).cloned(),
                tutor: w.data.tutors.contains_key(k),
                learners: n.unwrap_or(0),
            }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    MoveIndex { key, rows, learners, tm_of }
}

fn cat_color(c: &str) -> Color32 {
    match c {
        "Physical" => Color32::from_rgb(235, 110, 70),
        "Special" => Color32::from_rgb(90, 130, 240),
        _ => Color32::from_rgb(150, 150, 165),
    }
}

/// A small category mark: a burst (physical), rings (special) or a dot pair (status).
fn cat_icon(p: &egui::Painter, c: egui::Pos2, cat: &str) {
    let col = cat_color(cat);
    match cat {
        "Physical" => {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let r = if i % 2 == 0 { 6.0 } else { 3.2 };
                p.line_segment([c, c + egui::vec2(a.cos(), a.sin()) * r], Stroke::new(2.0f32, col));
            }
        }
        "Special" => {
            p.circle_stroke(c, 5.5, Stroke::new(1.6f32, col));
            p.circle_filled(c, 2.4, col);
        }
        _ => {
            p.circle_stroke(c, 5.5, Stroke::new(1.6f32, col));
            p.line_segment([c + egui::vec2(-3.0, 0.0), c + egui::vec2(3.0, 0.0)], Stroke::new(1.6f32, col));
        }
    }
}

fn cat_pill(ui: &mut egui::Ui, cat: &str) -> egui::Response {
    let col = cat_color(cat);
    let g = ui.painter().layout_no_wrap(cat.to_uppercase(), crate::fonts::sans_bold_font(10.5), col);
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(32.0, 8.0), Sense::hover());
    ui.painter().rect_filled(r, r.height() / 2.0, kit::tint(col, 0.16));
    cat_icon(ui.painter(), egui::pos2(r.min.x + 12.0, r.center().y), cat);
    ui.painter().galley(egui::pos2(r.min.x + 22.0, r.min.y + 4.0), g, col);
    resp
}

fn small(t: impl Into<String>) -> RichText {
    RichText::new(t).font(crate::fonts::sans_font(12.0)).color(MUTED())
}

fn tag(ui: &mut egui::Ui, text: &str, col: Color32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(10.5), col);
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(14.0, 6.0), Sense::hover());
    ui.painter().rect_filled(r, r.height() / 2.0, kit::tint(col, 0.16));
    ui.painter().galley(r.min + egui::vec2(7.0, 3.0), g, col);
    resp
}

fn shard_order(c: &str) -> usize {
    ["red", "blue", "yellow", "green"].iter().position(|x| *x == c).unwrap_or(9)
}

/// A shard price: coloured dots with counts.
fn shard_cost(ui: &mut egui::Ui, cost: &BTreeMap<String, u32>) {
    let mut list: Vec<(&String, &u32)> = cost.iter().collect();
    list.sort_by_key(|(c, _)| shard_order(c));
    for (c, n) in list {
        let col = match c.as_str() {
            "red" => Color32::from_rgb(232, 80, 80),
            "blue" => Color32::from_rgb(80, 140, 240),
            "yellow" => Color32::from_rgb(240, 200, 60),
            "green" => Color32::from_rgb(80, 200, 110),
            _ => MUTED(),
        };
        let g = ui.painter().layout_no_wrap(n.to_string(), crate::fonts::sans_bold_font(11.5), TEXT());
        let (r, resp) = ui.allocate_exact_size(egui::vec2(g.size().x + 26.0, 22.0), Sense::hover());
        ui.painter().rect_filled(r, 11.0, kit::tint(col, 0.14));
        // a little shard: a diamond
        let c0 = egui::pos2(r.min.x + 10.0, r.center().y);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![c0 + egui::vec2(0.0, -6.0), c0 + egui::vec2(4.5, 0.0), c0 + egui::vec2(0.0, 6.0), c0 + egui::vec2(-4.5, 0.0)],
            col,
            Stroke::NONE,
        ));
        ui.painter().galley(egui::pos2(r.min.x + 19.0, r.center().y - g.size().y / 2.0), g, TEXT());
        resp.on_hover_text(format!("{n} {c} shard{}", if *n == 1 { "" } else { "s" }));
    }
}

/// Where an item turns up, one line: "Ground · Route 212".
fn first_spot(it: &Item) -> Option<String> {
    let best = it.found.iter().find(|s| s.how != "Other").or(it.found.first())?;
    Some(format!("{} · {}", best.how, best.place))
}

fn tm_item<'a>(w: &'a Wiki, tm: &str) -> Option<&'a Item> {
    w.data.items.iter().find(|i| i.name == tm || i.id == format!("ITEM_{tm}"))
}

impl App {
    fn move_index(&mut self, w: &Arc<Wiki>) -> Arc<MoveIndex> {
        let key = Arc::as_ptr(w) as usize;
        if let Some(i) = self.setup.info.moves.index.as_ref().filter(|i| i.key == key) {
            return i.clone();
        }
        let mut i = build(w);
        i.key = key;
        let i = Arc::new(i);
        self.setup.info.moves.index = Some(i.clone());
        i
    }

    /// The Pokédex, on this Pokémon's moves.
    fn dex_moves(&mut self, id: String) {
        self.info_open_dex(Some(id));
        self.setup.pokedex.tab = EntryTab::Moves;
    }

    // ───────────────────────────── the list ─────────────────────────────

    pub(super) fn page_moves(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let idx = self.move_index(w);
        self.info_back_bar(ui, "Moves");
        let pl = theme::pal();
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let tms = w.data.tms.len();
        let tutors = w.data.tutors.len();
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 20.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            ui.label(RichText::new("MOVES").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), kit::C_VIOLET, 0.6)));
            ui.add_space(4.0);
            ui.label(RichText::new("Every move in the game").font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{} moves, {tms} TMs and HMs, {tutors} tutor moves. Pick one to see who learns it and how to teach it.", idx.rows.len())).size(13.0).color(MUTED()));
            ui.add_space(14.0);
            let mut v = self.setup.info.moves.view;
            kit::segmented(ui, &mut v, &[(MovesView::All, "All moves"), (MovesView::Tms, "TMs & HMs"), (MovesView::Tutors, "Move tutors")]);
            self.setup.info.moves.view = v;
            ui.add_space(12.0);
            let width = ui.available_width();
            kit::search_box_keyed(ui, Some(egui::Id::new("moves-search")), &mut self.setup.info.moves.query, "Filter by name, e.g. punch, beam, surf", width);
        });
        let rect = frame.response.rect;
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], theme::blend(CARD(), kit::C_VIOLET, if pl.light { 0.12 } else { 0.22 }), theme::blend(CARD(), pl.accent, if pl.light { 0.05 } else { 0.10 })),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_VIOLET, 0.35))),
            ]),
        );
        ui.add_space(16.0);
        match self.setup.info.moves.view {
            MovesView::All => self.moves_table(ui, w, &idx),
            MovesView::Tms => self.moves_tms(ui, w, &idx),
            MovesView::Tutors => self.moves_tutors(ui, w, &idx),
        }
    }

    fn moves_table(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>, idx: &MoveIndex) {
        let s = &mut self.setup.info.moves;
        // ── filters
        let types: Vec<String> = crate::dexextra::TYPES.iter().map(|t| t.to_string()).filter(|t| idx.rows.iter().any(|r| &r.typ == t)).collect();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for t in &types {
                let on = s.typ.as_deref() == Some(t.as_str());
                let c = type_color(t);
                let g = ui.painter().layout_no_wrap(t.to_uppercase(), crate::fonts::sans_bold_font(10.5), if on { Color32::WHITE } else { TEXT() });
                let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(16.0, 10.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let fill = if on { c } else { theme::blend(kit::sunken(), c, 0.22 + 0.2 * ht) };
                ui.painter().rect_filled(r, r.height() / 2.0, fill);
                if !on {
                    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, kit::tint(c, 0.55)));
                }
                ui.painter().galley(r.min + egui::vec2(8.0, 5.0), g, TEXT());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    s.typ = if on { None } else { Some(t.clone()) };
                }
            }
        });
        ui.add_space(10.0);
        let stack = ui.available_width() < 820.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
            let mut cat: &'static str = match s.cat.as_deref() {
                Some("Physical") => "Physical",
                Some("Special") => "Special",
                Some("Status") => "Status",
                _ => "",
            };
            if kit::segmented(ui, &mut cat, &[("", "Any kind"), ("Physical", "Physical"), ("Special", "Special"), ("Status", "Status")]) {
                s.cat = (!cat.is_empty()).then(|| cat.to_string());
            }
            if stack {
                ui.end_row();
            }
            let mut t = s.teach;
            if kit::segmented(ui, &mut t, &[(Teach::Any, "Any move"), (Teach::Tm, "On a TM / HM"), (Teach::Tutor, "From a tutor")]) {
                s.teach = t;
            }
        });
        ui.add_space(12.0);
        // ── rows that pass
        let q = s.query.trim().to_lowercase();
        let mut rows: Vec<&Row> = idx
            .rows
            .iter()
            .filter(|r| q.is_empty() || r.name.to_lowercase().contains(&q))
            .filter(|r| s.typ.as_ref().map_or(true, |t| &r.typ == t))
            .filter(|r| s.cat.as_ref().map_or(true, |c| &r.cat == c))
            .filter(|r| match s.teach {
                Teach::Any => true,
                Teach::Tm => r.tm.is_some(),
                Teach::Tutor => r.tutor,
            })
            .collect();
        let tord = |t: &str| crate::dexextra::TYPES.iter().position(|x| *x == t).unwrap_or(99);
        match s.sort {
            Sort::Name => rows.sort_by(|a, b| a.name.cmp(&b.name)),
            Sort::Type => rows.sort_by(|a, b| tord(&a.typ).cmp(&tord(&b.typ)).then(a.name.cmp(&b.name))),
            Sort::Cat => rows.sort_by(|a, b| a.cat.cmp(&b.cat).then(a.name.cmp(&b.name))),
            Sort::Power => rows.sort_by(|a, b| b.power.cmp(&a.power).then(a.name.cmp(&b.name))),
            Sort::Acc => rows.sort_by(|a, b| b.acc.cmp(&a.acc).then(a.name.cmp(&b.name))),
            Sort::Pp => rows.sort_by(|a, b| b.pp.cmp(&a.pp).then(a.name.cmp(&b.name))),
            Sort::Learners => rows.sort_by(|a, b| b.learners.cmp(&a.learners).then(a.name.cmp(&b.name))),
        }
        if s.desc {
            rows.reverse();
        }
        let filtered = s.typ.is_some() || s.cat.is_some() || s.teach != Teach::Any || !q.is_empty();
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} of {} moves", rows.len(), idx.rows.len())).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
            if filtered {
                ui.add_space(6.0);
                clear = ui.link(RichText::new("Clear filters").size(12.5)).clicked();
            }
        });
        if clear {
            s.typ = None;
            s.cat = None;
            s.teach = Teach::Any;
            s.query.clear();
        }
        ui.add_space(8.0);
        let mut open: Option<String> = None;
        let mut sort_to: Option<Sort> = None;
        let (cur_sort, desc) = (s.sort, s.desc);
        let most = idx.rows.iter().map(|r| r.learners).max().unwrap_or(1).max(1) as f32;
        kit::card(ui, |ui| {
            let w = ui.available_width();
            let narrow = w < 720.0;
            // columns: name, type, kind, power, acc, pp, learners
            let cols: Vec<(Sort, &str, f32, bool)> = if narrow {
                vec![(Sort::Name, "Move", 0.0, false), (Sort::Type, "Type", 92.0, false), (Sort::Power, "Power", 58.0, true), (Sort::Acc, "Acc.", 52.0, true), (Sort::Pp, "PP", 40.0, true)]
            } else {
                vec![
                    (Sort::Name, "Move", 0.0, false),
                    (Sort::Type, "Type", 104.0, false),
                    (Sort::Cat, "Kind", 104.0, false),
                    (Sort::Power, "Power", 70.0, true),
                    (Sort::Acc, "Accuracy", 84.0, true),
                    (Sort::Pp, "PP", 52.0, true),
                    (Sort::Learners, "Learned by", 104.0, true),
                ]
            };
            let fixed: f32 = cols.iter().map(|c| c.2).sum();
            let name_w = (w - fixed).max(140.0);
            let widths: Vec<f32> = cols.iter().map(|c| if c.2 == 0.0 { name_w } else { c.2 }).collect();
            // header
            let (hr, _) = ui.allocate_exact_size(egui::vec2(w, 30.0), Sense::hover());
            let mut x = hr.min.x;
            for (i, (sort, label, _, right)) in cols.iter().enumerate() {
                let cell = Rect::from_min_size(egui::pos2(x, hr.min.y), egui::vec2(widths[i], 30.0));
                let resp = ui.interact(cell, ui.id().with(("movesort", i)), Sense::click());
                let on = cur_sort == *sort;
                let col = if on || resp.hovered() { TEXT() } else { MUTED() };
                let (px, al) = if *right { (cell.max.x - 12.0, Align2::RIGHT_CENTER) } else { (cell.min.x + 12.0, Align2::LEFT_CENTER) };
                let tr = ui.painter().text(egui::pos2(px, cell.center().y), al, label.to_uppercase(), crate::fonts::sans_bold_font(10.5), col);
                if on {
                    // names A→Z and numbers high→low are the "down" order
                    let down = !desc;
                    let c = if *right { egui::pos2(tr.min.x - 8.0, cell.center().y) } else { egui::pos2(tr.max.x + 8.0, cell.center().y) };
                    let pts = if down {
                        vec![c + egui::vec2(-4.0, -2.0), c + egui::vec2(4.0, -2.0), c + egui::vec2(0.0, 3.0)]
                    } else {
                        vec![c + egui::vec2(-4.0, 2.0), c + egui::vec2(4.0, 2.0), c + egui::vec2(0.0, -3.0)]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(pts, kit::C_VIOLET, Stroke::NONE));
                }
                if resp.on_hover_text("Sort").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    sort_to = Some(*sort);
                }
                x += widths[i];
            }
            ui.painter().line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(1.0f32, LINE()));
            if rows.is_empty() {
                ui.add_space(14.0);
                ui.label(small("No moves match. Try clearing a filter."));
                ui.add_space(10.0);
            }
            for r in &rows {
                let (rr, resp) = ui.allocate_exact_size(egui::vec2(w, 42.0), Sense::click());
                if !ui.is_rect_visible(rr) {
                    continue;
                }
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(rr);
                if ht > 0.0 {
                    p.rect_filled(rr, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                p.line_segment([rr.left_bottom(), rr.right_bottom()], Stroke::new(1.0f32, kit::tint(LINE(), 0.5)));
                let mut x = rr.min.x;
                for (i, (sort, _, _, right)) in cols.iter().enumerate() {
                    let cell = Rect::from_min_size(egui::pos2(x, rr.min.y), egui::vec2(widths[i], rr.height()));
                    let cy = cell.center().y;
                    let value = |v: u32| if v == 0 { "—".to_string() } else { v.to_string() };
                    // the game lists moves whose power depends on something (Grass Knot, Return…) as 1
                    let power_txt = |v: u32| if v == 1 { "Varies".to_string() } else { value(v) };
                    let txt = |t: String| {
                        let (px, al) = if *right { (cell.max.x - 12.0, Align2::RIGHT_CENTER) } else { (cell.min.x + 12.0, Align2::LEFT_CENTER) };
                        p.text(egui::pos2(px, cy), al, t, crate::fonts::sans_font(13.0), TEXT());
                    };
                    match sort {
                        Sort::Name => {
                            let tc = type_color(&r.typ);
                            p.rect_filled(Rect::from_center_size(egui::pos2(cell.min.x + 8.0, cy), egui::vec2(4.0, 20.0)), 2.0, tc);
                            let f = crate::fonts::sans_bold_font(13.5);
                            let gal = p.layout_no_wrap(r.name.clone(), f, TEXT());
                            let nw = gal.size().x;
                            p.galley(egui::pos2(cell.min.x + 18.0, cy - gal.size().y / 2.0), gal, TEXT());
                            let mut tx = cell.min.x + 18.0 + nw + 8.0;
                            for (show, label, col) in [(r.tm.clone(), "", kit::C_GOLD), (r.tutor.then(|| "TUTOR".to_string()), "", kit::C_TEAL)] {
                                let _ = label;
                                if let Some(t) = show {
                                    let g = p.layout_no_wrap(t, crate::fonts::sans_bold_font(9.5), col);
                                    let tr = Rect::from_min_size(egui::pos2(tx, cy - 8.0), egui::vec2(g.size().x + 10.0, 16.0));
                                    if tr.max.x < cell.max.x - 4.0 {
                                        p.rect_filled(tr, 8.0, kit::tint(col, 0.16));
                                        p.galley(egui::pos2(tr.min.x + 5.0, cy - g.size().y / 2.0), g, col);
                                    }
                                    tx = tr.max.x + 5.0;
                                }
                            }
                        }
                        Sort::Type => {
                            let c = type_color(&r.typ);
                            let g = p.layout_no_wrap(r.typ.to_uppercase(), crate::fonts::sans_bold_font(10.0), Color32::WHITE);
                            let tr = Rect::from_min_size(egui::pos2(cell.min.x + 10.0, cy - 10.0), egui::vec2(g.size().x + 14.0, 20.0));
                            p.rect_filled(tr, 10.0, c);
                            let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
                            let ink = if lum > 150.0 { Color32::from_rgb(34, 34, 40) } else { Color32::WHITE };
                            p.text(tr.center(), Align2::CENTER_CENTER, r.typ.to_uppercase(), crate::fonts::sans_bold_font(10.0), ink);
                        }
                        Sort::Cat => {
                            cat_icon(&p, egui::pos2(cell.min.x + 18.0, cy), &r.cat);
                            p.text(egui::pos2(cell.min.x + 30.0, cy), Align2::LEFT_CENTER, &r.cat, crate::fonts::sans_font(12.5), MUTED());
                        }
                        Sort::Power => txt(power_txt(r.power)),
                        Sort::Acc => {
                            if r.acc == 0 && r.power > 0 {
                                p.text(egui::pos2(cell.max.x - 12.0, cy), Align2::RIGHT_CENTER, "Sure hit", crate::fonts::sans_font(12.0), MUTED());
                            } else {
                                txt(if r.acc == 0 { "—".into() } else { format!("{}%", r.acc) })
                            }
                        }
                        Sort::Pp => txt(value(r.pp)),
                        Sort::Learners => {
                            // a short bar behind the count
                            let f = r.learners as f32 / most;
                            let bar = Rect::from_min_size(egui::pos2(cell.min.x + 10.0, cy - 3.0), egui::vec2((cell.width() - 56.0) * f, 6.0));
                            p.rect_filled(bar, 3.0, kit::tint(kit::C_VIOLET, 0.45));
                            txt(r.learners.to_string());
                        }
                    }
                    x += widths[i];
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(r.key.clone());
                }
            }
        });
        if let Some(sort) = sort_to {
            let s = &mut self.setup.info.moves;
            if s.sort == sort {
                s.desc = !s.desc;
            } else {
                s.sort = sort;
                s.desc = false;
            }
        }
        if let Some(k) = open {
            self.info_push(Page::Move(k));
        }
    }

    /// TMs and HMs: number, move, and where to find it.
    fn moves_tms(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, idx: &MoveIndex) {
        let q = self.setup.info.moves.query.trim().to_lowercase();
        let mut list: Vec<(&String, &String)> = w.data.tms.iter().collect();
        // HMs after TMs
        list.sort_by_key(|(t, _)| (t.starts_with("HM"), (*t).clone()));
        let mut open: Option<String> = None;
        for (group, hm) in [("TMs", false), ("HMs", true)] {
            let items: Vec<&(&String, &String)> = list
                .iter()
                .filter(|(t, _)| t.starts_with("HM") == hm)
                .filter(|(t, k)| q.is_empty() || t.to_lowercase().contains(&q) || w.move_info(k).is_some_and(|m| m.n.to_lowercase().contains(&q)))
                .collect();
            if items.is_empty() {
                continue;
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(group).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                ui.label(small(format!("{}", items.len())));
                if hm {
                    ui.label(small("· used outside battle too, and can't be forgotten easily"));
                }
            });
            ui.add_space(10.0);
            let avail = ui.available_width();
            let cols = if avail >= 1150.0 { 4 } else if avail >= 820.0 { 3 } else if avail >= 520.0 { 2 } else { 1 };
            let gap = 12.0;
            let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in items.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (tm, key) in row {
                        let Some(m) = w.move_info(key) else { continue };
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 84.0), Sense::click());
                        if !ui.is_rect_visible(r) {
                            continue;
                        }
                        let tc = type_color(&m.t);
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter_at(r.expand(4.0));
                        p.add(kit::card_shadow().as_shape(r, 14.0));
                        p.rect_filled(r, 14.0, theme::blend(CARD(), CARD_HOVER(), ht));
                        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), tc, 0.25 + 0.5 * ht)));
                        // the disc: number on the type's colour
                        let disc = Rect::from_min_size(r.min + egui::vec2(12.0, 14.0), egui::vec2(52.0, 56.0));
                        p.rect_filled(disc, 12.0, theme::blend(CARD(), tc, 0.30));
                        p.circle_stroke(disc.center() - egui::vec2(0.0, 6.0), 13.0, Stroke::new(2.0f32, tc));
                        p.circle_filled(disc.center() - egui::vec2(0.0, 6.0), 3.5, tc);
                        p.text(egui::pos2(disc.center().x, disc.max.y - 9.0), Align2::CENTER_CENTER, tm.as_str(), crate::fonts::sans_bold_font(10.5), TEXT());
                        let x = disc.max.x + 12.0;
                        let maxw = r.max.x - x - 10.0;
                        let f = crate::fonts::sans_bold_font(14.0);
                        p.text(egui::pos2(x, r.min.y + 24.0), Align2::LEFT_CENTER, super::charts::clip(ui, &m.n, &f, maxw), f, TEXT());
                        let pw = match num(&m.p) {
                            0 => String::new(),
                            1 => " · power varies".to_string(),
                            p => format!(" · {p} power"),
                        };
                        p.text(egui::pos2(x, r.min.y + 43.0), Align2::LEFT_CENTER, format!("{} · {}{pw}", m.t, m.c), crate::fonts::sans_font(11.5), tc);
                        let where_ = tm_item(w, tm).and_then(first_spot).unwrap_or_else(|| "Not listed".into());
                        let f2 = crate::fonts::sans_font(11.5);
                        p.text(egui::pos2(x, r.min.y + 62.0), Align2::LEFT_CENTER, super::charts::clip(ui, &where_, &f2, maxw), f2, MUTED());
                        let learners = idx.rows.iter().find(|x| &x.key == *key).map(|x| x.learners).unwrap_or(0);
                        if resp.on_hover_text(format!("{} Pokémon can learn it", learners)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some((*key).clone());
                        }
                    }
                });
                ui.add_space(gap);
            }
            ui.add_space(14.0);
        }
        if let Some(k) = open {
            self.info_push(Page::Move(k));
        }
    }

    /// The tutors: where each one is, what they teach, and for how many shards.
    fn moves_tutors(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, idx: &MoveIndex) {
        let q = self.setup.info.moves.query.trim().to_lowercase();
        if let Some(t) = w.data.text.get("tutor") {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, "💡", kit::C_GOLD, false);
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(RichText::new(t).size(13.0).color(TEXT())).wrap());
                });
            });
            ui.add_space(14.0);
        }
        let mut places: BTreeMap<&str, Vec<(&String, &crate::wiki::Tutor)>> = BTreeMap::new();
        for (k, t) in &w.data.tutors {
            if q.is_empty() || w.move_info(k).is_some_and(|m| m.n.to_lowercase().contains(&q)) {
                places.entry(t.place.as_str()).or_default().push((k, t));
            }
        }
        let mut open: Option<String> = None;
        let avail = ui.available_width();
        let cols = if avail >= 1000.0 { 3.min(places.len().max(1)) } else if avail >= 640.0 { 2 } else { 1 };
        let gap = 16.0;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let list: Vec<(&str, Vec<(&String, &crate::wiki::Tutor)>)> = places.into_iter().collect();
        for row in list.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (place, moves) in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        kit::card(ui, |ui| {
                            card_head(ui, "🗺", kit::C_TEAL, place, None);
                            ui.label(small(format!("Teaches {} moves", moves.len())));
                            ui.add_space(8.0);
                            let mut moves = moves.clone();
                            moves.sort_by_key(|(k, _)| w.move_info(k).map(|m| m.n.clone()).unwrap_or_default());
                            for (k, t) in moves {
                                let Some(m) = w.move_info(k) else { continue };
                                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), Sense::click());
                                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                                let p = ui.painter_at(r.expand(2.0));
                                if ht > 0.0 {
                                    p.rect_filled(r, 10.0, theme::blend(CARD(), CARD_HOVER(), ht));
                                }
                                let tc = type_color(&m.t);
                                p.rect_filled(Rect::from_center_size(egui::pos2(r.min.x + 6.0, r.center().y), egui::vec2(4.0, 26.0)), 2.0, tc);
                                p.text(egui::pos2(r.min.x + 16.0, r.center().y - 9.0), Align2::LEFT_CENTER, &m.n, crate::fonts::sans_bold_font(13.5), TEXT());
                                p.text(egui::pos2(r.min.x + 16.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{} · {}", m.t, m.c), crate::fonts::sans_font(11.0), MUTED());
                                // the price on the right
                                let mut cost_ui = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(egui::pos2(r.center().x, r.min.y + 14.0), egui::pos2(r.max.x - 6.0, r.max.y))).layout(egui::Layout::right_to_left(egui::Align::Min)));
                                cost_ui.spacing_mut().item_spacing.x = 4.0;
                                // right to left, so the order reads red, blue, yellow, green
                                let mut items: Vec<(String, u32)> = t.cost.clone().into_iter().collect();
                                items.sort_by_key(|(c, _)| std::cmp::Reverse(shard_order(c)));
                                for (c, n) in items {
                                    let mut one = BTreeMap::new();
                                    one.insert(c, n);
                                    shard_cost(&mut cost_ui, &one);
                                }
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    open = Some(k.clone());
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(gap);
        }
        let _ = idx;
        if let Some(k) = open {
            self.info_push(Page::Move(k));
        }
    }

    // ───────────────────────────── one move ─────────────────────────────

    pub(super) fn page_move(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, key: &str) {
        let idx = self.move_index(w);
        let Some(m) = w.move_info(key).cloned() else {
            self.info_back_bar(ui, "Move");
            ui.label(small("This move isn't in the game's data any more."));
            return;
        };
        self.info_back_bar(ui, &m.n);
        let tc = type_color(&m.t);
        let pl = theme::pal();
        let tm = idx.tm_of.get(key).cloned();
        let tutor = w.data.tutors.get(key).cloned();

        // ── hero
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 22.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            let wide = full >= 860.0;
            ui.horizontal_top(|ui| {
                let text_w = if wide { (full - 56.0 - 440.0).max(300.0) } else { full - 56.0 };
                ui.allocate_ui_with_layout(egui::vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(text_w);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("MOVE").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), tc, 0.6)));
                        if let Some(t) = &tm {
                            tag(ui, t, kit::C_GOLD);
                        }
                        if let Some(t) = &tutor {
                            tag(ui, &format!("TUTOR · {}", t.place.to_uppercase()), kit::C_TEAL);
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(&m.n).font(crate::fonts::sans_bold_font(30.0)).color(TEXT()));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        type_chip(ui, &m.t, "");
                        cat_pill(ui, &m.c);
                    });
                    ui.add_space(10.0);
                    ui.add(egui::Label::new(RichText::new(&m.d).size(14.5).color(TEXT())).wrap());
                });
                if wide {
                    ui.add_space(20.0);
                    self.move_stats(ui, &m, 420.0);
                }
            });
            if !wide {
                ui.add_space(14.0);
                self.move_stats(ui, &m, full - 56.0);
            }
        });
        let rect = frame.response.rect;
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], theme::blend(CARD(), tc, if pl.light { 0.18 } else { 0.30 }), theme::blend(CARD(), tc, if pl.light { 0.04 } else { 0.08 })),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), tc, 0.4))),
            ]),
        );
        ui.add_space(18.0);

        let avail = ui.available_width();
        let gap = 18.0;
        if avail >= 1000.0 {
            let left_w = ((avail - gap) * 0.6).floor();
            let right_w = avail - gap - left_w;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    self.move_learners(ui, w, &idx, key);
                });
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    self.move_teach(ui, w, key, tm.as_deref(), tutor.as_ref());
                    ui.add_space(gap);
                    move_matchups(ui, &m);
                    ui.add_space(gap);
                    self.move_similar(ui, &idx, key);
                });
            });
        } else {
            self.move_teach(ui, w, key, tm.as_deref(), tutor.as_ref());
            ui.add_space(gap);
            self.move_learners(ui, w, &idx, key);
            ui.add_space(gap);
            move_matchups(ui, &m);
            ui.add_space(gap);
            self.move_similar(ui, &idx, key);
        }
    }

    /// Power, accuracy and PP as three tiles (PP with its most after PP Ups).
    fn move_stats(&mut self, ui: &mut egui::Ui, m: &crate::wiki::Move, width: f32) {
        let gap = 10.0;
        let tw = ((width - gap * 2.0) / 3.0).floor();
        let power = num(&m.p);
        let acc = num(&m.a);
        let pp = num(&m.pp);
        let tiles = [
            (
                "POWER",
                match power {
                    0 => "—".to_string(),
                    1 => "Varies".to_string(),
                    p => p.to_string(),
                },
                match power {
                    0 => "doesn't deal damage".to_string(),
                    1 => "depends on the battle".to_string(),
                    p if p >= 100 => "very strong".into(),
                    p if p >= 70 => "strong".into(),
                    _ => "light".into(),
                },
                if power == 1 { 0.5 } else { (power as f32 / 150.0).min(1.0) },
            ),
            (
                "ACCURACY",
                if acc == 0 && power > 0 { "Sure".to_string() } else if acc == 0 { "—".to_string() } else { format!("{acc}%") },
                if acc == 0 && power > 0 { "never misses".to_string() } else if acc == 0 { "no accuracy check".to_string() } else if acc == 100 { "hits unless dodged".into() } else { format!("misses {}%", 100 - acc) },
                if acc == 0 { 1.0 } else { acc as f32 / 100.0 },
            ),
            ("PP", pp.to_string(), format!("{} with PP Ups", pp + pp / 5 * 3), (pp as f32 / 40.0).min(1.0)),
        ];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (label, value, note, f) in tiles {
                let (r, _) = ui.allocate_exact_size(egui::vec2(tw, 92.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(r, 14.0, kit::tint(CARD(), 0.75));
                p.rect_stroke(r, 14.0, Stroke::new(1.0f32, kit::tint(LINE(), 0.8)));
                p.text(r.min + egui::vec2(14.0, 18.0), Align2::LEFT_CENTER, label, crate::fonts::sans_bold_font(10.0), MUTED());
                p.text(r.min + egui::vec2(14.0, 44.0), Align2::LEFT_CENTER, value.clone(), crate::fonts::sans_bold_font(24.0), TEXT());
                let bar = Rect::from_min_size(egui::pos2(r.min.x + 14.0, r.max.y - 26.0), egui::vec2(r.width() - 28.0, 4.0));
                p.rect_filled(bar, 2.0, kit::tint(LINE(), 0.9));
                let gf = kit::grow(ui.ctx(), egui::Id::new(("move-stat", label, value.clone())), f);
                p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * gf, 4.0)), 2.0, kit::C_VIOLET);
                let f2 = crate::fonts::sans_font(10.5);
                p.text(egui::pos2(r.min.x + 14.0, r.max.y - 12.0), Align2::LEFT_CENTER, super::charts::clip(ui, &note, &f2, r.width() - 28.0), f2, MUTED());
            }
        });
    }

    /// Who learns it: by level, TM, tutor or Egg; Platinum or MODERN worlds.
    fn move_learners(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>, idx: &MoveIndex, key: &str) {
        let all = idx.learners.get(key).cloned().unwrap_or_default();
        let differs = all.iter().any(|l| l.classic != l.modern);
        // a newer move nobody learns in Platinum: show the MODERN lists
        let modern_only = !all.is_empty() && !all.iter().any(|l| l.classic);
        let modern = modern_only || (self.setup.info.moves.modern.unwrap_or(false) && differs);
        let list: Vec<&Learner> = all.iter().filter(|l| if modern { l.modern } else { l.classic }).collect();
        let mut counts: BTreeMap<How, usize> = BTreeMap::new();
        for l in &list {
            *counts.entry(l.how).or_default() += 1;
        }
        // open on a method that has someone
        let mut how = self.setup.info.moves.how;
        if !counts.contains_key(&how) {
            if let Some((h, _)) = counts.iter().next() {
                how = *h;
            }
        }
        let mut open: Option<String> = None;
        let shinies = self.dex_caught();
        kit::card(ui, |ui| {
            let mut ids: Vec<&str> = list.iter().map(|l| l.mon.as_str()).collect();
            ids.sort_unstable();
            ids.dedup();
            ui.horizontal(|ui| {
                card_head(ui, "👥", kit::C_PINK, "Who learns it", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(small(format!("{} Pokémon", ids.len())));
                });
            });
            ui.add_space(10.0);
            if modern_only {
                ui.horizontal(|ui| {
                    tag(ui, "MODERN", kit::C_INDIGO);
                    ui.label(small("Only learned in worlds with MODERN POKéMON on"));
                });
                ui.add_space(8.0);
            } else if differs {
                let mut mm = modern;
                if kit::segmented(ui, &mut mm, &[(false, "Platinum worlds"), (true, "MODERN POKéMON worlds")]) {
                    self.setup.info.moves.modern = Some(mm);
                }
                ui.add_space(8.0);
            }
            if list.is_empty() {
                ui.label(small("Nothing learns it in these worlds."));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for h in [How::Level, How::Tm, How::Tutor, How::Egg] {
                    let Some(n) = counts.get(&h) else { continue };
                    let on = how == h;
                    let text = format!("{}  {n}", h.label());
                    let g = ui.painter().layout_no_wrap(text, crate::fonts::sans_bold_font(12.0), TEXT());
                    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 12.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let fill = if on { theme::blend(CARD(), kit::C_PINK, 0.22) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
                    ui.painter().rect_filled(r, r.height() / 2.0, fill);
                    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), kit::C_PINK, 0.7) } else { LINE() }));
                    ui.painter().galley(r.min + egui::vec2(10.0, 6.0), g, TEXT());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.setup.info.moves.how = h;
                    }
                }
            });
            ui.add_space(12.0);
            let mut shown: Vec<&&Learner> = list.iter().filter(|l| l.how == how).collect();
            if how == How::Level {
                let lv = |l: &Learner| l.level.parse::<i32>().unwrap_or(if l.level.to_lowercase().starts_with("evo") { 0 } else { 999 });
                shown.sort_by_key(|l| (lv(l), l.dex));
            } else {
                shown.sort_by_key(|l| l.dex);
            }
            let avail = ui.available_width();
            let tw = 128.0;
            let cols = ((avail + 8.0) / (tw + 8.0)).floor().max(1.0) as usize;
            let tw = ((avail - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in shown.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for l in row {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(tw, 92.0), Sense::click());
                        if !ui.is_rect_visible(r) {
                            continue;
                        }
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter_at(r.expand(2.0));
                        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_PINK, 0.5 * ht)));
                        let sc = egui::pos2(r.center().x, r.min.y + 34.0);
                        let sp = self.info_species(&l.name);
                        let shiny = sp.as_ref().is_some_and(|s| shinies.contains_key(&s.slug));
                        let drew = match sp {
                            Some(sp) if self.sprites.get(&sp, shiny).is_some() => self.draw_sprite_centered(&p, &sp, shiny, sc, egui::vec2(52.0, 52.0)),
                            _ => false,
                        };
                        if shiny {
                            theme::sparkle(&p, r.min + egui::vec2(14.0, 14.0), 5.0, theme::GOLD);
                        }
                        if !drew {
                            p.circle_filled(sc, 18.0, kit::tint(kit::C_PINK, 0.15));
                        }
                        let f = crate::fonts::sans_bold_font(12.5);
                        p.text(egui::pos2(r.center().x, r.max.y - 26.0), Align2::CENTER_CENTER, super::charts::clip(ui, &l.name, &f, tw - 12.0), f, TEXT());
                        let sub = match how {
                            How::Level => match l.level.as_str() {
                                "" => "—".to_string(),
                                lv if lv.chars().all(|c| c.is_ascii_digit()) => format!("Lv {lv}"),
                                other => other.to_string(),
                            },
                            _ => format!("#{:04}", l.dex),
                        };
                        p.text(egui::pos2(r.center().x, r.max.y - 11.0), Align2::CENTER_CENTER, sub, crate::fonts::sans_font(11.0), if how == How::Level { kit::C_PINK } else { MUTED() });
                        if !modern && l.classic && !l.modern {
                            p.text(r.right_top() + egui::vec2(-8.0, 10.0), Align2::RIGHT_CENTER, "PLAT", crate::fonts::sans_bold_font(8.5), MUTED());
                        }
                        let tip = if shiny { "You have this one shiny. Open its moves in the Pokédex" } else { "Open its moves in the Pokédex" };
                        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(l.mon.clone());
                        }
                    }
                });
                ui.add_space(8.0);
            }
        });
        self.setup.info.moves.how = how;
        if let Some(id) = open {
            self.dex_moves(id);
        }
    }

    /// How to teach it: the TM (and where to get it) or the tutor (and the price).
    fn move_teach(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, _key: &str, tm: Option<&str>, tutor: Option<&crate::wiki::Tutor>) {
        let mut show_item: Option<String> = None;
        let mut open_place: Option<String> = None;
        kit::card(ui, |ui| {
            card_head(ui, "🎓", kit::C_GOLD, "How to teach it", None);
            ui.add_space(10.0);
            if tm.is_none() && tutor.is_none() {
                ui.label(RichText::new("Only by levelling up or breeding").font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                ui.add_space(2.0);
                ui.label(small("No TM or tutor teaches it. See who learns it, and at what level, under Who learns it."));
                return;
            }
            if let Some(t) = tm {
                let item = tm_item(w, t);
                ui.horizontal(|ui| {
                    tag(ui, t, kit::C_GOLD);
                    ui.label(RichText::new(if t.starts_with("HM") { "Hidden Machine" } else { "Technical Machine" }).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                    if let Some(it) = item {
                        if let Ok(p) = it.price.parse::<u32>() {
                            if p > 0 {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(small(format!("₽{}", fmt_n(p))));
                                });
                            }
                        }
                    }
                });
                ui.add_space(8.0);
                match item {
                    Some(it) if !it.found.is_empty() => {
                        for s in it.found.iter().take(5) {
                            spot_row(ui, &s.how, &s.place, &s.detail);
                        }
                        if it.found.len() > 5 {
                            ui.label(small(format!("+ {} more", it.found.len() - 5)));
                        }
                        ui.add_space(4.0);
                        if ui.link(RichText::new(format!("All about {t}  ›")).size(12.5)).clicked() {
                            show_item = Some(it.id.clone());
                        }
                    }
                    _ => {
                        ui.label(small("The game's data doesn't list where to find it."));
                    }
                }
                if tutor.is_some() {
                    ui.add_space(12.0);
                }
            }
            if let Some(t) = tutor {
                ui.horizontal(|ui| {
                    tag(ui, "TUTOR", kit::C_TEAL);
                    match super::info_places::group_of(w, &t.place) {
                        Some(g) => {
                            if ui.link(RichText::new(&t.place).font(crate::fonts::sans_bold_font(13.5))).on_hover_text("Open this place").clicked() {
                                open_place = Some(g);
                            }
                        }
                        None => {
                            ui.label(RichText::new(&t.place).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(small("Costs"));
                    shard_cost(ui, &t.cost);
                });
            }
        });
        if let Some(id) = show_item {
            self.info_push(Page::Item(id));
        }
        if let Some(p) = open_place {
            self.info_push(Page::Place(p));
        }
    }

    /// Moves of the same type and kind, closest in power.
    fn move_similar(&mut self, ui: &mut egui::Ui, idx: &MoveIndex, key: &str) {
        let Some(me) = idx.rows.iter().find(|r| r.key == key) else { return };
        let mut like: Vec<&Row> = idx.rows.iter().filter(|r| r.key != key && r.typ == me.typ && r.cat == me.cat).collect();
        like.sort_by_key(|r| ((r.power as i64 - me.power as i64).abs(), r.name.clone()));
        if like.is_empty() {
            return;
        }
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            card_head(ui, "🔀", kit::C_VIOLET, "Moves like it", None);
            ui.label(small(format!("{} {} moves, closest in power", me.typ, me.cat.to_lowercase())));
            ui.add_space(8.0);
            for r in like.iter().take(6) {
                let (rr, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(rr.expand(2.0));
                if ht > 0.0 {
                    p.rect_filled(rr, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                cat_icon(&p, egui::pos2(rr.min.x + 12.0, rr.center().y), &r.cat);
                p.text(egui::pos2(rr.min.x + 28.0, rr.center().y), Align2::LEFT_CENTER, &r.name, crate::fonts::sans_bold_font(13.0), TEXT());
                let stats = format!(
                    "{}  ·  {}  ·  {} PP",
                    match r.power {
                        0 => "—".to_string(),
                        1 => "varies".to_string(),
                        p => format!("{p} power"),
                    },
                    if r.acc == 0 { "—".to_string() } else { format!("{}%", r.acc) },
                    r.pp
                );
                p.text(egui::pos2(rr.max.x - 8.0, rr.center().y), Align2::RIGHT_CENTER, stats, crate::fonts::sans_font(11.5), MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(r.key.clone());
                }
            }
        });
        if let Some(k) = open {
            self.info_push(Page::Move(k));
        }
    }
}

/// Super effective, not very effective, no effect: for damaging moves.
fn move_matchups(ui: &mut egui::Ui, m: &crate::wiki::Move) {
    if m.c == "Status" || num(&m.p) == 0 {
        return;
    }
    let mut strong = Vec::new();
    let mut weak = Vec::new();
    let mut none = Vec::new();
    for t in crate::dexextra::TYPES {
        let fx = crate::dexextra::defenses(&[t.to_string()]);
        let x = fx.iter().find(|(a, _)| *a == m.t.as_str()).map(|x| x.1).unwrap_or(100);
        match x {
            0 => none.push(t),
            x if x > 100 => strong.push(t),
            x if x < 100 => weak.push(t),
            _ => {}
        }
    }
    if strong.is_empty() && weak.is_empty() && none.is_empty() {
        return;
    }
    kit::card(ui, |ui| {
        card_head(ui, "🎯", kit::C_RED, "What it hits", None);
        ui.add_space(8.0);
        for (label, list, col) in [("Super effective ×2", &strong, kit::C_GREEN), ("Not very effective ×½", &weak, kit::C_ORANGE), ("No effect ×0", &none, kit::C_SLATE)] {
            if list.is_empty() {
                continue;
            }
            ui.label(RichText::new(label.to_uppercase()).font(crate::fonts::sans_bold_font(10.0)).color(col));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                for t in list.iter() {
                    type_chip(ui, t, "");
                }
            });
            ui.add_space(10.0);
        }
    });
}

fn spot_row(ui: &mut egui::Ui, how: &str, place: &str, detail: &str) {
    let col = match how {
        "Ground" => kit::C_GREEN,
        "Hidden" => kit::C_VIOLET,
        "Gift" => kit::C_PINK,
        "Shop" => kit::C_GOLD,
        "Held" => kit::C_CYAN,
        "Prize" => kit::C_ORANGE,
        _ => kit::C_SLATE,
    };
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(70.0, 18.0), Sense::hover());
        let f = crate::fonts::sans_bold_font(9.5);
        let label = super::charts::clip(ui, &how.to_uppercase(), &f, 62.0);
        let g = ui.painter().layout_no_wrap(label, f, col);
        let pr = Rect::from_min_size(r.min + egui::vec2(0.0, 1.0), egui::vec2(g.size().x + 12.0, 18.0));
        ui.painter().rect_filled(pr, 9.0, kit::tint(col, 0.16));
        ui.painter().galley(pr.min + egui::vec2(6.0, 9.0 - g.size().y / 2.0), g, col);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add(egui::Label::new(RichText::new(place).size(12.5).color(TEXT())).wrap());
            if !detail.is_empty() {
                ui.add(egui::Label::new(small(detail)).wrap());
            }
        });
    });
    ui.add_space(6.0);
}

fn fmt_n(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_from_real_data() {
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = Wiki::parse(&bytes).expect("parses");
        let i = build(&w);
        assert_eq!(i.rows.len(), w.data.moves.len());
        // Surf: an HM, learned by plenty, Water
        let surf = i.rows.iter().find(|r| r.key == "surf").expect("surf");
        assert_eq!(surf.tm.as_deref(), Some("HM03"));
        assert!(surf.learners > 50);
        // Bulbasaur learns Tackle at level 1 in Platinum
        let tackle = i.learners.get("tackle").expect("tackle learners");
        assert!(tackle.iter().any(|l| l.mon == "BULBASAUR" && l.how == How::Level && l.classic));
        // every TM in the list maps to a TM item
        for tm in w.data.tms.keys() {
            assert!(tm_item(&w, tm).is_some(), "{tm}");
        }
    }

    #[test]
    fn numbers() {
        assert_eq!(fmt_n(3000), "3,000");
        assert_eq!(num(" 40 "), 40);
        assert_eq!(num("—"), 0);
    }
}
