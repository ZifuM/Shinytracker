//! The Breeding Planner, filled in from your save (read only; the save is
//! never written). Turns each Pokémon the save keeps into a Day Care parent
//! the planner understands, counts the breeding items you own, and works
//! out the settings (charms, Egg Warmer, Flame Body) from your bag and party.
//!
//! Eggs are left out of the breeders: what's inside an Egg stays hidden
//! until it hatches (see the planner's spoiler setting).
#![allow(dead_code)] // the planner's screens use it from stage 2

use crate::battle::{self, World};
use crate::breed::{self, Gender, Held, Parent, Setup, Slot};
use crate::gamesave::{GameText, Mon, Save, NATURES};
use crate::wiki::Wiki;

/// Where a Pokémon is in your save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Place {
    DayCare(usize),
    Party(usize),
    /// (box, slot), from 0.
    Box(usize, usize),
}

impl Place {
    pub fn label(&self, save: &Save) -> String {
        match *self {
            Place::DayCare(_) => "Day Care".into(),
            Place::Party(i) => format!("Party · slot {}", i + 1),
            Place::Box(b, s) => {
                let name = save.boxes.get(b).map(|x| x.name.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Box {}", b + 1));
                format!("{name} · slot {}", s + 1)
            }
        }
    }
}

/// One of your Pokémon, ready to breed with.
#[derive(Debug, Clone)]
pub struct Owned {
    pub place: Place,
    /// The Pokémon's own number (it keeps it from Egg to hatched).
    pub pid: u32,
    pub parent: Parent,
    /// Its nickname, or "" when it has none.
    pub nickname: String,
    pub level: Option<u8>,
    pub shiny: bool,
    /// Its held item's name ("" for nothing).
    pub item: String,
    /// Its Ability's name.
    pub ability: String,
}

impl Owned {
    /// Its nickname, else its species' name.
    pub fn name(&self) -> String {
        if !self.nickname.is_empty() {
            return self.nickname.clone();
        }
        battle::data().species(&self.parent.species).map(|s| s.name.clone()).unwrap_or_else(|| self.parent.species.clone())
    }
}

/// The wiki's species id ("EEVEE") for the game's species number.
pub fn species_id(game: u16) -> Option<String> {
    let nat = crate::species::national_from_game(game as u32)?;
    let d = battle::data();
    let sp = d.species.iter().find(|s| s.num == nat && s.base.is_empty())?;
    let id = if !sp.wiki.is_empty() { sp.wiki.clone() } else { return None };
    breed::info(&id)?;
    Some(id)
}

/// The save keeps IVs as HP, Atk, Def, Spe, SpA, SpD; the planner as HP,
/// Atk, Def, SpA, SpD, Spe.
pub fn ivs_of(m: &Mon) -> [Option<u8>; 6] {
    let v = m.ivs;
    [v[0], v[1], v[2], v[4], v[5], v[3]].map(|x| Some(x.min(31)))
}

fn same(a: &str, b: &str) -> bool {
    let k = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    k(a) == k(b)
}

/// Which of its species' Abilities `name` is.
pub fn slot_of(id: &str, name: &str, world: World) -> Slot {
    let normal = breed::normal_abilities(id, world);
    if let Some(i) = normal.iter().position(|a| same(a, name)) {
        return if i == 1 { Slot::Second } else { Slot::First };
    }
    if breed::hidden_ability(id).is_some_and(|h| same(&h, name)) {
        return Slot::Hidden;
    }
    // the other world's list (a Pokémon from a world with other Abilities)
    let other = breed::normal_abilities(id, if world == World::Modern { World::Platinum } else { World::Modern });
    match other.iter().position(|a| same(a, name)) {
        Some(1) => Slot::Second,
        _ => Slot::First,
    }
}

/// A Pokémon from the save as a Day Care parent. None for an Egg, or a
/// species the planner doesn't know.
pub fn to_parent(m: &Mon, save: &Save, text: &GameText, wiki: Option<&Wiki>, world: World) -> Option<Parent> {
    to_parent_with(m, save, text, wiki.map(MoveKeys::new).as_ref(), world)
}

/// The wiki's move keys by name, looked up once for many Pokémon.
pub struct MoveKeys(std::collections::HashMap<String, String>);

impl MoveKeys {
    pub fn new(w: &Wiki) -> MoveKeys {
        MoveKeys(w.data.moves.iter().map(|(k, m)| (crate::wiki::slug(&m.n), k.clone())).collect())
    }
    pub fn key(&self, name: &str) -> Option<String> {
        self.0.get(&crate::wiki::slug(name)).cloned()
    }
}

fn to_parent_with(m: &Mon, save: &Save, text: &GameText, keys: Option<&MoveKeys>, world: World) -> Option<Parent> {
    if m.egg {
        return None;
    }
    let id = species_id(m.species)?;
    let gender = if m.genderless {
        Gender::Genderless
    } else if m.female {
        Gender::Female
    } else {
        Gender::Male
    };
    let mut p = Parent::new(&id, gender);
    p.gender = p.gender();
    p.nature = NATURES.get(m.nature() as usize).map(|s| s.to_string());
    p.ivs = ivs_of(m);
    p.ability = slot_of(&id, &text.ability_name(m.ability), world);
    p.held = if m.item == 0 { Held::Nothing } else { Held::from_name(&text.item_name(m.item)) };
    p.ball = if m.ball == 0 { "Poké Ball".into() } else { text.item_name(m.ball as u16) };
    if let Some(k) = keys {
        p.moves = m.moves.iter().filter(|mv| **mv != 0).filter_map(|mv| k.key(&text.move_name(*mv))).collect();
    }
    // Another trainer's (a trade): the Day Care notices. Masuda counts a
    // trade from another player; your own from your other worlds keep your
    // name, so they don't.
    let ot_differs = !m.ot.trim().is_empty() && !save.trainer.trim().is_empty() && !same(&m.ot, &save.trainer);
    p.other_trainer = ot_differs || m.tid != save.tid;
    p.masuda = ot_differs;
    Some(p)
}

/// Every Pokémon you can breed with: the Day Care pair, then the party,
/// then the boxes (Eggs left out).
pub fn owned(save: &Save, text: &GameText, wiki: Option<&Wiki>, world: World) -> Vec<Owned> {
    let keys = wiki.map(MoveKeys::new);
    let mut out = Vec::new();
    let mut add = |place: Place, m: &Mon| {
        if let Some(parent) = to_parent_with(m, save, text, keys.as_ref(), world) {
            let item = if m.item == 0 { String::new() } else { text.item_name(m.item) };
            out.push(Owned { place, pid: m.pid, parent, nickname: if m.nicknamed { m.nickname.clone() } else { String::new() }, level: m.level, shiny: m.shiny(), item, ability: text.ability_name(m.ability) });
        }
    };
    for (i, m) in save.daycare.iter().enumerate() {
        add(Place::DayCare(i), m);
    }
    for (i, m) in save.party.iter().enumerate() {
        add(Place::Party(i), m);
    }
    for (b, bx) in save.boxes.iter().enumerate() {
        for (s, m) in bx.slots.iter().enumerate() {
            if let Some(m) = m {
                add(Place::Box(b, s), m);
            }
        }
    }
    out
}

/// Items that matter for breeding.
pub fn is_breeding_item(name: &str) -> bool {
    matches!(Held::from_name(name), Held::Everstone | Held::DestinyKnot | Held::RiftKnot | Held::Power(_) | Held::Incense(_))
}

/// The breeding items you own: in the bag, plus held by your Pokémon
/// (name, how many, how many of those are held).
pub fn breeding_items(save: &Save, text: &GameText) -> Vec<(String, u32, u32)> {
    let mut v: Vec<(String, u32, u32)> = Vec::new();
    let mut add = |name: String, n: u32, held: bool| {
        if !is_breeding_item(&name) {
            return;
        }
        match v.iter_mut().find(|x| x.0 == name) {
            Some(x) => {
                x.1 += n;
                x.2 += if held { n } else { 0 };
            }
            None => v.push((name, n, if held { n } else { 0 })),
        }
    };
    for (_, item, qty) in &save.bag {
        add(text.item_name(*item), *qty as u32, false);
    }
    let mons = save.daycare.iter().chain(save.party.iter()).chain(save.boxes.iter().flat_map(|b| b.slots.iter().flatten()));
    for m in mons.filter(|m| m.item != 0 && !m.egg) {
        add(text.item_name(m.item), 1, true);
    }
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// What your bag and party say about the settings.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Found {
    pub shiny_charm: bool,
    pub oval_charm: bool,
    pub egg_warmer: bool,
    /// Flame Body or Magma Armor in the party (not an Egg).
    pub flame_body: bool,
}

pub fn found(save: &Save, text: &GameText) -> Found {
    let has = |n: &str| save.bag.iter().any(|b| b.2 > 0 && same(&text.item_name(b.1), n));
    Found {
        shiny_charm: has("Shiny Charm"),
        oval_charm: has("Oval Charm"),
        egg_warmer: has("Egg Warmer"),
        flame_body: save.party.iter().filter(|m| !m.egg).any(|m| {
            let a = text.ability_name(m.ability);
            same(&a, "Flame Body") || same(&a, "Magma Armor")
        }),
    }
}

/// The planner's settings with what the save says filled in.
pub fn apply(mut s: Setup, f: Found) -> Setup {
    s.shiny_charm = f.shiny_charm;
    s.oval_charm = f.oval_charm;
    s.egg_warmer = f.egg_warmer;
    s.flame_body = f.flame_body;
    s
}

// ───────────────────────────── best parents you own ─────────────────────────────

/// One pair you could put in the Day Care, with what each should hold.
#[derive(Debug, Clone)]
pub struct Pick {
    /// Indexes into the `owned` list.
    pub a: usize,
    pub b: usize,
    pub held_a: Held,
    pub held_b: Held,
    /// The chance one Egg is everything the goal asks for.
    pub chance: f64,
    /// Eggs for a 90% chance (None: it can't happen with this pair).
    pub eggs90: Option<u32>,
    pub masuda: bool,
}

impl Pick {
    pub fn parents(&self, owned: &[Owned]) -> (Parent, Parent) {
        let mut a = owned[self.a].parent.clone();
        let mut b = owned[self.b].parent.clone();
        a.held = self.held_a.clone();
        b.held = self.held_b.clone();
        (a, b)
    }
}

/// How many of each breeding item you own (bag and held), by what it does.
#[derive(Debug, Clone, Copy, Default)]
struct Stock {
    knot: u32,
    ever: u32,
    rift: u32,
    power: [u32; 6],
}

fn stock(items: &[(String, u32, u32)]) -> Stock {
    let mut s = Stock::default();
    for (name, n, _) in items {
        match Held::from_name(name) {
            Held::DestinyKnot => s.knot += n,
            Held::Everstone => s.ever += n,
            Held::RiftKnot => s.rift += n,
            Held::Power(i) => s.power[i] += n,
            _ => {}
        }
    }
    s
}

impl Stock {
    /// Both items can be given out at once.
    fn allows(&self, a: &Held, b: &Held) -> bool {
        let mut s = *self;
        for h in [a, b] {
            let left = match h {
                Held::Nothing => continue,
                Held::DestinyKnot => &mut s.knot,
                Held::Everstone => &mut s.ever,
                Held::RiftKnot => &mut s.rift,
                Held::Power(i) => &mut s.power[(*i).min(5)],
                _ => return false,
            };
            if *left == 0 {
                return false;
            }
            *left -= 1;
        }
        true
    }
}

/// What a parent could usefully hold for this goal.
fn holds_for(p: &Parent, goal: &breed::Goal, st: &Stock) -> Vec<Held> {
    let mut v = vec![Held::Nothing];
    let nature_ok = goal.nature.as_ref().is_some_and(|n| p.nature.as_deref() == Some(n.as_str()));
    if st.knot > 0 {
        v.push(Held::DestinyKnot);
    }
    if nature_ok && st.ever > 0 {
        v.push(Held::Everstone);
    }
    if st.rift > 0 {
        v.push(Held::RiftKnot);
    }
    for s in 0..6 {
        if goal.ivs[s] && p.ivs[s] == Some(31) && st.power[s] > 0 {
            v.push(Held::Power(s));
        }
    }
    v
}

/// The best pairs you own for a goal, best first: each pair with the items
/// that give it the best chance (only items you have). `baby` is the species
/// the Egg should hatch into (any stage of its family). `masuda_first`:
/// you're hunting a shiny with the Masuda Method, so pairs that count for
/// it come first.
pub fn best_pairs(owned: &[Owned], baby: &str, goal: &breed::Goal, setup: &Setup, items: &[(String, u32, u32)], wiki: &Wiki, limit: usize, masuda_first: bool) -> Vec<Pick> {
    let family = breed::baby_of(wiki, baby);
    let st = stock(items);
    // who can make this Egg: the family itself (a mother, or anyone with Ditto)
    let base: Vec<usize> = (0..owned.len()).filter(|i| breed::baby_of(wiki, &owned[*i].parent.species) == family).collect();
    let dittos: Vec<usize> = (0..owned.len()).filter(|i| owned[*i].parent.is_ditto()).collect();
    // a partner's worth before items: wanted stats at 31 and the nature
    let worth = |p: &Parent| p.goal_score(goal) * 10 + p.perfect();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for &i in &base {
        let p = &owned[i].parent;
        let mut partners: Vec<usize> = dittos.clone();
        if p.gender() == Gender::Female {
            partners.extend((0..owned.len()).filter(|j| *j != i && owned[*j].parent.gender() == Gender::Male));
        }
        partners.sort_by_key(|j| std::cmp::Reverse(worth(&owned[*j].parent)));
        let mut kept = 0;
        for j in partners {
            if kept >= 12 {
                break;
            }
            if breed::compat(p, &owned[j].parent, Some(wiki)).is_ok() {
                pairs.push((i, j));
                kept += 1;
            }
        }
    }
    let mut out: Vec<Pick> = Vec::new();
    for (i, j) in pairs {
        let (pa, pb) = (&owned[i].parent, &owned[j].parent);
        let mut best: Option<Pick> = None;
        for ha in holds_for(pa, goal, &st) {
            for hb in holds_for(pb, goal, &st) {
                if !st.allows(&ha, &hb) {
                    continue;
                }
                let (mut a, mut b) = (pa.clone(), pb.clone());
                a.held = ha.clone();
                b.held = hb.clone();
                let Ok(egg) = breed::egg(&a, &b, setup, wiki) else { continue };
                if egg.species.first().is_none_or(|(sp, _)| breed::baby_of(wiki, sp) != family) {
                    continue;
                }
                let chance = goal.chance(&a, &b, &egg);
                // fewer items moved wins a tie
                let moved = |h: &Held, now: &Held| (h != now && *h != Held::Nothing) as u8;
                let better = match &best {
                    None => true,
                    Some(bp) => chance > bp.chance * 1.000_001 || (chance >= bp.chance * 0.999_999 && moved(&ha, &pa.held) + moved(&hb, &pb.held) < moved(&bp.held_a, &pa.held) + moved(&bp.held_b, &pb.held)),
                };
                if better {
                    best = Some(Pick { a: i, b: j, held_a: ha.clone(), held_b: hb.clone(), chance, eggs90: breed::eggs_for(chance, 0.9), masuda: a.masuda || b.masuda });
                }
            }
        }
        if let Some(b) = best.filter(|b| b.chance > 0.0) {
            out.push(b);
        }
    }
    out.sort_by(|x, y| y.chance.partial_cmp(&x.chance).unwrap_or(std::cmp::Ordering::Equal).then((y.masuda as u8).cmp(&(x.masuda as u8))));
    // one line per pair of Pokémon (a Ditto pair once, not per Ditto twice over)
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert((p.a.min(p.b), p.a.max(p.b))));
    // hunting with Masuda: the best pair that counts for it is always shown
    let best_masuda = out.iter().find(|p| p.masuda).cloned();
    out.truncate(limit);
    if let Some(m) = best_masuda.filter(|_| masuda_first && limit > 0) {
        if !out.iter().any(|p| p.masuda) {
            out.pop();
            out.push(m);
        }
    }
    out
}

/// What to do to set up a pick, in the game: who goes in, who holds what,
/// and where each item is now.
pub fn how_to(pick: &Pick, owned: &[Owned], save: &Save, items: &[(String, u32, u32)]) -> Vec<String> {
    let mut v = Vec::new();
    // a parent in a box comes out through the party: it needs a free space
    let from_box = [pick.a, pick.b].iter().any(|k| matches!(owned[*k].place, Place::Box(..)));
    if from_box && room(save).0 == 0 {
        v.push("Your party is full: put one of them in a box first".into());
    }
    for (k, held) in [(pick.a, &pick.held_a), (pick.b, &pick.held_b)] {
        let o = &owned[k];
        if !matches!(o.place, Place::DayCare(_)) {
            v.push(format!("Put {} ({}) in the Day Care", o.name(), o.place.label(save)));
        }
        if *held != o.parent.held {
            if *held == Held::Nothing {
                v.push(format!("Take the {} off {}", o.parent.held.name(), o.name()));
            } else {
                let name = held.name();
                let in_bag = items.iter().any(|(n, total, held_n)| *n == name && total > held_n);
                let from = if in_bag {
                    "it's in your bag".to_string()
                } else {
                    owned.iter().find(|x| x.parent.held == *held && x.pid != o.pid).map(|x| format!("{} holds it", x.name())).unwrap_or_default()
                };
                v.push(if from.is_empty() { format!("Give {} the {name}", o.name()) } else { format!("Give {} the {name} ({from})", o.name()) });
            }
        }
    }
    v
}

// ───────────────────────────── the unusual cases ─────────────────────────────

/// Why none of your Pokémon can make `baby`'s Egg, in a sentence (for when
/// `best_pairs` finds nothing), and whether catching or breeding something
/// could change that (false: it can never be bred).
pub fn why_no_pairs(owned: &[Owned], baby: &str, wiki: &Wiki) -> (String, bool) {
    let family = breed::baby_of(wiki, baby);
    let name = |id: &str| battle::data().species(id).map(|s| s.name.clone()).unwrap_or_else(|| id.to_string());
    if family == "DITTO" {
        return ("Ditto can't be bred: they only come from the wild.".into(), false);
    }
    // a family where nobody can make Eggs (legendaries and the like)
    let breeds = |id: &str| breed::info(id).is_some_and(|i| !i.no_eggs());
    let family_breeds = breeds(&family) || battle::data().species.iter().any(|s| !s.wiki.is_empty() && s.base.is_empty() && breeds(&s.wiki) && breed::baby_of(wiki, &s.wiki) == family);
    if !family_breeds {
        return (format!("{} can't make Eggs, so it can't be bred.", name(baby)), false);
    }
    let mine: Vec<&Owned> = owned.iter().filter(|o| !o.parent.is_ditto() && breed::baby_of(wiki, &o.parent.species) == family && breeds(&o.parent.species)).collect();
    let ditto = owned.iter().any(|o| o.parent.is_ditto());
    let ratio = breed::info(&family).map(|i| i.ratio).unwrap_or(4);
    if mine.is_empty() {
        let fam = name(&family);
        let t = if ditto {
            format!("You don't have any of {fam}'s family yet that can breed. Catch one and it pairs with your Ditto.")
        } else {
            format!("You don't have any of {fam}'s family yet that can breed. Catch one, plus a partner (a Ditto works with anything).")
        };
        return (t, true);
    }
    if ditto {
        // a Ditto is there but no pair was found: nothing it makes is the goal
        return ("None of your pairs can make this Egg with what you own.".into(), true);
    }
    if ratio < 0 {
        return (format!("{} is genderless: it only breeds with a Ditto, and you don't have one.", name(&family)), true);
    }
    if ratio == 0 {
        return (format!("{}'s family is all male: it needs a Ditto to breed, and you don't have one.", name(&family)), true);
    }
    if mine.iter().all(|o| o.parent.gender() != Gender::Female) {
        return (format!("You only have males of {}'s family: a female needs to be the mother, or use a Ditto.", name(&family)), true);
    }
    ("None of your males share an egg group with it. A Ditto works with anything.".into(), true)
}

/// Free spaces: (in the party, in the boxes).
pub fn room(save: &Save) -> (usize, usize) {
    let party = 6usize.saturating_sub(save.party.len());
    let boxes = save.boxes.iter().map(|b| b.slots.iter().filter(|s| s.is_none()).count()).sum();
    (party, boxes)
}

/// Which save this is, so each world keeps its own Eggs: the trainer's name
/// and ID (each world is a new game).
pub fn save_key(save: &Save) -> String {
    format!("{}-{}", save.tid, save.trainer.trim())
}

// ───────────────────────────── the hatch log, by itself ─────────────────────────────

/// Every Egg the save has (carried or boxed), by its own number.
pub fn egg_pids(save: &Save) -> Vec<u32> {
    save.party.iter().chain(save.boxes.iter().flat_map(|b| b.slots.iter().flatten())).filter(|m| m.egg).map(|m| m.pid).collect()
}

/// Every hatched Pokémon in the save (met at level 0).
pub fn hatched(save: &Save) -> Vec<&Mon> {
    save.party.iter().chain(save.boxes.iter().flat_map(|b| b.slots.iter().flatten())).filter(|m| !m.egg && m.met_level == 0).collect()
}

/// A hatched Pokémon as a log entry. `pair` is the Day Care pair it came
/// from (as `breed::pair_key`), when known.
pub fn to_hatch(m: &Mon, text: &GameText, keys: Option<&MoveKeys>, world: World, pair: &str, when: u64) -> Option<breed::Hatch> {
    let id = species_id(m.species)?;
    let gender = if m.genderless {
        Gender::Genderless
    } else if m.female {
        Gender::Female
    } else {
        Gender::Male
    };
    let _ = world;
    Some(breed::Hatch {
        id: 0,
        when,
        species: id,
        gender,
        nature: NATURES.get(m.nature() as usize).map(|s| s.to_string()),
        ivs: ivs_of(m),
        ability: Some(text.ability_name(m.ability)),
        shiny: m.shiny(),
        ball: if m.ball == 0 { "Poké Ball".into() } else { text.item_name(m.ball as u16) },
        moves: keys.map(|k| m.moves.iter().filter(|mv| **mv != 0).filter_map(|mv| k.key(&text.move_name(*mv))).collect()).unwrap_or_default(),
        pair: pair.to_string(),
        kept: None,
        pid: m.pid,
        world: String::new(),
    })
}

/// Before comparing a save with the remembered Eggs: if it's another
/// world's save, put this world's Eggs aside and take that world's out, so
/// switching worlds never forgets an Egg (or logs one twice).
pub fn switch_eggs(f: &mut breed::BreedFile, key: &str) {
    if f.egg_save == key {
        return;
    }
    if !f.egg_save.is_empty() {
        let mine = std::mem::take(&mut f.egg_pids);
        if !mine.is_empty() {
            f.other_eggs.insert(f.egg_save.clone(), mine);
        }
        f.egg_pids = f.other_eggs.remove(key).unwrap_or_default();
    }
    // (Eggs remembered before worlds were told apart stay with this save)
    f.egg_save = key.to_string();
}

/// What changed since the planner last looked at the save: Eggs that have
/// hatched since (logged by themselves), and the Eggs to remember now.
pub struct Sync {
    pub new: Vec<breed::Hatch>,
    pub egg_pids: Vec<u32>,
}

/// The Day Care pair in the save as parents (none, one or two).
pub fn day_care(save: &Save, text: &GameText, world: World) -> Vec<Parent> {
    save.daycare.iter().filter_map(|m| to_parent(m, save, text, None, world)).collect()
}

/// Compare the save with the Eggs the planner remembers. A remembered Egg
/// that's now a hatched Pokémon gets logged once (never twice, even if it
/// moves boxes). New Eggs are remembered for next time.
/// `pair`: the Day Care pair the new hatches came from (see `day_care_pair`).
pub fn sync(save: &Save, text: &GameText, keys: Option<&MoveKeys>, world: World, remembered: &[u32], logged: &[u32], now: u64, pair: &str) -> Sync {
    let eggs_now = egg_pids(save);
    let mut new = Vec::new();
    for m in hatched(save) {
        if remembered.contains(&m.pid) && !logged.contains(&m.pid) && !new.iter().any(|h: &breed::Hatch| h.pid == m.pid) {
            if let Some(h) = to_hatch(m, text, keys, world, pair, now) {
                new.push(h);
            }
        }
    }
    // remember Eggs still around, plus new ones; forget hatched or gone
    // ones only once they're logged (a boxed Egg still counts)
    let mut keep: Vec<u32> = remembered.iter().copied().filter(|p| eggs_now.contains(p)).collect();
    for p in eggs_now {
        if !keep.contains(&p) {
            keep.push(p);
        }
    }
    Sync { new, egg_pids: keep }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::OnceLock;

    const GAME: &str = "/mnt/user-data/uploads/PlatinumTogether/Game";

    fn wiki() -> &'static Wiki {
        static W: OnceLock<Wiki> = OnceLock::new();
        W.get_or_init(|| Wiki::parse(include_bytes!("../assets/wiki.json")).expect("wiki"))
    }

    fn mon(id: &str, g: Gender) -> Owned {
        Owned { place: Place::Box(0, 0), pid: 1, parent: Parent::new(id, g), nickname: String::new(), level: None, shiny: false, item: String::new(), ability: String::new() }
    }

    #[test]
    fn why_nothing_can_make_the_egg() {
        let w = wiki();
        let says = |owned: &[Owned], baby: &str, want: &str| {
            let setup = Setup { world: World::Platinum, ..Default::default() };
            assert!(best_pairs(owned, baby, &breed::Goal::default(), &setup, &[], w, 3, false).is_empty(), "{baby}: a pair was found");
            let (t, can) = why_no_pairs(owned, baby, w);
            assert!(t.contains(want), "{baby}: {t}");
            // only Ditto and the no-Egg families can never be bred
            assert_eq!(can, !matches!(baby, "DITTO" | "MEWTWO"), "{baby}");
        };
        says(&[mon("DITTO", Gender::Genderless)], "DITTO", "only come from the wild");
        says(&[], "MEWTWO", "can't make Eggs");
        says(&[mon("MAGNEMITE", Gender::Genderless)], "MAGNEMITE", "genderless");
        says(&[mon("TAUROS", Gender::Male)], "TAUROS", "all male");
        says(&[mon("EEVEE", Gender::Male), mon("EEVEE", Gender::Male)], "EEVEE", "only have males");
        says(&[mon("PIKACHU", Gender::Male)], "EEVEE", "don't have any of Eevee");
        // a female with no male in a shared egg group (Field): a Magikarp male won't do
        says(&[mon("EEVEE", Gender::Female), mon("MAGIKARP", Gender::Male)], "EEVEE", "egg group");
        // and with a Ditto, the genderless one is found
        let setup = Setup { world: World::Platinum, ..Default::default() };
        assert!(!best_pairs(&[mon("MAGNEMITE", Gender::Genderless), mon("DITTO", Gender::Genderless)], "MAGNEMITE", &breed::Goal::default(), &setup, &[], w, 3, false).is_empty());
    }

    #[test]
    fn each_world_keeps_its_own_eggs() {
        let mut f = breed::BreedFile { egg_pids: vec![1, 2], ..Default::default() };
        // Eggs from before worlds were told apart stay with the first save seen
        switch_eggs(&mut f, "100-ZIFU");
        assert_eq!(f.egg_pids, vec![1, 2]);
        // another world: its own (none yet); the first world's are kept aside
        switch_eggs(&mut f, "200-ZIFU");
        assert!(f.egg_pids.is_empty());
        f.egg_pids = vec![9];
        // back again: nothing forgotten either way
        switch_eggs(&mut f, "100-ZIFU");
        assert_eq!(f.egg_pids, vec![1, 2]);
        assert_eq!(f.other_eggs.get("200-ZIFU"), Some(&vec![9]));
        assert!(!f.other_eggs.contains_key("100-ZIFU"));
        // the same save again changes nothing
        switch_eggs(&mut f, "100-ZIFU");
        assert_eq!(f.egg_pids, vec![1, 2]);
    }

    #[test]
    fn a_full_party_is_cleared_first() {
        let mut a = mon("EEVEE", Gender::Female);
        a.place = Place::Box(2, 4);
        let mut b = mon("DITTO", Gender::Genderless);
        b.place = Place::DayCare(0);
        b.pid = 2;
        let owned = vec![a, b];
        let pick = Pick { a: 0, b: 1, held_a: Held::Nothing, held_b: Held::Nothing, chance: 1.0, eggs90: Some(1), masuda: false };
        let mut s = Save { party: vec![Mon::default(); 6], ..Default::default() };
        let steps = how_to(&pick, &owned, &s, &[]);
        assert!(steps[0].contains("party is full"), "{steps:?}");
        assert!(steps[1].starts_with("Put Eevee"), "{steps:?}");
        // room in the party: straight to it
        s.party.pop();
        assert!(how_to(&pick, &owned, &s, &[])[0].starts_with("Put Eevee"));
    }

    #[test]
    fn room_in_the_party_and_boxes() {
        let mut s = Save { party: vec![Mon::default(); 6], ..Default::default() };
        s.boxes.push(crate::gamesave::GameBox { name: "BOX 1".into(), slots: vec![None; 30] });
        s.boxes[0].slots[3] = Some(Mon::default());
        assert_eq!(room(&s), (0, 29));
        s.party.truncate(4);
        assert_eq!(room(&s).0, 2);
    }

    #[test]
    fn ivs_come_in_the_planners_order() {
        let m = Mon { ivs: [1, 2, 3, 4, 5, 6], ..Default::default() };
        // save: HP Atk Def Spe SpA SpD → planner: HP Atk Def SpA SpD Spe
        assert_eq!(ivs_of(&m), [Some(1), Some(2), Some(3), Some(5), Some(6), Some(4)]);
    }

    #[test]
    fn species_and_slots() {
        assert_eq!(species_id(133).as_deref(), Some("EEVEE"));
        assert_eq!(species_id(132).as_deref(), Some("DITTO"));
        // a newer Pokémon by its game number
        assert!(species_id(541).is_some());
        assert_eq!(slot_of("EEVEE", "Run Away", World::Platinum), Slot::First);
        assert_eq!(slot_of("EEVEE", "Adaptability", World::Platinum), Slot::Second);
        if let Some(h) = breed::hidden_ability("EEVEE") {
            assert_eq!(slot_of("EEVEE", &h, World::Platinum), Slot::Hidden);
        }
    }

    /// Your save, if it's on this machine: the Day Care pair, items, settings.
    #[test]
    fn your_day_care() {
        let g = Path::new(GAME);
        let Ok(s) = crate::gamesave::read(&g.join("save.bin")) else { return };
        let Some(t) = crate::gamesave::load_text(g) else { return };
        let all = owned(&s, &t, Some(wiki()), World::Platinum);
        let dc: Vec<&Owned> = all.iter().filter(|o| matches!(o.place, Place::DayCare(_))).collect();
        assert_eq!(dc.len(), 2);
        let eevee = dc.iter().find(|o| o.parent.species == "EEVEE").unwrap();
        assert_eq!(eevee.parent.gender, Gender::Male);
        assert_eq!(eevee.parent.nature.as_deref(), Some("Bold"));
        // save order [4, 16, 31, 29, 29, 31] is HP Atk Def Spe SpA SpD
        assert_eq!(eevee.parent.ivs, [Some(4), Some(16), Some(31), Some(29), Some(31), Some(29)]);
        assert_eq!(eevee.parent.held, Held::DestinyKnot);
        assert_eq!(eevee.parent.ball, "Repeat Ball");
        assert!(!eevee.parent.masuda && !eevee.parent.other_trainer);
        assert!(!eevee.parent.moves.is_empty());
        let ditto = dc.iter().find(|o| o.parent.species == "DITTO").unwrap();
        assert_eq!(ditto.parent.gender, Gender::Genderless);
        assert!(ditto.parent.masuda && ditto.parent.other_trainer, "traded from another player");
        assert_eq!(ditto.parent.ball, "Great Ball");
        // the pair is compatible and gives Eevee
        let egg = breed::egg(&eevee.parent, &ditto.parent, &Setup { world: World::Platinum, hidden_abilities: true, ..Default::default() }, wiki()).unwrap();
        assert_eq!(egg.species[0].0, "EEVEE");
        assert_eq!(egg.inherited, 5, "the Destiny Knot");
        // no Eggs among the breeders, lots from the boxes
        assert!(all.len() > 100);
        // items: a Destiny Knot in the bag and one held
        let items = breeding_items(&s, &t);
        let knot = items.iter().find(|i| i.0 == "Destiny Knot").unwrap();
        assert_eq!((knot.1, knot.2), (2, 1));
        assert!(items.iter().any(|i| i.0 == "Everstone"));
        let f = found(&s, &t);
        assert!(!f.egg_warmer && !f.shiny_charm);
    }

    /// The backup with three Eevee Eggs: Eggs aren't breeders.
    #[test]
    fn eggs_are_not_breeders() {
        let g = Path::new(GAME);
        let Ok(s) = crate::gamesave::read(&g.join("save-backup-1.bin")) else { return };
        let Some(t) = crate::gamesave::load_text(g) else { return };
        let all = owned(&s, &t, None, World::Platinum);
        let eggs: Vec<u32> = s.party.iter().filter(|m| m.egg).map(|m| m.pid).collect();
        assert_eq!(eggs.len(), 3);
        assert!(all.iter().all(|o| !eggs.contains(&o.pid)));
        // Talonflame's Flame Body is in this party
        assert!(found(&s, &t).flame_body);
        // each Egg got at least 5 of its stats from the parents: the
        // Destiny Knot works as the planner counts it
        let dc = owned(&s, &t, None, World::Platinum).into_iter().filter(|o| matches!(o.place, Place::DayCare(_))).collect::<Vec<_>>();
        for e in s.party.iter().filter(|m| m.egg) {
            let iv = ivs_of(e);
            let from = (0..6).filter(|k| dc.iter().any(|p| p.parent.ivs[*k] == iv[*k])).count();
            assert!(from >= 5, "{from} of 6 from a parent");
        }
    }

    /// Your save: the best Eevee pairs for an Adamant Eevee with 5 IVs.
    #[test]
    fn best_eevee_pairs() {
        let g = Path::new(GAME);
        let Ok(s) = crate::gamesave::read(&g.join("save.bin")) else { return };
        let Some(t) = crate::gamesave::load_text(g) else { return };
        let all = owned(&s, &t, None, World::Platinum);
        let items = breeding_items(&s, &t);
        let goal = breed::Goal { ivs: [true, true, true, false, true, true], nature: Some("Adamant".into()), ..Default::default() };
        let setup = Setup { world: World::Platinum, hidden_abilities: true, ..Default::default() };
        let t0 = std::time::Instant::now();
        let picks = best_pairs(&all, "EEVEE", &goal, &setup, &items, wiki(), 5, false);
        // hunting with Masuda: the pair with the Ditto from another player first
        let m = best_pairs(&all, "EEVEE", &goal, &setup, &items, wiki(), 5, true);
        assert!(m.iter().any(|p| p.masuda));
        assert_eq!(m[0].chance, picks[0].chance, "the best pair still comes first");
        eprintln!("{} picks in {:?}", picks.len(), t0.elapsed());
        assert!(!picks.is_empty());
        for w in picks.windows(2) {
            assert!(w[0].chance >= w[1].chance);
        }
        let best = &picks[0];
        let (a, b) = best.parents(&all);
        assert!(a.species == "EEVEE" || b.species == "EEVEE");
        eprintln!("  {} {:?} {:?} + {} {:?} {:?}", a.species, a.nature, a.ivs, b.species, b.nature, b.ivs);
        // the items chosen beat holding nothing
        let (mut a0, mut b0) = (a.clone(), b.clone());
        a0.held = Held::Nothing;
        b0.held = Held::Nothing;
        let e0 = breed::egg(&a0, &b0, &setup, wiki()).unwrap();
        assert!(best.chance >= goal.chance(&a0, &b0, &e0));
        // never more items than you own
        let st = stock(&items);
        assert!(st.allows(&best.held_a, &best.held_b));
        for line in how_to(best, &all, &s, &items) {
            eprintln!("  {line}");
        }
        eprintln!("  best: {:.6} ({:?} eggs for 90%)", best.chance, best.eggs90);
    }

    /// An Egg the planner remembers hatches: logged once, with its details.
    #[test]
    fn a_remembered_egg_is_logged_when_it_hatches() {
        let g = Path::new(GAME);
        let Ok(before) = crate::gamesave::read(&g.join("save-backup-1.bin")) else { return };
        let Some(t) = crate::gamesave::load_text(g) else { return };
        let first = sync(&before, &t, None, World::Platinum, &[], &[], 100, "P");
        assert!(first.new.is_empty(), "nothing hatched yet");
        assert!(first.egg_pids.len() >= 3);
        // the game saves after one of the three hatches
        let mut after = before.clone();
        let i = after.party.iter().position(|m| m.egg).unwrap();
        let pid = after.party[i].pid;
        after.party[i].egg = false;
        after.party[i].met_level = 0;
        let s2 = sync(&after, &t, None, World::Platinum, &first.egg_pids, &[], 200, "P");
        assert_eq!(s2.new.len(), 1);
        let h = &s2.new[0];
        assert_eq!((h.pid, h.species.as_str(), h.when), (pid, "EEVEE", 200));
        assert_eq!(h.pair, "P");
        let dc = day_care(&after, &t, World::Platinum);
        assert_eq!(breed::pair_key(&dc[0], &dc[1]), "DITTO+EEVEE");
        assert!(!s2.egg_pids.contains(&pid));
        // read again: not logged twice
        let s3 = sync(&after, &t, None, World::Platinum, &first.egg_pids, &[pid], 300, "P");
        assert!(s3.new.is_empty());
        // a hatched Pokémon the planner never saw as an Egg isn't logged
        assert!(sync(&after, &t, None, World::Platinum, &[], &[], 400, "P").new.is_empty());
    }

    #[test]
    fn items_you_dont_have_arent_used() {
        let st = stock(&[("Destiny Knot".into(), 1, 0)]);
        assert!(st.allows(&Held::DestinyKnot, &Held::Nothing));
        assert!(!st.allows(&Held::DestinyKnot, &Held::DestinyKnot));
        assert!(!st.allows(&Held::Everstone, &Held::Nothing));
    }

    #[test]
    fn move_names_match_the_wiki() {
        let k = MoveKeys::new(wiki());
        for n in ["Tackle", "Earthquake", "Baby-Doll Eyes", "U-turn"] {
            assert_eq!(k.key(n), wiki().move_key(n), "{n}");
        }
    }
}
