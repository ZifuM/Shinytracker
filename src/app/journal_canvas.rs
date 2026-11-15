//! The Journal's canvas: an endless board of cards (text, notes, pictures,
//! live widgets, snippets from Game Info) in groups, joined by arrows.
//!
//! Drag the board to move around, scroll to pan, Ctrl+scroll to zoom.
//! Drag a card to move it, its corners to resize it, the dots on its sides
//! to draw an arrow. Double-click the board for a new card, a card to edit
//! it. Saved as a `.canvas` file (JSON Canvas, as Obsidian), a moment after
//! each change.

use super::journal_add::How;
use super::journal_app::{Open, JOURNAL_COL};
use super::journal_md::{self as md, Clicked, MdStyle};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::jcanvas::{self, Canvas, Node, Side};
use crate::journal as j;
use crate::theme;
use eframe::egui::{self, epaint::CubicBezierShape, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::time::{Duration, Instant};

/// A widget card's grid: squares of this many points, this far apart.
pub(super) const CELL: f32 = 100.0;
pub(super) const CGAP: f32 = 12.0;

/// A widget card's size for a size in squares.
pub(super) fn widget_size(cols: i32, rows: i32) -> (f32, f32) {
    (cols as f32 * (CELL + CGAP) - CGAP, rows as f32 * (CELL + CGAP) - CGAP)
}

/// A picture card's size: the picture's own shape, about 400 wide.
pub(super) fn image_size(rel: &str) -> (f32, f32) {
    let (w, h) = image::image_dimensions(j::root().join(rel)).map(|(w, h)| (w as f32, h as f32)).unwrap_or((400.0, 300.0));
    let s = (400.0 / w.max(1.0)).min(400.0 / h.max(1.0)).min(1.0);
    (jcanvas::snap((w * s).max(80.0)), jcanvas::snap((h * s).max(60.0)))
}

#[derive(Default, Clone, Debug)]
pub(super) enum Drag {
    #[default]
    None,
    Pan,
    /// Cards being moved: their starting places, and where the pointer started (world).
    Move { start: Vec<(String, f32, f32)>, from: Pos2 },
    /// A card's corner (0 top-left, clockwise).
    Resize { id: String, corner: u8, orig: (f32, f32, f32, f32), from: Pos2 },
    /// An arrow being drawn from a card's side.
    Connect { from: String, side: Side },
    /// A box to select with (world).
    Marquee { from: Pos2 },
}

/// The toolbar's pickers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Picker {
    Note,
    Image,
    Snippet,
}

/// An open canvas.
#[derive(Default)]
pub(super) struct CanvasBuf {
    pub rel: String,
    pub doc: Canvas,
    pub saved: String,
    pub edited: Option<Instant>,
    /// The file couldn't be read: shown, and never saved over.
    pub broken: Option<String>,
    /// Where the board's (0, 0) is, from the view's corner, and the zoom.
    pub off: Vec2,
    pub zoom: f32,
    pub fitted: bool,
    pub sel: Vec<String>,
    pub sel_edge: Option<String>,
    /// A text card being typed in, or a widget being used.
    pub editing: Option<String>,
    pub focus_edit: bool,
    pub drag: Drag,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
    pub picker: Option<Picker>,
    pub pick_q: String,
    /// Put the cursor in the picker's search box (once).
    pub focus_pick: bool,
    pub image_rx: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>>,
    /// A group's or arrow's name being typed: (id, text).
    pub label_edit: Option<(String, String)>,
    /// Where the board was right-clicked (world).
    pub menu_at: Option<Pos2>,
    /// The view's rect last frame (to put new cards in the middle of it).
    pub view: View,
}

impl CanvasBuf {
    pub fn load(rel: &str) -> CanvasBuf {
        let text = j::read(&j::root(), rel).unwrap_or_default();
        let (doc, broken) = match Canvas::parse(&text) {
            Ok(c) => (c, None),
            Err(e) => (Canvas::default(), Some(e)),
        };
        let saved = doc.to_json();
        CanvasBuf { rel: rel.to_string(), doc, saved, broken, zoom: 1.0, ..Default::default() }
    }

    fn touch(&mut self) {
        self.edited = Some(Instant::now());
    }

    /// Keep the board as it is now, for Undo.
    fn snapshot(&mut self) {
        let now = self.doc.to_json();
        if self.undo.last() != Some(&now) {
            self.undo.push(now);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.doc.to_json());
            if let Ok(c) = Canvas::parse(&prev) {
                self.doc = c;
            }
            self.sel.clear();
            self.editing = None;
            self.touch();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.doc.to_json());
            if let Ok(c) = Canvas::parse(&next) {
                self.doc = c;
            }
            self.sel.clear();
            self.touch();
        }
    }

    fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        self.view.0.min + self.off + vec2(x, y) * self.zoom
    }

    fn to_world(&self, p: Pos2) -> Pos2 {
        let v = (p - self.view.0.min - self.off) / self.zoom;
        pos2(v.x, v.y)
    }

    fn rect_of(&self, n: &Node) -> Rect {
        Rect::from_min_size(self.to_screen(n.x, n.y), vec2(n.width, n.height) * self.zoom)
    }

    /// Zoom by `f` keeping the board point under `at` still.
    fn zoom_at(&mut self, at: Pos2, f: f32) {
        let nz = (self.zoom * f).clamp(0.1, 3.0);
        let rel = at - self.view.0.min;
        self.off = rel - (rel - self.off) * (nz / self.zoom);
        self.zoom = nz;
    }

    /// Show every card (no closer than `max` zoom).
    fn fit(&mut self, max: f32) {
        let r = self.view.0;
        match self.doc.bounds() {
            None => {
                self.zoom = 1.0;
                self.off = r.size() / 2.0 - vec2(130.0, 60.0);
            }
            Some((x0, y0, x1, y1)) => {
                let (bw, bh) = (x1 - x0 + 160.0, y1 - y0 + 200.0);
                self.zoom = (r.width() / bw).min(r.height() / bh).clamp(0.1, max);
                let c = vec2((x0 + x1) / 2.0, (y0 + y1) / 2.0) * self.zoom;
                self.off = r.size() / 2.0 - c - vec2(0.0, 20.0);
            }
        }
    }

    /// The first look at a board: everything if it fits at a readable size,
    /// else readable from the top left.
    fn first_view(&mut self) {
        self.fit(1.0);
        if self.zoom < 0.7 {
            if let Some((x0, y0, _, _)) = self.doc.bounds() {
                self.zoom = 0.7;
                self.off = vec2(60.0, 70.0) - vec2(x0, y0) * self.zoom;
            }
        }
    }

    /// The middle of the view (world), on the grid.
    fn view_center(&self) -> Pos2 {
        let c = self.to_world(self.view.0.center());
        pos2(jcanvas::snap(c.x), jcanvas::snap(c.y))
    }

    /// Add cards: placed in the middle of the view (or right of everything,
    /// when it's never been seen), selected.
    pub fn add(&mut self, mut nodes: Vec<Node>) {
        if nodes.is_empty() {
            return;
        }
        self.snapshot();
        let n0 = &nodes[0];
        let (x, y) = if self.view.0.width() > 10.0 {
            // the clear space nearest the middle of the view
            let c = self.view_center();
            let (w, h) = (n0.width, n0.height);
            let free = |x: f32, y: f32| {
                let r = Rect::from_min_size(pos2(x, y), vec2(w, h)).expand(20.0);
                !self.doc.nodes.iter().filter(|m| !m.is_group()).any(|m| r.intersects(Rect::from_min_size(pos2(m.x, m.y), vec2(m.width, m.height))))
            };
            let mut spots: Vec<(f32, f32)> = Vec::new();
            for gx in -20..=20 {
                for gy in -14..=14 {
                    spots.push((jcanvas::snap(c.x - w / 2.0 + gx as f32 * 40.0), jcanvas::snap(c.y - h / 2.0 + gy as f32 * 40.0)));
                }
            }
            spots.sort_by(|a, b| {
                let da = (a.0 + w / 2.0 - c.x).powi(2) + (a.1 + h / 2.0 - c.y).powi(2);
                let db = (b.0 + w / 2.0 - c.x).powi(2) + (b.1 + h / 2.0 - c.y).powi(2);
                da.total_cmp(&db)
            });
            spots.into_iter().find(|s| free(s.0, s.1)).unwrap_or((jcanvas::snap(c.x - w / 2.0), jcanvas::snap(c.y - h / 2.0)))
        } else {
            self.doc.free_spot(n0.width, n0.height)
        };
        let (dx, dy) = (x - n0.x, y - n0.y);
        self.sel.clear();
        for n in nodes.iter_mut() {
            n.x += dx;
            n.y += dy;
            self.sel.push(n.id.clone());
        }
        self.doc.nodes.extend(nodes);
        self.touch();
    }

    fn remove_selected(&mut self) {
        if self.sel.is_empty() && self.sel_edge.is_none() {
            return;
        }
        self.snapshot();
        let mut ids = self.sel.clone();
        if let Some(e) = self.sel_edge.take() {
            ids.push(e);
        }
        self.doc.remove(&ids);
        self.sel.clear();
        self.editing = None;
        self.touch();
    }

    fn duplicate_selected(&mut self) {
        let copies: Vec<Node> = self
            .doc
            .nodes
            .iter()
            .filter(|n| self.sel.contains(&n.id))
            .map(|n| Node { id: jcanvas::new_id(), x: n.x + 40.0, y: n.y + 40.0, ..n.clone() })
            .collect();
        if copies.is_empty() {
            return;
        }
        self.snapshot();
        self.sel = copies.iter().map(|n| n.id.clone()).collect();
        self.doc.nodes.extend(copies);
        self.touch();
    }

    /// The arrow under a point (screen), if one's near.
    fn edge_at(&self, p: Pos2) -> Option<String> {
        let mut best: Option<(f32, String)> = None;
        for e in &self.doc.edges {
            let Some(pts) = self.edge_points(e) else { continue };
            let b = CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::NONE);
            let mut prev = pts[0];
            for k in 1..=24 {
                let q = b.sample(k as f32 / 24.0);
                let d = seg_dist(p, prev, q);
                if d < 7.0 && best.as_ref().map_or(true, |b| d < b.0) {
                    best = Some((d, e.id.clone()));
                }
                prev = q;
            }
        }
        best.map(|b| b.1)
    }

    /// An arrow's curve on screen.
    fn edge_points(&self, e: &jcanvas::Edge) -> Option<[Pos2; 4]> {
        let (a, b) = (self.doc.node(&e.from_node)?, self.doc.node(&e.to_node)?);
        let sa = Side::parse(e.from_side.as_deref()).unwrap_or_else(|| jcanvas::facing(a, b));
        let sb = Side::parse(e.to_side.as_deref()).unwrap_or_else(|| jcanvas::facing(b, a));
        let (ax, ay) = a.anchor(sa);
        let (bx, by) = b.anchor(sb);
        let (pa, pb) = (self.to_screen(ax, ay), self.to_screen(bx, by));
        let d = ((pb - pa).length() * 0.4).clamp(30.0 * self.zoom, 220.0 * self.zoom);
        let (da, db) = (sa.dir(), sb.dir());
        Some([pa, pa + vec2(da.0, da.1) * d, pb + vec2(db.0, db.1) * d, pb])
    }
}

/// The view's rect (nothing until it's been drawn).
pub(super) struct View(pub Rect);

impl Default for View {
    fn default() -> Self {
        View(Rect::NOTHING)
    }
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(0.0001)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn node_color(n: &Node) -> Option<Color32> {
    n.color.as_deref().and_then(jcanvas::color_rgb).map(|(r, g, b)| Color32::from_rgb(r, g, b))
}

/// The six colours a card can have, and none.
const SWATCHES: [Option<&str>; 7] = [None, Some("1"), Some("2"), Some("3"), Some("4"), Some("5"), Some("6")];

/// What a frame asked for, done once the canvas is put back.
pub(super) enum CvAct {
    Follow(String),
    Tag(String),
    OpenNote(String),
    OpenCanvas(String),
    OpenImage(String),
    Url(String),
    PickImage,
    AddSnippet(super::info_home::Kind, String),
    AddHunt,
    AddRare,
}

/// The toolbar's little pictures, drawn (not every font has them).
fn tool_icon(p: &egui::Painter, kind: &str, c: Pos2, col: Color32) {
    let st = Stroke::new(1.4f32, col);
    match kind {
        "note" => {
            let r = Rect::from_center_size(c, vec2(10.0, 13.0));
            p.rect_stroke(r, 1.5, st);
            for k in 0..3 {
                let y = r.min.y + 4.0 + k as f32 * 3.0;
                p.line_segment([pos2(r.min.x + 2.5, y), pos2(r.max.x - 2.5, y)], Stroke::new(1.0f32, col));
            }
        }
        "text" => {
            p.text(c, Align2::CENTER_CENTER, "T", crate::fonts::sans_bold_font(14.0), col);
        }
        "image" => {
            let r = Rect::from_center_size(c, vec2(14.0, 11.0));
            p.rect_stroke(r, 2.0, st);
            p.add(egui::Shape::convex_polygon(vec![pos2(r.min.x + 2.0, r.max.y - 2.0), pos2(r.min.x + 6.0, r.min.y + 5.0), pos2(r.min.x + 9.0, r.max.y - 2.0)], col, Stroke::NONE));
            p.circle_filled(pos2(r.max.x - 3.5, r.min.y + 3.5), 1.5, col);
        }
        "widget" => {
            for (dx, dy) in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
                p.rect_stroke(Rect::from_center_size(c + vec2(dx, dy), vec2(5.5, 5.5)), 1.0, Stroke::new(1.2f32, col));
            }
        }
        "snippet" => {
            p.circle_stroke(c, 6.0, st);
            p.circle_filled(c, 2.5, col);
        }
        "group" => {
            let r = Rect::from_center_size(c, vec2(14.0, 12.0));
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            for w in pts.windows(2) {
                p.extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1.2f32, col), 2.5, 2.0));
            }
        }
        "undo" | "redo" => {
            let s = if kind == "undo" { 1.0 } else { -1.0 };
            let pts: Vec<Pos2> = (0..=10).map(|k| {
                let a = std::f32::consts::PI * (0.15 + 0.85 * k as f32 / 10.0);
                c + vec2(-s * 5.0 * a.cos(), -5.0 * a.sin() + 2.0)
            }).collect();
            p.add(egui::Shape::line(pts.clone(), st));
            let tip = pts[0];
            p.add(egui::Shape::convex_polygon(vec![tip + vec2(s * 3.5, -3.5), tip + vec2(s * 3.5, 3.0), tip + vec2(-s * 1.5, 0.0)], col, Stroke::NONE));
        }
        "fit" => {
            let r = Rect::from_center_size(c, vec2(13.0, 13.0));
            for (a, dx, dy) in [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.right_bottom(), -1.0, -1.0), (r.left_bottom(), 1.0, -1.0)] {
                p.line_segment([a, a + vec2(dx * 4.5, 0.0)], st);
                p.line_segment([a, a + vec2(0.0, dy * 4.5)], st);
            }
        }
        _ => {}
    }
}

impl App {
    /// Open (or switch to) a canvas.
    pub(super) fn journal_open_canvas(&mut self, rel: &str) {
        self.journal_flush(true);
        let i = self.journal_cbuf(rel);
        let st = &mut self.setup.journal;
        match st.tabs.iter().position(|t| *t == Open::Canvas(i)) {
            Some(t) => st.active = t,
            None => {
                st.tabs.push(Open::Canvas(i));
                st.active = st.tabs.len() - 1;
            }
        }
        self.journal_remember();
    }

    /// A canvas loaded for showing (not opened as a tab): its place in `cbufs`.
    pub(super) fn journal_cbuf(&mut self, rel: &str) -> usize {
        let st = &mut self.setup.journal;
        match st.cbufs.iter().position(|b| b.rel == rel && b.broken.as_deref() != Some("Moved to the trash")) {
            Some(i) => i,
            None => {
                st.cbufs.push(CanvasBuf::load(rel));
                st.cbufs.len() - 1
            }
        }
    }

    /// Put cards on a canvas, open or not.
    pub(super) fn canvas_add_nodes(&mut self, rel: &str, nodes: Vec<Node>) {
        if let Some(cb) = self.setup.journal.cbufs.iter_mut().find(|b| b.rel == rel) {
            if cb.broken.is_none() {
                cb.add(nodes);
                return;
            }
        }
        let root = j::root();
        let text = j::read(&root, rel).unwrap_or_default();
        match Canvas::parse(&text) {
            Ok(mut c) => {
                let mut nodes = nodes;
                if let Some(n0) = nodes.first() {
                    let (x, y) = c.free_spot(n0.width, n0.height);
                    let (dx, dy) = (x - n0.x, y - n0.y);
                    for n in nodes.iter_mut() {
                        n.x += dx;
                        n.y += dy;
                    }
                }
                c.nodes.extend(nodes);
                if let Err(e) = j::write(&root, rel, &c.to_json()) {
                    self.setup.journal.error = Some(format!("Couldn't save the canvas: {e}"));
                }
            }
            Err(e) => self.setup.journal.error = Some(e),
        }
    }

    /// Save canvases changed a moment ago.
    pub(super) fn canvas_flush(&mut self, force: bool) {
        let root = j::root();
        let mut err = None;
        for b in self.setup.journal.cbufs.iter_mut() {
            if b.broken.is_some() {
                continue;
            }
            if b.edited.is_some_and(|t| force || t.elapsed() > Duration::from_millis(700)) {
                let json = b.doc.to_json();
                if json != b.saved {
                    match j::write(&root, &b.rel, &json) {
                        Ok(()) => b.saved = json,
                        Err(e) => err = Some(format!("Couldn't save {}: {e}", j::title_of(&b.rel))),
                    }
                }
                b.edited = None;
            }
        }
        if err.is_some() {
            self.setup.journal.error = err;
        }
    }

    /// A canvas tab.
    pub(super) fn journal_canvas(&mut self, ui: &mut egui::Ui, i: usize) {
        if i >= self.setup.journal.cbufs.len() {
            return;
        }
        let mut cb = std::mem::take(&mut self.setup.journal.cbufs[i]);
        let acts = self.cv_frame(ui, &mut cb);
        let rel = cb.rel.clone();
        self.setup.journal.cbufs[i] = cb;
        let ctx = ui.ctx().clone();
        for a in acts {
            match a {
                CvAct::Follow(t) => self.journal_follow(&ctx, &t),
                CvAct::Tag(t) => {
                    self.setup.journal.side = super::journal_app::Side::Search;
                    self.setup.journal.query = format!("#{t}");
                }
                CvAct::OpenNote(r) => self.journal_open(&r),
                CvAct::OpenCanvas(r) => self.journal_open_canvas(&r),
                CvAct::OpenImage(r) => self.journal_open_image(&r),
                CvAct::Url(u) => ctx.open_url(egui::OpenUrl::new_tab(u)),
                CvAct::PickImage => {
                    let start = dirs::picture_dir().unwrap_or_else(|| j::root());
                    let rx = crate::share::pick_image(start, ctx.clone());
                    if let Some(cb) = self.setup.journal.cbufs.iter_mut().find(|b| b.rel == rel) {
                        cb.image_rx = Some(rx);
                    }
                }
                CvAct::AddSnippet(kind, key) => {
                    use super::info_home::{Kind, Page};
                    let clip = match kind {
                        Kind::Pokemon => self.dex_clip(&key),
                        Kind::Move => self.info_clip(&Page::Move(key)),
                        Kind::Item => self.info_clip(&Page::Item(key)),
                        Kind::Place => self.info_clip(&Page::Place(key)),
                        Kind::Guide => self.info_clip(&Page::Guide(key)),
                    };
                    if let Some(c) = clip {
                        self.journal_put(c, super::journal_add::Target::Canvas(rel.clone()), How::Card);
                    }
                }
                CvAct::AddHunt => {
                    if let Some(c) = self.run_clip(super::RunRef::Current, false) {
                        self.journal_put(c, super::journal_add::Target::Canvas(rel.clone()), How::Card);
                    }
                }
                CvAct::AddRare => {
                    let spots = self.rare_spots_today();
                    let mut text = String::from("> [!place] Rare grass today\n");
                    for s in spots.iter().take(12) {
                        text.push_str(&format!("> **{}** · {}\n", s.species.name, s.place));
                    }
                    if spots.is_empty() {
                        text.push_str("> Nothing found yet today\n");
                    }
                    let h = jcanvas::snap((spots.len().min(12) as f32 * 26.0 + 50.0).max(100.0));
                    self.canvas_add_nodes(&rel, vec![jcanvas::text_node(0.0, 0.0, 340.0, h, &text)]);
                }
            }
        }
    }

    /// A picture from the PC, copied into the Journal's Attachments and put on the canvas.
    fn cv_poll_image(&mut self, cb: &mut CanvasBuf) {
        let Some(rx) = &cb.image_rx else { return };
        match rx.try_recv() {
            Ok(Some(path)) => {
                cb.image_rx = None;
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Picture".into());
                let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
                let root = j::root();
                let rel = j::free_rel_ext(&root, j::ATTACH, &name, &ext);
                match std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| j::write_bytes(&root, &rel, &b)) {
                    Ok(()) => {
                        let (w, h) = image_size(&rel);
                        cb.add(vec![jcanvas::file_node(0.0, 0.0, w, h, &rel)]);
                        self.journal_scan(true);
                    }
                    Err(e) => self.setup.journal.error = Some(format!("Couldn't add the picture: {e}")),
                }
            }
            Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => cb.image_rx = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn cv_frame(&mut self, ui: &mut egui::Ui, cb: &mut CanvasBuf) -> Vec<CvAct> {
        let mut acts: Vec<CvAct> = Vec::new();
        let full = ui.max_rect();
        let ctx = ui.ctx().clone();
        self.cv_poll_image(cb);
        if let Some(e) = cb.broken.clone() {
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full.shrink(40.0)), |ui| {
                ui.label(RichText::new("This canvas couldn't be opened").font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                ui.label(RichText::new(format!("{e}\n\nIt's left as it is, so nothing in it is lost. You can fix it in a text editor.")).size(kit::fs(13.0)).color(MUTED()));
            });
            return acts;
        }
        cb.view = View(full);
        if !cb.fitted {
            cb.first_view();
            cb.fitted = true;
        }
        let rel_id = cb.rel.clone();
        let rid = move |s: &str| egui::Id::new(("jc", rel_id.clone(), s.to_string()));
        let bg = ui.interact(full, rid("bg"), Sense::click_and_drag());
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|p| full.contains(*p));
        let pointer = ui.input(|i| i.pointer.interact_pos());
        // scroll pans, Ctrl+scroll (or a pinch) zooms
        if let Some(hp) = hover {
            let over_ui = ctx.is_pointer_over_area() && ctx.layer_id_at(hp).is_some_and(|l| l.order != egui::Order::Background && l.order != egui::Order::Middle);
            if !over_ui {
                let (zd, sd) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
                if (zd - 1.0).abs() > 0.0001 {
                    cb.zoom_at(hp, zd);
                } else if sd != Vec2::ZERO {
                    cb.off += sd;
                }
                // used here: a page this canvas sits in doesn't scroll too
                ui.input_mut(|i| {
                    i.smooth_scroll_delta = Vec2::ZERO;
                    i.raw_scroll_delta = Vec2::ZERO;
                });
            }
            let (mid, d) = ui.input(|i| (i.pointer.middle_down(), i.pointer.delta()));
            if mid {
                cb.off += d;
            }
        }
        let z = cb.zoom;
        let p = ui.painter_at(full);
        p.rect_filled(full, 0.0, theme::blend(kit::PAGE(), Color32::BLACK, 0.18));
        // the dotted grid
        {
            let mut step = 20.0 * z;
            while step < 14.0 {
                step *= 2.0;
            }
            let origin = full.min + cb.off;
            let x0 = origin.x + ((full.min.x - origin.x) / step).floor() * step;
            let y0 = origin.y + ((full.min.y - origin.y) / step).floor() * step;
            let dot = LINE().gamma_multiply(0.9);
            let mut y = y0;
            while y < full.max.y {
                let mut x = x0;
                while x < full.max.x {
                    p.rect_filled(Rect::from_center_size(pos2(x, y), vec2(1.6, 1.6)), 0.0, dot);
                    x += step;
                }
                y += step;
            }
        }
        let rounding = (10.0 * z).clamp(2.0, 12.0);
        let world_ptr = pointer.map(|q| cb.to_world(q));
        // where a press began (a drag only counts after a few points of movement)
        let press_world = ui.input(|i| i.pointer.press_origin()).map(|q| cb.to_world(q)).or(world_ptr);
        // the card the pointer's near (for its side dots)
        let near: Option<String> = hover.and_then(|hp| {
            cb.doc.nodes.iter().rev().filter(|n| !n.is_group()).find(|n| cb.rect_of(n).expand(14.0).contains(hp)).map(|n| n.id.clone())
        });

        // ── groups ──
        let mut node_resps: Vec<(String, egui::Response)> = Vec::new();
        let groups: Vec<Node> = cb.doc.nodes.iter().filter(|n| n.is_group()).cloned().collect();
        for g in &groups {
            let r = cb.rect_of(g);
            if !r.intersects(full) {
                continue;
            }
            let col = node_color(g).unwrap_or(JOURNAL_COL);
            let on = cb.sel.contains(&g.id);
            p.rect_filled(r, rounding, col.gamma_multiply(0.06));
            let st = Stroke::new(if on { 2.0f32 } else { 1.2 }, if on { JOURNAL_COL } else { col.gamma_multiply(0.7) });
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            for w in pts.windows(2) {
                p.extend(egui::Shape::dashed_line(&[w[0], w[1]], st, 6.0, 4.0));
            }
            let label = g.label.clone().unwrap_or_else(|| "Group".into());
            let f = crate::fonts::sans_font((13.0 * z).clamp(9.0, 18.0));
            let gal = p.layout_no_wrap(label, f, TEXT());
            let lr = Rect::from_min_size(r.left_top() + vec2(0.0, -gal.size().y - 10.0), gal.size() + vec2(18.0, 8.0));
            p.rect_filled(lr, lr.height() / 2.0, theme::blend(CARD(), col, 0.25));
            p.galley(lr.min + vec2(9.0, 4.0), gal, TEXT());
            let resp = ui.interact(r.union(lr), rid(&format!("g-{}", g.id)), Sense::click_and_drag());
            node_resps.push((g.id.clone(), resp));
        }

        // ── arrows ──
        let edges = cb.doc.edges.clone();
        for e in &edges {
            let Some(pts) = cb.edge_points(e) else { continue };
            let on = cb.sel_edge.as_deref() == Some(e.id.as_str());
            let col = e.color.as_deref().and_then(jcanvas::color_rgb).map(|(r, g, b)| Color32::from_rgb(r, g, b)).unwrap_or(theme::blend(MUTED(), JOURNAL_COL, 0.35));
            let col = if on { JOURNAL_COL } else { col };
            let w = if on { 3.0 } else { 2.0 } * z.clamp(0.6, 1.4);
            p.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(w, col)));
            let head = (10.0 * z).clamp(6.0, 14.0);
            let arrow = |tip: Pos2, from: Pos2| {
                let d = (tip - from).normalized();
                let n = vec2(-d.y, d.x);
                p.add(egui::Shape::convex_polygon(vec![tip, tip - d * head + n * head * 0.55, tip - d * head - n * head * 0.55], col, Stroke::NONE));
            };
            if e.to_end.as_deref() != Some("none") {
                arrow(pts[3], pts[2]);
            }
            if e.from_end.as_deref() == Some("arrow") {
                arrow(pts[0], pts[1]);
            }
            if let Some(l) = e.label.as_ref().filter(|l| !l.is_empty()) {
                let b = CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::NONE);
                let m = b.sample(0.5);
                let gal = p.layout_no_wrap(l.clone(), crate::fonts::sans_font((12.0 * z).clamp(9.0, 16.0)), TEXT());
                let lr = Rect::from_center_size(m, gal.size() + vec2(16.0, 8.0));
                p.rect_filled(lr, lr.height() / 2.0, theme::blend(kit::PAGE(), CARD(), 0.7));
                p.rect_stroke(lr, lr.height() / 2.0, Stroke::new(1.0f32, col.gamma_multiply(0.7)));
                p.galley(lr.min + vec2(8.0, 4.0), gal, TEXT());
            }
        }

        // ── cards ──
        let cards: Vec<Node> = cb.doc.nodes.iter().filter(|n| !n.is_group()).cloned().collect();
        let mut widget_data: Vec<(String, Vec<String>)> = Vec::new();
        let mut toggles: Vec<(String, usize)> = Vec::new();
        let mut stop_edit = false;
        for n in &cards {
            let r = cb.rect_of(n);
            if !r.intersects(full) {
                continue;
            }
            let on = cb.sel.contains(&n.id);
            let editing = cb.editing.as_deref() == Some(n.id.as_str());
            let col = node_color(n);
            let fill = match col {
                Some(c) => theme::blend(CARD(), c, 0.10),
                None => CARD(),
            };
            p.add(kit::shadow_tier(2).as_shape(r, rounding));
            p.rect_filled(r, rounding, fill);
            // the card's own handle, under what's in it (links in it still click)
            let is_widget = n.widget.is_some();
            let resp_under = if !is_widget && !editing { Some(ui.interact(r, rid(&format!("n-{}", n.id)), Sense::click_and_drag())) } else { None };
            let pad = (12.0 * z).clamp(3.0, 18.0);
            let inner = r.shrink(pad);
            let clip = r.shrink(1.0).intersect(full);
            let mut child = ui.new_child(egui::UiBuilder::new().id_salt(("jc-card", &n.id)).max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(clip);
            let st = MdStyle { size: 14.0 * z, accent: JOURNAL_COL };
            if let Some(kind) = n.widget.clone() {
                if z < 0.4 {
                    let d = super::widgets::def(&kind);
                    let chip = Rect::from_center_size(r.center() - vec2(0.0, 10.0 * z.max(0.5)), Vec2::splat((40.0 * z).max(14.0)));
                    kit::icon_chip(&p, chip, d.map_or("▣", |d| d.icon), JOURNAL_COL, false);
                } else if let Some(d) = super::widgets::def(&kind) {
                    let l = super::widgets::lim(Some(d));
                    let gap = CGAP * z;
                    let cols = (((n.width + CGAP) / (CELL + CGAP)).round() as i32).clamp(l.min.0, l.max.0);
                    let rows = (((n.height + CGAP) / (CELL + CGAP)).round() as i32).clamp(l.min.1, l.max.1);
                    let cell = ((r.width() + gap) / cols as f32 - gap).min((r.height() + gap) / rows as f32 - gap).max(20.0);
                    let mut wc = ui.new_child(egui::UiBuilder::new().id_salt(("jc-widget", &n.id)).max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
                    wc.set_clip_rect(clip);
                    let cfg = crate::apps::WidgetCfg { id: id_num(&n.id), kind: kind.clone(), span: d.span, data: n.data.clone(), pos: None };
                    super::w_fit::set_box(Some(super::w_fit::WBox { r, cols, rows, cell }));
                    if let Some(data) = self.draw_widget(&mut wc, &cfg, false) {
                        widget_data.push((n.id.clone(), data));
                    }
                    super::w_fit::set_box(None);
                } else {
                    p.text(r.center(), Align2::CENTER_CENTER, "This widget isn't in this version", crate::fonts::sans_font(12.0 * z.max(0.7)), MUTED());
                }
            } else if editing && n.text.is_some() {
                let id = egui::Id::new(("jc-edit", &cb.rel, &n.id));
                let mut text = n.text.clone().unwrap_or_default();
                let mut layouter = |ui: &egui::Ui, s: &str, wrap: f32| {
                    let mut job = md::live_job(s, st);
                    job.wrap.max_width = wrap;
                    ui.fonts(|f| f.layout_job(job))
                };
                let resp = child.add(egui::TextEdit::multiline(&mut text).id(id).frame(false).desired_width(inner.width()).min_size(inner.size()).hint_text("Type here… (Markdown works)").layouter(&mut layouter));
                if cb.focus_edit {
                    resp.request_focus();
                    cb.focus_edit = false;
                } else if resp.lost_focus() || (!resp.has_focus() && ui.input(|i| i.pointer.any_pressed()) && !hover.is_some_and(|h| r.contains(h))) {
                    stop_edit = true;
                }
                if resp.changed() {
                    if let Some(m) = cb.doc.node_mut(&n.id) {
                        m.text = Some(text);
                    }
                    cb.touch();
                }
            } else if let Some(t) = n.text.clone() {
                if z < 0.3 {
                    for k in 0..((inner.height() / (10.0f32).max(14.0 * z * 1.6)) as usize).min(8) {
                        let y = inner.min.y + k as f32 * (14.0 * z * 1.6).max(6.0) + 3.0;
                        p.rect_filled(Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width() * if k == 0 { 0.6 } else { 0.9 }, (14.0 * z * 0.7).max(2.0))), 1.0, MUTED().gamma_multiply(0.35));
                    }
                } else {
                    let mut embed = |ui: &mut egui::Ui, what: &str, w: f32| self.journal_embed(ui, what, w);
                    match md::reading(&mut child, &t, st, &mut embed) {
                        Some(Clicked::Link(l)) => acts.push(CvAct::Follow(l)),
                        Some(Clicked::Tag(tg)) => acts.push(CvAct::Tag(tg)),
                        Some(Clicked::Toggle(line)) => toggles.push((n.id.clone(), line)),
                        None => {}
                    }
                    if t.trim().is_empty() {
                        p.text(inner.left_top(), Align2::LEFT_TOP, "Double-click to write", crate::fonts::sans_font(13.0 * z), MUTED());
                    }
                }
            } else if let Some(f) = n.file.clone() {
                if j::is_image(&f) {
                    match self.journal_tex(&ctx, &f) {
                        Some((tex, size)) => {
                            let s = (r.width() / size.x).min(r.height() / size.y);
                            let ir = Rect::from_center_size(r.center(), size * s);
                            p.add(egui::Shape::image(tex, ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE));
                        }
                        None => {
                            p.text(r.center(), Align2::CENTER_CENTER, format!("Missing picture\n{f}"), crate::fonts::sans_font(12.0 * z.max(0.7)), MUTED());
                        }
                    }
                } else {
                    // a note (or another canvas): its name, then what it says
                    let hh = (30.0 * z).max(12.0);
                    let hr = Rect::from_min_size(r.min, vec2(r.width(), hh));
                    p.rect_filled(hr, egui::Rounding { nw: rounding, ne: rounding, sw: 0.0, se: 0.0 }, theme::blend(fill, JOURNAL_COL, 0.12));
                    let icon_c = pos2(hr.min.x + 14.0 * z.max(0.6), hr.center().y);
                    tool_icon(&p, "note", icon_c, JOURNAL_COL);
                    let tf = crate::fonts::sans_bold_font((13.0 * z).clamp(8.0, 20.0));
                    let tt = super::charts::clip(ui, &j::title_of(&f), &tf, (hr.width() - 40.0 * z).max(10.0));
                    p.text(pos2(hr.min.x + 28.0 * z.max(0.6), hr.center().y), Align2::LEFT_CENTER, tt, tf, TEXT());
                    let body = Rect::from_min_max(pos2(r.min.x + pad, hr.max.y + pad * 0.6), pos2(r.max.x - pad, r.max.y - pad));
                    let text = self.setup.journal.bufs.iter().find(|b| b.rel == f).map(|b| b.text.clone()).or_else(|| self.setup.journal.index.get(&f).map(|i| i.text.clone()));
                    match text {
                        Some(t) if z >= 0.3 && !j::is_canvas(&f) => {
                            let t = {
                                let (_, at) = j::front_matter(&t);
                                let body = &t[at.min(t.len())..];
                                let first = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                                if first.strip_prefix("# ").is_some_and(|h| h.trim().eq_ignore_ascii_case(&j::title_of(&f))) {
                                    body.splitn(2, first).nth(1).unwrap_or("").to_string()
                                } else {
                                    body.to_string()
                                }
                            };
                            let mut nc = ui.new_child(egui::UiBuilder::new().id_salt(("jc-note", &n.id)).max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
                            nc.set_clip_rect(body.intersect(full));
                            let mut embed = |ui: &mut egui::Ui, what: &str, w: f32| self.journal_embed(ui, what, w);
                            match md::reading(&mut nc, &t, MdStyle { size: 13.0 * z, accent: JOURNAL_COL }, &mut embed) {
                                Some(Clicked::Link(l)) => acts.push(CvAct::Follow(l)),
                                Some(Clicked::Tag(tg)) => acts.push(CvAct::Tag(tg)),
                                _ => {}
                            }
                        }
                        Some(_) => {}
                        None if j::is_canvas(&f) => {
                            p.text(body.left_top(), Align2::LEFT_TOP, "A canvas: double-click to open it", crate::fonts::sans_font(12.0 * z.max(0.6)), MUTED());
                        }
                        None => {
                            p.text(body.left_top(), Align2::LEFT_TOP, "This note isn't in the Journal any more", crate::fonts::sans_font(12.0 * z.max(0.6)), MUTED());
                        }
                    }
                }
            } else if let Some(u) = n.url.clone() {
                p.text(inner.left_top(), Align2::LEFT_TOP, format!("🔗 {u}"), crate::fonts::sans_font((13.0 * z).max(8.0)), md::LINK);
            }
            // the card's edge, drawn over what's in it
            let edge_col = if on || editing { JOURNAL_COL } else { col.map_or(LINE(), |c| c.gamma_multiply(0.85)) };
            p.rect_stroke(r, rounding, Stroke::new(if on || editing { 2.0f32 } else { 1.0 }, edge_col));
            if col.is_some() && !on {
                p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), (4.0 * z).clamp(2.0, 5.0))), egui::Rounding { nw: rounding, ne: rounding, sw: 0.0, se: 0.0 }, col.unwrap_or(LINE()));
            }
            // a widget's shield: it's moved like any card until it's double-clicked
            let resp = match resp_under {
                Some(r) => Some(r),
                None if is_widget && !editing => Some(ui.interact(r, rid(&format!("n-{}", n.id)), Sense::click_and_drag()).on_hover_text("Double-click to use the widget")),
                None => None,
            };
            if let Some(resp) = resp {
                node_resps.push((n.id.clone(), resp));
            }
            if editing && is_widget {
                let br = Rect::from_min_size(r.left_top() + vec2(0.0, -24.0), vec2(150.0, 20.0));
                p.rect_filled(br, 10.0, JOURNAL_COL);
                p.text(br.center(), Align2::CENTER_CENTER, "In use · click outside to stop", crate::fonts::sans_font(10.5), Color32::WHITE);
            }
        }
        for (id, data) in widget_data {
            if let Some(m) = cb.doc.node_mut(&id) {
                if m.data != data {
                    m.data = data;
                    cb.touch();
                }
            }
        }
        for (id, line) in toggles {
            cb.snapshot();
            if let Some(m) = cb.doc.node_mut(&id) {
                if let Some(t) = &m.text {
                    m.text = Some(md::toggle_task(t, line));
                }
            }
            cb.touch();
        }
        if stop_edit {
            cb.editing = None;
        }

        // ── handles: resize corners and side dots ──
        let mut handle_resps: Vec<(u8, egui::Response)> = Vec::new();
        let mut dot_resps: Vec<(Side, egui::Response)> = Vec::new();
        let single = if cb.sel.len() == 1 { cb.sel.first().cloned() } else { None };
        if let Some(id) = single.clone().filter(|id| cb.editing.as_deref() != Some(id.as_str())) {
            if let Some(n) = cb.doc.node(&id).cloned() {
                let r = cb.rect_of(&n);
                for (k, c) in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().enumerate() {
                    let hr = Rect::from_center_size(c, vec2(10.0, 10.0));
                    p.rect_filled(hr, 2.0, kit::PAGE());
                    p.rect_stroke(hr, 2.0, Stroke::new(1.5f32, JOURNAL_COL));
                    let resp = ui.interact(hr.expand(3.0), rid(&format!("h{k}")), Sense::drag());
                    let icon = if k % 2 == 0 { egui::CursorIcon::ResizeNwSe } else { egui::CursorIcon::ResizeNeSw };
                    handle_resps.push((k as u8, resp.on_hover_cursor(icon)));
                }
            }
        }
        let dot_for = match &cb.drag {
            Drag::Connect { from, .. } => Some(from.clone()),
            _ => near.clone().or(single.clone()),
        };
        if let Some(id) = dot_for {
            if let Some(n) = cb.doc.node(&id).filter(|n| !n.is_group() && cb.editing.as_deref() != Some(n.id.as_str())).cloned() {
                for s in Side::ALL {
                    let (x, y) = n.anchor(s);
                    let c = cb.to_screen(x, y);
                    let resp = ui.interact(Rect::from_center_size(c, vec2(18.0, 18.0)), rid(&format!("dot-{}", s.name())), Sense::drag());
                    let big = resp.hovered() || resp.dragged();
                    p.circle_filled(c, if big { 7.0 } else { 5.0 }, kit::PAGE());
                    p.circle_stroke(c, if big { 7.0 } else { 5.0 }, Stroke::new(2.0f32, JOURNAL_COL));
                    dot_resps.push((s, resp.on_hover_text("Drag to draw an arrow").on_hover_cursor(egui::CursorIcon::Crosshair)));
                }
            }
        }

        // ── what was done ──
        let shift = ui.input(|i| i.modifiers.shift);
        let mut menu_node: Option<(String, egui::Response)> = None;
        for (id, resp) in &node_resps {
            if resp.drag_started_by(egui::PointerButton::Primary) {
                if shift {
                    if !cb.sel.contains(id) {
                        cb.sel.push(id.clone());
                    }
                } else if !cb.sel.contains(id) {
                    cb.sel = vec![id.clone()];
                }
                cb.sel_edge = None;
                if cb.editing.as_deref() != Some(id.as_str()) {
                    cb.editing = None;
                }
                cb.snapshot();
                // a group takes what's in it along
                let mut ids: Vec<String> = cb.sel.clone();
                for sid in cb.sel.clone() {
                    if let Some(g) = cb.doc.node(&sid).filter(|g| g.is_group()).cloned() {
                        for m in cb.doc.nodes.iter().filter(|m| g.holds(m)) {
                            if !ids.contains(&m.id) {
                                ids.push(m.id.clone());
                            }
                        }
                    }
                }
                let start = ids.iter().filter_map(|i| cb.doc.node(i).map(|n| (i.clone(), n.x, n.y))).collect();
                cb.drag = Drag::Move { start, from: press_world.unwrap_or_default() };
            }
            if resp.clicked() {
                if shift {
                    if let Some(k) = cb.sel.iter().position(|s| s == id) {
                        cb.sel.remove(k);
                    } else {
                        cb.sel.push(id.clone());
                    }
                } else {
                    cb.sel = vec![id.clone()];
                }
                cb.sel_edge = None;
                if cb.editing.as_deref() != Some(id.as_str()) {
                    cb.editing = None;
                }
            }
            if resp.double_clicked() {
                if let Some(n) = cb.doc.node(id).cloned() {
                    if n.is_group() {
                        cb.label_edit = Some((n.id.clone(), n.label.clone().unwrap_or_default()));
                    } else if n.widget.is_some() {
                        cb.editing = Some(n.id.clone());
                    } else if n.text.is_some() {
                        cb.snapshot();
                        cb.editing = Some(n.id.clone());
                        cb.focus_edit = true;
                    } else if let Some(f) = &n.file {
                        acts.push(if j::is_image(f) {
                            CvAct::OpenImage(f.clone())
                        } else if j::is_canvas(f) {
                            CvAct::OpenCanvas(f.clone())
                        } else {
                            CvAct::OpenNote(f.clone())
                        });
                    } else if let Some(u) = &n.url {
                        acts.push(CvAct::Url(u.clone()));
                    }
                }
            }
            if resp.secondary_clicked() {
                if !cb.sel.contains(id) {
                    cb.sel = vec![id.clone()];
                }
                cb.sel_edge = None;
            }
            if menu_node.is_none() {
                menu_node = Some((id.clone(), resp.clone()));
            }
            // the menu belongs to the card it was opened on
            resp.context_menu(|ui| {
                self.cv_node_menu(ui, cb, id, &mut acts);
            });
        }
        let _ = menu_node;
        for (corner, resp) in &handle_resps {
            if resp.drag_started() {
                if let Some(id) = single.clone() {
                    if let Some(n) = cb.doc.node(&id) {
                        let orig = n.rect();
                        cb.snapshot();
                        cb.drag = Drag::Resize { id, corner: *corner, orig, from: press_world.unwrap_or_default() };
                    }
                }
            }
        }
        for (side, resp) in &dot_resps {
            if resp.drag_started() {
                if let Some(id) = near.clone().or(single.clone()) {
                    cb.drag = Drag::Connect { from: id, side: *side };
                }
            }
        }
        if bg.drag_started_by(egui::PointerButton::Primary) {
            cb.drag = if shift { Drag::Marquee { from: press_world.unwrap_or_default() } } else { Drag::Pan };
        }
        if bg.clicked() {
            cb.editing = None;
            cb.picker = None;
            cb.sel.clear();
            cb.sel_edge = hover.and_then(|h| cb.edge_at(h));
        }
        if bg.double_clicked() && cb.edge_at(hover.unwrap_or_default()).is_none() {
            if let Some(w) = world_ptr {
                cb.snapshot();
                let n = jcanvas::text_node(jcanvas::snap(w.x - 130.0), jcanvas::snap(w.y - 40.0), 260.0, 100.0, "");
                cb.sel = vec![n.id.clone()];
                cb.editing = Some(n.id.clone());
                cb.focus_edit = true;
                cb.doc.nodes.push(n);
                cb.touch();
            }
        } else if let Some(e) = hover.filter(|_| bg.double_clicked()).and_then(|h| cb.edge_at(h)) {
            let l = cb.doc.edges.iter().find(|x| x.id == e).and_then(|x| x.label.clone()).unwrap_or_default();
            cb.label_edit = Some((e, l));
        }
        if bg.secondary_clicked() {
            cb.menu_at = world_ptr;
            cb.sel_edge = hover.and_then(|h| cb.edge_at(h));
        }
        bg.context_menu(|ui| {
            self.cv_board_menu(ui, cb, &mut acts);
        });

        // dragging
        let primary_down = ui.input(|i| i.pointer.primary_down());
        let drag = cb.drag.clone();
        match drag {
            Drag::Pan => {
                cb.off += ui.input(|i| i.pointer.delta());
                if !primary_down {
                    cb.drag = Drag::None;
                }
            }
            Drag::Move { start, from } => {
                if let Some(w) = world_ptr {
                    let d = w - from;
                    for (id, x, y) in &start {
                        if let Some(n) = cb.doc.node_mut(id) {
                            n.x = x + d.x;
                            n.y = y + d.y;
                        }
                    }
                    if d != Vec2::ZERO {
                        cb.touch();
                    }
                }
                if !primary_down {
                    let mut moved = false;
                    for (id, x, y) in &start {
                        if let Some(n) = cb.doc.node_mut(id) {
                            n.x = jcanvas::snap(n.x);
                            n.y = jcanvas::snap(n.y);
                            moved |= n.x != *x || n.y != *y;
                        }
                    }
                    if !moved {
                        cb.undo.pop();
                    }
                    cb.drag = Drag::None;
                }
            }
            Drag::Resize { id, corner, orig, from } => {
                if let (Some(w), Some(n)) = (world_ptr, cb.doc.node_mut(&id)) {
                    let (ox, oy, ow, oh) = orig;
                    let d = w - from;
                    let (min_w, min_h) = (80.0, 40.0);
                    let (mut x, mut y) = (ox, oy);
                    let (wd, ht);
                    if corner == 0 || corner == 3 {
                        wd = (ow - d.x).max(min_w);
                        x = ox + ow - wd;
                    } else {
                        wd = (ow + d.x).max(min_w);
                    }
                    if corner <= 1 {
                        ht = (oh - d.y).max(min_h);
                        y = oy + oh - ht;
                    } else {
                        ht = (oh + d.y).max(min_h);
                    }
                    n.x = x;
                    n.y = y;
                    n.width = wd;
                    n.height = ht;
                    cb.touch();
                }
                if !primary_down {
                    if let Some(n) = cb.doc.node_mut(&id) {
                        let (x1, y1) = (jcanvas::snap(n.x + n.width), jcanvas::snap(n.y + n.height));
                        n.x = jcanvas::snap(n.x);
                        n.y = jcanvas::snap(n.y);
                        n.width = (x1 - n.x).max(80.0);
                        n.height = (y1 - n.y).max(40.0);
                    }
                    cb.drag = Drag::None;
                }
            }
            Drag::Connect { from, side } => {
                if let (Some(a), Some(ptr)) = (cb.doc.node(&from).cloned(), pointer) {
                    let (ax, ay) = a.anchor(side);
                    let pa = cb.to_screen(ax, ay);
                    let d = ((ptr - pa).length() * 0.4).clamp(20.0, 200.0);
                    let dir = side.dir();
                    p.add(CubicBezierShape::from_points_stroke([pa, pa + vec2(dir.0, dir.1) * d, ptr, ptr], false, Color32::TRANSPARENT, Stroke::new(2.0f32, JOURNAL_COL)));
                    p.circle_filled(ptr, 4.0, JOURNAL_COL);
                    if !primary_down {
                        let w = cb.to_world(ptr);
                        let to = cb.doc.nodes.iter().rev().filter(|n| !n.is_group() && n.id != from).find(|n| n.contains(w.x, w.y)).cloned();
                        cb.snapshot();
                        let to = match to {
                            Some(t) => t,
                            None => {
                                // let go on the board: a new card there
                                let n = jcanvas::text_node(jcanvas::snap(w.x - 130.0), jcanvas::snap(w.y - 50.0), 260.0, 100.0, "");
                                cb.editing = Some(n.id.clone());
                                cb.focus_edit = true;
                                cb.doc.nodes.push(n.clone());
                                n
                            }
                        };
                        let mut e = jcanvas::edge(&a, &to);
                        e.from_side = Some(side.name().into());
                        if !cb.doc.edges.iter().any(|x| x.from_node == e.from_node && x.to_node == e.to_node) {
                            cb.doc.edges.push(e);
                        }
                        cb.sel = vec![to.id.clone()];
                        cb.touch();
                        cb.drag = Drag::None;
                    }
                } else if !primary_down {
                    cb.drag = Drag::None;
                }
            }
            Drag::Marquee { from } => {
                if let Some(w) = world_ptr {
                    let a = cb.to_screen(from.x, from.y);
                    let b = cb.to_screen(w.x, w.y);
                    let mr = Rect::from_two_pos(a, b);
                    p.rect_filled(mr, 2.0, JOURNAL_COL.gamma_multiply(0.08));
                    p.rect_stroke(mr, 2.0, Stroke::new(1.0f32, JOURNAL_COL));
                    if !primary_down {
                        let wr = Rect::from_two_pos(from, w);
                        cb.sel = cb.doc.nodes.iter().filter(|n| wr.intersects(Rect::from_min_size(pos2(n.x, n.y), vec2(n.width, n.height)))).map(|n| n.id.clone()).collect();
                        cb.drag = Drag::None;
                    }
                }
                if !primary_down {
                    cb.drag = Drag::None;
                }
            }
            Drag::None => {}
        }

        // keys (when nothing's being typed in)
        if !ctx.wants_keyboard_input() && hover.is_some() {
            let (del, undo, redo, all, dup, esc) = ui.input_mut(|i| {
                let undo = i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
                let redo = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z) || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
                (
                    i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                    undo && !redo,
                    redo,
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::A),
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::D),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            if del {
                cb.remove_selected();
            }
            if undo {
                cb.undo();
            }
            if redo {
                cb.redo();
            }
            if all {
                cb.sel = cb.doc.nodes.iter().map(|n| n.id.clone()).collect();
            }
            if dup {
                cb.duplicate_selected();
            }
            if esc {
                cb.sel.clear();
                cb.sel_edge = None;
                cb.editing = None;
                cb.picker = None;
            }
        }

        self.cv_label_editor(ui, cb);
        self.cv_controls(ui, cb, &mut acts);
        if cb.drag.is_some_moving() || cb.edited.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if cb.doc.nodes.is_empty() {
            let c = full.center() - vec2(0.0, 40.0);
            p.text(c, Align2::CENTER_CENTER, "An empty canvas", crate::fonts::sans_bold_font(17.0), TEXT());
            p.text(c + vec2(0.0, 26.0), Align2::CENTER_CENTER, "Double-click anywhere for a card, or add notes, pictures and widgets below.", crate::fonts::sans_font(13.0), MUTED());
        }
        acts
    }

    /// A card's right-click menu.
    fn cv_node_menu(&mut self, ui: &mut egui::Ui, cb: &mut CanvasBuf, id: &str, acts: &mut Vec<CvAct>) {
        let Some(n) = cb.doc.node(id).cloned() else { return };
        if n.text.is_some() && n.widget.is_none() && ui.button("Edit").clicked() {
            cb.snapshot();
            cb.editing = Some(id.to_string());
            cb.focus_edit = true;
            ui.close_menu();
        }
        if n.is_group() && ui.button("Rename group").clicked() {
            cb.label_edit = Some((id.to_string(), n.label.clone().unwrap_or_default()));
            ui.close_menu();
        }
        if n.widget.is_some() && ui.button("Use the widget").clicked() {
            cb.editing = Some(id.to_string());
            ui.close_menu();
        }
        if let Some(f) = &n.file {
            if ui.button(if j::is_image(f) { "Open the picture" } else { "Open in a tab" }).clicked() {
                acts.push(if j::is_image(f) {
                    CvAct::OpenImage(f.clone())
                } else if j::is_canvas(f) {
                    CvAct::OpenCanvas(f.clone())
                } else {
                    CvAct::OpenNote(f.clone())
                });
                ui.close_menu();
            }
        }
        ui.separator();
        ui.label(RichText::new("Colour").size(11.5).color(MUTED()));
        ui.horizontal(|ui| {
            for s in SWATCHES {
                let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                let c = s.and_then(jcanvas::color_rgb).map(|(r, g, b)| Color32::from_rgb(r, g, b));
                match c {
                    Some(c) => {
                        ui.painter().circle_filled(r.center(), 8.0, c);
                    }
                    None => {
                        ui.painter().circle_stroke(r.center(), 7.5, Stroke::new(1.2f32, MUTED()));
                        ui.painter().line_segment([r.center() + vec2(-5.0, 5.0), r.center() + vec2(5.0, -5.0)], Stroke::new(1.2f32, MUTED()));
                    }
                }
                if n.color.as_deref() == s {
                    ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(1.5f32, TEXT()));
                }
                if resp.clicked() {
                    cb.snapshot();
                    let ids = if cb.sel.contains(&id.to_string()) { cb.sel.clone() } else { vec![id.to_string()] };
                    for i in ids {
                        if let Some(m) = cb.doc.node_mut(&i) {
                            m.color = s.map(String::from);
                        }
                    }
                    cb.touch();
                    ui.close_menu();
                }
            }
        });
        ui.separator();
        if ui.button("Bring to front").clicked() {
            cb.snapshot();
            if let Some(k) = cb.doc.nodes.iter().position(|m| m.id == id) {
                let m = cb.doc.nodes.remove(k);
                cb.doc.nodes.push(m);
            }
            cb.touch();
            ui.close_menu();
        }
        if ui.button("Duplicate   Ctrl+D").clicked() {
            if !cb.sel.contains(&id.to_string()) {
                cb.sel = vec![id.to_string()];
            }
            cb.duplicate_selected();
            ui.close_menu();
        }
        if cb.sel.len() > 1 && ui.button("Group these").clicked() {
            self.cv_group(cb);
            ui.close_menu();
        }
        if ui.button(RichText::new("Remove   Del").color(kit::C_RED)).clicked() {
            if !cb.sel.contains(&id.to_string()) {
                cb.sel = vec![id.to_string()];
            }
            cb.remove_selected();
            ui.close_menu();
        }
    }

    /// The board's (or an arrow's) right-click menu.
    fn cv_board_menu(&mut self, ui: &mut egui::Ui, cb: &mut CanvasBuf, acts: &mut Vec<CvAct>) {
        let _ = acts;
        if let Some(eid) = cb.sel_edge.clone() {
            let Some(e) = cb.doc.edges.iter().find(|e| e.id == eid).cloned() else { return };
            if ui.button("Name the arrow").clicked() {
                cb.label_edit = Some((eid.clone(), e.label.clone().unwrap_or_default()));
                ui.close_menu();
            }
            let both = e.from_end.as_deref() == Some("arrow");
            let none = e.to_end.as_deref() == Some("none");
            if ui.button(if both { "One arrowhead" } else { "Arrowheads at both ends" }).clicked() {
                cb.snapshot();
                if let Some(x) = cb.doc.edges.iter_mut().find(|x| x.id == eid) {
                    x.from_end = if both { None } else { Some("arrow".into()) };
                    x.to_end = None;
                }
                cb.touch();
                ui.close_menu();
            }
            if ui.button(if none { "Show the arrowhead" } else { "Just a line" }).clicked() {
                cb.snapshot();
                if let Some(x) = cb.doc.edges.iter_mut().find(|x| x.id == eid) {
                    x.to_end = if none { None } else { Some("none".into()) };
                    x.from_end = None;
                }
                cb.touch();
                ui.close_menu();
            }
            ui.horizontal(|ui| {
                for s in SWATCHES {
                    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                    match s.and_then(jcanvas::color_rgb) {
                        Some((r0, g, b)) => {
                            ui.painter().circle_filled(r.center(), 8.0, Color32::from_rgb(r0, g, b));
                        }
                        None => {
                            ui.painter().circle_stroke(r.center(), 7.5, Stroke::new(1.2f32, MUTED()));
                        }
                    }
                    if resp.clicked() {
                        cb.snapshot();
                        if let Some(x) = cb.doc.edges.iter_mut().find(|x| x.id == eid) {
                            x.color = s.map(String::from);
                        }
                        cb.touch();
                        ui.close_menu();
                    }
                }
            });
            if ui.button(RichText::new("Remove arrow").color(kit::C_RED)).clicked() {
                cb.sel.clear();
                cb.remove_selected();
                ui.close_menu();
            }
            return;
        }
        let at = cb.menu_at.unwrap_or_else(|| cb.view_center());
        if ui.button("Add a card here").clicked() {
            cb.snapshot();
            let n = jcanvas::text_node(jcanvas::snap(at.x - 130.0), jcanvas::snap(at.y - 40.0), 260.0, 100.0, "");
            cb.sel = vec![n.id.clone()];
            cb.editing = Some(n.id.clone());
            cb.focus_edit = true;
            cb.doc.nodes.push(n);
            cb.touch();
            ui.close_menu();
        }
        if ui.button("Add a group here").clicked() {
            cb.snapshot();
            let g = jcanvas::group_node(jcanvas::snap(at.x - 300.0), jcanvas::snap(at.y - 200.0), 600.0, 400.0, "Group");
            cb.sel = vec![g.id.clone()];
            // groups go under the cards
            cb.doc.nodes.insert(0, g);
            cb.touch();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Show everything").clicked() {
            cb.fit(1.0);
            ui.close_menu();
        }
        if ui.add_enabled(!cb.undo.is_empty(), egui::Button::new("Undo   Ctrl+Z")).clicked() {
            cb.undo();
            ui.close_menu();
        }
        if ui.add_enabled(!cb.redo.is_empty(), egui::Button::new("Redo   Ctrl+Y")).clicked() {
            cb.redo();
            ui.close_menu();
        }
    }

    /// Put a group around what's selected (or an empty one in the middle).
    fn cv_group(&mut self, cb: &mut CanvasBuf) {
        let picked: Vec<&Node> = cb.doc.nodes.iter().filter(|n| cb.sel.contains(&n.id)).collect();
        let g = if picked.is_empty() {
            let c = cb.view_center();
            jcanvas::group_node(c.x - 300.0, c.y - 200.0, 600.0, 400.0, "Group")
        } else {
            let x0 = picked.iter().map(|n| n.x).fold(f32::MAX, f32::min) - 40.0;
            let y0 = picked.iter().map(|n| n.y).fold(f32::MAX, f32::min) - 40.0;
            let x1 = picked.iter().map(|n| n.x + n.width).fold(f32::MIN, f32::max) + 40.0;
            let y1 = picked.iter().map(|n| n.y + n.height).fold(f32::MIN, f32::max) + 40.0;
            jcanvas::group_node(jcanvas::snap(x0), jcanvas::snap(y0), jcanvas::snap(x1 - x0), jcanvas::snap(y1 - y0), "Group")
        };
        cb.snapshot();
        cb.sel = vec![g.id.clone()];
        cb.label_edit = Some((g.id.clone(), "Group".into()));
        cb.doc.nodes.insert(0, g);
        cb.touch();
    }

    /// Typing a group's or arrow's name, where it is.
    fn cv_label_editor(&mut self, ui: &mut egui::Ui, cb: &mut CanvasBuf) {
        let Some((id, mut text)) = cb.label_edit.clone() else { return };
        let at = if let Some(n) = cb.doc.node(&id) {
            cb.to_screen(n.x, n.y) - vec2(0.0, 34.0)
        } else if let Some(e) = cb.doc.edges.iter().find(|e| e.id == id) {
            match cb.edge_points(e) {
                Some(pts) => CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::NONE).sample(0.5) - vec2(90.0, 14.0),
                None => {
                    cb.label_edit = None;
                    return;
                }
            }
        } else {
            cb.label_edit = None;
            return;
        };
        let r = Rect::from_min_size(at, vec2(180.0, 28.0));
        ui.painter().rect_filled(r.expand(3.0), 8.0, CARD());
        ui.painter().rect_stroke(r.expand(3.0), 8.0, Stroke::new(1.5f32, JOURNAL_COL));
        let tid = egui::Id::new(("jc-label", &cb.rel));
        let resp = ui.put(r, egui::TextEdit::singleline(&mut text).id(tid).frame(false).hint_text("Name").desired_width(176.0));
        if !resp.has_focus() && !resp.lost_focus() && resp.changed() == false && ui.memory(|m| m.focused().is_none()) {
            resp.request_focus();
        }
        let done = resp.lost_focus();
        if done {
            let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
            cb.label_edit = None;
            if !cancel {
                cb.snapshot();
                let v = text.trim().to_string();
                if let Some(n) = cb.doc.node_mut(&id) {
                    n.label = Some(if v.is_empty() { "Group".into() } else { v });
                } else if let Some(e) = cb.doc.edges.iter_mut().find(|e| e.id == id) {
                    e.label = if v.is_empty() { None } else { Some(v) };
                }
                cb.touch();
            }
        } else {
            cb.label_edit = Some((id, text));
        }
    }

    /// The toolbar, zoom buttons, minimap and the toolbar's pickers.
    fn cv_controls(&mut self, ui: &mut egui::Ui, cb: &mut CanvasBuf, acts: &mut Vec<CvAct>) {
        let full = cb.view.0;
        let rel_id = cb.rel.clone();
        let rid = move |s: &str| egui::Id::new(("jc-ctl", rel_id.clone(), s.to_string()));
        // ── the card bar, bottom centre (icons only, as Obsidian) ──
        let tools: [(&str, &str, Option<Picker>); 6] = [
            ("card", "Add a card (or double-click the board)", None),
            ("file-text", "Add a note from your Journal", Some(Picker::Note)),
            ("image-file", "Add a picture", Some(Picker::Image)),
            ("widget", "Add a live widget", None),
            ("snippet", "Add a snippet from Game Info", Some(Picker::Snippet)),
            ("group", "Group what's selected", None),
        ];
        let bw = 40.0;
        let tw = bw * tools.len() as f32 + 12.0;
        let bar = Rect::from_center_size(pos2(full.center().x, full.max.y - 34.0), vec2(tw, 48.0));
        let side = super::journal_ui::side_bg();
        ui.painter().add(kit::shadow_tier(2).as_shape(bar, 8.0));
        ui.painter().rect_filled(bar, 8.0, side);
        ui.painter().rect_stroke(bar, 8.0, Stroke::new(1.0f32, LINE()));
        ui.interact(bar, rid("bar"), Sense::click());
        let mut picker_x = None;
        for (k, (icon, tip, pk)) in tools.iter().enumerate() {
            let r = Rect::from_min_size(pos2(bar.min.x + 6.0 + k as f32 * bw, bar.min.y + 4.0), vec2(bw, 40.0));
            let on = pk.is_some() && cb.picker == *pk;
            if on {
                picker_x = Some(r.center().x);
            }
            if super::journal_ui::icon_btn(ui, r.shrink(2.0), ("cv-tool", &cb.rel, k), icon, tip, on).clicked() {
                match (*icon, *pk) {
                    ("card", _) => {
                        let n = jcanvas::text_node(0.0, 0.0, 260.0, 100.0, "");
                        let id = n.id.clone();
                        cb.add(vec![n]);
                        cb.editing = Some(id);
                        cb.focus_edit = true;
                        cb.picker = None;
                    }
                    ("group", _) => {
                        self.cv_group(cb);
                        cb.picker = None;
                    }
                    ("widget", _) => {
                        // the Dashboard's own Add widgets window, adding here
                        let g = &mut self.setup.home.gallery;
                        g.open = true;
                        g.layouts = false;
                        g.query.clear();
                        g.to_canvas = Some(cb.rel.clone());
                        cb.picker = None;
                    }
                    (_, Some(k)) => {
                        cb.picker = if cb.picker == Some(k) { None } else { Some(k) };
                        cb.pick_q.clear();
                        cb.focus_pick = true;
                    }
                    _ => {}
                }
            }
        }

        // ── the controls, top right, in little stacks (as Obsidian) ──
        let x = full.max.x - 22.0;
        let mut y = full.min.y + 16.0;
        let stacks: [&[(&str, &str)]; 4] = [
            &[("settings", "Canvas options")],
            &[("plus", "Zoom in"), ("reset", "Back to 100%"), ("frame", "Show everything"), ("minus", "Zoom out")],
            &[("undo", "Undo (Ctrl+Z)"), ("redo", "Redo (Ctrl+Y)")],
            &[("help", "How canvases work")],
        ];
        let mut settings_resp = None;
        let mut help_resp = None;
        for (si, stack) in stacks.iter().enumerate() {
            let sr = Rect::from_min_size(pos2(x - 17.0, y), vec2(34.0, stack.len() as f32 * 32.0 + 2.0));
            ui.painter().rect_filled(sr, 6.0, side);
            ui.painter().rect_stroke(sr, 6.0, Stroke::new(1.0f32, LINE()));
            ui.interact(sr, rid(&format!("stack{si}")), Sense::click());
            for (k, (icon, tip)) in stack.iter().enumerate() {
                let br = Rect::from_min_size(pos2(sr.min.x + 2.0, sr.min.y + 1.0 + k as f32 * 32.0), vec2(30.0, 32.0));
                let enabled = match *icon {
                    "undo" => !cb.undo.is_empty(),
                    "redo" => !cb.redo.is_empty(),
                    _ => true,
                };
                let tip = if *icon == "reset" { format!("Back to 100% (now {:.0}%)", cb.zoom * 100.0) } else { tip.to_string() };
                let resp = super::journal_ui::icon_btn_en(ui, br.shrink(2.0), ("cv-ctl", &cb.rel, *icon), icon, &tip, false, enabled);
                let c = full.center();
                if resp.clicked() {
                    match *icon {
                        "plus" => cb.zoom_at(c, 1.25),
                        "minus" => cb.zoom_at(c, 0.8),
                        "reset" => {
                            let f = 1.0 / cb.zoom;
                            cb.zoom_at(c, f);
                        }
                        "frame" => cb.fit(1.0),
                        "undo" => cb.undo(),
                        "redo" => cb.redo(),
                        _ => {}
                    }
                }
                if *icon == "settings" {
                    settings_resp = Some(resp);
                } else if *icon == "help" {
                    help_resp = Some(resp);
                }
            }
            y = sr.max.y + 10.0;
        }
        if let Some(resp) = settings_resp {
            let pid = rid("cv-settings");
            if resp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pid));
            }
            egui::popup::popup_above_or_below_widget(ui, pid, &resp, egui::AboveOrBelow::Below, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(200.0);
                if ui.button("Show everything").clicked() {
                    cb.fit(1.0);
                }
                if ui.button("Select every card   Ctrl+A").clicked() {
                    cb.sel = cb.doc.nodes.iter().map(|n| n.id.clone()).collect();
                }
                if ui.add_enabled(!cb.sel.is_empty(), egui::Button::new("Group the selection")).clicked() {
                    self.cv_group(cb);
                }
                ui.separator();
                ui.label(RichText::new(format!("{} cards · {} arrows · {:.0}%", cb.doc.nodes.len(), cb.doc.edges.len(), cb.zoom * 100.0)).size(11.5).color(MUTED()));
            });
        }
        if let Some(resp) = help_resp {
            let pid = rid("cv-help");
            if resp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pid));
            }
            egui::popup::popup_above_or_below_widget(ui, pid, &resp, egui::AboveOrBelow::Below, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(260.0);
                for (k, v) in [
                    ("Move around", "Drag the board, or scroll"),
                    ("Zoom", "Ctrl + scroll"),
                    ("New card", "Double-click the board"),
                    ("Edit a card", "Double-click it"),
                    ("Resize", "Drag a corner"),
                    ("Arrow", "Drag a dot on a card's side"),
                    ("Select a few", "Shift + drag, or Shift + click"),
                    ("Colours, names", "Right-click"),
                    ("Duplicate · Remove", "Ctrl+D · Delete"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(k).size(12.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(v).size(12.0).color(MUTED())));
                    });
                }
            });
        }

        // ── a picker over the toolbar ──
        if let (Some(k), Some(px)) = (cb.picker, picker_x) {
            let pw = 340.0;
            let x = (px - pw / 2.0).clamp(full.min.x + 8.0, full.max.x - pw - 8.0) + pw / 2.0;
            egui::Area::new(rid("picker")).order(egui::Order::Foreground).pivot(Align2::CENTER_BOTTOM).fixed_pos(pos2(x, bar.min.y - 10.0)).show(ui.ctx(), |ui| {
                egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(14.0).shadow(kit::shadow_tier(4)).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                    ui.set_width(pw - 24.0);
                    let iw = pw - 24.0;
                    let title = match k {
                        Picker::Note => "Put a note on the canvas",
                        Picker::Image => "Put a picture on the canvas",
                        Picker::Snippet => "A snippet from Game Info",
                    };
                    ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                    ui.add_space(6.0);
                    if k != Picker::Image {
                        let hint = match k {
                            Picker::Note => "Find a note",
                            _ => "A Pokémon, move, item, place…",
                        };
                        let sr = kit::search_box_keyed(ui, Some(rid("pick-q")), &mut cb.pick_q, hint, iw);
                        if std::mem::take(&mut cb.focus_pick) {
                            sr.request_focus();
                        }
                        ui.add_space(6.0);
                    }
                    let q = cb.pick_q.trim().to_lowercase();
                    egui::ScrollArea::vertical().id_salt(rid("pick-list")).max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.set_width(iw);
                        let row = |ui: &mut egui::Ui, title: &str, sub: &str, icon: &str, col: Color32| -> bool {
                            let (r, resp) = ui.allocate_exact_size(vec2(iw, 34.0), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(r, 8.0, CARD_HOVER());
                            }
                            if !icon.is_empty() {
                                kit::icon_chip(ui.painter(), Rect::from_center_size(pos2(r.min.x + 16.0, r.center().y), vec2(24.0, 24.0)), icon, col, false);
                            }
                            let f = crate::fonts::sans_font(12.5);
                            let t = super::charts::clip(ui, title, &f, iw - 150.0);
                            ui.painter().text(pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, t, f, TEXT());
                            let sf = crate::fonts::sans_font(11.0);
                            let s = super::charts::clip(ui, sub, &sf, 100.0);
                            ui.painter().text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, s, sf, MUTED());
                            resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                        };
                        match k {
                            Picker::Note => {
                                let mut notes: Vec<(String, u64)> = self.setup.journal.index.iter().map(|(r, i)| (r.clone(), i.modified)).collect();
                                notes.sort_by(|a, b| b.1.cmp(&a.1));
                                for c in self.setup.journal.canvases.clone() {
                                    if c != cb.rel {
                                        notes.push((c, 0));
                                    }
                                }
                                let mut shown = 0;
                                for (rel, _) in notes {
                                    if !q.is_empty() && !j::title_of(&rel).to_lowercase().contains(&q) {
                                        continue;
                                    }
                                    let folder = rel.rsplit_once('/').map(|(f, _)| f.to_string()).unwrap_or_default();
                                    let sub = if j::is_canvas(&rel) { "canvas".to_string() } else { folder };
                                    if row(ui, &j::title_of(&rel), &sub, "📄", JOURNAL_COL) {
                                        cb.add(vec![jcanvas::file_node(0.0, 0.0, if j::is_canvas(&rel) { 300.0 } else { 400.0 }, if j::is_canvas(&rel) { 100.0 } else { 300.0 }, &rel)]);
                                        cb.picker = None;
                                    }
                                    shown += 1;
                                    if shown > 40 {
                                        break;
                                    }
                                }
                                if shown == 0 {
                                    ui.label(RichText::new("No notes with that name.").size(12.0).color(MUTED()));
                                }
                            }
                            Picker::Image => {
                                if ui.add_sized(vec2(iw, 32.0), egui::Button::new("🖼  Pick a picture from your PC…").fill(kit::btn_fill())).clicked() {
                                    acts.push(CvAct::PickImage);
                                    cb.picker = None;
                                }
                                ui.add_space(8.0);
                                let imgs = self.setup.journal.images.clone();
                                if imgs.is_empty() {
                                    ui.label(RichText::new("No pictures in your Journal yet. Share images you add to the Journal land here.").size(12.0).color(MUTED()));
                                } else {
                                    ui.label(RichText::new("IN YOUR JOURNAL").size(10.5).strong().color(MUTED()));
                                }
                                for rel in imgs {
                                    let name = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                                    if row(ui, &name, "", "🖼", kit::C_ORANGE) {
                                        let (w, h) = image_size(&rel);
                                        cb.add(vec![jcanvas::file_node(0.0, 0.0, w, h, &rel)]);
                                        cb.picker = None;
                                    }
                                }
                            }
                            Picker::Snippet => {
                                if q.is_empty() {
                                    ui.label(RichText::new("QUICK").size(10.5).strong().color(MUTED()));
                                    if row(ui, "Your current hunt", "", "✨", kit::C_GOLD) {
                                        acts.push(CvAct::AddHunt);
                                        cb.picker = None;
                                    }
                                    if row(ui, "Rare grass today", "", "🌿", kit::C_GREEN) {
                                        acts.push(CvAct::AddRare);
                                        cb.picker = None;
                                    }
                                    ui.add_space(6.0);
                                    ui.label(RichText::new("Or type to find anything in Game Info.").size(12.0).color(MUTED()));
                                } else if let Some(w) = self.wiki.clone() {
                                    let hits = self.info_quick(&w, &q, 12);
                                    if hits.is_empty() {
                                        ui.label(RichText::new("Nothing found.").size(12.0).color(MUTED()));
                                    }
                                    for (kind, key, icon, col, title, sub) in hits {
                                        if row(ui, &title, &sub, icon, col) {
                                            acts.push(CvAct::AddSnippet(kind, key));
                                            cb.picker = None;
                                        }
                                    }
                                } else {
                                    ui.label(RichText::new("Game Info is still loading.").size(12.0).color(MUTED()));
                                }
                            }
                        }
                    });
                });
            });
        }
    }
}

impl Drag {
    fn is_some_moving(&self) -> bool {
        !matches!(self, Drag::None)
    }
}

/// A number from a card's id (for a widget's own id).
fn id_num(id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    8_000_000 + h.finish() % 1_000_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> CanvasBuf {
        let mut cb = CanvasBuf { rel: "Boards/Test.canvas".into(), zoom: 1.0, ..Default::default() };
        cb.view = View(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0)));
        cb
    }

    #[test]
    fn new_cards_go_in_clear_space_and_undo_brings_it_back() {
        let mut cb = board();
        cb.add(vec![jcanvas::text_node(0.0, 0.0, 260.0, 100.0, "one")]);
        cb.add(vec![jcanvas::text_node(0.0, 0.0, 260.0, 100.0, "two")]);
        assert_eq!(cb.doc.nodes.len(), 2);
        let (a, b) = (&cb.doc.nodes[0], &cb.doc.nodes[1]);
        let ra = Rect::from_min_size(pos2(a.x, a.y), vec2(a.width, a.height));
        let rb = Rect::from_min_size(pos2(b.x, b.y), vec2(b.width, b.height));
        assert!(!ra.intersects(rb), "the second card doesn't sit on the first");
        assert_eq!(cb.sel, vec![b.id.clone()], "the new card is selected");
        assert_eq!(a.x % 20.0, 0.0, "on the grid");
        cb.undo();
        assert_eq!(cb.doc.nodes.len(), 1);
        cb.redo();
        assert_eq!(cb.doc.nodes.len(), 2);
    }

    #[test]
    fn removing_takes_arrows_and_duplicating_copies() {
        let mut cb = board();
        let a = jcanvas::text_node(0.0, 0.0, 200.0, 100.0, "a");
        let b = jcanvas::text_node(400.0, 0.0, 200.0, 100.0, "b");
        cb.doc.edges.push(jcanvas::edge(&a, &b));
        cb.doc.nodes = vec![a.clone(), b.clone()];
        cb.sel = vec![a.id.clone()];
        cb.duplicate_selected();
        assert_eq!(cb.doc.nodes.len(), 3);
        assert_eq!((cb.doc.nodes[2].x, cb.doc.nodes[2].text.as_deref()), (40.0, Some("a")));
        cb.sel = vec![b.id.clone()];
        cb.remove_selected();
        assert!(cb.doc.edges.is_empty(), "the arrow to it went too");
        assert_eq!(cb.doc.nodes.len(), 2);
        cb.undo();
        assert_eq!(cb.doc.edges.len(), 1);
    }

    #[test]
    fn the_view_maps_both_ways_and_zooms_round_the_pointer() {
        let mut cb = board();
        cb.off = vec2(100.0, 50.0);
        let s = cb.to_screen(10.0, 20.0);
        assert_eq!(s, pos2(110.0, 70.0));
        let at = pos2(300.0, 300.0);
        let before = cb.to_world(at);
        cb.zoom_at(at, 2.0);
        let after = cb.to_world(at);
        assert!((before - after).length() < 0.001, "the point under the pointer stays put");
        assert_eq!(cb.zoom, 2.0);
        cb.zoom_at(at, 100.0);
        assert_eq!(cb.zoom, 3.0, "zoom is capped");
        assert_eq!(widget_size(4, 2), (436.0, 212.0));
    }
}
