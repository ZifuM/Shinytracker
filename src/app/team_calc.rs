//! Damage Calculator (its own page in the sidebar): Platinum's own damage
//! maths (the same steps as Pokémon Showdown's Gen 4 calculator), any
//! Pokémon against any other, or one of your teams against anything.

use super::team_build::{mkey, nature_label, pickable};
use super::team_home::{caps, chip, small, type_tag, x_button, ACCENT};
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
    /// The Damage Calculator page: the header row (your team, the rules),
    /// then the three columns.
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
        let team_label = |name: &str, n: usize| format!("{}  ·  {n}", if name.trim().is_empty() { "Untitled team" } else { name.trim() });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
            ui.label(caps("YOUR SIDE"));
            let shown = own.and_then(|id| teams.iter().find(|t| t.0 == id)).map(|(_, name, n)| team_label(name, *n)).unwrap_or_else(|| "No team".to_string());
            kit::Dropdown::from_id_salt("calc-team").selected_text(shown).width(240.0).height(320.0).show_ui(ui, |ui| {
                if ui.selectable_label(own.is_none(), "No team: any Pokémon against any other").clicked() {
                    pick_team = Some(None);
                }
                for (id, name, n) in teams.iter() {
                    if ui.selectable_label(own == Some(*id), team_label(name, *n)).on_hover_text("Pick from this team, and see the whole team against the foe").clicked() {
                        pick_team = Some(Some(*id));
                    }
                }
            });
            if own.is_none() {
                ui.add_space(6.0);
                kit::segmented(ui, &mut world, &[(World::Platinum, "Platinum"), (World::Modern, "Modern")]);
            }
        });
        ui.add_space(12.0);
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
        let mut rows: [Vec<Row>; 2] = [Vec::new(), Vec::new()];
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
        // who moves first (weather only: Swift Swim, Chlorophyll)
        let fs = Field { weather: field_lr.weather, ..Default::default() };
        let (sl, sr) = (calc::final_speed(&lm, &fs), calc::final_speed(&rm, &fs));
        let speed_tag = |role: &str, mine: u32, theirs: u32| -> (String, Color32) {
            if mine > theirs {
                (format!("{role} · faster ({mine} Spe)"), kit::C_GREEN)
            } else if mine < theirs {
                (format!("{role} · slower ({mine} Spe)"), kit::C_ORANGE)
            } else {
                (format!("{role} · speed tie ({mine})"), kit::C_GOLD)
            }
        };
        let tags = [speed_tag("Attacker", sl, sr), speed_tag("Defender", sr, sl)];
        // three equal columns: attacker, the answer and the field, defender
        let avail = ui.available_width();
        let gap = 14.0;
        if avail >= 1000.0 {
            let cw = ((avail - gap * 2.0) / 3.0).floor();
            // the three columns end level: each column's last card is
            // padded to the tallest column's natural height (measured last
            // frame, without the padding)
            let key = egui::Id::new("calc-col-heights");
            let prev: [f32; 3] = ui.data(|d| d.get_temp(key)).unwrap_or([0.0; 3]);
            let tallest = prev.iter().copied().fold(0.0f32, f32::max);
            let pads = prev.map(|h| if h > 0.0 { (tallest - h).max(0.0) } else { 0.0 });
            let mut natural = [0.0f32; 3];
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let r0 = ui
                    .allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        self.calc_side(ui, 0, world, &t, &tags[0], pads[0]);
                    })
                    .response
                    .rect;
                let r1 = ui
                    .allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        self.calc_results(ui, &l, &r, &lm, &rm, &rows);
                        ui.add_space(gap);
                        self.calc_field(ui, pads[1]);
                    })
                    .response
                    .rect;
                let r2 = ui
                    .allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        self.calc_side(ui, 1, world, &t, &tags[1], pads[2]);
                    })
                    .response
                    .rect;
                natural = [r0.height() - pads[0], r1.height() - pads[1], r2.height() - pads[2]];
            });
            if natural.iter().zip(prev.iter()).any(|(a, b)| (a - b).abs() > 0.5) {
                ui.data_mut(|d| d.insert_temp(key, natural));
                ui.ctx().request_repaint();
            }
        } else {
            self.calc_results(ui, &l, &r, &lm, &rm, &rows);
            ui.add_space(gap);
            self.calc_side(ui, 0, world, &t, &tags[0], 0.0);
            ui.add_space(gap);
            self.calc_side(ui, 1, world, &t, &tags[1], 0.0);
            ui.add_space(gap);
            self.calc_field(ui, 0.0);
        }
        if !t.members.is_empty() {
            ui.add_space(gap);
            self.calc_vs_team(ui, &t, &r, &rm, world);
        }
        ui.add_space(10.0);
        ui.label(small("Platinum's damage maths, step for step as Pokémon Showdown's Gen 4 calculator does it. Changes here never change your teams."));
    }

    /// The middle column's top card: which way, each move's damage, the
    /// chosen one in Showdown's words, Copy and Swap.
    fn calc_results(&mut self, ui: &mut egui::Ui, l: &Poke, r: &Poke, lm: &calc::Mon, rm: &calc::Mon, rows: &[Vec<Row>; 2]) {
        let mut pick: Option<(usize, usize)> = None;
        let mut swap = false;
        let mut copy: Option<String> = None;
        let sel = self.setup.team.calc.pick;
        let mut dir = sel.0.min(1);
        kit::card(ui, |ui| {
            let w = ui.available_width();
            let names = [format!("{} hits", short_name(&l.set)), format!("{} hits", short_name(&r.set))];
            ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                if kit::segmented(ui, &mut dir, &[(0usize, names[0].as_str()), (1usize, names[1].as_str())]) {
                    pick = Some((dir, 0));
                }
            });
            ui.add_space(10.0);
            let dm = if dir == 0 { rm } else { lm };
            let _ = if dir == 0 { l } else { r };
            if rows[dir].is_empty() {
                ui.label(small("No moves yet: add some in its column."));
            }
            for (i, (name, mv, h, ko, _)) in rows[dir].iter().enumerate() {
                let (rr, resp) = ui.allocate_exact_size(egui::vec2(w, 64.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let on = sel == (dir, i);
                let p = ui.painter().clone();
                p.rect_filled(rr, 11.0, if on { theme::blend(CARD(), ACCENT, 0.14) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                if on {
                    p.rect_stroke(rr, 11.0, Stroke::new(1.0f32, theme::blend(LINE(), ACCENT, 0.7)));
                }
                // type tag and name, the range on the right
                let x0 = rr.min.x + 12.0;
                let mut x = x0;
                if let Some(m) = mv {
                    x += type_tag(&p, egui::pos2(x, rr.min.y + 17.0), &m.t, 9.5) + 8.0;
                }
                let max = dm.stats[0].max(1) as f32;
                let (lo, hi) = (h.min() as f32 * 100.0 / max, h.max() as f32 * 100.0 / max);
                let range = match (&h.none, mv) {
                    (_, None) => "—".to_string(),
                    (Some(_), _) => "0%".to_string(),
                    _ => format!("{lo:.1} – {hi:.1}%"),
                };
                let rf = crate::fonts::sans_bold_font(13.0);
                let rw = p.layout_no_wrap(range.clone(), rf.clone(), TEXT()).size().x;
                p.text(egui::pos2(rr.max.x - 12.0, rr.min.y + 17.0), Align2::RIGHT_CENTER, &range, rf, TEXT());
                let f1 = crate::fonts::sans_bold_font(13.0);
                p.text(egui::pos2(x, rr.min.y + 17.0), Align2::LEFT_CENTER, super::charts::clip(ui, name, &f1, (rr.max.x - 24.0 - rw - x).max(40.0)), f1, TEXT());
                // the bar: how much of their HP it takes (solid = always,
                // faded = on a high roll)
                let kc = if ko.contains("OHKO") { kit::C_RED } else if ko.contains("2HKO") { kit::C_ORANGE } else if ko.contains("3HKO") || ko.contains("4HKO") { kit::C_GOLD } else { kit::C_SLATE };
                let bar = Rect::from_min_size(egui::pos2(x0, rr.min.y + 31.0), egui::vec2(rr.width() - 24.0, 7.0));
                p.rect_filled(bar, 3.5, theme::blend(kit::sunken(), TEXT(), 0.08));
                if h.none.is_none() && mv.is_some() {
                    let bw = bar.width();
                    let (a, b) = ((lo / 100.0).clamp(0.0, 1.0), (hi / 100.0).clamp(0.0, 1.0));
                    if b > 0.0 {
                        p.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bw * b).max(3.0), 7.0)), 3.5, super::charts::faint(kc, 0.45));
                    }
                    if a > 0.0 {
                        p.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bw * a).max(3.0), 7.0)), 3.5, kc);
                    }
                }
                let what = match (&h.none, mv) {
                    (_, None) => "Unknown move".to_string(),
                    (Some(why), _) => why.clone(),
                    _ if ko.is_empty() => "Not a KO in 4 hits".to_string(),
                    _ => ko.clone(),
                };
                let kf = crate::fonts::sans_bold_font(11.5);
                let kcol = if kc == kit::C_SLATE { MUTED() } else { super::charts::ink(kc) };
                p.text(egui::pos2(x0, rr.min.y + 51.0), Align2::LEFT_CENTER, super::charts::clip(ui, &what, &kf, rr.width() - 24.0), kf, kcol);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some((dir, i));
                }
                ui.add_space(6.0);
            }
            ui.add_space(4.0);
            // the chosen one, in Showdown's words
            let desc = rows.get(sel.0).and_then(|v| v.get(sel.1)).or_else(|| rows[dir].first()).map(|x| x.4.clone()).unwrap_or_default();
            egui::Frame::none().fill(kit::sunken()).rounding(10.0).inner_margin(egui::Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                ui.set_width(w - 24.0);
                ui.add(egui::Label::new(RichText::new(if desc.is_empty() { "Pick a move above." } else { &desc }).font(egui::FontId::monospace(12.0)).color(TEXT())).wrap());
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let bw = ((w - 8.0) / 2.0).floor();
                if kit::accent_button_w(ui, egui::Button::new(RichText::new("Copy").strong().color(kit::on_primary())).min_size(egui::vec2(bw, 34.0)), 10.0, bw).on_hover_text("Copy this line").clicked() && !desc.is_empty() {
                    copy = Some(desc.clone());
                }
                swap = ui
                    .add(egui::Button::new(RichText::new("Swap sides").strong()).fill(kit::btn_fill()).min_size(egui::vec2(bw, 34.0)))
                    .on_hover_text("Attacker and defender trade places")
                    .clicked();
            });
        });
        if let Some(p) = pick {
            self.setup.team.calc.pick = p;
        }
        if let Some(s) = copy {
            ui.ctx().copy_text(s);
            self.setup.toast = Some((std::time::Instant::now(), true, "✔ Copied".to_string()));
        }
        if swap {
            let c = &mut self.setup.team.calc;
            std::mem::swap(&mut c.left, &mut c.right);
            std::mem::swap(&mut c.left_side, &mut c.right_side);
            c.pick = (1 - c.pick.0.min(1), c.pick.1);
        }
    }

    /// The field: weather, the battle and each side's screens, hazards and
    /// helpers, all as toggles.
    fn calc_field(&mut self, ui: &mut egui::Ui, pad: f32) {
        let c = &mut self.setup.team.calc;
        let side_names = [c.left.as_ref(), c.right.as_ref()].map(|p| p.map_or("This".to_string(), |p| short_name(&p.set)).to_uppercase());
        kit::card(ui, |ui| {
            ui.label(RichText::new("Field").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (w, l) in [(Weather::None, "No weather"), (Weather::Sun, "Sun"), (Weather::Rain, "Rain"), (Weather::Sand, "Sand"), (Weather::Hail, "Hail")] {
                    if chip(ui, l, c.weather == w, ACCENT).clicked() {
                        c.weather = w;
                    }
                }
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                if chip(ui, "Singles", !c.doubles, ACCENT).clicked() {
                    c.doubles = false;
                }
                if chip(ui, "Doubles", c.doubles, ACCENT).clicked() {
                    c.doubles = true;
                }
                if chip(ui, "Gravity", c.gravity, ACCENT).clicked() {
                    c.gravity = !c.gravity;
                }
            });
            ui.add_space(10.0);
            let w = ui.available_width();
            let hw = ((w - 12.0) / 2.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (k, side) in [(0, &mut c.left_side), (1, &mut c.right_side)] {
                    ui.allocate_ui_with_layout(egui::vec2(hw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(hw);
                        ui.label(caps(&format!("{}'S SIDE", side_names[k])));
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            if chip(ui, "Reflect", side.reflect, ACCENT).clicked() {
                                side.reflect = !side.reflect;
                            }
                            if chip(ui, "Light Screen", side.light_screen, ACCENT).clicked() {
                                side.light_screen = !side.light_screen;
                            }
                            if chip(ui, "Stealth Rock", side.stealth_rock, ACCENT).clicked() {
                                side.stealth_rock = !side.stealth_rock;
                            }
                            if chip(ui, &format!("Spikes {}", side.spikes), side.spikes > 0, ACCENT).on_hover_text("Click for more layers (0–3)").clicked() {
                                side.spikes = (side.spikes + 1) % 4;
                            }
                            if chip(ui, "Helping Hand", side.helping_hand, ACCENT).clicked() {
                                side.helping_hand = !side.helping_hand;
                            }
                            if chip(ui, "Foresight", side.foresight, ACCENT).clicked() {
                                side.foresight = !side.foresight;
                            }
                            if chip(ui, "Switching", side.switching, ACCENT).on_hover_text("This side is switching out: Pursuit hits twice as hard").clicked() {
                                side.switching = !side.switching;
                            }
                        });
                    });
                }
            });
            ui.add_space(pad);
        });
    }

    /// One Pokémon's column: who, its set, its stats and its moves.
    fn calc_side(&mut self, ui: &mut egui::Ui, k: usize, world: World, t: &crate::teams::Team, tag: &(String, Color32), pad: f32) {
        let Some(orig) = (if k == 0 { self.setup.team.calc.left.clone() } else { self.setup.team.calc.right.clone() }) else { return };
        let mut p = orig.clone();
        let wiki = self.wiki.clone();
        let d = battle::data();
        let mut from_team: Option<usize> = None;
        kit::card(ui, |ui| {
            let w = ui.available_width();
            // who: the sprite, a box to change it, and its speed
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (sr, _) = ui.allocate_exact_size(egui::vec2(56.0, 56.0), Sense::hover());
                let pt = ui.painter().clone();
                pt.circle_filled(sr.center(), 27.0, theme::blend(kit::sunken(), tag.1, 0.10));
                if !self.team_sprite(&pt, &p.set.species, p.set.shiny, sr.center(), egui::vec2(54.0, 54.0)) {
                    pt.text(sr.center(), Align2::CENTER_CENTER, p.set.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(13.0), MUTED());
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let names: Vec<String> = pickable(world).iter().map(|s| s.name.clone()).collect();
                    if let Some(n) = search_pick(ui, egui::Id::new(("calc-sp", k)), &p.set.species, &names, (w - 66.0).min(260.0)) {
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
                    ui.label(RichText::new(&tag.0).font(crate::fonts::sans_bold_font(12.0)).color(super::charts::ink(tag.1)));
                });
            });
            // team members to pick from
            if !t.members.is_empty() {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for (i, m) in t.members.iter().enumerate() {
                        let on = p.from == Some(i);
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let pt = ui.painter().clone();
                        pt.rect_filled(r, 10.0, if on { theme::blend(CARD(), ACCENT, 0.25) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                        if on {
                            pt.rect_stroke(r, 10.0, Stroke::new(1.5f32, ACCENT));
                        }
                        if !self.team_sprite(&pt, &m.species, m.shiny, r.center(), egui::vec2(36.0, 36.0)) {
                            pt.text(r.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(11.0), MUTED());
                        }
                        if resp.on_hover_text(format!("Use {} from the team", m.display_name())).clicked() {
                            from_team = Some(i);
                        }
                    }
                });
            }
            // Smogon's sets for it
            let sets = d.sets_for(&p.set.species);
            if !sets.is_empty() {
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
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
            }
            ui.add_space(12.0);
            // the set, two to a row
            let hw = ((w - 10.0) / 2.0).floor();
            let field = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.allocate_ui_with_layout(egui::vec2(hw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(hw);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(caps(label));
                    add(ui);
                });
            };
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                field(ui, "LEVEL", &mut |ui| {
                    kit::Dropdown::from_id_salt(("calc-lv", k)).selected_text(p.set.level.to_string()).width(hw).height(300.0).show_ui(ui, |ui| {
                        for l in [50u8, 100].into_iter().chain(1..=100u8) {
                            if ui.selectable_label(p.set.level == l, l.to_string()).clicked() {
                                p.set.level = l;
                            }
                        }
                    });
                });
                field(ui, "ITEM", &mut |ui| {
                    let items: Vec<String> = std::iter::once("(none)".to_string()).chain(super::team_build::holdable(wiki.as_deref(), world).iter().map(|i| i.n.clone())).collect();
                    let cur = if p.set.item.is_empty() { "(none)".to_string() } else { p.set.item.clone() };
                    if let Some(n) = search_pick(ui, egui::Id::new(("calc-item", k)), &cur, &items, hw) {
                        p.set.item = if n == "(none)" { String::new() } else { n };
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                field(ui, "ABILITY", &mut |ui| {
                    let abs = d.species(&p.set.species).map(|s| s.abilities_in(world)).unwrap_or_default();
                    kit::Dropdown::from_id_salt(("calc-ab", k)).selected_text(if p.set.ability.is_empty() { "—".to_string() } else { p.set.ability.clone() }).width(hw).show_ui(ui, |ui| {
                        for a in &abs {
                            if ui.selectable_label(p.set.ability == *a, a).clicked() {
                                p.set.ability = a.clone();
                            }
                        }
                    });
                    if ["Flash Fire", "Plus", "Minus", "Slow Start", "Intimidate"].contains(&p.set.ability.as_str()) {
                        kit::checkbox(ui, &mut p.ability_on, "Active");
                    }
                });
                field(ui, "NATURE", &mut |ui| {
                    kit::Dropdown::from_id_salt(("calc-nat", k)).selected_text(nature_label(&p.set.nature)).width(hw).height(320.0).show_ui(ui, |ui| {
                        for n in &d.natures {
                            ui.selectable_value(&mut p.set.nature, n.name.clone(), nature_label(&n.name));
                        }
                    });
                });
            });
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                field(ui, "STATUS", &mut |ui| {
                    kit::Dropdown::from_id_salt(("calc-st", k)).selected_text(status_label(p.status)).width(hw).show_ui(ui, |ui| {
                        for s in [Status::None, Status::Burn, Status::Paralysis, Status::Poison, Status::Toxic, Status::Sleep, Status::Freeze] {
                            ui.selectable_value(&mut p.status, s, status_label(s));
                        }
                    });
                });
                field(ui, "CURRENT HP", &mut |ui| {
                    let mut pct = (p.hp * 100.0).round() as u32;
                    if pct_slider(ui, &mut pct, hw) {
                        p.hp = pct as f32 / 100.0;
                    }
                });
            });
            ui.add_space(14.0);
            // stats: EVs, IVs, boosts and what they come to. Every row is
            // one fixed grid, so the columns line up whatever the numbers.
            let stats = p.set.final_stats(world).unwrap_or_default();
            let nat = d.nature(&p.set.nature).filter(|n| !n.neutral()).map(|n| (n.plus, n.minus));
            let cols = StatCols::new(w);
            {
                let (hr, _) = ui.allocate_exact_size(egui::vec2(w, 16.0), Sense::hover());
                let c = cols.at(hr);
                let pt = ui.painter();
                let f = crate::fonts::sans_bold_font(10.5);
                pt.text(egui::pos2(c.slider.min.x + 6.0, hr.center().y), Align2::LEFT_CENTER, "EVS", f.clone(), MUTED());
                pt.text(egui::pos2(c.ev.center().x, hr.center().y), Align2::CENTER_CENTER, "EV", f.clone(), MUTED());
                pt.text(egui::pos2(c.iv.center().x, hr.center().y), Align2::CENTER_CENTER, "IV", f.clone(), MUTED());
                pt.text(egui::pos2(c.boost.center().x, hr.center().y), Align2::CENTER_CENTER, "BOOST", f.clone(), MUTED());
                pt.text(egui::pos2(c.stat.max.x, hr.center().y), Align2::RIGHT_CENTER, "STAT", f, MUTED());
            }
            ui.add_space(4.0);
            for i in 0..6 {
                let (row, _) = ui.allocate_exact_size(egui::vec2(w, 30.0), Sense::hover());
                let c = cols.at(row);
                let mid = row.center().y;
                // zebra rows keep the eye on one stat
                if i % 2 == 1 {
                    ui.painter().rect_filled(row.expand2(egui::vec2(6.0, 0.0)), 8.0, theme::blend(CARD(), TEXT(), 0.025));
                }
                let (txt, col) = match nat {
                    Some((plus, _)) if plus == i => (format!("{} ▲", STATS[i]), super::charts::ink(kit::C_GREEN)),
                    Some((_, minus)) if minus == i => (format!("{} ▼", STATS[i]), super::charts::ink(kit::C_RED)),
                    _ => (STATS[i].to_string(), TEXT()),
                };
                ui.painter().text(egui::pos2(c.name.min.x, mid), Align2::LEFT_CENTER, txt, crate::fonts::sans_bold_font(12.0), col);
                ev_slider_at(ui, egui::Id::new(("calc-ev-sl", k, i)), c.slider, &mut p.set.evs[i]);
                ui.put(c.ev, egui::DragValue::new(&mut p.set.evs[i]).range(0..=252).speed(2.0));
                ui.put(c.iv, egui::DragValue::new(&mut p.set.ivs[i]).range(0..=31).speed(0.2));
                if i > 0 {
                    let minus = Rect::from_center_size(egui::pos2(c.boost.min.x + 11.0, mid), egui::vec2(22.0, 22.0));
                    let plus = Rect::from_center_size(egui::pos2(c.boost.max.x - 11.0, mid), egui::vec2(22.0, 22.0));
                    if mini_btn_at(ui, egui::Id::new(("calc-b-", k, i)), minus, "−").on_hover_text("Lower").clicked() {
                        p.boosts[i] = (p.boosts[i] - 1).max(-6);
                    }
                    if mini_btn_at(ui, egui::Id::new(("calc-b+", k, i)), plus, "+").on_hover_text("Raise").clicked() {
                        p.boosts[i] = (p.boosts[i] + 1).min(6);
                    }
                    let b = p.boosts[i];
                    let bt = if b > 0 { format!("+{b}") } else { b.to_string() };
                    let bc = if b > 0 { super::charts::ink(kit::C_RED) } else if b < 0 { super::charts::ink(kit::C_BLUE) } else { MUTED() };
                    ui.painter().text(egui::pos2(c.boost.center().x, mid), Align2::CENTER_CENTER, bt, crate::fonts::sans_bold_font(12.0), bc);
                }
                let shown = if i > 0 { boosted(stats[i] as u32, p.boosts[i]) } else { stats[i] as u32 };
                let sc = if p.boosts[i] > 0 { super::charts::ink(kit::C_RED) } else if p.boosts[i] < 0 { super::charts::ink(kit::C_BLUE) } else { TEXT() };
                ui.painter().text(egui::pos2(c.stat.max.x, mid), Align2::RIGHT_CENTER, shown.to_string(), crate::fonts::sans_bold_font(13.5), sc);
                ui.add_space(2.0);
            }
            let used: u32 = p.set.evs.iter().map(|e| *e as u32).sum();
            ui.add_space(2.0);
            ui.label(RichText::new(format!("EVs used {used} / 510")).size(12.0).color(if used > 510 { super::charts::ink(kit::C_RED) } else { MUTED() }));
            ui.add_space(12.0);
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
                let ty = p.set.moves.get(i).and_then(|n| calc_move(wiki.as_deref(), n, &p.set, false)).map(|m| m.t);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let (tr, _) = ui.allocate_exact_size(egui::vec2(70.0, 30.0), Sense::hover());
                    if let Some(ty) = &ty {
                        type_tag(ui.painter(), egui::pos2(tr.min.x, tr.center().y), ty, 9.5);
                    }
                    if let Some(n) = search_pick(ui, egui::Id::new(("calc-mv", k, i)), &cur, &all, (ui.available_width() - 32.0).max(110.0)) {
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
            ui.add_space(6.0);
            kit::checkbox(ui, &mut p.crit, "Every move crits");
            ui.add_space(pad);
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
}

/// One move's result: (name, the move, the hit, KO text, Showdown's line).
type Row = (String, Option<calc::Move>, calc::Hit, String, String);

fn short_name(m: &Member) -> String {
    m.display_name().to_string()
}

/// An EV slider: 0–252 in steps of 4, dragged or clicked.
fn ev_slider(ui: &mut egui::Ui, v: &mut u16, w: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 26.0), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - r.min.x - 6.0) / (r.width() - 12.0)).clamp(0.0, 1.0);
            let nv = (((f * 252.0 / 4.0).round() * 4.0) as u16).min(252);
            if nv != *v {
                *v = nv;
                changed = true;
            }
        }
    }
    let frac = (*v as f32 / 252.0).clamp(0.0, 1.0);
    slider_paint(ui, r, frac, resp.hovered() || resp.dragged());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

/// A 0–100 % slider with its value on the right.
fn pct_slider(ui: &mut egui::Ui, v: &mut u32, w: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 32.0), Sense::click_and_drag());
    let track = Rect::from_min_max(r.min, egui::pos2(r.max.x - 50.0, r.max.y));
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - track.min.x - 6.0) / (track.width() - 12.0)).clamp(0.0, 1.0);
            let nv = (f * 100.0).round() as u32;
            if nv != *v {
                *v = nv;
                changed = true;
            }
        }
    }
    slider_paint(ui, track, *v as f32 / 100.0, resp.hovered() || resp.dragged());
    let pill = Rect::from_min_max(egui::pos2(r.max.x - 44.0, r.center().y - 13.0), egui::pos2(r.max.x, r.center().y + 13.0));
    ui.painter().rect_filled(pill, 13.0, kit::sunken());
    ui.painter().text(pill.center(), Align2::CENTER_CENTER, format!("{v}%"), crate::fonts::sans_bold_font(12.0), TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

/// The stat table's columns, the same for every row.
struct StatCols {
    slider_w: f32,
}

struct StatRow {
    name: Rect,
    slider: Rect,
    ev: Rect,
    iv: Rect,
    boost: Rect,
    stat: Rect,
}

impl StatCols {
    const NAME: f32 = 42.0;
    const EV: f32 = 50.0;
    const IV: f32 = 42.0;
    const BOOST: f32 = 70.0;
    const STAT: f32 = 38.0;
    const GAP: f32 = 8.0;

    fn new(w: f32) -> Self {
        let fixed = Self::NAME + Self::EV + Self::IV + Self::BOOST + Self::STAT + Self::GAP * 5.0;
        StatCols { slider_w: (w - fixed).max(40.0) }
    }

    /// The cells of one row (the boxes 26 tall, centred on the row).
    fn at(&self, row: Rect) -> StatRow {
        let (y, h) = (row.center().y, 26.0);
        let cell = |x: f32, w: f32| Rect::from_min_size(egui::pos2(x, y - h / 2.0), egui::vec2(w, h));
        let mut x = row.min.x;
        let name = cell(x, Self::NAME);
        x += Self::NAME + Self::GAP;
        let slider = cell(x, self.slider_w);
        x += self.slider_w + Self::GAP;
        let ev = cell(x, Self::EV);
        x += Self::EV + Self::GAP;
        let iv = cell(x, Self::IV);
        x += Self::IV + Self::GAP;
        let boost = cell(x, Self::BOOST);
        x += Self::BOOST + Self::GAP;
        let stat = cell(x, Self::STAT);
        StatRow { name, slider, ev, iv, boost, stat }
    }
}

/// `ev_slider` in a given rect.
fn ev_slider_at(ui: &mut egui::Ui, id: egui::Id, r: Rect, v: &mut u16) -> bool {
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - r.min.x - 6.0) / (r.width() - 12.0)).clamp(0.0, 1.0);
            let nv = (((f * 252.0 / 4.0).round() * 4.0) as u16).min(252);
            if nv != *v {
                *v = nv;
                changed = true;
            }
        }
    }
    slider_paint(ui, r, (*v as f32 / 252.0).clamp(0.0, 1.0), resp.hovered() || resp.dragged());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

/// `mini_btn` in a given rect.
fn mini_btn_at(ui: &mut egui::Ui, id: egui::Id, r: Rect, text: &str) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let ht = kit::anim(ui.ctx(), id.with("h"), resp.hovered(), kit::QUICK);
    ui.painter().rect_filled(r, 7.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
    ui.painter().rect_stroke(r, 7.0, Stroke::new(1.0f32, LINE()));
    ui.painter().text(r.center(), Align2::CENTER_CENTER, text, crate::fonts::sans_bold_font(13.0), TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small −/+ button for boosts.
fn mini_btn(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(20.0, 22.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    ui.painter().rect_filled(r, 6.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
    ui.painter().rect_stroke(r, 6.0, Stroke::new(1.0f32, LINE()));
    ui.painter().text(r.center(), Align2::CENTER_CENTER, text, crate::fonts::sans_bold_font(13.0), TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn slider_paint(ui: &egui::Ui, r: Rect, frac: f32, hot: bool) {
    let p = ui.painter();
    let y = r.center().y;
    let (x0, x1) = (r.min.x + 6.0, r.max.x - 6.0);
    let track = Rect::from_min_max(egui::pos2(x0, y - 2.5), egui::pos2(x1, y + 2.5));
    p.rect_filled(track, 2.5, theme::blend(kit::sunken(), TEXT(), 0.10));
    let x = x0 + (x1 - x0) * frac;
    if frac > 0.0 {
        p.rect_filled(Rect::from_min_max(track.min, egui::pos2(x, track.max.y)), 2.5, kit::primary());
    }
    p.circle_filled(egui::pos2(x, y), if hot { 7.5 } else { 6.5 }, Color32::WHITE);
    p.circle_stroke(egui::pos2(x, y), if hot { 7.5 } else { 6.5 }, Stroke::new(2.0f32, kit::primary()));
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
    kit::Dropdown::from_id_salt(id).selected_text(current).width(width).height(300.0).keep_open().show_ui(ui, |ui| {
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
                kit::close_dropdown(ui);
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
