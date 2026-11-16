//! Sticker Book: your Pokémon as stickers on a scrapbook page. Your shinies
//! (they shimmer), your party and your boxes (read from the save, never
//! written) and milestone ribbons are in the tray: click one to stick it
//! on, or drag it onto the page. Drag stickers around, stack them (the one
//! you touched last goes on top), right-click one for more. There's a page
//! for each game world, and you can add more pages.

use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub(super) const STICKER_COL: Color32 = Color32::from_rgb(255, 122, 144);
const SHINY_GOLD: Color32 = Color32::from_rgb(242, 193, 78);
/// A Pokémon sticker's size at scale 1 on a 900-wide page.
const BASE: f32 = 76.0;

// ───────────────────────────── the file ─────────────────────────────

/// Everything the Sticker Book keeps (sticker-book.json in the data folder).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerFile {
    pub pages: Vec<Page>,
    /// The paper: 0 dotted, 1 grid, 2 kraft, 3 night sky.
    pub paper: u8,
    /// Every sticker's size (0: the usual).
    pub size: f32,
    /// Stickers hold still instead of moving like the sprites do.
    pub still: bool,
    pub next_id: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub id: u64,
    pub name: String,
    /// The game world it belongs to (None: a page you added).
    pub world: Option<u32>,
    /// Bottom to top: the last one is on top.
    pub stickers: Vec<Sticker>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sticker {
    pub id: u64,
    /// A Pokémon (its sprite's slug)…
    pub slug: String,
    pub shiny: bool,
    /// …or a milestone ribbon (its id).
    pub ribbon: String,
    /// Shown when you point at it: its nickname, the hunt, where it's from.
    pub label: String,
    /// Its middle, as a share of the page's width and height.
    pub x: f32,
    pub y: f32,
    /// Tilt, in degrees.
    pub rot: f32,
    /// 1 = the usual size.
    pub scale: f32,
}

impl StickerFile {
    fn has_data(&self) -> bool {
        self.pages.iter().any(|p| !p.stickers.is_empty())
    }
    fn size(&self) -> f32 {
        if self.size > 0.0 {
            self.size
        } else {
            1.0
        }
    }
    fn new_id(&mut self) -> u64 {
        self.next_id = self.next_id.max(self.pages.iter().flat_map(|p| p.stickers.iter().map(|s| s.id).chain([p.id])).max().unwrap_or(0)) + 1;
        self.next_id
    }
}

fn load_from(dir: &std::path::Path) -> (StickerFile, Option<String>) {
    crate::safefile::load(dir, "sticker-book", "Sticker Book")
}

fn save_to(dir: &std::path::Path, f: &StickerFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "sticker-book", f, first_this_run, StickerFile::has_data)
}

// ───────────────────────────── ribbons ─────────────────────────────

struct Ribbon {
    id: &'static str,
    text: &'static str,
    col: Color32,
    /// How to earn it (shown while it's locked).
    how: &'static str,
}

static RIBBONS: [Ribbon; 12] = [
    Ribbon { id: "first_shiny", text: "First shiny!", col: Color32::from_rgb(245, 158, 11), how: "Catch your first shiny" },
    Ribbon { id: "shiny_5", text: "5 shinies", col: Color32::from_rgb(236, 72, 153), how: "Catch 5 shinies" },
    Ribbon { id: "shiny_10", text: "10 shinies", col: Color32::from_rgb(139, 92, 246), how: "Catch 10 shinies" },
    Ribbon { id: "shiny_25", text: "25 shinies", col: Color32::from_rgb(14, 165, 233), how: "Catch 25 shinies" },
    Ribbon { id: "shiny_50", text: "50 shinies", col: Color32::from_rgb(202, 138, 4), how: "Catch 50 shinies" },
    Ribbon { id: "enc_1k", text: "1,000 encounters", col: Color32::from_rgb(124, 58, 237), how: "Reach 1,000 encounters in all" },
    Ribbon { id: "enc_10k", text: "10,000 encounters", col: Color32::from_rgb(79, 70, 229), how: "Reach 10,000 encounters in all" },
    Ribbon { id: "enc_100k", text: "100,000 encounters", col: Color32::from_rgb(30, 58, 138), how: "Reach 100,000 encounters in all" },
    Ribbon { id: "day_1k", text: "1,000 in a day", col: Color32::from_rgb(220, 38, 38), how: "Meet 1,000 Pokémon in one day" },
    Ribbon { id: "odds_2x", text: "Past 2× the odds", col: Color32::from_rgb(190, 18, 60), how: "Keep a hunt going past twice the odds" },
    Ribbon { id: "streak_7", text: "7-day streak", col: Color32::from_rgb(22, 163, 74), how: "Hunt 7 days in a row" },
    Ribbon { id: "hours_100", text: "100 hours hunted", col: Color32::from_rgb(15, 118, 110), how: "Hunt for 100 hours in all" },
];

fn ribbon(id: &str) -> Option<&'static Ribbon> {
    RIBBONS.iter().find(|r| r.id == id)
}

fn ribbon_earned(id: &str, l: &crate::progress::Lifetime) -> bool {
    match id {
        "first_shiny" => l.shinies >= 1,
        "shiny_5" => l.shinies >= 5,
        "shiny_10" => l.shinies >= 10,
        "shiny_25" => l.shinies >= 25,
        "shiny_50" => l.shinies >= 50,
        "enc_1k" => l.encounters >= 1_000,
        "enc_10k" => l.encounters >= 10_000,
        "enc_100k" => l.encounters >= 100_000,
        "day_1k" => l.best_day >= 1_000,
        "odds_2x" => l.best_run_odds_x >= 2,
        "streak_7" => l.longest_streak >= 7,
        "hours_100" => l.hunt_secs >= 100 * 3600,
        _ => false,
    }
}

// ───────────────────────────── state ─────────────────────────────

/// Something that can become a sticker.
#[derive(Clone, PartialEq)]
pub(super) struct Spec {
    slug: String,
    shiny: bool,
    ribbon: String,
    label: String,
    /// Its display name (the tray's search).
    name: String,
}

#[derive(Default)]
pub(super) struct StickerState {
    file: Option<StickerFile>,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
    dirty: Option<Instant>,
    saved_once: bool,
    /// The page shown (its id).
    page: Option<u64>,
    pub settings_open: bool,
    /// A sticker being moved: its id and where it was grabbed (from its middle).
    drag: Option<(u64, Vec2)>,
    /// Something being dragged out of the tray.
    carry: Option<Spec>,
    /// The sticker that was right-clicked.
    menu: Option<u64>,
    /// When stickers were stuck on (a little pop).
    added: HashMap<u64, Instant>,
    /// The last sticker peeled off, for Undo: (page, sticker, at).
    undo: Option<(u64, Sticker, Instant)>,
    /// Where the page was last drawn (drops from the tray land here).
    page_rect: Option<Rect>,
    /// The tray's tab: 0 shinies, 1 party, 2 boxes, 3 ribbons.
    tray: u8,
    search: String,
    shiny_only: bool,
    /// Ribbons earned (worked out every few seconds).
    earned: Vec<&'static str>,
    earned_at: Option<Instant>,
    /// The tray's Pokémon from the save, kept until the save changes: (save time, party, boxes).
    save_items: Option<(u64, Vec<Spec>, Vec<Spec>)>,
    /// The tray's shinies, kept until the hunt history changes: (count, items).
    shiny_items: Option<(usize, Vec<Spec>)>,
    /// Settings: the page's new name being typed, and Clear page asked once.
    rename: Option<(u64, String)>,
    confirm_clear: bool,
}

impl StickerState {
    fn file(&mut self) -> &mut StickerFile {
        if self.file.is_none() {
            let (f, note) = load_from(&crate::config::data_dir());
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(1500) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match save_to(&crate::config::data_dir(), f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(format!("Your stickers couldn't be saved: {e}"));
                self.dirty = Some(Instant::now());
            }
        }
    }

    /// The page shown (the first one if the picked one is gone).
    fn page_mut(&mut self) -> Option<&mut Page> {
        let want = self.page;
        let f = self.file();
        let i = want.and_then(|id| f.pages.iter().position(|p| p.id == id)).unwrap_or(0);
        f.pages.get_mut(i)
    }
}

// ───────────────────────────── little helpers ─────────────────────────────

/// A number 0..1 that's different every call.
fn rnd() -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0);
    let mut s = SEED.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E37_79B9) | 1;
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    SEED.store(s, Ordering::Relaxed);
    (s >> 11) as f32 / (1u64 << 53) as f32
}

/// `v` turned by `a` radians (clockwise on screen).
fn turn(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// A rounded rectangle around `c` (half size `half`, corner `rad`), turned by `a`.
fn rounded(c: Pos2, half: Vec2, rad: f32, a: f32) -> Vec<Pos2> {
    let rad = rad.min(half.x).min(half.y);
    let corners = [(vec2(half.x - rad, -(half.y - rad)), -90.0f32), (vec2(half.x - rad, half.y - rad), 0.0), (vec2(-(half.x - rad), half.y - rad), 90.0), (vec2(-(half.x - rad), -(half.y - rad)), 180.0)];
    let mut pts = Vec::with_capacity(24);
    for (k, start) in corners {
        for i in 0..=5 {
            let t = (start + 90.0 * i as f32 / 5.0).to_radians();
            pts.push(c + turn(k + vec2(t.cos(), t.sin()) * rad, a));
        }
    }
    pts
}

/// Light that adds to what's under it (alpha 0 adds in egui's premultiplied colours).
fn glow(c: Color32, k: f32) -> Color32 {
    let [r, g, b, _] = c.to_array();
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(f(r), f(g), f(b), 0)
}

/// How big a sticker is on screen: its half width and height.
fn half_size(p: &egui::Painter, s: &Sticker, base: f32) -> Vec2 {
    let k = s.scale.max(0.3);
    if !s.ribbon.is_empty() {
        let text = ribbon(&s.ribbon).map_or("Ribbon", |r| r.text);
        let g = p.layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(13.0 * base / BASE * k), Color32::WHITE);
        vec2(g.size().x / 2.0 + 18.0 * base / BASE * k, g.size().y / 2.0 + 13.0 * base / BASE * k)
    } else {
        Vec2::splat(base * k / 2.0)
    }
}

/// Is `q` on the sticker (middle `c`, half size `h`, tilt `a`)?
fn inside(q: Pos2, c: Pos2, h: Vec2, a: f32) -> bool {
    let l = turn(q - c, -a);
    l.x.abs() <= h.x && l.y.abs() <= h.y
}

/// The paper.
fn paper(p: &egui::Painter, r: Rect, kind: u8, round: f32) {
    let (fill, mark) = match kind {
        1 => (Color32::from_rgb(244, 247, 251), Color32::from_rgb(214, 226, 240)),
        2 => (Color32::from_rgb(200, 164, 122), Color32::from_rgb(171, 133, 92)),
        3 => (Color32::from_rgb(27, 33, 64), Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
        _ => (Color32::from_rgb(246, 239, 224), Color32::from_rgb(217, 207, 189)),
    };
    p.rect_filled(r, round, fill);
    let inner = r.shrink(round * 0.6);
    let mut m = egui::Mesh::default();
    let mut dot = |c: Pos2, rad: f32, col: Color32| {
        let i = m.vertices.len() as u32;
        m.colored_vertex(c + vec2(-rad, -rad), col);
        m.colored_vertex(c + vec2(rad, -rad), col);
        m.colored_vertex(c + vec2(rad, rad), col);
        m.colored_vertex(c + vec2(-rad, rad), col);
        m.add_triangle(i, i + 1, i + 2);
        m.add_triangle(i, i + 2, i + 3);
    };
    match kind {
        1 => {
            let step = 22.0;
            let mut x = inner.min.x + step;
            while x < inner.max.x {
                p.line_segment([pos2(x, inner.min.y), pos2(x, inner.max.y)], Stroke::new(1.0f32, mark));
                x += step;
            }
            let mut y = inner.min.y + step;
            while y < inner.max.y {
                p.line_segment([pos2(inner.min.x, y), pos2(inner.max.x, y)], Stroke::new(1.0f32, mark));
                y += step;
            }
        }
        2 | 3 => {
            // speckles (kraft) or stars (night), the same every time
            let mut s: u64 = 0x2545_F491_4F6C_DD1D;
            let mut next = move || {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                (s >> 11) as f32 / (1u64 << 53) as f32
            };
            let n = (inner.area() / if kind == 2 { 260.0 } else { 900.0 }) as usize;
            for _ in 0..n {
                let c = pos2(inner.min.x + next() * inner.width(), inner.min.y + next() * inner.height());
                let rad = if kind == 2 { 0.6 + next() * 1.0 } else { 0.4 + next().powi(3) * 1.6 };
                let a = 0.3 + next() * 0.7;
                dot(c, rad, mark.gamma_multiply(a));
            }
        }
        _ => {
            let step = 16.0;
            let mut y = inner.min.y + step / 2.0;
            while y < inner.max.y {
                let mut x = inner.min.x + step / 2.0;
                while x < inner.max.x {
                    dot(pos2(x, y), 1.1, mark);
                    x += step;
                }
                y += step;
            }
        }
    }
    p.add(m);
}

// ───────────────────────────── the app ─────────────────────────────

impl App {
    /// Every frame: save a moment after a change.
    pub(super) fn stickers_tick(&mut self) {
        self.setup.stickers.save_tick(false);
    }

    /// A page for every world (and one to start with when there are none).
    fn stickers_pages_sync(&mut self) {
        let mut worlds: Vec<(u32, String)> = self.cfg.world_tally.iter().map(|(id, w)| (*id, w.name.trim().to_string())).collect();
        if let Some(now) = self.world_now() {
            if !worlds.iter().any(|w| w.0 == now) {
                worlds.push((now, String::new()));
            }
        }
        let st = &mut self.setup.stickers;
        let f = st.file();
        let mut changed = false;
        for (id, name) in worlds {
            match f.pages.iter().position(|p| p.world == Some(id)) {
                Some(i) => {
                    // a world named in the game since: take its name (unless you renamed the page)
                    let p = &mut f.pages[i];
                    if !name.is_empty() && p.name == format!("World {id}") {
                        p.name = name;
                        changed = true;
                    }
                }
                None => {
                    let pid = f.new_id();
                    f.pages.push(Page { id: pid, name: if name.is_empty() { format!("World {id}") } else { name }, world: Some(id), stickers: Vec::new() });
                    changed = true;
                }
            }
        }
        if f.pages.is_empty() {
            let pid = f.new_id();
            f.pages.push(Page { id: pid, name: "My stickers".into(), world: None, stickers: Vec::new() });
            changed = true;
        }
        if changed {
            st.touch();
        }
        // first time here: the world being played
        if st.page.is_none() {
            let now = self.world_now();
            let f = self.setup.stickers.file();
            let id = now.and_then(|w| f.pages.iter().find(|p| p.world == Some(w))).or_else(|| f.pages.first()).map(|p| p.id);
            self.setup.stickers.page = id;
        }
    }

    /// Which ribbons are earned (every few seconds).
    fn stickers_ribbons(&mut self) {
        if self.setup.stickers.earned_at.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
            return;
        }
        let l = self.lifetime();
        self.setup.stickers.earned = RIBBONS.iter().filter(|r| ribbon_earned(r.id, &l)).map(|r| r.id).collect();
        self.setup.stickers.earned_at = Some(Instant::now());
    }

    /// The tray's shinies (from your finished hunts), newest first.
    fn stickers_shinies(&mut self) -> Vec<Spec> {
        let n = self.cfg.history.len();
        if let Some((k, v)) = &self.setup.stickers.shiny_items {
            if *k == n {
                return v.clone();
            }
        }
        let v: Vec<Spec> = self
            .cfg
            .history
            .iter()
            .rev()
            .filter_map(|h| {
                let slug = h.slug.clone().unwrap_or_else(|| crate::species::slugify(&h.target));
                let sp = self.species.iter().find(|s| s.slug == slug)?;
                let how = if h.phase { "found during another hunt".to_string() } else { format!("{} encounters", crate::stats::thousands(h.encounters)) };
                Some(Spec { slug, shiny: true, ribbon: String::new(), label: format!("Shiny {} · {how} · {}", sp.name, crate::stats::fmt_date(h.when_unix)), name: sp.name.to_string() })
            })
            .collect();
        self.setup.stickers.shiny_items = Some((n, v.clone()));
        v
    }

    /// The tray's Pokémon from the save: (party, boxes). Eggs are left out.
    fn stickers_save_items(&mut self) -> Option<(Vec<Spec>, Vec<Spec>)> {
        let ms = self.setup.boxes.save_ms;
        if let Some((k, a, b)) = &self.setup.stickers.save_items {
            if *k == ms && self.setup.boxes.save.is_some() {
                return Some((a.clone(), b.clone()));
            }
        }
        let save = self.setup.boxes.save.clone()?;
        let spec = |me: &App, m: &crate::gamesave::Mon, place: &str| -> Option<Spec> {
            if m.egg {
                return None;
            }
            let sp = me.box_species(m.species)?;
            let shiny = m.shiny();
            let nick = if m.nicknamed && !m.nickname.trim().is_empty() { m.nickname.trim().to_string() } else { sp.name.to_string() };
            let lv = m.level.map(|l| format!(" · Lv {l}")).unwrap_or_default();
            Some(Spec { slug: sp.slug.clone(), shiny, ribbon: String::new(), label: format!("{}{nick}{lv} · {place}", if shiny { "Shiny " } else { "" }), name: format!("{nick} {}", sp.name) })
        };
        let party: Vec<Spec> = save.party.iter().filter_map(|m| spec(self, m, "your party")).collect();
        let boxes: Vec<Spec> = save
            .boxes
            .iter()
            .enumerate()
            .flat_map(|(i, b)| {
                let place = if b.name.trim().is_empty() { format!("Box {}", i + 1) } else { b.name.trim().to_string() };
                b.slots.iter().flatten().filter_map(move |m| Some((m.clone(), place.clone())))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|(m, place)| spec(self, &m, &place))
            .collect();
        self.setup.stickers.save_items = Some((ms, party.clone(), boxes.clone()));
        Some((party, boxes))
    }

    /// A sprite's picture right now: (texture, size, a still box sprite).
    fn sticker_tex(&mut self, slug: &str, shiny: bool, still: bool) -> Option<(egui::TextureId, Vec2, bool)> {
        let sp = self.species.iter().find(|s| s.slug == slug)?.clone();
        let t = if still { 0 } else { self.t_ms() };
        self.sprites.get(&sp, shiny).map(|a| (a.frame(t).id(), a.size, a.still))
    }

    /// Stick `spec` on the page shown: at `at` (screen), else near the middle.
    fn sticker_add(&mut self, spec: &Spec, at: Option<Pos2>) {
        let r = self.setup.stickers.page_rect;
        let (x, y) = match (at, r) {
            (Some(q), Some(r)) => (((q.x - r.min.x) / r.width()).clamp(0.04, 0.96), ((q.y - r.min.y) / r.height()).clamp(0.05, 0.95)),
            _ => (0.25 + rnd() * 0.5, 0.25 + rnd() * 0.5),
        };
        let st = &mut self.setup.stickers;
        let id = st.file().new_id();
        let Some(page) = st.page_mut() else { return };
        page.stickers.push(Sticker { id, slug: spec.slug.clone(), shiny: spec.shiny, ribbon: spec.ribbon.clone(), label: spec.label.clone(), x, y, rot: rnd() * 24.0 - 12.0, scale: 1.0 });
        st.added.insert(id, Instant::now());
        st.touch();
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_stickers(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "stickers");
        self.stickers_pages_sync();
        self.stickers_ribbons();
        ui.horizontal(|ui| {
            ui.vertical(|ui| kit::page_header(ui, "STICKER BOOK", "Your Pokémon as stickers: drag them onto the page, stack them, make a mess."));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                super::app_settings::cog_button(ui, &mut self.setup.stickers.settings_open, "Sticker Book settings: paper, sticker size, this page");
            });
        });
        for e in [self.setup.stickers.load_error.clone(), self.setup.stickers.save_error.clone()].into_iter().flatten() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(6.0);
        }
        let tour_top = ui.cursor().min.y;
        self.stickers_tabs(ui);
        super::app_tour::mark_since(ui, "stickers.tabs", tour_top);
        ui.add_space(10.0);
        let w = ui.available_width();
        let gap = 16.0;
        if w >= 760.0 {
            let tray_w = 300.0f32.min(w * 0.34);
            let page_w = w - tray_w - gap;
            let page_h = (page_w * 0.66).clamp(340.0, 760.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(page_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(page_w);
                    self.stickers_page(ui, page_w, page_h);
                    super::app_tour::mark(ui.ctx(), "stickers.page", ui.min_rect());
                });
                ui.allocate_ui_with_layout(vec2(tray_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(tray_w);
                    self.stickers_tray(ui, page_h);
                    super::app_tour::mark(ui.ctx(), "stickers.tray", ui.min_rect());
                });
            });
        } else {
            let page_h = (w * 0.75).clamp(320.0, 640.0);
            self.stickers_page(ui, w, page_h);
            ui.add_space(gap);
            self.stickers_tray(ui, 420.0);
        }
        ui.add_space(12.0);
        self.stickers_carry(ui.ctx());
        // the cog opens Settings → Sticker Book
        if std::mem::take(&mut self.setup.stickers.settings_open) {
            self.open_settings(super::Tab::Stickers);
        }
    }

    /// The pages along the top, New page, and Tidy up / Make a mess.
    fn stickers_tabs(&mut self, ui: &mut egui::Ui) {
        let now = self.world_now();
        let pages: Vec<(u64, String, Option<u32>, usize)> = self.setup.stickers.file().pages.iter().map(|p| (p.id, p.name.clone(), p.world, p.stickers.len())).collect();
        let shown = self.setup.stickers.page_mut().map(|p| p.id);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (id, name, world, n) in &pages {
                let on = Some(*id) == shown;
                let icon = if world.is_some() { "🌍" } else { "📄" };
                let label = if *n > 0 { format!("{icon} {name} · {n}") } else { format!("{icon} {name}") };
                let b = egui::Button::new(RichText::new(label).size(12.5).color(TEXT()))
                    .fill(if on { theme::blend(kit::CARD(), STICKER_COL, 0.18) } else { kit::btn_fill() })
                    .stroke(Stroke::new(1.0f32, if on { STICKER_COL } else { LINE() }))
                    .rounding(14.0);
                let tip = match world {
                    Some(w) if Some(*w) == now => "The world you're playing now",
                    Some(_) => "Another game world",
                    None => "A page you added",
                };
                if ui.add(b).on_hover_text(tip).clicked() {
                    self.setup.stickers.page = Some(*id);
                    self.setup.stickers.drag = None;
                }
            }
            let add = egui::Button::new(RichText::new("+ New page").size(12.5).color(MUTED())).fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0f32, LINE())).rounding(14.0);
            if ui.add(add).on_hover_text("A blank page of your own").clicked() {
                let st = &mut self.setup.stickers;
                let f = st.file();
                let n = f.pages.iter().filter(|p| p.world.is_none()).count() + 1;
                let id = f.new_id();
                f.pages.push(Page { id, name: format!("Page {n}"), world: None, stickers: Vec::new() });
                st.page = Some(id);
                st.touch();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let small = |t: &str| egui::Button::new(RichText::new(t).size(12.0).color(TEXT())).fill(kit::btn_fill()).stroke(Stroke::new(1.0f32, LINE())).rounding(10.0);
                if ui.add(small("Make a mess")).on_hover_text("Throw every sticker on this page somewhere new").clicked() {
                    let st = &mut self.setup.stickers;
                    if let Some(p) = st.page_mut() {
                        for s in &mut p.stickers {
                            s.x = 0.08 + rnd() * 0.84;
                            s.y = 0.1 + rnd() * 0.8;
                            s.rot = rnd() * 50.0 - 25.0;
                        }
                    }
                    st.touch();
                }
                if ui.add(small("Tidy up")).on_hover_text("Line this page's stickers up in rows").clicked() {
                    let st = &mut self.setup.stickers;
                    if let Some(p) = st.page_mut() {
                        let n = p.stickers.len().max(1);
                        let cols = ((n as f32 * 1.6).sqrt().ceil() as usize).clamp(1, 9);
                        let rows = n.div_ceil(cols);
                        for (i, s) in p.stickers.iter_mut().enumerate() {
                            let (cx, cy) = (i % cols, i / cols);
                            s.x = (cx as f32 + 0.5) / cols as f32 * 0.86 + 0.07;
                            s.y = (cy as f32 + 0.5) / rows as f32 * 0.8 + 0.1;
                            s.rot = rnd() * 6.0 - 3.0;
                        }
                    }
                    st.touch();
                }
                // Undo a peel, for a few seconds
                let undo_live = self.setup.stickers.undo.as_ref().is_some_and(|u| u.2.elapsed() < Duration::from_secs(8));
                if undo_live {
                    if ui.add(small("Undo")).on_hover_text("Put back the sticker you just peeled off").clicked() {
                        if let Some((pid, s, _)) = self.setup.stickers.undo.take() {
                            let st = &mut self.setup.stickers;
                            if let Some(p) = st.file().pages.iter_mut().find(|p| p.id == pid) {
                                p.stickers.push(s);
                            }
                            st.touch();
                        }
                    }
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                }
            });
        });
    }

    /// The page itself: paper, stickers, and moving them about.
    fn stickers_page(&mut self, ui: &mut egui::Ui, w: f32, h: f32) {
        let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        self.setup.stickers.page_rect = Some(r);
        let ctx = ui.ctx().clone();
        let p = ui.painter_at(r);
        let time = ui.input(|i| i.time) as f32;
        let motion = kit::motion();
        let (paper_kind, size, still) = {
            let f = self.setup.stickers.file();
            (f.paper, f.size(), f.still)
        };
        ui.painter().add(kit::shadow_tier(2).as_shape(r, 14.0));
        paper(&p, r, paper_kind, 14.0);
        let base = BASE * (w / 900.0).clamp(0.7, 1.3) * size;
        let page_id = self.setup.stickers.page_mut().map(|p| p.id).unwrap_or(0);
        let stickers: Vec<Sticker> = self.setup.stickers.page_mut().map(|p| p.stickers.clone()).unwrap_or_default();
        let centre = |s: &Sticker| pos2(r.min.x + s.x * r.width(), r.min.y + s.y * r.height());
        let hit = |q: Pos2| -> Option<u64> { stickers.iter().rev().find(|s| inside(q, centre(s), half_size(&p, s, base), s.rot.to_radians())).map(|s| s.id) };

        // ── moving them ──
        if resp.drag_started() {
            if let Some(q) = resp.interact_pointer_pos() {
                if let Some(id) = hit(q) {
                    let st = &mut self.setup.stickers;
                    if let Some(page) = st.page_mut() {
                        if let Some(i) = page.stickers.iter().position(|s| s.id == id) {
                            let s = page.stickers.remove(i);
                            let off = q - pos2(r.min.x + s.x * r.width(), r.min.y + s.y * r.height());
                            page.stickers.push(s);
                            st.drag = Some((id, off));
                        }
                    }
                }
            }
        }
        if let (Some((id, off)), Some(q)) = (self.setup.stickers.drag, resp.interact_pointer_pos()) {
            if resp.dragged() {
                let c = q - off;
                if let Some(page) = self.setup.stickers.page_mut() {
                    if let Some(s) = page.stickers.iter_mut().find(|s| s.id == id) {
                        s.x = ((c.x - r.min.x) / r.width()).clamp(0.02, 0.98);
                        s.y = ((c.y - r.min.y) / r.height()).clamp(0.03, 0.97);
                    }
                }
            }
        }
        if resp.drag_stopped() || (self.setup.stickers.drag.is_some() && !ui.input(|i| i.pointer.any_down())) {
            if let Some((id, _)) = self.setup.stickers.drag.take() {
                let st = &mut self.setup.stickers;
                if let Some(page) = st.page_mut() {
                    if let Some(s) = page.stickers.iter_mut().find(|s| s.id == id) {
                        s.rot = rnd() * 24.0 - 12.0;
                    }
                }
                st.touch();
            }
        }
        // a click on a sticker brings it to the top
        if resp.clicked() {
            if let Some(id) = resp.interact_pointer_pos().and_then(hit) {
                let st = &mut self.setup.stickers;
                if let Some(page) = st.page_mut() {
                    if let Some(i) = page.stickers.iter().position(|s| s.id == id) {
                        let s = page.stickers.remove(i);
                        page.stickers.push(s);
                    }
                }
                st.added.insert(id, Instant::now());
                st.touch();
            }
        }
        if resp.secondary_clicked() {
            self.setup.stickers.menu = resp.interact_pointer_pos().and_then(hit);
        }
        let hovered = if self.setup.stickers.drag.is_none() { resp.hover_pos().and_then(hit) } else { None };
        if self.setup.stickers.drag.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if hovered.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }

        // ── drawing them (bottom to top, as they are now) ──
        let stickers: Vec<Sticker> = self.setup.stickers.page_mut().map(|p| p.stickers.clone()).unwrap_or_default();
        let dragging = self.setup.stickers.drag.map(|d| d.0);
        let mut any_anim = false;
        for s in &stickers {
            let held = Some(s.id) == dragging;
            let lift = ctx.animate_value_with_time(egui::Id::new(("sticker-lift", s.id)), if held { 1.0 } else { 0.0 }, if motion { 0.15 } else { 0.0 });
            let pop = match self.setup.stickers.added.get(&s.id) {
                Some(t) if motion && t.elapsed().as_secs_f32() < 0.45 => {
                    any_anim = true;
                    let k = t.elapsed().as_secs_f32() / 0.45;
                    // a quick overshoot, then settle
                    1.0 + 0.22 * (k * std::f32::consts::PI).sin() * (1.0 - k)
                }
                _ => 1.0,
            };
            let scale = (1.0 + 0.12 * lift) * pop;
            let a = s.rot.to_radians() * (1.0 - lift);
            let c = centre(s);
            let half = half_size(&p, s, base) * scale;
            let tex = if s.ribbon.is_empty() { self.sticker_tex(&s.slug, s.shiny, still || !motion) } else { None };
            draw_sticker(&p, s, c, half, a, lift, tex, if motion { time } else { 0.0 }, base * scale);
            if s.shiny && motion {
                any_anim = true;
            }
        }
        self.setup.stickers.added.retain(|_, t| t.elapsed() < Duration::from_secs(1));
        if stickers.is_empty() {
            let col = if paper_kind == 3 { Color32::from_white_alpha(170) } else { Color32::from_rgb(140, 125, 100) };
            p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "This page is empty", crate::fonts::sans_bold_font(17.0), col);
            p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Click a sticker in the tray, or drag one onto the page.", crate::fonts::sans_font(13.0), col);
        }
        // what you're pointing at
        if let Some(s) = hovered.and_then(|id| stickers.iter().find(|s| s.id == id)) {
            let label = if s.ribbon.is_empty() { s.label.clone() } else { ribbon(&s.ribbon).map_or(String::new(), |r| format!("Ribbon: {}", r.text)) };
            if !label.is_empty() {
                if let Some(q) = resp.hover_pos() {
                    let lp = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("sticker-tip")));
                    let g = lp.layout_no_wrap(label, crate::fonts::sans_font(12.5), Color32::WHITE);
                    let br = Rect::from_min_size(q + vec2(14.0, 16.0), g.size() + vec2(16.0, 10.0));
                    lp.rect_filled(br, 8.0, Color32::from_rgba_unmultiplied(20, 20, 28, 235));
                    lp.galley(br.min + vec2(8.0, 5.0), g, Color32::WHITE);
                }
            }
        }
        // right-click
        let menu_id = self.setup.stickers.menu;
        resp.context_menu(|ui| {
            let Some(id) = menu_id else {
                ui.label(RichText::new("Right-click a sticker to change it.").size(12.5).color(MUTED()));
                return;
            };
            let mut act: Option<u8> = None;
            if ui.button("Bring to front").clicked() {
                act = Some(0);
            }
            if ui.button("Bigger").clicked() {
                act = Some(1);
            }
            if ui.button("Smaller").clicked() {
                act = Some(2);
            }
            if ui.button("Straighten").clicked() {
                act = Some(3);
            }
            ui.separator();
            if ui.button(RichText::new("Peel off").color(Color32::from_rgb(248, 113, 113))).clicked() {
                act = Some(4);
            }
            if let Some(a) = act {
                let st = &mut self.setup.stickers;
                let mut peeled: Option<Sticker> = None;
                if let Some(page) = st.page_mut() {
                    if let Some(i) = page.stickers.iter().position(|s| s.id == id) {
                        match a {
                            0 => {
                                let s = page.stickers.remove(i);
                                page.stickers.push(s);
                            }
                            1 => page.stickers[i].scale = (page.stickers[i].scale * 1.15).min(2.2),
                            2 => page.stickers[i].scale = (page.stickers[i].scale / 1.15).max(0.5),
                            3 => page.stickers[i].rot = 0.0,
                            _ => peeled = Some(page.stickers.remove(i)),
                        }
                    }
                }
                if let Some(s) = peeled {
                    st.undo = Some((page_id, s, Instant::now()));
                }
                st.added.insert(id, Instant::now());
                st.touch();
                ui.close_menu();
            }
        });
        if any_anim || dragging.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if !still && stickers.iter().any(|s| s.ribbon.is_empty()) && motion {
            // the sprites move
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    /// The tray: shinies, party, boxes and ribbons to stick on.
    fn stickers_tray(&mut self, ui: &mut egui::Ui, h: f32) {
        let shinies = self.stickers_shinies();
        let save = self.stickers_save_items();
        let (party_n, box_n) = save.as_ref().map_or((0, 0), |(a, b)| (a.len(), b.len()));
        let earned = self.setup.stickers.earned.clone();
        kit::card(ui, |ui| {
            ui.set_min_height(h - 32.0);
            ui.label(RichText::new("Your stickers").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(RichText::new("Click one to stick it on, or drag it onto the page.").size(12.0).color(MUTED()));
            ui.add_space(8.0);
            let mut tab = self.setup.stickers.tray;
            let opts: [(u8, &str); 4] = [(0, "Shinies"), (1, "Party"), (2, "Boxes"), (3, "Ribbons")];
            if kit::segmented(ui, &mut tab, &opts) {
                self.setup.stickers.tray = tab;
            }
            ui.add_space(8.0);
            let tab = self.setup.stickers.tray;
            let items: Vec<Spec> = match tab {
                0 => shinies.clone(),
                1 => save.as_ref().map(|s| s.0.clone()).unwrap_or_default(),
                2 => save.as_ref().map(|s| s.1.clone()).unwrap_or_default(),
                _ => Vec::new(),
            };
            if tab == 2 && box_n > 0 {
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 110.0).max(80.0);
                    kit::search_box(ui, &mut self.setup.stickers.search, "Search your boxes", w);
                    let on = self.setup.stickers.shiny_only;
                    let b = egui::Button::new(RichText::new("★ Shiny").size(12.0).color(if on { Color32::from_rgb(40, 30, 0) } else { TEXT() }))
                        .fill(if on { SHINY_GOLD } else { kit::btn_fill() })
                        .stroke(Stroke::new(1.0f32, if on { SHINY_GOLD } else { LINE() }))
                        .rounding(12.0);
                    if ui.add(b).on_hover_text("Only shiny Pokémon").clicked() {
                        self.setup.stickers.shiny_only = !on;
                    }
                });
                ui.add_space(6.0);
            }
            let q = self.setup.stickers.search.trim().to_lowercase();
            let shiny_only = self.setup.stickers.shiny_only;
            let items: Vec<Spec> = if tab == 2 { items.into_iter().filter(|s| (q.is_empty() || s.name.to_lowercase().contains(&q)) && (!shiny_only || s.shiny)).collect() } else { items };
            let list_h = (h - 170.0).max(160.0);
            if tab == 3 {
                egui::ScrollArea::vertical().id_salt("sticker-ribbons").max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
                    for rb in RIBBONS.iter() {
                        let got = earned.contains(&rb.id);
                        let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), if got { Sense::click_and_drag() } else { Sense::hover() });
                        let p = ui.painter_at(row);
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && got, kit::QUICK);
                        if ht > 0.0 {
                            p.rect_filled(row, 10.0, theme::blend(kit::CARD(), TEXT(), 0.05 * ht));
                        }
                        let g = p.layout_no_wrap(rb.text.to_string(), crate::fonts::sans_bold_font(12.0), Color32::WHITE);
                        let pill = Rect::from_min_size(pos2(row.min.x + 6.0, row.center().y - 13.0), vec2(g.size().x + 22.0, 26.0));
                        p.rect_filled(pill, 8.0, if got { rb.col } else { theme::blend(kit::CARD(), MUTED(), 0.35) });
                        p.galley(pos2(pill.min.x + 11.0, pill.center().y - g.size().y / 2.0), g, Color32::WHITE);
                        let note = if got { "Click or drag it on" } else { rb.how };
                        p.with_clip_rect(row).text(pos2(pill.max.x + 10.0, row.center().y), Align2::LEFT_CENTER, if got { note.to_string() } else { format!("🔒 {note}") }, crate::fonts::sans_font(11.5), MUTED());
                        if got {
                            let spec = Spec { slug: String::new(), shiny: false, ribbon: rb.id.to_string(), label: String::new(), name: rb.text.to_string() };
                            if resp.drag_started() {
                                self.setup.stickers.carry = Some(spec);
                            } else if resp.clicked() {
                                self.sticker_add(&spec, None);
                            }
                            resp.on_hover_cursor(egui::CursorIcon::Grab);
                        }
                    }
                });
                return;
            }
            if items.is_empty() {
                let (title, body) = match tab {
                    0 => ("No shinies yet", "Catch one with Shiny Hunter and it's here as a shimmering sticker."),
                    _ if save.is_none() => ("Your save hasn't been read yet", "Start the game once, and your party and boxes show up here. The save is only ever read, never written."),
                    1 => ("Nobody in your party", "Your party is read from your save."),
                    _ if !q.is_empty() || shiny_only => ("Nothing matches", "Try another name, or turn off ★ Shiny."),
                    _ => ("Your boxes are empty", "Your boxes are read from your save."),
                };
                ui.add_space(20.0);
                ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
                ui.label(RichText::new(body).size(12.0).color(MUTED()));
                if tab != 0 {
                    if let Some(e) = self.setup.boxes.error.clone() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(e).size(11.5).color(Color32::from_rgb(248, 180, 80)));
                    }
                }
                return;
            }
            let count = match tab {
                0 => format!("{} shin{}", items.len(), if items.len() == 1 { "y" } else { "ies" }),
                1 => format!("{party_n} in your party"),
                _ => format!("{} of {box_n}", items.len()),
            };
            ui.label(RichText::new(count).size(11.5).color(MUTED()));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().id_salt(("sticker-tray", tab)).max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
                let tile = 62.0;
                let gap = 6.0;
                let cols = (((ui.available_width() + gap) / (tile + gap)).floor() as usize).max(2);
                for row in items.chunks(cols) {
                    let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), tile), Sense::hover());
                    ui.add_space(gap);
                    if !ui.is_rect_visible(rr) {
                        continue;
                    }
                    for (k, spec) in row.iter().enumerate() {
                        let tr = Rect::from_min_size(pos2(rr.min.x + k as f32 * (tile + gap), rr.min.y), vec2(tile, tile));
                        let resp = ui.interact(tr, ui.id().with(("tray", tab, &spec.label, k)), Sense::click_and_drag());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter_at(tr.expand(2.0));
                        p.rect_filled(tr, 12.0, theme::blend(kit::sunken(), TEXT(), 0.05 * ht));
                        p.rect_stroke(tr, 12.0, Stroke::new(1.0f32, if spec.shiny { SHINY_GOLD.gamma_multiply(0.6 + 0.4 * ht) } else { theme::blend(LINE(), TEXT(), 0.2 * ht) }));
                        if let Some(sp) = self.species.iter().find(|s| s.slug == spec.slug).cloned() {
                            self.draw_sprite_centered(&p, &sp, spec.shiny, tr.center(), vec2(tile - 10.0, tile - 10.0));
                        }
                        if spec.shiny {
                            theme::sparkle(&p, tr.min + vec2(11.0, 11.0), 5.0, SHINY_GOLD);
                        }
                        if resp.drag_started() {
                            self.setup.stickers.carry = Some(spec.clone());
                        } else if resp.clicked() {
                            self.sticker_add(spec, None);
                        }
                        resp.on_hover_text(spec.label.as_str()).on_hover_cursor(egui::CursorIcon::Grab);
                    }
                }
            });
        });
    }

    /// A sticker being dragged out of the tray: it follows the pointer and
    /// sticks where it's let go over the page.
    fn stickers_carry(&mut self, ctx: &egui::Context) {
        let Some(spec) = self.setup.stickers.carry.clone() else { return };
        let q = ctx.pointer_latest_pos();
        let over = q.zip(self.setup.stickers.page_rect).is_some_and(|(q, r)| r.contains(q));
        if let Some(q) = q {
            let lp = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("sticker-carry")));
            let still = self.setup.stickers.file().still;
            let tex = if spec.ribbon.is_empty() { self.sticker_tex(&spec.slug, spec.shiny, still) } else { None };
            let ghost = Sticker { id: 0, slug: spec.slug.clone(), shiny: spec.shiny, ribbon: spec.ribbon.clone(), label: String::new(), x: 0.0, y: 0.0, rot: 0.0, scale: 1.0 };
            let half = half_size(&lp, &ghost, BASE) * 1.08;
            draw_sticker(&lp, &ghost, q, half, -0.08, 1.0, tex, ctx.input(|i| i.time) as f32, BASE * 1.08);
            if !over {
                lp.text(q + vec2(0.0, half.y + 14.0), Align2::CENTER_CENTER, "Drop it on the page", crate::fonts::sans_bold_font(12.0), TEXT());
            }
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            ctx.request_repaint();
        }
        if ctx.input(|i| i.pointer.any_released() || !i.pointer.any_down()) {
            if over {
                self.sticker_add(&spec, q);
            }
            self.setup.stickers.carry = None;
        }
    }

    /// The cog's window.
    pub(super) fn stickers_settings(&mut self, ui: &mut egui::Ui) {
        super::app_settings::group(ui, "Paper");
        let mut paper_kind = self.setup.stickers.file().paper;
        if kit::segmented(ui, &mut paper_kind, &[(0u8, "Dotted"), (1, "Grid"), (2, "Kraft"), (3, "Night sky")]) {
            self.setup.stickers.file().paper = paper_kind;
            self.setup.stickers.touch();
        }
        super::app_settings::group(ui, "Stickers");
        let mut size = self.setup.stickers.file().size();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Size").size(13.0).color(TEXT()));
            if ui.add(egui::Slider::new(&mut size, 0.7..=1.5).show_value(false)).changed() {
                self.setup.stickers.file().size = size;
                self.setup.stickers.touch();
            }
        });
        let mut moving = !self.setup.stickers.file().still;
        if kit::toggle_row(ui, "Moving stickers", "Stickers move like the sprites do. Off: they hold still.", &mut moving) {
            self.setup.stickers.file().still = !moving;
            self.setup.stickers.touch();
        }
        // this page
        let Some((pid, name, world, n)) = self.setup.stickers.page_mut().map(|p| (p.id, p.name.clone(), p.world, p.stickers.len())) else { return };
        super::app_settings::group(ui, "This page");
        let mut text = match &self.setup.stickers.rename {
            Some((id, t)) if *id == pid => t.clone(),
            _ => name.clone(),
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Name").size(13.0).color(TEXT()));
            let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(260.0));
            if r.changed() {
                self.setup.stickers.rename = Some((pid, text.clone()));
                let t = text.trim().chars().take(40).collect::<String>();
                if !t.is_empty() {
                    if let Some(p) = self.setup.stickers.page_mut() {
                        p.name = t;
                    }
                    self.setup.stickers.touch();
                }
            }
        });
        if world.is_some() {
            ui.label(RichText::new("This page belongs to a game world. Its stickers stay here even when you play another world.").size(11.5).color(MUTED()));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let clear_text = if self.setup.stickers.confirm_clear { "Sure? Click again to peel everything off" } else { "Clear this page" };
            if ui.add_enabled(n > 0, egui::Button::new(RichText::new(clear_text).size(12.5).color(Color32::from_rgb(248, 113, 113)))).clicked() {
                if self.setup.stickers.confirm_clear {
                    let st = &mut self.setup.stickers;
                    if let Some(p) = st.page_mut() {
                        p.stickers.clear();
                    }
                    st.confirm_clear = false;
                    st.touch();
                } else {
                    self.setup.stickers.confirm_clear = true;
                }
            }
            let extra = world.is_none() && self.setup.stickers.file().pages.len() > 1;
            if extra && ui.button(RichText::new("Delete this page").size(12.5)).on_hover_text("Only pages you added can be deleted").clicked() {
                let st = &mut self.setup.stickers;
                let f = st.file();
                f.pages.retain(|p| p.id != pid);
                st.page = None;
                st.touch();
            }
        });
        ui.add_space(4.0);
        ui.label(RichText::new("Your stickers are saved in sticker-book.json in your data folder, with backups. Your save file is only ever read.").size(11.5).color(MUTED()));
    }
}

/// One sticker: its shadow, the white die-cut edge, the picture (a sprite
/// or a ribbon), and a shimmer when it's shiny.
#[allow(clippy::too_many_arguments)]
fn draw_sticker(p: &egui::Painter, s: &Sticker, c: Pos2, half: Vec2, a: f32, lift: f32, tex: Option<(egui::TextureId, Vec2, bool)>, time: f32, size: f32) {
    let k = size / BASE;
    let rad = if s.ribbon.is_empty() { half.x * 0.36 } else { half.y * 0.6 };
    // the shadow, further away while it's held
    let sh = vec2(0.0, 3.0 * k + 8.0 * lift);
    p.add(egui::Shape::convex_polygon(rounded(c + sh, half + Vec2::splat(1.0 + 2.0 * lift), rad, a), Color32::from_black_alpha((55.0 + 35.0 * lift) as u8), Stroke::NONE));
    // shiny: a soft gold glow around the edge
    if s.shiny {
        let tw = 0.7 + 0.3 * (time * 2.2 + s.id as f32).sin();
        p.add(egui::Shape::convex_polygon(rounded(c, half + Vec2::splat(4.0 * k), rad + 4.0 * k, a), SHINY_GOLD.gamma_multiply(0.45 * tw), Stroke::NONE));
    }
    // the white edge
    p.add(egui::Shape::convex_polygon(rounded(c, half, rad, a), Color32::WHITE, Stroke::new(1.0f32, Color32::from_black_alpha(18))));
    if !s.ribbon.is_empty() {
        let Some(rb) = ribbon(&s.ribbon) else { return };
        let inner = half - Vec2::splat(5.0 * k);
        p.add(egui::Shape::convex_polygon(rounded(c, inner, (rad - 4.0 * k).max(2.0), a), rb.col, Stroke::NONE));
        let g = p.layout_no_wrap(rb.text.to_string(), crate::fonts::sans_bold_font(13.0 * k * s.scale.max(0.3)), Color32::WHITE);
        // TextShape turns about its top-left corner
        let pos = c + turn(-g.size() / 2.0, a);
        p.add(egui::epaint::TextShape::new(pos, g, Color32::WHITE).with_angle(a));
        return;
    }
    // the sprite, kept to its shape inside the edge
    let inner = half - Vec2::splat(6.0 * k);
    match tex {
        Some((id, sz, still)) if sz.x > 0.0 && sz.y > 0.0 => {
            let fit = (inner.x * 2.0 / sz.x).min(inner.y * 2.0 / sz.y) * if still { 0.92 } else { 1.0 };
            let hs = sz * fit / 2.0;
            let mut m = egui::Mesh::with_texture(id);
            for (corner, uv) in [(vec2(-hs.x, -hs.y), pos2(0.0, 0.0)), (vec2(hs.x, -hs.y), pos2(1.0, 0.0)), (vec2(hs.x, hs.y), pos2(1.0, 1.0)), (vec2(-hs.x, hs.y), pos2(0.0, 1.0))] {
                m.vertices.push(egui::epaint::Vertex { pos: c + turn(corner, a), uv, color: Color32::WHITE });
            }
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.add(m);
        }
        _ => {
            // still loading
            p.add(egui::Shape::convex_polygon(rounded(c, inner * 0.55, inner.x * 0.2, a), Color32::from_gray(232), Stroke::NONE));
        }
    }
    // shiny: a band of light sweeps across now and then, and sparkles twinkle
    if s.shiny {
        let ph = (time * 0.35 + (s.id % 7) as f32 * 0.13).fract() * 1.8 - 0.4;
        let bx = -half.x + ph * 2.0 * half.x;
        let bw = half.x * 0.35;
        let skew = half.y * 0.5;
        let mut m = egui::Mesh::default();
        for (y, sk) in [(-half.y, skew), (half.y, -skew)] {
            for (dx, col) in [(-bw, Color32::TRANSPARENT), (0.0, glow(Color32::from_rgb(255, 236, 170), 0.55)), (bw, Color32::TRANSPARENT)] {
                let x = (bx + sk + dx).clamp(-half.x + rad * 0.3, half.x - rad * 0.3);
                m.colored_vertex(c + turn(vec2(x, y * 0.92), a), col);
            }
        }
        for k2 in 0..2u32 {
            m.add_triangle(k2, k2 + 1, 3 + k2);
            m.add_triangle(k2 + 1, 4 + k2, 3 + k2);
        }
        p.add(m);
        for (i, corner) in [vec2(0.72, -0.74), vec2(-0.78, 0.66), vec2(0.6, 0.78)].iter().enumerate() {
            let tw = ((time * 2.6 + i as f32 * 2.1 + s.id as f32 * 0.7).sin() * 0.5 + 0.5).powi(2);
            if tw > 0.08 {
                theme::sparkle(p, c + turn(vec2(corner.x * half.x, corner.y * half.y), a), (3.0 + 4.0 * tw) * k, SHINY_GOLD.gamma_multiply(0.4 + 0.6 * tw));
            }
        }
    }
}
