//! Team Planner → Analysis: what hits the team hard, what the team hits
//! hard, who's fastest, and anything that looks off.

use super::pokedex::type_color;
use super::team_build::{best_ways, learnable};
use super::team_home::{caps, small, ACCENT};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World};
use crate::calc;
use crate::teams::{Member, Team};
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// What the analysis needs to know about one member.
pub(super) struct Facts {
    pub name: String,
    pub species: String,
    pub shiny: bool,
    pub types: Vec<String>,
    pub ability: String,
    pub stats: [u16; 6],
    /// Damaging moves: (name, type, category, power).
    pub attacks: Vec<(String, String, String, u32)>,
    pub item: String,
}

pub(super) fn facts(w: Option<&Wiki>, world: World, m: &Member) -> Option<Facts> {
    let d = battle::data();
    let sp = d.species(&m.species)?;
    let mut attacks = Vec::new();
    for mv in &m.moves {
        let info = w.and_then(|w| super::team_build::mkey(w, mv).and_then(|k| w.move_info(&k)));
        let Some(info) = info else { continue };
        let mut t = info.t.clone();
        if mv.starts_with("Hidden Power") {
            t = battle::hidden_power(m.ivs).0.to_string();
        }
        let p = info.p.parse::<u32>().unwrap_or(0);
        if info.c != "Status" {
            attacks.push((info.n.clone(), t, info.c.clone(), p));
        }
    }
    Some(Facts {
        name: m.display_name().to_string(),
        species: m.species.clone(),
        shiny: m.shiny,
        types: sp.types_in(world).to_vec(),
        ability: m.ability.clone(),
        stats: m.final_stats(world).unwrap_or_default(),
        attacks,
        item: m.item.clone(),
    })
}

/// One thing worth knowing about the team.
pub(super) struct Note {
    pub bad: bool,
    pub text: String,
    /// The member it's about, to jump to.
    pub slot: Option<usize>,
}

const HAZARDS: [&str; 3] = ["Stealth Rock", "Spikes", "Toxic Spikes"];
const PRIORITY: [&str; 9] = ["Extreme Speed", "ExtremeSpeed", "Mach Punch", "Bullet Punch", "Aqua Jet", "Ice Shard", "Shadow Sneak", "Sucker Punch", "Vacuum Wave"];

pub(super) fn notes(w: Option<&Wiki>, t: &Team) -> Vec<Note> {
    let mut out = Vec::new();
    let fs: Vec<Option<Facts>> = t.members.iter().map(|m| facts(w, t.world, m)).collect();
    if t.members.is_empty() {
        return out;
    }
    for (i, m) in t.members.iter().enumerate() {
        let name = m.display_name();
        if m.moves.is_empty() {
            out.push(Note { bad: true, text: format!("{name} has no moves yet."), slot: Some(i) });
        } else if m.moves.len() < 4 {
            out.push(Note { bad: false, text: format!("{name} has {} of 4 moves.", m.moves.len()), slot: Some(i) });
        }
        if let Some(w) = w {
            let ways = learnable(w, &m.species, t.world);
            let best = best_ways(&ways);
            for mv in &m.moves {
                let ok = super::team_build::mkey(w, mv).is_some_and(|k| best.iter().any(|l| l.key == k)) || (mv.starts_with("Hidden Power") && best.iter().any(|l| l.key == "hidden_power"));
                if !ok {
                    out.push(Note { bad: true, text: format!("{name} can't learn {mv} in this world."), slot: Some(i) });
                }
            }
        }
        let left = battle::ev_left(m.evs);
        if left < 0 {
            out.push(Note { bad: true, text: format!("{name} has {} EVs too many (510 is the most).", -left), slot: Some(i) });
        } else if left >= 8 {
            out.push(Note { bad: false, text: format!("{name} has {left} EVs left to spend."), slot: Some(i) });
        }
        if let Some(sp) = battle::data().species(&m.species) {
            let abs = sp.abilities_in(t.world);
            if !m.ability.is_empty() && !abs.contains(&m.ability) {
                out.push(Note { bad: true, text: format!("{name} can't have {} here (it can have {}).", m.ability, abs.join(" or ")), slot: Some(i) });
            }
            if sp.modern_only() && t.world == World::Platinum {
                out.push(Note { bad: true, text: format!("{name} is only in MODERN POKéMON worlds."), slot: Some(i) });
            }
        }
    }
    // shared weaknesses
    let n = fs.iter().flatten().count();
    if n >= 3 {
        for at in calc::types_in(t.world) {
            let (weak, resist) = weak_resist(&fs, at, t.world);
            if weak >= 3 && resist == 0 {
                out.push(Note { bad: true, text: format!("{weak} are weak to {at} and none resist it."), slot: None });
            } else if weak >= 3 {
                out.push(Note { bad: false, text: format!("{weak} are weak to {at} (only {resist} resist it)."), slot: None });
            }
        }
        let all_moves: Vec<&String> = t.members.iter().flat_map(|m| m.moves.iter()).collect();
        if !all_moves.iter().any(|m| HAZARDS.contains(&m.as_str())) {
            out.push(Note { bad: false, text: "No entry hazards (Stealth Rock, Spikes): handy against switching.".into(), slot: None });
        }
        if !all_moves.iter().any(|m| PRIORITY.contains(&m.as_str()) || m.as_str() == "Quick Attack") {
            out.push(Note { bad: false, text: "No priority moves to finish off faster foes.".into(), slot: None });
        }
        let rock4: Vec<&str> = fs.iter().flatten().filter(|f| calc::eff("Rock", &f.types, t.world) >= 4.0).map(|f| f.name.as_str()).collect();
        if !rock4.is_empty() {
            out.push(Note { bad: false, text: format!("Stealth Rock takes half of {}'s HP each time it comes in.", rock4.join(" and ")), slot: None });
        }
        let phys = fs.iter().flatten().filter(|f| f.attacks.iter().any(|a| a.2 == "Physical")).count();
        let spec = fs.iter().flatten().filter(|f| f.attacks.iter().any(|a| a.2 == "Special")).count();
        if phys == 0 && spec >= 3 {
            out.push(Note { bad: false, text: "Every attacker is special: a bulky special wall like Blissey walls the team.".into(), slot: None });
        } else if spec == 0 && phys >= 3 {
            out.push(Note { bad: false, text: "Every attacker is physical: Skarmory and Will-O-Wisp users slow the team down.".into(), slot: None });
        }
    }
    out.sort_by_key(|n| !n.bad);
    out
}

/// The Battle Colosseum's rule sets.
pub(super) const RULESETS: [&str; 3] = ["OU", "UBERS", "ANYTHING GOES"];

const OU_BANNED: [&str; 31] = [
    "Mewtwo", "Mew", "Lugia", "Ho-Oh", "Celebi", "Kyogre", "Groudon", "Rayquaza", "Jirachi", "Deoxys", "Dialga", "Palkia", "Giratina", "Phione", "Manaphy", "Darkrai", "Shaymin", "Arceus",
    "Victini", "Reshiram", "Zekrom", "Xerneas", "Yveltal", "Zygarde", "Diancie", "Magearna", "Zeraora", "Urshifu", "Wobbuffet", "Wynaut", "Aegislash",
];
const OU_ITEMS: [&str; 17] = [
    "Soul Dew", "Gengarite", "Mewtwonite X", "Mewtwonite Y", "Blazikenite", "Lucarionite", "Lucarionite Z", "Kangaskhanite", "Mawilite", "Garchompite", "Garchompite Z", "Latiasite", "Latiosite", "Salamencite", "Darkranite", "Zygardite", "Magearnite",
];
const BANNED_MOVES: [&str; 7] = ["Double Team", "Minimize", "Fissure", "Guillotine", "Horn Drill", "Sheer Cold", "Swagger"];
const BANNED_ABILITIES: [&str; 3] = ["Sand Veil", "Snow Cloak", "Moody"];

/// What keeps the team out of a rule set (0 OU, 1 UBERS, 2 ANYTHING GOES).
pub(super) fn colosseum(t: &Team, rules: usize) -> Vec<String> {
    let d = battle::data();
    let id = |s: &str| battle::to_id(s);
    let mut out = Vec::new();
    let strict = rules < 2;
    for m in &t.members {
        let name = m.display_name();
        let base = d.species(&m.species).map(|s| if s.base.is_empty() { s.name.clone() } else { d.species(&s.base).map(|b| b.name.clone()).unwrap_or_default() }).unwrap_or_default();
        if rules == 0 && OU_BANNED.iter().any(|b| id(b) == id(&base) || id(b) == id(&m.species)) {
            out.push(format!("{name} isn't allowed in OU"));
        }
        if rules == 0 && OU_ITEMS.iter().any(|b| id(b) == id(&m.item)) {
            out.push(format!("{name}'s {} isn't allowed in OU", m.item));
        }
        if strict && ["BrightPowder", "Lax Incense"].iter().any(|b| id(b) == id(&m.item)) {
            out.push(format!("{name}'s {} isn't allowed", m.item));
        }
        if strict {
            for mv in &m.moves {
                if BANNED_MOVES.iter().any(|b| id(b) == id(mv)) {
                    out.push(format!("{name} has {mv}"));
                }
            }
            if BANNED_ABILITIES.contains(&m.ability.as_str()) {
                out.push(format!("{name} has {}", m.ability));
            }
        }
        if id(&m.item) == "leppaberry" && m.moves.iter().any(|x| id(x) == "recycle") {
            out.push(format!("{name} has Leppa Berry with Recycle"));
        }
    }
    if strict {
        let mut seen: Vec<String> = Vec::new();
        for m in &t.members {
            let s = id(&m.species);
            if seen.contains(&s) {
                out.push(format!("Two {}", m.species));
            }
            seen.push(s);
        }
        let mut items: Vec<String> = Vec::new();
        for m in &t.members {
            if m.item.is_empty() {
                continue;
            }
            if items.contains(&m.item) {
                out.push(format!("Two hold {}", m.item));
            }
            items.push(m.item.clone());
        }
        if t.members.iter().filter(|m| m.moves.iter().any(|x| id(x) == "batonpass")).count() > 1 {
            out.push("More than one knows Baton Pass".into());
        }
    }
    if t.members.iter().filter(|m| d.item(&m.item).is_some_and(|i| !i.mega.is_empty())).count() > 1 {
        out.push("More than one Mega Stone".into());
    }
    out.dedup();
    out
}

pub(super) fn weak_resist(fs: &[Option<Facts>], at: &str, world: World) -> (usize, usize) {
    let mut weak = 0;
    let mut resist = 0;
    for f in fs.iter().flatten() {
        let e = calc::ability_eff(at, &f.ability, calc::eff(at, &f.types, world));
        if e > 1.0 {
            weak += 1;
        } else if e < 1.0 {
            resist += 1;
        }
    }
    (weak, resist)
}

/// Fast Pokémon to measure Speed against: (name, base Speed).
pub(super) const BENCH: [(&str, u16); 10] = [
    ("Jolteon", 130),
    ("Weavile", 125),
    ("Starmie", 115),
    ("Gengar", 110),
    ("Infernape", 108),
    ("Garchomp", 102),
    ("Heatran", 77),
    ("Lucario", 90),
    ("Metagross", 70),
    ("Tyranitar", 61),
];

fn mult_label(m: f32) -> &'static str {
    match m {
        x if x == 0.0 => "0",
        x if x <= 0.25 => "¼",
        x if x <= 0.5 => "½",
        x if x >= 4.0 => "4×",
        x if x >= 2.0 => "2×",
        x if x > 1.0 => "+",
        x if x < 1.0 => "−",
        _ => "",
    }
}

fn mult_color(m: f32) -> Option<Color32> {
    if m == 0.0 {
        Some(Color32::from_rgb(110, 120, 140))
    } else if m >= 4.0 {
        Some(Color32::from_rgb(226, 60, 60))
    } else if m > 1.0 {
        Some(Color32::from_rgb(240, 120, 80))
    } else if m <= 0.25 {
        Some(Color32::from_rgb(40, 170, 120))
    } else if m < 1.0 {
        Some(Color32::from_rgb(80, 190, 150))
    } else {
        None
    }
}

impl App {
    pub(super) fn team_analysis(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.setup.team.team().cloned() else { return };
        if t.members.is_empty() {
            kit::card(ui, |ui| {
                ui.label(RichText::new("Add a Pokémon in Build first").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            });
            return;
        }
        let wiki = self.wiki.clone();
        let fs: Vec<Option<Facts>> = t.members.iter().map(|m| facts(wiki.as_deref(), t.world, m)).collect();
        self.analysis_summary(ui, &t, &fs);
        ui.add_space(16.0);
        let avail = ui.available_width();
        if avail >= 1100.0 {
            let gap = 16.0;
            let lw = ((avail - gap) * 0.56).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(lw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(lw);
                    self.analysis_defense(ui, &t, &fs);
                });
                let rw = ui.available_width();
                ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(rw);
                    self.analysis_notes(ui, &t);
                    self.analysis_offense(ui, &t, &fs);
                });
            });
        } else {
            self.analysis_notes(ui, &t);
            self.analysis_defense(ui, &t, &fs);
            self.analysis_offense(ui, &t, &fs);
        }
        self.analysis_speed(ui, &t, &fs);
        self.analysis_colosseum(ui, &t);
    }

    fn analysis_summary(&mut self, ui: &mut egui::Ui, t: &Team, fs: &[Option<Facts>]) {
        let types = calc::types_in(t.world);
        // worst defensive type
        let mut worst: Option<(&str, usize, usize)> = None;
        let mut holes = 0;
        for at in types {
            let (wk, rs) = weak_resist(fs, at, t.world);
            if wk > rs {
                holes += 1;
            }
            if worst.is_none_or(|w| (wk as i32 - rs as i32) > (w.1 as i32 - w.2 as i32)) {
                worst = Some((at, wk, rs));
            }
        }
        // offensive coverage
        let covered = types.iter().filter(|d| fs.iter().flatten().any(|f| f.attacks.iter().any(|a| calc::single(&a.1, d, t.world) > 1.0))).count();
        let fastest = fs.iter().flatten().max_by_key(|f| f.stats[5]);
        let tiles: Vec<(&str, Color32, String, String)> = vec![
            (
                "Biggest threat",
                kit::C_RED,
                worst.map(|w| w.0.to_string()).unwrap_or_default(),
                worst.map(|w| format!("{} weak · {} resist", w.1, w.2)).unwrap_or_default(),
            ),
            ("Weak spots", kit::C_ORANGE, holes.to_string(), "types more of you are weak to than resist".into()),
            ("Super-effective on", ACCENT, format!("{covered} / {}", types.len()), "types your attacks hit hard".into()),
            ("Fastest", kit::C_CYAN, fastest.map(|f| f.stats[5].to_string()).unwrap_or_default(), fastest.map(|f| format!("{} (Speed)", f.name)).unwrap_or_default()),
        ];
        let gap = 14.0;
        let avail = ui.available_width();
        let cols = if avail > 900.0 { 4 } else { 2 };
        let w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in tiles.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (label, col, value, note) in row {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(w, 92.0), Sense::hover());
                    let p = ui.painter();
                    p.add(kit::card_shadow().as_shape(r, 14.0));
                    p.rect_filled(r, 14.0, CARD());
                    p.rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
                    p.rect_filled(Rect::from_min_size(r.min + egui::vec2(16.0, 18.0), egui::vec2(4.0, 56.0)), 2.0, *col);
                    p.text(r.min + egui::vec2(30.0, 24.0), Align2::LEFT_CENTER, label.to_uppercase(), crate::fonts::sans_bold_font(10.5), MUTED());
                    let is_type = calc::TYPES.contains(&value.as_str());
                    if is_type {
                        super::team_home::type_tag(p, r.min + egui::vec2(30.0, 50.0), value, 13.0);
                    } else {
                        p.text(r.min + egui::vec2(30.0, 50.0), Align2::LEFT_CENTER, value, crate::fonts::sans_bold_font(22.0), TEXT());
                    }
                    let f = crate::fonts::sans_font(11.5);
                    p.text(r.min + egui::vec2(30.0, 74.0), Align2::LEFT_CENTER, super::charts::clip(ui, note, &f, w - 44.0), f, MUTED());
                }
            });
            ui.add_space(gap);
        }
    }

    /// Rows: attacking types. Columns: the team.
    fn analysis_defense(&mut self, ui: &mut egui::Ui, t: &Team, fs: &[Option<Facts>]) {
        kit::card(ui, |ui| {
            ui.label(RichText::new("Taking hits").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(small("How hard each type hits each of them, abilities included (Levitate, Thick Fat…)."));
            ui.add_space(10.0);
            let full = ui.available_width();
            let type_w = 92.0;
            let sum_w = 44.0;
            let n = fs.len().max(1);
            let cw = ((full - type_w - sum_w * 2.0 - 8.0) / n as f32).clamp(30.0, 70.0);
            // header: sprites
            let (hr, _) = ui.allocate_exact_size(egui::vec2(full, 44.0), Sense::hover());
            let p = ui.painter().clone();
            for (i, f) in fs.iter().enumerate() {
                let c = egui::pos2(hr.min.x + type_w + cw * (i as f32 + 0.5), hr.center().y);
                if let Some(f) = f {
                    if !self.team_sprite(&p, &f.species, f.shiny, c, egui::vec2(40.0, 40.0)) {
                        p.text(c, Align2::CENTER_CENTER, f.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(11.0), MUTED());
                    }
                }
            }
            let sx = hr.min.x + type_w + cw * n as f32 + 8.0;
            p.text(egui::pos2(sx + sum_w / 2.0, hr.center().y), Align2::CENTER_CENTER, "WEAK", crate::fonts::sans_bold_font(9.5), MUTED());
            p.text(egui::pos2(sx + sum_w * 1.5, hr.center().y), Align2::CENTER_CENTER, "RESIST", crate::fonts::sans_bold_font(9.5), MUTED());
            for at in calc::types_in(t.world) {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 26.0), Sense::hover());
                let (wk, rs) = weak_resist(fs, at, t.world);
                let bad = wk >= 2 && wk > rs;
                if bad {
                    p.rect_filled(r, 6.0, theme::blend(CARD(), kit::C_RED, if wk >= 3 && rs == 0 { 0.14 } else { 0.07 }));
                } else if resp.hovered() {
                    p.rect_filled(r, 6.0, kit::sunken());
                }
                super::team_home::type_tag(&p, egui::pos2(r.min.x + 4.0, r.center().y), at, 9.5);
                for (i, f) in fs.iter().enumerate() {
                    let Some(f) = f else { continue };
                    let raw = calc::eff(at, &f.types, t.world);
                    let m = calc::ability_eff(at, &f.ability, raw);
                    let cell = Rect::from_center_size(egui::pos2(r.min.x + type_w + cw * (i as f32 + 0.5), r.center().y), egui::vec2((cw - 6.0).min(46.0), 20.0));
                    if let Some(c) = mult_color(m) {
                        p.rect_filled(cell, 5.0, super::charts::faint(c, if m >= 4.0 || m == 0.0 || m <= 0.25 { 0.42 } else { 0.24 }));
                        p.text(cell.center(), Align2::CENTER_CENTER, mult_label(m), crate::fonts::sans_bold_font(11.5), super::charts::ink(c));
                    }
                    if m != raw {
                        // the ability changed it
                        p.circle_filled(egui::pos2(cell.max.x - 3.0, cell.min.y + 3.0), 2.5, ACCENT);
                    }
                }
                let wc = if wk >= 3 { kit::C_RED } else if wk > 0 { kit::C_ORANGE } else { MUTED() };
                p.text(egui::pos2(sx + sum_w / 2.0, r.center().y), Align2::CENTER_CENTER, wk.to_string(), crate::fonts::sans_bold_font(12.5), if wk == 0 { theme::blend(MUTED(), CARD(), 0.4) } else { super::charts::ink(wc) });
                p.text(egui::pos2(sx + sum_w * 1.5, r.center().y), Align2::CENTER_CENTER, rs.to_string(), crate::fonts::sans_bold_font(12.5), if rs == 0 { theme::blend(MUTED(), CARD(), 0.4) } else { super::charts::ink(kit::C_GREEN) });
            }
            ui.add_space(6.0);
            ui.label(small("A dot means an ability changed it."));
        });
    }

    /// Which types the team's attacks hit super effectively.
    fn analysis_offense(&mut self, ui: &mut egui::Ui, t: &Team, fs: &[Option<Facts>]) {
        kit::card(ui, |ui| {
            ui.label(RichText::new("Dealing hits").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(small("The best any attack on the team does to each type. Hover one to see who."));
            ui.add_space(10.0);
            let types = calc::types_in(t.world);
            let gap = 6.0;
            let full = ui.available_width();
            let cols = ((full + gap) / (96.0 + gap)).floor().max(3.0) as usize;
            let w = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in types.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for d in row {
                        let mut best = 0.0f32;
                        let mut who: Vec<String> = Vec::new();
                        for f in fs.iter().flatten() {
                            for a in &f.attacks {
                                let m = calc::single(&a.1, d, t.world);
                                if m > best {
                                    best = m;
                                    who.clear();
                                }
                                if m == best && m > 0.0 {
                                    let s = format!("{} ({})", f.name, a.0);
                                    if !who.contains(&s) {
                                        who.push(s);
                                    }
                                }
                            }
                        }
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 50.0), Sense::hover());
                        let p = ui.painter();
                        let (fill, label) = if best > 1.0 {
                            (theme::blend(CARD(), kit::C_GREEN, 0.20), "2× hit")
                        } else if best == 1.0 {
                            (kit::sunken(), "neutral")
                        } else if best > 0.0 {
                            (theme::blend(CARD(), kit::C_ORANGE, 0.18), "resisted")
                        } else {
                            (theme::blend(CARD(), kit::C_RED, 0.18), "no hit")
                        };
                        p.rect_filled(r, 10.0, fill);
                        p.rect_filled(Rect::from_min_size(r.min + egui::vec2(10.0, 10.0), egui::vec2(10.0, 10.0)), 3.0, type_color(d));
                        p.text(r.min + egui::vec2(26.0, 15.0), Align2::LEFT_CENTER, *d, crate::fonts::sans_bold_font(12.0), TEXT());
                        p.text(r.min + egui::vec2(10.0, 35.0), Align2::LEFT_CENTER, label, crate::fonts::sans_font(11.0), MUTED());
                        if resp.hovered() {
                            let txt = if who.is_empty() { format!("Nothing on the team hits {d}.") } else { format!("Best against {d}:\n{}", who.join("\n")) };
                            resp.on_hover_text(txt);
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
    }

    fn analysis_colosseum(&mut self, ui: &mut egui::Ui, t: &Team) {
        let mut read = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Battle Colosseum").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small("Which rule sets this team can battle in (everyone's Lv 50 there)."));
            });
            ui.add_space(8.0);
            let full = ui.available_width();
            let gap = 10.0;
            let w = ((full - gap * 2.0) / 3.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (k, name) in RULESETS.iter().enumerate() {
                    let bad = colosseum(t, k);
                    ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(w);
                        let col = if bad.is_empty() { kit::C_GREEN } else { kit::C_RED };
                        egui::Frame::none().fill(theme::blend(CARD(), col, 0.10)).rounding(10.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                            ui.set_width(w - 24.0);
                            ui.label(RichText::new(*name).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                            ui.label(RichText::new(if bad.is_empty() { "✔ Can battle".to_string() } else { format!("{} problem{}", bad.len(), if bad.len() == 1 { "" } else { "s" }) }).size(12.5).color(super::charts::ink(col)));
                            for b in bad.iter().take(4) {
                                ui.add(egui::Label::new(RichText::new(format!("· {b}")).size(11.5).color(MUTED())).wrap());
                            }
                        });
                    });
                }
            });
            ui.add_space(6.0);
            read = ui.link(RichText::new("The Colosseum's rules ›").size(12.0)).clicked();
        });
        if read {
            self.info_open_page(super::info_home::Page::Guide("colosseum-rules".into()));
        }
    }

    fn analysis_notes(&mut self, ui: &mut egui::Ui, t: &Team) {
        let notes = notes(self.wiki.as_deref(), t);
        let mut jump: Option<usize> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Things to look at").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                let bad = notes.iter().filter(|n| n.bad).count();
                if bad > 0 {
                    ui.label(RichText::new(format!("{bad} to fix")).size(12.0).color(kit::C_RED));
                }
            });
            ui.add_space(8.0);
            if notes.is_empty() {
                ui.label(RichText::new("✔ Nothing stands out. Nice team.").size(13.0).color(kit::C_GREEN));
            }
            for n in notes.iter().take(14) {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, if n.bad { kit::C_RED } else { kit::C_GOLD });
                    ui.add(egui::Label::new(RichText::new(&n.text).size(12.5).color(TEXT())).wrap());
                    if let Some(s) = n.slot {
                        if ui.link(RichText::new("Fix").size(12.0)).clicked() {
                            jump = Some(s);
                        }
                    }
                });
            }
            if notes.len() > 14 {
                ui.label(small(format!("and {} more", notes.len() - 14)));
            }
        });
        if let Some(s) = jump {
            self.setup.team.slot = s;
            self.setup.team.view = super::team_home::View::Build;
            self.setup.team.picker = None;
            self.setup.scroll_top = true;
        }
    }

    /// Everyone's Speed, against well-known fast Pokémon at the same level.
    fn analysis_speed(&mut self, ui: &mut egui::Ui, t: &Team, fs: &[Option<Facts>]) {
        let level = t.members.iter().map(|m| m.level).max().unwrap_or(50);
        let d = battle::data();
        let jolly = d.nature("Jolly");
        let mut rows: Vec<(String, u16, bool, Option<String>, bool)> = Vec::new();
        for f in fs.iter().flatten() {
            let scarf = f.item == "Choice Scarf";
            let s = if scarf { f.stats[5] * 3 / 2 } else { f.stats[5] };
            rows.push((f.name.clone(), s, true, Some(f.species.clone()), scarf));
        }
        for (n, b) in BENCH {
            let s = battle::stat(5, b, 31, 252, level, jolly, false);
            rows.push((format!("{n} (max)"), s, false, None, false));
        }
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        let top = rows.first().map(|r| r.1).unwrap_or(1).max(1) as f32;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Speed").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small(format!("Your team against fast Pokémon with full Speed EVs and a +Speed nature, all at Lv {level}.")));
            });
            ui.add_space(8.0);
            let full = ui.available_width();
            let name_w = 170.0;
            for (name, s, mine, sp, scarf) in &rows {
                let (r, _) = ui.allocate_exact_size(egui::vec2(full, if *mine { 30.0 } else { 22.0 }), Sense::hover());
                let p = ui.painter().clone();
                let bar = Rect::from_min_size(egui::pos2(r.min.x + name_w, r.center().y - if *mine { 7.0 } else { 4.0 }), egui::vec2((full - name_w - 60.0) * (*s as f32 / top), if *mine { 14.0 } else { 8.0 }));
                if *mine {
                    if let Some(sp) = sp {
                        self.team_sprite(&p, sp, false, egui::pos2(r.min.x + 14.0, r.center().y), egui::vec2(28.0, 28.0));
                    }
                    p.text(egui::pos2(r.min.x + 34.0, r.center().y), Align2::LEFT_CENTER, format!("{name}{}", if *scarf { "  (Scarf)" } else { "" }), crate::fonts::sans_bold_font(12.5), TEXT());
                    p.rect_filled(bar, 5.0, ACCENT);
                } else {
                    p.text(egui::pos2(r.min.x + 34.0, r.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_font(11.5), MUTED());
                    p.rect_filled(bar, 3.0, theme::blend(kit::sunken(), MUTED(), 0.35));
                }
                p.text(egui::pos2(bar.max.x + 8.0, r.center().y), Align2::LEFT_CENTER, s.to_string(), if *mine { crate::fonts::sans_bold_font(12.5) } else { crate::fonts::sans_font(11.5) }, if *mine { TEXT() } else { MUTED() });
            }
        });
        let _ = caps;
    }
}
