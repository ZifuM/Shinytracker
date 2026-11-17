//! The Ctrl K launcher (also the search bar at the top of the sidebar):
//! type a few letters, Enter. It finds apps and pages, and what's inside
//! the apps: Pokémon (by name or #number), moves, items, places and guides
//! (Game Info's data), and today's rare grass. Apps you don't have yet point
//! to the App Store.

use super::info_home::{Kind, Peek};
use super::rare::RareSpot;
use super::shell::{app_tab, tab_meta};
use super::store::{app_col, app_icon};
use super::ui_kit::{self as kit, LINE, MUTED, SIDEBAR, TEXT};
use super::{App, Tab};
use crate::apps::{self, AppId};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Key, Rect, Sense, Stroke};

const VIOLET: Color32 = Color32::from_rgb(139, 92, 246);

#[derive(Default)]
pub(super) struct QuickState {
    pub open: bool,
    pub query: String,
    pub sel: usize,
    pub fresh: bool,
    /// Something clicked inside the quick look (a move a Pokémon learns…),
    /// shown in place of the picked result.
    pub look: Option<(Kind, String)>,
    /// Put the text cursor at the end (after a suggestion filled the box).
    pub to_end: bool,
}

/// What a launcher result does.
#[derive(Clone)]
pub(super) enum Act {
    /// Put this in the search box (a / command suggestion).
    Fill(String),
    /// One of your Pokémon, in Pokémon Boxes.
    Boxed(super::boxes_app::Sel),
    App(AppId),
    Store(AppId),
    Page(Tab),
    /// A Game Info / Pokédex result; the app it opens in.
    Info(Kind, String, AppId),
    Rare(Box<RareSpot>),
}

#[derive(Clone)]
pub(super) struct QHit {
    pub group: String,
    pub icon: String,
    pub col: Color32,
    pub title: String,
    pub sub: String,
    pub act: Act,
    /// A Pokémon's national dex number: its picture instead of the icon.
    pub dex: Option<u32>,
}

/// How well `name` matches `q` (lower is better): whole, start, a word's
/// start, inside, or its first letters ("dc" for Damage Calculator).
pub(super) fn name_score(name: &str, q: &str) -> Option<u32> {
    let n = name.to_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    if n == q {
        Some(0)
    } else if n.starts_with(q) {
        Some(1)
    } else if n.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(q)) {
        Some(2)
    } else if n.contains(q) {
        Some(3)
    } else {
        let initials: String = n.split(|c: char| !c.is_alphanumeric()).filter_map(|w| w.chars().next()).collect();
        (q.len() >= 2 && initials.starts_with(q)).then_some(4)
    }
}

/// Pages you can go to that aren't apps of their own.
const PAGES: [Tab; 13] = [
    Tab::Home,
    Tab::Store,
    Tab::Library,
    Tab::Profile,
    Tab::Stats,
    Tab::Dex,
    Tab::Trophies,
    Tab::Hunt,
    Tab::Counting,
    Tab::Runs,
    Tab::Overlay,
    Tab::Appearance,
    Tab::Hotkeys,
];

impl App {
    // ───────────────────────────── sidebar row ─────────────────────────────

    /// The search bar at the top of the sidebar: opens the Ctrl K launcher
    /// (folded: just the magnifier).
    pub(super) fn nav_search_bar(&mut self, ui: &mut egui::Ui, compact: bool) {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
        let hov = resp.hovered();
        let p = ui.painter();
        let glass = |p: &egui::Painter, c: egui::Pos2, col: Color32| {
            p.circle_stroke(c - vec2(1.5, 1.5), 5.0, Stroke::new(1.5f32, col));
            p.line_segment([c + vec2(2.2, 2.2), c + vec2(5.8, 5.8)], Stroke::new(1.7f32, col));
        };
        if compact {
            if hov {
                p.rect_filled(r, 9.0, theme::blend(SIDEBAR(), TEXT(), 0.06));
            }
            glass(p, r.center(), if hov { TEXT() } else { MUTED() });
        } else {
            p.rect_filled(r, 9.0, theme::blend(SIDEBAR(), TEXT(), if hov { 0.06 } else { 0.035 }));
            p.rect_stroke(r, 9.0, Stroke::new(1.0f32, theme::blend(SIDEBAR(), TEXT(), if hov { 0.16 } else { 0.09 })));
            glass(p, pos2(r.min.x + 17.0, r.center().y), MUTED());
            p.text(pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, "Search…", crate::fonts::sans_font(13.0), MUTED());
            // the keys, as little caps
            let mut x = r.max.x - 8.0;
            for key in ["K", "Ctrl"] {
                let g = p.layout_no_wrap(key.to_string(), crate::fonts::sans_font(10.0), MUTED());
                let cw = (g.size().x + 10.0).max(18.0);
                let cap = Rect::from_min_size(pos2(x - cw, r.center().y - 9.0), vec2(cw, 18.0));
                p.rect_filled(cap, 4.0, theme::blend(SIDEBAR(), TEXT(), 0.07));
                p.rect_stroke(cap, 4.0, Stroke::new(1.0f32, theme::blend(SIDEBAR(), TEXT(), 0.14)));
                p.galley(pos2(cap.center().x - g.size().x / 2.0, cap.center().y - g.size().y / 2.0), g, MUTED());
                x = cap.min.x - 3.0;
            }
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Search apps, Pokémon, moves, items, places and rare grass (Ctrl K)");
        if resp.clicked() {
            if self.setup.quick.open {
                self.setup.quick.open = false;
            } else {
                self.quick_open_with("");
            }
        }
    }

    /// Open the launcher (Ctrl K), maybe with something already typed.
    pub(super) fn quick_open_with(&mut self, q: &str) {
        let s = &mut self.setup.quick;
        s.open = true;
        s.fresh = true;
        s.query = q.to_string();
        s.sel = 0;
    }

    /// Ctrl K and the launcher: drawn over the page.
    pub(super) fn finders(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, Key::K))) {
            if self.setup.quick.open {
                self.setup.quick.open = false;
            } else {
                self.quick_open_with("");
            }
        }
        if self.setup.quick.open {
            self.quick_launcher(ctx);
        }
    }

    // ───────────────────────────── the launcher ─────────────────────────────

    /// Everything that matches what's typed, best first in each group.
    fn quick_hits(&mut self, q: &str) -> Vec<QHit> {
        // "/" commands, mixed and matched
        if q.trim_start().starts_with('/') {
            return self.command_hits(q.trim_start());
        }
        let q = q.trim().to_lowercase();
        let mut out: Vec<QHit> = Vec::new();
        let have = self.cfg.apps.installed();
        let app_hit = |a: AppId, group: &'static str, sub: String| QHit { group: group.into(), icon: app_icon(a).to_string(), col: app_col(a), title: a.name().to_string(), sub, act: Act::App(a), dex: None };
        let open_sub = |s: &Self, a: AppId| {
            let cat = apps::meta(a).cat.name();
            if s.cfg.apps.is_open(a) {
                format!("{cat} · open")
            } else {
                cat.to_string()
            }
        };
        if q.is_empty() {
            // your recent apps
            let mut recent: Vec<(u64, AppId)> = have.iter().filter_map(|a| self.cfg.apps.last_opened(*a).map(|t| (t, *a))).collect();
            recent.sort_by(|a, b| b.cmp(a));
            for (_, a) in recent.into_iter().take(6) {
                out.push(app_hit(a, "Recent apps", open_sub(self, a)));
            }
            return out;
        }
        // apps you have
        let mut apps_found: Vec<(u32, AppId)> = have.iter().filter_map(|a| name_score(a.name(), &q).map(|s| (s, *a))).collect();
        apps_found.sort();
        for (_, a) in apps_found.into_iter().take(5) {
            out.push(app_hit(a, "Apps", open_sub(self, a)));
        }
        // pages
        let mut pages: Vec<(u32, Tab)> = PAGES
            .iter()
            .filter(|t| super::shell::app_of(**t).map_or(true, |a| self.cfg.apps.is_installed(a)))
            .filter_map(|t| name_score(tab_meta(*t).1, &q).map(|s| (s, *t)))
            .collect();
        pages.sort_by_key(|(s, t)| (*s, *t as u8));
        for (_, t) in pages.into_iter().take(3) {
            let (icon, label, col) = tab_meta(t);
            let sub = match super::shell::app_of(t) {
                Some(a) if t != app_tab(a) => format!("Page in {}", a.name()),
                _ if matches!(t, Tab::Appearance | Tab::Hotkeys) => "Settings".into(),
                _ => "Page".into(),
            };
            out.push(QHit { group: "Pages".into(), icon: icon.to_string(), col, title: label.to_string(), sub, act: Act::Page(t), dex: None });
        }
        // inside the apps: Game Info's Pokémon, moves, items, places, guides
        if let Some(w) = self.wiki.clone().filter(|w| !w.is_empty()) {
            for (kind, key, icon, col, title, sub) in self.info_quick(&w, &q, 6) {
                let home = if kind == Kind::Pokemon { AppId::Pokedex } else { AppId::Info };
                let sub = if self.cfg.apps.is_installed(home) { format!("{sub} · {}", home.name()) } else { format!("{sub} · get {} to open", home.name()) };
                let group = match kind {
                    Kind::Pokemon => "Pokémon",
                    Kind::Move => "Moves",
                    Kind::Item => "Items",
                    Kind::Place => "Places",
                    Kind::Guide => "Guides",
                };
                let dex = (kind == Kind::Pokemon).then(|| sub.trim_start_matches('#').split(' ').next().and_then(|d| d.parse::<u32>().ok())).flatten();
                out.push(QHit { group: group.into(), icon: icon.to_string(), col, title, sub, act: Act::Info(kind, key, home), dex });
            }
        }
        // today's rare grass, by Pokémon or by place
        if q.chars().count() >= 2 {
            let mut spots: Vec<(u32, RareSpot)> = self
                .rare_spots_today()
                .into_iter()
                .filter_map(|s| {
                    let by_name = name_score(s.species.name, &q);
                    let by_place = name_score(&s.place, &q).map(|x| x + 1);
                    by_name.or(by_place).map(|x| (x, s))
                })
                .collect();
            spots.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.place.cmp(&b.1.place)));
            for (_, s) in spots.into_iter().take(4) {
                let tiles = if s.tiles == 1 { "1 tile".to_string() } else { format!("{} tiles", s.tiles) };
                out.push(QHit {
                    group: "Rare grass today".into(),
                    icon: "✨".into(),
                    col: kit::C_GREEN,
                    dex: Some(s.species.dex),
                    title: format!("{} on rare grass", s.species.name),
                    sub: format!("{} · {tiles} · show on the Map", s.place),
                    act: Act::Rare(Box::new(s)),
                });
            }
        }
        // apps you don't have yet
        let mut more: Vec<(u32, AppId)> = apps::ALL.iter().filter(|a| !self.cfg.apps.is_installed(**a)).filter_map(|a| name_score(a.name(), &q).map(|s| (s, *a))).collect();
        more.sort();
        for (_, a) in more.into_iter().take(2) {
            out.push(QHit { group: "In the App Store".into(), icon: app_icon(a).to_string(), col: app_col(a), title: a.name().to_string(), sub: apps::meta(a).tagline.to_string(), act: Act::Store(a), dex: None });
        }
        out
    }

    fn quick_do(&mut self, hit: &QHit, pin: bool) {
        match &hit.act {
            Act::Boxed(sel) => {
                if self.cfg.apps.is_installed(AppId::Boxes) {
                    self.cfg.apps.open(AppId::Boxes, crate::stats::now_unix());
                    self.dirty = true;
                    if let super::boxes_app::Sel::Slot(b, _) = sel {
                        self.setup.boxes.cur = *b;
                    }
                    self.setup.boxes.sel = Some(*sel);
                    self.go_to(Tab::Boxes);
                } else {
                    self.show_store_page(AppId::Boxes);
                }
            }
            Act::Fill(text) => {
                let s = &mut self.setup.quick;
                s.query = text.clone();
                s.sel = 0;
                s.look = None;
                s.to_end = true;
                return;
            }
            Act::App(a) => {
                if pin {
                    if self.cfg.apps.is_pinned(*a) {
                        self.cfg.apps.unpin(*a);
                    } else {
                        self.cfg.apps.pin(*a);
                    }
                    self.dirty = true;
                    return;
                }
                self.open_app(*a);
            }
            Act::Store(a) => self.show_store_page(*a),
            Act::Page(t) => {
                if let Some(a) = super::shell::app_of(*t) {
                    self.cfg.apps.open(a, crate::stats::now_unix());
                    self.dirty = true;
                }
                self.go_to(*t);
            }
            Act::Info(kind, key, home) => {
                if !self.cfg.apps.is_installed(*home) {
                    self.show_store_page(*home);
                } else {
                    self.cfg.apps.open(*home, crate::stats::now_unix());
                    self.dirty = true;
                    self.info_quick_open(*kind, key.clone());
                }
            }
            Act::Rare(s) => {
                if self.cfg.apps.is_installed(AppId::Map) {
                    self.cfg.apps.open(AppId::Map, crate::stats::now_unix());
                    self.dirty = true;
                    self.go_to_rare(s);
                } else {
                    self.show_store_page(AppId::Map);
                }
            }
        }
        self.setup.quick.open = false;
    }

    fn quick_launcher(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        // (the click that opened it mustn't close it)
        let fresh = self.setup.quick.fresh;
        let hits = self.quick_hits(&self.setup.quick.query.clone());
        // keys (taken before the text box sees them)
        let (up, down, esc, tab) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
                i.consume_key(egui::Modifiers::NONE, Key::Tab),
            )
        });
        // Tab takes the first suggestion (or the picked one)
        if tab {
            let i = self.setup.quick.sel.min(hits.len().saturating_sub(1));
            if let Some(h) = hits.get(i).filter(|h| matches!(h.act, Act::Fill(_))).or_else(|| hits.iter().find(|h| matches!(h.act, Act::Fill(_)))) {
                let h = h.clone();
                self.quick_do(&h, false);
                return;
            }
        }
        let (enter, pin) = ctx.input(|i| (i.key_pressed(Key::Enter), i.modifiers.command));
        if esc {
            // Esc steps back out of a look first
            if self.setup.quick.look.is_some() {
                self.setup.quick.look = None;
            } else {
                self.setup.quick.open = false;
                return;
            }
        }
        let n = hits.len();
        {
            let s = &mut self.setup.quick;
            let before = s.sel;
            if n == 0 {
                s.sel = 0;
            } else {
                if down {
                    s.sel = (s.sel + 1) % n;
                }
                if up {
                    s.sel = (s.sel + n - 1) % n;
                }
                s.sel = s.sel.min(n - 1);
            }
            if s.sel != before {
                s.look = None;
            }
        }
        // Wide enough for the quick look beside the results.
        let w = 960.0f32.min(screen.width() - 40.0);
        let two = w >= 720.0;
        let list_w = if two { (w - 20.0) * 0.42 } else { w - 20.0 };
        let body_h = (screen.height() - 250.0).clamp(200.0, 520.0);
        let panel_min = pos2(screen.center().x - w / 2.0, screen.min.y + 80.0);
        let mut run: Option<(usize, bool)> = None;
        let mut peek: Option<Peek> = None;
        let mut panel_rect = Rect::NOTHING;
        let wiki = self.wiki.clone().filter(|w| !w.is_empty());
        egui::Area::new(egui::Id::new("quick-launcher")).order(egui::Order::Tooltip).fixed_pos(panel_min).show(ctx, |ui| {
            // the page dims behind it
            ui.painter().with_clip_rect(screen).rect_filled(screen, 0.0, Color32::from_black_alpha(110));
            egui::Frame::none()
                .fill(kit::CARD())
                .stroke(Stroke::new(1.0f32, theme::blend(kit::CARD(), TEXT(), 0.18)))
                .rounding(14.0)
                .shadow(egui::epaint::Shadow { offset: vec2(0.0, 18.0), blur: 50.0, spread: 0.0, color: Color32::from_black_alpha(140) })
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_width(w - 20.0);
                    // the search line
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("🔍").size(14.0).color(MUTED()));
                        let te = egui::TextEdit::singleline(&mut self.setup.quick.query)
                            .id(egui::Id::new("quick-input"))
                            .hint_text("Search apps, Pokémon, moves, items, places, rare grass…")
                            .frame(false)
                            .font(egui::FontId::proportional(16.0))
                            .desired_width(ui.available_width() - 8.0);
                        let r = ui.add(te);
                        if std::mem::take(&mut self.setup.quick.to_end) {
                            if let Some(mut st) = egui::text_edit::TextEditState::load(ui.ctx(), r.id) {
                                let n = self.setup.quick.query.chars().count();
                                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(n))));
                                st.store(ui.ctx(), r.id);
                            }
                        }
                        // typing always goes here while it's open
                        if self.setup.quick.fresh || !r.has_focus() {
                            r.request_focus();
                        }
                        self.setup.quick.fresh = false;
                        if r.changed() {
                            self.setup.quick.sel = 0;
                            self.setup.quick.look = None;
                        }
                    });
                    ui.add_space(6.0);
                    let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(lr, 0.0, LINE());
                    ui.add_space(4.0);
                    if hits.is_empty() {
                        ui.add_space(10.0);
                        let msg = if self.setup.quick.query.trim().is_empty() { "Type to search your apps and everything in them." } else { "Nothing found. Try a Pokémon, a move, an item or a place." };
                        ui.label(egui::RichText::new(msg).size(12.5).color(MUTED()));
                        ui.add_space(10.0);
                    } else {
                        let sel = self.setup.quick.sel;
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            // the results
                            ui.allocate_ui_with_layout(vec2(list_w, body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.set_width(list_w);
                                egui::ScrollArea::vertical().id_salt("quick-scroll").max_height(body_h).auto_shrink([false, !two]).show(ui, |ui| {
                                    let mut last_group = "";
                                    for (i, h) in hits.iter().enumerate() {
                                        if h.group != last_group {
                                            last_group = h.group.as_str();
                                            ui.add_space(6.0);
                                            ui.horizontal(|ui| {
                                                ui.add_space(8.0);
                                                ui.label(egui::RichText::new(h.group.to_uppercase()).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
                                            });
                                            ui.add_space(2.0);
                                        }
                                        if let Some(c) = self.quick_row(ui, h, i == sel, pin, up || down) {
                                            if c {
                                                self.setup.quick.sel = i;
                                            } else {
                                                run = Some((i, false));
                                            }
                                        }
                                    }
                                });
                            });
                            if two {
                                // the quick look
                                let (sep, _) = ui.allocate_exact_size(vec2(17.0, body_h), Sense::hover());
                                ui.painter().line_segment([pos2(sep.center().x, sep.min.y + 4.0), pos2(sep.center().x, sep.max.y - 4.0)], Stroke::new(1.0f32, LINE()));
                                let rw = ui.available_width();
                                ui.allocate_ui_with_layout(vec2(rw, body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                                    ui.set_width(rw);
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.set_min_height(body_h);
                                    if let Some(h) = hits.get(sel) {
                                        peek = self.quick_preview(ui, h, wiki.as_ref(), body_h);
                                    }
                                });
                            }
                        });
                    }
                    ui.add_space(4.0);
                    let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(lr, 0.0, LINE());
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Type / for commands (/rare, /pokedex, /move surf…) and mix them  ·  ↑↓ move  ·  Tab complete  ·  Enter open  ·  Esc close").size(11.0).color(MUTED()));
                    });
                });
            panel_rect = ui.min_rect();
        });
        // what was clicked in the quick look
        match peek {
            Some(Peek::Look(k, key)) => {
                self.setup.quick.look = Some((k, key));
            }
            Some(Peek::Open) => {
                if let Some(h) = hits.get(self.setup.quick.sel) {
                    let h = h.clone();
                    self.quick_do(&h, false);
                }
                return;
            }
            Some(Peek::Read(id, sec)) => {
                if self.cfg.apps.is_installed(AppId::Info) {
                    self.cfg.apps.open(AppId::Info, crate::stats::now_unix());
                    self.dirty = true;
                    self.info_open_guide_at(&id, sec);
                } else {
                    self.show_store_page(AppId::Info);
                }
                self.setup.quick.open = false;
                return;
            }
            None => {}
        }
        // Enter: apps, pages and rare grass open; a Pokémon, move, item,
        // place or guide just shows here (its button opens it in its app)
        if enter && n > 0 && run.is_none() {
            let i = self.setup.quick.sel.min(n - 1);
            if !matches!(hits[i].act, Act::Info(..)) || !two {
                run = Some((i, pin));
            }
        }
        if let Some((i, pin)) = run {
            let h = hits[i].clone();
            self.quick_do(&h, pin);
            return;
        }
        // a click outside closes it
        if !fresh && ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !panel_rect.contains(pos) && !ctx.memory(|m| m.any_popup_open()) {
                    self.setup.quick.open = false;
                }
            }
        }
    }

    /// One result row. Some(true): picked (hovered or clicked to look at),
    /// Some(false): clicked to open.
    fn quick_row(&mut self, ui: &mut egui::Ui, h: &QHit, on: bool, pin: bool, keyed: bool) -> Option<bool> {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
        let mut out = None;
        if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) && !on {
            out = Some(true);
        }
        let p = ui.painter().clone();
        if on {
            p.rect_filled(r, 9.0, theme::blend(kit::CARD(), VIOLET, 0.16));
            if keyed {
                ui.scroll_to_rect(r, None);
            }
        }
        let chip = Rect::from_center_size(pos2(r.min.x + 24.0, r.center().y), vec2(28.0, 28.0));
        let sp = h.dex.and_then(|d| self.species.get((d as usize).wrapping_sub(1)).cloned()).filter(|sp| self.sprites.get(sp, false).is_some());
        match sp {
            Some(sp) => {
                p.circle_filled(chip.center(), 17.0, theme::blend(kit::CARD(), h.col, 0.16));
                self.draw_sprite_centered(&p, &sp, false, chip.center(), vec2(38.0, 38.0));
            }
            None => kit::icon_chip(&p, chip, &h.icon, h.col, false),
        }
        let tx = r.min.x + 48.0;
        let room = r.width() - 48.0 - 58.0;
        let tf = super::charts::bold(13.5);
        p.text(pos2(tx, r.center().y - 8.0), Align2::LEFT_CENTER, super::charts::clip(ui, &h.title, &tf, room), tf, TEXT());
        let sf = crate::fonts::sans_font(11.5);
        p.text(pos2(tx, r.center().y + 9.0), Align2::LEFT_CENTER, super::charts::clip(ui, &h.sub, &sf, room), sf, MUTED());
        if on {
            let hint = match h.act {
                Act::App(a) if pin => {
                    if self.cfg.apps.is_pinned(a) {
                        "Unpin"
                    } else {
                        "Pin"
                    }
                }
                Act::Info(..) => "",
                Act::Fill(_) => "Tab",
                Act::Store(_) => "Store ↵",
                _ => "Open ↵",
            };
            p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, hint, crate::fonts::sans_font(11.5), MUTED());
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            // a Pokémon, move… is looked at here; anything else opens
            out = Some(matches!(h.act, Act::Info(..)));
        }
        out
    }

    /// The right side: what the picked result is, without leaving the page.
    fn quick_preview(&mut self, ui: &mut egui::Ui, h: &QHit, wiki: Option<&std::sync::Arc<crate::wiki::Wiki>>, body_h: f32) -> Option<Peek> {
        let mut out = None;
        let foot = 44.0;
        egui::ScrollArea::vertical().id_salt(("quick-look", h.title.as_str())).max_height(body_h - foot).auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            match (&h.act, wiki) {
                (Act::Info(kind, key, _), Some(w)) => {
                    let cur = self.setup.quick.look.clone().unwrap_or((*kind, key.clone()));
                    if self.setup.quick.look.is_some() {
                        if ui.add(egui::Button::new(egui::RichText::new(format!("‹  Back to {}", h.title)).size(12.0).color(MUTED())).frame(false)).clicked() {
                            self.setup.quick.look = None;
                        }
                        ui.add_space(4.0);
                    }
                    out = self.info_peek(ui, w, &cur);
                }
                _ => self.quick_about(ui, h),
            }
        });
        // the button under it
        let (label, act): (String, bool) = match &h.act {
            Act::Fill(_) => ("Use it  ›".into(), true),
            Act::Boxed(_) => ("Open in Pokémon Boxes  ›".into(), true),
            Act::Info(Kind::Pokemon, ..) => ("Open the Pokédex entry  ›".into(), true),
            Act::Info(..) => ("Open in Game Info  ›".into(), true),
            Act::App(a) => (format!("Open {}  ›", a.name()), true),
            Act::Page(t) => (format!("Go to {}  ›", tab_meta(*t).1), true),
            Act::Rare(_) => ("Show it on the Map  ›".into(), true),
            Act::Store(_) => ("See it in the App Store  ›".into(), true),
        };
        let (fr, _) = ui.allocate_exact_size(vec2(ui.available_width(), foot), Sense::hover());
        let br = Rect::from_min_size(pos2(fr.min.x, fr.max.y - 34.0), vec2(fr.width(), 34.0));
        let resp = ui.put(br, egui::Button::new(egui::RichText::new(label).size(13.0).color(TEXT())).fill(theme::blend(kit::CARD(), VIOLET, 0.20)).stroke(Stroke::new(1.0f32, theme::blend(kit::CARD(), VIOLET, 0.45))).rounding(9.0));
        if act && resp.clicked() {
            out = Some(Peek::Open);
        }
        out
    }

    /// What an app, page, rare grass spot or store app is, for the quick look.
    fn quick_about(&mut self, ui: &mut egui::Ui, h: &QHit) {
        // the picture and the name
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(60.0, 60.0), Sense::hover());
            let sp = h.dex.and_then(|d| self.species.get((d as usize).wrapping_sub(1)).cloned()).filter(|sp| self.sprites.get(sp, false).is_some());
            let p = ui.painter().clone();
            match sp {
                Some(sp) => {
                    p.circle_filled(r.center(), 30.0, theme::blend(kit::CARD(), h.col, 0.18));
                    self.draw_sprite_centered(&p, &sp, false, r.center(), vec2(58.0, 58.0));
                }
                None => kit::icon_chip(&p, r.shrink(6.0), &h.icon, h.col, false),
            }
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(&h.title).font(super::charts::bold(18.0)).color(TEXT()));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (t, c) in self.quick_tags(h) {
                        quick_tag(ui, &t, c);
                    }
                });
            });
        });
        ui.add_space(12.0);
        match h.act.clone() {
            Act::App(a) | Act::Store(a) => self.about_app(ui, a),
            Act::Page(t) => self.about_page(ui, t),
            Act::Rare(spot) => self.about_rare(ui, &spot),
            Act::Info(..) => {
                ui.label(egui::RichText::new("Game Info's data is still loading (it comes from the Platinum Together wiki).").size(12.5).color(MUTED()));
            }
            Act::Fill(_) => super::quick_query::command_help(ui),
            Act::Boxed(sel) => self.about_boxed(ui, sel),
        }
    }

    /// The little tags under a result's name.
    fn quick_tags(&self, h: &QHit) -> Vec<(String, Color32)> {
        let mut v: Vec<(String, Color32)> = Vec::new();
        match &h.act {
            Act::App(a) | Act::Store(a) => {
                let m = apps::meta(*a);
                v.push((m.cat.name().to_uppercase(), kit::C_SLATE));
                if !self.cfg.apps.is_installed(*a) {
                    v.push(("NOT ADDED YET".into(), kit::C_GOLD));
                }
                if self.cfg.apps.is_open(*a) {
                    v.push(("OPEN".into(), kit::C_GREEN));
                }
                if self.cfg.apps.is_pinned(*a) {
                    v.push(("PINNED".into(), VIOLET));
                }
                if m.beta {
                    v.push(("EXPERIMENTAL".into(), kit::C_ORANGE));
                }
            }
            Act::Page(t) => match super::shell::app_of(*t) {
                Some(a) if *t != app_tab(a) => v.push((format!("IN {}", a.name().to_uppercase()), kit::C_SLATE)),
                _ if matches!(t, Tab::Appearance | Tab::Hotkeys) => v.push(("SETTINGS".into(), kit::C_SLATE)),
                _ => v.push(("PAGE".into(), kit::C_SLATE)),
            },
            Act::Rare(_) => v.push(("RARE GRASS TODAY".into(), kit::C_GREEN)),
            Act::Fill(_) => v.push(("SUGGESTION · TAB TO USE".into(), VIOLET)),
            Act::Boxed(_) => v.push(("YOURS · FROM YOUR SAVE".into(), super::boxes_app::BOX_COL)),
            Act::Info(..) => {}
        }
        v
    }

    /// An app: what it's for, what it does, what's new, what it works with.
    fn about_app(&mut self, ui: &mut egui::Ui, a: AppId) {
        let m = apps::meta(a);
        ui.add(egui::Label::new(egui::RichText::new(m.tagline).size(13.5).color(TEXT()).strong()).wrap());
        ui.add_space(4.0);
        if let Some(first) = m.about.first() {
            ui.add(egui::Label::new(egui::RichText::new(*first).size(12.5).color(MUTED())).wrap());
        }
        ui.add_space(10.0);
        if !m.can_do.is_empty() {
            quick_head(ui, "WHAT IT DOES");
            for c in m.can_do {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("•").color(app_col(a)));
                    ui.add(egui::Label::new(egui::RichText::new(*c).size(12.5).color(TEXT())).wrap());
                });
            }
            ui.add_space(8.0);
        }
        if !m.whats_new.is_empty() {
            quick_head(ui, &format!("NEW · {}", m.updated.to_uppercase()));
            ui.add(egui::Label::new(egui::RichText::new(m.whats_new).size(12.5).color(TEXT())).wrap());
            ui.add_space(8.0);
        }
        if !m.works_with.is_empty() {
            quick_head(ui, "WORKS WITH");
            for (other, why) in m.works_with {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(other.name()).size(12.5).color(TEXT()).strong());
                    ui.label(egui::RichText::new(*why).size(12.0).color(MUTED()));
                });
            }
            ui.add_space(8.0);
        }
        let needs: Vec<&str> = m.requires.iter().filter(|r| !self.cfg.apps.is_installed(**r)).map(|r| r.name()).collect();
        if !needs.is_empty() {
            quick_head(ui, "NEEDS");
            ui.label(egui::RichText::new(format!("{} (added with it)", needs.join(", "))).size(12.5).color(TEXT()));
            ui.add_space(8.0);
        }
        let mut foot: Vec<String> = Vec::new();
        if !m.uses.is_empty() {
            foot.push(format!("Uses: {}", m.uses));
        }
        if let Some(t) = self.cfg.apps.last_opened(a) {
            foot.push(format!("Last opened {}", ago(t)));
        }
        for f in foot {
            ui.add(egui::Label::new(egui::RichText::new(f).size(11.5).color(MUTED())).wrap());
        }
    }

    /// A page: what it's for, and a line about what's on it now.
    fn about_page(&mut self, ui: &mut egui::Ui, t: Tab) {
        let what = match t {
            Tab::Home => "Your hunt and today at a glance, with the widgets you pick and arrange.",
            Tab::Store => "Every app you can add, with screenshots, what each does and what's new.",
            Tab::Library => "The apps you have: open, pin or remove them.",
            Tab::Profile => "Your trainer card: level, avatar, name plate, titles and the battle pass.",
            Tab::Stats => "Your hunting in numbers: how much, how lucky, when you hunt and what you meet, per world too.",
            Tab::Dex => "Every shiny you've logged, with when, where and how long it took.",
            Tab::Trophies => "The trophies you've earned, and what's left to get.",
            Tab::Hunt => "Your current hunt: the target, the method and its odds, and the counter.",
            Tab::Counting => "How encounters are counted (the game's log or the screen), and the shiny ★ alert.",
            Tab::Runs => "Your hunts: the ones you've paused to come back to, and the ones you've finished.",
            Tab::Overlay => "The overlay's look and size, and which parts it shows (eggs, Poké Balls, the bar…).",
            Tab::Appearance => "Theme, colours, the sidebar and how the app looks.",
            Tab::Hotkeys => "Keys for counting, the overlay and the rest, even while you're in the game.",
            _ => "A page of the app.",
        };
        ui.add(egui::Label::new(egui::RichText::new(what).size(13.0).color(TEXT())).wrap());
        ui.add_space(10.0);
        let now: Option<String> = match t {
            Tab::Library => {
                let have = self.cfg.apps.installed().len();
                Some(format!("{have} apps added · {} open · {} pinned", self.cfg.apps.open_list().len(), self.cfg.apps.pin_list().len()))
            }
            Tab::Profile => Some(format!("Level {}", self.cfg.level_seen.max(1))),
            Tab::Dex => Some(match self.cfg.history.len() {
                0 => "No shinies logged yet".into(),
                1 => "1 shiny logged".into(),
                n => format!("{n} shinies logged"),
            }),
            Tab::Hunt | Tab::Home => {
                let target = self.cfg.target.clone();
                Some(match target {
                    Some(slug) => format!("Hunting {} · {} so far", self.target_name(Some(&slug)), self.cfg.count),
                    None => "No target picked yet".into(),
                })
            }
            Tab::Runs => Some(match self.cfg.saved_runs.len() {
                0 => "No paused hunts".into(),
                1 => "1 paused hunt".into(),
                n => format!("{n} paused hunts"),
            }),
            Tab::Overlay => Some(if self.overlay_on { "The overlay is running".into() } else { "The overlay is stopped".into() }),
            _ => None,
        };
        if let Some(n) = now {
            quick_head(ui, "RIGHT NOW");
            ui.label(egui::RichText::new(n).size(12.5).color(TEXT()));
        }
    }

    /// One of your Pokémon: where it is, and what the save knows about it.
    fn about_boxed(&mut self, ui: &mut egui::Ui, sel: super::boxes_app::Sel) {
        let Some(save) = self.setup.boxes.save.as_ref() else { return };
        let (wher, m) = match sel {
            super::boxes_app::Sel::Party(i) => ("Party".to_string(), save.party.get(i).cloned()),
            super::boxes_app::Sel::Slot(b, i) => (save.boxes.get(b).map_or(String::new(), |x| format!("{}, slot {}", x.name, i + 1)), save.boxes.get(b).and_then(|x| x.slots.get(i).cloned().flatten())),
        };
        let Some(m) = m else { return };
        let text = self.setup.boxes.text.clone();
        let fact = |ui: &mut egui::Ui, k: &str, v: String| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(96.0, 18.0), Sense::hover());
                ui.painter().text(r.left_center(), Align2::LEFT_CENTER, k, crate::fonts::sans_font(12.0), MUTED());
                ui.add(egui::Label::new(egui::RichText::new(v).size(12.5).color(TEXT())).wrap());
            });
        };
        fact(ui, "Where", wher);
        if m.nicknamed && !m.nickname.is_empty() {
            fact(ui, "Nickname", m.nickname.clone());
        }
        if let Some(l) = m.level {
            fact(ui, "Level", l.to_string());
        }
        fact(ui, "Nature", crate::gamesave::NATURES[m.nature() as usize % 25].to_string());
        if let Some(t) = text.as_ref() {
            if m.item != 0 {
                fact(ui, "Holding", t.item_name(m.item));
            }
            let moves: Vec<String> = m.moves.iter().filter(|x| **x != 0).map(|x| t.move_name(*x)).collect();
            if !moves.is_empty() {
                fact(ui, "Moves", moves.join(" · "));
            }
        }
        let iv = m.ivs;
        let order = [iv[0], iv[1], iv[2], iv[4], iv[5], iv[3]];
        fact(ui, "IVs", format!("{} · {}/186", order.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" / "), order.iter().map(|v| *v as u32).sum::<u32>()));
        if !m.ot.is_empty() {
            fact(ui, "Trainer", m.ot.clone());
        }
        if m.shiny() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("★ Shiny").size(13.0).strong().color(theme::GOLD));
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Read from your save (read only). Up to date as of the game's last save.").size(11.5).color(MUTED()));
    }

    /// A Pokémon on rare grass today: where, how many tiles, where else,
    /// and a quick way to hunt it.
    fn about_rare(&mut self, ui: &mut egui::Ui, spot: &RareSpot) {
        let mon = self.wiki.as_ref().and_then(|w| w.by_dex(spot.species.dex).cloned());
        if let Some(m) = mon.as_ref() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for t in &m.types {
                    super::pokedex::type_chip(ui, t, "");
                }
            });
            ui.add_space(8.0);
        }
        let reset = super::rare_app::countdown(super::rare_app::secs_to_reset());
        let fact = |ui: &mut egui::Ui, k: &str, v: String| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(96.0, 18.0), Sense::hover());
                ui.painter().text(r.left_center(), Align2::LEFT_CENTER, k, crate::fonts::sans_font(12.0), MUTED());
                ui.label(egui::RichText::new(v).size(12.5).color(TEXT()));
            });
        };
        fact(ui, "Where", spot.place.clone());
        fact(ui, "Tiles", format!("{} tile{}", spot.tiles, if spot.tiles == 1 { "" } else { "s" }));
        fact(ui, "Changes in", reset);
        if let crate::dexextra::State::Ready(e) = self.dexdb.get(&spot.species.slug) {
            if !e.catch_rate.is_empty() {
                fact(ui, "Catch rate", e.catch_rate.split(" (").next().unwrap_or(&e.catch_rate).to_string());
            }
        }
        ui.add_space(10.0);
        let all = self.rare_spots_for(&spot.species.slug);
        let others: Vec<&RareSpot> = all.iter().filter(|s| s.place != spot.place).collect();
        if !others.is_empty() {
            quick_head(ui, "ALSO ON RARE GRASS TODAY");
            for o in &others {
                ui.label(egui::RichText::new(format!("{} · {} tile{}", o.place, o.tiles, if o.tiles == 1 { "" } else { "s" })).size(12.5).color(TEXT()));
            }
            ui.add_space(8.0);
        }
        ui.add(egui::Label::new(egui::RichText::new("Use Sweet Scent on those tiles to meet it. The list changes at midnight UTC.").size(12.0).color(MUTED())).wrap());
        ui.add_space(10.0);
        if ui.add(egui::Button::new(egui::RichText::new("🎯  Hunt it on rare grass").size(13.0)).fill(kit::btn_fill()).rounding(9.0)).on_hover_text("Makes it your target, with the Sweet Scent on rare grass method").clicked() {
            let (slug, name) = (spot.species.slug.clone(), spot.species.name.to_string());
            self.hunt_on_rare_grass(&slug, &name, &all);
            self.setup.quick.open = false;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::name_score;

    #[test]
    fn finds_apps_by_bits_of_their_names() {
        assert_eq!(name_score("Nuzlocke Tracker", "nuz"), Some(1));
        assert_eq!(name_score("Damage Calculator", "calc"), Some(2));
        assert_eq!(name_score("Damage Calculator", "dc"), Some(4));
        assert_eq!(name_score("Pokémon Boxes", "box"), Some(2));
        assert_eq!(name_score("Map", "zzz"), None);
    }
}

/// A small heading in the quick look.
fn quick_head(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
    ui.add_space(2.0);
}

/// A small coloured tag ("OPEN", "PINNED"…).
fn quick_tag(ui: &mut egui::Ui, text: &str, col: Color32) {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(10.0), col);
    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 12.0, 18.0), Sense::hover());
    ui.painter().rect_filled(r, 9.0, theme::blend(kit::CARD(), col, 0.16));
    ui.painter().galley(pos2(r.min.x + 6.0, r.center().y - g.size().y / 2.0), g, col);
}

/// "3m ago", "5h ago", "2d ago".
fn ago(t: u64) -> String {
    let d = crate::stats::now_unix().saturating_sub(t);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", d / 60),
        3600..=86_399 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86_400),
    }
}
