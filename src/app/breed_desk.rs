//! Breeding Planner: the Desk. One page to keep open while you breed: your
//! game right now on the left (the real Day Care, the Eggs you carry, what
//! your bag and party have), the goal and the best parents you own in the
//! middle, and a live feed of hatches on the right.
//! READ ONLY: the save is only ever read, and nothing about an Egg shows
//! until it hatches.

use super::breed_home::{name_of, one_in, BreedView, GOLD, MOM};
use super::sim_home::{side_card, thousands};
use super::team_home::{caps, chip, small};
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::battle::STATS;
use crate::breed::{self, Gender, Goal, Held};
use crate::breed_plan::Climb;
use crate::breed_save::{self as bs, Found, Owned, Pick, Place};
use crate::wiki::Wiki;
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use std::sync::Arc;

const LIVE: Color32 = kit::C_GREEN;

/// What the Desk worked out, kept until the save, the goal or the settings
/// change.
#[derive(Default)]
pub(super) struct DeskState {
    key: Option<String>,
    pub owned: Vec<Owned>,
    pub items: Vec<(String, u32, u32)>,
    pub found: Found,
    pub picks: Vec<Pick>,
    /// The chance with the Day Care pair as it is now (None: no pair, or it
    /// can't make this Egg).
    pub now_chance: Option<f64>,
    /// The best pair, putting better babies in as they hatch.
    pub climb: Option<Climb>,
    /// The species planned for ("" = nothing yet).
    pub baby: String,
    pub goal: Goal,
    /// The goal comes from the Day Care pair (nothing set in Plan).
    pub from_day_care: bool,
    /// The "How to" open under a pick.
    pub open: Option<usize>,
    /// When the app started (for "this session").
    pub session_start: u64,
    /// The hatch open in the feed (its log id).
    pub feed_open: Option<u64>,
    /// Why the Day Care pair in your game can't make Eggs (None: it can, or
    /// there's no pair).
    pub dc_why: Option<String>,
    /// Why no pair turned up for the goal ("" when some did).
    pub why_none: String,
    /// Catching or breeding something could change that (false: the goal
    /// can never be bred, e.g. Ditto or a legendary).
    pub why_fixable: bool,
    /// Free spaces: (party, boxes).
    pub room: (usize, usize),
}

/// "HP Atk Def SpD Spe".
fn stat_list(ivs: &[bool; 6]) -> String {
    (0..6).filter(|s| ivs[*s]).map(|s| STATS[s]).collect::<Vec<_>>().join(" ")
}

pub(super) fn goal_empty(g: &Goal) -> bool {
    !g.ivs.iter().any(|b| *b) && g.nature.is_none() && g.ability.is_none() && g.gender.is_none()
}

/// "Adamant · 5 IVs (HP Atk Def SpD Spe)".
pub(super) fn goal_text(g: &Goal) -> String {
    let mut v = Vec::new();
    if let Some(n) = &g.nature {
        v.push(n.clone());
    }
    let n = g.ivs.iter().filter(|b| **b).count();
    if n > 0 {
        v.push(format!("{n} IV{} ({})", if n == 1 { "" } else { "s" }, stat_list(&g.ivs)));
    }
    if let Some(a) = &g.ability {
        v.push(a.clone());
    }
    match g.gender {
        Some(Gender::Female) => v.push("♀".into()),
        Some(Gender::Male) => v.push("♂".into()),
        _ => {}
    }
    if v.is_empty() {
        "anything".into()
    } else {
        v.join(" · ")
    }
}

/// "Knot on Eevee · Everstone on Ditto".
fn items_text(p: &Pick, owned: &[Owned]) -> String {
    let mut v = Vec::new();
    for (k, h) in [(p.a, &p.held_a), (p.b, &p.held_b)] {
        if *h != Held::Nothing {
            let short = match h {
                Held::DestinyKnot => "Knot".to_string(),
                _ => h.name(),
            };
            v.push(format!("{short} on {}", owned[k].name()));
        }
    }
    if v.is_empty() {
        "No items needed".into()
    } else {
        v.join(" · ")
    }
}

/// "Every Egg", "35% of Eggs", "1 in 1,234".
pub(super) fn odds(p: f64) -> String {
    if p >= 0.995 {
        "Every Egg".into()
    } else if p >= 0.1 {
        format!("{} of Eggs", super::breed_home::pct(p))
    } else {
        format!("{} Eggs", one_in(p))
    }
}

fn place_short(o: &Owned) -> String {
    match o.place {
        Place::DayCare(_) => "Day Care".into(),
        Place::Party(_) => "party".into(),
        Place::Box(b, _) => format!("Box {}", b + 1),
    }
}

/// What the planner is breeding for: Plan's goal, else what the Day Care
/// pair in your game points to. (species, goal, from the Day Care)
pub(super) fn desk_goal(target: &crate::breed_plan::Target, dc: &[breed::Parent], setup: &breed::Setup, w: &Wiki) -> Option<(String, Goal, bool)> {
    if !target.species.is_empty() {
        return Some((target.species.clone(), target.goal(), false));
    }
    if dc.len() == 2 {
        if let Ok(e) = breed::egg(&dc[0], &dc[1], setup, w) {
            return Some((e.species[0].0.clone(), Goal::from_parents(&dc[0], &dc[1]), true));
        }
    }
    None
}

impl App {
    /// Work things out again when the save, the goal or the settings change.
    pub(super) fn desk_refresh(&mut self, w: &Arc<Wiki>) {
        let masuda_hunt = self.cfg.hunt_method == crate::config::HuntMethod::Masuda;
        if self.setup.breed.desk.session_start == 0 {
            self.setup.breed.desk.session_start = crate::config::now_unix();
        }
        let (target, setup) = {
            let f = self.setup.breed.file();
            (f.target.clone(), f.setup)
        };
        let key = format!("{}|{:?}|{:?}|{masuda_hunt}", self.setup.boxes.save_ms, target, setup);
        if self.setup.breed.desk.key.as_deref() == Some(key.as_str()) {
            return;
        }
        let boxes = &self.setup.boxes;
        let desk = &mut self.setup.breed.desk;
        desk.key = Some(key);
        desk.open = None;
        let (Some(save), Some(text)) = (boxes.save.as_ref(), boxes.text.as_ref()) else {
            *desk = DeskState { key: desk.key.take(), session_start: desk.session_start, feed_open: desk.feed_open, ..Default::default() };
            // no save: Plan's goal still shows (with no Pokémon to pick from)
            if let Some((baby, goal, _)) = desk_goal(&target, &[], &setup, w) {
                desk.baby = baby;
                desk.goal = goal;
            }
            return;
        };
        desk.owned = bs::owned(save, text, Some(w), setup.world);
        desk.items = bs::breeding_items(save, text);
        desk.found = bs::found(save, text);
        desk.room = bs::room(save);
        let dc: Vec<usize> = (0..desk.owned.len()).filter(|i| matches!(desk.owned[*i].place, Place::DayCare(_))).collect();
        desk.dc_why = (dc.len() == 2).then(|| breed::compat(&desk.owned[dc[0]].parent, &desk.owned[dc[1]].parent, Some(w)).err().map(|e| e.text())).flatten();
        let dc_egg = (dc.len() == 2).then(|| breed::egg(&desk.owned[dc[0]].parent, &desk.owned[dc[1]].parent, &setup, w).ok()).flatten();
        // the goal: Plan's, else what the Day Care pair points to
        let pair: Vec<breed::Parent> = dc.iter().map(|i| desk.owned[*i].parent.clone()).collect();
        match desk_goal(&target, &pair, &setup, w) {
            Some((baby, goal, from)) => {
                desk.baby = baby;
                desk.goal = goal;
                desk.from_day_care = from;
            }
            None => {
                desk.baby.clear();
                desk.goal = Goal::default();
                desk.from_day_care = false;
            }
        }
        desk.picks = if desk.baby.is_empty() { Vec::new() } else { bs::best_pairs(&desk.owned, &desk.baby, &desk.goal, &setup, &desk.items, w, 3, masuda_hunt) };
        (desk.why_none, desk.why_fixable) = if desk.picks.is_empty() && !desk.baby.is_empty() { bs::why_no_pairs(&desk.owned, &desk.baby, w) } else { (String::new(), true) };
        desk.now_chance = dc_egg
            .filter(|e| !desk.baby.is_empty() && breed::baby_of(w, &e.species[0].0) == breed::baby_of(w, &desk.baby))
            .map(|e| desk.goal.chance(&desk.owned[dc[0]].parent, &desk.owned[dc[1]].parent, &e));
        desk.climb = desk.picks.first().filter(|p| p.chance > 0.0 && p.chance < 0.2 && !goal_empty(&desk.goal)).and_then(|p| {
            let (a, b) = p.parents(&desk.owned);
            crate::breed_plan::simulate_climb(w, &a, &b, &desk.goal, &setup, 80, 7)
        });
    }

    pub(super) fn page_breed_desk(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {

        super::app_tour::here(ui.ctx(), "breed/desk");
        self.desk_refresh(w);
        let full = ui.available_width();
        let gap = 14.0;
        let col = |ui: &mut egui::Ui, wd: f32, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.allocate_ui_with_layout(vec2(wd, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(wd);
                ui.spacing_mut().item_spacing.y = gap;
                add(ui);
            });
        };
        let side = 290.0;
        if full >= 1060.0 {
            let mid = (full - 2.0 * side - 2.0 * gap).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                col(ui, side, &mut |ui| {
                    self.desk_game(ui);
                    super::app_tour::mark(ui.ctx(), "breed.game", ui.min_rect());
                });
                col(ui, mid, &mut |ui| {
                    self.desk_plan(ui, w);
                    super::app_tour::mark(ui.ctx(), "breed.plan", ui.min_rect());
                });
                col(ui, side, &mut |ui| {
                    self.desk_feed(ui, w);
                    super::app_tour::mark(ui.ctx(), "breed.feed", ui.min_rect());
                });
            });
        } else if full >= 700.0 {
            // the plan first, your game and the feed beside it
            let mid = (full - side - gap).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                col(ui, mid, &mut |ui| {
                    self.desk_plan(ui, w);
                    super::app_tour::mark(ui.ctx(), "breed.plan", ui.min_rect());
                });
                col(ui, side, &mut |ui| {
                    let tour_top = ui.cursor().min.y;
                    self.desk_game(ui);
                    super::app_tour::mark_since(ui, "breed.game", tour_top);
                    let tour_top = ui.cursor().min.y;
                    self.desk_feed(ui, w);
                    super::app_tour::mark_since(ui, "breed.feed", tour_top);
                });
            });
        } else {
            ui.spacing_mut().item_spacing.y = gap;
            let tour_top = ui.cursor().min.y;
            self.desk_plan(ui, w);
            super::app_tour::mark_since(ui, "breed.plan", tour_top);
            let tour_top = ui.cursor().min.y;
            self.desk_game(ui);
            super::app_tour::mark_since(ui, "breed.game", tour_top);
            let tour_top = ui.cursor().min.y;
            self.desk_feed(ui, w);
            super::app_tour::mark_since(ui, "breed.feed", tour_top);
        }
    }

    // ───────────────────────────── left: your game now ─────────────────────────────

    fn desk_game(&mut self, ui: &mut egui::Ui) {
        let have_save = self.setup.boxes.save.is_some() && self.setup.boxes.text.is_some();
        let mut folder = false;
        side_card(ui, LIVE, "●  YOUR GAME NOW", |ui| {
            if !have_save {
                let b = &self.setup.boxes;
                let (title, why) = match &b.error {
                    _ if b.save.is_some() => ("Your game's text wasn't found", "Names of moves, items and Abilities come from the game folder's msgdata. Check the game folder.".to_string()),
                    Some(e) if e.contains("wasn't found") => ("Your save wasn't found", "The Desk reads your game's save to see your Day Care, Eggs and Pokémon. Check the game folder is the one Platinum Together uses.".to_string()),
                    Some(e) => ("Your save couldn't be read", format!("{e} It's tried again each time the game saves.")),
                    None => ("Reading your save…", "This takes a moment.".to_string()),
                };
                ui.label(RichText::new(title).font(super::charts::bold(13.5)).color(TEXT()));
                ui.label(small(why));
                if title != "Reading your save…" {
                    ui.add_space(6.0);
                    folder = ui.add(egui::Button::new(RichText::new("Game folder settings").size(12.0)).fill(kit::btn_fill()).rounding(8.0)).clicked();
                    ui.add_space(2.0);
                    ui.label(small("Plan and the Day Care page still work with Pokémon you type in."));
                }
                return;
            }
            let (waiting, steps) = self.setup.boxes.save.as_ref().map_or((false, 0), |s| (s.egg_waiting, s.daycare_steps));
            let dc_why = self.setup.breed.desk.dc_why.clone();
            let (party_free, box_free) = self.setup.breed.desk.room;
            let dc: Vec<Owned> = self.setup.breed.desk.owned.iter().filter(|o| matches!(o.place, Place::DayCare(_))).cloned().collect();
            ui.label(caps("DAY CARE"));
            ui.add_space(4.0);
            if dc.is_empty() {
                ui.label(small("The Day Care is empty."));
            } else {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for o in &dc {
                        let (r, resp) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
                        ui.painter().rect_filled(r, 10.0, kit::sunken());
                        if !self.team_sprite(&ui.painter().clone(), &o.parent.species, o.shiny, r.center(), vec2(38.0, 38.0)) {
                            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, o.name().chars().take(2).collect::<String>(), super::charts::bold(12.0), MUTED());
                        }
                        resp.on_hover_text(format!("{} · {} · {}", o.name(), o.parent.nature.clone().unwrap_or_default(), o.ability));
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let names: Vec<String> = dc.iter().map(|o| format!("{}{}", o.name(), gender_mark(o.parent.gender()))).collect();
                        ui.label(RichText::new(names.join(" + ")).font(super::charts::bold(13.0)).color(TEXT()));
                        if let Some(why) = &dc_why {
                            ui.label(RichText::new("These two won't make Eggs").size(12.0).color(kit::C_RED));
                            ui.label(small(why.clone()));
                        } else if waiting && party_free == 0 {
                            ui.label(RichText::new("An Egg is waiting").size(12.0).color(GOLD));
                            ui.label(small("Your party is full: make room to take it."));
                        } else if waiting {
                            ui.label(RichText::new("An Egg is waiting").size(12.0).color(GOLD));
                        } else if dc.len() == 2 {
                            ui.label(small(format!("Next Egg check in about {} steps", 255u32.saturating_sub(steps as u32).max(1))));
                        } else {
                            ui.label(small("One more Pokémon to make Eggs"));
                        }
                    });
                });
                if dc.iter().any(|o| o.parent.masuda) {
                    ui.add_space(4.0);
                    let from = dc.iter().find(|o| o.parent.masuda).map(|o| o.name()).unwrap_or_default();
                    ui.label(RichText::new(format!("Masuda: {from} is from another player")).size(11.5).color(LIVE));
                }
            }
            ui.add_space(10.0);
            ui.label(caps("EGGS ON YOU"));
            ui.add_space(4.0);
            match self.eggs_now() {
                Some(en) if !en.eggs.is_empty() => {
                    for e in &en.eggs {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let (r, _) = ui.allocate_exact_size(vec2(18.0, 22.0), Sense::hover());
                            super::eggs::paint_egg(ui.painter(), r.center(), 20.0, e.frac, kit::sunken(), GOLD, MUTED());
                            let bw = (ui.available_width() - 52.0).max(40.0);
                            kit::bar(ui, bw, 6.0, e.frac);
                            ui.label(small(if e.steps <= breed::CYCLE_STEPS { "Soon!".to_string() } else { short_k(e.steps) }));
                        });
                    }
                    ui.add_space(2.0);
                    let speed = if en.speed > 1 { format!(" · {}× as fast", en.speed) } else { String::new() };
                    ui.label(small(format!("What's inside stays hidden until it hatches{speed}.")));
                }
                _ => {
                    ui.label(small("No Eggs on you."));
                }
            }
            // room for what hatches
            ui.add_space(10.0);
            ui.label(caps("ROOM"));
            ui.add_space(4.0);
            let party = if party_free == 0 { "Party full".to_string() } else { format!("{party_free} free in your party") };
            ui.label(small(format!("{party} · {} free in your boxes", thousands(box_free as u32))));
            if box_free == 0 {
                ui.label(RichText::new("Your boxes are full: hatched Pokémon can't be put away.").size(11.5).color(kit::C_RED));
            } else if box_free < 30 {
                ui.label(RichText::new(format!("Less than a box left: room for {box_free} more hatchlings.")).size(11.5).color(GOLD));
            }
            ui.add_space(6.0);
            ui.label(RichText::new(format!("As of the game's save {}", super::home::ago(self.setup.boxes.save_ms / 1000))).size(11.0).color(MUTED()));
            if let Some(e) = &self.setup.boxes.error {
                // the newest save couldn't be read: say what's shown is older
                ui.label(RichText::new(format!("The newest save couldn't be read, so this is the one before. ({e})")).size(11.0).color(GOLD));
            }
        });
        if folder {
            self.open_game_folder_settings();
        }

        if have_save {
            let f = self.setup.breed.desk.found;
            let items = self.setup.breed.desk.items.clone();
            side_card(ui, GOLD, "FOUND IN YOUR BAG & PARTY", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    for (on, l, tip) in [
                        (f.flame_body, "Flame Body", "Flame Body or Magma Armor in your party: Eggs hatch twice as fast"),
                        (f.egg_warmer, "Egg Warmer", "In your bag: Eggs hatch twice as fast"),
                        (f.shiny_charm, "Shiny Charm", "In your bag: more tries at a shiny Egg"),
                        (f.oval_charm, "Oval Charm", "In your bag: Eggs turn up more often"),
                    ] {
                        let label = if on { format!("✔  {l}") } else { l.to_string() };
                        chip(ui, &label, on, LIVE).on_hover_text(if on { tip.to_string() } else { format!("Not found. {tip}") });
                    }
                    for (name, n, held) in &items {
                        let tip = if *held > 0 { format!("{n} in all: {held} held by your Pokémon") } else { format!("{n} in your bag") };
                        chip(ui, &format!("{name} ×{n}"), true, GOLD).on_hover_text(tip);
                    }
                });
                ui.add_space(6.0);
                ui.label(small("The Day Care page's settings follow these when your bag or party changes."));
            });
        }
    }

    // ───────────────────────────── middle: the goal and best parents ─────────────────────────────

    fn desk_plan(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let baby = self.setup.breed.desk.baby.clone();
        let goal = self.setup.breed.desk.goal.clone();
        let from_dc = self.setup.breed.desk.from_day_care;
        let best = self.setup.breed.desk.picks.first().map(|p| (p.chance, p.eggs90));
        let have_save = self.setup.boxes.save.is_some() && self.setup.boxes.text.is_some();
        let (why_none, fixable) = (self.setup.breed.desk.why_none.clone(), self.setup.breed.desk.why_fixable);
        let mut go_plan = false;
        side_card(ui, MOM, "GOAL", |ui| {
            if baby.is_empty() {
                ui.label(RichText::new("Nothing to plan yet").font(super::charts::bold(15.0)).color(TEXT()));
                ui.label(small(if have_save {
                    "Put two Pokémon in the Day Care in your game, or pick what you're breeding for in Plan."
                } else {
                    "Pick what you're breeding for in Plan. Your own Pokémon show up here once your save is read."
                }));
                ui.add_space(8.0);
                go_plan = kit::accent_button_w(ui, egui::Button::new(RichText::new("Pick a goal").size(12.5)), 8.0, 0.0).clicked();
                return;
            }
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
                ui.painter().rect_filled(r, 14.0, kit::sunken());
                self.team_sprite(&ui.painter().clone(), &baby, false, r.center(), vec2(58.0, 58.0));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.label(RichText::new(format!("{} · {}", name_of(w, &baby), goal_text(&goal))).font(super::charts::bold(15.0)).color(TEXT()));
                    match best {
                        Some((p, e90)) if p > 0.0 => {
                            let eggs = e90.map(|n| format!(" · about {} Eggs for a 90% chance", thousands(n))).unwrap_or_default();
                            ui.label(small(format!("{} with the best pair you own{eggs}", odds(p))));
                        }
                        _ if !have_save => {
                            ui.label(small("Without your save, the Desk can't look through your Pokémon."));
                        }
                        // it can never be bred: say why here (no parents card below)
                        _ if !fixable => {
                            ui.label(small(why_none.clone()));
                        }
                        _ => {
                            ui.label(small("None of your Pokémon can make this Egg yet."));
                        }
                    }
                    if from_dc {
                        ui.label(RichText::new("From your Day Care pair: set exactly what you want in Plan.").size(11.5).color(MUTED()));
                    }
                });
            });
            ui.add_space(8.0);
            go_plan = ui.add(egui::Button::new(RichText::new(if from_dc { "Set a goal" } else { "Change goal" }).size(12.5)).fill(kit::btn_fill()).rounding(8.0)).clicked();
        });
        if go_plan {
            self.setup.breed.view = BreedView::Plan;
            self.setup.scroll_top = true;
        }
        if baby.is_empty() || !have_save || !fixable {
            return;
        }
        self.desk_picks(ui);
        self.desk_next(ui);
    }

    fn desk_picks(&mut self, ui: &mut egui::Ui) {
        let picks = self.setup.breed.desk.picks.clone();
        let owned = self.setup.breed.desk.owned.clone();
        let open = self.setup.breed.desk.open;
        let why_none = self.setup.breed.desk.why_none.clone();
        let mut toggle: Option<usize> = None;
        side_card(ui, MOM, "BEST PARENTS YOU OWN", |ui| {
            if picks.is_empty() {
                ui.label(RichText::new(why_none.clone()).size(12.5).color(TEXT()));
                return;
            }
            for (k, p) in picks.iter().enumerate() {
                let (a, b) = (&owned[p.a], &owned[p.b]);
                let in_now = matches!(a.place, Place::DayCare(_)) && matches!(b.place, Place::DayCare(_)) && p.held_a == a.parent.held && p.held_b == b.parent.held;
                let fill = if k == 0 { kit::tint(kit::C_VIOLET, 0.10) } else { Color32::TRANSPARENT };
                egui::Frame::none().fill(fill).rounding(10.0).inner_margin(egui::Margin::symmetric(10.0, 8.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(RichText::new(format!("{}", k + 1)).font(super::charts::bold(14.0)).color(MUTED()));
                        for o in [a, b] {
                            let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                            ui.painter().rect_filled(r, 8.0, kit::sunken());
                            if !self.team_sprite(&ui.painter().clone(), &o.parent.species, o.shiny, r.center(), vec2(28.0, 28.0)) {
                                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, o.name().chars().take(2).collect::<String>(), super::charts::bold(10.5), MUTED());
                            }
                        }
                        // the right side first (its width), then the names in what's left
                        let right_w = 190.0;
                        let text_w = (ui.available_width() - right_w).max(120.0);
                        ui.allocate_ui_with_layout(vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(text_w);
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.add(egui::Label::new(RichText::new(format!("{} ({})  +  {} ({})", a.name(), place_short(a), b.name(), place_short(b))).size(12.5).strong().color(TEXT())).truncate());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.label(small(items_text(p, &owned)));
                                if p.masuda {
                                    ui.label(RichText::new("Masuda").size(11.0).color(LIVE)).on_hover_text("A parent from another player: more tries at a shiny Egg");
                                }
                                if in_now {
                                    ui.label(RichText::new("In now").size(11.0).color(LIVE)).on_hover_text("This is your Day Care pair, holding these items");
                                }
                            });
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if open == Some(k) { "Hide" } else { "How to" };
                            let btn = egui::Button::new(RichText::new(label).size(12.0)).rounding(8.0);
                            let clicked = if k == 0 { kit::accent_button_w(ui, btn, 8.0, 64.0).clicked() } else { ui.add(btn.fill(kit::btn_fill()).min_size(vec2(64.0, 0.0))).clicked() };
                            if clicked {
                                toggle = Some(k);
                            }
                            ui.label(RichText::new(odds(p.chance)).font(super::charts::bold(13.0)).color(TEXT()));
                        });
                    });
                    if open == Some(k) {
                        let steps = self.setup.boxes.save.as_ref().map(|s| bs::how_to(p, &owned, s, &self.setup.breed.desk.items)).unwrap_or_default();
                        ui.add_space(6.0);
                        if steps.is_empty() {
                            ui.label(small("Already set up like this in your game."));
                        }
                        for (i, s) in steps.iter().enumerate() {
                            ui.label(RichText::new(format!("{}.  {s}", i + 1)).size(12.5).color(TEXT()));
                        }
                    }
                });
            }
        });
        if let Some(k) = toggle {
            let d = &mut self.setup.breed.desk;
            d.open = if d.open == Some(k) { None } else { Some(k) };
        }
    }

    fn desk_next(&mut self, ui: &mut egui::Ui) {
        let d = &self.setup.breed.desk;
        let Some(best) = d.picks.first().cloned() else { return };
        let (a, b) = (&d.owned[best.a], &d.owned[best.b]);
        let best_in = matches!(a.place, Place::DayCare(_)) && matches!(b.place, Place::DayCare(_)) && best.held_a == a.parent.held && best.held_b == b.parent.held;
        let now = d.now_chance;
        let climb = d.climb.clone();
        let empty_goal = goal_empty(&d.goal);
        side_card(ui, GOLD, "NEXT STEP", |ui| {
            if empty_goal {
                ui.label(RichText::new("Any Egg from this pair is what you want: keep hatching.").size(13.0).color(TEXT()));
                return;
            }
            if best_in {
                let eggs = best.eggs90.map(|n| format!(" About {} Eggs for a 90% chance.", thousands(n))).unwrap_or_default();
                ui.label(RichText::new(format!("Your Day Care has the best pair you own.{eggs}")).size(13.0).color(TEXT()));
            } else {
                let from = match now {
                    Some(p) if p > 0.0 => odds(p),
                    Some(_) => "no chance as it is".into(),
                    None => "your Day Care pair can't make this Egg".into(),
                };
                ui.label(RichText::new(format!("Put pair 1 in: {from} → {}. Its “How to” has the steps.", odds(best.chance))).size(13.0).color(TEXT()));
            }
            match climb {
                Some(c) if c.p50 > 0 && c.stuck * 2 < c.runs => {
                    ui.add_space(4.0);
                    ui.label(small(format!(
                        "Putting each better baby in as it hatches gets there in about {} Eggs (most runs within {}).",
                        thousands(c.p50),
                        thousands(c.p90)
                    )));
                }
                _ if best.chance < 0.001 => {
                    ui.add_space(4.0);
                    ui.label(small("Your Pokémon are a long way off this goal. Breed better parents first: Plan has it step by step (perfect-stat Ditto, the nature with an Everstone, then the stats)."));
                }
                _ => {}
            }
        });
    }

    // ───────────────────────────── right: the hatch feed ─────────────────────────────

    fn desk_feed(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let pending = self.setup.boxes.hatched;
        let goal = self.setup.breed.desk.goal.clone();
        let wanted = goal.ivs.iter().filter(|b| **b).count();
        let start = self.setup.breed.desk.session_start;
        let this_world = self.setup.world.opts.as_ref().map(|o| o.name.clone()).unwrap_or_default();
        let (recent, session, import): (Vec<breed::Hatch>, (usize, usize, usize), usize) = {
            let save_hatched: Vec<u32> = self.setup.boxes.save.as_ref().map(|s| bs::hatched(s).iter().map(|m| m.pid).collect()).unwrap_or_default();
            let f = self.setup.breed.file();
            let recent = f.log.iter().rev().take(8).cloned().collect();
            let mine: Vec<&breed::Hatch> = f.log.iter().filter(|h| h.when >= start).collect();
            let s = (mine.len(), mine.iter().filter(|h| !goal_empty(&goal) && h.hits(&goal)).count(), mine.iter().filter(|h| h.shiny).count());
            let logged: Vec<u32> = f.log.iter().map(|h| h.pid).collect();
            let import = if f.log.iter().any(|h| h.pid != 0) { 0 } else { save_hatched.iter().filter(|p| !logged.contains(p)).count() };
            (recent, s, import)
        };
        let best_score = recent.iter().map(|h| h.to_parent().goal_score(&goal)).max().unwrap_or(0);
        let mut open_log = false;
        let mut do_import = false;
        let open = self.setup.breed.desk.feed_open;
        let mut toggle: Option<u64> = None;
        let mut act: Option<(breed::Hatch, bool)> = None;
        side_card(ui, GOLD, "HATCH FEED · LIVE", |ui| {
            for _ in 0..pending.min(6) {
                feed_row(ui, |ui, r| {
                    ui.painter().rect_filled(r, 8.0, kit::sunken());
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "?", super::charts::bold(13.0), MUTED());
                }, "Hatched just now", "Details when the game saves", None);
            }
            if recent.is_empty() && pending == 0 {
                ui.label(small("Hatches show up here by themselves as they happen in your game."));
            }
            for h in &recent {
                let n31 = (0..6).filter(|s| h.ivs[*s] == Some(31) && (wanted == 0 || goal.ivs[*s])).count();
                let title = match (&h.nature, wanted) {
                    (Some(n), 0) => format!("{n} · {n31} perfect IVs"),
                    (Some(n), _) => format!("{n} · {n31} of {wanted} IVs"),
                    (None, _) => format!("{} · {n31} perfect IVs", name_of(w, &h.species)),
                };
                // a hatch from another of your worlds says which
                let elsewhere = if !h.world.is_empty() && !this_world.is_empty() && h.world != this_world { format!(" · {}", h.world) } else { String::new() };
                let sub = format!("{}{}{elsewhere}", super::home::ago(h.when), if h.shiny { " · ★ shiny" } else { "" });
                let badge = if !goal_empty(&goal) && h.hits(&goal) {
                    Some(("Goal ✓", LIVE))
                } else if best_score > 0 && h.to_parent().goal_score(&goal) == best_score {
                    Some(("Best", LIVE))
                } else {
                    None
                };
                let sp = h.species.clone();
                let shiny = h.shiny;
                let resp = feed_row(ui, |ui, r| {
                    ui.painter().rect_filled(r, 8.0, kit::sunken());
                    self.team_sprite(&ui.painter().clone(), &sp, shiny, r.center(), vec2(28.0, 28.0));
                }, &title, &sub, badge);
                if resp.on_hover_text("Click for its stats, and to add it to a team").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    toggle = Some(h.id);
                }
                if open == Some(h.id) {
                    egui::Frame::none().fill(kit::sunken()).rounding(10.0).inner_margin(egui::Margin::symmetric(10.0, 8.0)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 4.0;
                        let ivs: Vec<String> = (0..6).map(|s| match h.ivs[s] {
                            Some(v) => format!("{} {v}", STATS[s]),
                            None => format!("{} ?", STATS[s]),
                        }).collect();
                        ui.label(RichText::new(ivs.join(" · ")).size(11.5).color(TEXT()));
                        let extra = [h.ability.clone().unwrap_or_default(), h.ball.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
                        if !extra.is_empty() {
                            ui.label(small(extra));
                        }
                        ui.horizontal_wrapped(|ui| {
                            if ui.add(egui::Button::new(RichText::new("+  Add to a team").size(12.0)).fill(kit::btn_fill()).rounding(8.0)).on_hover_text("A new team in the Team Planner, with its nature, IVs, Ability and moves").clicked() {
                                act = Some((h.clone(), false));
                            }
                            if h.shiny && ui.add(egui::Button::new(RichText::new("✨  Log as a shiny").size(12.0)).fill(kit::btn_fill()).rounding(8.0)).clicked() {
                                act = Some((h.clone(), true));
                            }
                        });
                    });
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(format!("This session: {} hatched · {} hit the goal · {} shiny", session.0, session.1, session.2)).size(11.5).color(MUTED()));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                open_log = ui.add(egui::Button::new(RichText::new("Open the Hatch log").size(12.0)).fill(kit::btn_fill()).rounding(8.0)).clicked();
                if import > 0 {
                    do_import = ui
                        .add(egui::Button::new(RichText::new(format!("Add {import} past hatches")).size(12.0)).fill(kit::btn_fill()).rounding(8.0))
                        .on_hover_text("Pokémon in your save that hatched from Eggs before the planner was watching: add them to the Hatch log")
                        .clicked();
                }
            });
        });
        if open_log {
            self.setup.breed.view = BreedView::Log;
            self.setup.scroll_top = true;
        }
        if do_import {
            self.breed_import_past(w);
        }
        if let Some(id) = toggle {
            let d = &mut self.setup.breed.desk;
            d.feed_open = if d.feed_open == Some(id) { None } else { Some(id) };
        }
        match act {
            Some((h, true)) => self.log_hatch(&h),
            Some((h, false)) => self.team_add_hatch(&h, w),
            None => {}
        }
    }

    /// Add the hatched Pokémon the save has (from before the planner was
    /// watching) to the Hatch log, dated the day they hatched.
    fn breed_import_past(&mut self, w: &Arc<Wiki>) {
        let boxes = &self.setup.boxes;
        let (Some(save), Some(text)) = (boxes.save.as_ref(), boxes.text.as_ref()) else { return };
        let keys = bs::MoveKeys::new(w);
        let world = self.setup.breed.file.as_ref().map(|f| f.setup.world).unwrap_or_default();
        // before this session, so "this session" counts only what you hatch now
        let before = self.setup.breed.desk.session_start.max(1) - 1;
        let mut new: Vec<breed::Hatch> = bs::hatched(save)
            .into_iter()
            .filter_map(|m| {
                let when = m.met_date.and_then(|(y, mo, d)| unix_day(2000 + y as i32, mo as u32, d as u32)).unwrap_or(0).min(before);
                bs::to_hatch(m, text, Some(&keys), world, "", when)
            })
            .collect();
        new.sort_by_key(|h| h.when);
        let f = self.setup.breed.file();
        let logged: Vec<u32> = f.log.iter().map(|h| h.pid).collect();
        let mut n = 0;
        for mut h in new.into_iter().filter(|h| !logged.contains(&h.pid)) {
            h.id = f.new_id();
            f.log.push(h);
            n += 1;
        }
        f.log.sort_by_key(|h| h.when);
        self.setup.breed.touch();
        self.setup.breed.flash = Some((format!("Added {n} past hatches to the Hatch log."), std::time::Instant::now()));
    }

    /// After each new read of the save: log Eggs that hatched since, and
    /// follow your bag and party for the settings.
    pub(super) fn breed_sync(&mut self, w: Option<Arc<Wiki>>) {
        if !crate::apps::live(crate::apps::AppId::Breed) {
            return;
        }
        // load the planner's file first (it may not be open yet)
        self.setup.breed.file();
        let news = self.breed_sync_inner(&w);
        // the hatches worth a pop-up (under the overlay, like milestones)
        for t in news {
            self.push_milestone(t);
        }
    }

    /// The work of `breed_sync`; returns pop-up lines for new hatches.
    fn breed_sync_inner(&mut self, w: &Option<Arc<Wiki>>) -> Vec<String> {
        let mut news = Vec::new();
        // read now, not the few-seconds-old copy: the save may be a world
        // you've only just switched to
        let root = crate::gamemap::game_dir(self.cfg.game_dir_setting().as_deref());
        let world_name = std::fs::read_to_string(root.join("world-name.txt")).map(|s| s.trim().to_string()).unwrap_or_default();
        let boxes = &self.setup.boxes;
        let st = &mut self.setup.breed;
        let (Some(save), Some(text)) = (boxes.save.as_ref(), boxes.text.as_ref()) else { return news };
        let Some(f) = st.file.as_mut() else { return news };
        // each world keeps its own Eggs
        let before = (f.egg_save.clone(), f.egg_pids.len());
        bs::switch_eggs(f, &bs::save_key(save));
        let mut changed = (f.egg_save.clone(), f.egg_pids.len()) != before;
        let world = f.setup.world;
        let keys = w.as_ref().map(|w| bs::MoveKeys::new(w));
        let dc = bs::day_care(save, text, world);
        let pair = match (&w, dc.len()) {
            (Some(w), 2) => breed::family_pair(w, &dc[0], &dc[1]),
            (None, 2) => breed::pair_key(&dc[0], &dc[1]),
            _ => String::new(),
        };
        let logged: Vec<u32> = f.log.iter().map(|h| h.pid).filter(|p| *p != 0).collect();
        let s = bs::sync(save, text, keys.as_ref(), world, &f.egg_pids, &logged, crate::config::now_unix(), &pair);
        changed |= s.egg_pids != f.egg_pids;
        f.egg_pids = s.egg_pids;
        let goal = w.as_ref().and_then(|w| desk_goal(&f.target, &dc, &f.setup, w)).map(|g| g.1).filter(|g| !goal_empty(g));
        for h in &s.new {
            let name = w.as_ref().map(|w| name_of(w, &h.species)).unwrap_or_else(|| h.species.clone());
            news.extend(hatch_news(h, &name, goal.as_ref()));
        }
        for mut h in s.new {
            h.id = f.new_id();
            h.world = world_name.clone();
            f.log.push(h);
            changed = true;
        }
        let found = bs::found(save, text);
        if st.found != Some(found) {
            f.setup = bs::apply(f.setup, found);
            st.found = Some(found);
            changed = true;
        }
        if changed {
            st.touch();
        }
        news
    }

    /// A hatched baby into a team (a new one, or the first with room is
    /// picked in the Team Planner), with its nature, IVs, Ability and moves.
    fn team_add_hatch(&mut self, h: &breed::Hatch, w: &Arc<Wiki>) {
        let Some(name) = crate::battle::data().species(&h.species).map(|s| s.name.clone()) else { return };
        self.team_add_from(&name, None);
        let Some(id) = self.setup.team.open else { return };
        let slot = self.setup.team.slot;
        if let Some(m) = self.setup.team.teams().teams.iter_mut().find(|t| t.id == id).and_then(|t| t.members.get_mut(slot)) {
            m.level = 1;
            m.shiny = h.shiny;
            m.gender = match h.gender {
                Gender::Male => Some('M'),
                Gender::Female => Some('F'),
                Gender::Genderless => None,
            };
            if let Some(n) = &h.nature {
                m.nature = n.clone();
            }
            for s in 0..6 {
                if let Some(v) = h.ivs[s] {
                    m.ivs[s] = v as u16;
                }
            }
            if let Some(a) = &h.ability {
                m.ability = a.clone();
            }
            if !h.moves.is_empty() {
                m.moves = h.moves.iter().map(|k| super::breed_home::move_name(w, k)).take(4).collect();
            }
        }
        self.setup.team.touch();
    }

    /// Log a shiny that hatched, filled in from the Egg.
    fn log_hatch(&mut self, h: &breed::Hatch) {
        let slug = self.team_species(&h.species).map(|s| s.slug.to_string());
        self.open_log(slug.as_deref());
        let masuda = h.pair.contains("DITTO") || self.cfg.hunt_method == crate::config::HuntMethod::Masuda;
        let d = &mut self.setup.log.draft;
        d.when_unix = if h.when > 0 { h.when } else { d.when_unix };
        d.method = if masuda { "Masuda Method".into() } else { "Hatched from an Egg".into() };
        d.mon.level = 1;
        d.level_known = true;
        d.mon.gender = match h.gender {
            Gender::Male => Some('M'),
            Gender::Female => Some('F'),
            Gender::Genderless => None,
        };
        if let Some(n) = &h.nature {
            d.mon.nature = n.clone();
            d.nature_known = true;
        }
        if h.ivs.iter().all(|v| v.is_some()) {
            for s in 0..6 {
                d.mon.ivs[s] = h.ivs[s].unwrap_or(0) as u16;
            }
            d.stats_known = true;
        }
        if let Some(a) = &h.ability {
            d.mon.ability = a.clone();
        }
    }
}

/// The pop-up for a new hatch, if it's worth one: a shiny, or one that's
/// everything the goal asks for.
fn hatch_news(h: &breed::Hatch, name: &str, goal: Option<&Goal>) -> Option<String> {
    let name = name.to_uppercase();
    if h.shiny {
        Some(format!("SHINY {name} HATCHED!"))
    } else if goal.is_some_and(|g| !goal_empty(g) && h.hits(g)) {
        Some(format!("{name} HATCHED: GOAL MET!"))
    } else {
        None
    }
}

fn gender_mark(g: Gender) -> &'static str {
    match g {
        Gender::Male => " ♂",
        Gender::Female => " ♀",
        Gender::Genderless => "",
    }
}

fn short_k(steps: u32) -> String {
    if steps >= 10_000 {
        format!("~{:.1}k", steps as f32 / 1000.0)
    } else {
        format!("~{}", thousands(steps))
    }
}

/// One row of the feed: a picture, two lines, and a badge on the right.
fn feed_row(ui: &mut egui::Ui, pic: impl FnOnce(&mut egui::Ui, Rect), title: &str, sub: &str, badge: Option<(&str, Color32)>) -> egui::Response {
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        pic(ui, r);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(RichText::new(title).size(12.5).strong().color(TEXT()));
            ui.label(RichText::new(sub).size(11.0).color(MUTED()));
        });
        if let Some((t, c)) = badge {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let g = ui.painter().layout_no_wrap(t.to_string(), super::charts::bold(11.0), c);
                let (r, _) = ui.allocate_exact_size(g.size() + vec2(16.0, 8.0), Sense::hover());
                ui.painter().rect_filled(r, 10.0, kit::tint(c, 0.14));
                ui.painter().galley(pos2(r.min.x + 8.0, r.center().y - g.size().y / 2.0), g, c);
            });
        }
    });
    let resp = ui.interact(row.response.rect, ui.id().with(("feed-row", title, sub)), Sense::click());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, LINE()));
    resp
}

/// The start of a date, as unix seconds (UTC; close enough for "when").
fn unix_day(y: i32, m: u32, d: u32) -> Option<u64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // days from 1970-01-01 (civil from days, Howard Hinnant)
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe as u32 * 365 + yoe as u32 / 4 - yoe as u32 / 100 + doy;
    let days = era as i64 * 146_097 + doe as i64 - 719_468;
    Some((days * 86_400).max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(unix_day(1970, 1, 1), Some(0));
        assert_eq!(unix_day(2026, 11, 15), Some(1_794_700_800));
        assert_eq!(unix_day(2026, 13, 1), None);
    }

    #[test]
    fn hatch_pop_ups() {
        let g = Goal { ivs: [true, false, false, false, false, false], nature: Some("Adamant".into()), ..Default::default() };
        let mut h = breed::Hatch { species: "EEVEE".into(), nature: Some("Adamant".into()), ivs: [Some(31); 6], ..Default::default() };
        assert_eq!(hatch_news(&h, "Eevee", Some(&g)).as_deref(), Some("EEVEE HATCHED: GOAL MET!"));
        h.nature = Some("Bold".into());
        assert_eq!(hatch_news(&h, "Eevee", Some(&g)), None);
        assert_eq!(hatch_news(&h, "Eevee", Some(&Goal::default())), None, "an empty goal isn't news");
        h.shiny = true;
        assert_eq!(hatch_news(&h, "Eevee", None).as_deref(), Some("SHINY EEVEE HATCHED!"));
    }

    #[test]
    fn goal_words() {
        let g = Goal { ivs: [true, true, true, false, true, true], nature: Some("Adamant".into()), ..Default::default() };
        assert_eq!(goal_text(&g), "Adamant · 5 IVs (HP Atk Def SpD Spe)");
        assert_eq!(goal_text(&Goal::default()), "anything");
    }
}
