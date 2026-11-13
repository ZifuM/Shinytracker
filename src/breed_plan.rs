//! The Breeding Planner's thinking: who can pass an egg move (stage 4),
//! a simulated IV climb, and a step-by-step plan for a goal (stage 5).
//! Built on the rules in `breed.rs`; reads only the app's built-in data.
#![allow(dead_code)]

use crate::battle::World;
use crate::breed::{self, Breeder, Gender, Goal, Held, Parent, Setup, Slot};
use crate::wiki::Wiki;
use std::collections::{HashMap, HashSet, VecDeque};

// ───────────────────────────── learning a move ─────────────────────────────

/// How a Pokémon learns a move.
#[derive(Debug, Clone, PartialEq)]
pub enum Learn {
    /// At this level ("1" for a starting move).
    Level(u32),
    /// TM or HM id ("TM26").
    Tm(String),
    Tutor,
    Egg,
}

impl Learn {
    pub fn text(&self) -> String {
        match self {
            Learn::Level(0) | Learn::Level(1) => "knows it from the start".into(),
            Learn::Level(n) => format!("learns it at Lv. {n}"),
            Learn::Tm(t) => format!("learns it from {t}"),
            Learn::Tutor => "learns it from a move tutor".into(),
            Learn::Egg => "gets it as an egg move".into(),
        }
    }

    pub fn short(&self) -> String {
        match self {
            Learn::Level(0) | Learn::Level(1) => "Start".into(),
            Learn::Level(n) => format!("Lv. {n}"),
            Learn::Tm(t) => t.clone(),
            Learn::Tutor => "Tutor".into(),
            Learn::Egg => "Egg move".into(),
        }
    }

    /// Lower is easier.
    fn effort(&self) -> f64 {
        match self {
            Learn::Level(n) => (*n as f64 / 12.0).max(0.2),
            Learn::Tm(t) if t.starts_with("HM") => 1.5,
            Learn::Tm(_) => 2.5,
            Learn::Tutor => 3.5,
            Learn::Egg => 9.0,
        }
    }
}

/// Every way a species learns a move (in that kind of world).
pub fn learns(w: &Wiki, species: &str, mv: &str, world: World) -> Vec<Learn> {
    let Some(ls) = w.learnset(species) else { return vec![] };
    let mut out = vec![];
    let levels = if world == World::Modern && !ls.mlv.is_empty() { &ls.mlv } else { &ls.lv };
    for l in levels {
        if l.key == mv {
            out.push(Learn::Level(l.level.trim().parse().unwrap_or(1)));
        }
    }
    let tm_key = |t: &String| w.data.tms.get(t).cloned().unwrap_or_default();
    for t in ls.tm.iter().chain(if world == World::Modern { ls.mtm.iter() } else { [].iter() }) {
        if tm_key(t) == mv {
            out.push(Learn::Tm(t.clone()));
        }
    }
    if ls.tutor.iter().chain(if world == World::Modern { ls.mtutor.iter() } else { [].iter() }).any(|k| k == mv) {
        out.push(Learn::Tutor);
    }
    if ls.egg.iter().chain(if world == World::Modern { ls.megg.iter() } else { [].iter() }).any(|k| k == mv) {
        out.push(Learn::Egg);
    }
    out
}

/// The easiest way to learn it without breeding.
pub fn native(w: &Wiki, species: &str, mv: &str, world: World) -> Option<Learn> {
    learns(w, species, mv, world)
        .into_iter()
        .filter(|l| *l != Learn::Egg)
        .min_by(|a, b| a.effort().partial_cmp(&b.effort()).unwrap_or(std::cmp::Ordering::Equal))
}

// ───────────────────────────── families ─────────────────────────────

/// A family as breeding sees it.
#[derive(Debug, Clone)]
pub struct Family {
    /// Its first stage (what hatches).
    pub baby: String,
    /// Everyone in it.
    pub members: Vec<String>,
    /// Its egg groups (from a member that can breed).
    pub groups: Vec<String>,
    /// Gender ratio of the members that breed.
    pub ratio: i8,
    /// Only in MODERN worlds.
    pub modern: bool,
}

impl Family {
    /// Can be a father: has males and can breed.
    pub fn can_father(&self) -> bool {
        self.ratio >= 0 && self.ratio < 8 && !self.groups.is_empty() && self.baby != breed::DITTO
    }
}

/// Every family that can breed, by its baby.
pub fn families(w: &Wiki, world: World) -> HashMap<String, Family> {
    let mut out: HashMap<String, Family> = HashMap::new();
    for m in w.all() {
        let Some(info) = breed::info(&m.id) else { continue };
        let baby = breed::baby_of(w, &m.id);
        let f = out.entry(baby.clone()).or_insert_with(|| Family { baby, members: vec![], groups: vec![], ratio: info.ratio, modern: true });
        f.members.push(m.id.clone());
        f.modern &= m.modern;
        if !info.no_eggs() && f.groups.is_empty() {
            f.groups = info.groups.clone();
            f.ratio = info.ratio;
        }
    }
    out.retain(|_, f| !f.groups.is_empty() && (world == World::Modern || !f.modern));
    out
}

/// The easiest member of a family that learns the move without breeding.
fn family_native(w: &Wiki, fam: &Family, mv: &str, world: World) -> Option<(String, Learn)> {
    fam.members
        .iter()
        .filter_map(|m| native(w, m, mv, world).map(|l| (m.clone(), l)))
        .min_by(|a, b| a.1.effort().partial_cmp(&b.1.effort()).unwrap_or(std::cmp::Ordering::Equal))
}

/// Where to get one of the family: (place, the stage it opens, sort key).
pub fn where_to_get(w: &Wiki, members: &[String], world: World) -> Option<(String, String, usize)> {
    let mut best: Option<(String, String, usize)> = None;
    for m in members {
        for e in w.wild(m) {
            if e.setting == "modern" && world != World::Modern {
                continue;
            }
            let place = w.place(&e.area).map(|p| p.name.clone()).unwrap_or_else(|| e.area.clone());
            let k = w.stage_index(&e.stage);
            if best.as_ref().is_none_or(|b| k < b.2) {
                best = Some((place, e.stage.clone(), k));
            }
        }
    }
    if best.is_none() {
        for m in members {
            if let Some(o) = w.other(m).into_iter().find(|o| o.works != Some(false) && !o.how.eq_ignore_ascii_case("Breed")) {
                return Some((o.place.clone(), String::new(), 50));
            }
        }
    }
    best
}

// ───────────────────────────── stage 4: egg move routes ─────────────────────────────

/// One step of a route: a family, and how it has the move.
#[derive(Debug, Clone)]
pub struct Hop {
    /// The member to use (the one that learns it, for the first hop).
    pub species: String,
    pub how: Learn,
}

/// A way to get the move onto the target: the first hop learns it, each
/// next one gets it as an egg move from a father, ending at the target.
#[derive(Debug, Clone)]
pub struct Route {
    pub hops: Vec<Hop>,
    /// The target's own family learns it: no other species needed.
    pub own_family: bool,
    /// One of your Breeders already knows it (its id).
    pub breeder: Option<u64>,
    pub place: Option<(String, String)>,
    pub score: f64,
}

/// What the finder found for one move.
#[derive(Debug, Clone, Default)]
pub struct MoveAnswer {
    /// It's an egg move for the target.
    pub egg_move: bool,
    /// How the target's family learns it otherwise.
    pub other_ways: Vec<(String, Learn)>,
    pub routes: Vec<Route>,
}

/// Who can pass `mv` to an Egg of `target`'s family, easiest first: the
/// family itself, your Breeders, direct fathers, then chains (up to 3).
pub fn move_routes(w: &Wiki, target: &str, mv: &str, world: World, breeders: &[Breeder]) -> MoveAnswer {
    let fams = families(w, world);
    let t = breed::baby_of(w, target);
    let mut ans = MoveAnswer { egg_move: breed::egg_moves(w, &t, world).contains(&mv.to_string()), ..Default::default() };
    let Some(tf) = fams.get(&t) else { return ans };
    for m in &tf.members {
        if let Some(l) = native(w, m, mv, world) {
            ans.other_ways.push((m.clone(), l));
        }
    }
    if !ans.egg_move {
        return ans;
    }
    let place = |f: &Family| where_to_get(w, &f.members, world).map(|(p, s, _)| (p, s));
    // Your Breeders that know it and could pass it.
    for b in breeders {
        let p = &b.parent;
        if p.is_ditto() || !p.moves.iter().any(|m| m == mv) {
            continue;
        }
        let bf = breed::baby_of(w, &p.species);
        let Some(f) = fams.get(&bf) else { continue };
        let own = bf == t;
        let shares = f.groups.iter().any(|g| tf.groups.contains(g));
        if own || (shares && p.gender() == Gender::Male) {
            ans.routes.push(Route {
                hops: vec![Hop { species: p.species.clone(), how: Learn::Level(0) }],
                own_family: own,
                breeder: Some(b.id),
                place: None,
                score: -10.0,
            });
        }
    }
    // The family itself.
    if let Some((m, l)) = family_native(w, tf, mv, world) {
        let s = l.effort();
        ans.routes.push(Route { hops: vec![Hop { species: m, how: l }], own_family: true, breeder: None, place: place(tf), score: s - 1.0 });
    }
    // Breadth-first from the target: carriers get it as an egg move from a
    // father in a shared group; a father that learns it ends a route.
    let mut parent_of: HashMap<String, String> = HashMap::new();
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut q = VecDeque::new();
    depth.insert(t.clone(), 0);
    q.push_back(t.clone());
    let mut seen_start: HashSet<String> = HashSet::new();
    let mut fams_sorted: Vec<&Family> = fams.values().collect();
    fams_sorted.sort_by(|a, b| a.baby.cmp(&b.baby));
    while let Some(x) = q.pop_front() {
        let d = depth[&x];
        let xf = &fams[&x];
        for s in &fams_sorted {
            if s.baby == x || s.baby == t || !s.can_father() || !s.groups.iter().any(|g| xf.groups.contains(g)) {
                continue;
            }
            if let Some((m, l)) = family_native(w, s, mv, world) {
                if seen_start.insert(s.baby.clone()) {
                    // walk back to the target
                    let mut hops = vec![Hop { species: m, how: l.clone() }];
                    let mut cur = x.clone();
                    while cur != t {
                        hops.push(Hop { species: cur.clone(), how: Learn::Egg });
                        cur = parent_of[&cur].clone();
                    }
                    let score = 10.0 * hops.len() as f64 + l.effort() + if place(s).is_some() { 0.0 } else { 6.0 };
                    ans.routes.push(Route { hops, own_family: false, breeder: None, place: place(s), score });
                }
            } else if d < 2 && !depth.contains_key(&s.baby) && breed::egg_moves(w, &s.baby, world).iter().any(|k| k == mv) {
                depth.insert(s.baby.clone(), d + 1);
                parent_of.insert(s.baby.clone(), x.clone());
                q.push_back(s.baby.clone());
            }
        }
    }
    ans.routes.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    ans
}

// ───────────────────────────── the IV climb, simulated ─────────────────────────────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn chance(&mut self, p: f64) -> bool {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64 <= p
    }
}

/// Which parent a hatched Egg should replace, if it beats one for the goal:
/// the parent of its family and gender, or the Ditto when it's a male and
/// the other parent is a female of its family (the two can breed). It
/// needs the goal's nature, if there is one.
pub fn swap_slot(w: &Wiki, a: &Parent, b: &Parent, child: &Parent, goal: &Goal) -> Option<usize> {
    if goal.nature.as_ref().is_some_and(|g| child.nature.as_deref() != Some(g.as_str())) {
        return None;
    }
    let fam = breed::baby_of(w, &child.species);
    let ps = [a, b];
    let same = (0..2).find(|i| {
        let p = ps[*i];
        !p.is_ditto() && p.gender() == child.gender() && breed::baby_of(w, &p.species) == fam
    });
    let slot = same.or_else(|| {
        (0..2).find(|i| {
            let other = ps[1 - *i];
            ps[*i].is_ditto() && child.gender() == Gender::Male && other.gender() == Gender::Female && breed::baby_of(w, &other.species) == fam
        })
    })?;
    (child.goal_score(goal) > ps[slot].goal_score(goal)).then_some(slot)
}

/// How many Eggs the whole climb takes, swapping in better offspring.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Climb {
    pub mean: f64,
    pub p50: u32,
    pub p90: u32,
    pub runs: u32,
    /// Runs that hit the cap without getting there.
    pub stuck: u32,
}

/// Breed `a` × `b` for `goal`, putting each Egg that beats its parent in
/// that parent's place, until an Egg is everything the goal asks for.
pub fn simulate_climb(w: &Wiki, a: &Parent, b: &Parent, goal: &Goal, setup: &Setup, runs: u32, seed: u64) -> Option<Climb> {
    let egg0 = breed::egg(a, b, setup, w).ok()?;
    let fam = egg0.species[0].0.clone();
    let mut rng = Rng(seed | 1);
    let mut counts: Vec<u32> = vec![];
    let mut stuck = 0;
    let natures: Vec<String> = crate::battle::data().natures.iter().map(|n| n.name.clone()).collect();
    let cap = 1200;
    for _ in 0..runs {
        let mut ps = [a.clone(), b.clone()];
        // unknown stats get a value for this run
        for p in ps.iter_mut() {
            for s in 0..6 {
                if p.ivs[s].is_none() {
                    p.ivs[s] = Some(rng.below(32) as u8);
                }
            }
        }
        let mut n = 0;
        let done = loop {
            n += 1;
            if n > cap {
                break false;
            }
            let e = match breed::egg(&ps[0], &ps[1], setup, w) {
                Ok(e) => e,
                Err(_) => break false,
            };
            // stats
            let mut iv = [None::<u8>; 6];
            let knot = ps.iter().any(|p| p.held.knot());
            let k = if knot { 5 } else { 3 };
            let powers: Vec<(usize, usize)> = ps.iter().enumerate().filter_map(|(i, p)| p.held.power().map(|s| (s, i))).collect();
            let mut left: Vec<usize> = (0..6).collect();
            let mut taken = 0;
            if !powers.is_empty() {
                let (s, i) = powers[rng.below(powers.len() as u64) as usize];
                iv[s] = ps[i].ivs[s];
                left.retain(|x| *x != s);
                taken = 1;
            }
            for _ in taken..k {
                let j = rng.below(left.len() as u64) as usize;
                let s = left.remove(j);
                iv[s] = ps[rng.below(2) as usize].ivs[s];
            }
            for v in iv.iter_mut() {
                if v.is_none() {
                    *v = Some(rng.below(32) as u8);
                }
            }
            // nature
            let keepers: Vec<&Parent> = ps.iter().filter(|p| p.held.keeps_nature()).collect();
            let nature = if keepers.is_empty() {
                natures.get(rng.below(natures.len() as u64) as usize).cloned()
            } else {
                keepers[rng.below(keepers.len() as u64) as usize].nature.clone()
            };
            // ability and gender
            let mut roll = (rng.next() >> 11) as f64 / (1u64 << 53) as f64;
            let mut ability = None;
            for (name, _, p) in &e.abilities {
                if roll < *p {
                    ability = Some(name.clone());
                    break;
                }
                roll -= p;
            }
            let gender = match e.female {
                Some(f) if rng.chance(f) => Gender::Female,
                Some(_) => Gender::Male,
                None => Gender::Genderless,
            };
            let hatch = breed::Hatch { species: fam.clone(), gender, nature, ivs: iv, ability, ..Default::default() };
            if hatch.hits(goal) {
                break true;
            }
            // Swap it in for the parent it beats.
            let child = hatch.to_parent();
            if let Some(i) = swap_slot(w, &ps[0], &ps[1], &child, goal) {
                let p = &mut ps[i];
                let held = p.held.clone();
                let moves = p.moves.clone();
                let ability = if p.is_ditto() { child.ability } else { p.ability };
                *p = Parent { held, moves, ability, ..child.clone() };
            }
        };
        if done {
            counts.push(n);
        } else {
            stuck += 1;
        }
    }
    if counts.is_empty() {
        return Some(Climb { runs, stuck, ..Default::default() });
    }
    counts.sort_unstable();
    let pick = |q: f64| counts[((counts.len() as f64 - 1.0) * q).round() as usize];
    Some(Climb {
        mean: counts.iter().map(|c| *c as f64).sum::<f64>() / counts.len() as f64,
        p50: pick(0.5),
        p90: pick(0.9),
        runs,
        stuck,
    })
}

// ───────────────────────────── stage 5: the plan ─────────────────────────────

/// What you want to end up with.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Target {
    /// Any member of the family ("GARCHOMP"); the Egg is its first stage.
    pub species: String,
    pub nature: Option<String>,
    pub ivs: [bool; 6],
    /// Its Ability by name (None = any).
    pub ability: Option<String>,
    pub ball: Option<String>,
    /// Egg moves (keys).
    pub moves: Vec<String>,
    pub shiny: bool,
    pub gender: Option<Gender>,
}

impl Target {
    pub fn goal(&self) -> Goal {
        Goal { ivs: self.ivs, nature: self.nature.clone(), ability: self.ability.clone(), gender: self.gender }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Mother,
    Ditto,
    Nature,
    Ability,
    Move,
    Climb,
    Ball,
    Shiny,
}

/// One step of the plan.
#[derive(Debug, Clone)]
pub struct Step {
    /// Stable id for ticking it off ("move:outrage").
    pub key: String,
    pub kind: StepKind,
    pub title: String,
    pub detail: String,
    /// Already true from your Breeders.
    pub done: bool,
    /// Pokémon to show (species ids).
    pub show: Vec<String>,
    /// For a move step: the move.
    pub mv: Option<String>,
    /// For the climb: expected Eggs.
    pub climb: Option<Climb>,
}

/// An item the plan needs, and where to get it.
#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    pub item: String,
    pub why: String,
    pub place: String,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub steps: Vec<Step>,
    pub items: Vec<Need>,
    /// The two parents to start the climb with (items on).
    pub parents: Option<(Parent, Parent)>,
    /// What's wrong with the goal, if it can't be done.
    pub problem: Option<String>,
}

fn item_place(w: &Wiki, name: &str) -> String {
    match name {
        "Oval Charm" => return "after 10 hatched Eggs (the Day Care man)".into(),
        "Destiny Knot" => {
            let first = w.data.items.iter().find(|i| i.name == name).and_then(|i| i.found.first()).map(|s| format!(", or {}", s.place)).unwrap_or_default();
            return format!("the Day Care man gives one after your first Egg hatches{first}");
        }
        _ => {}
    }
    w.data
        .items
        .iter()
        .find(|i| i.name == name)
        .and_then(|i| i.found.first())
        .map(|s| if s.detail.is_empty() || s.detail.len() > 70 { s.place.clone() } else { format!("{}: {}", s.place, s.detail) })
        .unwrap_or_else(|| "see Game Info".into())
}

/// "a Jolly", "an Adamant".
fn a_an(word: &str) -> String {
    let an = word.chars().next().is_some_and(|c| "AEIOUaeiou".contains(c));
    format!("{} {word}", if an { "an" } else { "a" })
}

/// 12,345
pub fn thousands(n: u32) -> String {
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

fn name(w: &Wiki, id: &str) -> String {
    w.mon(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string())
}

fn move_name(w: &Wiki, key: &str) -> String {
    w.move_info(key).map(|m| m.n.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| key.replace('_', " "))
}

/// The plan for a target, from what you have in your Breeders.
pub fn make_plan(w: &Wiki, t: &Target, breeders: &[Breeder], setup: &Setup, sim_runs: u32) -> Plan {
    let mut plan = Plan::default();
    if t.species.is_empty() {
        return plan;
    }
    let baby = breed::baby_of(w, &t.species);
    let Some(info) = breed::info(&t.species) else { return plan };
    if info.no_eggs() && breed::info(&baby).is_none_or(|b| b.no_eggs()) && !w.evolves_into(&baby).iter().any(|e| breed::info(&e.species).is_some_and(|i| !i.no_eggs())) {
        plan.problem = Some(format!("{} can't be bred.", name(w, &t.species)));
        return plan;
    }
    let fam_members: Vec<String> = w.family(&baby).into_iter().flatten().collect();
    let in_family = |p: &Parent| breed::baby_of(w, &p.species) == baby && !p.is_ditto();
    let genderless = breed::info(&baby).map(|i| i.ratio < 0).unwrap_or(false);
    let goal = t.goal();
    let mut need_items: Vec<(String, String)> = vec![];

    // 1. A parent from the family (the mother, or any gender with Ditto).
    let mut mothers: Vec<&Breeder> = breeders.iter().filter(|b| in_family(&b.parent) && !b.parent.partner).collect();
    mothers.sort_by_key(|b| {
        let p = &b.parent;
        let ab_ok = t.ability.as_ref().is_none_or(|_| p.ability == Slot::Hidden || t.ability.as_deref() != breed::hidden_ability(&p.species).as_deref());
        std::cmp::Reverse((p.goal_score(&goal) * 4 + t.moves.iter().filter(|m| p.moves.contains(m)).count() * 3 + ab_ok as usize * 2 + (p.gender() == Gender::Female) as usize, b.id))
    });
    let mother = mothers.first().map(|b| b.parent.clone());
    let where_fam = where_to_get(w, &fam_members, setup.world);
    plan.steps.push(Step {
        key: "mother".into(),
        kind: StepKind::Mother,
        title: match &mother {
            Some(m) => format!("Your {} is the parent to start from", name(w, &m.species)),
            None => format!("Get a {}{}", name(w, &t.species), if genderless { "" } else { " (a female is best)" }),
        },
        detail: match (&mother, &where_fam) {
            (Some(_), _) => "It's the one in your Breeders that's closest to what you want. The Egg takes its species, Ability and ball.".into(),
            (None, Some((p, s, _))) => format!("Catch one at {p}{}. A female passes on her Ability and ball; with Ditto any gender works. Add it to your Breeders.", if s.is_empty() { String::new() } else { format!(" ({s})") }),
            (None, None) => "Get one (a trade or the GTS works too) and add it to your Breeders.".into(),
        },
        done: mother.is_some(),
        show: vec![mother.as_ref().map(|m| m.species.clone()).unwrap_or_else(|| t.species.clone())],
        mv: None,
        climb: None,
    });

    // 2. Ditto.
    let mut dittos: Vec<&Breeder> = breeders.iter().filter(|b| b.parent.is_ditto()).collect();
    dittos.sort_by_key(|b| std::cmp::Reverse((b.parent.goal_score(&goal) * 4 + (t.shiny && b.parent.masuda) as usize * 10, b.id)));
    let ditto = dittos.first().map(|b| b.parent.clone());
    let where_ditto = where_to_get(w, &[breed::DITTO.to_string()], setup.world);
    plan.steps.push(Step {
        key: "ditto".into(),
        kind: StepKind::Ditto,
        title: match &ditto {
            Some(d) => format!("Your Ditto ({} stats at 31) is the partner", d.perfect()),
            None => "Get a Ditto".into(),
        },
        detail: match (&ditto, &where_ditto) {
            (Some(_), _) => "Ditto breeds with almost anything, so you never need a second of your own species.".into(),
            (None, Some((p, _, _))) => format!("{p}. One with lots of stats at 31 saves a lot of Eggs, and one traded from another player counts for the Masuda Method."),
            (None, None) => "A trade or the GTS works. One traded from another player counts for the Masuda Method.".into(),
        },
        done: ditto.is_some(),
        show: vec![breed::DITTO.to_string()],
        mv: None,
        climb: None,
    });

    // 3. Egg moves.
    for mv in &t.moves {
        let have = mother.as_ref().is_some_and(|m| m.moves.contains(mv)) || breeders.iter().any(|b| in_family(&b.parent) && b.parent.moves.contains(mv));
        let ans = move_routes(w, &baby, mv, setup.world, breeders);
        let mn = move_name(w, mv);
        let (title, detail, show) = if have {
            (format!("Your {} knows {mn}", name(w, &baby)), "Keep it on the family parent through the climb: egg moves pass from either parent.".to_string(), vec![baby.clone()])
        } else {
            match ans.routes.iter().find(|r| r.breeder.is_none()).or(ans.routes.first()) {
                _ if !ans.egg_move => (format!("{mn} isn't an egg move for {}", name(w, &baby)), "Teach it after hatching instead, or pick another move.".to_string(), vec![]),
                Some(r) if r.own_family => (
                    format!("Teach {mn} to your {}", name(w, &r.hops[0].species)),
                    format!("It {}. Then it passes to every Egg.", r.hops[0].how.text()),
                    vec![r.hops[0].species.clone()],
                ),
                Some(r) => {
                    let first = &r.hops[0];
                    let chain: Vec<String> = r.hops.iter().map(|h| name(w, &h.species)).chain(std::iter::once(name(w, &baby))).collect();
                    (
                        format!("Get {mn} from a male {}", name(w, &first.species)),
                        format!(
                            "It {}{}. Breed it with your {}: {}. Keep a {} that knows it.",
                            first.how.text(),
                            r.place.as_ref().map(|(p, _)| format!(" (catch one at {p})")).unwrap_or_default(),
                            if r.hops.len() > 1 { format!("{} line", name(w, &r.hops[1].species)) } else { name(w, &baby) },
                            chain.join(" → "),
                            name(w, &baby)
                        ),
                        r.hops.iter().map(|h| h.species.clone()).collect(),
                    )
                }
                None => (format!("No one can pass {mn}"), "Nothing in this world can teach it to this family.".to_string(), vec![]),
            }
        };
        let mv_ok = have || ans.egg_move;
        plan.steps.push(Step { key: format!("move:{mv}"), kind: StepKind::Move, title, detail, done: have, show, mv: mv_ok.then(|| mv.clone()), climb: None });
    }

    // 4. Ability.
    if let Some(ab) = &t.ability {
        let hidden = breed::hidden_ability(&baby).as_deref() == Some(ab.as_str()) || breed::hidden_ability(&t.species).as_deref() == Some(ab.as_str());
        let has = mother.as_ref().is_some_and(|m| {
            let mine = match m.ability {
                Slot::Hidden => breed::hidden_ability(&m.species),
                Slot::Second => breed::normal_abilities(&m.species, setup.world).get(1).cloned(),
                Slot::First => breed::normal_abilities(&m.species, setup.world).first().cloned(),
            };
            mine.as_deref() == Some(ab.as_str()) || (hidden && m.ability == Slot::Hidden)
        });
        if hidden && !setup.hidden_abilities {
            plan.problem = Some(format!("{ab} is a hidden Ability, and your world doesn't have them (turn on Hidden Abilities if it does)."));
        }
        let (title, detail) = if has {
            (format!("The parent has {ab}"), if hidden { "A hidden Ability passes 60% of the time.".to_string() } else { "It passes 80% of the time.".to_string() })
        } else if hidden {
            need_items.push(("Ability Patch".into(), format!("gives the parent {ab}")));
            (
                format!("Give the parent {ab}"),
                "An Ability Patch (Rift Shop) gives it its hidden Ability. Or catch one that has it: a Rift Incense makes them ten times likelier for 200 steps.".to_string(),
            )
        } else {
            need_items.push(("Ability Capsule".into(), format!("swaps the parent to {ab}")));
            (format!("Give the parent {ab}"), "An Ability Capsule (Rift Shop) swaps between its two Abilities.".to_string())
        };
        plan.steps.push(Step { key: "ability".into(), kind: StepKind::Ability, title, detail, done: has, show: vec![], mv: None, climb: None });
    }

    // 5. Nature.
    let mut a = mother.clone().unwrap_or_else(|| Parent::new(&t.species, Gender::Female));
    a.gender = a.gender();
    let mut b = ditto.clone().unwrap_or_else(|| Parent::new(breed::DITTO, Gender::Male));
    for m in &t.moves {
        if !a.moves.contains(m) && a.moves.len() < 4 {
            a.moves.push(m.clone());
        }
    }
    if let Some(n) = &t.nature {
        let holder = if a.nature.as_deref() == Some(n.as_str()) {
            Some(0)
        } else if b.nature.as_deref() == Some(n.as_str()) {
            Some(1)
        } else {
            None
        };
        let (title, detail, done) = match holder {
            Some(i) => {
                let who = if i == 0 { name(w, &a.species) } else { "Ditto".into() };
                (format!("Your {who} is {n}: it holds the nature item"), format!("Every Egg will be {n}."), true)
            }
            None => (
                format!("Get {} parent", a_an(n)),
                format!(
                    "Catch {} Ditto or {}: a Synchronize lead, or the Sync Bell (Rift Shop) held by your lead, makes wild Pokémon share its nature. Or keep {} Egg from the Hatch log.",
                    a_an(n),
                    name(w, &baby),
                    a_an(n)
                ),
                false,
            ),
        };
        if !done {
            need_items.push(("Sync Bell".into(), format!("catch {} parent", a_an(n))));
        }
        plan.steps.push(Step { key: "nature".into(), kind: StepKind::Nature, title, detail, done, show: vec![], mv: None, climb: None });
        // the parent that will hold it
        let keeper_is_b = holder == Some(1);
        let keeper = if keeper_is_b { &mut b } else { &mut a };
        if holder.is_none() {
            keeper.nature = Some(n.clone());
        }
        keeper.held = Held::Everstone;
    }

    // The climb assumes the steps above are done: the parent has the
    // Ability you want.
    if let Some(ab) = &t.ability {
        if breed::hidden_ability(&a.species).as_deref() == Some(ab.as_str()) && breed::normal_abilities(&a.species, setup.world).first() != Some(ab) {
            a.ability = Slot::Hidden;
        } else if breed::normal_abilities(&a.species, setup.world).iter().position(|x| x == ab) == Some(1) {
            a.ability = Slot::Second;
        } else if breed::normal_abilities(&a.species, setup.world).first() == Some(ab) {
            a.ability = Slot::First;
        }
    }

    // 6. Items: the knot on the other parent (or a Rift Knot when one holds both).
    let wants_stats = t.ivs.iter().any(|x| *x);
    if wants_stats {
        if a.held == Held::Everstone {
            b.held = Held::DestinyKnot;
        } else if b.held == Held::Everstone {
            a.held = Held::DestinyKnot;
        } else {
            b.held = Held::DestinyKnot;
        }
        need_items.push(("Destiny Knot".into(), "5 stats from the parents instead of 3".into()));
    }
    if t.nature.is_some() {
        need_items.push(("Everstone".into(), "passes the nature".into()));
    }
    if !setup.oval_charm {
        need_items.push(("Oval Charm".into(), "Eggs turn up more often".into()));
    }
    if !setup.egg_warmer {
        need_items.push(("Egg Warmer".into(), "Eggs hatch twice as fast".into()));
    }

    // 7. The climb.
    if wants_stats || t.nature.is_some() || t.ability.is_some() || t.gender.is_some() {
        let climb = if a.species.is_empty() {
            None
        } else if sim_runs == 0 {
            Some(Climb::default())
        } else {
            simulate_climb(w, &a, &b, &goal, setup, sim_runs, 0x5eed_1234)
        };
        let hatch = breed::egg(&a, &b, setup, w).ok().map(|e| e.hatch_steps(setup)).unwrap_or(5100);
        let per_egg = breed::compat(&a, &b, Some(w)).map(|c| c.steps_per_egg(setup.oval_charm)).unwrap_or(512);
        let detail = match &climb {
            Some(c) if c.runs == 0 => "Working out how many Eggs it takes…".to_string(),
            Some(c) if c.stuck < c.runs => format!(
                "Put them in the Day Care with the items above. Each time an Egg beats a parent for what you want, swap it in: a better {} for the {}, and a good male for the Ditto (the Hatch log flags them). Half the time it takes {} Eggs or fewer, 9 times in 10 within {}: roughly {} steps.",
                name(w, &baby),
                name(w, &a.species),
                c.p50,
                c.p90,
                thousands((c.p50 as u64 * per_egg as u64 + hatch as u64).min(u32::MAX as u64) as u32)
            ),
            Some(_) => "These two can't get there: check that the parents can pass on what you want.".into(),
            None => "Put the two parents in the Day Care and swap in each Egg that beats its parent.".into(),
        };
        plan.steps.push(Step {
            key: "climb".into(),
            kind: StepKind::Climb,
            title: "Breed and swap in better Eggs".into(),
            detail,
            done: false,
            show: vec![a.species.clone(), b.species.clone()],
            mv: None,
            climb,
        });
    }

    // 8. Ball.
    if let Some(ball) = &t.ball {
        let has = a.ball == *ball;
        plan.steps.push(Step {
            key: "ball".into(),
            kind: StepKind::Ball,
            title: if has { format!("The Egg comes in a {ball}") } else { format!("Get it in a {ball}") },
            detail: if has {
                "The family parent's ball passes on.".into()
            } else {
                format!("The Egg gets the family parent's ball, so use one caught in a {ball}. Or change it after hatching with a Ball Capsule (Rift Shop).")
            },
            done: has,
            show: vec![],
            mv: None,
            climb: None,
        });
        if !has {
            need_items.push(("Ball Capsule".into(), format!("puts it in a {ball}")));
        }
    }

    // 9. Shiny.
    if t.shiny {
        let masuda = a.masuda || b.masuda;
        let one_in = breed::shiny_one_in(breed::shiny_rolls(setup.shiny_charm, masuda)).round() as u32;
        plan.steps.push(Step {
            key: "shiny".into(),
            kind: StepKind::Shiny,
            title: if masuda { format!("Hatch for the shiny: 1 in {}", thousands(one_in)) } else { "Get a parent from another player (Masuda)".into() },
            detail: if masuda {
                format!(
                    "With the Masuda Method{} each Egg is 1 in {}. Half the time it takes {} Eggs. Breed the perfect parents first, then hunt the shiny with them.",
                    if setup.shiny_charm { " and the Shiny Charm" } else { "" },
                    thousands(one_in),
                    thousands(breed::eggs_for(1.0 / one_in as f64, 0.5).unwrap_or(0))
                )
            } else {
                format!(
                    "A Ditto (or parent) traded from another player makes each Egg 1 in {} to be shiny instead of 1 in {}. Mark it \"Masuda\" in your Breeders.",
                    thousands(breed::shiny_one_in(breed::shiny_rolls(setup.shiny_charm, true)).round() as u32),
                    thousands(one_in)
                )
            },
            done: false,
            show: vec![],
            mv: None,
            climb: None,
        });
    }

    let mut seen = HashSet::new();
    for (item, why) in need_items {
        if seen.insert(item.clone()) {
            plan.items.push(Need { place: item_place(w, &item), item, why });
        }
    }
    plan.parents = (!a.species.is_empty()).then_some((a, b));
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn wiki() -> &'static Wiki {
        static W: OnceLock<Wiki> = OnceLock::new();
        W.get_or_init(|| Wiki::parse(include_bytes!("../assets/wiki.json")).expect("wiki"))
    }

    #[test]
    fn learning_and_families() {
        let w = wiki();
        assert!(learns(w, "BAGON", "bite", World::Platinum).contains(&Learn::Level(5)));
        assert!(learns(w, "CHARMANDER", "dragon_rush", World::Platinum).contains(&Learn::Egg));
        let f = families(w, World::Platinum);
        let gible = &f["GIBLE"];
        assert!(gible.members.contains(&"GARCHOMP".to_string()));
        assert!(gible.can_father());
        assert!(!f.contains_key("FRIGIBAX"), "newer Pokémon only in MODERN worlds");
        assert!(families(w, World::Modern).contains_key("FRIGIBAX"));
        assert!(f.contains_key("PICHU"), "babies breed through their evolutions");
        assert!(where_to_get(w, &["DITTO".to_string()], World::Platinum).is_some());
    }

    #[test]
    fn egg_move_routes() {
        let w = wiki();
        // Dragon Dance onto Charmander: Dragon-group fathers that learn it.
        let ans = move_routes(w, "CHARIZARD", "dragon_dance", World::Platinum, &[]);
        assert!(ans.egg_move);
        assert!(!ans.routes.is_empty());
        for r in &ans.routes {
            let first = &r.hops[0];
            assert_ne!(first.how, Learn::Egg, "a route starts with one that learns it");
            assert!(r.hops.len() <= 3);
            // every carrier after the first gets it as an egg move
            for h in &r.hops[1..] {
                assert!(breed::egg_moves(w, &h.species, World::Platinum).contains(&"dragon_dance".to_string()), "{}", h.species);
            }
        }
        // sorted easiest first
        assert!(ans.routes.windows(2).all(|p| p[0].score <= p[1].score));
        // not an egg move
        let ans = move_routes(w, "GARCHOMP", "earthquake", World::Platinum, &[]);
        assert!(!ans.egg_move);
        assert!(ans.routes.is_empty());
        assert!(!ans.other_ways.is_empty());
        // a breeder that knows it comes first
        let mut dad = Parent::new("SALAMENCE", Gender::Male);
        dad.moves = vec!["dragon_dance".into()];
        let b = Breeder { id: 7, parent: dad, ..Default::default() };
        let ans = move_routes(w, "CHARMANDER", "dragon_dance", World::Platinum, &[b]);
        assert_eq!(ans.routes[0].breeder, Some(7));
    }

    #[test]
    fn climb_is_sensible() {
        let w = wiki();
        let s = Setup { world: World::Platinum, ..Default::default() };
        let mut a = Parent::new("GARCHOMP", Gender::Female);
        a.ivs = [Some(0); 6];
        a.nature = Some("Jolly".into());
        a.held = Held::Everstone;
        let mut b = Parent::new("DITTO", Gender::Male);
        b.ivs = [Some(31); 6];
        b.held = Held::DestinyKnot;
        let goal = Goal { ivs: [true; 6], nature: Some("Jolly".into()), ..Default::default() };
        let c = simulate_climb(w, &a, &b, &goal, &s, 200, 42).unwrap();
        assert_eq!(c.stuck, 0);
        assert!(c.p50 >= 1 && c.p90 >= c.p50);
        // Without swapping in it would be 1 in 32 × (1/2)^5 ≈ 1 in 1,024 per
        // Egg (the 6th stat random); swapping in makes it far quicker.
        assert!(c.mean < 300.0, "{c:?}");
        // a goal the parents already hit straight away
        let easy = Goal { nature: Some("Jolly".into()), ..Default::default() };
        let c = simulate_climb(w, &a, &b, &easy, &s, 50, 1).unwrap();
        assert_eq!(c.p90, 1);
    }

    #[test]
    fn plans() {
        let w = wiki();
        let s = Setup { world: World::Platinum, hidden_abilities: true, ..Default::default() };
        let t = Target {
            species: "GARCHOMP".into(),
            nature: Some("Jolly".into()),
            ivs: [true, true, false, true, true, true],
            ability: Some("Rough Skin".into()),
            moves: vec!["outrage".into()],
            shiny: true,
            ball: Some("Dusk Ball".into()),
            ..Default::default()
        };
        let p = make_plan(w, &t, &[], &s, 60);
        assert!(p.problem.is_none());
        let kinds: Vec<StepKind> = p.steps.iter().map(|s| s.kind).collect();
        for k in [StepKind::Mother, StepKind::Ditto, StepKind::Move, StepKind::Ability, StepKind::Nature, StepKind::Climb, StepKind::Ball, StepKind::Shiny] {
            assert!(kinds.contains(&k), "{k:?}");
        }
        assert!(p.steps.iter().all(|s| !s.done), "nothing in the Breeders yet");
        assert!(p.items.iter().any(|i| i.item == "Destiny Knot"));
        assert!(p.items.iter().any(|i| i.item == "Ability Patch"));
        let (a, b) = p.parents.clone().unwrap();
        assert!(a.held == Held::Everstone || b.held == Held::Everstone);
        assert!(a.held == Held::DestinyKnot || b.held == Held::DestinyKnot);
        // with a Ditto and a mother in the Breeders those steps are done
        let mut mom = Parent::new("GARCHOMP", Gender::Female);
        mom.nature = Some("Jolly".into());
        mom.moves = vec!["outrage".into()];
        mom.ability = Slot::Hidden;
        mom.ball = "Dusk Ball".into();
        let mut dit = Parent::new("DITTO", Gender::Male);
        dit.ivs = [Some(31); 6];
        dit.masuda = true;
        let br = vec![Breeder { id: 1, parent: mom, ..Default::default() }, Breeder { id: 2, parent: dit, ..Default::default() }];
        let p = make_plan(w, &t, &br, &s, 60);
        for k in ["mother", "ditto", "move:outrage", "ability", "nature", "ball"] {
            assert!(p.steps.iter().find(|s| s.key == k).is_some_and(|s| s.done), "{k}");
        }
        let climb = p.steps.iter().find(|s| s.kind == StepKind::Climb).and_then(|s| s.climb.clone()).unwrap();
        assert!(climb.p50 > 0);
        // From nothing, with a hidden Ability wanted, the climb still gets
        // there (it assumes the Ability step is done).
        let p = make_plan(w, &t, &[], &s, 40);
        let climb = p.steps.iter().find(|s| s.kind == StepKind::Climb).and_then(|s| s.climb.clone()).unwrap();
        assert!(climb.stuck < climb.runs, "{climb:?}");
        // a Pokémon that can't breed
        let p = make_plan(w, &Target { species: "MEW".into(), ..Default::default() }, &[], &s, 10);
        assert!(p.problem.is_some());
    }
}

