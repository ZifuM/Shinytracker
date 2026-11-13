//! Breeding Planner: the Plan. Say what you want (species, stats, nature,
//! Ability, ball, egg moves, shiny) and get the steps from what's in your
//! Breeders, with the items to get and how many Eggs the climb takes
//! (simulated in the background).

use super::breed_home::{breedable, move_name, name_of, nature_label, BreedView, DAD, GOLD, MOM};
use super::team_calc::search_pick;
use super::team_home::{caps, chip, small, soft};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{World, STATS};
use crate::breed::{self, Gender, Parent};
use crate::breed_plan::{self, Plan, Step, StepKind, Target};
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, RichText, Sense, Stroke};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default)]
pub(super) struct PlanView {
    /// The plan on screen and what it was made from.
    key: String,
    plan: Option<Plan>,
    /// The full plan (with the simulated climb) being worked out.
    working: Option<(String, Receiver<Plan>)>,
}

const PLAN: Color32 = kit::C_TEAL;

impl App {
    pub(super) fn page_breed_plan(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let (setup, mut t, breeders, done, have) = {
            let f = self.setup.breed.file();
            (f.setup, f.target.clone(), f.breeders.clone(), f.plan_done.clone(), f.have_items.clone())
        };
        let before = t.clone();
        let avail = ui.available_width();
        let wide = avail >= 1000.0;
        let left_w = if wide { 400.0 } else { avail };
        let mut act: Option<(StepKind, Option<String>)> = None;
        let mut toggle_done: Option<String> = None;
        let mut toggle_have: Option<String> = None;

        // The plan, quick now and complete when the simulation is done.
        let key = format!("{:?}|{:?}|{:?}", t, setup, breeders.iter().map(|b| (&b.parent, b.id)).collect::<Vec<_>>());
        if self.setup.breed.plan.key != key {
            self.setup.breed.plan.key = key.clone();
            self.setup.breed.plan.plan = Some(breed_plan::make_plan(w, &t, &breeders, &setup, 0));
            let (tx, rx) = channel();
            let (w2, t2, b2, s2) = (w.clone(), t.clone(), breeders.clone(), setup);
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let p = breed_plan::make_plan(&w2, &t2, &b2, &s2, 120);
                let _ = tx.send(p);
                ctx.request_repaint();
            });
            self.setup.breed.plan.working = Some((key.clone(), rx));
        }
        if let Some((k, rx)) = &self.setup.breed.plan.working {
            if let Ok(p) = rx.try_recv() {
                if *k == self.setup.breed.plan.key {
                    self.setup.breed.plan.plan = Some(p);
                }
                self.setup.breed.plan.working = None;
            }
        }
        let plan = self.setup.breed.plan.plan.clone().unwrap_or_default();
        let working = self.setup.breed.plan.working.is_some();

        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    self.plan_goal(ui, w, &mut t, setup.world, setup.hidden_abilities);
                });
                let rw = ui.available_width();
                let t2 = t.clone();
                ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(rw);
                    self.plan_steps(ui, w, &t2, &plan, &done, &have, working, &mut act, &mut toggle_done, &mut toggle_have);
                });
            });
        } else {
            self.plan_goal(ui, w, &mut t, setup.world, setup.hidden_abilities);
            ui.add_space(14.0);
            self.plan_steps(ui, w, &t, &plan, &done, &have, working, &mut act, &mut toggle_done, &mut toggle_have);
        }

        if t != before {
            let f = self.setup.breed.file();
            if t.species != before.species {
                f.plan_done.clear();
            }
            f.target = t.clone();
            self.setup.breed.touch();
        }
        if let Some(k) = toggle_done {
            let f = self.setup.breed.file();
            match f.plan_done.iter().position(|x| *x == k) {
                Some(i) => {
                    f.plan_done.remove(i);
                }
                None => f.plan_done.push(k),
            }
            self.setup.breed.touch();
        }
        if let Some(k) = toggle_have {
            let f = self.setup.breed.file();
            match f.have_items.iter().position(|x| *x == k) {
                Some(i) => {
                    f.have_items.remove(i);
                }
                None => f.have_items.push(k),
            }
            self.setup.breed.touch();
        }
        match act {
            Some((StepKind::Climb, _)) => {
                if let Some((a, b)) = plan.parents.clone() {
                    let f = self.setup.breed.file();
                    f.daycare.a = a;
                    f.daycare.b = b;
                    f.daycare.goal = t.goal();
                    f.daycare.goal_set = true;
                    self.setup.breed.touch();
                    self.setup.breed.view = BreedView::DayCare;
                    self.setup.scroll_top = true;
                    self.setup.breed.flash = Some(("The Day Care is set up for the plan, items and all. Change anything that's different from what you have.".into(), Instant::now()));
                }
            }
            Some((StepKind::Move, Some(mv))) => {
                let f = self.setup.breed.file();
                f.finder_species = t.species.clone();
                f.finder_move = mv;
                self.setup.breed.touch();
                self.setup.breed.view = BreedView::Moves;
                self.setup.scroll_top = true;
            }
            Some((StepKind::Mother, _)) | Some((StepKind::Ditto, _)) => {
                let ditto = matches!(act, Some((StepKind::Ditto, _)));
                let f = self.setup.breed.file();
                let id = f.new_id();
                let mut p = Parent::new(if ditto { breed::DITTO } else { &t.species }, if ditto { Gender::Male } else { Gender::Female });
                p.gender = p.gender();
                self.setup.breed.boxv.editing = Some((None, breed::Breeder { id, parent: p, added: id / 1000, ..Default::default() }));
                self.setup.breed.view = BreedView::Breeders;
                self.setup.scroll_top = true;
            }
            Some((StepKind::Shiny, _)) => self.plan_masuda_hunt(w, &t),
            _ => {}
        }
    }

    /// The goal card.
    fn plan_goal(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, t: &mut Target, world: World, hidden_on: bool) {
        let species = breedable(w, world).into_iter().filter(|(_, id)| id != breed::DITTO).collect::<Vec<_>>();
        let names: Vec<String> = species.iter().map(|s| s.0.clone()).collect();
        super::sim_home::side_card(ui, PLAN, "WHAT YOU WANT", |ui| {
            let full = ui.available_width();
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(80.0, 80.0), Sense::hover());
                let p = ui.painter().clone();
                p.rect_filled(r, 14.0, theme::blend(kit::sunken(), PLAN, 0.08));
                if t.species.is_empty() || !self.team_sprite(&p, &t.species, t.shiny, r.center(), egui::vec2(72.0, 72.0)) {
                    p.text(r.center(), Align2::CENTER_CENTER, if t.species.is_empty() { "?" } else { "…" }, crate::fonts::sans_bold_font(22.0), MUTED());
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let cur = if t.species.is_empty() { "Pick a Pokémon".to_string() } else { name_of(w, &t.species) };
                    if let Some(n) = search_pick(ui, egui::Id::new("plan-species"), &cur, &names, (full - 100.0).min(240.0)) {
                        if let Some(id) = species.iter().find(|s| s.0 == n).map(|s| s.1.clone()) {
                            *t = Target { species: id, ivs: t.ivs, shiny: t.shiny, ..Default::default() };
                        }
                    }
                    if !t.species.is_empty() {
                        let baby = breed::baby_of(w, &t.species);
                        ui.label(small(if baby == t.species { "Hatches from an Egg as itself".to_string() } else { format!("Hatches as {}", name_of(w, &baby)) }));
                    } else {
                        ui.label(small("What are you breeding?"));
                    }
                });
            });
            if t.species.is_empty() {
                ui.add_space(8.0);
                ui.label(small("Pick a Pokémon and what it should have. The plan uses what's in your Breeders, tells you what to get, and works out how many Eggs it takes."));
                return;
            }
            ui.add_space(10.0);
            // Presets.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("QUICK"));
                if chip(ui, "Physical 5IV", false, PLAN).on_hover_text("31 in everything but Sp. Atk").clicked() {
                    t.ivs = [true, true, true, false, true, true];
                }
                if chip(ui, "Special 5IV", false, PLAN).on_hover_text("31 in everything but Attack").clicked() {
                    t.ivs = [true, false, true, true, true, true];
                }
                if chip(ui, "6IV", false, PLAN).clicked() {
                    t.ivs = [true; 6];
                }
                if chip(ui, "Clear", false, kit::C_SLATE).clicked() {
                    *t = Target { species: t.species.clone(), ..Default::default() };
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("AT 31"));
                for s in 0..6 {
                    if chip(ui, STATS[s], t.ivs[s], GOLD).clicked() {
                        t.ivs[s] = !t.ivs[s];
                    }
                }
            });
            ui.add_space(8.0);
            let lw = 78.0;
            let cw = (full - lw - 8.0).clamp(140.0, 260.0);
            let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(lw, 24.0), Sense::hover());
                    ui.painter().text(egui::pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, label, crate::fonts::sans_font(12.5), MUTED());
                    add(ui);
                });
            };
            let natures: Vec<String> = std::iter::once("Any".to_string()).chain(crate::battle::data().natures.iter().map(|n| nature_label(&n.name))).collect();
            let cur = t.nature.as_deref().map(nature_label).unwrap_or_else(|| "Any".into());
            row(ui, "Nature", &mut |ui| {
                if let Some(n) = search_pick(ui, egui::Id::new("plan-nature"), &cur, &natures, cw) {
                    t.nature = if n == "Any" { None } else { n.split_whitespace().next().map(str::to_string) };
                }
            });
            let mut abilities: Vec<(String, String)> = breed::normal_abilities(&t.species, world).into_iter().map(|a| (a.clone(), a)).collect();
            abilities.dedup();
            if let Some(h) = breed::hidden_ability(&t.species).filter(|_| hidden_on) {
                if !abilities.iter().any(|a| a.0 == h) {
                    abilities.push((format!("{h} (hidden)"), h));
                }
            }
            let cur = t.ability.as_ref().and_then(|a| abilities.iter().find(|x| x.1 == *a).map(|x| x.0.clone())).unwrap_or_else(|| "Any".into());
            row(ui, "Ability", &mut |ui| {
                egui::ComboBox::from_id_salt("plan-ability").selected_text(&cur).width(cw).show_ui(ui, |ui| {
                    if ui.selectable_label(t.ability.is_none(), "Any").clicked() {
                        t.ability = None;
                    }
                    for (l, a) in &abilities {
                        if ui.selectable_label(t.ability.as_deref() == Some(a.as_str()), l).clicked() {
                            t.ability = Some(a.clone());
                        }
                    }
                });
            });
            let balls: Vec<String> = std::iter::once("Any".to_string()).chain(crate::sim::BALLS.iter().filter(|b| !matches!(b, crate::sim::Ball::Master | crate::sim::Ball::Cherish)).map(|b| b.name().to_string())).collect();
            let cur = t.ball.clone().unwrap_or_else(|| "Any".into());
            row(ui, "Ball", &mut |ui| {
                if let Some(b) = search_pick(ui, egui::Id::new("plan-ball"), &cur, &balls, cw) {
                    t.ball = if b == "Any" { None } else { Some(b) };
                }
            });
            let ratio = breed::info(&breed::baby_of(w, &t.species)).map(|i| i.ratio).unwrap_or(4);
            if ratio > 0 && ratio < 8 {
                row(ui, "Gender", &mut |ui| {
                    for (v, l, c) in [(None, "Either", kit::C_SLATE), (Some(Gender::Female), "♀", MOM), (Some(Gender::Male), "♂", DAD)] {
                        if chip(ui, l, t.gender == v, c).clicked() {
                            t.gender = v;
                        }
                    }
                });
            }
            ui.add_space(8.0);
            ui.label(caps("EGG MOVES"));
            ui.add_space(4.0);
            let baby = breed::baby_of(w, &t.species);
            let list = breed::egg_moves(w, &baby, world);
            if !t.moves.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    let mut remove = None;
                    for (i, m) in t.moves.iter().enumerate() {
                        if chip(ui, &format!("{}  ×", move_name(w, m)), true, kit::C_TEAL).clicked() {
                            remove = Some(i);
                        }
                    }
                    if let Some(i) = remove {
                        t.moves.remove(i);
                    }
                });
            }
            if t.moves.len() < 4 {
                let opts: Vec<String> = list.iter().filter(|k| !t.moves.contains(k)).map(|k| move_name(w, k)).collect();
                if opts.is_empty() {
                    ui.label(small("It has no egg moves."));
                } else if let Some(n) = search_pick(ui, egui::Id::new("plan-move"), "+ Add an egg move", &opts, 200.0) {
                    if let Some(k) = list.iter().find(|k| move_name(w, k) == n) {
                        t.moves.push(k.clone());
                    }
                }
            }
            ui.add_space(10.0);
            kit::checkbox(ui, &mut t.shiny, RichText::new("Shiny too (the Masuda Method)").size(13.0));
        });
    }

    /// The summary, the steps and the items.
    #[allow(clippy::too_many_arguments)]
    fn plan_steps(
        &mut self,
        ui: &mut egui::Ui,
        w: &Arc<Wiki>,
        t: &Target,
        plan: &Plan,
        done: &[String],
        have: &[String],
        working: bool,
        act: &mut Option<(StepKind, Option<String>)>,
        toggle_done: &mut Option<String>,
        toggle_have: &mut Option<String>,
    ) {
        if t.species.is_empty() {
            super::sim_home::side_card(ui, PLAN, "THE PLAN", |ui| {
                ui.label(RichText::new("Pick what you're breeding").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new("The steps show up here: which parents to use, which items to give them, how to get the egg moves, and how many Eggs it takes. Steps your Breeders already cover are ticked for you.")
                            .size(13.0)
                            .color(MUTED()),
                    )
                    .wrap(),
                );
            });
            return;
        }
        if let Some(p) = &plan.problem {
            super::breed_home::note_bar(ui, kit::C_ORANGE, p, None);
        }
        let is_done = |s: &Step| s.done || done.contains(&s.key);
        let n_done = plan.steps.iter().filter(|s| is_done(s)).count();
        let climb = plan.steps.iter().find(|s| s.kind == StepKind::Climb).and_then(|s| s.climb.clone());
        let to_get = plan.items.iter().filter(|i| !have.contains(&i.item)).count();
        // Summary tiles.
        let tiles = [
            ("STEPS DONE", format!("{n_done} of {}", plan.steps.len()), "ticked from your Breeders, or by you".to_string()),
            (
                "EGGS, HALF THE TIME",
                match &climb {
                    Some(c) if c.runs == 0 || working => "…".to_string(),
                    Some(c) if c.stuck >= c.runs => "–".to_string(),
                    Some(c) => breed_plan::thousands(c.p50),
                    None => "–".to_string(),
                },
                match &climb {
                    Some(c) if c.runs == 0 || working => "working it out…".to_string(),
                    Some(c) if c.stuck >= c.runs => "these parents can't get there".to_string(),
                    Some(c) => format!("9 times in 10 within {}", breed_plan::thousands(c.p90)),
                    None => "set stats, a nature or an Ability".to_string(),
                },
            ),
            ("ITEMS TO GET", to_get.to_string(), if to_get == 0 { "you have everything".into() } else { "listed below".into() }),
        ];
        let gap = 12.0;
        let avail = ui.available_width();
        let tw = ((avail - gap * 2.0) / 3.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (label, big, sub) in &tiles {
                let (r, _) = ui.allocate_exact_size(egui::vec2(tw, 84.0), Sense::hover());
                let p = ui.painter();
                p.add(kit::card_shadow().as_shape(r, 14.0));
                p.rect_filled(r, 14.0, CARD());
                p.rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
                p.text(r.min + egui::vec2(16.0, 18.0), Align2::LEFT_CENTER, *label, crate::fonts::sans_bold_font(10.5), MUTED());
                p.text(r.min + egui::vec2(16.0, 44.0), Align2::LEFT_CENTER, big, crate::fonts::sans_bold_font(24.0), TEXT());
                let f = crate::fonts::sans_font(11.5);
                p.text(r.min + egui::vec2(16.0, 68.0), Align2::LEFT_CENTER, super::charts::clip(ui, sub, &f, tw - 28.0), f, MUTED());
            }
        });
        ui.add_space(14.0);

        // The steps.
        super::sim_home::side_card(ui, PLAN, "THE STEPS", |ui| {
            for (i, s) in plan.steps.iter().enumerate() {
                let ok = is_done(s);
                ui.horizontal_top(|ui| {
                    // number or tick
                    let (r, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::hover());
                    let p = ui.painter();
                    let warn = s.kind == StepKind::Move && s.mv.is_none();
                    let col = if ok { kit::C_GREEN } else if warn { kit::C_ORANGE } else { PLAN };
                    p.circle_filled(r.center(), 15.0, if ok { theme::blend(CARD(), col, 0.85) } else { theme::blend(CARD(), col, 0.16) });
                    p.circle_stroke(r.center(), 15.0, Stroke::new(1.5f32, theme::blend(LINE(), col, 0.8)));
                    p.text(r.center(), Align2::CENTER_CENTER, if ok { "✔".to_string() } else if warn { "!".to_string() } else { (i + 1).to_string() }, crate::fonts::sans_bold_font(13.0), if ok { Color32::WHITE } else { super::charts::ink(col) });
                    ui.add_space(6.0);
                    let tw = ui.available_width();
                    ui.allocate_ui_with_layout(egui::vec2(tw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(tw);
                        ui.add_space(6.0);
                        let title = RichText::new(&s.title).font(crate::fonts::sans_bold_font(14.5)).color(if ok { MUTED() } else { TEXT() });
                        ui.add(egui::Label::new(if ok { title.strikethrough() } else { title }).wrap());
                        if ok {
                            // done: just the line, with a way to undo a tick
                            if !s.done && ui.link(RichText::new("Not done after all").size(11.5)).clicked() {
                                *toggle_done = Some(s.key.clone());
                            }
                            return;
                        }
                        ui.add_space(2.0);
                        ui.add(egui::Label::new(RichText::new(&s.detail).size(12.5).color(MUTED())).wrap());
                        // who's involved
                        let show: Vec<&String> = s.show.iter().filter(|x| !x.is_empty()).collect();
                        if !show.is_empty() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                for (k, sp) in show.iter().enumerate() {
                                    let (rr, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                                    let p = ui.painter().clone();
                                    p.rect_filled(rr, 8.0, kit::sunken());
                                    if !self.team_sprite(&p, sp, false, rr.center(), egui::vec2(36.0, 36.0)) {
                                        p.text(rr.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(11.0), MUTED());
                                    }
                                    if k + 1 < show.len() && s.kind == StepKind::Move {
                                        ui.label(RichText::new("→").size(14.0).color(kit::C_VIOLET));
                                    }
                                }
                            });
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let label = match s.kind {
                                StepKind::Climb => Some("Set up the Day Care"),
                                StepKind::Move if !s.done && s.mv.is_some() => Some("Who can teach it"),
                                StepKind::Mother | StepKind::Ditto if !s.done => Some("Add to your Breeders"),
                                StepKind::Shiny => Some("Start a Masuda hunt"),
                                _ => None,
                            };
                            if let Some(l) = label {
                                let resp = if s.kind == StepKind::Climb {
                                    kit::accent_button_w(ui, egui::Button::new(RichText::new(l).size(12.5)), 8.0, 0.0)
                                } else if s.kind == StepKind::Shiny {
                                    // the Masuda hunt method doesn't work yet
                                    ui.add_enabled_ui(false, |ui| soft(ui, l)).inner.on_disabled_hover_text("The Shiny Hunter's Masuda method isn't working yet, so this is off for now")
                                } else {
                                    soft(ui, l)
                                };
                                let resp = if s.kind == StepKind::Shiny { resp.on_hover_text("Sets the Shiny Hunter to the Masuda Method for this Pokémon (asks first if a hunt is going)") } else { resp };
                                if resp.clicked() && s.kind != StepKind::Shiny {
                                    *act = Some((s.kind, s.mv.clone()));
                                }
                            }
                            if !s.done && !warn {
                                let mut on = done.contains(&s.key);
                                if kit::checkbox(ui, &mut on, RichText::new("Done").size(12.5)).changed() {
                                    *toggle_done = Some(s.key.clone());
                                }
                            }
                        });
                    });
                });
                if i + 1 < plan.steps.len() {
                    let h = if ok { 4.0 } else { 12.0 };
                    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
                    ui.painter().line_segment([egui::pos2(r.min.x + 17.0, r.min.y - 4.0), egui::pos2(r.min.x + 17.0, r.max.y + 4.0)], Stroke::new(2.0f32, theme::blend(LINE(), PLAN, 0.3)));
                }
            }
        });

        // Items.
        if !plan.items.is_empty() {
            ui.add_space(4.0);
            super::sim_home::side_card(ui, GOLD, "ITEMS", |ui| {
                for it in &plan.items {
                    let had = have.contains(&it.item);
                    let mut on = had;
                    ui.horizontal(|ui| {
                        if kit::checkbox(ui, &mut on, RichText::new(&it.item).size(13.5).color(if had { MUTED() } else { TEXT() })).changed() {
                            *toggle_have = Some(it.item.clone());
                        }
                        ui.label(small(format!("· {}", it.why)));
                    });
                    ui.add_space(-6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(28.0);
                        ui.add(egui::Label::new(RichText::new(&it.place).size(11.5).color(MUTED())).wrap());
                    });
                    ui.add_space(8.0);
                }
            });
        }
        let _ = w;
    }

    /// The plan's shiny step: point the Shiny Hunter at this Pokémon with the
    /// Masuda Method. Never touches a hunt that's going.
    pub(super) fn plan_masuda_hunt(&mut self, w: &Arc<Wiki>, t: &Target) {
        let baby = breed::baby_of(w, &t.species);
        let name = name_of(w, &baby);
        if self.cfg.count > 0 {
            self.setup.breed.flash = Some((
                format!(
                    "Your Shiny Hunter has a hunt going ({} so far), so nothing was changed. Finish or save that hunt, then set the method to Masuda and the target to {name} on the Hunt page.",
                    self.cfg.count
                ),
                Instant::now(),
            ));
            return;
        }
        // The same path as picking a target on the Hunt page: the (empty)
        // run is put aside properly, never thrown away.
        self.set_target(crate::species::slugify(&name));
        self.cfg.hunt_method = crate::config::HuntMethod::Masuda;
        self.push_settings(true);
        self.dirty = true;
        self.setup.breed.flash = Some((format!("The Shiny Hunter is set to hunt {name} with the Masuda Method. Log each Egg on the Hatch log and tick \"Add it to your Masuda hunt too\"."), Instant::now()));
    }

    /// Open the Plan for a target (from the Pokédex or the Team Planner).
    pub(super) fn open_breed_plan(&mut self, t: Target, world: Option<World>) {
        let f = self.setup.breed.file();
        if f.target.species != t.species {
            f.plan_done.clear();
        }
        f.target = t;
        if let Some(wd) = world {
            f.setup.world = wd;
        }
        self.setup.breed.touch();
        self.setup.breed.view = BreedView::Plan;
        self.setup.breed.flash = None;
        self.go_to(super::Tab::Breed);
    }

    /// Open the Day Care with this Pokémon in it, to see who it breeds with.
    pub(super) fn open_breed_partners(&mut self, id: &str) {
        let f = self.setup.breed.file();
        let mut p = Parent::new(id, Gender::Female);
        p.gender = p.gender();
        f.daycare.a = p;
        f.daycare.b = Parent::default();
        f.daycare.goal_set = false;
        self.setup.breed.touch();
        self.setup.breed.view = BreedView::DayCare;
        self.setup.breed.flash = None;
        self.go_to(super::Tab::Breed);
    }

    /// A Team Planner set as the goal: species, nature, the stats at 31,
    /// Ability, egg moves, shiny and gender.
    pub(super) fn plan_from_member(&mut self, m: &crate::teams::Member, world: World) {
        let Some(w) = self.wiki.clone() else { return };
        let Some(p) = breed::from_member(m, world, &w) else { return };
        let baby = breed::baby_of(&w, &p.species);
        let eggs = breed::egg_moves(&w, &baby, world);
        let abilities = breed::normal_abilities(&p.species, world);
        let hidden = breed::hidden_ability(&p.species);
        let ability = if m.ability.is_empty() {
            None
        } else if abilities.len() > 1 || hidden.as_deref() == Some(m.ability.as_str()) {
            Some(m.ability.clone())
        } else {
            None
        };
        let t = Target {
            species: p.species.clone(),
            nature: p.nature.clone(),
            ivs: std::array::from_fn(|s| m.ivs[s] >= 31),
            ability,
            ball: None,
            moves: p.moves.iter().filter(|k| eggs.contains(k)).take(4).cloned().collect(),
            shiny: m.shiny,
            gender: match m.gender {
                Some('M') => Some(Gender::Male),
                Some('F') => Some(Gender::Female),
                _ => None,
            },
        };
        self.open_breed_plan(t, Some(world));
    }
}
