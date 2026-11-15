//! "/" commands in the Ctrl K launcher, mixed and matched with plain words.
//!
//! What to show (pick one or more):
//!   /pokedex (/dex)   Pokémon            /rare        today's rare grass
//!   /moves            moves              /items       items
//!   /places           places             /guides      guides
//!   /gameinfo (/info) moves, items, places and guides
//!   /apps             your apps
//! Narrow it down:
//!   /move surf        learns Surf (how: level, TM, tutor, egg)
//!   /type water       has the Water type (moves: are Water moves)
//!   /in route 210     lives in Route 210
//! Shorthand: "/surf", "/water", "/route 210" work out which they are.
//! Plain words search names. So "/move surf pikachu" checks one Pokémon,
//! "/rare /dragon" is today's Dragon types on rare grass, "/water /ice beam"
//! is Water Pokémon that learn Ice Beam, "/moves pikachu" is Pikachu's moves.
//! Suggestions show while a command is being typed; Tab fills one in.

use super::info_home::{place_groups, Kind};
use super::quick_search::{name_score, Act, QHit};
use super::rare::RareSpot;
use super::store::{app_col, app_icon};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::apps::{self, AppId};
use crate::wiki::Wiki;
use eframe::egui::{self, Color32};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const VIOLET: Color32 = Color32::from_rgb(139, 92, 246);
/// The most results a command lists.
const MAX: usize = 120;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Scope {
    Pokemon,
    Rare,
    Moves,
    Items,
    Places,
    Guides,
    Apps,
}

/// The commands, for suggestions: (what you type, what it does, aliases).
const COMMANDS: &[(&str, &str, &[&str])] = &[
    ("/pokedex", "Pokémon only", &["dex", "pokemon", "pokémon", "mon", "mons"]),
    ("/rare", "Today's rare grass (add a Pokémon, place or type)", &["raregrass", "grass"]),
    ("/move ", "Pokémon that learn a move (/move surf), or a Pokémon's moves (/move pikachu)", &["learn", "learns"]),
    ("/box", "Your boxes, from your save: /box eevee, /box1 eevee", &["boxes", "pc"]),
    ("/party", "Your party, from your save", &["team"]),
    ("/type ", "By type: /type water (Pokémon, moves, rare grass, boxes)", &[]),
    ("/in ", "Pokémon that live in a place: /in route 210", &["at", "where", "found"]),
    ("/moves", "Moves only (/moves pikachu: the moves it learns)", &[]),
    ("/items", "Items only", &["item"]),
    ("/places", "Places only", &["place", "map"]),
    ("/guides", "Guides only", &["guide", "help"]),
    ("/gameinfo", "Moves, items, places and guides", &["info", "wiki"]),
    ("/apps", "Apps (/apps /mini games)", &["app"]),
    ("/shiny", "Shiny ones only (boxes and party)", &["shinies"]),
    ("/new", "First time on rare grass today", &["firsttime"]),
    ("/night", "Out at night (with a place)", &[]),
    ("/pokedex /rare", "Pokémon on rare grass today", &[]),
];

/// Tags that narrow things down: (word, what it does).
const TAGS: &[(&str, &str)] = &[
    ("shiny", "Shiny ones only (boxes, party)"),
    ("new", "First time on rare grass today"),
    ("morning", "Out in the morning (4–10)"),
    ("day", "Out in the day (10–20)"),
    ("night", "Out at night (20–4)"),
    ("physical", "Physical moves"),
    ("special", "Special moves"),
    ("status", "Status moves"),
    ("31", "At least one perfect IV (boxes)"),
    ("6iv", "All six IVs perfect (boxes)"),
    ("5iv", "Five or more perfect IVs (boxes)"),
    ("4iv", "Four or more perfect IVs (boxes)"),
    ("hunting", "Hunting apps"),
    ("explore", "Explore apps"),
    ("battle", "Battle apps"),
    ("breeding", "Breeding apps"),
    ("mini games", "Mini game apps"),
];

/// Lower case, letters and digits only ("Ice Beam" = "icebeam").
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// What was typed, worked out.
#[derive(Default, Debug)]
struct Parsed {
    scopes: BTreeSet<Scope>,
    types: Vec<&'static str>,
    /// (move key, its name)
    moves: Vec<(String, String)>,
    /// place group names
    places: Vec<String>,
    /// Pokémon ids
    mons: Vec<(String, String)>,
    /// "/move pikachu": the moves a Pokémon learns
    want_moves: bool,
    /// "/move" came first: a name that's a move and a type is the move
    move_cmd: bool,
    /// your boxes (and which one), your party
    boxes: bool,
    box_no: Option<usize>,
    party: bool,
    shiny: bool,
    natures: Vec<&'static str>,
    /// at least this many perfect (31) IVs
    perfect: Option<u8>,
    /// morning 0, day 1, night 2
    times: Vec<usize>,
    /// move categories: Physical, Special, Status
    cats: Vec<&'static str>,
    app_cats: Vec<apps::Cat>,
    /// first time on rare grass
    new_rare: bool,
    words: Vec<String>,
    /// "/xyz" that means nothing
    unknown: Vec<String>,
}

impl Parsed {
    fn summary(&self) -> String {
        let mut v: Vec<String> = Vec::new();
        for t in &self.types {
            v.push(format!("{t} type"));
        }
        for (_, n) in &self.moves {
            v.push(format!("learn {n}"));
        }
        for p in &self.places {
            v.push(format!("in {p}"));
        }
        for (_, n) in &self.mons {
            v.push(n.clone());
        }
        if self.shiny {
            v.push("shiny".into());
        }
        for n in &self.natures {
            v.push(n.to_string());
        }
        if let Some(n) = self.perfect {
            v.push(if n == 1 { "a perfect IV".into() } else { format!("{n}+ perfect IVs") });
        }
        for t in &self.times {
            v.push(["morning", "day", "night"][*t].into());
        }
        for c in &self.cats {
            v.push(c.to_lowercase());
        }
        for c in &self.app_cats {
            v.push(c.name().to_lowercase());
        }
        if self.new_rare {
            v.push("first time".into());
        }
        if !self.words.is_empty() {
            v.push(format!("\"{}\"", self.words.join(" ")));
        }
        v.join(" · ")
    }
}

/// Finders over the wiki's names.
struct Names<'a> {
    w: &'a Wiki,
    moves: BTreeMap<String, (String, String)>,
    places: BTreeMap<String, String>,
    /// normalised name → (id, name)
    mons: BTreeMap<String, (String, String)>,
}

impl<'a> Names<'a> {
    fn new(w: &'a Wiki) -> Self {
        let moves = w.data.moves.iter().map(|(k, m)| (norm(&m.n), (k.clone(), m.n.clone()))).collect();
        let places = place_groups(w).into_keys().map(|g| (norm(&g), g)).collect();
        let mons = w.data.species.iter().map(|m| (norm(&m.name), (m.id.clone(), m.name.clone()))).collect();
        Self { w, moves, places, mons }
    }
    fn ty(&self, s: &str) -> Option<&'static str> {
        let n = norm(s);
        crate::dexextra::TYPES.iter().find(|t| norm(t) == n).copied()
    }
    fn mv(&self, s: &str) -> Option<(String, String)> {
        self.moves.get(&norm(s)).cloned()
    }
    fn place(&self, s: &str) -> Option<String> {
        self.places.get(&norm(s)).cloned()
    }
    /// Move names starting with `n` (normalised), shortest first.
    fn moves_starting(&self, n: &str, k: usize) -> Vec<String> {
        let mut v: Vec<&String> = self.moves.iter().filter(|(key, _)| key.starts_with(n)).map(|(_, (_, name))| name).collect();
        v.sort_by_key(|s| (s.len(), s.to_string()));
        v.into_iter().take(k).cloned().collect()
    }
    /// Place names starting with `n` (normalised), shortest first.
    fn places_starting(&self, n: &str, k: usize) -> Vec<String> {
        let mut v: Vec<&String> = self.places.iter().filter(|(key, _)| key.starts_with(n)).map(|(_, g)| g).collect();
        v.sort_by_key(|s| (s.len(), s.to_string()));
        v.into_iter().take(k).cloned().collect()
    }
    fn mon(&self, s: &str) -> Option<(String, String)> {
        let _ = self.w;
        self.mons.get(&norm(s)).cloned()
    }
    /// Every kind of name starting with `n`: (what to put in the box, what it is).
    fn starting(&self, n: &str, k: usize) -> Vec<(String, String, &'static str, Color32)> {
        let mut v: Vec<(String, String, &'static str, Color32)> = Vec::new();
        for t in crate::dexextra::TYPES.iter().filter(|t| norm(t).starts_with(n)) {
            v.push((t.to_string(), format!("{t} type"), "◆", super::pokedex::type_color(t)));
        }
        for name in self.moves_starting(n, k) {
            v.push((name.clone(), format!("Move: Pokémon that learn {name}"), "💥", kit::C_VIOLET));
        }
        for g in self.places_starting(n, k) {
            v.push((g.clone(), format!("Place: Pokémon in {g}"), "🗺", kit::C_GREEN));
        }
        for (t, what) in TAGS.iter().filter(|(t, _)| norm(t).starts_with(n)) {
            v.push((t.to_string(), what.to_string(), "#", kit::C_SLATE));
        }
        for nat in crate::gamesave::NATURES.iter().filter(|x| norm(x).starts_with(n)).take(2) {
            v.push((nat.to_string(), "Nature (boxes and party)".into(), "#", kit::C_SLATE));
        }
        let mut ms: Vec<&(String, String)> = self.mons.iter().filter(|(key, _)| key.starts_with(n)).map(|(_, m)| m).collect();
        ms.sort_by_key(|(_, name)| (name.len(), name.clone()));
        for (_, name) in ms.into_iter().take(k) {
            v.push((name.clone(), "Pokémon".into(), "📖", kit::C_RED));
        }
        v.truncate(k + 2);
        v
    }
}

/// A tag word ("shiny", "night", "adamant", "6iv"…): true if it was one.
fn read_tag(s: &str, p: &mut Parsed) -> bool {
    let n = norm(s);
    match n.as_str() {
        "shiny" | "shinies" => p.shiny = true,
        "new" | "firsttime" => p.new_rare = true,
        "morning" => p.times.push(0),
        "day" | "daytime" => p.times.push(1),
        "night" | "nighttime" => p.times.push(2),
        "physical" => p.cats.push("Physical"),
        "special" => p.cats.push("Special"),
        "status" => p.cats.push("Status"),
        "31" | "perfect" | "perfectiv" => p.perfect = Some(p.perfect.unwrap_or(0).max(1)),
        _ => {
            if let Some(k) = n.strip_suffix("iv").and_then(|d| d.parse::<u8>().ok()).filter(|k| (1..=6).contains(k)) {
                p.perfect = Some(k);
            } else if let Some(nat) = crate::gamesave::NATURES.iter().find(|x| norm(x) == n) {
                p.natures.push(nat);
            } else if let Some(c) = apps::Cat::ALL.iter().find(|c| norm(c.name()) == n || (n == "games" && **c == apps::Cat::Games)) {
                p.app_cats.push(*c);
            } else {
                return false;
            }
        }
    }
    true
}

/// Words read left to right, the longest name winning: a type, a move, a
/// place or a Pokémon becomes a filter, anything else stays a name search.
/// With `move_first` (after /move) a name that's both a move and a type
/// ("Psychic") is the move. True if it found anything.
fn read_words(words: &[&str], names: &Names, p: &mut Parsed, move_first: bool) -> bool {
    let mut any = false;
    let mut i = 0;
    while i < words.len() {
        let mut hit = false;
        for k in (1..=(words.len() - i).min(5)).rev() {
            let s = words[i..i + k].join(" ");
            if read_tag(&s, p) {
                i += k;
                hit = true;
                any = true;
                break;
            }
            let ty = || names.ty(&s);
            let mv = || names.mv(&s);
            let found = if move_first {
                if let Some(m) = mv() {
                    p.moves.push(m);
                    true
                } else if let Some(t) = ty() {
                    p.types.push(t);
                    true
                } else {
                    false
                }
            } else if let Some(t) = ty() {
                p.types.push(t);
                true
            } else if let Some(m) = mv() {
                p.moves.push(m);
                true
            } else {
                false
            };
            let found = found
                || if let Some(g) = names.place(&s) {
                    p.places.push(g);
                    true
                } else if let Some(m) = names.mon(&s) {
                    p.mons.push(m);
                    true
                } else {
                    false
                };
            if found {
                i += k;
                hit = true;
                any = true;
                break;
            }
        }
        if !hit {
            p.words.push(words[i].to_string());
            i += 1;
        }
    }
    any
}

fn parse(q: &str, names: &Names) -> Parsed {
    let mut p = Parsed::default();
    let mut parts = q.split('/');
    let lead: Vec<&str> = parts.next().unwrap_or("").split_whitespace().collect();
    read_words(&lead, names, &mut p, false);
    for part in parts {
        let words: Vec<&str> = part.split_whitespace().collect();
        let Some(cmd) = words.first().map(|w| w.to_lowercase()) else { continue };
        let rest: Vec<&str> = words[1..].to_vec();
        let scope = match cmd.as_str() {
            "pokedex" | "dex" | "pokemon" | "pokémon" | "mon" | "mons" => Some(Scope::Pokemon),
            "rare" | "raregrass" | "grass" => Some(Scope::Rare),
            "moves" => Some(Scope::Moves),
            "items" | "item" => Some(Scope::Items),
            "places" | "place" | "map" => Some(Scope::Places),
            "guides" | "guide" | "help" => Some(Scope::Guides),
            "apps" | "app" => Some(Scope::Apps),
            _ => None,
        };
        // your boxes: "/box", "/box1", "/box 1", "/party"
        let box_cmd = cmd.strip_prefix("box").filter(|r| !r.is_empty()).and_then(|r| r.parse::<usize>().ok());
        if box_cmd.is_some() || matches!(cmd.as_str(), "box" | "boxes" | "pc" | "party" | "team") {
            if matches!(cmd.as_str(), "party" | "team") {
                p.party = true;
            } else {
                p.boxes = true;
            }
            let mut rest = rest.clone();
            let no = box_cmd.or_else(|| rest.first().and_then(|w| w.parse::<usize>().ok()).inspect(|_| {
                rest.remove(0);
            }));
            if let Some(n) = no.filter(|n| *n >= 1) {
                p.box_no = Some(n - 1);
            }
            read_words(&rest, names, &mut p, false);
            continue;
        }
        if let Some(sc) = scope {
            p.scopes.insert(sc);
            // items, places, guides and apps: the words are names to find
            if matches!(sc, Scope::Items | Scope::Places | Scope::Guides | Scope::Apps) {
                p.words.extend(rest.iter().map(|s| s.to_string()));
            } else {
                read_words(&rest, names, &mut p, sc == Scope::Moves);
            }
            continue;
        }
        if matches!(cmd.as_str(), "gameinfo" | "info" | "wiki") {
            p.scopes.extend([Scope::Moves, Scope::Items, Scope::Places, Scope::Guides]);
            p.words.extend(rest.iter().map(|s| s.to_string()));
            continue;
        }
        match cmd.as_str() {
            // "/move surf", "/move flamethrower route 210", "/move pikachu"
            "move" | "learn" | "learns" => {
                p.move_cmd = true;
                read_words(&rest, names, &mut p, true);
                p.want_moves = true;
            }
            "type" | "in" | "at" | "where" | "found" => {
                read_words(&rest, names, &mut p, false);
            }
            // shorthand: "/surf", "/water", "/route 210", "/pikachu"
            _ => {
                let before = p.words.len();
                let move_first = p.move_cmd;
                if !read_words(&words, names, &mut p, move_first) || p.words.get(before).is_some_and(|w| w.eq_ignore_ascii_case(words[0])) {
                    p.words.truncate(before);
                    p.unknown.push(format!("/{}", part.trim()));
                }
            }
        }
    }
    // "/move" with a move anywhere is Pokémon that learn it; without one,
    // it's a Pokémon's moves
    if !p.moves.is_empty() {
        p.want_moves = false;
    }
    p
}

/// How a Pokémon learns a move ("Lv. 30", "TM03", "Tutor", "Egg move"), if it does.
fn learns_how(w: &Wiki, mon: &str, key: &str) -> Option<String> {
    let l = w.data.learnsets.get(mon)?;
    let tm: Vec<&String> = w.data.tms.iter().filter(|(_, v)| v.as_str() == key).map(|(k, _)| k).collect();
    let mut how: Vec<String> = Vec::new();
    if let Some(x) = l.lv.iter().chain(l.mlv.iter()).find(|x| x.key == key) {
        how.push(if x.level.chars().all(|c| c.is_ascii_digit()) && !x.level.is_empty() { format!("Lv. {}", x.level) } else { x.level.clone() });
    }
    if l.tm.iter().chain(l.mtm.iter()).any(|x| x == key || tm.iter().any(|t| t.as_str() == x.as_str())) {
        how.push(tm.first().map_or("TM".to_string(), |t| t.to_string()));
    }
    if l.tutor.iter().chain(l.mtutor.iter()).any(|x| x == key) {
        how.push("Tutor".into());
    }
    if l.egg.iter().chain(l.megg.iter()).any(|x| x == key) {
        how.push("Egg move".into());
    }
    (!how.is_empty()).then(|| how.join(" · "))
}

/// Every move a Pokémon learns: (key, how).
fn moves_of(w: &Wiki, mon: &str) -> Vec<(String, String)> {
    w.data.moves.keys().filter_map(|k| learns_how(w, mon, k).map(|h| (k.clone(), h))).collect()
}

/// The text box with its last (unfinished) "/word" swapped for `with`.
fn replace_last(q: &str, with: &str) -> String {
    match q.rfind('/') {
        Some(i) => format!("{}{with}", &q[..i]),
        None => with.to_string(),
    }
}

impl App {
    /// Results (and suggestions) for a query that starts with "/".
    pub(super) fn command_hits(&mut self, q: &str) -> Vec<QHit> {
        let Some(w) = self.wiki.clone().filter(|w| !w.is_empty()) else {
            return vec![QHit {
                group: "Commands".into(),
                icon: "⌛".into(),
                col: kit::C_SLATE,
                title: "Game Info's data is still loading".into(),
                sub: "Commands search it: try again in a moment.".into(),
                act: Act::Fill(q.to_string()),
                dex: None,
            }];
        };
        let names = Names::new(&w);
        let suggest = self.command_suggestions(q, &names);
        let mut p = parse(q, &names);
        // the "/word" still being typed isn't "unknown" yet
        let typing = (!q.ends_with(' ')).then(|| q.rfind('/').map(|i| q[i..].trim().to_string())).flatten();
        let mut out = Vec::new();
        if let Some(t) = &typing {
            if p.unknown.last() == Some(t) && !suggest.is_empty() {
                p.unknown.pop();
            }
        }
        // a finished word: its results first (Enter opens them); otherwise
        // the suggestions first (Tab or Enter fills one in)
        let finished = typing.as_ref().map_or(true, |t| !p.unknown.contains(t) && suggest.iter().any(|h| matches!(&h.act, Act::Fill(f) if norm(f.trim_end().rsplit('/').next().unwrap_or("")) == norm(t.trim_start_matches('/')))));
        if !finished {
            out.extend(suggest.iter().cloned());
        }
        for u in &p.unknown {
            out.push(QHit {
                group: "Not sure what that is".into(),
                icon: "?".into(),
                col: kit::C_ORANGE,
                title: format!("Nothing called {u}"),
                sub: "Try /pokedex, /rare, /move, /type, /in, or a move, type or place name".into(),
                act: Act::Fill(q.replace(u.as_str(), "").trim_end().to_string() + " "),
                dex: None,
            });
        }
        let results = self.command_results(&w, &p);
        let nothing = results.len() == 1 && results[0].title == "Nothing matches all of that";
        if !(nothing && !suggest.is_empty() && !finished) {
            out.extend(results);
        }
        if finished {
            out.extend(suggest);
        }
        out
    }

    /// While a "/word" is being typed: what it could be (each once).
    fn command_suggestions(&self, q: &str, names: &Names) -> Vec<QHit> {
        let mut out = self.command_suggestions_all(q, names);
        let mut seen = BTreeSet::new();
        out.retain(|h| seen.insert(h.title.trim().to_lowercase()));
        out
    }

    fn command_suggestions_all(&self, q: &str, names: &Names) -> Vec<QHit> {
        let mut out = Vec::new();
        if q.ends_with(' ') {
            return out;
        }
        let Some(i) = q.rfind('/') else { return out };
        let last = &q[i + 1..];
        let (cmd, rest) = match last.split_once(' ') {
            Some((c, r)) => (c.to_lowercase(), Some(r)),
            None => (last.to_lowercase(), None),
        };
        let fill = |title: String, sub: String, icon: &str, col: Color32, text: String| QHit { group: "Suggestions".into(), icon: icon.into(), col, title, sub, act: Act::Fill(text), dex: None };
        match rest {
            // a command's name being typed
            None => {
                for &(c, what, aliases) in COMMANDS {
                    let bare = c.trim_start_matches('/').trim_end();
                    let hit = cmd.is_empty() || bare.starts_with(&cmd) || aliases.iter().any(|a| a.starts_with(&cmd));
                    if hit && !(c.contains(" /") && !cmd.is_empty()) {
                        out.push(fill(c.trim_end().to_string(), what.to_string(), "⌘", VIOLET, replace_last(q, c) + if c.ends_with(' ') { "" } else { " " }));
                    }
                }
                if cmd.chars().count() >= 2 {
                    let n = norm(&cmd);
                    for (t, what) in TAGS.iter().filter(|(t, _)| norm(t).starts_with(&n)).take(3) {
                        out.push(fill(format!("/{t}"), what.to_string(), "#", kit::C_SLATE, replace_last(q, &format!("/{t} "))));
                    }
                    for t in crate::dexextra::TYPES.iter().filter(|t| norm(t).starts_with(&n)).take(3) {
                        out.push(fill(format!("/{}", t.to_lowercase()), format!("{t} type"), "◆", super::pokedex::type_color(t), replace_last(q, &format!("/{} ", t.to_lowercase()))));
                    }
                    for name in names.moves_starting(&n, 4) {
                        out.push(fill(format!("/{}", name.to_lowercase()), format!("Pokémon that learn {name}"), "💥", kit::C_VIOLET, replace_last(q, &format!("/{} ", name.to_lowercase()))));
                    }
                    for g in names.places_starting(&n, 3) {
                        out.push(fill(format!("/{}", g.to_lowercase()), format!("Pokémon in {g}"), "🗺", kit::C_GREEN, replace_last(q, &format!("/{} ", g.to_lowercase()))));
                    }
                }
            }
            // after a command, the last word(s) being typed: a type, move,
            // place or Pokémon ("/move su", "/move flamethrower rou", "/rare dra")
            Some(r) => {
                let ws: Vec<&str> = r.split_whitespace().collect();
                if ws.is_empty() || matches!(cmd.as_str(), "items" | "item" | "places" | "place" | "map" | "guides" | "guide" | "help" | "apps" | "app" | "gameinfo" | "info" | "wiki") {
                    return out;
                }
                // the longest tail that starts a name but isn't one yet
                for k in (1..=ws.len().min(3)).rev() {
                    let tail = ws[ws.len() - k..].join(" ");
                    let n = norm(&tail);
                    if n.is_empty() {
                        continue;
                    }
                    let found = names.starting(&n, 5);
                    let exact = found.iter().any(|(name, ..)| norm(name) == n);
                    if found.is_empty() || (exact && found.len() == 1) {
                        continue;
                    }
                    let start = q.rfind(ws[ws.len() - k]).unwrap_or(q.len());
                    let head = &q[..start];
                    for (name, what, icon, col) in found {
                        out.push(fill(name.clone(), what, icon, col, format!("{head}{} ", name.to_lowercase())));
                    }
                    break;
                }
            }
        }
        out
    }

    /// The results for what was typed.
    fn command_results(&mut self, w: &Arc<Wiki>, p: &Parsed) -> Vec<QHit> {
        let filtered = !p.types.is_empty() || !p.moves.is_empty() || !p.places.is_empty() || !p.mons.is_empty() || !p.times.is_empty();
        let mut scopes = p.scopes.clone();
        // your boxes / party: Pokémon names and tags narrow those down
        let in_save = p.boxes || p.party;
        if in_save {
            scopes.remove(&Scope::Pokemon);
        }
        if p.want_moves && !in_save {
            scopes.insert(Scope::Moves);
        }
        if scopes.is_empty() && in_save {
            return self.save_results(w, p);
        }
        if p.new_rare && scopes.is_empty() {
            scopes.insert(Scope::Rare);
        }
        if !p.cats.is_empty() && scopes.is_empty() {
            scopes.insert(Scope::Moves);
        }
        if !p.app_cats.is_empty() && scopes.is_empty() {
            scopes.insert(Scope::Apps);
        }
        if scopes.is_empty() {
            if !filtered {
                return Vec::new();
            }
            scopes.insert(Scope::Pokemon);
        }
        let only_mons: BTreeSet<&str> = p.mons.iter().map(|(id, _)| id.as_str()).collect();
        if p.want_moves && p.scopes.is_empty() && p.moves.is_empty() && !p.mons.is_empty() {
            scopes.remove(&Scope::Pokemon);
        }
        // "/pokedex /rare": Pokémon on rare grass today
        let on_rare = scopes.contains(&Scope::Pokemon) && scopes.contains(&Scope::Rare);
        let words = p.words.join(" ").to_lowercase();
        let summary = p.summary();
        let label = |what: &str, n: usize| {
            let more = if n > MAX { format!(" · first {MAX} of {n}") } else { format!(" · {n}") };
            if summary.is_empty() {
                format!("{what}{more}")
            } else {
                format!("{what} · {summary}{more}")
            }
        };
        let mut out = Vec::new();
        let have_dex = self.cfg.apps.is_installed(AppId::Pokedex);
        let have_info = self.cfg.apps.is_installed(AppId::Info);
        let mon_ok = |m: &crate::wiki::Mon| -> Option<String> {
            if !p.types.iter().all(|t| m.types.iter().any(|x| x.eq_ignore_ascii_case(t))) {
                return None;
            }
            let mut how: Vec<String> = Vec::new();
            for (k, n) in &p.moves {
                how.push(format!("{n}: {}", learns_how(w, &m.id, k)?));
            }
            Some(how.join(" · "))
        };
        // lives in every place asked for, out at every time asked for
        let at_time = |e: &crate::wiki::Wild| p.times.iter().all(|t| [e.chance.morning, e.chance.day, e.chance.night][*t] > 0);
        let in_places = |id: &str| -> bool {
            let wild = w.wild(id);
            if p.places.is_empty() {
                return p.times.is_empty() || wild.iter().any(|e| at_time(e));
            }
            p.places.iter().all(|g| wild.iter().any(|e| at_time(e) && w.place(&e.area).is_some_and(|pl| (if pl.group.is_empty() { &pl.name } else { &pl.group }) == g)))
        };
        let rare_today: Vec<RareSpot> = if scopes.contains(&Scope::Rare) { self.rare_spots_today() } else { Vec::new() };
        // first time on rare grass: seen on 1 day (today)
        let firsts: BTreeSet<String> = if p.new_rare { self.rare_out_today().into_iter().filter(|o| o.days == 1).map(|o| o.spots[0].species.slug.clone()).collect() } else { BTreeSet::new() };

        if scopes.contains(&Scope::Pokemon) {
            let rare_dex: BTreeSet<u32> = rare_today.iter().map(|s| s.species.dex).collect();
            let mut found: Vec<(u32, QHit)> = Vec::new();
            for &i in &w.order {
                let m = &w.data.species[i];
                let Some(how) = mon_ok(m) else { continue };
                if !in_places(&m.id) || (on_rare && !rare_dex.contains(&m.dex)) || (!only_mons.is_empty() && !only_mons.contains(m.id.as_str())) {
                    continue;
                }
                let Some(sc) = (if words.is_empty() { Some(0) } else { name_score(&m.name, &words).or_else(|| (words.trim_start_matches('#').parse::<u32>().ok() == Some(m.dex)).then_some(0)) }) else { continue };
                let mut sub = format!("#{:04} · {}", m.dex, m.types.join(" / "));
                if !how.is_empty() {
                    sub = format!("{how} · {sub}");
                }
                if !have_dex {
                    sub.push_str(" · get the Pokédex to open");
                }
                found.push((sc, QHit { group: String::new(), icon: "📖".into(), col: kit::C_RED, title: m.name.clone(), sub, act: Act::Info(Kind::Pokemon, m.id.clone(), AppId::Pokedex), dex: Some(m.dex) }));
            }
            found.sort_by_key(|(sc, _)| *sc);
            let g = label(if on_rare { "Pokémon on rare grass today" } else { "Pokémon" }, found.len());
            out.extend(found.into_iter().take(MAX).map(|(_, mut h)| {
                h.group = g.clone();
                h
            }));
        }
        if scopes.contains(&Scope::Rare) && !on_rare {
            let mut found: Vec<QHit> = Vec::new();
            for s in rare_today {
                let Some(m) = w.by_dex(s.species.dex) else { continue };
                let Some(how) = mon_ok(m) else { continue };
                if (!only_mons.is_empty() && !only_mons.contains(m.id.as_str())) || (p.new_rare && !firsts.contains(&s.species.slug)) {
                    continue;
                }
                if !p.places.iter().all(|g| s.place == *g || s.place.starts_with(&format!("{g} ")) || s.place.starts_with(&format!("{g}("))) {
                    continue;
                }
                if !words.is_empty() && name_score(s.species.name, &words).is_none() && name_score(&s.place, &words).is_none() {
                    continue;
                }
                let tiles = if s.tiles == 1 { "1 tile".to_string() } else { format!("{} tiles", s.tiles) };
                let sub = if how.is_empty() { format!("{} · {tiles}", s.place) } else { format!("{how} · {} · {tiles}", s.place) };
                found.push(QHit { group: String::new(), icon: "✨".into(), col: kit::C_GREEN, title: format!("{} on rare grass", s.species.name), sub, act: Act::Rare(Box::new(s.clone())), dex: Some(s.species.dex) });
            }
            let g = label("Rare grass today", found.len());
            out.extend(found.into_iter().take(MAX).map(|mut h| {
                h.group = g.clone();
                h
            }));
        }
        if scopes.contains(&Scope::Moves) {
            // "/moves pikachu": the moves a Pokémon learns
            let names = Names::new(w);
            let of = p.mons.first().map(|(id, _)| id.clone()).or_else(|| names.mon(&words).map(|m| m.0)).map(|id| (id.clone(), moves_of(w, &id)));
            let only_moves: BTreeSet<&str> = p.moves.iter().map(|(k, _)| k.as_str()).collect();
            let mut found: Vec<(u32, String, QHit)> = Vec::new();
            for (k, mv) in &w.data.moves {
                if !p.types.iter().all(|t| mv.t.eq_ignore_ascii_case(t)) || (!only_moves.is_empty() && !only_moves.contains(k.as_str())) || !p.cats.iter().all(|c| mv.c.eq_ignore_ascii_case(c)) {
                    continue;
                }
                let (sc, how) = match &of {
                    Some((_, list)) => match list.iter().find(|(key, _)| key == k) {
                        Some((_, h)) => (0, Some(h.clone())),
                        None => continue,
                    },
                    None => match if words.is_empty() || !only_moves.is_empty() { Some(0) } else { name_score(&mv.n, &words) } {
                        Some(sc) => (sc, None),
                        None => continue,
                    },
                };
                let power = if mv.p.is_empty() || mv.p == "0" { String::new() } else { format!(" · {} power", mv.p) };
                let mut sub = format!("{} · {}{power}", mv.t, mv.c);
                if let Some(h) = how {
                    sub = format!("{h} · {sub}");
                }
                found.push((sc, mv.n.clone(), QHit { group: String::new(), icon: "💥".into(), col: super::pokedex::type_color(&mv.t), title: mv.n.clone(), sub, act: Act::Info(Kind::Move, k.clone(), AppId::Info), dex: None }));
            }
            found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let what = match &of {
                Some((id, _)) => format!("Moves {} learns", w.mon(id).map_or(id.clone(), |m| m.name.clone())),
                None => "Moves".into(),
            };
            let g = match &of {
                Some((id, _)) => {
                    let name = w.mon(id).map_or(id.clone(), |m| m.name.clone());
                    label(&what, found.len()).replace(&format!(" · {name} · "), " · ").replace(&format!(" · {name}"), "")
                }
                None => label(&what, found.len()),
            };
            out.extend(found.into_iter().take(MAX).map(|(_, _, mut h)| {
                h.group = g.clone();
                h
            }));
        }
        for (scope, kind, what) in [(Scope::Items, Kind::Item, "Items"), (Scope::Places, Kind::Place, "Places"), (Scope::Guides, Kind::Guide, "Guides")] {
            if !scopes.contains(&scope) {
                continue;
            }
            let hits: Vec<_> = self.info_quick(w, &words, 4000).into_iter().filter(|h| h.0 == kind).collect();
            let g = label(what, hits.len());
            for (k, key, icon, col, title, sub) in hits.into_iter().take(MAX) {
                let sub = if have_info { sub } else { format!("{sub} · get Game Info to open") };
                out.push(QHit { group: g.clone(), icon: icon.to_string(), col, title, sub, act: Act::Info(k, key, AppId::Info), dex: None });
            }
        }
        if scopes.contains(&Scope::Apps) {
            // "/apps /mini games" (or "/apps mini games"): one kind of app
            let mut cats = p.app_cats.clone();
            if let Some(c) = apps::Cat::ALL.iter().find(|c| !words.is_empty() && norm(c.name()) == norm(&words)) {
                cats.push(*c);
            }
            let words = if cats.len() > p.app_cats.len() { String::new() } else { words.clone() };
            let mut found: Vec<(u32, AppId)> = apps::ALL
                .iter()
                .filter(|a| cats.iter().all(|c| apps::meta(**a).cat == *c))
                .filter_map(|a| (if words.is_empty() { Some(0) } else { name_score(a.name(), &words) }).map(|s| (s, *a)))
                .collect();
            found.sort();
            let g = label("Apps", found.len());
            for (_, a) in found {
                let have = self.cfg.apps.is_installed(a);
                let sub = format!("{}{}", apps::meta(a).cat.name(), if have { "" } else { " · in the App Store" });
                out.push(QHit { group: g.clone(), icon: app_icon(a).to_string(), col: app_col(a), title: a.name().to_string(), sub, act: if have { Act::App(a) } else { Act::Store(a) }, dex: None });
            }
        }
        if out.is_empty() {
            out.push(QHit {
                group: label("Nothing matches", 0),
                icon: "∅".into(),
                col: kit::C_SLATE,
                title: "Nothing matches all of that".into(),
                sub: "Take a filter away, or check the spelling".into(),
                act: Act::Fill(String::new()),
                dex: None,
            });
        }
        out
    }
}

impl App {
    /// "/box…" and "/party…": your own Pokémon, from the save (read only).
    fn save_results(&mut self, w: &Arc<Wiki>, p: &Parsed) -> Vec<QHit> {
        let Some(save) = self.setup.boxes.save.clone() else {
            return vec![QHit {
                group: "Your boxes".into(),
                icon: "📦".into(),
                col: super::boxes_app::BOX_COL,
                title: "Reading your save…".into(),
                sub: "It's read from the game's save file (read only). Choose the game folder in Settings if it doesn't show.".into(),
                act: Act::Fill(String::new()),
                dex: None,
            }];
        };
        let text = self.setup.boxes.text.clone();
        let words = p.words.join(" ").to_lowercase();
        let only_mons: BTreeSet<&str> = p.mons.iter().map(|(id, _)| id.as_str()).collect();
        // (where, sel, Pokémon)
        let mut all: Vec<(String, super::boxes_app::Sel, crate::gamesave::Mon)> = Vec::new();
        if p.party {
            all.extend(save.party.iter().enumerate().map(|(i, m)| ("Party".to_string(), super::boxes_app::Sel::Party(i), m.clone())));
        }
        if p.boxes {
            for (bi, b) in save.boxes.iter().enumerate() {
                if p.box_no.is_some_and(|n| n != bi) {
                    continue;
                }
                for (si, m) in b.slots.iter().enumerate() {
                    if let Some(m) = m {
                        all.push((b.name.clone(), super::boxes_app::Sel::Slot(bi, si), m.clone()));
                    }
                }
            }
        }
        let mut found: Vec<(u32, QHit)> = Vec::new();
        for (wher, sel, m) in all {
            if m.egg {
                continue;
            }
            let Some(sp) = self.box_species(m.species) else { continue };
            let mon = w.by_dex(sp.dex);
            if !only_mons.is_empty() && !mon.is_some_and(|x| only_mons.contains(x.id.as_str())) {
                continue;
            }
            if !p.types.iter().all(|t| mon.is_some_and(|x| x.types.iter().any(|y| y.eq_ignore_ascii_case(t)))) {
                continue;
            }
            if p.shiny && !m.shiny() {
                continue;
            }
            let nature = crate::gamesave::NATURES[m.nature() as usize % 25];
            if !p.natures.is_empty() && !p.natures.contains(&nature) {
                continue;
            }
            let perfect = m.ivs.iter().filter(|v| **v == 31).count() as u8;
            if p.perfect.is_some_and(|n| perfect < n) {
                continue;
            }
            // knows these moves (by name, from the game's text)
            let known: Vec<String> = m.moves.iter().filter(|x| **x != 0).map(|x| text.as_ref().map_or(format!("#{x}"), |t| t.move_name(*x))).collect();
            if !p.moves.iter().all(|(_, n)| known.iter().any(|k| norm(k) == norm(n))) {
                continue;
            }
            let nick = (m.nicknamed && !m.nickname.is_empty()).then(|| m.nickname.clone());
            let sc = if words.is_empty() {
                Some(0)
            } else {
                name_score(sp.name, &words).or_else(|| nick.as_deref().and_then(|n| name_score(n, &words)))
            };
            let Some(sc) = sc else { continue };
            let mut sub = vec![wher.clone()];
            if let Some(l) = m.level {
                sub.push(format!("Lv {l}"));
            }
            if let Some(n) = &nick {
                sub.push(format!("\"{n}\""));
            }
            sub.push(nature.to_string());
            if perfect > 0 {
                sub.push(format!("{perfect}×31"));
            }
            if !p.moves.is_empty() {
                sub.push(p.moves.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(", "));
            }
            let title = if m.shiny() { format!("{} ★", sp.name) } else { sp.name.to_string() };
            found.push((sc, QHit { group: String::new(), icon: "📦".into(), col: super::boxes_app::BOX_COL, title, sub: sub.join(" · "), act: Act::Boxed(sel), dex: Some(sp.dex) }));
        }
        found.sort_by_key(|(sc, _)| *sc);
        let what = match (p.party, p.boxes, p.box_no) {
            (true, false, _) => "Your party".to_string(),
            (_, true, Some(n)) => save.boxes.get(n).map_or(format!("Box {}", n + 1), |b| b.name.clone()),
            (true, true, None) => "Your party and boxes".into(),
            _ => "Your boxes".into(),
        };
        let summary = p.summary();
        let g = if summary.is_empty() { format!("{what} · {}", found.len()) } else { format!("{what} · {summary} · {}", found.len()) };
        let mut out: Vec<QHit> = found
            .into_iter()
            .take(MAX)
            .map(|(_, mut h)| {
                h.group = g.clone();
                h
            })
            .collect();
        if out.is_empty() {
            out.push(QHit { group: g, icon: "∅".into(), col: kit::C_SLATE, title: "None of yours match".into(), sub: "Take a filter away, or check the spelling".into(), act: Act::Fill(String::new()), dex: None });
        }
        out
    }
}

/// The quick look for a command suggestion: how commands work.
pub(super) fn command_help(ui: &mut egui::Ui) {
    let h = |ui: &mut egui::Ui, t: &str| {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
    };
    let line = |ui: &mut egui::Ui, a: &str, b: &str| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(a).monospace().size(12.0).color(TEXT()));
            ui.label(egui::RichText::new(b).size(12.0).color(MUTED()));
        });
    };
    ui.add(egui::Label::new(egui::RichText::new("Mix commands and words: each one narrows it down.").size(12.5).color(TEXT())).wrap());
    h(ui, "WHAT TO SHOW");
    line(ui, "/pokedex", "Pokémon");
    line(ui, "/rare", "today's rare grass");
    line(ui, "/moves  /items  /places  /guides", "Game Info, one kind");
    line(ui, "/gameinfo", "all of Game Info");
    line(ui, "/apps", "apps");
    h(ui, "NARROW IT DOWN");
    line(ui, "/move surf", "learns Surf, and how");
    line(ui, "/type water", "Water type");
    line(ui, "/in route 210", "lives there");
    line(ui, "/surf  /water  /route 210", "shorthand for the three above");
    h(ui, "TRY");
    line(ui, "/move surf pikachu", "does Pikachu learn Surf?");
    line(ui, "/rare /dragon", "Dragon types on rare grass today");
    line(ui, "/water /ice beam", "Water Pokémon that learn Ice Beam");
    line(ui, "/moves pikachu", "every move Pikachu learns");
    line(ui, "/moves /fire", "Fire moves");
    line(ui, "/pokedex /rare", "Pokémon on rare grass today");
    line(ui, "/in route 210 /water", "Water Pokémon on Route 210");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_the_word_being_typed() {
        assert_eq!(replace_last("/rare /dra", "/dragon "), "/rare /dragon ");
        assert_eq!(norm("Ice Beam"), "icebeam");
    }
}
