//! The Dashboard's widgets: under the hunt banner, on a grid of squares 12
//! columns wide (the squares grow and shrink with the window, so the layout
//! never rearranges itself). In Customize you drag a widget to move it (it
//! snaps to the squares and pushes anything in the way down), drag its
//! corner to resize it (whole squares, up to 8×8, never below the
//! widget's smallest size), remove it, and add more from the gallery, with
//! a live preview of each one. Each widget fills its box.
//!
//! Each widget is a `DEFS` entry plus an arm in `draw_widget`; the widgets
//! themselves live in `w_hunt` (Shiny Hunter and general), `w_explore`
//! (Pokédex, Map, Game Info) and `w_battle` (Team Planner, Battle Sim,
//! Breeding, Damage Calc).

use super::store::{app_col, app_icon, card_bg, pill_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::apps::{AppId, WidgetCfg};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::collections::HashMap;

/// Full width (wider than any grid has columns). Widgets made for the full
/// width before the grid.
pub(super) const FULL: u8 = 9;
const GAP: f32 = 12.0;
/// The grid's columns, and the biggest a widget gets (squares each way).
pub(super) const COLS: i32 = 12;
pub(super) const MAX_SIDE: i32 = 8;
/// Narrower squares than this: the widgets just stack (window too narrow).
const MIN_CELL: f32 = 46.0;
/// The grid the places are saved in (see `HomeCfg::grid`). 3: the same
/// grid as 2, with places repaired once (see `repair`).
pub(super) const GRID_VERSION: u8 = 3;

/// A place on the grid, in squares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Cell {
    pub(super) fn of(p: [u16; 4]) -> Cell {
        Cell { x: p[0] as i32, y: p[1] as i32, w: (p[2] as i32).max(1), h: (p[3] as i32).max(1) }
    }
    pub(super) fn save(self) -> [u16; 4] {
        [self.x.clamp(0, 60_000) as u16, self.y.clamp(0, 60_000) as u16, self.w.clamp(1, 255) as u16, self.h.clamp(1, 255) as u16]
    }
    pub(super) fn overlaps(self, o: Cell) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

/// A widget's sizes on the grid, in squares: the smallest, the size it's
/// added at, the biggest, and (for the square ones) its height fixed to its
/// width plus `tall_by` (a map square, the bingo card a square under its
/// header). Measured from each widget's content at 1920 wide, so a new
/// widget has next to no empty space and none can be stretched much past
/// what it has to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Lim {
    pub min: (i32, i32),
    pub start: (i32, i32),
    pub max: (i32, i32),
    pub tall_by: Option<i32>,
}

fn lim_of(kind: &str) -> Option<Lim> {
    match kind {
        "today" => Some(Lim { min: (2, 1), start: (8, 1), max: (8, 4), tall_by: None }),
        "almost" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "jump" => Some(Lim { min: (2, 1), start: (6, 2), max: (8, 4), tall_by: None }),
        "clock" => Some(Lim { min: (2, 1), start: (3, 2), max: (6, 4), tall_by: None }),
        "notes" => Some(Lim { min: (2, 1), start: (3, 2), max: (8, 6), tall_by: None }),
        "hunt_ctrl" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 5), tall_by: None }),
        "hunts" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "shinies" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "showcase" => Some(Lim { min: (2, 1), start: (3, 3), max: (6, 6), tall_by: None }),
        "week" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 4), tall_by: None }),
        "shinydex" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 5), tall_by: None }),
        "luck" => Some(Lim { min: (2, 1), start: (3, 2), max: (6, 4), tall_by: None }),
        "motd" => Some(Lim { min: (2, 1), start: (4, 2), max: (6, 4), tall_by: None }),
        "dexsearch" => Some(Lim { min: (2, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "favs" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "catchable" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "typematch" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 5), tall_by: None }),
        "rare" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "minimap" => Some(Lim { min: (2, 2), start: (4, 4), max: (8, 8), tall_by: Some(0) }),
        "mapfavs" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "huntwhere" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "maprecent" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "infonews" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "tip" => Some(Lim { min: (2, 1), start: (4, 2), max: (6, 4), tall_by: None }),
        "infosearch" => Some(Lim { min: (2, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "adventure" => Some(Lim { min: (4, 2), start: (6, 2), max: (8, 3), tall_by: None }),
        "bookmarks" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "mainteam" => Some(Lim { min: (2, 1), start: (6, 2), max: (8, 4), tall_by: None }),
        "teamweak" => Some(Lim { min: (2, 1), start: (4, 2), max: (6, 6), tall_by: None }),
        "teamswitch" => Some(Lim { min: (2, 1), start: (4, 2), max: (6, 6), tall_by: None }),
        "coverage" => Some(Lim { min: (3, 2), start: (6, 2), max: (8, 5), tall_by: None }),
        "teamidea" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 5), tall_by: None }),
        "lastbattle" => Some(Lim { min: (3, 1), start: (4, 2), max: (6, 4), tall_by: None }),
        "winrecord" => Some(Lim { min: (2, 1), start: (3, 2), max: (6, 3), tall_by: None }),
        "quickbattle" => Some(Lim { min: (3, 1), start: (4, 2), max: (8, 3), tall_by: None }),
        "trainers" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "battlehistory" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "breeding" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "breeders" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "eggmoves" => Some(Lim { min: (3, 2), start: (4, 2), max: (8, 5), tall_by: None }),
        "nextstep" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 5), tall_by: None }),
        "hatches" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 4), tall_by: None }),
        "quickcalc" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 5), tall_by: None }),
        "savedcalcs" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "speedtiers" => Some(Lim { min: (3, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "lastcalc" => Some(Lim { min: (3, 2), start: (4, 2), max: (8, 4), tall_by: None }),
        "threat" => Some(Lim { min: (3, 3), start: (4, 4), max: (8, 6), tall_by: None }),
        "cards_xp" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "cards_packs" => Some(Lim { min: (2, 2), start: (3, 2), max: (6, 4), tall_by: None }),
        "cards_pulls" => Some(Lim { min: (2, 1), start: (6, 2), max: (8, 6), tall_by: None }),
        "cards_sets" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 4), tall_by: None }),
        "cards_missions" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "gotchi_pet" => Some(Lim { min: (2, 2), start: (4, 2), max: (6, 6), tall_by: None }),
        "gotchi_needs" => Some(Lim { min: (2, 1), start: (3, 2), max: (8, 4), tall_by: None }),
        "gotchi_care" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "gotchi_pantry" => Some(Lim { min: (2, 2), start: (3, 2), max: (6, 4), tall_by: None }),
        "gotchi_box" => Some(Lim { min: (2, 1), start: (6, 2), max: (8, 6), tall_by: None }),
        "bingo_card" => Some(Lim { min: (3, 4), start: (4, 5), max: (7, 8), tall_by: Some(1) }),
        "bingo_week" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "bingo_next" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "bingo_meet" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "bingo_recent" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "rare_out" => Some(Lim { min: (2, 1), start: (4, 3), max: (8, 6), tall_by: None }),
        "rare_new" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "rare_stats" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "rare_places" => Some(Lim { min: (2, 1), start: (4, 3), max: (8, 6), tall_by: None }),
        "rare_missing" => Some(Lim { min: (2, 1), start: (4, 2), max: (8, 6), tall_by: None }),
        "replay_last" => Some(Lim { min: (2, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "replay_rec" => Some(Lim { min: (2, 1), start: (3, 2), max: (6, 3), tall_by: None }),
        "replay_recent" => Some(Lim { min: (2, 2), start: (4, 3), max: (8, 6), tall_by: None }),
        "replay_stats" => Some(Lim { min: (2, 1), start: (4, 1), max: (8, 3), tall_by: None }),
        "replay_save" => Some(Lim { min: (2, 1), start: (3, 1), max: (6, 3), tall_by: None }),
        _ => None,
    }
}

/// A widget's sizes (anything without its own: 2×2 up to `MAX_SIDE`).
pub(super) fn lim(d: Option<&WidgetDef>) -> Lim {
    d.and_then(|d| lim_of(d.kind)).unwrap_or(Lim { min: (2, 2), start: (4, 3), max: (MAX_SIDE, MAX_SIDE), tall_by: None })
}

/// The smallest a widget can be.
pub(super) fn min_size(d: &WidgetDef) -> (i32, i32) {
    lim(Some(d)).min
}

/// The biggest a widget gets.
pub(super) fn max_size(d: &WidgetDef) -> (i32, i32) {
    lim(Some(d)).max
}

/// The size a widget gets when added.
pub(super) fn start_size(d: &WidgetDef) -> (i32, i32) {
    lim(Some(d)).start
}

/// Keep `fixed` where it is and push anything it overlaps down (and what
/// that then overlaps, and so on), keeping everything's order. Nothing
/// moves up or sideways.
pub(super) fn settle(items: &mut [(u64, Cell)], fixed: Option<u64>) {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|i| (Some(items[*i].0) != fixed, items[*i].1.y, items[*i].1.x));
    let mut placed: Vec<Cell> = Vec::new();
    for i in order {
        let mut c = items[i].1;
        while let Some(o) = placed.iter().filter(|p| p.overlaps(c)).map(|p| p.y + p.h).max() {
            c.y = o;
        }
        items[i].1 = c;
        placed.push(c);
    }
}

/// `v` kept between `lo` and `hi` (`lo` wins when they cross, instead of
/// a panic).
fn between(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
}

/// A place made possible: inside the 12 columns, within the widget's
/// smallest and biggest size (and square, for the square ones).
pub(super) fn fit(c: Cell, l: Lim) -> Cell {
    let (mw, mh) = (l.min.0.min(COLS), l.min.1);
    let (xw, xh) = (l.max.0.max(mw).min(COLS), l.max.1.max(l.min.1));
    let mut w = between(c.w, mw, xw);
    let h = match l.tall_by {
        Some(d) => {
            // the height follows the width
            w = between(w, between(mh - d, mw, xw), between(xh - d, mw, xw));
            between(w + d, mh, xh)
        }
        None => between(c.h, mh, xh),
    };
    Cell { x: between(c.x, 0, COLS - w), y: c.y.max(0), w, h }
}

/// Could be on the grid at all (whatever the widget): inside the 12
/// columns and no side past `MAX_SIDE`.
fn on_grid(c: Cell) -> bool {
    c.x >= 0 && c.y >= 0 && c.w >= 1 && c.h >= 1 && c.w <= MAX_SIDE && c.h <= MAX_SIDE && c.x + c.w <= COLS
}

/// Mend places once. v0.7.1 and v0.7.2 doubled the first widget added to
/// an empty Dashboard (an 8×3 became 16×6, wider than the grid, and
/// grabbing it crashed): those are halved back. Then every widget is made
/// to fit its own sizes, whatever now overlaps moves down, and the room
/// freed is closed up.
pub(super) fn repair(items: &mut [(u64, Cell)], lim_of: impl Fn(u64) -> Lim) {
    let mut changed = false;
    for (id, c) in items.iter_mut() {
        let l = lim_of(*id);
        let even = [c.x, c.y, c.w, c.h].iter().all(|v| v % 2 == 0);
        let was = *c;
        if !on_grid(*c) && even {
            *c = Cell { x: c.x / 2, y: c.y / 2, w: c.w / 2, h: c.h / 2 };
        }
        *c = fit(*c, l);
        changed |= *c != was;
    }
    settle(items, None);
    // the room a widget no longer takes is closed up (what was under it
    // moves up as far as it can, in order)
    if changed {
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|i| (items[*i].1.y, items[*i].1.x));
        for i in order {
            loop {
                let mut up = items[i].1;
                up.y -= 1;
                if up.y < 0 || items.iter().enumerate().any(|(j, (_, o))| j != i && o.overlaps(up)) {
                    break;
                }
                items[i].1 = up;
            }
        }
    }
}

/// The first spot (top to bottom, left to right) a w × h widget fits.
pub(super) fn first_free(items: &[(u64, Cell)], w: i32, h: i32) -> Cell {
    for y in 0..1000 {
        for x in 0..=(COLS - w).max(0) {
            let c = Cell { x, y, w, h };
            if !items.iter().any(|(_, o)| o.overlaps(c)) {
                return c;
            }
        }
    }
    Cell { x: 0, y: 0, w, h }
}

/// A kind of widget.
pub(super) struct WidgetDef {
    pub kind: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub icon: &'static str,
    /// The app it belongs to (None: always there).
    pub app: Option<AppId>,
    /// Its width when added.
    pub span: u8,
    /// Can be added more than once (each keeps its own settings).
    pub multi: bool,
}

const fn wd(kind: &'static str, name: &'static str, about: &'static str, icon: &'static str, app: Option<AppId>, span: u8, multi: bool) -> WidgetDef {
    WidgetDef { kind, name, about, icon, app, span, multi }
}

use AppId::{Bingo, Breed, Calc, Cards, Gotchi, Info, Map, Pokedex, Rare, Replay, Shiny, Sim, Team};

pub(super) const DEFS: &[WidgetDef] = &[
    // the Dashboard's own
    wd("today", "Today at a glance", "Today's encounters, your daily goal, streak and level", "📅", None, FULL, false),
    wd("almost", "Almost there", "The achievements you're closest to", "🏆", None, 1, false),
    wd("jump", "Jump to", "Every app you have, one click away", "📌", None, FULL, false),
    wd("clock", "Clock & session", "The time, how long the app's been open and today's hunting", "🕘", None, 1, false),
    wd("notes", "Notes", "Jot things down; they stay here", "📝", None, 1, true),
    // Shiny Hunter
    wd("hunt_ctrl", "Current hunt", "Your count, odds and time, with Start overlay", "🎯", Some(Shiny), 2, false),
    wd("hunts", "Pick up a hunt", "Hunts you can resume, or the ones you've finished", "📜", Some(Shiny), 2, false),
    wd("shinies", "Recent shinies", "Your latest catches", "✨", Some(Shiny), 1, false),
    wd("showcase", "Shiny showcase", "Your latest shinies, big, one after another", "🌟", Some(Shiny), 1, false),
    wd("week", "This week", "Encounters each day for the last 7 days, and your best day", "📊", Some(Shiny), 1, false),
    wd("shinydex", "Shiny Dex progress", "How much of the Shiny Dex you've filled, and what's next", "💎", Some(Shiny), 1, false),
    wd("luck", "Luck meter", "How your current hunt compares with everyone's luck", "🍀", Some(Shiny), 1, false),
    // Pokédex
    wd("motd", "Pokémon of the day", "A different Pokémon every day: its types and where to find it", "📖", Some(Pokedex), 1, false),
    wd("dexsearch", "Pokédex search", "Look a Pokémon up without leaving the Dashboard", "🔍", Some(Pokedex), FULL, false),
    wd("favs", "Favourite Pokémon", "The Pokémon you pick, a click from their page", "❤", Some(Pokedex), 2, true),
    wd("catchable", "Catchable here", "What lives in a place you pick", "🌿", Some(Pokedex), 1, true),
    wd("typematch", "Type matchup", "What a type hits hard, and what hits it hard", "⚔", Some(Pokedex), 1, true),
    // Map
    wd("rare", "Rare grass today", "Today's rare grass spots", "🌾", Some(Map), 2, false),
    wd("minimap", "Mini map", "Sinnoh at a glance, with today's rare grass. Scroll to zoom, drag to look around", "🗺", Some(Map), 2, false),
    wd("mapfavs", "Favourite places", "Places you pick, with what lives there", "📍", Some(Map), 1, true),
    wd("huntwhere", "Where's my hunt", "Where your current target can be found", "🔭", Some(Map), 1, false),
    wd("maprecent", "Recent places", "The places you looked at last on the Map", "🕘", Some(Map), 1, false),
    // Game Info
    wd("infonews", "Latest game news", "The newest news from the game's wiki", "📰", Some(Info), 2, false),
    wd("tip", "Tip of the day", "A different how-to-play guide every day", "💡", Some(Info), 1, false),
    wd("infosearch", "Quick lookup", "Search moves, items, places and guides", "🔎", Some(Info), FULL, false),
    wd("adventure", "Your adventure", "Where you are in the story", "🏅", Some(Info), 2, false),
    wd("bookmarks", "Pinned pages", "Game Info pages you pinned, and ones you looked at lately", "🔖", Some(Info), 1, false),
    // Team Planner
    wd("mainteam", "Main team", "Your team at a glance", "🛡", Some(Team), 2, false),
    wd("teamweak", "Team weaknesses", "Types your team struggles against", "⚠", Some(Team), 1, false),
    wd("teamswitch", "Team switcher", "Pick which team the Dashboard shows", "🔁", Some(Team), 1, false),
    wd("coverage", "Coverage check", "How hard your team's moves hit every type", "🎯", Some(Team), 2, false),
    wd("teamidea", "Team ideas", "A type to add that covers your team's gaps", "💭", Some(Team), 1, false),
    // Battle Sim
    wd("lastbattle", "Last battle", "How your last battle went, with Rematch", "⚔", Some(Sim), 1, false),
    wd("winrecord", "Win record", "Wins and losses, and your best team", "🏆", Some(Sim), 1, false),
    wd("quickbattle", "Quick battle", "Battle a random wild Pokémon in one click", "⚡", Some(Sim), 1, false),
    wd("trainers", "Your trainers", "Trainers you made, ready to battle", "🎖", Some(Sim), 2, false),
    wd("battlehistory", "Battle history", "Your last five battles", "📜", Some(Sim), 2, false),
    // Breeding
    wd("breeding", "Breeding", "What's in the Day Care", "🐣", Some(Breed), 1, false),
    wd("breeders", "Breeders box", "Your breeders, with their stats and natures", "📦", Some(Breed), 1, false),
    wd("eggmoves", "Egg move finder", "The egg moves a Pokémon you pick can get", "💥", Some(Breed), 1, true),
    wd("nextstep", "Next step", "The one next thing to do in your breeding plan", "👣", Some(Breed), 1, false),
    wd("hatches", "Hatch tracker", "Eggs hatched, and shinies from Eggs", "🐥", Some(Breed), 1, false),
    // Damage Calc
    wd("quickcalc", "Quick calc", "One Pokémon's best hit on another, right here", "🔢", Some(Calc), 2, true),
    wd("savedcalcs", "Saved calcs", "Calcs you saved from Quick calc", "⭐", Some(Calc), 1, false),
    wd("speedtiers", "Speed tiers", "Who your main team outspeeds", "💨", Some(Calc), 1, false),
    wd("lastcalc", "Last result", "The calculation open in the Damage Calc", "📋", Some(Calc), 1, false),
    wd("threat", "Threat check", "Your main team against one Pokémon you pick", "🚨", Some(Calc), 2, true),
    // Card Collector
    wd("cards_xp", "Booster progress", "XP towards your next booster pack", "⭐", Some(Cards), 1, false),
    wd("cards_packs", "Packs to open", "Booster packs waiting, one click to open", "🎁", Some(Cards), 1, false),
    wd("cards_pulls", "Latest pulls", "The best cards from your last packs", "🃏", Some(Cards), 2, false),
    wd("cards_sets", "Set completion", "How much of each set is in your binder", "📚", Some(Cards), 1, false),
    wd("cards_missions", "Card missions", "Today's missions for more XP", "🎯", Some(Cards), 1, false),
    // Pokégotchi
    wd("gotchi_pet", "Your Pokégotchi", "Your pet in its device, and how it's feeling", "🐾", Some(Gotchi), 1, false),
    wd("gotchi_needs", "Pet needs", "Full, Happy, Energy and Clean at a glance", "💗", Some(Gotchi), 1, false),
    wd("gotchi_care", "Quick care", "Feed, treat, clean or pat without opening the app", "🍓", Some(Gotchi), 1, false),
    wd("gotchi_pantry", "Pantry", "Berries and treats, and what hunting earned today", "🎒", Some(Gotchi), 1, false),
    wd("gotchi_box", "The Box", "Every Pokégotchi you have; click one to bring it out", "📦", Some(Gotchi), 2, false),
    wd("bingo_card", "Bingo card", "Today's Shiny Bingo card, ticking itself as you hunt", "🎯", Some(Bingo), 1, false),
    wd("bingo_week", "Bingo today", "Squares and lines done, and when the next card comes", "📅", Some(Bingo), 1, false),
    wd("bingo_next", "Closest to a line", "The lines nearest to done, and what each still needs", "📌", Some(Bingo), 1, false),
    wd("bingo_meet", "Bingo Pokémon", "Today's Pokémon to meet, ticked when you do", "🔍", Some(Bingo), 1, false),
    wd("rare_out", "Rare grass today", "The rarest Pokémon on rare grass today, and where", "✨", Some(Rare), 1, false),
    wd("rare_new", "New on rare grass", "Pokémon on rare grass for the first time today", "🆕", Some(Rare), 1, false),
    wd("rare_stats", "Rare grass in numbers", "Pokémon and tiles today, everything seen, and the reset", "🌿", Some(Rare), 1, false),
    wd("rare_places", "Busiest places today", "The places with the most Pokémon on rare grass today", "📍", Some(Rare), 1, false),
    wd("rare_missing", "Not seen lately", "Pokémon that were on rare grass before but not in the last few days", "⏳", Some(Rare), 1, false),
    wd("replay_last", "Last Shiny Replay", "Your newest clip, ready to watch, and whether the recorder is on", "🎬", Some(Replay), 1, false),
    wd("replay_rec", "Replay recorder", "How many seconds are ready to save, with Save replay now", "⏺", Some(Replay), 1, false),
    wd("replay_recent", "Recent clips", "Your latest clips, one click to watch", "🎞", Some(Replay), 1, false),
    wd("replay_stats", "Clips in numbers", "Clips, shinies on video, and the space they take", "📼", Some(Replay), 1, false),
    wd("replay_save", "Save replay now", "One big button that saves the last seconds of the game", "💾", Some(Replay), 1, false),
    wd("bingo_recent", "Just ticked", "The latest bingo squares you filled in", "✅", Some(Bingo), 1, false),
];

pub(super) fn def(kind: &str) -> Option<&'static WidgetDef> {
    DEFS.iter().find(|d| d.kind == kind)
}

/// The layout everyone starts with: nothing (you pick your own).
pub(super) fn default_widgets() -> Vec<WidgetCfg> {
    Vec::new()
}

/// How the gallery lists the widgets.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Sort {
    #[default]
    App,
    Name,
    Size,
}

/// The "Add widgets" gallery.
#[derive(Default)]
pub(super) struct Gallery {
    pub open: bool,
    pub query: String,
    pub sort: Sort,
    /// Only this app's widgets (Some(None): the Dashboard's own).
    pub app: Option<Option<AppId>>,
    /// Hide the ones already on the Dashboard.
    pub not_added: bool,
    /// The Starter layouts tab (else the widgets).
    pub layouts: bool,
}

#[derive(Default)]
pub(super) struct HomeState {
    pub edit: bool,
    /// The widget being moved, and where it was grabbed (from its corner).
    pub drag: Option<u64>,
    pub grab: egui::Vec2,
    /// The widget being resized.
    pub resize: Option<u64>,
    /// The grid while moving or resizing (where everything would go).
    pub preview: Option<Vec<(u64, Cell)>>,
    /// An empty square was clicked: the next widget added goes there.
    pub add_at: Option<(i32, i32)>,
    /// The "Add widgets" gallery.
    pub gallery: Gallery,
    /// "Clear all" was pressed once (press again to confirm), and when.
    pub clear_armed: Option<std::time::Instant>,
    /// Widgets' working state (search boxes and the like), by widget id.
    pub text: HashMap<(u64, &'static str), String>,
    /// The breeding plan the Next step widget shows, and what it was made from.
    pub plan: Option<(String, crate::breed_plan::Plan)>,
    /// The Dashboard from before the last starter layout went on (Undo).
    pub undo: Option<(&'static str, Vec<WidgetCfg>)>,
    /// A starter layout clicked on a Dashboard with widgets: asking
    /// replace or add below.
    pub preset_pick: Option<usize>,
    /// The full plan (with the Egg estimate) being worked out.
    pub plan_rx: Option<(String, std::sync::mpsc::Receiver<crate::breed_plan::Plan>)>,
}

impl App {
    pub(super) fn home_widgets(&self) -> Vec<WidgetCfg> {
        self.cfg.home.widgets.clone().unwrap_or_else(default_widgets)
    }

    pub(super) fn set_home_widgets(&mut self, w: Vec<WidgetCfg>) {
        self.cfg.home.widgets = Some(w);
        self.dirty = true;
    }

    /// Put a widget in the first space it fits.
    fn add_widget(&mut self, d: &'static WidgetDef) {
        let mut w = self.home_widgets();
        let mut cells = place_all(&w);
        let id = w.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        let (sw, sh) = start_size(d);
        let c = match self.setup.home.add_at.take() {
            // where you clicked (anything in the way moves down), and back
            // to the Dashboard to see it
            Some((x, y)) => {
                self.setup.home.gallery.open = false;
                Cell { x: x.min(COLS - sw).max(0), y, w: sw, h: sh }
            }
            None => first_free(&cells, sw, sh),
        };
        cells.push((id, c));
        settle(&mut cells, Some(id));
        for x in w.iter_mut() {
            if let Some((_, nc)) = cells.iter().find(|(i, _)| *i == x.id) {
                x.pos = Some(nc.save());
            }
        }
        let c = cells.iter().find(|(i, _)| *i == id).map_or(c, |x| x.1);
        w.push(WidgetCfg { id, kind: d.kind.to_string(), span: d.span, data: Vec::new(), pos: Some(c.save()) });
        // placed on this grid (never to be doubled as an old place)
        self.cfg.home.grid = GRID_VERSION;
        self.set_home_widgets(w);
        self.app_toast(format!("Added {} to your Dashboard", d.name));
    }

    /// The edit bar: Customize; in Customize, Add widget, Clear all, Done.
    /// (An empty Dashboard has its own big start instead.)
    pub(super) fn home_widgets_bar(&mut self, ui: &mut egui::Ui) {
        let edit = self.setup.home.edit;
        let none = self.home_widgets().is_empty();
        if none && !edit {
            return;
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(if edit { "Customize your Dashboard" } else { "Your widgets" }).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
            if edit {
                ui.label(egui::RichText::new("Drag to move · drag the corner to resize · click an empty square to add · × to remove").size(kit::fs(12.5)).color(MUTED()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(if edit { 84.0 } else { 116.0 }, 32.0), Sense::hover());
                if pill_button(ui, r, if edit { "Done" } else { "✏  Customize" }, edit, "home-edit").clicked() {
                    self.setup.home.edit = !edit;
                    self.setup.home.clear_armed = None;
                    self.setup.home.drag = None;
                    self.setup.home.resize = None;
                    self.setup.home.preview = None;
                }
                if let Some((name, _)) = self.setup.home.undo {
                    let label = format!("↩  Undo {name} layout");
                    let lw = ui.painter().layout_no_wrap(label.clone(), crate::fonts::sans_bold_font(12.5), TEXT()).size().x + 30.0;
                    let (r, _) = ui.allocate_exact_size(vec2(lw, 32.0), Sense::hover());
                    if pill_button(ui, r, &label, false, "home-undo").on_hover_text("Put back the widgets you had before").clicked() {
                        self.undo_preset();
                    }
                }
                if !edit {
                    return;
                }
                let (r, _) = ui.allocate_exact_size(vec2(118.0, 32.0), Sense::hover());
                if pill_button(ui, r, "+  Add widget", false, "home-add").on_hover_text("Open the widget gallery").clicked() {
                    self.setup.home.add_at = None;
                    self.setup.home.gallery.open = true;
                }
                if !none {
                    let armed = self.setup.home.clear_armed.is_some_and(|t| t.elapsed().as_secs_f32() < 3.0);
                    let (r, _) = ui.allocate_exact_size(vec2(if armed { 118.0 } else { 86.0 }, 32.0), Sense::hover());
                    let resp = pill_button(ui, r, if armed { "Sure? Clear all" } else { "Clear all" }, false, "home-clear").on_hover_text("Take every widget off the Dashboard");
                    if resp.clicked() {
                        if armed {
                            self.set_home_widgets(Vec::new());
                            self.setup.home.clear_armed = None;
                        } else {
                            self.setup.home.clear_armed = Some(std::time::Instant::now());
                        }
                    }
                    if armed {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                    }
                }
            });
        });
        ui.add_space(12.0);
    }

    /// An empty Dashboard: faint outlines of widgets to come, and one big
    /// button. A few suggestions underneath add with one click.
    fn home_empty(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let h = 284.0;
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(r, 20.0, theme::blend(kit::PAGE(), CARD(), 0.55));
        dashed(&p, r, theme::blend(LINE(), kit::primary(), 0.35));
        // ghost widgets: a little waterfall of outlines
        let ghost = Rect::from_center_size(pos2(r.center().x, r.min.y + 92.0), vec2(300.0_f32.min(w - 60.0), 112.0));
        let gw = (ghost.width() - 16.0) / 3.0;
        let t = ui.input(|i| i.time) as f32;
        let boxes = [(0.0, 0.0, 1.0, 46.0), (1.0, 0.0, 2.0, 30.0), (1.0, 38.0, 1.0, 66.0), (2.0, 38.0, 1.0, 66.0), (0.0, 54.0, 1.0, 50.0)];
        for (k, (c, y, span, bh)) in boxes.iter().enumerate() {
            let br = Rect::from_min_size(pos2(ghost.min.x + c * (gw + 8.0), ghost.min.y + y), vec2(gw * span + 8.0 * (span - 1.0), *bh));
            let pulse = 0.5 + 0.5 * (t * 1.6 - k as f32 * 0.6).sin();
            let col = [kit::C_VIOLET, kit::C_CYAN, kit::C_GOLD, kit::C_GREEN, kit::C_PINK][k];
            p.rect_filled(br, 10.0, theme::blend(CARD(), col, 0.10 + 0.08 * pulse));
            p.rect_stroke(br, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.35 + 0.3 * pulse)));
            p.rect_filled(Rect::from_min_size(br.min + vec2(8.0, 8.0), vec2((br.width() * 0.45).min(60.0), 6.0)), 3.0, theme::blend(CARD(), col, 0.45));
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        let edit = self.setup.home.edit;
        p.text(pos2(r.center().x, r.min.y + 178.0), Align2::CENTER_CENTER, "Your Dashboard is empty", crate::fonts::sans_bold_font(20.0), TEXT());
        p.text(
            pos2(r.center().x, r.min.y + 204.0),
            Align2::CENTER_CENTER,
            if edit { "Add widgets from any of your apps, then drag them where you like." } else { "Press Customize to add widgets from any of your apps and arrange them." },
            crate::fonts::sans_font(13.0),
            MUTED(),
        );
        let br = Rect::from_center_size(pos2(r.center().x, r.min.y + 248.0), vec2(230.0, 44.0));
        if edit {
            if pill_button(ui, br, "+  Add your first widget", true, "home-first").clicked() {
                self.setup.home.gallery.open = true;
            }
        } else if pill_button(ui, br, "✏  Customize", true, "home-first").clicked() {
            self.setup.home.edit = true;
        }
        // or a ready-made layout
        ui.add_space(26.0);
        ui.set_max_width(w);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Or start with a ready-made layout").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
            ui.label(egui::RichText::new("A few widgets each, laid out for you. Change anything after.").size(kit::fs(12.5)).color(MUTED()));
        });
        ui.add_space(12.0);
        self.preset_cards(ui, "empty", false);
    }

    /// The widget gallery: every widget with a live preview, a search box,
    /// sorting and a filter for each app.
    pub(super) fn home_gallery(&mut self, ctx: &egui::Context) {
        if !self.setup.home.gallery.open {
            return;
        }
        let screen = ctx.screen_rect();
        let size = vec2((screen.width() - 80.0).clamp(360.0, 1180.0), (screen.height() - 70.0).clamp(300.0, 860.0));
        let panel = Rect::from_center_size(screen.center(), size);
        let mut close = false;
        let mut add: Option<&'static WidgetDef> = None;
        let mut get: Option<(AppId, &'static str)> = None;
        let have: Vec<String> = self.home_widgets().into_iter().map(|w| w.kind).collect();
        // one area: the scrim, with the panel drawn on it (so the panel
        // never ends up under its own scrim)
        egui::Area::new(egui::Id::new("widget-gallery")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(150));
            if scrim.clicked() {
                close = true;
            }
            ui.painter().add(kit::shadow_tier(3).as_shape(panel, 22.0));
            ui.painter().rect_filled(panel, 22.0, CARD());
            ui.painter().rect_stroke(panel, 22.0, Stroke::new(1.0f32, kit::CONTROL()));
            // the panel eats its own clicks
            ui.interact(panel, egui::Id::new("widget-gallery-panel"), Sense::click());
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel.shrink2(vec2(26.0, 22.0))).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                // header
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Add widgets").font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
                        ui.label(egui::RichText::new("Pick what your Dashboard shows. Each app brings its own; add as many as you like.").size(kit::fs(12.5)).color(MUTED()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                        if pill_button(ui, r, "×", false, "gal-x").on_hover_text("Close (Esc)").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(12.0);
                // two tabs: single widgets, or a whole layout
                let mut tab = self.setup.home.gallery.layouts;
                let wl = format!("Widgets  {}", DEFS.len());
                let ll = format!("Starter layouts  {}", super::w_presets::PRESETS.len());
                kit::segmented(ui, &mut tab, &[(false, wl.as_str()), (true, ll.as_str())]);
                self.setup.home.gallery.layouts = tab;
                ui.add_space(12.0);
                if tab {
                    ui.label(egui::RichText::new("A whole Dashboard in one click, laid out for a kind of player. Apps a layout needs are added with it.").size(kit::fs(12.5)).color(MUTED()));
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical().id_salt("gal-layouts").auto_shrink([false, false]).show(ui, |ui| {
                        ui.set_max_width(ui.available_width() - 6.0);
                        self.preset_cards(ui, "gal", true);
                    });
                    return;
                }
                // search, sort, not-added
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    kit::search_box_keyed(ui, Some(egui::Id::new("gal-search")), &mut self.setup.home.gallery.query, "🔍  Search widgets", 260.0);
                    ui.label(egui::RichText::new("Sort").size(kit::fs(12.0)).color(MUTED()));
                    let mut sort = self.setup.home.gallery.sort;
                    kit::segmented(ui, &mut sort, &[(Sort::App, "By app"), (Sort::Name, "A–Z"), (Sort::Size, "Size")]);
                    self.setup.home.gallery.sort = sort;
                    ui.add_space(6.0);
                    let mut na = self.setup.home.gallery.not_added;
                    if ui.checkbox(&mut na, egui::RichText::new("Hide added").size(kit::fs(12.5)).color(TEXT())).changed() {
                        self.setup.home.gallery.not_added = na;
                    }
                });
                ui.add_space(10.0);
                // app filters
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    let mut opts: Vec<(Option<Option<AppId>>, String, Color32)> = vec![(None, "All".into(), kit::primary()), (Some(None), "📅  Dashboard".into(), kit::C_CYAN)];
                    for a in crate::apps::ALL {
                        opts.push((Some(Some(a)), format!("{}  {}", app_icon(a), a.name()), app_col(a)));
                    }
                    for (val, label, col) in opts {
                        let n = DEFS.iter().filter(|d| val.map_or(true, |v| d.app == v)).count();
                        let on = self.setup.home.gallery.app == val;
                        let text = format!("{label}  {n}");
                        let g = ui.painter().layout_no_wrap(text, crate::fonts::sans_bold_font(12.0), if on { Color32::WHITE } else { TEXT() });
                        let (r, resp) = ui.allocate_exact_size(g.size() + vec2(22.0, 12.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        ui.painter().rect_filled(r, r.height() / 2.0, if on { col } else { theme::blend(kit::sunken(), col, 0.08 + 0.12 * ht) });
                        ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { col } else { theme::blend(LINE(), col, 0.2 + 0.4 * ht) }));
                        ui.painter().galley(r.min + vec2(11.0, 6.0), g, TEXT());
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.setup.home.gallery.app = if on { None } else { val };
                        }
                    }
                });
                ui.add_space(12.0);
                // which widgets, in what order
                let g = &self.setup.home.gallery;
                let q = g.query.trim().to_lowercase();
                let mut list: Vec<&'static WidgetDef> = DEFS
                    .iter()
                    .filter(|d| g.app.map_or(true, |v| d.app == v))
                    .filter(|d| !g.not_added || d.multi || !have.iter().any(|k| k == d.kind))
                    .filter(|d| q.is_empty() || d.name.to_lowercase().contains(&q) || d.about.to_lowercase().contains(&q) || d.app.map_or("dashboard", |a| a.name()).to_lowercase().contains(&q))
                    .collect();
                let app_rank = |d: &WidgetDef| d.app.map_or(0, |a| 1 + crate::apps::ALL.iter().position(|x| *x == a).unwrap_or(0));
                match g.sort {
                    Sort::App => list.sort_by_key(|d| app_rank(d)),
                    Sort::Name => list.sort_by_key(|d| d.name),
                    Sort::Size => list.sort_by_key(|d| (std::cmp::Reverse(d.span), app_rank(d))),
                }
                let shown = list.len();
                ui.label(egui::RichText::new(format!("{shown} widget{}", if shown == 1 { "" } else { "s" })).size(kit::fs(12.0)).color(MUTED()));
                ui.add_space(6.0);
                if list.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| ui.label(egui::RichText::new("No widgets match. Try another word or app.").color(MUTED())));
                    return;
                }
                // the grid of tiles, each with a live preview
                const TILE_H: f32 = 300.0;
                const PREVIEW_H: f32 = 184.0;
                let gap = 14.0;
                egui::ScrollArea::vertical().id_salt("gal-scroll").auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width() - 6.0;
                    let cols = ((w + gap) / (300.0 + gap)).floor().max(1.0) as usize;
                    let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                    for (row_i, row) in list.chunks(cols).enumerate() {
                        let (row_r, _) = ui.allocate_exact_size(vec2(w, TILE_H), Sense::hover());
                        ui.add_space(gap);
                        if !ui.is_rect_visible(row_r) {
                            continue;
                        }
                        for (c, d) in row.iter().enumerate() {
                            let r = Rect::from_min_size(pos2(row_r.min.x + c as f32 * (tw + gap), row_r.min.y), vec2(tw, TILE_H));
                            let added = have.iter().filter(|k| *k == d.kind).count();
                            let installed = d.app.map_or(true, |a| self.has_app(a));
                            let col = d.app.map_or(kit::C_CYAN, app_col);
                            let tile = ui.interact(r, ui.id().with(("gal-tile", d.kind)), Sense::hover());
                            let ht = kit::anim(ui.ctx(), tile.id.with("h"), tile.hovered(), kit::QUICK);
                            let p = ui.painter().clone();
                            p.rect_filled(r, 16.0, theme::blend(kit::sunken(), CARD_HOVER(), 0.4 * ht));
                            p.rect_stroke(r, 16.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.15 + 0.5 * ht)));
                            // the live preview, clipped and faded at the bottom
                            let pr = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(tw - 20.0, PREVIEW_H));
                            p.rect_filled(pr, 12.0, theme::blend(kit::PAGE(), col, 0.05));
                            let fake = WidgetCfg { id: 9_000_000 + (row_i * 16 + c) as u64, kind: d.kind.to_string(), span: d.span, data: Vec::new(), pos: None };
                            let inner = pr.shrink(8.0);
                            // drawn in a child that takes no room in the grid
                            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min, vec2(inner.width(), 2000.0))).layout(egui::Layout::top_down(egui::Align::Min)));
                            child.set_clip_rect(inner.intersect(ui.clip_rect()));
                            child.set_width(inner.width());
                            if installed {
                                // the widget at the size (within its own) that fills the
                                // preview best, its squares big enough to read
                                let l = lim(Some(d));
                                let (mut sw, mut sh) = l.start;
                                let mut best = -1.0f32;
                                for w in l.min.0..=l.max.0 {
                                    for h in l.min.1..=l.max.1 {
                                        if l.tall_by.is_some_and(|t| h != w + t) {
                                            continue;
                                        }
                                        let cell = ((inner.width() + GAP) / w as f32 - GAP).min((inner.height() + GAP) / h as f32 - GAP);
                                        if cell < 52.0 {
                                            continue;
                                        }
                                        let cover = (w as f32 * cell) * (h as f32 * cell) / (inner.width() * inner.height());
                                        let near = ((w - l.start.0).abs() + (h - l.start.1).abs()) as f32 * 0.01;
                                        if cover - near > best {
                                            best = cover - near;
                                            (sw, sh) = (w, h);
                                        }
                                    }
                                }
                                let cell = ((inner.width() + GAP) / sw as f32 - GAP).min((inner.height() + GAP) / sh as f32 - GAP).max(30.0);
                                let size = vec2(sw as f32 * (cell + GAP) - GAP, sh as f32 * (cell + GAP) - GAP);
                                let pr2 = Rect::from_min_size(inner.min, size.min(inner.size()));
                                card_bg(ui.painter(), pr2, false);
                                super::w_fit::set_box(Some(super::w_fit::WBox { r: pr2, cols: sw, rows: sh, cell }));
                                let _ = self.draw_widget(&mut child, &fake, true);
                                super::w_fit::set_box(None);
                            } else {
                                widget_ghost(&mut child, d);
                            }
                            // nothing in the preview clicks
                            ui.interact(pr, ui.id().with(("gal-shield", d.kind)), Sense::click());
                            fade_bottom(&p, Rect::from_min_max(pos2(pr.min.x, pr.max.y - 46.0), pr.max), theme::blend(kit::sunken(), CARD_HOVER(), 0.4 * ht));
                            // size tag on the preview
                            let (sw, sh) = start_size(d);
                            let (mw, mh) = min_size(d);
                            let (xw, xh) = max_size(d);
                            let mut size = if (mw, mh) == (1, 1) { format!("{sw}×{sh}") } else { format!("{sw}×{sh} · smallest {mw}×{mh}") };
                            if (xw, xh) != (MAX_SIDE, MAX_SIDE) {
                                size.push_str(&format!(" · biggest {xw}×{xh}"));
                            }
                            let sg = p.layout_no_wrap(size, crate::fonts::sans_bold_font(10.5), TEXT());
                            let sr = Rect::from_min_size(pos2(pr.max.x - sg.size().x - 22.0, pr.min.y + 8.0), sg.size() + vec2(14.0, 6.0));
                            p.rect_filled(sr, 8.0, CARD().gamma_multiply(0.92));
                            p.rect_stroke(sr, 8.0, Stroke::new(1.0f32, LINE()));
                            p.galley(sr.min + vec2(7.0, 3.0), sg, TEXT());
                            // name, app, what it does
                            let ty = pr.max.y + 12.0;
                            let chip = Rect::from_min_size(pos2(r.min.x + 12.0, ty), vec2(30.0, 30.0));
                            kit::icon_chip(&p, chip, d.icon, col, false);
                            let clip = p.with_clip_rect(Rect::from_min_max(pos2(chip.max.x + 8.0, ty - 2.0), pos2(r.max.x - 12.0, r.max.y)));
                            clip.text(pos2(chip.max.x + 10.0, ty + 6.0), Align2::LEFT_CENTER, d.name, crate::fonts::sans_bold_font(14.0), TEXT());
                            clip.text(pos2(chip.max.x + 10.0, ty + 24.0), Align2::LEFT_CENTER, d.app.map_or("Dashboard", |a| a.name()), crate::fonts::sans_font(11.5), theme::blend(MUTED(), col, 0.5));
                            let ab = p.layout(d.about.to_string(), crate::fonts::sans_font(12.0), MUTED(), tw - 24.0);
                            p.with_clip_rect(Rect::from_min_max(pos2(r.min.x, ty + 40.0), pos2(r.max.x, ty + 72.0))).galley(pos2(r.min.x + 12.0, ty + 40.0), ab, MUTED());
                            // the button
                            let br = Rect::from_min_size(pos2(r.max.x - 112.0, r.max.y - 40.0), vec2(100.0, 30.0));
                            if added > 0 {
                                p.text(pos2(r.min.x + 14.0, br.center().y), Align2::LEFT_CENTER, if added > 1 { format!("✓ On your Dashboard ×{added}") } else { "✓ On your Dashboard".to_string() }, crate::fonts::sans_bold_font(11.5), theme::GREEN);
                            }
                            if !installed {
                                if pill_button(ui, br, "🔒  Get app", false, ("gal-get", d.kind)).on_hover_text(format!("Needs the {} app", d.app.map(|a| a.name()).unwrap_or(""))).clicked() {
                                    get = d.app.map(|a| (a, d.name));
                                }
                            } else if added > 0 && !d.multi {
                                p.text(br.center(), Align2::CENTER_CENTER, "Added", crate::fonts::sans_bold_font(12.5), MUTED());
                            } else if pill_button(ui, br, if added > 0 { "+  Add another" } else { "+  Add" }, true, ("gal-add", d.kind)).clicked() {
                                add = Some(*d);
                            }
                        }
                    }
                });
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.setup.store.modal.is_none() {
            close = true;
        }
        if let Some(d) = add {
            self.add_widget(d);
        }
        if let Some((a, name)) = get {
            self.ask_for_app(a, name);
        }
        if close {
            self.setup.home.gallery.open = false;
            self.setup.home.add_at = None;
        }
    }

    /// The grid: every widget in its box. In Customize, drag one to move it
    /// (it snaps to the squares; anything in the way is pushed down) or drag
    /// its corner to resize it.
    pub(super) fn home_widget_grid(&mut self, ui: &mut egui::Ui) {
        let edit = self.setup.home.edit;
        let all = self.home_widgets();
        if all.is_empty() {
            // nothing to bring over: whatever gets added is on this grid
            // (v0.7.1 and v0.7.2 left this at 0, so the first widget added
            // was then doubled as if it came from the first grid)
            if self.cfg.home.grid != GRID_VERSION {
                self.cfg.home.grid = GRID_VERSION;
                self.dirty = true;
            }
            self.home_empty(ui);
            return;
        }
        // saves from older versions (and anything odd in one) brought up
        // to date once
        if self.cfg.home.grid < GRID_VERSION || needs_upgrade(&all) {
            let fixed = upgrade(&all, self.cfg.home.grid);
            self.cfg.home.grid = GRID_VERSION;
            self.set_home_widgets(fixed);
            return self.home_widget_grid(ui);
        }
        let width = ui.available_width();
        let cell = ((width - GAP * (COLS as f32 - 1.0)) / COLS as f32).floor();
        if cell < MIN_CELL {
            return self.home_widget_stack(ui, &all);
        }
        let pitch = cell + GAP;
        let origin = ui.cursor().min;
        let rect_of = |c: Cell| Rect::from_min_size(origin + vec2(c.x as f32 * pitch, c.y as f32 * pitch), vec2(c.w as f32 * pitch - GAP, c.h as f32 * pitch - GAP));
        // Widgets whose app isn't installed hide (they show, locked, while
        // customising; they come back when the app does).
        let shown: Vec<WidgetCfg> = all.iter().filter(|w| def(&w.kind).is_some_and(|d| edit || d.app.map_or(true, |a| self.has_app(a)))).cloned().collect();
        // (always inside the grid, whatever the save says)
        let mut saved: Vec<(u64, Cell)> = shown.iter().map(|w| (w.id, fit(Cell::of(w.pos.unwrap_or([0, 0, 2, 2])), lim(def(&w.kind))))).collect();
        settle(&mut saved, None);
        if !edit {
            self.setup.home.drag = None;
            self.setup.home.resize = None;
            self.setup.home.preview = None;
        }
        let pointer = ui.ctx().pointer_interact_pos();
        let released = ui.input(|i| i.pointer.any_released());
        // moving or resizing: where everything would go
        let busy = self.setup.home.drag.or(self.setup.home.resize);
        if let (Some(id), Some(pp)) = (busy, pointer) {
            if let Some(me) = saved.iter().find(|(i, _)| *i == id).map(|x| x.1) {
                let d = shown.iter().find(|w| w.id == id).and_then(|w| def(&w.kind));
                let l = lim(d);
                let ((mw, mh), (xw, xh)) = (l.min, l.max);
                let mut want = me;
                if self.setup.home.drag.is_some() {
                    let at = pp - self.setup.home.grab - origin;
                    want.x = between((at.x / pitch).round() as i32, 0, COLS - me.w);
                    want.y = ((at.y / pitch).round() as i32).max(0);
                } else {
                    let r = rect_of(me);
                    want.w = between(((pp.x - r.min.x + GAP / 2.0) / pitch).round() as i32, mw, xw.min(COLS - me.x));
                    want.h = between(((pp.y - r.min.y + GAP / 2.0) / pitch).round() as i32, mh, xh);
                    // square ones: the bigger pull wins, the other follows
                    if let Some(dh) = l.tall_by {
                        want.w = want.w.max(want.h - dh);
                    }
                    want = fit(want, l);
                }
                let mut next = saved.clone();
                if let Some(x) = next.iter_mut().find(|(i, _)| *i == id) {
                    x.1 = want;
                }
                settle(&mut next, Some(id));
                self.setup.home.preview = Some(next);
            }
        }
        let layout = self.setup.home.preview.clone().unwrap_or_else(|| saved.clone());
        let rows = layout.iter().map(|(_, c)| c.y + c.h).max().unwrap_or(0) + if edit { 2 } else { 0 };
        let full = Rect::from_min_size(origin, vec2(width, (rows as f32 * pitch - GAP).max(0.0)));
        // in Customize: the squares, faintly; click an empty one to add a
        // widget there
        if edit {
            let p = ui.painter().clone();
            let free = busy.is_none();
            for y in 0..rows {
                for x in 0..COLS {
                    let sq = Cell { x, y, w: 1, h: 1 };
                    let r = rect_of(sq);
                    let empty = !layout.iter().any(|(_, c)| c.overlaps(sq));
                    let picked = self.setup.home.add_at == Some((x, y)) && self.setup.home.gallery.open;
                    if empty && free {
                        let resp = ui.interact(r, ui.id().with(("w-sq", x, y)), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || picked, kit::QUICK);
                        if ht > 0.0 {
                            p.rect_filled(r, 8.0, kit::primary().gamma_multiply(0.08 * ht));
                            p.text(r.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(22.0), kit::primary().gamma_multiply(0.4 + 0.5 * ht));
                        }
                        if resp.on_hover_text("Add a widget here").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.setup.home.add_at = Some((x, y));
                            self.setup.home.gallery.open = true;
                        }
                    }
                    p.rect_stroke(r, 8.0, Stroke::new(1.0f32, if picked { kit::primary() } else { theme::blend(kit::PAGE(), LINE(), 0.45) }));
                }
            }
        }
        let mut action: Option<Action> = None;
        let mut lifted: Option<(WidgetCfg, Rect)> = None;
        for w in &shown {
            let Some(c) = layout.iter().find(|(i, _)| *i == w.id).map(|x| x.1) else { continue };
            let target = rect_of(c);
            if self.setup.home.drag == Some(w.id) {
                // where it would land, and the widget following the pointer
                let p = ui.painter();
                p.rect_filled(target, 16.0, kit::primary().gamma_multiply(0.08));
                dashed(p, target, kit::primary());
                let at = pointer.map_or(target.min, |pp| pp - self.setup.home.grab);
                lifted = Some((w.clone(), Rect::from_min_size(at, rect_of(saved.iter().find(|(i, _)| *i == w.id).map_or(c, |x| x.1)).size())));
                continue;
            }
            // others glide to their new places
            let r = if edit { kit::glide_rect(ui.ctx(), egui::Id::new(("w-pos", w.id)), target, origin, kit::BASE) } else { target };
            self.draw_widget_in(ui, w, r, c, edit, false, &mut action);
        }
        if let Some((w, r)) = lifted {
            ui.painter().add(kit::shadow_tier(3).as_shape(r, 16.0));
            let c = layout.iter().find(|(i, _)| *i == w.id).map_or(Cell { x: 0, y: 0, w: 1, h: 1 }, |x| x.1);
            self.draw_widget_in(ui, &w, r, c, edit, true, &mut action);
        }
        if busy.is_some() && released {
            if let Some(next) = self.setup.home.preview.take() {
                action = Some(Action::Layout(next));
            }
            self.setup.home.drag = None;
            self.setup.home.resize = None;
        }
        if busy.is_some() {
            ui.ctx().request_repaint();
        }
        ui.allocate_rect(full, Sense::hover());
        if let Some(a) = action {
            self.apply_widget_action(a);
        }
    }

    /// A window too narrow for the grid: the widgets one under another.
    fn home_widget_stack(&mut self, ui: &mut egui::Ui, all: &[WidgetCfg]) {
        let mut list: Vec<&WidgetCfg> = all.iter().filter(|w| def(&w.kind).is_some_and(|d| d.app.map_or(true, |a| self.has_app(a)))).collect();
        list.sort_by_key(|w| w.pos.map_or((0, 0), |p| (p[1], p[0])));
        if self.setup.home.edit {
            ui.label(egui::RichText::new("Make the window wider to move and resize widgets.").size(kit::fs(12.5)).color(MUTED()));
            ui.add_space(8.0);
        }
        let mut action = None;
        for w in list {
            if let Some(data) = self.draw_widget(ui, w, false) {
                action = Some(Action::Data(w.id, data));
            }
        }
        if let Some(a) = action {
            self.apply_widget_action(a);
        }
    }

    /// Draw one widget filling `r` (cut off with a fade if it has more to
    /// show), with its Customize chrome when editing.
    #[allow(clippy::too_many_arguments)]
    fn draw_widget_in(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, r: Rect, c: Cell, edit: bool, lifted: bool, action: &mut Option<Action>) {
        let Some(d) = def(&w.kind) else { return };
        let locked = d.app.is_some_and(|a| !self.has_app(a));
        let round = kit::card_rounding();
        // the box itself, so a widget shorter than it still fills it
        card_bg(ui.painter(), r, false);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
        // (room above for glows, but nothing spills sideways into the
        // widget next to it)
        let clip = Rect::from_min_max(pos2(r.min.x, r.min.y - 18.0), r.max).intersect(ui.clip_rect());
        child.set_clip_rect(clip);
        child.set_width(r.width());
        kit::set_card_fill(Some(r.height()));
        // the box it fills, for widgets that lay themselves out by it
        let cell = ((r.width() + GAP) / c.w.max(1) as f32 - GAP).max(1.0);
        super::w_fit::set_box(Some(super::w_fit::WBox { r, cols: c.w, rows: c.h, cell }));
        if locked {
            let p = child.painter().clone();
            p.text(r.min + vec2(20.0, 22.0), Align2::LEFT_TOP, format!("{}  {}", d.icon, d.name), crate::fonts::sans_bold_font(15.0), TEXT());
            if let (Some(a), false) = (d.app, edit) {
                self.lock_veil(&mut child, r, a, d.name);
            }
        } else if let Some(data) = self.draw_widget(&mut child, w, false) {
            *action = Some(Action::Data(w.id, data));
        }
        kit::set_card_fill(None);
        super::w_fit::set_box(None);
        // more than fits: fade out at the bottom and close the box neatly
        let used = child.min_rect();
        if used.max.y > r.max.y + 1.0 {
            let p = ui.painter().with_clip_rect(ui.clip_rect());
            let foot = Rect::from_min_max(pos2(r.min.x, r.max.y - round - 2.0), r.max);
            p.rect_filled(foot, egui::Rounding { sw: round, se: round, ..Default::default() }, CARD());
            fade_bottom(&p, Rect::from_min_max(pos2(r.min.x + 1.0, foot.min.y - 34.0), pos2(r.max.x - 1.0, foot.min.y)), CARD());
            p.rect_stroke(r, round, Stroke::new(1.0f32, LINE()));
        }
        if edit {
            if let Some(a) = self.widget_edit_chrome(ui, r, w, d, c, lifted) {
                *action = Some(a);
            }
        }
    }

    /// In Customize: the dashed edge, the bar on top (name, size, remove),
    /// picking the card up to move it, and the corner to resize it.
    fn widget_edit_chrome(&mut self, ui: &mut egui::Ui, r: Rect, w: &WidgetCfg, d: &WidgetDef, c: Cell, lifted: bool) -> Option<Action> {
        let mut out = None;
        // The whole card is the handle (and nothing inside it clicks).
        let body = ui.interact(r, ui.id().with(("w-body", w.id)), Sense::click_and_drag());
        let p = ui.painter().clone();
        let busy = self.setup.home.drag == Some(w.id) || self.setup.home.resize == Some(w.id);
        p.rect_filled(r, 16.0, theme::blend(kit::PAGE(), CARD(), 0.3).gamma_multiply(if lifted { 0.1 } else { 0.25 }));
        dashed(&p, r.expand(4.0), if busy || body.hovered() { kit::primary() } else { kit::CONTROL() });
        if body.drag_started() && self.setup.home.resize.is_none() {
            if let Some(pp) = ui.ctx().pointer_interact_pos() {
                self.setup.home.drag = Some(w.id);
                self.setup.home.grab = pp - r.min;
            }
        }
        if body.hovered() || body.dragged() {
            ui.ctx().set_cursor_icon(if body.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        // the bar: name, size, ×
        // a compact pill (name, size, ×) so the widget shows under it
        let size = format!("{}×{}", c.w, c.h);
        let sg = p.layout_no_wrap(size, crate::fonts::sans_bold_font(11.0), if busy { kit::primary() } else { MUTED() });
        let ng = p.layout_no_wrap(format!("☰  {}", d.name), crate::fonts::sans_bold_font(11.5), TEXT());
        let want_w = ng.size().x + sg.size().x + 60.0;
        let bar = Rect::from_min_size(pos2(r.min.x + 8.0, r.min.y + 8.0), vec2(want_w.min(r.width() - 16.0).max(60.0), 26.0));
        p.rect_filled(bar, 13.0, CARD_HOVER().gamma_multiply(0.96));
        p.rect_stroke(bar, 13.0, Stroke::new(1.0f32, LINE()));
        let name_room = Rect::from_min_max(bar.min + vec2(2.0, 2.0), pos2(bar.max.x - 34.0 - sg.size().x, bar.max.y - 2.0));
        p.with_clip_rect(name_room).galley(pos2(bar.min.x + 10.0, bar.center().y - ng.size().y / 2.0), ng, TEXT());
        p.galley(pos2(bar.max.x - 30.0 - sg.size().x, bar.center().y - sg.size().y / 2.0), sg, MUTED());
        if lifted {
            return None;
        }
        let xr = Rect::from_min_size(pos2(bar.max.x - 24.0, bar.min.y + 3.0), vec2(20.0, 20.0));
        if pill_button(ui, xr, "×", false, ("w-x", w.id)).on_hover_text("Remove this widget").clicked() {
            out = Some(Action::Remove(w.id));
        }
        // the corner: drag to resize
        let corner = Rect::from_min_max(r.max - vec2(26.0, 26.0), r.max);
        let cresp = ui.interact(corner, ui.id().with(("w-size", w.id)), Sense::drag());
        let hot = cresp.hovered() || self.setup.home.resize == Some(w.id);
        let cc = if hot { kit::primary() } else { kit::CONTROL() };
        for k in 0..3 {
            let o = 6.0 + k as f32 * 5.0;
            p.line_segment([r.max - vec2(o, 5.0), r.max - vec2(5.0, o)], Stroke::new(2.0f32, cc));
        }
        if cresp.hovered() || cresp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        }
        if cresp.drag_started() {
            self.setup.home.resize = Some(w.id);
            self.setup.home.drag = None;
        }
        let l = lim(Some(d));
        let ((mw, mh), (xw, xh)) = (l.min, l.max);
        let _ = cresp.on_hover_text(format!("Drag to resize ({mw}×{mh} up to {xw}×{xh}{})", if l.tall_by.is_some() { ", kept square" } else { "" }));
        out
    }

    fn apply_widget_action(&mut self, a: Action) {
        let mut all = self.home_widgets();
        match a {
            Action::Remove(id) => all.retain(|w| w.id != id),
            Action::Data(id, data) => {
                if let Some(w) = all.iter_mut().find(|w| w.id == id) {
                    w.data = data;
                }
            }
            Action::Layout(cells) => {
                for (id, c) in cells {
                    if let Some(w) = all.iter_mut().find(|w| w.id == id) {
                        w.pos = Some(c.save());
                    }
                }
            }
        }
        self.set_home_widgets(all);
    }

    /// Draw one widget's card. `preview`: in the gallery (no settings of
    /// its own yet; show something even when there's nothing to show).
    /// Returns its settings when it changed them.
    pub(super) fn draw_widget(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool) -> Option<Vec<String>> {
        let mut out: Option<Vec<String>> = None;
        match w.kind.as_str() {
            "today" => self.f_today(ui),
            "hunts" => self.f_hunts(ui),
            "almost" => self.f_almost(ui),
            "rare" => self.f_rare(ui),
            "shinies" => self.f_shinies(ui),
            "breeding" => self.f_breeding(ui),
            "jump" => self.f_jump(ui),
            "clock" => self.f_clock(ui),
            "notes" => self.w_notes(ui, w, preview, &mut out),
            "hunt_ctrl" => self.f_hunt(ui),
            "showcase" => self.w_showcase(ui),
            "week" => self.f_week(ui),
            "shinydex" => self.w_shinydex(ui),
            "luck" => self.w_luck(ui),
            "motd" => self.w_motd(ui),
            "dexsearch" => self.w_dexsearch(ui, w),
            "favs" => self.w_favs(ui, w, preview, &mut out),
            "catchable" => self.w_catchable(ui, w, preview, &mut out),
            "typematch" => self.w_typematch(ui, w, preview, &mut out),
            "minimap" => self.w_minimap(ui),
            "mapfavs" => self.w_mapfavs(ui, w, preview, &mut out),
            "huntwhere" => self.w_huntwhere(ui),
            "maprecent" => self.w_maprecent(ui),
            "infonews" => self.w_infonews(ui),
            "tip" => self.w_tip(ui),
            "infosearch" => self.w_infosearch(ui, w),
            "adventure" => self.w_adventure(ui),
            "bookmarks" => self.w_bookmarks(ui),
            "mainteam" => self.w_mainteam(ui),
            "teamweak" => self.w_teamweak(ui),
            "teamswitch" => self.w_teamswitch(ui),
            "coverage" => self.w_coverage(ui),
            "teamidea" => self.w_teamidea(ui),
            "lastbattle" => self.w_lastbattle(ui),
            "winrecord" => self.w_winrecord(ui),
            "quickbattle" => self.w_quickbattle(ui),
            "trainers" => self.w_trainers(ui),
            "battlehistory" => self.w_battlehistory(ui),
            "breeders" => self.w_breeders(ui),
            "eggmoves" => self.w_eggmoves(ui, w, preview, &mut out),
            "nextstep" => self.w_nextstep(ui),
            "hatches" => self.w_hatches(ui),
            "quickcalc" => self.w_quickcalc(ui, w, preview, &mut out),
            "savedcalcs" => self.w_savedcalcs(ui),
            "speedtiers" => self.w_speedtiers(ui),
            "lastcalc" => self.w_lastcalc(ui),
            "threat" => self.w_threat(ui, w, preview, &mut out),
            "cards_xp" => self.w_cards_xp(ui),
            "cards_packs" => self.w_cards_packs(ui),
            "cards_pulls" => self.w_cards_pulls(ui),
            "cards_sets" => self.w_cards_sets(ui),
            "cards_missions" => self.w_cards_missions(ui),
            "gotchi_pet" => self.w_gotchi_pet(ui),
            "gotchi_needs" => self.w_gotchi_needs(ui),
            "gotchi_care" => self.w_gotchi_care(ui),
            "gotchi_pantry" => self.w_gotchi_pantry(ui),
            "gotchi_box" => self.w_gotchi_box(ui),
            "bingo_card" => self.w_bingo_card(ui),
            "bingo_week" => self.w_bingo_week(ui),
            "bingo_next" => self.w_bingo_next(ui),
            "bingo_meet" => self.w_bingo_meet(ui),
            "bingo_recent" => self.w_bingo_recent(ui),
            "rare_out" => self.w_rare_out(ui),
            "rare_new" => self.w_rare_new(ui),
            "rare_stats" => self.w_rare_stats(ui),
            "rare_places" => self.w_rare_places(ui),
            "rare_missing" => self.w_rare_missing(ui),
            "replay_last" => self.w_replay_last(ui),
            "replay_rec" => self.w_replay_rec(ui),
            "replay_recent" => self.w_replay_recent(ui),
            "replay_stats" => self.w_replay_stats(ui),
            "replay_save" => self.w_replay_save(ui),
            _ => {}
        }
        out
    }
}

enum Action {
    Remove(u64),
    /// A widget changed its own settings.
    Data(u64, Vec<String>),
    /// Moved or resized: everyone's new place.
    Layout(Vec<(u64, Cell)>),
}

/// A layout that needs bringing up to date: a widget with no place yet
/// (from before the grid) or two with the same id.
pub(super) fn needs_upgrade(all: &[WidgetCfg]) -> bool {
    all.iter().any(|w| w.pos.is_none()) || all.iter().enumerate().any(|(i, w)| all[..i].iter().any(|o| o.id == w.id))
}

/// A saved layout from any version, made right for this one:
/// - two widgets with the same id: the later one gets a new id;
/// - places from the first grid (`grid` below 2, squares twice the size):
///   doubled;
/// - a widget doubled by mistake in v0.7.1/v0.7.2 (wider than the grid):
///   halved back, and every widget fitted to its own sizes (then whatever
///   overlaps moves down and the room freed is closed up);
/// - widgets from before the grid (v0.7.0, no place): put in the first
///   space each fits, at the size it's added at.
pub(super) fn upgrade(all: &[WidgetCfg], grid: u8) -> Vec<WidgetCfg> {
    let mut fixed: Vec<WidgetCfg> = all.to_vec();
    let mut next = fixed.iter().map(|w| w.id).max().unwrap_or(0) + 1;
    for i in 0..fixed.len() {
        if fixed[..i].iter().any(|o| o.id == fixed[i].id) {
            fixed[i].id = next;
            next += 1;
        }
    }
    if grid < 2 {
        for w in fixed.iter_mut() {
            if let Some(p) = w.pos {
                w.pos = Some([p[0].saturating_mul(2), p[1].saturating_mul(2), p[2].saturating_mul(2), p[3].saturating_mul(2)]);
            }
        }
    }
    if grid < GRID_VERSION {
        let mut cells: Vec<(u64, Cell)> = fixed.iter().filter_map(|w| w.pos.map(|p| (w.id, Cell::of(p)))).collect();
        repair(&mut cells, |id| lim(fixed.iter().find(|w| w.id == id).and_then(|w| def(&w.kind))));
        for w in fixed.iter_mut() {
            if let Some((_, c)) = cells.iter().find(|(i, _)| *i == w.id) {
                w.pos = Some(c.save());
            }
        }
    }
    if fixed.iter().any(|w| w.pos.is_none()) {
        let cells = place_all(&fixed);
        for (w, (_, c)) in fixed.iter_mut().zip(cells.iter()) {
            w.pos = Some(c.save());
        }
    }
    fixed
}

/// Every widget's place: its own, or (from before the grid) the first space
/// that fits it, in order, sized from its old width.
pub(super) fn place_all(all: &[WidgetCfg]) -> Vec<(u64, Cell)> {
    let mut out: Vec<(u64, Cell)> = all.iter().filter_map(|w| w.pos.map(|p| (w.id, Cell::of(p)))).collect();
    for w in all.iter().filter(|w| w.pos.is_none()) {
        // (the size it would be added at today)
        let (cw, ch) = lim(def(&w.kind)).start;
        let c = first_free(&out, cw, ch);
        out.push((w.id, c));
    }
    // back in the widgets' own order
    all.iter().filter_map(|w| out.iter().find(|(i, _)| *i == w.id).copied()).collect()
}

/// A dashed rounded-ish outline.
fn dashed(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.5f32, col);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for w in pts.windows(2) {
        p.extend(egui::Shape::dashed_line(&[w[0], w[1]], s, 8.0, 6.0));
    }
}

/// A widget whose app isn't installed, in the gallery: its name on a
/// plain card with a lock.
fn widget_ghost(ui: &mut egui::Ui, d: &WidgetDef) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 160.0), Sense::hover());
    let p = ui.painter();
    card_bg(p, r, false);
    p.text(r.min + vec2(18.0, 20.0), Align2::LEFT_TOP, format!("{}  {}", d.icon, d.name), crate::fonts::sans_bold_font(14.5), TEXT());
    p.text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "🔒", egui::FontId::proportional(26.0), MUTED());
    if let Some(a) = d.app {
        p.text(r.center() + vec2(0.0, 44.0), Align2::CENTER_CENTER, format!("Comes with {}", a.name()), crate::fonts::sans_font(12.0), MUTED());
    }
}

/// From clear at the top to `col` at the bottom.
fn fade_bottom(p: &egui::Painter, r: Rect, col: Color32) {
    let mut m = egui::Mesh::default();
    let clear = col.gamma_multiply(0.0);
    m.colored_vertex(r.left_top(), clear);
    m.colored_vertex(r.right_top(), clear);
    m.colored_vertex(r.right_bottom(), col);
    m.colored_vertex(r.left_bottom(), col);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widgets_are_unique_and_sized() {
        for (i, d) in DEFS.iter().enumerate() {
            assert!(DEFS.iter().skip(i + 1).all(|e| e.kind != d.kind), "{} twice", d.kind);
            assert!(matches!(d.span, 1 | 2 | FULL), "{} has width {}", d.kind, d.span);
        }
        // five (or more) for every app
        for a in crate::apps::ALL {
            assert!(DEFS.iter().filter(|d| d.app == Some(a)).count() >= 5, "{a:?} needs five widgets");
        }
    }

    #[test]
    fn dashboard_starts_empty_and_old_layouts_load() {
        assert!(default_widgets().is_empty());
        let w: WidgetCfg = serde_json::from_str(r#"{"id":1,"kind":"today","span":9}"#).unwrap();
        assert!(w.data.is_empty());
    }
}

#[cfg(test)]
mod grid_tests {
    use super::*;

    fn c(x: i32, y: i32, w: i32, h: i32) -> Cell {
        Cell { x, y, w, h }
    }

    #[test]
    fn moving_pushes_down_and_keeps_order() {
        let mut items = vec![(1, c(0, 0, 2, 2)), (2, c(2, 0, 2, 2)), (3, c(2, 2, 2, 1))];
        // widget 1 moved onto widget 2's spot
        items[0].1 = c(2, 0, 2, 2);
        settle(&mut items, Some(1));
        assert_eq!(items[0].1, c(2, 0, 2, 2));
        assert_eq!(items[1].1, c(2, 2, 2, 2));
        assert_eq!(items[2].1, c(2, 4, 2, 1));
        // nothing overlaps
        for i in 0..items.len() {
            for j in i + 1..items.len() {
                assert!(!items[i].1.overlaps(items[j].1));
            }
        }
    }

    #[test]
    fn gaps_stay_and_new_ones_fill_the_first_space() {
        let mut items = vec![(1, c(0, 3, 2, 2))];
        settle(&mut items, None);
        assert_eq!(items[0].1, c(0, 3, 2, 2), "nothing floats up");
        assert_eq!(first_free(&items, 2, 2), c(0, 0, 2, 2));
        let full = vec![(1, c(0, 0, COLS, 1))];
        assert_eq!(first_free(&full, 3, 2), c(0, 1, 3, 2));
    }

    #[test]
    fn adding_where_clicked_pushes_down() {
        let mut items = vec![(1, c(0, 0, 2, 2)), (2, c(2, 1, 2, 2))];
        items.push((3, c(2, 0, 3, 2)));
        settle(&mut items, Some(3));
        assert_eq!(items[2].1, c(2, 0, 3, 2));
        assert_eq!(items[1].1, c(2, 2, 2, 2));
        assert_eq!(items[0].1, c(0, 0, 2, 2));
    }

    impl Cell {
        fn min_with(self, l: Lim) -> Cell {
            fit(self, l)
        }
    }

    fn l(min: (i32, i32), max: (i32, i32)) -> Lim {
        Lim { min, start: min, max, tall_by: None }
    }

    fn fitm(c: Cell, m: (i32, i32)) -> Cell {
        fit(c, l(m, (MAX_SIDE, MAX_SIDE)))
    }

    #[test]
    fn places_are_always_possible() {
        // wider than the grid, too tall, past the right edge, too small
        assert_eq!(fitm(c(0, 0, 16, 6), (6, 2)), c(0, 0, 8, 6));
        assert_eq!(fitm(c(0, 0, 4, 12), (2, 2)), c(0, 0, 4, 8));
        assert_eq!(fitm(c(10, 2, 6, 3), (6, 2)), c(6, 2, 6, 3));
        assert_eq!(fitm(c(11, 0, 1, 1), (3, 4)), c(9, 0, 3, 4));
        assert_eq!(fitm(c(-2, -1, 4, 3), (2, 2)), c(0, 0, 4, 3));
        // fine already: unchanged
        assert_eq!(fitm(c(4, 7, 8, 8), (6, 2)), c(4, 7, 8, 8));
        // never a panic when the limits cross
        assert_eq!(between(5, 6, 4), 6);
        assert_eq!(between(-3, 0, -4), 0);
    }

    #[test]
    fn today_at_a_glance_is_capped() {
        let d = def("today").unwrap();
        assert_eq!((lim(Some(d)).min, lim(Some(d)).max), ((2, 1), (8, 4)));
        // an old 8×5 (or the doubled 16×6) comes back no bigger than 8×4
        assert_eq!(fit(c(0, 0, 8, 5), lim(Some(d))), c(0, 0, 8, 4));
        assert_eq!(fit(c(0, 0, 16, 6), lim(Some(d))), c(0, 0, 8, 4));
    }

    #[test]
    fn every_widget_has_its_own_sizes() {
        for d in DEFS {
            let l = lim_of(d.kind).unwrap_or_else(|| panic!("{} has no sizes", d.kind));
            assert!(l.min.0 <= l.start.0 && l.start.0 <= l.max.0 && l.min.1 <= l.start.1 && l.start.1 <= l.max.1, "{}", d.kind);
            assert!(l.max.0 <= MAX_SIDE && l.max.1 <= MAX_SIDE && l.min.0 >= 1 && l.min.1 >= 1, "{}", d.kind);
            assert_eq!(fit(c(0, 0, l.start.0, l.start.1), l), c(0, 0, l.start.0, l.start.1), "{} starts at a size it can be", d.kind);
        }
    }

    #[test]
    fn square_widgets_stay_square() {
        let map = lim(def("minimap"));
        assert_eq!(fit(c(0, 0, 6, 3), map), c(0, 0, 6, 6));
        assert_eq!(fit(c(0, 0, 1, 9), map), c(0, 0, 2, 2));
        assert_eq!(fit(c(6, 0, 8, 8), map), c(4, 0, 8, 8));
        // the bingo card: a square under its header
        let card = lim(def("bingo_card"));
        assert_eq!(fit(c(0, 0, 5, 3), card), c(0, 0, 5, 6));
        assert_eq!(fit(c(0, 0, 8, 8), card), c(0, 0, 7, 8));
    }

    #[test]
    fn widgets_doubled_by_mistake_are_halved_back() {
        // Today at a glance added to an empty Dashboard on v0.7.1/v0.7.2
        // (8×3 doubled to 16×6), then two more added the right way
        let mins = |id: u64| l(if id == 1 { (6, 2) } else { (2, 2) }, (MAX_SIDE, MAX_SIDE));
        // (the two under it move up into the room it no longer takes)
        let mut items = vec![(1, c(0, 0, 16, 6)), (2, c(0, 6, 4, 3)), (3, c(4, 6, 8, 3))];
        repair(&mut items, mins);
        assert_eq!(items, vec![(1, c(0, 0, 8, 3)), (2, c(0, 3, 4, 3)), (3, c(4, 3, 8, 3))]);
        // added where clicked (x 4, y 2), doubled to (8, 4) 16×6: halved,
        // and up to the top (nothing above it)
        let mut items = vec![(1, c(8, 4, 16, 6))];
        repair(&mut items, mins);
        assert_eq!(items, vec![(1, c(4, 0, 8, 3))]);
        // a bingo card doubled to 8×10: back to 4×5
        let mut items = vec![(1, c(0, 0, 8, 10))];
        repair(&mut items, |_| Lim { min: (3, 4), start: (4, 5), max: (7, 8), tall_by: Some(1) });
        assert_eq!(items, vec![(1, c(0, 0, 4, 5))]);
        // an odd impossible place is fitted, and what it now covers moves down
        let mut items = vec![(1, c(0, 0, 13, 3)), (2, c(9, 0, 3, 3))];
        repair(&mut items, mins);
        assert_eq!(items[0].1, c(0, 0, 8, 3));
        assert_eq!(items[1].1, c(9, 0, 3, 3));
        let mut items = vec![(1, c(5, 0, 9, 3)), (2, c(3, 0, 3, 3))];
        repair(&mut items, mins);
        assert_eq!(items[1].1, c(3, 0, 3, 3));
        assert_eq!(items[0].1, c(4, 3, 8, 3), "fitted onto widget 2, so moved below it");
        // a good layout is left exactly as it was
        let good = vec![(1, c(0, 0, 8, 3)), (2, c(8, 0, 4, 4)), (3, c(0, 3, 6, 6))];
        let mut items = good.clone();
        repair(&mut items, mins);
        assert_eq!(items, good);
    }

    /// No widget overlaps another, and each is inside the grid at a size
    /// it can be.
    fn sound(ws: &[WidgetCfg]) {
        let cells: Vec<(u64, Cell, &str)> = ws.iter().map(|w| (w.id, Cell::of(w.pos.expect("placed")), w.kind.as_str())).collect();
        for (i, (id, c, k)) in cells.iter().enumerate() {
            assert!(c.x >= 0 && c.y >= 0 && c.x + c.w <= COLS && c.w <= MAX_SIDE && c.h <= MAX_SIDE, "{k} at {c:?}");
            if let Some(d) = def(k) {
                assert_eq!(fit(*c, lim(Some(d))), *c, "{k} at {c:?} isn't a size it can be");
            }
            for (jd, o, ok) in cells.iter().skip(i + 1) {
                assert!(!c.overlaps(*o), "{k} ({id}) overlaps {ok} ({jd})");
            }
            assert!(cells.iter().filter(|x| x.0 == *id).count() == 1, "id {id} twice");
        }
    }

    fn load(json: &str) -> Vec<WidgetCfg> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn every_old_save_upgrades_cleanly() {
        // v0.6.x: no Dashboard at all (an empty one)
        assert!(upgrade(&[], 0).is_empty());
        // v0.7.0: widgets in a list, no places, any width
        let v070 = load(r#"[{"id":1,"kind":"today","span":9},{"id":2,"kind":"rare","span":1},{"id":3,"kind":"shinies","span":2},{"id":4,"kind":"jump","span":9},{"id":5,"kind":"notes","span":1,"data":["hello"]}]"#);
        let up = upgrade(&v070, 0);
        sound(&up);
        assert_eq!(up.len(), 5);
        assert_eq!(up[4].data, vec!["hello".to_string()], "settings kept");
        // v0.7.1/v0.7.2: the first widget doubled by mistake, the rest fine
        let doubled = load(r#"[{"id":1,"kind":"today","span":9,"pos":[0,0,16,6]},{"id":2,"kind":"almost","span":1,"pos":[0,6,4,3]},{"id":3,"kind":"jump","span":9,"pos":[4,6,8,3]}]"#);
        let up = upgrade(&doubled, 2);
        sound(&up);
        assert_eq!(Cell::of(up[0].pos.unwrap()), c(0, 0, 8, 3).min_with(lim(def("today"))));
        // v0.7.1/v0.7.2: a normal layout, some bigger than their new caps
        let normal = load(r#"[{"id":1,"kind":"clock","span":1,"pos":[0,0,8,8]},{"id":2,"kind":"minimap","span":2,"pos":[8,0,4,4]},{"id":3,"kind":"luck","span":1,"pos":[0,8,8,6]},{"id":4,"kind":"bingo_card","span":1,"pos":[8,4,4,4]}]"#);
        let up = upgrade(&normal, 2);
        sound(&up);
        // a just-added widget saved while the grid version was still 0
        // (doubled on its first draw in v0.7.1/v0.7.2): comes back as added
        let fresh = load(r#"[{"id":1,"kind":"today","span":9,"pos":[0,0,8,1]}]"#);
        let up = upgrade(&fresh, 0);
        sound(&up);
        assert_eq!(Cell::of(up[0].pos.unwrap()), c(0, 0, 8, 1));
        // the developer builds' first grid (6 columns): doubled into this one
        let six = load(r#"[{"id":1,"kind":"rare","span":1,"pos":[0,0,2,2]},{"id":2,"kind":"week","span":1,"pos":[2,0,4,1]}]"#);
        let up = upgrade(&six, 0);
        sound(&up);
        assert_eq!(Cell::of(up[0].pos.unwrap()), c(0, 0, 4, 4));
        assert_eq!(Cell::of(up[1].pos.unwrap()), c(4, 0, 8, 2));
    }

    #[test]
    fn deep_dashboards_keep_their_rows() {
        // every widget at its biggest, stacked: well past row 255
        let all: Vec<WidgetCfg> = DEFS.iter().enumerate().map(|(i, d)| WidgetCfg { id: i as u64 + 1, kind: d.kind.to_string(), span: d.span, data: Vec::new(), pos: None }).collect();
        let mut y = 0;
        let mut ws = all.clone();
        for (w, d) in ws.iter_mut().zip(DEFS.iter()) {
            let (mw, mh) = max_size(d);
            w.pos = Some(Cell { x: 0, y, w: mw, h: mh }.save());
            y += mh;
        }
        assert!(y > 255);
        let json = serde_json::to_string(&ws).unwrap();
        let back: Vec<WidgetCfg> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last().unwrap().pos, ws.last().unwrap().pos);
        assert!(!needs_upgrade(&back));
        // and an old save's small numbers still read
        let old: Vec<WidgetCfg> = serde_json::from_str(r#"[{"id":1,"kind":"today","span":9,"pos":[0,0,8,1]}]"#).unwrap();
        assert_eq!(old[0].pos, Some([0, 0, 8, 1]));
    }

    #[test]
    fn broken_saves_upgrade_without_trouble() {
        // nonsense places, zero sizes, the same id twice, a widget this
        // version doesn't know, and one with no place among placed ones
        let bad = load(r#"[{"id":1,"kind":"today","span":9,"pos":[250,250,0,0]},{"id":1,"kind":"clock","span":1,"pos":[11,0,255,255]},{"id":7,"kind":"from_the_future","span":1,"pos":[3,3,3,3]},{"id":8,"kind":"week","span":1},{"id":9,"kind":"minimap","span":2,"pos":[0,0,7,2]}]"#);
        let up = upgrade(&bad, 2);
        assert_eq!(up.len(), 5, "nothing lost");
        assert!(!needs_upgrade(&up));
        // the ones this version knows are sound
        let known: Vec<WidgetCfg> = up.iter().filter(|w| def(&w.kind).is_some()).cloned().collect();
        sound(&known);
        // and a second pass changes nothing
        assert_eq!(upgrade(&up, GRID_VERSION), up);
    }

    #[test]
    fn old_layouts_get_places_once() {
        let old: Vec<WidgetCfg> = serde_json::from_str(r#"[{"id":1,"kind":"today","span":9},{"id":2,"kind":"rare","span":1},{"id":3,"kind":"shinies","span":1}]"#).unwrap();
        let cells = place_all(&old);
        assert_eq!(cells, vec![(1, c(0, 0, 8, 1)), (2, c(8, 0, 4, 2)), (3, c(0, 1, 4, 2))]);
        for d in DEFS {
            let Lim { min: (mw, mh), max: (xw, xh), .. } = lim(Some(d));
            let (sw, sh) = start_size(d);
            assert!(mw <= sw && mh <= sh && sw <= xw && sh <= xh && xw <= MAX_SIDE && xh <= MAX_SIDE, "{}", d.kind);
        }
    }
}
