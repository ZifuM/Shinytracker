//! Battle Sim → the opponent: roll a real wild encounter from a place in
//! the game, build any Pokémon down to its IVs, or build a trainer.

use super::pokedex::type_color;
use super::sim_home::{side_card, RED};
use super::team_build::{best_ways, holdable, learnable, mkey, nature_label, pickable};
use super::team_calc::search_pick;
use super::team_home::{caps, chip, now_secs, small, soft, type_tag, x_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World, STATS};
use crate::sim::{self, Ai, Format, Mode, Rng, SavedFoe, Trainer};
use crate::teams::Member;
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// How you're picking the opponent.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum FoeTab {
    #[default]
    Route,
    Build,
    Saved,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Time {
    Morning,
    #[default]
    Day,
    Night,
}

impl Time {
    fn label(self) -> &'static str {
        match self {
            Time::Morning => "Morning",
            Time::Day => "Day",
            Time::Night => "Night",
        }
    }
    /// What time it is now (Platinum: morning 4–10, day 10–20, night 20–4).
    pub fn now() -> Time {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
        let h = ((secs + crate::stats::utc_offset_secs()) / 3600).rem_euclid(24);
        match h {
            4..=9 => Time::Morning,
            10..=19 => Time::Day,
            _ => Time::Night,
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct FoeView {
    pub tab: FoeTab,
    /// Which wild Pokémon (doubles) or trainer member is being edited.
    pub sel: usize,
    /// The place (area key), method and time for rolling.
    pub area: String,
    pub method: String,
    pub time: Option<Time>,
    /// What the last roll gave, in words.
    pub rolled: Option<String>,
}

/// One kind of Pokémon you can meet in a place.
#[derive(Clone, Debug)]
pub(super) struct Slot {
    pub id: String,
    pub name: String,
    pub lv: (u8, u8),
    /// % for the chosen time.
    pub chance: u32,
}

const METHODS: [(&str, &str); 7] = [
    ("grass", "Grass"),
    ("cave", "Walking"),
    ("surf", "Surfing"),
    ("old_rod", "Old Rod"),
    ("good_rod", "Good Rod"),
    ("super_rod", "Super Rod"),
    ("radar", "Poké Radar"),
];

fn method_label(m: &str) -> &str {
    METHODS.iter().find(|x| x.0 == m).map(|x| x.1).unwrap_or(m)
}

/// Does your world have this wild slot?
fn has_setting(setting: &str, world: World, more: bool) -> bool {
    match setting {
        "modern" => world == World::Modern,
        "more" => more,
        _ => true,
    }
}

/// Places with wild Pokémon, in story order: (area key, name, methods).
pub(super) fn places(w: &Wiki, world: World, more: bool) -> Vec<(String, String, Vec<String>)> {
    let mut map: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for e in &w.data.wild {
        if !has_setting(&e.setting, world, more) {
            continue;
        }
        let v = map.entry(e.area.clone()).or_default();
        if !v.contains(&e.method) {
            v.push(e.method.clone());
        }
    }
    let mut out: Vec<(usize, String, String, Vec<String>)> = map
        .into_iter()
        .map(|(k, mut ms)| {
            ms.sort_by_key(|m| METHODS.iter().position(|x| x.0 == m).unwrap_or(99));
            let p = w.place(&k);
            (p.map(|p| w.stage_index(&p.stage)).unwrap_or(usize::MAX), p.map(|p| p.name.clone()).unwrap_or_else(|| k.clone()), k, ms)
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, n, k, ms)| (k, n, ms)).collect()
}

/// What you can meet in a place, by one method at one time.
pub(super) fn table(w: &Wiki, area: &str, method: &str, time: Time, world: World, more: bool) -> Vec<Slot> {
    let mut out: Vec<Slot> = Vec::new();
    for e in w.data.wild.iter().filter(|e| e.area == area && e.method == method && has_setting(&e.setting, world, more)) {
        let c = match time {
            Time::Morning => e.chance.morning,
            Time::Day => e.chance.day,
            Time::Night => e.chance.night,
        };
        if c == 0 {
            continue;
        }
        let lv = sim::parse_levels(&e.levels);
        match out.iter_mut().find(|s| s.id == e.species) {
            Some(s) => {
                s.chance += c;
                s.lv = (s.lv.0.min(lv.0), s.lv.1.max(lv.1));
            }
            None => out.push(Slot { id: e.species.clone(), name: w.mon(&e.species).map(|m| m.name.clone()).unwrap_or_else(|| e.species.clone()), lv, chance: c }),
        }
    }
    out.sort_by(|a, b| b.chance.cmp(&a.chance).then(a.name.cmp(&b.name)));
    out
}

/// A wild Pokémon as the game would make it: a level in range, a random
/// nature, IVs, ability and gender, and the moves it knows at that level.
pub(super) fn roll(w: &Wiki, wiki_id: &str, lv: (u8, u8), world: World, rng: &mut Rng) -> Option<Member> {
    let d = battle::data();
    let sp = d.species(wiki_id)?;
    let level = lv.0 + rng.below((lv.1.saturating_sub(lv.0)) as u32 + 1) as u8;
    let mut m = Member::new(&sp.name, world);
    m.level = level;
    m.item.clear();
    m.nature = d.natures.get(rng.below(d.natures.len() as u32) as usize).map(|n| n.name.clone()).unwrap_or_else(|| "Hardy".into());
    m.ivs = sim::random_ivs(rng);
    m.evs = [0; 6];
    let mut abs = sp.abilities_in(world);
    if world == World::Modern && abs.len() > 1 {
        abs.retain(|a| *a != sp.hidden);
    }
    if !abs.is_empty() {
        m.ability = abs[rng.below(abs.len() as u32) as usize].clone();
    }
    m.gender = sim::random_gender(sim::data().gender(&sp.name), rng);
    m.moves = wild_set(w, &sp.name, level, world);
    m.happiness = 70;
    Some(m)
}

/// The moves a wild one knows at this level.
pub(super) fn wild_set(w: &Wiki, species: &str, level: u8, world: World) -> Vec<String> {
    let Some(id) = super::team_build::wiki_id(species) else { return Vec::new() };
    let Some(ls) = w.learnset(&id) else { return Vec::new() };
    let list = if world == World::Modern && !ls.mlv.is_empty() { &ls.mlv } else { &ls.lv };
    let pairs: Vec<(String, String)> = list.iter().map(|l| (l.level.clone(), l.key.clone())).collect();
    sim::wild_moves(&pairs, level).into_iter().filter_map(|k| w.move_info(&k).map(|i| i.n.clone())).collect()
}

/// "Lv 18 · Adamant · ♂ · IVs 31/4/…"
pub(super) fn describe(m: &Member) -> String {
    let g = match m.gender {
        Some('M') => " · ♂",
        Some('F') => " · ♀",
        _ => "",
    };
    format!("Lv {} · {}{g} · IVs {}", m.level, m.nature, m.ivs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("/"))
}

pub(super) const CLASSES: [&str; 14] = [
    "Trainer", "Ace Trainer", "Gym Leader", "Elite Four", "Champion", "Rival", "Team Galactic", "Veteran", "Black Belt", "Psychic", "Swimmer", "Hiker", "Dragon Tamer", "Bug Catcher",
];

/// Items a trainer can use in battle.
pub(super) const TRAINER_ITEMS: [&str; 11] = ["Potion", "Super Potion", "Hyper Potion", "Max Potion", "Full Restore", "Full Heal", "X Attack", "X Defend", "X Special", "X Speed", "Guard Spec."];

impl App {
    fn sim_world(&mut self) -> World {
        let s = &self.setup.sim.setup;
        let team = s.team;
        team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default()
    }

    /// Their side for a wild battle or a catch test.
    pub(super) fn sim_wild(&mut self, ui: &mut egui::Ui) {
        let world = self.sim_world();
        let s = self.setup.sim.setup.clone();
        let doubles = s.format == Format::Doubles && s.mode == Mode::Wild;
        let want = if doubles { 2 } else { 1 };
        let mut foes = s.foes.clone();
        let lv0 = foes.first().map(|f| f.level).unwrap_or(20);
        foes.resize_with(want, || Member { level: lv0, ..Member::new("Bidoof", world) });
        let mut v = self.setup.sim.foe.clone();
        v.sel = v.sel.min(want - 1);
        let wiki = self.wiki.clone();
        let mut save_one: Option<Member> = None;
        side_card(ui, RED, if doubles { "THEIR SIDE · TWO WILD POKÉMON" } else { "THEIR SIDE · A WILD POKÉMON" }, |ui| {
            // in doubles: which one you're setting up
            if doubles {
                ui.horizontal(|ui| {
                    for (i, f) in foes.iter().enumerate() {
                        if chip(ui, &format!("{}  {} Lv {}", i + 1, f.species, f.level), v.sel == i, RED).clicked() {
                            v.sel = i;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            let sel = v.sel;
            self.foe_hero(ui, &foes[sel], world);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for (t, l) in [(FoeTab::Route, "🗺  From a route"), (FoeTab::Build, "🐾  Build it"), (FoeTab::Saved, "📌  Saved")] {
                    if chip(ui, l, v.tab == t, RED).clicked() {
                        v.tab = t;
                    }
                }
            });
            ui.add_space(10.0);
            match v.tab {
                FoeTab::Route => match wiki.as_deref() {
                    Some(w) => self.foe_route(ui, w, world, &mut v, &mut foes, doubles),
                    None => {
                        ui.label(small("Getting the game's data…"));
                    }
                },
                FoeTab::Build => {
                    let m = &mut foes[sel];
                    self.foe_editor(ui, m, world, &format!("wild{sel}"), RED);
                    ui.add_space(6.0);
                    if ui.link(RichText::new("Save this Pokémon").size(12.0)).on_hover_text("Keep it under Saved to use again").clicked() {
                        save_one = Some(m.clone());
                    }
                }
                FoeTab::Saved => self.foe_saved(ui, &mut foes[sel]),
            }
        });
        if foes != s.foes {
            self.setup.sim.setup.foes = foes;
        }
        self.setup.sim.foe = v;
        if let Some(m) = save_one {
            let id = self.setup.sim.new_id();
            let f = self.setup.sim.file();
            if !f.foes.iter().any(|x| x.member == m) {
                f.foes.insert(0, SavedFoe { id, member: m, world, updated: now_secs() });
                self.setup.sim.touch();
            }
        }
    }

    /// Sprite, name and what it's like.
    fn foe_hero(&mut self, ui: &mut egui::Ui, f: &Member, world: World) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 96.0), Sense::hover());
        let p = ui.painter().clone();
        let sp = battle::data().species(&f.species);
        let ty: Vec<String> = sp.map(|s| s.types_in(world).to_vec()).unwrap_or_default();
        let col = ty.first().map(|t| type_color(t)).unwrap_or(kit::C_SLATE);
        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), col, 0.12));
        let c = egui::pos2(r.min.x + 52.0, r.center().y);
        if !self.team_sprite(&p, &f.species, f.shiny, c, egui::vec2(88.0, 88.0)) {
            p.circle_filled(c, 30.0, theme::blend(kit::sunken(), col, 0.25));
            p.text(c, Align2::CENTER_CENTER, f.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(16.0), MUTED());
        }
        let x = r.min.x + 110.0;
        let tw = r.max.x - x - 10.0;
        let fb = crate::fonts::sans_bold_font(17.0);
        p.text(egui::pos2(x, r.min.y + 22.0), Align2::LEFT_CENTER, super::charts::clip(ui, &format!("{}{}", f.display_name(), if f.shiny { "  ✨" } else { "" }), &fb, tw), fb, TEXT());
        let mut tx = x;
        for t in &ty {
            tx += type_tag(&p, egui::pos2(tx, r.min.y + 44.0), t, 9.0) + 4.0;
        }
        let rate = sp.and_then(|s| sim::data().catch_rate(s.num));
        if let Some(rate) = rate {
            p.text(egui::pos2(tx + 6.0, r.min.y + 44.0), Align2::LEFT_CENTER, format!("catch rate {rate}"), crate::fonts::sans_font(11.0), MUTED());
        }
        let f2 = crate::fonts::sans_font(11.5);
        p.text(egui::pos2(x, r.min.y + 64.0), Align2::LEFT_CENTER, super::charts::clip(ui, &describe(f), &f2, tw), f2.clone(), MUTED());
        let mv = if f.moves.is_empty() { "No moves".to_string() } else { f.moves.join(" · ") };
        p.text(egui::pos2(x, r.min.y + 82.0), Align2::LEFT_CENTER, super::charts::clip(ui, &format!("{} · {mv}", if f.ability.is_empty() { "—" } else { f.ability.as_str() }), &f2, tw), f2, MUTED());
    }

    /// Pick a place, a way to meet them and a time; roll one or click one.
    fn foe_route(&mut self, ui: &mut egui::Ui, w: &Wiki, world: World, v: &mut FoeView, foes: &mut [Member], doubles: bool) {
        let more = self.cfg.info_more;
        let list = places(w, world, more);
        if list.is_empty() {
            ui.label(small("No wild Pokémon in the game's data."));
            return;
        }
        if !list.iter().any(|p| p.0 == v.area) {
            v.area = list[0].0.clone();
        }
        let time = *v.time.get_or_insert_with(Time::now);
        let (_, pname, methods) = list.iter().find(|p| p.0 == v.area).cloned().unwrap_or_default();
        if !methods.contains(&v.method) {
            v.method = methods.first().cloned().unwrap_or_default();
        }
        let names: Vec<String> = list.iter().map(|p| p.1.clone()).collect();
        ui.horizontal(|ui| {
            ui.label(caps("PLACE"));
            ui.add_space(6.0);
            if let Some(n) = search_pick(ui, egui::Id::new("sim-place"), &pname, &names, 230.0) {
                if let Some(p) = list.iter().find(|p| p.1 == n) {
                    v.area = p.0.clone();
                    v.rolled = None;
                }
            }
            if let Some(stage) = w.place(&v.area).map(|p| p.stage.clone()).filter(|s| !s.is_empty()) {
                ui.label(small(stage));
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for m in &methods {
                if chip(ui, method_label(m), v.method == *m, RED).clicked() {
                    v.method = m.clone();
                    v.rolled = None;
                }
            }
            ui.add_space(10.0);
            for t in [Time::Morning, Time::Day, Time::Night] {
                if chip(ui, t.label(), time == t, kit::C_INDIGO).on_hover_text("Some Pokémon only come out at some times of day").clicked() {
                    v.time = Some(t);
                }
            }
        });
        let time = v.time.unwrap_or_default();
        let slots = table(w, &v.area, &v.method, time, world, more);
        ui.add_space(8.0);
        let mut chosen: Option<Slot> = None;
        let mut roll_any = false;
        ui.horizontal(|ui| {
            roll_any = kit::accent_button_w(ui, egui::Button::new(RichText::new(if doubles { "🎲  Roll two encounters" } else { "🎲  Roll an encounter" }).size(13.0)), 8.0, 150.0).clicked();
            ui.label(small("by the game's odds"));
        });
        if let Some(r) = &v.rolled {
            ui.add_space(4.0);
            ui.label(RichText::new(r).size(12.0).color(super::charts::ink(RED)));
        }
        ui.add_space(6.0);
        if slots.is_empty() {
            ui.label(small(format!("Nothing to meet this way at this time of day ({}).", time.label().to_lowercase())));
        }
        let full = ui.available_width();
        for s in &slots {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 38.0), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter().clone();
            let on = foes.get(v.sel).and_then(|f| battle::data().species(&f.species)).is_some_and(|sp| battle::to_id(&sp.name) == battle::to_id(&s.name));
            p.rect_filled(r, 8.0, if on { theme::blend(CARD(), RED, 0.12) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
            let c = egui::pos2(r.min.x + 20.0, r.center().y);
            if !self.team_sprite(&p, &s.name, false, c, egui::vec2(34.0, 34.0)) {
                p.text(c, Align2::CENTER_CENTER, s.name.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(10.0), MUTED());
            }
            p.text(egui::pos2(r.min.x + 44.0, r.center().y), Align2::LEFT_CENTER, &s.name, crate::fonts::sans_bold_font(12.5), TEXT());
            let lv = if s.lv.0 == s.lv.1 { format!("Lv {}", s.lv.0) } else { format!("Lv {}–{}", s.lv.0, s.lv.1) };
            p.text(egui::pos2(r.min.x + 170.0, r.center().y), Align2::LEFT_CENTER, lv, crate::fonts::sans_font(11.5), MUTED());
            let bx = r.min.x + 250.0;
            let bw = (r.max.x - bx - 50.0).max(30.0);
            let bar = Rect::from_min_size(egui::pos2(bx, r.center().y - 4.0), egui::vec2(bw, 8.0));
            p.rect_filled(bar, 4.0, kit::sunken());
            p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bw * (s.chance.min(100) as f32 / 100.0), 8.0)), 4.0, theme::blend(RED, MUTED(), 0.2));
            p.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, format!("{}%", s.chance), crate::fonts::sans_bold_font(12.0), TEXT());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Meet this one (a random one of them)").clicked() {
                chosen = Some(s.clone());
            }
        }
        // make them
        let mut rng = sim::Rng::from_time();
        let mut made: Vec<(usize, Member)> = Vec::new();
        if let Some(s) = chosen {
            if let Some(m) = roll(w, &s.id, s.lv, world, &mut rng) {
                made.push((v.sel, m));
            }
        } else if roll_any && !slots.is_empty() {
            let total: u32 = slots.iter().map(|s| s.chance).sum();
            let n = if doubles { 2 } else { 1 };
            for k in 0..n {
                let mut pick = rng.below(total.max(1));
                let s = slots.iter().find(|s| {
                    if pick < s.chance {
                        true
                    } else {
                        pick -= s.chance;
                        false
                    }
                });
                if let Some(s) = s {
                    if let Some(m) = roll(w, &s.id, s.lv, world, &mut rng) {
                        made.push((if doubles { k } else { v.sel }, m));
                    }
                }
            }
        }
        if !made.is_empty() {
            let names: Vec<String> = made.iter().map(|(_, m)| format!("{} Lv {}", m.species, m.level)).collect();
            v.rolled = Some(format!("A wild {} appeared!", names.join(" and a wild ")));
            for (i, m) in made {
                if let Some(f) = foes.get_mut(i) {
                    *f = m;
                }
            }
            // what the catch odds should know about this place
            let cave = w.place(&v.area).is_some_and(|p| p.cave) || v.method == "cave";
            let c = &mut self.setup.sim.catch;
            c.sit.night_or_cave = cave || time == Time::Night;
            c.sit.water = matches!(v.method.as_str(), "surf" | "old_rod" | "good_rod" | "super_rod");
            c.sit.turn = 0;
            c.hp = 1.0;
            self.setup.sim.setup.place = pname;
            self.setup.sim.setup.method = v.method.clone();
        }
    }

    fn foe_saved(&mut self, ui: &mut egui::Ui, foe: &mut Member) {
        let saved: Vec<SavedFoe> = self.setup.sim.file().foes.clone();
        let mut forget: Option<u64> = None;
        if saved.is_empty() {
            ui.label(small("Nothing saved yet. Build one or roll one, then \"Save this Pokémon\"."));
            return;
        }
        let full = ui.available_width();
        for sf in saved.iter().take(20) {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 40.0), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter().clone();
            p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
            let c = egui::pos2(r.min.x + 20.0, r.center().y);
            if !self.team_sprite(&p, &sf.member.species, sf.member.shiny, c, egui::vec2(34.0, 34.0)) {
                p.text(c, Align2::CENTER_CENTER, sf.member.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(10.0), MUTED());
            }
            p.text(egui::pos2(r.min.x + 44.0, r.min.y + 13.0), Align2::LEFT_CENTER, sf.member.display_name(), crate::fonts::sans_bold_font(12.5), TEXT());
            let f = crate::fonts::sans_font(11.0);
            p.text(egui::pos2(r.min.x + 44.0, r.min.y + 29.0), Align2::LEFT_CENTER, super::charts::clip(ui, &describe(&sf.member), &f, full - 90.0), f, MUTED());
            let xr = Rect::from_center_size(egui::pos2(r.max.x - 18.0, r.center().y), egui::vec2(24.0, 24.0));
            let xresp = ui.interact(xr, resp.id.with("x"), Sense::click());
            let xc = if xresp.hovered() { TEXT() } else { MUTED() };
            p.line_segment([xr.center() + egui::vec2(-4.5, -4.5), xr.center() + egui::vec2(4.5, 4.5)], Stroke::new(1.6f32, xc));
            p.line_segment([xr.center() + egui::vec2(-4.5, 4.5), xr.center() + egui::vec2(4.5, -4.5)], Stroke::new(1.6f32, xc));
            if xresp.on_hover_text("Forget it").clicked() {
                forget = Some(sf.id);
            } else if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *foe = sf.member.clone();
            }
            ui.add_space(4.0);
        }
        if let Some(id) = forget {
            self.setup.sim.file().foes.retain(|x| x.id != id);
            self.setup.sim.touch();
        }
    }

    /// Everything about one Pokémon, in a compact form.
    pub(super) fn foe_editor(&mut self, ui: &mut egui::Ui, m: &mut Member, world: World, key: &str, col: Color32) {
        let d = battle::data();
        let wiki = self.wiki.clone();
        let names: Vec<String> = pickable(world).iter().map(|s| s.name.clone()).collect();
        let Some(sp) = d.species(&m.species) else { return };
        egui::Grid::new(("foe-ed", key)).num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
            ui.label(caps("POKÉMON"));
            if let Some(n) = search_pick(ui, egui::Id::new(("foe-sp", key)), &m.species, &names, 200.0) {
                let lv = m.level;
                *m = Member { level: lv, ..Member::new(&n, world) };
                m.item.clear();
                if let Some(w) = wiki.as_deref() {
                    m.moves = wild_set(w, &n, lv, world);
                }
            }
            ui.end_row();
            ui.label(caps("LEVEL"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut m.level).range(1..=100).speed(0.2));
                for l in [5u8, 25, 50, 100] {
                    if chip(ui, &l.to_string(), m.level == l, col).clicked() {
                        m.level = l;
                    }
                }
            });
            ui.end_row();
            ui.label(caps("ABILITY"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for a in sp.abilities_in(world) {
                    if chip(ui, &a, m.ability == a, col).clicked() {
                        m.ability = a.clone();
                    }
                }
            });
            ui.end_row();
            ui.label(caps("NATURE"));
            egui::ComboBox::from_id_salt(("foe-nat", key)).selected_text(nature_label(&m.nature)).width(200.0).height(320.0).show_ui(ui, |ui| {
                for n in &d.natures {
                    ui.selectable_value(&mut m.nature, n.name.clone(), nature_label(&n.name));
                }
            });
            ui.end_row();
            ui.label(caps("ITEM"));
            let items: Vec<String> = std::iter::once("(none)".to_string()).chain(holdable(wiki.as_deref(), world).iter().map(|i| i.n.clone())).collect();
            let cur = if m.item.is_empty() { "(none)".to_string() } else { m.item.clone() };
            if let Some(n) = search_pick(ui, egui::Id::new(("foe-item", key)), &cur, &items, 200.0) {
                m.item = if n == "(none)" { String::new() } else { n };
            }
            ui.end_row();
            ui.label(caps("GENDER"));
            ui.horizontal(|ui| {
                let g = sim::data().gender(&m.species);
                let opts: Vec<(Option<char>, &str)> = match g {
                    sim::Gender::Male => vec![(Some('M'), "♂ only")],
                    sim::Gender::Female => vec![(Some('F'), "♀ only")],
                    sim::Gender::None => vec![(None, "None")],
                    sim::Gender::Ratio(_) => vec![(Some('M'), "♂"), (Some('F'), "♀"), (None, "Any")],
                };
                for (gv, l) in opts {
                    if chip(ui, l, m.gender == gv, col).clicked() {
                        m.gender = gv;
                    }
                }
                ui.add_space(10.0);
                kit::toggle(ui, &mut m.shiny);
                ui.label(small("Shiny"));
            });
            ui.end_row();
        });
        // moves: what it can learn first
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(caps("MOVES"));
            if let Some(w) = wiki.as_deref() {
                if ui.link(RichText::new("What a wild one knows at this level").size(11.5)).clicked() {
                    m.moves = wild_set(w, &m.species, m.level, world);
                }
            }
        });
        ui.add_space(4.0);
        let options: Vec<String> = match wiki.as_deref() {
            Some(w) => {
                let ways = learnable(w, &m.species, world);
                let best = best_ways(&ways);
                let mut v: Vec<String> = best.iter().filter_map(|l| w.move_info(&l.key)).map(|i| i.n.clone()).collect();
                v.sort();
                v
            }
            None => Vec::new(),
        };
        for i in 0..4 {
            let cur = m.moves.get(i).cloned().unwrap_or_else(|| "(empty)".into());
            ui.horizontal(|ui| {
                let info = wiki.as_deref().and_then(|w| mkey(w, &cur).and_then(|k| w.move_info(&k).map(|i| (i.t.clone(), i.c.clone(), i.p.clone()))));
                let (r, _) = ui.allocate_exact_size(egui::vec2(6.0, 26.0), Sense::hover());
                ui.painter().rect_filled(r, 3.0, info.as_ref().map(|i| type_color(&i.0)).unwrap_or(LINE()));
                if let Some(n) = search_pick(ui, egui::Id::new(("foe-mv", key, i)), &cur, &options, (ui.available_width() - 190.0).max(120.0)) {
                    if i < m.moves.len() {
                        m.moves[i] = n;
                    } else {
                        m.moves.push(n);
                    }
                }
                let (ir, _) = ui.allocate_exact_size(egui::vec2(110.0, 26.0), Sense::hover());
                if let Some((t, c, p)) = &info {
                    let pw = if p.is_empty() || p == "0" { "—".to_string() } else { p.clone() };
                    let f = crate::fonts::sans_font(11.0);
                    ui.painter().text(egui::pos2(ir.min.x, ir.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &format!("{t} · {} {pw}", &c[..c.len().min(4)]), &f, 110.0), f, MUTED());
                }
                if i < m.moves.len() && x_button(ui).on_hover_text("Clear").clicked() {
                    m.moves.remove(i);
                }
            });
        }
        // stats, EVs and IVs
        ui.add_space(8.0);
        let open_id = egui::Id::new(("foe-stats-open", key));
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
        let stats = m.final_stats(world).unwrap_or_default();
        let head = format!("Stats  {}", (0..6).map(|i| format!("{} {}", STATS[i], stats[i])).collect::<Vec<_>>().join(" · "));
        if kit::expander(ui, &head, "EVs and IVs", open).clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
        }
        if open {
            ui.add_space(6.0);
            egui::Grid::new(("foe-stats", key)).num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(caps(""));
                ui.label(caps("EVS"));
                ui.label(caps("IVS"));
                ui.label(caps("STAT"));
                ui.end_row();
                for i in 0..6 {
                    ui.label(RichText::new(STATS[i]).font(crate::fonts::sans_bold_font(12.0)).color(TEXT()));
                    ui.add(egui::DragValue::new(&mut m.evs[i]).range(0..=252).speed(2.0));
                    ui.add(egui::DragValue::new(&mut m.ivs[i]).range(0..=31).speed(0.2));
                    ui.label(RichText::new(stats[i].to_string()).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if soft(ui, "Random IVs").on_hover_text("Like a wild one").clicked() {
                    m.ivs = sim::random_ivs(&mut sim::Rng::from_time());
                }
                if soft(ui, "All 31").clicked() {
                    m.ivs = [31; 6];
                }
                if soft(ui, "No EVs").clicked() {
                    m.evs = [0; 6];
                }
            });
        }
        // Smogon's sets for it
        let sets = d.sets_for(&m.species);
        if !sets.is_empty() {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("SETS"));
                for s in sets.into_iter().take(5) {
                    let short = s.n.split_once(' ').filter(|(t, _)| ["OU", "UU", "Ubers", "NU", "RU", "LC", "UUBL"].contains(t)).map(|(_, r)| r).unwrap_or(&s.n);
                    if chip(ui, short, m.moves == s.m, col).on_hover_text(format!("{}\n{}", s.n, s.m.join(" / "))).clicked() {
                        m.item = s.item.clone();
                        m.nature = s.nat.clone();
                        m.evs = s.ev;
                        m.ivs = s.iv;
                        m.moves = s.m.clone();
                        if sp.abilities_in(world).contains(&s.ab) {
                            m.ability = s.ab.clone();
                        }
                    }
                }
            });
        }
    }

    /// Their side for a trainer battle: pick one, or build one.
    pub(super) fn sim_trainers(&mut self, ui: &mut egui::Ui) {
        let s = self.setup.sim.setup.clone();
        let trainers: Vec<Trainer> = self.setup.sim.file().trainers.clone();
        let teams: Vec<crate::teams::Team> = self.setup.team.teams().teams.clone();
        let mut pick: Option<u64> = None;
        let mut from_team: Option<u64> = None;
        let mut new_blank = false;
        let mut edit: Option<Trainer> = None;
        let mut delete: Option<u64> = None;
        let mut copy: Option<u64> = None;
        let mut v = self.setup.sim.foe.clone();
        side_card(ui, RED, "THEIR SIDE · A TRAINER", |ui| {
            if trainers.is_empty() {
                ui.label(RichText::new("No trainers yet").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.label(small("Build a trainer from scratch, or turn one of your Team Planner teams into one (a mirror match, or a friend's team you pasted in)."));
                ui.add_space(8.0);
            }
            for t in &trainers {
                let on = s.trainer == Some(t.id);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 60.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                p.rect_filled(r, 12.0, if on { theme::blend(CARD(), RED, 0.14) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                if on {
                    p.rect_stroke(r, 12.0, Stroke::new(1.5f32, RED));
                }
                let f1 = crate::fonts::sans_bold_font(14.0);
                p.text(egui::pos2(r.min.x + 14.0, r.min.y + 19.0), Align2::LEFT_CENTER, super::charts::clip(ui, &t.name, &f1, 200.0), f1, TEXT());
                let ai = match t.ai {
                    Ai::Random => "random moves",
                    Ai::Smart => "smart",
                    Ai::Expert => "expert",
                };
                p.text(egui::pos2(r.min.x + 14.0, r.min.y + 40.0), Align2::LEFT_CENTER, format!("{}  ·  {} Pokémon  ·  {ai}", if t.class.is_empty() { "Trainer" } else { t.class.as_str() }, t.members.len()), crate::fonts::sans_font(11.5), MUTED());
                let slot = 36.0;
                let n = t.members.len().min(6);
                for (i, m) in t.members.iter().take(6).enumerate() {
                    let c = egui::pos2(r.max.x - 12.0 - slot * (n - i) as f32 + slot / 2.0, r.center().y);
                    if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(slot, slot)) {
                        p.circle_filled(c, 14.0, theme::blend(CARD(), RED, 0.12));
                        p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(10.0), MUTED());
                    }
                }
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    pick = Some(t.id);
                }
                resp.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        copy = Some(t.id);
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        delete = Some(t.id);
                        ui.close_menu();
                    }
                });
                ui.add_space(6.0);
            }
            ui.horizontal(|ui| {
                new_blank = soft(ui, "+  New trainer").clicked();
                ui.menu_button(RichText::new("From a team").size(13.0), |ui| {
                    ui.set_min_width(220.0);
                    if teams.iter().all(|t| t.members.is_empty()) {
                        ui.label("Make a team in the Team Planner first");
                    }
                    for t in teams.iter().filter(|t| !t.members.is_empty()).take(15) {
                        if ui.button(format!("{}  ({})", t.name, t.members.len())).clicked() {
                            from_team = Some(t.id);
                            ui.close_menu();
                        }
                    }
                });
            });
            if !trainers.is_empty() {
                ui.label(small("Right-click a trainer to duplicate or delete it."));
            }
            // the chosen trainer, to edit
            if let Some(t) = s.trainer.and_then(|id| trainers.iter().find(|t| t.id == id)) {
                ui.add_space(12.0);
                let line = ui.available_width();
                let (lr, _) = ui.allocate_exact_size(egui::vec2(line, 1.0), Sense::hover());
                ui.painter().rect_filled(lr, 0.0, LINE());
                ui.add_space(12.0);
                let mut t2 = t.clone();
                self.trainer_editor(ui, &mut t2, &mut v, &teams);
                if t2 != *t {
                    t2.updated = now_secs();
                    edit = Some(t2);
                }
            }
        });
        self.setup.sim.foe = v;
        let world = self.sim_world();
        if let Some(id) = pick {
            self.setup.sim.setup.trainer = Some(id);
            self.setup.sim.foe.sel = 0;
        }
        if new_blank {
            let id = self.setup.sim.new_id();
            let n = self.setup.sim.file().trainers.len() + 1;
            let mut first = Member { level: 30, ..Member::new("Staravia", world) };
            if let Some(w) = self.wiki.clone() {
                first.moves = wild_set(&w, "Staravia", 30, world);
            }
            let tr = Trainer { id, name: format!("Trainer {n}"), class: "Ace Trainer".into(), world, members: vec![first], updated: now_secs(), ..Default::default() };
            self.setup.sim.file().trainers.insert(0, tr);
            self.setup.sim.setup.trainer = Some(id);
            self.setup.sim.foe.sel = 0;
            self.setup.sim.touch();
        }
        if let Some(tid) = from_team {
            if let Some(t) = teams.iter().find(|t| t.id == tid) {
                let id = self.setup.sim.new_id();
                let tr = Trainer {
                    id,
                    name: if t.name.trim().is_empty() { "Trainer".into() } else { t.name.clone() },
                    class: "Ace Trainer".into(),
                    world: t.world,
                    members: t.members.clone(),
                    updated: now_secs(),
                    ..Default::default()
                };
                self.setup.sim.file().trainers.insert(0, tr);
                self.setup.sim.setup.trainer = Some(id);
                self.setup.sim.foe.sel = 0;
                self.setup.sim.touch();
            }
        }
        if let Some(t) = edit {
            if let Some(x) = self.setup.sim.file().trainers.iter_mut().find(|x| x.id == t.id) {
                *x = t;
            }
            self.setup.sim.touch();
        }
        if let Some(id) = copy {
            let nid = self.setup.sim.new_id();
            let f = self.setup.sim.file();
            if let Some(i) = f.trainers.iter().position(|t| t.id == id) {
                let mut t = f.trainers[i].clone();
                t.id = nid;
                t.name = format!("{} (copy)", t.name);
                f.trainers.insert(i + 1, t);
            }
            self.setup.sim.touch();
        }
        if let Some(id) = delete {
            self.setup.sim.file().trainers.retain(|t| t.id != id);
            if self.setup.sim.setup.trainer == Some(id) {
                self.setup.sim.setup.trainer = None;
            }
            self.setup.sim.touch();
        }
    }

    /// Name, class, how it battles, items, and its team.
    fn trainer_editor(&mut self, ui: &mut egui::Ui, t: &mut Trainer, v: &mut FoeView, teams: &[crate::teams::Team]) {
        egui::Grid::new("trainer-ed").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
            ui.label(caps("NAME"));
            ui.add(egui::TextEdit::singleline(&mut t.name).desired_width(200.0).char_limit(24));
            ui.end_row();
            ui.label(caps("CLASS"));
            egui::ComboBox::from_id_salt("trainer-class").selected_text(if t.class.is_empty() { "Trainer" } else { t.class.as_str() }).width(200.0).show_ui(ui, |ui| {
                for c in CLASSES {
                    ui.selectable_value(&mut t.class, c.to_string(), c);
                }
            });
            ui.end_row();
            ui.label(caps("BATTLES"));
            ui.horizontal(|ui| {
                for (a, l, tip) in [
                    (Ai::Random, "Random", "Uses any of its moves at random, like a wild Pokémon"),
                    (Ai::Smart, "Smart", "Goes for strong hits, KOs and good type matchups, like a gym leader"),
                    (Ai::Expert, "Expert", "Smart, and also switches out of bad matchups and uses its items well"),
                ] {
                    if chip(ui, l, t.ai == a, RED).on_hover_text(tip).clicked() {
                        t.ai = a;
                    }
                }
            });
            ui.end_row();
            ui.label(caps("WORLD"));
            ui.horizontal(|ui| {
                for (w, l) in [(World::Platinum, "Platinum"), (World::Modern, "MODERN")] {
                    if chip(ui, l, t.world == w, RED).clicked() {
                        t.world = w;
                    }
                }
            });
            ui.end_row();
            ui.label(caps("ITEMS"));
            ui.vertical(|ui| {
                let mut remove: Option<usize> = None;
                for (i, (name, n)) in t.items.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(n).range(1..=9).prefix("×"));
                        ui.label(RichText::new(name.as_str()).size(12.5).color(TEXT()));
                        if x_button(ui).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    t.items.remove(i);
                }
                if t.items.len() < 4 {
                    ui.menu_button(RichText::new("+  Item it can use").size(12.0), |ui| {
                        for it in TRAINER_ITEMS {
                            if ui.button(it).clicked() {
                                match t.items.iter_mut().find(|x| x.0 == it) {
                                    Some(x) => x.1 = (x.1 + 1).min(9),
                                    None => t.items.push((it.to_string(), 1)),
                                }
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
            ui.end_row();
        });
        // its team: six small slots
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(caps("ITS TEAM"));
            ui.label(small(format!("{} of 6", t.members.len())));
        });
        ui.add_space(4.0);
        v.sel = v.sel.min(t.members.len().saturating_sub(1));
        let mut remove: Option<usize> = None;
        let mut add_from: Option<Member> = None;
        let full = ui.available_width();
        let w = ((full - 6.0 * 5.0) / 6.0).floor().min(76.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (i, m) in t.members.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, w + 16.0), Sense::click());
                let on = v.sel == i;
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                let br = Rect::from_min_size(r.min, egui::vec2(w, w));
                p.rect_filled(br, 10.0, if on { theme::blend(CARD(), RED, 0.16) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                if on {
                    p.rect_stroke(br, 10.0, Stroke::new(1.5f32, RED));
                }
                if !self.team_sprite(&p, &m.species, m.shiny, br.center(), egui::vec2(w - 6.0, w - 6.0)) {
                    p.text(br.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(12.0), MUTED());
                }
                p.text(egui::pos2(br.center().x, r.max.y - 6.0), Align2::CENTER_CENTER, format!("Lv {}", m.level), crate::fonts::sans_font(10.5), MUTED());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} · right-click to remove", m.species));
                if resp.clicked() {
                    v.sel = i;
                }
                resp.context_menu(|ui| {
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                        ui.close_menu();
                    }
                });
            }
            if t.members.len() < 6 {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, w + 16.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let br = Rect::from_min_size(r.min, egui::vec2(w, w));
                ui.painter().rect_stroke(br, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), RED, 0.3 + 0.5 * ht)));
                ui.painter().text(br.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(20.0), MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Add a Pokémon").clicked() {
                    let lv = t.members.last().map(|m| m.level).unwrap_or(30);
                    let mut m = Member { level: lv, ..Member::new("Bibarel", t.world) };
                    if let Some(w) = self.wiki.clone() {
                        m.moves = wild_set(&w, "Bibarel", lv, t.world);
                    }
                    add_from = Some(m);
                }
            }
        });
        if t.members.len() < 6 {
            ui.horizontal(|ui| {
                ui.menu_button(RichText::new("Add one from a team").size(12.0), |ui| {
                    ui.set_min_width(240.0);
                    for tm in teams.iter().filter(|x| !x.members.is_empty()).take(10) {
                        ui.menu_button(tm.name.clone(), |ui| {
                            for m in &tm.members {
                                if ui.button(m.display_name()).clicked() {
                                    add_from = Some(m.clone());
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                });
                if !t.members.is_empty() {
                    ui.add_space(8.0);
                    let mut lv = t.members.iter().map(|m| m.level).max().unwrap_or(50);
                    ui.label(small("Set every level to"));
                    if ui.add(egui::DragValue::new(&mut lv).range(1..=100).speed(0.2)).changed() {
                        for m in &mut t.members {
                            m.level = lv;
                        }
                    }
                }
            });
        }
        if let Some(i) = remove {
            t.members.remove(i);
            v.sel = v.sel.min(t.members.len().saturating_sub(1));
        }
        if let Some(m) = add_from {
            if t.members.len() < 6 {
                t.members.push(m);
                v.sel = t.members.len() - 1;
            }
        }
        if let Some(m) = t.members.get_mut(v.sel) {
            ui.add_space(10.0);
            let world = t.world;
            let mut m2 = m.clone();
            self.foe_hero(ui, &m2, world);
            ui.add_space(8.0);
            self.foe_editor(ui, &mut m2, world, &format!("tr{}", v.sel), RED);
            if m2 != *m {
                *m = m2;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_rolls() {
        let w = crate::wiki::Wiki::parse(include_bytes!("../../assets/wiki.json")).expect("wiki");
        let ps = places(&w, World::Platinum, false);
        assert!(ps.len() > 50);
        let (area, _, methods) = ps.iter().find(|p| p.1 == "Route 201").cloned().expect("route 201");
        assert!(methods.contains(&"grass".to_string()));
        let t = table(&w, &area, "grass", Time::Day, World::Platinum, false);
        assert!(!t.is_empty());
        assert!(t.iter().all(|s| s.chance > 0 && s.lv.0 <= s.lv.1));
        let mut rng = Rng::new(7);
        for s in &t {
            let m = roll(&w, &s.id, s.lv, World::Platinum, &mut rng).expect("roll");
            assert!(m.level >= s.lv.0 && m.level <= s.lv.1, "{} {}", m.species, m.level);
            assert!(!m.moves.is_empty() && m.moves.len() <= 4, "{} {:?}", m.species, m.moves);
            assert!(m.ivs.iter().all(|v| *v <= 31));
        }
    }
}
