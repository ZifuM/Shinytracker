//! Battle Sim engine: a Platinum (Gen 4) battle, turn by turn.
//!
//! The damage is the verified calculator (`calc::damage`); this adds what
//! happens around it: who moves first, accuracy, critical hits, every move's
//! effects (from Showdown's data, `assets/sim.json`), status, confusion,
//! weather, screens and hazards, items, the abilities that matter in battle,
//! switching, catching, and the trainer AI. A turn runs as a queue of steps,
//! so it can stop to ask who to send out (after a faint, U-turn, Baton Pass)
//! and carry on from there.

use crate::battle::{self, to_id, World};
use crate::calc::{self, Status, Weather};
use crate::sim::{self, Ai, Ball, CatchSituation, MoveEffect, Rng};
use crate::teams::Member;
use crate::wiki::Wiki;
use std::collections::VecDeque;

// ───────────────────────────── the pieces ─────────────────────────────

/// One of a Pokémon's moves, ready for battle.
#[derive(Clone, Debug, Default)]
pub struct MoveSlot {
    pub name: String,
    pub id: String,
    pub t: String,
    /// "Physical", "Special" or "Status".
    pub cat: String,
    pub bp: u32,
    /// 0 = never misses.
    pub acc: u8,
    pub pp: u8,
    pub max_pp: u8,
    pub pri: i8,
    pub fx: MoveEffect,
    /// The wiki's one-line description.
    pub desc: String,
}

impl MoveSlot {
    pub fn status(&self) -> bool {
        self.cat == "Status"
    }
    fn flag(&self, f: &str) -> bool {
        self.fx.has(f)
    }
    fn struggle() -> MoveSlot {
        MoveSlot {
            name: "Struggle".into(),
            id: "struggle".into(),
            t: "???".into(),
            cat: "Physical".into(),
            bp: 50,
            acc: 0,
            pp: 1,
            max_pp: 1,
            fx: MoveEffect { flags: "contact,protect".into(), ..Default::default() },
            desc: "Used when it has no moves left.".into(),
            ..Default::default()
        }
    }
}

/// What a move does to the one using it beyond this turn.
#[derive(Clone, Debug, Default)]
pub struct Vol {
    pub confusion: u8,
    pub flinch: bool,
    pub protect: bool,
    pub protect_n: u8,
    pub endure: bool,
    pub sub: u16,
    pub seeded: bool,
    /// Who planted the seed (it gets the HP).
    pub seeder: usize,
    /// Outrage and the like: (move, turns left).
    pub locked: Option<(usize, u8)>,
    /// Fly, Solar Beam…: the move it's charging.
    pub charging: Option<usize>,
    /// Off the field mid Fly/Dig/Dive/Bounce/Shadow Force.
    pub hidden: Option<String>,
    pub recharge: bool,
    /// Choice item lock.
    pub choice: Option<usize>,
    pub last_move: Option<usize>,
    /// Damage taken this turn and whether it was physical (Counter, Mirror Coat).
    pub hurt: Option<(u32, bool)>,
    pub taunt: u8,
    pub encore: Option<(usize, u8)>,
    pub disable: Option<(usize, u8)>,
    pub focus_energy: bool,
    pub ingrain: bool,
    pub aqua_ring: bool,
    pub cursed: bool,
    pub nightmare: bool,
    pub yawn: u8,
    pub perish: u8,
    /// Bind, Fire Spin…: (turns, by what).
    pub bound: Option<(u8, String)>,
    pub cant_escape: bool,
    pub destiny_bond: bool,
    pub flash_fire: bool,
    pub magnet_rise: u8,
    pub roost: bool,
    pub attract: bool,
    pub foresight: bool,
    pub charge: bool,
    pub torment: bool,
    pub minimized: bool,
    pub defense_curl: bool,
    pub embargo: u8,
    pub heal_block: u8,
    pub no_ability: bool,
    pub truant_rest: bool,
    pub moved: bool,
    pub turns_out: u32,
    pub stockpile: u8,
    pub transformed: bool,
}

/// A Pokémon in battle.
#[derive(Clone, Debug, Default)]
pub struct BMon {
    pub set: Member,
    pub species: String,
    pub name: String,
    pub types: Vec<String>,
    pub stats: [u16; 6],
    pub hp: u16,
    pub status: Status,
    pub sleep: u8,
    pub toxic: u8,
    /// 1–5: Atk, Def, SpA, SpD, Spe; 6 accuracy; 7 evasion.
    pub boosts: [i8; 8],
    pub moves: Vec<MoveSlot>,
    pub item: String,
    pub ability: String,
    pub weight: f32,
    pub level: u8,
    pub gender: Option<char>,
    pub shiny: bool,
    pub v: Vol,
    /// The other side has seen it.
    pub seen: bool,
    /// Damage it dealt, for the summary.
    pub dealt: u32,
    pub kos: u32,
    pub used_item: Option<String>,
}

impl BMon {
    pub fn fainted(&self) -> bool {
        self.hp == 0
    }
    pub fn max(&self) -> u16 {
        self.stats[0]
    }
    pub fn has_ab(&self, a: &str) -> bool {
        !self.v.no_ability && self.ability == a
    }
    pub fn has_item(&self, i: &str) -> bool {
        self.v.embargo == 0 && !self.has_ab("Klutz") && to_id(&self.item) == to_id(i)
    }
    pub fn is(&self, t: &str) -> bool {
        self.types.iter().any(|x| x == t) && !(t == "Flying" && self.v.roost)
    }
    pub fn grounded(&self, gravity: bool) -> bool {
        gravity || self.has_item("Iron Ball") || !(self.is("Flying") || self.has_ab("Levitate") || self.v.magnet_rise > 0)
    }
    pub fn hp_frac(&self) -> f32 {
        self.hp as f32 / self.max().max(1) as f32
    }
}

/// One side of the field.
#[derive(Clone, Debug, Default)]
pub struct BSide {
    pub name: String,
    pub mons: Vec<BMon>,
    /// Who's on the field: one slot in singles, two in doubles.
    pub slots: Vec<usize>,
    pub reflect: u8,
    pub light_screen: u8,
    pub safeguard: u8,
    pub mist: u8,
    pub tailwind: u8,
    pub lucky_chant: u8,
    pub spikes: u8,
    pub tspikes: u8,
    pub rocks: bool,
    /// (turns, HP, slot) a Wish heals.
    pub wish: Option<(u8, u16, usize)>,
    /// Future Sight / Doom Desire: (turns, damage, move, slot).
    pub future: Option<(u8, u16, String, usize)>,
    pub items: Vec<(String, u8)>,
    pub ai: Ai,
    #[allow(dead_code)]
    pub trainer: bool,
    /// The Pokémon that leaves the field heal fully (Healing Wish).
    pub healing_wish: bool,
}

impl BSide {
    /// The first one on the field.
    pub fn act(&self) -> &BMon {
        &self.mons[self.active()]
    }
    pub fn active(&self) -> usize {
        self.slots.first().copied().unwrap_or(0)
    }
    pub fn left(&self) -> usize {
        self.mons.iter().filter(|m| !m.fainted()).count()
    }
    pub fn can_switch_to(&self, i: usize) -> bool {
        !self.slots.contains(&i) && self.mons.get(i).is_some_and(|m| !m.fainted())
    }
}

/// What a side does this turn.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A move at the usual target.
    Move(usize),
    /// A move at a chosen battler (doubles).
    At(usize, usize),
    Switch(usize),
    Ball(Ball),
    /// (item, which party member).
    Item(String, usize),
    Run,
    /// Nothing to choose (locked in, recharging, charging).
    Auto,
}

/// How a battle ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
    Caught,
    Ran,
    /// The wild Pokémon left (Roar, Whirlwind, Teleport).
    Fled,
    Draw,
}

/// What happened, in order, for the screen to play back.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Turn(u32),
    Text(String),
    /// A Pokémon's HP changed.
    Hp { side: usize, hp: u16, max: u16 },
    /// A hit landed (the indicator).
    Hit { side: usize, eff: f32, crit: bool },
    Status { side: usize, status: Status },
    SwitchIn { side: usize, mon: usize, hp: u16, max: u16, status: Status },
    Out { side: usize },
    Faint { side: usize },
    Throw { at: usize, ball: Ball, shakes: u8, caught: bool },
    Weather(Weather),
    End(Outcome),
}

/// What the battle is waiting for from you.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    /// Pick who goes in at this slot (after a faint, or U-turn / Baton Pass).
    Switch { slot: usize, baton: bool },
}

#[derive(Clone, Debug)]
enum Step {
    Act { side: usize, action: Action },
    EndOfTurn,
    Replace,
}

/// Running totals for the end screen.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub damage: u32,
    pub crits: u32,
    pub misses: u32,
    pub balls: Vec<Ball>,
    pub items: u32,
}

/// A whole battle.
#[derive(Clone, Debug)]
pub struct Battle {
    pub sides: [BSide; 2],
    pub weather: Weather,
    /// 0 = until it's changed (abilities).
    pub weather_turns: u8,
    pub trick_room: u8,
    pub gravity: u8,
    pub turn: u32,
    pub world: World,
    pub wild: bool,
    pub result: Option<Outcome>,
    pub ask: Option<Ask>,
    pub catch: CatchSituation,
    pub log: Vec<Ev>,
    pub tally: [Tally; 2],
    pub rng: Rng,
    pub run_tries: u32,
    /// Moves Metronome can call (only filled when someone knows it).
    pub pool: std::sync::Arc<Vec<MoveSlot>>,
    /// This move hits more than one target (the 0.75 cut).
    spread_now: bool,
    /// Life Orb already took its toll this move.
    lo_done: bool,
    steps: VecDeque<Step>,
    /// Sides that already acted this turn (Sucker Punch, flinch…).
    order: Vec<usize>,
    last_used: Option<String>,
}

// ───────────────────────────── building ─────────────────────────────

fn wiki_move<'a>(w: &'a Wiki, name: &str) -> Option<&'a crate::wiki::Move> {
    if let Some(k) = w.move_key(name) {
        return w.move_info(&k);
    }
    let want = battle::move_alias(&to_id(name)).to_string();
    w.data.moves.values().find(|m| battle::move_alias(&to_id(&m.n)) == want)
}

/// A move as this Pokémon would use it: the game's type, power, accuracy
/// and PP (the wiki), and its effects.
pub fn move_slot(w: Option<&Wiki>, name: &str, m: &Member) -> Option<MoveSlot> {
    let fx = sim::data().effect(name).cloned().unwrap_or_default();
    let info = w.and_then(|w| wiki_move(w, name));
    let bfx = battle::data().move_fx(name);
    let (n, mut t, cat, mut bp, acc, pp, desc) = match info {
        Some(i) => (
            i.n.clone(),
            i.t.clone(),
            i.c.clone(),
            i.p.parse::<u32>().unwrap_or(0),
            i.a.parse::<u8>().unwrap_or(0),
            i.pp.parse::<u8>().unwrap_or(10).max(1),
            i.d.clone(),
        ),
        None => return None,
    };
    if n.starts_with("Hidden Power") {
        let (ty, p) = battle::hidden_power(m.ivs);
        t = ty.to_string();
        bp = p as u32;
    }
    let pri = bfx.map(|f| f.pri).unwrap_or(fx.pri);
    // PP Ups aren't in a set; use the full 8/5 like a trained Pokémon
    let max_pp = ((pp as u32 * 8) / 5).min(64) as u8;
    Some(MoveSlot {
        id: to_id(&n),
        name: n,
        t,
        cat,
        bp: if bp == 1 { 0 } else { bp },
        acc,
        pp: max_pp,
        max_pp,
        pri,
        fx,
        desc,
    })
}

impl BMon {
    /// A team member, ready to battle.
    pub fn from_member(m: &Member, world: World, w: Option<&Wiki>) -> Option<BMon> {
        let d = battle::data();
        let sp = d.species(&m.species)?;
        let stats = m.final_stats(world)?;
        let ability = if m.ability.is_empty() { sp.abilities_in(world).into_iter().next().unwrap_or_default() } else { m.ability.clone() };
        let moves: Vec<MoveSlot> = m.moves.iter().filter(|x| !x.trim().is_empty()).filter_map(|x| move_slot(w, x, m)).take(4).collect();
        Some(BMon {
            set: m.clone(),
            species: sp.name.clone(),
            name: if m.nickname.trim().is_empty() { sp.name.clone() } else { m.nickname.trim().to_string() },
            types: sp.types_in(world).to_vec(),
            stats,
            hp: stats[0],
            status: Status::None,
            moves,
            item: m.item.clone(),
            ability,
            weight: sp.weight,
            level: m.level.max(1),
            gender: m.gender,
            shiny: m.shiny,
            ..Default::default()
        })
    }
}

impl Battle {
    /// A battle between your Pokémon and theirs. `wild` for a wild encounter.
    #[allow(dead_code)]
    pub fn new(ours: Vec<BMon>, theirs: Vec<BMon>, wild: bool, world: World, seed: u64) -> Battle {
        Battle::new_format(ours, theirs, wild, world, seed, false)
    }

    /// The same, in singles or doubles.
    pub fn new_format(ours: Vec<BMon>, theirs: Vec<BMon>, wild: bool, world: World, seed: u64, doubles: bool) -> Battle {
        let mut b = Battle {
            sides: [BSide { name: "You".into(), mons: ours, ..Default::default() }, BSide { name: "Foe".into(), mons: theirs, trainer: !wild, ..Default::default() }],
            weather: Weather::None,
            weather_turns: 0,
            trick_room: 0,
            gravity: 0,
            turn: 0,
            world,
            wild,
            result: None,
            ask: None,
            catch: CatchSituation::default(),
            log: Vec::new(),
            tally: Default::default(),
            rng: Rng::new(seed),
            run_tries: 0,
            pool: Default::default(),
            spread_now: false,
            lo_done: false,
            steps: VecDeque::new(),
            order: Vec::new(),
            last_used: None,
        };
        // the first able ones lead (two each in doubles, when there are two)
        for s in 0..2 {
            let able: Vec<usize> = (0..b.sides[s].mons.len()).filter(|&i| !b.sides[s].mons[i].fainted()).collect();
            let n = if doubles { 2 } else { 1 };
            b.sides[s].slots = able.into_iter().take(n).collect();
            if b.sides[s].slots.is_empty() {
                b.sides[s].slots.push(0);
            }
        }
        b
    }

    /// Fill Metronome's pool if anyone knows it.
    pub fn with_wiki(mut self, w: &Wiki) -> Battle {
        let wants = self.sides.iter().any(|sd| sd.mons.iter().any(|m| m.moves.iter().any(|x| x.id == "metronome")));
        if wants {
            let banned = ["metronome", "struggle", "sketch", "mimic", "counter", "mirrorcoat", "protect", "detect", "endure", "destinybond", "sleeptalk", "assist", "mefirst", "trick", "thief", "covet", "focuspunch", "feint", "chatter", "transform", "snatch", "helpinghand", "followme", "switcheroo"];
            let me = Member::default();
            let bd = battle::data();
            let world = self.world;
            let mut v: Vec<MoveSlot> = sim::data()
                .moves
                .keys()
                .filter(|k| !banned.contains(&k.as_str()))
                .filter(|k| world != World::Platinum || bd.move_fx(k).is_some_and(|f| f.g != 9))
                .filter_map(|k| move_slot(Some(w), k, &me))
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            self.pool = std::sync::Arc::new(v);
        }
        self
    }

    /// Send out the leads and say hello.
    pub fn start(&mut self, intro: &str) {
        self.say(intro.to_string());
        for side in [1, 0] {
            if self.sides[side].mons.is_empty() {
                continue;
            }
            let names: Vec<String> = self.sides[side].slots.iter().map(|&i| self.sides[side].mons[i].name.clone()).collect();
            let both = names.join(" and ");
            if side == 1 && !self.wild {
                self.say(format!("{} sent out {both}!", self.sides[1].name));
            } else if side == 0 {
                self.say(format!("Go! {both}!"));
            }
            for slot in 0..self.sides[side].slots.len() {
                self.enter(slot * 2 + side);
            }
        }
        self.check_end();
    }

    // ───────────────────────────── battlers ─────────────────────────────
    // A battler is a place on the field: side = b & 1, slot = b >> 1.
    // In singles that's just 0 (you) and 1 (them).

    /// The party index of the one at battler `b`.
    pub fn mi(&self, b: usize) -> usize {
        let sd = &self.sides[b & 1];
        sd.slots.get(b >> 1).or(sd.slots.first()).copied().unwrap_or(0)
    }
    pub fn act(&self, b: usize) -> &BMon {
        &self.sides[b & 1].mons[self.mi(b)]
    }
    fn actm(&mut self, b: usize) -> &mut BMon {
        let i = self.mi(b);
        &mut self.sides[b & 1].mons[i]
    }
    pub fn doubles(&self) -> bool {
        self.sides.iter().any(|s| s.slots.len() > 1)
    }
    /// Every place on the field (fainted ones too).
    pub fn battlers(&self) -> Vec<usize> {
        let n = self.sides[0].slots.len().max(self.sides[1].slots.len());
        (0..n * 2).filter(|b| (b >> 1) < self.sides[b & 1].slots.len()).collect()
    }
    /// Someone able is there.
    pub fn present(&self, b: usize) -> bool {
        (b >> 1) < self.sides[b & 1].slots.len() && !self.sides[b & 1].mons.is_empty() && !self.act(b).fainted()
    }
    /// The able ones on the other side.
    pub fn foes(&self, b: usize) -> Vec<usize> {
        self.battlers().into_iter().filter(|x| (x & 1) != (b & 1) && self.present(*x)).collect()
    }
    pub fn ally(&self, b: usize) -> Option<usize> {
        let a = b ^ 2;
        (self.present(a)).then_some(a)
    }
    /// The usual target: the one across, or the other foe.
    pub fn foe(&self, b: usize) -> usize {
        let across = b ^ 1;
        if self.present(across) {
            return across;
        }
        self.foes(b).first().copied().unwrap_or(across)
    }
    /// Battler of a party member, if it's on the field.
    pub fn battler_of(&self, side: usize, mon: usize) -> Option<usize> {
        self.sides[side].slots.iter().position(|&m| m == mon).map(|slot| slot * 2 + side)
    }

    // ───────────────────────────── words ─────────────────────────────

    /// "Gible", "The foe's Gible", "The wild Gible".
    pub fn who(&self, s: usize) -> String {
        let n = &self.act(s).name;
        if s & 1 == 0 {
            n.clone()
        } else if self.wild {
            format!("The wild {n}")
        } else {
            format!("The foe's {n}")
        }
    }
    /// The same, mid-sentence.
    fn who_l(&self, s: usize) -> String {
        let w = self.who(s);
        if s & 1 == 0 {
            w
        } else {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
                None => w,
            }
        }
    }
    fn whos(&self, s: usize) -> String {
        format!("{}'s", self.who(s))
    }
    fn say(&mut self, t: String) {
        self.log.push(Ev::Text(t));
    }
    fn ev(&mut self, e: Ev) {
        self.log.push(e);
    }
    fn hp_ev(&mut self, s: usize) {
        let m = self.act(s);
        let (hp, max) = (m.hp, m.max());
        self.ev(Ev::Hp { side: s, hp, max });
    }

    // ───────────────────────────── the field ─────────────────────────────

    /// The weather as it works now (Air Lock / Cloud Nine switch it off).
    pub fn eff_weather(&self) -> Weather {
        if (0..2).any(|s| !self.sides[s & 1].mons.is_empty() && !self.act(s).fainted() && (self.act(s).has_ab("Air Lock") || self.act(s).has_ab("Cloud Nine"))) {
            Weather::None
        } else {
            self.weather
        }
    }

    fn cside(&self, s: usize) -> calc::Side {
        let x = &self.sides[s & 1];
        calc::Side {
            reflect: x.reflect > 0,
            light_screen: x.light_screen > 0,
            stealth_rock: x.rocks,
            spikes: x.spikes,
            helping_hand: false,
            foresight: self.present(s) && self.act(s).v.foresight,
            switching: false,
        }
    }

    fn field(&self, att: usize, def: usize) -> calc::Field {
        calc::Field { weather: self.weather, doubles: self.doubles(), gravity: self.gravity > 0, atk: self.cside(att), def: self.cside(def) }
    }

    fn cmon(&self, s: usize) -> calc::Mon {
        let m = self.act(s);
        let mut boosts = [0i8; 6];
        boosts[1..6].copy_from_slice(&m.boosts[1..6]);
        let mut types = m.types.clone();
        if m.v.roost {
            types.retain(|t| t != "Flying");
            if types.is_empty() {
                types.push("Normal".into());
            }
        }
        calc::Mon {
            name: m.species.clone(),
            types,
            level: m.level,
            stats: m.stats,
            hp: m.hp,
            boosts,
            ability: if m.v.no_ability { String::new() } else { m.ability.clone() },
            ability_on: m.v.flash_fire || (m.ability == "Slow Start" && m.v.turns_out < 5),
            item: if m.v.embargo > 0 { String::new() } else { m.item.clone() },
            status: m.status,
            gender: m.gender,
            weight: m.weight,
        }
    }

    /// Speed as it counts for turn order.
    pub fn speed(&self, s: usize) -> u32 {
        let f = calc::Field { weather: self.weather, ..Default::default() };
        let mut sp = calc::final_speed(&self.cmon(s), &f);
        if self.sides[s & 1].tailwind > 0 {
            sp *= 2;
        }
        sp
    }

    // ───────────────────────────── choosing ─────────────────────────────

    /// The moves it can pick now (indices), with why not for the rest.
    pub fn usable(&self, s: usize) -> Vec<(usize, Option<&'static str>)> {
        let m = self.act(s);
        m.moves
            .iter()
            .enumerate()
            .map(|(i, mv)| {
                let why = if mv.pp == 0 {
                    Some("No PP left")
                } else if m.v.choice.is_some_and(|c| c != i) && (m.has_item("Choice Band") || m.has_item("Choice Specs") || m.has_item("Choice Scarf")) {
                    Some("Locked in by its Choice item")
                } else if m.v.encore.is_some_and(|(e, _)| e != i) {
                    Some("It must use its encored move")
                } else if m.v.disable.is_some_and(|(d, _)| d == i) {
                    Some("Disabled")
                } else if m.v.taunt > 0 && mv.status() {
                    Some("Taunted: no status moves")
                } else if m.v.torment && m.v.last_move == Some(i) {
                    Some("Tormented: not twice in a row")
                } else {
                    None
                };
                (i, why)
            })
            .collect()
    }

    /// It can't choose this turn (recharging, charging, locked into Outrage).
    pub fn forced(&self, s: usize) -> bool {
        let v = &self.act(s).v;
        v.recharge || v.charging.is_some() || v.locked.is_some()
    }

    /// It can't switch out (and why).
    pub fn trapped(&self, s: usize) -> Option<String> {
        let m = self.act(s);
        if m.has_item("Shed Shell") {
            return None;
        }
        if m.v.ingrain {
            return Some(format!("{} is rooted in place by Ingrain.", m.name));
        }
        if let Some((_, by)) = &m.v.bound {
            return Some(format!("{} is trapped by {by}.", m.name));
        }
        if m.v.cant_escape {
            return Some(format!("{} can't escape!", m.name));
        }
        for f in self.foes(s) {
            let o = self.act(f);
            if o.has_ab("Shadow Tag") && !m.has_ab("Shadow Tag") {
                return Some(format!("{}'s Shadow Tag stops it leaving.", o.name));
            }
            if o.has_ab("Arena Trap") && m.grounded(self.gravity > 0) {
                return Some(format!("{}'s Arena Trap stops it leaving.", o.name));
            }
            if o.has_ab("Magnet Pull") && m.is("Steel") {
                return Some(format!("{}'s Magnet Pull stops it leaving.", o.name));
            }
        }
        None
    }

    /// Run one turn in singles: your action and theirs.
    #[allow(dead_code)]
    pub fn play(&mut self, mine: Action, theirs: Action) {
        self.play_all(vec![(0, mine), (1, theirs)]);
    }

    /// Run one turn: an action for each battler on the field.
    pub fn play_all(&mut self, acts: Vec<(usize, Action)>) {
        if self.result.is_some() || self.ask.is_some() {
            return;
        }
        self.turn += 1;
        self.ev(Ev::Turn(self.turn));
        self.order.clear();
        for b in self.battlers() {
            let m = self.actm(b);
            m.v.flinch = false;
            m.v.protect = false;
            m.v.endure = false;
            m.v.hurt = None;
            m.v.moved = false;
            m.v.roost = false;
        }
        let acts: Vec<(usize, Action)> = acts.into_iter().filter(|(b, _)| self.present(*b)).map(|(b, a)| (b, self.fill(b, a))).collect();
        // running, balls, items and switches go before moves
        let mut first: Vec<(usize, Action)> = Vec::new();
        let mut moves: Vec<(usize, Action)> = Vec::new();
        for (s, a) in acts {
            match a {
                Action::Move(_) | Action::At(..) | Action::Auto => moves.push((s, a)),
                _ => first.push((s, a)),
            }
        }
        first.sort_by_key(|(s, a)| (matches!(a, Action::Switch(_)) as u8, *s));
        let ordered = self.move_order(moves);
        for (s, a) in first.into_iter().chain(ordered) {
            self.steps.push_back(Step::Act { side: s, action: a });
        }
        self.steps.push_back(Step::EndOfTurn);
        self.steps.push_back(Step::Replace);
        self.run();
    }

    /// Fill in a forced action.
    fn fill(&self, s: usize, a: Action) -> Action {
        let v = &self.act(s).v;
        if let Some((i, _)) = v.locked {
            return Action::Move(i);
        }
        if let Some(i) = v.charging {
            return Action::Move(i);
        }
        if v.recharge {
            return Action::Auto;
        }
        match a {
            Action::Move(i) | Action::At(i, _) if v.encore.is_some() => {
                let e = v.encore.map(|x| x.0).unwrap_or(i);
                match a {
                    Action::At(_, t) => Action::At(e, t),
                    _ => Action::Move(e),
                }
            }
            _ => a,
        }
    }

    fn priority(&self, s: usize, a: &Action) -> i8 {
        match a {
            Action::Move(i) | Action::At(i, _) => self.act(s).moves.get(*i).map(|m| m.pri).unwrap_or(0),
            _ => 0,
        }
    }

    fn move_order(&mut self, mut moves: Vec<(usize, Action)>) -> Vec<(usize, Action)> {
        if moves.len() < 2 {
            return moves;
        }
        let key = |b: &mut Battle, s: usize, a: &Action| -> (i8, i8, i64, u32) {
            let pri = b.priority(s, a);
            let (qc, slow) = {
                let m = b.act(s);
                (m.has_item("Quick Claw"), m.has_item("Lagging Tail") || m.has_item("Full Incense") || m.has_ab("Stall"))
            };
            let bracket: i8 = if qc && b.rng.chance(0.2) {
                1
            } else if slow {
                -1
            } else {
                0
            };
            let sp = b.speed(s) as i64;
            let sp = if b.trick_room > 0 { -sp } else { sp };
            (pri, bracket, sp, b.rng.below(1000))
        };
        let mut keyed: Vec<((i8, i8, i64, u32), (usize, Action))> = moves.drain(..).map(|(s, a)| (key(self, s, &a), (s, a))).collect();
        keyed.sort_by(|a, b| b.0.cmp(&a.0));
        keyed.into_iter().map(|(_, x)| x).collect()
    }

    fn run(&mut self) {
        while self.result.is_none() && self.ask.is_none() {
            let Some(step) = self.steps.pop_front() else { break };
            match step {
                Step::Act { side, action } => self.do_action(side, action),
                Step::EndOfTurn => self.end_of_turn(),
                Step::Replace => {
                    if !self.replace() {
                        self.steps.push_front(Step::Replace);
                    }
                }
            }
            self.check_end();
        }
        if self.result.is_some() {
            self.steps.clear();
        }
    }

    /// Your pick of who goes in (when `ask` is set).
    pub fn answer(&mut self, mon: usize) {
        let Some(Ask::Switch { slot, baton }) = self.ask.clone() else { return };
        if !self.sides[0].can_switch_to(mon) {
            return;
        }
        let b = slot * 2;
        self.ask = None;
        if !self.act(b).fainted() {
            let n = self.act(b).name.clone();
            self.say(format!("{n}, come back!"));
        }
        self.switch_to(b, mon, baton);
        self.run();
    }

    /// Someone not on the field who can still fight.
    fn has_bench(&self, side: usize) -> bool {
        (0..self.sides[side].mons.len()).any(|k| self.sides[side].can_switch_to(k))
    }

    /// Fill the places of fainted Pokémon: you choose, the AI picks for them.
    fn replace(&mut self) -> bool {
        for b in self.battlers() {
            let side = b & 1;
            if !self.act(b).fainted() || !self.has_bench(side) {
                continue;
            }
            if side == 0 {
                if self.ask.is_none() {
                    self.ask = Some(Ask::Switch { slot: b >> 1, baton: false });
                }
                return false;
            }
            let i = self.ai_replacement(b);
            let n = self.sides[1].mons[i].name.clone();
            self.say(format!("{} sent out {n}!", self.sides[1].name));
            self.switch_to(b, i, false);
        }
        true
    }

    fn check_end(&mut self) {
        if self.result.is_some() {
            return;
        }
        let gone = [self.sides[0].left() == 0, self.sides[1].left() == 0];
        let r = match gone {
            [true, true] => Some(Outcome::Draw),
            [true, false] => Some(Outcome::Lost),
            [false, true] => Some(Outcome::Won),
            _ => None,
        };
        if let Some(r) = r {
            self.finish(r);
        }
    }

    fn finish(&mut self, r: Outcome) {
        if self.result.is_some() {
            return;
        }
        match r {
            Outcome::Won if self.wild => {}
            Outcome::Won => {
                let n = self.sides[1].name.clone();
                self.say(format!("You defeated {n}!"));
            }
            Outcome::Lost => self.say("You have no more Pokémon that can fight! You blacked out!".into()),
            Outcome::Draw => self.say("Both sides are out of Pokémon. It's a draw!".into()),
            _ => {}
        }
        self.result = Some(r);
        self.ask = None;
        self.ev(Ev::End(r));
    }

    // ───────────────────────────── actions ─────────────────────────────

    fn do_action(&mut self, s: usize, a: Action) {
        if self.act(s).fainted() && !matches!(a, Action::Switch(_) | Action::Item(..)) {
            return;
        }
        match a {
            Action::Run => self.try_run(),
            Action::Ball(b) => self.throw_ball(s, b),
            Action::Item(name, mon) => self.use_item(s, &name, mon),
            Action::Switch(i) => {
                if !self.sides[s & 1].can_switch_to(i) {
                    return;
                }
                if s & 1 == 0 {
                    let n = self.act(s).name.clone();
                    self.say(format!("{n}, come back!"));
                } else {
                    let n = self.sides[1].name.clone();
                    self.say(format!("{n} withdrew {}!", self.act(s).name));
                }
                self.switch_to(s, i, false);
            }
            Action::Move(i) => self.use_move(s, i, false, None),
            Action::At(i, t) => self.use_move(s, i, false, Some(t)),
            Action::Auto => {
                if self.act(s).v.recharge {
                    self.actm(s).v.recharge = false;
                    let w = self.who(s);
                    self.say(format!("{w} must recharge!"));
                }
            }
        }
        self.order.push(s);
    }

    fn try_run(&mut self) {
        if !self.wild {
            self.say("No! There's no running from a Trainer battle!".into());
            return;
        }
        if let Some(why) = self.trapped(0) {
            if !self.act(0).has_ab("Run Away") && !self.act(0).has_item("Smoke Ball") {
                self.say(why);
                return;
            }
        }
        self.run_tries += 1;
        let a = self.speed(0);
        let b = self.speed(self.foe(0));
        let ok = self.act(0).has_ab("Run Away") || self.act(0).has_item("Smoke Ball") || a >= b || {
            let f = (a * 128 / ((b / 4) % 256).max(1)) + 30 * self.run_tries;
            f > 255 || self.rng.below(256) < f
        };
        if ok {
            self.say("Got away safely!".into());
            self.finish(Outcome::Ran);
        } else {
            self.say("Can't escape!".into());
        }
    }

    fn throw_ball(&mut self, s: usize, b: Ball) {
        let you = s & 1 == 0;
        self.tally[s & 1].balls.push(b);
        if !self.wild || !you {
            self.say(format!("You threw a {}!", b.name()));
            self.say("The Trainer blocked the Ball! Don't be a thief!".into());
            return;
        }
        let foes = self.foes(s);
        if foes.len() > 1 {
            self.say("It's no good! It's impossible to aim when there are two Pokémon!".into());
            return;
        }
        let at = self.foe(s);
        self.say(format!("You threw a {}!", b.name()));
        let m = self.act(at).clone();
        if m.v.hidden.is_some() {
            self.say("But it missed!".into());
            return;
        }
        let rate = battle::data().species(&m.species).and_then(|sp| sim::data().catch_rate(sp.num)).unwrap_or(45);
        let mut sit = self.catch;
        sit.level = m.level;
        sit.turn = self.turn.saturating_sub(1);
        sit.water_or_bug = m.types.iter().any(|t| t == "Water" || t == "Bug");
        let mult = sim::ball_mult(b, &sit);
        let (shakes, caught) = sim::throw(rate, m.max() as u32, m.hp as u32, mult, sim::status_mult(m.status), &mut self.rng);
        self.ev(Ev::Throw { at, ball: b, shakes, caught });
        if caught {
            self.say(format!("Gotcha! {} was caught!", m.name));
            self.finish(Outcome::Caught);
        } else {
            self.say(
                match shakes {
                    0 => "Oh, no! The Pokémon broke free!",
                    1 => "Aww! It appeared to be caught!",
                    2 => "Aargh! Almost had it!",
                    _ => "Shoot! It was so close, too!",
                }
                .into(),
            );
        }
    }

    fn use_item(&mut self, s: usize, name: &str, mon: usize) {
        if s & 1 == 1 {
            if let Some(x) = self.sides[1].items.iter_mut().find(|x| x.0 == name && x.1 > 0) {
                x.1 -= 1;
            } else {
                return;
            }
        }
        self.tally[s & 1].items += 1;
        let user = if s & 1 == 0 { "You".to_string() } else { self.sides[1].name.clone() };
        let mon = if self.sides[s & 1].mons.get(mon).is_some() { mon } else { self.mi(s) };
        let on_field = self.battler_of(s & 1, mon);
        let is_active = on_field.is_some();
        let fb = on_field.unwrap_or(s);
        let target_name = self.sides[s & 1].mons[mon].name.clone();
        self.say(format!("{user} used {name}{}!", if s & 1 == 0 { format!(" on {target_name}") } else { String::new() }));
        if let Some(i) = match name {
            "X Attack" => Some(1),
            "X Defend" => Some(2),
            "X Special" => Some(3),
            "X Sp. Def" => Some(4),
            "X Speed" => Some(5),
            "X Accuracy" => Some(6),
            _ => None,
        } {
            if is_active {
                self.boost(fb, i, 1, true);
            } else {
                self.say("But it had no effect.".into());
            }
            return;
        }
        let m = &mut self.sides[s & 1].mons[mon];
        let max = m.max();
        let mut words: Vec<String> = Vec::new();
        let revive = matches!(name, "Revive" | "Max Revive");
        if m.fainted() != revive {
            let _ = m;
            self.say("But it had no effect.".into());
            return;
        }
        match name {
            "Potion" | "Super Potion" | "Hyper Potion" | "Max Potion" | "Full Restore" | "Fresh Water" | "Soda Pop" | "Lemonade" | "Moomoo Milk" => {
                let heal = match name {
                    "Potion" => 20,
                    "Fresh Water" => 50,
                    "Super Potion" | "Soda Pop" => 50,
                    "Lemonade" => 80,
                    "Moomoo Milk" => 100,
                    "Hyper Potion" => 200,
                    _ => max,
                };
                let before = m.hp;
                m.hp = (m.hp + heal).min(max);
                if m.hp > before {
                    words.push(format!("{target_name}'s HP was restored by {} point{}.", m.hp - before, if m.hp - before == 1 { "" } else { "s" }));
                }
                if name == "Full Restore" && (m.status != Status::None || m.v.confusion > 0) {
                    m.status = Status::None;
                    m.v.confusion = 0;
                    words.push(format!("{target_name} was cured of its status problem."));
                }
            }
            "Full Heal" => {
                m.status = Status::None;
                m.v.confusion = 0;
                words.push(format!("{target_name} was cured of its status problem."));
            }
            "Revive" | "Max Revive" => {
                m.hp = if name == "Revive" { (max / 2).max(1) } else { max };
                m.status = Status::None;
                words.push(format!("{target_name} was revived!"));
            }
            "Dire Hit" => {
                m.v.focus_energy = true;
                words.push(format!("{target_name} is getting pumped!"));
            }
            "Guard Spec." => {
                self.sides[s & 1].mist = 5;
                words.push("Your team became shrouded in mist!".into());
            }
            _ => words.push("But it had no effect.".into()),
        }
        let st = self.sides[s & 1].mons[mon].status;
        for w in words {
            self.say(w);
        }
        if is_active {
            self.hp_ev(fb);
            self.ev(Ev::Status { side: fb, status: st });
        }
    }

    // ───────────────────────────── switching ─────────────────────────────

    fn switch_to(&mut self, s: usize, i: usize, baton: bool) {
        let old = self.mi(s);
        let carried = if baton { Some((self.sides[s & 1].mons[old].boosts, self.sides[s & 1].mons[old].v.clone())) } else { None };
        // leaving
        {
            let m = &mut self.sides[s & 1].mons[old];
            if m.has_ab("Natural Cure") && !m.fainted() {
                m.status = Status::None;
            }
            m.toxic = 0;
            m.boosts = [0; 8];
            if m.v.transformed {
                // back to itself
                if let Some(b) = BMon::from_member(&m.set, self.world, None) {
                    m.types = b.types;
                    m.stats = [m.stats[0], b.stats[1], b.stats[2], b.stats[3], b.stats[4], b.stats[5]];
                    m.ability = b.ability;
                }
            }
            m.v = Vol::default();
        }
        // foes held by this one are let go
        for f in self.foes(s) {
            let m = self.actm(f);
            m.v.bound = None;
            m.v.cant_escape = false;
            m.v.attract = false;
        }
        self.ev(Ev::Out { side: s });
        let slot = (s >> 1).min(self.sides[s & 1].slots.len().saturating_sub(1));
        self.sides[s & 1].slots[slot] = i;
        if let Some((b, v)) = carried {
            let m = self.actm(s);
            m.boosts = b;
            m.v.confusion = v.confusion;
            m.v.sub = v.sub;
            m.v.seeded = v.seeded;
            m.v.focus_energy = v.focus_energy;
            m.v.ingrain = v.ingrain;
            m.v.aqua_ring = v.aqua_ring;
            m.v.cursed = v.cursed;
            m.v.perish = v.perish;
            m.v.magnet_rise = v.magnet_rise;
            m.v.embargo = v.embargo;
            m.v.heal_block = v.heal_block;
            m.v.cant_escape = v.cant_escape;
            m.v.no_ability = v.no_ability;
        }
        if s & 1 == 0 {
            let n = self.act(s).name.clone();
            self.say(format!("Go! {n}!"));
        }
        self.enter(s);
    }

    /// Coming in: hazards, then its ability.
    fn enter(&mut self, s: usize) {
        {
            let m = self.actm(s);
            m.seen = true;
            m.v.turns_out = 0;
        }
        let i = self.mi(s);
        let (hp, max, status) = (self.act(s).hp, self.act(s).max(), self.act(s).status);
        self.ev(Ev::SwitchIn { side: s, mon: i, hp, max, status });
        if self.sides[s & 1].healing_wish {
            self.sides[s & 1].healing_wish = false;
            let m = self.actm(s);
            m.hp = m.max();
            m.status = Status::None;
            let n = self.who(s);
            self.say(format!("The healing wish came true for {n}!"));
            self.hp_ev(s);
            self.ev(Ev::Status { side: s, status: Status::None });
        }
        // hazards
        let m = self.act(s).clone();
        if !m.has_ab("Magic Guard") {
            if self.sides[s & 1].rocks {
                let e = calc::eff("Rock", &m.types, self.world);
                let d = ((e * m.max() as f32 / 8.0).floor() as u16).max(1);
                let w = self.who(s);
                let _ = w;
                let wl = self.who_l(s);
                self.say(format!("Pointed stones dug into {wl}!"));
                self.damage_raw(s, d);
                if self.act(s).fainted() {
                    return;
                }
            }
            let ground = m.grounded(self.gravity > 0);
            if ground && self.sides[s & 1].spikes > 0 {
                let d = m.max() / [8, 6, 4][(self.sides[s & 1].spikes as usize - 1).min(2)];
                let w = self.who(s);
                self.say(format!("{w} is hurt by spikes!"));
                self.damage_raw(s, d.max(1));
                if self.act(s).fainted() {
                    return;
                }
            }
            if ground && self.sides[s & 1].tspikes > 0 {
                if m.is("Poison") {
                    self.sides[s & 1].tspikes = 0;
                    let w = self.who(s);
                    self.say(format!("{w} absorbed the poison spikes!"));
                } else {
                    let st = if self.sides[s & 1].tspikes >= 2 { Status::Toxic } else { Status::Poison };
                    self.set_status(s, st, None, false);
                }
            }
        }
        self.on_enter_ability(s);
    }

    fn on_enter_ability(&mut self, s: usize) {
        let m = self.act(s).clone();
        if self.sides[(s & 1) ^ 1].mons.is_empty() {
            return;
        }
        let o = self.foe(s);
        let ab = if m.v.no_ability { "" } else { m.ability.as_str() };
        match ab {
            "Intimidate" => {
                for o in self.foes(s) {
                    let w = self.whos(s);
                    self.say(format!("{w} Intimidate cuts {}'s Attack!", self.who_l(o)));
                    let om = self.act(o);
                    if om.has_ab("Clear Body") || om.has_ab("White Smoke") || om.has_ab("Hyper Cutter") || om.v.sub > 0 {
                        let n = self.whos(o);
                        self.say(format!("{n} Attack was not lowered!"));
                    } else {
                        let n = if om.has_ab("Simple") { -2 } else { -1 };
                        self.boost(o, 1, n, false);
                    }
                }
            }
            "Drizzle" => self.set_weather(Weather::Rain, 0, Some(s)),
            "Drought" => self.set_weather(Weather::Sun, 0, Some(s)),
            "Sand Stream" => self.set_weather(Weather::Sand, 0, Some(s)),
            "Snow Warning" => self.set_weather(Weather::Hail, 0, Some(s)),
            "Pressure" => {
                let w = self.who(s);
                self.say(format!("{w} is exerting its Pressure!"));
            }
            "Mold Breaker" => {
                let w = self.who(s);
                self.say(format!("{w} breaks the mold!"));
            }
            "Download" if !self.act(o).fainted() => {
                let om = self.act(o);
                let i = if om.stats[2] < om.stats[4] { 1 } else { 3 };
                self.boost(s, i, 1, true);
            }
            "Trace" if !self.act(o).fainted() => {
                let a = self.act(o).ability.clone();
                if !["Trace", "Multitype", "Forecast", "Wonder Guard"].contains(&a.as_str()) && !a.is_empty() {
                    self.actm(s).ability = a.clone();
                    let w = self.who(s);
                    self.say(format!("{w} traced {}'s {a}!", self.who_l(o)));
                    self.on_enter_ability(s);
                }
            }
            "Anticipation" | "Forewarn" | "Frisk" => {
                if ab == "Frisk" && !self.act(o).item.is_empty() {
                    let w = self.who(s);
                    let it = self.act(o).item.clone();
                    self.say(format!("{w} frisked its target and found one {it}!"));
                }
            }
            "Slow Start" => {
                let w = self.who(s);
                self.say(format!("{w} can't get it going!"));
            }
            _ => {}
        }
    }

    fn set_weather(&mut self, w: Weather, turns: u8, by: Option<usize>) {
        if self.weather == w {
            if by.is_none() {
                self.say("But it failed!".into());
            }
            return;
        }
        self.weather = w;
        self.weather_turns = turns;
        self.ev(Ev::Weather(w));
        self.say(
            match w {
                Weather::Rain => "It started to rain!",
                Weather::Sun => "The sunlight turned harsh!",
                Weather::Sand => "A sandstorm kicked up!",
                Weather::Hail => "It started to hail!",
                Weather::None => "The weather cleared up.",
            }
            .into(),
        );
    }

    // ───────────────────────────── HP ─────────────────────────────

    /// Lose HP (no substitute, no Endure): hazards, weather, recoil.
    fn damage_raw(&mut self, s: usize, d: u16) {
        let m = self.actm(s);
        m.hp = m.hp.saturating_sub(d);
        self.hp_ev(s);
        if self.act(s).fainted() {
            self.faint(s);
        }
    }

    /// Lose HP without fainting yet (recoil, Life Orb, Rough Skin during a
    /// move: the faint comes once the move is done, after the target's).
    fn lose_hp(&mut self, s: usize, d: u16) {
        let m = self.actm(s);
        m.hp = m.hp.saturating_sub(d);
        self.hp_ev(s);
    }

    fn heal(&mut self, s: usize, n: u16) -> u16 {
        let m = self.actm(s);
        if m.fainted() || m.v.heal_block > 0 {
            return 0;
        }
        let before = m.hp;
        m.hp = (m.hp + n).min(m.max());
        let got = m.hp - before;
        if got > 0 {
            self.hp_ev(s);
        }
        got
    }

    fn faint(&mut self, s: usize) {
        let w = self.who(s);
        self.ev(Ev::Faint { side: s });
        self.say(format!("{w} fainted!"));
        let m = self.actm(s);
        m.status = Status::None;
        m.v = Vol::default();
        if self.wild && s & 1 == 1 && self.sides[1].left() == 0 {
            self.finish(Outcome::Won);
        }
    }

    // ───────────────────────────── stats ─────────────────────────────

    /// Raise or lower a stat. `own` = it did it to itself (Mist, Clear Body don't stop it).
    fn boost(&mut self, s: usize, i: usize, n: i8, own: bool) -> bool {
        let names = ["", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed", "accuracy", "evasiveness"];
        let m = self.act(s).clone();
        if m.fainted() {
            return false;
        }
        let w = self.whos(s);
        let mut n = n;
        if m.has_ab("Simple") {
            n = (n * 2).clamp(-6, 6);
        }
        if n < 0 && !own {
            if self.sides[s & 1].mist > 0 {
                let n = self.who(s);
                self.say(format!("{n} is protected by the mist!"));
                return false;
            }
            let blocked = m.has_ab("Clear Body")
                || m.has_ab("White Smoke")
                || (i == 1 && m.has_ab("Hyper Cutter"))
                || (i == 6 && m.has_ab("Keen Eye"));
            if blocked {
                self.say(format!("{w} {} prevents its {} from dropping!", m.ability, names[i]));
                return false;
            }
        }
        let cur = m.boosts[i];
        let new = (cur + n).clamp(-6, 6);
        if new == cur {
            self.say(format!("{w} {} won't go {}!", names[i], if n > 0 { "higher" } else { "lower" }));
            return false;
        }
        self.actm(s).boosts[i] = new;
        let how = match n {
            1 => "rose!",
            2 => "sharply rose!",
            x if x >= 3 => "rose drastically!",
            -1 => "fell!",
            -2 => "harshly fell!",
            _ => "severely fell!",
        };
        self.say(format!("{w} {} {how}", names[i]));
        true
    }

    fn boosts_from(&mut self, s: usize, b: &sim::Boosts, own: bool) -> bool {
        let mut any = false;
        let order = ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"];
        for (k, key) in order.iter().enumerate() {
            if let Some(n) = b.get(*key) {
                any |= self.boost(s, k + 1, *n, own);
            }
        }
        any
    }

    // ───────────────────────────── status ─────────────────────────────

    /// Try to give a status. `by` = the side causing it (Safeguard, Synchronize).
    fn set_status(&mut self, s: usize, st: Status, by: Option<usize>, loud: bool) -> bool {
        let m = self.act(s).clone();
        if m.fainted() {
            return false;
        }
        let w = self.who(s);
        if m.status != Status::None {
            if loud {
                let what = match m.status {
                    Status::Burn => "already burned",
                    Status::Paralysis => "already paralyzed",
                    Status::Poison | Status::Toxic => "already poisoned",
                    Status::Sleep => "already asleep",
                    Status::Freeze => "already frozen",
                    Status::None => "",
                };
                self.say(format!("{w} is {what}!"));
            }
            return false;
        }
        if by.is_some_and(|b| b != s) && self.sides[s & 1].safeguard > 0 {
            if loud {
                self.say(format!("{w} is protected by Safeguard!"));
            }
            return false;
        }
        let weather = self.eff_weather();
        let immune = match st {
            Status::Burn => m.is("Fire") || m.has_ab("Water Veil"),
            Status::Paralysis => m.has_ab("Limber"),
            Status::Poison | Status::Toxic => m.is("Poison") || m.is("Steel") || m.has_ab("Immunity"),
            Status::Sleep => m.has_ab("Insomnia") || m.has_ab("Vital Spirit"),
            Status::Freeze => m.is("Ice") || m.has_ab("Magma Armor") || weather == Weather::Sun,
            Status::None => false,
        } || (m.has_ab("Leaf Guard") && weather == Weather::Sun);
        if immune {
            if loud {
                self.say(format!("It doesn't affect {}...", self.who_l(s)));
            }
            return false;
        }
        {
            let mm = self.actm(s);
            mm.status = st;
            if st == Status::Sleep {
                mm.sleep = 0;
            }
            if st == Status::Toxic {
                mm.toxic = 0;
            }
        }
        if st == Status::Sleep {
            let t = self.rng.below(4) as u8 + 1;
            self.actm(s).sleep = t;
        }
        self.ev(Ev::Status { side: s, status: st });
        self.say(
            match st {
                Status::Burn => format!("{w} was burned!"),
                Status::Paralysis => format!("{w} is paralyzed! It may be unable to move!"),
                Status::Poison => format!("{w} was poisoned!"),
                Status::Toxic => format!("{w} was badly poisoned!"),
                Status::Sleep => format!("{w} fell asleep!"),
                Status::Freeze => format!("{w} was frozen solid!"),
                Status::None => String::new(),
            },
        );
        // Synchronize passes it back
        if let Some(b) = by {
            if b != s && m.has_ab("Synchronize") && matches!(st, Status::Burn | Status::Paralysis | Status::Poison | Status::Toxic) {
                let back = if st == Status::Toxic { Status::Poison } else { st };
                self.set_status(b, back, Some(s), false);
            }
        }
        self.berry_check(s);
        true
    }

    fn confuse(&mut self, s: usize, loud: bool) -> bool {
        let m = self.act(s);
        let w = self.who(s);
        if m.fainted() {
            return false;
        }
        if m.v.confusion > 0 {
            if loud {
                self.say(format!("{w} is already confused!"));
            }
            return false;
        }
        if m.has_ab("Own Tempo") {
            if loud {
                self.say(format!("{w}'s Own Tempo prevents confusion!"));
            }
            return false;
        }
        let t = self.rng.below(4) as u8 + 1;
        self.actm(s).v.confusion = t;
        self.say(format!("{w} became confused!"));
        self.berry_check(s);
        true
    }

    /// Status-curing and pinch berries.
    fn berry_check(&mut self, s: usize) {
        let m = self.act(s).clone();
        if m.fainted() || m.v.embargo > 0 || m.has_ab("Klutz") {
            return;
        }
        let id = to_id(&m.item);
        let w = self.who(s);
        let cure = |st: Status| match id.as_str() {
            "lumberry" => st != Status::None || m.v.confusion > 0,
            "cheriberry" => st == Status::Paralysis,
            "chestoberry" => st == Status::Sleep,
            "pechaberry" => matches!(st, Status::Poison | Status::Toxic),
            "rawstberry" => st == Status::Burn,
            "aspearberry" => st == Status::Freeze,
            "persimberry" => m.v.confusion > 0,
            _ => false,
        };
        if cure(m.status) {
            let item = m.item.clone();
            let mm = self.actm(s);
            if id == "persimberry" || id == "lumberry" {
                mm.v.confusion = 0;
            }
            if id != "persimberry" {
                mm.status = Status::None;
            }
            mm.used_item = Some(item.clone());
            mm.item.clear();
            self.say(format!("{w}'s {item} cured its status!"));
            self.ev(Ev::Status { side: s, status: Status::None });
            return;
        }
        let quarter = m.hp as u32 * 4 <= m.max() as u32 || (m.has_ab("Gluttony") && m.hp as u32 * 2 <= m.max() as u32);
        let half = m.hp as u32 * 2 <= m.max() as u32;
        let heal = match id.as_str() {
            "sitrusberry" if half => Some(m.max() / 4),
            "oranberry" if half => Some(10),
            "figyberry" | "wikiberry" | "magoberry" | "aguavberry" | "iapapaberry" if quarter => Some(m.max() / 8),
            _ => None,
        };
        if let Some(h) = heal {
            if m.hp > 0 {
                let item = m.item.clone();
                self.actm(s).item.clear();
                self.actm(s).used_item = Some(item.clone());
                self.say(format!("{w} restored its health using its {item}!"));
                self.heal(s, h.max(1));
            }
            return;
        }
        let pinch = match id.as_str() {
            "liechiberry" => Some(1),
            "ganlonberry" => Some(2),
            "petayaberry" => Some(3),
            "apicotberry" => Some(4),
            "salacberry" => Some(5),
            _ => None,
        };
        if let (Some(i), true) = (pinch, quarter && m.hp > 0) {
            let item = m.item.clone();
            self.actm(s).item.clear();
            self.actm(s).used_item = Some(item.clone());
            self.say(format!("{w} used its {item}!"));
            self.boost(s, i, 1, true);
        }
    }

    // ───────────────────────────── moves ─────────────────────────────

    /// Can it move at all this turn? (sleep, freeze, paralysis, confusion…)
    fn can_move(&mut self, s: usize, mv: &MoveSlot) -> bool {
        let w = self.who(s);
        let m = self.act(s).clone();
        if m.v.flinch {
            self.say(format!("{w} flinched!"));
            if m.has_ab("Steadfast") {
                self.boost(s, 5, 1, true);
            }
            return false;
        }
        if m.has_ab("Truant") && m.v.truant_rest {
            self.actm(s).v.truant_rest = false;
            self.say(format!("{w} is loafing around!"));
            return false;
        }
        match m.status {
            Status::Sleep => {
                if m.sleep > 0 {
                    let dec = if m.has_ab("Early Bird") { 2 } else { 1 };
                    self.actm(s).sleep = m.sleep.saturating_sub(dec);
                    self.say(format!("{w} is fast asleep."));
                    if !(mv.fx.sleep_usable) {
                        return false;
                    }
                } else {
                    self.actm(s).status = Status::None;
                    self.actm(s).v.nightmare = false;
                    self.say(format!("{w} woke up!"));
                    self.ev(Ev::Status { side: s, status: Status::None });
                }
            }
            Status::Freeze => {
                if mv.flag("defrost") || self.rng.chance(0.2) {
                    self.actm(s).status = Status::None;
                    self.say(format!("{w} thawed out!"));
                    self.ev(Ev::Status { side: s, status: Status::None });
                } else {
                    self.say(format!("{w} is frozen solid!"));
                    return false;
                }
            }
            _ => {}
        }
        if m.v.confusion > 0 {
            let c = m.v.confusion - 1;
            self.actm(s).v.confusion = c;
            if c == 0 {
                self.say(format!("{w} snapped out of confusion!"));
            } else {
                self.say(format!("{w} is confused!"));
                if self.rng.chance(0.5) {
                    self.say("It hurt itself in its confusion!".into());
                    let mon = self.cmon(s);
                    let mv = calc::Move { name: "Confusion hit".into(), t: "???".into(), cat: "Physical".into(), bp: 40, hits: 1, ..Default::default() };
                    let mut mon2 = mon.clone();
                    mon2.ability.clear();
                    mon2.item.clear();
                    let f = calc::Field { weather: Weather::None, ..Default::default() };
                    let h = calc::damage(&mon, &mon2, &mv, &f, self.world);
                    let d = h.rolls[self.rng.below(16) as usize] as u16;
                    self.ev(Ev::Hit { side: s, eff: 1.0, crit: false });
                    self.damage_raw(s, d.max(1));
                    return false;
                }
            }
        }
        if m.v.attract {
            let o = self.act(self.foe(s)).name.clone();
            self.say(format!("{w} is in love with {o}!"));
            if self.rng.chance(0.5) {
                self.say(format!("{w} is immobilized by love!"));
                return false;
            }
        }
        if m.status == Status::Paralysis && self.rng.chance(0.25) {
            self.say(format!("{w} is paralyzed! It can't move!"));
            return false;
        }
        true
    }

    fn use_move(&mut self, s: usize, i: usize, called: bool, want: Option<usize>) {
        // the one it's aimed at (moves that hit several work it out below)
        let o = match want {
            Some(t) if self.present(t) && t != s => t,
            _ => self.foe(s),
        };
        // Struggle when nothing has PP
        let me = self.act(s).clone();
        let mut mv = if me.moves.iter().all(|m| m.pp == 0) || me.moves.is_empty() { MoveSlot::struggle() } else { me.moves.get(i).cloned().unwrap_or_else(MoveSlot::struggle) };
        let struggle = mv.id == "struggle";
        let charging_now = me.v.charging == Some(i);
        let locked_now = me.v.locked.is_some_and(|(l, _)| l == i);
        if !called && !self.can_move(s, &mv) {
            // a lock or charge is broken off
            let mm = self.actm(s);
            mm.v.locked = None;
            mm.v.charging = None;
            mm.v.hidden = None;
            if mm.has_ab("Truant") {
                mm.v.truant_rest = true;
            }
            self.actm(s).v.moved = true;
            return;
        }
        if self.act(s).fainted() {
            return;
        }
        if self.act(s).has_ab("Truant") {
            self.actm(s).v.truant_rest = true;
        }
        // Taunt, Disable stop it here too (encore picked for it)
        if !called && !struggle && !charging_now && !locked_now {
            if self.act(s).v.taunt > 0 && mv.status() {
                let w = self.who(s);
                self.say(format!("{w} can't use {} after the taunt!", mv.name));
                self.actm(s).v.moved = true;
                return;
            }
            if self.act(s).v.disable.is_some_and(|(d, _)| d == i) {
                let w = self.who(s);
                self.say(format!("{w}'s {} is disabled!", mv.name));
                self.actm(s).v.moved = true;
                return;
            }
        }
        let w = self.who(s);
        // PP (charging turn 2 and locked turns don't use more)
        if !struggle && !called && !charging_now && !locked_now {
            let cost = if self.foes(s).iter().any(|f| self.act(*f).has_ab("Pressure")) { 2 } else { 1 };
            let mm = self.actm(s);
            if let Some(m) = mm.moves.get_mut(i) {
                m.pp = m.pp.saturating_sub(cost);
            }
            mm.v.last_move = Some(i);
            if mm.has_item("Choice Band") || mm.has_item("Choice Specs") || mm.has_item("Choice Scarf") {
                if mm.v.choice.is_none() {
                    mm.v.choice = Some(i);
                }
            }
        }
        self.actm(s).v.moved = true;
        if struggle && !called {
            self.say(format!("{w} has no moves left!"));
        }
        // charge moves: the first turn
        if mv.flag("charge") && !charging_now && !called {
            let skip = (mv.id == "solarbeam" && self.eff_weather() == Weather::Sun) || self.act(s).has_item("Power Herb");
            let (line, hide) = match mv.id.as_str() {
                "fly" => (format!("{w} flew up high!"), Some("fly")),
                "bounce" => (format!("{w} sprang up!"), Some("fly")),
                "dig" => (format!("{w} burrowed its way under the ground!"), Some("dig")),
                "dive" => (format!("{w} hid underwater!"), Some("dive")),
                "shadowforce" => (format!("{w} vanished instantly!"), Some("shadow")),
                "solarbeam" => (format!("{w} took in sunlight!"), None),
                "skullbash" => (format!("{w} tucked in its head!"), None),
                "skyattack" => (format!("{w} became cloaked in a harsh light!"), None),
                "razorwind" => (format!("{w} whipped up a whirlwind!"), None),
                _ => (format!("{w} is charging up!"), None),
            };
            self.say(format!("{w} used {}!", mv.name));
            self.say(line);
            if mv.id == "skullbash" {
                self.boost(s, 2, 1, true);
            }
            if skip {
                if self.act(s).has_item("Power Herb") && !(mv.id == "solarbeam" && self.eff_weather() == Weather::Sun) {
                    let it = self.act(s).item.clone();
                    self.actm(s).item.clear();
                    self.actm(s).used_item = Some(it);
                    self.say(format!("{w} became fully charged due to its Power Herb!"));
                }
            } else {
                let mm = self.actm(s);
                mm.v.charging = Some(i);
                mm.v.hidden = hide.map(String::from);
                return;
            }
        } else {
            self.say(format!("{w} used {}!", mv.name));
        }
        {
            let mm = self.actm(s);
            mm.v.charging = None;
            mm.v.hidden = None;
        }
        self.last_used = Some(mv.id.clone());
        // moves that become other moves
        match mv.id.as_str() {
            "metronome" => {
                match self.random_move() {
                    Some(p) => {
                        self.say(format!("Waggling a finger let it use {}!", p.name));
                        self.actm(s).moves.push(MoveSlot { pp: 1, ..p });
                        let idx = self.act(s).moves.len() - 1;
                        self.use_move(s, idx, true, want);
                        if !self.act(s).moves.is_empty() && self.act(s).moves.len() > idx {
                            self.actm(s).moves.truncate(idx);
                        }
                    }
                    None => self.say("But it failed!".into()),
                }
                return;
            }
            "sleeptalk" => {
                if self.act(s).status != Status::Sleep {
                    self.say("But it failed!".into());
                    return;
                }
                let opts: Vec<usize> = self.act(s).moves.iter().enumerate().filter(|(_, m)| !matches!(m.id.as_str(), "sleeptalk" | "focuspunch" | "bide" | "uproar" | "chatter") && !m.flag("charge")).map(|(k, _)| k).collect();
                if opts.is_empty() {
                    self.say("But it failed!".into());
                    return;
                }
                let k = opts[self.rng.below(opts.len() as u32) as usize];
                self.use_move(s, k, true, want);
                return;
            }
            "mirrormove" | "copycat" => {
                let last = if mv.id == "mirrormove" { self.act(o).v.last_move.and_then(|k| self.act(o).moves.get(k).cloned()) } else { self.last_used.clone().and_then(|id| self.find_move(&id)) };
                match last {
                    Some(m2) if m2.id != "mirrormove" && m2.id != "copycat" => {
                        self.actm(s).moves.push(MoveSlot { pp: 1, ..m2 });
                        let idx = self.act(s).moves.len() - 1;
                        self.use_move(s, idx, true, want);
                        self.actm(s).moves.pop();
                    }
                    _ => self.say("But it failed!".into()),
                }
                return;
            }
            _ => {}
        }
        // Return / Frustration / Magnitude / Present / Trump Card power
        let happy = self.act(s).set.happiness as u32;
        match mv.id.as_str() {
            "return" => mv.bp = (happy * 2 / 5).max(1),
            "frustration" => mv.bp = ((255 - happy.min(255)) * 2 / 5).max(1),
            "magnitude" => {
                let r = self.rng.below(100);
                let (n, bp) = match r {
                    0..=4 => (4, 10),
                    5..=14 => (5, 30),
                    15..=34 => (6, 50),
                    35..=64 => (7, 70),
                    65..=84 => (8, 90),
                    85..=94 => (9, 110),
                    _ => (10, 150),
                };
                mv.bp = bp;
                self.say(format!("Magnitude {n}!"));
            }
            "trumpcard" => {
                let left = self.act(s).moves.get(i).map(|m| m.pp).unwrap_or(0);
                mv.bp = match left {
                    0 => 200,
                    1 => 80,
                    2 => 60,
                    3 => 50,
                    _ => 40,
                };
            }
            "present" => {
                let r = self.rng.below(10);
                if r < 2 {
                    let h = self.act(o).max() / 4;
                    let got = self.heal(o, h);
                    let n = self.who(o);
                    if got > 0 {
                        self.say(format!("{n} regained health!"));
                    } else {
                        self.say(format!("{n} can't receive the gift!"));
                    }
                    return;
                }
                mv.bp = [40, 40, 40, 40, 80, 80, 80, 120][(r - 2) as usize];
            }
            "fling" => {
                if self.act(s).item.is_empty() {
                    self.say("But it failed!".into());
                    return;
                }
                mv.bp = 30;
            }
            "naturalgift" | "spitup" | "beatup" | "bide" | "snatch" | "magiccoat" | "followme" | "helpinghand" | "imprison" | "grudge" | "conversion2" | "assist" | "mefirst" | "sketch" | "mimic" | "recycle" => {
                self.say("But it failed!".into());
                return;
            }
            _ => {}
        }
        // who it reaches
        let targets = self.targets_for(s, &mv, want);
        let target_self = targets.first() == Some(&s);
        let targets_foe = !target_self;
        if targets.is_empty() || (targets_foe && !targets.iter().any(|t| self.present(*t))) {
            self.say("But there was no target...".into());
            self.end_lock(s, i);
            return;
        }
        let o = if targets_foe { targets.iter().copied().find(|t| self.present(*t)).unwrap_or(o) } else { o };
        let om = self.act(o).clone();
        // Sucker Punch / Focus Punch / Fake Out conditions
        match mv.id.as_str() {
            "suckerpunch" => {
                let foe_pick_damaging = !self.order.contains(&o) && self.pending_move(o).is_some_and(|m| !m.status());
                if !foe_pick_damaging {
                    self.say("But it failed!".into());
                    return;
                }
            }
            "focuspunch" => {
                if self.act(s).v.hurt.is_some() {
                    self.say(format!("{w} lost its focus and couldn't move!"));
                    return;
                }
            }
            "fakeout" => {
                if self.act(s).v.turns_out > 0 {
                    self.say("But it failed!".into());
                    return;
                }
            }
            "snore" if self.act(s).status != Status::Sleep => {
                self.say("But it failed!".into());
                return;
            }
            "dreameater" if om.status != Status::Sleep => {
                self.say(format!("{} wasn't affected!", self.who(o)));
                return;
            }
            "lastresort" => {
                self.say("But it failed!".into());
                return;
            }
            _ => {}
        }
        let multi = targets.len() > 1;
        self.spread_now = multi && !mv.status();
        self.lo_done = false;
        let mut hit_any = false;
        for t in targets {
            if self.result.is_some() || self.act(s).fainted() {
                break;
            }
            let on_foe = t != s;
            if on_foe && !self.present(t) {
                continue;
            }
            let tm = self.act(t).clone();
            // Protect
            if on_foe && tm.v.protect && mv.flag("protect") {
                let n = self.who(t);
                self.say(format!("{n} protected itself!"));
                if mv.fx.crash && !multi {
                    self.crash(s, t, &mv);
                }
                continue;
            }
            // semi-invulnerable targets
            if on_foe {
                if let Some(h) = &tm.v.hidden {
                    let hits = match h.as_str() {
                        "fly" => matches!(mv.id.as_str(), "thunder" | "gust" | "twister" | "skyuppercut"),
                        "dig" => matches!(mv.id.as_str(), "earthquake" | "magnitude"),
                        "dive" => matches!(mv.id.as_str(), "surf" | "whirlpool"),
                        _ => false,
                    };
                    if !hits && !self.act(s).has_ab("No Guard") && !tm.has_ab("No Guard") {
                        self.say(format!("{} avoided the attack!", self.who(t)));
                        continue;
                    }
                }
            }
            // accuracy
            if on_foe && !self.hits(s, t, &mv) {
                self.tally[s & 1].misses += 1;
                if multi {
                    self.say(format!("{} avoided the attack!", self.who(t)));
                } else {
                    self.say(format!("{w}'s attack missed!"));
                }
                if mv.fx.crash && !multi {
                    self.crash(s, t, &mv);
                }
                continue;
            }
            hit_any = true;
            if mv.status() {
                self.status_move(s, i, &mv, t);
            } else {
                self.damaging_move(s, i, &mv, t);
            }
        }
        // Life Orb after a move that hit several
        if self.spread_now && self.lo_done && mv.fx.selfdestruct.is_none() && !self.act(s).fainted() && self.act(s).has_item("Life Orb") && !self.act(s).has_ab("Magic Guard") {
            let d = (self.act(s).max() / 10).max(1);
            self.say(format!("{w} lost some of its HP!"));
            self.damage_raw(s, d);
        }
        self.spread_now = false;
        if !hit_any {
            self.end_lock(s, i);
        }
        // Explosion takes the user down whatever happened
        if mv.fx.selfdestruct.is_some() && !self.act(s).fainted() && self.result.is_none() {
            self.actm(s).hp = 0;
            self.hp_ev(s);
            self.faint(s);
        }
        // a lock (Outrage) counts down (not for a move another move called)
        if self.result.is_none() && !called && hit_any {
            self.tick_lock(s, i, &mv);
        }
    }

    /// Who a move hits: one chosen foe, both foes, everyone near it, its
    /// ally, a random foe, or itself (and the field).
    fn targets_for(&mut self, s: usize, mv: &MoveSlot, want: Option<usize>) -> Vec<usize> {
        let foes = self.foes(s);
        match mv.fx.target.as_str() {
            "self" | "allySide" | "all" | "allyTeam" | "adjacentAllyOrSelf" => vec![s],
            _ if mv.fx.side_condition.is_some() && mv.fx.target != "foeSide" => vec![s],
            "foeSide" => vec![self.foe(s)],
            "allAdjacentFoes" => {
                if foes.is_empty() {
                    vec![self.foe(s)]
                } else {
                    foes
                }
            }
            "allAdjacent" => {
                let mut v = foes;
                if let Some(a) = self.ally(s) {
                    v.push(a);
                }
                if v.is_empty() {
                    v.push(self.foe(s));
                }
                v
            }
            "adjacentAlly" => self.ally(s).into_iter().collect(),
            "randomNormal" => {
                if foes.is_empty() {
                    vec![self.foe(s)]
                } else {
                    let k = self.rng.below(foes.len() as u32) as usize;
                    vec![foes[k]]
                }
            }
            _ => match want {
                Some(t) if t != s && self.present(t) => vec![t],
                _ => vec![self.foe(s)],
            },
        }
    }

    fn pending_move(&self, s: usize) -> Option<&MoveSlot> {
        self.steps.iter().find_map(|st| match st {
            Step::Act { side, action: Action::Move(k) | Action::At(k, _) } if *side == s => self.act(s).moves.get(*k),
            _ => None,
        })
    }

    fn find_move(&self, id: &str) -> Option<MoveSlot> {
        for s in 0..2 {
            for m in &self.sides[s & 1].mons {
                if let Some(x) = m.moves.iter().find(|x| x.id == id) {
                    return Some(x.clone());
                }
            }
        }
        None
    }

    /// A random move for Metronome, from every move either side knows the
    /// effects of.
    fn random_move(&mut self) -> Option<MoveSlot> {
        if self.pool.is_empty() {
            return None;
        }
        let k = self.rng.below(self.pool.len() as u32) as usize;
        Some(self.pool[k].clone())
    }

    fn end_lock(&mut self, s: usize, _i: usize) {
        let mm = self.actm(s);
        if mm.v.locked.is_some() {
            mm.v.locked = None;
        }
    }

    fn tick_lock(&mut self, s: usize, i: usize, mv: &MoveSlot) {
        let lock = mv.fx.on_self.as_ref().and_then(|x| x.volatile.as_deref()) == Some("lockedmove") || matches!(mv.id.as_str(), "outrage" | "thrash" | "petaldance");
        if !lock {
            return;
        }
        let cur = self.act(s).v.locked;
        match cur {
            None => {
                let t = self.rng.below(2) as u8 + 1;
                self.actm(s).v.locked = Some((i, t));
            }
            Some((_, 0)) | Some((_, 1)) => {
                self.actm(s).v.locked = None;
                let w = self.who(s);
                self.say(format!("{w} became confused due to fatigue!"));
                if self.act(s).v.confusion == 0 && !self.act(s).has_ab("Own Tempo") {
                    let t = self.rng.below(4) as u8 + 1;
                    self.actm(s).v.confusion = t;
                }
            }
            Some((k, t)) => self.actm(s).v.locked = Some((k, t - 1)),
        }
    }

    fn crash(&mut self, s: usize, o: usize, mv: &MoveSlot) {
        let (h, _) = self.calc_hit(s, o, mv, false);
        let d = ((h.rolls[8] / 2) as u16).min(self.act(o).max() / 2).max(1);
        let w = self.who(s);
        self.say(format!("{w} kept going and crashed!"));
        if !self.act(s).has_ab("Magic Guard") {
            self.damage_raw(s, d);
        }
    }

    /// Accuracy check.
    fn hits(&mut self, s: usize, o: usize, mv: &MoveSlot) -> bool {
        let a = self.act(s).clone();
        let d = self.act(o).clone();
        if a.has_ab("No Guard") || d.has_ab("No Guard") {
            return true;
        }
        let w = self.eff_weather();
        if mv.fx.ohko.is_some() {
            if d.level > a.level || d.has_ab("Sturdy") {
                return false;
            }
            let acc = 30 + (a.level - d.level) as u32;
            return self.rng.below(100) < acc;
        }
        let mut acc = mv.acc as f64;
        if mv.id == "thunder" {
            match w {
                Weather::Rain => return true,
                Weather::Sun => acc = 50.0,
                _ => {}
            }
        }
        if mv.id == "blizzard" && w == Weather::Hail {
            return true;
        }
        if acc == 0.0 || acc > 100.0 {
            return true;
        }
        let ev = if d.v.foresight { d.boosts[7].min(0) } else { d.boosts[7] };
        let stage = (a.boosts[6] - ev).clamp(-6, 6) as i32;
        let m = if stage >= 0 { (3 + stage) as f64 / 3.0 } else { 3.0 / (3 - stage) as f64 };
        acc *= m;
        if a.has_ab("Compound Eyes") {
            acc *= 1.3;
        }
        if a.has_ab("Hustle") && mv.cat == "Physical" {
            acc *= 0.8;
        }
        if a.has_item("Wide Lens") {
            acc *= 1.1;
        }
        if a.has_item("Zoom Lens") && self.order.contains(&o) {
            acc *= 1.2;
        }
        if d.has_item("BrightPowder") || d.has_item("Bright Powder") || d.has_item("Lax Incense") {
            acc *= 0.9;
        }
        if (d.has_ab("Sand Veil") && w == Weather::Sand) || (d.has_ab("Snow Cloak") && w == Weather::Hail) {
            acc *= 0.8;
        }
        if d.has_ab("Tangled Feet") && d.v.confusion > 0 {
            acc *= 0.5;
        }
        if self.gravity > 0 {
            acc = acc * 5.0 / 3.0;
        }
        (self.rng.below(100) as f64) < acc
    }

    /// The calculator's damage for this move now (16 rolls).
    fn calc_hit(&self, s: usize, o: usize, mv: &MoveSlot, crit: bool) -> (calc::Hit, calc::Move) {
        let a = self.cmon(s);
        let d = self.cmon(o);
        let mut f = self.field(s, o);
        f.def.foresight = self.act(o).v.foresight;
        let am = self.act(s);
        let mut bp = mv.bp;
        let mut t = mv.t.clone();
        let om = self.act(o);
        // powers that depend on this turn
        match mv.id.as_str() {
            "avalanche" | "revenge" if am.v.hurt.is_some() => bp *= 2,
            "assurance" if om.v.hurt.is_some() => bp *= 2,
            "payback" if self.order.contains(&o) => bp *= 2,
            "earthquake" | "magnitude" if om.v.hidden.as_deref() == Some("dig") => bp *= 2,
            "surf" | "whirlpool" if om.v.hidden.as_deref() == Some("dive") => bp *= 2,
            "gust" | "twister" if om.v.hidden.as_deref() == Some("fly") => bp *= 2,
            "pursuit" => {}
            "rollout" | "iceball" if am.v.defense_curl => bp *= 2,
            "stomp" | "needlearm" | "astonish" | "extrasensory" if om.v.minimized => bp *= 2,
            "solarbeam" if matches!(self.eff_weather(), Weather::Rain | Weather::Sand | Weather::Hail) => {}
            "struggle" => t = "???".into(),
            _ => {}
        }
        if am.v.charge && t == "Electric" {
            bp *= 2;
        }
        let cm = calc::Move {
            name: mv.name.clone(),
            t,
            cat: mv.cat.clone(),
            bp,
            hits: 1,
            punch: mv.flag("punch"),
            sound: mv.flag("sound"),
            recoil: mv.fx.recoil.is_some(),
            crash: mv.fx.crash,
            spread: self.spread_now,
            crit,
        };
        (calc::damage(&a, &d, &cm, &f, self.world), cm)
    }

    fn crit_roll(&mut self, s: usize, o: usize, mv: &MoveSlot) -> bool {
        let a = self.act(s);
        let d = self.act(o);
        if d.has_ab("Battle Armor") || d.has_ab("Shell Armor") || self.sides[o & 1].lucky_chant > 0 {
            return false;
        }
        if mv.fx.will_crit {
            return true;
        }
        let mut st = mv.fx.crit_ratio.unwrap_or(1).saturating_sub(1) as u32;
        if a.v.focus_energy {
            st += 2;
        }
        if a.has_ab("Super Luck") {
            st += 1;
        }
        if a.has_item("Scope Lens") || a.has_item("Razor Claw") {
            st += 1;
        }
        if (a.has_item("Lucky Punch") && a.species == "Chansey") || (a.has_item("Stick") && a.species == "Farfetch'd") {
            st += 2;
        }
        let chance = match st {
            0 => 1.0 / 16.0,
            1 => 1.0 / 8.0,
            2 => 1.0 / 4.0,
            3 => 1.0 / 3.0,
            _ => 1.0 / 2.0,
        };
        self.rng.chance(chance)
    }

    fn damaging_move(&mut self, s: usize, i: usize, mv: &MoveSlot, o: usize) {
        let w = self.who(s);
        let om = self.act(o).clone();
        let am = self.act(s).clone();
        // special damage
        let fixed: Option<u16> = match mv.id.as_str() {
            "counter" | "mirrorcoat" | "metalburst" => {
                let want_phys = mv.id == "counter";
                match am.v.hurt {
                    Some((d, phys)) if mv.id == "metalburst" || phys == want_phys => Some(((d as f32) * if mv.id == "metalburst" { 1.5 } else { 2.0 }) as u16),
                    _ => {
                        self.say("But it failed!".into());
                        return;
                    }
                }
            }
            "endeavor" => {
                if om.hp <= am.hp {
                    self.say("But it failed!".into());
                    return;
                }
                Some(om.hp - am.hp)
            }
            "psywave" => Some(((am.level as u32 * (self.rng.below(101) + 50)) / 100).max(1) as u16),
            _ if mv.fx.ohko.is_some() => Some(om.hp),
            _ => None,
        };
        // type immunity still applies to fixed damage
        if fixed.is_some() && mv.t != "???" && calc::eff(&mv.t, &om.types, self.world) == 0.0 && !(om.v.foresight && om.is("Ghost")) {
            self.say(format!("It doesn't affect {}...", self.who_l(o)));
            return;
        }
        let (probe, _) = self.calc_hit(s, o, mv, false);
        if let Some(why) = &probe.none {
            if why == "No effect" || om.ability == *why {
                if why != "No effect" && !why.is_empty() {
                    self.absorb(o, mv, why);
                } else {
                    self.say(format!("It doesn't affect {}...", self.who_l(o)));
                }
                self.end_lock(s, i);
                if mv.fx.crash {
                    self.crash(s, o, mv);
                }
                return;
            }
        }
        if fixed.is_none() && probe.none.is_some() {
            self.say("But it failed!".into());
            return;
        }
        // Wonder Guard style absorbs are reported by calc as the ability
        let (lo, hi) = mv.fx.hits();
        let mut hits = if hi > lo {
            if am.has_ab("Skill Link") {
                hi
            } else if lo == 2 && hi == 5 {
                [2, 2, 2, 3, 3, 3, 4, 5][self.rng.below(8) as usize]
            } else {
                lo + self.rng.below((hi - lo + 1) as u32) as u8
            }
        } else {
            lo.max(1)
        };
        if mv.id == "triplekick" {
            hits = 3;
        }
        let mut total: u32 = 0;
        let mut landed = 0u8;
        let mut eff = 1.0f32;
        let mut sub_hit = false;
        let mut any_crit = false;
        let mut said_eff = false;
        for k in 0..hits {
            if self.act(o).fainted() || self.act(s).fainted() {
                break;
            }
            if mv.id == "triplekick" && k > 0 && !self.hits(s, o, mv) {
                break;
            }
            let crit = fixed.is_none() && self.crit_roll(s, o, mv);
            let (mut h, _) = self.calc_hit(s, o, mv, crit);
            if mv.id == "triplekick" {
                let (h2, _) = self.calc_hit(s, o, &MoveSlot { bp: 10 * (k as u32 + 1), ..mv.clone() }, crit);
                h = h2;
            }
            eff = h.eff;
            let mut d: u32 = match fixed {
                Some(f) => f as u32,
                None => h.rolls[self.rng.below(16) as usize],
            };
            if crit {
                any_crit = true;
                self.tally[s & 1].crits += 1;
            }
            // a resist berry was used up
            if fixed.is_none() && calc::berry_resist_type(&self.act(o).item).is_some_and(|t| t == mv.t) && (eff > 1.0 || mv.t == "Normal") && self.act(o).v.embargo == 0 {
                let it = self.act(o).item.clone();
                self.actm(o).item.clear();
                self.actm(o).used_item = Some(it.clone());
                let n = self.who_l(o);
                self.say(format!("The {it} weakened the damage to {n}!"));
            }
            // the substitute takes it
            let sub = self.act(o).v.sub;
            if sub > 0 && mv.id != "struggle" {
                sub_hit = true;
                let take = (d as u16).min(sub);
                self.actm(o).v.sub = sub - take;
                self.ev(Ev::Hit { side: o, eff, crit });
                if crit {
                    self.say("A critical hit!".into());
                }
                let n = self.who(o);
                if self.act(o).v.sub == 0 {
                    self.say(format!("The substitute took damage for {}!", n.to_lowercase()));
                    self.say(format!("{n}'s substitute faded!"));
                } else {
                    self.say(format!("The substitute took damage for {}!", n.to_lowercase()));
                }
                total += take as u32;
                landed += 1;
                continue;
            }
            let tgt = self.act(o).clone();
            // survive on 1 HP
            let full = tgt.hp == tgt.max();
            if d >= tgt.hp as u32 {
                let keep = if mv.id == "falseswipe" {
                    Some(String::new())
                } else if tgt.v.endure {
                    Some(format!("{} endured the hit!", self.who(o)))
                } else if full && tgt.has_item("Focus Sash") {
                    let it = tgt.item.clone();
                    self.actm(o).item.clear();
                    self.actm(o).used_item = Some(it);
                    Some(format!("{} hung on using its Focus Sash!", self.who(o)))
                } else if tgt.has_item("Focus Band") && self.rng.chance(0.1) {
                    Some(format!("{} hung on using its Focus Band!", self.who(o)))
                } else {
                    None
                };
                if let Some(line) = keep {
                    d = tgt.hp.saturating_sub(1) as u32;
                    if !line.is_empty() {
                        self.ev(Ev::Hit { side: o, eff, crit });
                        self.actm(o).hp = 1;
                        self.hp_ev(o);
                        if crit {
                            self.say("A critical hit!".into());
                        }
                        self.say(line);
                        total += d;
                        landed += 1;
                        self.actm(o).v.hurt = Some((d, mv.cat == "Physical"));
                        continue;
                    }
                }
            }
            let d16 = (d.min(tgt.hp as u32)) as u16;
            self.ev(Ev::Hit { side: o, eff, crit });
            {
                let t = self.actm(o);
                t.hp -= d16;
                t.v.hurt = Some((d16 as u32, mv.cat == "Physical"));
            }
            self.hp_ev(o);
            total += d16 as u32;
            landed += 1;
            if crit {
                self.say("A critical hit!".into());
            }
            let ko = self.act(o).fainted();
            if ko {
                // the effectiveness line comes before the faint
                if hits == 1 && fixed.is_none() {
                    if eff > 1.0 {
                        self.say(self.eff_line(true, o));
                    } else if eff < 1.0 && eff > 0.0 {
                        self.say(self.eff_line(false, o));
                    }
                    said_eff = true;
                }
                // it faints once the move is done (after recoil, stat drops…)
            }
            // contact abilities, per hit
            if mv.flag("contact") {
                self.contact(s, o);
            }
            if ko {
                break;
            }
            // Rage, Anger Point
            if crit && self.act(o).has_ab("Anger Point") && !self.act(o).fainted() {
                self.actm(o).boosts[1] = 6;
                let n = self.whos(o);
                self.say(format!("{n} Anger Point maxed its Attack!"));
            }
        }
        if landed == 0 {
            return;
        }
        if !said_eff {
            if eff > 1.0 && fixed.is_none() {
                self.say(self.eff_line(true, o));
            } else if eff < 1.0 && eff > 0.0 && fixed.is_none() {
                self.say(self.eff_line(false, o));
            }
        }
        let _ = any_crit;
        if hits > 1 {
            self.say(format!("Hit {landed} time{}!", if landed == 1 { "" } else { "s" }));
        }
        if mv.fx.ohko.is_some() && self.act(o).fainted() {
            self.say("It's a one-hit KO!".into());
        }
        self.tally[s & 1].damage += total;
        self.actm(s).dealt += total;
        let fainted_now = self.act(o).fainted();
        if fainted_now {
            self.actm(s).kos += 1;
        }
        // drain
        if let Some([n, dn]) = mv.fx.drain {
            if total > 0 {
                let mut h = ((total * n as u32) / dn.max(1) as u32).max(1) as u16;
                if self.act(s).has_item("Big Root") {
                    h = h * 13 / 10;
                }
                if self.act(o).has_ab("Liquid Ooze") {
                    let n2 = self.who(s);
                    self.say(format!("{n2} sucked up the liquid ooze!"));
                    self.lose_hp(s, h);
                } else {
                    let got = self.heal(s, h);
                    if got > 0 {
                        let n2 = self.who_l(o);
                        self.say(format!("{n2} had its energy drained!"));
                    }
                }
            }
        }
        // recoil
        if !self.act(s).fainted() && !self.act(s).has_ab("Magic Guard") {
            let rec: u16 = if mv.id == "struggle" {
                (self.act(s).max() / 4).max(1)
            } else if let Some([n, dn]) = mv.fx.recoil {
                if self.act(s).has_ab("Rock Head") {
                    0
                } else {
                    ((total * n as u32) / dn.max(1) as u32).max(1) as u16
                }
            } else {
                0
            };
            if rec > 0 {
                self.say(format!("{w} is hit with recoil!"));
                self.lose_hp(s, rec);
            }
        }
        // Life Orb, Shell Bell
        if !self.act(s).fainted() && total > 0 {
            if self.act(s).has_item("Shell Bell") {
                let got = self.heal(s, (total / 8).max(1) as u16);
                if got > 0 {
                    self.say(format!("{w} restored a little HP using its Shell Bell!"));
                }
            }
            if self.spread_now && mv.fx.selfdestruct.is_none() {
                // on a move that hits several, it takes its toll once, after them all
                self.lo_done = true;
            } else if self.act(s).has_item("Life Orb") && !self.act(s).has_ab("Magic Guard") && mv.fx.selfdestruct.is_none() && !self.lo_done {
                self.lo_done = true;
                let d = (self.act(s).max() / 10).max(1);
                self.say(format!("{w} lost some of its HP!"));
                self.lose_hp(s, d);
            }
        }
        // what happens to them: secondaries
        if !sub_hit && !self.act(o).fainted() {
            self.secondaries(s, o, mv);
            // thaw a frozen target with fire
            if mv.t == "Fire" && self.act(o).status == Status::Freeze {
                self.actm(o).status = Status::None;
                let n = self.who(o);
                self.say(format!("{n} thawed out!"));
                self.ev(Ev::Status { side: o, status: Status::None });
            }
            self.berry_check(o);
        }
        // what happens to the user
        if !self.act(s).fainted() {
            if let Some(se) = mv.fx.on_self.clone() {
                if let Some(b) = &se.boosts {
                    self.boosts_from(s, b, true);
                }
                if se.volatile.as_deref() == Some("mustrecharge") {
                    self.actm(s).v.recharge = true;
                }
            }
            if let Some(se) = mv.fx.self_boost.clone() {
                if let Some(b) = &se.boosts {
                    self.boosts_from(s, b, true);
                }
            }
        }
        // moves with more to them
        match mv.id.as_str() {
            "rapidspin" if !self.act(s).fainted() => {
                let sd = &mut self.sides[s & 1];
                sd.spikes = 0;
                sd.tspikes = 0;
                sd.rocks = false;
                let m = self.actm(s);
                m.v.seeded = false;
                m.v.bound = None;
            }
            "brickbreak" => {
                if self.sides[o & 1].reflect > 0 || self.sides[o & 1].light_screen > 0 {
                    self.sides[o & 1].reflect = 0;
                    self.sides[o & 1].light_screen = 0;
                    self.say("It shattered the barrier!".into());
                }
            }
            "knockoff" if !sub_hit && !self.act(o).item.is_empty() && !self.act(o).has_ab("Sticky Hold") => {
                let it = self.act(o).item.clone();
                self.actm(o).item.clear();
                let n = self.who(o);
                self.say(format!("{w} knocked off {}'s {it}!", n.to_lowercase()));
            }
            "thief" | "covet" if !sub_hit && self.act(s).item.is_empty() && !self.act(o).item.is_empty() && !self.act(o).has_ab("Sticky Hold") => {
                let it = self.act(o).item.clone();
                self.actm(o).item.clear();
                self.actm(s).item = it.clone();
                self.say(format!("{w} stole {}'s {it}!", self.who_l(o)));
            }
            "smellingsalts" if self.act(o).status == Status::Paralysis => {
                self.actm(o).status = Status::None;
                let n = self.who(o);
                self.say(format!("{n} was cured of paralysis."));
                self.ev(Ev::Status { side: o, status: Status::None });
            }
            "wakeupslap" if self.act(o).status == Status::Sleep => {
                self.actm(o).status = Status::None;
                let n = self.who(o);
                self.say(format!("{n} woke up!"));
                self.ev(Ev::Status { side: o, status: Status::None });
            }
            "uproar" => {}
            "futuresight" | "doomdesire" => {}
            "pluck" | "bugbite" if !self.act(o).item.to_lowercase().contains("berry") => {}
            "pluck" | "bugbite" => {
                let it = self.act(o).item.clone();
                self.actm(o).item.clear();
                self.say(format!("{w} stole and ate its target's {it}!"));
            }
            "fling" => {
                self.actm(s).item.clear();
            }
            _ => {}
        }
        if fainted_now {
            self.faint(o);
        }
        // and then the user, if recoil or Life Orb took it down
        let user_down = self.act(s).fainted() && mv.fx.selfdestruct.is_none();
        if user_down {
            self.faint(s);
        }
        // Destiny Bond
        if fainted_now && om.v.destiny_bond {
            if !self.act(s).fainted() && !user_down {
                let n = self.who(o);
                self.say(format!("{n} took its attacker down with it!"));
                self.actm(s).hp = 0;
                self.hp_ev(s);
                self.faint(s);
            }
        }
        // U-turn
        if mv.fx.self_switch.is_some() && !self.act(s).fainted() && self.result.is_none() {
            self.self_switch(s, false);
        }
        // Roar-like damaging moves (none in Gen 4) are status moves
    }

    /// "It's super effective!" (on whom, when the move hit more than one).
    fn eff_line(&self, sup: bool, o: usize) -> String {
        let base = if sup { "It's super effective" } else { "It's not very effective" };
        if self.spread_now {
            format!("{base} on {}{}", self.who_l(o), if sup { "!" } else { "..." })
        } else {
            format!("{base}{}", if sup { "!" } else { "..." })
        }
    }

    /// Volt Absorb, Water Absorb, Flash Fire, Motor Drive, Dry Skin…
    fn absorb(&mut self, o: usize, mv: &MoveSlot, ab: &str) {
        let n = self.who(o);
        match ab {
            "Volt Absorb" | "Water Absorb" | "Dry Skin" => {
                let got = self.heal(o, self.act(o).max() / 4);
                if got > 0 {
                    self.say(format!("{n} restored HP using its {ab}!"));
                } else {
                    self.say(format!("{n}'s {ab} made {} useless!", mv.name));
                }
            }
            "Flash Fire" => {
                self.actm(o).v.flash_fire = true;
                self.say(format!("{n}'s Flash Fire raised the power of its Fire-type moves!"));
            }
            "Motor Drive" => {
                self.boost(o, 5, 1, true);
            }
            _ => self.say(format!("{n}'s {ab} made {} useless!", mv.name)),
        }
    }

    fn contact(&mut self, s: usize, o: usize) {
        let d = self.act(o).clone();
        if self.act(s).fainted() {
            return;
        }
        let ab = if d.v.no_ability { "" } else { d.ability.as_str() };
        let n = self.who_l(s);
        match ab {
            "Rough Skin" if !self.act(s).has_ab("Magic Guard") => {
                let w = self.whos(o);
                self.say(format!("{w} Rough Skin hurt {n}!"));
                let x = (self.act(s).max() / 8).max(1);
                self.lose_hp(s, x);
            }
            "Static" if self.rng.chance(0.3) => {
                self.set_status(s, Status::Paralysis, Some(o), false);
            }
            "Flame Body" if self.rng.chance(0.3) => {
                self.set_status(s, Status::Burn, Some(o), false);
            }
            "Poison Point" if self.rng.chance(0.3) => {
                self.set_status(s, Status::Poison, Some(o), false);
            }
            "Effect Spore" if self.rng.chance(0.3) => {
                let st = [Status::Poison, Status::Paralysis, Status::Sleep][self.rng.below(3) as usize];
                self.set_status(s, st, Some(o), false);
            }
            "Cute Charm" if self.rng.chance(0.3) => {
                let a = self.act(s);
                if let (Some(x), Some(y)) = (a.gender, d.gender) {
                    if x != y && !a.v.attract && !a.has_ab("Oblivious") {
                        self.actm(s).v.attract = true;
                        let w = self.who(s);
                        self.say(format!("{w} fell in love!"));
                    }
                }
            }
            _ => {}
        }
    }

    fn secondaries(&mut self, s: usize, o: usize, mv: &MoveSlot) {
        let a = self.act(s).clone();
        let d = self.act(o).clone();
        let mut had_flinch = false;
        let serene = if a.has_ab("Serene Grace") { 2.0 } else { 1.0 };
        for sec in mv.fx.secondaries().into_iter().cloned().collect::<Vec<_>>() {
            let c = sec.chance.unwrap_or(100) as f64 / 100.0 * serene;
            if sec.volatile.as_deref() == Some("flinch") {
                had_flinch = true;
            }
            let on_target = sec.status.is_some() || sec.volatile.is_some() || sec.boosts.is_some();
            if on_target && d.has_ab("Shield Dust") {
                continue;
            }
            if !self.rng.chance(c) {
                continue;
            }
            if let Some(st) = &sec.status {
                let st = match st.as_str() {
                    "brn" => Status::Burn,
                    "par" => Status::Paralysis,
                    "psn" => Status::Poison,
                    "tox" => Status::Toxic,
                    "slp" => Status::Sleep,
                    "frz" => Status::Freeze,
                    _ => Status::None,
                };
                if st != Status::None {
                    self.set_status(o, st, Some(s), false);
                }
            }
            if mv.id == "triattack" {
                let st = [Status::Burn, Status::Paralysis, Status::Freeze][self.rng.below(3) as usize];
                self.set_status(o, st, Some(s), false);
            }
            match sec.volatile.as_deref() {
                Some("flinch") => {
                    if !self.order.contains(&o) && !d.has_ab("Inner Focus") {
                        self.actm(o).v.flinch = true;
                    }
                }
                Some("confusion") => {
                    self.confuse(o, false);
                }
                _ => {}
            }
            if let Some(b) = &sec.boosts {
                self.boosts_from(o, b, false);
            }
            if let Some(se) = &sec.on_self {
                if let Some(b) = &se.boosts {
                    self.boosts_from(s, b, true);
                }
            }
        }
        // King's Rock / Razor Fang
        if !had_flinch && (a.has_item("King's Rock") || a.has_item("Razor Fang")) && self.rng.chance(0.1) && !self.order.contains(&o) && !d.has_ab("Inner Focus") {
            self.actm(o).v.flinch = true;
        }
        // bound by Wrap, Fire Spin…
        if mv.fx.volatile.as_deref() == Some("partiallytrapped") && self.act(o).v.bound.is_none() {
            let t = if a.has_item("Grip Claw") { 5 } else { self.rng.below(4) as u8 + 2 };
            self.actm(o).v.bound = Some((t, mv.name.clone()));
            let n = self.who(o);
            self.say(format!("{n} was trapped by {}!", mv.name));
        }
    }

    fn self_switch(&mut self, s: usize, baton: bool) {
        let others = (0..self.sides[s & 1].mons.len()).filter(|&k| self.sides[s & 1].can_switch_to(k)).count();
        if others == 0 {
            return;
        }
        if s & 1 == 0 {
            self.ask = Some(Ask::Switch { slot: s >> 1, baton });
        } else {
            let k = self.ai_replacement(s);
            let n = self.sides[1].name.clone();
            let w = self.who(s);
            self.say(format!("{w} went back to {n}!"));
            self.say(format!("{n} sent out {}!", self.sides[1].mons[k].name));
            self.switch_to(s, k, baton);
        }
    }

    fn status_move(&mut self, s: usize, i: usize, mv: &MoveSlot, t: usize) {
        let o = if t == s { self.foe(s) } else { t };
        let w = self.who(s);
        let fx = mv.fx.clone();
        let target_self = fx.target == "self" || fx.target == "allySide" || fx.target == "adjacentAllyOrSelf";
        let d = self.act(o).clone();
        // a substitute blocks most status moves aimed at it
        if !target_self && d.v.sub > 0 && !matches!(fx.target.as_str(), "foeSide" | "all") && !mv.flag("sound") && !matches!(mv.id.as_str(), "roar" | "whirlwind" | "haze" | "perishsong" | "transform" | "curse") {
            self.say("But it failed!".into());
            return;
        }
        // Thunder Wave can't hit Ground types; others get type immunity too
        if !target_self && mv.id == "thunderwave" && calc::eff("Electric", &d.types, self.world) == 0.0 {
            self.say(format!("It doesn't affect {}...", self.who_l(o)));
            return;
        }
        let mut did = false;
        match mv.id.as_str() {
            "protect" | "detect" | "endure" => {
                let n = self.act(s).v.protect_n;
                let ok = !self.steps.iter().all(|st| matches!(st, Step::EndOfTurn | Step::Replace)) && self.rng.chance(1.0 / (1u32 << n.min(8)) as f64);
                if ok {
                    let m = self.actm(s);
                    m.v.protect_n = n + 1;
                    if mv.id == "endure" {
                        m.v.endure = true;
                        self.say(format!("{w} braced itself!"));
                    } else {
                        m.v.protect = true;
                        self.say(format!("{w} protected itself!"));
                    }
                } else {
                    self.actm(s).v.protect_n = 0;
                    self.say("But it failed!".into());
                }
                return;
            }
            "rest" => {
                let m = self.act(s);
                if m.hp == m.max() || m.has_ab("Insomnia") || m.has_ab("Vital Spirit") {
                    self.say("But it failed!".into());
                    return;
                }
                let mm = self.actm(s);
                mm.status = Status::Sleep;
                mm.sleep = 2;
                mm.hp = mm.max();
                self.ev(Ev::Status { side: s, status: Status::Sleep });
                self.hp_ev(s);
                self.say(format!("{w} slept and became healthy!"));
                return;
            }
            "painsplit" => {
                let a = self.act(s).hp as u32;
                let b = d.hp as u32;
                let avg = ((a + b) / 2) as u16;
                {
                    let m = self.actm(s);
                    m.hp = avg.min(m.max());
                }
                {
                    let m = self.actm(o);
                    m.hp = avg.min(m.max());
                }
                self.hp_ev(s);
                self.hp_ev(o);
                self.say("The battlers shared their pain!".into());
                return;
            }
            "bellydrum" => {
                let m = self.act(s);
                if m.hp <= m.max() / 2 || m.boosts[1] >= 6 {
                    self.say("But it failed!".into());
                    return;
                }
                let cost = m.max() / 2;
                self.damage_raw(s, cost);
                self.actm(s).boosts[1] = 6;
                self.say(format!("{w} cut its own HP and maximized its Attack!"));
                return;
            }
            "haze" => {
                for k in 0..2 {
                    self.actm(k).boosts = [0; 8];
                }
                self.say("All stat changes were eliminated!".into());
                return;
            }
            "psychup" => {
                let b = d.boosts;
                self.actm(s).boosts = b;
                self.say(format!("{w} copied {}'s stat changes!", self.who_l(o)));
                return;
            }
            "transform" => {
                let mut t = d.clone();
                let me = self.act(s).clone();
                for m in &mut t.moves {
                    m.pp = 5.min(m.max_pp);
                    m.max_pp = 5;
                }
                let mm = self.actm(s);
                mm.types = t.types;
                mm.stats = [me.stats[0], t.stats[1], t.stats[2], t.stats[3], t.stats[4], t.stats[5]];
                mm.boosts = t.boosts;
                mm.moves = t.moves;
                mm.ability = t.ability;
                mm.v.transformed = true;
                self.say(format!("{w} transformed into {}!", d.name));
                return;
            }
            "roar" | "whirlwind" => {
                if d.has_ab("Suction Cups") || d.v.ingrain {
                    self.say(format!("{} anchored itself!", self.who(o)));
                    return;
                }
                if self.wild {
                    if s & 1 == 1 {
                        self.say("You were blown away!".into());
                        self.finish(Outcome::Ran);
                    } else {
                        self.say(format!("{} fled in fear!", self.who(o)));
                        self.finish(Outcome::Fled);
                    }
                    return;
                }
                let opts: Vec<usize> = (0..self.sides[o & 1].mons.len()).filter(|&k| self.sides[o & 1].can_switch_to(k)).collect();
                if opts.is_empty() {
                    self.say("But it failed!".into());
                    return;
                }
                let k = opts[self.rng.below(opts.len() as u32) as usize];
                let n = self.sides[o & 1].mons[k].name.clone();
                self.switch_to(o, k, false);
                self.say(format!("{n} was dragged out!"));
                return;
            }
            "batonpass" => {
                self.self_switch(s, true);
                return;
            }
            "teleport" => {
                if self.wild && s & 1 == 0 {
                    self.say("Got away safely!".into());
                    self.finish(Outcome::Ran);
                } else if self.wild {
                    self.say(format!("{w} fled from battle!"));
                    self.finish(Outcome::Fled);
                } else {
                    self.say("But it failed!".into());
                }
                return;
            }
            "moonlight" | "morningsun" | "synthesis" => {
                let max = self.act(s).max();
                let h = match self.eff_weather() {
                    Weather::Sun => max * 2 / 3,
                    Weather::None => max / 2,
                    _ => max / 4,
                };
                let got = self.heal(s, h);
                if got > 0 {
                    self.say(format!("{w} regained health!"));
                } else {
                    self.say(format!("{w}'s HP is full!"));
                }
                return;
            }
            "wish" => {
                if self.sides[s & 1].wish.is_some() {
                    self.say("But it failed!".into());
                } else {
                    self.sides[s & 1].wish = Some((2, self.act(s).max() / 2, s >> 1));
                    self.say(format!("{w} made a wish!"));
                }
                return;
            }
            "healbell" | "aromatherapy" => {
                for m in &mut self.sides[s & 1].mons {
                    if !m.fainted() && !(mv.id == "healbell" && m.has_ab("Soundproof")) {
                        m.status = Status::None;
                    }
                }
                self.ev(Ev::Status { side: s, status: Status::None });
                self.say(if mv.id == "healbell" { "A bell chimed!".into() } else { "A soothing aroma wafted through the area!".to_string() });
                return;
            }
            "refresh" => {
                if matches!(self.act(s).status, Status::Burn | Status::Paralysis | Status::Poison | Status::Toxic) {
                    self.actm(s).status = Status::None;
                    self.ev(Ev::Status { side: s, status: Status::None });
                    self.say(format!("{w}'s status returned to normal!"));
                } else {
                    self.say("But it failed!".into());
                }
                return;
            }
            "trick" | "switcheroo" => {
                let a = self.act(s).item.clone();
                let b = d.item.clone();
                if (a.is_empty() && b.is_empty()) || d.has_ab("Sticky Hold") || d.v.sub > 0 {
                    self.say("But it failed!".into());
                    return;
                }
                self.actm(s).item = b.clone();
                self.actm(o).item = a.clone();
                self.actm(s).v.choice = None;
                self.actm(o).v.choice = None;
                self.say(format!("{w} switched items with its target!"));
                if !b.is_empty() {
                    self.say(format!("{w} obtained {b}."));
                }
                if !a.is_empty() {
                    self.say(format!("{} obtained {a}.", self.who(o)));
                }
                return;
            }
            "perishsong" => {
                for k in 0..2 {
                    if !self.act(k).has_ab("Soundproof") && self.act(k).v.perish == 0 {
                        self.actm(k).v.perish = 4;
                    }
                }
                self.say("All Pokémon hearing the song will faint in three turns!".into());
                return;
            }
            "memento" => {
                self.boost(o, 1, -2, false);
                self.boost(o, 3, -2, false);
                self.actm(s).hp = 0;
                self.hp_ev(s);
                self.faint(s);
                return;
            }
            "healingwish" | "lunardance" => {
                if self.sides[s & 1].left() <= 1 {
                    self.say("But it failed!".into());
                    return;
                }
                self.sides[s & 1].healing_wish = true;
                self.actm(s).hp = 0;
                self.hp_ev(s);
                self.faint(s);
                return;
            }
            "destinybond" => {
                self.actm(s).v.destiny_bond = true;
                self.say(format!("{w} is trying to take its foe down with it!"));
                return;
            }
            "meanlook" | "block" | "spiderweb" => {
                if d.v.cant_escape {
                    self.say("But it failed!".into());
                } else {
                    self.actm(o).v.cant_escape = true;
                    self.say(format!("{} can't escape now!", self.who(o)));
                }
                return;
            }
            "curse" if self.act(s).is("Ghost") => {
                if d.v.cursed {
                    self.say("But it failed!".into());
                    return;
                }
                let cost = self.act(s).max() / 2;
                self.say(format!("{w} cut its own HP and laid a curse on {}!", self.who_l(o)));
                self.damage_raw(s, cost);
                self.actm(o).v.cursed = true;
                return;
            }
            "curse" => {
                self.boost(s, 5, -1, true);
                self.boost(s, 1, 1, true);
                self.boost(s, 2, 1, true);
                return;
            }
            "spite" => {
                if let Some(k) = d.v.last_move {
                    let mm = self.actm(o);
                    if let Some(m) = mm.moves.get_mut(k) {
                        let before = m.pp;
                        m.pp = m.pp.saturating_sub(4);
                        let (name, cut) = (m.name.clone(), before - m.pp);
                        self.say(format!("It reduced the PP of {}'s {name} by {cut}!", self.who_l(o)));
                        return;
                    }
                }
                self.say("But it failed!".into());
                return;
            }
            "stockpile" => {
                if self.act(s).v.stockpile >= 3 {
                    self.say("But it failed!".into());
                    return;
                }
                self.actm(s).v.stockpile += 1;
                let n = self.act(s).v.stockpile;
                self.say(format!("{w} stockpiled {n}!"));
                self.boost(s, 2, 1, true);
                self.boost(s, 4, 1, true);
                return;
            }
            "swallow" => {
                let n = self.act(s).v.stockpile;
                if n == 0 {
                    self.say("But it failed to swallow a thing!".into());
                    return;
                }
                let max = self.act(s).max();
                let h = [max / 4, max / 2, max][(n as usize - 1).min(2)];
                self.heal(s, h);
                self.actm(s).v.stockpile = 0;
                self.say(format!("{w} regained health!"));
                return;
            }
            "splash" => {
                self.say("But nothing happened!".into());
                return;
            }
            "conversion" => {
                if let Some(t) = self.act(s).moves.first().map(|m| m.t.clone()) {
                    self.actm(s).types = vec![t.clone()];
                    self.say(format!("{w} transformed into the {t} type!"));
                }
                return;
            }
            "foresight" | "odorsleuth" | "miracleeye" => {
                self.actm(o).v.foresight = true;
                self.say(format!("{w} identified {}!", self.who_l(o)));
                return;
            }
            "lockon" | "mindreader" => {
                self.say(format!("{w} took aim at {}!", self.who_l(o)));
                self.actm(s).boosts[6] = (self.act(s).boosts[6] + 0).max(0);
                return;
            }
            "futuresight" | "doomdesire" => {
                if self.sides[o & 1].future.is_some() {
                    self.say("But it failed!".into());
                    return;
                }
                let (h, _) = self.calc_hit(s, o, &MoveSlot { cat: "Special".into(), bp: if mv.id == "futuresight" { 80 } else { 120 }, t: if mv.id == "futuresight" { "???".into() } else { "Steel".into() }, ..mv.clone() }, false);
                let dmg = h.rolls[self.rng.below(16) as usize] as u16;
                self.sides[o & 1].future = Some((3, dmg, mv.name.clone(), o >> 1));
                self.say(format!("{w} foresaw an attack!"));
                return;
            }
            _ => {}
        }
        // the data-driven part
        if let Some(st) = &fx.status {
            let st = match st.as_str() {
                "brn" => Status::Burn,
                "par" => Status::Paralysis,
                "psn" => Status::Poison,
                "tox" => Status::Toxic,
                "slp" => Status::Sleep,
                "frz" => Status::Freeze,
                _ => Status::None,
            };
            let tgt = if target_self { s } else { o };
            did |= self.set_status(tgt, st, Some(s), true);
            if !did {
                return;
            }
        }
        if let Some(wx) = &fx.weather {
            let (w2, rock) = match wx.as_str() {
                "RainDance" | "raindance" => (Weather::Rain, "Damp Rock"),
                "sunnyday" | "SunnyDay" => (Weather::Sun, "Heat Rock"),
                "Sandstorm" | "sandstorm" => (Weather::Sand, "Smooth Rock"),
                "hail" | "Hail" => (Weather::Hail, "Icy Rock"),
                _ => (Weather::None, ""),
            };
            if w2 != Weather::None {
                let turns = if self.act(s).has_item(rock) { 8 } else { 5 };
                self.set_weather(w2, turns, None);
                return;
            }
        }
        if let Some(sc) = &fx.side_condition {
            let mine = fx.target != "foeSide";
            let side = if mine { s & 1 } else { o & 1 };
            let light_clay = self.act(s).has_item("Light Clay");
            let sd = &mut self.sides[side];
            let line = match sc.as_str() {
                "reflect" if sd.reflect == 0 => {
                    sd.reflect = if light_clay { 8 } else { 5 };
                    Some("Reflect raised the team's Defense!".to_string())
                }
                "lightscreen" if sd.light_screen == 0 => {
                    sd.light_screen = if light_clay { 8 } else { 5 };
                    Some("Light Screen raised the team's Sp. Def!".to_string())
                }
                "safeguard" if sd.safeguard == 0 => {
                    sd.safeguard = 5;
                    Some("The team became cloaked in a mystical veil!".to_string())
                }
                "mist" if sd.mist == 0 => {
                    sd.mist = 5;
                    Some("The team became shrouded in mist!".to_string())
                }
                "tailwind" if sd.tailwind == 0 => {
                    sd.tailwind = 3;
                    Some("The tailwind blew from behind the team!".to_string())
                }
                "luckychant" if sd.lucky_chant == 0 => {
                    sd.lucky_chant = 5;
                    Some("The Lucky Chant shielded the team from critical hits!".to_string())
                }
                "spikes" if sd.spikes < 3 => {
                    sd.spikes += 1;
                    Some("Spikes were scattered all around the feet of the foe's team!".to_string())
                }
                "toxicspikes" if sd.tspikes < 2 => {
                    sd.tspikes += 1;
                    Some("Poison spikes were scattered all around the foe's team's feet!".to_string())
                }
                "stealthrock" if !sd.rocks => {
                    sd.rocks = true;
                    Some("Pointed stones float in the air around the foe's team!".to_string())
                }
                _ => None,
            };
            match line {
                Some(l) => {
                    let l = if side == 1 { l } else { l.replace("the foe's team", "your team").replace("the foe's team's", "your team's") };
                    self.say(l);
                }
                None => self.say("But it failed!".into()),
            }
            return;
        }
        if let Some(pw) = &fx.pseudo_weather {
            match pw.as_str() {
                "trickroom" => {
                    if self.trick_room > 0 {
                        self.trick_room = 0;
                        self.say("The twisted dimensions returned to normal!".into());
                    } else {
                        self.trick_room = 5;
                        self.say(format!("{w} twisted the dimensions!"));
                    }
                }
                "gravity" => {
                    if self.gravity > 0 {
                        self.say("But it failed!".into());
                    } else {
                        self.gravity = 5;
                        self.say("Gravity intensified!".into());
                    }
                }
                _ => self.say("But nothing happened!".into()),
            }
            return;
        }
        if let Some(vol) = fx.volatile.clone() {
            let tgt = if target_self { s } else { o };
            let tn = self.who(tgt);
            let ok = match vol.as_str() {
                "confusion" => self.confuse(tgt, true),
                "substitute" => {
                    let m = self.act(s);
                    let cost = m.max() / 4;
                    if m.v.sub > 0 {
                        self.say(format!("{w} already has a substitute!"));
                        false
                    } else if m.hp <= cost || cost == 0 {
                        self.say("It was too weak to make a substitute!".into());
                        false
                    } else {
                        self.damage_raw(s, cost);
                        self.actm(s).v.sub = cost + 1;
                        self.say(format!("{w} made a substitute!"));
                        true
                    }
                }
                "leechseed" => {
                    if d.is("Grass") {
                        self.say(format!("It doesn't affect {}...", self.who_l(o)));
                        false
                    } else if d.v.seeded {
                        self.say(format!("{} is already seeded!", self.who(o)));
                        false
                    } else {
                        self.actm(o).v.seeded = true;
                        self.actm(o).v.seeder = s;
                        self.say(format!("{} was seeded!", self.who(o)));
                        true
                    }
                }
                "focusenergy" => {
                    self.actm(s).v.focus_energy = true;
                    self.say(format!("{w} is getting pumped!"));
                    true
                }
                "taunt" => {
                    if d.v.taunt > 0 {
                        false
                    } else {
                        let t = self.rng.below(3) as u8 + 3;
                        self.actm(o).v.taunt = t;
                        self.say(format!("{tn} fell for the taunt!"));
                        true
                    }
                }
                "encore" => match d.v.last_move {
                    Some(k) if d.v.encore.is_none() && !matches!(d.moves.get(k).map(|m| m.id.as_str()), Some("encore" | "mimic" | "transform" | "struggle")) => {
                        let t = self.rng.below(5) as u8 + 4;
                        self.actm(o).v.encore = Some((k, t));
                        self.say(format!("{tn} received an encore!"));
                        true
                    }
                    _ => false,
                },
                "disable" => match d.v.last_move {
                    Some(k) if d.v.disable.is_none() => {
                        let t = self.rng.below(4) as u8 + 4;
                        let name = d.moves.get(k).map(|m| m.name.clone()).unwrap_or_default();
                        self.actm(o).v.disable = Some((k, t));
                        self.say(format!("{}'s {name} was disabled!", tn));
                        true
                    }
                    _ => false,
                },
                "attract" => {
                    let a = self.act(s);
                    match (a.gender, d.gender) {
                        (Some(x), Some(y)) if x != y && !d.v.attract && !d.has_ab("Oblivious") => {
                            self.actm(o).v.attract = true;
                            self.say(format!("{tn} fell in love!"));
                            true
                        }
                        _ => false,
                    }
                }
                "ingrain" => {
                    if self.act(s).v.ingrain {
                        false
                    } else {
                        self.actm(s).v.ingrain = true;
                        self.say(format!("{w} planted its roots!"));
                        true
                    }
                }
                "aquaring" => {
                    self.actm(s).v.aqua_ring = true;
                    self.say(format!("{w} surrounded itself with a veil of water!"));
                    true
                }
                "yawn" => {
                    if d.status != Status::None || d.v.yawn > 0 || self.sides[o & 1].safeguard > 0 {
                        false
                    } else {
                        self.actm(o).v.yawn = 2;
                        self.say(format!("{w} made {} drowsy!", self.who_l(o)));
                        true
                    }
                }
                "nightmare" => {
                    if d.status != Status::Sleep || d.v.nightmare {
                        false
                    } else {
                        self.actm(o).v.nightmare = true;
                        self.say(format!("{tn} began having a nightmare!"));
                        true
                    }
                }
                "torment" => {
                    self.actm(o).v.torment = true;
                    self.say(format!("{tn} was subjected to torment!"));
                    true
                }
                "magnetrise" => {
                    if self.act(s).v.magnet_rise > 0 {
                        false
                    } else {
                        self.actm(s).v.magnet_rise = 5;
                        self.say(format!("{w} levitated with electromagnetism!"));
                        true
                    }
                }
                "embargo" => {
                    self.actm(o).v.embargo = 5;
                    self.say(format!("{tn} can't use items anymore!"));
                    true
                }
                "healblock" => {
                    self.actm(o).v.heal_block = 5;
                    self.say(format!("{tn} was prevented from healing!"));
                    true
                }
                "gastroacid" => {
                    self.actm(o).v.no_ability = true;
                    self.say(format!("{tn}'s ability was suppressed!"));
                    true
                }
                "charge" => {
                    self.actm(s).v.charge = true;
                    self.say(format!("{w} began charging power!"));
                    true
                }
                "minimize" => {
                    self.actm(s).v.minimized = true;
                    true
                }
                "defensecurl" => {
                    self.actm(s).v.defense_curl = true;
                    true
                }
                "roost" => {
                    self.actm(s).v.roost = true;
                    true
                }
                "foresight" | "miracleeye" => {
                    self.actm(o).v.foresight = true;
                    self.say(format!("{w} identified {}!", self.who_l(o)));
                    true
                }
                "powertrick" => {
                    let m = self.actm(s);
                    m.stats.swap(1, 2);
                    self.say(format!("{w} switched its Attack and Defense!"));
                    true
                }
                _ => {
                    self.say("But nothing happened!".into());
                    return;
                }
            };
            if !ok && !matches!(vol.as_str(), "confusion" | "substitute" | "leechseed") {
                self.say("But it failed!".into());
                return;
            }
            did |= ok;
        }
        if let Some(b) = fx.boosts.clone() {
            let tgt = if target_self { s } else { o };
            did |= self.boosts_from(tgt, &b, target_self);
        }
        if let Some([n, dn]) = fx.heal {
            let m = self.act(s);
            let h = ((m.max() as u32 * n as u32) / dn.max(1) as u32) as u16;
            let got = self.heal(s, h.max(1));
            if got > 0 {
                self.say(format!("{w} regained health!"));
            } else {
                self.say(format!("{w}'s HP is full!"));
            }
            did = true;
        }
        if let Some(se) = fx.on_self.clone() {
            if let Some(b) = &se.boosts {
                did |= self.boosts_from(s, b, true);
            }
        }
        if fx.self_switch.is_some() && mv.id != "batonpass" {
            self.self_switch(s, false);
            did = true;
        }
        if !did && fx.status.is_none() && fx.boosts.is_none() && fx.volatile.is_none() {
            self.say("But nothing happened!".into());
        }
        let _ = i;
    }

    // ───────────────────────────── end of turn ─────────────────────────────

    fn end_of_turn(&mut self) {
        // the faster ones first
        let mut order: Vec<usize> = self.battlers();
        order.sort_by_key(|b| std::cmp::Reverse(self.speed(*b)));
        // weather
        if self.weather != Weather::None {
            if self.weather_turns > 0 {
                self.weather_turns -= 1;
                if self.weather_turns == 0 {
                    let w = self.weather;
                    self.weather = Weather::None;
                    self.ev(Ev::Weather(Weather::None));
                    self.say(
                        match w {
                            Weather::Rain => "The rain stopped.",
                            Weather::Sun => "The sunlight faded.",
                            Weather::Sand => "The sandstorm subsided.",
                            Weather::Hail => "The hail stopped.",
                            Weather::None => "",
                        }
                        .into(),
                    );
                }
            }
            let ew = self.eff_weather();
            if ew != Weather::None {
                self.say(
                    match ew {
                        Weather::Rain => "Rain continues to fall.",
                        Weather::Sun => "The sunlight is strong.",
                        Weather::Sand => "The sandstorm rages.",
                        Weather::Hail => "Hail continues to fall.",
                        Weather::None => "",
                    }
                    .into(),
                );
                for &s in &order {
                    let m = self.act(s).clone();
                    if m.fainted() || m.v.hidden.as_deref().is_some_and(|h| h == "dig" || h == "dive") {
                        continue;
                    }
                    let w = self.who(s);
                    let max = m.max();
                    match ew {
                        Weather::Sand if !(m.is("Rock") || m.is("Ground") || m.is("Steel") || m.has_ab("Sand Veil") || m.has_ab("Magic Guard")) => {
                            self.say(format!("{w} is buffeted by the sandstorm!"));
                            self.damage_raw(s, (max / 16).max(1));
                        }
                        Weather::Hail if m.has_ab("Ice Body") => {
                            if self.heal(s, (max / 16).max(1)) > 0 {
                                self.say(format!("{w}'s Ice Body heals it!"));
                            }
                        }
                        Weather::Hail if !(m.is("Ice") || m.has_ab("Snow Cloak") || m.has_ab("Magic Guard")) => {
                            self.say(format!("{w} is pelted by hail!"));
                            self.damage_raw(s, (max / 16).max(1));
                        }
                        Weather::Rain if m.has_ab("Rain Dish") || m.has_ab("Dry Skin") => {
                            let h = if m.has_ab("Dry Skin") { max / 8 } else { max / 16 };
                            if self.heal(s, h.max(1)) > 0 {
                                self.say(format!("{w}'s {} restored its HP!", m.ability));
                            }
                        }
                        Weather::Rain if m.has_ab("Hydration") && m.status != Status::None => {
                            self.actm(s).status = Status::None;
                            self.ev(Ev::Status { side: s, status: Status::None });
                            self.say(format!("{w}'s Hydration cured its status!"));
                        }
                        Weather::Sun if m.has_ab("Dry Skin") || m.has_ab("Solar Power") => {
                            self.say(format!("{w} was hurt by its {}!", m.ability));
                            self.damage_raw(s, (max / 8).max(1));
                        }
                        _ => {}
                    }
                }
            }
        }
        // Future Sight, Wish
        for side in 0..2 {
            if let Some((t, dmg, name, slot)) = self.sides[side].future.clone() {
                let s = (slot * 2 + side).min(self.sides[side].slots.len().saturating_sub(1) * 2 + side);
                if t <= 1 {
                    self.sides[side].future = None;
                    if self.present(s) {
                        let w = self.who(s);
                        self.say(format!("{w} took the {name} attack!"));
                        self.ev(Ev::Hit { side: s, eff: 1.0, crit: false });
                        self.damage_raw(s, dmg.max(1));
                    }
                } else {
                    self.sides[side].future = Some((t - 1, dmg, name, slot));
                }
            }
            if let Some((t, h, slot)) = self.sides[side].wish {
                let s = (slot * 2 + side).min(self.sides[side].slots.len().saturating_sub(1) * 2 + side);
                if t <= 1 {
                    self.sides[side].wish = None;
                    if self.present(s) && self.heal(s, h) > 0 {
                        let w = self.who(s);
                        self.say(format!("{w}'s wish came true!"));
                    }
                } else {
                    self.sides[side].wish = Some((t - 1, h, slot));
                }
            }
        }
        for &s in &order {
            if self.result.is_some() {
                return;
            }
            let m = self.act(s).clone();
            if m.fainted() {
                continue;
            }
            let w = self.who(s);
            let max = m.max();
            // Ingrain, Aqua Ring, Leftovers
            if m.v.ingrain && self.heal(s, (max / 16).max(1)) > 0 {
                self.say(format!("{w} absorbed nutrients with its roots!"));
            }
            if m.v.aqua_ring && self.heal(s, (max / 16).max(1)) > 0 {
                self.say(format!("A veil of water restored {}'s HP!", self.who_l(s)));
            }
            if m.has_item("Leftovers") && self.heal(s, (max / 16).max(1)) > 0 {
                self.say(format!("{w} restored a little HP using its Leftovers!"));
            }
            if m.has_item("Black Sludge") {
                if m.is("Poison") {
                    if self.heal(s, (max / 16).max(1)) > 0 {
                        self.say(format!("{w} restored a little HP using its Black Sludge!"));
                    }
                } else if !m.has_ab("Magic Guard") {
                    self.say(format!("{w} is hurt by its Black Sludge!"));
                    self.damage_raw(s, (max / 8).max(1));
                }
            }
            if self.act(s).fainted() {
                continue;
            }
            // Leech Seed
            let seeder = if self.present(m.v.seeder) && (m.v.seeder & 1) != (s & 1) { m.v.seeder } else { self.foe(s) };
            if m.v.seeded && self.present(seeder) && !m.has_ab("Magic Guard") {
                let d = (max / 8).max(1).min(self.act(s).hp);
                self.say(format!("{w}'s health is sapped by Leech Seed!"));
                self.damage_raw(s, d);
                let mut h = d;
                if self.act(seeder).has_item("Big Root") {
                    h = h * 13 / 10;
                }
                if m.has_ab("Liquid Ooze") {
                    self.damage_raw(seeder, h);
                } else {
                    self.heal(seeder, h);
                }
                if self.act(s).fainted() {
                    continue;
                }
            }
            // status damage
            let m = self.act(s).clone();
            match m.status {
                Status::Poison | Status::Toxic if m.has_ab("Poison Heal") => {
                    if self.heal(s, (max / 8).max(1)) > 0 {
                        self.say(format!("{w} is healed by Poison Heal!"));
                    }
                }
                Status::Poison if !m.has_ab("Magic Guard") => {
                    self.say(format!("{w} is hurt by poison!"));
                    self.damage_raw(s, (max / 8).max(1));
                }
                Status::Toxic if !m.has_ab("Magic Guard") => {
                    let n = (m.toxic + 1).min(15);
                    self.actm(s).toxic = n;
                    self.say(format!("{w} is hurt by poison!"));
                    self.damage_raw(s, ((max as u32 * n as u32) / 16).max(1) as u16);
                }
                Status::Burn if !m.has_ab("Magic Guard") => {
                    self.say(format!("{w} is hurt by its burn!"));
                    let d = if m.has_ab("Heatproof") { max / 16 } else { max / 8 };
                    self.damage_raw(s, d.max(1));
                }
                Status::Sleep if m.v.nightmare && !m.has_ab("Magic Guard") => {
                    self.say(format!("{w} is locked in a nightmare!"));
                    self.damage_raw(s, (max / 4).max(1));
                }
                _ => {}
            }
            if self.act(s).fainted() {
                continue;
            }
            if m.v.cursed && !m.has_ab("Magic Guard") {
                self.say(format!("{w} is afflicted by the curse!"));
                self.damage_raw(s, (max / 4).max(1));
                if self.act(s).fainted() {
                    continue;
                }
            }
            if let Some((t, by)) = m.v.bound.clone() {
                if t <= 1 {
                    self.actm(s).v.bound = None;
                    self.say(format!("{w} was freed from {by}!"));
                } else {
                    self.actm(s).v.bound = Some((t - 1, by.clone()));
                    if !m.has_ab("Magic Guard") {
                        self.say(format!("{w} is hurt by {by}!"));
                        self.damage_raw(s, (max / 16).max(1));
                    }
                }
                if self.act(s).fainted() {
                    continue;
                }
            }
            // counters
            let mm = self.actm(s);
            mm.v.turns_out += 1;
            if mm.v.taunt > 0 {
                mm.v.taunt -= 1;
                if mm.v.taunt == 0 {
                    let w2 = w.clone();
                    self.say(format!("{w2}'s taunt wore off!"));
                }
            }
            let mm = self.actm(s);
            if let Some((k, t)) = mm.v.encore {
                mm.v.encore = if t <= 1 { None } else { Some((k, t - 1)) };
                if mm.v.encore.is_none() {
                    self.say(format!("{w}'s encore ended!"));
                }
            }
            let mm = self.actm(s);
            if let Some((k, t)) = mm.v.disable {
                mm.v.disable = if t <= 1 { None } else { Some((k, t - 1)) };
            }
            mm.v.magnet_rise = mm.v.magnet_rise.saturating_sub(1);
            mm.v.embargo = mm.v.embargo.saturating_sub(1);
            mm.v.heal_block = mm.v.heal_block.saturating_sub(1);
            if !mm.v.moved || mm.v.protect_n > 0 && !mm.v.protect && !mm.v.endure {
                // a turn without Protect resets its chain
            }
            if !mm.v.protect && !mm.v.endure {
                mm.v.protect_n = 0;
            }
            mm.v.charge = mm.v.charge && mm.v.turns_out <= 1;
            // Yawn
            if mm.v.yawn > 0 {
                mm.v.yawn -= 1;
                if mm.v.yawn == 0 {
                    self.set_status(s, Status::Sleep, None, false);
                }
            }
            // Perish Song
            let mm = self.actm(s);
            if mm.v.perish > 0 {
                mm.v.perish -= 1;
                let n = mm.v.perish;
                self.say(format!("{w}'s perish count fell to {n}."));
                if n == 0 {
                    self.actm(s).hp = 0;
                    self.hp_ev(s);
                    self.faint(s);
                    continue;
                }
            }
            // Speed Boost, Shed Skin, orbs
            let m = self.act(s).clone();
            if m.has_ab("Speed Boost") && m.v.turns_out > 0 && m.boosts[5] < 6 {
                self.boost(s, 5, 1, true);
            }
            if m.has_ab("Shed Skin") && m.status != Status::None && self.rng.chance(0.3) {
                self.actm(s).status = Status::None;
                self.ev(Ev::Status { side: s, status: Status::None });
                self.say(format!("{w}'s Shed Skin cured its status!"));
            }
            if m.status == Status::None {
                if m.has_item("Toxic Orb") {
                    self.set_status(s, Status::Toxic, None, false);
                } else if m.has_item("Flame Orb") {
                    self.set_status(s, Status::Burn, None, false);
                }
            }
            self.berry_check(s);
        }
        // the field counts down
        for s in 0..2 {
            let sd = &mut self.sides[s & 1];
            let mut ended: Vec<&str> = Vec::new();
            for (v, name) in [(&mut sd.reflect, "Reflect"), (&mut sd.light_screen, "Light Screen"), (&mut sd.safeguard, "Safeguard"), (&mut sd.mist, "Mist"), (&mut sd.tailwind, "Tailwind"), (&mut sd.lucky_chant, "Lucky Chant")] {
                if *v > 0 {
                    *v -= 1;
                    if *v == 0 {
                        ended.push(name);
                    }
                }
            }
            for n in ended {
                let whose = if s == 0 { "Your team's" } else { "The foe's" };
                self.say(format!("{whose} {n} wore off!"));
            }
        }
        if self.trick_room > 0 {
            self.trick_room -= 1;
            if self.trick_room == 0 {
                self.say("The twisted dimensions returned to normal!".into());
            }
        }
        if self.gravity > 0 {
            self.gravity -= 1;
            if self.gravity == 0 {
                self.say("Gravity returned to normal!".into());
            }
        }
    }

    // ───────────────────────────── the AI ─────────────────────────────

    /// Damage as a share of the usual target's max HP (min, max) for one use.
    #[allow(dead_code)]
    pub fn estimate(&self, s: usize, i: usize) -> Option<(f32, f32)> {
        self.estimate_vs(s, i, self.foe(s))
    }

    /// The same against battler `o`.
    pub fn estimate_vs(&self, s: usize, i: usize, o: usize) -> Option<(f32, f32)> {
        let mv = self.act(s).moves.get(i)?;
        if mv.status() || self.act(o).fainted() {
            return None;
        }
        let (h, _) = self.calc_hit(s, o, mv, false);
        if h.none.is_some() {
            let fixed = matches!(mv.id.as_str(), "seismictoss" | "nightshade" | "dragonrage" | "sonicboom" | "superfang");
            if !fixed {
                return Some((0.0, 0.0));
            }
        }
        let (lo, hi) = mv.fx.hits();
        let n = if lo == 2 && hi == 5 {
            if self.act(s).has_ab("Skill Link") {
                5.0
            } else {
                3.0
            }
        } else {
            lo.max(1) as f32
        };
        let max = self.act(o).max().max(1) as f32;
        Some((h.rolls[0] as f32 * n / max, h.rolls[15] as f32 * n / max))
    }

    fn acc_of(&self, s: usize, mv: &MoveSlot) -> f32 {
        if mv.acc == 0 || self.act(s).has_ab("No Guard") {
            1.0
        } else {
            (mv.acc as f32 / 100.0).min(1.0)
        }
    }

    /// How good a move looks right now (higher is better).
    fn score_move(&self, s: usize, i: usize, o: usize, ai: Ai) -> f32 {
        let me = self.act(s);
        let them = self.act(o);
        let Some(mv) = me.moves.get(i) else { return -1.0 };
        let acc = self.acc_of(s, mv);
        if !mv.status() {
            let Some((lo, hi)) = self.estimate_vs(s, i, o) else { return 0.0 };
            let hp = them.hp_frac();
            let avg = (lo + hi) / 2.0;
            let mut sc = avg.min(hp) / hp.max(0.01) * 100.0 * acc;
            if lo >= hp {
                sc += 80.0;
            } else if hi >= hp {
                sc += 40.0;
            }
            if ai == Ai::Expert {
                // priority to finish it off, or to go first when we're slower
                if mv.pri > 0 && hi >= hp {
                    sc += 40.0;
                }
                if mv.fx.recoil.is_some() && me.hp_frac() < 0.3 {
                    sc -= 15.0;
                }
                if mv.fx.self_switch.is_some() {
                    sc += 5.0;
                }
            }
            if mv.flag("charge") && !(mv.id == "solarbeam" && self.eff_weather() == Weather::Sun) && !me.has_item("Power Herb") {
                sc *= 0.55;
            }
            if mv.fx.on_self.as_ref().and_then(|x| x.volatile.as_deref()) == Some("mustrecharge") && lo < hp {
                sc *= 0.6;
            }
            if mv.fx.selfdestruct.is_some() {
                sc *= if me.hp_frac() < 0.25 { 1.0 } else { 0.3 };
            }
            if mv.id == "falseswipe" {
                sc *= 0.5;
            }
            return sc;
        }
        // status moves
        let id = mv.id.as_str();
        let healthy = me.hp_frac();
        let fx = &mv.fx;
        let mut sc: f32 = 8.0;
        if let Some(st) = &fx.status {
            let target_self = fx.target == "self";
            if !target_self && them.status == Status::None && them.v.sub == 0 && self.sides[o & 1].safeguard == 0 {
                sc = match st.as_str() {
                    "slp" => 70.0,
                    "par" => {
                        if self.speed(o) > self.speed(s) {
                            62.0
                        } else {
                            45.0
                        }
                    }
                    "tox" => 50.0,
                    "brn" => {
                        if them.stats[1] > them.stats[3] {
                            55.0
                        } else {
                            30.0
                        }
                    }
                    _ => 35.0,
                } * acc;
                if (st == "par" && (calc::eff("Electric", &them.types, self.world) == 0.0 && id == "thunderwave"))
                    || (st == "brn" && them.is("Fire"))
                    || ((st == "psn" || st == "tox") && (them.is("Poison") || them.is("Steel")))
                {
                    sc = 0.0;
                }
            } else {
                sc = 0.0;
            }
        } else if fx.heal.is_some() || matches!(id, "roost" | "moonlight" | "morningsun" | "synthesis" | "rest" | "wish") {
            sc = if healthy < 0.35 {
                75.0
            } else if healthy < 0.55 {
                45.0
            } else {
                2.0
            };
            if id == "rest" && me.status == Status::None && healthy > 0.4 {
                sc = 4.0;
            }
        } else if let Some(b) = &fx.boosts {
            let selfb = fx.target == "self";
            let total: i32 = b.values().map(|x| *x as i32).sum();
            if selfb {
                let cur: i32 = b.keys().filter_map(|k| ["", "atk", "def", "spa", "spd", "spe", "accuracy", "evasion"].iter().position(|x| x == k)).map(|k| me.boosts[k] as i32).max().unwrap_or(0);
                sc = if healthy > 0.65 && cur < 2 { 30.0 + 8.0 * total as f32 } else { 3.0 };
                // don't set up into a KO
                if ai == Ai::Expert && self.threat_to(o, s) >= healthy {
                    sc = 1.0;
                }
            } else {
                sc = if them.hp_frac() > 0.5 { 18.0 } else { 6.0 };
            }
        } else if let Some(sc_name) = &fx.side_condition {
            let side = if fx.target == "foeSide" { o & 1 } else { s & 1 };
            let sd = &self.sides[side];
            sc = match sc_name.as_str() {
                "stealthrock" if !sd.rocks && self.sides[o & 1].left() > 1 => 40.0,
                "spikes" if sd.spikes < 3 && self.sides[o & 1].left() > 1 => 30.0,
                "toxicspikes" if sd.tspikes < 2 && self.sides[o & 1].left() > 1 => 25.0,
                "reflect" if sd.reflect == 0 => 28.0,
                "lightscreen" if sd.light_screen == 0 => 28.0,
                "safeguard" if sd.safeguard == 0 => 10.0,
                "tailwind" if sd.tailwind == 0 => 22.0,
                _ => 0.0,
            };
        } else if fx.weather.is_some() {
            sc = 12.0;
        } else if let Some(v) = &fx.volatile {
            sc = match v.as_str() {
                "confusion" if them.v.confusion == 0 => 35.0 * acc,
                "substitute" if me.v.sub == 0 && healthy > 0.5 => 25.0,
                "leechseed" if !them.v.seeded && !them.is("Grass") => 35.0,
                "protect" if me.v.protect_n == 0 => 10.0,
                "taunt" if them.v.taunt == 0 => 15.0,
                "focusenergy" if !me.v.focus_energy => 12.0,
                "attract" if !them.v.attract => 25.0,
                "yawn" if them.status == Status::None => 35.0,
                _ => 2.0,
            };
        } else if matches!(id, "painsplit") {
            sc = if healthy < them.hp_frac() - 0.2 { 45.0 } else { 0.0 };
        } else if matches!(id, "bellydrum") {
            sc = if healthy > 0.75 && me.boosts[1] < 2 { 40.0 } else { 0.0 };
        } else if matches!(id, "haze") {
            sc = them.boosts.iter().filter(|b| **b > 0).count() as f32 * 15.0;
        } else if matches!(id, "roar" | "whirlwind") {
            sc = if them.boosts.iter().any(|b| *b >= 2) { 50.0 } else { 5.0 };
        }
        if ai == Ai::Smart {
            sc *= 0.85;
        }
        sc
    }

    /// The most damage (share of max HP) `a` can do to `d` with one move.
    fn threat_to(&self, a: usize, d: usize) -> f32 {
        (0..self.act(a).moves.len()).filter_map(|i| self.estimate_vs(a, i, d)).map(|(_, hi)| hi).fold(0.0, f32::max)
    }

    /// How well `mon` (of the side of battler `s`) would face the foes on the field.
    fn matchup(&self, s: usize, mon: usize) -> f32 {
        let m = &self.sides[s & 1].mons[mon];
        if m.fainted() {
            return -999.0;
        }
        let mut foes = self.foes(s);
        if foes.is_empty() {
            foes.push(self.foe(s));
        }
        let mut total = 0.0;
        for &o in &foes {
            let them = self.act(o);
            // how hard they hit us: their types against ours
            let hurt = them.types.iter().map(|t| calc::eff(t, &m.types, self.world)).fold(0.0, f32::max);
            // how hard we hit them
            let hit = m.moves.iter().filter(|x| !x.status() && x.bp > 0).map(|x| calc::eff(&x.t, &them.types, self.world) * if m.types.contains(&x.t) { 1.5 } else { 1.0 }).fold(0.0, f32::max);
            total += hit * 2.0 - hurt * 2.0;
        }
        total / foes.len() as f32 + m.hp_frac() * 2.0
    }

    /// How good a move looks against the best target, and that target.
    fn best_use(&self, s: usize, i: usize, ai: Ai) -> (f32, usize) {
        let Some(mv) = self.act(s).moves.get(i) else { return (-1.0, self.foe(s)) };
        let foes = self.foes(s);
        if foes.is_empty() {
            return (self.score_move(s, i, self.foe(s), ai), self.foe(s));
        }
        let t = mv.fx.target.as_str();
        if !mv.status() && (t == "allAdjacentFoes" || t == "allAdjacent") && self.doubles() {
            // both foes (and the ally, for Earthquake and friends) at 0.75
            let mut sc: f32 = foes.iter().map(|&o| self.score_move(s, i, o, ai) * 0.75).sum();
            if t == "allAdjacent" {
                if let Some(a) = self.ally(s) {
                    let (_, hi) = self.estimate_vs(s, i, a).unwrap_or((0.0, 0.0));
                    let ahp = self.act(a).hp_frac();
                    sc -= hi.min(ahp) * 140.0 + if hi >= ahp && hi > 0.0 { 150.0 } else { 0.0 };
                }
            }
            return (sc, foes[0]);
        }
        let mut best = (f32::MIN, foes[0]);
        for &o in &foes {
            let sc = self.score_move(s, i, o, ai);
            if sc > best.0 {
                best = (sc, o);
            }
        }
        best
    }

    /// Who the AI sends out next.
    pub fn ai_replacement(&self, s: usize) -> usize {
        let side = &self.sides[s & 1];
        let opts: Vec<usize> = (0..side.mons.len()).filter(|&k| side.can_switch_to(k)).collect();
        if opts.is_empty() {
            return self.mi(s);
        }
        if side.ai == Ai::Random && s & 1 == 1 {
            return opts[0];
        }
        opts.iter().copied().max_by(|a, b| self.matchup(s, *a).partial_cmp(&self.matchup(s, *b)).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(opts[0])
    }

    /// What the AI does this turn for side `s`.
    pub fn ai_action(&mut self, s: usize) -> Action {
        self.ai_action_with(s, self.sides[s & 1].ai)
    }

    /// The same with a chosen AI (also used for "what would the AI do" for you).
    pub fn ai_action_with(&mut self, s: usize, ai: Ai) -> Action {
        if self.forced(s) {
            return Action::Auto;
        }
        let me = self.act(s).clone();
        let usable: Vec<usize> = self.usable(s).into_iter().filter(|(_, why)| why.is_none()).map(|(i, _)| i).collect();
        if usable.is_empty() {
            return Action::Move(0);
        }
        let doubles = self.doubles();
        if ai == Ai::Random {
            let i = usable[self.rng.below(usable.len() as u32) as usize];
            let foes = self.foes(s);
            if doubles && !foes.is_empty() {
                let t = foes[self.rng.below(foes.len() as u32) as usize];
                return Action::At(i, t);
            }
            return Action::Move(i);
        }
        // items (trainers): heal when low, like the games
        let here = self.mi(s);
        if s & 1 == 1 && !self.wild {
            let hpf = me.hp_frac();
            let items = self.sides[1].items.clone();
            let has = |n: &str| items.iter().any(|x| x.0 == n && x.1 > 0);
            if hpf < 0.25 && self.rng.chance(if ai == Ai::Expert { 0.9 } else { 0.6 }) {
                for it in ["Full Restore", "Max Potion", "Hyper Potion", "Super Potion", "Potion"] {
                    if has(it) {
                        return Action::Item(it.into(), here);
                    }
                }
            }
            if me.status != Status::None && me.status != Status::Poison && has("Full Heal") && self.rng.chance(0.5) {
                return Action::Item("Full Heal".into(), here);
            }
            if me.v.turns_out == 0 && hpf > 0.7 {
                for (it, k) in [("X Attack", 1), ("X Special", 3), ("X Speed", 5), ("X Defend", 2)] {
                    if has(it) && me.boosts[k] == 0 && self.rng.chance(0.4) {
                        return Action::Item(it.into(), here);
                    }
                }
            }
        }
        // Expert switches out of a bad spot
        if ai == Ai::Expert && self.trapped(s).is_none() {
            let foes = self.foes(s);
            let danger = foes.iter().any(|&f| self.threat_to(f, s) >= me.hp_frac() && self.speed(f) > self.speed(s));
            let best_hit = usable.iter().flat_map(|&i| foes.iter().filter_map(move |&f| Some((i, f)))).filter_map(|(i, f)| self.estimate_vs(s, i, f)).map(|(_, hi)| hi).fold(0.0, f32::max);
            let hopeless = best_hit < 0.15;
            if (danger || hopeless) && me.v.turns_out > 0 || hopeless && danger {
                let now = self.matchup(s, here);
                let k = self.ai_replacement(s);
                if k != here && self.matchup(s, k) > now + 1.5 && self.rng.chance(0.75) {
                    return Action::Switch(k);
                }
            }
        }
        let mut best = (usable[0], f32::MIN, self.foe(s));
        for &i in &usable {
            let noise = if ai == Ai::Expert { self.rng.below(6) as f32 } else { self.rng.below(16) as f32 };
            let (sc, t) = self.best_use(s, i, ai);
            if sc + noise > best.1 {
                best = (i, sc + noise, t);
            }
        }
        if doubles {
            Action::At(best.0, best.2)
        } else {
            Action::Move(best.0)
        }
    }
}

// ───────────────────────────── auto-sim ─────────────────────────────

/// How one Pokémon did over many battles.
#[derive(Clone, Debug, Default)]
pub struct MonReport {
    pub name: String,
    pub species: String,
    pub shiny: bool,
    /// Battles it got onto the field.
    pub fielded: u32,
    pub kos: u32,
    pub damage: u64,
    pub fainted: u32,
}

/// Many battles, added up.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub n: u32,
    pub won: u32,
    pub lost: u32,
    pub draw: u32,
    /// Ran, caught, fled.
    pub other: u32,
    /// Battles that hit the turn limit (counted as draws).
    pub capped: u32,
    /// How many turns each battle took.
    pub turns: Vec<u32>,
    pub ours: Vec<MonReport>,
    pub theirs: Vec<MonReport>,
    /// Your Pokémon left standing when you won, summed.
    pub left_standing: u32,
}

impl Report {
    fn add(&mut self, b: &Battle, capped: bool) {
        self.n += 1;
        match b.result {
            Some(Outcome::Won) => self.won += 1,
            Some(Outcome::Lost) => self.lost += 1,
            Some(Outcome::Draw) | None => self.draw += 1,
            _ => self.other += 1,
        }
        if capped {
            self.capped += 1;
        }
        if b.result == Some(Outcome::Won) {
            self.left_standing += b.sides[0].left() as u32;
        }
        self.turns.push(b.turn);
        for (side, out) in [(0usize, &mut self.ours), (1, &mut self.theirs)] {
            if out.is_empty() {
                *out = b.sides[side].mons.iter().map(|m| MonReport { name: m.name.clone(), species: m.species.clone(), shiny: m.shiny, ..Default::default() }).collect();
            }
            for (r, m) in out.iter_mut().zip(&b.sides[side].mons) {
                if m.seen {
                    r.fielded += 1;
                }
                r.kos += m.kos;
                r.damage += m.dealt as u64;
                if m.fainted() {
                    r.fainted += 1;
                }
            }
        }
    }

    fn merge(&mut self, o: Report) {
        if self.ours.is_empty() {
            self.ours = o.ours.clone();
            self.theirs = o.theirs.clone();
            for r in self.ours.iter_mut().chain(self.theirs.iter_mut()) {
                *r = MonReport { name: r.name.clone(), species: r.species.clone(), shiny: r.shiny, ..Default::default() };
            }
        }
        self.n += o.n;
        self.won += o.won;
        self.lost += o.lost;
        self.draw += o.draw;
        self.other += o.other;
        self.capped += o.capped;
        self.left_standing += o.left_standing;
        self.turns.extend(o.turns);
        for (a, b) in self.ours.iter_mut().zip(o.ours).chain(self.theirs.iter_mut().zip(o.theirs)) {
            a.fielded += b.fielded;
            a.kos += b.kos;
            a.damage += b.damage;
            a.fainted += b.fainted;
        }
    }

    /// Share won (0–1).
    pub fn win_rate(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.won as f64 / self.n as f64
        }
    }

    /// The 95% range for the true win rate (Wilson interval).
    pub fn win_range(&self) -> (f64, f64) {
        let n = self.n as f64;
        if n == 0.0 {
            return (0.0, 1.0);
        }
        let p = self.win_rate();
        let z = 1.96;
        let d = 1.0 + z * z / n;
        let c = (p + z * z / (2.0 * n)) / d;
        let h = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / d;
        ((c - h).max(0.0), (c + h).min(1.0))
    }

    pub fn avg_turns(&self) -> f64 {
        if self.turns.is_empty() {
            0.0
        } else {
            self.turns.iter().map(|t| *t as f64).sum::<f64>() / self.turns.len() as f64
        }
    }
}

/// Play one battle out with the AI on both sides. `template` is a battle
/// that hasn't started yet.
pub fn auto_battle(template: &Battle, our_ai: Ai, seed: u64) -> (Battle, bool) {
    let mut b = template.clone();
    b.rng = Rng::new(seed);
    b.sides[0].ai = our_ai;
    b.start("");
    let mut steps = 0;
    while b.result.is_none() && steps < 400 {
        steps += 1;
        if b.ask.is_some() {
            let slot = match b.ask {
                Some(Ask::Switch { slot, .. }) => slot,
                None => 0,
            };
            let k = b.ai_replacement(slot * 2);
            b.answer(k);
            continue;
        }
        let mut acts = Vec::new();
        for x in b.battlers() {
            if b.present(x) {
                let a = if x & 1 == 0 { b.ai_action_with(x, our_ai) } else { b.ai_action(x) };
                // the auto-sim fights: no running, no balls
                let a = match a {
                    Action::Run | Action::Ball(_) => b.ai_action_with(x, Ai::Smart),
                    a => a,
                };
                acts.push((x, a));
            }
        }
        b.play_all(acts);
    }
    let capped = b.result.is_none();
    (b, capped)
}

/// Many battles, split over the CPU's cores. `progress` counts battles
/// done; set `cancel` to stop early.
pub fn simulate(template: &Battle, n: u32, our_ai: Ai, seed: u64, progress: &std::sync::atomic::AtomicU32, cancel: &std::sync::atomic::AtomicBool) -> Report {
    use std::sync::atomic::Ordering;
    let workers = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(2).clamp(1, 8) as u32;
    let mut total = Report::default();
    std::thread::scope(|sc| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                let template = template;
                sc.spawn(move || {
                    let mut r = Report::default();
                    let mut k = w;
                    while k < n {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let (b, capped) = auto_battle(template, our_ai, seed.wrapping_add(k as u64 * 7919 + 1));
                        r.add(&b, capped);
                        progress.fetch_add(1, Ordering::Relaxed);
                        k += workers;
                    }
                    r
                })
            })
            .collect();
        for h in handles {
            if let Ok(r) = h.join() {
                total.merge(r);
            }
        }
    });
    total
}

// ───────────────────────────── tests ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn wiki() -> &'static Wiki {
        static W: std::sync::OnceLock<Wiki> = std::sync::OnceLock::new();
        W.get_or_init(|| Wiki::parse(include_bytes!("../assets/wiki.json")).expect("wiki"))
    }

    fn mon(species: &str, lv: u8, moves: &[&str]) -> BMon {
        let mut m = Member::new(species, World::Platinum);
        m.level = lv;
        m.moves = moves.iter().map(|s| s.to_string()).collect();
        m.ivs = [31; 6];
        m.nature = "Hardy".into();
        BMon::from_member(&m, World::Platinum, Some(wiki())).expect("mon")
    }

    #[test]
    fn a_wild_battle_ends() {
        for seed in 1..40 {
            let mut b = Battle::new(vec![mon("Garchomp", 50, &["Dragon Claw", "Earthquake", "Swords Dance", "Fire Fang"])], vec![mon("Bidoof", 5, &["Tackle", "Growl"])], true, World::Platinum, seed);
            b.start("A wild Bidoof appeared!");
            let mut n = 0;
            while b.result.is_none() && n < 50 {
                let a = b.ai_action_with(0, Ai::Smart);
                let t = b.ai_action_with(1, Ai::Random);
                b.play(a, t);
                n += 1;
            }
            assert_eq!(b.result, Some(Outcome::Won), "seed {seed}: {:?}", b.log.iter().rev().take(5).collect::<Vec<_>>());
        }
    }

    #[test]
    fn trainer_battles_finish() {
        let team = || vec![mon("Infernape", 50, &["Flamethrower", "Close Combat", "U-turn", "Nasty Plot"]), mon("Swampert", 50, &["Surf", "Earthquake", "Stealth Rock", "Ice Beam"]), mon("Togekiss", 50, &["Air Slash", "Thunder Wave", "Roost", "Aura Sphere"])];
        let foes = || vec![mon("Gengar", 50, &["Shadow Ball", "Hypnosis", "Focus Blast", "Thunderbolt"]), mon("Scizor", 50, &["Bullet Punch", "X-Scissor", "Swords Dance", "Roost"]), mon("Gyarados", 50, &["Waterfall", "Dragon Dance", "Earthquake", "Protect"])];
        let mut ends = [0; 6];
        for seed in 1..60 {
            let mut b = Battle::new(team(), foes(), false, World::Platinum, seed);
            b.sides[1].ai = Ai::Expert;
            b.sides[1].items = vec![("Hyper Potion".into(), 2)];
            b.start("Trainer battle");
            let mut n = 0;
            while b.result.is_none() && n < 300 {
                if b.ask.is_some() {
                    let k = b.ai_replacement(0);
                    b.answer(k);
                    n += 1;
                    continue;
                }
                let a = b.ai_action_with(0, Ai::Expert);
                let t = b.ai_action(1);
                b.play(a, t);
                n += 1;
            }
            let r = b.result.unwrap_or_else(|| panic!("seed {seed} never ended: {:?}", b.log.iter().rev().take(12).collect::<Vec<_>>()));
            ends[r as usize] += 1;
            // HP never above max, never a fainted one active at the end of a turn unless over
            for s in 0..2 {
                for m in &b.sides[s].mons {
                    assert!(m.hp <= m.max());
                }
            }
        }
        assert!(ends[Outcome::Won as usize] > 0 && ends[Outcome::Lost as usize] > 0, "{ends:?}");
    }

    #[test]
    fn catching_and_running() {
        let mut caught = 0;
        for seed in 1..200 {
            let mut b = Battle::new(vec![mon("Garchomp", 50, &["Dragon Claw"])], vec![mon("Bidoof", 3, &["Tackle"])], true, World::Platinum, seed);
            b.start("hi");
            b.play(Action::Ball(Ball::Ultra), Action::Move(0));
            if b.result == Some(Outcome::Caught) {
                caught += 1;
            }
        }
        // Bidoof: catch rate 255, so most Ultra Balls work
        assert!(caught > 150, "{caught}");
        let mut b = Battle::new(vec![mon("Garchomp", 50, &["Dragon Claw"])], vec![mon("Bidoof", 3, &["Tackle"])], true, World::Platinum, 3);
        b.start("hi");
        b.play(Action::Run, Action::Move(0));
        assert_eq!(b.result, Some(Outcome::Ran));
    }

    #[test]
    fn u_turn_asks_who_goes_in() {
        let mut b = Battle::new(vec![mon("Infernape", 60, &["U-turn"]), mon("Swampert", 50, &["Surf"])], vec![mon("Snorlax", 60, &["Splash"]), mon("Bidoof", 5, &["Tackle"])], false, World::Platinum, 5);
        b.start("t");
        b.play(Action::Move(0), Action::Move(0));
        assert_eq!(b.ask, Some(Ask::Switch { slot: 0, baton: false }));
        b.answer(1);
        assert_eq!(b.sides[0].active(), 1);
        assert!(b.ask.is_none());
    }

    #[test]
    fn status_and_weather() {
        // Thunder Wave paralyses; Sandstorm hurts non-Rock types
        let mut b = Battle::new(vec![mon("Pikachu", 30, &["Thunder Wave", "Sandstorm"])], vec![mon("Snorlax", 30, &["Splash"])], true, World::Platinum, 9);
        b.start("t");
        b.play(Action::Move(0), Action::Move(0));
        assert_eq!(b.act(1).status, Status::Paralysis);
        let before = b.act(1).hp;
        b.play(Action::Move(1), Action::Move(0));
        assert_eq!(b.weather, Weather::Sand);
        assert!(b.act(1).hp < before);
        // Thunder Wave can't hit a Ground type
        let mut b = Battle::new(vec![mon("Pikachu", 30, &["Thunder Wave"])], vec![mon("Gible", 30, &["Splash"])], true, World::Platinum, 9);
        b.start("t");
        b.play(Action::Move(0), Action::Move(0));
        assert_eq!(b.act(1).status, Status::None);
    }
}

#[cfg(test)]
mod dump {
    use super::*;
    #[test]
    #[ignore]
    fn print_tricky_moves() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let mk = |sp: &str, mv: &[&str], item: &str| {
            let mut m = Member::new(sp, World::Platinum);
            m.level = 50;
            m.item = item.into();
            m.moves = mv.iter().map(|s| s.to_string()).collect();
            BMon::from_member(&m, World::Platinum, Some(w)).unwrap()
        };
        let mut b = Battle::new(
            vec![mk("Ninjask", &["Swords Dance", "Substitute", "Protect", "Baton Pass"], "Leftovers"), mk("Skarmory", &["Spikes", "Roar", "Brave Bird", "Roost"], "Leftovers"), mk("Jirachi", &["Wish", "Iron Head", "Toxic", "Metronome"], "Leftovers")],
            vec![mk("Forretress", &["Stealth Rock", "Toxic Spikes", "Rapid Spin", "Explosion"], "Leftovers"), mk("Gengar", &["Hypnosis", "Shadow Ball", "Perish Song", "Destiny Bond"], "Life Orb"), mk("Clefable", &["Metronome", "Leech Seed", "Thunder Wave", "Encore"], "Leftovers")],
            false,
            World::Platinum,
            5,
        )
        .with_wiki(w);
        b.sides[1].name = "Veteran Ann".into();
        b.sides[1].ai = Ai::Expert;
        b.start("Veteran Ann would like to battle!");
        let script = [Action::Move(0), Action::Move(1), Action::Move(2), Action::Move(3), Action::Move(0), Action::Move(1), Action::Move(3), Action::Move(2), Action::Move(3), Action::Move(1), Action::Move(0), Action::Move(2)];
        for a in script.iter().cycle().take(30) {
            if b.result.is_some() {
                break;
            }
            if b.ask.is_some() {
                let k = b.ai_replacement(0);
                b.answer(k);
                continue;
            }
            let usable = b.usable(0);
            let a = match a {
                Action::Move(i) if usable.get(*i).is_some_and(|x| x.1.is_none()) => a.clone(),
                _ => b.ai_action_with(0, Ai::Smart),
            };
            let t = b.ai_action(1);
            b.play(a, t);
        }
        for e in &b.log {
            match e {
                Ev::Text(t) => println!("  {t}"),
                Ev::Turn(t) => println!("— Turn {t} —"),
                _ => {}
            }
        }
    }

    #[test]
    #[ignore]
    fn print_a_battle() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let mk = |sp: &str, lv: u8, mv: &[&str], item: &str| {
            let mut m = Member::new(sp, World::Platinum);
            m.level = lv;
            m.item = item.into();
            m.moves = mv.iter().map(|s| s.to_string()).collect();
            BMon::from_member(&m, World::Platinum, Some(w)).unwrap()
        };
        let mut b = Battle::new(
            vec![mk("Infernape", 50, &["Flamethrower", "Close Combat", "U-turn", "Will-O-Wisp"], "Life Orb"), mk("Swampert", 50, &["Surf", "Earthquake", "Stealth Rock", "Toxic"], "Leftovers")],
            vec![mk("Gengar", 50, &["Shadow Ball", "Hypnosis", "Focus Blast", "Substitute"], "Focus Sash"), mk("Gyarados", 50, &["Waterfall", "Dragon Dance", "Earthquake", "Thunder Wave"], "Sitrus Berry")],
            false,
            World::Platinum,
            11,
        )
        .with_wiki(w);
        b.sides[1].name = "Ace Trainer Kim".into();
        b.sides[1].ai = Ai::Expert;
        b.start("Ace Trainer Kim would like to battle!");
        let mut n = 0;
        while b.result.is_none() && n < 60 {
            if b.ask.is_some() {
                let k = b.ai_replacement(0);
                b.answer(k);
            } else {
                let a = b.ai_action_with(0, Ai::Expert);
                let t = b.ai_action(1);
                b.play(a, t);
            }
            n += 1;
        }
        for e in &b.log {
            match e {
                Ev::Text(t) => println!("  {t}"),
                Ev::Turn(t) => println!("— Turn {t} —"),
                Ev::Hp { side, hp, max } => println!("      [hp {side}: {hp}/{max}]"),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod fuzz {
    use super::*;

    /// Many battles between random Smogon sets, with random choices mixed
    /// in: nothing panics, every battle ends, HP stays in bounds.
    #[test]
    fn random_battles() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let d = battle::data();
        let sets: Vec<_> = d.sets.iter().filter(|s| d.species(&s.s).is_some()).collect();
        assert!(sets.len() > 100);
        let mut rng = Rng::new(42);
        let mut pick = |rng: &mut Rng| {
            let s = sets[rng.below(sets.len() as u32) as usize];
            let mut m = Member::new(&s.s, World::Platinum);
            m.level = 50;
            m.item = s.item.clone();
            m.ability = s.ab.clone();
            m.nature = s.nat.clone();
            m.evs = s.ev;
            m.ivs = s.iv;
            m.moves = s.m.clone();
            BMon::from_member(&m, World::Platinum, Some(w))
        };
        let mut ends = std::collections::HashMap::new();
        for game in 0..300u64 {
            let ours: Vec<BMon> = (0..3).filter_map(|_| pick(&mut rng)).collect();
            let theirs: Vec<BMon> = (0..3).filter_map(|_| pick(&mut rng)).collect();
            if ours.is_empty() || theirs.is_empty() {
                continue;
            }
            let wild = game % 5 == 0;
            let theirs = if wild { theirs.into_iter().take(1).collect() } else { theirs };
            let mut b = Battle::new(ours, theirs, wild, World::Platinum, game + 1).with_wiki(w);
            b.sides[1].ai = [Ai::Random, Ai::Smart, Ai::Expert][(game % 3) as usize];
            b.sides[1].items = vec![("Full Restore".into(), 1), ("X Speed".into(), 1)];
            b.start("go");
            let mut n = 0;
            while b.result.is_none() && n < 600 {
                n += 1;
                if b.ask.is_some() {
                    let opts: Vec<usize> = (0..b.sides[0].mons.len()).filter(|&k| b.sides[0].can_switch_to(k)).collect();
                    b.answer(opts[rng.below(opts.len() as u32) as usize]);
                    continue;
                }
                let r = rng.below(20);
                let mine = if r == 0 && !b.forced(0) {
                    let opts: Vec<usize> = (0..b.sides[0].mons.len()).filter(|&k| b.sides[0].can_switch_to(k)).collect();
                    if opts.is_empty() || b.trapped(0).is_some() {
                        b.ai_action_with(0, Ai::Smart)
                    } else {
                        Action::Switch(opts[0])
                    }
                } else if r == 1 && wild {
                    Action::Ball(Ball::Great)
                } else if r == 2 {
                    Action::Item("Hyper Potion".into(), b.sides[0].active())
                } else {
                    b.ai_action_with(0, if r < 10 { Ai::Random } else { Ai::Expert })
                };
                let theirs = b.ai_action(1);
                b.play(mine, theirs);
                for s in 0..2 {
                    for m in &b.sides[s].mons {
                        assert!(m.hp <= m.max(), "game {game}: {} over max", m.name);
                        assert!(m.boosts.iter().all(|x| (-6..=6).contains(x)));
                    }
                }
                if b.result.is_none() && b.ask.is_none() {
                    // a fainted Pokémon is never left in front between turns
                    for s in 0..2 {
                        if b.sides[s].left() > 0 {
                            assert!(!b.act(s).fainted(), "game {game}: side {s} left with a fainted one in: {:?}", b.log.iter().rev().take(10).collect::<Vec<_>>());
                        }
                    }
                }
            }
            *ends.entry(format!("{:?}", b.result)).or_insert(0) += 1;
        }
        eprintln!("{ends:?}");
        assert!(ends.get("None").copied().unwrap_or(0) <= 3, "{ends:?}");
    }
}

#[cfg(test)]
mod doubles {
    use super::*;

    fn w() -> &'static Wiki {
        static W: std::sync::OnceLock<Wiki> = std::sync::OnceLock::new();
        W.get_or_init(|| Wiki::parse(include_bytes!("../assets/wiki.json")).expect("wiki"))
    }
    fn mon(sp: &str, lv: u8, mv: &[&str]) -> BMon {
        let mut m = Member::new(sp, World::Platinum);
        m.level = lv;
        m.moves = mv.iter().map(|s| s.to_string()).collect();
        BMon::from_member(&m, World::Platinum, Some(w())).unwrap()
    }

    #[test]
    fn spread_moves_hit_both_and_the_ally() {
        let mut b = Battle::new_format(
            vec![mon("Garchomp", 60, &["Earthquake"]), mon("Snorlax", 60, &["Splash"])],
            vec![mon("Snorlax", 60, &["Splash"]), mon("Snorlax", 60, &["Splash"])],
            false,
            World::Platinum,
            7,
            true,
        );
        b.start("t");
        assert!(b.doubles());
        assert_eq!(b.battlers(), vec![0, 1, 2, 3]);
        let hp: Vec<u16> = (0..4).map(|x| b.act(x).hp).collect();
        b.play_all(vec![(0, Action::Move(0)), (2, Action::Move(0)), (1, Action::Move(0)), (3, Action::Move(0))]);
        assert!(b.act(1).hp < hp[1] && b.act(3).hp < hp[3], "both foes hit");
        assert!(b.act(2).hp < hp[2], "the ally is hit by Earthquake");
        // a Flying ally isn't
        let mut b = Battle::new_format(
            vec![mon("Garchomp", 60, &["Earthquake"]), mon("Staraptor", 60, &["Splash"])],
            vec![mon("Snorlax", 60, &["Splash"]), mon("Snorlax", 60, &["Splash"])],
            false,
            World::Platinum,
            7,
            true,
        );
        b.start("t");
        let ally = b.act(2).hp;
        b.play_all(vec![(0, Action::Move(0)), (2, Action::Move(0)), (1, Action::Move(0)), (3, Action::Move(0))]);
        assert_eq!(b.act(2).hp, ally);
    }

    #[test]
    fn a_chosen_target_is_the_one_hit() {
        let mut b = Battle::new_format(
            vec![mon("Garchomp", 60, &["Dragon Claw"]), mon("Snorlax", 60, &["Splash"])],
            vec![mon("Snorlax", 60, &["Splash"]), mon("Blissey", 60, &["Splash"])],
            false,
            World::Platinum,
            3,
            true,
        );
        b.start("t");
        let (a, c) = (b.act(1).hp, b.act(3).hp);
        b.play_all(vec![(0, Action::At(0, 3)), (2, Action::Move(0)), (1, Action::Move(0)), (3, Action::Move(0))]);
        assert_eq!(b.act(1).hp, a);
        assert!(b.act(3).hp < c || b.tally[0].misses > 0);
    }

    #[test]
    fn wild_doubles_no_ball_with_two() {
        let mut b = Battle::new_format(vec![mon("Garchomp", 60, &["Dragon Claw"]), mon("Snorlax", 60, &["Splash"])], vec![mon("Bidoof", 5, &["Tackle"]), mon("Starly", 5, &["Tackle"])], true, World::Platinum, 3, true);
        b.start("t");
        b.play_all(vec![(0, Action::Ball(Ball::Master)), (2, Action::Move(0)), (1, Action::Move(0)), (3, Action::Move(0))]);
        assert!(b.result != Some(Outcome::Caught));
        assert!(b.log.iter().any(|e| matches!(e, Ev::Text(t) if t.contains("impossible to aim"))));
    }

    /// Random doubles: nothing panics, every battle ends, fainted places
    /// get filled when there's someone left to send.
    #[test]
    fn random_doubles() {
        let d = battle::data();
        let sets: Vec<_> = d.sets.iter().filter(|s| d.species(&s.s).is_some()).collect();
        let mut rng = Rng::new(99);
        let pick = |rng: &mut Rng| {
            let s = sets[rng.below(sets.len() as u32) as usize];
            let mut m = Member::new(&s.s, World::Platinum);
            m.level = 50;
            m.item = s.item.clone();
            m.ability = s.ab.clone();
            m.nature = s.nat.clone();
            m.evs = s.ev;
            m.ivs = s.iv;
            m.moves = s.m.clone();
            BMon::from_member(&m, World::Platinum, Some(w()))
        };
        let mut ends = std::collections::HashMap::new();
        for game in 0..150u64 {
            let n_ours = 2 + (game % 3) as usize;
            let ours: Vec<BMon> = (0..n_ours).filter_map(|_| pick(&mut rng)).collect();
            let theirs: Vec<BMon> = (0..4).filter_map(|_| pick(&mut rng)).collect();
            let wild = game % 6 == 0;
            let theirs: Vec<BMon> = if wild { theirs.into_iter().take(2).collect() } else { theirs };
            let mut b = Battle::new_format(ours, theirs, wild, World::Platinum, game + 1, true).with_wiki(w());
            b.sides[1].ai = [Ai::Random, Ai::Smart, Ai::Expert][(game % 3) as usize];
            b.start("go");
            let mut n = 0;
            while b.result.is_none() && n < 800 {
                n += 1;
                if let Some(Ask::Switch { .. }) = b.ask.clone() {
                    let opts: Vec<usize> = (0..b.sides[0].mons.len()).filter(|&k| b.sides[0].can_switch_to(k)).collect();
                    b.answer(opts[rng.below(opts.len() as u32) as usize]);
                    continue;
                }
                let mut acts = Vec::new();
                for x in b.battlers() {
                    if !b.present(x) {
                        continue;
                    }
                    let a = if x & 1 == 0 { b.ai_action_with(x, if rng.below(2) == 0 { Ai::Random } else { Ai::Expert }) } else { b.ai_action(x) };
                    acts.push((x, a));
                }
                b.play_all(acts);
                for s in 0..2 {
                    for m in &b.sides[s].mons {
                        assert!(m.hp <= m.max());
                    }
                    let mut sl = b.sides[s].slots.clone();
                    sl.sort();
                    sl.dedup();
                    assert_eq!(sl.len(), b.sides[s].slots.len(), "game {game}: one Pokémon in two places");
                }
                if b.result.is_none() && b.ask.is_none() {
                    for x in b.battlers() {
                        let side = x & 1;
                        let bench = (0..b.sides[side].mons.len()).any(|k| b.sides[side].can_switch_to(k));
                        assert!(!(b.act(x).fainted() && bench), "game {game}: battler {x} left fainted with someone on the bench");
                    }
                }
            }
            *ends.entry(format!("{:?}", b.result)).or_insert(0) += 1;
        }
        eprintln!("{ends:?}");
        assert!(ends.get("None").copied().unwrap_or(0) <= 2, "{ends:?}");
    }
}

#[cfg(test)]
mod doubles_dump {
    use super::*;
    #[test]
    #[ignore]
    fn print_doubles() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let mk = |sp: &str, mv: &[&str], item: &str| {
            let mut m = Member::new(sp, World::Platinum);
            m.level = 50;
            m.item = item.into();
            m.moves = mv.iter().map(|s| s.to_string()).collect();
            BMon::from_member(&m, World::Platinum, Some(w)).unwrap()
        };
        let mut b = Battle::new_format(
            vec![mk("Garchomp", &["Earthquake", "Dragon Claw", "Protect", "Fire Fang"], "Life Orb"), mk("Gyarados", &["Waterfall", "Dragon Dance", "Ice Fang", "Protect"], "Leftovers"), mk("Togekiss", &["Air Slash", "Thunder Wave", "Roost", "Tailwind"], "Leftovers")],
            vec![mk("Tyranitar", &["Rock Slide", "Crunch", "Protect", "Earthquake"], "Choice Band"), mk("Metagross", &["Meteor Mash", "Explosion", "Bullet Punch", "Protect"], "Life Orb"), mk("Salamence", &["Draco Meteor", "Flamethrower", "Hydro Pump", "Protect"], "Life Orb")],
            false,
            World::Platinum,
            21,
            true,
        )
        .with_wiki(w);
        b.sides[1].name = "Cool Couple Ana & Bo".into();
        b.sides[1].ai = Ai::Expert;
        b.start("Cool Couple Ana & Bo would like to battle!");
        let mut n = 0;
        while b.result.is_none() && n < 40 {
            n += 1;
            if b.ask.is_some() {
                let k = b.ai_replacement(0);
                b.answer(k);
                continue;
            }
            let mut acts = Vec::new();
            for x in b.battlers() {
                if b.present(x) {
                    let a = if x & 1 == 0 { b.ai_action_with(x, Ai::Expert) } else { b.ai_action(x) };
                    acts.push((x, a));
                }
            }
            b.play_all(acts);
        }
        for e in &b.log {
            match e {
                Ev::Text(t) => println!("  {t}"),
                Ev::Turn(t) => println!("— Turn {t} —"),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod boom {
    use super::*;
    #[test]
    #[ignore]
    fn explosion_in_doubles() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let mk = |sp: &str, mv: &[&str]| {
            let mut m = Member::new(sp, World::Platinum);
            m.level = 50;
            m.moves = mv.iter().map(|s| s.to_string()).collect();
            BMon::from_member(&m, World::Platinum, Some(w)).unwrap()
        };
        let mut b = Battle::new_format(vec![mk("Snorlax", &["Splash"]), mk("Snorlax", &["Splash"])], vec![mk("Metagross", &["Explosion"]), mk("Snorlax", &["Splash"])], false, World::Platinum, 2, true);
        b.start("x");
        println!("{:?}", b.act(1).moves[0].fx);
        b.play_all(vec![(0, Action::Move(0)), (2, Action::Move(0)), (1, Action::Move(0)), (3, Action::Move(0))]);
        for e in &b.log { println!("{e:?}"); }
    }
}

#[cfg(test)]
mod autosim {
    use super::*;
    #[test]
    fn many_battles_add_up() {
        let w = Box::leak(Box::new(Wiki::parse(include_bytes!("../assets/wiki.json")).unwrap()));
        let mk = |sp: &str, lv: u8, mv: &[&str]| {
            let mut m = Member::new(sp, World::Platinum);
            m.level = lv;
            m.moves = mv.iter().map(|s| s.to_string()).collect();
            BMon::from_member(&m, World::Platinum, Some(w)).unwrap()
        };
        // a strong team against a weak one wins nearly always
        let t = Battle::new(vec![mk("Garchomp", 60, &["Dragon Claw", "Earthquake"])], vec![mk("Bidoof", 10, &["Tackle"]), mk("Starly", 10, &["Tackle"])], false, World::Platinum, 1);
        let p = std::sync::atomic::AtomicU32::new(0);
        let c = std::sync::atomic::AtomicBool::new(false);
        let r = simulate(&t, 60, Ai::Expert, 5, &p, &c);
        assert_eq!(r.n, 60);
        assert_eq!(p.load(std::sync::atomic::Ordering::Relaxed), 60);
        assert!(r.win_rate() > 0.95, "{r:?}");
        assert_eq!(r.turns.len(), 60);
        assert!(r.ours[0].kos >= 110);
        let (lo, hi) = r.win_range();
        assert!(lo < r.win_rate() && r.win_rate() <= hi + 1e-9);
        // doubles too
        let t = Battle::new_format(vec![mk("Garchomp", 50, &["Earthquake"]), mk("Staraptor", 50, &["Brave Bird"])], vec![mk("Infernape", 50, &["Close Combat"]), mk("Lucario", 50, &["Aura Sphere"])], false, World::Platinum, 1, true);
        let r = simulate(&t, 40, Ai::Expert, 9, &p, &c);
        assert_eq!(r.n, 40);
        assert_eq!(r.won + r.lost + r.draw + r.other, 40);
    }
}
