//! Breeding Planner: the Day Care. Put two Pokémon in, see what the Egg
//! will be like (stats, nature, Ability, ball, moves, gender, shiny odds and
//! hatch steps), how many Eggs it takes to get the one you want, and what
//! would make it quicker. Nothing here reads the game: you enter the parents.

use super::sim_home::{side_card, thousands};
use super::team_calc::search_pick;
use super::team_home::{caps, chip, small};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World, STATS};
use crate::breed::{self, BreedFile, Compat, Egg, Gender, Goal, Held, Parent, Setup, Slot, Why};
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) const MOM: Color32 = kit::C_PINK;
pub(super) const DAD: Color32 = kit::C_BLUE;
pub(super) const GOLD: Color32 = kit::C_GOLD;
pub(super) const EGG: Color32 = Color32::from_rgb(244, 214, 150);

#[derive(Default)]
pub(super) struct BreedState {
    pub file: Option<BreedFile>,
    pub load_error: Option<String>,
    pub dirty: Option<Instant>,
    pub saved_once: bool,
    pub save_error: Option<String>,
    /// The "not confirmed" list under the Egg is open.
    pub notes_open: bool,
    /// Filter for the partner finder.
    pub partner_q: String,
    pub view: BreedView,
    /// The Breeders and Hatch log pages.
    pub boxv: super::breed_box::BoxView,
    /// A short "done" message: (text, when).
    pub flash: Option<(String, Instant)>,
    pub moves: super::breed_moves::MovesView,
    pub plan: super::breed_planner::PlanView,
}

/// The Breeding Planner's pages.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum BreedView {
    #[default]
    Plan,
    DayCare,
    Moves,
    Breeders,
    Log,
}

impl BreedState {
    pub fn file(&mut self) -> &mut BreedFile {
        if self.file.is_none() {
            let (f, note) = breed::load();
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(800) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match breed::save(f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                self.dirty = Some(Instant::now());
            }
        }
    }
}

// ───────────────────────────── small helpers ─────────────────────────────

pub(super) fn name_of(w: &Wiki, id: &str) -> String {
    w.mon(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string())
}

pub(super) fn move_name(w: &Wiki, key: &str) -> String {
    w.move_info(key).map(|m| m.n.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| key.replace('_', " "))
}

/// "37.5%", "100%", "<0.1%".
pub(super) fn pct(p: f64) -> String {
    let v = p * 100.0;
    if p <= 0.0 {
        "0%".into()
    } else if v >= 99.95 {
        "100%".into()
    } else if v < 0.1 {
        "<0.1%".into()
    } else if v < 10.0 {
        format!("{v:.1}%")
    } else {
        format!("{v:.0}%")
    }
}

/// "1 in 1,234" (per Egg).
pub(super) fn one_in(p: f64) -> String {
    if p >= 0.9995 {
        "Every Egg".into()
    } else {
        format!("1 in {}", thousands((1.0 / p).round().max(1.0) as u32))
    }
}

/// What to call a parent, and its colour.
pub(super) fn role(p: &Parent, i: usize) -> (String, Color32) {
    if p.species.is_empty() {
        return (if i < 2 { format!("PARENT {}", i + 1) } else { "NEW BREEDER".into() }, kit::C_SLATE);
    }
    if p.is_ditto() {
        return ("DITTO".into(), kit::C_VIOLET);
    }
    match p.gender() {
        Gender::Female => ("♀  MOTHER".into(), MOM),
        Gender::Male => ("♂  FATHER".into(), DAD),
        Gender::Genderless => ("GENDERLESS PARENT".into(), kit::C_SLATE),
    }
}

/// The species you can put in the Day Care (Ditto first), as (name, id).
pub(super) fn breedable(w: &Wiki, world: World) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = w
        .all()
        .filter(|m| world == World::Modern || !m.modern)
        .filter(|m| breed::info(&m.id).is_some_and(|i| !i.no_eggs()))
        .map(|m| (m.name.clone(), m.id.clone()))
        .collect();
    if let Some(i) = v.iter().position(|(_, id)| id == breed::DITTO) {
        let d = v.remove(i);
        v.insert(0, d);
    }
    v
}

fn held_label(h: &Held) -> String {
    match h {
        Held::Nothing => "Nothing".into(),
        Held::Power(i) => format!("{} ({})", breed::POWER_ITEMS[*i], STATS[*i]),
        Held::Other(n) if n.is_empty() => "Something else".into(),
        _ => h.name(),
    }
}

pub(super) fn nature_label(n: &str) -> String {
    match battle::data().nature(n) {
        Some(x) if x.plus != x.minus => format!("{n}  (+{} −{})", STATS[x.plus], STATS[x.minus]),
        Some(_) => format!("{n}  (neutral)"),
        None => n.to_string(),
    }
}

/// The ball by name, for drawing.
pub(super) fn ball_of(name: &str) -> crate::sim::Ball {
    crate::sim::BALLS.iter().copied().find(|b| b.name() == name).unwrap_or(crate::sim::Ball::Poke)
}

/// A parent's stat as a chip: "31", "?" or "–".
pub(super) fn iv_text(v: Option<u8>) -> &'static str {
    match v {
        Some(31) => "31",
        Some(_) => "–",
        None => "?",
    }
}

/// What a tip would change.
#[derive(Clone)]
enum Apply {
    Hold(usize, Held),
    Oval,
    Warmer,
    Flame,
}

struct Tip {
    icon: &'static str,
    col: Color32,
    text: String,
    /// One Egg's chance before → after.
    gain: Option<(f64, f64)>,
    apply: Option<(Apply, &'static str)>,
}

impl App {
    pub(super) fn breed_save_tick(&mut self, force: bool) {
        self.setup.breed.save_tick(force);
    }

    pub(super) fn page_breed(&mut self, ui: &mut egui::Ui) {
        self.setup.breed.file();
        // Keep clear of the scrollbar on the right.
        let w = (ui.available_width() - 14.0).max(300.0);
        ui.set_max_width(w);
        kit::page_header(
            ui,
            "BREEDING",
            "Pair two Pokémon at the Day Care and see what the Egg will be like, how many Eggs it takes to get the one you want, and what would make it quicker.",
        );
        if let Some(e) = self.setup.breed.load_error.clone() {
            let mut close = false;
            note_bar(ui, kit::C_ORANGE, &e, Some(&mut close));
            if close {
                self.setup.breed.load_error = None;
            }
        }
        if let Some(e) = self.setup.breed.save_error.clone() {
            note_bar(ui, kit::C_RED, &format!("Couldn't save the Day Care: {e}. Trying again…"), None);
        }
        let Some(w) = self.wiki.clone() else {
            ui.label(small("Loading the game's data…"));
            return;
        };
        let (nb, nl) = (self.setup.breed.file().breeders.len(), self.setup.breed.file().log.len());
        let tabs = [
            (BreedView::Plan, "Plan".to_string()),
            (BreedView::DayCare, "Day Care".to_string()),
            (BreedView::Moves, "Egg moves".to_string()),
            (BreedView::Breeders, if nb > 0 { format!("Breeders  {nb}") } else { "Breeders".into() }),
            (BreedView::Log, if nl > 0 { format!("Hatch log  {nl}") } else { "Hatch log".into() }),
        ];
        let tabs_ref: Vec<(BreedView, &str)> = tabs.iter().map(|(v, l)| (*v, l.as_str())).collect();
        let mut view = self.setup.breed.view;
        kit::tab_bar(ui, &mut view, &tabs_ref);
        if view != self.setup.breed.view {
            self.setup.breed.view = view;
            self.setup.breed.flash = None;
            self.setup.scroll_top = true;
        }
        ui.add_space(14.0);
        if let Some((msg, at)) = self.setup.breed.flash.clone() {
            if at.elapsed() < Duration::from_secs(4) {
                note_bar(ui, kit::C_GREEN, &msg, None);
                ui.ctx().request_repaint_after(Duration::from_millis(250));
            } else {
                self.setup.breed.flash = None;
            }
        }
        match view {
            BreedView::Plan => return self.page_breed_plan(ui, &w),
            BreedView::Moves => return self.page_egg_moves(ui, &w),
            BreedView::Breeders => return self.page_breeders(ui, &w),
            BreedView::Log => return self.page_hatch_log(ui, &w),
            BreedView::DayCare => {}
        }
        let before = (self.setup.breed.file().setup, self.setup.breed.file().daycare.clone());
        let mut setup = before.0;
        let mut dc = before.1.clone();

        self.breed_settings(ui, &mut setup);
        ui.add_space(14.0);

        let egg = (!dc.a.species.is_empty() && !dc.b.species.is_empty()).then(|| breed::egg(&dc.a, &dc.b, &setup, &w));
        if !dc.goal_set {
            dc.goal = Goal::from_parents(&dc.a, &dc.b);
        }
        self.breed_board(ui, &w, &mut dc, &setup, egg.as_ref());
        ui.add_space(14.0);

        match &egg {
            Some(Ok(e)) => {
                let e = e.clone();
                self.breed_egg_card(ui, &w, &dc, &setup, &e);
                ui.add_space(14.0);
                self.breed_goal(ui, &w, &mut dc, &setup, &e);
                ui.add_space(14.0);
                self.breed_tips(ui, &w, &mut dc, &mut setup, &e);
            }
            Some(Err(_)) | None => {
                // One parent (or a pair that can't breed): who could go with it.
                let have: Vec<usize> = (0..2).filter(|i| ![&dc.a, &dc.b][*i].species.is_empty()).collect();
                match have.as_slice() {
                    [] => self.breed_empty(ui, &mut dc),
                    [i] => self.breed_partners(ui, &w, &mut dc, *i, setup.world),
                    _ => {
                        let keep = if dc.a.is_ditto() { 1 } else { 0 };
                        self.breed_partners(ui, &w, &mut dc, keep, setup.world);
                    }
                }
            }
        }

        if (setup, &dc) != (before.0, &before.1) {
            let f = self.setup.breed.file();
            f.setup = setup;
            f.daycare = dc;
            self.setup.breed.touch();
        }
    }

    // ───────────────────────────── settings ─────────────────────────────

    fn breed_settings(&mut self, ui: &mut egui::Ui, s: &mut Setup) {
        kit::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                ui.label(caps("YOUR WORLD"));
                ui.add_space(2.0);
                for (wv, l) in [(World::Platinum, "Platinum Pokémon"), (World::Modern, "MODERN POKéMON")] {
                    if chip(ui, l, s.world == wv, kit::C_PINK).on_hover_text("MODERN POKéMON worlds have the newer Pokémon, moves and Abilities").clicked() {
                        s.world = wv;
                    }
                }
                ui.add_space(10.0);
                let flag = |ui: &mut egui::Ui, on: &mut bool, l: &str, tip: &str| {
                    if chip(ui, &format!("{}  {l}", if *on { "✔" } else { "+" }), *on, GOLD).on_hover_text(tip).clicked() {
                        *on = !*on;
                    }
                };
                flag(ui, &mut s.hidden_abilities, "Hidden Abilities", "Your world has hidden Abilities: a mother with one can pass it on");
                flag(ui, &mut s.shiny_charm, "Shiny Charm", "Eggs get 3 tries at being shiny, 8 with the Masuda Method");
                flag(ui, &mut s.oval_charm, "Oval Charm", "Eggs turn up more often. You get it after hatching 10 Eggs");
                flag(ui, &mut s.egg_warmer, "Egg Warmer", "In the Bag: Eggs hatch twice as fast (Rift Shop)");
                flag(ui, &mut s.flame_body, "Flame Body in party", "Flame Body or Magma Armor in your party: Eggs hatch twice as fast, 4 times with the Egg Warmer");
            });
        });
    }

    // ───────────────────────────── the two parents ─────────────────────────────

    fn breed_board(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &mut breed::DayCare, s: &Setup, egg: Option<&Result<Egg, Why>>) {
        let avail = ui.available_width();
        let wide = avail >= 980.0;
        let gap = 16.0;
        let mid = if wide { 170.0 } else { avail };
        let side = if wide { ((avail - mid - gap * 2.0) / 2.0).floor() } else { avail };
        // The Egg's species decides which moves and incense matter.
        let egg_species = match egg {
            Some(Ok(e)) => Some(e.species[0].0.clone()),
            _ => None,
        };
        let mut swap = false;
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(side);
                    let other = dc.b.clone();
                    self.parent_card(ui, w, &mut dc.a, &other, 0, egg_species.as_deref(), s, egg);
                });
                ui.allocate_ui_with_layout(egui::vec2(mid, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.set_width(mid);
                    swap = compat_medal(ui, dc, egg, s, false);
                });
                ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(side);
                    let other = dc.a.clone();
                    self.parent_card(ui, w, &mut dc.b, &other, 1, egg_species.as_deref(), s, egg);
                });
            });
        } else {
            let other = dc.b.clone();
            self.parent_card(ui, w, &mut dc.a, &other, 0, egg_species.as_deref(), s, egg);
            ui.add_space(12.0);
            swap = compat_medal(ui, dc, egg, s, true);
            ui.add_space(12.0);
            let other = dc.a.clone();
            self.parent_card(ui, w, &mut dc.b, &other, 1, egg_species.as_deref(), s, egg);
        }
        if swap {
            std::mem::swap(&mut dc.a, &mut dc.b);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn parent_card(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, p: &mut Parent, other: &Parent, i: usize, egg_species: Option<&str>, s: &Setup, egg: Option<&Result<Egg, Why>>) {
        let (title, col) = role(p, i);
        let species = breedable(w, s.world);
        let names: Vec<String> = species.iter().map(|(n, _)| n.clone()).collect();
        let passing: Vec<String> = match egg {
            Some(Ok(e)) => e.moves.clone(),
            _ => vec![],
        };
        let mut new_species: Option<String> = None;
        let mut clear = false;
        let mut save = false;
        let mut load: Option<Parent> = None;
        let saved: Vec<(String, Parent)> = if i < 2 {
            self.setup.breed.file().breeders.iter().map(|b| (super::breed_box::breeder_label(w, b), b.parent.clone())).collect()
        } else {
            vec![]
        };
        let saved_names: Vec<String> = saved.iter().map(|b| b.0.clone()).collect();
        side_card(ui, col, &title, |ui| {
            let full = ui.available_width();
            // Sprite, species and gender.
            ui.horizontal(|ui| {
                let tile = 92.0;
                let (r, _) = ui.allocate_exact_size(egui::vec2(tile, tile), Sense::hover());
                let pnt = ui.painter().clone();
                pnt.rect_filled(r, 14.0, theme::blend(kit::sunken(), col, 0.08));
                pnt.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.35)));
                if p.species.is_empty() || !self.team_sprite(&pnt, &p.species, false, r.center(), egui::vec2(tile - 10.0, tile - 10.0)) {
                    pnt.text(r.center(), Align2::CENTER_CENTER, if p.species.is_empty() { "?" } else { "…" }, crate::fonts::sans_bold_font(26.0), MUTED());
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let cur = if p.species.is_empty() { "Pick a Pokémon".to_string() } else { name_of(w, &p.species) };
                    let pw = (full - tile - 20.0).clamp(150.0, 260.0);
                    ui.horizontal(|ui| {
                        if let Some(n) = search_pick(ui, egui::Id::new(("breed-species", i)), &cur, &names, pw) {
                            new_species = species.iter().find(|(name, _)| *name == n).map(|(_, id)| id.clone());
                        }
                        if !p.species.is_empty() && super::team_home::x_button(ui).on_hover_text("Take it out").clicked() {
                            clear = true;
                        }
                    });
                    if let Some(info) = breed::info(&p.species) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            match info.ratio {
                                -1 => {
                                    ui.label(small("Genderless"));
                                }
                                0 => {
                                    ui.label(small("Always ♂"));
                                }
                                8 => {
                                    ui.label(small("Always ♀"));
                                }
                                _ if p.is_ditto() => {}
                                _ => {
                                    for (g, l) in [(Gender::Female, "♀ Female"), (Gender::Male, "♂ Male")] {
                                        if chip(ui, l, p.gender == g, if g == Gender::Female { MOM } else { DAD }).clicked() {
                                            p.gender = g;
                                        }
                                    }
                                }
                            }
                        });
                        let groups = info.groups.join(" · ");
                        ui.label(small(if p.is_ditto() { "Breeds with almost anything".to_string() } else { format!("Egg groups: {groups}") }));
                    } else {
                        ui.label(small("Type a name to search"));
                    }
                });
            });
            if p.species.is_empty() {
                return;
            }
            ui.add_space(10.0);
            // Stats as tap-to-change chips.
            ui.horizontal(|ui| {
                ui.label(caps("STATS"));
                ui.add_space(4.0);
                ui.label(small("Tap: 31 · not 31 · don't know"));
            });
            ui.add_space(2.0);
            iv_chips(ui, &mut p.ivs, full);
            ui.horizontal(|ui| {
                if ui.link(RichText::new("All 31").size(12.0)).clicked() {
                    p.ivs = [Some(31); 6];
                }
                if ui.link(RichText::new("Don't know").size(12.0)).clicked() {
                    p.ivs = [None; 6];
                }
            });
            ui.add_space(8.0);

            // The rest as label / control rows.
            let lw = 84.0;
            let cw = (full - lw - 8.0).clamp(140.0, 300.0);
            let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(lw, 24.0), Sense::hover());
                    ui.painter().text(egui::pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, label, crate::fonts::sans_font(12.5), MUTED());
                    add(ui);
                });
            };
            // Nature.
            let natures: Vec<String> = std::iter::once("Not set".to_string()).chain(battle::data().natures.iter().map(|n| nature_label(&n.name))).collect();
            let cur_nat = p.nature.as_deref().map(nature_label).unwrap_or_else(|| "Not set".into());
            row(ui, "Nature", &mut |ui| {
                if let Some(n) = search_pick(ui, egui::Id::new(("breed-nature", i)), &cur_nat, &natures, cw) {
                    p.nature = if n == "Not set" { None } else { n.split_whitespace().next().map(str::to_string) };
                }
            });
            // Ability (not Ditto's: it never passes).
            if !p.is_ditto() {
                let normal = breed::normal_abilities(&p.species, s.world);
                let hidden = breed::hidden_ability(&p.species).filter(|_| s.hidden_abilities);
                let mut opts: Vec<(Slot, String)> = vec![];
                if let Some(a) = normal.first() {
                    opts.push((Slot::First, a.clone()));
                }
                if let Some(a) = normal.get(1) {
                    opts.push((Slot::Second, a.clone()));
                }
                if let Some(h) = &hidden {
                    opts.push((Slot::Hidden, format!("{h} (hidden)")));
                }
                if (p.ability == Slot::Hidden && hidden.is_none()) || (p.ability == Slot::Second && normal.len() < 2) {
                    p.ability = Slot::First;
                }
                let cur = opts.iter().find(|o| o.0 == p.ability).map(|o| o.1.clone()).unwrap_or_default();
                row(ui, "Ability", &mut |ui| {
                    egui::ComboBox::from_id_salt(("breed-ability", i)).selected_text(&cur).width(cw).show_ui(ui, |ui| {
                        for (slot, l) in &opts {
                            if ui.selectable_label(p.ability == *slot, l).clicked() {
                                p.ability = *slot;
                            }
                        }
                    });
                });
            }
            // Held item.
            let mut items = vec![Held::Nothing, Held::Everstone, Held::DestinyKnot, Held::RiftKnot];
            items.extend((0..6).map(Held::Power));
            let fam = egg_species.or(Some(p.species.as_str())).and_then(|sp| breed::family_incense(w, sp)).or_else(|| breed::family_incense(w, &other.species));
            if let Some((inc, _)) = &fam {
                items.push(Held::Incense(inc.clone()));
            }
            if !items.contains(&p.held) {
                items.push(p.held.clone());
            }
            let cur = held_label(&p.held);
            row(ui, "Holds", &mut |ui| {
                egui::ComboBox::from_id_salt(("breed-held", i)).selected_text(&cur).width(cw).height(320.0).show_ui(ui, |ui| {
                    for h in &items {
                        if ui.selectable_label(p.held == *h, held_label(h)).clicked() {
                            p.held = h.clone();
                        }
                    }
                });
            });
            // Ball (not Ditto's: it never passes).
            if !p.is_ditto() {
                let balls: Vec<String> = crate::sim::BALLS.iter().map(|b| b.name().to_string()).collect();
                if p.ball.is_empty() {
                    p.ball = "Poké Ball".into();
                }
                let cur = p.ball.clone();
                row(ui, "Ball", &mut |ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                    super::sim_catch::draw_ball(ui.painter(), r.center(), 8.0, 0.0, ball_of(&cur));
                    if let Some(b) = search_pick(ui, egui::Id::new(("breed-ball", i)), &cur, &balls, cw - 24.0) {
                        p.ball = b;
                    }
                });
            }
            // Egg moves it knows.
            if !p.is_ditto() {
                let base = egg_species.map(str::to_string).unwrap_or_else(|| breed::baby_of(w, &p.species));
                let list = breed::egg_moves(w, &base, s.world);
                ui.add_space(8.0);
                ui.label(caps("EGG MOVES IT KNOWS"));
                ui.add_space(2.0);
                if !p.moves.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        let mut remove = None;
                        for (k, m) in p.moves.iter().enumerate() {
                            let passes = passing.contains(m);
                            let resp = chip(ui, &format!("{}  ×", move_name(w, m)), passes, kit::C_TEAL)
                                .on_hover_text(if passes { "Passes to the Egg. Click to remove" } else { "Not an egg move for this Egg. Click to remove" });
                            if resp.clicked() {
                                remove = Some(k);
                            }
                        }
                        if let Some(k) = remove {
                            p.moves.remove(k);
                        }
                    });
                }
                {
                    if p.moves.len() < 4 {
                        let opts: Vec<String> = list.iter().filter(|k| !p.moves.contains(k)).map(|k| move_name(w, k)).collect();
                        if opts.is_empty() {
                            if p.moves.is_empty() {
                                ui.label(small("This Egg has no egg moves"));
                            }
                        } else {
                            if !p.moves.is_empty() {
                                ui.add_space(4.0);
                            }
                            if let Some(n) = search_pick(ui, egui::Id::new(("breed-move", i)), "+ Add an egg move it knows", &opts, 220.0) {
                                if let Some(k) = list.iter().find(|k| move_name(w, k) == n) {
                                    p.moves.push(k.clone());
                                }
                            }
                        }
                    }
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(14.0, 4.0);
                kit::checkbox(ui, &mut p.other_trainer, RichText::new("Other trainer").size(12.5))
                    .on_hover_text("Traded or from the GTS: the Day Care counts different trainers, and Eggs turn up more often");
                if kit::checkbox(ui, &mut p.masuda, RichText::new("Masuda").size(12.5))
                    .on_hover_text("Traded from another player since 0.3.2 (not your own from another world): Eggs are likelier to be shiny")
                    .changed()
                    && p.masuda
                {
                    p.other_trainer = true;
                }
                kit::checkbox(ui, &mut p.partner, RichText::new("Partner").size(12.5)).on_hover_text("A Partner starter: they can never be bred");
            });
            if i < 2 {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.link(RichText::new("Save to your Breeders").size(12.0)).on_hover_text("Keep this Pokémon to use again").clicked() {
                        save = true;
                    }
                    if !saved_names.is_empty() {
                        ui.label(small("·"));
                        if let Some(n) = search_pick(ui, egui::Id::new(("breed-load", i)), "Use one of your Breeders", &saved_names, 210.0) {
                            load = saved.iter().find(|b| b.0 == n).map(|b| b.1.clone());
                        }
                    }
                });
            }
        });
        if save {
            let f = self.setup.breed.file();
            let id = f.new_id();
            f.breeders.push(breed::Breeder { id, parent: p.clone(), added: id / 1000, ..Default::default() });
            self.setup.breed.touch();
            self.setup.breed.flash = Some((format!("Saved {} to your Breeders.", name_of(w, &p.species)), Instant::now()));
        }
        if let Some(b) = load {
            *p = b;
        }
        if clear {
            *p = Parent::default();
        }
        if let Some(id) = new_species {
            let keep = p.clone();
            let mut np = Parent::new(&id, keep.gender);
            np.ivs = keep.ivs;
            np.nature = keep.nature;
            np.held = keep.held;
            np.ball = if keep.ball.is_empty() { "Poké Ball".into() } else { keep.ball };
            np.other_trainer = keep.other_trainer;
            np.masuda = keep.masuda;
            // Opposite the other parent, when the species allows.
            if !other.species.is_empty() && !other.is_ditto() && keep.species.is_empty() {
                np.gender = match other.gender() {
                    Gender::Female => Gender::Male,
                    _ => Gender::Female,
                };
            }
            np.gender = np.gender();
            *p = np;
        }
    }

    // ───────────────────────────── the Egg ─────────────────────────────

    fn breed_egg_card(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &breed::DayCare, s: &Setup, e: &Egg) {
        let parents = [&dc.a, &dc.b];
        let base = parents[e.base];
        side_card(ui, EGG, "THE EGG", |ui| {
            if let Some((inc, baby)) = &e.incense_missing {
                note_bar(ui, kit::C_ORANGE, &format!("A parent holding a {inc} makes it a {} Egg instead.", name_of(w, baby)), None);
            }
            let full = ui.available_width();
            let three = full >= 900.0;
            let left_w = if three { 250.0 } else { full };
            let right_w = if three { 330.0 } else { full };
            let mid_w = if three { full - left_w - right_w - 40.0 } else { full };
            let mut body = |ui: &mut egui::Ui, part: usize| match part {
                0 => self.egg_who(ui, w, s, e, left_w),
                1 => egg_stats(ui, e, dc, mid_w),
                _ => egg_details(ui, w, e, base, right_w),
            };
            if three {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    for (part, wd) in [(0, left_w), (1, mid_w), (2, right_w)] {
                        ui.allocate_ui_with_layout(egui::vec2(wd, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.set_width(wd);
                            body(ui, part);
                        });
                    }
                });
            } else {
                for part in 0..3 {
                    body(ui, part);
                    ui.add_space(14.0);
                }
            }
            // What isn't confirmed.
            ui.add_space(12.0);
            let n = e.notes.len();
            let open = self.setup.breed.notes_open;
            let label = format!("{n} {} here {} confirmed for this mod  {}", if n == 1 { "rule" } else { "rules" }, if n == 1 { "isn't" } else { "aren't" }, if open { "▲" } else { "▼" });
            if ui.add(egui::Label::new(RichText::new(label).size(12.0).color(MUTED())).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.breed.notes_open = !open;
            }
            if open {
                ui.add_space(4.0);
                for note in &e.notes {
                    ui.add(egui::Label::new(RichText::new(format!("•  {note}")).size(12.0).color(MUTED())).wrap());
                }
            }
        });
    }

    /// The Egg itself: who hatches, gender, shiny odds, hatching.
    fn egg_who(&mut self, ui: &mut egui::Ui, w: &Wiki, s: &Setup, e: &Egg, width: f32) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(width, 150.0), Sense::hover());
        let p = ui.painter().clone();
        let c = egui::pos2(r.center().x, r.center().y + 2.0);
        draw_egg(&p, c, 54.0, 70.0);
        let main = &e.species[0].0;
        if !self.team_sprite(&p, main, false, c + egui::vec2(0.0, 6.0), egui::vec2(96.0, 96.0)) {
            p.text(c, Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(30.0), super::charts::ink(EGG));
        }
        let title = if e.species.len() > 1 {
            e.species.iter().map(|(sp, _)| name_of(w, sp)).collect::<Vec<_>>().join(" or ")
        } else {
            name_of(w, main)
        };
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(18.0)).color(TEXT()));
            if e.species.len() > 1 {
                ui.label(small("Either one, 50/50"));
            }
        });
        ui.add_space(8.0);
        // Gender split.
        match e.female {
            Some(f) => {
                let (br, _) = ui.allocate_exact_size(egui::vec2(width, 8.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(br, 4.0, theme::blend(CARD(), DAD, 0.8));
                if f > 0.0 {
                    let fr = Rect::from_min_max(egui::pos2(br.max.x - br.width() * f as f32, br.min.y), br.max);
                    p.rect_filled(fr, 4.0, theme::blend(CARD(), MOM, 0.85));
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("♂ {}", pct(1.0 - f))).size(12.0).color(super::charts::ink(DAD)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("♀ {}", pct(f))).size(12.0).color(super::charts::ink(MOM)));
                    });
                });
            }
            None => {
                ui.vertical_centered(|ui| ui.label(small("Genderless")));
            }
        }
        ui.add_space(6.0);
        let masuda = e.shiny_one_in < breed::shiny_one_in(breed::shiny_rolls(s.shiny_charm, false)) - 0.5;
        let shiny_tag = match (masuda, s.shiny_charm) {
            (true, true) => "  Masuda + Charm",
            (true, false) => "  Masuda",
            (false, true) => "  Shiny Charm",
            _ => "",
        };
        fact(ui, "✨", "Shiny", &format!("1 in {}", thousands(e.shiny_one_in.round() as u32)));
        tag_line(ui, shiny_tag);
        let steps = e.hatch_steps(s);
        let speed = match (s.egg_warmer, s.flame_body) {
            (true, true) => "  Egg Warmer + Flame Body",
            (true, false) => "  Egg Warmer",
            (false, true) => "  Flame Body",
            _ => "",
        };
        fact(ui, "🐾", "Hatches in", &format!("about {} steps", thousands(steps)));
        tag_line(ui, speed);
        if let Some(c) = e.compat {
            fact(ui, "🐣", "An Egg every", &format!("about {} steps", thousands(c.steps_per_egg(s.oval_charm))));
        }
    }

    // ───────────────────────────── the goal ─────────────────────────────

    fn breed_goal(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &mut breed::DayCare, s: &Setup, e: &Egg) {
        let mut g = dc.goal.clone();
        let mut reset = false;
        let p_one = dc.goal.chance(&dc.a, &dc.b, e);
        side_card(ui, kit::C_TEAL, "THE ONE YOU WANT", |ui| {
            let full = ui.available_width();
            let two = full >= 820.0;
            let left_w = if two { (full * 0.55).floor() } else { full };
            let pick = |ui: &mut egui::Ui, g: &mut Goal| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.label(caps("PERFECT"));
                    for st in 0..6 {
                        if chip(ui, STATS[st], g.ivs[st], GOLD).on_hover_text("Want this stat at 31").clicked() {
                            g.ivs[st] = !g.ivs[st];
                        }
                    }
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                    ui.label(caps("NATURE"));
                    let natures: Vec<String> = std::iter::once("Any".to_string()).chain(battle::data().natures.iter().map(|n| nature_label(&n.name))).collect();
                    let cur = g.nature.as_deref().map(nature_label).unwrap_or_else(|| "Any".into());
                    if let Some(n) = search_pick(ui, egui::Id::new("breed-goal-nature"), &cur, &natures, 170.0) {
                        g.nature = if n == "Any" { None } else { n.split_whitespace().next().map(str::to_string) };
                    }
                    ui.add_space(6.0);
                    ui.label(caps("ABILITY"));
                    let mut opts = vec!["Any".to_string()];
                    opts.extend(e.abilities.iter().map(|(n, _, _)| n.clone()));
                    let cur = g.ability.clone().unwrap_or_else(|| "Any".into());
                    egui::ComboBox::from_id_salt("breed-goal-ability").selected_text(&cur).width(140.0).show_ui(ui, |ui| {
                        for o in &opts {
                            if ui.selectable_label(*o == cur, o).clicked() {
                                g.ability = if o == "Any" { None } else { Some(o.clone()) };
                            }
                        }
                    });
                });
                if e.female.is_some_and(|f| f > 0.0 && f < 1.0) {
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        ui.label(caps("GENDER"));
                        for (v, l, c) in [(None, "Either", kit::C_SLATE), (Some(Gender::Female), "♀ Female", MOM), (Some(Gender::Male), "♂ Male", DAD)] {
                            if chip(ui, l, g.gender == v, c).clicked() {
                                g.gender = v;
                            }
                        }
                    });
                }
            };
            let result = |ui: &mut egui::Ui, reset: &mut bool| {
                if p_one <= 0.0 {
                    ui.label(RichText::new("Can't happen with these two").font(crate::fonts::sans_bold_font(20.0)).color(super::charts::ink(kit::C_RED)));
                    ui.label(small(why_never(&dc.goal, dc, e, w)));
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(one_in(p_one)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                        if p_one < 0.9995 {
                            ui.label(small(format!("Eggs  ·  {} each", pct(p_one))));
                        }
                    });
                    if p_one < 0.9995 {
                        let e50 = breed::eggs_for(p_one, 0.5).unwrap_or(0);
                        let e90 = breed::eggs_for(p_one, 0.9).unwrap_or(0);
                        ui.label(RichText::new(format!("Half the time within {} Eggs, 9 times in 10 within {}", thousands(e50), thousands(e90))).size(13.0).color(TEXT()));
                        if let Some(c) = e.compat {
                            let walk = e50 as u64 * c.steps_per_egg(s.oval_charm) as u64 + e.hatch_steps(s) as u64;
                            ui.label(small(format!("Roughly {} steps for the half-way mark, hatching as you go", thousands(walk.min(u32::MAX as u64) as u32))));
                        }
                        ui.add_space(6.0);
                        // How the odds build up.
                        let marks = [10u32, 25, 50, 100];
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for n in marks {
                                let have = 1.0 - (1.0 - p_one).powi(n as i32);
                                milestone(ui, n, have);
                            }
                        });
                    }
                }
                if dc.goal_set {
                    ui.add_space(4.0);
                    if ui.link(RichText::new("Go back to what the parents have").size(12.0)).clicked() {
                        *reset = true;
                    }
                }
            };
            if two {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(left_w);
                        pick(ui, &mut g);
                    });
                    ui.vertical(|ui| result(ui, &mut reset));
                });
            } else {
                pick(ui, &mut g);
                ui.add_space(10.0);
                result(ui, &mut reset);
            }
        });
        if reset {
            dc.goal_set = false;
        } else if g != dc.goal {
            dc.goal = g;
            dc.goal_set = true;
        }
    }

    // ───────────────────────────── tips ─────────────────────────────

    fn breed_tips(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &mut breed::DayCare, s: &mut Setup, e: &Egg) {
        let tips = tips(w, dc, s, e);
        if tips.is_empty() {
            return;
        }
        let mut apply: Option<Apply> = None;
        side_card(ui, kit::C_VIOLET, "MAKE IT QUICKER", |ui| {
            for t in &tips {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, t.icon, t.col, false);
                    ui.add_space(4.0);
                    let btn_w = if t.apply.is_some() { 110.0 } else { 0.0 };
                    let gain_w = if t.gain.is_some() { 190.0 } else { 0.0 };
                    let text_w = (ui.available_width() - btn_w - gain_w - 16.0).max(160.0);
                    ui.allocate_ui_with_layout(egui::vec2(text_w, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_width(text_w);
                        ui.add(egui::Label::new(RichText::new(&t.text).size(13.0).color(TEXT())).wrap());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some((a, l)) = &t.apply {
                            if super::team_home::soft(ui, l).clicked() {
                                apply = Some(a.clone());
                            }
                        }
                        if let Some((b, a)) = t.gain {
                            ui.label(RichText::new(format!("{}  →  {}", one_in(b), one_in(a))).size(12.5).color(super::charts::ink(kit::C_GREEN)));
                        }
                    });
                });
                ui.add_space(6.0);
            }
        });
        match apply {
            Some(Apply::Hold(i, h)) => {
                let p = if i == 0 { &mut dc.a } else { &mut dc.b };
                p.held = h;
            }
            Some(Apply::Oval) => s.oval_charm = true,
            Some(Apply::Warmer) => s.egg_warmer = true,
            Some(Apply::Flame) => s.flame_body = true,
            None => {}
        }
    }

    // ───────────────────────────── empty and partner finder ─────────────────────────────

    fn breed_empty(&mut self, ui: &mut egui::Ui, dc: &mut breed::DayCare) {
        let mut example = false;
        side_card(ui, EGG, "HOW IT WORKS", |ui| {
            ui.label(RichText::new("Put two Pokémon in the Day Care").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Pick a mother and a father (or Ditto) above, tap the stats they have at 31, and set what they hold. You'll see what the Egg will be like, how many Eggs it takes to get the one you want, and what would make it quicker. Pick just one and you'll see everything it can breed with.",
                    )
                    .size(13.0)
                    .color(MUTED()),
                )
                .wrap(),
            );
            ui.add_space(10.0);
            example = kit::accent_button_w(ui, egui::Button::new(RichText::new("Try an example").size(13.0)), 8.0, 140.0)
                .on_hover_text("A Jolly Garchomp with an Everstone and a perfect Ditto with a Destiny Knot")
                .clicked();
        });
        if example {
            let mut a = Parent::new("GARCHOMP", Gender::Female);
            a.nature = Some("Jolly".into());
            a.held = Held::Everstone;
            a.ivs = [Some(31), Some(31), None, None, None, Some(31)];
            let mut b = Parent::new(breed::DITTO, Gender::Male);
            b.held = Held::DestinyKnot;
            b.ivs = [Some(31), Some(31), Some(31), Some(0), Some(31), Some(31)];
            b.other_trainer = true;
            dc.a = a;
            dc.b = b;
            dc.goal_set = false;
        }
    }

    fn breed_partners(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &mut breed::DayCare, have: usize, world: World) {
        let parent = if have == 0 { dc.a.clone() } else { dc.b.clone() };
        let other = if have == 0 { dc.b.clone() } else { dc.a.clone() };
        let why = (!other.species.is_empty()).then(|| breed::compat(&dc.a, &dc.b, Some(w)).err()).flatten();
        // Only ones that can be the other gender.
        let list: Vec<String> = breed::partners(&parent.species, w, world)
            .into_iter()
            .filter(|id| match (parent.gender(), breed::info(id).map(|i| i.ratio)) {
                _ if parent.is_ditto() || id == breed::DITTO => true,
                (Gender::Female, Some(8)) | (Gender::Male, Some(0)) => false,
                _ => true,
            })
            .collect();
        let mut pick: Option<String> = None;
        let name = name_of(w, &parent.species);
        side_card(ui, kit::C_TEAL, &format!("WHO {} CAN BREED WITH", name.to_uppercase()), |ui| {
            if let Some(why) = &why {
                note_bar(ui, kit::C_RED, &format!("No Egg: {}", why.text()), None);
            }
            if list.is_empty() {
                ui.label(small(format!("{name} can't make Eggs.")));
                return;
            }
            ui.horizontal(|ui| {
                ui.label(small(format!("{} Pokémon, Ditto included. Click one to put it in the Day Care.", list.len())));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    kit::search_box(ui, &mut self.setup.breed.partner_q, "Search…", 180.0);
                });
            });
            ui.add_space(8.0);
            let q = self.setup.breed.partner_q.trim().to_lowercase();
            let shown: Vec<&String> = list.iter().filter(|id| q.is_empty() || name_of(w, id).to_lowercase().contains(&q)).collect();
            let tile = 74.0;
            let cap = 60;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for id in shown.iter().take(cap) {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(tile, tile + 16.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter().clone();
                    let box_r = Rect::from_min_size(r.min, egui::vec2(tile, tile));
                    p.rect_filled(box_r, 12.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                    p.rect_stroke(box_r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_TEAL, 0.5 * ht)));
                    if !self.team_sprite(&p, id, false, box_r.center(), egui::vec2(tile - 10.0, tile - 10.0)) {
                        p.text(box_r.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(14.0), MUTED());
                    }
                    let f = crate::fonts::sans_font(11.0);
                    let n = name_of(w, id);
                    p.text(egui::pos2(box_r.center().x, r.max.y - 6.0), Align2::CENTER_CENTER, super::charts::clip(ui, &n, &f, tile), f, TEXT());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Put {n} in the Day Care")).clicked() {
                        pick = Some((*id).clone());
                    }
                }
            });
            if shown.len() > cap {
                ui.add_space(4.0);
                ui.label(small(format!("and {} more: search to find one", shown.len() - cap)));
            } else if shown.is_empty() {
                ui.label(small("Nothing matches"));
            }
        });
        if let Some(id) = pick {
            let mut np = Parent::new(&id, match parent.gender() {
                Gender::Female => Gender::Male,
                _ => Gender::Female,
            });
            np.gender = np.gender();
            if have == 0 {
                dc.b = np;
            } else {
                dc.a = np;
            }
            self.setup.breed.partner_q.clear();
        }
    }
}

// ───────────────────────────── drawing ─────────────────────────────

/// Six tap-to-change stat chips: 31 → not 31 → don't know.
pub(super) fn iv_chips(ui: &mut egui::Ui, ivs: &mut [Option<u8>; 6], full: f32) {
    let gap = 6.0;
    let cw = ((full - gap * 5.0) / 6.0).clamp(40.0, 70.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for st in 0..6 {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 46.0), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let v = ivs[st];
            let on = v == Some(31);
            let pnt = ui.painter();
            let fill = if on { theme::blend(CARD(), GOLD, 0.22) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
            pnt.rect_filled(r, 10.0, fill);
            pnt.rect_stroke(r, 10.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), GOLD, 0.8) } else { LINE() }));
            pnt.text(egui::pos2(r.center().x, r.min.y + 12.0), Align2::CENTER_CENTER, STATS[st].to_uppercase(), crate::fonts::sans_bold_font(9.5), MUTED());
            let ink = if on { super::charts::ink(GOLD) } else if v.is_some() { MUTED() } else { theme::blend(MUTED(), CARD(), 0.3) };
            pnt.text(egui::pos2(r.center().x, r.min.y + 31.0), Align2::CENTER_CENTER, iv_text(v), crate::fonts::sans_bold_font(16.0), ink);
            let tip = match v {
                Some(31) => "31 (perfect). Tap: not 31",
                Some(_) => "Not 31. Tap: don't know",
                None => "Don't know. Tap: 31",
            };
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
                ivs[st] = match v {
                    None => Some(31),
                    Some(31) => Some(0),
                    Some(_) => None,
                };
            }
        }
    });
}

/// The middle of the board: how well they get along, and Swap.
fn compat_medal(ui: &mut egui::Ui, dc: &breed::DayCare, egg: Option<&Result<Egg, Why>>, s: &Setup, compact: bool) -> bool {
    let state: Result<Option<Compat>, String> = match egg {
        Some(Ok(e)) => Ok(e.compat),
        Some(Err(why)) => Err(why.text()),
        None => Ok(None),
    };
    let hearts = match &state {
        Ok(Some(Compat::High)) => 3,
        Ok(Some(Compat::Medium)) => 2,
        Ok(Some(Compat::Low)) => 1,
        _ => 0,
    };
    let medal = |ui: &mut egui::Ui, size: f32| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
        let p = ui.painter();
        let c = r.center();
        let rad = size / 2.0 - 2.0;
        let col = if state.is_err() { kit::C_RED } else { MOM };
        p.circle_filled(c, rad, theme::blend(CARD(), col, 0.10));
        p.circle_stroke(c, rad, Stroke::new(2.0f32, theme::blend(LINE(), col, 0.6)));
        if state.is_err() {
            let k = rad * 0.32;
            let st = Stroke::new(3.0f32, super::charts::ink(kit::C_RED));
            p.line_segment([c + egui::vec2(-k, -k), c + egui::vec2(k, k)], st);
            p.line_segment([c + egui::vec2(-k, k), c + egui::vec2(k, -k)], st);
        } else {
            let hs = rad * 0.34;
            let xs = [-hs * 1.15, 0.0, hs * 1.15];
            for (k, x) in xs.iter().enumerate() {
                let on = k < hearts;
                draw_heart(p, c + egui::vec2(*x, if k == 1 { -hs * 0.35 } else { hs * 0.2 }), hs * 0.55, if on { MOM } else { theme::blend(CARD(), MUTED(), 0.35) });
            }
        }
    };
    let words = match &state {
        Ok(Some(c)) => format!("“{}”", c.line()),
        Ok(None) if dc.a.species.is_empty() && dc.b.species.is_empty() => "Pick two Pokémon".into(),
        Ok(None) => "Pick the other parent".into(),
        Err(why) => why.clone(),
    };
    let rate = match &state {
        Ok(Some(c)) => Some(format!("An Egg about every {} steps", thousands(c.steps_per_egg(s.oval_charm)))),
        _ => None,
    };
    let mut swap = false;
    let can_swap = !dc.a.species.is_empty() || !dc.b.species.is_empty();
    if compact {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            medal(ui, 60.0);
            ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(&words).size(13.0).color(TEXT())).wrap());
                if let Some(r) = &rate {
                    ui.label(small(r));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if can_swap {
                    swap = super::team_home::soft(ui, "Swap parents").on_hover_text("Swap the two parents").clicked();
                }
            });
        });
    } else {
        ui.add_space(40.0);
        medal(ui, 104.0);
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Label::new(RichText::new(&words).size(13.0).color(TEXT())).wrap());
            if let Some(r) = &rate {
                ui.add_space(4.0);
                ui.add(egui::Label::new(small(r)).wrap());
            }
            ui.add_space(12.0);
            if can_swap {
                swap = super::team_home::soft(ui, "Swap parents").on_hover_text("Swap the two parents").clicked();
            }
        });
    }
    swap
}

/// Each stat's chance of coming out at 31.
fn egg_stats(ui: &mut egui::Ui, e: &Egg, dc: &breed::DayCare, width: f32) {
    let knot = [&dc.a, &dc.b].iter().find(|p| p.held.knot()).map(|p| p.held.name());
    ui.horizontal(|ui| {
        ui.label(caps("STATS AT 31"));
        ui.add_space(4.0);
        ui.label(small(match &knot {
            Some(k) => format!("5 of 6 come from the parents ({k})"),
            None => "3 of 6 come from the parents".to_string(),
        }));
    });
    ui.add_space(6.0);
    for st in 0..6 {
        let (r, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::hover());
        let p = ui.painter();
        let v = e.perfect[st];
        p.text(egui::pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, STATS[st], crate::fonts::sans_bold_font(12.0), MUTED());
        let bar = Rect::from_min_max(egui::pos2(r.min.x + 44.0, r.center().y - 5.0), egui::pos2(r.max.x - 56.0, r.center().y + 5.0));
        p.rect_filled(bar, 5.0, kit::sunken());
        let full = v >= 0.9995;
        let col = if full { GOLD } else { kit::C_TEAL };
        if v > 0.0 {
            let fr = Rect::from_min_size(bar.min, egui::vec2((bar.width() * v as f32).max(6.0), bar.height()));
            p.rect_filled(fr, 5.0, theme::blend(CARD(), col, 0.9));
        }
        p.text(egui::pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, pct(v), crate::fonts::sans_bold_font(12.5), if full { super::charts::ink(GOLD) } else { TEXT() });
    }
    // Where each stat can come from: a tiny legend of the parents' 31s.
    ui.add_space(4.0);
    let have = |p: &Parent| (0..6).filter(|s| p.ivs[*s] == Some(31)).map(|s| STATS[s]).collect::<Vec<_>>().join(" ");
    for (k, p) in [&dc.a, &dc.b].into_iter().enumerate() {
        let (r, _) = role(p, k);
        let r = r.trim_start_matches(['♀', '♂', ' ']).to_string();
        let h = have(p);
        ui.label(small(format!("{} at 31: {}", cap_first(&r.to_lowercase()), if h.is_empty() { "none entered".to_string() } else { h })));
    }
}

/// Nature, Ability, ball and moves.
fn egg_details(ui: &mut egui::Ui, w: &Wiki, e: &Egg, base: &Parent, _width: f32) {
    let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
        ui.label(caps(label));
        ui.add_space(2.0);
        add(ui);
        ui.add_space(10.0);
    };
    row(ui, "NATURE", &mut |ui| {
        if e.nature.is_empty() {
            ui.label(RichText::new("Random").size(14.0).color(TEXT()));
            ui.label(small("1 in 25 for each. An Everstone or Rift Knot passes one on."));
        } else {
            for (n, p) in &e.nature {
                let name = n.clone().unwrap_or_else(|| "The holder's nature".into());
                ui.label(RichText::new(format!("{name}  {}", pct(*p))).size(14.0).color(TEXT()));
            }
        }
    });
    row(ui, "ABILITY", &mut |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 4.0);
            for (n, slot, p) in &e.abilities {
                let hidden = *slot == Slot::Hidden;
                ui.label(RichText::new(format!("{n}{}  {}", if hidden { " (hidden)" } else { "" }, pct(*p))).size(14.0).color(TEXT()));
            }
        });
        ui.label(small(format!("From the {}", if base.is_ditto() { "Ditto's partner" } else if base.gender() == Gender::Female { "mother" } else { "parent that isn't Ditto" })));
    });
    row(ui, "BALL", &mut |ui| {
        for (b, p) in &e.ball {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                super::sim_catch::draw_ball(ui.painter(), r.center(), 8.0, 0.0, ball_of(b));
                ui.label(RichText::new(if *p < 0.999 { format!("{b}  {}", pct(*p)) } else { b.clone() }).size(14.0).color(TEXT()));
            });
        }
    });
    row(ui, "EGG MOVES", &mut |ui| {
        if e.moves.is_empty() && e.maybe_moves.is_empty() {
            ui.label(small("None. Add the egg moves a parent knows to see them here."));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for m in &e.moves {
                chip(ui, &move_name(w, m), true, kit::C_TEAL);
            }
            for m in &e.maybe_moves {
                chip(ui, &format!("{}?", move_name(w, m)), false, kit::C_TEAL).on_hover_text("Both parents know it and the baby learns it by level: may pass too");
            }
        });
    });
}

fn fact(ui: &mut egui::Ui, icon: &str, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
        ui.label(RichText::new(icon).size(13.0));
        ui.label(RichText::new(label).size(12.5).color(MUTED()));
        ui.label(RichText::new(value).size(13.0).color(TEXT()));
    });
}

/// "Masuda + Charm" under a fact, when there is one.
fn tag_line(ui: &mut egui::Ui, tag: &str) {
    let tag = tag.trim();
    if !tag.is_empty() {
        ui.add_space(-6.0);
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.label(RichText::new(tag).size(11.5).color(super::charts::ink(GOLD)));
        });
    }
}

fn milestone(ui: &mut egui::Ui, n: u32, have: f64) {
    let text = format!("{n} Eggs  {}", pct(have));
    let g = ui.painter().layout_no_wrap(text, crate::fonts::sans_bold_font(11.5), TEXT());
    let (r, _) = ui.allocate_exact_size(g.size() + egui::vec2(18.0, 12.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, r.height() / 2.0, kit::sunken());
    let fw = r.width() * have.clamp(0.0, 1.0) as f32;
    if fw >= r.height() * 0.6 {
        let fr = Rect::from_min_size(r.min, egui::vec2(fw, r.height()));
        p.rect_filled(fr, r.height() / 2.0, theme::blend(CARD(), kit::C_TEAL, 0.35));
    }
    p.rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, LINE()));
    p.galley(r.min + egui::vec2(9.0, 6.0), g, TEXT());
}

/// An Egg shape: a taller top, a rounder bottom, a few spots.
fn draw_egg(p: &egui::Painter, c: egui::Pos2, rx: f32, ry: f32) {
    let n = 48;
    let pts: Vec<egui::Pos2> = (0..n)
        .map(|k| {
            let a = std::f32::consts::TAU * k as f32 / n as f32;
            let (sx, sy) = (a.cos(), a.sin());
            // narrower above the middle
            let squeeze = if sy < 0.0 { 1.0 - 0.18 * (-sy) } else { 1.0 };
            egui::pos2(c.x + rx * sx * squeeze, c.y + ry * sy * if sy < 0.0 { 1.0 } else { 0.86 })
        })
        .collect();
    p.add(egui::Shape::convex_polygon(pts.clone(), theme::blend(CARD(), EGG, 0.22), Stroke::new(1.5f32, theme::blend(LINE(), EGG, 0.7))));
    for (dx, dy, r) in [(-0.45, -0.35, 0.16), (0.42, -0.1, 0.12), (-0.2, 0.45, 0.1), (0.35, 0.5, 0.14)] {
        p.circle_filled(c + egui::vec2(rx * dx, ry * dy), rx * r, theme::blend(CARD(), EGG, 0.32));
    }
}

fn draw_heart(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32) {
    // Two circles and a triangle.
    let r = s * 0.55;
    p.circle_filled(c + egui::vec2(-r * 0.92, -r * 0.3), r, col);
    p.circle_filled(c + egui::vec2(r * 0.92, -r * 0.3), r, col);
    p.add(egui::Shape::convex_polygon(
        vec![c + egui::vec2(-r * 1.86, -r * 0.05), c + egui::vec2(r * 1.86, -r * 0.05), c + egui::vec2(0.0, s * 1.35)],
        col,
        Stroke::NONE,
    ));
}

pub(super) fn note_bar(ui: &mut egui::Ui, col: Color32, text: &str, close: Option<&mut bool>) {
    egui::Frame::none()
        .fill(theme::blend(CARD(), col, 0.12))
        .stroke(Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5)))
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(text).size(13.0).color(TEXT())).wrap());
                if let Some(c) = close {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        *c = super::team_home::x_button(ui).clicked();
                    });
                }
            });
        });
    ui.add_space(10.0);
}

/// Why the goal can never come out.
fn why_never(g: &Goal, dc: &breed::DayCare, e: &Egg, w: &Wiki) -> String {
    let _ = w;
    if let Some(n) = &g.nature {
        if e.nature_chance(n) <= 0.0 {
            return format!("An Everstone passes the holder's nature, so the Egg can't be {n}.");
        }
    }
    if let Some(a) = &g.ability {
        if e.ability_chance(a) <= 0.0 {
            return format!("{a} can't come from these parents.");
        }
    }
    let want = g.want();
    if breed::iv_chance(&dc.a, &dc.b, &want) <= 0.0 {
        return "A Power item always passes its stat from the holder, so a stat it holds below 31 can't be 31.".into();
    }
    "One of the things you picked can't come from these parents.".into()
}

// ───────────────────────────── tips ─────────────────────────────

fn tips(w: &Wiki, dc: &breed::DayCare, s: &Setup, e: &Egg) -> Vec<Tip> {
    let g = &dc.goal;
    let now = g.chance(&dc.a, &dc.b, e);
    let mut out: Vec<Tip> = vec![];
    // Try an item on a parent: the goal's chance with it.
    let with_item = |i: usize, h: Held| -> Option<f64> {
        let mut a = dc.a.clone();
        let mut b = dc.b.clone();
        if i == 0 {
            a.held = h;
        } else {
            b.held = h;
        }
        let e2 = breed::egg(&a, &b, s, w).ok()?;
        Some(g.chance(&a, &b, &e2))
    };
    let parents = [&dc.a, &dc.b];
    let who = |i: usize| -> String {
        let p = parents[i];
        if p.is_ditto() {
            "the Ditto".into()
        } else {
            format!("the {}", name_of(w, &p.species))
        }
    };
    let mut item_tips: Vec<(f64, Tip)> = vec![];
    let mut consider = |i: usize, h: Held, text: String| {
        if parents[i].held == h {
            return;
        }
        if let Some(after) = with_item(i, h.clone()) {
            if after > now * 1.15 && after > 0.0 {
                let label = if matches!(h, Held::Power(_)) { "Hold it" } else { "Give it" };
                item_tips.push((after / now.max(1e-12), Tip { icon: "🎒", col: kit::C_GOLD, text, gain: Some((now, after)), apply: Some((Apply::Hold(i, h), label)) }));
            }
        }
    };
    let knot = parents.iter().any(|p| p.held.knot());
    for i in 0..2 {
        let p = parents[i];
        if !knot {
            match p.held {
                Held::Everstone => consider(i, Held::RiftKnot, format!("Swap {}'s Everstone for a Rift Knot: it keeps the nature and passes 5 stats instead of 3.", who(i))),
                Held::Nothing | Held::Other(_) => consider(i, Held::DestinyKnot, format!("Give {} a Destiny Knot: 5 stats come from the parents instead of 3.", who(i))),
                _ => {}
            }
        }
        if let Some(n) = &g.nature {
            if p.nature.as_deref() == Some(n.as_str()) && !p.held.keeps_nature() {
                let h = if p.held == Held::DestinyKnot { Held::RiftKnot } else { Held::Everstone };
                let what = if h == Held::RiftKnot { "a Rift Knot (Everstone and Destiny Knot in one)" } else { "an Everstone" };
                consider(i, h, format!("{} is {n}: give it {what} and every Egg is {n}.", cap_first(&who(i))));
            }
        }
        // A Power item for a wanted stat only this parent has at 31.
        if matches!(p.held, Held::Nothing | Held::Other(_)) {
            for st in 0..6 {
                if g.ivs[st] && p.ivs[st] == Some(31) && e.perfect[st] < 0.999 {
                    consider(i, Held::Power(st), format!("A {} on {}: its {} always comes from it.", breed::POWER_ITEMS[st], who(i), STATS[st]));
                }
            }
        }
    }
    item_tips.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // The best tip per parent, and one per kind of item.
    let mut used = [false; 2];
    let mut kinds: Vec<std::mem::Discriminant<Held>> = vec![];
    for (_, t) in item_tips {
        let Some((Apply::Hold(i, h), _)) = &t.apply else { continue };
        let kind = std::mem::discriminant(h);
        if used[*i] || kinds.contains(&kind) {
            continue;
        }
        used[*i] = true;
        kinds.push(kind);
        out.push(t);
    }
    // Faster Eggs.
    if let Some(c) = e.compat {
        if !s.oval_charm {
            out.push(Tip {
                icon: "🐣",
                col: kit::C_GOLD,
                text: format!(
                    "Got the Oval Charm (10 hatched Eggs)? Eggs turn up about every {} steps instead of {}.",
                    thousands(c.steps_per_egg(true)),
                    thousands(c.steps_per_egg(false))
                ),
                gain: None,
                apply: Some((Apply::Oval, "I have it")),
            });
        }
        if c != Compat::High && !dc.a.other_trainer && !dc.b.other_trainer {
            let better = if dc.a.species == dc.b.species { Compat::High } else { Compat::Medium };
            out.push(Tip {
                icon: "👥",
                col: MOM,
                text: format!(
                    "A parent from another trainer gets along better: an Egg about every {} steps instead of {}.",
                    thousands(better.steps_per_egg(s.oval_charm)),
                    thousands(c.steps_per_egg(s.oval_charm))
                ),
                gain: None,
                apply: None,
            });
        }
    }
    if !s.flame_body {
        out.push(Tip {
            icon: "🔥",
            col: kit::C_ORANGE,
            text: format!(
                "Flame Body in your party (Ponyta, Slugma, Magby…) halves hatching: about {} steps instead of {}.",
                thousands(breed::hatch_steps(e.cycles, s.egg_warmer, true)),
                thousands(e.hatch_steps(s))
            ),
            gain: None,
            apply: Some((Apply::Flame, "I have one")),
        });
    } else if !s.egg_warmer {
        out.push(Tip {
            icon: "🔥",
            col: kit::C_ORANGE,
            text: format!(
                "The Egg Warmer (Rift Shop) in your Bag doubles it again: about {} steps instead of {}.",
                thousands(breed::hatch_steps(e.cycles, true, s.flame_body)),
                thousands(e.hatch_steps(s))
            ),
            gain: None,
            apply: Some((Apply::Warmer, "I have it")),
        });
    }
    if !dc.a.masuda && !dc.b.masuda {
        out.push(Tip {
            icon: "✨",
            col: kit::C_VIOLET,
            text: format!(
                "Hunting a shiny? A parent traded from another player makes it 1 in {} instead of 1 in {} (Masuda Method).",
                thousands(breed::shiny_one_in(breed::shiny_rolls(s.shiny_charm, true)).round() as u32),
                thousands(e.shiny_one_in.round() as u32)
            ),
            gain: None,
            apply: None,
        });
    }
    out.truncate(5);
    out
}

pub(super) fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
