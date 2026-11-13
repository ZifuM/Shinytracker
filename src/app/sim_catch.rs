//! Battle Sim → Catch Lab: every ball's odds in the moment, the odds as its
//! HP drops, the best plan (HP × status), and throwing balls for real, with
//! the shakes.

use super::sim_home::{thousands, trim};
use super::team_home::{caps, chip, small};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::battle;
use crate::calc::Status;
use crate::sim::{self, Ball, CatchSituation, Format, Mode, BALLS};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

const VIOLET: Color32 = kit::C_VIOLET;

#[derive(Clone)]
pub(super) struct CatchView {
    pub hp: f32,
    pub status: Status,
    pub sit: CatchSituation,
    /// Which Pokémon the lab is for (the HP and status reset when it changes).
    pub at: String,
}

impl Default for CatchView {
    fn default() -> Self {
        CatchView { hp: 1.0, status: Status::None, sit: CatchSituation::default(), at: String::new() }
    }
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
        s.mode = Mode::Wild;
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
        self.setup.sim.view = super::sim_home::SimView::Catch;
        self.go_to(super::Tab::Sim);
        self.setup.scroll_top = true;
    }

    /// Everything the catch views need for the wild Pokémon (or the
    /// selected one in doubles).
    fn catch_prep(&mut self) -> Option<Prep> {
        let s = self.setup.sim.setup.clone();
        let sel = if s.format == Format::Doubles && s.mode == Mode::Wild { self.setup.sim.foe.sel } else { 0 };
        let f = s.foes.get(sel).or(s.foes.first()).cloned()?;
        let world = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default();
        let sp = battle::data().species(&f.species)?;
        let rate = sim::data().catch_rate(sp.num)?;
        let max_hp = f.final_stats(world).map(|s| s[0] as u32).unwrap_or(100).max(1);
        let ty = sp.types_in(world).to_vec();
        let mut cv = self.setup.sim.catch.clone();
        // the HP and status belong to one Pokémon
        let who = format!("{}:{}", f.species, f.level);
        if cv.at != who {
            cv.at = who;
            cv.hp = 1.0;
            cv.status = Status::None;
        }
        cv.sit.level = f.level;
        cv.sit.water_or_bug = ty.iter().any(|t| t == "Water" || t == "Bug");
        let prices: Vec<(String, u32)> = self
            .wiki
            .as_ref()
            .map(|w| w.data.items.iter().filter(|i| i.pocket == "Poké Balls").map(|i| (i.name.clone(), i.price.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))).collect())
            .unwrap_or_default();
        Some(Prep { foe: f, rate, lab: Lab { rate, max_hp }, cv, prices })
    }
}

impl App {
    /// The Pokémon you're testing, with a button that opens the picker
    /// under it.
    fn catch_target(&mut self, ui: &mut egui::Ui, f: &crate::teams::Member, rate: u16, best: Option<(Ball, f64, f64)>) {
        let id = egui::Id::new("catch-pick-open");
        let mut open: bool = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(false);
        let mut throw = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(84.0, 84.0), Sense::hover());
                let p = ui.painter().clone();
                p.rect_filled(r, 14.0, theme::blend(kit::sunken(), VIOLET, 0.12));
                if !self.team_sprite(&p, &f.species, f.shiny, r.center(), egui::vec2(78.0, 78.0)) {
                    p.text(r.center(), Align2::CENTER_CENTER, f.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(16.0), MUTED());
                }
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("{}{}", f.display_name(), if f.shiny { "  ✨" } else { "" })).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
                    ui.label(small(super::sim_foes::describe(f)));
                    ui.label(small(format!("Catch rate {rate}  ·  Platinum's formula")));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ready = self.sim_catch_ready();
                    let b = ui.add_enabled_ui(ready.is_ok(), |ui| kit::accent_button_w(ui, egui::Button::new(RichText::new("🎯  Try catching it").size(14.0)), 8.0, 170.0)).inner;
                    let b = match &ready {
                        Err(why) => b.on_disabled_hover_text(why.as_str()),
                        Ok(()) => b.on_hover_text("Meet it in a battle with your team and throw for real"),
                    };
                    throw = b.clicked();
                    if ui.add(egui::Button::new(RichText::new(if open { "Done" } else { "Change Pokémon" }).size(13.0)).fill(kit::btn_fill()).min_size(egui::vec2(0.0, 32.0))).clicked() {
                        open = !open;
                    }
                    if let Some((b, _, p)) = best {
                        ui.add_space(8.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(150.0, 48.0), Sense::hover());
                        let pt = ui.painter();
                        pt.rect_filled(r, 12.0, super::charts::faint(VIOLET, 0.16));
                        draw_ball(pt, egui::pos2(r.min.x + 22.0, r.center().y), 13.0, 0.0, b);
                        pt.text(egui::pos2(r.min.x + 42.0, r.min.y + 15.0), Align2::LEFT_CENTER, "BEST RIGHT NOW", crate::fonts::sans_bold_font(9.5), MUTED());
                        pt.text(egui::pos2(r.min.x + 42.0, r.min.y + 33.0), Align2::LEFT_CENTER, format!("{} {:.1}%", short_ball(b), p * 100.0), crate::fonts::sans_bold_font(13.5), super::charts::ink(VIOLET));
                    }
                });
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
        if open {
            ui.add_space(10.0);
            self.sim_wild(ui);
        }
        if throw {
            self.sim_catch_battle();
        }
    }

    /// The catch test: the Pokémon, the conditions on one bar, every ball
    /// as a card, and the odds as its HP drops.
    pub(super) fn catch_shelf(&mut self, ui: &mut egui::Ui) {
        let Some(Prep { foe: f, rate, lab, mut cv, prices }) = self.catch_prep() else { return };
        let price = |b: Ball| prices.iter().find(|(n, _)| n == b.name()).map(|x| x.1).unwrap_or(0);
        let ranked = lab.ranked(&cv.sit, lab.hp(cv.hp), sim::status_mult(cv.status));
        let best = ranked.iter().find(|r| r.0 != Ball::Master).copied();
        self.catch_target(ui, &f, rate, best);
        ui.add_space(12.0);
        kit::card(ui, |ui| controls_row(ui, &mut cv));
        ui.add_space(12.0);
        // the shelf
        let gap = 12.0;
        let full = ui.available_width();
        let cols = ((full + gap) / (190.0 + gap)).floor().max(2.0) as usize;
        let w = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in ranked.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (b, m, p) in row {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 132.0), Sense::hover());
                    let top = best.is_some_and(|x| x.0 == *b);
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let pt = ui.painter();
                    pt.rect_filled(r, 14.0, if top { theme::blend(CARD(), VIOLET, 0.16) } else { theme::blend(CARD(), kit::CARD_HOVER(), ht) });
                    pt.rect_stroke(r, 14.0, Stroke::new(if top { 2.0f32 } else { 1.0 }, if top { VIOLET } else { LINE() }));
                    draw_ball(pt, egui::pos2(r.min.x + 30.0, r.min.y + 32.0), 18.0, 0.0, *b);
                    pt.text(egui::pos2(r.min.x + 58.0, r.min.y + 24.0), Align2::LEFT_CENTER, b.name(), crate::fonts::sans_bold_font(13.5), TEXT());
                    let ms = if *b == Ball::Master { "always".to_string() } else { format!("×{}", trim(*m)) };
                    pt.text(egui::pos2(r.min.x + 58.0, r.min.y + 42.0), Align2::LEFT_CENTER, ms, crate::fonts::sans_font(11.5), if *m > 1.0 { super::charts::ink(VIOLET) } else { MUTED() });
                    if top {
                        let tag = Rect::from_min_size(egui::pos2(r.max.x - 52.0, r.min.y + 12.0), egui::vec2(40.0, 18.0));
                        pt.rect_filled(tag, 9.0, VIOLET);
                        pt.text(tag.center(), Align2::CENTER_CENTER, "BEST", crate::fonts::sans_bold_font(10.0), Color32::WHITE);
                    }
                    let pct = if *p >= 0.9995 { "100%".to_string() } else { format!("{:.1}%", p * 100.0) };
                    pt.text(egui::pos2(r.min.x + 16.0, r.min.y + 82.0), Align2::LEFT_CENTER, pct, crate::fonts::sans_bold_font(26.0), if top { super::charts::ink(VIOLET) } else { TEXT() });
                    let need = match sim::balls_for(*p, 0.9) {
                        Some(1) => "1 is enough".to_string(),
                        Some(n) if n <= 999 => {
                            let pr = price(*b);
                            if pr > 0 {
                                format!("{n} for 90%  ·  ₽{}", thousands(n * pr))
                            } else {
                                format!("{n} for 90%")
                            }
                        }
                        _ => "almost never".to_string(),
                    };
                    pt.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, need, crate::fonts::sans_font(11.5), MUTED());
                    if !b.when().is_empty() {
                        resp.on_hover_text(b.when());
                    }
                }
            });
            ui.add_space(gap);
        }
        ui.add_space(4.0);
        kit::card(ui, |ui| {
            ui.label(RichText::new("Your odds as its HP drops").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(small("The best balls from full HP down to 1 HP, with the status and battle above."));
            ui.add_space(8.0);
            chart_view(ui, &cv, &lab, &ranked);
        });
        self.setup.sim.catch = cv;
    }
}

/// The conditions on one wide bar: HP, status and the battle.
fn controls_row(ui: &mut egui::Ui, cv: &mut CatchView) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 8.0);
        ui.label(caps("ITS HP"));
        ui.spacing_mut().slider_width = 160.0;
        let mut pct = (cv.hp * 100.0).round() as u32;
        let label = if pct == 0 { "1 HP".to_string() } else { format!("{pct}%") };
        if kit::slider_u32(ui, &mut pct, 0..=100, &label).changed() {
            cv.hp = pct as f32 / 100.0;
        }
        for (v, l) in [(1.0f32, "Full"), (0.5, "Half"), (0.0, "1 HP")] {
            if chip(ui, l, (cv.hp - v).abs() < 0.005, VIOLET).clicked() {
                cv.hp = v;
            }
        }
        ui.add_space(16.0);
        ui.label(caps("STATUS"));
        for (v, l) in [(Status::None, "None"), (Status::Sleep, "Asleep"), (Status::Freeze, "Frozen"), (Status::Paralysis, "Paralyzed"), (Status::Poison, "Poisoned"), (Status::Burn, "Burned")] {
            if chip(ui, l, cv.status == v, VIOLET).clicked() {
                cv.status = v;
            }
        }
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 6.0);
        ui.label(caps("THE BATTLE"));
        ui.label(small("Turn"));
        let mut t = cv.sit.turn + 1;
        ui.add(egui::DragValue::new(&mut t).range(1..=99).speed(0.2));
        cv.sit.turn = t - 1;
        kit::checkbox(ui, &mut cv.sit.night_or_cave, "Night or cave");
        kit::checkbox(ui, &mut cv.sit.water, "Surfing or fishing");
        kit::checkbox(ui, &mut cv.sit.caught_before, "Caught one before");
    });
}

/// What the catch views work from.
struct Prep {
    foe: crate::teams::Member,
    rate: u16,
    lab: Lab,
    cv: CatchView,
    prices: Vec<(String, u32)>,
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

/// The ball's item picture (from PokéSprite), loaded once.
fn ball_texture(ctx: &egui::Context, b: Ball) -> Option<egui::TextureHandle> {
    let (key, bytes): (&str, &[u8]) = match b {
        Ball::Poke => ("poke", include_bytes!("../../assets/balls/poke.png")),
        Ball::Great => ("great", include_bytes!("../../assets/balls/great.png")),
        Ball::Ultra => ("ultra", include_bytes!("../../assets/balls/ultra.png")),
        Ball::Master => ("master", include_bytes!("../../assets/balls/master.png")),
        Ball::Safari => ("safari", include_bytes!("../../assets/balls/safari.png")),
        Ball::Net => ("net", include_bytes!("../../assets/balls/net.png")),
        Ball::Dive => ("dive", include_bytes!("../../assets/balls/dive.png")),
        Ball::Nest => ("nest", include_bytes!("../../assets/balls/nest.png")),
        Ball::Repeat => ("repeat", include_bytes!("../../assets/balls/repeat.png")),
        Ball::Timer => ("timer", include_bytes!("../../assets/balls/timer.png")),
        Ball::Luxury => ("luxury", include_bytes!("../../assets/balls/luxury.png")),
        Ball::Premier => ("premier", include_bytes!("../../assets/balls/premier.png")),
        Ball::Dusk => ("dusk", include_bytes!("../../assets/balls/dusk.png")),
        Ball::Heal => ("heal", include_bytes!("../../assets/balls/heal.png")),
        Ball::Quick => ("quick", include_bytes!("../../assets/balls/quick.png")),
        Ball::Cherish => ("cherish", include_bytes!("../../assets/balls/cherish.png")),
    };
    let id = egui::Id::new(("ball-sprite", key));
    if let Some(t) = ctx.data(|d| d.get_temp::<Option<egui::TextureHandle>>(id)) {
        return t;
    }
    let t = image::load_from_memory(bytes).ok().map(|i| {
        let i = i.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw());
        ctx.load_texture(format!("ball-{key}"), ci, egui::TextureOptions::NEAREST)
    });
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// A Poké Ball item picture, `r` points in radius, turned by `tilt` radians.
pub(super) fn draw_ball(p: &egui::Painter, c: egui::Pos2, r: f32, tilt: f32, b: Ball) {
    let Some(tex) = ball_texture(p.ctx(), b) else { return draw_ball_drawn(p, c, r, tilt, b) };
    let h = r * 1.08;
    let (s, co) = tilt.sin_cos();
    let rot = |v: egui::Vec2| c + egui::vec2(v.x * co - v.y * s, v.x * s + v.y * co);
    let mut mesh = egui::Mesh::with_texture(tex.id());
    for (v, uv) in [(egui::vec2(-h, -h), egui::pos2(0.0, 0.0)), (egui::vec2(h, -h), egui::pos2(1.0, 0.0)), (egui::vec2(h, h), egui::pos2(1.0, 1.0)), (egui::vec2(-h, h), egui::pos2(0.0, 1.0))] {
        mesh.vertices.push(egui::epaint::Vertex { pos: rot(v), uv, color: Color32::WHITE });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
}

/// The drawn ball, if the picture can't be loaded.
fn draw_ball_drawn(p: &egui::Painter, c: egui::Pos2, r: f32, tilt: f32, b: Ball) {
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

/// A ball in one colour (a trainer's party: red for able, grey for fainted).
pub(super) fn draw_ball_color(p: &egui::Painter, c: egui::Pos2, r: f32, top: Color32, dim: bool) {
    let n = 20;
    let half = |from: f32| -> Vec<egui::Pos2> { (0..=n).map(|i| from + std::f32::consts::PI * i as f32 / n as f32).map(|a| c + egui::vec2(a.cos(), a.sin()) * r).collect() };
    let ink = if dim { MUTED() } else { Color32::from_rgb(30, 30, 34) };
    p.add(egui::Shape::convex_polygon(half(std::f32::consts::PI), top, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(half(0.0), if dim { kit::sunken() } else { Color32::from_rgb(245, 245, 245) }, Stroke::NONE));
    p.circle_stroke(c, r, Stroke::new(1.5f32, ink));
    p.line_segment([c - egui::vec2(r, 0.0), c + egui::vec2(r, 0.0)], Stroke::new(1.5f32, ink));
    p.circle_filled(c, r * 0.3, ink);
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

