//! The apps in the companion: what each one is, what it needs, and the packs
//! that install a few at once. The App Store, the Library and the sidebar
//! read from here; which apps are installed and open lives in the config
//! (`AppsCfg`).
//!
//! Phase 1: every app's code ships inside the one exe, so installing turns an
//! app on and uninstalling turns it off. Nothing a user saved (runs, teams,
//! breeding plans, battle history) is ever deleted by either.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum AppId {
    Shiny,
    Pokedex,
    Map,
    Info,
    Team,
    Sim,
    Breed,
    Calc,
    Cards,
    Gotchi,
    Bingo,
    Rare,
    Replay,
}

pub const ALL: [AppId; 13] = [AppId::Shiny, AppId::Pokedex, AppId::Map, AppId::Info, AppId::Team, AppId::Sim, AppId::Breed, AppId::Calc, AppId::Cards, AppId::Gotchi, AppId::Bingo, AppId::Rare, AppId::Replay];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cat {
    Hunting,
    Explore,
    Battle,
    Breed,
    Games,
}

impl Cat {
    pub const ALL: [Cat; 5] = [Cat::Hunting, Cat::Explore, Cat::Battle, Cat::Breed, Cat::Games];
    pub fn name(self) -> &'static str {
        match self {
            Cat::Hunting => "Hunting",
            Cat::Explore => "Explore",
            Cat::Battle => "Battle",
            Cat::Breed => "Breeding",
            Cat::Games => "Mini games",
        }
    }
}

/// Everything the store shows about an app.
pub struct Meta {
    #[allow(dead_code)]
    pub id: AppId,
    /// Saved in the config and used for screenshot names.
    pub slug: &'static str,
    pub name: &'static str,
    pub cat: Cat,
    /// Always installed (the companion is built around it).
    pub core: bool,
    /// Still being built (the sidebar's warning triangle).
    pub beta: bool,
    /// Puts something on top of the game while you play (the OVERLAY tag).
    pub overlay: bool,
    pub tagline: &'static str,
    pub about: &'static [&'static str],
    pub can_do: &'static [&'static str],
    pub whats_new: &'static str,
    pub updated: &'static str,
    /// What it reaches for outside the app (shown under Details).
    pub uses: &'static str,
    /// Store screenshots: file name (without .png) and caption.
    pub shots: &'static [(&'static str, &'static str)],
    /// Apps it can't work without (installed together, removed together).
    pub requires: &'static [AppId],
    /// Features inside this app that need another app, and which one.
    pub works_with: &'static [(AppId, &'static str)],
}

const SHINY: Meta = Meta {
    id: AppId::Shiny,
    slug: "shiny",
    name: "Shiny Hunter",
    cat: Cat::Hunting,
    core: true,
    beta: false,
    overlay: true,
    tagline: "Count encounters and catch shinies, with an overlay on top of the game.",
    about: &[
        "The heart of the companion. Pick a target and a method, and Shiny Hunter counts every encounter for you, from the game log or by reading your screen. When the shiny shows up it flashes before you can run.",
        "Every hunt is saved as a run with its own stats, so you can pause one, start another and pick it back up later.",
    ],
    can_do: &[
        "Count encounters from the game log or by reading the screen",
        "An overlay on top of the game, in nine looks",
        "Runs: every hunt, paused or finished, with odds and time",
        "Your Shiny Dex of everything you've caught",
    ],
    whats_new: "The shiny alert pops up as fast as it can, even at 4× speed, and hunts with several targets take turns on the bar overlay.",
    updated: "13 Nov 2026",
    uses: "Reads the game log or your screen",
    shots: &[
        ("shiny", "Your hunt and the overlay"),
        ("shiny_runs", "Runs: every hunt with its stats"),
        ("shiny_count", "Counting: game log or screen reading"),
    ],
    requires: &[],
    works_with: &[(AppId::Map, "Hunt on today's rare grass"), (AppId::Pokedex, "Your Shiny Dex in the Pokédex")],
};

const POKEDEX: Meta = Meta {
    id: AppId::Pokedex,
    slug: "pokedex",
    name: "Pokédex",
    cat: Cat::Explore,
    core: false,
    beta: false,
    overlay: false,
    tagline: "Every Pokémon in Platinum Together, and where to catch it.",
    about: &[
        "Every Pokémon in Platinum Together, straight from the Platinum Together Wiki. Filter by type, by what you've caught and by where you are in the game.",
        "Each Pokémon has its stats, evolutions, moves and where to find it, with one click to start hunting it.",
    ],
    can_do: &[
        "612 Pokémon with stats, moves and evolutions",
        "Your Shiny Dex, region by region",
        "Filter by type, caught, method and point in the game",
        "Hunt any shiny in one click",
    ],
    whats_new: "Sprites load as you scroll, so the first open is quicker.",
    updated: "13 Nov 2026",
    uses: "Downloads sprites as you browse",
    shots: &[("pokedex", "Every Pokémon, with filters"), ("pokedex_char", "A Pokémon's page"), ("shiny_dex", "The Shiny Dex view")],
    requires: &[],
    works_with: &[(AppId::Map, "Show on map"), (AppId::Team, "Add to team")],
};

const MAP: Meta = Meta {
    id: AppId::Map,
    slug: "map",
    name: "Map",
    cat: Cat::Explore,
    core: false,
    beta: false,
    overlay: false,
    tagline: "Sinnoh's map with every area, and today's rare grass spots.",
    about: &[
        "The whole Sinnoh map with every route, cave and side area. Click a place to see what lives there.",
        "Rare grass mode shows today's rare grass spots for everyone, refreshed at each daily reset.",
    ],
    can_do: &[
        "Every area: overworld, caves, ruins and side areas",
        "Today's rare grass spots, shared every day",
        "Search for a place or a Pokémon",
        "An infographic of today's spots to share",
    ],
    whats_new: "Today's rare grass spots.",
    updated: "13 Nov 2026",
    uses: "Downloads the day's rare grass spots",
    shots: &[("map", "The whole Sinnoh map"), ("map_rare", "Rare grass today")],
    requires: &[],
    works_with: &[(AppId::Pokedex, "Open a Pokémon in the Pokédex")],
};

const INFO: Meta = Meta {
    id: AppId::Info,
    slug: "info",
    name: "Game Info",
    cat: Cat::Explore,
    core: false,
    beta: false,
    overlay: false,
    tagline: "What's in the game, where to find it, and how it works.",
    about: &[
        "A searchable guide to Platinum Together: Pokémon, moves, items, places and guides, with what changed in each game patch.",
        "Tell it where you are in the story and it shows what you can catch now and what comes later.",
    ],
    can_do: &[
        "Pokémon, moves, items, places and guides in one search",
        "What changed in each game update",
        "Your adventure: what's open at your point in the game",
    ],
    whats_new: "Updated for Platinum Together 0.3.4.",
    updated: "12 Nov 2026",
    uses: "Downloads the wiki's data",
    shots: &[("info", "Look anything up"), ("info_adv", "Game updates and your adventure")],
    requires: &[],
    works_with: &[(AppId::Pokedex, "Open in the Pokédex")],
};

const TEAM: Meta = Meta {
    id: AppId::Team,
    slug: "team",
    name: "Team Planner",
    cat: Cat::Battle,
    core: false,
    beta: false,
    overlay: false,
    tagline: "Build a team, see what it covers, and plan how to get it.",
    about: &[
        "Pick up to six Pokémon, give them moves, items and EVs, and the planner works out the rest: type coverage, what hits you hardest, and how to get every part in the game.",
        "Paste a team straight from Showdown, or start from a classic.",
    ],
    can_do: &["Showdown-style sets with live stats", "Analysis: weaknesses, speed and coverage", "Checklist: where to catch, which TMs and tutors"],
    whats_new: "Your teams can be used in the Damage Calculator.",
    updated: "13 Nov 2026",
    uses: "Saves your teams on this PC",
    shots: &[("team", "Plan your first team"), ("team_sand", "Building a team")],
    requires: &[],
    works_with: &[(AppId::Calc, "Damage Calculator button"), (AppId::Sim, "Battle Sim button"), (AppId::Pokedex, "Open in the Pokédex"), (AppId::Breed, "Plan breeding")],
};

const SIM: Meta = Meta {
    id: AppId::Sim,
    slug: "sim",
    name: "Battle Sim",
    cat: Cat::Battle,
    core: false,
    beta: true,
    overlay: false,
    tagline: "Put your teams up against wild Pokémon and trainers.",
    about: &[
        "Battle your Team Planner teams against wild Pokémon by the game's real odds, or against any trainer in the game, in singles or doubles.",
        "Catch test shows every ball's odds as the HP drops.",
    ],
    can_do: &["Wild battles by each route's odds", "Trainer battles, singles or doubles", "Catch test for every ball"],
    whats_new: "Doubles labels and new battle backgrounds.",
    updated: "13 Nov 2026",
    uses: "Uses your Team Planner teams",
    shots: &[("sim", "Wild battle"), ("sim_trainer", "Trainer battle"), ("sim_battles", "Your battles")],
    requires: &[AppId::Team],
    works_with: &[],
};

const BREED: Meta = Meta {
    id: AppId::Breed,
    slug: "breed",
    name: "Breeding",
    cat: Cat::Breed,
    core: false,
    beta: true,
    overlay: false,
    tagline: "Plan an Egg: parents, items, egg moves and how many Eggs it takes.",
    about: &[
        "Pair two Pokémon at the Day Care and see what the Egg will be like, how many Eggs it takes to get the one you want, and what would make it quicker.",
        "Keep your breeders in one place and check egg moves.",
    ],
    can_do: &["The plan: parents, items and how many Eggs", "A box for your breeders", "Egg moves and a hatch log"],
    whats_new: "A hatch log.",
    updated: "10 Nov 2026",
    uses: "Saves your breeders on this PC",
    shots: &[("breed", "Plan an Egg"), ("breed_breeders", "Your breeders"), ("breed_egg", "Egg moves")],
    requires: &[],
    works_with: &[(AppId::Team, "Breeders from your teams")],
};

const CALC: Meta = Meta {
    id: AppId::Calc,
    slug: "calc",
    name: "Damage Calculator",
    cat: Cat::Battle,
    core: false,
    beta: false,
    overlay: false,
    tagline: "Platinum's own damage maths: any Pokémon against any other.",
    about: &[
        "Platinum's own damage formula: any Pokémon against any other, with every item, ability, nature, boost and field effect.",
        "Use one of your teams as your side and see every matchup at once.",
    ],
    can_do: &["Attacker, results and defender side by side", "Field: weather, screens and hazards", "Showdown text you can copy"],
    whats_new: "A new three-column layout.",
    updated: "13 Nov 2026",
    uses: "Nothing extra",
    shots: &[("calc", "The calculator"), ("calc_res", "Results with KO chances")],
    requires: &[],
    works_with: &[(AppId::Team, "Your side: one of your teams")],
};

const CARDS: Meta = Meta {
    id: AppId::Cards,
    slug: "cards",
    name: "Card Collector",
    cat: Cat::Games,
    core: false,
    beta: true,
    overlay: true,
    tagline: "Hunt, fill the bar, open booster packs and collect the original cards.",
    about: &[
        "Every Pokémon you meet earns XP. Fill the bar and you get a booster pack of the original Base Set or Base Set 2 to open.",
        "Collect all 232 cards in your binder, finish daily and weekly missions for more packs, and trade spare duplicates for XP.",
        "A slim XP bar overlay sits with your shiny overlay: drag it onto the overlay and the two snap together.",
    ],
    can_do: &["XP from your hunting, read from the game's log", "Booster packs with a reveal, holo rares one in three", "A binder with search, sets, rarity and type filters", "Daily and weekly missions", "An XP bar that snaps to the shiny overlay"],
    whats_new: "New: the first mini game.",
    updated: "14 Nov 2026",
    uses: "The game's session log (read only). Card pictures download from images.pokemontcg.io the first time they're shown.",
    shots: &[("cards", "Your binder"), ("cards_open", "Opening a pack"), ("cards_missions", "Missions")],
    requires: &[],
    works_with: &[(AppId::Shiny, "XP from your hunts, and the overlay bar")],
};

const GOTCHI: Meta = Meta {
    id: AppId::Gotchi,
    slug: "gotchi",
    name: "Pokégotchi",
    cat: Cat::Games,
    core: false,
    beta: true,
    overlay: true,
    tagline: "Adopt a shiny you caught as a pocket pet that keeps you company while you hunt.",
    about: &[
        "Pick a shiny you've caught and it moves into a little device of its own. Feed it, play with it, keep it clean and let it nap. It never runs away or faints: left alone it just gets hungry or grumpy, and a bit of care brings it back.",
        "Hunting feeds it. Every 10 Pokémon you meet earns a berry, rare grass drops treats, and it cheers when you find a shiny. It sits on your overlay while you play, so you never have to leave the game.",
    ],
    can_do: &[
        "Adopt any shiny you've caught, up to 6 in the Box",
        "Feed, play, clean, pat and lights out",
        "Berries from your hunting, read from the game's log",
        "Higher or Lower, played on the device",
        "Three frames: Sinnoh Pearl (four more colours in the battle pass), Distortion and Rare Grass",
        "On the overlay as a mini device, on the bar, or a needs strip",
    ],
    whats_new: "New: your shinies can come home with you.",
    updated: "14 Nov 2026",
    uses: "Your caught shinies, and the game's session log (read only)",
    shots: &[("gotchi", "Your Pokégotchi and its Box"), ("gotchi_play", "Higher or Lower, on the device"), ("gotchi_frames", "Three frames, and three looks on the overlay")],
    requires: &[],
    works_with: &[(AppId::Shiny, "Berries from your hunts, and the overlay")],
};

const RARE: Meta = Meta {
    id: AppId::Rare,
    slug: "rare",
    name: "Rare Grass",
    cat: Cat::Hunting,
    core: false,
    beta: true,
    overlay: false,
    tagline: "What's on rare grass today, and every Pokémon that's been on it before.",
    about: &[
        "Every Pokémon on rare grass today gets a card: where it is, how many tiles, and whether it's the first time it's been seen. The rarest come first, so the special ones are easy to spot. One click shows it on the Map or makes it your target.",
        "The glossary remembers every Pokémon that's been on rare grass and on which days, so you can see what turns up often, what's rare, and plan what to hunt next. It never guesses what comes next: it only remembers what was there.",
    ],
    can_do: &[
        "Today's rare grass, a card per Pokémon, rarest first",
        "Where each one is, with tile counts, one click to the Map",
        "A glossary of every Pokémon seen on rare grass, with the days",
        "Search and filters: out today, seen only once, all seen",
        "Five Dashboard widgets",
    ],
    whats_new: "New: today's rare grass and its history in one place.",
    updated: "14 Nov 2026",
    uses: "The shared daily rare grass list and the game's session log (read only), like the Map",
    shots: &[("rare", "Today's rare grass and the glossary")],
    requires: &[],
    works_with: &[(AppId::Map, "Show on Map"), (AppId::Shiny, "Hunt it on rare grass"), (AppId::Pokedex, "Open a Pokémon's entry")],
};

const REPLAY: Meta = Meta {
    id: AppId::Replay,
    slug: "replay",
    name: "Shiny Replay",
    cat: Cat::Hunting,
    core: false,
    beta: true,
    overlay: false,
    tagline: "The moment you found it, saved as a video clip.",
    about: &[
        "While the overlay runs, Shiny Replay keeps the last 15 seconds of the game window ready. When the shiny alert fires it records 5 seconds more and saves the whole moment as an MP4: the encounter, the sparkle, all of it. Nothing to start, nothing to remember.",
        "Watch it, show it in its folder, or copy the file straight into Discord. Save replay now (a button and a hotkey) keeps near misses and funny moments too. It only ever looks at the game window, like a screenshot: never the rest of your screen, and never the game's files or memory.",
    ],
    can_do: &[
        "Saves the 15 s before every shiny alert and the 5 s after, as an MP4",
        "Resolution up to 1080p, 30 or 60 fps, bitrate presets or your own",
        "A big player with a moving preview, and every clip on a shelf",
        "Save replay now: a button and a hotkey",
        "Copy a clip for Discord, and a Dashboard widget",
    ],
    whats_new: "New: your shinies on video, saved by themselves.",
    updated: "14 Nov 2026",
    uses: "The game window's picture while the overlay runs (never your files or the game's memory), and Windows' built-in video encoder",
    shots: &[("replay", "The newest shiny in the big player, and the recorder")],
    requires: &[AppId::Shiny],
    works_with: &[(AppId::Shiny, "Saves a clip when the shiny alert fires")],
};

const BINGO: Meta = Meta {
    id: AppId::Bingo,
    slug: "bingo",
    name: "Shiny Bingo",
    cat: Cat::Games,
    core: false,
    beta: true,
    overlay: false,
    tagline: "A new bingo card every day that fills itself in while you hunt.",
    about: &[
        "Every day at the game's reset there's a new 5×5 card, the same for everyone, so you and your friends can race on it. Squares ask for things like meeting a Bidoof, a double battle, 300 encounters in one session or an hour of hunting.",
        "You don't tick anything yourself: squares fill in from the game's log while you play. Your first line each day wins the daily reward: a booster pack or battle pass XP, you pick. Every line also gives Card Collector XP and a Pokégotchi treat, and the whole card a booster pack.",
    ],
    can_do: &[
        "A new card every day at the reset, with S H I N Y columns and a free middle",
        "Squares that tick themselves from the game's log",
        "A daily reward for your first bingo: a booster pack or battle pass XP",
        "Rewards for every line, and a booster pack for a blackout",
        "Shows which lines are closest, and your past cards",
        "Five Dashboard widgets, including the card itself",
    ],
    whats_new: "New: the third mini game.",
    updated: "14 Nov 2026",
    uses: "The game's session log (read only), your hunting time and battle pass missions",
    shots: &[("bingo", "Today's card")],
    requires: &[],
    works_with: &[(AppId::Cards, "Card Collector XP and booster packs as rewards"), (AppId::Gotchi, "Pokégotchi treats as rewards")],
};

pub fn meta(id: AppId) -> &'static Meta {
    match id {
        AppId::Shiny => &SHINY,
        AppId::Pokedex => &POKEDEX,
        AppId::Map => &MAP,
        AppId::Info => &INFO,
        AppId::Team => &TEAM,
        AppId::Sim => &SIM,
        AppId::Breed => &BREED,
        AppId::Calc => &CALC,
        AppId::Cards => &CARDS,
        AppId::Gotchi => &GOTCHI,
        AppId::Bingo => &BINGO,
        AppId::Rare => &RARE,
        AppId::Replay => &REPLAY,
    }
}

impl AppId {
    pub fn meta(self) -> &'static Meta {
        meta(self)
    }
    pub fn name(self) -> &'static str {
        meta(self).name
    }
    pub fn slug(self) -> &'static str {
        meta(self).slug
    }
    pub fn from_slug(s: &str) -> Option<AppId> {
        ALL.into_iter().find(|a| a.slug() == s)
    }
}

static LIVE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// Note which apps are installed right now (the window does this every
/// frame), so any page can ask without borrowing the config.
pub fn set_live(list: &[AppId]) {
    let mut bits = 0u32;
    for (i, a) in ALL.iter().enumerate() {
        if list.contains(a) {
            bits |= 1 << i;
        }
    }
    LIVE.store(bits, std::sync::atomic::Ordering::Relaxed);
}

/// Is this app installed (as of this frame)?
pub fn live(a: AppId) -> bool {
    let i = ALL.iter().position(|x| *x == a).unwrap_or(0);
    LIVE.load(std::sync::atomic::Ordering::Relaxed) & (1 << i) != 0
}

/// Installed apps that can't work without `id` (removing `id` would break them).
pub fn needed_by(id: AppId, installed: &[AppId]) -> Vec<AppId> {
    installed.iter().copied().filter(|a| meta(*a).requires.contains(&id)).collect()
}

/// `id` plus everything it requires that isn't installed yet (requirements first).
pub fn with_requirements(id: AppId, installed: &[AppId]) -> Vec<AppId> {
    let mut out: Vec<AppId> = meta(id).requires.iter().copied().filter(|r| !installed.contains(r)).collect();
    if !installed.contains(&id) {
        out.push(id);
    }
    out
}

/// A few apps that go together, installed in one click.
pub struct Pack {
    pub name: &'static str,
    pub icon: &'static str,
    pub blurb: &'static str,
    pub apps: &'static [AppId],
}

pub const PACKS: [Pack; 4] = [
    Pack { name: "Shiny hunter", icon: "✨", blurb: "Count, find rare grass, check the dex.", apps: &[AppId::Shiny, AppId::Map, AppId::Pokedex] },
    Pack { name: "Battler", icon: "⚔", blurb: "Build teams, test them, do the maths.", apps: &[AppId::Team, AppId::Sim, AppId::Calc] },
    Pack { name: "Explorer", icon: "🗺", blurb: "Everything about the game and where things are.", apps: &[AppId::Pokedex, AppId::Map, AppId::Info] },
    Pack { name: "Breeder", icon: "🐣", blurb: "Plan Eggs, keep breeders, check moves.", apps: &[AppId::Breed, AppId::Pokedex, AppId::Team] },
];

/// Where each app's store screenshots are kept (downloaded when a store page
/// shows them, then kept in the cache).
pub const SHOTS_URL: &str = "https://raw.githubusercontent.com/ZifuM/Shinytracker/main/store";

/// Which apps are installed and open. Saved in the config as slugs, so an app
/// this version doesn't know (from a newer version) is skipped, not fatal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppsCfg {
    /// None: saved by a version from before apps existed, so every app is
    /// installed (nobody loses a page by updating).
    pub installed: Option<Vec<String>>,
    /// Open apps, in sidebar order. Reopened as they were on the next launch.
    pub open: Vec<String>,
    /// The first-launch "everything came across" screen has been through.
    pub welcomed: u32,
    /// The Library shows a list rather than tiles.
    pub list_view: bool,
    /// "Notify me" on the Coming soon card.
    pub notify_soon: bool,
    /// When each app was last opened (slug, unix seconds).
    pub last_opened: Vec<(String, u64)>,
    /// Apps pinned for quick opening (up to `MAX_PINS`), in order.
    pub pins: Vec<String>,
    /// Where the pins show.
    pub pins_place: PinsPlace,
}

/// The most apps that can be pinned.
pub const MAX_PINS: usize = 8;

/// Where pinned apps show: a 2 × 4 grid in the sidebar, or a dock in the top bar.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PinsPlace {
    #[default]
    Sidebar,
    TopBar,
}

/// Raise to show the first-launch screen again after a future update.
pub const APPS_WELCOME_VERSION: u32 = 1;

impl AppsCfg {
    pub fn installed(&self) -> Vec<AppId> {
        match &self.installed {
            None => ALL.to_vec(),
            Some(list) => {
                let mut v: Vec<AppId> = ALL.into_iter().filter(|a| meta(*a).core || list.iter().any(|s| s == a.slug())).collect();
                v.sort();
                v
            }
        }
    }
    pub fn is_installed(&self, id: AppId) -> bool {
        meta(id).core || self.installed.as_ref().map_or(true, |l| l.iter().any(|s| s == id.slug()))
    }
    pub fn set_installed(&mut self, list: &[AppId]) {
        let mut v: Vec<String> = ALL.into_iter().filter(|a| meta(*a).core || list.contains(a)).map(|a| a.slug().to_string()).collect();
        v.dedup();
        self.installed = Some(v);
        self.open.retain(|s| AppId::from_slug(s).is_some_and(|a| list.contains(&a) || meta(a).core));
        self.pins.retain(|s| AppId::from_slug(s).is_some_and(|a| list.contains(&a) || meta(a).core));
    }
    pub fn install(&mut self, id: AppId) {
        let mut l = self.installed();
        if !l.contains(&id) {
            l.push(id);
        }
        self.set_installed(&l);
    }
    pub fn uninstall(&mut self, id: AppId) {
        if meta(id).core {
            return;
        }
        let l: Vec<AppId> = self.installed().into_iter().filter(|a| *a != id).collect();
        self.set_installed(&l);
    }
    pub fn open_list(&self) -> Vec<AppId> {
        let mut out: Vec<AppId> = Vec::new();
        for s in &self.open {
            if let Some(a) = AppId::from_slug(s) {
                if self.is_installed(a) && !out.contains(&a) {
                    out.push(a);
                }
            }
        }
        out
    }
    pub fn is_open(&self, id: AppId) -> bool {
        self.open.iter().any(|s| s == id.slug())
    }
    /// Open an app (joins the end of the sidebar). False if not installed.
    pub fn open(&mut self, id: AppId, now: u64) -> bool {
        if !self.is_installed(id) {
            return false;
        }
        if !self.is_open(id) {
            self.open.push(id.slug().to_string());
        }
        self.last_opened.retain(|(s, _)| s != id.slug());
        self.last_opened.push((id.slug().to_string(), now));
        true
    }
    pub fn close(&mut self, id: AppId) {
        self.open.retain(|s| s != id.slug());
    }
    /// Pinned apps that are installed, in order.
    pub fn pin_list(&self) -> Vec<AppId> {
        let mut out: Vec<AppId> = Vec::new();
        for s in &self.pins {
            if let Some(a) = AppId::from_slug(s) {
                if self.is_installed(a) && !out.contains(&a) {
                    out.push(a);
                }
            }
        }
        out.truncate(MAX_PINS);
        out
    }
    pub fn is_pinned(&self, id: AppId) -> bool {
        self.pin_list().contains(&id)
    }
    /// Pin an app (at the end). False if it's full, not installed or already pinned.
    pub fn pin(&mut self, id: AppId) -> bool {
        let list = self.pin_list();
        if list.len() >= MAX_PINS || list.contains(&id) || !self.is_installed(id) {
            return false;
        }
        self.pins = list.iter().map(|a| a.slug().to_string()).collect();
        self.pins.push(id.slug().to_string());
        true
    }
    pub fn unpin(&mut self, id: AppId) {
        self.pins.retain(|s| s != id.slug());
    }
    /// Move a pin one place left (-1) or right (+1).
    pub fn move_pin(&mut self, id: AppId, by: i32) {
        let mut list = self.pin_list();
        let Some(i) = list.iter().position(|a| *a == id) else { return };
        let j = (i as i32 + by).clamp(0, list.len() as i32 - 1) as usize;
        list.swap(i, j);
        self.pins = list.iter().map(|a| a.slug().to_string()).collect();
    }
    pub fn last_opened(&self, id: AppId) -> Option<u64> {
        self.last_opened.iter().find(|(s, _)| s == id.slug()).map(|x| x.1)
    }
}

/// One widget on Home: which kind, and how many columns wide (a big number
/// means the full width).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetCfg {
    pub id: u64,
    pub kind: String,
    pub span: u8,
    /// The widget's own settings (a Pokémon it shows, a type, your notes…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<String>,
    /// Its place on the Dashboard's grid of squares: [column, row, width,
    /// height]. None: from before the grid (placed in order the first time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[u8; 4]>,
}

/// The Dashboard's widgets, in order. None: the default layout, which is
/// empty (you pick your own).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HomeCfg {
    pub widgets: Option<Vec<WidgetCfg>>,
    /// The team the Dashboard's team widgets show (None: the newest).
    pub main_team: Option<u64>,
    /// Map places opened lately, newest first (zone ids).
    pub map_recent: Vec<u16>,
    /// Calculations kept from the Quick calc widget: "attacker|defender".
    pub saved_calcs: Vec<String>,
    /// Which grid the widgets' places are in: 0 the first (6 columns),
    /// 2 the finer one (12 columns, half-size squares).
    pub grid: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_saves_have_every_app() {
        let c: AppsCfg = serde_json::from_str("{}").unwrap();
        assert_eq!(c.installed(), ALL.to_vec());
        for a in ALL {
            assert!(c.is_installed(a));
        }
    }

    #[test]
    fn core_app_can_not_be_removed_and_unknown_slugs_are_skipped() {
        let mut c = AppsCfg { installed: Some(vec!["map".into(), "from-the-future".into()]), ..Default::default() };
        assert_eq!(c.installed(), vec![AppId::Shiny, AppId::Map]);
        c.uninstall(AppId::Shiny);
        assert!(c.is_installed(AppId::Shiny));
        c.open.push("from-the-future".into());
        assert!(c.open_list().is_empty());
    }

    #[test]
    fn requirements_install_first_and_dependents_are_found() {
        let have = vec![AppId::Shiny];
        assert_eq!(with_requirements(AppId::Sim, &have), vec![AppId::Team, AppId::Sim]);
        let have = vec![AppId::Shiny, AppId::Team, AppId::Sim];
        assert_eq!(needed_by(AppId::Team, &have), vec![AppId::Sim]);
        assert!(needed_by(AppId::Map, &have).is_empty());
    }

    #[test]
    fn uninstall_closes_and_open_keeps_order() {
        let mut c = AppsCfg::default();
        c.open(AppId::Calc, 5);
        c.open(AppId::Map, 6);
        c.open(AppId::Calc, 7);
        assert_eq!(c.open_list(), vec![AppId::Calc, AppId::Map]);
        assert_eq!(c.last_opened(AppId::Calc), Some(7));
        c.uninstall(AppId::Calc);
        assert_eq!(c.open_list(), vec![AppId::Map]);
        assert!(!c.open(AppId::Calc, 8));
    }

    #[test]
    fn pins_hold_eight_skip_missing_apps_and_move() {
        let mut c = AppsCfg::default();
        for a in ALL {
            c.pin(a);
        }
        assert_eq!(c.pin_list().len(), MAX_PINS);
        c.unpin(AppId::Map);
        assert!(!c.is_pinned(AppId::Map));
        c.move_pin(AppId::Shiny, 1);
        assert_eq!(c.pin_list()[1], AppId::Shiny);
        c.uninstall(AppId::Pokedex);
        assert!(!c.is_pinned(AppId::Pokedex));
        assert!(!c.pin(AppId::Pokedex));
        let old: AppsCfg = serde_json::from_str("{}").unwrap();
        assert_eq!(old.pins_place, PinsPlace::Sidebar);
    }

    #[test]
    fn every_app_has_shots_and_sane_links() {
        for a in ALL {
            let m = meta(a);
            assert_eq!(m.id, a);
            assert!(!m.shots.is_empty(), "{} has no screenshots", m.name);
            assert!(!m.requires.contains(&a) && m.works_with.iter().all(|w| w.0 != a));
            assert_eq!(AppId::from_slug(m.slug), Some(a));
        }
        for p in PACKS {
            assert!(p.apps.len() >= 2);
        }
    }
}
