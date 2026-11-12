//! Battle Sim, stage 5: the battle screen. The field (a DS-style background
//! with the two bases), HP boxes, the battle text one line at a time, your
//! moves with their damage odds, your party, the bag (Poké Balls with the
//! real catch odds, medicine, battle items), and the log. The battle itself
//! is `crate::engine`.

use super::battlebacks::{self as bb, Light, Scene, SCENES};
use super::pokedex::type_color;
use super::sim_home::{thousands, OURS, RED};
use super::team_home::{caps, chip, small, soft};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World};
use crate::calc::{Status, Weather};
use crate::engine::{Action, BMon, Battle, Ev, Outcome};
use crate::sim::{self, Ai, Ball, Format, Mode};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle};
use std::collections::HashMap;
use std::time::Instant;

/// Which set of buttons is open under the battle text.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Panel {
    #[default]
    Fight,
    Party,
    Bag,
}

/// How long each line of battle text stays up.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Speed {
    Slow,
    #[default]
    Normal,
    Fast,
    Instant,
}

impl Speed {
    fn secs(self) -> f32 {
        match self {
            Speed::Slow => 1.7,
            Speed::Normal => 1.0,
            Speed::Fast => 0.45,
            Speed::Instant => 0.0,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Speed::Slow => "Slow",
            Speed::Normal => "Normal",
            Speed::Fast => "Fast",
            Speed::Instant => "Instant",
        }
    }
}

/// What the screen shows for one side while the turn plays back.
#[derive(Clone, Copy, Default, Debug)]
struct Disp {
    mon: usize,
    hp: u16,
    max: u16,
    status: Status,
    out: bool,
}

/// A battle on screen.
pub(super) struct BattleView {
    pub b: Battle,
    undo: Vec<Battle>,
    /// Log events already played.
    played: usize,
    /// By battler (0, 2 yours; 1, 3 theirs).
    disp: [Disp; 4],
    /// The weather as the screen has got to it.
    weather: Weather,
    /// The HP bars as drawn (share of max), easing toward `disp`.
    bar: [f32; 4],
    text: Option<(String, Instant)>,
    /// The whole battle in words: (turn, line).
    lines: Vec<(u32, String)>,
    flash: [Option<Instant>; 4],
    hold: Option<Instant>,
    throw: Option<(usize, Ball, u8, bool, Instant)>,
    /// This turn's choices so far (doubles: one for each of yours), and
    /// whose turn it is to choose.
    chosen: Vec<(usize, Action)>,
    cur: usize,
    /// A move waiting for you to pick its target (doubles).
    aim: Option<usize>,
    panel: Panel,
    /// An item waiting for you to pick who it's for.
    item: Option<String>,
    speed: Speed,
    odds: bool,
    /// The AI's pick for the one choosing: (move, target).
    suggest: Option<(usize, Option<usize>)>,
    scene: Scene,
    light: Light,
    tex: HashMap<String, TextureHandle>,
    pub title: String,
    pub sub: String,
    setup: sim::Setup,
    trainer: Option<sim::Trainer>,
    wild_ai: Ai,
    /// The battle as of the last time the screen caught up (the side
    /// column shows this, so it doesn't give away what's coming).
    snap: Option<Box<Battle>>,
    snap_len: usize,
    /// Leave and run the auto-sim of this battle.
    auto_after: bool,
}

/// A battle built from the setup, not started yet.
pub(super) struct Built {
    pub b: Battle,
    pub title: String,
    pub sub: String,
    pub intro: String,
    pub scene: Scene,
    pub trainer: Option<sim::Trainer>,
}

fn now() -> Instant {
    Instant::now()
}

fn hp_color(f: f32) -> Color32 {
    if f > 0.5 {
        kit::C_GREEN
    } else if f > 0.2 {
        kit::C_GOLD
    } else {
        kit::C_RED
    }
}

fn status_tag(st: Status) -> Option<(&'static str, Color32)> {
    match st {
        Status::None => None,
        Status::Paralysis => Some(("PAR", kit::C_GOLD)),
        Status::Burn => Some(("BRN", kit::C_ORANGE)),
        Status::Poison => Some(("PSN", kit::C_VIOLET)),
        Status::Toxic => Some(("TOX", kit::C_VIOLET)),
        Status::Sleep => Some(("SLP", kit::C_SLATE)),
        Status::Freeze => Some(("FRZ", kit::C_CYAN)),
    }
}

fn weather_text(w: Weather) -> &'static str {
    match w {
        Weather::Sun => "Harsh sunlight",
        Weather::Rain => "Rain",
        Weather::Sand => "Sandstorm",
        Weather::Hail => "Hail",
        Weather::None => "",
    }
}

fn outcome_title(o: Outcome, foe: &str) -> (String, Color32) {
    match o {
        Outcome::Won => ("You won!".into(), kit::C_GREEN),
        Outcome::Lost => ("You lost...".into(), kit::C_RED),
        Outcome::Caught => (format!("Gotcha! {foe} was caught!"), kit::C_GOLD),
        Outcome::Ran => ("Got away safely".into(), kit::C_SLATE),
        Outcome::Fled => (format!("The wild {foe} fled"), kit::C_SLATE),
        Outcome::Draw => ("It's a draw".into(), kit::C_SLATE),
    }
}

/// Where a wild battle happens, from the place, method and time.
fn wild_scene(place: &str, cave: bool, method: &str) -> Scene {
    let p = place.to_lowercase();
    let pick = |label: &str| SCENES.iter().copied().find(|s| s.label == label).unwrap_or(SCENES[0]);
    if matches!(method, "surf" | "old_rod" | "good_rod" | "super_rod") {
        return if cave { pick("Cave water") } else { pick("Water") };
    }
    if p.contains("distortion") {
        return pick("Distortion World");
    }
    if p.contains("ice") || p.contains("snowpoint temple") {
        return pick("Ice cave");
    }
    if cave || method == "cave" || p.contains("cave") || p.contains("tunnel") || p.contains("ruins") || p.contains("mine") {
        return pick("Cave");
    }
    if p.contains("216") || p.contains("217") || p.contains("snowpoint") || p.contains("acuity") || p.contains("summit") {
        return pick("Snow");
    }
    if p.contains("forest") || p.contains("woods") || p.contains("eterna forest") {
        return pick("Forest");
    }
    if p.contains("228") || p.contains("desert") || p.contains("beach") || p.contains("229") {
        return pick("Sand");
    }
    if p.contains("mt.") || p.contains("mount") || p.contains("206") || p.contains("211") || p.contains("227") {
        return pick("Mountain");
    }
    pick("Tall grass")
}

fn trainer_scene(class: &str) -> Scene {
    let pick = |label: &str| SCENES.iter().copied().find(|s| s.label == label).unwrap_or(SCENES[1]);
    match class {
        "Champion" => pick("Champion"),
        "Elite Four" => pick("Elite Four"),
        "Gym Leader" => pick("Gym"),
        "Team Galactic" => pick("Indoors"),
        "Swimmer" => pick("Water"),
        "Hiker" => pick("Mountain"),
        _ => pick("Field"),
    }
}

impl BattleView {
    fn sync(&mut self) {
        self.disp = [Disp { out: true, ..Default::default() }; 4];
        for b in self.b.battlers() {
            if self.b.sides[b & 1].mons.is_empty() {
                continue;
            }
            let m = self.b.act(b);
            self.disp[b] = Disp { mon: self.b.mi(b), hp: m.hp, max: m.max(), status: m.status, out: m.fainted() };
            self.bar[b] = m.hp_frac();
        }
        self.weather = self.b.weather;
    }

    /// Yours on the field, in order.
    fn mine(&self) -> Vec<usize> {
        self.b.battlers().into_iter().filter(|b| b & 1 == 0 && self.b.present(*b)).collect()
    }

    /// The next of yours that still has to choose this turn.
    fn next_to_choose(&self) -> Option<usize> {
        self.mine().into_iter().find(|b| !self.chosen.iter().any(|(x, _)| x == b))
    }

    fn busy(&self) -> bool {
        self.text.is_some() || self.played < self.b.log.len() || self.hold.is_some_and(|t| t > now())
    }

    /// Play the log forward: one line of text at a time, HP bars easing down.
    fn pump(&mut self, dt: f32) -> bool {
        let speed = self.speed.secs();
        // HP bars move at a steady pace
        let mut moving = false;
        for s in 0..4 {
            let target = self.disp[s].hp as f32 / self.disp[s].max.max(1) as f32;
            let d = target - self.bar[s];
            if d.abs() > 0.001 {
                if speed == 0.0 || self.disp[s].out {
                    self.bar[s] = target;
                } else {
                    let step = dt * 0.9 / speed.max(0.3);
                    self.bar[s] += d.signum() * d.abs().min(step);
                    moving = true;
                }
            }
        }
        loop {
            if let Some((_, t0)) = &self.text {
                if speed > 0.0 && t0.elapsed().as_secs_f32() < speed {
                    return true;
                }
                self.text = None;
            }
            if moving {
                return true;
            }
            if self.hold.is_some_and(|t| t > now()) {
                return true;
            }
            let Some(ev) = self.b.log.get(self.played).cloned() else { return false };
            self.played += 1;
            let hold = |secs: f32| if speed == 0.0 { None } else { Some(now() + std::time::Duration::from_secs_f32(secs * speed.max(0.4))) };
            match ev {
                Ev::Text(t) => {
                    let turn = self.b.turn;
                    self.lines.push((self.turn_at(), t.clone()));
                    let _ = turn;
                    self.text = Some((t, now()));
                }
                Ev::Turn(n) => self.lines.push((n, String::new())),
                Ev::Hp { side, hp, max } => {
                    self.disp[side].hp = hp;
                    self.disp[side].max = max;
                    let target = hp as f32 / max.max(1) as f32;
                    if (target - self.bar[side]).abs() > 0.001 && speed > 0.0 {
                        moving = true;
                    } else {
                        self.bar[side] = target;
                    }
                }
                Ev::Hit { side, .. } => {
                    self.flash[side] = Some(now());
                    self.hold = hold(0.32);
                }
                Ev::Status { side, status } => self.disp[side].status = status,
                Ev::SwitchIn { side, mon, hp, max, status } => {
                    self.disp[side] = Disp { mon, hp, max, status, out: false };
                    self.bar[side] = hp as f32 / max.max(1) as f32;
                    self.hold = hold(0.2);
                }
                Ev::Out { side } | Ev::Faint { side } => {
                    self.disp[side].out = true;
                    self.hold = hold(0.25);
                }
                Ev::Throw { at, ball, shakes, caught } => {
                    self.throw = Some((at, ball, shakes, caught, now()));
                    self.hold = hold(0.4 + 0.35 * shakes as f32);
                }
                Ev::Weather(w) => self.weather = w,
                Ev::End(_) => {}
            }
        }
    }

    fn turn_at(&self) -> u32 {
        self.lines.iter().rev().find(|(_, t)| t.is_empty()).map(|(n, _)| *n).unwrap_or(0)
    }

    /// Skip the rest of the playback.
    fn skip(&mut self) {
        let speed = self.speed;
        self.speed = Speed::Instant;
        self.text = None;
        self.hold = None;
        self.pump(1.0);
        self.speed = speed;
        self.text = None;
        self.hold = None;
    }

    /// Your choice for the one choosing now. Once all of yours have chosen
    /// (or you run), the AI picks for theirs and the turn plays.
    fn go(&mut self, mine: Action) {
        if self.b.result.is_some() {
            return;
        }
        let run = matches!(mine, Action::Run);
        self.chosen.retain(|(b, _)| *b != self.cur);
        self.chosen.push((self.cur, mine));
        self.suggest = None;
        self.item = None;
        self.aim = None;
        self.panel = Panel::Fight;
        // the ones that can't choose (recharging, locked in) go along
        while let Some(b) = self.next_to_choose() {
            if self.b.forced(b) {
                self.chosen.push((b, Action::Auto));
            } else {
                break;
            }
        }
        if let (Some(b), false) = (self.next_to_choose(), run) {
            self.cur = b;
            return;
        }
        self.push_undo();
        let mut all = std::mem::take(&mut self.chosen);
        let theirs: Vec<usize> = self.b.battlers().into_iter().filter(|b| b & 1 == 1 && self.b.present(*b)).collect();
        for b in theirs {
            let a = if self.b.wild { self.b.ai_action_with(b, self.wild_ai) } else { self.b.ai_action(b) };
            all.push((b, a));
        }
        self.b.play_all(all);
        self.cur = self.mine().first().copied().unwrap_or(0);
    }

    /// A move picked: in doubles a one-target move asks which foe.
    fn pick_move(&mut self, i: usize) {
        let foes = self.b.foes(self.cur);
        let single = self.b.act(self.cur).moves.get(i).is_some_and(|m| matches!(m.fx.target.as_str(), "" | "normal" | "any" | "adjacentFoe"));
        if self.b.doubles() && single && foes.len() > 1 {
            self.aim = Some(i);
        } else {
            self.go(Action::Move(i));
        }
    }

    /// Go back to the previous one's choice (doubles).
    fn back(&mut self) {
        if let Some((b, _)) = self.chosen.pop() {
            self.cur = b;
        }
        self.aim = None;
        self.item = None;
        self.suggest = None;
    }

    fn push_undo(&mut self) {
        self.undo.push(self.b.clone());
        if self.undo.len() > 60 {
            self.undo.remove(0);
        }
    }

    fn take_back(&mut self) {
        if let Some(b) = self.undo.pop() {
            self.b = b;
            self.played = self.b.log.len();
            self.text = None;
            self.hold = None;
            self.throw = None;
            self.flash = [None; 4];
            self.chosen.clear();
            self.aim = None;
            self.cur = 0;
            let keep = self.b.log.iter().filter(|e| matches!(e, Ev::Text(_) | Ev::Turn(_))).count();
            self.lines.truncate(keep);
            self.sync();
            self.suggest = None;
            self.item = None;
            self.panel = Panel::Fight;
        }
    }

    fn texture(&mut self, ctx: &egui::Context, name: &str) -> Option<TextureHandle> {
        if let Some(t) = self.tex.get(name) {
            return Some(t.clone());
        }
        let bytes = bb::bytes(name)?;
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        let t = ctx.load_texture(format!("battleback-{name}"), ci, egui::TextureOptions::NEAREST);
        self.tex.insert(name.to_string(), t.clone());
        Some(t)
    }
}

impl App {
    /// Start the battle that's set up (wild or trainer, singles).
    /// The battle that's set up, ready to start (the auto-sim plays it many times).
    pub(super) fn sim_build(&mut self) -> Result<Built, String> {
        let s = self.setup.sim.setup.clone();
        let w = self.wiki.clone().ok_or("The game data is still loading. Try again in a moment.")?;
        let team = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned()).ok_or("Pick one of your teams")?;
        let world = team.world;
        let ours: Vec<BMon> = s.picks.iter().filter_map(|i| team.members.get(*i)).filter_map(|m| BMon::from_member(m, world, Some(&w))).collect();
        if ours.is_empty() {
            return Err("None of the Pokémon you picked can battle (check their species).".into());
        }
        let (theirs, wild, title, sub, intro, scene, trainer) = match s.mode {
            Mode::Trainer => {
                let t = s.trainer.and_then(|id| self.setup.sim.file().trainers.iter().find(|x| x.id == id).cloned()).ok_or("Pick a trainer")?;
                let mons: Vec<BMon> = t.members.iter().filter_map(|m| BMon::from_member(m, t.world, Some(&w))).collect();
                if mons.is_empty() {
                    return Err(format!("{} has no Pokémon that can battle.", if t.name.is_empty() { "That trainer" } else { &t.name }));
                }
                let name = format!("{} {}", t.class, if t.name.trim().is_empty() { "Unnamed" } else { t.name.trim() }).trim().to_string();
                let sc = trainer_scene(&t.class);
                (mons, false, format!("vs {name}"), format!("{} · {} AI", world_name(t.world), ai_name(t.ai)), format!("{name} would like to battle!"), sc, Some(t))
            }
            _ => {
                s.foes.first().ok_or("Pick a wild Pokémon")?;
                // two wild ones in doubles (Platinum's double grass, Sweet Scent with a partner…)
                let take = if s.format == Format::Doubles { 2 } else { 1 };
                let mons: Vec<BMon> = s.foes.iter().take(take).filter_map(|m| BMon::from_member(m, world, Some(&w))).collect();
                if mons.is_empty() {
                    return Err("That wild Pokémon can't battle (check its species).".into());
                }
                let names: Vec<String> = mons.iter().map(|m| m.name.clone()).collect();
                let cave = self.setup.sim.foe.area.is_empty().then_some(false).unwrap_or_else(|| w.place(&self.setup.sim.foe.area).is_some_and(|p| p.cave));
                let sc = wild_scene(&s.place, cave, &s.method);
                let place = if s.place.is_empty() { "Wild encounter".to_string() } else { s.place.clone() };
                let lv: Vec<String> = mons.iter().map(|m| m.level.to_string()).collect();
                (mons, true, format!("Wild {}", names.join(" & ")), format!("{place} · Lv {}", lv.join(" & ")), format!("A wild {} appeared!", names.join(" and a wild ")), sc, None)
            }
        };
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        let doubles = s.format == Format::Doubles;
        let mut b = Battle::new_format(ours, theirs, wild, world, seed, doubles).with_wiki(&w);
        if let Some(t) = &trainer {
            b.sides[1].name = format!("{} {}", t.class, t.name.trim()).trim().to_string();
            b.sides[1].ai = t.ai;
            b.sides[1].items = t.items.clone();
        } else {
            b.sides[1].ai = Ai::Random;
            let sit = self.setup.sim.catch.sit;
            b.catch.night_or_cave = sit.night_or_cave;
            b.catch.water = sit.water;
            b.catch.caught_before = sit.caught_before;
        }
        Ok(Built { b, title, sub, intro, scene, trainer })
    }

    /// Start the battle that's set up (wild or trainer, singles or doubles).
    pub(super) fn sim_begin(&mut self) -> Result<(), String> {
        let s = self.setup.sim.setup.clone();
        let Built { mut b, title, sub, intro, scene, trainer } = self.sim_build()?;
        let wild = b.wild;
        let mut first = b.clone();
        first.log.clear();
        b.start(&intro);
        let light = match self.setup.sim.foe.time {
            Some(super::sim_foes::Time::Night) if wild => Light::Night,
            _ => Light::Day,
        };
        let mut v = BattleView {
            b,
            undo: Vec::new(),
            played: 0,
            disp: Default::default(),
            weather: Weather::None,
            bar: [0.0; 4],
            text: None,
            lines: Vec::new(),
            flash: [None; 4],
            hold: None,
            throw: None,
            chosen: Vec::new(),
            cur: 0,
            aim: None,
            panel: Panel::Fight,
            item: None,
            speed: self.setup.sim.speed,
            odds: !self.setup.sim.hide_odds,
            suggest: None,
            scene,
            light,
            tex: HashMap::new(),
            title,
            sub,
            setup: s,
            trainer,
            wild_ai: Ai::Random,
            snap: Some(Box::new(first)),
            snap_len: usize::MAX,
            auto_after: false,
        };
        // nothing's on the field until the intro plays
        v.disp = [Disp { out: true, ..Default::default() }; 4];
        self.setup.sim.battle = Some(Box::new(v));
        self.setup.scroll_top = true;
        Ok(())
    }

    /// The battle screen (instead of the setup page while a battle is on).
    pub(super) fn page_battle(&mut self, ui: &mut egui::Ui) {
        let Some(mut v) = self.setup.sim.battle.take() else { return };
        let dt = ui.ctx().input(|i| i.stable_dt).min(0.1);
        if v.pump(dt) {
            ui.ctx().request_repaint();
        }
        if !v.busy() && v.snap_len != v.b.log.len() {
            let mut c = v.b.clone();
            c.log.clear();
            v.snap = Some(Box::new(c));
            v.snap_len = v.b.log.len();
        }
        let mut leave = false;
        let mut rematch = false;
        self.battle_header(ui, &mut v, &mut leave, &mut rematch);
        ui.add_space(10.0);
        let w = ui.available_width();
        let wide = w >= 1000.0;
        if wide {
            // the field stays a comfortable size; the side column takes the rest
            let main_w = (w - 16.0 - (w * 0.30).clamp(300.0, 400.0)).min(900.0);
            let side_w = w - main_w - 16.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.allocate_ui_with_layout(egui::vec2(main_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(main_w);
                    self.battle_main(ui, &mut v, &mut leave, &mut rematch);
                });
                ui.allocate_ui_with_layout(egui::vec2(side_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(side_w);
                    self.battle_side(ui, &mut v);
                });
            });
        } else {
            self.battle_main(ui, &mut v, &mut leave, &mut rematch);
            ui.add_space(12.0);
            self.battle_side(ui, &mut v);
        }
        // keys: Space / Enter move the text on, 1–4 pick a move
        let (adv, num) = ui.ctx().input(|i| {
            let adv = i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter);
            let num = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4].iter().position(|k| i.key_pressed(*k));
            (adv, num)
        });
        if adv && v.busy() {
            v.text = None;
            v.hold = None;
        }
        // the one choosing is always one of yours that's there
        if !v.busy() && v.b.result.is_none() && !v.b.present(v.cur) {
            v.chosen.clear();
            v.cur = v.mine().first().copied().unwrap_or(0);
        }
        if let (Some(k), false) = (num, v.busy()) {
            if v.b.result.is_none() && v.b.ask.is_none() && v.panel == Panel::Fight && v.b.usable(v.cur).get(k).is_some_and(|(_, why)| why.is_none()) {
                v.pick_move(k);
            }
        }
        // locked in (Outrage, charging, recharging): the turn plays on its own
        if !v.busy() && v.b.result.is_none() && v.b.ask.is_none() && v.b.present(v.cur) && v.b.forced(v.cur) {
            v.go(Action::Auto);
            ui.ctx().request_repaint();
        }
        self.setup.sim.speed = v.speed;
        self.setup.sim.hide_odds = !v.odds;
        if rematch {
            self.setup.sim.setup = v.setup.clone();
            self.setup.sim.battle = None;
            if let Err(e) = self.sim_begin() {
                self.setup.sim.note = Some(e);
            }
            return;
        }
        if !leave {
            self.setup.sim.battle = Some(v);
        } else {
            self.setup.scroll_top = true;
            if v.auto_after {
                self.setup.sim.setup = v.setup.clone();
                self.sim_auto_start();
            }
        }
    }

    fn battle_header(&mut self, ui: &mut egui::Ui, v: &mut BattleView, leave: &mut bool, rematch: &mut bool) {
        ui.horizontal(|ui| {
            if soft(ui, "‹  Setup").on_hover_text("Leave the battle and go back to the setup").clicked() {
                *leave = true;
            }
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&v.title).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
                let turn = if v.b.turn == 0 { "Start".to_string() } else { format!("Turn {}", v.b.turn) };
                ui.label(small(format!("{} · {turn}", v.sub)));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if soft(ui, "Rematch").on_hover_text("Start this battle again from the beginning").clicked() {
                    *rematch = true;
                }
                let can_undo = !v.undo.is_empty();
                if ui.add_enabled(can_undo, egui::Button::new(RichText::new("Take back").size(13.0)).fill(kit::btn_fill()).min_size(egui::vec2(0.0, 30.0))).on_hover_text("Undo the last turn and choose again").clicked() {
                    v.take_back();
                }
                ui.add_space(8.0);
                ui.menu_button(RichText::new(format!("Text: {}", v.speed.label())).size(13.0), |ui| {
                    for sp in [Speed::Slow, Speed::Normal, Speed::Fast, Speed::Instant] {
                        if ui.selectable_label(v.speed == sp, sp.label()).clicked() {
                            v.speed = sp;
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button(RichText::new(format!("Place: {}", v.scene.label)).size(13.0), |ui| {
                    for sc in SCENES {
                        if ui.selectable_label(v.scene == sc, sc.label).clicked() {
                            v.scene = sc;
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    for (l, n) in [(Light::Day, "Day"), (Light::Evening, "Evening"), (Light::Night, "Night")] {
                        if ui.selectable_label(v.light == l, n).clicked() {
                            v.light = l;
                            ui.close_menu();
                        }
                    }
                });
            });
        });
    }

    // ───────────────────────────── the main column ─────────────────────────────

    fn battle_main(&mut self, ui: &mut egui::Ui, v: &mut BattleView, leave: &mut bool, rematch: &mut bool) {
        let w = ui.available_width();
        let h = (w * 288.0 / 512.0).round();
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
        self.battle_field(ui, rect, v);
        if resp.clicked() && v.busy() {
            v.text = None;
            v.hold = None;
        }
        ui.add_space(10.0);
        self.battle_message(ui, v);
        ui.add_space(10.0);
        if v.busy() {
            // nothing to choose while the turn plays
            ui.horizontal(|ui| {
                if soft(ui, "Skip  »").on_hover_text("Show the rest of this turn at once").clicked() {
                    v.skip();
                }
                ui.add_space(6.0);
                ui.label(small("or click the field / press Space to move the text on."));
            });
            return;
        }
        v.sync();
        if let Some(r) = v.b.result {
            self.battle_end(ui, v, r, leave, rematch);
            return;
        }
        if v.b.ask.is_some() {
            self.battle_party(ui, v, true);
            return;
        }
        // the four choices
        let can_run = v.b.wild;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let tabs: [(Panel, &str); 3] = [(Panel::Fight, "Fight"), (Panel::Bag, "Bag"), (Panel::Party, "Pokémon")];
            for (p, label) in tabs {
                if chip(ui, label, v.panel == p, if p == Panel::Fight { RED } else { OURS }).clicked() {
                    v.panel = p;
                    v.item = None;
                }
            }
            ui.add_space(6.0);
            if can_run {
                let r = chip(ui, "Run", false, kit::C_SLATE);
                if r.clicked() {
                    v.go(Action::Run);
                }
            }
            if !v.chosen.is_empty() && soft(ui, "‹  Back").on_hover_text("Change what the one before chose").clicked() {
                v.back();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if v.panel == Panel::Fight {
                    kit::checkbox(ui, &mut v.odds, "Show damage").on_hover_text("Damage range and KO chance on each move, against the Pokémon in front of you");
                    if soft(ui, "What would the AI do?").on_hover_text("Ask the Expert AI which move it would pick here").clicked() {
                        let mut probe = v.b.clone();
                        v.suggest = Some(match probe.ai_action_with(v.cur, Ai::Expert) {
                            Action::Move(k) => (k, None),
                            Action::At(k, t) => (k, Some(t)),
                            _ => (usize::MAX, None),
                        });
                    }
                }
            });
        });
        ui.add_space(10.0);
        match v.panel {
            Panel::Fight if v.aim.is_some() => self.battle_targets(ui, v),
            Panel::Fight => self.battle_moves(ui, v),
            Panel::Party => self.battle_party(ui, v, false),
            Panel::Bag => self.battle_bag(ui, v),
        }
    }

    /// Doubles: which foe the move is for, with the damage on each.
    fn battle_targets(&mut self, ui: &mut egui::Ui, v: &mut BattleView) {
        let Some(i) = v.aim else { return };
        let me = v.b.act(v.cur).clone();
        let mv = me.moves.get(i).cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} on which one?", mv.name)).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if soft(ui, "Cancel").clicked() {
                    v.aim = None;
                }
            });
        });
        ui.add_space(8.0);
        let foes = v.b.foes(v.cur);
        let w = ui.available_width();
        let tile_w = ((w - 10.0) / 2.0).max(160.0);
        let mut pick: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
            for t in foes {
                let m = v.b.act(t).clone();
                let (r, resp) = ui.allocate_exact_size(egui::vec2(tile_w, 64.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                p.rect_filled(r, kit::r(10.0), theme::blend(CARD(), RED, 0.06 + 0.12 * ht));
                p.rect_stroke(r, kit::r(10.0), Stroke::new(1.0f32, theme::blend(LINE(), RED, 0.4 + 0.4 * ht)));
                if !self.team_sprite(&p, &m.species, m.shiny, r.min + egui::vec2(32.0, 32.0), egui::vec2(52.0, 52.0)) {
                    p.text(r.min + egui::vec2(32.0, 32.0), Align2::CENTER_CENTER, m.name.chars().take(2).collect::<String>(), kit::font_bold(14.0), MUTED());
                }
                p.text(r.min + egui::vec2(64.0, 12.0), Align2::LEFT_TOP, format!("{} · Lv {}", m.name, m.level), crate::fonts::sans_bold_font(13.5), TEXT());
                let line = match v.b.estimate_vs(v.cur, i, t) {
                    Some((lo, hi)) if hi > 0.0 => {
                        let hp = m.hp_frac();
                        let pct = |x: f32| ((x * 100.0).round() as u32).min(100);
                        if lo >= hp {
                            "KOs".to_string()
                        } else if hi >= hp {
                            format!("{}–{}% · may KO", pct(lo), pct(hi))
                        } else {
                            format!("{}–{}%", pct(lo), pct(hi))
                        }
                    }
                    Some(_) => "No effect".into(),
                    None => format!("{}% HP", (m.hp_frac() * 100.0).ceil()),
                };
                p.text(r.min + egui::vec2(64.0, 36.0), Align2::LEFT_TOP, line, kit::font_small(12.0), MUTED());
                if v.suggest.is_some_and(|(k, tt)| k == i && tt == Some(t)) {
                    p.text(egui::pos2(r.max.x - 10.0, r.min.y + 12.0), Align2::RIGHT_TOP, "AI PICK", kit::font_bold(9.5), super::charts::ink(kit::C_GOLD));
                }
                if resp.clicked() {
                    pick = Some(t);
                }
            }
        });
        if let Some(t) = pick {
            v.go(Action::At(i, t));
        }
    }

    fn battle_field(&mut self, ui: &mut egui::Ui, rect: Rect, v: &mut BattleView) {
        let ctx = ui.ctx().clone();
        let p = ui.painter_at(rect);
        let k = rect.width() / 512.0;
        let at = |x: f32, y: f32| rect.min + egui::vec2(x * k, y * k);
        let uv = Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0));
        let bg = bb::file(v.scene.bg, "bg", v.light);
        let b0 = bb::file(v.scene.base, "base0", v.light);
        let b1 = bb::file(v.scene.base, "base1", v.light);
        let round = kit::r(14.0);
        if let Some(t) = v.texture(&ctx, &bg) {
            p.image(t.id(), rect, uv, Color32::WHITE);
        } else {
            p.rect_filled(rect, round, kit::sunken());
        }
        // the bases: theirs centred at (384, 176), yours at (128, 304) from the bottom
        if let Some(t) = v.texture(&ctx, &b1) {
            p.image(t.id(), Rect::from_min_max(at(256.0, 112.0), at(512.0, 240.0)), uv, Color32::WHITE);
        }
        if let Some(t) = v.texture(&ctx, &b0) {
            p.image(t.id(), Rect::from_min_max(at(-128.0, 240.0), at(384.0, 304.0)), uv, Color32::WHITE);
        }
        // weather tint
        let wt = match v.weather {
            Weather::Rain => Some(Color32::from_rgba_unmultiplied(30, 60, 130, 34)),
            Weather::Sun => Some(Color32::from_rgba_unmultiplied(255, 170, 60, 26)),
            Weather::Sand => Some(Color32::from_rgba_unmultiplied(170, 120, 50, 30)),
            Weather::Hail => Some(Color32::from_rgba_unmultiplied(220, 235, 255, 34)),
            Weather::None => None,
        };
        if let Some(c) = wt {
            p.rect_filled(rect, 0.0, c);
        }
        // the Pokémon (the far ones first)
        let dbl = v.b.doubles();
        for s in [3usize, 1, 2, 0] {
            let d = v.disp[s];
            if d.out || (s >> 1) >= v.b.sides[s & 1].slots.len() {
                continue;
            }
            // inside a ball while it shakes
            if v.throw.is_some_and(|t| t.0 == s) {
                continue;
            }
            let Some(m) = v.b.sides[s & 1].mons.get(d.mon).cloned() else { continue };
            // the hit indicator: a few quick blinks
            let blink = v.flash[s].map(|t| t.elapsed().as_secs_f32()).filter(|e| *e < 0.36);
            if blink.is_some() {
                ctx.request_repaint();
            }
            if blink.is_some_and(|e| ((e / 0.06) as u32) % 2 == 0) {
                continue;
            }
            let Some(sp) = self.team_species(&m.species) else { continue };
            let (x, y, size) = spot(s, dbl);
            let size = egui::vec2(size * k, size * k);
            // theirs stands further off, so it's drawn smaller
            let drawn = if s & 1 == 1 {
                self.draw_sprite(&p, &sp, m.shiny, at(x, y), size)
            } else {
                self.draw_back_sprite(&p, &sp, m.shiny, at(x, y), size) || self.draw_sprite(&p, &sp, m.shiny, at(x, y - 8.0), size * 0.8)
            };
            if !drawn {
                let c = at(x, y - if s & 1 == 1 { 52.0 } else { 70.0 });
                p.text(c, Align2::CENTER_CENTER, &m.name, kit::font_bold(16.0 * k.max(0.8)), Color32::WHITE);
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        // a thrown ball
        if let Some((tb, ball, shakes, caught, t0)) = v.throw {
            let e = t0.elapsed().as_secs_f32();
            let total = 0.4 + 0.35 * shakes as f32 + 0.6;
            if e < total {
                ctx.request_repaint();
                let (x, y, _) = spot(tb, dbl);
                let c = at(x, y - 6.0);
                let tilt = if e > 0.4 && !caught || e > 0.4 { (((e - 0.4) / 0.35).floor() as i32 % 2 * 2 - 1) as f32 * 0.25 * ((e - 0.4) < 0.35 * shakes as f32) as i32 as f32 } else { 0.0 };
                super::sim_catch::draw_ball(&p, c, 13.0 * k.max(0.7), tilt, ball);
            } else {
                v.throw = None;
            }
        }
        // the HP boxes: theirs top left, yours bottom right
        let fw = if dbl { (200.0 * k).clamp(176.0, 260.0) } else { (220.0 * k).clamp(190.0, 300.0) };
        let gap = 6.0;
        let theirs: Vec<usize> = [1usize, 3].into_iter().filter(|b| (b >> 1) < v.b.sides[1].slots.len()).collect();
        for (n, b) in theirs.into_iter().enumerate() {
            if !v.disp[b].out || (v.b.result.is_some() && !dbl) {
                let r = Rect::from_min_size(at(14.0, 14.0) + egui::vec2(0.0, n as f32 * (62.0 + gap)), egui::vec2(fw, 62.0));
                self.hp_box(&p, r, v, b);
            }
        }
        let ours: Vec<usize> = [0usize, 2].into_iter().filter(|b| (b >> 1) < v.b.sides[0].slots.len()).collect();
        let k_ours = ours.len() as f32;
        // two of yours: shorter boxes, so they stay clear of their base
        let oh = if dbl { 56.0 } else { 74.0 };
        for (n, b) in ours.into_iter().enumerate() {
            if !v.disp[b].out {
                let top = rect.max.y - 10.0 * k - k_ours * oh - (k_ours - 1.0) * gap + n as f32 * (oh + gap);
                let r = Rect::from_min_size(egui::pos2(rect.max.x - fw - 14.0 * k, top), egui::vec2(fw, oh));
                self.hp_box(&p, r, v, b);
            }
        }
        // weather and field words
        let mut tags: Vec<(String, Color32)> = Vec::new();
        if v.weather != Weather::None {
            let t = if v.b.weather_turns > 0 && !v.busy() { format!("{} · {} turns", weather_text(v.weather), v.b.weather_turns) } else { weather_text(v.weather).to_string() };
            tags.push((t, kit::C_CYAN));
        }
        if v.b.trick_room > 0 {
            tags.push((format!("Trick Room · {}", v.b.trick_room), kit::C_VIOLET));
        }
        if v.b.gravity > 0 {
            tags.push((format!("Gravity · {}", v.b.gravity), kit::C_INDIGO));
        }
        let mut y = rect.min.y + 12.0;
        for (t, c) in tags {
            let g = p.layout_no_wrap(t, kit::font_bold(11.0), Color32::WHITE);
            let r = Rect::from_min_size(egui::pos2(rect.max.x - g.size().x - 26.0, y), g.size() + egui::vec2(16.0, 8.0));
            p.rect_filled(r, r.height() / 2.0, Color32::from_black_alpha(130));
            p.circle_filled(egui::pos2(r.min.x - 0.0 + 8.0, r.center().y), 0.0, c);
            p.galley(r.min + egui::vec2(8.0, 4.0), g, Color32::WHITE);
            y += 26.0;
        }
        p.rect_stroke(rect, 0.0, Stroke::new(1.0f32, LINE()));
    }

    fn hp_box(&mut self, p: &egui::Painter, r: Rect, v: &BattleView, s: usize) {
        let d = v.disp[s];
        let Some(m) = v.b.sides[s & 1].mons.get(d.mon) else { return };
        let bg = theme::blend(CARD(), Color32::BLACK, 0.06).gamma_multiply(0.94);
        p.rect_filled(r.translate(egui::vec2(0.0, 2.0)), 10.0, Color32::from_black_alpha(40));
        p.rect_filled(r, 10.0, bg);
        // in doubles, the one choosing now is outlined
        let choosing = v.b.doubles() && s == v.cur && !v.busy() && v.b.result.is_none() && v.b.ask.is_none();
        p.rect_stroke(r, 10.0, if choosing { Stroke::new(2.0f32, OURS) } else { Stroke::new(1.0f32, LINE()) });
        let x = r.min.x + 12.0;
        let mut name = m.name.clone();
        if let Some(g) = m.gender {
            name.push_str(if g == 'M' { " ♂" } else { " ♀" });
        }
        let g = p.layout_no_wrap(name, crate::fonts::sans_bold_font(14.0), TEXT());
        let nw = g.size().x;
        p.galley(egui::pos2(x, r.min.y + 8.0), g, TEXT());
        let mut tx = x + nw + 8.0;
        if let Some((t, c)) = status_tag(d.status) {
            let g = p.layout_no_wrap(t.to_string(), crate::fonts::sans_bold_font(10.0), Color32::WHITE);
            let rr = Rect::from_min_size(egui::pos2(tx, r.min.y + 10.0), g.size() + egui::vec2(10.0, 4.0));
            p.rect_filled(rr, 4.0, c);
            p.galley(rr.min + egui::vec2(5.0, 2.0), g, Color32::WHITE);
            tx = rr.max.x + 6.0;
        }
        let _ = tx;
        p.text(egui::pos2(r.max.x - 12.0, r.min.y + 9.0), Align2::RIGHT_TOP, format!("Lv {}", m.level), kit::font_bold(12.0), MUTED());
        // the bar (a little higher in a short box)
        let by = if r.height() < 60.0 { 28.0 } else { 32.0 };
        let bar = Rect::from_min_max(egui::pos2(x, r.min.y + by), egui::pos2(r.max.x - 12.0, r.min.y + by + 8.0));
        p.rect_filled(bar, 4.0, kit::sunken());
        let f = v.bar[s].clamp(0.0, 1.0);
        if f > 0.0 {
            p.rect_filled(Rect::from_min_max(bar.min, egui::pos2(bar.min.x + bar.width() * f, bar.max.y)), 4.0, hp_color(f));
        }
        if s & 1 == 0 {
            let hp = (f * d.max as f32).round() as u32;
            p.text(egui::pos2(r.max.x - 12.0, bar.max.y + if r.height() < 60.0 { 4.0 } else { 6.0 }), Align2::RIGHT_TOP, format!("{hp} / {}", d.max), kit::font_bold(13.0), TEXT());
            // stat changes, when nothing's playing
            let boosts = v.b.act(s).boosts;
            let names = ["", "Atk", "Def", "SpA", "SpD", "Spe", "Acc", "Eva"];
            let txt: Vec<String> = (1..8).filter(|i| boosts[*i] != 0).map(|i| format!("{} {:+}", names[i], boosts[i])).collect();
            if !txt.is_empty() && !v.busy() {
                p.text(egui::pos2(x, bar.max.y + if r.height() < 60.0 { 5.0 } else { 7.0 }), Align2::LEFT_TOP, txt.join("  "), kit::font_small(11.0), MUTED());
            }
        } else if !v.busy() {
            let boosts = v.b.act(s).boosts;
            let names = ["", "Atk", "Def", "SpA", "SpD", "Spe", "Acc", "Eva"];
            let txt: Vec<String> = (1..8).filter(|i| boosts[*i] != 0).map(|i| format!("{} {:+}", names[i], boosts[i])).collect();
            let pct = format!("{}%", (f * 100.0).ceil() as u32);
            let line = if txt.is_empty() { pct } else { format!("{pct}   {}", txt.join("  ")) };
            p.text(egui::pos2(x, bar.max.y + 4.0), Align2::LEFT_TOP, line, kit::font_small(11.0), MUTED());
        }
    }

    fn battle_message(&mut self, ui: &mut egui::Ui, v: &mut BattleView) {
        let text = match &v.text {
            Some((t, _)) => t.clone(),
            // between lines (HP draining, a switch) the last line stays up
            None if v.b.result.is_some() || v.busy() => v.lines.iter().rev().find(|(_, t)| !t.is_empty()).map(|(_, t)| t.clone()).unwrap_or_default(),
            None if v.b.ask.is_some() => "Which Pokémon will you send out?".to_string(),
            None => {
                let n = &v.b.act(v.cur).name;
                if v.item.is_some() {
                    "Use it on which Pokémon?".to_string()
                } else {
                    format!("What will {n} do?")
                }
            }
        };
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 64.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(r, kit::r(12.0), CARD());
        p.rect_stroke(r, kit::r(12.0), Stroke::new(1.0f32, LINE()));
        p.rect_filled(Rect::from_min_size(r.min + egui::vec2(0.0, 10.0), egui::vec2(4.0, r.height() - 20.0)), 2.0, RED);
        let g = p.layout(text, kit::font(16.0), TEXT(), w - 70.0);
        p.galley(egui::pos2(r.min.x + 20.0, r.center().y - g.size().y / 2.0), g, TEXT());
        if v.text.is_some() {
            p.text(egui::pos2(r.max.x - 22.0, r.center().y), Align2::CENTER_CENTER, "▼", kit::font(12.0), MUTED());
        }
        if resp.clicked() && v.busy() {
            v.text = None;
            v.hold = None;
        }
    }

    fn battle_moves(&mut self, ui: &mut egui::Ui, v: &mut BattleView) {
        let me = v.b.act(v.cur).clone();
        let usable = v.b.usable(v.cur);
        if me.moves.is_empty() || me.moves.iter().all(|m| m.pp == 0) {
            ui.label(small(format!("{} has no moves it can use, so it will Struggle.", me.name)));
            ui.add_space(6.0);
            if kit::accent_button_w(ui, egui::Button::new(RichText::new("Struggle").size(14.0)), 8.0, 160.0).clicked() {
                v.go(Action::Move(0));
            }
            return;
        }
        if v.suggest.is_some_and(|x| x.0 == usize::MAX) {
            ui.label(small("The AI would switch out here (or use an item)."));
            ui.add_space(6.0);
        }
        let w = ui.available_width();
        let cols = if w > 520.0 { 2 } else { 1 };
        let tile_w = (w - 10.0 * (cols as f32 - 1.0)) / cols as f32;
        let mut pick: Option<usize> = None;
        egui::Grid::new("battle-moves").num_columns(cols).spacing(egui::vec2(10.0, 10.0)).show(ui, |ui| {
            for (i, mv) in me.moves.iter().enumerate() {
                let why = usable.get(i).and_then(|x| x.1);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(tile_w, 66.0), if why.is_none() { Sense::click() } else { Sense::hover() });
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && why.is_none(), kit::QUICK);
                let col = type_color(&mv.t);
                let p = ui.painter();
                let fill = if why.is_some() { kit::sunken() } else { theme::blend(CARD(), col, 0.10 + 0.10 * ht) };
                p.rect_filled(r, kit::r(10.0), fill);
                let ring = if v.suggest.is_some_and(|x| x.0 == i) { kit::C_GOLD } else { theme::blend(LINE(), col, 0.5 + 0.3 * ht) };
                p.rect_stroke(r, kit::r(10.0), Stroke::new(if v.suggest.is_some_and(|x| x.0 == i) { 2.0f32 } else { 1.0 }, ring));
                p.rect_filled(Rect::from_min_size(r.min + egui::vec2(0.0, 10.0), egui::vec2(4.0, r.height() - 20.0)), 2.0, col);
                let ink = if why.is_some() { MUTED() } else { TEXT() };
                p.text(r.min + egui::vec2(16.0, 10.0), Align2::LEFT_TOP, format!("{}", mv.name), crate::fonts::sans_bold_font(15.0), ink);
                let tw = super::team_home::type_tag(p, r.min + egui::vec2(16.0, 46.0), &mv.t, 9.5);
                let cat = match mv.cat.as_str() {
                    "Physical" => "Physical",
                    "Special" => "Special",
                    _ => "Status",
                };
                p.text(r.min + egui::vec2(16.0 + tw + 8.0, 46.0), Align2::LEFT_CENTER, cat, kit::font_small(11.5), MUTED());
                let pp = p.text(egui::pos2(r.max.x - 12.0, r.min.y + 12.0), Align2::RIGHT_TOP, format!("PP {}/{}", mv.pp, mv.max_pp), kit::font_bold(12.0), if mv.pp == 0 { kit::C_RED } else { MUTED() });
                if v.suggest.is_some_and(|x| x.0 == i) {
                    let g = p.layout_no_wrap("AI PICK".into(), kit::font_bold(9.5), Color32::from_rgb(40, 30, 0));
                    let tag = Rect::from_min_size(egui::pos2(pp.min.x - g.size().x - 18.0, pp.center().y - g.size().y / 2.0 - 2.0), g.size() + egui::vec2(10.0, 4.0));
                    p.rect_filled(tag, 4.0, kit::C_GOLD);
                    p.galley(tag.min + egui::vec2(5.0, 2.0), g, Color32::from_rgb(40, 30, 0));
                }
                // damage odds
                if v.odds && why.is_none() {
                    // in doubles: against the foe it hurts most, by name
                    let foes = v.b.foes(v.cur);
                    let best_foe = foes.iter().copied().max_by(|a, b| {
                        let ea = v.b.estimate_vs(v.cur, i, *a).map(|x| x.1 / v.b.act(*a).hp_frac().max(0.01)).unwrap_or(0.0);
                        let eb = v.b.estimate_vs(v.cur, i, *b).map(|x| x.1 / v.b.act(*b).hp_frac().max(0.01)).unwrap_or(0.0);
                        ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let tgt = best_foe.unwrap_or_else(|| v.b.foe(v.cur));
                    let named = foes.len() > 1;
                    let line = match v.b.estimate_vs(v.cur, i, tgt) {
                        Some((lo, hi)) if hi > 0.0 => {
                            let hp = v.b.act(tgt).hp_frac();
                            let pct = |x: f32| ((x * 100.0).round() as u32).min(100);
                            let on = if named { format!(" on {}", v.b.act(tgt).name) } else { String::new() };
                            if lo >= hp {
                                format!("KOs{on}")
                            } else if hi >= hp {
                                format!("{}–{}% · may KO{on}", pct(lo), pct(hi))
                            } else {
                                format!("{}–{}%{on}", pct(lo), pct(hi))
                            }
                        }
                        Some(_) => "No effect".into(),
                        None => String::new(),
                    };
                    if !line.is_empty() {
                        p.text(egui::pos2(r.max.x - 12.0, r.min.y + 46.0), Align2::RIGHT_CENTER, line, kit::font_bold(12.0), TEXT());
                    }
                } else if let Some(w) = why {
                    p.text(egui::pos2(r.max.x - 12.0, r.min.y + 46.0), Align2::RIGHT_CENTER, w, kit::font_small(11.0), kit::C_ORANGE);
                }
                let desc = if mv.desc.is_empty() { String::new() } else { mv.desc.clone() };
                let acc = if mv.acc == 0 { "—".to_string() } else { format!("{}%", mv.acc) };
                let pw = if mv.bp == 0 { "—".to_string() } else { mv.bp.to_string() };
                let resp = resp.on_hover_text(format!("Power {pw} · Accuracy {acc}{}\n{desc}", if mv.pri != 0 { format!(" · Priority {:+}", mv.pri) } else { String::new() }));
                if resp.clicked() && why.is_none() {
                    pick = Some(i);
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
        if let Some(i) = pick {
            v.pick_move(i);
        }
    }

    fn battle_party(&mut self, ui: &mut egui::Ui, v: &mut BattleView, forced: bool) {
        let trapped = if forced || v.item.is_some() { None } else { v.b.trapped(v.cur) };
        if let Some(t) = &trapped {
            ui.label(RichText::new(t).size(13.0).color(kit::C_ORANGE));
            ui.add_space(6.0);
        }
        if let Some(it) = &v.item {
            ui.label(small(format!("Using {it}. Pick a Pokémon, or open the Bag again to cancel.")));
            ui.add_space(6.0);
        }
        let mons = v.b.sides[0].mons.clone();
        let on_field: Vec<usize> = v.b.sides[0].slots.clone();
        // a partner already picked to come in this turn
        let coming: Vec<usize> = v.chosen.iter().filter_map(|(_, a)| if let Action::Switch(k) = a { Some(*k) } else { None }).collect();
        let w = ui.available_width();
        let cols = if w > 700.0 { 3 } else if w > 440.0 { 2 } else { 1 };
        let tile_w = (w - 10.0 * (cols as f32 - 1.0)) / cols as f32;
        let mut pick: Option<usize> = None;
        let item = v.item.clone();
        egui::Grid::new("battle-party").num_columns(cols).spacing(egui::vec2(10.0, 10.0)).show(ui, |ui| {
            for (i, m) in mons.iter().enumerate() {
                let ok = match &item {
                    Some(it) if it == "Revive" || it == "Max Revive" => m.fainted(),
                    Some(_) => !m.fainted(),
                    None => !m.fainted() && !on_field.contains(&i) && !coming.contains(&i) && trapped.is_none(),
                };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(tile_w, 62.0), if ok { Sense::click() } else { Sense::hover() });
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && ok, kit::QUICK);
                let p = ui.painter().clone();
                p.rect_filled(r, kit::r(10.0), if ok { theme::blend(CARD(), OURS, 0.06 + 0.10 * ht) } else { kit::sunken() });
                p.rect_stroke(r, kit::r(10.0), Stroke::new(1.0f32, if on_field.contains(&i) && !m.fainted() { theme::blend(LINE(), OURS, 0.8) } else { LINE() }));
                if !self.team_sprite(&p, &m.species, m.shiny, r.min + egui::vec2(32.0, 31.0), egui::vec2(52.0, 52.0)) {
                    p.text(r.min + egui::vec2(32.0, 31.0), Align2::CENTER_CENTER, &m.name.chars().take(2).collect::<String>(), kit::font_bold(14.0), MUTED());
                }
                let ink = if m.fainted() { MUTED() } else { TEXT() };
                p.text(r.min + egui::vec2(64.0, 10.0), Align2::LEFT_TOP, &m.name, crate::fonts::sans_bold_font(13.5), ink);
                let mut tag = format!("Lv {}", m.level);
                if m.fainted() {
                    tag.push_str(" · fainted");
                } else if on_field.contains(&i) {
                    tag.push_str(" · in battle");
                }
                p.text(egui::pos2(r.max.x - 10.0, r.min.y + 11.0), Align2::RIGHT_TOP, tag, kit::font_small(11.0), MUTED());
                let bar = Rect::from_min_max(egui::pos2(r.min.x + 64.0, r.min.y + 34.0), egui::pos2(r.max.x - 12.0, r.min.y + 40.0));
                p.rect_filled(bar, 3.0, kit::sunken());
                let f = m.hp_frac();
                if f > 0.0 {
                    p.rect_filled(Rect::from_min_max(bar.min, egui::pos2(bar.min.x + bar.width() * f, bar.max.y)), 3.0, hp_color(f));
                }
                let mut sub = format!("{} / {}", m.hp, m.max());
                if let Some((t, _)) = status_tag(m.status) {
                    sub = format!("{sub}  ·  {t}");
                }
                p.text(egui::pos2(r.min.x + 64.0, r.min.y + 45.0), Align2::LEFT_TOP, sub, kit::font_small(11.0), MUTED());
                if resp.clicked() && ok {
                    pick = Some(i);
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
        if let Some(i) = pick {
            if forced {
                v.push_undo();
                v.b.answer(i);
                v.panel = Panel::Fight;
            } else if let Some(it) = v.item.clone() {
                v.go(Action::Item(it, i));
            } else {
                v.go(Action::Switch(i));
            }
        }
    }

    fn battle_bag(&mut self, ui: &mut egui::Ui, v: &mut BattleView) {
        let mut act: Option<Action> = None;
        if v.b.wild && v.b.foes(0).len() > 1 {
            ui.label(caps("POKÉ BALLS"));
            ui.add_space(4.0);
            ui.label(small("You can't aim a ball while there are two wild Pokémon. Knock one out first, like in the game."));
            ui.add_space(10.0);
        } else if v.b.wild {
            let them = v.b.act(v.b.foe(0)).clone();
            let rate = battle::data().species(&them.species).and_then(|sp| sim::data().catch_rate(sp.num)).unwrap_or(45);
            let mut sit = v.b.catch;
            sit.level = them.level;
            sit.turn = v.b.turn;
            sit.water_or_bug = them.types.iter().any(|t| t == "Water" || t == "Bug");
            ui.label(caps("POKÉ BALLS  ·  YOUR ODDS THIS TURN"));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                let mut balls: Vec<(Ball, f64)> = sim::BALLS
                    .iter()
                    .filter(|b| !matches!(b, Ball::Safari | Ball::Cherish))
                    .map(|b| (*b, sim::catch_chance(rate, them.max() as u32, them.hp as u32, sim::ball_mult(*b, &sit), sim::status_mult(them.status))))
                    .collect();
                balls.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                for (b, c) in balls {
                    let label = format!("{}  {}", b.name(), if c >= 0.9995 { "100%".to_string() } else { format!("{:.1}%", c * 100.0) });
                    if chip(ui, &label, false, kit::C_GOLD).clicked() {
                        act = Some(Action::Ball(b));
                    }
                }
            });
            ui.add_space(10.0);
        }
        ui.label(caps("MEDICINE"));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for it in ["Potion", "Super Potion", "Hyper Potion", "Max Potion", "Full Restore", "Full Heal", "Revive", "Max Revive"] {
                if chip(ui, it, v.item.as_deref() == Some(it), kit::C_GREEN).clicked() {
                    v.item = Some(it.to_string());
                    v.panel = Panel::Party;
                }
            }
        });
        ui.add_space(10.0);
        ui.label(caps("BATTLE ITEMS"));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for it in ["X Attack", "X Defend", "X Special", "X Sp. Def", "X Speed", "X Accuracy", "Dire Hit", "Guard Spec."] {
                if chip(ui, it, false, kit::C_INDIGO).clicked() {
                    act = Some(Action::Item(it.to_string(), v.b.mi(v.cur)));
                }
            }
        });
        ui.add_space(6.0);
        ui.label(small("The bag never runs out here. Using an item takes your turn, like in the game."));
        if let Some(a) = act {
            v.go(a);
        }
    }

    fn battle_end(&mut self, ui: &mut egui::Ui, v: &mut BattleView, r: Outcome, leave: &mut bool, rematch: &mut bool) {
        let foe = v.b.sides[1].mons.first().map(|m| m.name.clone()).unwrap_or_default();
        let (title, col) = outcome_title(r, &foe);
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 6.0, col);
                ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
            });
            ui.add_space(4.0);
            let t = &v.b.tally;
            let balls = t[0].balls.len();
            let mut bits = vec![format!("{} turn{}", v.b.turn, if v.b.turn == 1 { "" } else { "s" })];
            bits.push(format!("you dealt {} damage, took {}", thousands(t[0].damage), thousands(t[1].damage)));
            if t[0].crits + t[1].crits > 0 {
                bits.push(format!("{} critical hit{} ({} yours)", t[0].crits + t[1].crits, if t[0].crits + t[1].crits == 1 { "" } else { "s" }, t[0].crits));
            }
            if balls > 0 {
                bits.push(format!("{balls} ball{} thrown", if balls == 1 { "" } else { "s" }));
            }
            if t[0].items > 0 {
                bits.push(format!("{} item{} used", t[0].items, if t[0].items == 1 { "" } else { "s" }));
            }
            ui.label(small(bits.join(" · ")));
            // the best of your team
            if let Some(best) = v.b.sides[0].mons.iter().filter(|m| m.dealt > 0).max_by_key(|m| (m.kos, m.dealt)) {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(caps("STAR OF THE BATTLE"));
                    ui.label(RichText::new(format!("{} · {} damage{}", best.name, thousands(best.dealt), if best.kos > 0 { format!(" · {} KO{}", best.kos, if best.kos == 1 { "" } else { "s" }) } else { String::new() })).size(13.0).color(TEXT()));
                });
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if kit::accent_button_w(ui, egui::Button::new(RichText::new("Rematch").size(14.0)), 8.0, 120.0).clicked() {
                    *rematch = true;
                }
                if soft(ui, "Back to setup").clicked() {
                    *leave = true;
                }
                if !v.undo.is_empty() && soft(ui, "Take back the last turn").clicked() {
                    v.take_back();
                }
                let n = super::sim_auto::auto_count(self.setup.sim.auto_n);
                if soft(ui, &format!("Simulate {} like this", thousands(n))).on_hover_text("The AI plays this battle on both sides, many times: how often would you win?").clicked() {
                    v.auto_after = true;
                    *leave = true;
                }
            });
        });
    }

    // ───────────────────────────── the side column ─────────────────────────────

    fn battle_side(&mut self, ui: &mut egui::Ui, v: &mut BattleView) {
        let Some(snap) = v.snap.clone() else { return };
        let b = &*snap;
        // the opponent
        kit::card(ui, |ui| {
            if b.wild {
                let m = b.act(1).clone();
                ui.label(caps("THE WILD POKÉMON"));
                ui.add_space(4.0);
                ui.label(RichText::new(format!("{}  ·  Lv {}", m.name, m.level)).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.horizontal_wrapped(|ui| {
                    for t in &m.types {
                        super::pokedex::type_pill(ui, t);
                    }
                    ui.label(small(format!("{} · {}", m.ability, if m.item.is_empty() { "no item".to_string() } else { m.item.clone() })));
                });
                let rate = battle::data().species(&m.species).and_then(|sp| sim::data().catch_rate(sp.num)).unwrap_or(45);
                let mut sit = b.catch;
                sit.level = m.level;
                sit.turn = b.turn;
                sit.water_or_bug = m.types.iter().any(|t| t == "Water" || t == "Bug");
                let best = sim::BALLS.iter().filter(|b| !matches!(b, Ball::Master | Ball::Safari | Ball::Cherish)).map(|b| (*b, sim::catch_chance(rate, m.max() as u32, m.hp as u32, sim::ball_mult(*b, &sit), sim::status_mult(m.status)))).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((b, c)) = best {
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Best ball now: {} · {:.1}%", b.name(), c * 100.0)).size(13.0).color(super::charts::ink(kit::C_GOLD)));
                    ui.label(small(format!("Catch rate {rate}. Lower its HP or give it a status to raise the odds.")));
                }
            } else {
                let side = &b.sides[1];
                ui.label(caps("THE TRAINER"));
                ui.add_space(4.0);
                ui.label(RichText::new(&side.name).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for m in &side.mons {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
                        let c = if m.fainted() { MUTED() } else if m.seen { RED } else { theme::blend(RED, CARD(), 0.75) };
                        super::sim_catch::draw_ball_color(ui.painter(), r.center(), 9.0, c, m.fainted() || !m.seen);
                        if m.seen {
                            resp.on_hover_text(format!("{} · Lv {} · {}", m.name, m.level, if m.fainted() { "fainted".to_string() } else { format!("{}%", (m.hp_frac() * 100.0).ceil()) }));
                        } else {
                            resp.on_hover_text("Not seen yet");
                        }
                    }
                });
                let items: Vec<String> = side.items.iter().filter(|x| x.1 > 0).map(|x| format!("{} ×{}", x.0, x.1)).collect();
                ui.add_space(6.0);
                ui.label(small(if items.is_empty() { "No items left".to_string() } else { format!("Items: {}", items.join(", ")) }));
                if let Some(t) = &v.trainer {
                    ui.label(small(format!("{} AI: {}", ai_name(t.ai), ai_note(t.ai))));
                }
            }
        });
        ui.add_space(10.0);
        // the field
        kit::card(ui, |ui| {
            ui.label(caps("THE FIELD"));
            ui.add_space(4.0);
            let mut any = false;
            if b.weather != Weather::None {
                any = true;
                ui.label(RichText::new(format!("{}{}", weather_text(b.weather), if b.weather_turns > 0 { format!(" · {} turns left", b.weather_turns) } else { " · until it changes".to_string() })).size(13.0).color(TEXT()));
            }
            for (s, who) in [(0usize, "Your side"), (1, "Their side")] {
                let sd = &b.sides[s];
                let mut bits: Vec<String> = Vec::new();
                for (n, t) in [("Reflect", sd.reflect), ("Light Screen", sd.light_screen), ("Safeguard", sd.safeguard), ("Mist", sd.mist), ("Tailwind", sd.tailwind), ("Lucky Chant", sd.lucky_chant)] {
                    if t > 0 {
                        bits.push(format!("{n} ({t})"));
                    }
                }
                if sd.rocks {
                    bits.push("Stealth Rock".into());
                }
                if sd.spikes > 0 {
                    bits.push(format!("Spikes ×{}", sd.spikes));
                }
                if sd.tspikes > 0 {
                    bits.push(format!("Toxic Spikes ×{}", sd.tspikes));
                }
                let m = sd.act();
                if m.v.sub > 0 {
                    bits.push(format!("Substitute ({} HP)", m.v.sub));
                }
                if m.v.seeded {
                    bits.push("Leech Seed".into());
                }
                if m.v.confusion > 0 {
                    bits.push("Confused".into());
                }
                if m.v.taunt > 0 {
                    bits.push("Taunted".into());
                }
                if m.v.ingrain {
                    bits.push("Ingrain".into());
                }
                if let Some((_, by)) = &m.v.bound {
                    bits.push(format!("Trapped by {by}"));
                }
                if m.v.perish > 0 {
                    bits.push(format!("Perish {}", m.v.perish - 1));
                }
                if !bits.is_empty() {
                    any = true;
                    ui.label(RichText::new(format!("{who}: {}", bits.join(", "))).size(12.5).color(TEXT()));
                }
            }
            if !any {
                ui.label(small("Nothing on the field yet: no weather, screens or hazards."));
            }
        });
        ui.add_space(10.0);
        // the log
        kit::card(ui, |ui| {
            ui.label(caps("BATTLE LOG"));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().id_salt("battle-log").max_height(280.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (n, t) in &v.lines {
                    if t.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!("Turn {n}")).font(crate::fonts::sans_bold_font(11.5)).color(MUTED()));
                    } else {
                        ui.label(RichText::new(t).size(12.5).color(TEXT()));
                    }
                }
            });
        });
    }
}

/// Where a battler stands on the 512×288 field: (x, y of its feet, size).
fn spot(b: usize, doubles: bool) -> (f32, f32, f32) {
    match (b, doubles) {
        (1, false) => (384.0, 192.0, 150.0),
        (_, false) => (128.0, 300.0, 205.0),
        (1, true) => (336.0, 198.0, 126.0),
        (3, true) => (440.0, 184.0, 126.0),
        (0, true) => (84.0, 302.0, 178.0),
        (_, true) => (218.0, 306.0, 178.0),
    }
}

fn world_name(w: World) -> &'static str {
    match w {
        World::Platinum => "Platinum",
        _ => "Modern",
    }
}

fn ai_name(a: Ai) -> &'static str {
    match a {
        Ai::Random => "Random",
        Ai::Smart => "Smart",
        Ai::Expert => "Expert",
    }
}

fn ai_note(a: Ai) -> &'static str {
    match a {
        Ai::Random => "picks moves at random",
        Ai::Smart => "goes for strong hits and KOs, heals when low",
        Ai::Expert => "plays to win: KOs, status, set-up, switching out of bad matchups",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenes_for_places() {
        assert_eq!(wild_scene("Route 201", false, "grass").label, "Tall grass");
        assert_eq!(wild_scene("Lake Verity", false, "surf").label, "Water");
        assert_eq!(wild_scene("Wayward Cave", true, "cave").label, "Cave");
        assert_eq!(wild_scene("Route 217", false, "grass").label, "Snow");
        assert_eq!(wild_scene("Eterna Forest", false, "grass").label, "Forest");
        assert_eq!(trainer_scene("Champion").label, "Champion");
    }
}
