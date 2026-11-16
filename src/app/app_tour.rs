//! First-time tours (the UI Lab's spotlight tour). The first time you open an
//! app, everything dims except one part of its page, a little card explains
//! it in a line or two, then the spotlight glides to the next part. Each
//! section of an app (a tab, a sub-page) has its own short tour too, the
//! first time you open it, and the Dashboard has one for the Companion itself
//! (the sidebar, the overlay button and Settings). Skip ends the tour and
//! turns off the rest of that app's tours; "Don't show any more tours" on
//! the card turns every tour off (Settings → Appearance turns them back on,
//! or shows them all again).
//!
//! A part further down the page is scrolled into view before the spotlight
//! lands on it (the page area does the scrolling, asked through
//! `request_scroll`).
//!
//! Pages say which tour belongs to them with `here`, and where their parts
//! are with `mark` / `mark_since`, as they draw; the tour reads those the
//! same frame. A step whose part isn't on screen is passed over.
//!
//! Tour ids: an app's main tour is its slug ("cards"); a section inside its
//! page is "slug/section" ("cards/packs"), and its main tour plays first; a
//! page of its own is "slug-page" ("shiny-hunt"). The Companion's own tour is
//! "home" (the Dashboard). Parts outside the page (the sidebar: "nav.…", the
//! top bar: "top.…") are never scrolled or cut to the page.

use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Sense, Stroke};
use std::time::{Duration, Instant};

// ───────────────────────────── marks ─────────────────────────────

/// Say where a part of the page is (screen rect), this frame.
pub(super) fn mark(ctx: &egui::Context, key: &str, r: Rect) {
    let n = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("tour-mark", key)), (r, n)));
}

/// Mark everything drawn in `ui` since `top` (a y taken before drawing it),
/// across the full width.
pub(super) fn mark_since(ui: &egui::Ui, key: &str, top: f32) {
    let m = ui.max_rect();
    let bottom = ui.cursor().min.y - ui.spacing().item_spacing.y;
    if bottom > top + 4.0 {
        mark(ui.ctx(), key, Rect::from_min_max(pos2(m.min.x, top), pos2(m.max.x, bottom)));
    }
}

/// Say which tour belongs to what's on screen (the last call in a frame wins,
/// so a page's section calls it after the page does).
pub(super) fn here(ctx: &egui::Context, id: &'static str) {
    let n = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("tour-here"), (id, n)));
}

fn here_now(ctx: &egui::Context) -> Option<&'static str> {
    let n = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(&'static str, u64)>(egui::Id::new("tour-here"))).filter(|(_, k)| *k == n).map(|(id, _)| id)
}

/// The app a tour id belongs to ("shiny-hunt", "cards/packs" → its slug).
fn root(id: &str) -> &str {
    id.split(['/', '-']).next().unwrap_or(id)
}

pub(super) fn find(ctx: &egui::Context, key: &str) -> Option<Rect> {
    let n = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(Rect, u64)>(egui::Id::new(("tour-mark", key)))).filter(|(_, k)| n.saturating_sub(*k) <= 1).map(|(r, _)| r)
}

// ───────────────────────────── the tours ─────────────────────────────

struct Step {
    /// Which part: a mark's key, or "page" for the whole page.
    key: &'static str,
    title: &'static str,
    text: &'static str,
    /// Something the page does while this step shows, so you see what it's
    /// about (open a panel, turn a mode on); undone when the tour ends. ""
    /// for nothing. Pages ask with `wants` / `wants_undo`.
    act: &'static str,
}

const fn s(key: &'static str, title: &'static str, text: &'static str) -> Step {
    Step { key, title, text, act: "" }
}

/// A step that has the page do something while it shows (see `Step::act`).
const fn sa(key: &'static str, title: &'static str, text: &'static str, act: &'static str) -> Step {
    Step { key, title, text, act }
}

fn act_id(act: &str, undo: bool) -> egui::Id {
    egui::Id::new(("tour-act", act, undo))
}

/// Whether a tour step wants the page to do `act` now (true once).
pub(super) fn wants(ctx: &egui::Context, act: &str) -> bool {
    take_act(ctx, act_id(act, false))
}

/// Whether the tour has ended and `act` should be undone (true once).
pub(super) fn wants_undo(ctx: &egui::Context, act: &str) -> bool {
    take_act(ctx, act_id(act, true))
}

fn take_act(ctx: &egui::Context, id: egui::Id) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let Some(n) = ctx.data(|d| d.get_temp::<u64>(id)) else { return false };
    ctx.data_mut(|d| d.remove::<u64>(id));
    // (a request nobody took for a second or so is stale)
    pass.saturating_sub(n) < 60
}

fn ask_act(ctx: &egui::Context, act: &str, undo: bool) {
    let n = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(act_id(act, undo), n));
    ctx.request_repaint();
}

/// Tours that explain each of their tabs in turn, with what's on each one
/// (open the tab, then its parts, then the next tab): the tabs' own tours
/// are covered by them, so they don't play again after.
const COVERS: &[(&str, &[&str])] = &[
    ("cards", &["cards/binder", "cards/packs", "cards/missions", "cards/trade"]),
    ("team-team", &["team-team/build", "team-team/analysis", "team-team/checklist"]),
];

/// Every tour: its id and its steps.
static TOURS: &[(&str, &[Step])] = &[
    // ── The Companion itself: the first time you see the Dashboard
    (
        "home",
        &[
            s("header", "Welcome to the Companion", "This is your Dashboard. Here's a quick look around: it only takes a minute."),
            s("home.hero", "Your current hunt", "What you're hunting right now, how many you've met and your odds."),
            s("home.widgets", "Your widgets", "Little cards with your stats, today's rare grass and more. Customize lets you add, move, resize and remove them."),
            s("nav.search", "Search", "Find an app, a Pokémon, a page or a setting. Ctrl+K jumps here from anywhere."),
            s("nav.home", "Dashboard", "Brings you back here, whatever you're doing."),
            s("nav.store", "App Store", "Every app the Companion has. Get the ones you want: each one shows you around the first time you open it."),
            s("nav.library", "Library", "The apps you've got, ready to open. Open apps show up in the sidebar, or in the bottom dock if you use it."),
            s("nav.dock", "Your apps, on the dock", "Your pinned and open apps. Click one to jump to it; + adds another."),
            sa("nav.dockedit", "Edit the dock", "Press the pencil to pin, unpin or close apps, or drag them into the order you like (it's on now, so you can see). Press it again (or Esc) when you're done.", "dock.edit"),
            s("nav.profile", "Profile", "Your trainer card, your level and the rewards you've earned."),
            s("nav.stats", "Stats", "Every encounter and shiny you've counted, across all your hunts."),
            s("top.start", "Start the overlay", "Puts a small counter on top of the game that counts every encounter by itself. It's up here on every page."),
            s("top.settings", "Settings", "Every app's settings in one place, plus appearance and hotkeys. It opens at the app you're on. You can turn these tours back on here too."),
        ],
    ),
    // ── What's new: once, on the Dashboard, for anyone who'd already had the
    // welcome (new users get the welcome instead)
    (
        "whatsnew",
        &[
            s("none", "A few things have moved", "The Companion has had a tidy up. Here's where things are now: it only takes a moment."),
            s("top.settings", "Settings, all in one place", "Every app's settings now live in one window, with your apps listed down the side. It opens at the app you're on, and each app's own ⚙ takes you there too."),
            s("top.settings", "What moved into Settings", "Shiny Hunter's Counting and Overlay pages, and Sessions & breaks (Settings → Shiny Hunter). The Card Collector's card art, XP bar and playmat. Pokégotchi, Journal, Shiny Replay and Sticker Book settings."),
            sa("nav.dockedit", "Editing the dock", "The pin and close buttons on dock apps only show while you edit now, and you can drag apps into the order you like (it's on, so you can see). Press the pencil, and again (or Esc) when you're done.", "dock.edit"),
            s("top.settings", "Try the new look", "Settings → Appearance → Modern Plus (preview): your pages as tabs along the top, your apps on a dock that tucks away, and your hunt riding on the dock. Modern stays as it is."),
        ],
    ),
    // ── Modern Plus's welcome (new and old users alike: it moves things)
    (
        "plushome",
        &[
            s("none", "Welcome to Modern Plus", "The new look moves a few things around. Here's where everything is now."),
            s("nav.tabs", "Your pages are tabs", "Dashboard, App Store, Library, Profile and Stats sit along the top now, instead of a sidebar. The app you're in shows beside them."),
            s("nav.search", "Search", "Search moved up here too. Find an app, a Pokémon, a page or a setting; Ctrl+K jumps here from anywhere."),
            s("top.start", "Your hunt, on the dock", "Your target and count. Start overlay is right here; click the rest to open Shiny Hunter. It stays on screen when the dock tucks away."),
            s("nav.dock", "Your apps, on the dock", "Your apps live on the dock at the bottom now. It tucks away while you work: move to the bottom of the window to bring it back."),
            sa("nav.dockedit", "Edit the dock", "Press the pencil to pin, unpin or close apps, or drag them into the order you like (it's on now, so you can see). Press it again (or Esc) when you're done.", "dock.edit"),
            s("top.settings", "Settings, all in one place", "Every app's settings, appearance and hotkeys in one window. It opens at the app you're on. Counting and the Overlay moved in here too, and you can switch back to Modern here."),
        ],
    ),
    // ── Shiny Hunter
    (
        "shiny",
        &[
            s("header", "Shiny Hunter", "Your home for hunting: what you're after, how many you've met, and the overlay that counts for you."),
            s("shiny.hero", "Your current hunt", "Your target, encounters so far and your odds. Press + New hunt to pick a Pokémon and a method, step by step."),
            s("overlay.button", "Start the overlay", "Puts a small counter on top of the game. It counts every encounter by itself, so you just play. Click it again for the overlay menu."),
            s("top.settings", "Its settings", "Shiny Hunter's settings are in Settings (this ⚙), under Shiny Hunter: Counting, the Overlay's look, the overlay window options, and Sessions & breaks."),
            s("shiny.pages", "More hunting pages", "Hunt and Runs have their own pages: your target and odds, and every hunt you've done."),
        ],
    ),
    (
        "shiny-hunt",
        &[
            s("hunt.target", "Your target", "Pick the Pokémon you're hunting: search by name or Pokédex number. Your encounters count towards it from then on."),
            s("hunt.method", "How you're hunting", "Wild battles, Sweet Scent, eggs or Masuda, plus your Shiny Charms and Lure. They all change your odds."),
            s("hunt.odds", "Your odds", "Your chance of a shiny as 1 in this many, and how your luck is going so far."),
        ],
    ),
    (
        "shiny-counting",
        &[
            s("count.hero", "Is it counting?", "Whether it's counting right now, and your encounters live."),
            s("count.source", "Where encounters come from", "Game log reads every wild Pokémon from the game itself, exact even with the overlay hidden. Screen reading is the backup."),
            s("count.feed", "Live feed", "Every battle shows up here the moment it's counted."),
            s("count.auto", "Automation", "Pause counting when the game closes, and the shiny star alert."),
            s("count.keys", "Hotkeys", "Keys to change the count without leaving the game."),
            s("count.names", "Name reading", "Only needed for screen reading or the star alert: draw a box over each enemy name once."),
        ],
    ),
    (
        "shiny-runs",
        &[
            s("runs.summary", "Your totals", "Runs on the go, shinies caught and all your encounters."),
            s("runs.now", "Your current run", "What you're hunting now. You can put it aside and pick it up again any time."),
            s("below:runs.now", "Every run", "Search, filter and sort every hunt you've started, saved or caught. Pick any of them back up."),
        ],
    ),
    (
        "shiny-overlay",
        &[
            s("ov.preview", "Live preview", "Exactly how your overlay looks. It changes as you change the settings."),
            s("ov.settings", "Make it yours", "Its theme, what it shows, its size and its sounds."),
        ],
    ),
    // ── Modern Plus's own versions, where the look changes what a step
    // points at ("plus:" + the tour; used instead of the trimmed tour)
    (
        "plus:shiny",
        &[
            s("header", "Shiny Hunter", "Your home for hunting: what you're after, how many you've met, and the overlay that counts for you."),
            s("shiny.hero", "Your current hunt", "Your target, encounters so far and your odds. Press + New hunt to pick a Pokémon and a method, step by step."),
            s("top.start", "Start the overlay", "Your hunt rides on the dock. Start overlay puts a small counter on top of the game that counts every encounter by itself."),
            s("top.settings", "Its settings", "Shiny Hunter's settings are in Settings (this ⚙), under Shiny Hunter: Counting, the Overlay's look, the overlay window options, and Sessions & breaks."),
            s("shiny.pages", "More hunting pages", "Hunt and Runs have their own pages: your target and odds, and every hunt you've done."),
        ],
    ),
    (
        "plus:shiny-counting",
        &[
            s("count.hero", "Is it counting?", "Whether it's counting right now, and your encounters live."),
            s("count.source", "Where encounters come from", "Game log reads every wild Pokémon from the game itself, exact even with the overlay hidden. Screen reading is the backup."),
            s("count.feed", "Live feed", "Every battle shows up here the moment it's counted."),
            s("count.auto", "Automation", "Pause counting when the game closes, and the shiny star alert."),
            s("count.names", "Name reading", "Only needed for screen reading or the star alert: draw a box over each enemy name once."),
        ],
    ),
    (
        "plus:pokedex",
        &[
            s("header", "Pokédex", "Every Pokémon in Platinum Together, and how much of it you've caught."),
            s("dex.progress", "Your progress", "How many you've caught so far. Your game's own Pokédex and your Shiny Dex show here too."),
            sa("dex.tools", "Find any Pokémon", "Type a name or a number (25 or #0025). Filters (open now) narrows it down by type, how to get it, era, or caught; each one shows as a chip you can remove.", "dex.filters"),
            s("below:dex.tools", "Open one", "Click a Pokémon to open its page. Its page shows you around the first time."),
        ],
    ),
    (
        "plus:rare",
        &[
            s("rare.hero", "Today's rare grass", "The rarest Pokémon out on rare grass today, and where to find it."),
            s("rare.search", "Search and filter", "Find a Pokémon or a place. Show what's out today, ones seen only once, or everything."),
            s("rare.search", "MODERN POKéMON", "Set this to match your world's option: it changes which Pokémon can show up."),
            s("rare.today", "Where to go", "Every spot today. Click a card to open it on the Map; hover it for Hunt it, which makes it your target."),
            s("rare.glossary", "Every Pokémon seen", "Everything that's ever been on rare grass, so you can plan future hunts."),
        ],
    ),
    (
        "plus:gotchi",
        &[
            s("header", "Pokégotchi", "One of your caught shinies, as a little pet."),
            s("gotchi.device", "Your pet", "Feed it and play with it. Hunting earns berries: one for every 10 Pokémon you meet."),
            s("gotchi.side", "Looking after it", "How Full, Happy and rested it is. Lights off lets it sleep."),
            s("gotchi.box", "Your shinies", "Any shiny you've caught can come out. Right-click one to rename it or send it back."),
            s("top.settings", "Its settings", "Its name, frame and how it shows on the overlay are in Settings (this ⚙), under Pokégotchi. The Settings button on its page goes there too."),
        ],
    ),
    // ── "Its settings have moved": once, for anyone who'd already had an
    // app's tour before its settings moved into the Settings window
    // ("moved:" + the app; new users get the step in the app's own tour)
    ("moved:shiny", &[s("top.settings", "Shiny Hunter's settings have moved", "They're all in Settings now (this ⚙), under Shiny Hunter: Counting and the Overlay pages, the overlay window options, and Sessions & breaks from the Hunt page.")]),
    ("moved:cards", &[s("top.settings", "Card Collector's settings have moved", "Card art downloads, the XP bar on the overlay and your playmat are in Settings now (this ⚙), under Card Collector.")]),
    ("moved:gotchi", &[s("top.settings", "Pokégotchi's settings have moved", "Its name, frame and how it shows on the overlay are in Settings now (this ⚙), under Pokégotchi. The Settings button on its page takes you there too.")]),
    ("moved:replay", &[s("top.settings", "Shiny Replay's settings have moved", "Quality, frame rate, where clips go, and recording on or off are in Settings now (this ⚙), under Shiny Replay. The ⚙ on its page takes you there too.")]),
    ("moved:journal", &[s("top.settings", "Journal's settings have moved", "Text size, line length, the right panel and the daily notes folder are in Settings now (this ⚙), under Journal. The ⚙ in the Journal takes you there too.")]),
    ("moved:stickers", &[s("top.settings", "Sticker Book's settings have moved", "The paper and the sticker size are in Settings now (this ⚙), under Sticker Book. The ⚙ on its page takes you there too.")]),
    (
        "plus:cards",
        &[
            s("header", "Card Collector", "Collect the original Pokémon cards, just by hunting."),
            s("cards.hero", "Your XP bar", "Every Pokémon you meet adds XP. Fill the bar and you get a booster pack."),
            s("tab.0", "Binder", "Every card you own, set by set."),
            s("below:binder.filters", "Your cards", "Search and filter them, and click one to see it up close."),
            s("tab.1", "Packs", "Your booster packs."),
            s("packs.next", "The next set", "A new set comes out every Monday at the game's daily reset (midnight CET). This counts down to the next one. Remind me tells you the moment it's out, and its card art can download by itself."),
            s("packs.boosters", "Open a pack", "Pick a booster and tear it open along the top, then click each card to flip it over."),
            s("tab.2", "Missions", "Daily and weekly missions. They tick themselves as you play and pay out XP."),
            s("tab.3", "Trade", "Send a card to a friend with a code, or claim one they sent you."),
            s("top.settings", "Its settings", "Card art downloads, the XP bar on the overlay and your playmat are in Settings (this ⚙), under Card Collector."),
        ],
    ),
    (
        "plus:cards/packs",
        &[
            s("packs.next", "The next set", "A new set comes out every Monday at the game's daily reset (midnight CET). This counts down to the next one. Remind me tells you the moment it's out, and its card art can download by itself."),
            s("packs.boosters", "Open a pack", "Pick a booster and tear it open along the top, then click each card to flip it over."),
            s("packs.pulls", "Latest pulls", "Your best recent cards, best on top."),
            s("packs.art", "Card art", "What card art is on this PC. Downloading sets and picking the playmat are in Settings → Card Collector."),
        ],
    ),
    // ── Pokédex
    (
        "pokedex",
        &[
            s("header", "Pokédex", "Every Pokémon in Platinum Together, and how much of it you've caught."),
            s("dex.progress", "Your progress", "How many you've caught so far. Your game's own Pokédex and your Shiny Dex show here too."),
            s("dex.tools", "Find any Pokémon", "Type a name or a number (25 or #0025), then narrow it down by type, how to get it, era, or caught and not caught."),
            s("below:dex.tools", "Open one", "Click a Pokémon to open its page. Its page shows you around the first time."),
        ],
    ),
    (
        "pokedex-mon",
        &[
            s("dex.entry", "Its page", "Its types, stats and abilities. The buttons here show it on the Map, check catch odds, plan breeding it, log a shiny or hunt it."),
            s("tab.0", "Overview", "The basics: stats, abilities and its Pokédex entry."),
            s("tab.1", "Evolution", "How it evolves, and what into."),
            s("tab.2", "Where to find it", "Every place it lives in Platinum Together, and how common it is there."),
            s("tab.3", "Moves", "Everything it can learn: by level, TMs, tutors and egg moves."),
            s("tab.4", "TCG", "Its cards from the Card Collector."),
        ],
    ),
    // ── Map
    (
        "map",
        &[
            s("map.canvas", "The map", "Sinnoh, drawn from your game's own files. Drag to move around, scroll to zoom, and click an area to open it."),
            s("map.search", "Search", "Find a place or a Pokémon. Press / to jump here."),
            sa("map.tabs", "Area or rare grass", "Switch between the area you clicked and today's rare grass list (open here now).", "map.rare"),
            s("map.panel", "What's there", "Its Pokémon, today's rare grass and the items lying around. Right-click a rare grass Pokémon to hunt it there."),
        ],
    ),
    // ── Game Info
    (
        "info",
        &[
            s("header", "Game Info", "Everything in Platinum Together, from the game's own data."),
            s("info.search", "Search everything", "Type a Pokémon, move, item, place or guide. A type or a dex number works too."),
            s("info.front", "Browse", "Guides, moves, items and places. Click anything to read its page; pages link on to the Pokédex and the Map."),
        ],
    ),
    // ── Team Planner
    (
        "team",
        &[
            s("header", "Team Planner", "Build teams of up to six, see what they cover, and plan how to get every part in the game."),
            s("team.actions", "Start a team", "+ New team starts one, or paste a team from Showdown. Search finds a team, or a Pokémon in any of them."),
            s("below:team.actions", "Your teams", "Click a team to plan it. Right-click one to copy or delete it."),
        ],
    ),
    (
        "team-team",
        &[
            s("team.bar", "This team", "Rename it, put it in a folder, pick Platinum or MODERN, and send it to the Damage Calculator or Battle Sim."),
            s("tab.0", "Build", "Pick moves, items and EVs for each one."),
            s("team.slots", "Your six", "Click one to edit it, or + to add a Pokémon. Right-click one to move, copy or remove it."),
            s("below:team.slots", "Edit it", "Moves (only ones it can really learn here), item, Ability, nature and EVs, with its stats live."),
            s("tab.1", "Analysis", "What your team covers, and what it's weak to."),
            s("team.analysis", "Analysis", "What hits your team hard, what you hit hard, and who's fastest."),
            s("tab.2", "Checklist", "How to get every part of the team in the game."),
            s("team.checklist", "Checklist", "Where to catch each one, the TMs and tutors you need, and items to buy. Tick things off as you go."),
        ],
    ),
    (
        "team-team/build",
        &[
            s("team.slots", "Your six", "Click one to edit it, or + to add a Pokémon. Right-click one to move, copy or remove it."),
            s("below:team.slots", "Edit it", "Moves (only ones it can really learn here), item, Ability, nature and EVs, with its stats live."),
        ],
    ),
    ("team-team/analysis", &[s("team.analysis", "Analysis", "What hits your team hard, what you hit hard, and who's fastest.")]),
    ("team-team/checklist", &[s("team.checklist", "Checklist", "Where to catch each one, the TMs and tutors you need, and items to buy. Tick things off as you go.")]),
    // ── Battle Sim
    (
        "sim",
        &[
            s("header", "Battle Sim", "Practice battles with your teams before you do them for real."),
            s("sim.modes", "Pick a battle", "Wild Pokémon, trainers, or the Catch Lab to test your catch odds."),
            s("sim.setup", "Set it up", "Pick your team and who you're facing, singles or doubles, then start. It uses Platinum's own battle rules."),
        ],
    ),
    // ── Breeding Planner
    (
        "breed",
        &[
            s("header", "Breeding Planner", "Everything for breeding in one place, read from your save as you play."),
            s("breed.tabs", "Its pages", "Desk is your game right now. Plan sets your goal. Then Egg moves, Breeders and the Hatch log. Each one shows you around the first time."),
        ],
    ),
    (
        "breed/desk",
        &[
            s("breed.game", "Your game now", "What's in your Day-Care, straight from your save."),
            s("breed.plan", "Your goal", "What you're breeding for, the best parents you own for it, and tips to get there faster."),
            s("breed.feed", "Hatch feed", "Eggs show up here as they hatch in your game."),
        ],
    ),
    (
        "breed/plan",
        &[
            s("plan.goal", "Pick a goal", "The Pokémon, nature, IVs, Ability and egg moves you want."),
            s("plan.steps", "The steps", "How to get there from the Pokémon you own, step by step. Tick each one off as you go."),
        ],
    ),
    (
        "breed/breeders",
        &[
            s("breeders.tools", "Your Breeders", "Search and filter them (Ditto, Masuda, 5+ at 31), add one by hand, or bring them in from your teams."),
            s("below:breeders.tools", "The list", "Every Pokémon you keep for breeding, with its nature and IVs."),
        ],
    ),
    ("breed/log", &[s("below:breed.tabs", "Hatch log", "Every Egg you hatch is logged by itself. A shiny Egg stays a surprise until it hatches.")]),
    // ── Damage Calculator
    (
        "calc",
        &[
            s("header", "Damage Calculator", "Platinum's own damage maths: will this move knock that Pokémon out?"),
            s("calc.left", "One side", "Any Pokémon, or one from your teams: its moves, item, nature, EVs and stat stages."),
            s("calc.right", "The other side", "The Pokémon it's up against. Swap sides flips the two."),
            s("calc.results", "The damage", "Each move's damage range and every roll, so you can see if it's a sure KO or needs luck."),
            s("calc.field", "The field", "Weather, screens and the rest, to match your battle."),
            s("calc.team", "Your whole team", "Each of your team's best hit on it, and its best hit back. Click one to calc it."),
        ],
    ),
    // ── Card Collector
    (
        "cards",
        &[
            s("header", "Card Collector", "Collect the original Pokémon cards, just by hunting."),
            s("cards.hero", "Your XP bar", "Every Pokémon you meet adds XP. Fill the bar and you get a booster pack."),
            s("tab.0", "Binder", "Every card you own, set by set."),
            s("binder.filters", "Find cards", "Search by name or number, and filter by set, rarity, type or what you own."),
            s("below:binder.filters", "Your cards", "Click a card to see it up close. Holo rares shine as you move over them."),
            s("tab.1", "Packs", "Your booster packs."),
            s("packs.boosters", "Open a pack", "Pick a booster. Tear it open along the top, then click each card to flip it over. Sets still to come count down to their release: a new one every Monday at the game's daily reset."),
            s("packs.pulls", "Latest pulls", "Your best recent cards, best on top."),
            s("packs.art", "Card art", "What card art is on this PC. Downloading sets and picking the playmat are in Settings → Card Collector."),
            s("tab.2", "Missions", "Daily and weekly missions for extra XP."),
            s("missions.list", "Missions", "They tick themselves as you play and pay out XP."),
            s("missions.xp", "How XP works", "Every way to earn XP, and what you've earned lately."),
            s("tab.3", "Trade", "Swap cards with friends."),
            s("trade.id", "Your Trainer ID", "A friend needs this to send you a card."),
            s("trade.send", "Send a card", "Pick a card and your friend's Trainer ID to make a code for them. The card leaves your binder."),
            s("trade.claim", "Claim a card", "Paste a code a friend sent you. A code works once, and only for you."),
            s("top.settings", "Its settings", "Card art downloads, the XP bar on the overlay and your playmat are in Settings (this ⚙), under Card Collector."),
        ],
    ),
    (
        "cards/binder",
        &[
            s("binder.filters", "Find cards", "Search by name or number, and filter by set, rarity, type or what you own."),
            s("below:binder.filters", "Your cards", "Click a card to see it up close. Holo rares shine as you move over them."),
        ],
    ),
    (
        "cards/packs",
        &[
            s("packs.boosters", "Open a pack", "Pick a booster. Tear it open along the top, then click each card to flip it over. Sets still to come count down to their release: a new one every Monday at the game's daily reset."),
            s("packs.pulls", "Latest pulls", "Your best recent cards, best on top."),
            s("packs.art", "Card art", "What card art is on this PC. Downloading sets and picking the playmat are in Settings → Card Collector."),
        ],
    ),
    (
        "cards/missions",
        &[
            s("missions.list", "Missions", "Daily and weekly missions. They tick themselves as you play and pay out XP."),
            s("missions.xp", "How XP works", "Every way to earn XP, and what you've earned lately."),
        ],
    ),
    (
        "cards/trade",
        &[
            s("trade.id", "Your Trainer ID", "A friend needs this to send you a card."),
            s("trade.send", "Send a card", "Pick a card and your friend's Trainer ID to make a code for them. The card leaves your binder."),
            s("trade.claim", "Claim a card", "Paste a code a friend sent you. A code works once, and only for you."),
            s("trade.spares", "Spares", "Your duplicates, ready to trade."),
            s("trade.history", "History", "Every code you've made or claimed. Lost one? Copy it again from here."),
        ],
    ),
    // ── Pokégotchi
    (
        "gotchi",
        &[
            s("header", "Pokégotchi", "One of your caught shinies, as a little pet."),
            s("gotchi.device", "Your pet", "Feed it and play with it. Hunting earns berries: one for every 10 Pokémon you meet."),
            s("gotchi.side", "Looking after it", "How Full, Happy and rested it is. Lights off lets it sleep."),
            s("gotchi.box", "Your shinies", "Any shiny you've caught can come out. Right-click one to rename it or send it back."),
            s("gotchi.journal", "Its journal", "Little moments from its day."),
            s("top.settings", "Its settings", "Its name, frame and how it shows on the overlay are in Settings (this ⚙), under Pokégotchi. The Settings button on its page goes there too."),
        ],
    ),
    // ── Shiny Bingo
    (
        "bingo",
        &[
            s("header", "Shiny Bingo", "A new bingo card every day at the game's reset. You just hunt as usual."),
            s("bingo.card", "Today's card", "Squares tick themselves from the game's log as you play."),
            s("bingo.card", "Get a line", "Five in a row, a column or a diagonal wins the day's reward."),
            s("bingo.side", "Your reward", "Pick a booster pack or battle pass XP, and collect it here."),
        ],
    ),
    // ── Rare Grass
    (
        "rare",
        &[
            s("rare.hero", "Today's rare grass", "The rarest Pokémon out on rare grass today, and where to find it."),
            s("rare.search", "Search and filter", "Find a Pokémon or a place. Show what's out today, ones seen only once, or everything."),
            s("rare.search", "MODERN POKéMON", "Set this to match your world's option: it changes which Pokémon can show up."),
            s("rare.today", "Where to go", "Every spot today. Hunt it makes it your target; Show on Map opens the spot."),
            s("rare.glossary", "Every Pokémon seen", "Everything that's ever been on rare grass, so you can plan future hunts."),
        ],
    ),
    // ── Shiny Replay
    (
        "replay",
        &[
            s("header", "Shiny Replay", "Keeps the moment you find a shiny as a video clip."),
            s("replay.player", "Your clip", "When the shiny alert goes off, the seconds before and after are saved. Press Watch to open the video."),
            s("replay.side", "Clip details", "When and where it was, and Save replay now to keep the last few seconds whenever you want."),
            s("replay.shelf", "All your clips", "Every clip you've saved. Click one to play it. Shiny clips are never deleted by themselves."),
            s("top.settings", "Its settings", "Quality, frame rate, where clips go, and recording on or off: in Settings (this ⚙), under Shiny Replay. The ⚙ on its page goes there too."),
        ],
    ),
    // ── Journal
    (
        "journal",
        &[
            s("journal.left", "Your notes", "Every note and canvas, in folders. Click one to open it."),
            s("journal.main", "Write", "Start a line with # for a heading, - for a list, or [ ] for a checkbox."),
            s("journal.main", "Link and tag", "Write [[note name]] to link to another note, and #anything to tag it."),
            s("journal.ribbon", "Quick tools", "New note, new canvas and search. A canvas is a board you drag notes and pictures onto."),
            s("top.settings", "Its settings", "Text size, line length, the right panel and the daily notes folder are in Settings (this ⚙), under Journal."),
        ],
    ),
    // ── Pokémon Boxes
    (
        "boxes",
        &[
            s("header", "Pokémon Boxes", "Your party and boxes, read straight from your save. Nothing in your game is ever changed."),
            s("boxes.party", "Your party", "The six Pokémon with you. Click one to see it."),
            s("boxes.box", "Your boxes", "Every box in your PC. Use ‹ and › to switch boxes."),
            s("boxes.side", "Details", "Its stats, IVs, nature and moves. ▲ and ▼ show which stat its nature raises and lowers."),
            s("boxes.side", "Use it elsewhere", "Send it to the Team Planner or the Damage Calculator, or log it as a shiny. It's a copy: your game isn't touched."),
        ],
    ),
    // ── Nuzlocke Tracker
    (
        "nuzlocke",
        &[
            s("header", "Nuzlocke Tracker", "Your Nuzlocke runs, read from the game's own files as you play."),
            s("nuz.worlds", "Your runs", "Each world with a Nuzlocke run. Pick the one you want to see."),
            s("nuz.trail", "Every route", "Who you got on each route, who's still in, and who fell."),
            s("nuz.run", "The run", "Your rules, level caps and current team for this run."),
        ],
    ),
    // ── Sticker Book
    (
        "stickers",
        &[
            s("header", "Sticker Book", "Your Pokémon as stickers. Make each page however you like."),
            s("stickers.tray", "Your stickers", "Your shinies, party, boxes and ribbons. Click one, or drag it onto the page. Ribbons unlock at hunting milestones."),
            s("stickers.page", "Arrange them", "Drag stickers around and stack them. Right-click one to resize, turn or peel it off."),
            s("stickers.tabs", "Pages", "There's a page for each world. + New page adds your own."),
            s("top.settings", "Its settings", "The paper and the sticker size are in Settings (this ⚙), under Sticker Book. The ⚙ on its page goes there too."),
        ],
    ),
];

fn steps(id: &str) -> &'static [Step] {
    let all = TOURS.iter().find(|(t, _)| *t == id).map(|(_, st)| *st).unwrap_or(&[]);
    // Modern Plus: short tours (research: long ones get skipped), at most
    // five steps, and its own welcome
    if kit::plus() {
        if id == "home" {
            return steps("plushome");
        }
        if matches!(id, "plushome" | "whatsnew") || COVERS.iter().any(|(t, _)| *t == id) {
            return all;
        }
        // its own version, where the look changes what a step points at
        if let Some((_, alt)) = TOURS.iter().find(|(t, _)| t.strip_prefix("plus:") == Some(id)) {
            return *alt;
        }
        return &all[..all.len().min(5)];
    }
    all
}

/// The page a tour belongs to (the Companion's three all play on the Dashboard).
fn page_of(id: &str) -> &str {
    match id {
        "whatsnew" | "plushome" => "home",
        _ => id,
    }
}

// ───────────────────────────── state ─────────────────────────────

#[derive(Default)]
pub(super) struct TourState {
    /// The tour showing.
    id: Option<&'static str>,
    step: usize,
    /// The spotlight as last drawn (where a glide starts from).
    shown: Option<Rect>,
    /// Gliding from here, since then.
    from: Option<Rect>,
    at: Option<Instant>,
    /// Landed on a page with a tour to show (it waits a moment for the page
    /// to settle).
    landed: Option<(&'static str, Instant)>,
    /// Since when the current step's part has been missing (passed over soon).
    missing: Option<Instant>,
    /// The step whose part has been scrolled into view (asked once a step).
    scrolled: Option<usize>,
    /// The page started scrolling to the part then.
    scrolling: Option<Instant>,
    /// The step whose tab has been opened (asked once a step).
    tabbed: Option<usize>,
    /// The tab open before the tour opened others (opened again at the end).
    tab_home: Option<usize>,
    /// The step whose act has been asked for, and every act done so far
    /// (undone when the tour ends).
    acted: Option<usize>,
    undos: Vec<&'static str>,
}

/// Ask the page to scroll by `dy` points (down is positive). The page area
/// glides there over the next few frames. False when it can't move (already
/// at the top).
fn request_scroll(ctx: &egui::Context, dy: f32) -> bool {
    let cur: f32 = ctx.data(|d| d.get_temp(egui::Id::new("page-scroll-y"))).unwrap_or(0.0);
    let to = (cur + dy).max(0.0);
    if (to - cur).abs() < 1.0 {
        return false;
    }
    let n = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("tour-scroll-to"), (to, n)));
    ctx.request_repaint();
    true
}

/// A part outside the page: the sidebar or the top bar.
fn chrome(key: &str) -> bool {
    key.starts_with("nav.") || key.starts_with("top.") || key == "none"
}

/// Springy ease out (close to cubic-bezier(.3, 1.2, .4, 1)).
fn spring(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.3;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let l = |x: f32, y: f32| x + (y - x) * t;
    Rect::from_min_max(pos2(l(a.min.x, b.min.x), l(a.min.y, b.min.y)), pos2(l(a.max.x, b.max.x), l(a.max.y, b.max.y)))
}

/// Dim the whole screen except a rounded hole.
fn dim(p: &egui::Painter, screen: Rect, hole: Option<Rect>, rad: f32, col: Color32) {
    let Some(h) = hole.map(|h| h.intersect(screen)).filter(|h| h.width() > 2.0 && h.height() > 2.0) else {
        p.rect_filled(screen, 0.0, col);
        return;
    };
    p.rect_filled(Rect::from_min_max(screen.min, pos2(screen.max.x, h.min.y)), 0.0, col);
    p.rect_filled(Rect::from_min_max(pos2(screen.min.x, h.max.y), screen.max), 0.0, col);
    p.rect_filled(Rect::from_min_max(pos2(screen.min.x, h.min.y), pos2(h.min.x, h.max.y)), 0.0, col);
    p.rect_filled(Rect::from_min_max(pos2(h.max.x, h.min.y), pos2(screen.max.x, h.max.y)), 0.0, col);
    // round the hole's corners: fill between each corner and its arc
    let r = rad.min(h.width() / 2.0).min(h.height() / 2.0);
    let corners: [(Pos2, Pos2, f32); 4] = [
        (h.left_top(), h.left_top() + vec2(r, r), 180.0),
        (h.right_top(), h.right_top() + vec2(-r, r), 270.0),
        (h.right_bottom(), h.right_bottom() + vec2(-r, -r), 0.0),
        (h.left_bottom(), h.left_bottom() + vec2(r, -r), 90.0),
    ];
    let mut m = egui::Mesh::default();
    for (corner, centre, a0) in corners {
        let base = m.vertices.len() as u32;
        m.colored_vertex(corner, col);
        for i in 0..=8 {
            let a = (a0 + 90.0 * i as f32 / 8.0).to_radians();
            m.colored_vertex(centre + vec2(a.cos(), a.sin()) * r, col);
        }
        for i in 0..8 {
            m.add_triangle(base, base + 1 + i, base + 2 + i);
        }
    }
    p.add(m);
}

impl App {
    /// Show apps' tours again (Settings → Appearance).
    pub(super) fn tours_reset(&mut self) {
        self.cfg.apps.tours_seen.clear();
        self.cfg.apps.tours_init = true;
        self.cfg.apps.tours_v2 = true;
        self.cfg.apps.tours_off = false;
        self.setup.tour = TourState::default();
        self.dirty = true;
    }

    /// Every frame, after the page: start an app's tour the first time its
    /// page opens, and draw the one that's running.
    pub(super) fn app_tour(&mut self, ctx: &egui::Context) {
        // where pages are drawn (saved by the page area as it draws)
        // (Counting and Overlay's tours play in the Settings window: its body
        // is the page then)
        let in_settings_page = matches!(self.setup.settings_open, Some(super::Tab::Counting | super::Tab::Overlay));
        let page_key = if in_settings_page { "tour-page-settings" } else { "tour-page" };
        let page = ctx.data(|d| d.get_temp::<Rect>(egui::Id::new(page_key))).unwrap_or_else(|| {
            let s = ctx.screen_rect();
            Rect::from_min_max(pos2(s.min.x + 240.0, s.min.y + 56.0), s.max)
        });
        // the first time this runs, apps you've already used count as toured
        if !self.cfg.apps.tours_init {
            self.cfg.apps.tours_seen = self.cfg.apps.last_opened.iter().map(|(s, _)| s.clone()).collect();
            self.cfg.apps.tours_init = true;
            self.dirty = true;
        }
        // section tours came later: anyone who'd already had an app's tour
        // (or used the app before tours existed) skips its section tours
        if !self.cfg.apps.tours_v2 {
            let mut add: Vec<String> = Vec::new();
            for (id, _) in TOURS {
                if *id != root(id) && self.cfg.apps.tours_seen.iter().any(|s| s == root(id)) {
                    add.push(id.to_string());
                }
            }
            self.cfg.apps.tours_seen.extend(add);
            self.cfg.apps.tours_v2 = true;
            self.dirty = true;
        }
        let seen = |id: &str| self.cfg.apps.tours_seen.iter().any(|s| s == id);
        let here = here_now(ctx);
        // (not over a settings window, the welcome screens or a page that isn't an app's)
        // (Shiny Hunter's Counting and Overlay live in Settings now: their
        // tours play there)
        let in_settings = matches!(self.setup.settings_open, Some(super::Tab::Counting | super::Tab::Overlay));
        let busy = (self.setup.settings_open.is_some() && !in_settings) || self.takeover_open() || find(ctx, "__window").is_some();
        match (self.setup.tour.id, here) {
            // still on its page (a section of it counts)
            (Some(t), Some(h)) if h == page_of(t) || h.strip_prefix(t).is_some_and(|r| r.starts_with('/')) || t.strip_prefix("moved:").is_some_and(|a| root(h) == a) => {}
            (Some(_), _) => {
                // left its page: the tour ends
                self.setup.tour = TourState::default();
                return;
            }
            (None, Some(h)) => {
                // a section's tour waits for its page's main tour
                let main = h.split('/').next().unwrap_or(h);
                let mut want: &'static str = if main != h && !seen(main) && !steps(main).is_empty() { TOURS.iter().find(|(t, _)| *t == main).map_or(h, |(t, _)| *t) } else { h };
                // the Dashboard: Modern Plus's welcome the first time in that
                // look; else the welcome, or (for anyone who's had it) What's new
                // an app you've had the tour of, whose settings moved since:
                // a one-step note pointing at Settings
                if want == h && seen(root(h)) {
                    if let Some((t, _)) = TOURS.iter().find(|(t, _)| t.strip_prefix("moved:") == Some(root(h))) {
                        if !seen(*t) {
                            want = *t;
                        }
                    }
                }
                if h == "home" {
                    // someone who's used the Companion before (the welcome tour
                    // is newer than most of them): What's new, not the welcome
                    let used_before = seen("home") || !self.cfg.apps.last_opened.is_empty() || self.cfg.apps.tours_seen.iter().any(|t| !matches!(t.as_str(), "home" | "plushome" | "whatsnew"));
                    want = if kit::plus() {
                        "plushome"
                    } else if used_before && !seen("whatsnew") {
                        "whatsnew"
                    } else {
                        "home"
                    };
                }
                // ("Don't show any more tours")
                if busy || self.cfg.apps.tours_off || seen(want) || steps(want).is_empty() {
                    return;
                }
                // wait a moment on the page so it's drawn and settled
                match self.setup.tour.landed {
                    Some((a, t)) if a == want => {
                        if t.elapsed() < Duration::from_millis(700) {
                            ctx.request_repaint_after(Duration::from_millis(100));
                            return;
                        }
                    }
                    _ => {
                        self.setup.tour.landed = Some((want, Instant::now()));
                        ctx.request_repaint_after(Duration::from_millis(100));
                        return;
                    }
                }
                self.setup.tour = TourState { id: Some(want), at: Some(Instant::now()), ..Default::default() };
                // seen as soon as it starts: it never comes back by itself
                self.cfg.apps.tours_seen.push(want.to_string());
                // (a welcome covers what's new; What's new is instead of the welcome)
                let mut also: Vec<&str> = match want {
                    "home" | "plushome" => vec!["whatsnew"],
                    "whatsnew" => vec!["home"],
                    _ => vec![],
                };
                if let Some((t, _)) = TOURS.iter().find(|(t, _)| t.strip_prefix("moved:") == Some(want)) {
                    also.push(*t);
                }
                if let Some((_, tabs)) = COVERS.iter().find(|(t, _)| *t == want) {
                    also.extend(tabs.iter().copied());
                }
                for t in also {
                    if !self.cfg.apps.tours_seen.iter().any(|s| s == t) {
                        self.cfg.apps.tours_seen.push(t.to_string());
                    }
                }
                self.dirty = true;
            }
            (None, None) => {
                self.setup.tour.landed = None;
                return;
            }
        }
        if busy {
            return;
        }
        let Some(tour) = self.setup.tour.id else { return };
        let list = steps(tour);
        if list.is_empty() {
            self.setup.tour = TourState::default();
            return;
        }
        let i = self.setup.tour.step.min(list.len() - 1);
        let step = &list[i];
        // a step about a tab ("tab.2"): open that tab, so you see what it's
        // about (once a step; Back opens it again)
        if let Some(k) = step.key.strip_prefix("tab.").and_then(|n| n.parse::<usize>().ok()) {
            if self.setup.tour.tabbed != Some(i) {
                self.setup.tour.tabbed = Some(i);
                if self.setup.tour.tab_home.is_none() {
                    self.setup.tour.tab_home = ctx.data(|d| d.get_temp::<usize>(egui::Id::new("tour-tab-now")));
                }
                let n = ctx.cumulative_pass_nr();
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("tour-tab"), (k, n)));
                ctx.request_repaint();
            }
        } else {
            self.setup.tour.tabbed = None;
        }
        // a step that has the page do something (open a panel…): ask once
        // a step, and remember to undo it at the end
        if !step.act.is_empty() {
            if self.setup.tour.acted != Some(i) {
                self.setup.tour.acted = Some(i);
                ask_act(ctx, step.act, false);
                if !self.setup.tour.undos.contains(&step.act) {
                    self.setup.tour.undos.push(step.act);
                }
            }
        } else {
            self.setup.tour.acted = None;
        }
        // a step on the dock: Modern Plus brings the dock up for it
        if step.key.starts_with("nav.dock") {
            let n = ctx.cumulative_pass_nr();
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("tour-dock"), n));
        }
        let last = i + 1 == list.len();
        // where the spotlight goes
        let screen = ctx.screen_rect();
        let outside = chrome(step.key);
        // the part as drawn, even if it's scrolled out of sight
        // ("below:KEY": everything under that part, down to the page's end)
        // ("none": no part at all, just the card in the middle)
        let centred = step.key == "none";
        let raw = match step.key {
            "page" => Some(page.shrink(10.0)),
            "none" => Some(Rect::from_center_size(screen.center(), vec2(60.0, 40.0))),
            k => match k.strip_prefix("below:") {
                Some(base) => find(ctx, base).map(|b| Rect::from_min_max(pos2(page.min.x, b.max.y + 6.0), pos2(page.max.x, page.max.y.max(b.max.y + 226.0)))),
                None => find(ctx, k),
            },
        };
        // a part further down (or up) the page: scroll it into view first,
        // clear of the dock, with a little room around it (a part taller
        // than the page shows from its top). Once a step.
        if !outside && self.setup.tour.scrolled != Some(i) {
            let want = match step.key.strip_prefix("below:") {
                Some(base) => find(ctx, base).map(|b| Rect::from_min_size(pos2(page.min.x, b.max.y + 6.0), vec2(page.width(), 200.0))),
                None => raw,
            };
            if let Some(w) = want {
                self.setup.tour.scrolled = Some(i);
                // (clear of the dock; in Settings there's no dock in the way)
                let dock = if in_settings_page { 0.0 } else { self.dock_pad() };
                let view = Rect::from_min_max(page.min, pos2(page.max.x, page.max.y - dock));
                let m = 28.0;
                let dy = if w.min.y < view.min.y + m || w.height() > view.height() - 2.0 * m {
                    w.min.y - (view.min.y + m)
                } else if w.max.y > view.max.y - m {
                    w.max.y - (view.max.y - m)
                } else {
                    0.0
                };
                if dy.abs() > 2.0 && request_scroll(ctx, dy) {
                    self.setup.tour.scrolling = Some(Instant::now());
                }
            }
        }
        let found = raw
            .map(|r| if outside { r.expand(4.0).intersect(screen) } else { r.expand(6.0).intersect(page.expand(6.0)) })
            .filter(|r| r.width() > 20.0 && r.height() > 16.0);
        let scrolling = self.setup.tour.scrolling.is_some_and(|t| t.elapsed() < Duration::from_millis(900));
        let target = match found {
            Some(r) => {
                self.setup.tour.missing = None;
                r
            }
            // still scrolling to it: the spotlight waits where it was
            None if scrolling => {
                ctx.request_repaint();
                self.setup.tour.shown.unwrap_or(page.shrink(10.0))
            }
            // a part that isn't on the page (an empty list, a narrow window):
            // give it a moment, then go on to the next step
            None => {
                let since = *self.setup.tour.missing.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_millis(400) {
                    let st = &mut self.setup.tour;
                    st.missing = None;
                    if last {
                        *st = TourState::default();
                    } else {
                        st.step = i + 1;
                    }
                }
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
        };
        if scrolling {
            // follow the part as the page glides
            ctx.request_repaint();
        }
        let motion = kit::motion();
        let spot = match (self.setup.tour.from, self.setup.tour.at) {
            (Some(from), Some(at)) if motion => {
                let t = at.elapsed().as_secs_f32() / 0.45;
                if t < 1.0 {
                    ctx.request_repaint();
                }
                lerp_rect(from, target, spring(t))
            }
            _ => target,
        };
        self.setup.tour.shown = Some(spot);
        let appear = self.setup.tour.at.map_or(1.0, |t| if i == 0 && motion { (t.elapsed().as_secs_f32() / 0.3).min(1.0) } else { 1.0 });
        if appear < 1.0 {
            ctx.request_repaint();
        }

        let (mut next, mut back, mut skip) = (false, false, false);
        ctx.input(|inp| {
            if inp.key_pressed(egui::Key::Escape) {
                skip = true;
            }
            if inp.key_pressed(egui::Key::Enter) || inp.key_pressed(egui::Key::ArrowRight) {
                next = true;
            }
            if inp.key_pressed(egui::Key::ArrowLeft) && i > 0 {
                back = true;
            }
        });
        // whose tour it is (the Companion's own: "Welcome")
        let (app_name, app_col) = match AppId::from_slug(root(tour.trim_start_matches("moved:"))) {
            Some(app) => {
                let (_, n, c) = super::shell::tab_meta(super::shell::app_tab(app));
                (n, c)
            }
            None => ("Welcome", kit::C_BLUE),
        };
        let never = self.cfg.apps.tours_off;
        let mut flip_never = false;
        egui::Area::new(egui::Id::new("app-tour")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            // takes every click, so the page under it waits
            let (sr, _) = ui.allocate_exact_size(screen.size(), Sense::click());
            let p = ui.painter().clone();
            dim(&p, sr, if centred { None } else { Some(spot) }, 14.0, Color32::from_rgba_unmultiplied(5, 5, 12, (174.0 * appear) as u8));
            if !centred {
                p.rect_stroke(spot.expand(2.0), 16.0, Stroke::new(2.0f32, kit::primary().gamma_multiply(appear)));
            }

            // the card
            let tw = 300.0;
            let pad = 16.0;
            let head = p.layout_no_wrap(format!("{}  ·  TOUR", app_name.to_uppercase()), crate::fonts::sans_bold_font(10.5), MUTED());
            let title = p.layout(step.title.to_string(), crate::fonts::sans_bold_font(16.0), TEXT(), tw - 2.0 * pad);
            let body = p.layout(step.text.to_string(), crate::fonts::sans_font(13.0), TEXT(), tw - 2.0 * pad);
            let btn_h = 30.0;
            // "Don't show any more tours": a checkbox row over the buttons
            let check_h = 22.0;
            let th = pad + head.size().y + 6.0 + title.size().y + 4.0 + body.size().y + 14.0 + check_h + 10.0 + btn_h + pad;
            let gap = 14.0;
            let room = screen.shrink(12.0);
            let fits = |r: Rect| room.contains_rect(r);
            let below = Rect::from_min_size(pos2((spot.center().x - tw / 2.0).min(room.max.x - tw).max(room.min.x), spot.max.y + gap), vec2(tw, th));
            let right = Rect::from_min_size(pos2(spot.max.x + gap, spot.min.y.min(room.max.y - th).max(room.min.y)), vec2(tw, th));
            let above = Rect::from_min_size(pos2(below.min.x, spot.min.y - gap - th), vec2(tw, th));
            let left = Rect::from_min_size(pos2(spot.min.x - gap - tw, right.min.y), vec2(tw, th));
            // nowhere outside it: tucked inside the spotlight's bottom right
            let inside = Rect::from_min_size(pos2((spot.max.x - tw - gap).max(room.min.x), (spot.max.y - th - gap).max(room.min.y)), vec2(tw, th));
            let card = if centred { Rect::from_center_size(screen.center(), vec2(tw, th)) } else { [below, right, above, left].into_iter().find(|r| fits(*r)).unwrap_or(inside) };
            let card = card.translate(vec2(0.0, 8.0 * (1.0 - appear)));
            p.add(kit::shadow_tier(4).as_shape(card, 14.0));
            p.rect_filled(card, 14.0, kit::CARD());
            p.rect_stroke(card, 14.0, Stroke::new(1.0f32, LINE()));
            p.rect_filled(Rect::from_min_size(card.min + vec2(pad, 0.0), vec2(40.0, 3.0)), 2.0, app_col);
            let mut y = card.min.y + pad;
            p.galley(pos2(card.min.x + pad, y), head.clone(), MUTED());
            y += head.size().y + 6.0;
            p.galley(pos2(card.min.x + pad, y), title.clone(), TEXT());
            y += title.size().y + 4.0;
            p.galley(pos2(card.min.x + pad, y), body.clone(), TEXT());
            let by = card.max.y - pad - btn_h;
            // the "no more tours" checkbox
            let cy = by - 10.0 - check_h / 2.0;
            let row = Rect::from_min_max(pos2(card.min.x + pad - 4.0, cy - check_h / 2.0), pos2(card.max.x - pad, cy + check_h / 2.0));
            let cresp = ui.interact(row, egui::Id::new("tour-never"), Sense::click());
            let chov = cresp.hovered();
            let bx = Rect::from_center_size(pos2(card.min.x + pad + 8.0, cy), vec2(16.0, 16.0));
            p.rect_filled(bx, 4.0, if never { kit::primary() } else { kit::btn_fill() });
            p.rect_stroke(bx, 4.0, Stroke::new(1.0f32, if never { kit::primary() } else if chov { TEXT() } else { kit::CONTROL() }));
            if never {
                let ink = Stroke::new(2.0f32, kit::on_primary());
                let c = bx.center();
                p.line_segment([c + vec2(-4.0, 0.2), c + vec2(-1.2, 3.0)], ink);
                p.line_segment([c + vec2(-1.2, 3.0), c + vec2(4.2, -3.2)], ink);
            }
            p.text(pos2(bx.max.x + 8.0, cy), egui::Align2::LEFT_CENTER, "Don't show any more tours", crate::fonts::sans_font(12.5), if chov || never { TEXT() } else { MUTED() });
            if cresp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("No tours start from now on, in any app. Turn them back on in Settings → Appearance.")
                .clicked()
            {
                flip_never = true;
            }
            // step dots: on the card's top line, at the right (the buttons
            // need the whole bottom row)
            let dot_y = card.min.y + pad + head.size().y / 2.0;
            let n = list.len();
            if n > 9 {
                // (too many for dots: "3 of 17")
                p.text(pos2(card.max.x - pad, dot_y), egui::Align2::RIGHT_CENTER, format!("{} of {n}", i + 1), crate::fonts::sans_bold_font(11.0), MUTED());
            } else {
                for k in 0..n {
                    let c = pos2(card.max.x - pad - 4.0 - (n - 1 - k) as f32 * 12.0, dot_y);
                    p.circle_filled(c, if k == i { 4.0 } else { 3.0 }, if k == i { kit::primary() } else { theme_line() });
                }
            }
            // buttons: Skip tour, Back, Next / Done
            let next_w = 76.0;
            let nr = Rect::from_min_size(pos2(card.max.x - pad - next_w, by), vec2(next_w, btn_h));
            let next_b = egui::Button::new(egui::RichText::new(if last { "Done" } else { "Next" }).strong().color(kit::on_primary())).fill(kit::primary()).rounding(10.0);
            if ui.put(nr, next_b).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                next = true;
            }
            let mut x = nr.min.x - 6.0;
            if i > 0 {
                let br = Rect::from_min_size(pos2(x - 60.0, by), vec2(60.0, btn_h));
                let b = egui::Button::new(egui::RichText::new("Back").color(TEXT())).fill(kit::btn_fill()).stroke(Stroke::new(1.0f32, LINE())).rounding(10.0);
                if ui.put(br, b).clicked() {
                    back = true;
                }
                x = br.min.x - 6.0;
            }
            if !last {
                let sr2 = Rect::from_min_size(pos2(x - 78.0, by), vec2(78.0, btn_h));
                let b = egui::Button::new(egui::RichText::new("Skip tour").color(MUTED())).fill(Color32::TRANSPARENT).stroke(Stroke::NONE);
                if ui.put(sr2, b).on_hover_text("End the tour (Esc). This app's other tours won't show either.").clicked() {
                    skip = true;
                }
            }
        });
        if flip_never {
            self.cfg.apps.tours_off = !never;
            self.dirty = true;
        }
        if skip {
            // skipping turns off the rest of this app's tours too
            let r = root(tour).to_string();
            for (id, _) in TOURS {
                if root(id) == r && !self.cfg.apps.tours_seen.iter().any(|s| s == id) {
                    self.cfg.apps.tours_seen.push(id.to_string());
                }
            }
            self.dirty = true;
        }
        let st = &mut self.setup.tour;
        if skip || (next && last) {
            // undo what its steps opened, and back to the tab it started on
            for a in st.undos.iter() {
                ask_act(ctx, a, true);
            }
            if let Some(k) = st.tab_home {
                let n = ctx.cumulative_pass_nr();
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("tour-tab"), (k, n)));
            }
            *st = TourState::default();
            // (a scroll it asked for that hasn't finished stops where it is)
            ctx.data_mut(|d| d.remove::<(f32, u64)>(egui::Id::new("tour-scroll-to")));
        } else if next || back {
            st.from = st.shown;
            st.at = Some(Instant::now());
            st.step = if next { i + 1 } else { i.saturating_sub(1) };
        }
    }
}

/// The step dots' resting colour.
fn theme_line() -> Color32 {
    crate::theme::blend(kit::CARD(), TEXT(), 0.25)
}

/// Every part of a page a tour can point at (for the dev build's outlines).
pub(super) fn mark_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = vec!["header", "cog"];
    for (_, list) in TOURS {
        for st in list.iter() {
            if st.key != "page" && !st.key.contains(':') && !keys.contains(&st.key) {
                keys.push(st.key);
            }
        }
    }
    keys
}

/// How many tours an app has (its main one and its sections), and how many
/// steps in all.
pub(super) fn tour_count(a: AppId) -> (usize, usize) {
    let mine: Vec<_> = TOURS.iter().filter(|(id, _)| root(id) == a.slug()).collect();
    (mine.len(), mine.iter().map(|(_, l)| l.len()).sum())
}

/// Every tour id (the Companion's own and every app's).
pub(super) fn all_tour_ids() -> Vec<&'static str> {
    TOURS.iter().map(|(id, _)| *id).filter(|id| !id.starts_with("plus:")).collect()
}

/// Every tour id an app has.
pub(super) fn tour_ids(a: AppId) -> Vec<&'static str> {
    TOURS.iter().filter(|(id, _)| root(id) == a.slug()).map(|(id, _)| *id).collect()
}
