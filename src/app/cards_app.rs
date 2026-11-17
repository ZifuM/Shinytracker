//! Card Collector: XP from hunting, booster packs to open, the binder of
//! cards you own, and missions. The rules and the save file live in
//! `crate::cards`; this is the app side (what earns XP, the pages, the
//! card pictures).

use super::store::pill_button;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_kit::{head, note};
use super::App;
use crate::apps::AppId;
use crate::cards::{self, Card, CardFile, Gain, Kind, Rarity, XP_PER_PACK};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// Width to height of a card.
pub(super) const CARD_ASPECT: f32 = 1.395;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum View {
    #[default]
    Binder,
    Packs,
    Missions,
    Trade,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Show {
    #[default]
    All,
    Owned,
    Missing,
    Dupes,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Sort {
    #[default]
    Number,
    Name,
    Rarity,
    Newest,
}


/// Where a pack opening is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stage {
    /// The pack, waiting for its top to be torn off.
    Tear,
    /// The cards sliding out.
    Out,
    /// The stack in your hand, face down: move the back two to the front?
    Back2,
    /// Face up: one card at a time, click for the next.
    Reveal,
    /// All of them together.
    Summary,
}

/// A pack being opened.
pub(super) struct Opening {
    pub set: String,
    /// Its art (one of the set's packs, at random).
    pub art: &'static str,
    /// The cards in the order they'd best be seen (rare last), and which
    /// are new to your binder.
    pub reveal: Vec<String>,
    pub new: Vec<bool>,
    /// The stack as it is now, front to back (indexes into `reveal`).
    pub order: Vec<usize>,
    pub stage: Stage,
    /// How much of the top is torn (0 to 1), and from which side.
    pub tear: f32,
    pub from_right: bool,
    /// The back two moving to the front (when it started).
    pub moving: Option<Instant>,
    /// Cards seen so far (front of the stack is `order[at]`).
    pub at: usize,
    /// The card sliding off to the pile, and when.
    pub slide: Option<(usize, Instant)>,
    pub since: Instant,
    /// When the front card was turned over (None: still face down, waiting
    /// for a click).
    pub flipped: Option<Instant>,
    /// "Show them all": the cards flip and go down by themselves, a little
    /// quicker than clicking (when the last step was).
    pub auto: Option<Instant>,
}

enum Pic {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// Card pictures: downloaded once (four at a time), then kept on disk.
#[derive(Default)]
pub(super) struct Pics {
    tex: HashMap<(String, bool), Pic>,
    queue: VecDeque<(String, bool)>,
    busy: usize,
    tx: Option<Sender<((String, bool), Option<egui::ColorImage>)>>,
    rx: Option<Receiver<((String, bool), Option<egui::ColorImage>)>>,
}

impl Pics {
    /// The picture, once it's here (asking for it if it isn't).
    pub fn get(&mut self, ctx: &egui::Context, id: &str, large: bool) -> Option<egui::TextureHandle> {
        let key = (id.to_string(), large);
        match self.tex.get(&key) {
            Some(Pic::Ready(t)) => return Some(t.clone()),
            Some(_) => return None,
            None => {}
        }
        self.tex.insert(key.clone(), Pic::Loading);
        // the big one asked for first jumps the queue
        if large {
            self.queue.push_front(key);
        } else {
            self.queue.push_back(key);
        }
        self.pump(ctx);
        None
    }

    fn pump(&mut self, ctx: &egui::Context) {
        if self.tx.is_none() {
            let (tx, rx) = channel();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }
        if let Some(rx) = &self.rx {
            while let Ok((key, img)) = rx.try_recv() {
                self.busy = self.busy.saturating_sub(1);
                let pic = match img {
                    Some(img) => Pic::Ready(ctx.load_texture(format!("card-{}-{}", key.0, key.1), img, egui::TextureOptions::LINEAR)),
                    None => Pic::Failed,
                };
                self.tex.insert(key, pic);
            }
        }
        while self.busy < 4 {
            let Some(key) = self.queue.pop_front() else { break };
            // a card's picture, a booster's art ("pack:<name>", or in grey
            // for a set not out yet: "greypack:<name>"), or a card you don't
            // have yet, darkened to grey ("grey:<id>")
            let grey = key.0.starts_with("grey:") || key.0.starts_with("greypack:");
            let pack_grey = key.0.starts_with("greypack:");
            let (url, file) = if let Some(name) = key.0.strip_prefix("pack:").or_else(|| key.0.strip_prefix("greypack:")) {
                (cards::pack_art_url(name), crate::config::cache_dir().join("cards").join("packs").join(format!("{name}.jpg")))
            } else {
                let id = key.0.trim_start_matches("grey:");
                let Some(card) = cards::by_id(id) else { continue };
                (card.image_url(key.1), crate::config::cache_dir().join("cards").join(format!("{}{}.png", id, if key.1 { "_hires" } else { "" })))
            };
            let tx = self.tx.clone().expect("made above");
            let c = ctx.clone();
            self.busy += 1;
            std::thread::spawn(move || {
                let decode = |b: &[u8]| -> Option<image::RgbaImage> {
                    if b.starts_with(&[0xFF, 0xD8]) {
                        super::avatar::decode_jpeg(b).ok()
                    } else {
                        image::load_from_memory(b).ok().map(|i| i.to_rgba8())
                    }
                };
                // only what's been downloaded (Settings → Card art); without
                // it the card is drawn by the app instead
                let _ = &url;
                let bytes = std::fs::read(&file).ok();
                let img = bytes.and_then(|b| decode(&b)).map(|mut i| {
                    if grey {
                        for px in i.pixels_mut() {
                            let l = 0.30 * px[0] as f32 + 0.59 * px[1] as f32 + 0.11 * px[2] as f32;
                            // a pack to come: a lighter grey, so its art still shows
                            let v = if pack_grey { (l * 0.62 + 14.0).min(255.0) as u8 } else { (l * 0.42 + 10.0).min(255.0) as u8 };
                            px[0] = v;
                            px[1] = v;
                            px[2] = v;
                        }
                    }
                    egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], &i)
                });
                let _ = tx.send((key, img));
                c.request_repaint();
            });
        }
    }

    /// Forget every picture, so they load again from disk (after art is
    /// downloaded or removed).
    pub fn reset(&mut self) {
        self.tex.clear();
        self.queue.clear();
    }

    /// Booster arts that weren't on this PC: look again.
    pub fn forget_missing_packs(&mut self) {
        self.tex.retain(|k, p| !((k.0.starts_with("pack:") || k.0.starts_with("greypack:")) && matches!(p, Pic::Failed)));
    }

    /// Keep downloads moving (called every frame).
    pub fn tick(&mut self, ctx: &egui::Context) {
        if self.busy > 0 || !self.queue.is_empty() {
            self.pump(ctx);
        }
    }
}

/// One set's art downloading.
pub(super) struct SaveAll {
    pub set: &'static str,
    pub total: usize,
    pub done: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub failed: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub bytes: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// One set's pictures: every card (small and large) and its booster arts,
/// as (address, where it's kept).
pub(super) fn set_art(set: &str) -> Vec<(String, std::path::PathBuf)> {
    let dir = crate::config::cache_dir().join("cards");
    let mut v = Vec::new();
    for c in cards::of_set(set) {
        for large in [false, true] {
            v.push((c.image_url(large), dir.join(format!("{}{}.png", c.id, if large { "_hires" } else { "" }))));
        }
    }
    if let Some(s) = cards::set(set) {
        for name in s.packs {
            v.push((cards::pack_art_url(name), dir.join("packs").join(format!("{name}.jpg"))));
        }
    }
    v
}

/// The playmat colours to pick from ("" = the set's own colour).
pub(super) const MATS: [(&str, &str, [u8; 3]); 8] = [
    ("", "Set colour", [0, 0, 0]),
    ("navy", "Navy", [44, 78, 170]),
    ("forest", "Forest", [44, 136, 74]),
    ("crimson", "Crimson", [176, 44, 56]),
    ("violet", "Violet", [116, 66, 180]),
    ("teal", "Teal", [32, 146, 156]),
    ("gold", "Gold", [196, 152, 44]),
    ("charcoal", "Charcoal", [96, 96, 108]),
];

/// The playmat's colour for a set.
pub(super) fn mat_col(mat: &str, set: &cards::Set) -> Color32 {
    match MATS.iter().find(|m| !m.0.is_empty() && m.0 == mat) {
        Some(m) => Color32::from_rgb(m.2[0], m.2[1], m.2[2]),
        None => Color32::from_rgb(set.col[0], set.col[1], set.col[2]),
    }
}

#[derive(Default)]
pub(super) struct CardsState {
    pub file: Option<CardFile>,
    pub load_error: Option<String>,
    pub dirty: Option<Instant>,
    pub saved_once: bool,
    pub save_error: Option<String>,
    pub view: View,
    pub query: String,
    pub set: Option<&'static str>,
    pub rarity: Option<Rarity>,
    pub group: Option<String>,
    pub show: Show,
    pub sort: Sort,
    /// The card open in the big view.
    pub open: Option<String>,
    /// Picking which art of this set's pack to open.
    pub choose: Option<&'static str>,
    pub opening: Option<Opening>,
    pub pics: Pics,
    /// XP just earned, for the bar to show for a moment.
    pub gains: VecDeque<(Gain, Instant)>,
    /// When the last pack was earned (the bar glows).
    pub pack_at: Option<Instant>,
    /// The XP bar overlay's window.
    pub bar_ui: super::cards_bar::BarState,
    /// The holo's sound has played for this pack.
    pub holo_played: bool,
    /// The cards made it out of the pack (sound played).
    pub out_played: bool,
    /// Every set's booster art fetched (once a run), so none is ever blank;
    /// and when to look for the new pictures.
    pub packs_fetched: bool,
    pub packs_done: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// The tear sound: how far along it last played, and when.
    tear_sound: (f32, Option<Instant>, u32),
    /// Saving every card picture and pack art to this PC.
    pub save_all: Vec<SaveAll>,
    /// When new sets coming out were last looked at.
    pub last_release_check: Option<Instant>,
    /// Each set's art on this PC: (files, bytes), and when that was checked.
    pub saved: Option<(Vec<(usize, u64)>, Instant)>,
    /// Settings opened by itself this session (the first-time art question).
    pub settings: bool,
    /// The trade record (read the first time it's needed).
    pub book: Option<crate::trade::Book>,
    /// The Trade tab.
    pub trade: super::cards_trade::TradeUi,
    /// The game's logs were found and are being read for XP.
    pub reads_logs: bool,
    last_log_read: Option<Instant>,
    /// Hunting time last passed on (seconds today).
    last_secs: u64,
    last_secs_check: Option<Instant>,
}

impl CardsState {
    pub fn file(&mut self) -> &mut CardFile {
        if self.file.is_none() {
            let (f, note) = cards::load_from(&crate::config::data_dir());
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(1500) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match cards::save_to(&crate::config::data_dir(), f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                self.dirty = Some(Instant::now());
            }
        }
    }

    /// Returns true when a pack was earned.
    fn note_gains(&mut self, gains: Vec<Gain>) -> bool {
        let mut pack = false;
        for g in gains {
            if g.packs > 0 {
                self.pack_at = Some(Instant::now());
                pack = true;
            }
            self.gains.push_back((g, Instant::now()));
        }
        while self.gains.len() > 6 {
            self.gains.pop_front();
        }
        self.touch();
        pack
    }
}

fn today() -> i64 {
    crate::stats::local_day(crate::config::now_unix())
}

impl App {
    fn cards_on(&self) -> bool {
        crate::apps::live(AppId::Cards)
    }

    /// Wild Pokémon met in one battle (species indexes, from the log).
    pub(super) fn cards_met(&mut self, names: &[usize], added: u32, rare_grass: bool) {
        if !self.cards_on() || added == 0 {
            return;
        }
        let mut dex: Vec<u32> = names.iter().map(|i| *i as u32 + 1).collect();
        // a name not read: still an encounter
        while dex.len() < added as usize {
            dex.push(0);
        }
        let gains = self.setup.cards.file().met(today(), &dex, rare_grass);
        if self.setup.cards.note_gains(gains) {
            self.play_popup(crate::sound::Sfx::Goal);
        }
    }

    /// Read what's new in the game's session logs and turn each wild
    /// Pokémon into XP. Only reads the log text; the first time, it starts
    /// from where the logs are now (no XP for older play).
    fn cards_read_logs(&mut self) {
        let Some(dir) = self.game_log_dir() else {
            self.setup.cards.reads_logs = false;
            return;
        };
        let mut cursors = std::mem::take(&mut self.setup.cards.file().logs);
        let read = crate::gamelog::read_new_wild(&dir, &mut cursors);
        self.setup.cards.file().logs = cursors;
        let Some((found, moved)) = read else {
            self.setup.cards.reads_logs = false;
            return;
        };
        self.setup.cards.reads_logs = true;
        if moved {
            self.setup.cards.touch();
        }
        if found.is_empty() {
            return;
        }
        let mut pack = false;
        for rare in [false, true] {
            let dex: Vec<u32> = found.iter().filter(|f| f.1 == rare).map(|f| f.0).collect();
            if dex.is_empty() {
                continue;
            }
            let gains = self.setup.cards.file().met(today(), &dex, rare);
            pack |= self.setup.cards.note_gains(gains);
        }
        if pack {
            self.play_popup(crate::sound::Sfx::Goal);
        }
        self.setup.cards.touch();
    }

    /// The folder with the game's session logs (None: not found).
    pub(super) fn game_log_dir(&self) -> Option<std::path::PathBuf> {
        let root = self.cfg.game_dir_setting().map(|d| crate::gamemap::game_dir(Some(d.as_path())));
        crate::gamelog::log_dir(root.as_deref())
    }

    /// A shiny logged (once per hunt and Pokémon, so Undo can't repeat it).
    pub(super) fn cards_shiny(&mut self, key: String) {
        if !self.cards_on() {
            return;
        }
        let seen = self.cfg.cards_shiny_keys.contains(&key);
        if seen {
            return;
        }
        self.cfg.cards_shiny_keys.push(key);
        let gains = self.setup.cards.file().shiny(today());
        self.setup.cards.note_gains(gains);
    }

    /// Every frame: pictures, hunting time (each few seconds), saving.
    pub(super) fn cards_tick(&mut self, ctx: &egui::Context) {
        self.setup.cards.pics.tick(ctx);
        if !self.cards_on() {
            return;
        }
        // the trade record, checked against the card save once per run
        if self.setup.cards.book.is_none() {
            let _ = self.cards_book();
        }
        // the game's logs, every few seconds: XP whether the overlay is
        // running or not, and for what happened while the app was closed
        if self.setup.cards.last_log_read.map_or(true, |t| t.elapsed() > Duration::from_secs(8)) {
            self.setup.cards.last_log_read = Some(Instant::now());
            self.cards_read_logs();
        }
        let due = self.setup.cards.last_secs_check.map_or(true, |t| t.elapsed() > Duration::from_secs(5));
        if due {
            self.setup.cards.last_secs_check = Some(Instant::now());
            let secs = self.today_hunt_secs();
            if secs != self.setup.cards.last_secs {
                self.setup.cards.last_secs = secs;
                let gains = self.setup.cards.file().hunt_time(today(), secs);
                if !gains.is_empty() && self.setup.cards.note_gains(gains) {
                    self.play_popup(crate::sound::Sfx::Goal);
                }
            }
        }
        // every booster's art, the sets to come too
        if !self.setup.cards.packs_fetched {
            self.setup.cards.packs_fetched = true;
            self.cards_fetch_pack_art(ctx);
        }
        // the boosters that were missing load again now they're here
        if self.setup.cards.packs_done.as_ref().is_some_and(|d| d.load(std::sync::atomic::Ordering::Relaxed)) {
            self.setup.cards.packs_done = None;
            self.setup.cards.pics.forget_missing_packs();
            self.setup.cards.saved = None;
        }
        // new sets coming out
        if self.setup.cards.last_release_check.map_or(true, |t| t.elapsed() > Duration::from_secs(2)) {
            self.setup.cards.last_release_check = Some(Instant::now());
            self.cards_release_tick(ctx);
        }
        // drop old XP pop-ups
        while self.setup.cards.gains.front().is_some_and(|g| g.1.elapsed() > Duration::from_secs(4)) {
            self.setup.cards.gains.pop_front();
        }
        self.setup.cards.save_tick(false);
    }

    /// Every set's booster art that isn't on this PC yet (a few small
    /// pictures), so the packs, the ones still to come included, are never
    /// blank. The card pictures themselves still download only when asked.
    fn cards_fetch_pack_art(&mut self, ctx: &egui::Context) {
        let dir = crate::config::cache_dir().join("cards").join("packs");
        let todo: Vec<(String, std::path::PathBuf)> = cards::SETS
            .iter()
            .flat_map(|s| s.packs.iter())
            .map(|name| (cards::pack_art_url(name), dir.join(format!("{name}.jpg"))))
            .filter(|(_, f)| !f.exists())
            .collect();
        if todo.is_empty() {
            return;
        }
        let c = ctx.clone();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.setup.cards.packs_done = Some(done.clone());
        std::thread::spawn(move || {
            for (url, file) in todo {
                let ok = crate::sprites::fetch(&url).ok().filter(|b| b.len() > 100 && b.starts_with(&[0xFF, 0xD8]));
                if let Some(b) = ok {
                    if let Some(parent) = file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&file, &b);
                }
            }
            done.store(true, std::sync::atomic::Ordering::Relaxed);
            c.request_repaint();
        });
    }

    /// A set that's just come out: tell the player if they asked, and get
    /// its art if they asked for that. Each set once (also when the app
    /// wasn't open at the time).
    fn cards_release_tick(&mut self, ctx: &egui::Context) {
        let now = crate::config::now_unix();
        for s in cards::SETS.iter().filter(|s| s.release > 0 && s.out_at(now)) {
            let file = self.setup.cards.file();
            if file.seen_out.iter().any(|x| x == s.id) {
                continue;
            }
            file.seen_out.push(s.id.to_string());
            let tell = file.notify.iter().any(|x| x == s.id);
            let get = file.auto_get.iter().any(|x| x == s.id);
            self.setup.cards.touch();
            if get {
                self.cards_download_set(ctx, s.id);
            }
            if tell {
                self.notify(super::notices::Notice {
                    id: "cards-new-set",
                    icon: "🃏",
                    title: format!("{} is out!", s.name),
                    body: if get { "Its card art is downloading. Open a pack in the Card Collector.".to_string() } else { "A new booster to open in the Card Collector.".to_string() },
                    progress: None,
                    action: Some(("Open packs", super::notices::NoticeAction::OpenPacks)),
                    expires: Some(Instant::now() + Duration::from_secs(30)),
                    accent: GOLD,
                });
                self.app_toast(format!("🃏  {} is out! Open a pack in the Card Collector", s.name));
                self.play_popup(crate::sound::Sfx::Milestone);
            }
        }
    }

    // ───────────────────────────── drawing cards ─────────────────────────────

    /// A card at `r`: its picture when it's here, else a drawn stand-in.
    /// `holo`: a shine moves across it.
    pub(super) fn draw_card(&mut self, ui: &egui::Ui, r: Rect, c: &Card, large: bool, holo: bool) {
        let p = ui.painter_at(r.expand(2.0));
        let tex = self.setup.cards.pics.get(ui.ctx(), &c.id, large).or_else(|| if large { self.setup.cards.pics.get(ui.ctx(), &c.id, false) } else { None });
        let round = r.width() * 0.045;
        // A holo with its picture here: it tips towards the pointer and its
        // starlight foil catches the light (see `foil_card`).
        if holo && c.rarity == Rarity::Holo {
            if let Some(t) = &tex {
                let fp = ui.painter_at(r.expand(r.width() * 0.08));
                let evolves = c.stage.starts_with("Stage");
                let ecard = cards::set(c.set).is_some_and(|s| s.ecard());
                let hovered = foil_card(ui, &fp, r, round, &c.id, evolves, ecard, t);
                ui.ctx().request_repaint_after(Duration::from_millis(if hovered { 16 } else { 33 }));
                return;
            }
        }
        match tex {
            Some(t) => {
                super::store::paint_image(&p, &t, r, egui::Rounding::same(round), 1.0, false);
            }
            None => self.draw_card_standin(&p, r, c),
        }
        if holo && c.rarity == Rarity::Holo {
            shine(&p, r, ui.input(|i| i.time) as f32);
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }
    }

    /// A card you don't have yet: its picture, dark and grey.
    pub(super) fn draw_card_missing(&mut self, ui: &egui::Ui, r: Rect, c: &Card, large: bool) {
        let p = ui.painter_at(r.expand(2.0));
        let round = r.width() * 0.045;
        match self.setup.cards.pics.get(ui.ctx(), &format!("grey:{}", c.id), large) {
            Some(t) => super::store::paint_image(&p, &t, r, egui::Rounding::same(round), 1.0, false),
            None => {
                self.draw_card_standin(&p, r, c);
                p.rect_filled(r, round, Color32::from_rgba_unmultiplied(14, 14, 20, 200));
            }
        }
    }

    /// The card drawn by the app (before its picture downloads, or offline).
    fn draw_card_standin(&mut self, p: &egui::Painter, r: Rect, c: &Card) {
        let s = r.width() / 200.0;
        let round = r.width() * 0.045;
        let col = type_col(c.group());
        p.rect_filled(r, round, Color32::from_rgb(236, 206, 72));
        let inner = r.shrink(9.0 * s);
        p.rect_filled(inner, round * 0.6, theme::blend(Color32::from_rgb(250, 248, 240), col, 0.55));
        // name and HP
        let f = |size: f32| egui::FontId::proportional((size * s).max(6.0));
        p.with_clip_rect(inner).text(inner.min + vec2(8.0 * s, 6.0 * s), Align2::LEFT_TOP, &c.name, crate::fonts::sans_bold_font((13.0 * s).max(6.0)), Color32::from_rgb(30, 30, 36));
        if c.hp > 0 {
            p.text(pos2(inner.max.x - 8.0 * s, inner.min.y + 7.0 * s), Align2::RIGHT_TOP, format!("{} HP", c.hp), f(10.0), Color32::from_rgb(170, 30, 30));
        }
        // the picture box
        let art = Rect::from_min_size(inner.min + vec2(8.0 * s, 26.0 * s), vec2(inner.width() - 16.0 * s, inner.height() * 0.42));
        p.rect_filled(art, 3.0 * s, theme::blend(Color32::from_rgb(220, 226, 236), col, 0.3));
        p.rect_stroke(art, 3.0 * s, Stroke::new(1.5 * s, Color32::from_rgb(190, 170, 90)));
        match c.kind {
            Kind::Pokemon => {
                if let Some(sp) = self.species.get((c.dex as usize).wrapping_sub(1)).cloned() {
                    self.draw_sprite_centered(p, &sp, false, art.center(), art.size() * 0.92);
                }
            }
            Kind::Trainer => {
                p.text(art.center(), Align2::CENTER_CENTER, "🎒", egui::FontId::proportional(art.height() * 0.45), Color32::from_rgb(90, 90, 100));
            }
            Kind::Energy => {
                p.circle_filled(art.center(), art.height() * 0.3, col);
                p.circle_stroke(art.center(), art.height() * 0.3, Stroke::new(2.0 * s, Color32::WHITE));
            }
        }
        // what it is, then the number and rarity at the bottom
        let kind = match c.kind {
            Kind::Pokemon => format!("{} Pokémon", c.stage),
            Kind::Trainer => "Trainer".into(),
            Kind::Energy => if c.stage == "Special" { "Special Energy".into() } else { "Basic Energy".into() },
        };
        p.with_clip_rect(inner).text(pos2(inner.center().x, art.max.y + 12.0 * s), Align2::CENTER_CENTER, kind, f(9.5), Color32::from_rgb(70, 70, 80));
        let total = cards::of_set(c.set).len();
        p.text(pos2(inner.max.x - 8.0 * s, inner.max.y - 7.0 * s), Align2::RIGHT_BOTTOM, format!("{}/{total} {}", c.number(), c.rarity.symbol()), f(9.0), Color32::from_rgb(40, 40, 46));
        p.text(pos2(inner.min.x + 8.0 * s, inner.max.y - 7.0 * s), Align2::LEFT_BOTTOM, c.set_name(), f(9.0), Color32::from_rgb(90, 90, 100));
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_cards(&mut self, ui: &mut egui::Ui) {
        self.setup.cards.file().roll_over(today());
        if self.setup.cards.opening.is_some() {
            return self.cards_opening(ui);
        }
        super::app_tour::here(ui.ctx(), "cards");
        let n_out = cards::all_out().len();
        kit::page_header(ui, "CARD COLLECTOR", &format!("Hunting earns XP. Fill the bar for a booster pack of the original cards, and collect all {n_out}."));
        if let Some(e) = self.setup.cards.load_error.clone() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        let tour_top = ui.cursor().min.y;
        self.cards_hero(ui);
        super::app_tour::mark_since(ui, "cards.hero", tour_top);
        ui.add_space(4.0);
        let packs = self.setup.cards.file().packs;
        let mut view = self.setup.cards.view;
        let packs_label = if packs > 0 { format!("🎁  Open packs ({packs})") } else { "🎁  Packs".to_string() };
        let tour_top = ui.cursor().min.y;
        kit::tab_bar(ui, &mut view, &[(View::Binder, "📚  Binder"), (View::Packs, packs_label.as_str()), (View::Missions, "🎯  Missions"), (View::Trade, "🔄  Trade")]);
        super::app_tour::mark_since(ui, "cards.tabs", tour_top);
        self.setup.cards.view = view;
        ui.add_space(14.0);
        match view {
            View::Binder => self.cards_binder(ui),
            View::Packs => self.cards_packs(ui),
            View::Missions => self.cards_missions(ui),
            View::Trade => self.cards_trade(ui),
        }
        self.cards_detail(ui.ctx());
        self.cards_choose_pack(ui.ctx());
        self.cards_jobs_tick();
        // the first time: Settings opens at the Card Collector, to pick which
        // sets' art to download (once a session; closing it counts as asked)
        if !self.setup.cards.file().art_asked && !self.setup.cards.settings && self.setup.settings_open.is_none() {
            self.setup.cards.settings = true;
            self.open_settings(super::Tab::Cards);
        }
    }

    /// The top: one panel, three even columns (next pack, packs to open,
    /// collection). Every column has the same rows at the same heights:
    /// heading, big number, a bar or button, a footer line.
    fn cards_hero(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        let mut open = false;
        let mut go_missions = false;
        kit::card(ui, |ui| {
            let w = ui.available_width();
            let wide = w >= 760.0;
            const COL_H: f32 = 134.0;
            let gap = 40.0;
            let (area, _) = ui.allocate_exact_size(vec2(w, if wide { COL_H } else { COL_H * 3.0 + gap * 2.0 }), Sense::hover());
            let cw = if wide { ((w - gap * 2.0) / 3.0).floor() } else { w };
            let col = |k: usize| {
                if wide {
                    Rect::from_min_size(pos2(area.min.x + k as f32 * (cw + gap), area.min.y), vec2(cw, COL_H))
                } else {
                    Rect::from_min_size(pos2(area.min.x, area.min.y + k as f32 * (COL_H + gap)), vec2(cw, COL_H))
                }
            };
            // dividers between the columns
            for k in 1..3 {
                let r = col(k);
                let (a, b) = if wide {
                    (pos2(r.min.x - gap / 2.0, area.min.y + 2.0), pos2(r.min.x - gap / 2.0, area.max.y - 2.0))
                } else {
                    (pos2(area.min.x, r.min.y - gap / 2.0), pos2(area.max.x, r.min.y - gap / 2.0))
                };
                ui.painter().line_segment([a, b], Stroke::new(1.0f32, LINE()));
            }
            let t = ui.input(|i| i.time) as f32;

            // ── next pack
            let r = col(0);
            hero_head(ui, r, "⭐", GOLD, "Next pack", "");
            hero_big(ui, r, &f.xp.to_string(), &format!("/ {XP_PER_PACK} XP"));
            let br = hero_row(r, 10.0);
            xp_bar(ui.painter(), br, f.xp as f32 / XP_PER_PACK as f32, t, self.setup.cards.pack_at);
            ui.interact(br.expand2(vec2(0.0, 8.0)), ui.id().with("hero-xp"), Sense::hover()).on_hover_text(xp_rules());
            hero_foot(ui, r, &format!("{} XP to go", XP_PER_PACK.saturating_sub(f.xp)), 0.45);
            let left = f.day.missions.iter().filter(|m| !m.done).count();
            if left > 0 && hero_link(ui, r, &format!("{left} daily mission{} left ›", if left == 1 { "" } else { "s" }), "hero-missions") {
                go_missions = true;
            }

            // ── packs to open
            let r = col(1);
            hero_head(ui, r, "🎁", kit::C_PINK, "Packs to open", &format!("{} opened", f.opened));
            hero_big(ui, r, &f.packs.to_string(), if f.packs == 1 { "pack waiting" } else { "packs waiting" });
            let row = hero_row(r, 30.0);
            let br = Rect::from_min_size(row.min, vec2(150.0f32.min(row.width()), 30.0));
            if pill_button(ui, br, if f.packs > 0 { "🎁  Open a pack" } else { "No packs yet" }, f.packs > 0, "cards-open-hero").clicked() && f.packs > 0 {
                open = true;
            }
            hero_foot(ui, r, if f.packs > 0 { "Ready to open" } else { "Fill the XP bar or finish missions for one" }, 1.0);

            // ── collection
            let r = col(2);
            let total = cards::all_out().len();
            let have = f.unique(None);
            hero_head(ui, r, "📚", kit::C_CYAN, "Collection", &format!("{}%", (have * 100) / total.max(1)));
            hero_big(ui, r, &have.to_string(), &format!("/ {total} cards"));
            let br = hero_row(r, 10.0);
            kit::paint_bar(ui.painter(), br, have as f32 / total.max(1) as f32);
            let sets: Vec<String> = cards::sets_out().iter().map(|s| format!("{} {}/{}", s.name, f.unique(Some(s.id)), cards::of_set(s.id).len())).collect();
            hero_foot(ui, r, &sets.join("  ·  "), 1.0);
        });
        if open {
            self.setup.cards.view = View::Packs;
        }
        if go_missions {
            self.setup.cards.view = View::Missions;
        }
    }

    // ───────────────────────────── the binder ─────────────────────────────

    fn cards_binder(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "cards/binder");
        let f = self.setup.cards.file().clone();
        // filters: one slim toolbar. A search box that takes the spare
        // width, then five same-size drop-downs (set, rarity, type, show,
        // sort). Each lights up in its colour when it's narrowing the list.
        {
            let all = cards::all_out();
            let out = cards::sets_out();
            let owned = |c: &Card| f.count(&c.id) > 0;
            let tally = |keep: &dyn Fn(&Card) -> bool| {
                let (h, n) = all.iter().filter(|c| keep(c)).fold((0, 0), |(h, n), c| (h + owned(c) as usize, n + 1));
                format!("{h}/{n}")
            };
            let s = &self.setup.cards;
            // (key, value shown, colour when on, is it narrowing?)
            let set_v = s.set.and_then(|id| out.iter().copied().find(|x| x.id == id));
            let pills: [(&str, String, Color32, bool); 5] = [
                ("Set", set_v.map_or("All".into(), |x| x.name.to_string()), set_v.map_or(kit::primary(), |x| Color32::from_rgb(x.col[0], x.col[1], x.col[2])), set_v.is_some()),
                ("Rarity", s.rarity.map_or("Any".into(), |r| r.name().to_string()), s.rarity.map_or(kit::primary(), rarity_col), s.rarity.is_some()),
                ("Type", s.group.clone().unwrap_or_else(|| "Any".into()), s.group.as_deref().map_or(kit::primary(), type_col), s.group.is_some()),
                ("Show", match s.show { Show::All => "All", Show::Owned => "Owned", Show::Missing => "Missing", Show::Dupes => "Duplicates" }.into(), kit::primary(), s.show != Show::All),
                ("Sort", match s.sort { Sort::Number => "No.", Sort::Name => "A–Z", Sort::Rarity => "Rarity", Sort::Newest => "Newest" }.into(), kit::primary(), false),
            ];
            const H: f32 = 34.0;
            const GAP: f32 = 8.0;
            // each pill is as wide as its longest choice (plus the dot), so
            // picking something never shifts the row
            let choices: [Vec<&str>; 5] = [
                std::iter::once("All").chain(out.iter().map(|x| x.name)).collect(),
                vec!["Any", "Common", "Uncommon", "Rare", "Holo Rare"],
                std::iter::once("Any").chain(GROUPS.iter().copied()).collect(),
                vec!["All", "Owned", "Missing", "Duplicates"],
                vec!["No.", "A–Z", "Rarity", "Newest"],
            ];
            let mut widths: Vec<f32> = pills
                .iter()
                .zip(choices.iter())
                .enumerate()
                .map(|(k, ((key, _, _, _), vs))| vs.iter().map(|v| drop_width(ui, key, v)).fold(0.0f32, f32::max) + if k < 4 { 12.0 } else { 0.0 })
                .collect();
            let drops_w: f32 = widths.iter().sum::<f32>() + GAP * (widths.len() as f32 - 1.0);
            let w = ui.available_width();
            let one_line = w - drops_w - GAP >= 220.0;
            // where each pill goes: on the search box's row when there's
            // room, else in rows of their own under it
            let mut spots: Vec<egui::Pos2> = Vec::new();
            let mut lines = 1usize;
            if one_line {
                let mut x = w - drops_w;
                for pw in &widths {
                    spots.push(pos2(x, 0.0));
                    x += pw + GAP;
                }
            } else {
                // a tidy grid: as few rows as will fit, split evenly, every
                // pill the same width so the columns line up
                let maxw = widths.iter().copied().fold(0.0f32, f32::max);
                let n = widths.len();
                let fit = (((w + GAP) / (maxw + GAP)).floor() as usize).clamp(1, n);
                let rows = n.div_ceil(fit);
                let per = n.div_ceil(rows);
                let cw = ((w - GAP * (per as f32 - 1.0)) / per as f32).floor();
                lines = 1 + rows;
                for k in 0..n {
                    spots.push(pos2((k % per) as f32 * (cw + GAP), (1 + k / per) as f32 * (H + GAP)));
                    widths[k] = cw;
                }
            }
            let (row, _) = ui.allocate_exact_size(vec2(w, H * lines as f32 + GAP * (lines as f32 - 1.0)), Sense::hover());
            super::app_tour::mark(ui.ctx(), "binder.filters", row);
            let search_r = Rect::from_min_size(row.min, vec2(if one_line { w - drops_w - GAP * 2.0 } else { w }, H));
            // the search box, sized to its rect (its margins are measured once and kept)
            let pad_id = egui::Id::new("cards-filter-search-pad");
            let pad: f32 = ui.ctx().data(|d| d.get_temp(pad_id)).unwrap_or(40.0);
            let want = (search_r.width() - pad).max(80.0);
            let q = &mut self.setup.cards.query;
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(search_r).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let got = kit::search_box_keyed(&mut child, Some(egui::Id::new("cards-search")), q, "Search cards by name or number", want).rect.width();

            // the drop-downs
            let mut pick: Option<(usize, usize)> = None;
            for (k, (key, val, col, on)) in pills.iter().enumerate() {
                let r = Rect::from_min_size(row.min + spots[k].to_vec2(), vec2(widths[k], H));
                let resp = drop_pill(ui, r, key, val, *col, *on, ("cards-drop", k));
                let s = &self.setup.cards;
                let items: Vec<(String, Option<Color32>, String, bool)> = match k {
                    0 => std::iter::once(("All sets".to_string(), None, tally(&|_| true), s.set.is_none()))
                        .chain(out.iter().map(|x| (x.name.to_string(), Some(Color32::from_rgb(x.col[0], x.col[1], x.col[2])), tally(&|c: &Card| c.set == x.id), s.set == Some(x.id))))
                        .collect(),
                    1 => std::iter::once(("Any rarity".to_string(), None, tally(&|_| true), s.rarity.is_none()))
                        .chain([Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Holo].into_iter().map(|r| (r.name().to_string(), Some(rarity_col(r)), tally(&|c: &Card| c.rarity == r), s.rarity == Some(r))))
                        .collect(),
                    2 => std::iter::once(("Any type".to_string(), None, tally(&|_| true), s.group.is_none()))
                        .chain(GROUPS.iter().map(|g| (g.to_string(), Some(type_col(g)), tally(&|c: &Card| c.group() == *g), s.group.as_deref() == Some(*g))))
                        .collect(),
                    3 => {
                        let n = all.len();
                        let have = all.iter().filter(|c| owned(c)).count();
                        let dup = all.iter().filter(|c| f.count(&c.id) > 1).count();
                        vec![
                            ("All cards".into(), None, n.to_string(), s.show == Show::All),
                            ("Owned".into(), None, have.to_string(), s.show == Show::Owned),
                            ("Missing".into(), None, (n - have).to_string(), s.show == Show::Missing),
                            ("Duplicates".into(), None, dup.to_string(), s.show == Show::Dupes),
                        ]
                    }
                    _ => vec![
                        ("Card number".into(), None, String::new(), s.sort == Sort::Number),
                        ("Name A–Z".into(), None, String::new(), s.sort == Sort::Name),
                        ("Rarity".into(), None, String::new(), s.sort == Sort::Rarity),
                        ("Newest pulls".into(), None, String::new(), s.sort == Sort::Newest),
                    ],
                };
                if let Some(i) = drop_menu(ui, &resp, ui.id().with(("cards-drop-menu", k)), &items) {
                    pick = Some((k, i));
                }
            }
            if let Some((k, i)) = pick {
                let s = &mut self.setup.cards;
                match k {
                    0 => s.set = if i == 0 { None } else { out.get(i - 1).map(|x| x.id) },
                    1 => s.rarity = [None, Some(Rarity::Common), Some(Rarity::Uncommon), Some(Rarity::Rare), Some(Rarity::Holo)][i],
                    2 => s.group = if i == 0 { None } else { Some(GROUPS[i - 1].to_string()) },
                    3 => s.show = [Show::All, Show::Owned, Show::Missing, Show::Dupes][i],
                    _ => s.sort = [Sort::Number, Sort::Name, Sort::Rarity, Sort::Newest][i],
                }
            }
            // keep the search box's own margins for next frame's sizing
            let p2 = got - want;
            if (p2 - pad).abs() > 0.5 && p2 > -1.0 {
                ui.ctx().data_mut(|d| d.insert_temp(pad_id, p2.max(0.0)));
                ui.ctx().request_repaint();
            }
            ui.add_space(10.0);
        }

        // which cards
        let s = &self.setup.cards;
        let q = s.query.trim().to_lowercase();
        let num: Option<u16> = q.parse().ok();
        let mut list: Vec<&'static Card> = cards::all_out()
            .into_iter()
            .filter(|c| s.set.map_or(true, |x| c.set == x))
            .filter(|c| s.rarity.map_or(true, |r| c.rarity == r))
            .filter(|c| s.group.as_deref().map_or(true, |g| c.group() == g))
            .filter(|c| match s.show {
                Show::All => true,
                Show::Owned => f.count(&c.id) > 0,
                Show::Missing => f.count(&c.id) == 0,
                Show::Dupes => f.count(&c.id) > 1,
            })
            .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q) || num == Some(c.num) || c.number().to_lowercase() == q)
            .collect();
        match s.sort {
            Sort::Number => {}
            Sort::Name => list.sort_by(|a, b| a.name.cmp(&b.name)),
            Sort::Rarity => list.sort_by_key(|c| std::cmp::Reverse(c.rarity)),
            Sort::Newest => list.sort_by_key(|c| std::cmp::Reverse(f.first.get(&c.id).copied().unwrap_or(0))),
        }
        let any = s.set.is_some() || s.rarity.is_some() || s.group.is_some() || s.show != Show::All || !s.query.is_empty();
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Showing {} of {} cards", list.len(), cards::all_out().len())).size(kit::fs(12.0)).color(MUTED()));
            if any {
                ui.add_space(6.0);
                if super::w_kit::link(ui, "Clear filters") {
                    clear = true;
                }
            }
        });
        if clear {
            let s = &mut self.setup.cards;
            s.set = None;
            s.rarity = None;
            s.group = None;
            s.show = Show::All;
            s.query.clear();
            return;
        }
        ui.add_space(10.0);
        if list.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                note(ui, if f.owned.is_empty() && s.show == Show::Owned { "No cards yet. Hunt to fill the XP bar, then open your first pack." } else { "No cards match. Try clearing the filters." });
            });
            return;
        }
        // the grid (only the rows on screen draw)
        let w = ui.available_width();
        let gap = 14.0;
        let cols = ((w + gap) / (150.0 + gap)).floor().max(2.0) as usize;
        let cw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let ch = (cw * CARD_ASPECT).round();
        let row_h = ch + 40.0;
        let now = crate::config::now_unix();
        let mut open: Option<String> = None;
        for row in list.chunks(cols) {
            let (rr, _) = ui.allocate_exact_size(vec2(w, row_h), Sense::hover());
            ui.add_space(gap);
            if !ui.is_rect_visible(rr) {
                continue;
            }
            for (k, c) in row.iter().enumerate() {
                let r = Rect::from_min_size(pos2(rr.min.x + k as f32 * (cw + gap), rr.min.y), vec2(cw, ch));
                let n = f.count(&c.id);
                let resp = ui.interact(r, ui.id().with(("card", &c.id)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let lift = r.translate(vec2(0.0, -4.0 * ht));
                if n > 0 {
                    ui.painter().add(kit::shadow_tier(if ht > 0.0 { 3 } else { 1 }).as_shape(lift, 8.0));
                    self.draw_card(ui, lift, c, false, ht > 0.0);
                    // badges sit on the corners, off the card's name and HP
                    if n > 1 {
                        let b = Rect::from_center_size(pos2(lift.max.x - 4.0, lift.max.y - 4.0), vec2(30.0, 20.0));
                        ui.painter().rect_filled(b, 10.0, Color32::from_rgb(30, 32, 44));
                        ui.painter().rect_stroke(b, 10.0, Stroke::new(1.0f32, Color32::from_white_alpha(60)));
                        ui.painter().text(b.center(), Align2::CENTER_CENTER, format!("×{n}"), crate::fonts::sans_bold_font(11.0), Color32::WHITE);
                    }
                    if f.first.get(&c.id).is_some_and(|t| now.saturating_sub(*t) < 86_400) {
                        let b = Rect::from_center_size(pos2(lift.min.x + 14.0, lift.min.y - 2.0), vec2(38.0, 18.0));
                        ui.painter().rect_filled(b, 9.0, kit::C_PINK);
                        ui.painter().text(b.center(), Align2::CENTER_CENTER, "NEW", crate::fonts::sans_bold_font(10.0), Color32::WHITE);
                    }
                } else {
                    // not yours yet: shown dark and grey
                    self.draw_card_missing(ui, lift, c, false);
                    if ht > 0.0 {
                        ui.painter().rect_stroke(lift, 8.0, Stroke::new(1.0f32, theme::blend(LINE(), rarity_col(c.rarity), 0.6 * ht)));
                    }
                }
                // name and number under it
                let ty = rr.min.y + ch + 8.0;
                let p = ui.painter();
                let name = super::charts::clip(ui, &c.name, &crate::fonts::sans_bold_font(12.5), cw);
                p.text(pos2(r.min.x + 2.0, ty + 7.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(12.5), if n > 0 { TEXT() } else { MUTED() });
                p.text(pos2(r.min.x + 2.0, ty + 23.0), Align2::LEFT_CENTER, format!("{} · #{}", c.set_name(), c.number()), crate::fonts::sans_font(11.0), MUTED());
                p.text(pos2(r.max.x - 2.0, ty + 23.0), Align2::RIGHT_CENTER, c.rarity.symbol(), egui::FontId::proportional(11.0), rarity_col(c.rarity));
                if resp.on_hover_text(if n > 0 { "Look at it" } else { "Not in your binder yet" }).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(c.id.clone());
                }
            }
        }
        if open.is_some() {
            self.setup.cards.open = open;
        }
    }

    /// Spare duplicates for XP (on the Trade tab, next to trading with
    /// other players).
    pub(super) fn cards_trade_spares(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.cards.file().clone();
        // trade spares
        let (spare, xp) = f.spare();
        let mut trade = false;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "♻", kit::C_GOLD, "Or trade your spares for XP", None);
            ui.add_space(4.0);
            if spare == 0 {
                note(ui, "No spares right now. Duplicates you pull can go to a friend above, or be traded in here for XP. You always keep one of every card.");
            } else {
                note(ui, format!("You have {spare} spare duplicate{}. Trade them all in for {xp} XP; you keep one of each card. (Want to give one to a friend instead? Send it above first.)", if spare == 1 { "" } else { "s" }));
                ui.add_space(8.0);
                let (r, _) = ui.allocate_exact_size(vec2(220.0, 32.0), Sense::hover());
                trade = pill_button(ui, r, &format!("♻  Trade for {xp} XP"), true, "cards-trade").clicked();
            }
        });
        if trade {
            if let Some(g) = self.setup.cards.file().trade_spares() {
                self.app_toast(format!("+{} XP from duplicates", g.xp));
                let _ = self.setup.cards.note_gains(vec![g]);
            }
        }
    }

    /// A card, big, over the page.
    fn cards_detail(&mut self, ctx: &egui::Context) {
        let Some(id) = self.setup.cards.open.clone() else { return };
        let Some(c) = cards::by_id(&id) else {
            self.setup.cards.open = None;
            return;
        };
        let f = self.setup.cards.file().clone();
        let n = f.count(&id);
        let screen = ctx.screen_rect();
        let mut close = false;
        let mut dex = false;
        let mut send = false;
        egui::Area::new(egui::Id::new("card-detail")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(170));
            if scrim.clicked() {
                close = true;
            }
            let ch = (screen.height() - 120.0).clamp(300.0, 620.0);
            let cw = ch / CARD_ASPECT;
            let total_w = cw + 40.0 + 300.0;
            let left = screen.center().x - total_w / 2.0;
            let cr = Rect::from_min_size(pos2(left, screen.center().y - ch / 2.0), vec2(cw, ch));
            ui.painter().add(kit::shadow_tier(3).as_shape(cr, 14.0));
            if n > 0 {
                // tilt a little towards the pointer for a bit of life
                self.draw_card(ui, cr, c, true, true);
            } else {
                self.draw_card_missing(ui, cr, c, true);
            }
            ui.interact(cr, egui::Id::new("card-detail-card"), Sense::click());
            // the facts on the right
            let ir = Rect::from_min_size(pos2(cr.max.x + 40.0, cr.min.y + 20.0), vec2(300.0, ch - 40.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(ir).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                ui.label(RichText::new(c.set_name().to_uppercase()).font(crate::fonts::sans_bold_font(11.0)).color(Color32::from_gray(190)));
                ui.label(RichText::new(&c.name).font(crate::fonts::sans_bold_font(28.0)).color(Color32::WHITE));
                ui.add_space(6.0);
                let line = |ui: &mut egui::Ui, a: &str, b: String| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(a).size(13.0).color(Color32::from_gray(170)));
                        ui.label(RichText::new(b).size(13.0).color(Color32::WHITE));
                    });
                };
                line(ui, "Number", format!("{} of {}", c.number(), cards::of_set(c.set).len()));
                line(ui, "Rarity", format!("{} {}", c.rarity.symbol(), c.rarity.name()));
                if let Some(n) = cards::packs_per(c) {
                    let n = if n < 10.0 { format!("{n:.1}") } else { format!("{}", n.round() as u64) };
                    line(ui, "Pull rate", format!("about 1 in {n} {} packs", cards::set(c.set).map_or("", |s| s.name)));
                }
                match c.kind {
                    Kind::Pokemon => line(ui, "Pokémon", format!("{} · {} type · {} HP", c.stage, c.ty, c.hp)),
                    Kind::Trainer => line(ui, "Kind", "Trainer".into()),
                    Kind::Energy => line(ui, "Kind", "Energy".into()),
                }
                ui.add_space(10.0);
                if n > 0 {
                    line(ui, "You have", format!("{n}"));
                    if let Some(t) = f.first.get(&id) {
                        line(ui, "First pulled", crate::stats::fmt_day(crate::stats::local_day(*t)));
                    }
                } else {
                    ui.label(RichText::new("Not in your binder yet. Keep opening packs!").size(13.0).color(Color32::from_gray(200)));
                }
                if let Some(t) = f.traded_in.get(&id).filter(|t| **t > 0) {
                    line(ui, "From trades", format!("{t}"));
                }
                ui.add_space(16.0);
                if c.kind == Kind::Pokemon && self.has_app(AppId::Pokedex) && ui.add(egui::Button::new(RichText::new("📖  Its Pokédex page").size(13.0))).clicked() {
                    dex = true;
                }
                if n > 0 {
                    ui.add_space(6.0);
                    if ui.add(egui::Button::new(RichText::new("🔄  Trade this card").size(13.0))).clicked() {
                        send = true;
                    }
                }
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new("Close").size(13.0))).clicked() {
                    close = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if send {
            self.setup.cards.open = None;
            let t = &mut self.setup.cards.trade;
            t.pick = Some(id.clone());
            t.made = None;
            t.confirm_last = false;
            self.setup.cards.view = View::Trade;
        } else if dex {
            let id = self.wiki.as_ref().and_then(|w| w.by_dex(c.dex as u32)).map(|m| m.id.clone());
            self.setup.cards.open = None;
            if let Some(id) = id {
                self.open_dex_entry(&id);
            }
        } else if close {
            self.setup.cards.open = None;
        }
    }

    // ───────────────────────────── packs ─────────────────────────────

    /// Modern Plus: the next card set as an event. Its booster (in grey),
    /// when it comes out (always at the game's daily reset), a big
    /// countdown, Remind me, and the sets after it. Returns a change to
    /// Remind me.
    fn cards_next_drop(&mut self, ui: &mut egui::Ui, s: &cards::Set, f: &CardFile, now: u64) -> Option<(&'static str, bool)> {
        let mut out = None;
        let left = s.release.saturating_sub(now);
        let pl = theme::pal();
        let frame = egui::Frame::none().rounding(18.0).inner_margin(egui::Margin::same(18.0));
        let slot = ui.painter().add(egui::Shape::Noop);
        let resp = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // its booster, in grey
                let (r, _) = ui.allocate_exact_size(vec2(96.0, 168.0), Sense::hover());
                let art = s.packs[0];
                let pp = ui.painter_at(r.expand(8.0));
                pp.add(kit::shadow_tier(2).as_shape(r, 6.0));
                match self.setup.cards.pics.get(ui.ctx(), &format!("greypack:{art}"), true) {
                    Some(tex) => super::store::paint_image(&pp, &tex, r, egui::Rounding::same(4.0), 1.0, false),
                    None => {
                        self.draw_booster_painted(&pp, r, s);
                        pp.rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(30, 32, 42, 150));
                    }
                }
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    let soon = left < 86_400;
                    let day = if crate::stats::local_day(s.release) == crate::stats::local_day(now) { "TODAY".to_string() } else { release_when(s.release).to_uppercase() };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("NEXT SET · {day} · AT THE GAME'S DAILY RESET")).font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(TEXT(), MUTED(), 0.4)));
                        kit::info(ui, "A new set comes out every Monday at the game's daily reset (12 am CET), when rare grass and Shiny Bingo change too.");
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("{} ({})", s.name, s.year)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                    ui.add_space(4.0);
                    // the countdown: hours, minutes and seconds on the day; days before that
                    let (d, h, m, sec) = (left / 86_400, (left % 86_400) / 3600, (left % 3600) / 60, left % 60);
                    let big = if soon { format!("{h:02} : {m:02} : {sec:02}") } else { format!("{d} days {h} h") };
                    ui.label(RichText::new(big).font(egui::FontId::monospace(34.0)).color(TEXT()));
                    if soon {
                        ui.ctx().request_repaint_after(Duration::from_secs(1));
                    }
                    ui.label(RichText::new(format!("{} cards · {}", cards::of_set(s.id).len(), countdown_words(left))).size(kit::fs(12.5)).color(theme::blend(TEXT(), MUTED(), 0.5)));
                    ui.add_space(10.0);
                    let on = f.notify.iter().any(|x| x == s.id);
                    let (br, _) = ui.allocate_exact_size(vec2(150.0, 32.0), Sense::hover());
                    if pill_button(ui, br, if on { "🔔  I'll tell you" } else { "🔔  Remind me" }, !on, ("drop-notify", s.id))
                        .on_hover_text(if on { "A pop-up in the app when it comes out. Click to turn it off." } else { "A pop-up in the app when it comes out" })
                        .clicked()
                    {
                        out = Some((s.id, !on));
                    }
                });
                // the sets after it
                let later: Vec<&cards::Set> = {
                    let mut v: Vec<&cards::Set> = cards::SETS.iter().filter(|x| x.release > s.release).collect();
                    v.sort_by_key(|x| x.release);
                    v
                };
                if !later.is_empty() && ui.available_width() > 220.0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(200.0);
                            ui.label(RichText::new("AFTER THAT").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(TEXT(), MUTED(), 0.4)));
                            ui.add_space(4.0);
                            for x in later.iter().take(4) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(x.name).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(countdown_words(x.release.saturating_sub(now)).trim_start_matches("Out ").to_string()).size(kit::fs(12.0)).color(theme::blend(TEXT(), MUTED(), 0.5)));
                                    });
                                });
                            }
                            if later.len() > 4 {
                                ui.label(RichText::new(format!("+ {} more, one each Monday", later.len() - 4)).size(kit::fs(12.0)).color(theme::blend(TEXT(), MUTED(), 0.5)));
                            }
                        });
                    });
                }
            });
        });
        // behind it: the action colour, softly
        let rect = resp.response.rect;
        let a = theme::blend(CARD(), kit::primary(), if pl.light { 0.14 } else { 0.30 });
        let b = theme::blend(CARD(), kit::primary(), if pl.light { 0.05 } else { 0.10 });
        ui.painter().set(slot, egui::Shape::Vec(vec![kit::card_shadow().as_shape(rect, 18.0).into(), theme::gradient_rect(rect, [18.0; 4], a, b), egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::primary(), 0.4)))]));
        out
    }

    fn cards_packs(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "cards/packs");
        let f = self.setup.cards.file().clone();
        let mut open: Option<&'static str> = None;
        let here = self.cards_art_here();
        let jobs: Vec<(&'static str, usize, usize)> = {
            use std::sync::atomic::Ordering;
            self.setup.cards.save_all.iter().map(|j| (j.set, j.done.load(Ordering::Relaxed), j.total)).collect()
        };
        let now = crate::config::now_unix();
        let mut get: Option<&'static str> = None;
        let mut notify: Option<(&'static str, bool)> = None;
        let mut auto: Option<(&'static str, bool)> = None;
        // Modern Plus: the next set's release as an event, with a big countdown
        if kit::plus() {
            if let Some(next) = cards::SETS.iter().filter(|s| !s.out_at(now)).min_by_key(|s| s.release) {
                let tour_top = ui.cursor().min.y;
                if let Some(n) = self.cards_next_drop(ui, next, &f, now) {
                    notify = Some(n);
                }
                super::app_tour::mark_since(ui, "packs.next", tour_top);
                ui.add_space(14.0);
            }
        }
        let tour_top = ui.cursor().min.y;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "🎁", kit::C_PINK, if f.packs > 0 { "Pick a booster to open" } else { "Booster packs" }, None);
            ui.add_space(4.0);
            note(
                ui,
                if f.packs > 0 {
                    format!("You have {} pack{}. Most have 11 cards: a rare (holo one time in three), 3 uncommons and 7 commons, some of them Energy in sets that have it. From Legendary Collection on, one card is a reverse holo, and the e-Card sets have 9 cards. A new set comes out every Monday at the game's daily reset.", f.packs, if f.packs == 1 { "" } else { "s" })
                } else {
                    format!("No packs right now. {} more XP for the next one: keep hunting, or finish a mission.", XP_PER_PACK - f.xp)
                },
            );
            ui.add_space(14.0);
            // as many to a row as fit, the ones out first, then the ones coming
            let per = (((ui.available_width() + 30.0) / 200.0).floor() as usize).max(1);
            let mut tiles: Vec<(usize, bool)> = cards::SETS.iter().enumerate().map(|(k, s)| (k, s.out_at(now))).collect();
            tiles.sort_by_key(|t| !t.1);
            let more_soon = cards::sets_coming().is_empty();
            let n_tiles = tiles.len() + more_soon as usize;
            for row_start in (0..n_tiles).step_by(per) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(30.0, 22.0);
                for &(k, _) in tiles.iter().skip(row_start).take(per).filter(|t| t.1) {
                    let s = &cards::SETS[k];
                    ui.vertical(|ui| {
                        let (r, resp) = ui.allocate_exact_size(vec2(170.0, 295.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && f.packs > 0, kit::QUICK);
                        let t = ui.input(|i| i.time) as f32;
                        let wob = if ht > 0.0 { (t * 9.0).sin() * 1.5 * ht } else { 0.0 };
                        let pr = r.translate(vec2(wob, -6.0 * ht));
                        self.draw_booster(ui, pr, s, f.packs > 0);
                        // just out: NEW for a week
                        if s.release > 0 && now < s.release + 7 * 86_400 {
                            let b = Rect::from_center_size(pos2(pr.max.x - 8.0, pr.min.y + 10.0), vec2(52.0, 22.0));
                            ui.painter().rect_filled(b, 11.0, kit::C_PINK);
                            ui.painter().text(b.center(), Align2::CENTER_CENTER, "NEW", crate::fonts::sans_bold_font(11.5), Color32::WHITE);
                        }
                        if ht > 0.0 {
                            ui.ctx().request_repaint();
                        }
                        let have = f.unique(Some(s.id));
                        let n = cards::of_set(s.id).len();
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("{} ({})", s.name, s.year)).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                        ui.label(RichText::new(format!("{n} cards · you have {have}")).size(kit::fs(11.5)).color(MUTED()));
                        ui.add_space(4.0);
                        let (br, _) = ui.allocate_exact_size(vec2(170.0, 32.0), Sense::hover());
                        let can = f.packs > 0;
                        if (pill_button(ui, br, if can { "Open this one" } else { "🔒  No packs" }, can, ("open-set", s.id)).clicked() || resp.clicked()) && can {
                            open = Some(s.id);
                        }
                        // a new set's card art, to download
                        let all = set_art(s.id).len();
                        let got = here.get(k).map_or(0, |h| h.0);
                        if s.release > 0 && got < all {
                            ui.add_space(2.0);
                            match jobs.iter().find(|j| j.0 == s.id) {
                                Some((_, d, tot)) => {
                                    ui.label(RichText::new(format!("Downloading its art… {}%", d * 100 / (*tot).max(1))).size(kit::fs(11.5)).color(MUTED()));
                                }
                                None => {
                                    if ui.link(RichText::new("⬇  Download its card pictures").size(kit::fs(11.5))).on_hover_text("Its card pictures and pack art, kept on this PC. Without them the cards are drawn by the app.").clicked() {
                                        get = Some(s.id);
                                    }
                                }
                            }
                        }
                    });
                }
                // the next sets, coming one a week
                for &(k, _) in tiles.iter().skip(row_start).take(per).filter(|t| !t.1) {
                    let s = &cards::SETS[k];
                    ui.vertical(|ui| {
                        let (r, resp) = ui.allocate_exact_size(vec2(170.0, 295.0), Sense::hover());
                        // its booster, in grey (drawn by the app until the art is here)
                        let t = ui.input(|i| i.time);
                        let art = s.packs[((t / 4.0) as usize + s.id.len()) % s.packs.len()];
                        let pp = ui.painter_at(r.expand(8.0));
                        pp.add(kit::shadow_tier(2).as_shape(r, 6.0));
                        match self.setup.cards.pics.get(ui.ctx(), &format!("greypack:{art}"), true) {
                            Some(tex) => super::store::paint_image(&pp, &tex, r, egui::Rounding::same(4.0), 1.0, false),
                            None => {
                                self.draw_booster_painted(&pp, r, s);
                                pp.rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(30, 32, 42, 175));
                            }
                        }
                        for name in s.packs {
                            let _ = self.setup.cards.pics.get(ui.ctx(), &format!("greypack:{name}"), true);
                        }
                        let p = ui.painter();
                        // the lock and how long to go
                        let c = pos2(r.center().x, r.center().y - 30.0);
                        p.circle_filled(c, 30.0, Color32::from_black_alpha(90));
                        lock_icon(p, c, 22.0, Color32::from_white_alpha(220));
                        let left = s.release.saturating_sub(now);
                        let band = Rect::from_center_size(pos2(r.center().x, r.center().y + 28.0), vec2(r.width() - 24.0, 30.0));
                        p.rect_filled(band, 15.0, Color32::from_black_alpha(140));
                        // (Modern Plus: in words, so 2 minutes never reads like 2 hours)
                        let when = if kit::plus() { countdown_words(left) } else { countdown(left) };
                        p.text(band.center(), Align2::CENTER_CENTER, when, crate::fonts::sans_bold_font(if kit::plus() { 13.0 } else { 14.0 }), GOLD);
                        if left < 3600 {
                            ui.ctx().request_repaint_after(Duration::from_secs(1));
                        }
                        let _ = resp.on_hover_text(format!("{} comes out {}", s.name, release_when(s.release)));
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("{} ({})", s.name, s.year)).font(crate::fonts::sans_bold_font(13.5)).color(MUTED()));
                        ui.label(RichText::new(format!("Out {}", release_when(s.release))).size(kit::fs(11.5)).color(MUTED()));
                        ui.add_space(4.0);
                        let on = f.notify.iter().any(|x| x == s.id);
                        let (br, _) = ui.allocate_exact_size(vec2(170.0, 32.0), Sense::hover());
                        let label = match (on, kit::plus()) {
                            (true, _) => "🔔  I'll tell you",
                            (false, true) => "🔔  Remind me",
                            (false, false) => "🔔  Tell me when it's out",
                        };
                        if pill_button(ui, br, label, on, ("notify-set", s.id))
                            .on_hover_text(if on { "A pop-up in the app when it comes out. Click to turn it off." } else { "A pop-up in the app when it comes out" })
                            .clicked()
                        {
                            notify = Some((s.id, !on));
                        }
                        let mut a = f.auto_get.iter().any(|x| x == s.id);
                        if kit::checkbox(ui, &mut a, "Download its cards then").on_hover_text("When it comes out, its card pictures download by themselves (its booster art is already here)").changed() {
                            auto = Some((s.id, a));
                        }
                    });
                }
                // none left to come: more on the way
                if more_soon && row_start + per >= n_tiles {
                    ui.vertical(|ui| {
                        let (r, resp) = ui.allocate_exact_size(vec2(170.0, 295.0), Sense::hover());
                        let t = ui.input(|i| i.time) as f32;
                        coming_soon_pack(ui.painter(), r, t, resp.hovered());
                        resp.on_hover_text("More card sets are on the way");
                        ui.add_space(6.0);
                        ui.label(RichText::new("More sets").font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                        ui.label(RichText::new("Coming soon").size(kit::fs(11.5)).color(MUTED()));
                    });
                }
            });
            ui.add_space(22.0);
            }
        });
        super::app_tour::mark_since(ui, "packs.boosters", tour_top);
        if let Some(set) = get {
            let ctx = ui.ctx().clone();
            self.cards_download_set(&ctx, set);
        }
        if let Some((set, on)) = notify {
            let file = self.setup.cards.file();
            file.notify.retain(|x| x != set);
            if on {
                file.notify.push(set.to_string());
            }
            self.setup.cards.touch();
        }
        if let Some((set, on)) = auto {
            let file = self.setup.cards.file();
            file.auto_get.retain(|x| x != set);
            if on {
                file.auto_get.push(set.to_string());
            }
            self.setup.cards.touch();
        }
        ui.add_space(16.0);
        // the last packs opened: one small tile each, its cards fanned
        // (best on top), in a grid
        let tour_top = ui.cursor().min.y;
        if !f.pulls.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                head(ui, "🕘", kit::C_SLATE, "Latest pulls", None);
                ui.add_space(8.0);
                let w = ui.available_width();
                let gap = 12.0;
                let cols = ((w + gap) / (300.0 + gap)).floor().clamp(1.0, 4.0) as usize;
                let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                let cw = 48.0;
                let ch = (cw * CARD_ASPECT).round();
                let th = 36.0 + ch + 12.0;
                let pulls: Vec<&cards::Pull> = f.pulls.iter().take(8).collect();
                let rows = pulls.len().div_ceil(cols);
                let (grid, _) = ui.allocate_exact_size(vec2(w, rows as f32 * th + (rows as f32 - 1.0) * gap), Sense::hover());
                let mut look: Option<String> = None;
                for (i, pull) in pulls.iter().enumerate() {
                    let tile = Rect::from_min_size(pos2(grid.min.x + (i % cols) as f32 * (tw + gap), grid.min.y + (i / cols) as f32 * (th + gap)), vec2(tw, th));
                    let p = ui.painter();
                    p.rect_filled(tile, 12.0, kit::sunken());
                    let set = cards::set(&pull.set);
                    if let Some(s) = set {
                        p.circle_filled(pos2(tile.min.x + 16.0, tile.min.y + 18.0), 4.0, Color32::from_rgb(s.col[0], s.col[1], s.col[2]));
                    }
                    p.text(pos2(tile.min.x + 26.0, tile.min.y + 18.0), Align2::LEFT_CENTER, set.map_or("", |s| s.name), crate::fonts::sans_bold_font(12.5), TEXT());
                    p.text(pos2(tile.max.x - 12.0, tile.min.y + 18.0), Align2::RIGHT_CENTER, super::home::ago(pull.when), crate::fonts::sans_font(11.5), MUTED());
                    // commons first, the best last (on top)
                    let mut list: Vec<&Card> = pull.cards.iter().filter_map(|id| cards::by_id(id)).filter(|c| c.rarity != Rarity::Energy).collect();
                    list.sort_by_key(|c| c.rarity);
                    let n = list.len().max(1);
                    let room = tile.width() - 24.0 - cw;
                    let step = if n > 1 { (room / (n as f32 - 1.0)).min(cw + 6.0) } else { 0.0 };
                    let y0 = tile.min.y + 36.0;
                    let rects: Vec<Rect> = (0..list.len()).map(|k| Rect::from_min_size(pos2(tile.min.x + 12.0 + k as f32 * step, y0), vec2(cw, ch))).collect();
                    // which card the pointer is on (the top one where they overlap)
                    let hover = ui.ctx().pointer_hover_pos().and_then(|pp| rects.iter().rposition(|r| r.contains(pp)));
                    for (k, c) in list.iter().enumerate() {
                        let lift = if hover == Some(k) { -6.0 } else { 0.0 };
                        let r = rects[k].translate(vec2(0.0, lift));
                        ui.painter().add(kit::shadow_tier(1).as_shape(r, 4.0));
                        self.draw_card(ui, r, c, false, false);
                    }
                    let resp = ui.interact(tile, ui.id().with(("pull", pull.when, i)), Sense::click());
                    if let Some(k) = hover {
                        let resp = resp.on_hover_text(&list[k].name).on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            look = Some(list[k].id.clone());
                        }
                    }
                }
                if look.is_some() {
                    self.setup.cards.open = look;
                }
            });
        }
        super::app_tour::mark_since(ui, "packs.pulls", tour_top);
        ui.add_space(16.0);
        let tour_top = ui.cursor().min.y;
        self.cards_art_card(ui);
        super::app_tour::mark_since(ui, "packs.art", tour_top);
        if let Some(set) = open {
            self.setup.cards.choose = Some(set);
        }
    }

    pub(super) fn cards_start_opening(&mut self, set: &str, art: Option<&'static str>) {
        let Some(s) = cards::set(set) else { return };
        let mut rng = cards::Rng::seeded_now(self.setup.cards.file().opened as u64);
        let pick = s.packs[rng.below(s.packs.len())];
        let art = art.filter(|a| s.packs.contains(a)).unwrap_or(pick);
        let now = crate::config::now_unix();
        let Some((reveal, new, gains)) = self.setup.cards.file().open(set, &mut rng, today(), now) else { return };
        let _ = self.setup.cards.note_gains(gains);
        // the stack in reveal order: the rare is always the last card
        // (no "move the back 2 to the front" step any more: it put the rare
        // up front in sets where it already sat at the back)
        let order: Vec<usize> = (0..reveal.len()).collect();
        self.setup.cards.opening = Some(Opening {
            set: set.to_string(),
            art,
            reveal,
            new,
            order,
            stage: Stage::Tear,
            tear: 0.0,
            from_right: false,
            moving: None,
            at: 0,
            slide: None,
            since: Instant::now(),
            flipped: None,
            auto: None,
        });
        self.setup.cards.holo_played = false;
        self.setup.cards.out_played = false;
        self.setup.cards.touch();
        self.setup.cards.save_tick(true);
    }

    /// Download every picture not on this PC yet (four at a time).
    /// Download one set's art (only the files not here yet).
    fn cards_download_set(&mut self, ctx: &egui::Context, set: &'static str) {
        use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
        use std::sync::Arc;
        if self.setup.cards.save_all.iter().any(|j| j.set == set) {
            return;
        }
        let todo: Vec<(String, std::path::PathBuf)> = set_art(set).into_iter().filter(|(_, f)| !f.exists()).collect();
        let total = todo.len();
        let job = SaveAll { set, total, done: Arc::new(AtomicUsize::new(0)), failed: Arc::new(AtomicUsize::new(0)), bytes: Arc::new(AtomicU64::new(0)) };
        let list = Arc::new(todo);
        let next = Arc::new(AtomicUsize::new(0));
        for _ in 0..4 {
            let (list, next, done, failed, bytes, c) = (list.clone(), next.clone(), job.done.clone(), job.failed.clone(), job.bytes.clone(), ctx.clone());
            std::thread::spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((url, file)) = list.get(i) else { break };
                let ok = crate::sprites::fetch(url).ok().filter(|b| b.len() > 100 && (b.starts_with(&[0xFF, 0xD8]) || b.starts_with(&[0x89, b'P']))).and_then(|b| {
                    std::fs::create_dir_all(file.parent()?).ok()?;
                    std::fs::write(file, &b).ok()?;
                    Some(b.len() as u64)
                });
                match ok {
                    Some(n) => {
                        bytes.fetch_add(n, Ordering::Relaxed);
                    }
                    None => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                done.fetch_add(1, Ordering::Relaxed);
                c.request_repaint();
            });
        }
        self.setup.cards.save_all.push(job);
        self.setup.cards.saved = None;
    }

    /// Delete one set's art from this PC (its cards go back to drawn ones).
    fn cards_remove_set(&mut self, set: &str) {
        for (_, f) in set_art(set) {
            let _ = std::fs::remove_file(f);
        }
        self.setup.cards.pics.reset();
        self.setup.cards.saved = None;
    }

    /// Each set's art on this PC: (files, bytes). Re-counted every few
    /// seconds (a few hundred file checks).
    fn cards_art_here(&mut self) -> Vec<(usize, u64)> {
        let stale = self.setup.cards.saved.as_ref().map_or(true, |(_, t)| t.elapsed() > Duration::from_secs(4));
        if stale {
            let v = cards::SETS
                .iter()
                .map(|s| {
                    set_art(s.id).iter().filter_map(|(_, f)| std::fs::metadata(f).ok()).fold((0usize, 0u64), |(n, b), m| (n + 1, b + m.len()))
                })
                .collect();
            self.setup.cards.saved = Some((v, Instant::now()));
        }
        self.setup.cards.saved.as_ref().map(|s| s.0.clone()).unwrap_or_default()
    }

    /// Finished downloads: tell the user, and load the new pictures.
    fn cards_jobs_tick(&mut self) {
        use std::sync::atomic::Ordering;
        let mut finished = Vec::new();
        self.setup.cards.save_all.retain(|j| {
            let done = j.done.load(Ordering::Relaxed) >= j.total;
            if done {
                finished.push((j.set, j.failed.load(Ordering::Relaxed)));
            }
            !done
        });
        for (set, failed) in finished {
            let name = cards::set(set).map_or("", |s| s.name);
            self.app_toast(if failed == 0 { format!("{name} card art downloaded") } else { format!("{name} card art downloaded ({failed} couldn't; try again later)") });
            self.setup.cards.pics.reset();
            self.setup.cards.saved = None;
        }
    }

    /// On the Packs page: what art is here, the playmat, and the settings button.
    fn cards_art_card(&mut self, ui: &mut egui::Ui) {
        let here = self.cards_art_here();
        let mut open = false;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "⚙", kit::C_TEAL, "Card art and playmat", None);
            ui.add_space(4.0);
            let parts: Vec<String> = cards::SETS
                .iter()
                .zip(here.iter())
                .filter(|(s, _)| s.out())
                .map(|(s, (n, _))| {
                    let all = set_art(s.id).len();
                    format!("{}: {}", s.name, if *n >= all { "downloaded" } else if *n == 0 { "not downloaded" } else { "partly downloaded" })
                })
                .collect();
            note(ui, format!("{}. Sets without art use drawn cards.", parts.join(" · ")));
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(200.0, 32.0), Sense::hover());
            open = pill_button(ui, r, "⚙  Open settings", true, "cards-settings-open").clicked();
        });
        if open {
            self.open_settings(super::Tab::Cards);
        }
    }

    /// Settings → Card Collector: download or remove each set's art, the XP
    /// bar on the overlay, and the opening playmat's colour. Opens by itself
    /// the first time the app is opened.
    pub(super) fn cards_settings_body(&mut self, ui: &mut egui::Ui) {
        let first = !self.setup.cards.file().art_asked;
        let mat = self.setup.cards.file().mat.clone();
        let mut pick_mat: Option<&'static str> = None;
        if first {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                head(ui, "🃏", kit::C_GOLD, "Welcome to the Card Collector", None);
                ui.add_space(4.0);
                note(ui, "Pick which sets' card art to download. You can change this any time: it's always here, in Settings.");
            });
            ui.add_space(14.0);
        }
        // ── card art
        kit::section(ui, "Card art");
        self.cards_art_list(ui);
        ui.add_space(14.0);
        // ── the XP bar on the overlay
        kit::section(ui, "On the overlay");
        self.cards_bar_settings(ui);
        ui.add_space(18.0);
        // ── playmat
        kit::section(ui, "Playmat");
        let w = ui.available_width();
        let sw = 34.0;
        let prev_w = 170.0;
        let per = ((w - prev_w - 22.0 + 12.0) / (sw + 52.0)).floor().max(1.0) as usize;
        let rows = MATS.len().div_ceil(per);
        let h = (18.0 + rows as f32 * 58.0 + 6.0).max(118.0);
        let (area, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter().clone();
        let y = area.min.y;
        // a little preview on the left, the colours on the right
        let prev = Rect::from_min_size(pos2(area.min.x, y), vec2(prev_w, 110.0));
        let base = &cards::SETS[0];
        playmat(&p, prev, mat_col(&mat, base), true);
        let pc = Rect::from_center_size(prev.center(), vec2(30.0, 30.0 * CARD_ASPECT));
        card_back(&p, pc);
        let sx = prev.max.x + 22.0;
        for (k, (key, name, rgb)) in MATS.iter().enumerate() {
            let cx = sx + (k % per) as f32 * (sw + 52.0) + sw / 2.0 + 16.0;
            let cy = y + 18.0 + (k / per) as f32 * 58.0;
            let c = pos2(cx, cy);
            let resp = ui.interact(Rect::from_center_size(pos2(cx, cy + 10.0), vec2(80.0, 54.0)), ui.id().with(("cards-mat", k)), Sense::click());
            let on = mat == *key;
            if key.is_empty() {
                // half and half: each set's own colour
                let a = Color32::from_rgb(cards::SETS[0].col[0], cards::SETS[0].col[1], cards::SETS[0].col[2]);
                let b = Color32::from_rgb(cards::SETS[1].col[0], cards::SETS[1].col[1], cards::SETS[1].col[2]);
                p.circle_filled(c, sw / 2.0, a);
                let mut m = egui::Mesh::default();
                m.colored_vertex(c, b);
                for i in 0..=16 {
                    let ang = -std::f32::consts::FRAC_PI_4 + std::f32::consts::PI * i as f32 / 16.0;
                    m.colored_vertex(c + vec2(ang.cos(), ang.sin()) * sw / 2.0, b);
                }
                for i in 0..16u32 {
                    m.add_triangle(0, 1 + i, 2 + i);
                }
                p.add(m);
            } else {
                p.circle_filled(c, sw / 2.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            }
            if on || resp.hovered() {
                p.circle_stroke(c, sw / 2.0 + 4.0, Stroke::new(2.0f32, if on { TEXT() } else { MUTED() }));
            }
            p.text(pos2(cx, cy + sw / 2.0 + 12.0), Align2::CENTER_CENTER, *name, crate::fonts::sans_font(11.0), if on { TEXT() } else { MUTED() });
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick_mat = Some(key);
            }
        }
        if let Some(m) = pick_mat {
            self.setup.cards.file().mat = m.to_string();
            self.setup.cards.touch();
        }
    }

    /// Each set's card art on this PC: download, finish or remove it (in
    /// Settings → Card Collector, and in the Library's Card art tab).
    pub(super) fn cards_art_list(&mut self, ui: &mut egui::Ui) {
        use std::sync::atomic::Ordering;
        self.cards_jobs_tick();
        let here = self.cards_art_here();
        let mut get: Option<&'static str> = None;
        let mut remove: Option<&'static str> = None;
        let row_h = 92.0;
        for (k, set) in cards::SETS.iter().enumerate().filter(|(_, s)| s.out()) {
            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());
            let p = ui.painter().clone();
            p.rect_filled(row, 12.0, kit::sunken());
            // the set's booster, small
            let br = Rect::from_min_size(row.min + vec2(12.0, 10.0), vec2(42.0, 72.0));
            match self.setup.cards.pics.get(ui.ctx(), &format!("pack:{}", set.packs[0]), true) {
                Some(t) => super::store::paint_image(&p, &t, br, egui::Rounding::same(3.0), 1.0, false),
                None => self.draw_booster_painted(&p, br, set),
            }
            let tx = br.max.x + 14.0;
            p.text(pos2(tx, row.min.y + 24.0), Align2::LEFT_CENTER, format!("{} ({})", set.name, set.year), crate::fonts::sans_bold_font(14.0), TEXT());
            let files = set_art(set.id).len();
            let (have, bytes) = here.get(k).copied().unwrap_or((0, 0));
            let job = self.setup.cards.save_all.iter().find(|j| j.set == set.id);
            let mb = |b: u64| format!("{:.0} MB", (b as f64 / 1_048_576.0).max(1.0));
            let status = match job {
                Some(j) => format!("Downloading… {} of {}", j.done.load(Ordering::Relaxed), j.total),
                None if have >= files => format!("Downloaded · {} on this PC", mb(bytes)),
                None if have == 0 => format!("{} cards and {} booster arts · not downloaded", cards::of_set(set.id).len(), set.packs.len()),
                None => format!("Partly downloaded ({have} of {files} files) · {}", mb(bytes)),
            };
            p.text(pos2(tx, row.min.y + 46.0), Align2::LEFT_CENTER, status, crate::fonts::sans_font(12.0), MUTED());
            if let Some(j) = job {
                let fr = j.done.load(Ordering::Relaxed) as f32 / j.total.max(1) as f32;
                kit::paint_bar(&p, Rect::from_min_size(pos2(tx, row.min.y + 64.0), vec2((row.max.x - 150.0 - tx).max(40.0), 6.0)), fr);
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
            // buttons on the right
            let bw = 120.0;
            let b1 = Rect::from_min_size(pos2(row.max.x - bw - 14.0, row.center().y - 30.0 + if have > 0 && have < files { 0.0 } else { 14.0 }), vec2(bw, 32.0));
            if job.is_none() {
                if have < files {
                    if pill_button(ui, b1, if have == 0 { "⬇  Download" } else { "⬇  Finish" }, true, ("cards-get", set.id)).clicked() {
                        get = Some(set.id);
                    }
                    if have > 0 {
                        let b2 = b1.translate(vec2(0.0, 38.0));
                        if pill_button(ui, b2, "Remove", false, ("cards-rm", set.id)).clicked() {
                            remove = Some(set.id);
                        }
                    }
                } else if pill_button(ui, b1, "🗑  Remove", false, ("cards-rm", set.id)).clicked() {
                    remove = Some(set.id);
                }
            }
            ui.add_space(10.0);
        }
        if let Some(set) = get {
            let ctx = ui.ctx().clone();
            self.cards_download_set(&ctx, set);
        }
        if let Some(set) = remove {
            self.cards_remove_set(set);
        }
        if get.is_some() || remove.is_some() {
            // (the Library's storage bar measures again)
            self.setup.store.sizes_got = None;
        }
    }

    /// Settings → Card Collector: the XP bar under the overlay, with a preview.
    pub(super) fn cards_bar_settings(&mut self, ui: &mut egui::Ui) {
        let mut bar = self.setup.cards.file().bar.clone();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "▬", kit::C_INDIGO, "XP bar on the overlay", None);
            ui.add_space(4.0);
            note(ui, "A slim bar with your XP and packs, pinned under the shiny overlay while it runs (a third of its width). It moves with the overlay; click it to open the Card Collector.");
            ui.add_space(8.0);
            ui.checkbox(&mut bar.on, RichText::new("Show the XP bar under the overlay").size(kit::fs(13.0)).color(TEXT()));
            ui.add_enabled_ui(bar.on, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Pinned to the").size(kit::fs(13.0)).color(TEXT()));
                    let mut side = bar.right;
                    kit::segmented(ui, &mut side, &[(false, "Left"), (true, "Right")]);
                    bar.right = side;
                });
            });
            ui.add_space(4.0);
            note(ui, "It takes the look of your overlay theme. Retro DS has its own style; the others use this one for now.");
            ui.add_space(8.0);
            // what it looks like, at its real size, in your theme
            let retro = self.cfg.overlay.theme == crate::config::OverlayTheme::Retro;
            let f = self.setup.cards.file().clone();
            let gain = self.setup.cards.gains.back().cloned();
            let (r, _) = ui.allocate_exact_size(vec2(200.0, super::cards_bar::BAR_H), Sense::hover());
            super::cards_bar::paint_bar(ui.painter(), r, &f, gain.as_ref().map(|g| (&g.0, g.1)), self.setup.cards.pack_at, false, None, retro, false, ui.input(|i| i.time) as f32);
            ui.label(RichText::new(if retro { "Preview: Retro DS" } else { "Preview" }).size(kit::fs(11.0)).color(MUTED()));
        });
        if bar != self.setup.cards.file().bar {
            self.setup.cards.file().bar = bar;
            self.setup.cards.touch();
        }
    }

    /// Choosing which pack to open: every art of the set's booster, side
    /// by side over the dimmed page (no panel behind them). Click one to
    /// open it; click outside or press Esc to back out.
    fn cards_choose_pack(&mut self, ctx: &egui::Context) {
        let Some(set_id) = self.setup.cards.choose else { return };
        let Some(set) = cards::set(set_id) else {
            self.setup.cards.choose = None;
            return;
        };
        if self.setup.cards.file().packs == 0 {
            self.setup.cards.choose = None;
            return;
        }
        let screen = ctx.screen_rect();
        let now = ctx.input(|i| i.time);
        let since = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new(("cards-choose-at", set_id)), || now));
        let age = (now - since) as f32;
        let mut picked: Option<&'static str> = None;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::Area::new(egui::Id::new("cards-choose")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            let fade = (age / 0.18).clamp(0.0, 1.0);
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha((225.0 * fade) as u8));
            // the packs: as big as fits, up to a real-pack size
            let n = set.packs.len() as f32;
            let gap = 34.0;
            let pw = ((sr.width() - 120.0 - gap * (n - 1.0)) / n).min((sr.height() - 260.0) / 1.735).clamp(110.0, 230.0);
            let ph = (pw * 1.735).round();
            let row_w = pw * n + gap * (n - 1.0);
            let top = sr.center().y - ph / 2.0 + 16.0;
            let p = ui.painter();
            p.text(pos2(sr.center().x, top - 62.0), Align2::CENTER_CENTER, format!("Choose your {} pack", set.name), crate::fonts::sans_bold_font(24.0), Color32::WHITE.gamma_multiply(fade));
            p.text(pos2(sr.center().x, top - 32.0), Align2::CENTER_CENTER, "Same cards inside, whichever art you pick", crate::fonts::sans_font(13.0), Color32::from_white_alpha((150.0 * fade) as u8));
            let t = ui.input(|i| i.time) as f32;
            let mut over_pack = false;
            for (k, name) in set.packs.iter().enumerate() {
                // each one rises in, a beat after the last
                let rise = ((age - 0.05 * k as f32) / 0.28).clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - rise).powi(3);
                let base = Rect::from_min_size(pos2(sr.center().x - row_w / 2.0 + k as f32 * (pw + gap), top), vec2(pw, ph));
                let resp = ui.interact(base.expand(6.0), ui.id().with(("choose-pack", k)), Sense::click());
                over_pack |= resp.hovered();
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let r = Rect::from_center_size(base.center() + vec2(0.0, 30.0 * (1.0 - ease) - 14.0 * ht), base.size() * (1.0 + 0.05 * ht));
                let p = ui.painter_at(sr);
                // a soft glow under the one you're over
                if ht > 0.0 {
                    let col = Color32::from_rgb(set.col[0], set.col[1], set.col[2]);
                    let glow = egui::epaint::Shadow { offset: vec2(0.0, 0.0), blur: 48.0, spread: 6.0, color: theme::blend(col, Color32::WHITE, 0.2).gamma_multiply(0.55 * ht * ease) };
                    p.add(glow.as_shape(r, 8.0));
                }
                p.add(kit::shadow_tier(3).as_shape(r, 6.0));
                match self.setup.cards.pics.get(ui.ctx(), &format!("pack:{name}"), true) {
                    Some(tex) => super::store::paint_image(&p, &tex, r, egui::Rounding::same(4.0), ease, false),
                    None => self.draw_booster_painted(&p, r, set),
                }
                if ht > 0.0 {
                    shine(&p, r, t * 0.6);
                    p.rect_stroke(r, 4.0, Stroke::new(2.0f32, Color32::from_white_alpha((170.0 * ht) as u8)));
                    ui.ctx().request_repaint();
                }
                if rise < 1.0 {
                    p.rect_filled(r, 4.0, Color32::from_black_alpha((200.0 * (1.0 - ease)) as u8));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    picked = Some(name);
                }
            }
            let p = ui.painter();
            p.text(pos2(sr.center().x, top + ph + 40.0), Align2::CENTER_CENTER, "Click outside or press Esc to go back", crate::fonts::sans_font(12.0), Color32::from_white_alpha((110.0 * fade) as u8));
            if scrim.clicked() && !over_pack {
                close = true;
            }
            if age < 0.6 {
                ui.ctx().request_repaint();
            }
        });
        if let Some(art) = picked {
            ctx.data_mut(|d| d.remove::<f64>(egui::Id::new(("cards-choose-at", set_id))));
            self.setup.cards.choose = None;
            self.cards_start_opening(set_id, Some(art));
        } else if close {
            ctx.data_mut(|d| d.remove::<f64>(egui::Id::new(("cards-choose-at", set_id))));
            self.setup.cards.choose = None;
        }
    }

    /// A booster on the shelf: its art (taking turns between the set's
    /// packs), or a drawn one until the art downloads.
    pub(super) fn draw_booster(&mut self, ui: &egui::Ui, r: Rect, s: &cards::Set, lit: bool) {
        let t = ui.input(|i| i.time);
        let k = ((t / 4.0) as usize + s.id.len()) % s.packs.len();
        let tex = self.setup.cards.pics.get(ui.ctx(), &format!("pack:{}", s.packs[k]), true);
        let p = ui.painter_at(r.expand(8.0));
        p.add(kit::shadow_tier(2).as_shape(r, 6.0));
        match tex {
            Some(t) => super::store::paint_image(&p, &t, r, egui::Rounding::same(4.0), 1.0, false),
            None => self.draw_booster_painted(&p, r, s),
        }
        if lit {
            shine(&p, r, t as f32 * 0.6);
        } else {
            p.rect_filled(r, 4.0, Color32::from_black_alpha(110));
        }
        // other pack arts load in the background
        for name in s.packs {
            let _ = self.setup.cards.pics.get(ui.ctx(), &format!("pack:{name}"), true);
        }
    }

    /// A booster drawn by the app (before the art downloads).
    pub(super) fn draw_booster_painted(&mut self, p: &egui::Painter, r: Rect, s: &cards::Set) {
        let col = Color32::from_rgb(s.col[0], s.col[1], s.col[2]);
        p.add(theme::gradient_rect(r, [6.0; 4], theme::blend(col, Color32::WHITE, 0.15), theme::blend(col, Color32::BLACK, 0.45)));
        for y in [r.min.y + r.height() * 0.06, r.max.y - r.height() * 0.04] {
            let band = Rect::from_center_size(pos2(r.center().x, y), vec2(r.width(), r.height() * 0.06));
            p.rect_filled(band, 2.0, Color32::from_black_alpha(60));
            let mut x = band.min.x + 3.0;
            while x < band.max.x {
                p.line_segment([pos2(x, band.min.y + 2.0), pos2(x, band.max.y - 2.0)], Stroke::new(1.0f32, Color32::from_white_alpha(40)));
                x += 6.0;
            }
        }
        if let Some(sp) = self.species.get(s.mascot as usize - 1).cloned() {
            p.circle_filled(pos2(r.center().x, r.min.y + r.height() * 0.45), r.width() * 0.36, Color32::from_white_alpha(30));
            self.draw_sprite_centered(p, &sp, false, pos2(r.center().x, r.min.y + r.height() * 0.45), vec2(r.width() * 0.8, r.width() * 0.8));
        }
        p.text(pos2(r.center().x, r.max.y - r.height() * 0.2), Align2::CENTER_CENTER, s.name.to_uppercase(), crate::fonts::sans_bold_font(r.width() * 0.1), Color32::WHITE);
    }

    /// The pack being torn: the body, the part of the top still on, and the
    /// torn part curling up (or flying off once it's all torn, `fly` 0 to 1).
    /// A card sound (see `sound::CardSnd`), with the app's sounds on, at
    /// their volume, on their device.
    fn card_sound(&mut self, k: crate::sound::CardSnd) {
        let snd = &self.cfg.sounds;
        if !snd.enabled {
            return;
        }
        let (vol, dev) = (snd.volume, snd.device.clone());
        let ts = &mut self.setup.cards.tear_sound;
        ts.2 = ts.2.wrapping_add(1);
        crate::sound::play_card(k, ts.2, vol, &dev);
    }

    fn draw_pack_tearing(&mut self, ui: &egui::Ui, pr: Rect, op_art: &str, set: &cards::Set, tear: f32, from_right: bool, fly: f32, alpha: f32) {
        const STRIP: f32 = 0.12;
        let tex = self.setup.cards.pics.get(ui.ctx(), &format!("pack:{op_art}"), true).map(|t| t.id());
        let p = ui.painter().clone();
        let sy = pr.min.y + pr.height() * STRIP;
        let tint = Color32::WHITE.gamma_multiply(alpha);
        let body = Rect::from_min_max(pos2(pr.min.x, sy), pr.max);
        p.add(kit::shadow_tier(3).as_shape(body, 4.0));
        // the body
        match tex {
            Some(id) => {
                p.add(quad(Some(id), Rect::from_min_max(pos2(0.0, STRIP), pos2(1.0, 1.0)), [body.left_top(), body.right_top(), body.right_bottom(), body.left_bottom()], tint));
            }
            None => {
                let clip = p.with_clip_rect(body);
                self.draw_booster_painted(&clip, pr, set);
            }
        }
        let col = Color32::from_rgb(set.col[0], set.col[1], set.col[2]);
        let w = pr.width();
        let tw = w * tear.clamp(0.0, 1.0);
        // where the tear has got to, and the two parts of the top
        let (torn_x0, torn_x1, flat_x0, flat_x1, pivot, sign) =
            if from_right { (pr.max.x - tw, pr.max.x, pr.min.x, pr.max.x - tw, pos2(pr.max.x - tw, sy), -1.0) } else { (pr.min.x, pr.min.x + tw, pr.min.x + tw, pr.max.x, pos2(pr.min.x + tw, sy), 1.0) };
        let u = |x: f32| (x - pr.min.x) / w;
        let strip_piece = |x0: f32, x1: f32, f: &dyn Fn(egui::Pos2) -> egui::Pos2| -> egui::Shape {
            let pts = [f(pos2(x0, pr.min.y)), f(pos2(x1, pr.min.y)), f(pos2(x1, sy)), f(pos2(x0, sy))];
            match tex {
                Some(id) => quad(Some(id), Rect::from_min_max(pos2(u(x0), 0.0), pos2(u(x1), STRIP)), pts, tint),
                None => quad(None, Rect::NOTHING, pts, theme::blend(col, Color32::BLACK, 0.3).gamma_multiply(alpha)),
            }
        };
        if flat_x1 - flat_x0 > 0.5 && fly <= 0.0 {
            p.add(strip_piece(flat_x0, flat_x1, &|q| q));
        }
        if tw > 0.5 {
            // curls up from where the tear is
            let a = sign * (0.15 + 0.45 * tear) + sign * fly * 0.8;
            let lift = vec2(sign * -fly * 60.0, -fly * 140.0);
            let rot = move |q: egui::Pos2| {
                let d = q - pivot;
                let (s, c) = (a.sin(), a.cos());
                pivot + vec2(d.x * c - d.y * s, d.x * s + d.y * c) + lift
            };
            let fade = 1.0 - fly;
            if fade > 0.0 {
                let (x0, x1) = if fly > 0.0 { (pr.min.x, pr.max.x) } else { (torn_x0, torn_x1) };
                let shape = strip_piece(x0, x1, &rot);
                p.add(fade_shape(shape, fade));
            }
            // the torn edge: a white ragged line on the pack
            let mut pts = Vec::new();
            let mut x = torn_x0;
            let mut k = 0;
            while x <= torn_x1 {
                pts.push(pos2(x, sy + if k % 2 == 0 { -1.5 } else { 1.5 }));
                x += 5.0;
                k += 1;
            }
            if pts.len() > 1 {
                p.add(egui::Shape::line(pts, Stroke::new(2.0f32, Color32::from_rgb(240, 236, 226).gamma_multiply(alpha))));
            }
        }
    }

    /// Opening a pack, the way you would a real one: tear the top off,
    /// slide the cards out, move the back two to the front, then turn them
    /// over one at a time with the rare last.
    fn cards_opening(&mut self, ui: &mut egui::Ui) {
        let Some(op) = self.setup.cards.opening.as_ref() else { return };
        let set_id = op.set.clone();
        let Some(set) = cards::set(&set_id) else {
            self.setup.cards.opening = None;
            return;
        };
        let (art, stage, tear, from_right, since, at, slide, moving, flipped) = (op.art, op.stage, op.tear, op.from_right, op.since, op.at, op.slide, op.moving, op.flipped);
        let (reveal, new, order) = (op.reveal.clone(), op.new.clone(), op.order.clone());
        let n = reveal.len();
        let full = ui.available_rect_before_wrap();
        let h = (ui.ctx().screen_rect().height() - full.min.y - 30.0).max(560.0);
        let (area, _) = ui.allocate_exact_size(vec2(full.width(), h), Sense::hover());
        let p = ui.painter().clone();
        let mat = self.setup.cards.file().mat.clone();
        playmat(&p, area, mat_col(&mat, set), stage != Stage::Summary);
        ui.ctx().request_repaint();
        let t = since.elapsed().as_secs_f32();
        // the pictures load now, ahead of when they're needed
        for id in &reveal {
            let _ = self.setup.cards.pics.get(ui.ctx(), id, true);
        }
        let left = self.setup.cards.file().packs;
        // the pack's size on the stage, and a card's
        let ph = (area.height() * 0.66).min(470.0);
        let pw = ph * 0.58;
        // the pack right in the middle (a little room under it for the words)
        let pr = Rect::from_center_size(pos2(area.center().x, area.center().y - 24.0), vec2(pw, ph));
        let ch = (area.height() * 0.62).min(470.0);
        let cw = ch / CARD_ASPECT;
        let stack_c = pos2(area.center().x - cw * 0.35, area.center().y - 30.0);
        let mut set_stage: Option<Stage> = None;
        let mut new_tear: Option<(f32, bool)> = None;
        let mut start_move = false;
        let mut next = false;
        let mut flip_now = false;
        let mut skip = false;
        let mut start_auto = false;
        let mut done = false;
        let mut again = false;
        let caption = |p: &egui::Painter, y: f32, a: &str, b: &str| {
            p.text(pos2(area.center().x, y), Align2::CENTER_CENTER, a, crate::fonts::sans_bold_font(17.0), TEXT());
            if !b.is_empty() {
                p.text(pos2(area.center().x, y + 24.0), Align2::CENTER_CENTER, b, crate::fonts::sans_font(13.0), MUTED());
            }
        };
        match stage {
            Stage::Tear => {
                self.draw_pack_tearing(ui, pr, art, set, tear, from_right, 0.0, 1.0);
                // drag along the top to tear it
                let sy = pr.min.y + pr.height() * 0.12;
                let zone = Rect::from_min_max(pos2(pr.min.x - 40.0, pr.min.y - 30.0), pos2(pr.max.x + 40.0, sy + 30.0));
                let resp = ui.interact(zone, ui.id().with("tear-zone"), Sense::click_and_drag());
                let mut fr = from_right;
                if let Some(pp) = ui.ctx().pointer_interact_pos() {
                    if resp.drag_started() && tear <= 0.0 {
                        fr = pp.x > pr.center().x;
                    }
                    if resp.dragged() {
                        let f = if fr { (pr.max.x - pp.x) / pr.width() } else { (pp.x - pr.min.x) / pr.width() };
                        let nt = tear.max(f.clamp(0.0, 1.0));
                        if nt != tear || fr != from_right {
                            new_tear = Some((nt, fr));
                        }
                    }
                }
                if resp.hovered() || resp.dragged() {
                    ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                }
                // a dotted line and scissors show where to tear
                if tear < 1.0 {
                    let head_x = if from_right { pr.max.x - pr.width() * tear } else { pr.min.x + pr.width() * tear };
                    let (a, b) = if from_right { (pr.min.x, head_x) } else { (head_x, pr.max.x) };
                    let pulse = 0.5 + 0.5 * (t * 4.0).sin();
                    p.extend(egui::Shape::dashed_line(&[pos2(a, sy), pos2(b, sy)], Stroke::new(1.5f32, Color32::WHITE.gamma_multiply(0.4 + 0.4 * pulse)), 5.0, 4.0));
                    let sc = if tear > 0.0 { pos2(head_x, sy - 14.0) } else { pos2(pr.min.x - 18.0, sy - 2.0) };
                    p.text(sc, Align2::CENTER_CENTER, "✂", egui::FontId::proportional(20.0), Color32::WHITE);
                }
                caption(&p, pr.max.y + 34.0, if tear > 0.0 { "Keep going…" } else { "Drag along the top to tear it open" }, "Slowly, like a real one. From either side.");
                let lr = Rect::from_min_size(pos2(area.max.x - 170.0, area.min.y + 16.0), vec2(154.0, 30.0));
                if pill_button(ui, lr, "Tear it all at once", false, "tear-all").clicked() {
                    new_tear = Some((1.0, false));
                }
            }
            Stage::Out => {
                // the top flies off, the cards rise out, the pack drops away
                let fly = (t / 0.5).min(1.0);
                let rise = ((t - 0.35) / 0.7).clamp(0.0, 1.0);
                let drop = ((t - 0.85) / 0.5).clamp(0.0, 1.0);
                let ease = |x: f32| 1.0 - (1.0 - x) * (1.0 - x);
                let cards_c = pos2(pr.center().x, pr.center().y - ease(rise) * ph * 0.42 + drop * (stack_c.y - pr.center().y + ph * 0.42));
                let cards_c = pos2(cards_c.x + (stack_c.x - cards_c.x) * ease(drop), cards_c.y);
                let cr = Rect::from_center_size(cards_c, vec2(pw * 0.86, pw * 0.86 * CARD_ASPECT));
                // cards behind the pack's front while still inside
                let inside = p.with_clip_rect(Rect::from_min_max(pos2(area.min.x, area.min.y), pos2(area.max.x, if drop > 0.0 { area.max.y } else { pr.min.y + ph * 0.12 + 2.0 })));
                for k in (0..4).rev() {
                    card_back(&inside, cr.translate(vec2(k as f32 * 2.0, k as f32 * 2.0)));
                }
                let pr2 = pr.translate(vec2(0.0, ease(drop) * area.height() * 0.6));
                self.draw_pack_tearing(ui, pr2, art, set, 1.0, from_right, fly, 1.0 - drop);
                if rise > 0.0 && !self.setup.cards.out_played {
                    self.setup.cards.out_played = true;
                    self.play(crate::sound::Sfx::Milestone);
                }
                if t > 1.45 {
                    set_stage = Some(Stage::Back2);
                }
            }
            Stage::Back2 => {
                // the stack in your hand, face down
                let cr = Rect::from_center_size(stack_c, vec2(cw, ch));
                let m = moving.map(|m| (m.elapsed().as_secs_f32() / 0.8).min(1.0));
                for k in (1..5).rev() {
                    card_back(&p, cr.translate(vec2(k as f32 * 2.5, k as f32 * 2.5)));
                }
                match m {
                    // the back two coming round to the front
                    Some(f) if f < 1.0 => {
                        card_back(&p, cr);
                        for j in 0..2 {
                            let g = ((f - j as f32 * 0.15) / 0.85).clamp(0.0, 1.0);
                            let arc = (g * std::f32::consts::PI).sin();
                            let c = cr.translate(vec2(-arc * cw * 0.8, -arc * 30.0 + (1.0 - g) * 10.0));
                            p.add(kit::shadow_tier(2).as_shape(c, 8.0));
                            card_back(&p, c);
                        }
                    }
                    _ => card_back(&p, cr),
                }
                let ready = m.is_some_and(|f| f >= 1.0);
                let tx = cr.max.x + 60.0;
                let tr = Rect::from_min_max(pos2(tx, cr.min.y + 20.0), pos2(area.max.x - 30.0, cr.max.y));
                let tp = p.with_clip_rect(tr);
                if !ready {
                    tp.text(pos2(tx, cr.min.y + 40.0), Align2::LEFT_CENTER, "The cards are in your hand.", crate::fonts::sans_bold_font(18.0), TEXT());
                    let g = p.layout("The 2 Energy sit at the back with the rare just in front of them. Move the back 2 to the front and the rare comes last, the way most people open a pack.".into(), crate::fonts::sans_font(13.5), MUTED(), (tr.width()).min(360.0));
                    tp.galley(pos2(tx, cr.min.y + 62.0), g, MUTED());
                    let b1 = Rect::from_min_size(pos2(tx, cr.min.y + 150.0), vec2(250.0, 38.0));
                    let b2 = Rect::from_min_size(pos2(tx, cr.min.y + 198.0), vec2(250.0, 34.0));
                    if moving.is_none() {
                        if pill_button(ui, b1, "↻  Move the back 2 to the front", true, "back2").clicked() {
                            start_move = true;
                        }
                        if pill_button(ui, b2, "Leave them as they are", false, "back2-skip").clicked() {
                            self.setup.cards.opening.as_mut().expect("open").moving = Some(Instant::now() - Duration::from_secs(5));
                            // as they are: no change to the order
                        }
                    }
                } else {
                    tp.text(pos2(tx, cr.min.y + 40.0), Align2::LEFT_CENTER, "Turn them over", crate::fonts::sans_bold_font(18.0), TEXT());
                    tp.text(pos2(tx, cr.min.y + 66.0), Align2::LEFT_CENTER, "Click the cards to start.", crate::fonts::sans_font(13.5), MUTED());
                    if ui.interact(cr, ui.id().with("flip-stack"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        set_stage = Some(Stage::Reveal);
                    }
                    let pulse = 0.5 + 0.5 * (t * 4.0).sin();
                    p.rect_stroke(cr.expand(4.0), 10.0, Stroke::new(2.0f32, GOLD.gamma_multiply(0.3 + 0.5 * pulse)));
                }
            }
            Stage::Reveal => {
                let cr = Rect::from_center_size(stack_c, vec2(cw, ch));
                // what's still in your hand, edges showing behind the front card
                let rest = n.saturating_sub(at + 1).min(4);
                // (face down: every card in the deck shows its back)
                for k in (1..=rest).rev() {
                    let e = cr.translate(vec2(k as f32 * 2.5, k as f32 * 2.5));
                    card_back(&p, e);
                    p.rect_stroke(e, e.width() * 0.045, Stroke::new(1.0f32, Color32::from_black_alpha(70)));
                }
                // the front card (the first one turns over as you start)
                if at < n {
                    let ri = order[at];
                    let c = cards::by_id(&reveal[ri]).expect("card");
                    // flip to reveal: face down until clicked, then a springy
                    // 3D turn (it swings a little past and settles back)
                    let ft = flipped.map_or(0.0, |f| f.elapsed().as_secs_f32());
                    let turning = flipped.is_some() && ft < FLIP_SECS;
                    let flip = if flipped.is_some() { (ft / FLIP_SECS * 2.2).min(1.0) } else { 0.0 };
                    let fr = cr;
                    let rare = c.rarity >= Rarity::Rare;
                    let holo = c.rarity == Rarity::Holo;
                    if rare && flip >= 0.5 {
                        for k in 0..12 {
                            let a = k as f32 / 12.0 * std::f32::consts::TAU + t * 0.6;
                            let d = vec2(a.cos(), a.sin());
                            p.line_segment([cr.center() + d * cw * 0.3, cr.center() + d * cw * (0.95 + 0.1 * (t * 3.0 + k as f32).sin())], Stroke::new(6.0f32, GOLD.gamma_multiply(0.22)));
                        }
                    }
                    if holo && flip >= 0.5 {
                        if !self.setup.cards.holo_played {
                            self.setup.cards.holo_played = true;
                            self.play(crate::sound::Sfx::Achievement);
                        }
                        for k in 0..10 {
                            let ph2 = (t * 0.5 + k as f32 * 0.37).fract();
                            let x = cr.center().x + ((k as f32 * 1.7).sin()) * cw * 0.75;
                            let y = cr.center().y + ch * 0.5 - ph2 * ch * 1.1;
                            theme::sparkle(&p, pos2(x, y), 4.0 + 3.0 * (1.0 - ph2), GOLD.gamma_multiply(1.0 - ph2));
                        }
                    }
                    if turning {
                        let pic = self.setup.cards.pics.get(ui.ctx(), &c.id, true).or_else(|| self.setup.cards.pics.get(ui.ctx(), &c.id, false));
                        flip_turn(&p, fr, pic.as_ref(), ft / FLIP_SECS);
                    } else if flipped.is_none() {
                        p.add(kit::shadow_tier(3).as_shape(fr, 10.0));
                        card_back(&p, fr);
                        // a gentle nudge: click me
                        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
                        p.rect_stroke(fr.expand(4.0), 10.0, Stroke::new(2.0f32, GOLD.gamma_multiply(0.25 + 0.45 * pulse)));
                    } else {
                        p.add(kit::shadow_tier(3).as_shape(fr, 10.0));
                        self.draw_card(ui, fr, c, true, true);
                        if new[ri] {
                            let b = Rect::from_center_size(pos2(fr.min.x + 18.0, fr.min.y - 4.0), vec2(56.0, 24.0));
                            p.rect_filled(b, 12.0, kit::C_PINK);
                            p.text(b.center(), Align2::CENTER_CENTER, "NEW!", crate::fonts::sans_bold_font(12.0), Color32::WHITE);
                        }
                    }
                    let last = at + 1 == n;
                    if flipped.is_none() {
                        caption(&p, cr.max.y + 28.0, "Click to turn it over", &format!("Card {} of {n}", at + 1));
                    } else if !turning {
                        caption(&p, cr.max.y + 28.0, &format!("{}  {}", c.rarity.symbol(), c.name), &format!("Card {} of {n} · {}", at + 1, if last { "click to see them all" } else { "click for the next" }));
                    }
                }
                // the cards already seen, in a little fan on the right
                let pile_x = cr.max.x + 70.0;
                let sw = 74.0;
                let seen: Vec<usize> = order[..at.min(n)].to_vec();
                for (k, ri) in seen.iter().enumerate() {
                    let sliding = slide.is_some_and(|s| s.0 == *ri);
                    if sliding {
                        continue;
                    }
                    let r = Rect::from_min_size(pos2(pile_x + k as f32 * 22.0, cr.min.y + 30.0 + (k % 2) as f32 * 6.0), vec2(sw, sw * CARD_ASPECT));
                    if r.max.x > area.max.x - 10.0 {
                        continue;
                    }
                    let c = cards::by_id(&reveal[*ri]).expect("card");
                    p.add(kit::shadow_tier(1).as_shape(r, 4.0));
                    self.draw_card(ui, r, c, false, false);
                }
                // the one just put down, gliding to the pile
                if let Some((ri, t0)) = slide {
                    let g = (t0.elapsed().as_secs_f32() / 0.3).min(1.0);
                    let k = seen.iter().position(|x| *x == ri).unwrap_or(0);
                    let to = Rect::from_min_size(pos2(pile_x + k as f32 * 22.0, cr.min.y + 30.0 + (k % 2) as f32 * 6.0), vec2(sw, sw * CARD_ASPECT));
                    let lerp = |a: f32, b: f32| a + (b - a) * (1.0 - (1.0 - g) * (1.0 - g));
                    let r = Rect::from_min_max(pos2(lerp(cr.min.x, to.min.x), lerp(cr.min.y, to.min.y)), pos2(lerp(cr.max.x, to.max.x), lerp(cr.max.y, to.max.y)));
                    let c = cards::by_id(&reveal[ri]).expect("card");
                    self.draw_card(ui, r, c, false, false);
                    if g >= 1.0 {
                        self.setup.cards.opening.as_mut().expect("open").slide = None;
                    }
                }
                // a click turns the front card over; once it's face up, the next click puts it down
                if ui.interact(area, ui.id().with("next-card"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    match flipped {
                        None => flip_now = true,
                        Some(f) if f.elapsed().as_secs_f32() > FLIP_SECS * 0.6 => next = true,
                        _ => {}
                    }
                }
                // "Show them all": it clicks through for you, a little quicker
                // (each card turns over, shows a moment, and goes on the pile;
                // the last one, the rare, gets a longer look); once it's going,
                // the same button skips to the end
                let auto = self.setup.cards.opening.as_ref().and_then(|o| o.auto);
                if let Some(a0) = auto {
                    match flipped {
                        None if a0.elapsed().as_secs_f32() > 0.2 => flip_now = true,
                        Some(f) => {
                            let last = at + 1 >= order.len();
                            let dwell = if last { FLIP_SECS + 0.9 } else { FLIP_SECS * 0.6 + 0.25 };
                            if f.elapsed().as_secs_f32() > dwell {
                                next = true;
                            }
                        }
                        _ => {}
                    }
                    ui.ctx().request_repaint();
                }
                let sr = Rect::from_min_size(pos2(area.max.x - 150.0, area.min.y + 16.0), vec2(134.0, 32.0));
                let label = if auto.is_some() { "Skip to the end  ›" } else { "Show them all  ›" };
                if pill_button(ui, sr, label, false, "cards-skip").clicked() {
                    if auto.is_some() {
                        skip = true;
                    } else {
                        start_auto = true;
                    }
                }
            }
            Stage::Summary => {
                // the whole block (title, two rows of cards, buttons) sits
                // right in the middle of the mat
                let cols = 6usize;
                let gap = 14.0;
                const HEAD: f32 = 74.0;
                const BTN: f32 = 40.0;
                const BTN_GAP: f32 = 28.0;
                let room_w = (area.width() - 120.0).min(1240.0);
                let room_h = area.height() - 100.0 - HEAD - BTN - BTN_GAP - gap;
                let sw = ((room_w - gap * (cols as f32 - 1.0)) / cols as f32).min(room_h / 2.0 / CARD_ASPECT).floor();
                let shh = (sw * CARD_ASPECT).floor();
                let block_h = HEAD + 2.0 * shh + gap + BTN_GAP + BTN;
                let top = (area.center().y - block_h / 2.0).round();
                p.text(pos2(area.center().x, top + 14.0), Align2::CENTER_CENTER, format!("Your {} pack", set.name), crate::fonts::sans_bold_font(22.0), Color32::WHITE);
                let n_new = new.iter().filter(|x| **x).count();
                p.text(pos2(area.center().x, top + 42.0), Align2::CENTER_CENTER, if n_new > 0 { format!("{n_new} new for your binder") } else { "No new ones this time".to_string() }, crate::fonts::sans_font(13.5), Color32::from_white_alpha(170));
                let grid_top = top + HEAD;
                let mut look: Option<String> = None;
                for (k, ri) in order.iter().enumerate() {
                    let (row, col_i) = (k / cols, k % cols);
                    let row_n = if row == 0 { cols.min(n) } else { n - cols };
                    let row_w = sw * row_n as f32 + gap * (row_n as f32 - 1.0);
                    let x0 = area.center().x - row_w / 2.0;
                    let r = Rect::from_min_size(pos2((x0 + col_i as f32 * (sw + gap)).round(), grid_top + row as f32 * (shh + gap)), vec2(sw, shh));
                    let appear = ((t - k as f32 * 0.05) / 0.25).clamp(0.0, 1.0);
                    let r = r.translate(vec2(0.0, 20.0 * (1.0 - appear)));
                    let c = cards::by_id(&reveal[*ri]).expect("card");
                    p.add(kit::shadow_tier(2).as_shape(r, 8.0));
                    self.draw_card(ui, r, c, false, true);
                    if new[*ri] {
                        let b = Rect::from_center_size(pos2(r.center().x, r.min.y - 2.0), vec2(40.0, 17.0));
                        p.rect_filled(b, 8.0, kit::C_PINK);
                        p.text(b.center(), Align2::CENTER_CENTER, "NEW", crate::fonts::sans_bold_font(9.5), Color32::WHITE);
                    }
                    if ui.interact(r, ui.id().with(("summary", k)), Sense::click()).on_hover_text(&c.name).clicked() {
                        look = Some(c.id.clone());
                    }
                }
                let by = grid_top + 2.0 * shh + gap + BTN_GAP;
                let bw = 210.0;
                let b1 = Rect::from_min_size(pos2(area.center().x - bw - 8.0, by), vec2(bw, BTN));
                let b2 = Rect::from_min_size(pos2(area.center().x + 8.0, by), vec2(bw, BTN));
                if pill_button(ui, b1, &if left > 0 { format!("🎁  Open another ({left})") } else { "No packs left".to_string() }, left > 0, "cards-again").clicked() && left > 0 {
                    again = true;
                }
                if pill_button(ui, b2, "📚  To my binder", false, "cards-done").clicked() {
                    done = true;
                }
                if look.is_some() {
                    self.setup.cards.open = look;
                }
            }
        }
        // apply what happened
        // the tear sound: a bit of ripping for each stretch the tear moves,
        // so a slow tear crackles slowly and a quick one rips; a last snap
        // as the top comes off
        if let Some((nt, _)) = new_tear {
            let snd = &self.cfg.sounds;
            let on = snd.enabled;
            let (vol, dev) = (snd.volume, snd.device.clone());
            let ts = &mut self.setup.cards.tear_sound;
            if tear <= 0.0 {
                *ts = (0.0, None, ts.2);
            }
            if on && nt >= 0.97 {
                ts.2 = ts.2.wrapping_add(1);
                crate::sound::play_tear(ts.2, true, vol, &dev);
                ts.0 = 1.0;
            } else if on && nt - ts.0 >= 0.04 && ts.1.map_or(true, |t| t.elapsed() > Duration::from_millis(45)) {
                // quicker tearing plays a little louder
                let speed = ts.1.map_or(1.0, |t| ((nt - ts.0) / t.elapsed().as_secs_f32().max(0.01)).clamp(0.2, 3.0));
                ts.2 = ts.2.wrapping_add(1);
                crate::sound::play_tear(ts.2, false, vol * (0.55 + 0.15 * speed).min(1.0), &dev);
                ts.0 = nt;
                ts.1 = Some(Instant::now());
            }
        }
        // the cards' own sounds: turning the first over, each one flicked
        // onto the pile, the back two sliding round, and a riffle as they're
        // all laid out
        {
            use crate::sound::CardSnd;
            let snd = if set_stage == Some(Stage::Reveal) {
                Some(CardSnd::Flip)
            } else if flip_now {
                Some(CardSnd::Flip)
            } else if start_move {
                Some(CardSnd::Slide)
            } else if next && at + 1 < order.len() {
                Some(CardSnd::Flick)
            } else if (next || skip) && stage == Stage::Reveal {
                Some(CardSnd::Riffle(n.min(12) as u8))
            } else {
                None
            };
            if let Some(k) = snd {
                self.card_sound(k);
            }
        }
        if let Some(op) = self.setup.cards.opening.as_mut() {
            if let Some((nt, fr)) = new_tear {
                op.tear = nt;
                op.from_right = fr;
                if nt >= 0.97 {
                    op.tear = 1.0;
                    op.stage = Stage::Out;
                    op.since = Instant::now();
                }
            }
            if start_move {
                op.moving = Some(Instant::now());
                // the back two go to the front
                let k = op.order.len().saturating_sub(2);
                let back: Vec<usize> = op.order.split_off(k);
                let mut v = back;
                v.extend(op.order.drain(..));
                op.order = v;
            }
            if let Some(s) = set_stage {
                op.stage = s;
                op.since = Instant::now();
                op.at = 0;
                // the stack in your hand is ready to turn over straight away
                if s == Stage::Back2 && op.moving.is_none() {
                    op.moving = Some(Instant::now() - Duration::from_secs(5));
                }
                // the click on the stack turns the first card over
                op.flipped = (s == Stage::Reveal).then(Instant::now);
            }
            if start_auto {
                op.auto = Some(Instant::now());
            }
            // (each step it takes by itself starts the clock for the next)
            if (flip_now || next) && op.auto.is_some() {
                op.auto = Some(Instant::now());
            }
            if flip_now {
                op.flipped = Some(Instant::now());
            }
            if next {
                if op.at + 1 < op.order.len() {
                    op.slide = Some((op.order[op.at], Instant::now()));
                    op.at += 1;
                    op.flipped = None;
                } else {
                    op.stage = Stage::Summary;
                    op.since = Instant::now();
                }
            }
            if skip {
                op.stage = Stage::Summary;
                op.since = Instant::now();
            }
        }
        if done {
            self.setup.cards.opening = None;
            self.setup.cards.view = View::Binder;
            self.setup.cards.show = Show::All;
            self.setup.cards.sort = Sort::Newest;
        }
        if again {
            self.setup.cards.opening = None;
            self.setup.cards.choose = cards::set(&set_id).map(|s| s.id);
        }
        self.cards_detail(ui.ctx());
    }


    // ───────────────────────────── missions ─────────────────────────────

    fn cards_missions(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "cards/missions");
        let f = self.setup.cards.file().clone();
        let now = crate::config::now_unix();
        let secs_left_day = 86_400 - ((now as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400)) as u64;
        let days_left_week = 7 - (today() - f.week.when).clamp(0, 6);
        let tour_top = ui.cursor().min.y;
        for (title, sub, t, col) in [
            ("Daily missions", format!("New ones in {}", crate::stats::fmt_duration(secs_left_day)), &f.day, kit::C_CYAN),
            ("Weekly missions", format!("New ones in {days_left_week} day{}", if days_left_week == 1 { "" } else { "s" }), &f.week, kit::C_VIOLET),
        ] {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    head(ui, "🎯", col, title, None);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(sub).size(kit::fs(12.0)).color(MUTED()));
                    });
                });
                ui.add_space(8.0);
                for m in &t.missions {
                    mission_row(ui, m, t.progress(m), col);
                }
            });
            ui.add_space(14.0);
        }
        super::app_tour::mark_since(ui, "missions.list", tour_top);
        let tour_top = ui.cursor().min.y;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "⭐", GOLD, "How XP works", None);
            ui.add_space(6.0);
            // what was earned lately, and why (the sum of each part)
            let recent = self.setup.cards.file().recent.clone();
            if !recent.is_empty() {
                ui.label(RichText::new("LATEST XP").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                for (when, xp, why) in recent.iter().take(6) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(super::home::ago(*when)).size(kit::fs(12.0)).color(MUTED()));
                        ui.label(RichText::new(why).size(kit::fs(12.5)).color(TEXT()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("+{xp}")).font(crate::fonts::sans_bold_font(12.5)).color(GOLD));
                        });
                    });
                }
                ui.add_space(8.0);
                ui.label(RichText::new("WHAT EARNS XP").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            }
            for (what, xp) in [
                ("Each wild Pokémon the game logs", format!("+{}", cards::XP_ENCOUNTER)),
                ("…on rare grass", format!("+{} more", cards::XP_RARE_GRASS)),
                ("Each species you meet for the first time today (once a day each)", format!("+{}", cards::XP_NEW_SPECIES)),
                ("A shiny", format!("+{}", cards::XP_SHINY)),
                ("A mission", "its XP".to_string()),
                ("A full bar", format!("{XP_PER_PACK} XP = 1 pack")),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(what).size(kit::fs(12.5)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(xp).font(crate::fonts::sans_bold_font(12.5)).color(GOLD));
                    });
                });
            }
            ui.add_space(4.0);
            note(ui, "Wild Pokémon in the game's log earn XP whether the overlay is running or not: the log is checked every few seconds, and catches up on what you played while the app was closed. Screen-counted ones earn XP while the overlay counts; ones you add by hand don't.");
        });
        ui.add_space(14.0);
        super::app_tour::mark_since(ui, "missions.xp", tour_top);
        // (the XP bar's options are in Settings → Card Collector)
        let (r, _) = ui.allocate_exact_size(vec2(240.0, 32.0), Sense::hover());
        if pill_button(ui, r, "⚙  XP bar on the overlay", false, "cards-bar-settings").on_hover_text("Its options are in Settings → Card Collector").clicked() {
            self.open_settings(super::Tab::Cards);
        }
    }
}

/// One mission: icon, what to do, a bar, and its XP.
pub(super) fn mission_row(ui: &mut egui::Ui, m: &cards::Mission, have: u64, col: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let p = ui.painter();
    let done = m.done;
    let chip = Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(32.0, 32.0));
    kit::icon_chip(p, chip, if done { "✔" } else { m.icon() }, if done { theme::GREEN } else { col }, done);
    let x = chip.max.x + 12.0;
    let right = r.max.x - 80.0;
    p.text(pos2(x, r.center().y - 10.0), Align2::LEFT_CENTER, m.text(), crate::fonts::sans_bold_font(13.0), if done { MUTED() } else { TEXT() });
    let bar = Rect::from_min_max(pos2(x, r.center().y + 7.0), pos2(right - 70.0, r.center().y + 13.0));
    let f = have as f32 / m.goal.max(1) as f32;
    p.rect_filled(bar, 3.0, theme::blend(CARD(), TEXT(), 0.08));
    p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * f.min(1.0), bar.height())), 3.0, if done { theme::GREEN } else { col });
    p.text(pos2(right - 60.0, bar.center().y), Align2::LEFT_CENTER, format!("{}/{}", have.min(m.goal), m.goal), crate::fonts::sans_font(11.5), MUTED());
    let xr = Rect::from_min_size(pos2(r.max.x - 72.0, r.center().y - 13.0), vec2(72.0, 26.0));
    p.rect_filled(xr, 13.0, if done { theme::blend(CARD(), theme::GREEN, 0.2) } else { theme::blend(CARD(), GOLD, 0.18) });
    p.text(xr.center(), Align2::CENTER_CENTER, if done { "Done".to_string() } else { format!("+{} XP", m.xp) }, crate::fonts::sans_bold_font(11.5), if done { theme::GREEN } else { GOLD });
}

/// The XP bar: gold, with a shine; brighter just after a pack.
pub(super) fn xp_bar(p: &egui::Painter, r: Rect, frac: f32, t: f32, pack_at: Option<Instant>) {
    let round = r.height() / 2.0;
    p.rect_filled(r, round, Color32::from_black_alpha(90));
    p.rect_stroke(r, round, Stroke::new(1.0f32, Color32::from_white_alpha(30)));
    let w = (r.width() * frac.clamp(0.0, 1.0)).max(if frac > 0.0 { r.height() } else { 0.0 });
    if w > 0.0 {
        let fr = Rect::from_min_size(r.min, vec2(w, r.height()));
        p.add(theme::gradient_rect(fr, [round; 4], Color32::from_rgb(255, 214, 90), Color32::from_rgb(255, 150, 60)));
        // a soft glint running along it every few seconds
        let k = (t * 0.25).fract();
        if k < 0.35 {
            let gx = fr.min.x + k / 0.35 * (fr.width() + 60.0) - 30.0;
            for (half, a) in [(26.0, 14u8), (14.0, 22), (6.0, 30)] {
                let g = Rect::from_min_max(pos2((gx - half).max(fr.min.x), fr.min.y), pos2((gx + half).min(fr.max.x), fr.max.y));
                if g.width() > 0.0 {
                    p.rect_filled(g, round, Color32::from_white_alpha(a));
                }
            }
        }
    }
    if let Some(at) = pack_at {
        let e = at.elapsed().as_secs_f32();
        if e < 2.0 {
            let a = (1.0 - e / 2.0) * (0.5 + 0.5 * (e * 12.0).sin());
            p.rect_stroke(r.expand(2.0), round + 2.0, Stroke::new(2.0f32, GOLD.gamma_multiply(a)));
        }
    }
}

/// How XP works, for tooltips.
pub(super) fn xp_rules() -> String {
    format!(
        "+{} XP for each wild Pokémon the game logs (+{} more on rare grass)\n+{} XP for a species you haven't met today\n+{} XP for a shiny\nMissions give their own XP\n{} XP = one booster pack",
        cards::XP_ENCOUNTER,
        cards::XP_RARE_GRASS,
        cards::XP_NEW_SPECIES,
        cards::XP_SHINY,
        XP_PER_PACK
    )
}

/// The back of a card (the picture in `assets/card_back.png`).
pub(super) fn card_back(p: &egui::Painter, r: Rect) {
    let tex = card_back_tex(p.ctx());
    let round = r.width() * 0.045;
    match tex {
        Some(t) => super::store::paint_image(p, &t, r, egui::Rounding::same(round), 1.0, false),
        None => {
            p.rect_filled(r, round, Color32::from_rgb(28, 38, 92));
            p.rect_stroke(r, round, Stroke::new(2.0f32, Color32::from_rgb(54, 86, 196)));
        }
    }
}

// ── the top panel's rows (fixed heights, so the three columns line up)

/// Heading: an icon tile, the title, and a muted note on the right.
fn hero_head(ui: &egui::Ui, r: Rect, icon: &str, col: Color32, title: &str, meta: &str) {
    let p = ui.painter();
    let tile = Rect::from_min_size(r.min, vec2(28.0, 28.0));
    p.rect_filled(tile, 8.0, col.gamma_multiply(0.18));
    p.text(tile.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(14.0), col);
    p.text(pos2(tile.max.x + 10.0, tile.center().y), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(14.0), TEXT());
    if !meta.is_empty() {
        p.text(pos2(r.max.x, tile.center().y), Align2::RIGHT_CENTER, meta, crate::fonts::sans_font(12.0), MUTED());
    }
}

/// The big number and what it counts.
fn hero_big(ui: &egui::Ui, r: Rect, n: &str, unit: &str) {
    let p = ui.painter();
    let y = r.min.y + 70.0;
    let g = p.layout_no_wrap(n.to_string(), crate::fonts::sans_bold_font(28.0), TEXT());
    let nw = g.size().x;
    p.galley(pos2(r.min.x, y - g.size().y), g, TEXT());
    p.text(pos2(r.min.x + nw + 8.0, y - 5.0), Align2::LEFT_BOTTOM, unit, crate::fonts::sans_font(13.0), MUTED());
}

/// The bar or button row, `h` tall, centred on the same line in every column.
fn hero_row(r: Rect, h: f32) -> Rect {
    Rect::from_center_size(pos2(r.center().x, r.min.y + 94.0), vec2(r.width(), h))
}

/// The footer line, left.
fn hero_foot(ui: &egui::Ui, r: Rect, text: &str, room: f32) {
    let p = ui.painter();
    let text = super::charts::clip(ui, text, &crate::fonts::sans_font(12.0), r.width() * room);
    p.text(pos2(r.min.x, r.max.y - 8.0), Align2::LEFT_CENTER, text, crate::fonts::sans_font(12.0), MUTED());
}

/// A link on the footer line, right. True when clicked.
fn hero_link(ui: &egui::Ui, r: Rect, text: &str, id: &str) -> bool {
    let font = crate::fonts::sans_bold_font(12.0);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, kit::primary());
    let lr = Rect::from_min_size(pos2(r.max.x - g.size().x, r.max.y - 8.0 - g.size().y / 2.0), g.size());
    let resp = ui.interact(lr.expand(3.0), ui.id().with(id), Sense::click());
    let col = if resp.hovered() { TEXT() } else { kit::primary() };
    ui.painter().galley_with_override_text_color(lr.min, g, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

// ───────────────────────────── holo foil ─────────────────────────────

/// Where the picture window sits on a Wizards-era card, as fractions of the
/// card (left, top, right, bottom), inside its gold frame. Base Set to Team
/// Rocket share the layout (measured on the cards' own pictures).
const ART_BOX: [f32; 4] = [0.108, 0.118, 0.898, 0.517];
/// A Stage 1 / Stage 2 card's evolution badge (the gold starburst with the
/// Pokémon it evolves from), which sits over the picture's top-left corner:
/// its centre as fractions of the card, and its radius as a fraction of the
/// card's width. The foil never goes over it.
const BADGE: [f32; 3] = [0.139, 0.101, 0.100];
/// The e-Card frame (Expedition, Aquapolis, Skyridge): its picture window,
/// the rounded corner at the window's top left (radius as a fraction of the
/// card's width), and a Stage card's evolution badge, which bulges into
/// that corner. Measured on the cards' own pictures.
const ART_BOX_E: [f32; 4] = [0.102, 0.122, 0.967, 0.487];
const CORNER_E: f32 = 0.105;
const BADGE_E: [f32; 3] = [0.100, 0.085, 0.110];
/// The most a holo card tips towards the pointer, in degrees.
const FOIL_TILT_DEG: f32 = 9.0;
/// How many starbursts are in a holo's picture window.
const FOIL_STARS: usize = 56;
/// How strong the rainbow sheen in the picture window is (0..255, on the
/// brightest foil; darker parts of the picture get less).
const FOIL_RAINBOW: f32 = 60.0;
/// How bright the band of rainbow light that sweeps across a holo is (0..255).
const FOIL_SWEEP: f32 = 105.0;

/// A holo card with its picture: it tips away from the pointer like you're
/// pressing on it, a soft glare follows the pointer, and the foil in the
/// picture window is the "starlight" pattern the Base Set, Jungle and Fossil
/// holos used: starbursts of different sizes that light up as the light
/// moves over them, with a faint rainbow sheen. With the pointer elsewhere it
/// drifts gently on its own. True while the pointer is over it.
#[allow(clippy::too_many_arguments)]
fn foil_card(ui: &egui::Ui, p: &egui::Painter, r: Rect, round: f32, card_id: &str, evolves: bool, ecard: bool, t: &egui::TextureHandle) -> bool {
    use std::f32::consts::{PI, TAU};
    let ctx = ui.ctx();
    let time = ui.input(|i| i.time) as f32;
    let id = egui::Id::new(("card-foil", card_id));
    let hover = ctx.pointer_hover_pos().filter(|q| r.contains(*q));
    // Appearance → Animations off: no tipping, drifting or twinkling; the
    // glare still sits where the pointer is
    let motion = kit::motion();
    // where the light is, -1..1 across the card
    let (tx, ty) = match hover {
        Some(q) => (((q.x - r.center().x) / (r.width() / 2.0)).clamp(-1.0, 1.0), ((q.y - r.center().y) / (r.height() / 2.0)).clamp(-1.0, 1.0)),
        // big cards (the close-up, a pack's holo) drift on their own; small
        // ones settle flat so they hand back to the plain picture smoothly
        None if motion && r.width() > 240.0 => ((time * 0.7).sin() * 0.35, (time * 0.53).cos() * 0.25),
        None => (0.0, 0.0),
    };
    let speed = if !motion { 0.0 } else if hover.is_some() { 0.12 } else { 0.5 };
    let lx = ctx.animate_value_with_time(id.with("x"), tx, speed);
    let ly = ctx.animate_value_with_time(id.with("y"), ty, speed);
    let lit = ctx.animate_value_with_time(id.with("lit"), if hover.is_some() { 1.0 } else { 0.55 }, if motion { 0.25 } else { 0.0 });

    // flat card space -> tipped card on screen (a little perspective)
    let (c0, half) = (r.center(), r.size() / 2.0);
    let max = if motion { FOIL_TILT_DEG.to_radians() } else { 0.0 };
    let (sy, cy) = (-lx * max).sin_cos();
    let (sx, cx) = (-ly * max).sin_cos();
    let d = r.height() * 3.0;
    let proj = |q: egui::Pos2| -> egui::Pos2 {
        let (x, y) = (q.x - c0.x, q.y - c0.y);
        let (x1, z1) = (x * cy, x * sy);
        let (y2, z2) = (y * cx - z1 * sx, y * sx + z1 * cx);
        let f = d / (d - z2);
        pos2(c0.x + x1 * f, c0.y + y2 * f)
    };
    // the light's spot on the card, in flat card space
    let g = pos2(c0.x + lx * half.x, c0.y + ly * half.y);

    // ── the card: a grid that keeps the rounded corners ──
    let rr = round.min(r.width() / 2.0);
    let mut ys: Vec<f32> = Vec::new();
    for k in 0..=6 {
        ys.push(r.min.y + rr * (1.0 - (k as f32 / 6.0 * PI / 2.0).cos()));
    }
    let mid = 12;
    for k in 1..mid {
        ys.push(r.min.y + rr + (r.height() - 2.0 * rr) * k as f32 / mid as f32);
    }
    for k in (0..=6).rev() {
        ys.push(r.max.y - rr * (1.0 - (k as f32 / 6.0 * PI / 2.0).cos()));
    }
    const COLS: usize = 10;
    let mut pts: Vec<egui::Pos2> = Vec::with_capacity(ys.len() * (COLS + 1));
    for y in &ys {
        let dd = (r.min.y + rr - y).max(y - (r.max.y - rr)).clamp(0.0, rr);
        let inset = rr - (rr * rr - dd * dd).max(0.0).sqrt();
        let (lo, hi) = (r.min.x + inset, r.max.x - inset);
        for k in 0..=COLS {
            pts.push(pos2(lo + (hi - lo) * k as f32 / COLS as f32, *y));
        }
    }
    let grid = |m: &mut egui::Mesh| {
        for row in 1..ys.len() as u32 {
            let (a, b) = ((row - 1) * (COLS as u32 + 1), row * (COLS as u32 + 1));
            for k in 0..COLS as u32 {
                m.add_triangle(a + k, a + k + 1, b + k);
                m.add_triangle(a + k + 1, b + k + 1, b + k);
            }
        }
    };
    // the picture, cropped the same way `store::paint_image` does
    let ts = t.size_vec2();
    let (ra, ia) = (r.width() / r.height(), ts.x / ts.y);
    let uv = if ia > ra {
        let w = ra / ia;
        Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
    } else {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, ia / ra))
    };
    // where a spot on the card is in its picture
    let at = |q: egui::Pos2| uv.min + vec2((q.x - r.min.x) / r.width() * uv.width(), (q.y - r.min.y) / r.height() * uv.height());
    // Light that blends with the card's own foil: drawn with the card's
    // picture as the texture, so it's the picture times the light's colour,
    // added on. Bright foil catches it, the dark parts of the art barely do,
    // so the sheen reads as the foil shining, not a layer over the top.
    let lit_vertex = |m: &mut egui::Mesh, q: egui::Pos2, col: Color32| m.vertices.push(egui::epaint::Vertex { pos: proj(q), uv: at(q), color: col });
    let mut card = egui::Mesh::with_texture(t.id());
    for q in &pts {
        let f = vec2((q.x - r.min.x) / r.width(), (q.y - r.min.y) / r.height());
        card.vertices.push(egui::epaint::Vertex { pos: proj(*q), uv: uv.min + vec2(f.x * uv.width(), f.y * uv.height()), color: Color32::WHITE });
    }
    grid(&mut card);
    p.add(card);

    // ── the starlight foil, in the picture window only ──
    let ab = if ecard { ART_BOX_E } else { ART_BOX };
    let art = Rect::from_min_max(pos2(r.min.x + r.width() * ab[0], r.min.y + r.height() * ab[1]), pos2(r.min.x + r.width() * ab[2], r.min.y + r.height() * ab[3]));
    let reach = r.height() * 0.8;
    let near = |q: egui::Pos2| (1.0 - q.distance(g) / reach).clamp(0.0, 1.0);
    // a faint rainbow sheen in diagonal bands that slide as the card tips
    const SN: usize = 10;
    let mut sheen = egui::Mesh::with_texture(t.id());
    for j in 0..=SN {
        for i in 0..=SN {
            let (u, v) = (i as f32 / SN as f32, j as f32 / SN as f32);
            let q = pos2(art.min.x + art.width() * u, art.min.y + art.height() * v);
            let band = 0.5 + 0.5 * ((u * 1.3 + v) * 7.0 + (lx + ly) * 4.0).sin();
            let k = FOIL_RAINBOW * lit * (0.30 + 0.70 * band) * (0.45 + 0.55 * near(q));
            let drift = if motion { time * 0.02 } else { 0.0 };
            lit_vertex(&mut sheen, q, add_light(vivid(u * 0.55 + v * 0.35 + lx * 0.3 + ly * 0.2 + drift), k / 255.0));
        }
    }
    for j in 1..=SN as u32 {
        let (a, b) = ((j - 1) * (SN as u32 + 1), j * (SN as u32 + 1));
        for k in 0..SN as u32 {
            sheen.add_triangle(a + k, a + k + 1, b + k);
            sheen.add_triangle(a + k + 1, b + k + 1, b + k);
        }
    }
    p.add(sheen);
    // the holo sweep: a bright band of rainbow light across the picture,
    // sliding over as the card tips (and drifting on its own when it's not
    // being pointed at), the way light runs across a real holo
    const WN: usize = 18;
    let centre_s = 0.5 + lx * 0.6 + ly * 0.3;
    let width_s = 0.24;
    let mut sweep = egui::Mesh::with_texture(t.id());
    for j in 0..=WN {
        for i in 0..=WN {
            let (u, v) = (i as f32 / WN as f32, j as f32 / WN as f32);
            let q = pos2(art.min.x + art.width() * u, art.min.y + art.height() * v);
            // how far along the diagonal, against the band's middle
            let s_d = (u * 0.62 + v * 0.38 - centre_s) / width_s;
            let k = if s_d.abs() >= 1.0 { 0.0 } else { 0.5 + 0.5 * (s_d * PI).cos() };
            // the colours run through the band like a prism
            let col = vivid(0.55 + s_d * 0.45 + (lx - ly) * 0.15);
            lit_vertex(&mut sweep, q, add_light(col, FOIL_SWEEP * lit * k / 255.0));
        }
    }
    for j in 1..=WN as u32 {
        let (a, b) = ((j - 1) * (WN as u32 + 1), j * (WN as u32 + 1));
        for k in 0..WN as u32 {
            sweep.add_triangle(a + k, a + k + 1, b + k);
            sweep.add_triangle(a + k + 1, b + k + 1, b + k);
        }
    }
    p.add(sweep);
    // the starbursts: the same pattern on the same card every time
    let mut seed: u64 = 0xcbf2_9ce4_8422_2325;
    for b in card_id.bytes() {
        seed ^= b as u64;
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut stars = egui::Mesh::default();
    for _ in 0..FOIL_STARS {
        let (u, v) = (0.04 + 0.92 * rnd(), 0.05 + 0.90 * rnd());
        let size = r.width() * (0.010 + 0.034 * rnd().powi(3));
        let (hue, phase) = (rnd(), rnd() * TAU);
        let q = pos2(art.min.x + art.width() * u, art.min.y + art.height() * v);
        let twinkle = if motion { 0.6 + 0.4 * (time * 1.7 + phase).sin() } else { 0.8 };
        let b = lit * (0.25 + 0.75 * near(q)) * twinkle;
        if b < 0.03 {
            continue;
        }
        let col = pastel(hue + lx * 0.25 + ly * 0.15);
        let bright = [col[0] * 0.65 + 89.0, col[1] * 0.65 + 89.0, col[2] * 0.65 + 89.0];
        // a soft glow behind it
        let base = stars.vertices.len() as u32;
        stars.colored_vertex(proj(q), add_light(col, 55.0 * b / 255.0));
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            stars.colored_vertex(proj(q + vec2(a.cos(), a.sin()) * size * 0.55), Color32::TRANSPARENT);
        }
        for k in 0..10u32 {
            stars.add_triangle(base, base + 1 + k, base + 1 + (k + 1) % 10);
        }
        // the burst: four long rays, four short ones, a bright middle
        let base = stars.vertices.len() as u32;
        stars.colored_vertex(proj(q), add_light(bright, 1.7 * b));
        for k in 0..16 {
            let a = k as f32 * PI / 8.0;
            let (len, col_k) = if k % 4 == 0 {
                (size, Color32::TRANSPARENT)
            } else if k % 2 == 0 {
                (size * 0.45, Color32::TRANSPARENT)
            } else {
                (size * 0.10, add_light(bright, 1.1 * b))
            };
            stars.colored_vertex(proj(q + vec2(a.cos(), a.sin()) * len), col_k);
        }
        for k in 0..16u32 {
            stars.add_triangle(base, base + 1 + k, base + 1 + (k + 1) % 16);
        }
    }
    p.add(stars);
    // what isn't foil gets its own picture back over the top: a Stage card's
    // evolution badge, and the e-Card frame's rounded corner
    let patch = |pts: &[egui::Pos2]| {
        let mut m = egui::Mesh::with_texture(t.id());
        for q in pts {
            m.vertices.push(egui::epaint::Vertex { pos: proj(*q), uv: at(*q), color: Color32::WHITE });
        }
        for k in 1..pts.len().saturating_sub(1) as u32 {
            m.add_triangle(0, k, k + 1);
        }
        p.add(m);
    };
    let circle = |c: egui::Pos2, rad: f32| -> Vec<egui::Pos2> {
        let mut v = vec![c];
        for k in 0..=40 {
            let a = k as f32 / 40.0 * TAU;
            v.push(c + vec2(a.cos(), a.sin()) * rad);
        }
        v
    };
    if ecard {
        // the corner outside the window's rounded top left
        let rad = r.width() * CORNER_E;
        let c0 = art.min;
        let mut v = vec![c0];
        for k in 0..=16 {
            let a = (180.0 + 90.0 * k as f32 / 16.0).to_radians();
            v.push(pos2(c0.x + rad, c0.y + rad) + vec2(a.cos(), a.sin()) * rad);
        }
        v.push(c0);
        patch(&v);
        if evolves {
            patch(&circle(pos2(r.min.x + r.width() * BADGE_E[0], r.min.y + r.height() * BADGE_E[1]), r.width() * BADGE_E[2]));
        }
    } else if evolves {
        patch(&circle(pos2(r.min.x + r.width() * BADGE[0], r.min.y + r.height() * BADGE[1]), r.width() * BADGE[2]));
    }

    // ── the glare: soft light where the pointer is, over the whole card ──
    let mut glare = egui::Mesh::with_texture(t.id());
    for q in &pts {
        let k = (1.0 - q.distance(g) / (r.height() * 0.85)).clamp(0.0, 1.0);
        let tint = vivid((q.x - r.min.x) / r.width() * 0.6 + (q.y - r.min.y) / r.height() * 0.4 + lx * 0.4 + ly * 0.3);
        let mixed = [tint[0] * 0.35 + 255.0 * 0.65, tint[1] * 0.35 + 255.0 * 0.65, tint[2] * 0.35 + 255.0 * 0.65];
        lit_vertex(&mut glare, *q, add_light(mixed, 0.42 * lit * k * k));
    }
    grid(&mut glare);
    p.add(glare);
    hover.is_some()
}

/// A rainbow colour at `h` (0..1, wraps), softened towards white, 0..255.
fn pastel(h: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let [r, g, b] = match h as u32 {
        0 => [1.0, x, 0.0],
        1 => [x, 1.0, 0.0],
        2 => [0.0, 1.0, x],
        3 => [0.0, x, 1.0],
        4 => [x, 0.0, 1.0],
        _ => [1.0, 0.0, x],
    };
    [(r * 0.65 + 0.35) * 255.0, (g * 0.65 + 0.35) * 255.0, (b * 0.65 + 0.35) * 255.0]
}

/// A strong rainbow colour at `h` (0..1, wraps), 0..255.
fn vivid(h: f32) -> [f32; 3] {
    let p = pastel(h);
    // undo most of `pastel`'s softening
    [((p[0] / 255.0 - 0.35) / 0.65 * 0.85 + 0.15) * 255.0, ((p[1] / 255.0 - 0.35) / 0.65 * 0.85 + 0.15) * 255.0, ((p[2] / 255.0 - 0.35) / 0.65 * 0.85 + 0.15) * 255.0]
}

/// Light that adds to what's under it (alpha 0 in egui's premultiplied
/// colours adds instead of covering): `rgb` 0..255 scaled by `k`.
fn add_light(rgb: [f32; 3], k: f32) -> Color32 {
    let c = |v: f32| (v * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(c(rgb[0]), c(rgb[1]), c(rgb[2]), 0)
}

/// A rainbow shine sweeping across (holo cards, foil packs).
fn shine(p: &egui::Painter, r: Rect, t: f32) {
    let x = r.min.x + ((t * 0.25).fract() * 1.6 - 0.3) * r.width();
    let w = r.width() * 0.35;
    let cols = [Color32::from_rgba_unmultiplied(255, 120, 200, 0), Color32::from_rgba_unmultiplied(255, 255, 255, 70), Color32::from_rgba_unmultiplied(120, 220, 255, 0)];
    let mix = |a: Color32, b: Color32, f: f32| {
        let (a, b) = (a.to_array(), b.to_array());
        let l = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * f).round() as u8;
        Color32::from_rgba_premultiplied(l(0), l(1), l(2), l(3))
    };
    // the band's colour at `px` when its middle is at `cx`
    let col_at = |px: f32, cx: f32| {
        let d = ((px - cx) / w).clamp(-1.0, 1.0);
        if d < 0.0 { mix(cols[0], cols[1], d + 1.0) } else { mix(cols[1], cols[2], d) }
    };
    // a slanted band, built row by row so it stays inside the card's rounded corners
    let skew = r.height() * 0.4;
    let rr = r.width() * 0.045;
    const ROWS: u32 = 28;
    let mut m = egui::Mesh::default();
    for i in 0..=ROWS {
        let y = r.min.y + r.height() * i as f32 / ROWS as f32;
        let d = (r.min.y + rr - y).max(y - (r.max.y - rr)).max(0.0).min(rr);
        let inset = rr - (rr * rr - d * d).max(0.0).sqrt();
        let (lo, hi) = (r.min.x + inset, r.max.x - inset);
        let cx = x - skew * (y - r.min.y) / r.height();
        for px in [cx - w, cx, cx + w] {
            let px = px.clamp(lo, hi);
            m.colored_vertex(pos2(px, y), col_at(px, cx));
        }
        if i > 0 {
            let a = (i - 1) * 3;
            let b = i * 3;
            for k in 0..2 {
                m.add_triangle(a + k, a + k + 1, b + k);
                m.add_triangle(a + k + 1, b + k + 1, b + k);
            }
        }
    }
    p.with_clip_rect(r.intersect(p.clip_rect())).add(m);
}

/// The card groups the Type filter offers.
const GROUPS: [&str; 9] = ["Grass", "Fire", "Water", "Lightning", "Psychic", "Fighting", "Colorless", "Trainer", "Energy"];

/// How wide a drop-down pill is for its key and value.
fn drop_width(ui: &egui::Ui, key: &str, val: &str) -> f32 {
    let p = ui.painter();
    let k = p.layout_no_wrap(key.to_string(), crate::fonts::sans_font(12.0), MUTED()).size().x;
    let v = p.layout_no_wrap(val.to_string(), crate::fonts::sans_bold_font(12.5), TEXT()).size().x;
    (14.0 + k + 6.0 + v + 10.0 + 22.0).ceil()
}

/// A drop-down pill: "Key  Value ⌄". Tinted in `col` when it's narrowing
/// the list. Opens its menu when clicked.
fn drop_pill(ui: &egui::Ui, r: Rect, key: &str, val: &str, col: Color32, on: bool, id: impl std::hash::Hash) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(id), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    let rr = r.height() / 2.0;
    let fill = if on { theme::blend(kit::sunken(), col, 0.22 + 0.08 * ht) } else { theme::blend(kit::sunken(), TEXT(), 0.04 * ht) };
    let edge = if on { col } else { theme::blend(LINE(), TEXT(), 0.25 * ht) };
    p.rect_filled(r, rr, fill);
    p.rect_stroke(r, rr, Stroke::new(1.0f32, edge));
    let mut x = r.min.x + 14.0;
    if on {
        p.circle_filled(pos2(x + 3.0, r.center().y), 3.5, col);
        x += 12.0;
    }
    let kg = p.layout_no_wrap(key.to_string(), crate::fonts::sans_font(12.0), MUTED());
    let kw = kg.size().x;
    p.galley(pos2(x, r.center().y - kg.size().y / 2.0), kg, MUTED());
    x += kw + 6.0;
    let vmax = r.max.x - 26.0 - x;
    let v = super::charts::clip(ui, val, &crate::fonts::sans_bold_font(12.5), vmax);
    p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, v, crate::fonts::sans_bold_font(12.5), if on { TEXT() } else { theme::blend(MUTED(), TEXT(), 0.6) });
    // a small chevron
    let c = pos2(r.max.x - 16.0, r.center().y + 1.0);
    let ink = Stroke::new(1.5f32, theme::blend(MUTED(), TEXT(), ht));
    p.line_segment([c + vec2(-4.0, -2.5), c], ink);
    p.line_segment([c, c + vec2(4.0, -2.5)], ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The menu under a drop-down pill: rows of (label, colour dot, count,
/// chosen). Returns the row picked.
fn drop_menu(ui: &egui::Ui, anchor: &egui::Response, popup: egui::Id, items: &[(String, Option<Color32>, String, bool)]) -> Option<usize> {
    if anchor.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let mut pick = None;
    let w = anchor.rect.width().max(210.0);
    egui::popup::popup_below_widget(ui, popup, anchor, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ui.set_min_width(w);
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, (label, dot, count, chosen)) in items.iter().enumerate() {
            let (row, rr) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
            let p = ui.painter();
            if rr.hovered() || *chosen {
                p.rect_filled(row, 8.0, if *chosen { kit::CARD_HOVER() } else { kit::CARD_HOVER().gamma_multiply(0.7) });
            }
            let mut x = row.min.x + 10.0;
            if let Some(c) = dot {
                p.circle_filled(pos2(x + 4.0, row.center().y), 4.5, *c);
                x += 16.0;
            }
            p.text(pos2(x, row.center().y), Align2::LEFT_CENTER, label, if *chosen { crate::fonts::sans_bold_font(13.0) } else { crate::fonts::sans_font(13.0) }, TEXT());
            let mut rx = row.max.x - 10.0;
            if *chosen {
                // a drawn tick
                let c = pos2(rx - 5.0, row.center().y);
                let st = Stroke::new(1.8f32, kit::primary());
                p.line_segment([c + vec2(-5.0, 0.0), c + vec2(-1.5, 3.5)], st);
                p.line_segment([c + vec2(-1.5, 3.5), c + vec2(5.0, -4.0)], st);
                rx -= 20.0;
            }
            if !count.is_empty() {
                p.text(pos2(rx, row.center().y), Align2::RIGHT_CENTER, count, crate::fonts::sans_font(12.0), MUTED());
            }
            if rr.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some(i);
                ui.memory_mut(|m| m.close_popup());
            }
        }
    });
    pick
}

/// The opening screen's playmat: soft felt in the set's colour, lit in the
/// middle and darker to the edges, with faint play zones printed on it and
/// stitching round the rim.
fn playmat(p: &egui::Painter, area: Rect, col: Color32, zones: bool) {
    let round = 18.0;
    let felt = theme::blend(Color32::from_rgb(14, 20, 34), col, 0.16);
    p.rect_filled(area, round, felt);
    // light in the middle, darker at the rim: one smooth fan (no banding)
    let rim = rounded_pts(area, round, 6);
    let mut m = egui::Mesh::default();
    let c = area.center();
    m.colored_vertex(pos2(c.x, c.y - area.height() * 0.06), theme::blend(felt, col, 0.32).gamma_multiply(1.0));
    let dark = Color32::from_black_alpha(150);
    for q in &rim {
        m.colored_vertex(*q, dark);
    }
    let n = rim.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(m);
    let clip = p.with_clip_rect(area.shrink(1.0).intersect(p.clip_rect()));
    // felt grain: fixed specks (the same every frame)
    let hash = |i: u32| {
        let mut x = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        (x & 0xFFFF) as f32 / 65535.0
    };
    let specks = ((area.width() * area.height()) / 1800.0) as u32;
    for i in 0..specks.min(900) {
        let at = pos2(area.min.x + hash(i * 3) * area.width(), area.min.y + hash(i * 3 + 1) * area.height());
        let a = (4.0 + hash(i * 3 + 2) * 10.0) as u8;
        clip.circle_filled(at, 0.9, if i % 2 == 0 { Color32::from_white_alpha(a) } else { Color32::from_black_alpha(a * 2) });
    }
    // the printed zones, very faint (left off under the pack's summary)
    if !zones {
        let inner = rounded_pts(area.shrink((area.height() * 0.01).clamp(4.0, 9.0)), round - 6.0, 6);
        let mut closed = inner.clone();
        closed.push(inner[0]);
        clip.extend(egui::Shape::dashed_line(&closed, Stroke::new(1.2f32, Color32::from_white_alpha(22)), 6.0, 5.0));
        return;
    }
    let ink = Stroke::new(1.5f32, Color32::from_white_alpha(16));
    let ch = (area.height() * 0.2).min(150.0);
    let m = (area.height() * 0.04).clamp(6.0, 36.0);
    let cw = ch / CARD_ASPECT;
    // active spot in the middle, with a ring round it
    clip.circle_stroke(c, ch * 0.95, Stroke::new(1.5f32, Color32::from_white_alpha(12)));
    clip.circle_stroke(c, ch * 1.02, Stroke::new(1.0f32, Color32::from_white_alpha(8)));
    // bench along the bottom
    let by = area.max.y - m * 0.7 - ch * 0.8;
    for k in 0..5 {
        let x = c.x + (k as f32 - 2.0) * (cw * 0.8 + ch * 0.12);
        clip.rect_stroke(Rect::from_center_size(pos2(x, by + ch * 0.4), vec2(cw * 0.8, ch * 0.8)), ch * 0.06, ink);
    }
    // deck and discard on the right, prizes on the left
    let dx = area.max.x - m - cw * 0.4;
    clip.rect_stroke(Rect::from_center_size(pos2(dx, c.y - ch * 0.5), vec2(cw * 0.8, ch * 0.8)), ch * 0.06, ink);
    clip.rect_stroke(Rect::from_center_size(pos2(dx, c.y + ch * 0.45), vec2(cw * 0.8, ch * 0.8)), ch * 0.06, ink);
    for k in 0..6 {
        let x = area.min.x + m + cw * 0.3 + (k % 2) as f32 * (cw * 0.6 + ch * 0.07);
        let y = c.y - ch * 0.75 + (k / 2) as f32 * (ch * 0.6 + ch * 0.07);
        clip.rect_stroke(Rect::from_center_size(pos2(x, y), vec2(cw * 0.6, ch * 0.6)), ch * 0.045, Stroke::new(1.2f32, Color32::from_white_alpha(12)));
    }
    // stitched rim
    let inner = rounded_pts(area.shrink((m * 0.25).max(4.0)), round - 6.0, 6);
    let mut closed = inner.clone();
    closed.push(inner[0]);
    clip.extend(egui::Shape::dashed_line(&closed, Stroke::new(1.2f32, Color32::from_white_alpha(22)), 6.0, 5.0));
}

/// The outline of a rounded rectangle, clockwise from the top left.
fn rounded_pts(r: Rect, round: f32, steps: usize) -> Vec<egui::Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let rr = round.min(r.width().min(r.height()) / 2.0);
    let corners = [(pos2(r.min.x + rr, r.min.y + rr), PI), (pos2(r.max.x - rr, r.min.y + rr), -FRAC_PI_2), (pos2(r.max.x - rr, r.max.y - rr), 0.0), (pos2(r.min.x + rr, r.max.y - rr), FRAC_PI_2)];
    let mut out = Vec::new();
    for (c, a0) in corners {
        for i in 0..=steps {
            let a = a0 + FRAC_PI_2 * i as f32 / steps as f32;
            out.push(c + vec2(a.cos(), a.sin()) * rr);
        }
    }
    // a few points along the long edges too, so the fan's shading stays smooth
    let mut dense = Vec::new();
    for i in 0..out.len() {
        let (a, b) = (out[i], out[(i + 1) % out.len()]);
        let n = ((b - a).length() / 60.0).ceil().max(1.0) as usize;
        for k in 0..n {
            dense.push(a + (b - a) * (k as f32 / n as f32));
        }
    }
    dense
}

/// A padlock, `size` tall, centred on `c`.
fn lock_icon(p: &egui::Painter, c: egui::Pos2, size: f32, col: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, size * 0.18), vec2(size * 0.8, size * 0.6));
    // the shackle: a half ring over the body
    let r = size * 0.26;
    let top = pos2(c.x, body.min.y);
    let pts: Vec<egui::Pos2> = (0..=16).map(|i| {
        let a = std::f32::consts::PI * (i as f32 / 16.0);
        pos2(top.x - r * a.cos(), top.y - r * a.sin())
    }).collect();
    let mut shackle = vec![pos2(top.x - r, top.y + 1.0)];
    shackle.extend(pts);
    shackle.push(pos2(top.x + r, top.y + 1.0));
    p.add(egui::Shape::line(shackle, Stroke::new(size * 0.12, col)));
    p.rect_filled(body, size * 0.12, col);
    p.circle_filled(body.center() - vec2(0.0, size * 0.04), size * 0.07, Color32::from_black_alpha(160));
}

/// "Mon 23 Nov, 6:00 pm" (local time).
fn release_when(unix: u64) -> String {
    let day = crate::stats::local_day(unix);
    format!("{} {}, {}", crate::stats::WEEKDAYS[crate::stats::weekday(day)], crate::stats::fmt_day_short(day), crate::stats::fmt_clock(unix))
}

/// Modern Plus: how long until it's out, in words: "Out in 45 s",
/// "Out in 3 min", "Out in 5 h 20 min", "Out in 7 days".
fn countdown_words(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    if secs < 60 {
        format!("Out in {secs} s")
    } else if secs < 3600 {
        format!("Out in {} min", m.max(1))
    } else if d == 0 {
        if m > 0 { format!("Out in {h} h {m} min") } else { format!("Out in {h} h") }
    } else if d == 1 {
        "Out tomorrow".to_string()
    } else {
        format!("Out in {d} days")
    }
}

/// How long until it's out: "6d 18h", "5h 12m", "12:05".
fn countdown(secs: u64) -> String {
    let (d, h, m, s) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60, secs % 60);
    if d > 0 {
        format!("IN {d}d {h}h")
    } else if h > 0 {
        format!("IN {h}h {m}m")
    } else {
        format!("IN {m}:{s:02}")
    }
}

/// A booster that isn't out yet: a dark foil wrapper inside a dashed
/// outline, with a big "?".
fn coming_soon_pack(p: &egui::Painter, r: Rect, _t: f32, hovered: bool) {
    let lift = if hovered { -4.0 } else { 0.0 };
    let r = r.translate(vec2(0.0, lift));
    p.add(kit::shadow_tier(if hovered { 3 } else { 2 }).as_shape(r, 6.0));
    p.add(theme::gradient_rect(r, [6.0; 4], Color32::from_rgb(46, 50, 68), Color32::from_rgb(28, 30, 42)));
    // crimped ends, like the real ones
    for y in [r.min.y + r.height() * 0.06, r.max.y - r.height() * 0.04] {
        let band = Rect::from_center_size(pos2(r.center().x, y), vec2(r.width(), r.height() * 0.06));
        p.rect_filled(band, 2.0, Color32::from_black_alpha(70));
        let mut x = band.min.x + 3.0;
        while x < band.max.x - 2.0 {
            p.line_segment([pos2(x, band.min.y + 2.0), pos2(x, band.max.y - 2.0)], Stroke::new(1.0f32, Color32::from_white_alpha(22)));
            x += 4.0;
        }
    }
    // the "?" in a ring
    let c = pos2(r.center().x, r.center().y - 14.0);
    p.circle_filled(c, 42.0, Color32::from_white_alpha(8));
    p.circle_stroke(c, 42.0, Stroke::new(2.0f32, Color32::from_white_alpha(if hovered { 70 } else { 40 })));
    p.text(c, Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(48.0), Color32::from_white_alpha(if hovered { 200 } else { 140 }));
    p.text(pos2(r.center().x, r.max.y - r.height() * 0.2), Align2::CENTER_CENTER, "COMING SOON", crate::fonts::sans_bold_font(13.0), GOLD.gamma_multiply(if hovered { 1.0 } else { 0.8 }));
    // a dashed outline round it all
    let o = r.expand(5.0);
    let pts = [o.left_top(), o.right_top(), o.right_bottom(), o.left_bottom(), o.left_top()];
    p.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.2f32, Color32::from_white_alpha(40)), 6.0, 5.0));
}

/// A card type's colour.
pub(super) fn type_col(t: &str) -> Color32 {
    match t {
        "Grass" => Color32::from_rgb(92, 178, 72),
        "Fire" => Color32::from_rgb(232, 96, 52),
        "Water" => Color32::from_rgb(72, 140, 230),
        "Lightning" => Color32::from_rgb(240, 200, 40),
        "Psychic" => Color32::from_rgb(170, 90, 200),
        "Fighting" => Color32::from_rgb(196, 110, 60),
        "Colorless" => Color32::from_rgb(170, 170, 160),
        "Trainer" => Color32::from_rgb(120, 140, 160),
        "Energy" => Color32::from_rgb(110, 190, 190),
        _ => Color32::from_rgb(150, 150, 150),
    }
}

pub(super) fn rarity_col(r: Rarity) -> Color32 {
    match r {
        Rarity::Energy => Color32::from_rgb(140, 170, 180),
        Rarity::Common => Color32::from_rgb(150, 160, 175),
        Rarity::Uncommon => Color32::from_rgb(100, 190, 140),
        Rarity::Rare => Color32::from_rgb(110, 150, 255),
        Rarity::Holo => GOLD,
    }
}

/// Each card once in the pack order (two of the same Energy share an id).
fn dedupe_order(order: Vec<usize>, n: usize) -> Vec<usize> {
    let mut used = vec![false; n];
    let mut out = Vec::with_capacity(n);
    for i in order {
        let j = if used[i] { (0..n).find(|k| !used[*k] && *k != i).unwrap_or(i) } else { i };
        used[j] = true;
        out.push(j);
    }
    out
}

/// How long a card takes to turn over (with its little swing back).
const FLIP_SECS: f32 = 0.8;

/// The card back's picture (loaded once).
fn card_back_tex(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("card-back-texture");
    ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)).or_else(|| {
        let img = image::load_from_memory(include_bytes!("../../assets/card_back.png")).ok()?.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], &img);
        let t = ctx.load_texture("card-back", ci, egui::TextureOptions::LINEAR);
        ctx.data_mut(|d| d.insert_temp(id, t.clone()));
        Some(t)
    })
}

/// A card turning over at `k` (0 face down → 1 face up): a real 3D turn
/// with a little perspective, the springy way the UI Lab's "Flip to reveal"
/// does it (it swings a touch past and settles back), lifting off the table
/// as it goes.
fn flip_turn(p: &egui::Painter, r: Rect, front: Option<&egui::TextureHandle>, k: f32) {
    use std::f32::consts::PI;
    let k = k.clamp(0.0, 1.0);
    // ease out with an overshoot (close to cubic-bezier(.4, 1.4, .5, 1))
    let c1 = 1.9;
    let c3 = c1 + 1.0;
    let e = 1.0 + c3 * (k - 1.0).powi(3) + c1 * (k - 1.0).powi(2);
    let angle = PI * e;
    let lift = (k * PI).sin().max(0.0);
    let r = Rect::from_center_size(r.center() - vec2(0.0, 10.0 * lift), r.size() * (1.0 + 0.06 * lift));
    p.add(kit::shadow_tier(3).as_shape(r.translate(vec2(0.0, 12.0 * lift)), 10.0));
    // which side is towards you: the back until halfway, then the front
    // (turned the other half so it isn't mirrored)
    let (tex, a) = if angle.cos() >= 0.0 { (card_back_tex(p.ctx()), angle) } else { (front.cloned(), angle - PI) };
    let (s, c) = a.sin_cos();
    let d = r.height() * 3.5;
    let mid = r.center();
    let proj = |q: egui::Pos2| -> egui::Pos2 {
        let x = q.x - mid.x;
        let z = x * s;
        let f = d / (d - z);
        pos2(mid.x + x * c * f, mid.y + (q.y - mid.y) * f)
    };
    match tex {
        Some(t) => {
            // the picture cropped like `store::paint_image`, on a grid that
            // keeps the rounded corners
            let ts = t.size_vec2();
            let (ra, ia) = (r.width() / r.height(), ts.x / ts.y);
            let uv = if ia > ra {
                let w = ra / ia;
                Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
            } else {
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, ia / ra))
            };
            let rr = r.width() * 0.045;
            let mut ys: Vec<f32> = (0..=4).map(|i| r.min.y + rr * (1.0 - (i as f32 / 4.0 * PI / 2.0).cos())).collect();
            ys.extend((1..8).map(|i| r.min.y + rr + (r.height() - 2.0 * rr) * i as f32 / 8.0));
            ys.extend((0..=4).rev().map(|i| r.max.y - rr * (1.0 - (i as f32 / 4.0 * PI / 2.0).cos())));
            const COLS: u32 = 8;
            let mut m = egui::Mesh::with_texture(t.id());
            // a little darker while it's edge-on, like light leaving it
            let shade = (0.55 + 0.45 * c.abs()).clamp(0.0, 1.0);
            let col = Color32::from_gray((255.0 * shade) as u8);
            for y in &ys {
                let dd = (r.min.y + rr - y).max(y - (r.max.y - rr)).clamp(0.0, rr);
                let inset = rr - (rr * rr - dd * dd).max(0.0).sqrt();
                let (lo, hi) = (r.min.x + inset, r.max.x - inset);
                for i in 0..=COLS {
                    let q = pos2(lo + (hi - lo) * i as f32 / COLS as f32, *y);
                    let f = vec2((q.x - r.min.x) / r.width(), (q.y - r.min.y) / r.height());
                    m.vertices.push(egui::epaint::Vertex { pos: proj(q), uv: uv.min + vec2(f.x * uv.width(), f.y * uv.height()), color: col });
                }
            }
            for row in 1..ys.len() as u32 {
                let (a0, b0) = ((row - 1) * (COLS + 1), row * (COLS + 1));
                for i in 0..COLS {
                    m.add_triangle(a0 + i, a0 + i + 1, b0 + i);
                    m.add_triangle(a0 + i + 1, b0 + i + 1, b0 + i);
                }
            }
            p.add(m);
        }
        None => {
            // the picture isn't here yet: a plain card, turning
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(proj);
            p.add(egui::Shape::convex_polygon(pts.to_vec(), Color32::from_rgb(236, 206, 72), Stroke::NONE));
        }
    }
}

/// A four-cornered piece of a picture (or a plain colour) at any angle.
fn quad(tex: Option<egui::TextureId>, uv: Rect, pts: [egui::Pos2; 4], col: Color32) -> egui::Shape {
    let mut m = match tex {
        Some(id) => egui::Mesh::with_texture(id),
        None => egui::Mesh::default(),
    };
    let uvs = if tex.is_some() { [uv.left_top(), uv.right_top(), uv.right_bottom(), uv.left_bottom()] } else { [egui::epaint::WHITE_UV; 4] };
    for (q, u) in pts.iter().zip(uvs) {
        m.vertices.push(egui::epaint::Vertex { pos: *q, uv: u, color: col });
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    egui::Shape::mesh(m)
}

/// A shape, fainter.
fn fade_shape(s: egui::Shape, a: f32) -> egui::Shape {
    match s {
        egui::Shape::Mesh(mut m) => {
            for v in &mut m.vertices {
                v.color = v.color.gamma_multiply(a);
            }
            egui::Shape::Mesh(m)
        }
        other => other,
    }
}
