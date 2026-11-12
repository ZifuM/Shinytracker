//! Battle Sim → Catch Lab: every ball's odds in the moment, the odds as its
//! HP drops, the best plan (HP × status), and throwing balls for real, with
//! the shakes.

use super::sim_home::{thousands, trim, RED};
use super::team_home::{caps, chip, small, soft};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::battle;
use crate::calc::Status;
use crate::sim::{self, Ball, CatchSituation, Format, Mode, BALLS};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::Instant;

const VIOLET: Color32 = kit::C_VIOLET;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum LabTab {
    #[default]
    Balls,
    Chart,
    Plan,
}

/// One ball thrown.
#[derive(Clone, Debug)]
pub(super) struct Throw {
    pub ball: Ball,
    pub shakes: u8,
    pub caught: bool,
    pub chance: f64,
}

#[derive(Clone)]
pub(super) struct CatchView {
    pub hp: f32,
    pub status: Status,
    pub sit: CatchSituation,
    pub tab: LabTab,
    /// The ball you throw.
    pub ball: Ball,
    pub throws: Vec<Throw>,
    /// The throw on screen: when it started.
    pub anim: Option<(Instant, Throw)>,
    /// Which Pokémon the throws were at (they reset when it changes).
    pub at: String,
}

impl Default for CatchView {
    fn default() -> Self {
        CatchView { hp: 1.0, status: Status::None, sit: CatchSituation::default(), tab: LabTab::Balls, ball: Ball::Ultra, throws: Vec::new(), anim: None, at: String::new() }
    }
}

/// Seconds a throw takes on screen: the arc, a beat per shake, the result.
fn throw_secs(t: &Throw) -> f32 {
    0.45 + 0.55 * t.shakes as f32 + 0.35
}

impl App {
    /// Open the Catch Lab for a Pokémon (from the Pokédex): at the level
    /// you'd meet it, as you'd meet it.
    pub(super) fn sim_test_catch(&mut self, wiki_id: &str) {
        let Some(sp) = battle::data().species(wiki_id) else { return };
        let world = if self.cfg.info_modern { crate::battle::World::Modern } else { crate::battle::World::Platinum };
        // where you'd first meet it: the base game's slot with the lowest level
        let first: Option<(String, String, u8)> = self.wiki.as_ref().and_then(|w| {
            let all = w.wild(wiki_id);
            let base: Vec<_> = all.iter().filter(|e| e.setting.is_empty()).collect();
            let pool = if base.is_empty() { all.iter().collect() } else { base };
            pool.into_iter().map(|e| (e.area.clone(), e.method.clone(), sim::parse_levels(&e.levels).0)).min_by_key(|e| e.2)
        });
        let lv = first.as_ref().map(|f| f.2).unwrap_or(30);
        let fv = &mut self.setup.sim.foe;
        fv.rolled = None;
        fv.synced = None;
        fv.sel = 0;
        match &first {
            Some((area, method, _)) => {
                fv.tab = super::sim_foes::FoeTab::Route;
                fv.area = area.clone();
                fv.method = method.clone();
            }
            None => fv.tab = super::sim_foes::FoeTab::Build,
        }
        let mut m = crate::teams::Member { level: lv, ..crate::teams::Member::new(&sp.name, world) };
        if let Some(w) = self.wiki.clone() {
            m.moves = super::sim_foes::wild_set(&w, &sp.name, lv, world);
        }
        self.setup.sim.file();
        self.setup.sim.started = true;
        let s = &mut self.setup.sim.setup;
        s.mode = Mode::Catch;
        s.format = Format::Singles;
        s.foes = vec![m];
        s.place.clear();
        if s.team.is_none() {
            let teams = self.team_choices();
            let s = &mut self.setup.sim.setup;
            s.team = teams.first().map(|t| t.0);
            s.picks = (0..teams.first().map(|t| t.2).unwrap_or(0).min(6)).collect();
        }
        self.setup.sim.catch = CatchView::default();
        self.go_to(super::Tab::Sim);
        self.setup.scroll_top = true;
    }

    /// The Catch Lab for the wild Pokémon (or the selected one in doubles).
    pub(super) fn sim_catch(&mut self, ui: &mut egui::Ui) {
        let s = self.setup.sim.setup.clone();
        let sel = if s.format == Format::Doubles && s.mode == Mode::Wild { self.setup.sim.foe.sel } else { 0 };
        let Some(f) = s.foes.get(sel).or(s.foes.first()).cloned() else { return };
        let world = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default();
        let Some(sp) = battle::data().species(&f.species) else { return };
        let Some(rate) = sim::data().catch_rate(sp.num) else { return };
        let max_hp = f.final_stats(world).map(|s| s[0] as u32).unwrap_or(100).max(1);
        let ty = sp.types_in(world).to_vec();
        let mut cv = self.setup.sim.catch.clone();
        // throws belong to one Pokémon
        let who = format!("{}:{}", f.species, f.level);
        if cv.at != who {
            cv.at = who;
            cv.throws.clear();
            cv.anim = None;
        }
        cv.sit.level = f.level;
        cv.sit.water_or_bug = ty.iter().any(|t| t == "Water" || t == "Bug");
        let prices: Vec<(String, u32)> = self
            .wiki
            .as_ref()
            .map(|w| w.data.items.iter().filter(|i| i.pocket == "Poké Balls").map(|i| (i.name.clone(), i.price.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))).collect())
            .unwrap_or_default();
        let price = |b: Ball| prices.iter().find(|(n, _)| n == b.name()).map(|x| x.1).unwrap_or(0);
        let lab = Lab { rate, max_hp };
        let hp_now = lab.hp(cv.hp);
        let ranked = lab.ranked(&cv.sit, hp_now, sim::status_mult(cv.status));
        let best = ranked.iter().find(|r| r.0 != Ball::Master).copied();
        kit::card(ui, |ui| {
            // head: who, and the best ball right now
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Catch Lab: {}", f.species)).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                ui.label(small(format!("Lv {} · catch rate {rate} · Platinum's formula", f.level)));
                if let Some((b, _, p)) = best {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let txt = format!("Best now: {} {:.1}%", b.name(), p * 100.0);
                        let g = ui.painter().layout_no_wrap(txt, crate::fonts::sans_bold_font(12.5), super::charts::ink(VIOLET));
                        let (r, _) = ui.allocate_exact_size(g.size() + egui::vec2(22.0, 12.0), Sense::hover());
                        ui.painter().rect_filled(r, r.height() / 2.0, super::charts::faint(VIOLET, 0.18));
                        ui.painter().galley(r.min + egui::vec2(11.0, 6.0), g, Color32::WHITE);
                    });
                }
            });
            ui.add_space(12.0);
            let avail = ui.available_width();
            let wide = avail >= 980.0;
            let ctl_w = if wide { 280.0 } else { avail };
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.allocate_ui_with_layout(egui::vec2(ctl_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(ctl_w);
                        controls(ui, &mut cv);
                    });
                    let rw = ui.available_width();
                    ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(rw);
                        tabs_and_view(ui, &mut cv, &lab, &ranked, &price);
                    });
                });
            } else {
                controls(ui, &mut cv);
                ui.add_space(12.0);
                tabs_and_view(ui, &mut cv, &lab, &ranked, &price);
            }
            ui.add_space(14.0);
            let (lr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(lr, 0.0, LINE());
            ui.add_space(14.0);
            self.catch_throws(ui, &mut cv, &lab, &f.species, f.shiny, &price);
        });
        self.setup.sim.catch = cv;
    }

    /// Pick a ball, throw it, watch it shake.
    fn catch_throws(&mut self, ui: &mut egui::Ui, cv: &mut CatchView, lab: &Lab, species: &str, shiny: bool, price: &dyn Fn(Ball) -> u32) {
        let head = ui.horizontal(|ui| {
            ui.label(RichText::new("Throw some").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(small("Every throw is a real roll of the four shake checks. Turns go up as you throw (Timer and Quick Ball care)."));
        });
        if std::mem::take(&mut self.setup.sim.scroll_lab) {
            head.response.scroll_to_me(Some(egui::Align::Min));
        }
        ui.add_space(8.0);
        let avail = ui.available_width();
        let stage_w = 300.0f32.min(avail);
        let mut throw_now = false;
        let mut reset = false;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            // the stage: the Pokémon, and the ball when thrown
            let (r, _) = ui.allocate_exact_size(egui::vec2(stage_w, 190.0), Sense::hover());
            self.catch_stage(ui, r, cv, species, shiny);
            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(caps("BALL"));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for b in BALLS.iter().filter(|b| !matches!(b, Ball::Safari | Ball::Cherish)) {
                        if chip(ui, b.name(), cv.ball == *b, VIOLET).clicked() {
                            cv.ball = *b;
                        }
                    }
                });
                ui.add_space(8.0);
                let p = lab.chance(cv.ball, &cv.sit, lab.hp(cv.hp), sim::status_mult(cv.status));
                let busy = cv.anim.as_ref().is_some_and(|(t, th)| t.elapsed().as_secs_f32() < throw_secs(th));
                // the newest throw stays hidden until the ball has settled
                let shown: &[Throw] = if busy { &cv.throws[..cv.throws.len().saturating_sub(1)] } else { &cv.throws };
                let caught = shown.last().is_some_and(|t| t.caught);
                ui.horizontal(|ui| {
                    let label = if busy { "Throwing…" } else if caught { "Caught! Throw again" } else { "Throw" };
                    throw_now = ui.add_enabled_ui(!busy, |ui| kit::accent_button_w(ui, egui::Button::new(RichText::new(format!("●  {label}")).size(14.0)), 8.0, 120.0)).inner.clicked();
                    ui.label(small(format!("{:.1}% this throw (turn {})", p * 100.0, cv.sit.turn + 1)));
                    if !cv.throws.is_empty() {
                        reset = soft(ui, "Start over").clicked();
                    }
                });
                ui.add_space(10.0);
                // the tally
                let n = shown.len();
                if n > 0 {
                    let spent: u32 = shown.iter().map(|t| price(t.ball)).sum();
                    let luck = (1.0 - shown.iter().map(|t| 1.0 - t.chance).product::<f64>()) * 100.0;
                    let got = shown.iter().position(|t| t.caught);
                    let line = match got {
                        Some(i) => format!("Caught on throw {} · {} · {:.0}% of hunters would have it by this throw", i + 1, if spent > 0 { format!("₽{} spent", thousands(spent)) } else { "free balls".into() }, luck),
                        None => format!("{n} thrown · ₽{} spent · about {luck:.0}% would have caught it by now", thousands(spent)),
                    };
                    ui.label(RichText::new(line).size(12.5).color(TEXT()));
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for (i, t) in shown.iter().enumerate().rev().take(24) {
                            let (txt, col) = if t.caught { ("caught".to_string(), kit::C_GREEN) } else { (format!("{} shake{}", t.shakes, if t.shakes == 1 { "" } else { "s" }), MUTED()) };
                            let g = ui.painter().layout_no_wrap(format!("{}. {} · {txt}", i + 1, short_ball(t.ball)), crate::fonts::sans_font(11.5), super::charts::ink(col));
                            let (r, _) = ui.allocate_exact_size(g.size() + egui::vec2(16.0, 8.0), Sense::hover());
                            ui.painter().rect_filled(r, r.height() / 2.0, if t.caught { super::charts::faint(kit::C_GREEN, 0.18) } else { kit::sunken() });
                            ui.painter().galley(r.min + egui::vec2(8.0, 4.0), g, Color32::WHITE);
                        }
                    });
                } else {
                    ui.label(small(if busy { "The ball's in the air…" } else { "Nothing thrown yet." }));
                }
            });
        });
        if reset {
            cv.throws.clear();
            cv.anim = None;
            cv.sit.turn = 0;
        }
        if throw_now {
            if cv.throws.last().is_some_and(|t| t.caught) {
                cv.throws.clear();
                cv.sit.turn = 0;
            }
            let hp = lab.hp(cv.hp);
            let m = sim::ball_mult(cv.ball, &cv.sit);
            let sm = sim::status_mult(cv.status);
            let chance = sim::catch_chance(lab.rate, lab.max_hp, hp, m, sm);
            let mut rng = sim::Rng::from_time();
            let (shakes, caught) = sim::throw(lab.rate, lab.max_hp, hp, m, sm, &mut rng);
            let t = Throw { ball: cv.ball, shakes, caught, chance };
            cv.throws.push(t.clone());
            cv.anim = Some((Instant::now(), t));
            cv.sit.turn += 1;
        }
    }

    /// The Pokémon, and a ball that flies, shakes and clicks shut (or opens).
    fn catch_stage(&mut self, ui: &mut egui::Ui, r: Rect, cv: &CatchView, species: &str, shiny: bool) {
        let p = ui.painter_at(r);
        p.rect_filled(r, 14.0, theme::blend(kit::sunken(), VIOLET, 0.06));
        // a little ground
        let ground = egui::pos2(r.center().x + 50.0, r.max.y - 44.0);
        p.add(egui::Shape::ellipse_filled(ground, egui::vec2(52.0, 10.0), theme::blend(kit::sunken(), MUTED(), 0.18)));
        let Some((start, t)) = cv.anim.clone() else {
            if !self.team_sprite(&p, species, shiny, ground - egui::vec2(0.0, 40.0), egui::vec2(96.0, 96.0)) {
                p.text(ground - egui::vec2(0.0, 40.0), Align2::CENTER_CENTER, species, crate::fonts::sans_bold_font(14.0), MUTED());
            }
            p.text(egui::pos2(r.min.x + 14.0, r.max.y - 16.0), Align2::LEFT_CENTER, "Pick a ball and throw", crate::fonts::sans_font(11.5), MUTED());
            return;
        };
        let el = start.elapsed().as_secs_f32();
        let total = throw_secs(&t);
        let fly = 0.45;
        let from = egui::pos2(r.min.x + 30.0, r.max.y - 30.0);
        let rest = ground - egui::vec2(0.0, 14.0);
        if el < fly {
            // still flying: the Pokémon is there, the ball arcs over
            if !self.team_sprite(&p, species, shiny, ground - egui::vec2(0.0, 40.0), egui::vec2(96.0, 96.0)) {
                p.text(ground - egui::vec2(0.0, 40.0), Align2::CENTER_CENTER, species, crate::fonts::sans_bold_font(14.0), MUTED());
            }
            let k = el / fly;
            let pos = from + (rest - from) * k - egui::vec2(0.0, 90.0 * (k * std::f32::consts::PI).sin());
            draw_ball(&p, pos, 13.0, k * 9.0, t.ball);
        } else {
            let after = el - fly;
            let shake_t = 0.55;
            let shaking = after < shake_t * t.shakes as f32;
            if shaking {
                let k = (after % shake_t) / shake_t;
                let wob = if k < 0.6 { (k / 0.6 * std::f32::consts::TAU).sin() * 0.45 } else { 0.0 };
                draw_ball(&p, rest, 13.0, wob, t.ball);
                let n = (after / shake_t) as u8 + 1;
                p.text(egui::pos2(r.min.x + 14.0, r.max.y - 16.0), Align2::LEFT_CENTER, format!("{}…", ["", "Shake", "Shake shake", "Shake shake shake"][n.min(3) as usize]), crate::fonts::sans_bold_font(12.0), TEXT());
            } else if t.caught {
                draw_ball(&p, rest, 13.0, 0.0, t.ball);
                // little stars
                let k = ((el - fly - shake_t * t.shakes as f32) / 0.35).min(1.0);
                for i in 0..3 {
                    let a = -std::f32::consts::FRAC_PI_2 + (i as f32 - 1.0) * 0.7;
                    let c = rest + egui::vec2(a.cos(), a.sin()) * (18.0 + 14.0 * k);
                    p.text(c, Align2::CENTER_CENTER, "✨", crate::fonts::sans_font(13.0), kit::C_GOLD);
                }
                p.text(egui::pos2(r.min.x + 14.0, r.max.y - 16.0), Align2::LEFT_CENTER, format!("Gotcha! {species} was caught!"), crate::fonts::sans_bold_font(12.5), super::charts::ink(kit::C_GREEN));
            } else {
                if !self.team_sprite(&p, species, shiny, ground - egui::vec2(0.0, 40.0), egui::vec2(96.0, 96.0)) {
                    p.text(ground - egui::vec2(0.0, 40.0), Align2::CENTER_CENTER, species, crate::fonts::sans_bold_font(14.0), MUTED());
                }
                let msg = match t.shakes {
                    0 => "Oh no! It broke free!",
                    1 => "Aww! It appeared to be caught!",
                    2 => "Aargh! Almost had it!",
                    _ => "Gah! It was so close, too!",
                };
                p.text(egui::pos2(r.min.x + 14.0, r.max.y - 16.0), Align2::LEFT_CENTER, msg, crate::fonts::sans_bold_font(12.5), super::charts::ink(RED));
            }
        }
        if el < total {
            ui.ctx().request_repaint();
        }
    }
}

/// The numbers behind the lab.
struct Lab {
    rate: u16,
    max_hp: u32,
}

impl Lab {
    fn hp(&self, frac: f32) -> u32 {
        ((self.max_hp as f32 * frac).round() as u32).clamp(1, self.max_hp)
    }

    fn chance(&self, b: Ball, sit: &CatchSituation, hp: u32, status: f64) -> f64 {
        sim::catch_chance(self.rate, self.max_hp, hp, sim::ball_mult(b, sit), status)
    }

    /// Every ball, best first (the Master Ball last): (ball, multiplier, chance).
    fn ranked(&self, sit: &CatchSituation, hp: u32, status: f64) -> Vec<(Ball, f64, f64)> {
        let mut v: Vec<(Ball, f64, f64)> = BALLS
            .iter()
            .filter(|b| !matches!(b, Ball::Safari | Ball::Cherish))
            .map(|b| {
                let m = sim::ball_mult(*b, sit);
                (*b, m, sim::catch_chance(self.rate, self.max_hp, hp, m, status))
            })
            .collect();
        v.sort_by(|a, b| (a.0 == Ball::Master).cmp(&(b.0 == Ball::Master)).then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)));
        v
    }
}

fn short_ball(b: Ball) -> &'static str {
    b.name().trim_end_matches(" Ball")
}

/// A Poké Ball, turned by `tilt` radians, in the ball's colours.
fn draw_ball(p: &egui::Painter, c: egui::Pos2, r: f32, tilt: f32, b: Ball) {
    let top = match b {
        Ball::Great => Color32::from_rgb(60, 110, 220),
        Ball::Ultra => Color32::from_rgb(40, 40, 46),
        Ball::Master => Color32::from_rgb(130, 60, 190),
        Ball::Net => Color32::from_rgb(40, 160, 170),
        Ball::Dive => Color32::from_rgb(40, 110, 200),
        Ball::Nest => Color32::from_rgb(120, 180, 60),
        Ball::Repeat | Ball::Heal => Color32::from_rgb(230, 120, 160),
        Ball::Timer => Color32::from_rgb(230, 230, 230),
        Ball::Luxury | Ball::Dusk => Color32::from_rgb(40, 50, 60),
        Ball::Quick => Color32::from_rgb(70, 140, 230),
        Ball::Premier => Color32::from_rgb(245, 245, 245),
        _ => Color32::from_rgb(222, 64, 60),
    };
    let n = 24;
    let (s, co) = tilt.sin_cos();
    let rot = |v: egui::Vec2| egui::vec2(v.x * co - v.y * s, v.x * s + v.y * co);
    let half = |from: f32| -> Vec<egui::Pos2> { (0..=n).map(|i| from + std::f32::consts::PI * i as f32 / n as f32).map(|a| c + rot(egui::vec2(a.cos(), a.sin()) * r)).collect() };
    p.add(egui::Shape::convex_polygon(half(std::f32::consts::PI), top, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(half(0.0), Color32::from_rgb(245, 245, 245), Stroke::NONE));
    p.circle_stroke(c, r, Stroke::new(2.0f32, Color32::from_rgb(30, 30, 34)));
    p.line_segment([c + rot(egui::vec2(-r, 0.0)), c + rot(egui::vec2(r, 0.0))], Stroke::new(2.0f32, Color32::from_rgb(30, 30, 34)));
    p.circle_filled(c, r * 0.32, Color32::from_rgb(30, 30, 34));
    p.circle_filled(c, r * 0.2, Color32::WHITE);
}

/// HP, status and the battle around the throw.
fn controls(ui: &mut egui::Ui, cv: &mut CatchView) {
    ui.label(caps("ITS HP"));
    ui.horizontal(|ui| {
        let mut pct = (cv.hp * 100.0).round() as u32;
        let label = if pct == 0 { "1 HP".to_string() } else { format!("{pct}%") };
        if kit::slider_u32(ui, &mut pct, 0..=100, &label).changed() {
            cv.hp = pct as f32 / 100.0;
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (v, l) in [(1.0f32, "Full"), (0.5, "Half"), (0.2, "Red HP"), (0.0, "1 HP")] {
            if chip(ui, l, (cv.hp - v).abs() < 0.005, VIOLET).clicked() {
                cv.hp = v;
            }
        }
    });
    ui.label(small("False Swipe always leaves it at 1 HP."));
    ui.add_space(8.0);
    ui.label(caps("STATUS"));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (v, l) in [(Status::None, "None"), (Status::Sleep, "Asleep ×2"), (Status::Freeze, "Frozen ×2"), (Status::Paralysis, "Paralyzed ×1.5"), (Status::Poison, "Poisoned ×1.5"), (Status::Burn, "Burned ×1.5")] {
            if chip(ui, l, cv.status == v, VIOLET).clicked() {
                cv.status = v;
            }
        }
    });
    ui.add_space(8.0);
    ui.label(caps("THE BATTLE"));
    ui.horizontal(|ui| {
        ui.label(small("Turn"));
        let mut t = cv.sit.turn + 1;
        ui.add(egui::DragValue::new(&mut t).range(1..=99).speed(0.2));
        cv.sit.turn = t - 1;
        ui.label(small(if cv.sit.turn == 0 { "(first throw)" } else { "" }));
    });
    kit::checkbox(ui, &mut cv.sit.night_or_cave, "At night or in a cave");
    kit::checkbox(ui, &mut cv.sit.water, "Surfing or fishing");
    kit::checkbox(ui, &mut cv.sit.caught_before, "Caught one before");
}

fn tabs_and_view(ui: &mut egui::Ui, cv: &mut CatchView, lab: &Lab, ranked: &[(Ball, f64, f64)], price: &dyn Fn(Ball) -> u32) {
    ui.horizontal(|ui| {
        for (t, l) in [(LabTab::Balls, "Every ball"), (LabTab::Chart, "As its HP drops"), (LabTab::Plan, "Best plan")] {
            if chip(ui, l, cv.tab == t, VIOLET).clicked() {
                cv.tab = t;
            }
        }
    });
    ui.add_space(10.0);
    match cv.tab {
        LabTab::Balls => balls_view(ui, ranked, price),
        LabTab::Chart => chart_view(ui, cv, lab, ranked),
        LabTab::Plan => plan_view(ui, cv, lab),
    }
}

/// Every ball's chance, best first.
fn balls_view(ui: &mut egui::Ui, ranked: &[(Ball, f64, f64)], price: &dyn Fn(Ball) -> u32) {
    let best = ranked.first().map(|r| r.2).unwrap_or(0.0);
    let full = ui.available_width();
    for (b, m, p) in ranked {
        let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 30.0), Sense::hover());
        let pt = ui.painter().clone();
        let top = (*p - best).abs() < 1e-9 && *b != Ball::Master;
        if top {
            pt.rect_filled(r, 8.0, theme::blend(CARD(), VIOLET, 0.12));
        } else if resp.hovered() {
            pt.rect_filled(r, 8.0, kit::sunken());
        }
        draw_ball(&pt, egui::pos2(r.min.x + 16.0, r.center().y), 7.5, 0.0, *b);
        pt.text(egui::pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, b.name(), crate::fonts::sans_bold_font(12.5), TEXT());
        let ms = if *b == Ball::Master { "—".to_string() } else { format!("×{}", trim(*m)) };
        pt.text(egui::pos2(r.min.x + 132.0, r.center().y), Align2::LEFT_CENTER, ms, crate::fonts::sans_font(11.5), if *m > 1.0 { super::charts::ink(VIOLET) } else { MUTED() });
        let bx = r.min.x + 172.0;
        let bw = (full - 172.0 - 230.0).max(60.0);
        let bar = Rect::from_min_size(egui::pos2(bx, r.center().y - 5.0), egui::vec2(bw, 10.0));
        pt.rect_filled(bar, 5.0, kit::sunken());
        pt.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bw * *p as f32, 10.0)), 5.0, if top { VIOLET } else { theme::blend(VIOLET, MUTED(), 0.45) });
        pt.text(egui::pos2(bar.max.x + 10.0, r.center().y), Align2::LEFT_CENTER, format!("{:.1}%", p * 100.0), crate::fonts::sans_bold_font(12.5), TEXT());
        let need = sim::balls_for(*p, 0.9);
        let pr = price(*b);
        let txt = match need {
            Some(1) => "1 is enough".to_string(),
            Some(n) if n <= 999 => {
                if pr > 0 {
                    format!("{n} for 90%  ·  ₽{}", thousands(n * pr))
                } else {
                    format!("{n} for 90%")
                }
            }
            _ => "almost never".to_string(),
        };
        pt.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, txt, crate::fonts::sans_font(11.5), MUTED());
        if !b.when().is_empty() {
            resp.on_hover_text(b.when());
        }
    }
}

/// Odds against HP for a few balls, with where it is now.
fn chart_view(ui: &mut egui::Ui, cv: &CatchView, lab: &Lab, ranked: &[(Ball, f64, f64)]) {
    // the best three now, plus the Poké Ball as the baseline
    // one line per multiplier: balls that work the same share it
    let mut groups: Vec<(f64, Vec<Ball>)> = Vec::new();
    for (b, m, _) in ranked.iter().filter(|r| r.0 != Ball::Master) {
        match groups.iter_mut().find(|g| (g.0 - m).abs() < 1e-9) {
            Some(g) => g.1.push(*b),
            None => groups.push((*m, vec![*b])),
        }
    }
    let mut picked: Vec<(f64, Vec<Ball>)> = groups.iter().take(3).cloned().collect();
    if !picked.iter().any(|g| g.1.contains(&Ball::Poke)) {
        if let Some(g) = groups.iter().find(|g| g.1.contains(&Ball::Poke)) {
            picked.push(g.clone());
        }
    }
    let balls: Vec<Ball> = picked.iter().map(|g| g.1[0]).collect();
    let names: Vec<String> = picked.iter().map(|g| g.1.iter().take(3).map(|b| short_ball(*b)).collect::<Vec<_>>().join(" / ") + if g.1.len() > 3 { " / …" } else { "" }).collect();
    let colors = [VIOLET, kit::C_CYAN, kit::C_GOLD, MUTED()];
    let sm = sim::status_mult(cv.status);
    let full = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 260.0), Sense::hover());
    let p = ui.painter().clone();
    let plot = Rect::from_min_max(r.min + egui::vec2(40.0, 10.0), r.max - egui::vec2(10.0, 28.0));
    // grid
    for i in 0..=4 {
        let y = plot.max.y - plot.height() * i as f32 / 4.0;
        p.line_segment([egui::pos2(plot.min.x, y), egui::pos2(plot.max.x, y)], Stroke::new(1.0f32, theme::blend(LINE(), CARD(), 0.3)));
        p.text(egui::pos2(plot.min.x - 6.0, y), Align2::RIGHT_CENTER, format!("{}%", i * 25), crate::fonts::sans_font(10.5), MUTED());
    }
    for (frac, l) in [(1.0f32, "Full HP"), (0.5, "Half"), (0.0, "1 HP")] {
        let x = plot.min.x + plot.width() * (1.0 - frac);
        p.text(egui::pos2(x, plot.max.y + 14.0), Align2::CENTER_CENTER, l, crate::fonts::sans_font(10.5), MUTED());
    }
    // the lines: x goes from full HP (left) to 1 HP (right)
    let steps = 60;
    for (k, b) in balls.iter().enumerate() {
        let pts: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let frac = 1.0 - i as f32 / steps as f32;
                let c = lab.chance(*b, &cv.sit, lab.hp(frac), sm);
                egui::pos2(plot.min.x + plot.width() * i as f32 / steps as f32, plot.max.y - plot.height() * c as f32)
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(if k == 0 { 2.5f32 } else { 1.8 }, colors[k.min(3)])));
    }
    // now
    let x_now = plot.min.x + plot.width() * (1.0 - cv.hp);
    p.line_segment([egui::pos2(x_now, plot.min.y), egui::pos2(x_now, plot.max.y)], Stroke::new(1.0f32, theme::blend(TEXT(), CARD(), 0.4)));
    p.text(egui::pos2(x_now + 4.0, plot.min.y + 6.0), Align2::LEFT_TOP, "now", crate::fonts::sans_font(10.5), MUTED());
    // hover: read the odds at that HP
    if let Some(pos) = resp.hover_pos().filter(|p| plot.contains(*p)) {
        let frac = 1.0 - ((pos.x - plot.min.x) / plot.width()).clamp(0.0, 1.0);
        let hp = lab.hp(frac);
        let lines: Vec<String> = balls.iter().zip(&names).map(|(b, n)| format!("{n}: {:.1}%", lab.chance(*b, &cv.sit, hp, sm) * 100.0)).collect();
        p.line_segment([egui::pos2(pos.x, plot.min.y), egui::pos2(pos.x, plot.max.y)], Stroke::new(1.0f32, theme::blend(LINE(), VIOLET, 0.6)));
        resp.on_hover_text(format!("At {} HP ({:.0}%)\n{}", hp, frac * 100.0, lines.join("\n")));
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for (k, n) in names.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
            ui.painter().rect_filled(Rect::from_center_size(r.center(), egui::vec2(12.0, 4.0)), 2.0, colors[k.min(3)]);
            ui.label(small(format!("{n} Ball")));
            ui.add_space(8.0);
        }
    });
}

/// The best ball for each mix of HP and status: what's worth doing first.
fn plan_view(ui: &mut egui::Ui, cv: &CatchView, lab: &Lab) {
    let hps: [(f32, &str); 4] = [(1.0, "Full HP"), (0.5, "Half"), (0.2, "Red HP"), (0.0, "1 HP")];
    let sts: [(f64, &str); 3] = [(1.0, "No status"), (1.5, "Paralyzed etc."), (2.0, "Asleep")];
    let full = ui.available_width();
    let lw = 120.0;
    let cw = ((full - lw) / hps.len() as f32).floor();
    let (hr, _) = ui.allocate_exact_size(egui::vec2(full, 22.0), Sense::hover());
    for (i, (_, l)) in hps.iter().enumerate() {
        ui.painter().text(egui::pos2(hr.min.x + lw + cw * (i as f32 + 0.5), hr.center().y), Align2::CENTER_CENTER, l.to_uppercase(), crate::fonts::sans_bold_font(10.0), MUTED());
    }
    for (sm, sl) in sts {
        let (r, _) = ui.allocate_exact_size(egui::vec2(full, 56.0), Sense::hover());
        let p = ui.painter().clone();
        p.text(egui::pos2(r.min.x + 4.0, r.center().y), Align2::LEFT_CENTER, sl, crate::fonts::sans_bold_font(12.0), TEXT());
        for (i, (frac, _)) in hps.iter().enumerate() {
            let hp = lab.hp(*frac);
            // getting its HP down or a status on it takes a turn or more
            let mut sit = cv.sit;
            if *frac < 1.0 || sm > 1.0 {
                sit.turn = sit.turn.max(1 + (*frac < 1.0) as u32 + (sm > 1.0) as u32 - 1);
            }
            let mut best = (Ball::Poke, 0.0f64);
            for b in BALLS.iter().filter(|b| !matches!(b, Ball::Master | Ball::Safari | Ball::Cherish)) {
                let c = lab.chance(*b, &sit, hp, sm);
                if c > best.1 {
                    best = (*b, c);
                }
            }
            let cell = Rect::from_min_size(egui::pos2(r.min.x + lw + cw * i as f32 + 3.0, r.min.y + 3.0), egui::vec2(cw - 6.0, r.height() - 6.0));
            p.rect_filled(cell, 8.0, theme::blend(kit::sunken(), VIOLET, (best.1 as f32 * 0.55).min(0.55)));
            let now = (cv.hp - frac).abs() < 0.005 && (sim::status_mult(cv.status) - sm).abs() < 1e-9;
            if now {
                p.rect_stroke(cell, 8.0, Stroke::new(1.5f32, VIOLET));
            }
            p.text(egui::pos2(cell.center().x, cell.min.y + 18.0), Align2::CENTER_CENTER, format!("{:.0}%", best.1 * 100.0), crate::fonts::sans_bold_font(15.0), TEXT());
            let need = sim::balls_for(best.1, 0.9).map(|n| if n == 1 { "1 ball".to_string() } else { format!("{n} for 90%") }).unwrap_or_default();
            p.text(egui::pos2(cell.center().x, cell.min.y + 36.0), Align2::CENTER_CENTER, format!("{} · {need}", short_ball(best.0)), crate::fonts::sans_font(10.5), MUTED());
        }
        ui.add_space(4.0);
    }
    ui.add_space(4.0);
    ui.label(small("Each square is the best ball for that HP and status here (place, time). Lowering its HP or giving it a status takes a turn, so the Quick Ball only counts at full HP with no status. The outlined square is where you are now."));
}
