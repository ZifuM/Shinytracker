//! Game Info: everything in Platinum Together (Pokémon, moves, items,
//! places and the how-to-play guides) from the game's own wiki data, laid out
//! for the app. This is its front page: one search over all of it, the
//! game's news, where your current target lives, and ways in.

use super::pokedex::{type_chip, type_color};
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::info_guides::guide_icon;
use super::{App, Tab};
use crate::theme;
use crate::wiki::{self, Mon, Wiki, Wild};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a search result is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub(super) enum Kind {
    Pokemon,
    Move,
    Item,
    Place,
    Guide,
}

const KINDS: [Kind; 5] = [Kind::Pokemon, Kind::Move, Kind::Item, Kind::Place, Kind::Guide];

impl Kind {
    fn icon(self) -> &'static str {
        match self {
            Kind::Pokemon => "📖",
            Kind::Move => "💥",
            Kind::Item => "🎒",
            Kind::Place => "🗺",
            Kind::Guide => "📚",
        }
    }
    fn color(self) -> Color32 {
        match self {
            Kind::Pokemon => kit::C_RED,
            Kind::Move => kit::C_VIOLET,
            Kind::Item => kit::C_GOLD,
            Kind::Place => kit::C_GREEN,
            Kind::Guide => kit::C_CYAN,
        }
    }
    fn plural(self) -> &'static str {
        match self {
            Kind::Pokemon => "Pokémon",
            Kind::Move => "Moves",
            Kind::Item => "Items",
            Kind::Place => "Places",
            Kind::Guide => "Guides",
        }
    }
    fn one(self) -> &'static str {
        match self {
            Kind::Pokemon => "Pokémon",
            Kind::Move => "move",
            Kind::Item => "item",
            Kind::Place => "place",
            Kind::Guide => "guide",
        }
    }
}

/// One searchable thing.
#[derive(Clone)]
struct Hit {
    kind: Kind,
    /// Pokémon / item id, move key, place group name, guide id.
    key: String,
    title: String,
    sub: String,
    /// lower-case title, and lower-case extra words to match on
    hay: String,
    more: String,
    /// Pokémon dex number (for "#25")
    dex: u32,
}

/// Which Game Info page is open.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) enum Page {
    #[default]
    Home,
    /// How to Play: the guides library.
    Guides,
    /// One guide, by id.
    Guide(String),
    /// Every move.
    Moves,
    /// One move, by key.
    Move(String),
    /// Every item.
    Items,
    /// One item, by id.
    Item(String),
    /// Every place, in story order.
    Places,
    /// One place, by name ("Route 210").
    Place(String),
}

impl Page {
    /// A pin's key: "move:surf", "item:ITEM_LEFTOVERS", "place:Route 210", "guide:saving".
    pub fn key(&self) -> Option<String> {
        match self {
            Page::Move(k) => Some(format!("move:{k}")),
            Page::Item(k) => Some(format!("item:{k}")),
            Page::Place(k) => Some(format!("place:{k}")),
            Page::Guide(k) => Some(format!("guide:{k}")),
            _ => None,
        }
    }

    pub fn from_key(s: &str) -> Option<Page> {
        let (kind, k) = s.split_once(':')?;
        let k = k.to_string();
        Some(match kind {
            "move" => Page::Move(k),
            "item" => Page::Item(k),
            "place" => Page::Place(k),
            "guide" => Page::Guide(k),
            _ => return None,
        })
    }
}

#[derive(Default)]
pub(super) struct InfoState {
    pub page: Page,
    /// Pages to go back to.
    pub history: Vec<Page>,
    /// The guides library and reader.
    pub reader: super::info_guides::ReaderState,
    /// The moves list and move pages.
    pub moves: super::info_moves::MovesState,
    /// The items lists and item pages.
    pub items: super::info_items::ItemsState,
    /// The places list and place pages.
    pub places: super::info_places::PlacesState,
    /// Pages looked at lately, newest first.
    pub recent: Vec<Page>,
    /// The app page Game Info was opened from (the Pokédex…), for "Back".
    pub from_tab: Option<Tab>,
    /// What changed since the last wiki copy you saw.
    pub fresh: super::info_story::FreshState,
    pub query: String,
    /// Only this kind in the results; with no query, list all of it.
    pub kind: Option<Kind>,
    /// The result shown in the quick look.
    pub pick: Option<(Kind, String)>,
    /// Guides card: open topic.
    pub topic: usize,
    /// Search index, built per wiki copy (keyed by the Arc's address).
    index: Option<(usize, Vec<Hit>)>,
    /// Keep the results list's selected row in view after arrow keys.
    scroll_pick: bool,
}

impl InfoState {
    fn browsing(&self) -> bool {
        !self.query.trim().is_empty() || self.kind.is_some()
    }

    /// Back to the front page with nothing searched (clicking Game Info in
    /// the sidebar while you're in it).
    pub fn reset_home(&mut self) {
        self.page = Page::Home;
        self.history.clear();
        self.from_tab = None;
        self.query.clear();
        self.kind = None;
        self.pick = None;
    }
}

/// Places grouped by name ("Route 210" covers its north and south parts).
fn place_groups(w: &Wiki) -> BTreeMap<String, Vec<String>> {
    let mut g: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, p) in &w.data.places {
        let name = if p.group.is_empty() { p.name.clone() } else { p.group.clone() };
        g.entry(name).or_default().push(k.clone());
    }
    g
}

fn build_index(w: &Wiki) -> Vec<Hit> {
    let mut out = Vec::new();
    for &i in &w.order {
        let m = &w.data.species[i];
        out.push(Hit {
            kind: Kind::Pokemon,
            key: m.id.clone(),
            title: m.name.clone(),
            sub: format!("#{:04} · {}", m.dex, m.types.join(" / ")),
            hay: m.name.to_lowercase(),
            more: m.types.join(" ").to_lowercase(),
            dex: m.dex,
        });
    }
    let mut moves: Vec<(&String, &wiki::Move)> = w.data.moves.iter().collect();
    moves.sort_by(|a, b| a.1.n.cmp(&b.1.n));
    for (k, m) in moves {
        out.push(Hit {
            kind: Kind::Move,
            key: k.clone(),
            title: m.n.clone(),
            sub: format!("{} · {}", m.t, m.c),
            hay: m.n.to_lowercase(),
            more: format!("{} {}", m.t, m.c).to_lowercase(),
            dex: 0,
        });
    }
    let mut items: Vec<&wiki::Item> = w.data.items.iter().collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    for it in items {
        out.push(Hit {
            kind: Kind::Item,
            key: it.id.clone(),
            title: it.name.clone(),
            sub: it.pocket.clone(),
            hay: it.name.to_lowercase(),
            more: it.pocket.to_lowercase(),
            dex: 0,
        });
    }
    for (name, keys) in place_groups(w) {
        let stage = keys
            .iter()
            .filter_map(|k| w.place(k))
            .map(|p| p.stage.clone())
            .min_by_key(|s| w.stage_index(s))
            .unwrap_or_default();
        let parts = if keys.len() > 1 { format!(" · {} areas", keys.len()) } else { String::new() };
        out.push(Hit {
            kind: Kind::Place,
            key: name.clone(),
            title: name.clone(),
            sub: format!("{stage}{parts}"),
            hay: name.to_lowercase(),
            more: String::new(),
            dex: 0,
        });
    }
    for g in w.guides_sorted() {
        out.push(Hit {
            kind: Kind::Guide,
            key: g.id.clone(),
            title: g.title.clone(),
            sub: g.group.clone(),
            hay: g.title.to_lowercase(),
            more: format!("{} {}", g.short, g.summary).to_lowercase(),
            dex: 0,
        });
    }
    out
}

/// How well a thing matches (lower is better), or None.
fn score(h: &Hit, q: &str) -> Option<u32> {
    if q.is_empty() {
        return Some(0);
    }
    let num = q.trim_start_matches('#');
    if h.kind == Kind::Pokemon && !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
        return (num.parse::<u32>().ok() == Some(h.dex)).then_some(0);
    }
    if h.hay == q {
        Some(0)
    } else if h.hay.starts_with(q) {
        Some(1)
    } else if h.hay.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(q)) {
        Some(2)
    } else if h.hay.contains(q) {
        Some(3)
    } else if h.more.split(|c: char| !c.is_alphanumeric()).any(|w| w == q) {
        Some(4)
    } else if q.len() >= 3 && h.more.contains(q) {
        Some(5)
    } else {
        None
    }
}

/// Platinum's clock: morning 4–10, day 10–20, night 20–4 (0, 1, 2).
fn period_now() -> usize {
    match crate::stats::local_hour(crate::stats::now_unix()) {
        4..=9 => 0,
        10..=19 => 1,
        _ => 2,
    }
}

const PERIODS: [&str; 3] = ["Morning", "Day", "Night"];

fn chance_at(e: &Wild, p: usize) -> u32 {
    [e.chance.morning, e.chance.day, e.chance.night][p]
}

/// (year, month, day) for days since 1970-01-01.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

const MONTHS: [&str; 12] = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];

/// "… before 27 November." → days left until then (negative once it's over).
fn deadline_days(text: &str) -> Option<i64> {
    let lower = text.to_lowercase();
    let at = lower.find("before ").or_else(|| lower.find("until "))?;
    let rest = &lower[at..];
    let mut words = rest.split(|c: char| c.is_whitespace() || c == '.' || c == ',').filter(|w| !w.is_empty()).skip(1);
    let day: u32 = words.next()?.parse().ok()?;
    let mw = words.next()?;
    let month = MONTHS.iter().position(|m| *m == mw)? as u32 + 1;
    let today = crate::stats::local_day(crate::stats::now_unix());
    let (y, _, _) = civil(today);
    let mut end = days_from_civil(y, month, day);
    // a date long past is next year's
    if end < today - 180 {
        end = days_from_civil(y + 1, month, day);
    }
    Some(end - today)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", fmt_n(n), if n == 1 { one } else { many })
}

fn fmt_n(n: usize) -> String {
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

impl App {
    pub(super) fn page_info(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone().filter(|w| !w.is_empty()) else {
            kit::page_header(ui, "GAME INFO", "Everything in Platinum Together, from the game's own data.");
            kit::card(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Getting the game's data…").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.label(small("It comes from the Platinum Together wiki. This needs the internet the first time."));
                ui.add_space(6.0);
            });
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
            return;
        };
        let key = Arc::as_ptr(&w) as usize;
        if self.setup.info.index.as_ref().map(|i| i.0) != Some(key) {
            self.setup.info.index = Some((key, build_index(&w)));
        }
        // keys: Alt+Left goes back
        if ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) && !self.setup.info.history.is_empty() {
            self.info_back();
        }
        match self.setup.info.page.clone() {
            Page::Guides => return self.page_guides(ui, &w),
            Page::Guide(id) => return self.page_guide(ui, &w, &id),
            Page::Moves => return self.page_moves(ui, &w),
            Page::Move(k) => return self.page_move(ui, &w, &k),
            Page::Items => return self.page_items(ui, &w),
            Page::Item(id) => return self.page_item(ui, &w, &id),
            Page::Places => return self.page_places(ui, &w),
            Page::Place(n) => return self.page_place(ui, &w, &n),
            Page::Home => {}
        }
        kit::page_header(
            ui,
            "GAME INFO",
            &format!("Platinum Together {}: what's in the game, where to find it and how it works.", w.data.version),
        );
        self.info_search_hero(ui, &w);
        ui.add_space(18.0);
        if self.setup.info.browsing() {
            self.info_results(ui, &w);
        } else {
            self.info_front(ui, &w);
        }
        ui.add_space(18.0);
        self.info_source(ui, &w);
    }

    // ───────────────────────────── search ─────────────────────────────

    fn info_search_hero(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let pl = theme::pal();
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let counts = [
            (Kind::Pokemon, w.data.species.len()),
            (Kind::Move, w.data.moves.len()),
            (Kind::Item, w.data.items.len()),
            (Kind::Place, place_groups(w).len()),
            (Kind::Guide, w.data.guides.len()),
        ];
        let mut pick_kind: Option<Kind> = None;
        // while looking things up the card shrinks to the box and the chips
        let compact = self.setup.info.browsing();
        let pad = if compact { 16.0 } else { 22.0 };
        let frame = egui::Frame::none()
            .inner_margin(egui::Margin { left: 28.0, right: 28.0, top: pad, bottom: pad })
            .show(ui, |ui| {
                ui.set_width(full - 56.0);
                if !compact {
                    ui.label(
                        RichText::new(format!("SEARCH THE GAME  ·  v{}", w.data.version))
                            .font(crate::fonts::sans_bold_font(11.0))
                            .color(theme::blend(MUTED(), pl.accent, 0.5)),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("Look anything up").font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                    ui.add_space(2.0);
                    ui.label(RichText::new("Pokémon, moves, items, places and guides. Type a name, a type or a dex number.").size(13.0).color(MUTED()));
                    ui.add_space(14.0);
                }
                self.info_search_box(ui, w);
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    for (k, n) in counts {
                        let on = self.setup.info.kind == Some(k);
                        let label = format!("{}  {} {}", k.icon(), fmt_n(n), k.plural());
                        let g = ui.painter().layout_no_wrap(label, crate::fonts::sans_bold_font(12.0), TEXT());
                        let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(22.0, 12.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || on, kit::QUICK);
                        let fill = theme::blend(theme::blend(CARD(), pl.accent, 0.10), k.color(), 0.10 + 0.18 * ht);
                        ui.painter().rect_filled(r, r.height() / 2.0, fill);
                        ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, theme::blend(LINE(), k.color(), 0.25 + 0.5 * ht)));
                        ui.painter().galley(r.min + egui::vec2(11.0, 6.0), g, TEXT());
                        let tip = match k {
                            Kind::Pokemon => "Open the Pokédex".to_string(),
                            Kind::Guide => "Open How to Play".to_string(),
                            Kind::Move => "Open Moves".to_string(),
                            Kind::Item => "Open Items".to_string(),
                            Kind::Place => "Open Places".to_string(),
                        };
                        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            pick_kind = Some(k);
                        }
                    }
                });
            });
        let rect = frame.response.rect;
        let a = theme::blend(CARD(), pl.accent, if pl.light { 0.16 } else { 0.30 });
        let b = theme::blend(CARD(), if pl.gradient { pl.accent2 } else { pl.accent }, if pl.light { 0.08 } else { 0.12 });
        let mut shapes = vec![
            kit::card_shadow().as_shape(rect, 18.0).into(),
            theme::gradient_rect(rect, [18.0; 4], a, b),
            egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), pl.accent, 0.35))),
        ];
        // a few soft rings at the right, like the hunt card's glow
        if full >= 700.0 && !compact {
            let c = egui::pos2(rect.max.x - 120.0, rect.min.y + 70.0);
            for (i, r) in [44.0f32, 78.0, 112.0].iter().enumerate() {
                shapes.push(egui::Shape::circle_stroke(c, *r, Stroke::new(1.0f32, kit::tint(pl.accent, 0.20 - i as f32 * 0.05))));
            }
        }
        ui.painter().set(slot, egui::Shape::Vec(shapes));
        if full >= 700.0 && !compact {
            let p = ui.painter_at(rect);
            let c = egui::pos2(rect.max.x - 120.0, rect.min.y + 70.0);
            theme::sparkle(&p, c + egui::vec2(-58.0, -26.0), 6.0, theme::GOLD);
            theme::sparkle(&p, c + egui::vec2(64.0, 18.0), 4.0, theme::GOLD);
            theme::sparkle(&p, c + egui::vec2(10.0, 52.0), 3.5, theme::GOLD);
            p.text(c, Align2::CENTER_CENTER, "📚", egui::FontId::proportional(40.0), Color32::WHITE);
        }
        if let Some(k) = pick_kind {
            if k == Kind::Pokemon {
                self.info_open_dex(None);
            } else if k == Kind::Guide && self.setup.info.query.trim().is_empty() {
                self.info_push(Page::Guides);
            } else if k == Kind::Move && self.setup.info.query.trim().is_empty() {
                self.info_push(Page::Moves);
            } else if k == Kind::Item && self.setup.info.query.trim().is_empty() {
                self.info_push(Page::Items);
            } else if k == Kind::Place && self.setup.info.query.trim().is_empty() {
                self.info_push(Page::Places);
            } else {
                let s = &mut self.setup.info;
                if s.kind == Some(k) && s.query.trim().is_empty() {
                    s.kind = None;
                } else {
                    s.kind = Some(k);
                    s.pick = None;
                }
            }
        }
    }

    fn info_search_box(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        // keys first (never inside a data closure)
        let (up, down, enter, esc) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        let width = ui.available_width();
        let before = self.setup.info.query.clone();
        // own id, so the box keeps focus when the card shrinks around it
        let resp = kit::search_box_keyed(
            ui,
            Some(egui::Id::new("game-info-search")),
            &mut self.setup.info.query,
            "Search, e.g. Scyther, Surf, Leftovers, Route 210, Megas",
            width,
        );
        if self.setup.info.query != before {
            self.setup.info.pick = None;
        }
        let focused = resp.has_focus() || resp.lost_focus();
        if !focused {
            return;
        }
        if esc {
            self.setup.info.query.clear();
            self.setup.info.kind = None;
            self.setup.info.pick = None;
            return;
        }
        let hits = self.info_hits(w);
        if hits.is_empty() {
            return;
        }
        let cur = self.info_current(&hits);
        let i = hits.iter().position(|h| (h.kind, h.key.as_str()) == (cur.0, cur.1.as_str())).unwrap_or(0);
        if up || down {
            let j = if down { (i + 1).min(hits.len() - 1) } else { i.saturating_sub(1) };
            self.setup.info.pick = Some((hits[j].kind, hits[j].key.clone()));
            self.setup.info.scroll_pick = true;
            resp.request_focus();
        }
        if enter && resp.lost_focus() {
            let h = &hits[i];
            if h.kind == Kind::Pokemon {
                let id = h.key.clone();
                self.info_open_dex(Some(id));
            } else if h.kind == Kind::Move {
                let k = h.key.clone();
                self.info_push(Page::Move(k));
            } else if h.kind == Kind::Item {
                let k = h.key.clone();
                self.info_push(Page::Item(k));
            } else if h.kind == Kind::Place {
                let k = h.key.clone();
                self.info_push(Page::Place(k));
            } else if h.kind == Kind::Guide {
                let id = h.key.clone();
                self.info_read(&id, None, None);
            } else {
                self.setup.info.pick = Some((h.kind, h.key.clone()));
            }
        }
    }

    /// Results for the query and kind filter, best first, grouped by kind.
    fn info_hits(&self, _w: &Wiki) -> Vec<Hit> {
        let s = &self.setup.info;
        let q = s.query.trim().to_lowercase();
        let Some((_, idx)) = s.index.as_ref() else { return Vec::new() };
        let mut out: Vec<(Kind, u32, usize)> = idx
            .iter()
            .enumerate()
            .filter(|(_, h)| s.kind.map_or(true, |k| k == h.kind))
            .filter_map(|(i, h)| score(h, &q).map(|sc| (h.kind, sc, i)))
            .collect();
        // kinds with the closest match first, then best match first in each
        // (shorter names first among equal matches: "Surf" before "Surskit")
        let key = |sc: u32, i: usize| (sc, idx[i].title.chars().count());
        let mut best: BTreeMap<Kind, (u32, usize)> = BTreeMap::new();
        for (k, sc, i) in &out {
            let b = best.entry(*k).or_insert((u32::MAX, usize::MAX));
            *b = (*b).min(key(*sc, *i));
        }
        if q.is_empty() {
            out.sort_by_key(|(k, _, i)| (*k, *i));
        } else {
            out.sort_by_key(|(k, sc, i)| (best[k], *k, key(*sc, *i), *i));
        }
        out.into_iter().map(|(_, _, i)| idx[i].clone()).collect()
    }

    /// The picked result, or the first one.
    fn info_current(&self, hits: &[Hit]) -> (Kind, String) {
        match &self.setup.info.pick {
            Some(p) if hits.iter().any(|h| h.kind == p.0 && h.key == p.1) => p.clone(),
            Some(p) if self.setup.info.query.trim().is_empty() && self.setup.info.kind.is_none() => p.clone(),
            _ => hits.first().map(|h| (h.kind, h.key.clone())).unwrap_or((Kind::Pokemon, String::new())),
        }
    }

    /// Go to a page, remembering this one for "Back".
    pub(super) fn info_push(&mut self, page: Page) {
        let s = &mut self.setup.info;
        if page.key().is_some() {
            s.recent.retain(|p| *p != page);
            s.recent.insert(0, page.clone());
            s.recent.truncate(12);
        }
        if s.page != page {
            let cur = std::mem::replace(&mut s.page, page);
            // coming back to a page you were on: drop the loop
            if let Some(i) = s.history.iter().position(|p| *p == s.page) {
                s.history.truncate(i);
            } else {
                s.history.push(cur);
            }
            if s.history.len() > 30 {
                s.history.remove(0);
            }
        }
        self.setup.scroll_top = true;
    }

    pub(super) fn info_back(&mut self) {
        let s = &mut self.setup.info;
        if s.history.is_empty() {
            if let Some(t) = s.from_tab.take() {
                s.page = Page::Home;
                self.go_to(t);
                return;
            }
        }
        s.page = s.history.pop().unwrap_or_default();
        s.reader.scroll_to = None;
        self.setup.scroll_top = true;
    }

    /// Open a Game Info page from anywhere in the app.
    pub(super) fn info_open_page(&mut self, page: Page) {
        if self.setup.tab != Tab::Info {
            // "Back" takes you to where you were (the Pokédex entry…)
            let from = self.setup.tab;
            self.go_to(Tab::Info);
            let s = &mut self.setup.info;
            s.history.clear();
            s.from_tab = Some(from);
            s.page = Page::Home;
        }
        self.info_push(page);
        // its Back goes to the app page, not the Game Info front page
        self.setup.info.history.retain(|p| *p != Page::Home);
    }

    /// Show one thing in the search panel.
    #[allow(dead_code)]
    pub(super) fn info_show(&mut self, kind: Kind, key: String) {
        self.info_push(Page::Home);
        let s = &mut self.setup.info;
        s.query.clear();
        s.kind = Some(kind);
        s.pick = Some((kind, key));
        s.scroll_pick = true;
    }

    pub(super) fn info_open_dex(&mut self, id: Option<String>) {
        self.setup.pokedex.shiny = false;
        self.setup.pokedex.open = id;
        self.setup.pokedex.tab = Default::default();
        self.go_to(Tab::Pokedex);
    }

    fn info_results(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let hits = self.info_hits(w);
        let q = self.setup.info.query.trim().to_string();
        let mut per: BTreeMap<Kind, usize> = BTreeMap::new();
        {
            let s = &self.setup.info;
            let ql = q.to_lowercase();
            if let Some((_, idx)) = s.index.as_ref() {
                for h in idx {
                    if score(h, &ql).is_some() {
                        *per.entry(h.kind).or_default() += 1;
                    }
                }
            }
        }
        // the bar: back, and what's shown
        let mut back = false;
        let mut set_kind: Option<Option<Kind>> = None;
        ui.horizontal(|ui| {
            let back_label = match self.setup.info.history.last() {
                Some(Page::Guide(id)) => w.guide(id).map(|g| if g.short.is_empty() { g.title.clone() } else { g.short.clone() }).unwrap_or_else(|| "Back".into()),
                Some(Page::Guides) => "How to Play".into(),
                _ => "Game Info".into(),
            };
            if ui.add(egui::Button::new(RichText::new(format!("‹  {back_label}")).size(13.0)).fill(kit::btn_fill())).clicked() {
                back = true;
            }
            ui.add_space(8.0);
            let total: usize = per.values().sum();
            let all_on = self.setup.info.kind.is_none();
            if !q.is_empty() && filter_chip(ui, &format!("All  {}", fmt_n(total)), all_on, kit::C_SLATE) {
                set_kind = Some(None);
            }
            for k in KINDS {
                let n = per.get(&k).copied().unwrap_or(0);
                if n == 0 && self.setup.info.kind != Some(k) {
                    continue;
                }
                if filter_chip(ui, &format!("{}  {}  {}", k.icon(), k.plural(), fmt_n(n)), self.setup.info.kind == Some(k), k.color()) {
                    set_kind = Some(Some(k));
                }
            }
        });
        if back {
            let s = &mut self.setup.info;
            s.query.clear();
            s.kind = None;
            s.pick = None;
            if !s.history.is_empty() {
                self.info_back();
            }
            return;
        }
        if let Some(k) = set_kind {
            self.setup.info.kind = k;
            self.setup.info.pick = None;
        }
        ui.add_space(12.0);
        if hits.is_empty() {
            kit::card(ui, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Nothing called \"{q}\"")).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.label(small("Try part of a name (\"scy\"), a type (\"dragon\"), a pocket (\"berries\") or a dex number (\"#123\")."));
                ui.add_space(8.0);
            });
            return;
        }
        let cur = self.info_current(&hits);
        let avail = ui.available_width();
        let wide = avail >= 900.0;
        let gap = 18.0;
        let list_w = if wide { ((avail - gap) * 0.42).floor() } else { avail };
        let mut picked: Option<(Kind, String)> = None;
        let mut open_dex: Option<String> = None;
        let mut list = |app: &mut App, ui: &mut egui::Ui| {
            kit::card(ui, |ui| {
                let grouped = app.setup.info.kind.is_none();
                let cap = if grouped { 5 } else { usize::MAX };
                let mut shown: BTreeMap<Kind, usize> = BTreeMap::new();
                let scroll_pick = std::mem::take(&mut app.setup.info.scroll_pick);
                // tall enough for what's listed, up to a cap
                let rows: usize = if grouped { per.values().map(|n| (*n).min(cap) + 1).sum::<usize>() } else { hits.len() };
                let groups = if grouped { per.len() } else { 0 };
                let max_h = if wide { 560.0 } else { 420.0 };
                let est = (rows as f32 * 48.0 + groups as f32 * 30.0).min(max_h);
                egui::ScrollArea::vertical()
                    .id_salt(("info-results", app.setup.info.kind, q.clone()))
                    .max_height(max_h)
                    .min_scrolled_height(est)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let mut last: Option<Kind> = None;
                        for h in &hits {
                            let n = shown.entry(h.kind).or_default();
                            if *n >= cap {
                                if *n == cap {
                                    *n += 1;
                                    let more = per.get(&h.kind).copied().unwrap_or(0) - cap;
                                    if ui.link(RichText::new(format!("Show all {} {} ›", fmt_n(cap + more), h.kind.plural().to_lowercase())).size(12.5)).clicked() {
                                        set_kind_later(&mut app.setup.info, h.kind);
                                    }
                                    ui.add_space(4.0);
                                }
                                continue;
                            }
                            *n += 1;
                            if grouped && last != Some(h.kind) {
                                if last.is_some() {
                                    ui.add_space(8.0);
                                }
                                ui.label(RichText::new(h.kind.plural().to_uppercase()).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
                                ui.add_space(2.0);
                                last = Some(h.kind);
                            }
                            let on = cur.0 == h.kind && cur.1 == h.key;
                            let resp = app.hit_row(ui, w, h, on);
                            if on && scroll_pick {
                                resp.scroll_to_me(Some(egui::Align::Center));
                            }
                            if resp.double_clicked() && h.kind == Kind::Pokemon {
                                open_dex = Some(h.key.clone());
                            } else if resp.clicked() {
                                picked = Some((h.kind, h.key.clone()));
                            }
                        }
                    });
            });
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(list_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(list_w);
                    list(self, ui);
                });
                let rw = ui.available_width();
                ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(rw);
                    self.info_quick_look(ui, w, &cur);
                });
            });
        } else {
            list(self, ui);
            ui.add_space(gap);
            self.info_quick_look(ui, w, &cur);
        }
        if let Some(p) = picked {
            self.setup.info.pick = Some(p);
        }
        if let Some(id) = open_dex {
            self.info_open_dex(Some(id));
        }
    }

    /// A result row: icon (or sprite), name, what it is.
    fn hit_row(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>, h: &Hit, on: bool) -> egui::Response {
        // room on the right for the scroll bar
        let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width() - 12.0, 46.0), Sense::click());
        if !ui.is_rect_visible(r) {
            return resp;
        }
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let p = ui.painter_at(r.expand(2.0));
        if on {
            p.rect_filled(r, 10.0, theme::blend(CARD(), h.kind.color(), 0.14));
            p.rect_stroke(r, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), h.kind.color(), 0.45)));
        } else if ht > 0.0 {
            p.rect_filled(r, 10.0, theme::blend(CARD(), CARD_HOVER(), ht));
        }
        let ic = egui::pos2(r.min.x + 24.0, r.center().y);
        let mut drew = false;
        if h.kind == Kind::Pokemon {
            if let Some(sp) = self.info_species(&h.title) {
                p.circle_filled(ic, 17.0, theme::blend(CARD(), type_color(h.sub.split(" · ").nth(1).and_then(|t| t.split(" / ").next()).unwrap_or("")), 0.22));
                drew = self.info_sprite(&p, &sp, ic, egui::vec2(34.0, 34.0));
            }
        }
        if !drew {
            // moves wear their type's colour
            let col = match h.kind {
                Kind::Move => type_color(h.sub.split(" · ").next().unwrap_or("")),
                k => k.color(),
            };
            kit::icon_chip(&p, Rect::from_center_size(ic, egui::vec2(30.0, 30.0)), h.kind.icon(), col, false);
        }
        let x = r.min.x + 50.0;
        let maxw = r.max.x - x - 10.0;
        let f = crate::fonts::sans_bold_font(13.5);
        p.text(egui::pos2(x, r.center().y - 8.0), Align2::LEFT_CENTER, super::charts::clip(ui, &h.title, &f, maxw), f, TEXT());
        let f2 = crate::fonts::sans_font(11.5);
        p.text(egui::pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, &h.sub, &f2, maxw), f2, MUTED());
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Draw a sprite only once it's loaded (no "loading" text in small spots).
    fn info_sprite(&mut self, p: &egui::Painter, sp: &crate::species::Species, center: egui::Pos2, max: egui::Vec2) -> bool {
        if self.sprites.get(sp, false).is_none() {
            return false;
        }
        self.draw_sprite_centered(p, sp, false, center, max)
    }

    /// The app's species for a wiki name (for sprites).
    pub(super) fn info_species(&self, name: &str) -> Option<crate::species::Species> {
        let want = wiki::slug(name);
        self.species.iter().find(|s| wiki::slug(s.name) == want).cloned()
    }

    // ───────────────────────────── quick look ─────────────────────────────

    fn info_quick_look(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, cur: &(Kind, String)) {
        let mut go: Option<(Kind, String)> = None;
        let mut dex: Option<String> = None;
        let mut read: Option<(String, Option<usize>)> = None;
        let mut open_move: Option<String> = None;
        let mut open_item: Option<String> = None;
        let mut open_place: Option<String> = None;
        kit::card(ui, |ui| match cur.0 {
            Kind::Pokemon => {
                if let Some(m) = w.mon(&cur.1).cloned() {
                    self.look_mon(ui, w, &m, &mut go, &mut dex);
                }
            }
            Kind::Move => {
                look_move(ui, w, &cur.1, &mut go);
                ui.add_space(12.0);
                if ui.add(egui::Button::new(RichText::new("Open the move's page  ›").size(13.0)).fill(kit::btn_fill())).on_hover_text("Who learns it and how, what it hits, and moves like it").clicked() {
                    open_move = Some(cur.1.clone());
                }
            }
            Kind::Item => {
                look_item(ui, w, &cur.1, &mut go);
                ui.add_space(12.0);
                if ui.add(egui::Button::new(RichText::new("Open the item's page  ›").size(13.0)).fill(kit::btn_fill())).on_hover_text("Every way to get it, prices, and what it's for").clicked() {
                    open_item = Some(cur.1.clone());
                }
            }
            Kind::Place => {
                self.look_place(ui, w, &cur.1, &mut go);
                ui.add_space(12.0);
                if ui.add(egui::Button::new(RichText::new("Open the place's page  ›").size(13.0)).fill(kit::btn_fill())).on_hover_text("Every Pokémon by method and time of day, its items, and the Map").clicked() {
                    open_place = Some(cur.1.clone());
                }
            }
            Kind::Guide => {
                if let Some(doc) = self.guide_doc(w, &cur.1) {
                    read = look_guide(ui, w, &cur.1, &doc);
                }
            }
        });
        if let Some((id, sec)) = read {
            self.info_read(&id, sec, None);
        }
        if let Some(k) = open_move {
            self.info_push(Page::Move(k));
        }
        if let Some(k) = open_item {
            self.info_push(Page::Item(k));
        }
        if let Some(k) = open_place {
            self.info_push(Page::Place(k));
        }
        if let Some(p) = go {
            // jump to it: make sure it's in the list
            let s = &mut self.setup.info;
            s.query.clear();
            s.kind = Some(p.0);
            s.pick = Some(p);
            s.scroll_pick = true;
        }
        if let Some(id) = dex {
            self.info_open_dex(Some(id));
        }
    }

    fn look_mon(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &Mon, go: &mut Option<(Kind, String)>, dex: &mut Option<String>) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), Sense::hover());
            let col = type_color(m.types.first().map(String::as_str).unwrap_or(""));
            ui.painter().circle_filled(r.center(), 36.0, theme::blend(CARD(), col, 0.20));
            if let Some(sp) = self.info_species(&m.name) {
                self.info_sprite(ui.painter(), &sp, r.center(), egui::vec2(64.0, 64.0));
            }
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&m.name).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
                    ui.label(small(format!("#{:04}", m.dex)));
                    if m.modern {
                        tag(ui, "MODERN", kit::C_INDIGO).on_hover_text("Only in worlds with MODERN POKéMON on");
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for t in &m.types {
                        type_chip(ui, t, "");
                    }
                });
            });
        });
        ui.add_space(12.0);
        let now = period_now();
        let mut wild = w.wild(&m.id);
        wild.sort_by_key(|e| (std::cmp::Reverse(chance_at(e, now)), w.stage_index(&e.stage)));
        if !wild.is_empty() {
            look_head(ui, "WHERE TO CATCH IT", &format!("best first for {} now", PERIODS[now].to_lowercase()));
            for e in wild.iter().take(5) {
                if wild_row(ui, w, e, now, false) {
                    *go = w.place(&e.area).map(|p| (Kind::Place, if p.group.is_empty() { p.name.clone() } else { p.group.clone() }));
                }
            }
            if wild.len() > 5 {
                ui.label(small(format!("+ {} more in the Pokédex", wild.len() - 5)));
            }
            ui.add_space(8.0);
        }
        let other = w.other(&m.id);
        if !other.is_empty() {
            look_head(ui, "OTHER WAYS TO GET IT", "");
            for o in other.iter().take(3) {
                other_row(ui, o);
            }
            if other.len() > 3 {
                ui.label(small(format!("+ {} more in the Pokédex", other.len() - 3)));
            }
            ui.add_space(8.0);
        }
        if wild.is_empty() && other.is_empty() {
            if let Some(why) = w.unobtainable(&m.id) {
                look_head(ui, "CAN'T BE CAUGHT RIGHT NOW", "");
                ui.add(egui::Label::new(small(why)).wrap());
                ui.add_space(8.0);
            }
        }
        if let Some(e) = w.evolves_from(&m.id) {
            let from = w.mon(&e.from).map(|f| f.name.clone()).unwrap_or_else(|| e.from.clone());
            look_head(ui, "EVOLVES FROM", "");
            ui.horizontal_wrapped(|ui| {
                if ui.link(RichText::new(&from).size(12.5).strong()).clicked() {
                    *go = Some((Kind::Pokemon, e.from.clone()));
                }
                ui.label(small(&e.method));
            });
            ui.add_space(8.0);
        }
        ui.add_space(4.0);
        if ui.add(egui::Button::new(RichText::new("Open in the Pokédex  ›").size(13.0)).fill(kit::btn_fill())).on_hover_text("Moves, stats, evolutions and every spot on the map").clicked() {
            *dex = Some(m.id.clone());
        }
    }

    fn look_place(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, group: &str, go: &mut Option<(Kind, String)>) {
        let groups = place_groups(w);
        let Some(keys) = groups.get(group) else { return };
        let places: Vec<&wiki::Place> = keys.iter().filter_map(|k| w.place(k)).collect();
        let stage = places.iter().map(|p| p.stage.clone()).min_by_key(|s| w.stage_index(s)).unwrap_or_default();
        look_title(ui, Kind::Place, group);
        ui.horizontal_wrapped(|ui| {
            if !stage.is_empty() {
                tag(ui, &stage.to_uppercase(), kit::C_GREEN).on_hover_text("The first time in the story you can get here");
            }
            if places.iter().any(|p| p.cave) {
                tag(ui, "CAVE", kit::C_SLATE);
            }
            if keys.len() > 1 {
                tag(ui, &format!("{} AREAS", keys.len()), kit::C_SLATE).on_hover_text(places.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join("\n"));
            }
        });
        ui.add_space(12.0);
        // Pokémon here: best chance at any time, any area
        let mut best: BTreeMap<String, (u32, String)> = BTreeMap::new();
        for e in w.data.wild.iter().filter(|e| keys.contains(&e.area)) {
            let c = e.chance.morning.max(e.chance.day).max(e.chance.night);
            let ent = best.entry(e.species.clone()).or_insert((0, e.method.clone()));
            if c > ent.0 {
                *ent = (c, e.method.clone());
            }
        }
        let mut mons: Vec<(String, u32)> = best.into_iter().map(|(k, v)| (k, v.0)).collect();
        mons.sort_by_key(|m| std::cmp::Reverse(m.1));
        look_head(ui, "POKéMON HERE", &plural(mons.len(), "kind", "kinds"));
        if mons.is_empty() {
            ui.label(small("No wild Pokémon listed here."));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (id, _) in mons.iter().take(24) {
                    let name = w.mon(id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
                    if self.mon_chip(ui, &name).clicked() {
                        *go = Some((Kind::Pokemon, id.clone()));
                    }
                }
            });
            if mons.len() > 24 {
                ui.label(small(format!("+ {} more", mons.len() - 24)));
            }
        }
        ui.add_space(10.0);
        let names: Vec<String> = places.iter().map(|p| p.name.to_lowercase()).chain(std::iter::once(group.to_lowercase())).collect();
        let items: Vec<&wiki::Item> = w
            .data
            .items
            .iter()
            .filter(|it| it.found.iter().any(|s| {
                let pl = s.place.to_lowercase();
                names.iter().any(|n| pl == *n || pl.starts_with(&format!("{n} ")) || pl.starts_with(&format!("{n},")))
            }))
            .collect();
        look_head(ui, "ITEMS HERE", &plural(items.len(), "item", "items"));
        if items.is_empty() {
            ui.label(small("None listed."));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for it in items.iter().take(18) {
                    if link_chip(ui, &it.name, Kind::Item.color()).clicked() {
                        *go = Some((Kind::Item, it.id.clone()));
                    }
                }
            });
            if items.len() > 18 {
                ui.label(small(format!("+ {} more", items.len() - 18)));
            }
        }
    }

    /// A small Pokémon chip with its sprite. Clickable.
    fn mon_chip(&mut self, ui: &mut egui::Ui, name: &str) -> egui::Response {
        let g = ui.painter().layout_no_wrap(name.to_string(), crate::fonts::sans_font(12.0), TEXT());
        let (r, resp) = ui.allocate_exact_size(egui::vec2(g.size().x + 40.0, 28.0), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let p = ui.painter().clone();
        p.rect_filled(r, 14.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_RED, 0.4 * ht)));
        let drew = match self.info_species(name) {
            Some(sp) => self.info_sprite(&p, &sp, egui::pos2(r.min.x + 16.0, r.center().y), egui::vec2(26.0, 26.0)),
            None => false,
        };
        if !drew {
            p.circle_filled(egui::pos2(r.min.x + 16.0, r.center().y), 8.0, kit::tint(kit::C_RED, 0.25));
        }
        p.galley(egui::pos2(r.min.x + 32.0, r.center().y - g.size().y / 2.0), g, TEXT());
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    // ───────────────────────────── front page ─────────────────────────────

    fn info_front(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        self.info_changes(ui, w);
        self.info_news(ui, w);
        self.info_shortcuts(ui, w);
        self.info_adventure(ui, w);
        ui.add_space(18.0);
        let avail = ui.available_width();
        let gap = 18.0;
        if avail >= 980.0 {
            let left_w = ((avail - gap) * 0.58).floor();
            let right_w = avail - gap - left_w;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    self.info_hunt(ui, w);
                });
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    self.info_browse(ui, w);
                });
            });
        } else {
            self.info_hunt(ui, w);
            ui.add_space(gap);
            self.info_browse(ui, w);
        }
        ui.add_space(gap);
        self.info_guides(ui, w);
    }

    /// Pinned pages and the ones looked at lately.
    fn info_shortcuts(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>) {
        let pins: Vec<Page> = self.cfg.info_pins.iter().filter_map(|k| Page::from_key(k)).collect();
        let recent: Vec<Page> = self.setup.info.recent.iter().filter(|p| !pins.contains(p)).take(8).cloned().collect();
        if pins.is_empty() && recent.is_empty() {
            return;
        }
        let mut open: Option<Page> = None;
        let mut unpin: Option<String> = None;
        kit::card(ui, |ui| {
            for (title, icon, list, pinned) in [("Pinned", "📌", &pins, true), ("Recently viewed", "🕘", &recent, false)] {
                if list.is_empty() {
                    continue;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    let (r, _) = ui.allocate_exact_size(egui::vec2(132.0, 28.0), Sense::hover());
                    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, format!("{icon}  {title}"), crate::fonts::sans_bold_font(12.5), TEXT());
                    for p in list.iter() {
                        let (kind, col) = match p {
                            Page::Move(_) => ("💥", kit::C_VIOLET),
                            Page::Item(_) => ("🎒", kit::C_GOLD),
                            Page::Place(_) => ("🗺", kit::C_GREEN),
                            Page::Guide(_) => ("📚", kit::C_CYAN),
                            _ => ("•", kit::C_SLATE),
                        };
                        let label = self.page_label(p);
                        let g = ui.painter().layout_no_wrap(label, crate::fonts::sans_font(12.5), TEXT());
                        let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(38.0, 12.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        ui.painter().rect_filled(r, r.height() / 2.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.3 + 0.5 * ht)));
                        ui.painter().text(egui::pos2(r.min.x + 15.0, r.center().y), Align2::CENTER_CENTER, kind, egui::FontId::proportional(12.0), col);
                        ui.painter().galley(egui::pos2(r.min.x + 28.0, r.min.y + 6.0), g, TEXT());
                        let tip = if pinned { "Right-click to unpin" } else { "Open it again" };
                        let resp = resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            open = Some(p.clone());
                        }
                        if pinned && resp.secondary_clicked() {
                            unpin = p.key();
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });
        ui.add_space(18.0);
        if let Some(k) = unpin {
            self.cfg.info_pins.retain(|p| *p != k);
            self.cfg.save();
        }
        if let Some(p) = open {
            if let Page::Guide(id) = &p {
                let id = id.clone();
                self.info_read(&id, None, None);
            } else {
                self.info_push(p);
            }
        }
    }

    /// The game's news: the gifts (with how long is left) and the latest version.
    fn info_news(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let notices = w.data.home.notices.clone();
        if notices.is_empty() {
            return;
        }
        let avail = ui.available_width();
        let gap = 18.0;
        let cols = if avail >= 760.0 { notices.len().min(2) } else { 1 };
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut read: Option<String> = None;
        for row in notices.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for n in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        let is_update = n.guide() == Some("whats-new") || n.title.to_lowercase().starts_with("new in");
                        let (icon, col) = if is_update { ("✨", kit::C_VIOLET) } else { ("🎁", kit::C_GOLD) };
                        kit::card(ui, |ui| {
                            ui.set_min_height(118.0);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                                kit::icon_chip(ui.painter(), r, icon, col, false);
                                ui.add_space(4.0);
                                let title = n.title.trim_end_matches(['.', ':']);
                                ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                                // right: how long is left, or the version
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if is_update {
                                        tag(ui, &format!("v{}", w.data.version), kit::C_VIOLET);
                                    } else if let Some(d) = deadline_days(&n.text) {
                                        let (t, c) = match d {
                                            d if d < 0 => ("Ended".to_string(), kit::C_SLATE),
                                            0 => ("Last day".to_string(), kit::C_RED),
                                            1 => ("1 day left".to_string(), kit::C_ORANGE),
                                            d if d <= 7 => (format!("{d} days left"), kit::C_ORANGE),
                                            d => (format!("{d} days left"), kit::C_GREEN),
                                        };
                                        tag(ui, &t, c);
                                    }
                                });
                            });
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(RichText::new(&n.text).size(13.0).color(TEXT())).wrap());
                            ui.add_space(10.0);
                            if let Some(g) = n.guide() {
                                let label = if n.label.is_empty() { "Read more".to_string() } else { n.label.clone() };
                                if ui.link(RichText::new(format!("{label}  ›")).size(12.5)).clicked() {
                                    read = Some(g.to_string());
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(gap);
        }
        if let Some(g) = read {
            self.info_open_guide(&g);
        }
    }

    fn info_open_guide(&mut self, id: &str) {
        self.info_read(id, None, None);
    }

    /// Where the current target lives, best spots for this time of day first.
    fn info_hunt(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let target = self.all_target_idx().first().map(|&i| self.species[i].clone());
        let mon = target.as_ref().and_then(|sp| {
            let want = wiki::slug(sp.name);
            w.data.species.iter().find(|m| wiki::slug(&m.name) == want).cloned()
        });
        let mut dex: Option<String> = None;
        let mut pick = false;
        let mut hunt_place: Option<String> = None;
        let stage = self.story_stage();
        let (more, modern) = (self.cfg.info_more, self.cfg.info_modern);
        let world = move |setting: &str| match setting {
            "more" => more,
            "modern" => modern,
            _ => true,
        };
        kit::card(ui, |ui| {
            let link = if mon.is_some() { Some("Pokédex entry ›") } else { None };
            if card_head(ui, "🎯", kit::C_PINK, "Your hunt in Sinnoh", link) {
                dex = mon.as_ref().map(|m| m.id.clone());
            }
            ui.add_space(10.0);
            let (Some(sp), Some(m)) = (target.as_ref(), mon.as_ref()) else {
                ui.label(RichText::new("No target yet").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(2.0);
                ui.label(small("Pick a Pokémon to hunt and this shows where it lives and when it's most common."));
                ui.add_space(10.0);
                pick = ui.add(egui::Button::new(RichText::new("🎯  Pick a target").size(13.0)).fill(kit::btn_fill())).clicked();
                return;
            };
            // who
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(60.0, 60.0), Sense::hover());
                let col = type_color(m.types.first().map(String::as_str).unwrap_or(""));
                ui.painter().circle_filled(r.center(), 30.0, theme::blend(CARD(), col, 0.20));
                self.info_sprite(ui.painter(), sp, r.center(), egui::vec2(54.0, 54.0));
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&m.name).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                        ui.label(small(format!("#{:04}", m.dex)));
                    });
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        for t in &m.types {
                            type_chip(ui, t, "");
                        }
                    });
                });
            });
            ui.add_space(12.0);
            let now = period_now();
            // only the spots your world has; the ones you can reach first
            let all_wild = w.wild(&m.id);
            let mut wild: Vec<&Wild> = all_wild.iter().copied().filter(|e| world(&e.setting)).collect();
            let other_worlds = all_wild.len() - wild.len();
            let later = |e: &Wild| stage.is_some_and(|s| w.stage_index(&e.stage) > s);
            let best_of = |e: &Wild| e.chance.morning.max(e.chance.day).max(e.chance.night);
            wild.sort_by_key(|e| (later(e), std::cmp::Reverse(chance_at(e, now)), std::cmp::Reverse(best_of(e)), w.stage_index(&e.stage)));
            let reach: Vec<&Wild> = wild.iter().copied().filter(|e| !later(e)).collect();
            // the most common spot at its best time; ties go to the earlier spot
            let best = reach.iter().copied().fold(None::<&Wild>, |b, e| match b {
                Some(x) if (best_of(x), chance_at(x, now)) >= (best_of(e), chance_at(e, now)) => Some(x),
                _ => Some(e),
            });
            if let Some(best) = best {
                let place = w.place(&best.area).map(|p| p.name.clone()).unwrap_or_else(|| best.area.clone());
                let t = (0..3).max_by_key(|&p| (chance_at(best, p), p == now)).unwrap_or(now);
                let mut text = format!(
                    "{} at Lv {}, best in the {} ({}%).",
                    super::dex_tabs::method_label(&best.method),
                    best.levels,
                    PERIODS[t].to_lowercase(),
                    chance_at(best, t)
                );
                if t != now {
                    text.push_str(&format!(" Right now ({}) it's {}%.", PERIODS[now].to_lowercase(), chance_at(best, now)));
                }
                if let Some((item, what)) = super::info_story::method_needs(&best.method) {
                    let from = w.data.items.iter().find(|i| i.name == item).and_then(|i| i.found.first()).map(|f| format!(" (from {})", f.place)).unwrap_or_default();
                    text.push_str(&format!(" You'll need {what}{from}."));
                }
                callout(ui, &format!("Your best bet: {place}"), &text);
                ui.add_space(10.0);
            } else if let Some(first) = wild.iter().min_by_key(|e| w.stage_index(&e.stage)) {
                let place = w.place(&first.area).map(|p| p.name.clone()).unwrap_or_else(|| first.area.clone());
                let si = w.stage_index(&first.stage);
                let when = w.data.stages.get(si).map(|n| super::info_story::stage_words(si, n)).unwrap_or_default();
                callout(ui, "Not reachable yet", &format!("The first spot is {place}, once you're at: {when}."));
                ui.add_space(10.0);
            } else if other_worlds > 0 {
                callout(ui, "Not in your world", "It's only in the wild with MORE or MODERN POKéMON on. Change your world's options in Your adventure.");
                ui.add_space(10.0);
            }
            if wild.is_empty() {
                let other = w.other(&m.id);
                if other.is_empty() && other_worlds > 0 {
                    // said above
                } else if other.is_empty() {
                    match w.unobtainable(&m.id) {
                        Some(why) => callout(ui, "Can't be caught right now", why),
                        None => callout(ui, "Not in the wild", "The game's data doesn't list anywhere to get it."),
                    }
                } else {
                    callout(ui, "Not in the wild", "You can get it these ways instead:");
                    ui.add_space(8.0);
                    for o in other.iter().take(4) {
                        other_row(ui, o);
                    }
                }
            } else {
                look_head(ui, &format!("{} IN THE WILD", plural(wild.len(), "SPOT", "SPOTS")), "Morning · Day · Night");
                for e in wild.iter().take(6) {
                    if wild_row(ui, w, e, now, later(e)) {
                        hunt_place = w.place(&e.area).map(|p| if p.group.is_empty() { p.name.clone() } else { p.group.clone() });
                    }
                }
                if wild.len() > 6 {
                    if ui.link(RichText::new(format!("All {} spots in the Pokédex ›", wild.len())).size(12.5)).clicked() {
                        dex = Some(m.id.clone());
                    }
                }
            }
        });
        if pick {
            self.go_to(Tab::Hunt);
        }
        if let Some(p) = hunt_place {
            self.info_push(Page::Place(p));
        }
        if let Some(id) = dex {
            self.info_open_dex(Some(id));
        }
    }

    /// Ways in: every kind, with what's in it.
    fn info_browse(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let wild_kinds = {
            let mut s: Vec<&str> = w.data.wild.iter().map(|e| e.species.as_str()).collect();
            s.sort_unstable();
            s.dedup();
            s.len()
        };
        let modern = w.data.species.iter().filter(|m| m.modern).count();
        let tms = w.data.tms.len();
        let tutors = w.data.tutors.len();
        let pockets = {
            let mut p: Vec<&str> = w.data.items.iter().map(|i| i.pocket.as_str()).collect();
            p.sort_unstable();
            p.dedup();
            p.len()
        };
        let berries = w.data.items.iter().filter(|i| i.pocket == "Berries").count();
        let groups = place_groups(w).len();
        let caves = place_groups(w).values().filter(|ks| ks.iter().any(|k| w.place(k).is_some_and(|p| p.cave))).count();
        let topics = {
            let mut t: Vec<&str> = w.data.guides.iter().map(|g| g.group.as_str()).collect();
            t.sort_unstable();
            t.dedup();
            t.len()
        };
        let rows: [(Kind, String, String); 5] = [
            (Kind::Pokemon, plural(w.data.species.len(), "Pokémon", "Pokémon"), format!("{wild_kinds} in the wild · {modern} Modern")),
            (Kind::Move, plural(w.data.moves.len(), "move", "moves"), format!("{tms} TMs & HMs · {tutors} tutor moves")),
            (Kind::Item, plural(w.data.items.len(), "item", "items"), format!("{pockets} Bag pockets · {berries} Berries")),
            (Kind::Place, plural(groups, "place", "places"), format!("{caves} caves · {} story stages", w.data.stages.len())),
            (Kind::Guide, plural(w.data.guides.len(), "guide", "guides"), format!("{topics} topics on how it all works")),
        ];
        let mut go: Option<Kind> = None;
        kit::card(ui, |ui| {
            card_head(ui, "📂", kit::C_ORANGE, "Browse", None);
            ui.add_space(8.0);
            for (k, title, sub) in &rows {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(r.expand(2.0));
                if ht > 0.0 {
                    p.rect_filled(r, 12.0, theme::blend(CARD(), theme::blend(CARD_HOVER(), k.color(), 0.06), ht));
                }
                let chip = Rect::from_center_size(egui::pos2(r.min.x + 24.0, r.center().y), egui::vec2(34.0, 34.0));
                kit::icon_chip(&p, chip, k.icon(), k.color(), true);
                let x = chip.max.x + 12.0;
                p.text(egui::pos2(x, r.center().y - 8.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(14.0), TEXT());
                p.text(egui::pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(11.5), MUTED());
                let hint = match k {
                    Kind::Pokemon => "Pokédex",
                    Kind::Guide => "How to Play",
                    Kind::Move => "Moves",
                    Kind::Item => "Items",
                    Kind::Place => "Places",
                };
                let g = p.layout_no_wrap(hint.to_string(), crate::fonts::sans_font(11.5), MUTED());
                p.text(egui::pos2(r.max.x - 12.0, r.center().y), Align2::RIGHT_CENTER, "›", egui::FontId::proportional(18.0), if ht > 0.5 { TEXT() } else { MUTED() });
                if ht > 0.0 {
                    p.galley(egui::pos2(r.max.x - 28.0 - g.size().x, r.center().y - g.size().y / 2.0), g, kit::tint(MUTED(), ht));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    go = Some(*k);
                }
            }
        });
        match go {
            Some(Kind::Pokemon) => self.info_open_dex(None),
            Some(Kind::Guide) => self.info_push(Page::Guides),
            Some(Kind::Move) => self.info_push(Page::Moves),
            Some(Kind::Item) => self.info_push(Page::Items),
            Some(Kind::Place) => self.info_push(Page::Places),
            None => {}
        }
    }

    /// The how-to-play guides, a topic at a time.
    fn info_guides(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let guides = w.guides_sorted();
        if guides.is_empty() {
            return;
        }
        let mut topics: Vec<&str> = Vec::new();
        for g in &guides {
            if !topics.contains(&g.group.as_str()) {
                topics.push(g.group.as_str());
            }
        }
        let topic_i = self.setup.info.topic.min(topics.len() - 1);
        let mut open: Option<String> = None;
        let mut all = false;
        kit::card(ui, |ui| {
            all = card_head(ui, "📚", kit::C_CYAN, "How it all works", Some(&format!("All {} guides ›", guides.len())));
            ui.add_space(10.0);
            let mut t = topic_i;
            let opts: Vec<(usize, &str)> = topics.iter().enumerate().map(|(i, s)| (i, *s)).collect();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (i, name) in &opts {
                    let n = guides.iter().filter(|g| g.group == *name).count();
                    if filter_chip(ui, &format!("{name}  {n}"), t == *i, kit::C_CYAN) {
                        t = *i;
                    }
                }
            });
            self.setup.info.topic = t;
            ui.add_space(12.0);
            let list: Vec<&&wiki::Guide> = guides.iter().filter(|g| g.group == topics[t]).collect();
            let avail = ui.available_width();
            let cols = if avail >= 700.0 { 2 } else { 1 };
            let gap = 10.0;
            let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in list.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for g in row {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 66.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter_at(r.expand(2.0));
                        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_CYAN, 0.45 * ht)));
                        let chip = Rect::from_min_size(egui::pos2(r.min.x + 12.0, r.min.y + 12.0), egui::vec2(28.0, 28.0));
                        kit::icon_chip(&p, chip, guide_icon(&g.icon), kit::C_CYAN, false);
                        let x = chip.max.x + 10.0;
                        let maxw = r.max.x - x - 12.0;
                        let f = crate::fonts::sans_bold_font(13.0);
                        p.text(egui::pos2(x, r.min.y + 20.0), Align2::LEFT_CENTER, super::charts::clip(ui, &g.title, &f, maxw), f, TEXT());
                        // summary, two lines at most
                        let mut job = egui::text::LayoutJob::simple(g.summary.clone(), crate::fonts::sans_font(11.5), MUTED(), maxw);
                        job.wrap.max_rows = 2;
                        job.wrap.break_anywhere = false;
                        let gal = ui.fonts(|f| f.layout_job(job));
                        p.galley(egui::pos2(x, r.min.y + 30.0), gal, MUTED());
                        if resp.on_hover_text(&g.summary).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(g.id.clone());
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
        if let Some(id) = open {
            self.info_open_guide(&id);
        } else if all {
            self.info_push(Page::Guides);
        }
    }

    /// Where the data comes from, how fresh it is, and a way to check.
    fn info_source(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let check = wiki::check_state();
        let mut refresh = false;
        let mut open = false;
        let line = match (&check.last, wiki::cache_age()) {
            _ if check.running => "Checking the wiki for a newer copy…".to_string(),
            (Some((_, Err(e))), _) => e.clone(),
            (Some((_, Ok(true))), _) => "Updated just now".to_string(),
            (Some((_, Ok(false))), _) => "Up to date (checked just now)".to_string(),
            (None, Some(age)) => format!("Checked {}", age_text(age.as_secs())),
            (None, None) => "Built-in copy".to_string(),
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.label(small(format!("Game data from the Platinum Together wiki · game version {} · {line}", w.data.version)));
            ui.add_enabled_ui(!check.running, |ui| {
                refresh = ui.link(RichText::new("Check now").size(12.0)).clicked();
            });
            open = ui.link(RichText::new("Open the wiki  ↗").size(12.0)).clicked();
        });
        if check.running {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        }
        if refresh {
            self.wiki_rx = wiki::start_refresh();
        }
        if open {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://platinumtogether.online/wiki/"));
        }
    }
}

fn age_text(secs: u64) -> String {
    match secs {
        0..=89 => "just now".into(),
        90..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => {
            let h = secs / 3600;
            format!("{h} hour{} ago", if h == 1 { "" } else { "s" })
        }
        _ => {
            let d = secs / 86_400;
            format!("{d} day{} ago", if d == 1 { "" } else { "s" })
        }
    }
}

fn set_kind_later(s: &mut InfoState, k: Kind) {
    s.kind = Some(k);
}

/// A label in a fixed-width column, then wrapping text beside it.
fn how_row(ui: &mut egui::Ui, how: &str, col: Color32, title: &str, detail: &str) {
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(96.0, 18.0), Sense::hover());
        let mut label = how.to_uppercase();
        let f = crate::fonts::sans_bold_font(9.5);
        label = super::charts::clip(ui, &label, &f, 84.0);
        let g = ui.painter().layout_no_wrap(label, f, col);
        let pr = Rect::from_min_size(r.min + egui::vec2(0.0, 1.0), egui::vec2(g.size().x + 12.0, 18.0));
        ui.painter().rect_filled(pr, 9.0, kit::tint(col, 0.16));
        ui.painter().galley(pr.min + egui::vec2(6.0, 9.0 - g.size().y / 2.0), g, col);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add(egui::Label::new(RichText::new(title).size(12.5).color(TEXT())).wrap());
            if !detail.is_empty() {
                ui.add(egui::Label::new(small(detail)).wrap());
            }
        });
    });
    ui.add_space(6.0);
}

fn other_row(ui: &mut egui::Ui, o: &wiki::Other) {
    let lv = if o.level.is_empty() { String::new() } else { format!(" · Lv {}", o.level) };
    let col = match o.how.as_str() {
        "Gift" => kit::C_PINK,
        "Egg" => kit::C_GOLD,
        "Trade" => kit::C_CYAN,
        _ => kit::C_TEAL,
    };
    how_row(ui, &o.how, col, &format!("{}{lv}", o.place), "");
}

/// A toggle chip for filters. True when clicked.
fn filter_chip(ui: &mut egui::Ui, text: &str, on: bool, col: Color32) -> bool {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), if on { TEXT() } else { MUTED() });
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), col, 0.22) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    ui.painter().rect_filled(r, r.height() / 2.0, fill);
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), col, 0.7) } else { LINE() }));
    ui.painter().galley(r.min + egui::vec2(10.0, 6.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn link_chip(ui: &mut egui::Ui, text: &str, col: Color32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_font(12.0), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(18.0, 10.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    ui.painter().rect_filled(r, r.height() / 2.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5 * ht)));
    ui.painter().galley(r.min + egui::vec2(9.0, 5.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small heading inside a quick look: caps on the left, a note on the right.
fn look_head(ui: &mut egui::Ui, text: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
        if !note.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(note).size(11.0).color(MUTED()));
            });
        }
    });
    ui.add_space(4.0);
}

fn look_title(ui: &mut egui::Ui, k: Kind, title: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, k.icon(), k.color(), true);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(k.one().to_uppercase()).font(crate::fonts::sans_bold_font(10.0)).color(MUTED()));
            ui.add(egui::Label::new(RichText::new(title).font(crate::fonts::sans_bold_font(20.0)).color(TEXT())).wrap());
        });
    });
    ui.add_space(8.0);
}

/// A highlighted line with a title.
fn callout(ui: &mut egui::Ui, title: &str, text: &str) {
    let pl = theme::pal();
    egui::Frame::none()
        .fill(theme::blend(CARD(), pl.accent, 0.12))
        .stroke(Stroke::new(1.0f32, theme::blend(LINE(), pl.accent, 0.35)))
        .rounding(12.0)
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
            ui.add_space(2.0);
            ui.add(egui::Label::new(RichText::new(text).size(12.5).color(MUTED())).wrap());
        });
}

/// One wild spot: place, how and levels, stage, and the three times of day.
/// Returns true when clicked (open the place). `later`: not reachable yet (dimmed).
fn wild_row(ui: &mut egui::Ui, w: &Wiki, e: &Wild, now: usize, later: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
    if !ui.is_rect_visible(r) {
        return false;
    }
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter_at(r);
    if ht > 0.0 {
        p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht * 0.8));
    }
    p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0f32, kit::tint(LINE(), 0.6)));
    let place = w.place(&e.area).map(|p| p.name.clone()).unwrap_or_else(|| e.area.clone());
    let bars_w = 126.0;
    let text_w = r.width() - bars_w - 12.0;
    let f = crate::fonts::sans_bold_font(13.0);
    let mut title = place;
    match e.setting.as_str() {
        "more" => title.push_str("  · MORE"),
        "modern" => title.push_str("  · MODERN"),
        _ => {}
    }
    if later {
        title.push_str("  · LATER");
    }
    let ink = if later { MUTED() } else { TEXT() };
    p.text(egui::pos2(r.min.x, r.center().y - 8.0), Align2::LEFT_CENTER, super::charts::clip(ui, &title, &f, text_w), f, ink);
    let sub = format!("{} · Lv {} · {}", super::dex_tabs::method_label(&e.method), e.levels, e.stage);
    let f2 = crate::fonts::sans_font(11.5);
    p.text(egui::pos2(r.min.x, r.center().y + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub, &f2, text_w), f2, MUTED());
    // three little columns: morning, day, night (now highlighted)
    let cw = 38.0;
    let x0 = r.max.x - 3.0 * cw - 4.0;
    for (i, c) in [e.chance.morning, e.chance.day, e.chance.night].iter().enumerate() {
        let cell = Rect::from_min_size(egui::pos2(x0 + i as f32 * (cw + 2.0), r.min.y + 7.0), egui::vec2(cw, 30.0));
        let on = i == now;
        let col = if *c == 0 || later { MUTED() } else { kit::C_GREEN };
        if on {
            p.rect_filled(cell, 8.0, theme::blend(CARD(), kit::C_GREEN, 0.16));
            p.rect_stroke(cell, 8.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_GREEN, 0.5)));
        }
        let t = if *c == 0 { "—".to_string() } else { format!("{c}%") };
        p.text(cell.center() - egui::vec2(0.0, 5.0), Align2::CENTER_CENTER, t, crate::fonts::sans_bold_font(12.0), if on && !later { TEXT() } else { col });
        p.text(cell.center() + egui::vec2(0.0, 8.0), Align2::CENTER_CENTER, ["Morn", "Day", "Night"][i], crate::fonts::sans_font(9.0), MUTED());
    }
    let tip = if later { format!("You can't get here yet ({}). Open this place", e.stage) } else { "Open this place".to_string() };
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn stat_box(ui: &mut egui::Ui, label: &str, value: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(w, 52.0), Sense::hover());
    ui.painter().rect_filled(r, 10.0, kit::sunken());
    ui.painter().text(egui::pos2(r.min.x + 12.0, r.min.y + 16.0), Align2::LEFT_CENTER, label, crate::fonts::sans_font(11.0), MUTED());
    ui.painter().text(egui::pos2(r.min.x + 12.0, r.min.y + 35.0), Align2::LEFT_CENTER, value, crate::fonts::sans_bold_font(16.0), TEXT());
}

fn look_move(ui: &mut egui::Ui, w: &Wiki, key: &str, go: &mut Option<(Kind, String)>) {
    let Some(m) = w.move_info(key) else { return };
    look_title(ui, Kind::Move, &m.n);
    ui.horizontal(|ui| {
        type_chip(ui, &m.t, "");
        tag(ui, &m.c.to_uppercase(), kit::C_SLATE);
    });
    ui.add_space(12.0);
    let dash = |s: &str| if s.is_empty() || s == "0" { "—".to_string() } else { s.to_string() };
    let avail = ui.available_width();
    let bw = ((avail - 16.0) / 3.0).floor();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        stat_box(ui, "Power", &dash(&m.p), bw);
        let acc = if m.a.is_empty() || m.a == "0" { "—".to_string() } else { format!("{}%", m.a) };
        stat_box(ui, "Accuracy", &acc, bw);
        stat_box(ui, "PP", &dash(&m.pp), bw);
    });
    ui.add_space(12.0);
    ui.add(egui::Label::new(RichText::new(&m.d).size(13.0).color(TEXT())).wrap());
    ui.add_space(12.0);
    // how you teach it
    let tm: Vec<&String> = w.data.tms.iter().filter(|(_, v)| v.as_str() == key).map(|(k, _)| k).collect();
    if let Some(t) = tm.first() {
        let item = w.data.items.iter().find(|i| i.teaches == key || i.name.starts_with(t.as_str()));
        look_head(ui, "TEACH IT WITH", "");
        ui.horizontal_wrapped(|ui| {
            match item {
                Some(it) => {
                    if link_chip(ui, &it.name, Kind::Item.color()).clicked() {
                        *go = Some((Kind::Item, it.id.clone()));
                    }
                }
                None => {
                    tag(ui, t, kit::C_GOLD);
                }
            }
        });
        ui.add_space(10.0);
    }
    if let Some(t) = w.data.tutors.get(key) {
        look_head(ui, "MOVE TUTOR", "");
        let cost: Vec<String> = t.cost.iter().map(|(c, n)| format!("{n} {c}")).collect();
        ui.label(RichText::new(format!("{} · {} shards", t.place, cost.join(", "))).size(12.5).color(TEXT()));
        ui.add_space(10.0);
    }
    // who learns it
    let mut who: Vec<&Mon> = w
        .data
        .learnsets
        .iter()
        .filter(|(_, l)| {
            l.lv.iter().chain(l.mlv.iter()).any(|x| x.key == key)
                || l.tm.iter().chain(l.mtm.iter()).any(|x| x == key || tm.iter().any(|t| t.as_str() == x.as_str()))
                || l.tutor.iter().chain(l.mtutor.iter()).any(|x| x == key)
                || l.egg.iter().chain(l.megg.iter()).any(|x| x == key)
        })
        .filter_map(|(id, _)| w.mon(id))
        .collect();
    who.sort_by_key(|m| m.dex);
    look_head(ui, "LEARNED BY", &plural(who.len(), "Pokémon", "Pokémon"));
    if who.is_empty() {
        ui.label(small("Nothing learns it."));
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for m in who.iter().take(20) {
                if link_chip(ui, &m.name, Kind::Pokemon.color()).clicked() {
                    *go = Some((Kind::Pokemon, m.id.clone()));
                }
            }
        });
        if who.len() > 20 {
            ui.label(small(format!("+ {} more", who.len() - 20)));
        }
    }
}

fn look_item(ui: &mut egui::Ui, w: &Wiki, id: &str, go: &mut Option<(Kind, String)>) {
    let Some(it) = w.data.items.iter().find(|i| i.id == id) else { return };
    look_title(ui, Kind::Item, &it.name);
    ui.horizontal(|ui| {
        tag(ui, &it.pocket.to_uppercase(), kit::C_GOLD);
        match it.price.parse::<u64>() {
            Ok(p) if p > 0 => {
                tag(ui, &format!("₽{} IN SHOPS", fmt_n(p as usize)), kit::C_SLATE);
            }
            _ => {}
        }
    });
    ui.add_space(10.0);
    ui.add(egui::Label::new(RichText::new(&it.description).size(13.0).color(TEXT())).wrap());
    ui.add_space(10.0);
    if !it.teaches.is_empty() {
        let key = w.move_key(&it.teaches);
        if let Some((k, m)) = key.as_ref().and_then(|k| w.move_info(k).map(|m| (k.clone(), m))) {
            ui.horizontal(|ui| {
                ui.label(small("Teaches"));
                if link_chip(ui, &m.n, Kind::Move.color()).clicked() {
                    *go = Some((Kind::Move, k.clone()));
                }
            });
            ui.add_space(10.0);
        }
    }
    look_head(ui, "WHERE TO GET IT", &plural(it.found.len(), "way", "ways"));
    if it.found.is_empty() {
        ui.label(small("The game's data doesn't list anywhere."));
    }
    for s in it.found.iter().take(8) {
        let col = match s.how.as_str() {
            "Ground" => kit::C_GREEN,
            "Hidden" => kit::C_VIOLET,
            "Gift" => kit::C_PINK,
            "Shop" => kit::C_GOLD,
            "Held" => kit::C_CYAN,
            "Prize" => kit::C_ORANGE,
            "Berry tree" => kit::C_TEAL,
            _ => kit::C_SLATE,
        };
        how_row(ui, &s.how, col, &s.place, &s.detail);
    }
    if it.found.len() > 8 {
        ui.label(small(format!("+ {} more", it.found.len() - 8)));
    }
}

/// A guide in the side panel; returns (guide, part) to open.
fn look_guide(ui: &mut egui::Ui, w: &Wiki, id: &str, doc: &super::info_md::Doc) -> Option<(String, Option<usize>)> {
    let g = w.guide(id)?;
    let mut open = None;
    let (_, col) = super::info_guides::topic_meta(&g.group);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, guide_icon(&g.icon), col, true);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(g.group.to_uppercase()).font(crate::fonts::sans_bold_font(10.0)).color(MUTED()));
            ui.add(egui::Label::new(RichText::new(&g.title).font(crate::fonts::sans_bold_font(19.0)).color(TEXT())).wrap());
        });
    });
    ui.add_space(10.0);
    ui.add(egui::Label::new(RichText::new(&g.summary).size(13.0).color(TEXT())).wrap());
    ui.add_space(12.0);
    let heads: Vec<(usize, &super::info_md::Section)> = doc.sections.iter().enumerate().filter(|(_, s)| s.level == 2).collect();
    if !heads.is_empty() {
        look_head(ui, "JUMP TO A PART", &plural(heads.len(), "part", "parts"));
        for (i, h) in heads.iter().take(10) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("•").color(col));
                if ui.link(RichText::new(h.title.replace("**", "")).size(12.5)).clicked() {
                    open = Some((g.id.clone(), Some(*i)));
                }
            });
        }
        if heads.len() > 10 {
            ui.label(small(format!("+ {} more", heads.len() - 10)));
        }
        ui.add_space(10.0);
    }
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(RichText::new("Read the guide  ›").size(13.0)).fill(kit::btn_fill())).clicked() {
            open = Some((g.id.clone(), None));
        }
        ui.label(small(format!("about {} min read", (doc.words / 200).max(1))));
    });
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        for (y, m, d) in [(2026, 11, 27), (2024, 2, 29), (1970, 1, 1), (2027, 1, 3)] {
            assert_eq!(civil(days_from_civil(y, m, d)), (y, m, d));
        }
    }

    #[test]
    fn finds_the_deadline() {
        assert!(deadline_days("Claim them in the RiftPad's MAIL before 27 November.").is_some());
        assert!(deadline_days("Until 3 January, maybe").is_some());
        assert!(deadline_days("No date here.").is_none());
        assert!(deadline_days("before lunch").is_none());
    }

    #[test]
    fn search_scores() {
        let h = Hit { kind: Kind::Move, key: "surf".into(), title: "Surf".into(), sub: String::new(), hay: "surf".into(), more: "water special".into(), dex: 0 };
        assert_eq!(score(&h, "surf"), Some(0));
        assert_eq!(score(&h, "su"), Some(1));
        assert_eq!(score(&h, "water"), Some(4));
        assert_eq!(score(&h, "xyz"), None);
        let m = Hit { kind: Kind::Pokemon, key: "PIKACHU".into(), title: "Pikachu".into(), sub: String::new(), hay: "pikachu".into(), more: String::new(), dex: 25 };
        assert_eq!(score(&m, "#25"), Some(0));
        assert_eq!(score(&m, "26"), None);
    }

    #[test]
    fn real_data_parses_the_new_parts() {
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = Wiki::parse(&bytes).expect("parses");
        assert!(!w.data.items.is_empty());
        assert!(!w.data.guides.is_empty());
        assert!(!w.data.home.notices.is_empty());
        let idx = build_index(&w);
        assert!(idx.iter().any(|h| h.kind == Kind::Place));
    }
}
