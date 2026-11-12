//! Game Info › your world and your story (how far in you are, and which
//! options your world has, so the rest of Game Info can say what you can
//! reach and catch now), and keeping the data fresh (what changed when the
//! wiki updated).

use super::info_home::Page;
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::wiki::{Wiki, Wild};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn small(t: impl Into<String>) -> RichText {
    RichText::new(t).font(crate::fonts::sans_font(12.0)).color(MUTED())
}

/// "Before Roark" → "No badges yet", "Before Byron" → "5 badges", …
pub(super) fn stage_words(i: usize, name: &str) -> String {
    let l = name.to_lowercase();
    if l.starts_with("after") {
        "Beat the League".into()
    } else if l.contains("league") {
        "All 8 badges".into()
    } else if i == 0 {
        "No badges yet".into()
    } else {
        format!("{i} badge{}", if i == 1 { "" } else { "s" })
    }
}

/// What you need before you can meet Pokémon this way, and the item that gives it.
pub(super) fn method_needs(m: &str) -> Option<(&'static str, &'static str)> {
    match m {
        "surf" => Some(("HM03", "Surf (HM03), and the Fen Badge to use it outside battle")),
        "old_rod" => Some(("Old Rod", "the Old Rod")),
        "good_rod" => Some(("Good Rod", "the Good Rod")),
        "super_rod" => Some(("Super Rod", "the Super Rod")),
        "radar" => Some(("Poké Radar", "the Poké Radar, after the League")),
        _ => None,
    }
}

// ───────────────────────────── what changed ─────────────────────────────

/// A short record of a wiki copy, kept to compare the next one with.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub(super) struct Seen {
    version: String,
    source: String,
    species: BTreeMap<String, String>,
    items: BTreeMap<String, String>,
    moves: BTreeMap<String, String>,
    places: BTreeMap<String, String>,
    guides: BTreeMap<String, String>,
    /// "SPECIES|area|method|setting" → "levels|morning/day/night"
    wild: BTreeMap<String, String>,
}

fn wild_key(e: &Wild) -> String {
    format!("{}|{}|{}|{}", e.species, e.area, e.method, e.setting)
}

fn wild_val(e: &Wild) -> String {
    format!("{}|{}/{}/{}", e.levels, e.chance.morning, e.chance.day, e.chance.night)
}

impl Seen {
    fn of(w: &Wiki) -> Self {
        Seen {
            version: w.data.version.clone(),
            source: w.data.source.clone(),
            species: w.data.species.iter().map(|m| (m.id.clone(), m.name.clone())).collect(),
            items: w.data.items.iter().map(|i| (i.id.clone(), i.name.clone())).collect(),
            moves: w.data.moves.iter().map(|(k, m)| (k.clone(), m.n.clone())).collect(),
            places: w.data.places.iter().map(|(k, p)| (k.clone(), p.name.clone())).collect(),
            guides: w.data.guides.iter().map(|g| (g.id.clone(), g.title.clone())).collect(),
            wild: w.data.wild.iter().map(|e| (wild_key(e), wild_val(e))).collect(),
        }
    }
}

/// What's different between the copy you last saw and this one.
#[derive(Default, Clone)]
pub(super) struct Changes {
    pub from: String,
    pub to: String,
    pub species: Vec<(String, String)>,
    pub items: Vec<(String, String)>,
    pub moves: Vec<(String, String)>,
    pub places: Vec<(String, String)>,
    pub guides: Vec<(String, String)>,
    pub gone: usize,
    /// (species, area) of new wild spots and of spots whose rates or levels changed
    pub new_spots: Vec<(String, String)>,
    pub changed_spots: Vec<(String, String)>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.species.is_empty() && self.items.is_empty() && self.moves.is_empty() && self.places.is_empty() && self.guides.is_empty() && self.new_spots.is_empty() && self.changed_spots.is_empty() && self.gone == 0
    }
}

fn added(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> Vec<(String, String)> {
    new.iter().filter(|(k, _)| !old.contains_key(*k)).map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn diff(old: &Seen, new: &Seen) -> Changes {
    let gone = [(&old.species, &new.species), (&old.items, &new.items), (&old.moves, &new.moves), (&old.places, &new.places), (&old.guides, &new.guides)]
        .iter()
        .map(|(a, b)| a.keys().filter(|k| !b.contains_key(*k)).count())
        .sum();
    let split = |k: &str| {
        let mut p = k.split('|');
        (p.next().unwrap_or_default().to_string(), p.next().unwrap_or_default().to_string())
    };
    let mut new_spots = BTreeSet::new();
    let mut changed = BTreeSet::new();
    for (k, v) in &new.wild {
        match old.wild.get(k) {
            None => {
                new_spots.insert(split(k));
            }
            Some(o) if o != v => {
                changed.insert(split(k));
            }
            _ => {}
        }
    }
    Changes {
        from: old.version.clone(),
        to: new.version.clone(),
        species: added(&old.species, &new.species),
        items: added(&old.items, &new.items),
        moves: added(&old.moves, &new.moves),
        places: added(&old.places, &new.places),
        guides: added(&old.guides, &new.guides),
        gone,
        new_spots: new_spots.into_iter().collect(),
        changed_spots: changed.into_iter().collect(),
    }
}

fn seen_file() -> std::path::PathBuf {
    crate::config::cache_dir().join("wiki-seen.json")
}

fn read_seen() -> Option<Seen> {
    let bytes = std::fs::read(seen_file()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Remember this copy as seen (beside the cache, written whole then swapped in).
fn write_seen(s: &Seen) {
    let path = seen_file();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(bytes) = serde_json::to_vec(s) {
        let tmp = path.with_extension("json.part");
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

#[derive(Default)]
pub(super) struct FreshState {
    /// The wiki copy (Arc address) last compared, and what changed.
    key: usize,
    pub changes: Option<Changes>,
    open: bool,
}

impl App {
    /// How far into the story you said you are (an index into the badge stages).
    pub(super) fn story_stage(&self) -> Option<usize> {
        self.cfg.info_stage.map(|s| s as usize)
    }

    /// Does your world have what this wild slot needs ("more", "modern")?
    pub(super) fn world_has(&self, setting: &str) -> bool {
        match setting {
            "more" => self.cfg.info_more,
            "modern" => self.cfg.info_modern,
            _ => true,
        }
    }

    // ───────────────────────────── your adventure ─────────────────────────────

    /// Front page: where you are in the story, your world's options, and
    /// what that opens up.
    pub(super) fn info_adventure(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let stages = w.data.stages.clone();
        if stages.is_empty() {
            return;
        }
        let at = self.story_stage().map(|s| s.min(stages.len() - 1));
        let groups = super::info_places::groups(w);
        let mut set: Option<Option<u8>> = None;
        let mut open_place: Option<String> = None;
        let mut flip_more = false;
        let mut flip_modern = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                card_head(ui, "🏅", kit::C_TEAL, "Your adventure", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if at.is_some() && ui.link(RichText::new("Show everything").size(12.5)).on_hover_text("Forget where you are: nothing is marked as later").clicked() {
                        set = Some(None);
                    }
                });
            });
            ui.add_space(4.0);
            ui.label(small(match at {
                None => "Where are you in the story? Click the gym you're heading to next, and Game Info marks what you can reach and catch now and what comes later.".to_string(),
                Some(i) => {
                    let heading = stages[i].strip_prefix("Before ").map(|g| format!(", heading to {g}")).unwrap_or_default();
                    format!("You're at {}{heading}. Places and Pokémon after this are marked LATER.", stage_words(i, &stages[i]).to_lowercase())
                }
            }));
            ui.add_space(12.0);
            // the track: a stop for each stage
            let n = stages.len();
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), Sense::hover());
            let pad = 18.0;
            let step = (r.width() - pad * 2.0) / (n.max(2) - 1) as f32;
            let y = r.min.y + 18.0;
            let p = ui.painter();
            p.line_segment([egui::pos2(r.min.x + pad, y), egui::pos2(r.max.x - pad, y)], Stroke::new(3.0f32, kit::tint(LINE(), 0.9)));
            if let Some(i) = at {
                p.line_segment([egui::pos2(r.min.x + pad, y), egui::pos2(r.min.x + pad + step * i as f32, y)], Stroke::new(3.0f32, kit::C_TEAL));
            }
            for (i, name) in stages.iter().enumerate() {
                let c = egui::pos2(r.min.x + pad + step * i as f32, y);
                let hit = Rect::from_center_size(c + egui::vec2(0.0, 12.0), egui::vec2(step.max(30.0), 52.0));
                let resp = ui.interact(hit, ui.id().with(("story-stop", i)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let done = at.is_some_and(|a| i <= a);
                let here = at == Some(i);
                let rad = if here { 11.0 } else { 8.0 } + 2.0 * ht;
                let p = ui.painter();
                p.circle_filled(c, rad, if done { kit::C_TEAL } else { theme::blend(CARD(), CARD_HOVER(), 0.5 + ht * 0.5) });
                p.circle_stroke(c, rad, Stroke::new(1.5f32, if done || ht > 0.3 { kit::C_TEAL } else { LINE() }));
                let label = if name.to_lowercase().starts_with("after") {
                    "★".to_string()
                } else if name.to_lowercase().contains("league") {
                    "8".to_string()
                } else {
                    i.to_string()
                };
                p.text(c, Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(if here { 11.5 } else { 10.0 }), if done { Color32::WHITE } else { MUTED() });
                // a word under every other stop when narrow
                if step >= 64.0 || here || i == 0 || i + 1 == n {
                    let word = name.trim_start_matches("Before ").trim_start_matches("the ").to_string();
                    let word = if name.to_lowercase().starts_with("after") { "Postgame".to_string() } else { word };
                    p.text(c + egui::vec2(0.0, 24.0), Align2::CENTER_CENTER, word, crate::fonts::sans_font(10.5), if here { TEXT() } else { MUTED() });
                }
                let tip = format!("{}: {}", stage_words(i, name), name);
                if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    set = Some(Some(i as u8));
                }
            }
            ui.add_space(10.0);
            // what it opens up
            if let Some(i) = at {
                let open_now = groups.iter().filter(|g| g.stage <= i).count();
                let mons = {
                    let mut s: BTreeSet<&str> = BTreeSet::new();
                    for e in &w.data.wild {
                        if w.stage_index(&e.stage) <= i && self.world_has(&e.setting) {
                            s.insert(e.species.as_str());
                        }
                    }
                    s.len()
                };
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    stat(ui, &open_now.to_string(), "places you can reach");
                    stat(ui, &mons.to_string(), "kinds of wild Pokémon");
                });
                let next: Vec<&super::info_places::PlaceGroup> = groups.iter().filter(|g| g.stage == i + 1).collect();
                if !next.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("YOUR NEXT BADGE OPENS {} PLACES", next.len())).font(crate::fonts::sans_bold_font(10.5)).color(kit::C_TEAL));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for g in next {
                            let gal = ui.painter().layout_no_wrap(g.name.clone(), crate::fonts::sans_font(12.0), TEXT());
                            let (r, resp) = ui.allocate_exact_size(gal.size() + egui::vec2(18.0, 10.0), Sense::click());
                            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                            ui.painter().rect_filled(r, r.height() / 2.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                            ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_TEAL, 0.3 + 0.5 * ht)));
                            ui.painter().galley(r.min + egui::vec2(9.0, 5.0), gal, TEXT());
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                open_place = Some(g.name.clone());
                            }
                        }
                    });
                }
            }
            ui.add_space(12.0);
            // the world's options
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                ui.label(RichText::new("Your world").font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                if toggle_chip(ui, "MORE POKéMON", self.cfg.info_more).on_hover_text("On unless you turned it off: Pokémon Platinum keeps for later turn up earlier").clicked() {
                    flip_more = true;
                }
                if toggle_chip(ui, "MODERN POKéMON", self.cfg.info_modern).on_hover_text("Newer Pokémon in the wild, Megas, and newer moves").clicked() {
                    flip_modern = true;
                }
                ui.label(small("Game Info only shows wild Pokémon your world has."));
            });
        });
        if let Some(s) = set {
            self.cfg.info_stage = s;
            self.cfg.save();
        }
        if flip_more {
            self.cfg.info_more = !self.cfg.info_more;
            self.cfg.save();
        }
        if flip_modern {
            self.cfg.info_modern = !self.cfg.info_modern;
            self.cfg.save();
        }
        if let Some(p) = open_place {
            self.info_push(Page::Place(p));
        }
    }

    // ───────────────────────────── what changed ─────────────────────────────

    /// Compare this wiki copy with the one you saw last. The first copy ever
    /// is just remembered.
    fn fresh_check(&mut self, w: &Arc<Wiki>) {
        let key = Arc::as_ptr(w) as usize;
        if self.setup.info.fresh.key == key {
            return;
        }
        self.setup.info.fresh.key = key;
        let now = Seen::of(w);
        match read_seen() {
            None => {
                write_seen(&now);
                self.setup.info.fresh.changes = None;
            }
            Some(old) if old.version == now.version && old.source == now.source => {
                self.setup.info.fresh.changes = None;
            }
            Some(old) => {
                let c = diff(&old, &now);
                if c.is_empty() {
                    write_seen(&now);
                    self.setup.info.fresh.changes = None;
                } else {
                    self.setup.info.fresh.changes = Some(c);
                }
            }
        }
    }

    /// Front page: "What changed in the game's data" after the wiki updated.
    pub(super) fn info_changes(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        self.fresh_check(w);
        let Some(c) = self.setup.info.fresh.changes.clone() else { return };
        let mut done = false;
        let mut notes = false;
        let mut go: Option<Page> = None;
        let mut dex: Option<String> = None;
        let open = self.setup.info.fresh.open;
        let pl = theme::pal();
        egui::Frame::none()
            .fill(theme::blend(CARD(), pl.accent, if pl.light { 0.08 } else { 0.14 }))
            .stroke(Stroke::new(1.0f32, theme::blend(LINE(), pl.accent, 0.5)))
            .rounding(16.0)
            .inner_margin(egui::Margin::symmetric(20.0, 16.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, "✨", pl.accent, true);
                    ui.add_space(4.0);
                    let title = if c.from != c.to && !c.from.is_empty() {
                        format!("The game's data was updated: {} → {}", c.from, c.to)
                    } else {
                        "The game's data was updated".to_string()
                    };
                    ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        done = ui.add(egui::Button::new(RichText::new("Got it").size(12.5)).fill(kit::btn_fill())).on_hover_text("Hide this until the next update").clicked();
                        notes = ui.link(RichText::new("Patch notes").size(12.5)).clicked();
                    });
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for (n, what) in [
                        (c.species.len(), "new Pokémon"),
                        (c.moves.len(), "new moves"),
                        (c.items.len(), "new items"),
                        (c.places.len(), "new places"),
                        (c.guides.len(), "new guides"),
                        (c.new_spots.len(), "new wild spots"),
                        (c.changed_spots.len(), "changed rates or levels"),
                        (c.gone, "removed"),
                    ] {
                        if n > 0 {
                            stat(ui, &n.to_string(), what);
                        }
                    }
                });
                ui.add_space(6.0);
                if ui.link(RichText::new(if open { "Hide the list" } else { "Show the list" }).size(12.5)).clicked() {
                    self.setup.info.fresh.open = !open;
                }
                if open {
                    ui.add_space(8.0);
                    let mut list = |ui: &mut egui::Ui, title: &str, items: &[(String, String)], make: &dyn Fn(&str) -> Option<Page>, is_mon: bool| {
                        if items.is_empty() {
                            return;
                        }
                        ui.label(RichText::new(title.to_uppercase()).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for (k, name) in items.iter().take(30) {
                                if ui.add(egui::Button::new(RichText::new(name).size(12.0)).fill(kit::btn_fill())).clicked() {
                                    if is_mon {
                                        dex = Some(k.clone());
                                    } else {
                                        go = make(k);
                                    }
                                }
                            }
                            if items.len() > 30 {
                                ui.label(small(format!("+ {} more", items.len() - 30)));
                            }
                        });
                        ui.add_space(8.0);
                    };
                    list(ui, "New Pokémon", &c.species, &|_| None, true);
                    list(ui, "New moves", &c.moves, &|k| Some(Page::Move(k.to_string())), false);
                    list(ui, "New items", &c.items, &|k| Some(Page::Item(k.to_string())), false);
                    list(ui, "New guides", &c.guides, &|k| Some(Page::Guide(k.to_string())), false);
                    // spots: as "Pokémon at Place"
                    let named = |v: &[(String, String)]| -> Vec<(String, String)> {
                        v.iter()
                            .map(|(s, a)| {
                                let mon = w.mon(s).map(|m| m.name.clone()).unwrap_or_else(|| s.clone());
                                let place = w.place(a).map(|p| p.name.clone()).unwrap_or_else(|| a.clone());
                                let group = w.place(a).map(|p| if p.group.is_empty() { p.name.clone() } else { p.group.clone() }).unwrap_or_default();
                                (group, format!("{mon} · {place}"))
                            })
                            .collect()
                    };
                    list(ui, "New wild spots", &named(&c.new_spots), &|k| Some(Page::Place(k.to_string())), false);
                    list(ui, "Changed rates or levels", &named(&c.changed_spots), &|k| Some(Page::Place(k.to_string())), false);
                }
            });
        ui.add_space(18.0);
        if done {
            write_seen(&Seen::of(w));
            self.setup.info.fresh.changes = None;
        }
        if notes {
            self.info_read("whats-new", Some(0), None);
        } else if let Some(Page::Guide(id)) = go.clone() {
            self.info_read(&id, None, None);
        } else if let Some(p) = go {
            self.info_push(p);
        }
        if let Some(id) = dex {
            self.info_open_dex(Some(id));
        }
    }

    /// Advanced › Updates: the game data's version and freshness, and a check.
    pub(super) fn adv_game_data(&mut self, ui: &mut egui::Ui) {
        let check = crate::wiki::check_state();
        let (version, source) = self.wiki.as_ref().map(|w| (w.data.version.clone(), w.data.source.clone())).unwrap_or_default();
        let age = crate::wiki::cache_age().map(|a| a.as_secs());
        let when = match (&check.last, age) {
            _ if check.running => "checking…".to_string(),
            (Some((_, Err(e))), _) => e.clone(),
            (Some((_, Ok(true))), _) => "updated just now".into(),
            (Some((_, Ok(false))), _) => "up to date".into(),
            (None, Some(s)) if s < 3600 => format!("checked {} min ago", (s / 60).max(1)),
            (None, Some(s)) if s < 86_400 => format!("checked {} h ago", s / 3600),
            (None, Some(s)) => format!("checked {} days ago", s / 86_400),
            (None, None) => "the built-in copy".into(),
        };
        let help = "The Pokémon, moves, items, places and guides in Game Info and the Pokédex, from the Platinum Together wiki. Checked once a day by itself.";
        let mut check_now = false;
        super::appearance_page::setting_row(ui, "Game data", help, |ui| {
            check_now = ui.add_enabled(!check.running, egui::Button::new("Check now").fill(kit::btn_fill())).clicked();
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            if version.is_empty() {
                kit::pill(ui, "Not loaded yet", kit::PillKind::Info);
            } else {
                kit::pill(ui, &format!("Platinum Together {version}"), kit::PillKind::Ok).on_hover_text(format!("Wiki build: {source}"));
            }
            ui.label(small(when));
        });
        if check_now {
            self.wiki_rx = crate::wiki::start_refresh();
        }
        if check.running {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        }
    }
}

fn stat(ui: &mut egui::Ui, n: &str, what: &str) {
    let g1 = ui.painter().layout_no_wrap(n.to_string(), crate::fonts::sans_bold_font(14.0), TEXT());
    let g2 = ui.painter().layout_no_wrap(what.to_string(), crate::fonts::sans_font(12.0), MUTED());
    let (r, _) = ui.allocate_exact_size(egui::vec2(g1.size().x + g2.size().x + 30.0, 30.0), Sense::hover());
    ui.painter().rect_filled(r, 15.0, kit::sunken());
    ui.painter().galley(egui::pos2(r.min.x + 12.0, r.center().y - g1.size().y / 2.0), g1.clone(), TEXT());
    ui.painter().galley(egui::pos2(r.min.x + 18.0 + g1.size().x, r.center().y - g2.size().y / 2.0), g2, MUTED());
}

fn toggle_chip(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let label = if on { format!("✔  {text}") } else { text.to_string() };
    let g = ui.painter().layout_no_wrap(label, crate::fonts::sans_bold_font(11.5), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), kit::C_INDIGO, 0.28) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    ui.painter().rect_filled(r, r.height() / 2.0, fill);
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { kit::tint(kit::C_INDIGO, 0.8) } else { LINE() }));
    ui.painter().galley(r.min + egui::vec2(10.0, 6.0), g, if on { TEXT() } else { MUTED() });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_and_lists_diff() {
        let mut a = Seen { version: "0.3.4".into(), ..Default::default() };
        a.species.insert("EEVEE".into(), "Eevee".into());
        a.wild.insert("EEVEE|route_210|grass|".into(), "20|5/5/5".into());
        a.wild.insert("SCYTHER|route_210|grass|".into(), "20|15/5/5".into());
        let mut b = a.clone();
        b.version = "0.3.5".into();
        b.species.insert("SLURPUFF".into(), "Slurpuff".into());
        b.wild.insert("SLURPUFF|route_210|grass|modern".into(), "20|5/5/5".into());
        b.wild.insert("SCYTHER|route_210|grass|".into(), "20|20/5/5".into());
        let c = diff(&a, &b);
        assert_eq!(c.species, vec![("SLURPUFF".to_string(), "Slurpuff".to_string())]);
        assert_eq!(c.new_spots, vec![("SLURPUFF".to_string(), "route_210".to_string())]);
        assert_eq!(c.changed_spots, vec![("SCYTHER".to_string(), "route_210".to_string())]);
        assert_eq!(c.gone, 0);
        assert!(diff(&a, &a).is_empty());
        let mut d = a.clone();
        d.species.clear();
        assert_eq!(diff(&a, &d).gone, 1);
    }

    #[test]
    fn words() {
        assert_eq!(stage_words(0, "Before Roark"), "No badges yet");
        assert_eq!(stage_words(3, "Before Maylene"), "3 badges");
        assert_eq!(stage_words(8, "Before the League"), "All 8 badges");
        assert_eq!(stage_words(9, "After the League"), "Beat the League");
    }
}
