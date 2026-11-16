//! The Journal app: notes in Markdown, like Obsidian. Files and folders on
//! the left, open notes as tabs, each in Live preview, Source or Reading,
//! and the note's outline, backlinks and tags on the right. A run's hunt
//! journal opens here too, as the same notes as on its page.

use super::journal_md::{self as md, Clicked, MdStyle};
use super::journal_ui as ju;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, RunRef, Tab};
use crate::journal::{self as j, Entry};
use crate::theme;
use eframe::egui::{self, pos2, text::CCursor, text_selection::CCursorRange, vec2, Align2, Color32, FontId, Rect, RichText, Sense, Stroke};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

pub(super) const JOURNAL_COL: Color32 = Color32::from_rgb(38, 190, 170);

/// How a note is shown.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub(super) enum Mode {
    #[default]
    Live,
    Source,
    Reading,
}

impl Mode {
    fn from_u8(v: u8) -> Mode {
        match v {
            1 => Mode::Source,
            2 => Mode::Reading,
            _ => Mode::Live,
        }
    }
}

/// An open tab: a note, or a run's hunt journal.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Open {
    Note(usize),
    Run(RunRef),
    /// A canvas (in `cbufs`).
    Canvas(usize),
    /// A picture (in `imgs`).
    Image(usize),
}

/// A note open in a tab.
pub(super) struct Buf {
    pub rel: String,
    pub text: String,
    pub saved: String,
    pub edited: Option<Instant>,
    pub mode: Mode,
    /// The title being edited (renames the file when it's done).
    pub title: String,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum Side {
    #[default]
    Files,
    Search,
    Tags,
    Marks,
}

/// What's read out of every note, for search, tags and backlinks.
#[derive(Default, Clone)]
pub(super) struct NoteInfo {
    pub tags: BTreeSet<String>,
    pub links: Vec<String>,
    pub lower: String,
    /// The start of the note as written (for previews).
    pub text: String,
    /// Last changed (unix seconds).
    pub modified: u64,
    pub words: usize,
    /// Unticked tasks: (line, text).
    pub tasks: Vec<(usize, String)>,
}

#[derive(Default)]
pub(super) struct JournalState {
    pub tree: Vec<Entry>,
    pub notes: Vec<String>,
    pub index: HashMap<String, NoteInfo>,
    pub scanned: Option<Instant>,
    pub bufs: Vec<Buf>,
    pub tabs: Vec<Open>,
    pub active: usize,
    pub side: Side,
    pub query: String,
    pub open_folders: BTreeSet<String>,
    pub restored: bool,
    /// Renaming (rel, new name).
    pub renaming: Option<(String, String)>,
    /// Ctrl+O: what's typed, and the highlighted row.
    pub switcher: Option<(String, usize)>,
    pub settings: bool,
    /// Put the cursor at the start of this line of the open note.
    pub goto_line: Option<usize>,
    pub error: Option<String>,
    /// A note was just made: put the cursor in its title (once).
    pub focus_title: bool,
    /// Open canvases.
    pub cbufs: Vec<super::journal_canvas::CanvasBuf>,
    /// Pictures open in tabs.
    pub imgs: Vec<String>,
    /// Every canvas and picture in the Journal.
    pub canvases: Vec<String>,
    pub images: Vec<String>,
    /// Pictures loaded for showing (None: couldn't be read).
    pub tex: HashMap<String, Option<(egui::TextureHandle, egui::Vec2)>>,
    /// The file list is hidden (the ribbon's panel button).
    pub hide_left: bool,
    /// The right panel's tab: 0 backlinks, 1 links out, 2 tags, 3 outline.
    pub right_tab: u8,
    /// Newest first in the file list (else by name).
    pub sort_new: bool,
    /// Back and forward, like a browser: the files seen before and after.
    pub back: Vec<String>,
    pub fwd: Vec<String>,
    pub last_rel: Option<String>,
    pub nav_silent: bool,
}

const WELCOME: &str = "# Welcome to your Journal\n\nYour notes live here as plain Markdown files, so they open anywhere.\n\n## The basics\n\n- **Bold**, *italic*, ~~strike~~ and ==highlight==\n- [[Links to other notes]] (click one in Reading to open or make it)\n- #tags, collected under Tags on the left\n- [ ] Tasks you can tick\n\n> [!tip] Three ways to see a note\n> **Live** shows it styled as you type, **Source** the plain text, and **Reading** the finished page. Switch at the top right (Ctrl+E).\n\nPress **Ctrl+O** to jump to any note, **Ctrl+N** for a new one.\n";

impl App {
    fn jroot(&self) -> std::path::PathBuf {
        j::root()
    }

    /// Re-read the folder (every few seconds, or now).
    pub(super) fn journal_scan(&mut self, now: bool) {
        let st = &self.setup.journal;
        if !now && st.scanned.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
            return;
        }
        let root = self.jroot();
        if !root.exists() {
            let _ = j::write(&root, "Welcome.md", WELCOME);
        }
        let tree = j::tree(&root);
        let notes = j::all_notes(&tree);
        let mut index = HashMap::new();
        for rel in &notes {
            let text = self.setup.journal.bufs.iter().find(|b| &b.rel == rel).map(|b| b.text.clone()).or_else(|| j::read(&root, rel)).unwrap_or_default();
            let modified = std::fs::metadata(root.join(rel)).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            let tasks = text.lines().enumerate().filter_map(|(n, l)| l.trim_start().strip_prefix("- [ ] ").map(|t| (n, t.trim().to_string()))).filter(|(_, t)| !t.is_empty()).collect();
            index.insert(rel.clone(), NoteInfo { tags: j::tags(&text), links: j::links(&text), lower: text.chars().take(20_000).collect::<String>().to_lowercase(), text: text.chars().take(20_000).collect(), modified, words: j::words(&text), tasks });
        }
        let st = &mut self.setup.journal;
        st.canvases = j::all_canvases(&tree);
        st.images = j::all_images(&tree);
        st.tree = tree;
        st.notes = notes;
        st.index = index;
        st.scanned = Some(Instant::now());
    }

    /// Save notes typed in a moment ago (runs every frame, on any page).
    pub(super) fn journal_flush(&mut self, force: bool) {
        let root = self.jroot();
        let mut wrote = false;
        for b in self.setup.journal.bufs.iter_mut() {
            if b.text != b.saved && (force || b.edited.is_some_and(|t| t.elapsed() > Duration::from_millis(700))) {
                match j::write(&root, &b.rel, &b.text) {
                    Ok(()) => {
                        b.saved = b.text.clone();
                        b.edited = None;
                        wrote = true;
                    }
                    Err(e) => self.setup.journal.error = Some(format!("Couldn't save {}: {e}", j::title_of(&b.rel))),
                }
            }
        }
        if wrote {
            self.setup.journal.scanned = None;
        }
        self.canvas_flush(force);
    }

    /// Open (or switch to) a note.
    pub(super) fn journal_open(&mut self, rel: &str) {
        self.journal_flush(true);
        let i = self.journal_buf(rel);
        let st = &mut self.setup.journal;
        match st.tabs.iter().position(|t| *t == Open::Note(i)) {
            Some(t) => st.active = t,
            None => {
                st.tabs.push(Open::Note(i));
                st.active = st.tabs.len() - 1;
            }
        }
        // its folders open in the tree
        let mut acc = String::new();
        for part in rel.split('/').collect::<Vec<_>>().split_last().map(|(_, f)| f.to_vec()).unwrap_or_default() {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            st.open_folders.insert(acc.clone());
        }
        self.journal_remember();
    }

    /// A note loaded for editing (not opened as a tab): its place in `bufs`.
    pub(super) fn journal_buf(&mut self, rel: &str) -> usize {
        let st = &mut self.setup.journal;
        match st.bufs.iter().position(|b| b.rel == rel) {
            Some(i) => i,
            None => {
                let text = j::read(&j::root(), rel).unwrap_or_default();
                st.bufs.push(Buf { rel: rel.to_string(), saved: text.clone(), text, edited: None, mode: Mode::from_u8(self.cfg.journal.default_mode), title: j::title_of(rel) });
                st.bufs.len() - 1
            }
        }
    }

    /// Open a run's hunt journal in a tab.
    pub(super) fn journal_open_run(&mut self, run: RunRef) {
        let st = &mut self.setup.journal;
        match st.tabs.iter().position(|t| *t == Open::Run(run)) {
            Some(t) => st.active = t,
            None => {
                st.tabs.push(Open::Run(run));
                st.active = st.tabs.len() - 1;
            }
        }
    }

    /// A new note (in `folder`), opened, with its title ready to type.
    fn journal_new(&mut self, folder: &str, name: &str, body: &str) -> String {
        let root = self.jroot();
        let rel = j::free_rel(&root, folder, name);
        if let Err(e) = j::write(&root, &rel, body) {
            self.setup.journal.error = Some(format!("Couldn't make the note: {e}"));
        }
        self.journal_scan(true);
        self.journal_open(&rel);
        // an empty title to type into ("Untitled" shows as its hint)
        if let Some(b) = self.setup.journal.bufs.iter_mut().find(|b| b.rel == rel) {
            if b.title.starts_with("Untitled") {
                b.title.clear();
            }
            b.mode = b.mode.min_edit();
        }
        self.setup.journal.focus_title = true;
        rel
    }

    /// Today's note (made the first time).
    fn journal_today(&mut self) {
        let rel = self.journal_today_rel();
        self.journal_open(&rel);
    }

    /// Today's note's `rel`, making it the first time.
    pub(super) fn journal_today_rel(&mut self) -> String {
        let day = crate::stats::local_day(crate::stats::now_unix());
        let (y, m, d) = crate::stats::civil_from_days(day);
        let name = format!("{y:04}-{m:02}-{d:02}");
        let folder = j::clean_name(&self.cfg.journal.daily_folder);
        let rel = format!("{folder}/{name}.md");
        let root = self.jroot();
        if !root.join(&rel).exists() {
            let _ = j::write(&root, &rel, &j::daily_template(&crate::stats::fmt_day(day)));
            self.journal_scan(true);
        }
        rel
    }

    /// Add a line to the end of a note (open or not).
    pub(super) fn journal_append(&mut self, rel: &str, line: &str) {
        let root = self.jroot();
        let add = |t: &mut String| {
            if !t.is_empty() && !t.ends_with('\n') {
                t.push('\n');
            }
            t.push_str(line);
            t.push('\n');
        };
        if let Some(b) = self.setup.journal.bufs.iter_mut().find(|b| b.rel == rel) {
            add(&mut b.text);
            b.edited = Some(Instant::now());
        } else {
            let mut t = j::read(&root, rel).unwrap_or_default();
            add(&mut t);
            if let Err(e) = j::write(&root, rel, &t) {
                self.setup.journal.error = Some(format!("Couldn't save {}: {e}", j::title_of(rel)));
            }
        }
        self.setup.journal.scanned = None;
    }

    /// Reopen last time's tabs (once).
    fn journal_restore(&mut self) {
        if self.setup.journal.restored {
            return;
        }
        self.setup.journal.restored = true;
        self.journal_scan(false);
        let tabs: Vec<String> = self.cfg.journal.tabs.iter().filter(|r| self.setup.journal.notes.contains(r) || self.setup.journal.canvases.contains(r)).cloned().collect();
        for rel in &tabs {
            if j::is_canvas(rel) {
                self.journal_open_canvas(rel);
            } else {
                self.journal_open(rel);
            }
        }
        if self.setup.journal.tabs.is_empty() {
            if let Some(first) = self.setup.journal.notes.first().cloned() {
                self.journal_open(&first);
            }
        }
    }

    /// Go to the Journal with this note open (from anywhere).
    pub(super) fn journal_go(&mut self, rel: Option<&str>, line: Option<usize>) {
        self.journal_restore();
        if let Some(r) = rel {
            self.journal_open(r);
            if line.is_some() {
                self.setup.journal.goto_line = line;
                if let Some(Open::Note(i)) = self.setup.journal.tabs.get(self.setup.journal.active).copied() {
                    if self.setup.journal.bufs[i].mode == Mode::Reading {
                        self.setup.journal.bufs[i].mode = Mode::Live;
                    }
                }
            }
        }
        self.go_to(Tab::Journal);
    }

    /// Keep the open tabs for next time.
    pub(super) fn journal_remember(&mut self) {
        let st = &self.setup.journal;
        let tabs: Vec<String> = st
            .tabs
            .iter()
            .filter_map(|t| match t {
                Open::Note(i) => st.bufs.get(*i).map(|b| b.rel.clone()),
                Open::Canvas(i) => st.cbufs.get(*i).map(|b| b.rel.clone()),
                _ => None,
            })
            .collect();
        if tabs != self.cfg.journal.tabs {
            self.cfg.journal.tabs = tabs;
            self.dirty = true;
        }
    }

    fn journal_close_tab(&mut self, t: usize) {
        self.journal_flush(true);
        let st = &mut self.setup.journal;
        if t < st.tabs.len() {
            st.tabs.remove(t);
            if st.active >= st.tabs.len() {
                st.active = st.tabs.len().saturating_sub(1);
            } else if st.active > t {
                st.active -= 1;
            }
        }
        self.journal_remember();
    }

    /// A note was renamed or moved: its tabs follow it.
    fn journal_moved(&mut self, from: &str, to: &str) {
        for b in self.setup.journal.bufs.iter_mut() {
            if b.rel == from {
                b.rel = to.to_string();
                b.title = j::title_of(to);
            } else if let Some(rest) = b.rel.strip_prefix(&format!("{from}/")) {
                b.rel = format!("{to}/{rest}");
            }
        }
        for b in self.setup.journal.cbufs.iter_mut() {
            if b.rel == from {
                b.rel = to.to_string();
            } else if let Some(rest) = b.rel.strip_prefix(&format!("{from}/")) {
                b.rel = format!("{to}/{rest}");
            }
        }
        for b in self.setup.journal.imgs.iter_mut() {
            if b == from {
                *b = to.to_string();
            } else if let Some(rest) = b.strip_prefix(&format!("{from}/")) {
                *b = format!("{to}/{rest}");
            }
        }
        for m in self.cfg.journal.bookmarks.iter_mut() {
            if m == from {
                *m = to.to_string();
            }
        }
        // a hunt's (or the Pokégotchi's) note and canvas follow too
        let moved = |p: &mut Option<String>| {
            if let Some(r) = p.as_mut() {
                if r == from {
                    *r = to.to_string();
                } else if let Some(rest) = r.strip_prefix(&format!("{from}/")) {
                    *r = format!("{to}/{rest}");
                }
            }
        };
        for l in self.cfg.journal.links.values_mut() {
            moved(&mut l.note);
            moved(&mut l.canvas);
        }
        self.dirty = true;
        self.journal_scan(true);
        self.journal_remember();
    }

    /// A note (or folder) went to the trash: close its tabs.
    fn journal_trashed(&mut self, rel: &str) {
        let st = &mut self.setup.journal;
        let inside = |r: &str| r == rel || r.starts_with(&format!("{rel}/"));
        let gone: Vec<usize> = st.bufs.iter().enumerate().filter(|(_, b)| inside(&b.rel)).map(|(i, _)| i).collect();
        let gone_c: Vec<usize> = st.cbufs.iter().enumerate().filter(|(_, b)| inside(&b.rel)).map(|(i, _)| i).collect();
        let gone_i: Vec<usize> = st.imgs.iter().enumerate().filter(|(_, b)| inside(b)).map(|(i, _)| i).collect();
        st.tabs.retain(|t| match t {
            Open::Note(i) => !gone.contains(i),
            Open::Canvas(i) => !gone_c.contains(i),
            Open::Image(i) => !gone_i.contains(i),
            Open::Run(_) => true,
        });
        // a trashed canvas's changes aren't saved over it
        for i in gone_c {
            st.cbufs[i].edited = None;
            st.cbufs[i].broken = Some("Moved to the trash".into());
        }
        if st.active >= st.tabs.len() {
            st.active = st.tabs.len().saturating_sub(1);
        }
        self.cfg.journal.bookmarks.retain(|m| m != rel);
        self.journal_scan(true);
        self.journal_remember();
    }

    /// Follow a `[[link]]` (making the note if there isn't one), an app
    /// link (`ptc://…`), a picture or canvas, or a web address.
    pub(super) fn journal_follow(&mut self, ctx: &egui::Context, target: &str) {
        if target.starts_with("http://") || target.starts_with("https://") {
            ctx.open_url(egui::OpenUrl::new_tab(target));
            return;
        }
        if target.starts_with("ptc://") {
            self.open_ptc(target);
            return;
        }
        self.journal_scan(true);
        if j::is_image(target) {
            if let Some(rel) = j::resolve_image(target, &self.setup.journal.images) {
                self.journal_open_image(&rel);
            }
            return;
        }
        if j::is_canvas(target) {
            let l = target.to_lowercase();
            if let Some(rel) = self.setup.journal.canvases.iter().find(|c| c.to_lowercase() == l || c.to_lowercase().ends_with(&format!("/{l}"))).cloned() {
                self.journal_open_canvas(&rel);
            }
            return;
        }
        match j::resolve(target, &self.setup.journal.notes) {
            Some(rel) => self.journal_open(&rel),
            None => {
                let (folder, name) = match target.rsplit_once('/') {
                    Some((f, n)) => (f.to_string(), n.to_string()),
                    None => (String::new(), target.to_string()),
                };
                let folder: String = folder.split('/').map(j::clean_name).collect::<Vec<_>>().join("/");
                self.journal_new(&folder, &name, "");
            }
        }
    }

    /// The Journal page.
    pub(super) fn page_journal(&mut self, ui: &mut egui::Ui) {
        super::app_tour::here(ui.ctx(), "journal");
        let ctx = ui.ctx().clone();
        self.journal_scan(false);
        self.journal_restore();
        // keys
        let (ctrl_o, ctrl_n, ctrl_e) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::N),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::E),
            )
        });
        if ctrl_o {
            self.setup.journal.switcher = Some((String::new(), 0));
        }
        if ctrl_n {
            self.journal_new("", "Untitled", "");
        }
        if ctrl_e {
            if let Some(Open::Note(i)) = self.setup.journal.tabs.get(self.setup.journal.active).copied() {
                if let Some(b) = self.setup.journal.bufs.get_mut(i) {
                    b.mode = if b.mode == Mode::Reading { Mode::from_u8(self.cfg.journal.default_mode).min_edit() } else { Mode::Reading };
                }
            }
        }

        self.journal_track_history();
        let full = ui.max_rect();
        ui.allocate_rect(full, Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, ju::main_bg());
        // ribbon | files | the open tab | right panel, as Obsidian
        let rib_w = 44.0;
        let narrow = full.width() < 860.0;
        let ex_w = if narrow || self.setup.journal.hide_left { 0.0 } else { 272.0 };
        let on_board = matches!(self.setup.journal.tabs.get(self.setup.journal.active), Some(Open::Canvas(_)) | Some(Open::Image(_)));
        let right_w = if self.cfg.journal.right_panel && full.width() >= 1180.0 && !on_board { 280.0 } else { 0.0 };
        let rib = Rect::from_min_size(full.min, vec2(rib_w, full.height()));
        let ex = Rect::from_min_size(pos2(rib.max.x, full.min.y), vec2(ex_w, full.height()));
        let right = Rect::from_min_max(pos2(full.max.x - right_w, full.min.y), full.max);
        let mid = Rect::from_min_max(pos2(ex.max.x, full.min.y), pos2(right.min.x, full.max.y));
        p.rect_filled(rib, 0.0, ju::side_bg());
        p.vline(rib.max.x, rib.y_range(), Stroke::new(1.0f32, LINE()));
        self.journal_ribbon(ui, rib);
        super::app_tour::mark(ui.ctx(), "journal.ribbon", rib);
        super::app_tour::mark(ui.ctx(), "journal.main", mid);
        if ex_w > 0.0 {
            p.rect_filled(ex, 0.0, ju::side_bg());
            p.vline(ex.max.x, ex.y_range(), Stroke::new(1.0f32, LINE()));
            self.journal_left(ui, ex);
            super::app_tour::mark(ui.ctx(), "journal.left", ex);
        }
        if right_w > 0.0 {
            p.rect_filled(right, 0.0, ju::side_bg());
            p.vline(right.min.x, right.y_range(), Stroke::new(1.0f32, LINE()));
            self.journal_right(ui, right);
        }
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(mid).layout(egui::Layout::top_down(egui::Align::Min)), |ui| self.journal_main(ui));
        self.journal_switcher(&ctx);
        if self.setup.home.gallery.to_canvas.is_some() {
            self.home_gallery(&ctx);
        }
        // (the cogs open Settings → Journal)
        if std::mem::take(&mut self.setup.journal.settings) {
            self.open_settings(super::Tab::Journal);
        }
        self.journal_flush(false);
        if self.setup.journal.bufs.iter().any(|b| b.text != b.saved) {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }

    // ───────────────────────────── the left: files, search, tags, bookmarks ─────────────────────────────

    /// Back and forward: note when the open file changes.
    fn journal_track_history(&mut self) {
        let cur = self.journal_active_rel();
        let st = &mut self.setup.journal;
        if cur != st.last_rel {
            if !st.nav_silent {
                if let Some(l) = st.last_rel.take() {
                    st.back.push(l);
                    if st.back.len() > 60 {
                        st.back.remove(0);
                    }
                }
                st.fwd.clear();
            }
            st.nav_silent = false;
            st.last_rel = cur;
        }
    }

    fn journal_step(&mut self, back: bool) {
        let root = self.jroot();
        loop {
            let st = &mut self.setup.journal;
            let Some(r) = (if back { st.back.pop() } else { st.fwd.pop() }) else { return };
            if !root.join(&r).exists() {
                continue;
            }
            if let Some(cur) = st.last_rel.clone() {
                if back {
                    st.fwd.push(cur);
                } else {
                    st.back.push(cur);
                }
            }
            st.nav_silent = true;
            self.journal_open_any(&r);
            return;
        }
    }

    /// The ribbon: the thin strip of buttons down the left, as Obsidian.
    fn journal_ribbon(&mut self, ui: &mut egui::Ui, r: Rect) {
        let top = [
            ("sidebar-left", if self.setup.journal.hide_left { "Show the file list" } else { "Hide the file list" }),
            ("switcher", "Open the quick switcher (Ctrl+O)"),
            ("note-plus", "New note (Ctrl+N)"),
            ("canvas", "New canvas"),
            ("calendar", "Open today's note"),
        ];
        for (k, (icon, tip)) in top.iter().enumerate() {
            let br = Rect::from_center_size(pos2(r.center().x, r.min.y + 24.0 + k as f32 * 38.0), vec2(32.0, 32.0));
            if ju::icon_btn(ui, br, ("rib", k), icon, tip, k == 0 && !self.setup.journal.hide_left).clicked() {
                match k {
                    0 => self.setup.journal.hide_left = !self.setup.journal.hide_left,
                    1 => self.setup.journal.switcher = Some((String::new(), 0)),
                    2 => {
                        self.journal_new("", "Untitled", "");
                    }
                    3 => self.journal_new_canvas(""),
                    _ => self.journal_today(),
                }
            }
        }
        let bottom = [("folder-open", "Open the Journal folder"), ("settings", "Journal settings")];
        for (k, (icon, tip)) in bottom.iter().enumerate() {
            let br = Rect::from_center_size(pos2(r.center().x, r.max.y - 24.0 - (1 - k) as f32 * 38.0), vec2(32.0, 32.0));
            if ju::icon_btn(ui, br, ("rib-b", k), icon, tip, false).clicked() {
                if k == 0 {
                    let root = self.jroot();
                    let _ = std::fs::create_dir_all(&root);
                    crate::share::reveal(&root);
                } else {
                    self.setup.journal.settings = true;
                }
            }
        }
    }

    /// The left panel: files, search, bookmarks and tags, then the vault.
    fn journal_left(&mut self, ui: &mut egui::Ui, r: Rect) {
        let side = self.setup.journal.side;
        // the panel's own tabs
        let tabs = [(Side::Files, "folder", "Files"), (Side::Search, "search", "Search"), (Side::Marks, "bookmark", "Bookmarks"), (Side::Tags, "hash", "Tags")];
        for (k, (s, icon, tip)) in tabs.iter().enumerate() {
            let br = Rect::from_min_size(pos2(r.min.x + 10.0 + k as f32 * 36.0, r.min.y + 6.0), vec2(32.0, 30.0));
            if ju::icon_btn(ui, br, ("side", k), icon, tip, side == *s).clicked() {
                self.setup.journal.side = *s;
            }
        }
        let mut y = r.min.y + 44.0;
        let w = r.width() - 20.0;
        match side {
            Side::Files => {
                // new note, new folder, new canvas, sort, fold all: centred, as Obsidian
                let acts = [
                    ("note-plus", "New note"),
                    ("folder-plus", "New folder"),
                    ("canvas", "New canvas"),
                    ("sort", if self.setup.journal.sort_new { "Sorted newest first (click for A to Z)" } else { "Sorted A to Z (click for newest first)" }),
                    ("collapse", "Fold every folder"),
                ];
                let x0 = r.center().x - acts.len() as f32 * 32.0 / 2.0;
                for (k, (icon, tip)) in acts.iter().enumerate() {
                    let br = Rect::from_min_size(pos2(x0 + k as f32 * 32.0, y), vec2(30.0, 28.0));
                    if ju::icon_btn(ui, br, ("files-act", k), icon, tip, k == 3 && self.setup.journal.sort_new).clicked() {
                        match k {
                            0 => {
                                self.journal_new("", "Untitled", "");
                            }
                            1 => {
                                let root = self.jroot();
                                let mut name = "New folder".to_string();
                                let mut n = 2;
                                while root.join(&name).exists() {
                                    name = format!("New folder {n}");
                                    n += 1;
                                }
                                let _ = std::fs::create_dir_all(root.join(&name));
                                self.journal_scan(true);
                                self.setup.journal.renaming = Some((name.clone(), name));
                            }
                            2 => self.journal_new_canvas(""),
                            3 => self.setup.journal.sort_new = !self.setup.journal.sort_new,
                            _ => self.setup.journal.open_folders.clear(),
                        }
                    }
                }
                y += 36.0;
            }
            Side::Search => {
                let mut q = std::mem::take(&mut self.setup.journal.query);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(r.min.x + 10.0, y), vec2(w, 34.0))), |ui| {
                    kit::search_box_keyed(ui, Some(egui::Id::new("j-search")), &mut q, "Search notes or #tags", w);
                });
                self.setup.journal.query = q;
                y += 42.0;
            }
            Side::Marks | Side::Tags => {
                let t = if side == Side::Marks { "Bookmarks" } else { "Tags" };
                ui.painter().text(pos2(r.min.x + 16.0, y + 12.0), Align2::LEFT_CENTER, t, crate::fonts::sans_bold_font(12.5), MUTED());
                y += 30.0;
            }
        }
        // the vault, at the bottom
        let foot = Rect::from_min_max(pos2(r.min.x, r.max.y - 44.0), r.max);
        ui.painter().hline(foot.x_range(), foot.min.y, Stroke::new(1.0f32, LINE()));
        ju::draw(ui.painter(), "vault", pos2(foot.min.x + 22.0, foot.center().y), ju::icon_col());
        ui.painter().text(pos2(foot.min.x + 36.0, foot.center().y), Align2::LEFT_CENTER, "Journal", crate::fonts::sans_bold_font(13.0), TEXT());
        let n = self.setup.journal.notes.len();
        let help = Rect::from_center_size(pos2(foot.max.x - 50.0, foot.center().y), vec2(28.0, 28.0));
        let cog = Rect::from_center_size(pos2(foot.max.x - 20.0, foot.center().y), vec2(28.0, 28.0));
        if ju::icon_btn(ui, help, "vault-help", "help", &format!("{n} notes. Open the Welcome note: how the Journal works"), false).clicked() {
            let root = self.jroot();
            if !root.join("Welcome.md").exists() {
                let _ = j::write(&root, "Welcome.md", WELCOME);
                self.journal_scan(true);
            }
            self.journal_open("Welcome.md");
        }
        if ju::icon_btn(ui, cog, "vault-cog", "settings", "Journal settings", false).clicked() {
            self.setup.journal.settings = true;
        }
        // the list
        let list = Rect::from_min_max(pos2(r.min.x + 8.0, y), pos2(r.max.x - 8.0, foot.min.y - 4.0));
        let mut open: Option<String> = None;
        let mut open_run: Option<RunRef> = None;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            egui::ScrollArea::vertical().id_salt(("j-side", side as u8)).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(list.width());
                ui.spacing_mut().item_spacing.y = 1.0;
                match side {
                    Side::Files => {
                        let tree = self.setup.journal.tree.clone();
                        self.journal_tree(ui, &tree, 0, &mut open);
                        if tree.is_empty() {
                            ui.label(RichText::new("No notes yet.").size(kit::fs(12.0)).color(MUTED()));
                        }
                        // hunt journals, as notes
                        let runs = self.journal_runs();
                        if !runs.is_empty() {
                            ui.add_space(14.0);
                            ui.label(RichText::new("  Hunt journals").size(kit::fs(11.5)).color(MUTED()));
                            ui.add_space(2.0);
                            for (run, name, n) in runs {
                                let on = self.setup.journal.tabs.get(self.setup.journal.active) == Some(&Open::Run(run));
                                let resp = tree_row(ui, 0, "✨", &name, &format!("{n}"), on, kit::C_VIOLET);
                                if resp.on_hover_text("This run's hunt journal: the same notes as on its page").clicked() {
                                    open_run = Some(run);
                                }
                            }
                        }
                    }
                    Side::Search => self.journal_search_list(ui, &mut open),
                    Side::Tags => self.journal_tag_list(ui),
                    Side::Marks => {
                        let marks = self.cfg.journal.bookmarks.clone();
                        if marks.is_empty() {
                            ui.label(RichText::new("  Nothing bookmarked yet. Use a note's ⋮ menu.").size(kit::fs(12.0)).color(MUTED()));
                        }
                        for m in marks {
                            let on = self.journal_active_rel().as_deref() == Some(m.as_str());
                            if tree_row(ui, 0, "", &j::title_of(&m), "", on, kit::C_GOLD).clicked() {
                                open = Some(m);
                            }
                        }
                    }
                }
            });
        });
        if let Some(rel) = open {
            self.journal_open_any(&rel);
        }
        if let Some(r) = open_run {
            self.journal_open_run(r);
        }
    }

    fn journal_active_rel(&self) -> Option<String> {
        let st = &self.setup.journal;
        match st.tabs.get(st.active) {
            Some(Open::Note(i)) => st.bufs.get(*i).map(|b| b.rel.clone()),
            Some(Open::Canvas(i)) => st.cbufs.get(*i).map(|b| b.rel.clone()),
            Some(Open::Image(i)) => st.imgs.get(*i).cloned(),
            _ => None,
        }
    }

    /// Open a note, canvas or picture, whichever it is.
    pub(super) fn journal_open_any(&mut self, rel: &str) {
        if j::is_canvas(rel) {
            self.journal_open_canvas(rel);
        } else if j::is_image(rel) {
            self.journal_open_image(rel);
        } else {
            self.journal_open(rel);
        }
    }

    /// A new canvas (in `folder`), opened.
    pub(super) fn journal_new_canvas(&mut self, folder: &str) {
        let root = self.jroot();
        let rel = j::free_rel_ext(&root, folder, "Untitled canvas", "canvas");
        if let Err(e) = j::write(&root, &rel, &crate::jcanvas::Canvas::default().to_json()) {
            self.setup.journal.error = Some(format!("Couldn't make the canvas: {e}"));
            return;
        }
        self.journal_scan(true);
        self.journal_open_canvas(&rel);
        // straight to naming it
        let leaf = rel.rsplit('/').next().unwrap_or(&rel).trim_end_matches(".canvas").to_string();
        self.setup.journal.renaming = Some((rel.clone(), leaf));
        if !folder.is_empty() {
            self.setup.journal.open_folders.insert(folder.to_string());
        }
        self.setup.journal.side = Side::Files;
    }

    /// Open a picture in a tab.
    pub(super) fn journal_open_image(&mut self, rel: &str) {
        let st = &mut self.setup.journal;
        let i = match st.imgs.iter().position(|r| r == rel) {
            Some(i) => i,
            None => {
                st.imgs.push(rel.to_string());
                st.imgs.len() - 1
            }
        };
        match st.tabs.iter().position(|t| *t == Open::Image(i)) {
            Some(t) => st.active = t,
            None => {
                st.tabs.push(Open::Image(i));
                st.active = st.tabs.len() - 1;
            }
        }
    }

    /// A picture, as big as fits.
    fn journal_image_tab(&mut self, ui: &mut egui::Ui, i: usize) {
        let Some(rel) = self.setup.journal.imgs.get(i).cloned() else { return };
        let full = ui.max_rect();
        let ctx = ui.ctx().clone();
        match self.journal_tex(&ctx, &rel) {
            Some((tex, size)) => {
                let room = full.shrink(28.0);
                let s = (room.width() / size.x).min(room.height() / size.y).min(2.0);
                let r = Rect::from_center_size(room.center(), size * s);
                ui.painter().add(kit::shadow_tier(3).as_shape(r, 6.0));
                ui.painter().add(egui::Shape::image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE));
                ui.painter().text(pos2(full.center().x, full.max.y - 14.0), Align2::CENTER_BOTTOM, format!("{rel} · {}×{}", size.x as u32, size.y as u32), crate::fonts::sans_font(11.5), MUTED());
            }
            None => {
                ui.painter().text(full.center(), Align2::CENTER_CENTER, format!("Couldn't show {rel}"), crate::fonts::sans_font(13.0), MUTED());
            }
        }
    }

    /// A picture from the Journal, ready to draw: (texture, its size).
    pub(super) fn journal_tex(&mut self, ctx: &egui::Context, rel: &str) -> Option<(egui::TextureId, egui::Vec2)> {
        if !self.setup.journal.tex.contains_key(rel) {
            let loaded = std::fs::read(j::root().join(rel)).ok().and_then(|b| super::avatar::decode(&b).ok()).map(|img| {
                // big photos are made smaller (the card shows them small anyway)
                let img = if img.width() > 2048 || img.height() > 2048 { image::imageops::thumbnail(&img, 2048.min(img.width()), (2048 * img.height() / img.width().max(1)).min(2048)) } else { img };
                let size = [img.width() as usize, img.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                (ctx.load_texture(format!("journal-{rel}"), ci, egui::TextureOptions::LINEAR), vec2(size[0] as f32, size[1] as f32))
            });
            self.setup.journal.tex.insert(rel.to_string(), loaded);
        }
        self.setup.journal.tex.get(rel).and_then(|t| t.as_ref().map(|(h, s)| (h.id(), *s)))
    }

    /// An embed in Reading: a picture (fit to the width), or a note as a link.
    pub(super) fn journal_embed(&mut self, ui: &mut egui::Ui, what: &str, width: f32) -> Option<md::Clicked> {
        if j::is_image(what) {
            let rel = j::resolve_image(what, &self.setup.journal.images);
            let ctx = ui.ctx().clone();
            match rel.as_deref().and_then(|r| self.journal_tex(&ctx, r)) {
                Some((tex, size)) => {
                    let s = (width / size.x).min(1.0);
                    let (r, resp) = ui.allocate_exact_size(size * s, Sense::click());
                    ui.painter().add(egui::Shape::image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE));
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open the picture").clicked() {
                        return rel.map(md::Clicked::Link);
                    }
                }
                None => {
                    ui.label(RichText::new(format!("🖼 Missing picture: {what}")).color(MUTED()));
                }
            }
            return None;
        }
        let name = j::title_of(what);
        if ui.link(RichText::new(format!("↪ {name}")).color(md::LINK)).clicked() {
            return Some(md::Clicked::Link(what.to_string()));
        }
        None
    }

    /// The folder tree, with right-click menus and renaming in place.
    fn journal_tree(&mut self, ui: &mut egui::Ui, es: &[Entry], depth: usize, open: &mut Option<String>) {
        let active = self.journal_active_rel();
        // newest first (folders stay first, by name)
        let mut sorted: Vec<Entry> = es.to_vec();
        if self.setup.journal.sort_new {
            let root = self.jroot();
            let when = |e: &Entry| std::fs::metadata(root.join(e.rel())).and_then(|m| m.modified()).ok();
            sorted.sort_by(|a, b| match (matches!(a, Entry::Folder { .. }), matches!(b, Entry::Folder { .. })) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => std::cmp::Ordering::Equal,
                _ => when(b).cmp(&when(a)),
            });
        }
        for e in &sorted {
            let rel = e.rel().to_string();
            // renaming this one?
            if let Some((r, mut name)) = self.setup.journal.renaming.clone() {
                if r == rel {
                    let id = egui::Id::new(("j-rename", &rel));
                    let resp = ui.horizontal(|ui| {
                        ui.add_space(8.0 + depth as f32 * 14.0);
                        ui.add(egui::TextEdit::singleline(&mut name).id(id).desired_width(ui.available_width() - 6.0))
                    }).inner;
                    if !resp.has_focus() && !resp.lost_focus() {
                        resp.request_focus();
                    }
                    let done = resp.lost_focus();
                    if done {
                        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        self.setup.journal.renaming = None;
                        if !cancel {
                            self.journal_rename(&rel, &name, matches!(e, Entry::Folder { .. }));
                        }
                    } else {
                        self.setup.journal.renaming = Some((r, name));
                    }
                    continue;
                }
            }
            match e {
                Entry::Folder { name, children, .. } => {
                    let is_open = self.setup.journal.open_folders.contains(&rel);
                    let resp = tree_row(ui, depth, if is_open { "▾" } else { "▸" }, name, "", false, TEXT());
                    if resp.clicked() {
                        if is_open {
                            self.setup.journal.open_folders.remove(&rel);
                        } else {
                            self.setup.journal.open_folders.insert(rel.clone());
                        }
                    }
                    let mut act: Option<u8> = None;
                    resp.context_menu(|ui| {
                        if ui.button("+ New note here").clicked() {
                            act = Some(0);
                            ui.close_menu();
                        }
                        if ui.button("+ New folder here").clicked() {
                            act = Some(1);
                            ui.close_menu();
                        }
                        if ui.button("Rename").clicked() {
                            act = Some(2);
                            ui.close_menu();
                        }
                        if ui.button("Move to trash").on_hover_text("Moved to the Journal's .trash folder, not deleted").clicked() {
                            act = Some(3);
                            ui.close_menu();
                        }
                    });
                    match act {
                        Some(0) => {
                            self.setup.journal.open_folders.insert(rel.clone());
                            self.journal_new(&rel, "Untitled", "");
                        }
                        Some(1) => {
                            let root = self.jroot();
                            let mut sub = format!("{rel}/New folder");
                            let mut n = 2;
                            while root.join(&sub).exists() {
                                sub = format!("{rel}/New folder {n}");
                                n += 1;
                            }
                            let _ = std::fs::create_dir_all(root.join(&sub));
                            self.setup.journal.open_folders.insert(rel.clone());
                            self.journal_scan(true);
                            let leaf = sub.rsplit('/').next().unwrap_or("").to_string();
                            self.setup.journal.renaming = Some((sub, leaf));
                        }
                        Some(2) => self.setup.journal.renaming = Some((rel.clone(), name.clone())),
                        Some(3) => {
                            self.journal_flush(true);
                            if j::trash(&self.jroot(), &rel).is_ok() {
                                self.journal_trashed(&rel);
                            }
                        }
                        _ => {}
                    }
                    if is_open {
                        self.journal_tree(ui, children, depth + 1, open);
                    }
                }
                Entry::Note { name, .. } | Entry::Canvas { name, .. } | Entry::Image { name, .. } => {
                    let on = active.as_deref() == Some(rel.as_str());
                    let badge = match e {
                        Entry::Canvas { .. } => "CANVAS".to_string(),
                        Entry::Image { .. } => rel.rsplit('.').next().unwrap_or("").to_uppercase(),
                        _ => String::new(),
                    };
                    let shown = match e {
                        Entry::Image { .. } => name.rsplit_once('.').map_or(name.as_str(), |(n, _)| n),
                        _ => name.as_str(),
                    };
                    let resp = tree_row(ui, depth, "", shown, &badge, on, JOURNAL_COL);
                    if resp.clicked() {
                        *open = Some(rel.clone());
                    }
                    let marked = self.cfg.journal.bookmarks.contains(&rel);
                    let mut act: Option<u8> = None;
                    resp.context_menu(|ui| {
                        if ui.button("Open").clicked() {
                            act = Some(0);
                            ui.close_menu();
                        }
                        if ui.button("Rename").clicked() {
                            act = Some(1);
                            ui.close_menu();
                        }
                        if ui.button(if marked { "Remove bookmark" } else { "★ Bookmark" }).clicked() {
                            act = Some(2);
                            ui.close_menu();
                        }
                        if ui.button("Move to trash").on_hover_text("Moved to the Journal's .trash folder, not deleted").clicked() {
                            act = Some(3);
                            ui.close_menu();
                        }
                    });
                    match act {
                        Some(0) => *open = Some(rel.clone()),
                        Some(1) => self.setup.journal.renaming = Some((rel.clone(), name.clone())),
                        Some(2) => {
                            if marked {
                                self.cfg.journal.bookmarks.retain(|m| *m != rel);
                            } else {
                                self.cfg.journal.bookmarks.push(rel.clone());
                            }
                            self.dirty = true;
                        }
                        Some(3) => {
                            self.journal_flush(true);
                            if j::trash(&self.jroot(), &rel).is_ok() {
                                self.journal_trashed(&rel);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Rename a note or folder to `name` (a `/` in it moves it).
    fn journal_rename(&mut self, rel: &str, name: &str, folder: bool) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let parent = rel.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default();
        let parts: Vec<String> = name.split('/').map(j::clean_name).collect();
        let mut joined = parts.join("/");
        // a file keeps its type (.md, .canvas, .png…)
        let ext = if folder { String::new() } else { rel.rsplit_once('.').map(|(_, e)| format!(".{e}")).unwrap_or_else(|| ".md".into()) };
        if !ext.is_empty() && joined.to_lowercase().ends_with(&ext.to_lowercase()) {
            joined.truncate(joined.len() - ext.len());
        }
        let new_rel = if parent.is_empty() { format!("{joined}{ext}") } else { format!("{parent}/{joined}{ext}") };
        if new_rel == rel {
            return;
        }
        self.journal_flush(true);
        match j::rename(&self.jroot(), rel, &new_rel) {
            Ok(to) => {
                if folder && self.setup.journal.open_folders.remove(rel) {
                    self.setup.journal.open_folders.insert(to.clone());
                }
                self.journal_moved(rel, &to);
            }
            Err(e) => self.setup.journal.error = Some(e),
        }
    }

    fn journal_search_list(&mut self, ui: &mut egui::Ui, open: &mut Option<String>) {
        let q = self.setup.journal.query.trim().to_lowercase();
        if q.is_empty() {
            ui.label(RichText::new("Type to search every note. Start with # for a tag.").size(kit::fs(12.0)).color(MUTED()));
            return;
        }
        let tag = q.strip_prefix('#').map(|t| t.to_string());
        let mut hits: Vec<(String, String)> = Vec::new();
        for rel in &self.setup.journal.notes {
            let Some(info) = self.setup.journal.index.get(rel) else { continue };
            let ok = match &tag {
                Some(t) => info.tags.iter().any(|x| x.to_lowercase() == *t || x.to_lowercase().starts_with(&format!("{t}/"))),
                None => j::title_of(rel).to_lowercase().contains(&q) || info.lower.contains(&q),
            };
            if ok {
                let line = match &tag {
                    Some(_) => String::new(),
                    None => info.text.lines().find(|l| l.to_lowercase().contains(&q)).map(|l| plain(l).chars().take(70).collect()).unwrap_or_default(),
                };
                hits.push((rel.clone(), line));
            }
        }
        ui.label(RichText::new(format!("{} result{}", hits.len(), if hits.len() == 1 { "" } else { "s" })).size(kit::fs(11.5)).color(MUTED()));
        ui.add_space(4.0);
        let active = self.journal_active_rel();
        for (rel, line) in hits {
            let on = active.as_deref() == Some(rel.as_str());
            let w = ui.available_width();
            let h = if line.is_empty() { 34.0 } else { 52.0 };
            let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter();
            p.rect_filled(r, 9.0, if on { theme::blend(CARD(), JOURNAL_COL, 0.14) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
            let tf = crate::fonts::sans_bold_font(12.5);
            let title = super::charts::clip(ui, &j::title_of(&rel), &tf, w - 24.0);
            p.text(pos2(r.min.x + 12.0, r.min.y + 17.0), Align2::LEFT_CENTER, title, tf, TEXT());
            if !line.is_empty() {
                let lf = crate::fonts::sans_font(11.5);
                let l = super::charts::clip(ui, &line, &lf, w - 24.0);
                p.text(pos2(r.min.x + 12.0, r.min.y + 36.0), Align2::LEFT_CENTER, l, lf, MUTED());
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *open = Some(rel.clone());
            }
            ui.add_space(2.0);
        }
    }

    fn journal_tag_list(&mut self, ui: &mut egui::Ui) {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for info in self.setup.journal.index.values() {
            for t in &info.tags {
                *counts.entry(t.clone()).or_default() += 1;
            }
        }
        if counts.is_empty() {
            ui.label(RichText::new("No tags yet. Write #anything in a note.").size(kit::fs(12.0)).color(MUTED()));
        }
        let mut pick = None;
        for (t, n) in counts {
            if tree_row(ui, 0, "#", &t, &n.to_string(), false, md::TAG).clicked() {
                pick = Some(t);
            }
        }
        if let Some(t) = pick {
            self.setup.journal.side = Side::Search;
            self.setup.journal.query = format!("#{t}");
        }
    }

    /// Runs with a hunt journal (and the current run): (run, name, notes).
    fn journal_runs(&self) -> Vec<(RunRef, String, usize)> {
        let mut out = vec![(RunRef::Current, format!("{} (now)", self.target_name(self.cfg.target.as_deref())), self.cfg.run_stats.notes.len())];
        for r in &self.cfg.saved_runs {
            if r.id != self.cfg.run_id && !r.stats.notes.is_empty() {
                out.push((RunRef::Saved(r.id), self.target_name(r.target.as_deref()), r.stats.notes.len()));
            }
        }
        for (i, h) in self.cfg.history.iter().enumerate().rev() {
            if let Some(st) = &h.stats {
                if !st.notes.is_empty() {
                    out.push((RunRef::Finished(i), h.target.clone(), st.notes.len()));
                }
            }
        }
        out
    }

    // ───────────────────────────── the middle: tabs, toolbar, the note ─────────────────────────────

    fn journal_main(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        // the tab bar, as Obsidian: flat tabs, the open one joined to the page
        let bar = Rect::from_min_size(full.min, vec2(full.width(), 40.0));
        ui.painter().rect_filled(bar, 0.0, ju::side_bg());
        ui.painter().hline(bar.x_range(), bar.max.y - 0.5, Stroke::new(1.0f32, LINE()));
        let mut close: Option<usize> = None;
        let mut pick: Option<usize> = None;
        let active = self.setup.journal.active;
        let side_r = Rect::from_center_size(pos2(bar.max.x - 22.0, bar.center().y), vec2(30.0, 28.0));
        let list_r = Rect::from_center_size(pos2(side_r.min.x - 18.0, bar.center().y), vec2(30.0, 28.0));
        if ju::icon_btn(ui, side_r, "tab-right", "sidebar-right", if self.cfg.journal.right_panel { "Hide the right panel" } else { "Show the right panel (links, tags, outline)" }, false).clicked() {
            self.cfg.journal.right_panel = !self.cfg.journal.right_panel;
            self.dirty = true;
        }
        let list_resp = ju::icon_btn(ui, list_r, "tab-list", "chevron-down", "Every open tab", false);
        let tabs_right = list_r.min.x - 8.0;
        let mut x = bar.min.x + 8.0;
        let tab_list = self.setup.journal.tabs.clone();
        let n = tab_list.len().max(1);
        let tab_w = ((tabs_right - x - 40.0) / n as f32).clamp(90.0, 230.0);
        let titles: Vec<(String, &'static str)> = tab_list
            .iter()
            .map(|t| match t {
                Open::Note(i) => (self.setup.journal.bufs.get(*i).map_or("?".to_string(), |b| j::title_of(&b.rel)), ""),
                Open::Run(r) => (self.run_info(*r).map(|i| i.name).unwrap_or_default(), "sparkle"),
                Open::Canvas(i) => (self.setup.journal.cbufs.get(*i).map_or("?".to_string(), |b| j::title_of(&b.rel)), "canvas"),
                Open::Image(i) => (self.setup.journal.imgs.get(*i).map_or("?".to_string(), |r| r.rsplit('/').next().unwrap_or(r).to_string()), "image"),
            })
            .collect();
        for (k, (title, icon)) in titles.iter().enumerate() {
            if x + tab_w > tabs_right - 30.0 {
                break;
            }
            let r = Rect::from_min_size(pos2(x, bar.min.y + 7.0), vec2(tab_w, bar.height() - 7.0));
            let resp = ui.interact(r, ui.id().with(("j-tab", k)), Sense::click());
            let on = k == active;
            let p = ui.painter();
            if on {
                p.rect_filled(r, egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, ju::main_bg());
                p.vline(r.min.x, r.y_range(), Stroke::new(1.0f32, LINE()));
                p.vline(r.max.x, r.y_range(), Stroke::new(1.0f32, LINE()));
                p.hline(r.x_range(), r.min.y, Stroke::new(1.0f32, LINE()));
            } else if resp.hovered() {
                p.rect_filled(r.shrink2(vec2(2.0, 3.0)), 6.0, ju::hover_bg());
            } else if k + 1 != active && k + 1 < titles.len() {
                p.vline(r.max.x + 2.0, r.y_range().shrink(9.0), Stroke::new(1.0f32, LINE()));
            }
            let mut tx = r.min.x + 12.0;
            if !icon.is_empty() {
                ju::draw(p, icon, pos2(tx + 6.0, r.center().y), if on { TEXT() } else { ju::icon_col() });
                tx += 18.0;
            }
            let f = crate::fonts::sans_font(13.0);
            let label = super::charts::clip(ui, title, &f, r.max.x - tx - 30.0);
            p.text(pos2(tx, r.center().y), Align2::LEFT_CENTER, label, f, if on { TEXT() } else { MUTED() });
            if on || resp.hovered() {
                let xr = Rect::from_center_size(pos2(r.max.x - 16.0, r.center().y), vec2(20.0, 20.0));
                if ju::icon_btn(ui, xr, ("tab-x", k), "x", "Close", false).clicked() {
                    close = Some(k);
                }
            }
            if close.is_none() && resp.middle_clicked() {
                close = Some(k);
            } else if resp.clicked() {
                pick = Some(k);
            }
            x += tab_w + 4.0;
        }
        let plus = Rect::from_min_size(pos2(x + 2.0, bar.min.y + 7.0), vec2(28.0, 26.0));
        if ju::icon_btn(ui, plus, "tab-plus", "plus", "New tab: a new note (Ctrl+N)", false).clicked() {
            self.journal_new("", "Untitled", "");
        }
        // every open tab, from the arrow
        let pid = ui.id().with("j-tab-list");
        if list_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(pid));
        }
        egui::popup::popup_below_widget(ui, pid, &list_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(220.0);
            if titles.is_empty() {
                ui.label(RichText::new("No tabs open").color(MUTED()));
            }
            for (k, (t, _)) in titles.iter().enumerate() {
                if ui.selectable_label(k == active, t).clicked() {
                    pick = Some(k);
                }
            }
        });
        if let Some(k) = pick {
            self.setup.journal.active = k;
        }
        if let Some(k) = close {
            self.journal_close_tab(k);
        }
        let body = Rect::from_min_max(pos2(full.min.x, bar.max.y), full.max);
        let tab = self.setup.journal.tabs.get(self.setup.journal.active).copied();
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)), |ui| match tab {
            Some(Open::Note(i)) => self.journal_note_in(ui, i, false),
            Some(Open::Run(r)) => {
                let (head, rest) = self.journal_head_rect(ui);
                let name = self.run_info(r).map(|i| format!("Hunt journal / {}", i.name)).unwrap_or_default();
                self.journal_view_head(ui, head, &name, None);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)), |ui| self.journal_run_tab(ui, r));
            }
            Some(Open::Canvas(i)) => {
                let (head, rest) = self.journal_head_rect(ui);
                if let Some(rel) = self.setup.journal.cbufs.get(i).map(|b| b.rel.clone()) {
                    self.journal_view_head(ui, head, &rel, Some(&rel));
                }
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)), |ui| self.journal_canvas(ui, i));
            }
            Some(Open::Image(i)) => {
                let (head, rest) = self.journal_head_rect(ui);
                if let Some(rel) = self.setup.journal.imgs.get(i).cloned() {
                    self.journal_view_head(ui, head, &rel, Some(&rel));
                }
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)), |ui| self.journal_image_tab(ui, i));
            }
            None => self.journal_empty(ui),
        });
        // a save that went wrong, over the page
        if let Some(e) = self.setup.journal.error.clone() {
            let er = Rect::from_min_size(body.min + vec2(16.0, 48.0), vec2(body.width() - 32.0, 30.0));
            ui.painter().rect_filled(er, 8.0, theme::blend(ju::main_bg(), kit::C_RED, 0.25));
            ui.painter().text(er.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, &e, crate::fonts::sans_font(12.0), TEXT());
            if ui.interact(er, ui.id().with("j-err"), Sense::click()).on_hover_text("Click to dismiss").clicked() {
                self.setup.journal.error = None;
            }
        }
    }

    /// A view's header row, and the room under it.
    fn journal_head_rect(&self, ui: &egui::Ui) -> (Rect, Rect) {
        let r = ui.max_rect();
        (Rect::from_min_size(r.min, vec2(r.width(), 40.0)), Rect::from_min_max(pos2(r.min.x, r.min.y + 40.0), r.max))
    }

    /// The header row of a view: back and forward, where it is (in the
    /// middle), and the ⋮ menu for a file. Returns the right edge free for
    /// more buttons.
    fn journal_view_head(&mut self, ui: &mut egui::Ui, r: Rect, path: &str, file: Option<&str>) -> f32 {
        let can_back = !self.setup.journal.back.is_empty();
        let can_fwd = !self.setup.journal.fwd.is_empty();
        let b = Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), vec2(28.0, 28.0));
        let f = Rect::from_center_size(pos2(r.min.x + 52.0, r.center().y), vec2(28.0, 28.0));
        if ju::icon_btn_en(ui, b, "head-back", "back", "Back", false, can_back).clicked() {
            self.journal_step(true);
        }
        if ju::icon_btn_en(ui, f, "head-fwd", "forward", "Forward", false, can_fwd).clicked() {
            self.journal_step(false);
        }
        self.journal_path_title(ui, r, path);
        let more = Rect::from_center_size(pos2(r.max.x - 22.0, r.center().y), vec2(28.0, 28.0));
        if let Some(rel) = file {
            let resp = ju::icon_btn(ui, more, "head-more", "more", "More options", false);
            self.journal_file_menu(ui, &resp, rel, None);
            more.min.x - 4.0
        } else {
            r.max.x - 8.0
        }
    }

    /// "Folder / Title" in the middle of a header.
    fn journal_path_title(&self, ui: &egui::Ui, r: Rect, path: &str) {
        let p = ui.painter();
        let shown = path.trim_end_matches(".md").trim_end_matches(".canvas");
        let (folder, name) = match shown.rsplit_once('/') {
            Some((f, n)) => (format!("{} / ", f.replace('/', " / ")), n.to_string()),
            None => (String::new(), shown.to_string()),
        };
        let fnt = crate::fonts::sans_font(13.0);
        let g1 = p.layout_no_wrap(folder, fnt.clone(), MUTED());
        let g2 = p.layout_no_wrap(name, fnt, TEXT());
        let w = g1.size().x + g2.size().x;
        let x = (r.center().x - w / 2.0).max(r.min.x + 80.0);
        let y = r.center().y - g2.size().y / 2.0;
        let w1 = g1.size().x;
        p.galley(pos2(x, y), g1, MUTED());
        p.galley(pos2(x + w1, y), g2, TEXT());
    }

    /// The ⋮ menu for a file (a note gets its view options too).
    fn journal_file_menu(&mut self, ui: &mut egui::Ui, resp: &egui::Response, rel: &str, note: Option<usize>) {
        let pid = ui.id().with(("j-file-menu", rel));
        if resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(pid));
        }
        let marked = self.cfg.journal.bookmarks.iter().any(|m| m == rel);
        let mode = note.and_then(|i| self.setup.journal.bufs.get(i).map(|b| b.mode));
        let mut act: Option<u8> = None;
        egui::popup::popup_below_widget(ui, pid, resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(210.0);
            if let Some(m) = mode {
                if ui.button(if m == Mode::Reading { "Editing view   Ctrl+E" } else { "Reading view   Ctrl+E" }).clicked() {
                    act = Some(10);
                }
                if m != Mode::Reading && ui.button(if m == Mode::Source { "✓ Source mode" } else { "   Source mode" }).on_hover_text("Plain text, with every mark showing").clicked() {
                    act = Some(11);
                }
                ui.separator();
            }
            if ui.button("Rename…").clicked() {
                act = Some(1);
            }
            if ui.button(if marked { "Remove bookmark" } else { "Bookmark" }).clicked() {
                act = Some(2);
            }
            if ui.button("Show in folder").clicked() {
                act = Some(3);
            }
            ui.separator();
            if ui.button(RichText::new("Move to trash").color(kit::C_RED)).on_hover_text("Moved to the Journal's .trash folder, not deleted").clicked() {
                act = Some(4);
            }
        });
        match act {
            Some(10) | Some(11) => {
                if let Some(b) = note.and_then(|i| self.setup.journal.bufs.get_mut(i)) {
                    b.mode = match (act, b.mode) {
                        (Some(10), Mode::Reading) => Mode::from_u8(self.cfg.journal.default_mode).min_edit(),
                        (Some(10), _) => Mode::Reading,
                        (_, Mode::Source) => Mode::Live,
                        _ => Mode::Source,
                    };
                }
            }
            Some(1) => {
                let leaf = rel.rsplit('/').next().unwrap_or(rel);
                let leaf = if j::is_image(leaf) { leaf.to_string() } else { j::title_of(leaf) };
                self.setup.journal.renaming = Some((rel.to_string(), leaf));
                self.setup.journal.side = Side::Files;
                self.setup.journal.hide_left = false;
                // its folders open, so it can be seen
                let mut acc = String::new();
                for part in rel.split('/').collect::<Vec<_>>().split_last().map(|(_, f)| f.to_vec()).unwrap_or_default() {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    acc.push_str(part);
                    self.setup.journal.open_folders.insert(acc.clone());
                }
            }
            Some(2) => {
                if marked {
                    self.cfg.journal.bookmarks.retain(|m| m != rel);
                } else {
                    self.cfg.journal.bookmarks.push(rel.to_string());
                }
                self.dirty = true;
            }
            Some(3) => crate::share::reveal(&self.jroot().join(rel)),
            Some(4) => {
                self.journal_flush(true);
                if j::trash(&self.jroot(), rel).is_ok() {
                    self.journal_trashed(rel);
                }
            }
            _ => {}
        }
    }

    /// No tab open: Obsidian's quiet page with three ways on.
    fn journal_empty(&mut self, ui: &mut egui::Ui) {
        let r = ui.max_rect();
        let c = r.center() - vec2(0.0, 40.0);
        ui.painter().text(c, Align2::CENTER_CENTER, "No file is open", crate::fonts::sans_bold_font(20.0), TEXT());
        let links = [("Create new note  (Ctrl+N)", 0), ("Go to file  (Ctrl+O)", 1), ("Open today's note", 2)];
        for (k, (t, a)) in links.iter().enumerate() {
            let lr = Rect::from_center_size(c + vec2(0.0, 44.0 + k as f32 * 30.0), vec2(260.0, 26.0));
            let resp = ui.interact(lr, ui.id().with(("j-empty", k)), Sense::click());
            ui.painter().text(lr.center(), Align2::CENTER_CENTER, *t, crate::fonts::sans_font(14.0), if resp.hovered() { TEXT() } else { JOURNAL_COL });
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                match a {
                    0 => {
                        self.journal_new("", "Untitled", "");
                    }
                    1 => self.setup.journal.switcher = Some((String::new(), 0)),
                    _ => self.journal_today(),
                }
            }
        }
    }

    /// A note shown somewhere else in the app (a hunt's own note).
    pub(super) fn journal_note(&mut self, ui: &mut egui::Ui, i: usize) {
        self.journal_note_in(ui, i, true);
    }

    /// One note: its header, the page, and the little status corner.
    /// `embedded`: shown inside another page (no back and forward).
    fn journal_note_in(&mut self, ui: &mut egui::Ui, i: usize, embedded: bool) {
        let full = ui.max_rect();
        let ctx = ui.ctx().clone();
        let Some(b) = self.setup.journal.bufs.get(i) else { return };
        let mode = b.mode;
        let rel = b.rel.clone();
        let id = egui::Id::new(("j-note-edit", i));
        let st = MdStyle { size: self.cfg.journal.font_size.clamp(11.0, 26.0), accent: JOURNAL_COL };
        // header: back, forward, the path, then reading/editing and ⋮
        let tool_h = 40.0;
        let head = Rect::from_min_size(full.min, vec2(full.width(), tool_h));
        let more = Rect::from_center_size(pos2(head.max.x - 22.0, head.center().y), vec2(28.0, 28.0));
        let view = Rect::from_center_size(pos2(more.min.x - 18.0, head.center().y), vec2(28.0, 28.0));
        if embedded {
            self.journal_path_title(ui, Rect::from_min_max(head.min, pos2(view.min.x, head.max.y)), &rel);
        } else {
            let back = !self.setup.journal.back.is_empty();
            let fwd = !self.setup.journal.fwd.is_empty();
            if ju::icon_btn_en(ui, Rect::from_center_size(pos2(head.min.x + 22.0, head.center().y), vec2(28.0, 28.0)), ("nb", i), "back", "Back", false, back).clicked() {
                self.journal_step(true);
            }
            if ju::icon_btn_en(ui, Rect::from_center_size(pos2(head.min.x + 52.0, head.center().y), vec2(28.0, 28.0)), ("nf", i), "forward", "Forward", false, fwd).clicked() {
                self.journal_step(false);
            }
            self.journal_path_title(ui, head, &rel);
        }
        let reading = mode == Mode::Reading;
        if ju::icon_btn(ui, view, ("nview", i), if reading { "pencil" } else { "book" }, if reading { "Edit (Ctrl+E)" } else { "Reading view (Ctrl+E)" }, false).clicked() {
            let b = &mut self.setup.journal.bufs[i];
            b.mode = if reading { Mode::from_u8(self.cfg.journal.default_mode).min_edit() } else { Mode::Reading };
        }
        let mresp = ju::icon_btn(ui, more, ("nmore", i), "more", "More options", false);
        self.journal_file_menu(ui, &mresp, &rel, Some(i));
        // Ctrl+B / Ctrl+I in the editor
        let mut tool: Option<Tool> = None;
        if mode != Mode::Reading && ctx.memory(|m| m.has_focus(id)) {
            let (bk, ik) = ctx.input_mut(|inp| (inp.consume_key(egui::Modifiers::COMMAND, egui::Key::B), inp.consume_key(egui::Modifiers::COMMAND, egui::Key::I)));
            if bk {
                tool = Some(Tool::Bold);
            }
            if ik {
                tool = Some(Tool::Italic);
            }
        }
        // the corner: backlinks, words, characters (as Obsidian's status bar)
        let status = Rect::from_min_max(pos2(full.min.x, full.max.y), full.max);
        {
            let b = &self.setup.journal.bufs[i];
            let backlinks = self.journal_backlinks(&rel).len();
            let words = j::words(&b.text);
            let saving = if b.text == b.saved { "" } else { "   saving…" };
            let txt = format!("{backlinks} backlink{}      {words} words  {} characters{saving}", if backlinks == 1 { "" } else { "s" }, b.text.chars().count());
            let f = crate::fonts::sans_font(11.5);
            let g = ui.painter().layout_no_wrap(txt, f, MUTED());
            let pr = Rect::from_min_max(pos2(full.max.x - g.size().x - 24.0, full.max.y - g.size().y - 10.0), full.max);
            ui.painter().rect_filled(pr, egui::Rounding { nw: 8.0, ne: 0.0, sw: 0.0, se: 0.0 }, ju::side_bg());
            ui.painter().rect_stroke(pr, egui::Rounding { nw: 8.0, ne: 0.0, sw: 0.0, se: 0.0 }, Stroke::new(1.0f32, LINE()));
            ui.painter().galley(pos2(pr.min.x + 12.0, pr.min.y + 5.0), g, MUTED());
        }
        let page = Rect::from_min_max(pos2(full.min.x, full.min.y + tool_h), pos2(full.max.x, status.min.y));
        let col_w = if self.cfg.journal.readable { (page.width() - 64.0).min(760.0) } else { page.width() - 64.0 };
        let mut clicked: Option<Clicked> = None;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(page).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            egui::ScrollArea::vertical().id_salt(("j-note-scroll", i)).auto_shrink([false, false]).show(ui, |ui| {
                let pad = ((ui.available_width() - col_w) / 2.0).max(16.0);
                ui.add_space(26.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        // the title: the file's name (editing it renames the note),
                        // unless the note already opens with that heading
                        let tid = egui::Id::new(("j-title", i));
                        let mut title = self.setup.journal.bufs[i].title.clone();
                        let same_head = {
                            let t = &self.setup.journal.bufs[i].text;
                            let (_, at) = j::front_matter(t);
                            t[at.min(t.len())..].lines().find(|l| !l.trim().is_empty()).and_then(|l| l.strip_prefix("# ")).is_some_and(|h| h.trim().eq_ignore_ascii_case(title.trim()))
                        };
                        if !same_head {
                            let resp = ui.add(egui::TextEdit::singleline(&mut title).id(tid).frame(false).font(FontId::new(st.size * 1.9, crate::fonts::title())).desired_width(col_w).hint_text("Untitled"));
                            if std::mem::take(&mut self.setup.journal.focus_title) {
                                // a brand new note: straight to naming it
                                resp.request_focus();
                            }
                            let enter = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            self.setup.journal.bufs[i].title = title.clone();
                            if resp.lost_focus() || enter {
                                if enter {
                                    ui.memory_mut(|m| m.request_focus(id));
                                }
                                let cur = j::title_of(&rel);
                                if title.trim() != cur && !title.trim().is_empty() {
                                    self.journal_rename(&rel, &title, false);
                                } else {
                                    self.setup.journal.bufs[i].title = cur;
                                }
                            }
                            ui.add_space(6.0);
                        }
                        let b = &mut self.setup.journal.bufs[i];
                        let (props, _) = j::front_matter(&b.text);
                        match mode {
                            Mode::Reading => {
                                if !props.is_empty() {
                                    properties_card(ui, &props);
                                    ui.add_space(10.0);
                                }
                                if b.text.trim().is_empty() {
                                    ui.label(RichText::new("This note is empty. Switch to Live to write in it.").size(st.size).color(MUTED()));
                                }
                                let text = b.text.clone();
                                let mut embed = |ui: &mut egui::Ui, what: &str, w: f32| self.journal_embed(ui, what, w);
                                clicked = md::reading(ui, &text, st, &mut embed);
                                ui.add_space(80.0);
                            }
                            Mode::Live | Mode::Source => {
                                let min_h = (page.height() - 120.0).max(200.0);
                                let resp = if mode == Mode::Live {
                                    let mut layouter = |ui: &egui::Ui, s: &str, wrap: f32| {
                                        let mut job = md::live_job(s, st);
                                        job.wrap.max_width = wrap;
                                        ui.fonts(|f| f.layout_job(job))
                                    };
                                    ui.add(egui::TextEdit::multiline(&mut b.text).id(id).frame(false).desired_width(col_w).min_size(vec2(col_w, min_h)).lock_focus(true).hint_text("Start writing…").layouter(&mut layouter))
                                } else {
                                    ui.add(egui::TextEdit::multiline(&mut b.text).id(id).frame(false).desired_width(col_w).min_size(vec2(col_w, min_h)).lock_focus(true).font(FontId::monospace(st.size * 0.92)).hint_text("Start writing…"))
                                };
                                if resp.changed() {
                                    b.edited = Some(Instant::now());
                                }
                                // right-click: format, paragraph, insert (as Obsidian's editor menu)
                                resp.context_menu(|ui| {
                                    ui.set_min_width(190.0);
                                    ui.menu_button("Format", |ui| {
                                        for (t, l) in [(Tool::Bold, "Bold   Ctrl+B"), (Tool::Italic, "Italic   Ctrl+I"), (Tool::Strike, "Strikethrough"), (Tool::Mark, "Highlight"), (Tool::Code, "Code")] {
                                            if ui.button(l).clicked() {
                                                tool = Some(t);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                    ui.menu_button("Paragraph", |ui| {
                                        for (t, l) in [(Tool::Bullet, "Bullet list"), (Tool::Number, "Numbered list"), (Tool::Task, "Task list"), (Tool::H1, "Heading 1"), (Tool::H2, "Heading 2"), (Tool::H3, "Heading 3"), (Tool::Quote, "Quote")] {
                                            if ui.button(l).clicked() {
                                                tool = Some(t);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                    ui.menu_button("Insert", |ui| {
                                        for (t, l) in [(Tool::Link, "Link to a note   [[ ]]"), (Tool::Callout, "Callout"), (Tool::Table, "Table"), (Tool::Rule, "Divider")] {
                                            if ui.button(l).clicked() {
                                                tool = Some(t);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                });
                                // jump to a heading from the outline
                                if let Some(line) = self.setup.journal.goto_line.take() {
                                    let b = &self.setup.journal.bufs[i];
                                    let at: usize = b.text.split_inclusive('\n').take(line).map(|l| l.chars().count()).sum();
                                    if let Some(mut s) = egui::TextEdit::load_state(&ctx, id) {
                                        s.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at))));
                                        s.store(&ctx, id);
                                    }
                                    ctx.memory_mut(|m| m.request_focus(id));
                                }
                            }
                        }
                    });
                });
            });
        });
        if let Some(t) = tool {
            let b = &mut self.setup.journal.bufs[i];
            apply_tool(&ctx, id, &mut b.text, t);
            b.edited = Some(Instant::now());
        }
        match clicked {
            Some(Clicked::Link(t)) => self.journal_follow(&ctx, &t),
            Some(Clicked::Tag(t)) => {
                self.setup.journal.side = Side::Search;
                self.setup.journal.query = format!("#{t}");
            }
            Some(Clicked::Toggle(line)) => {
                let b = &mut self.setup.journal.bufs[i];
                b.text = md::toggle_task(&b.text, line);
                b.edited = Some(Instant::now());
            }
            None => {}
        }
    }

    /// A run's hunt journal, as a tab: the same notes as on the run's page.
    fn journal_run_tab(&mut self, ui: &mut egui::Ui, run: RunRef) {
        let full = ui.max_rect();
        let Some(info) = self.run_info(run) else {
            ui.label(RichText::new("This run is gone.").color(MUTED()));
            return;
        };
        let mut open_run = false;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full.shrink2(vec2(32.0, 24.0))).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            egui::ScrollArea::vertical().id_salt(("j-run", format!("{run:?}"))).auto_shrink([false, false]).show(ui, |ui| {
                let w = ui.available_width().min(820.0);
                ui.set_max_width(w);
                ui.label(RichText::new(format!("✨  {}", info.name)).font(FontId::new(26.0, crate::fonts::title())).color(TEXT()));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} encounters  ·  {}", theme::thousands(info.count), info.method)).size(kit::fs(12.5)).color(MUTED()));
                    if ui.link(RichText::new("Open the run ›").size(kit::fs(12.5))).clicked() {
                        open_run = true;
                    }
                });
                ui.add_space(6.0);
                ui.label(RichText::new("These are the run's own notes: add or remove one here or on its page and both change.").size(kit::fs(12.0)).color(MUTED()));
                self.journal_section(ui, run, info.count);
            });
        });
        if open_run {
            self.setup.tab = Tab::Runs;
            self.setup.view_run = Some(run);
        }
    }

    /// Notes that link to `rel`, with the line that does.
    fn journal_backlinks(&self, rel: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (other, info) in &self.setup.journal.index {
            if other == rel {
                continue;
            }
            if info.links.iter().any(|l| j::link_hits(l, rel) || j::resolve(l, &self.setup.journal.notes).as_deref() == Some(rel)) {
                let t = j::title_of(rel).to_lowercase();
                let line = info.text.lines().find(|l| l.to_lowercase().contains(&format!("[[{t}"))).or_else(|| info.text.lines().find(|l| l.to_lowercase().contains(&t))).map(|l| plain(l).chars().take(90).collect()).unwrap_or_default();
                out.push((other.clone(), line));
            }
        }
        out.sort();
        out
    }

    // ───────────────────────────── the right: outline, links, tags ─────────────────────────────

    fn journal_right(&mut self, ui: &mut egui::Ui, r: Rect) {
        // its own tabs, as Obsidian's right sidebar
        let tabs = [("link", "Backlinks"), ("arrow-out", "Outgoing links"), ("hash", "Tags"), ("list", "Outline")];
        let on = self.setup.journal.right_tab.min(3);
        for (k, (icon, tip)) in tabs.iter().enumerate() {
            let br = Rect::from_min_size(pos2(r.min.x + 10.0 + k as f32 * 36.0, r.min.y + 6.0), vec2(32.0, 30.0));
            if ju::icon_btn(ui, br, ("right", k), icon, tip, on as usize == k).clicked() {
                self.setup.journal.right_tab = k as u8;
            }
        }
        let body = Rect::from_min_max(pos2(r.min.x + 14.0, r.min.y + 46.0), pos2(r.max.x - 12.0, r.max.y - 8.0));
        let st = &self.setup.journal;
        let Some(Open::Note(i)) = st.tabs.get(st.active).copied() else {
            ui.painter().text(body.left_top(), Align2::LEFT_TOP, "No note open", crate::fonts::sans_font(12.5), MUTED());
            return;
        };
        let Some(b) = st.bufs.get(i) else { return };
        let rel = b.rel.clone();
        let text = b.text.clone();
        let mut go_line = None;
        let mut open: Option<String> = None;
        let mut follow: Option<String> = None;
        let mut tag: Option<String> = None;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            egui::ScrollArea::vertical().id_salt(("j-right", on)).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(body.width());
                match on {
                    0 => {
                        let backs = self.journal_backlinks(&rel);
                        side_title(ui, &format!("Linked mentions  {}", backs.len()));
                        if backs.is_empty() {
                            ui.label(RichText::new(format!("No backlinks found. Write [[{}]] in another note.", j::title_of(&rel))).size(kit::fs(12.0)).color(MUTED()));
                        }
                        for (other, line) in backs {
                            let resp = egui::Frame::none().fill(ju::hover_bg()).rounding(6.0).inner_margin(egui::Margin::same(8.0)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(j::title_of(&other)).size(kit::fs(12.5)).strong().color(TEXT()));
                                if !line.is_empty() {
                                    ui.label(RichText::new(line).size(kit::fs(11.5)).color(MUTED()));
                                }
                            });
                            if ui.interact(resp.response.rect, ui.id().with(("j-back", &other)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                open = Some(other.clone());
                            }
                            ui.add_space(4.0);
                        }
                    }
                    1 => {
                        let outs = j::links(&text);
                        side_title(ui, "Outgoing links");
                        if outs.is_empty() {
                            ui.label(RichText::new("No links in this note.").size(kit::fs(12.0)).color(MUTED()));
                        }
                        let mut seen = BTreeSet::new();
                        for l in outs {
                            if !seen.insert(l.to_lowercase()) {
                                continue;
                            }
                            let exists = j::resolve(&l, &self.setup.journal.notes).is_some() || j::is_image(&l) || j::is_canvas(&l);
                            if tree_row(ui, 0, "", &l, if exists { "" } else { "not made" }, false, JOURNAL_COL).on_hover_text(if exists { "Open it" } else { "Not made yet: click to make it" }).clicked() {
                                follow = Some(l);
                            }
                        }
                    }
                    2 => {
                        let tags = j::tags(&text);
                        side_title(ui, "Tags in this note");
                        if tags.is_empty() {
                            ui.label(RichText::new("No tags. Write #anything.").size(kit::fs(12.0)).color(MUTED()));
                        }
                        for t in tags {
                            if tree_row(ui, 0, "#", &t, "", false, md::TAG).clicked() {
                                tag = Some(t.clone());
                            }
                        }
                    }
                    _ => {
                        let heads = j::headings(&text);
                        side_title(ui, "Outline");
                        if heads.is_empty() {
                            ui.label(RichText::new("No headings. Start a line with # for one.").size(kit::fs(12.0)).color(MUTED()));
                        }
                        for (lvl, t, line) in heads {
                            if tree_row(ui, (lvl.saturating_sub(1)) as usize, "", &t, "", false, JOURNAL_COL).clicked() {
                                go_line = Some(line);
                            }
                        }
                    }
                }
            });
        });
        if let Some(l) = go_line {
            let b = &mut self.setup.journal.bufs[i];
            if b.mode == Mode::Reading {
                b.mode = Mode::Live;
            }
            self.setup.journal.goto_line = Some(l);
        }
        if let Some(r) = open {
            self.journal_open(&r);
        }
        if let Some(l) = follow {
            let ctx = ui.ctx().clone();
            self.journal_follow(&ctx, &l);
        }
        if let Some(t) = tag {
            self.setup.journal.side = Side::Search;
            self.setup.journal.query = format!("#{t}");
            self.setup.journal.hide_left = false;
        }
    }

    // ───────────────────────────── Ctrl+O and settings ─────────────────────────────

    fn journal_switcher(&mut self, ctx: &egui::Context) {
        let Some((mut q, mut sel)) = self.setup.journal.switcher.clone() else { return };
        let screen = ctx.screen_rect();
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let ql = q.trim().to_lowercase();
        let mut list: Vec<String> = self.setup.journal.notes.iter().chain(self.setup.journal.canvases.iter()).filter(|r| ql.is_empty() || j::title_of(r).to_lowercase().contains(&ql) || r.to_lowercase().contains(&ql)).cloned().collect();
        list.sort_by_key(|r| (!j::title_of(r).to_lowercase().starts_with(&ql), j::title_of(r).to_lowercase()));
        list.truncate(12);
        let create = !ql.is_empty() && !list.iter().any(|r| j::title_of(r).to_lowercase() == ql);
        let rows = list.len() + usize::from(create);
        let (down, up, enter) = ctx.input(|i| (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::Enter)));
        if down && rows > 0 {
            sel = (sel + 1) % rows;
        }
        if up && rows > 0 {
            sel = (sel + rows - 1) % rows;
        }
        sel = sel.min(rows.saturating_sub(1));
        let mut chosen: Option<usize> = if enter && rows > 0 { Some(sel) } else { None };
        egui::Area::new(egui::Id::new("j-switcher")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(130));
            if scrim.clicked() {
                close = true;
            }
            let w = 560.0f32.min(sr.width() - 40.0);
            let panel = Rect::from_min_size(pos2(sr.center().x - w / 2.0, sr.min.y + 110.0), vec2(w, 64.0 + rows as f32 * 38.0));
            ui.painter().add(kit::shadow_tier(4).as_shape(panel, 14.0));
            ui.painter().rect_filled(panel, 14.0, CARD());
            ui.painter().rect_stroke(panel, 14.0, Stroke::new(1.0f32, LINE()));
            ui.interact(panel, ui.id().with("j-sw-panel"), Sense::click());
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel.shrink(10.0)).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                let id = egui::Id::new("j-sw-q");
                let resp = ui.add(egui::TextEdit::singleline(&mut q).id(id).hint_text("Find or make a note…").desired_width(w - 20.0).font(FontId::proportional(15.0)));
                if !resp.has_focus() {
                    resp.request_focus();
                }
                ui.add_space(6.0);
                for (k, r) in list.iter().enumerate() {
                    let rr = ui.allocate_exact_size(vec2(w - 20.0, 36.0), Sense::click());
                    if rr.1.hovered() {
                        sel = k;
                    }
                    if k == sel {
                        ui.painter().rect_filled(rr.0, 8.0, CARD_HOVER());
                    }
                    ui.painter().text(rr.0.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, j::title_of(r), crate::fonts::sans_font(13.5), TEXT());
                    let folder = r.rsplit_once('/').map(|(f, _)| f).unwrap_or("");
                    ui.painter().text(rr.0.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, folder, crate::fonts::sans_font(11.5), MUTED());
                    if rr.1.clicked() {
                        chosen = Some(k);
                    }
                }
                if create {
                    let k = list.len();
                    let rr = ui.allocate_exact_size(vec2(w - 20.0, 36.0), Sense::click());
                    if rr.1.hovered() {
                        sel = k;
                    }
                    if k == sel {
                        ui.painter().rect_filled(rr.0, 8.0, CARD_HOVER());
                    }
                    ui.painter().text(rr.0.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, format!("+  Make “{}”", q.trim()), crate::fonts::sans_bold_font(13.5), JOURNAL_COL);
                    if rr.1.clicked() {
                        chosen = Some(k);
                    }
                }
            });
        });
        if let Some(k) = chosen {
            self.setup.journal.switcher = None;
            match list.get(k) {
                Some(r) => {
                    let r = r.clone();
                    self.journal_open_any(&r);
                }
                None => {
                    let name = q.trim().to_string();
                    self.journal_follow(ctx, &name);
                }
            }
            return;
        }
        self.setup.journal.switcher = if close { None } else { Some((q, sel)) };
    }

    /// Settings → Journal: how notes look and where new ones go.
    pub(super) fn journal_settings_body(&mut self, ui: &mut egui::Ui) {
        let mut cfg = self.cfg.journal.clone();
        kit::section(ui, "Notes");
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Text size").size(kit::fs(13.0)).strong().color(TEXT()));
            ui.spacing_mut().slider_width = 260.0;
            ui.add(egui::Slider::new(&mut cfg.font_size, 12.0..=22.0).step_by(1.0).fixed_decimals(0).suffix(" px"));
            ui.add_space(12.0);
            kit::toggle_row(ui, "Readable line length", "Keep notes to a comfortable width on wide windows.", &mut cfg.readable);
            ui.add_space(8.0);
            kit::toggle_row(ui, "Show the right panel", "Outline, backlinks, links and tags beside the note (on wide windows).", &mut cfg.right_panel);
            ui.add_space(12.0);
            ui.label(RichText::new("Notes open in").size(kit::fs(13.0)).strong().color(TEXT()));
            let mut m = cfg.default_mode;
            kit::segmented(ui, &mut m, &[(0u8, "Live"), (1, "Source"), (2, "Reading")]);
            cfg.default_mode = m;
            ui.add_space(12.0);
            ui.label(RichText::new("Daily notes folder").size(kit::fs(13.0)).strong().color(TEXT()));
            ui.add(egui::TextEdit::singleline(&mut cfg.daily_folder).desired_width(220.0));
            ui.add_space(14.0);
            ui.label(RichText::new(format!("Your notes are plain Markdown files in:\n{}", j::root().display())).size(kit::fs(12.0)).color(MUTED()));
            if ui.button("Open the Journal folder").clicked() {
                let _ = std::fs::create_dir_all(j::root());
                crate::share::reveal(&j::root());
            }
        });
        if cfg != self.cfg.journal {
            self.cfg.journal = cfg;
            self.dirty = true;
        }
    }
}

impl Mode {
    /// Reading has no cursor: the mode to edit in instead.
    fn min_edit(self) -> Mode {
        if self == Mode::Reading {
            Mode::Live
        } else {
            self
        }
    }
}

/// A line of Markdown without its marks ("- [ ] see [[Note]]" -> "see Note").
fn plain(line: &str) -> String {
    let t = line.trim_start();
    let t = ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "> "].iter().find_map(|m| t.strip_prefix(m)).unwrap_or(t);
    let t = t.trim_start_matches('#').trim();
    md::inline(t).iter().filter(|s| !s.fx.marker).map(|s| s.text.as_str()).collect::<String>().trim().to_string()
}

fn side_title(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).size(kit::fs(12.0)).strong().color(MUTED()));
    ui.add_space(6.0);
}

/// One row of a list in the side panels, Obsidian-style: flat, with fine
/// lines showing how deep it is, a chevron for a folder, and a small tag
/// (CANVAS, PNG) for what isn't a note.
fn tree_row(ui: &mut egui::Ui, depth: usize, lead: &str, text: &str, right: &str, on: bool, col: Color32) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 5.0, ju::sel_bg());
    } else if resp.hovered() {
        p.rect_filled(r, 5.0, ju::hover_bg());
    }
    // indent guides
    for d in 0..depth {
        let gx = r.min.x + 14.0 + d as f32 * 16.0;
        p.vline(gx, r.y_range(), Stroke::new(1.0f32, LINE()));
    }
    let lx = r.min.x + 6.0 + depth as f32 * 16.0;
    let mut x = lx + 4.0;
    match lead {
        "" => {}
        "▸" | "▾" => {
            let c = pos2(lx + 8.0, r.center().y);
            let pts = if lead == "▸" { vec![c + vec2(-1.5, -4.0), c + vec2(2.5, 0.0), c + vec2(-1.5, 4.0)] } else { vec![c + vec2(-4.0, -1.5), c + vec2(0.0, 2.5), c + vec2(4.0, -1.5)] };
            p.add(egui::Shape::line(pts, Stroke::new(1.5f32, ju::icon_col())));
            x = lx + 20.0;
        }
        "✨" => {
            ju::draw(p, "sparkle", pos2(lx + 9.0, r.center().y), if on { col } else { theme::blend(ju::icon_col(), col, 0.5) });
            x = lx + 22.0;
        }
        _ => {
            p.text(pos2(lx + 8.0, r.center().y), Align2::CENTER_CENTER, lead, crate::fonts::sans_font(12.5), theme::blend(ju::icon_col(), col, 0.5));
            x = lx + 20.0;
        }
    }
    let f = crate::fonts::sans_font(13.0);
    let badge_w = if right.is_empty() { 0.0 } else { 60.0 };
    let label = super::charts::clip(ui, text, &f, r.max.x - x - 10.0 - badge_w);
    p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, label, f, if on { TEXT() } else { theme::blend(TEXT(), MUTED(), 0.18) });
    if !right.is_empty() {
        let small = right.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) && right.chars().any(|c| c.is_ascii_uppercase());
        if small {
            // a file type tag, as Obsidian's CANVAS
            let g = p.layout_no_wrap(right.to_string(), crate::fonts::sans_font(9.5), MUTED());
            let br = Rect::from_min_size(pos2(r.max.x - g.size().x - 14.0, r.center().y - g.size().y / 2.0 - 2.0), g.size() + vec2(8.0, 4.0));
            p.rect_filled(br, 4.0, theme::blend(ju::side_bg(), TEXT(), 0.06));
            p.galley(br.min + vec2(4.0, 2.0), g, MUTED());
        } else {
            p.text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, right, crate::fonts::sans_font(11.0), MUTED());
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A note's properties, as Reading shows them.
fn properties_card(ui: &mut egui::Ui, props: &[(String, String)]) {
    egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(12.0).inner_margin(egui::Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new("j-props").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
            for (k, v) in props {
                ui.label(RichText::new(k).size(kit::fs(12.5)).color(MUTED()));
                if k.eq_ignore_ascii_case("tags") {
                    ui.horizontal_wrapped(|ui| {
                        for t in v.split([',', ' ']).filter(|t| !t.trim().is_empty()) {
                            egui::Frame::none().fill(md::TAG.gamma_multiply(0.16)).rounding(10.0).inner_margin(egui::Margin::symmetric(8.0, 1.0)).show(ui, |ui| {
                                ui.label(RichText::new(format!("#{}", t.trim().trim_start_matches('#'))).size(kit::fs(12.0)).color(md::TAG));
                            });
                        }
                    });
                } else {
                    ui.label(RichText::new(v).size(kit::fs(12.5)).color(TEXT()));
                }
                ui.end_row();
            }
        });
    });
}

// ───────────────────────────── the toolbar ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Tool {
    H1,
    H2,
    H3,
    Bold,
    Italic,
    Strike,
    Mark,
    Bullet,
    Number,
    Task,
    Quote,
    Callout,
    Table,
    Link,
    Code,
    Rule,
}


/// Char index -> byte index.
fn byte_at(s: &str, ch: usize) -> usize {
    s.char_indices().nth(ch).map_or(s.len(), |(b, _)| b)
}

/// Apply a toolbar button to the text at the editor's cursor or selection,
/// leaving the cursor where you'd want it.
pub(super) fn apply_tool(ctx: &egui::Context, id: egui::Id, text: &mut String, t: Tool) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let n = text.chars().count();
    let (a, b) = state.cursor.char_range().map_or((n, n), |r| {
        let (x, y) = (r.primary.index.min(n), r.secondary.index.min(n));
        (x.min(y), x.max(y))
    });
    let (na, nb) = edit_text(text, a, b, t);
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(na), CCursor::new(nb))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// The edit itself (char positions in and out), kept apart so it's testable.
pub(super) fn edit_text(text: &mut String, a: usize, b: usize, t: Tool) -> (usize, usize) {
    let wrap = |text: &mut String, open: &str, close: &str| -> (usize, usize) {
        let (ba, bb) = (byte_at(text, a), byte_at(text, b));
        // already wrapped: unwrap
        let ol = open.chars().count();
        if ba >= open.len() && text[..ba].ends_with(open) && text[bb..].starts_with(close) {
            text.replace_range(bb..bb + close.len(), "");
            text.replace_range(ba - open.len()..ba, "");
            return (a - ol, b - ol);
        }
        text.insert_str(bb, close);
        text.insert_str(ba, open);
        (a + ol, b + ol)
    };
    match t {
        Tool::Bold => wrap(text, "**", "**"),
        Tool::Italic => wrap(text, "*", "*"),
        Tool::Strike => wrap(text, "~~", "~~"),
        Tool::Mark => wrap(text, "==", "=="),
        Tool::Link => wrap(text, "[[", "]]"),
        Tool::Code => wrap(text, "`", "`"),
        Tool::H1 | Tool::H2 | Tool::H3 | Tool::Bullet | Tool::Number | Tool::Task | Tool::Quote => {
            let want = match t {
                Tool::H1 => "# ",
                Tool::H2 => "## ",
                Tool::H3 => "### ",
                Tool::Bullet => "- ",
                Tool::Number => "1. ",
                Tool::Task => "- [ ] ",
                _ => "> ",
            };
            // the lines the selection touches
            let ba = byte_at(text, a);
            let bb = byte_at(text, b);
            let start = text[..ba].rfind('\n').map_or(0, |i| i + 1);
            let end = text[bb..].find('\n').map_or(text.len(), |i| bb + i);
            let block = text[start..end].to_string();
            let lines: Vec<&str> = block.split('\n').collect();
            let all_have = lines.iter().all(|l| l.starts_with(want));
            let mut out = Vec::new();
            for l in &lines {
                let bare = strip_prefix_any(l);
                out.push(if all_have { bare.to_string() } else { format!("{want}{bare}") });
            }
            let new_block = out.join("\n");
            let before = text[..start].chars().count();
            text.replace_range(start..end, &new_block);
            let len = new_block.chars().count();
            if lines.len() == 1 {
                let p = before + len;
                (p, p)
            } else {
                (before, before + len)
            }
        }
        Tool::Callout | Tool::Table | Tool::Rule => {
            let snippet = match t {
                Tool::Callout => "> [!tip] Tip\n> ",
                Tool::Table => "| Column | Column |\n| --- | --- |\n|  |  |",
                _ => "---",
            };
            let bb = byte_at(text, b);
            // on a line of its own
            let at_line_start = bb == 0 || text[..bb].ends_with('\n');
            let pre = if at_line_start { "" } else { "\n" };
            let ins = format!("{pre}{snippet}\n");
            text.insert_str(bb, &ins);
            let p = b + pre.chars().count() + snippet.chars().count();
            (p, p)
        }
    }
}

/// A line without its heading, list, task or quote mark.
fn strip_prefix_any(l: &str) -> &str {
    for m in ["- [ ] ", "- [x] ", "- [X] ", "###### ", "##### ", "#### ", "### ", "## ", "# ", "- ", "* ", "> "] {
        if let Some(r) = l.strip_prefix(m) {
            return r;
        }
    }
    let d = l.chars().take_while(|c| c.is_ascii_digit()).count();
    if d > 0 && l[d..].starts_with(". ") {
        return &l[d + 2..];
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_wraps_and_unwraps() {
        let mut t = "rare grass today".to_string();
        let (a, b) = edit_text(&mut t, 0, 4, Tool::Bold);
        assert_eq!(t, "**rare** grass today");
        assert_eq!((a, b), (2, 6));
        let (a, b) = edit_text(&mut t, a, b, Tool::Bold);
        assert_eq!(t, "rare grass today");
        assert_eq!((a, b), (0, 4));
        let mut t = "é x".to_string();
        edit_text(&mut t, 0, 1, Tool::Mark);
        assert_eq!(t, "==é== x");
    }

    #[test]
    fn toolbar_line_marks() {
        let mut t = "one\ntwo\nthree".to_string();
        edit_text(&mut t, 0, 5, Tool::Task);
        assert_eq!(t, "- [ ] one\n- [ ] two\nthree");
        edit_text(&mut t, 0, 12, Tool::Task);
        assert_eq!(t, "one\ntwo\nthree");
        let mut t = "## Old".to_string();
        edit_text(&mut t, 3, 3, Tool::H1);
        assert_eq!(t, "# Old");
        let mut t = "text".to_string();
        let p = edit_text(&mut t, 4, 4, Tool::Callout);
        assert_eq!(t, "text\n> [!tip] Tip\n> \n");
        assert_eq!(p.0, "text\n> [!tip] Tip\n> ".chars().count());
    }
}
