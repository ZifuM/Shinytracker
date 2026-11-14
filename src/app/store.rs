//! The App Store: a featured banner, packs, every app as a card, and a page
//! for each app (banner, Install, screenshots, about and details). Also the
//! install / open / close / uninstall plumbing the Library and the sidebar
//! use. Installing never touches saved data; neither does uninstalling.

use super::shell::{app_of, app_tab};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::apps::{self, AppId, Cat, PACKS};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// How long "Installing…" shows (the app's already in the exe; this is
/// just enough for the button to say what happened).
const INSTALL_MS: u64 = 700;

/// A question the store or the Library is asking, drawn over everything.
#[derive(Clone)]
pub(super) enum AppModal {
    /// A feature needs another app: install it?
    Lock { need: AppId, feature: String },
    /// Installing `app` needs other apps first: install them together?
    Requires { app: AppId, need: Vec<AppId> },
    /// Removing `app` would break these: remove them too?
    Dependents { app: AppId, deps: Vec<AppId> },
    /// Are you sure (your data stays)?
    Uninstall { app: AppId },
    /// Clear the downloaded sprites?
    ClearSprites,
}

pub(super) enum Shot {
    Loading(Receiver<Option<egui::ColorImage>>),
    Ready(egui::TextureHandle),
    Failed,
}

#[derive(Default)]
pub(super) struct StoreState {
    /// The app page that's open (None: the store's front page).
    pub app: Option<AppId>,
    pub query: String,
    pub cat: Option<Cat>,
    /// The screenshot showing on the app page.
    pub shot: usize,
    /// The featured banner the user picked, and when.
    pub feat_pick: Option<(usize, Instant)>,
    /// Apps installing, and since when.
    pub busy: HashMap<AppId, Instant>,
    pub modal: Option<AppModal>,
    pub toast: Option<(Instant, String)>,
    pub shots: HashMap<String, Shot>,
    /// The Library tile whose ⋯ menu is open.
    pub menu: Option<AppId>,
    /// Data on this PC by app (measured off the UI thread).
    pub sizes: Option<Receiver<Vec<(String, u64)>>>,
    pub sizes_got: Option<(Instant, Vec<(String, u64)>)>,
    /// The first-launch screen's run (backup done, what came across).
    pub welcome: Option<super::apps_welcome::WelcomeRun>,
    /// The look someone was on before v0.7.0 retired it (for the first-launch screen).
    pub old_look: Option<&'static str>,
    /// The page before this frame's (to stay on when a link needs an app).
    pub last_tab: Option<Tab>,
}

/// The apps the store features, in turn.
const FEATURED: [AppId; 4] = [AppId::Calc, AppId::Map, AppId::Team, AppId::Pokedex];

/// Each app's colour (its sidebar chip).
pub(super) fn app_col(a: AppId) -> Color32 {
    super::shell::tab_meta(app_tab(a)).2
}
pub(super) fn app_icon(a: AppId) -> &'static str {
    super::shell::tab_meta(app_tab(a)).0
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl App {
    // ───────────────────────── plumbing ─────────────────────────

    pub(super) fn has_app(&self, a: AppId) -> bool {
        self.cfg.apps.is_installed(a)
    }

    /// Open an app: it joins the sidebar and its page shows. Not installed:
    /// its store page instead.
    pub(super) fn open_app(&mut self, a: AppId) {
        if !self.has_app(a) {
            self.show_store_page(a);
            return;
        }
        self.cfg.apps.open(a, now_unix());
        self.dirty = true;
        self.go_to(app_tab(a));
    }

    /// Close an app: off the sidebar. If you were in it, you land on the
    /// last app still open, or the Library.
    pub(super) fn close_app(&mut self, a: AppId) {
        self.cfg.apps.close(a);
        self.dirty = true;
        if app_of(self.setup.tab) == Some(a) {
            match self.cfg.apps.open_list().last().copied() {
                Some(n) => self.go_to(app_tab(n)),
                None => self.go_to(Tab::Library),
            }
        }
    }

    pub(super) fn show_store_page(&mut self, a: AppId) {
        self.go_to(Tab::Store);
        self.setup.store.app = Some(a);
        self.setup.store.shot = 0;
        self.setup.scroll_top = true;
    }

    /// Install an app, asking first if it needs others.
    pub(super) fn request_install(&mut self, a: AppId) {
        let have = self.cfg.apps.installed();
        let need = apps::with_requirements(a, &have);
        if need.len() > 1 {
            self.setup.store.modal = Some(AppModal::Requires { app: a, need: need[..need.len() - 1].to_vec() });
        } else {
            self.start_install(&need);
        }
    }

    pub(super) fn start_install(&mut self, ids: &[AppId]) {
        let now = Instant::now();
        for a in ids {
            if !self.has_app(*a) {
                self.setup.store.busy.entry(*a).or_insert(now);
            }
        }
    }

    pub(super) fn install_pack(&mut self, i: usize) {
        let have = self.cfg.apps.installed();
        let mut ids: Vec<AppId> = Vec::new();
        for a in PACKS[i].apps {
            for x in apps::with_requirements(*a, &have) {
                if !ids.contains(&x) {
                    ids.push(x);
                }
            }
        }
        if ids.is_empty() {
            self.app_toast(format!("You already have the {} pack", PACKS[i].name));
        }
        self.start_install(&ids);
    }

    /// Remove an app, asking first (and about anything that needs it).
    pub(super) fn request_uninstall(&mut self, a: AppId) {
        if a.meta().core {
            return;
        }
        let deps = apps::needed_by(a, &self.cfg.apps.installed());
        self.setup.store.menu = None;
        self.setup.store.modal = Some(if deps.is_empty() { AppModal::Uninstall { app: a } } else { AppModal::Dependents { app: a, deps } });
    }

    pub(super) fn do_uninstall(&mut self, ids: &[AppId]) {
        for a in ids {
            self.cfg.apps.uninstall(*a);
        }
        self.dirty = true;
        if app_of(self.setup.tab).is_some_and(|a| ids.contains(&a)) {
            self.go_to(Tab::Library);
        }
        let names: Vec<&str> = ids.iter().map(|a| a.name()).collect();
        self.app_toast(format!("{} removed. Your saved data is still here", names.join(" + ")));
    }

    pub(super) fn app_toast(&mut self, msg: String) {
        if !self.cfg.all_toasts {
            return;
        }
        self.setup.store.toast = Some((Instant::now(), msg));
    }

    /// Every frame: finish installs, and keep the sidebar in step with the
    /// page you're on (any link into an app opens it; an app that isn't
    /// installed shows its store page instead).
    pub(super) fn apps_tick(&mut self, ctx: &egui::Context) {
        // the Map places you open, for the Dashboard's Recent places
        if self.setup.tab == Tab::Map {
            if let Some(z) = self.mapui.selected {
                if self.cfg.home.map_recent.first() != Some(&z) {
                    self.cfg.home.map_recent.retain(|x| *x != z);
                    self.cfg.home.map_recent.insert(0, z);
                    self.cfg.home.map_recent.truncate(10);
                    self.dirty = true;
                }
            }
        }
        apps::set_live(&self.cfg.apps.installed());
        let done: Vec<AppId> = self.setup.store.busy.iter().filter(|(_, t)| t.elapsed() >= Duration::from_millis(INSTALL_MS)).map(|(a, _)| *a).collect();
        if !done.is_empty() {
            let mut sorted = done.clone();
            sorted.sort();
            for a in &sorted {
                self.setup.store.busy.remove(a);
                self.cfg.apps.install(*a);
            }
            self.dirty = true;
            let names: Vec<&str> = sorted.iter().map(|a| a.name()).collect();
            self.app_toast(format!("{} installed", names.join(" + ")));
            if let Some(AppModal::Lock { need, .. }) = &self.setup.store.modal {
                if sorted.contains(need) {
                    self.setup.store.modal = None;
                }
            }
            if let Some(AppModal::Requires { app, .. }) = &self.setup.store.modal {
                if sorted.contains(app) {
                    self.setup.store.modal = None;
                }
            }
        }
        if !self.setup.store.busy.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        if let Some(a) = app_of(self.setup.tab) {
            if !self.has_app(a) {
                // A link into an app that isn't installed: stay put and say
                // which app it needs.
                let back = self.setup.store.last_tab.filter(|t| app_of(*t).map_or(true, |x| self.has_app(x))).unwrap_or(Tab::Home);
                self.setup.tab = back;
                self.ask_for_app(a, "");
            } else if !self.cfg.apps.is_open(a) {
                self.cfg.apps.open(a, now_unix());
                self.dirty = true;
            }
        }
        self.setup.store.last_tab = Some(self.setup.tab);
    }

    /// A store screenshot (downloaded the first time, then from the cache).
    pub(super) fn store_shot(&mut self, ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
        let st = &mut self.setup.store;
        if !st.shots.contains_key(name) {
            let (tx, rx) = channel();
            let file = crate::config::cache_dir().join("store").join(format!("{name}.png"));
            let url = format!("{}/{name}.png", apps::SHOTS_URL);
            let c = ctx.clone();
            let name_owned = name.to_string();
            std::thread::spawn(move || {
                // Built-in pictures first (work offline), then the download cache, then GitHub.
                let bytes = local_shot(&name_owned).map(|b| b.to_vec()).or_else(|| std::fs::read(&file).ok()).or_else(|| {
                    let b = crate::sprites::fetch(&url).ok()?;
                    image::load_from_memory(&b).ok()?;
                    let _ = std::fs::create_dir_all(file.parent()?);
                    let _ = std::fs::write(&file, &b);
                    Some(b)
                });
                let img = bytes.and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                    let i = i.to_rgba8();
                    egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], &i)
                });
                let _ = tx.send(img);
                c.request_repaint();
            });
            st.shots.insert(name.to_string(), Shot::Loading(rx));
        }
        let entry = st.shots.get_mut(name)?;
        if let Shot::Loading(rx) = entry {
            match rx.try_recv() {
                Ok(Some(img)) => *entry = Shot::Ready(ctx.load_texture(format!("store-{name}"), img, egui::TextureOptions::LINEAR)),
                Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => *entry = Shot::Failed,
                Err(_) => {}
            }
        }
        match entry {
            Shot::Ready(t) => Some(t.clone()),
            _ => None,
        }
    }

    fn shot_failed(&self, name: &str) -> bool {
        matches!(self.setup.store.shots.get(name), Some(Shot::Failed))
    }

    // ───────────────────────── the store ─────────────────────────

    pub(super) fn page_store(&mut self, ui: &mut egui::Ui) {
        if let Some(a) = self.setup.store.app {
            return self.page_store_app(ui, a);
        }
        let ctx = ui.ctx().clone();
        // Header: title, and the search box on the right.
        let top = ui.cursor().min;
        let full = ui.available_width();
        kit::page_header(ui, "App Store", "Get the apps you want. Everything is free, and your saved data is never touched.");
        if full > 700.0 {
            let sr = Rect::from_min_size(pos2(top.x + full - 262.0, top.y + 4.0), vec2(260.0, 40.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(sr).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
                kit::search_box_keyed(ui, Some(egui::Id::new("store-search")), &mut self.setup.store.query, "🔍  Search apps", 258.0);
            });
        } else {
            kit::search_box_keyed(ui, Some(egui::Id::new("store-search")), &mut self.setup.store.query, "🔍  Search apps", full.min(320.0));
        }
        ui.add_space(14.0);
        let searching = !self.setup.store.query.trim().is_empty();
        let w = ui.available_width();
        // the featured banner and packs always stay put: picking a category
        // or searching only changes the app list below
        {
            self.store_featured(ui, &ctx, w);
            ui.add_space(26.0);
            shelf_title(ui, "Packs", "Install a whole set in one click");
            self.store_packs(ui, w);
            ui.add_space(26.0);
        }
        let q = self.setup.store.query.trim().to_lowercase();
        shelf_title(ui, if searching { "Results" } else { "All apps" }, "");
        // Category chips.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let pick = |ui: &mut egui::Ui, label: &str, on: bool| chip(ui, label, on).clicked();
            if pick(ui, "All", self.setup.store.cat.is_none()) {
                self.setup.store.cat = None;
            }
            for c in Cat::ALL {
                if pick(ui, c.name(), self.setup.store.cat == Some(c)) {
                    self.setup.store.cat = Some(c);
                }
            }
        });
        ui.add_space(12.0);
        let list: Vec<AppId> = apps::ALL
            .into_iter()
            .filter(|a| self.setup.store.cat.map_or(true, |c| a.meta().cat == c))
            .filter(|a| q.is_empty() || a.name().to_lowercase().contains(&q) || a.meta().tagline.to_lowercase().contains(&q))
            .collect();
        let show_soon = !searching && self.setup.store.cat.is_none();
        let cols = (((w + 12.0) / 252.0).floor() as usize).clamp(2, 5);
        let cw = (w - 12.0 * (cols as f32 - 1.0)) / cols as f32;
        let n = list.len() + show_soon as usize;
        if list.is_empty() {
            ui.label(egui::RichText::new("No apps match that search.").color(MUTED()));
        }
        for row in 0..n.div_ceil(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for k in 0..cols {
                    let i = row * cols + k;
                    if i < list.len() {
                        self.store_card(ui, &ctx, list[i], cw);
                    } else if i == list.len() && show_soon {
                        self.soon_card(ui, cw);
                    }
                }
            });
            ui.add_space(12.0);
        }
    }

    /// The big rotating banner at the top of the store.
    fn store_featured(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, w: f32) {
        let auto = (now_unix() / 9) as usize % FEATURED.len();
        let i = match self.setup.store.feat_pick {
            Some((i, t)) if t.elapsed() < Duration::from_secs(30) => i,
            _ => auto,
        };
        ctx.request_repaint_after(Duration::from_secs(1));
        let a = FEATURED[i];
        let m = a.meta();
        let col = app_col(a);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 250.0), Sense::click());
        let p = ui.painter().clone();
        p.add(kit::card_shadow().as_shape(rect, 22.0));
        p.rect_filled(rect, 22.0, theme::blend(CARD(), col, 0.16));
        if let Some(t) = self.store_shot(ctx, m.shots[0].0) {
            let ir = Rect::from_min_max(pos2(rect.min.x + rect.width() * 0.38, rect.min.y), rect.max);
            paint_image(&p, &t, ir, egui::Rounding { ne: 22.0, se: 22.0, ..Default::default() }, 0.55, true);
        }
        fade_left(&p, rect, theme::blend(CARD(), col, 0.16), 0.38, 0.75);
        let x = rect.min.x + 30.0;
        let mut y = rect.min.y + 34.0;
        tag(&p, pos2(x, y), "FEATURED", col);
        y += 30.0;
        p.text(pos2(x, y), Align2::LEFT_TOP, m.name, crate::fonts::title_font(32.0), TEXT());
        y += 46.0;
        let g = p.layout(m.about[0].to_string(), crate::fonts::sans_font(14.0), theme::blend(CARD(), TEXT(), 0.85), rect.width() * 0.46);
        let gh = g.size().y.min(64.0);
        p.with_clip_rect(Rect::from_min_size(pos2(x, y), vec2(rect.width() * 0.5, gh))).galley(pos2(x, y), g, TEXT());
        let br = Rect::from_min_size(pos2(x, rect.max.y - 62.0), vec2(150.0, 40.0));
        let clicked_btn = self.app_button(ui, br, a, true);
        // dots
        for k in 0..FEATURED.len() {
            let on = k == i;
            let dr = Rect::from_min_size(pos2(rect.max.x - 30.0 - (FEATURED.len() - k) as f32 * 18.0 + if on { 0.0 } else { 6.0 }, rect.max.y - 26.0), vec2(if on { 20.0 } else { 8.0 }, 8.0));
            p.rect_filled(dr, 4.0, if on { TEXT() } else { TEXT().gamma_multiply(0.3) });
            let hit = ui.interact(dr.expand(5.0), ui.id().with(("feat-dot", k)), Sense::click());
            if hit.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.store.feat_pick = Some((k, Instant::now()));
            }
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !clicked_btn {
            self.show_store_page(a);
        }
    }

    fn store_packs(&mut self, ui: &mut egui::Ui, w: f32) {
        let cols = if w > 760.0 { 4 } else { 2 };
        let cw = (w - 12.0 * (cols as f32 - 1.0)) / cols as f32;
        for row in 0..PACKS.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for k in 0..cols {
                    let i = row * cols + k;
                    if i >= PACKS.len() {
                        break;
                    }
                    let pk = &PACKS[i];
                    let (rect, resp) = ui.allocate_exact_size(vec2(cw, 168.0), Sense::click());
                    let hov = resp.hovered();
                    let p = ui.painter().clone();
                    card_bg(&p, rect, hov);
                    let mut x = rect.min.x + 16.0;
                    for a in pk.apps {
                        kit::icon_chip(&p, Rect::from_min_size(pos2(x, rect.min.y + 16.0), vec2(30.0, 30.0)), app_icon(*a), app_col(*a), false);
                        x += 34.0;
                    }
                    p.text(pos2(rect.min.x + 16.0, rect.min.y + 60.0), Align2::LEFT_TOP, format!("{} {}", pk.icon, pk.name), crate::fonts::sans_bold_font(14.5), TEXT());
                    let g = p.layout(pk.blurb.to_string(), crate::fonts::sans_font(12.0), MUTED(), cw - 32.0);
                    p.galley(pos2(rect.min.x + 16.0, rect.min.y + 82.0), g, MUTED());
                    let have = pk.apps.iter().all(|a| self.has_app(*a));
                    let busy = pk.apps.iter().any(|a| self.setup.store.busy.contains_key(a));
                    let br = Rect::from_min_size(pos2(rect.min.x + 16.0, rect.max.y - 40.0), vec2(124.0, 28.0));
                    let label = if busy { "Installing…" } else if have { "✓ All installed" } else { "Install pack" };
                    let b = pill_button(ui, br, label, !have && !busy, ("pack", i));
                    if (b.clicked() || (resp.clicked() && !b.hovered())) && !have && !busy {
                        self.install_pack(i);
                    }
                    let tip: Vec<&str> = pk.apps.iter().map(|a| a.name()).collect();
                    resp.on_hover_text(tip.join(", "));
                }
            });
            ui.add_space(12.0);
        }
    }

    fn store_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, a: AppId, w: f32) {
        let m = a.meta();
        let col = app_col(a);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 236.0), Sense::click());
        let hov = resp.hovered();
        let lift = kit::anim(ctx, resp.id.with("lift"), hov, kit::QUICK);
        let rect = rect.translate(vec2(0.0, -2.0 * lift));
        let p = ui.painter().clone();
        card_bg(&p, rect, hov);
        // the picture across the top, fading into the card
        let th = Rect::from_min_size(rect.min, vec2(rect.width(), 104.0));
        match self.store_shot(ctx, m.shots[0].0) {
            Some(t) => paint_image(&p, &t, th.shrink(1.0), egui::Rounding { nw: 15.0, ne: 15.0, ..Default::default() }, 0.9, false),
            None => {
                p.rect_filled(th.shrink(1.0), egui::Rounding { nw: 15.0, ne: 15.0, ..Default::default() }, theme::blend(CARD(), col, 0.22));
            }
        }
        fade_down(&p, th, if hov { CARD_HOVER() } else { CARD() });
        let ic = Rect::from_min_size(pos2(rect.min.x + 14.0, th.max.y - 24.0), vec2(42.0, 42.0));
        p.rect_filled(ic.expand(3.0), 14.0, if hov { CARD_HOVER() } else { CARD() });
        kit::icon_chip(&p, ic, app_icon(a), col, false);
        let mut y = ic.max.y + 10.0;
        let ng = p.text(pos2(rect.min.x + 14.0, y), Align2::LEFT_TOP, m.name, crate::fonts::sans_bold_font(14.5), TEXT());
        let mut tx = ng.max.x + 8.0;
        for (on, t, c) in [(m.beta, "BETA", kit::C_GOLD), (m.overlay, "OVERLAY", OVERLAY_COL)] {
            if on {
                tx = tag(&p, pos2(tx, y + 1.0), t, c).max.x + 5.0;
            }
        }
        y += 24.0;
        let g = p.layout(m.tagline.to_string(), crate::fonts::sans_font(12.0), MUTED(), w - 28.0);
        p.with_clip_rect(Rect::from_min_size(pos2(rect.min.x, y), vec2(w, 34.0))).galley(pos2(rect.min.x + 14.0, y), g, MUTED());
        p.text(pos2(rect.min.x + 14.0, rect.max.y - 22.0), Align2::LEFT_CENTER, m.cat.name(), crate::fonts::sans_font(11.5), MUTED());
        let br = Rect::from_min_size(pos2(rect.max.x - 104.0, rect.max.y - 38.0), vec2(90.0, 30.0));
        let b = self.app_button(ui, br, a, false);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !b {
            self.show_store_page(a);
        }
    }

    fn soon_card(&mut self, ui: &mut egui::Ui, w: f32) {
        let (rect, _) = ui.allocate_exact_size(vec2(w, 236.0), Sense::hover());
        let p = ui.painter().clone();
        card_bg(&p, rect, false);
        let th = Rect::from_min_size(rect.min, vec2(rect.width(), 104.0)).shrink(1.0);
        // diagonal stripes
        let clip = p.with_clip_rect(th);
        let mut x = th.min.x - th.height();
        while x < th.max.x {
            clip.line_segment([pos2(x, th.max.y), pos2(x + th.height(), th.min.y)], Stroke::new(8.0f32, theme::blend(CARD(), TEXT(), 0.04)));
            x += 22.0;
        }
        let ic = Rect::from_min_size(pos2(rect.min.x + 14.0, th.max.y - 24.0), vec2(42.0, 42.0));
        p.rect_filled(ic.expand(3.0), 14.0, CARD());
        kit::icon_chip(&p, ic, "🎁", MUTED(), false);
        p.text(pos2(rect.min.x + 14.0, ic.max.y + 10.0), Align2::LEFT_TOP, "Coming soon", crate::fonts::sans_bold_font(14.5), TEXT());
        let g = p.layout("More apps are on the way. Tap Notify me to hear first.".to_string(), crate::fonts::sans_font(12.0), MUTED(), w - 28.0);
        p.galley(pos2(rect.min.x + 14.0, ic.max.y + 34.0), g, MUTED());
        p.text(pos2(rect.min.x + 14.0, rect.max.y - 22.0), Align2::LEFT_CENTER, "In the works", crate::fonts::sans_font(11.5), MUTED());
        let on = self.cfg.apps.notify_soon;
        let br = Rect::from_min_size(pos2(rect.max.x - 112.0, rect.max.y - 38.0), vec2(98.0, 30.0));
        if pill_button(ui, br, if on { "✓ Notifying" } else { "Notify me" }, false, "notify-soon")
            .on_hover_text(if on { "We'll show a note in What's new when a new app lands" } else { "Get a note when a new app lands" })
            .clicked()
        {
            self.cfg.apps.notify_soon = !on;
            self.dirty = true;
        }
    }

    /// Install / Installing… / Open, at `rect`. True when it was clicked.
    pub(super) fn app_button(&mut self, ui: &mut egui::Ui, rect: Rect, a: AppId, big: bool) -> bool {
        if let Some(t) = self.setup.store.busy.get(&a) {
            let f = (t.elapsed().as_millis() as f32 / INSTALL_MS as f32).clamp(0.0, 1.0);
            let p = ui.painter();
            p.rect_filled(rect, rect.height() / 2.0, kit::sunken());
            p.rect_filled(Rect::from_min_size(rect.min, vec2(rect.width() * f, rect.height())), rect.height() / 2.0, kit::primary().gamma_multiply(0.45));
            p.text(rect.center(), Align2::CENTER_CENTER, "Installing…", crate::fonts::sans_bold_font(if big { 13.5 } else { 12.0 }), TEXT());
            ui.interact(rect, ui.id().with(("app-btn", a)), Sense::hover());
            return false;
        }
        if self.has_app(a) {
            let open = self.cfg.apps.is_open(a);
            let r = pill_button(ui, rect, if open { "Go to" } else { "Open" }, big, ("app-btn", a));
            if r.clicked() {
                self.open_app(a);
                return true;
            }
            return r.hovered();
        }
        let r = pill_button(ui, rect, "Install", true, ("app-btn", a));
        if r.clicked() {
            self.request_install(a);
            return true;
        }
        r.hovered()
    }

    // ───────────────────────── an app's page ─────────────────────────

    fn page_store_app(&mut self, ui: &mut egui::Ui, a: AppId) {
        let ctx = ui.ctx().clone();
        let m = a.meta();
        let col = app_col(a);
        let back = ui.add(egui::Label::new(egui::RichText::new("‹  App Store").size(kit::fs(13.0)).color(MUTED())).sense(Sense::click()));
        if back.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.setup.store.app = None;
            self.setup.scroll_top = true;
            return;
        }
        ui.add_space(10.0);
        let w = ui.available_width();

        // Banner.
        let (rect, _) = ui.allocate_exact_size(vec2(w, 220.0), Sense::hover());
        let p = ui.painter().clone();
        let base = theme::blend(CARD(), col, 0.2);
        p.add(kit::card_shadow().as_shape(rect, 22.0));
        p.rect_filled(rect, 22.0, base);
        p.rect_stroke(rect, 22.0, Stroke::new(1.0f32, theme::blend(CARD(), col, 0.4)));
        if let Some(t) = self.store_shot(&ctx, m.shots[0].0) {
            let ir = Rect::from_min_max(pos2(rect.min.x + rect.width() * 0.42, rect.min.y + 1.0), rect.max - vec2(1.0, 1.0));
            paint_image(&p, &t, ir, egui::Rounding { ne: 21.0, se: 21.0, ..Default::default() }, 0.5, true);
        }
        fade_left(&p, rect, base, 0.42, 0.72);
        fade_right(&p, rect, base, 0.66, 0.82);
        let ic = Rect::from_min_size(pos2(rect.min.x + 30.0, rect.center().y - 46.0), vec2(92.0, 92.0));
        kit::icon_chip(&p, ic, app_icon(a), col, true);
        let x = ic.max.x + 22.0;
        let mut tx = x;
        for (t, c) in [(m.cat.name().to_uppercase(), MUTED())]
            .into_iter()
            .chain(m.core.then(|| ("CORE".to_string(), kit::C_BLUE)))
            .chain(m.beta.then(|| ("BETA".to_string(), kit::C_GOLD)))
            .chain(m.overlay.then(|| ("OVERLAY".to_string(), OVERLAY_COL)))
        {
            let r = tag(&p, pos2(tx, rect.center().y - 50.0), &t, c);
            tx = r.max.x + 6.0;
        }
        p.text(pos2(x, rect.center().y - 22.0), Align2::LEFT_TOP, m.name, crate::fonts::title_font(32.0), TEXT());
        let g = p.layout(m.tagline.to_string(), crate::fonts::sans_font(14.0), TEXT(), (rect.max.x - 230.0 - x).max(200.0));
        p.galley(pos2(x, rect.center().y + 22.0), g, TEXT());
        let br = Rect::from_min_size(pos2(rect.max.x - 196.0, rect.center().y - 30.0), vec2(166.0, 44.0));
        self.app_button(ui, br, a, true);
        p.text(pos2(br.center().x, br.max.y + 14.0), Align2::CENTER_CENTER, format!("v{} · free", crate::config::update_version()), crate::fonts::sans_font(12.0), theme::blend(base, TEXT(), 0.7));
        if self.has_app(a) && !m.core && !self.setup.store.busy.contains_key(&a) {
            let ur = Rect::from_center_size(pos2(br.center().x, br.max.y + 36.0), vec2(90.0, 20.0));
            let u = ui.interact(ur, ui.id().with(("uninstall-link", a)), Sense::click());
            let c = if u.hovered() { Color32::from_rgb(255, 150, 160) } else { Color32::from_rgb(230, 130, 140) };
            p.text(ur.center(), Align2::CENTER_CENTER, "Uninstall", crate::fonts::sans_font(12.5), c);
            if u.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.request_uninstall(a);
            }
        }
        ui.add_space(18.0);

        // Screenshots.
        let n = m.shots.len();
        let cur = self.setup.store.shot.min(n - 1);
        let mh = (w * 0.5).clamp(260.0, 460.0);
        let (mr, _) = ui.allocate_exact_size(vec2(w, mh), Sense::hover());
        let p = ui.painter().clone();
        let name = m.shots[cur].0;
        match self.store_shot(&ctx, name) {
            Some(t) => {
                p.rect_filled(mr, 16.0, kit::sunken());
                let s = t.size_vec2();
                let fit = (mr.width() / s.x).min(mr.height() / s.y);
                let ir = Rect::from_center_size(mr.center(), s * fit);
                paint_image(&p, &t, ir, egui::Rounding::same(14.0), 1.0, false);
            }
            None => {
                p.rect_filled(mr, 16.0, kit::sunken());
                let msg = if self.shot_failed(name) { "Screenshot not available right now" } else { "Loading screenshot…" };
                p.text(mr.center(), Align2::CENTER_CENTER, msg, crate::fonts::sans_font(13.0), MUTED());
                if !self.shot_failed(name) {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        p.rect_stroke(mr, 16.0, Stroke::new(1.0f32, LINE()));
        let cap = format!("{} / {n} · {}", cur + 1, m.shots[cur].1);
        let cg = p.layout_no_wrap(cap, crate::fonts::sans_bold_font(12.0), TEXT());
        let cr = Rect::from_min_size(pos2(mr.min.x + 14.0, mr.max.y - 14.0 - cg.size().y - 12.0), cg.size() + vec2(20.0, 12.0));
        p.rect_filled(cr, 9.0, Color32::from_black_alpha(190));
        p.galley(cr.min + vec2(10.0, 6.0), cg, TEXT());
        if n > 1 {
            for (k, dir) in [(-1i32, "‹"), (1, "›")] {
                let c = if k < 0 { pos2(mr.min.x + 30.0, mr.center().y) } else { pos2(mr.max.x - 30.0, mr.center().y) };
                let r = Rect::from_center_size(c, vec2(40.0, 40.0));
                let resp = ui.interact(r, ui.id().with(("shot-arrow", k)), Sense::click());
                p.circle_filled(c, 20.0, Color32::from_black_alpha(if resp.hovered() { 230 } else { 170 }));
                p.circle_stroke(c, 20.0, Stroke::new(1.0f32, LINE()));
                p.text(c + vec2(0.0, -1.0), Align2::CENTER_CENTER, dir, crate::fonts::sans_bold_font(20.0), Color32::WHITE);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.setup.store.shot = (cur as i32 + k).rem_euclid(n as i32) as usize;
                }
            }
            // thumbnails
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for (k, (s, cap)) in m.shots.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(vec2(132.0, 84.0), Sense::click());
                    let p = ui.painter().clone();
                    match self.store_shot(&ctx, s) {
                        Some(t) => paint_image(&p, &t, r, egui::Rounding::same(10.0), if k == cur || resp.hovered() { 1.0 } else { 0.5 }, false),
                        None => {
                            p.rect_filled(r, 10.0, kit::sunken());
                        }
                    }
                    p.rect_stroke(r, 10.0, if k == cur { Stroke::new(2.0f32, kit::primary()) } else { Stroke::new(1.0f32, LINE()) });
                    if resp.on_hover_text(*cap).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.setup.store.shot = k;
                    }
                }
            });
        }
        ui.add_space(18.0);

        // About and details, side by side (stacked when narrow).
        let side = w > 760.0;
        let dw = if side { 320.0 } else { w };
        let aw = if side { w - dw - 16.0 } else { w };
        let about = |ui: &mut egui::Ui| {
            ui.set_width(aw - 36.0);
            small_head(ui, "ABOUT");
            for para in m.about {
                ui.label(egui::RichText::new(*para).size(kit::fs(13.5)).color(TEXT()));
                ui.add_space(6.0);
            }
            ui.add_space(4.0);
            small_head(ui, "WHAT YOU CAN DO");
            for c in m.can_do {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("•").color(col));
                    ui.label(egui::RichText::new(*c).size(kit::fs(13.5)).color(TEXT()));
                });
            }
            ui.add_space(10.0);
            egui::Frame::none().fill(theme::blend(CARD(), kit::C_VIOLET, 0.1)).stroke(Stroke::new(1.0f32, theme::blend(CARD(), kit::C_VIOLET, 0.3))).rounding(12.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(format!("What's new in v{}", crate::config::update_version())).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                ui.label(egui::RichText::new(m.whats_new).size(kit::fs(13.0)).color(theme::blend(CARD(), TEXT(), 0.85)));
            });
        };
        let mut goto: Option<AppId> = None;
        let busy = self.setup.store.busy.clone();
        let installed: Vec<AppId> = self.cfg.apps.installed();
        let details = |ui: &mut egui::Ui| {
            ui.set_width(dw - 36.0);
            small_head(ui, "DETAILS");
            let rows = [
                ("Version", crate::config::update_version().to_string()),
                ("Updated", m.updated.to_string()),
                ("Category", m.cat.name().to_string()),
                ("Made by", "ZifuM".to_string()),
                ("Uses", m.uses.to_string()),
                ("If you uninstall", "Your saved data stays".to_string()),
            ];
            for (k, v) in rows {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(k).size(kit::fs(12.5)).color(MUTED()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(v).size(kit::fs(12.5)).color(TEXT()).strong()).truncate());
                    });
                });
                let r = ui.cursor();
                ui.painter().hline(r.x_range(), r.min.y + 3.0, Stroke::new(1.0f32, LINE()));
                ui.add_space(7.0);
            }
            // What it needs, and what it works with.
            let mut links: Vec<(AppId, String, bool)> = m.requires.iter().map(|r| (*r, "Needs this to work".to_string(), true)).collect();
            for (o, why) in m.works_with {
                links.push((*o, format!("Turns on: {why}"), false));
            }
            for o in apps::ALL {
                for (x, why) in o.meta().works_with {
                    if *x == a && !links.iter().any(|l| l.0 == o) {
                        links.push((o, format!("Uses this for: {why}"), false));
                    }
                }
                if o.meta().requires.contains(&a) && !links.iter().any(|l| l.0 == o) {
                    links.push((o, "Needs this app to work".to_string(), false));
                }
            }
            if !links.is_empty() {
                ui.add_space(8.0);
                small_head(ui, "WORKS WITH");
                for (o, why, req) in links {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                    let p = ui.painter();
                    p.rect_filled(r, 10.0, if resp.hovered() { CARD_HOVER() } else { kit::sunken() });
                    let have = installed.contains(&o) || busy.contains_key(&o);
                    kit::icon_chip(p, Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), vec2(26.0, 26.0)), app_icon(o), app_col(o), false);
                    p.text(pos2(r.min.x + 44.0, r.min.y + 8.0), Align2::LEFT_TOP, o.name(), crate::fonts::sans_bold_font(12.5), TEXT());
                    p.with_clip_rect(r.shrink2(vec2(70.0, 0.0)).translate(vec2(-26.0, 0.0)).with_min_x(r.min.x + 44.0)).text(
                        pos2(r.min.x + 44.0, r.min.y + 25.0),
                        Align2::LEFT_TOP,
                        &why,
                        crate::fonts::sans_font(11.0),
                        if req { kit::C_GOLD } else { MUTED() },
                    );
                    let (st, c) = if have { ("Installed", theme::GREEN) } else { ("Not installed", kit::C_GOLD) };
                    p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, st, crate::fonts::sans_bold_font(11.0), c);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        goto = Some(o);
                    }
                    ui.add_space(6.0);
                }
            }
        };
        let frame = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(16.0).inner_margin(egui::Margin::same(18.0)).show(ui, |ui| add(ui));
        };
        let mut about = about;
        let mut details = details;
        if side {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.allocate_ui_with_layout(vec2(aw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| frame(ui, &mut about));
                ui.allocate_ui_with_layout(vec2(dw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| frame(ui, &mut details));
            });
        } else {
            frame(ui, &mut about);
            ui.add_space(14.0);
            frame(ui, &mut details);
        }
        if let Some(o) = goto {
            self.show_store_page(o);
        }
    }
}

// ───────────────────────── drawing bits ─────────────────────────

fn shelf_title(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
        if !sub.is_empty() {
            ui.label(egui::RichText::new(sub).size(kit::fs(12.5)).color(MUTED()));
        }
    });
    ui.add_space(10.0);
}

fn small_head(ui: &mut egui::Ui, t: &str) {
    ui.label(egui::RichText::new(t).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
    ui.add_space(6.0);
}

pub(super) fn card_bg(p: &egui::Painter, rect: Rect, hov: bool) {
    p.add(kit::card_shadow().as_shape(rect, 16.0));
    p.rect_filled(rect, 16.0, if hov { CARD_HOVER() } else { CARD() });
    p.rect_stroke(rect, 16.0, Stroke::new(1.0f32, if hov { kit::CONTROL() } else { LINE() }));
}

/// The OVERLAY tag's colour (apps that put something on top of the game).
pub(super) const OVERLAY_COL: Color32 = Color32::from_rgb(110, 196, 255);

/// A small coloured label (CATEGORY, BETA…). Returns its rect.
pub(super) fn tag(p: &egui::Painter, at: egui::Pos2, text: &str, col: Color32) -> Rect {
    let g = p.layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(10.5), col);
    let r = Rect::from_min_size(at, g.size() + vec2(14.0, 6.0));
    p.rect_filled(r, 6.0, theme::blend(CARD(), col, 0.2));
    p.galley(r.min + vec2(7.0, 3.0), g, col);
    r
}

/// A category chip; returns its response.
fn chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    let g = ui.painter().layout_no_wrap(label.to_string(), crate::fonts::sans_font(12.5), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + vec2(26.0, 14.0), Sense::click());
    let p = ui.painter();
    let fill = if on { theme::blend(CARD(), kit::primary(), 0.25) } else if resp.hovered() { CARD_HOVER() } else { CARD() };
    p.rect_filled(r, r.height() / 2.0, fill);
    p.rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { kit::primary() } else { LINE() }));
    p.galley(r.min + vec2(13.0, 7.0), g, if on { TEXT() } else { theme::blend(CARD(), TEXT(), 0.8) });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A drawn pill button at `rect`: primary (the button colour) or plain.
pub(super) fn pill_button(ui: &mut egui::Ui, rect: Rect, label: &str, primary: bool, id: impl std::hash::Hash) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(id), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("hov"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if primary {
        p.add(kit::primary_shape_t(rect, ht));
        p.text(rect.center(), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(if rect.height() > 36.0 { 14.5 } else { 12.5 }), kit::on_primary());
    } else {
        p.rect_filled(rect, rect.height() / 2.0, theme::blend(kit::sunken(), TEXT(), 0.04 * ht));
        p.rect_stroke(rect, rect.height() / 2.0, Stroke::new(1.0f32, if ht > 0.5 { kit::CONTROL() } else { LINE() }));
        p.text(rect.center(), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(if rect.height() > 36.0 { 14.0 } else { 12.5 }), TEXT());
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A texture filling `rect` (cropped to keep its shape), rounded, faded by `alpha`.
/// `left`: keep the image's left edge (else its top-centre).
pub(super) fn paint_image(p: &egui::Painter, t: &egui::TextureHandle, rect: Rect, rounding: egui::Rounding, alpha: f32, left: bool) {
    let s = t.size_vec2();
    let (ra, ia) = (rect.width() / rect.height(), s.x / s.y);
    let uv = if ia > ra {
        let wfrac = ra / ia;
        let x0 = if left { 0.0 } else { (1.0 - wfrac) / 2.0 };
        Rect::from_min_max(pos2(x0, 0.0), pos2(x0 + wfrac, 1.0))
    } else {
        let hfrac = ia / ra;
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, hfrac))
    };
    let mut shape = egui::epaint::RectShape::filled(rect, rounding, Color32::from_white_alpha((alpha * 255.0) as u8));
    shape.fill_texture_id = t.id();
    shape.uv = uv;
    p.add(shape);
}

/// Cover the left part of `rect` in `col`, fading out to the right.
fn fade_left(p: &egui::Painter, rect: Rect, col: Color32, from: f32, to: f32) {
    let x0 = rect.min.x + rect.width() * from;
    let x1 = rect.min.x + rect.width() * to;
    let mut m = egui::Mesh::default();
    let clear = col.gamma_multiply(0.0);
    let solid = col;
    m.colored_vertex(pos2(x0, rect.min.y + 1.0), solid);
    m.colored_vertex(pos2(x1, rect.min.y + 1.0), clear);
    m.colored_vertex(pos2(x1, rect.max.y - 1.0), clear);
    m.colored_vertex(pos2(x0, rect.max.y - 1.0), solid);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

/// Cover the right part of `rect` in `col`, fading in from the left.
fn fade_right(p: &egui::Painter, rect: Rect, col: Color32, from: f32, to: f32) {
    let x0 = rect.min.x + rect.width() * from;
    let x1 = rect.min.x + rect.width() * to;
    let mut m = egui::Mesh::default();
    let clear = col.gamma_multiply(0.0);
    m.colored_vertex(pos2(x0, rect.min.y + 1.0), clear);
    m.colored_vertex(pos2(x1, rect.min.y + 1.0), col);
    m.colored_vertex(pos2(x1, rect.max.y - 1.0), col);
    m.colored_vertex(pos2(x0, rect.max.y - 1.0), clear);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
    p.rect_filled(Rect::from_min_max(pos2(x1, rect.min.y + 1.0), rect.max - vec2(1.0, 1.0)), egui::Rounding { ne: 21.0, se: 21.0, ..Default::default() }, col);
}

/// Fade the bottom half of `rect` into `col` (pictures sinking into a card).
fn fade_down(p: &egui::Painter, rect: Rect, col: Color32) {
    let y0 = rect.min.y + rect.height() * 0.3;
    let mut m = egui::Mesh::default();
    let clear = col.gamma_multiply(0.0);
    m.colored_vertex(pos2(rect.min.x + 1.0, y0), clear);
    m.colored_vertex(pos2(rect.max.x - 1.0, y0), clear);
    m.colored_vertex(pos2(rect.max.x - 1.0, rect.max.y), col);
    m.colored_vertex(pos2(rect.min.x + 1.0, rect.max.y), col);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

/// Store screenshots built into the app for now, so the store works before
/// the `store/` folder is on GitHub (and offline). Unknown names fall back to
/// the download.
fn local_shot(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "breed" => include_bytes!("../../store/breed.png"),
        "breed_breeders" => include_bytes!("../../store/breed_breeders.png"),
        "breed_egg" => include_bytes!("../../store/breed_egg.png"),
        "calc" => include_bytes!("../../store/calc.png"),
        "cards" => include_bytes!("../../store/cards.png"),
        "cards_open" => include_bytes!("../../store/cards_open.png"),
        "cards_missions" => include_bytes!("../../store/cards_missions.png"),
        "calc_res" => include_bytes!("../../store/calc_res.png"),
        "gotchi" => include_bytes!("../../store/gotchi.png"),
        "bingo" => include_bytes!("../../store/bingo.png"),
        "rare" => include_bytes!("../../store/rare.png"),
        "replay" => include_bytes!("../../store/replay.png"),
        "gotchi_play" => include_bytes!("../../store/gotchi_play.png"),
        "gotchi_frames" => include_bytes!("../../store/gotchi_frames.png"),
        "info" => include_bytes!("../../store/info.png"),
        "info_adv" => include_bytes!("../../store/info_adv.png"),
        "map" => include_bytes!("../../store/map.png"),
        "map_rare" => include_bytes!("../../store/map_rare.png"),
        "pokedex" => include_bytes!("../../store/pokedex.png"),
        "pokedex_char" => include_bytes!("../../store/pokedex_char.png"),
        "shiny" => include_bytes!("../../store/shiny.png"),
        "shiny_count" => include_bytes!("../../store/shiny_count.png"),
        "shiny_dex" => include_bytes!("../../store/shiny_dex.png"),
        "shiny_runs" => include_bytes!("../../store/shiny_runs.png"),
        "sim" => include_bytes!("../../store/sim.png"),
        "sim_battles" => include_bytes!("../../store/sim_battles.png"),
        "sim_trainer" => include_bytes!("../../store/sim_trainer.png"),
        "team" => include_bytes!("../../store/team.png"),
        "team_sand" => include_bytes!("../../store/team_sand.png"),
        _ => return None,
    })
}
