//! Damage Calculator (its own page in the sidebar): Platinum's own damage
//! maths (the same steps as Pokémon Showdown's Gen 4 calculator), any
//! Pokémon against any other, or one of your teams against anything.

use super::pokedex::type_color;
use super::team_build::{mkey, nature_label, pickable};
use super::team_home::{caps, chip, small, soft, x_button, ACCENT};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World, STATS};
use crate::calc::{self, Field, Side, Status, Weather};
use crate::teams::Member;
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// One side of the calculator.
#[derive(Clone)]
pub(super) struct Poke {
    pub set: Member,
    pub boosts: [i8; 6],
    /// Current HP as a share of max (0–1).
    pub hp: f32,
    pub status: Status,
    pub ability_on: bool,
    /// The team slot it came from.
    pub from: Option<usize>,
    pub crit: bool,
}

impl Poke {
    pub(super) fn new(set: Member, from: Option<usize>) -> Self {
        Poke { set, boosts: [0; 6], hp: 1.0, status: Status::None, ability_on: false, from, crit: false }
    }
}

#[derive(Default)]
pub(super) struct CalcState {
    pub team: Option<u64>,
    pub left: Option<Poke>,
    pub right: Option<Poke>,
    pub weather: Weather,
    pub doubles: bool,
    pub gravity: bool,
    pub left_side: Side,
    pub right_side: Side,
    /// (0 = yours, 1 = theirs, move).
    pub pick: (usize, usize),
    /// The team on your side (None = no team), once picked.
    pub own: Option<u64>,
    pub own_set: bool,
    /// The world when there's no team.
    pub world: World,
}

/// A set to face when there's nothing else: a strong, common Pokémon.
fn default_foe(world: World) -> Member {
    default_set("Garchomp", world)
}

/// A well-known set for a Pokémon.
fn default_set(name: &str, world: World) -> Member {
    let d = battle::data();
    let s = d.sets_for(name).into_iter().next();
    let mut m = Member::new(name, world);
    if let Some(s) = s {
        m.item = s.item.clone();
        m.ability = s.ab.clone();
        m.nature = s.nat.clone();
        m.evs = s.ev;
        m.ivs = s.iv;
        m.moves = s.m.clone();
    }
    m.level = 50;
    m
}

/// The calculator's view of a move.
pub(super) fn calc_move(w: Option<&Wiki>, name: &str, m: &Member, crit: bool) -> Option<calc::Move> {
    let d = battle::data();
    let fx = d.move_fx(name);
    let info = w.and_then(|w| mkey(w, name).and_then(|k| w.move_info(&k)));
    let (mut t, cat, mut bp) = match info {
        Some(i) => (i.t.clone(), i.c.clone(), i.p.parse::<u32>().unwrap_or(0)),
        None => return None,
    };
    if name.starts_with("Hidden Power") {
        let (ty, p) = battle::hidden_power(m.ivs);
        t = ty.to_string();
        bp = p as u32;
    }
    let skill_link = m.ability == "Skill Link";
    let hits = match fx.map(|f| f.hits.as_str()).unwrap_or("") {
        "2-5" => {
            if skill_link {
                5
            } else {
                3
            }
        }
        "" => 1,
        x => x.parse().unwrap_or(1),
    };
    Some(calc::Move {
        name: fx.map(|f| f.n.clone()).unwrap_or_else(|| name.to_string()),
        t,
        cat,
        bp: if bp == 1 { 0 } else { bp },
        hits,
        punch: fx.is_some_and(|f| f.has("punch")),
        sound: fx.is_some_and(|f| f.has("sound")),
        recoil: fx.is_some_and(|f| !f.recoil.is_empty()),
        crash: fx.is_some_and(|f| f.crash > 0),
        spread: fx.is_some_and(|f| f.spread()),
        crit,
    })
}

pub(super) fn calc_mon(p: &Poke, world: World) -> Option<calc::Mon> {
    let d = battle::data();
    let sp = d.species(&p.set.species)?;
    let stats = p.set.final_stats(world)?;
    Some(calc::Mon {
        name: sp.name.clone(),
        types: sp.types_in(world).to_vec(),
        level: p.set.level,
        stats,
        hp: ((stats[0] as f32 * p.hp).round() as u16).clamp(if p.hp > 0.0 { 1 } else { 0 }, stats[0]),
        boosts: p.boosts,
        ability: p.set.ability.clone(),
        ability_on: p.ability_on,
        item: p.set.item.clone(),
        status: p.status,
        gender: p.set.gender,
        weight: sp.weight,
    })
}

/// "252+ Atk" / "0 SpD".
fn ev_text(m: &Member, i: usize) -> String {
    let n = battle::data().nature(&m.nature);
    let sign = match n {
        Some(n) if !n.neutral() && n.plus == i => "+",
        Some(n) if !n.neutral() && n.minus == i => "-",
        _ => "",
    };
    format!("{}{sign} {}", m.evs[i], STATS[i])
}

/// Showdown-style one-line description.
fn describe(a: &Poke, d: &Poke, mv: &calc::Move, h: &calc::Hit, dm: &calc::Mon, f: &Field, ko: &str) -> String {
    let phys = mv.cat == "Physical";
    let ai = if phys { 1 } else { 3 };
    let di = if phys { 2 } else { 4 };
    let boost = |b: i8| if b > 0 { format!("+{b} ") } else if b < 0 { format!("{b} ") } else { String::new() };
    let mut s = format!("{}{} ", boost(a.boosts[ai]), ev_text(&a.set, ai));
    if !a.set.item.is_empty() {
        s.push_str(&format!("{} ", a.set.item));
    }
    if a.status == Status::Burn && phys {
        s.push_str("burned ");
    }
    s.push_str(&format!("{} {}", a.set.species, mv.name));
    if mv.hits > 1 {
        s.push_str(&format!(" ({} hits)", mv.hits));
    }
    s.push_str(&format!(" vs. {}{} / {} ", boost(d.boosts[di]), ev_text(&d.set, 0), ev_text(&d.set, di)));
    if !d.set.item.is_empty() && (calc::berry_resist_type(&d.set.item).is_some() || d.set.item == "Soul Dew") {
        s.push_str(&format!("{} ", d.set.item));
    }
    s.push_str(&d.set.species);
    match f.weather {
        Weather::Sun => s.push_str(" in Sun"),
        Weather::Rain => s.push_str(" in Rain"),
        Weather::Sand => s.push_str(" in Sand"),
        Weather::Hail => s.push_str(" in Hail"),
        Weather::None => {}
    }
    if (phys && f.def.reflect) || (!phys && f.def.light_screen) {
        s.push_str(if phys { " through Reflect" } else { " through Light Screen" });
    }
    if mv.crit {
        s.push_str(" on a critical hit");
    }
    if let Some(why) = &h.none {
        return format!("{s}: 0-0 (0 - 0%) -- {why}");
    }
    let max = dm.stats[0].max(1) as f32;
    let (lo, hi) = (h.min(), h.max());
    format!("{s}: {lo}-{hi} ({:.1} - {:.1}%){}", lo as f32 * 100.0 / max, hi as f32 * 100.0 / max, if ko.is_empty() { String::new() } else { format!(" -- {ko}") })
}

impl App {
    /// The Damage Calculator page: pick a team for your side (or none),
    /// then the calculator.
    pub(super) fn page_damage_calc(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "DAMAGE CALCULATOR", "Platinum's own damage maths: any Pokémon against any other, or one of your teams against anything.");
        let teams = self.team_choices();
        {
            let c = &mut self.setup.team.calc;
            if !c.own_set {
                c.own = teams.first().map(|t| t.0);
                c.own_set = true;
            }
            if c.own.is_some_and(|id| !teams.iter().any(|t| t.0 == id)) {
                c.own = None;
            }
        }
        let own = self.setup.team.calc.own;
        let mut pick_team: Option<Option<u64>> = None;
        let mut world = self.setup.team.calc.world;
        kit::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("YOUR SIDE"));
                ui.add_space(4.0);
                for (id, name, n) in teams.iter().take(10) {
                    let label = format!("{}  {n}", if name.trim().is_empty() { "Untitled team" } else { name.trim() });
                    if chip(ui, &label, own == Some(*id), ACCENT).on_hover_text("Pick from this team, and see the whole team against the foe").clicked() {
                        pick_team = Some(Some(*id));
                    }
                }
                if chip(ui, "No team", own.is_none(), kit::C_SLATE).on_hover_text("Any Pokémon against any other").clicked() {
                    pick_team = Some(None);
                }
                if own.is_none() {
                    ui.add_space(10.0);
                    kit::segmented(ui, &mut world, &[(World::Platinum, "Platinum"), (World::Modern, "MODERN")]);
                }
            });
        });
        if let Some(p) = pick_team {
            let c = &mut self.setup.team.calc;
            c.own = p;
            c.team = None;
        }
        if world != self.setup.team.calc.world {
            let c = &mut self.setup.team.calc;
            c.world = world;
            c.team = None;
        }
        let t = match self.setup.team.calc.own {
            Some(id) => self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned(),
            None => None,
        }
        .unwrap_or_else(|| crate::teams::Team { world: self.setup.team.calc.world, ..Default::default() });
        self.team_calc(ui, t.world, t);
    }

    pub(super) fn team_calc(&mut self, ui: &mut egui::Ui, world: World, t: crate::teams::Team) {
        let wiki = self.wiki.clone();
        // start: your selected member against a strong foe
        {
            let c = &mut self.setup.team.calc;
            if c.team != Some(t.id) {
                c.team = Some(t.id);
                c.left = None;
                c.right = None;
            }
            if c.left.is_none() {
                let slot = if self.setup.team.open == Some(t.id) { self.setup.team.slot.min(t.members.len().saturating_sub(1)) } else { 0 };
                c.left = Some(match t.members.get(slot) {
                    Some(m) => Poke::new(m.clone(), Some(slot)),
                    None => Poke::new(default_set("Infernape", world), None),
                });
            }
            if c.right.is_none() {
                c.right = Some(Poke::new(default_foe(world), None));
            }
            // keep a team member in step with the team
            for p in [c.left.as_mut(), c.right.as_mut()].into_iter().flatten() {
                if let Some(i) = p.from {
                    match t.members.get(i) {
                        Some(m) if m.species == p.set.species => p.set = m.clone(),
                        _ => p.from = None,
                    }
                }
            }
        }
        let c = &self.setup.team.calc;
        let (Some(l), Some(r)) = (c.left.clone(), c.right.clone()) else { return };
        let field_lr = Field { weather: c.weather, doubles: c.doubles, gravity: c.gravity, atk: c.left_side, def: c.right_side };
        let field_rl = Field { weather: c.weather, doubles: c.doubles, gravity: c.gravity, atk: c.right_side, def: c.left_side };
        let (Some(lm), Some(rm)) = (calc_mon(&l, world), calc_mon(&r, world)) else {
            ui.label(small("Pick two Pokémon."));
            return;
        };
        // every move both ways
        let mut rows: [Vec<(String, Option<calc::Move>, calc::Hit, String, String)>; 2] = [Vec::new(), Vec::new()];
        for (k, (a, am, d, dm, f)) in [(&l, &lm, &r, &rm, &field_lr), (&r, &rm, &l, &lm, &field_rl)].into_iter().enumerate() {
            for name in a.set.moves.iter().take(4) {
                let mv = calc_move(wiki.as_deref(), name, &a.set, a.crit);
                let (h, ko, desc) = match &mv {
                    Some(mv) => {
                        let h = calc::damage(am, dm, mv, f, world);
                        let ko = calc::ko_desc(&h, dm, f, world);
                        let desc = describe(a, d, mv, &h, dm, f, &ko);
                        (h, ko, desc)
                    }
                    None => (calc::Hit::default(), String::new(), String::new()),
                };
                rows[k].push((name.clone(), mv, h, ko, desc));
            }
        }
        self.calc_results(ui, &l, &r, &lm, &rm, &rows, world, &field_lr);
        if !t.members.is_empty() {
            self.calc_vs_team(ui, &t, &r, &rm, world);
        }
        ui.add_space(4.0);
        // the two Pokémon and the field
        let avail = ui.available_width();
        let gap = 16.0;
        if avail >= 1100.0 {
            let fw = 250.0;
            let pw = ((avail - fw - gap * 2.0) / 2.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(pw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(pw);
                    self.calc_side(ui, 0, world, &t);
                });
                ui.allocate_ui_with_layout(egui::vec2(fw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(fw);
                    self.calc_field(ui);
                });
                ui.allocate_ui_with_layout(egui::vec2(pw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(pw);
                    self.calc_side(ui, 1, world, &t);
                });
            });
        } else {
            self.calc_side(ui, 0, world, &t);
            self.calc_field(ui);
            self.calc_side(ui, 1, world, &t);
        }
        ui.label(small("Platinum's damage maths, step for step as Pokémon Showdown's Gen 4 calculator does it. Changes here never change your teams."));
    }

    #[allow(clippy::too_many_arguments)]
    fn calc_results(&mut self, ui: &mut egui::Ui, l: &Poke, r: &Poke, lm: &calc::Mon, rm: &calc::Mon, rows: &[Vec<(String, Option<calc::Move>, calc::Hit, String, String)>; 2], world: World, f: &Field) {
        let mut pick: Option<(usize, usize)> = None;
        let mut swap = false;
        let mut copy: Option<String> = None;
        let sel = self.setup.team.calc.pick;
        kit::card(ui, |ui| {
            let full = ui.available_width();
            let gap = 20.0;
            let two = full >= 820.0;
            let cw = if two { ((full - gap) / 2.0).floor() } else { full };
            let col = |app: &mut App, ui: &mut egui::Ui, k: usize, a: &Poke, am: &calc::Mon, dm: &calc::Mon, pick: &mut Option<(usize, usize)>| {
                // head: sprite, "Tyranitar's moves", speed
                let (hr, _) = ui.allocate_exact_size(egui::vec2(cw, 40.0), Sense::hover());
                let p = ui.painter().clone();
                let c = egui::pos2(hr.min.x + 18.0, hr.center().y);
                app.team_sprite(&p, &a.set.species, a.set.shiny, c, egui::vec2(38.0, 38.0));
                p.text(egui::pos2(hr.min.x + 44.0, hr.center().y - 8.0), Align2::LEFT_CENTER, format!("{}'s moves", a.set.display_name()), crate::fonts::sans_bold_font(14.0), TEXT());
                let fs = Field { weather: f.weather, ..Default::default() };
                let (sa, sd) = (calc::final_speed(am, &fs), calc::final_speed(dm, &fs));
                let (txt, col) = if sa > sd { (format!("Faster: {sa} vs {sd} Speed"), kit::C_GREEN) } else if sa < sd { (format!("Slower: {sa} vs {sd} Speed"), kit::C_ORANGE) } else { (format!("Speed tie at {sa}"), kit::C_GOLD) };
                p.text(egui::pos2(hr.min.x + 44.0, hr.center().y + 10.0), Align2::LEFT_CENTER, txt, crate::fonts::sans_font(11.5), super::charts::ink(col));
                if rows[k].is_empty() {
                    ui.label(small("No moves yet: add some below."));
                }
                for (i, (name, mv, h, ko, _)) in rows[k].iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 46.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let on = sel == (k, i);
                    let p = ui.painter().clone();
                    p.rect_filled(r, 10.0, if on { theme::blend(CARD(), ACCENT, 0.14) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                    if on {
                        p.rect_stroke(r, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), ACCENT, 0.6)));
                    }
                    let tcol = mv.as_ref().map(|m| type_color(&m.t)).unwrap_or(kit::C_SLATE);
                    p.rect_filled(Rect::from_min_size(r.min + egui::vec2(0.0, 8.0), egui::vec2(4.0, 30.0)), 2.0, tcol);
                    let f1 = crate::fonts::sans_bold_font(13.0);
                    p.text(egui::pos2(r.min.x + 14.0, r.min.y + 15.0), Align2::LEFT_CENTER, super::charts::clip(ui, name, &f1, 150.0), f1, TEXT());
                    let max = dm.stats[0].max(1) as f32;
                    let (lo, hi) = (h.min() as f32 * 100.0 / max, h.max() as f32 * 100.0 / max);
                    let label = match (&h.none, mv) {
                        (_, None) => "unknown move".to_string(),
                        (Some(w), _) => w.clone(),
                        _ => format!("{lo:.1} – {hi:.1}%"),
                    };
                    p.text(egui::pos2(r.min.x + 14.0, r.min.y + 33.0), Align2::LEFT_CENTER, &label, crate::fonts::sans_font(11.5), MUTED());
                    // the bar: their HP, how much this takes
                    let bx = r.min.x + 170.0;
                    let bw = (r.max.x - bx - 12.0).max(40.0);
                    let bar = Rect::from_min_size(egui::pos2(bx, r.min.y + 10.0), egui::vec2(bw, 10.0));
                    let cur = dm.hp as f32 / max;
                    p.rect_filled(bar, 5.0, theme::blend(kit::sunken(), CARD(), 0.2));
                    p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bw * cur, 10.0)), 5.0, theme::blend(kit::sunken(), kit::C_GREEN, 0.28));
                    if h.none.is_none() && mv.is_some() {
                        let x_hi = bar.min.x + bw * (cur - hi / 100.0).max(0.0);
                        let x_lo = bar.min.x + bw * (cur - lo / 100.0).max(0.0);
                        let x_cur = bar.min.x + bw * cur;
                        let kc = if ko.contains("OHKO") { kit::C_RED } else if ko.contains("2HKO") { kit::C_ORANGE } else { kit::C_GOLD };
                        p.rect_filled(Rect::from_min_max(egui::pos2(x_lo, bar.min.y), egui::pos2(x_cur, bar.max.y)), 5.0, kc);
                        p.rect_filled(Rect::from_min_max(egui::pos2(x_hi, bar.min.y), egui::pos2(x_lo.max(x_hi + 1.0), bar.max.y)), 2.0, super::charts::faint(kc, 0.55));
                    }
                    let kf = crate::fonts::sans_bold_font(11.5);
                    let kc = if ko.contains("OHKO") { kit::C_RED } else if ko.contains("2HKO") { kit::C_ORANGE } else { MUTED() };
                    p.text(egui::pos2(bx, r.min.y + 33.0), Align2::LEFT_CENTER, super::charts::clip(ui, ko, &kf, bw), kf, super::charts::ink(kc));
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        *pick = Some((k, i));
                    }
                    ui.add_space(4.0);
                }
            };
            if two {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        col(self, ui, 0, l, lm, rm, &mut pick);
                    });
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        col(self, ui, 1, r, rm, lm, &mut pick);
                    });
                });
            } else {
                col(self, ui, 0, l, lm, rm, &mut pick);
                ui.add_space(10.0);
                col(self, ui, 1, r, rm, lm, &mut pick);
            }
            ui.add_space(8.0);
            // the chosen one, in words
            let desc = rows.get(sel.0).and_then(|v| v.get(sel.1)).or_else(|| rows[0].first()).map(|x| x.4.clone()).unwrap_or_default();
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 250.0).max(200.0);
                egui::Frame::none().fill(kit::sunken()).rounding(8.0).inner_margin(egui::Margin::symmetric(12.0, 9.0)).show(ui, |ui| {
                    ui.set_width(w);
                    ui.add(egui::Label::new(RichText::new(if desc.is_empty() { "Pick a move above." } else { &desc }).size(12.5).color(TEXT())).wrap());
                });
                if soft(ui, "Copy").on_hover_text("Copy this line").clicked() && !desc.is_empty() {
                    copy = Some(desc.clone());
                }
                swap = soft(ui, "Swap sides").on_hover_text("Attacker and defender trade places").clicked();
            });
            let _ = world;
        });
        if let Some(p) = pick {
            self.setup.team.calc.pick = p;
        }
        if let Some(s) = copy {
            ui.ctx().copy_text(s);
        }
        if swap {
            let c = &mut self.setup.team.calc;
            std::mem::swap(&mut c.left, &mut c.right);
            std::mem::swap(&mut c.left_side, &mut c.right_side);
            c.pick = (1 - c.pick.0, c.pick.1);
        }
    }

    /// The defender against every member of the team, both ways.
    fn calc_vs_team(&mut self, ui: &mut egui::Ui, t: &crate::teams::Team, foe: &Poke, fm: &calc::Mon, world: World) {
        let wiki = self.wiki.clone();
        let c = &self.setup.team.calc;
        let f_out = Field { weather: c.weather, doubles: c.doubles, gravity: c.gravity, atk: c.left_side, def: c.right_side };
        let f_in = Field { weather: c.weather, doubles: c.doubles, gravity: c.gravity, atk: c.right_side, def: c.left_side };
        // best hit from `a` on `d`: (move, min %, max %, ko)
        let best = |a: &Poke, am: &calc::Mon, dm: &calc::Mon, f: &Field| -> Option<(String, f32, f32, String)> {
            let mut out: Option<(String, f32, f32, String)> = None;
            for name in a.set.moves.iter().take(4) {
                let Some(mv) = calc_move(wiki.as_deref(), name, &a.set, false) else { continue };
                let h = calc::damage(am, dm, &mv, f, world);
                if h.none.is_some() {
                    continue;
                }
                let max = dm.stats[0].max(1) as f32;
                let (lo, hi) = (h.min() as f32 * 100.0 / max, h.max() as f32 * 100.0 / max);
                if out.as_ref().is_none_or(|o| hi > o.2) {
                    out = Some((name.clone(), lo, hi, calc::ko_desc(&h, dm, f, world)));
                }
            }
            out
        };
        let mut pick: Option<usize> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Your team against {}", foe.set.species)).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small("Each one's best hit on it, and its best hit back. Click one to calc it."));
            });
            ui.add_space(8.0);
            let full = ui.available_width();
            for (i, m) in t.members.iter().enumerate() {
                let me = Poke::new(m.clone(), Some(i));
                let Some(mm) = calc_mon(&me, world) else { continue };
                let mine = best(&me, &mm, fm, &f_out);
                let theirs = best(foe, fm, &mm, &f_in);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 40.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                if ht > 0.0 {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                let cx = egui::pos2(r.min.x + 20.0, r.center().y);
                if !self.team_sprite(&p, &m.species, m.shiny, cx, egui::vec2(36.0, 36.0)) {
                    p.text(cx, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(11.0), MUTED());
                }
                let f1 = crate::fonts::sans_bold_font(12.5);
                p.text(egui::pos2(r.min.x + 46.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, m.display_name(), &f1, 110.0), f1, TEXT());
                let fs = Field { weather: f_out.weather, ..Default::default() };
                let (sa, sd) = (calc::final_speed(&mm, &fs), calc::final_speed(fm, &fs));
                let (st, sc) = if sa > sd { ("faster", kit::C_GREEN) } else if sa < sd { ("slower", kit::C_ORANGE) } else { ("tie", kit::C_GOLD) };
                p.text(egui::pos2(r.min.x + 166.0, r.center().y), Align2::LEFT_CENTER, st, crate::fonts::sans_bold_font(11.0), super::charts::ink(sc));
                let colw = ((full - 220.0) / 2.0).max(120.0);
                for (k, b) in [(0, &mine), (1, &theirs)] {
                    let x = r.min.x + 220.0 + colw * k as f32;
                    let (txt, col) = match b {
                        Some((n, lo, hi, ko)) => {
                            let c = if ko.contains("OHKO") { kit::C_RED } else if ko.contains("2HKO") { kit::C_ORANGE } else { MUTED() };
                            (format!("{}{n}: {lo:.0}–{hi:.0}%  {ko}", if k == 0 { "→ " } else { "← " }), c)
                        }
                        None => (if k == 0 { "→ nothing that hurts it".into() } else { "← nothing that hurts you".into() }, MUTED()),
                    };
                    let f2 = crate::fonts::sans_font(12.0);
                    p.text(egui::pos2(x, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &txt, &f2, colw - 12.0), f2, if col == MUTED() { MUTED() } else { super::charts::ink(col) });
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(i);
                }
            }
        });
        if let Some(i) = pick {
            if let Some(m) = t.members.get(i) {
                self.setup.team.calc.left = Some(Poke::new(m.clone(), Some(i)));
                self.setup.team.calc.pick = (0, 0);
            }
        }
    }

    fn calc_field(&mut self, ui: &mut egui::Ui) {
        let c = &mut self.setup.team.calc;
        kit::card(ui, |ui| {
            ui.label(RichText::new("Field").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(8.0);
            ui.label(caps("WEATHER"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (w, l) in [(Weather::None, "None"), (Weather::Sun, "Sun"), (Weather::Rain, "Rain"), (Weather::Sand, "Sand"), (Weather::Hail, "Hail")] {
                    if chip(ui, l, c.weather == w, ACCENT).clicked() {
                        c.weather = w;
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(caps("BATTLE"));
            ui.horizontal(|ui| {
                if chip(ui, "Singles", !c.doubles, ACCENT).clicked() {
                    c.doubles = false;
                }
                if chip(ui, "Doubles", c.doubles, ACCENT).clicked() {
                    c.doubles = true;
                }
            });
            kit::checkbox(ui, &mut c.gravity, "Gravity");
            for (k, side) in [(0, &mut c.left_side), (1, &mut c.right_side)] {
                ui.add_space(10.0);
                ui.label(caps(if k == 0 { "YOUR SIDE" } else { "THEIR SIDE" }));
                kit::checkbox(ui, &mut side.reflect, "Reflect");
                kit::checkbox(ui, &mut side.light_screen, "Light Screen");
                kit::checkbox(ui, &mut side.stealth_rock, "Stealth Rock");
                ui.horizontal(|ui| {
                    ui.label(small("Spikes"));
                    for n in 0..=3u8 {
                        if chip(ui, &n.to_string(), side.spikes == n, ACCENT).clicked() {
                            side.spikes = n;
                        }
                    }
                });
                kit::checkbox(ui, &mut side.helping_hand, "Helping Hand");
                kit::checkbox(ui, &mut side.foresight, "Foresight");
                kit::checkbox(ui, &mut side.switching, "Switching (Pursuit)").on_hover_text("The defender is switching out: Pursuit hits twice as hard");
            }
        });
    }

    /// One Pokémon: who, its set, and its state in battle.
    fn calc_side(&mut self, ui: &mut egui::Ui, k: usize, world: World, t: &crate::teams::Team) {
        let Some(orig) = (if k == 0 { self.setup.team.calc.left.clone() } else { self.setup.team.calc.right.clone() }) else { return };
        let mut p = orig.clone();
        let wiki = self.wiki.clone();
        let d = battle::data();
        let mut from_team: Option<usize> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(if k == 0 { "Attacker" } else { "Defender" }).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small(if k == 0 { "usually yours" } else { "usually theirs" }));
            });
            ui.add_space(6.0);
            // team members to pick from
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (i, m) in t.members.iter().enumerate() {
                    let on = p.from == Some(i);
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let pt = ui.painter().clone();
                    pt.rect_filled(r, 10.0, if on { theme::blend(CARD(), ACCENT, 0.25) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                    if on {
                        pt.rect_stroke(r, 10.0, Stroke::new(1.5f32, ACCENT));
                    }
                    if !self.team_sprite(&pt, &m.species, m.shiny, r.center(), egui::vec2(38.0, 38.0)) {
                        pt.text(r.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(11.0), MUTED());
                    }
                    if resp.on_hover_text(format!("Use {} from the team", m.display_name())).clicked() {
                        from_team = Some(i);
                    }
                }
            });
            // Smogon's sets for it
            let sets = d.sets_for(&p.set.species);
            if !sets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.label(caps("SETS"));
                    for s in sets.into_iter().take(5) {
                        let on = p.set.moves == s.m;
                        let short = s.n.split_once(' ').filter(|(t, _)| ["OU", "UU", "Ubers", "NU", "RU", "LC", "UUBL"].contains(t)).map(|(_, r)| r).unwrap_or(&s.n);
                        if chip(ui, short, on, ACCENT).on_hover_text(format!("{}\n{}", s.n, s.m.join(" / "))).clicked() {
                            p.set.item = s.item.clone();
                            p.set.nature = s.nat.clone();
                            p.set.evs = s.ev;
                            p.set.ivs = s.iv;
                            p.set.moves = s.m.clone();
                            p.set.ability = s.ab.clone();
                            p.from = None;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(8.0);
            egui::Grid::new(("calc-side", k)).num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label(caps("POKÉMON"));
                let names: Vec<String> = pickable(world).iter().map(|s| s.name.clone()).collect();
                if let Some(n) = search_pick(ui, egui::Id::new(("calc-sp", k)), &p.set.species, &names, 190.0) {
                    let mut m = Member::new(&n, world);
                    m.level = p.set.level;
                    if let Some(s) = d.sets_for(&n).into_iter().next() {
                        m.item = s.item.clone();
                        m.nature = s.nat.clone();
                        m.evs = s.ev;
                        m.ivs = s.iv;
                        m.moves = s.m.clone();
                        if d.species(&n).is_some_and(|sp| sp.abilities_in(world).contains(&s.ab)) {
                            m.ability = s.ab.clone();
                        }
                    }
                    p = Poke::new(m, None);
                }
                ui.end_row();
                ui.label(caps("LEVEL"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut p.set.level).range(1..=100).speed(0.2));
                    for l in [50u8, 100] {
                        if chip(ui, &l.to_string(), p.set.level == l, ACCENT).clicked() {
                            p.set.level = l;
                        }
                    }
                });
                ui.end_row();
                ui.label(caps("ITEM"));
                let items: Vec<String> = std::iter::once("(none)".to_string()).chain(super::team_build::holdable(wiki.as_deref(), world).iter().map(|i| i.n.clone())).collect();
                let cur = if p.set.item.is_empty() { "(none)".to_string() } else { p.set.item.clone() };
                if let Some(n) = search_pick(ui, egui::Id::new(("calc-item", k)), &cur, &items, 190.0) {
                    p.set.item = if n == "(none)" { String::new() } else { n };
                }
                ui.end_row();
                ui.label(caps("ABILITY"));
                let abs = d.species(&p.set.species).map(|s| s.abilities_in(world)).unwrap_or_default();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for a in &abs {
                        if chip(ui, a, p.set.ability == *a, ACCENT).clicked() {
                            p.set.ability = a.clone();
                        }
                    }
                    if ["Flash Fire", "Plus", "Minus", "Slow Start", "Intimidate"].contains(&p.set.ability.as_str()) {
                        kit::checkbox(ui, &mut p.ability_on, "active");
                    }
                });
                ui.end_row();
                ui.label(caps("NATURE"));
                egui::ComboBox::from_id_salt(("calc-nat", k)).selected_text(nature_label(&p.set.nature)).width(190.0).height(320.0).show_ui(ui, |ui| {
                    for n in &d.natures {
                        ui.selectable_value(&mut p.set.nature, n.name.clone(), nature_label(&n.name));
                    }
                });
                ui.end_row();
                ui.label(caps("STATUS"));
                egui::ComboBox::from_id_salt(("calc-st", k))
                    .selected_text(status_label(p.status))
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        for s in [Status::None, Status::Burn, Status::Paralysis, Status::Poison, Status::Toxic, Status::Sleep, Status::Freeze] {
                            ui.selectable_value(&mut p.status, s, status_label(s));
                        }
                    });
                ui.end_row();
                ui.label(caps("HP"));
                ui.horizontal(|ui| {
                    let mut pct = (p.hp * 100.0).round() as u32;
                    let label = format!("{pct}%");
                    if kit::slider_u32(ui, &mut pct, 0..=100, &label).changed() {
                        p.hp = pct as f32 / 100.0;
                    }
                });
                ui.end_row();
                ui.label(caps("CRIT"));
                kit::checkbox(ui, &mut p.crit, "Every move crits");
                ui.end_row();
            });
            ui.add_space(8.0);
            // stats: EVs, boosts and the result
            let stats = p.set.final_stats(world).unwrap_or_default();
            egui::Grid::new(("calc-stats", k)).num_columns(5).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label(caps(""));
                ui.label(caps("EVS"));
                ui.label(caps("IVS"));
                ui.label(caps("BOOST"));
                ui.label(caps("STAT"));
                ui.end_row();
                for i in 0..6 {
                    ui.label(RichText::new(STATS[i]).font(crate::fonts::sans_bold_font(12.0)).color(TEXT()));
                    ui.add(egui::DragValue::new(&mut p.set.evs[i]).range(0..=252).speed(2.0));
                    ui.add(egui::DragValue::new(&mut p.set.ivs[i]).range(0..=31).speed(0.2));
                    if i > 0 {
                        ui.add(egui::DragValue::new(&mut p.boosts[i]).range(-6..=6).speed(0.1).custom_formatter(|v, _| if v > 0.0 { format!("+{v}") } else { format!("{v}") }));
                    } else {
                        ui.label("");
                    }
                    let shown = if i > 0 { boosted(stats[i] as u32, p.boosts[i]) } else { stats[i] as u32 };
                    let col = if p.boosts[i] > 0 { kit::C_RED } else if p.boosts[i] < 0 { kit::C_BLUE } else { TEXT() };
                    ui.label(RichText::new(shown.to_string()).font(crate::fonts::sans_bold_font(13.0)).color(col));
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            ui.label(caps("MOVES"));
            ui.add_space(4.0);
            let all: Vec<String> = match wiki.as_deref() {
                Some(w) => {
                    let ways = super::team_build::learnable(w, &p.set.species, world);
                    let best = super::team_build::best_ways(&ways);
                    let mut v: Vec<String> = best.iter().filter_map(|l| w.move_info(&l.key)).filter(|i| i.c != "Status").map(|i| i.n.clone()).collect();
                    v.sort();
                    let mut rest: Vec<String> = w.data.moves.values().filter(|i| i.c != "Status" && !v.contains(&i.n)).map(|i| i.n.clone()).collect();
                    rest.sort();
                    v.extend(rest);
                    v
                }
                None => Vec::new(),
            };
            for i in 0..4 {
                let cur = p.set.moves.get(i).cloned().unwrap_or_else(|| "(empty)".into());
                ui.horizontal(|ui| {
                    if let Some(n) = search_pick(ui, egui::Id::new(("calc-mv", k, i)), &cur, &all, (ui.available_width() - 76.0).max(120.0)) {
                        if i < p.set.moves.len() {
                            p.set.moves[i] = n;
                        } else {
                            p.set.moves.push(n);
                        }
                    }
                    if i < p.set.moves.len() && x_button(ui).on_hover_text("Clear this move").clicked() {
                        p.set.moves.remove(i);
                    }
                });
            }
        });
        if let Some(i) = from_team {
            if let Some(m) = t.members.get(i) {
                p = Poke::new(m.clone(), Some(i));
            }
        }
        // editing a team member here stops following the team
        if p.set != orig.set && p.from.is_some() && from_team.is_none() {
            p.from = None;
        }
        let c = &mut self.setup.team.calc;
        if k == 0 {
            c.left = Some(p);
        } else {
            c.right = Some(p);
        }
        let _ = Color32::WHITE;
    }
}

fn boosted(stat: u32, b: i8) -> u32 {
    let b = b as i32;
    if b > 0 {
        stat * (2 + b as u32) / 2
    } else if b < 0 {
        stat * 2 / (2 + (-b) as u32)
    } else {
        stat
    }
}

fn status_label(s: Status) -> &'static str {
    match s {
        Status::None => "Healthy",
        Status::Burn => "Burned",
        Status::Paralysis => "Paralyzed",
        Status::Poison => "Poisoned",
        Status::Toxic => "Badly poisoned",
        Status::Sleep => "Asleep",
        Status::Freeze => "Frozen",
    }
}

/// A box that shows the current choice and opens a searchable list.
pub(super) fn search_pick(ui: &mut egui::Ui, id: egui::Id, current: &str, options: &[String], width: f32) -> Option<String> {
    let mut out = None;
    let qid = id.with("q");
    egui::ComboBox::from_id_salt(id).selected_text(current).width(width).height(300.0).show_ui(ui, |ui| {
        let mut q = ui.ctx().data(|d| d.get_temp::<String>(qid).unwrap_or_default());
        let r = ui.add(egui::TextEdit::singleline(&mut q).hint_text("Search…").desired_width(width - 10.0));
        if !r.has_focus() && q.is_empty() {
            r.request_focus();
        }
        let ql = q.to_lowercase();
        let mut shown = 0;
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        for o in options {
            if !ql.is_empty() && !o.to_lowercase().contains(&ql) {
                continue;
            }
            shown += 1;
            if shown > 120 {
                ui.label(small("Type to narrow it down…"));
                break;
            }
            if ui.selectable_label(o == current, o).clicked() || (enter && shown == 1 && !ql.is_empty()) {
                out = Some(o.clone());
                ui.close_menu();
            }
        }
        if shown == 0 {
            ui.label(small("Nothing matches"));
        }
        ui.ctx().data_mut(|d| d.insert_temp(qid, q));
    });
    if out.is_some() {
        ui.ctx().data_mut(|d| d.remove::<String>(qid));
    }
    out
}
