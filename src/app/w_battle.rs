//! Dashboard widgets for the Team Planner, the Battle Sim, Breeding and
//! the Damage Calc. The team ones all show your "main team": the one you
//! pick in the Team switcher widget, or your newest.

use super::home::ago;
use super::team_analysis::{facts, weak_resist, Facts, BENCH};
use super::team_calc::{calc_mon, calc_move, default_set, Poke};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_build::{Head, Lead, Row};
use super::w_explore::loading;
use super::w_fit::{self as fit, wbox, Shape, WBox};
use super::w_kit::{self as wk, n};
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::battle::{self, World};
use crate::calc;
use crate::sim::{Format, Mode, Setup};
use crate::teams::{Member, Team};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};

/// One Pokémon's best hit on another: (move, min %, max %, KO words).
pub(super) struct BestHit {
    pub mv: String,
    pub lo: f32,
    pub hi: f32,
    pub ko: String,
}

/// The move that does the most to `def`, with both at their sets.
fn best_hit(wiki: Option<&crate::wiki::Wiki>, atk: &Member, def: &Member, world: World) -> Option<BestHit> {
    let am = calc_mon(&Poke::new(atk.clone(), None), world)?;
    let dm = calc_mon(&Poke::new(def.clone(), None), world)?;
    let f = calc::Field::default();
    let hp = dm.stats[0].max(1) as f32;
    let mut best: Option<BestHit> = None;
    for name in atk.moves.iter().take(4) {
        let Some(mv) = calc_move(wiki, name, atk, false) else { continue };
        if mv.cat == "Status" {
            continue;
        }
        let h = calc::damage(&am, &dm, &mv, &f, world);
        let (lo, hi) = (h.min() as f32 * 100.0 / hp, h.max() as f32 * 100.0 / hp);
        if best.as_ref().map_or(true, |b| hi > b.hi) {
            best = Some(BestHit { mv: mv.name.clone(), lo, hi, ko: calc::ko_text(&h, &dm) });
        }
    }
    best
}

/// Red for big hits, orange for middling, green for small.
fn hit_col(hi: f32) -> Color32 {
    if hi >= 100.0 {
        kit::C_RED
    } else if hi >= 50.0 {
        kit::C_ORANGE
    } else {
        theme::GREEN
    }
}

fn result_col(r: &str) -> Color32 {
    match r {
        "Won" | "Caught" => theme::GREEN,
        "Lost" => kit::C_RED,
        "" => kit::C_SLATE,
        _ => kit::C_ORANGE,
    }
}

impl App {
    /// The team the Dashboard shows: the one picked, or the newest.
    pub(super) fn dash_team(&mut self) -> Option<Team> {
        let want = self.cfg.home.main_team;
        let teams = &self.setup.team.teams().teams;
        want.and_then(|id| teams.iter().find(|t| t.id == id)).or_else(|| teams.iter().max_by_key(|t| t.updated)).cloned()
    }

    fn open_team_page(&mut self, id: u64) {
        self.setup.team.open_team(id);
        self.go_to(Tab::Team);
    }

    /// "Make a team first", with a button when there's room.
    fn no_team(&mut self, ui: &mut egui::Ui, b: &WBox, r: Rect) {
        let p = ui.painter_at(b.r);
        let (top, btn) = if r.height() > b.px(80.0) { fit::split_bottom(r, b.px(30.0), b.px(8.0)) } else { (r, Rect::NOTHING) };
        fit::empty(&p, b, top, "🛡", "Make a team in the Team Planner and it shows here.");
        if btn.is_positive() {
            let br = Rect::from_center_size(btn.center(), vec2(btn.width().min(b.px(220.0)), btn.height()));
            if fit::button(ui, br, "Open the Team Planner  ›", false, ("no-team", b.r.min.x as i32, b.r.min.y as i32)) {
                self.go_to(Tab::Team);
            }
        } else if ui.interact(r, ui.id().with(("no-team-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open the Team Planner").clicked() {
            self.go_to(Tab::Team);
        }
    }

    /// A row of sprites across `r`, as big as fits (up to `max` across).
    fn sprite_row(&mut self, p: &egui::Painter, r: Rect, mons: &[(String, bool)], max: f32) {
        if mons.is_empty() {
            return;
        }
        let n = mons.len() as f32;
        let side = (r.width() / n).min(r.height()).min(max);
        let x0 = r.center().x - side * n / 2.0;
        for (k, (sp, shiny)) in mons.iter().enumerate() {
            self.team_sprite(p, sp, *shiny, pos2(x0 + side * (k as f32 + 0.5), r.center().y), vec2(side, side));
        }
    }

    // ───────────────────────────── Team Planner ─────────────────────────────

    /// The main team's six, as big as the box allows.
    pub(super) fn w_mainteam(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "mainteam");
        let team = self.dash_team();
        let title = team.as_ref().map(|t| if t.name.trim().is_empty() { "Untitled team".to_string() } else { t.name.clone() }).unwrap_or_else(|| "Main team".into());
        let (body, mut open) = fit::head(ui, &b, "🛡", kit::C_BLUE, &title, team.as_ref().map(|_| "Open ›"));
        let Some(t) = team.clone() else { return self.no_team(ui, &b, body) };
        for (i, r) in fit::tiles(body, 6, 0.85, b.px(44.0), b.px(44.0), b.px(8.0)).iter().enumerate() {
            let p = ui.painter_at(*r);
            match t.members.get(i) {
                Some(m) => {
                    let types = battle::data().species(&m.species).map(|s| s.types_in(t.world).to_vec()).unwrap_or_default();
                    let col = types.first().map(|x| super::pokedex::type_color(x)).unwrap_or(kit::C_BLUE);
                    let resp = ui.interact(*r, ui.id().with(("mt", i, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    p.rect_filled(*r, b.px(12.0), theme::blend(CARD(), col, 0.08 + 0.08 * ht));
                    let text_h = if r.height() > b.px(104.0) { b.px(30.0) } else if r.height() > b.px(80.0) { b.px(16.0) } else { 0.0 };
                    let (pic, words) = fit::split_bottom(r.shrink(b.px(6.0)), text_h, 0.0);
                    let side = pic.width().min(pic.height());
                    self.team_sprite(&p, &m.species, m.shiny, pic.center(), vec2(side, side));
                    if text_h > 0.0 {
                        let parts = fit::stack(words, if text_h > b.px(20.0) { 2 } else { 1 }, 0.0);
                        fit::label(&p, parts[0], &m.display_name(), b.bold(11.5), TEXT(), Align2::CENTER_CENTER);
                        if parts.len() > 1 {
                            fit::label(&p, parts[1], &if m.item.is_empty() { format!("Lv {}", m.level) } else { m.item.clone() }, b.font(10.5), MUTED(), Align2::CENTER_CENTER);
                        }
                    }
                    if resp.on_hover_text(m.display_name()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = true;
                    }
                }
                None => {
                    p.rect_stroke(r.shrink(0.5), b.px(12.0), Stroke::new(1.0f32, theme::blend(LINE(), kit::C_BLUE, 0.15)));
                    p.text(r.center(), Align2::CENTER_CENTER, "+", b.bold(20.0), MUTED().gamma_multiply(0.5));
                }
            }
        }
        if open {
            self.open_team_page(t.id);
        }
    }

    fn team_facts(&self, t: &Team) -> Vec<Option<Facts>> {
        t.members.iter().map(|m| facts(self.wiki.as_deref(), t.world, m)).collect()
    }

    /// The types most of the team is weak to.
    pub(super) fn w_teamweak(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "teamweak");
        let team = self.dash_team();
        let Some(t) = team.filter(|t| !t.members.is_empty()) else {
            let (body, _) = fit::head(ui, &b, "⚠", kit::C_ORANGE, "Team weaknesses", None);
            return self.no_team(ui, &b, body);
        };
        let fs = self.team_facts(&t);
        let mut list: Vec<(&str, usize, usize)> = calc::types_in(t.world)
            .iter()
            .map(|at| {
                let (w, r) = weak_resist(&fs, at, t.world);
                (*at, w, r)
            })
            .filter(|x| x.1 >= 2 || (x.1 >= 1 && x.2 == 0 && t.members.len() <= 2))
            .collect();
        list.sort_by(|a, b| (b.1 as i32 - b.2 as i32).cmp(&(a.1 as i32 - a.2 as i32)));
        let rows: Vec<Row> = list.iter().map(|(at, weak, resist)| Row::new(*at).lead(Lead::Badge(at.to_uppercase(), super::pokedex::type_color(at))).right(format!("{weak} weak · {resist} resist"), Some(if *resist == 0 { kit::C_RED } else { MUTED() }))).collect();
        let title = format!("Weak spots · {}", if t.name.trim().is_empty() { "your team" } else { &t.name });
        let c = self.f_list(ui, &b, Head { icon: "⚠", col: kit::C_ORANGE, title: &title, link: None }, &rows, "✔  No type hits more than one of them hard.");
        if c.item.is_some() || c.more {
            self.open_team_page(t.id);
        }
    }

    /// Pick which team the Dashboard shows.
    pub(super) fn w_teamswitch(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "teamswitch");
        let teams = self.team_choices();
        let cur = self.dash_team().map(|t| t.id);
        if teams.is_empty() {
            let (body, _) = fit::head(ui, &b, "🔁", kit::C_BLUE, "Team switcher", Some("Teams ›"));
            return self.no_team(ui, &b, body);
        }
        let rows: Vec<Row> = teams
            .iter()
            .map(|(id, name, count)| {
                let on = Some(*id) == cur;
                Row::new(if name.trim().is_empty() { "Untitled team".to_string() } else { name.clone() })
                    .lead(Lead::Icon(if on { "✔".into() } else { "🛡".into() }, if on { kit::primary() } else { kit::C_SLATE }))
                    .sub(if on { "Shown on your Dashboard" } else { "" })
                    .right(format!("{count}/6"), Some(MUTED()))
                    .tip("Show this team in the team widgets")
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "🔁", col: kit::C_BLUE, title: "Team switcher", link: Some("Teams ›") }, &rows, "");
        if let Some(k) = c.item {
            self.cfg.home.main_team = Some(teams[k].0);
            self.dirty = true;
        }
        if c.link || c.more {
            self.go_to(Tab::Team);
        }
    }

    /// How hard the team's moves hit every type: a tile per type filling
    /// the box.
    pub(super) fn w_coverage(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "coverage");
        let team = self.dash_team();
        let (body, _) = fit::head(ui, &b, "🎯", kit::C_VIOLET, "Coverage check", None);
        let Some(t) = team.filter(|t| !t.members.is_empty()) else { return self.no_team(ui, &b, body) };
        let fs = self.team_facts(&t);
        let types = calc::types_in(t.world);
        let best: Vec<(&str, f32)> = types
            .iter()
            .map(|d| {
                let bv = fs.iter().flatten().flat_map(|f| f.attacks.iter()).map(|a| calc::eff(&a.1, &[d.to_string()], t.world)).fold(-1.0f32, f32::max);
                (*d, bv)
            })
            .collect();
        let se = best.iter().filter(|b| b.1 > 1.0).count();
        let walls: Vec<&str> = best.iter().filter(|b| b.1 >= 0.0 && b.1 < 1.0).map(|b| b.0).collect();
        let p = ui.painter_at(b.r);
        let (top, rest) = fit::split_top(body, b.px(17.0), b.px(6.0));
        fit::label(&p, top, &format!("Hits {se} of {} types super effectively", types.len()), b.bold(12.5), TEXT(), Align2::LEFT_CENTER);
        let (grid, foot) = if !walls.is_empty() && rest.height() > b.px(130.0) { fit::split_bottom(rest, b.px(15.0), b.px(6.0)) } else { (rest, Rect::NOTHING) };
        let cells = fit::tiles(grid, best.len(), 2.6, b.px(46.0), b.px(20.0), b.px(5.0));
        for (k, r) in cells.iter().enumerate() {
            let (d, bv) = best[k];
            let (label, col) = match bv {
                x if x < 0.0 => ("—", kit::C_SLATE),
                x if x >= 2.0 => ("2×", theme::GREEN),
                x if x > 1.0 => ("+", theme::GREEN),
                x if x == 1.0 => ("1×", kit::C_SLATE),
                x if x == 0.0 => ("0×", kit::C_RED),
                _ => ("½×", kit::C_ORANGE),
            };
            p.rect_filled(*r, b.px(7.0), theme::blend(CARD(), col, 0.18));
            p.rect_filled(Rect::from_min_size(r.min, vec2(b.px(4.0), r.height())), 2.0, super::pokedex::type_color(d));
            let ir = r.shrink2(vec2(b.px(8.0), 0.0));
            let short = if r.width() > b.px(90.0) { d.to_string() } else { d[..d.len().min(4)].to_string() };
            fit::label(&p, ir, &short, b.bold(11.0), TEXT(), Align2::LEFT_CENTER);
            fit::label(&p, ir, label, b.bold(11.5), super::charts::ink(col), Align2::RIGHT_CENTER);
            ui.interact(*r, ui.id().with(("cov", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::hover()).on_hover_text(format!("Best hit on {d}: {label}"));
        }
        if foot.is_positive() {
            fit::label(&p, foot, &format!("Nothing hits these hard, and something resists: {}", walls.join(", ")), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
    }

    /// A type that would cover the team's biggest gaps, with examples.
    pub(super) fn w_teamidea(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "teamidea");
        let team = self.dash_team();
        let (body, _) = fit::head(ui, &b, "💭", kit::C_TEAL, "Team ideas", None);
        let Some(t) = team.filter(|t| !t.members.is_empty()) else { return self.no_team(ui, &b, body) };
        let p = ui.painter_at(b.r);
        let fs = self.team_facts(&t);
        let types = calc::types_in(t.world);
        let mut gaps: Vec<(&str, usize)> = types.iter().map(|at| (*at, weak_resist(&fs, at, t.world))).filter(|(_, (w, r))| *w >= 2 && *w > *r).map(|(a, (w, _))| (a, w)).collect();
        gaps.sort_by(|a, b| b.1.cmp(&a.1));
        gaps.truncate(3);
        if gaps.is_empty() {
            fit::empty(&p, &b, body, "✔", "No big gaps: no type hits most of them hard.");
            return;
        }
        let best = types.iter().map(|d| (*d, gaps.iter().filter(|(a, _)| calc::eff(a, &[d.to_string()], t.world) < 1.0).count())).max_by_key(|x| x.1).filter(|x| x.1 > 0);
        let Some((ty, covers)) = best else {
            fit::empty(&p, &b, body, "💭", "No one type covers those. Try a Pokémon with a resisting Ability.");
            return;
        };
        let names: Vec<&str> = gaps.iter().filter(|(a, _)| calc::eff(a, &[ty.to_string()], t.world) < 1.0).map(|g| g.0).collect();
        // the idea, why, then examples filling the rest
        let (line, rest) = fit::split_top(body, b.px(24.0), b.px(4.0));
        let lg = p.layout_no_wrap("Add a".to_string(), b.font(13.0), TEXT());
        let lw = lg.size().x;
        p.galley(pos2(line.min.x, line.center().y - lg.size().y / 2.0), lg, TEXT());
        let tag = wk::type_tag(&p, pos2(line.min.x + lw + b.px(6.0), line.center().y), ty, "");
        p.text(pos2(tag.max.x + b.px(6.0), line.center().y), Align2::LEFT_CENTER, "type", b.font(13.0), TEXT());
        let (why, pics) = if rest.height() > b.px(60.0) { fit::split_top(rest, b.px(30.0), b.px(6.0)) } else { (rest, Rect::NOTHING) };
        fit::note(&p, why, &format!("It takes little from {} ({covers} of your team's weak spots).", names.join(", ")), b.font(11.5));
        let mut open: Option<String> = None;
        if pics.is_positive() {
            let d = battle::data();
            let mut picks: Vec<&battle::Species> = d.species.iter().filter(|s| s.num >= 1 && s.num <= 493 && !s.is_mega() && s.base.is_empty() && s.types_in(t.world).iter().any(|x| x == ty) && (t.world != World::Platinum || !s.modern_only()) && !t.members.iter().any(|m| battle::to_id(&m.species) == s.id)).collect();
            picks.sort_by_key(|s| std::cmp::Reverse(s.bst(t.world)));
            let wiki = self.wiki.clone();
            let cells = fit::tiles(pics, picks.len().min(12), 0.9, b.px(54.0), b.px(54.0), b.px(6.0));
            for (k, r) in cells.iter().enumerate() {
                let sp = picks[k];
                let resp = ui.interact(*r, ui.id().with(("idea", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
                p.rect_filled(*r, b.px(10.0), theme::blend(CARD(), super::pokedex::type_color(ty), if resp.hovered() { 0.18 } else { 0.1 }));
                let with_name = r.height() > b.px(74.0);
                let (pic, name_r) = if with_name { fit::split_bottom(r.shrink(b.px(4.0)), b.px(15.0), 0.0) } else { (r.shrink(b.px(4.0)), Rect::NOTHING) };
                let side = pic.width().min(pic.height());
                self.team_sprite(&p, &sp.name, false, pic.center(), vec2(side, side));
                if with_name {
                    fit::label(&p, name_r, &sp.name, b.bold(11.0), TEXT(), Align2::CENTER_CENTER);
                }
                if resp.on_hover_text(format!("{}: its Pokédex page", sp.name)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = wiki.as_ref().and_then(|w| w.by_dex(sp.num)).map(|m| m.id.clone());
                }
            }
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    // ───────────────────────────── Battle Sim ─────────────────────────────

    /// Open the Battle Sim and start this setup.
    fn sim_go(&mut self, s: Setup) {
        self.setup.sim.file();
        self.setup.sim.started = true;
        self.setup.sim.view = super::sim_home::SimView::Battle;
        self.setup.sim.setup = s;
        self.go_to(Tab::Sim);
        self.sim_start();
    }

    /// How the last battle went, with a rematch.
    pub(super) fn w_lastbattle(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "lastbattle");
        let last = self.setup.sim.file().history.first().cloned();
        let (body, mut open) = fit::head(ui, &b, "⚔", kit::C_RED, "Last battle", Some("Battle Sim ›"));
        let p = ui.painter_at(b.r);
        let Some(lb) = last else {
            fit::empty(&p, &b, body, "⚔", "Battle in the Battle Sim and how it went shows here.");
            if open {
                self.go_to(Tab::Sim);
            }
            return;
        };
        let res = if lb.result.is_empty() { "Left early".to_string() } else { lb.result.clone() };
        let col = result_col(&lb.result);
        let mut want = vec![(b.px(20.0), b.px(22.0)), (b.px(17.0), b.px(19.0))];
        let room = body.height();
        let with_sub = room > b.px(80.0);
        if with_sub {
            want.push((b.px(14.0), b.px(16.0)));
        }
        let with_mons = room > b.px(120.0);
        if with_mons {
            want.push((b.px(36.0), b.px(80.0)));
        }
        let with_btn = room > b.px(150.0);
        if with_btn {
            want.push((b.px(30.0), b.px(32.0)));
        }
        let fl = fit::vflow(body, &want, b.px(5.0));
        let g = p.layout_no_wrap(res.to_uppercase(), b.bold(11.0), Color32::WHITE);
        let pill = Rect::from_min_size(pos2(fl[0].min.x, fl[0].center().y - (g.size().y + b.px(8.0)) / 2.0), g.size() + vec2(b.px(16.0), b.px(8.0)));
        p.rect_filled(pill, pill.height() / 2.0, col);
        p.galley(pill.min + vec2(b.px(8.0), b.px(4.0)), g, Color32::WHITE);
        fit::label(&p, Rect::from_min_max(pos2(pill.max.x + b.px(8.0), fl[0].min.y), fl[0].max), &ago(lb.when), b.font(11.5), MUTED(), Align2::LEFT_CENTER);
        fit::label(&p, fl[1], &lb.title, b.bold(14.0), TEXT(), Align2::LEFT_CENTER);
        let mut k = 2;
        if with_sub {
            fit::label(&p, fl[k], &format!("{} · {} turns", lb.team, lb.turns), b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            k += 1;
        }
        if with_mons {
            let (l, r) = fit::split_left(fl[k], fl[k].width() * 0.45, b.px(4.0));
            let (mid, r) = fit::split_left(r, b.px(24.0).min(r.width() * 0.2), b.px(4.0));
            self.sprite_row(&p, l, &lb.ours.iter().take(3).cloned().collect::<Vec<_>>(), b.px(80.0));
            p.text(mid.center(), Align2::CENTER_CENTER, "vs", b.bold(12.0), MUTED());
            self.sprite_row(&p, r, &lb.theirs.iter().take(3).cloned().collect::<Vec<_>>(), b.px(80.0));
            k += 1;
        }
        let mut again = false;
        if with_btn {
            let br = Rect::from_min_size(fl[k].min, vec2(fl[k].width().min(b.px(160.0)), fl[k].height()));
            again = fit::button(ui, br, "↻  Rematch", true, ("rematch", b.r.min.x as i32, b.r.min.y as i32));
        } else if ui.interact(body, ui.id().with(("lb-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open the Battle Sim").clicked() {
            open = true;
        }
        if again {
            self.sim_go(lb.setup);
        } else if open {
            self.go_to(Tab::Sim);
        }
    }

    /// The share of battles won, as a big number and a bar.
    pub(super) fn w_winrecord(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "winrecord");
        let hist = self.setup.sim.file().history.clone();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🏆", kit::C_GOLD, "Win record", None) };
        let p = ui.painter_at(b.r);
        let played: Vec<&crate::sim::BattleRecord> = hist.iter().filter(|b| !b.result.is_empty()).collect();
        if played.is_empty() {
            fit::empty(&p, &b, body, "🏆", "Finish a battle in the Battle Sim and your record starts here.");
            return;
        }
        let won = played.iter().filter(|b| b.result == "Won" || b.result == "Caught").count();
        let lost = played.iter().filter(|b| b.result == "Lost").count();
        let other = played.len() - won - lost;
        let rate = won as f32 / played.len() as f32;
        let mut by: Vec<(String, usize)> = Vec::new();
        for r in played.iter().filter(|b| b.result == "Won") {
            match by.iter_mut().find(|x| x.0 == r.team) {
                Some(x) => x.1 += 1,
                None => by.push((r.team.clone(), 1)),
            }
        }
        let best = by.into_iter().max_by_key(|x| x.1);
        let mut want = vec![(b.px(22.0), b.px(64.0)), (b.px(8.0), b.px(14.0))];
        if body.height() > b.px(70.0) {
            want.push((b.px(14.0), b.px(16.0)));
        }
        if best.is_some() && body.height() > b.px(100.0) {
            want.push((b.px(15.0), b.px(17.0)));
        }
        let fl = fit::vflow(body, &want, b.px(5.0));
        let at = fit::hero(&p, Rect::from_min_max(fl[0].min, pos2(fl[0].max.x - b.px(80.0), fl[0].max.y)), &format!("{:.0}%", rate * 100.0), fl[0].height(), TEXT(), Align2::LEFT_CENTER);
        p.text(pos2(at.max.x + b.px(6.0), at.max.y - at.height() * 0.18), Align2::LEFT_BOTTOM, format!("won of {}", played.len()), b.font(12.0), MUTED());
        let r = fl[1];
        let tot = played.len() as f32;
        let mut x = r.min.x;
        for (k, col) in [(won, theme::GREEN), (lost, kit::C_RED), (other, kit::C_ORANGE)] {
            let wv = r.width() * k as f32 / tot;
            if wv > 0.0 {
                p.rect_filled(Rect::from_min_size(pos2(x, r.min.y), vec2(wv, r.height())), r.height() / 2.0, col);
            }
            x += wv;
        }
        if want.len() > 2 {
            fit::label(&p, fl[2], &format!("{won} won · {lost} lost · {other} ran or other"), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if let (Some((team, k)), true) = (best, want.len() > 3) {
            fit::label(&p, fl[3], &format!("🏅  Best team: {team} ({k} win{})", if k == 1 { "" } else { "s" }), b.font(12.0), TEXT(), Align2::LEFT_CENTER);
        }
    }

    /// One click: your main team against a random wild Pokémon.
    pub(super) fn w_quickbattle(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "quickbattle");
        let team = self.dash_team();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "⚡", kit::C_GOLD, "Quick battle", None) };
        let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui, &b, body) };
        let p = ui.painter_at(b.r);
        let name = if t.name.trim().is_empty() { "Your team".to_string() } else { t.name.clone() };
        // the line, the team and a "?", and the button
        let (main, btn) = if b.flat() { fit::split_right(body, (body.width() * 0.32).min(b.px(150.0)), b.px(10.0)) } else { fit::split_bottom(body, b.px(32.0), b.px(8.0)) };
        let mut want = vec![(b.px(17.0), b.px(19.0))];
        if main.height() > b.px(64.0) {
            want.push((b.px(13.0), b.px(15.0)));
        }
        if main.height() > b.px(40.0) {
            want.push((b.px(24.0), b.px(90.0)));
        }
        let fl = fit::vflow(main, &want, b.px(4.0));
        fit::label(&p, fl[0], &format!("{name} vs a random wild Pokémon"), b.bold(13.0), TEXT(), Align2::LEFT_CENTER);
        if want.len() == 3 {
            fit::label(&p, fl[1], "From anywhere in Sinnoh, at its own level.", b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if want.len() > 1 {
            let mr = *fl.last().unwrap_or(&main);
            let (mons, q) = fit::split_right(mr, mr.height().min(b.px(60.0)), b.px(6.0));
            self.sprite_row(&p, mons, &t.members.iter().take(6).map(|m| (m.species.clone(), m.shiny)).collect::<Vec<_>>(), b.px(70.0));
            p.text(q.center(), Align2::CENTER_CENTER, "?", b.bold((q.height() / b.s() * 0.5).clamp(14.0, 34.0)), kit::C_GOLD);
        }
        let br = Rect::from_center_size(btn.center(), vec2(btn.width().min(b.px(180.0)), b.px(32.0).min(btn.height())));
        let go = fit::button(ui, br, "⚡  Battle!", true, ("qb", b.r.min.x as i32, b.r.min.y as i32));
        if !go {
            return;
        }
        let Some(w) = self.wiki.clone() else { return };
        let slots: Vec<&crate::wiki::Wild> = w.data.wild.iter().filter(|x| x.setting.is_empty() && battle::data().species(&x.species).is_some()).collect();
        if slots.is_empty() {
            return;
        }
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize ^ d.as_secs() as usize).unwrap_or(7);
        let x = slots[seed.wrapping_mul(2_654_435_761) % slots.len()];
        let level: u8 = x.levels.split(|c: char| !c.is_ascii_digit()).find(|s| !s.is_empty()).and_then(|s| s.parse().ok()).unwrap_or(20);
        let mut m = Member { level, ..Member::new(&x.species, t.world) };
        m.moves = super::sim_foes::wild_set(&w, &x.species, level, t.world);
        let place = w.place(&x.area).map(|p| p.name.clone()).unwrap_or_default();
        let s = Setup { mode: Mode::Wild, format: Format::Singles, team: Some(t.id), picks: (0..t.members.len().min(6)).collect(), foes: vec![m], place, method: x.method.clone(), world: t.world, ..Default::default() };
        self.sim_go(s);
    }

    /// Trainers you made: click one to battle it with your main team.
    pub(super) fn w_trainers(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "trainers");
        let trainers = self.setup.sim.file().trainers.clone();
        let team = self.dash_team();
        let ok_team = team.as_ref().is_some_and(|t| !t.members.is_empty());
        let rows: Vec<Row> = trainers
            .iter()
            .map(|tr| {
                let title = if tr.class.is_empty() { tr.name.clone() } else { format!("{} {}", tr.class, tr.name) };
                let ok = ok_team && !tr.members.is_empty();
                Row::new(title)
                    .lead(tr.members.first().map_or(Lead::Icon("🎖".into(), kit::C_ORANGE), |m| Lead::Wiki(m.species.clone(), m.shiny)))
                    .sub(format!("{} Pokémon{}", tr.members.len(), if tr.doubles { " · doubles" } else { "" }))
                    .right(if ok { "Battle ›" } else { "" }, Some(kit::primary()))
                    .tip(if ok { "Battle this trainer with your main team" } else { "Needs a team with Pokémon on both sides" })
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "🎖", col: kit::C_ORANGE, title: "Your trainers", link: Some("Battle Sim ›") }, &rows, "Make trainers to battle (a gym leader, a rival…) in the Battle Sim and they show here.");
        if let (Some(k), Some(t)) = (c.item, team) {
            let tr = &trainers[k];
            if !t.members.is_empty() && !tr.members.is_empty() {
                let doubles = tr.doubles && t.members.len() >= 2;
                let s = Setup { mode: Mode::Trainer, format: if doubles { Format::Doubles } else { Format::Singles }, team: Some(t.id), picks: (0..t.members.len().min(6)).collect(), trainer: Some(tr.id), world: t.world, ..Default::default() };
                self.sim_go(s);
            }
        } else if c.link || c.more {
            self.go_to(Tab::Sim);
        }
    }

    /// Your battles, newest first.
    pub(super) fn w_battlehistory(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "battlehistory");
        let hist: Vec<crate::sim::BattleRecord> = self.setup.sim.file().history.iter().take(40).cloned().collect();
        let rows: Vec<Row> = hist.iter().map(|r| Row::new(r.title.clone()).lead(Lead::Badge(if r.result.is_empty() { "—".into() } else { r.result.clone() }, result_col(&r.result))).sub(format!("{} · {} turns", r.team, r.turns)).right(ago(r.when), Some(MUTED()))).collect();
        let c = self.f_list(ui, &b, Head { icon: "📜", col: kit::C_SLATE, title: "Battle history", link: Some("All ›") }, &rows, "Your battles show here once you've played some in the Battle Sim.");
        if c.link || c.item.is_some() || c.more {
            self.setup.sim.file();
            self.setup.sim.view = super::sim_home::SimView::Battle;
            self.setup.sim.drawer = 4;
            self.go_to(Tab::Sim);
        }
    }

    // ───────────────────────────── Breeding ─────────────────────────────

    fn open_breed(&mut self, v: super::breed_home::BreedView) {
        self.setup.breed.view = v;
        self.go_to(Tab::Breed);
    }

    /// The Pokémon you breed with, newest first.
    pub(super) fn w_breeders(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "breeders");
        let breeders = self.setup.breed.file().breeders.clone();
        let rows: Vec<Row> = breeders
            .iter()
            .rev()
            .map(|br| {
                let name = if br.name.trim().is_empty() { self.mon_name(&br.parent.species) } else { br.name.clone() };
                let g = match br.parent.gender() {
                    crate::breed::Gender::Male => " ♂",
                    crate::breed::Gender::Female => " ♀",
                    crate::breed::Gender::Genderless => "",
                };
                let perfect = br.parent.ivs.iter().filter(|iv| **iv == Some(31)).count();
                Row::new(format!("{name}{g}")).lead(Lead::Wiki(br.parent.species.clone(), false)).sub(br.parent.nature.clone().unwrap_or_else(|| "Any nature".into())).right(format!("{perfect}× 31"), Some(if perfect > 0 { kit::C_PINK } else { MUTED() }))
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "📦", col: kit::C_PINK, title: "Breeders box", link: Some("All ›") }, &rows, "Add the Pokémon you breed with in Breeding → Breeders and they show here.");
        if c.link || c.item.is_some() || c.more {
            self.open_breed(super::breed_home::BreedView::Breeders);
        }
    }

    /// The egg moves a Pokémon you pick can get.
    pub(super) fn w_eggmoves(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "eggmoves");
        let species = if preview { Some("GIBLE".to_string()) } else { w.data.first().cloned() };
        let (body, change) = fit::head(ui, &b, "💥", kit::C_PINK, "Egg moves", species.as_ref().map(|_| "Change"));
        let p = ui.painter_at(b.r);
        let Some(wiki) = self.wiki.clone() else { return loading(&p, &b, body, ui) };
        let modern = self.setup.breed.file().setup.world != World::Platinum;
        let mut open: Option<String> = None;
        let Some(id) = species else {
            let (note_r, rest) = fit::split_top(body, b.px(16.0), b.px(6.0));
            fit::label(&p, note_r, "Pick a Pokémon to see the moves it can get from an Egg.", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(rest.intersect(ui.clip_rect()));
            if let Some(pk) = self.mon_picker(&mut child, w.id, "egg", "🔍  Gible, Eevee…") {
                *out = Some(vec![pk]);
            }
            return;
        };
        let baby = crate::breed::baby_of(&wiki, &id);
        let flat = body.height() < b.px(70.0);
        let (who, rest) = if flat { fit::split_left(body, (body.width() * 0.3).min(b.px(170.0)), b.px(10.0)) } else { fit::split_top(body, b.px(36.0), b.px(6.0)) };
        let (sp_r, name_r) = fit::split_left(who, who.height().min(b.px(40.0)), b.px(8.0));
        self.team_sprite(&p, &baby, false, sp_r.center(), vec2(sp_r.width(), sp_r.width()));
        fit::label(&p, name_r, &self.mon_name(&baby), b.bold(14.0), TEXT(), Align2::LEFT_CENTER);
        let moves: Vec<String> = wiki.learnset(&baby).map(|l| if modern && !l.megg.is_empty() { l.megg.clone() } else { l.egg.clone() }).unwrap_or_default();
        if moves.is_empty() {
            fit::label(&p, rest, "No egg moves.", b.font(11.5), MUTED(), Align2::LEFT_TOP);
        }
        // pills that wrap in the room left (the rest are on its page)
        let clip = p.with_clip_rect(rest);
        let (mut x, mut y) = (rest.min.x, rest.min.y);
        let h = b.px(22.0);
        for (k, key) in moves.iter().enumerate() {
            let Some(mv) = wiki.move_info(key) else { continue };
            let col = super::pokedex::type_color(&mv.t);
            let g = clip.layout_no_wrap(mv.n.clone(), b.bold(11.0), TEXT());
            let w_ = g.size().x + b.px(20.0);
            if x + w_ > rest.max.x && x > rest.min.x {
                x = rest.min.x;
                y += h + b.px(5.0);
            }
            if y + h > rest.max.y {
                break;
            }
            let r = Rect::from_min_size(pos2(x, y), vec2(w_, h));
            let resp = ui.interact(r, ui.id().with(("egg-mv", k, w.id)), Sense::click());
            clip.rect_filled(r, h / 2.0, theme::blend(CARD(), col, if resp.hovered() { 0.32 } else { 0.22 }));
            clip.circle_filled(pos2(r.min.x + b.px(8.0), r.center().y), b.px(3.0), col);
            clip.galley(pos2(r.min.x + b.px(14.0), r.center().y - g.size().y / 2.0), g, TEXT());
            if resp.on_hover_text(format!("{} · {} · open its page", mv.t, mv.c)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = Some(key.clone());
            }
            x += w_ + b.px(5.0);
        }
        if change && !preview {
            *out = Some(Vec::new());
        }
        if let Some(k) = open {
            self.info_open_page(super::info_home::Page::Move(k));
        }
    }

    /// The next thing to do in your breeding plan, with a tick.
    pub(super) fn w_nextstep(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "nextstep");
        let (body, open) = fit::head(ui, &b, "👣", kit::C_TEAL, "Next step", Some("Plan ›"));
        let p = ui.painter_at(b.r);
        let Some(wiki) = self.wiki.clone() else { return loading(&p, &b, body, ui) };
        let (target, breeders, setup, done) = {
            let f = self.setup.breed.file();
            (f.target.clone(), f.breeders.clone(), f.setup, f.plan_done.clone())
        };
        let mut tick: Option<String> = None;
        'draw: {
            if target.species.is_empty() {
                fit::empty(&p, &b, body, "👣", "Pick what you're breeding for in Breeding → Plan and its next step shows here.");
                break 'draw;
            }
            let key = format!("{:?}|{:?}|{:?}", target, setup, breeders.iter().map(|b| (&b.parent, b.id)).collect::<Vec<_>>());
            // quick now, with the Egg estimate once it's worked out
            if self.setup.home.plan.as_ref().map(|p| &p.0) != Some(&key) {
                let plan = crate::breed_plan::make_plan(&wiki, &target, &breeders, &setup, 0);
                self.setup.home.plan = Some((key.clone(), plan));
                let (tx, rx) = std::sync::mpsc::channel();
                let (w2, t2, b2) = (wiki.clone(), target.clone(), breeders.clone());
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let _ = tx.send(crate::breed_plan::make_plan(&w2, &t2, &b2, &setup, 120));
                    ctx.request_repaint();
                });
                self.setup.home.plan_rx = Some((key.clone(), rx));
            }
            if let Some((k, rx)) = &self.setup.home.plan_rx {
                if let Ok(pl) = rx.try_recv() {
                    if *k == key {
                        self.setup.home.plan = Some((key.clone(), pl));
                    }
                    self.setup.home.plan_rx = None;
                }
            }
            let Some((_, plan)) = self.setup.home.plan.clone() else { break 'draw };
            if let Some(pr) = &plan.problem {
                fit::note(&p, body, pr, b.font(12.0));
                break 'draw;
            }
            let total = plan.steps.len();
            let finished = plan.steps.iter().filter(|s| s.done || done.contains(&s.key)).count();
            let next = plan.steps.iter().find(|s| !s.done && !done.contains(&s.key)).cloned();
            // who and how far, the bar, the step (its words take what's left), the tick
            let with_btn = next.is_some() && body.height() > b.px(110.0);
            let mut want = vec![(b.px(26.0), b.px(34.0)), (b.px(6.0), b.px(8.0)), (b.px(17.0), b.px(20.0))];
            if next.is_some() && body.height() > b.px(90.0) {
                want.push((b.px(14.0), body.height()));
            }
            if with_btn {
                want.push((b.px(30.0), b.px(32.0)));
            }
            let fl = fit::vflow(body, &want, b.px(5.0));
            let (sp_r, line_r) = fit::split_left(fl[0], fl[0].height(), b.px(8.0));
            self.team_sprite(&p, &target.species, target.shiny, sp_r.center(), vec2(sp_r.width(), sp_r.width()));
            fit::label(&p, line_r, &format!("{} · step {} of {total}", self.mon_name(&target.species), (finished + 1).min(total)), b.bold(13.0), TEXT(), Align2::LEFT_CENTER);
            fit::meter(&p, fl[1], if total > 0 { finished as f32 / total as f32 } else { 1.0 }, kit::C_TEAL);
            match &next {
                Some(s) => {
                    fit::label(&p, fl[2], &s.title, b.bold(14.0), TEXT(), Align2::LEFT_CENTER);
                    if want.len() > 3 {
                        fit::note(&p, fl[3], &s.detail, b.font(12.0));
                    }
                    if with_btn {
                        let br = Rect::from_min_size(fl[fl.len() - 1].min, vec2(fl[fl.len() - 1].width().min(b.px(140.0)), fl[fl.len() - 1].height()));
                        if fit::button(ui, br, "✔  Done", true, ("ns-done", b.r.min.x as i32, b.r.min.y as i32)) {
                            tick = Some(s.key.clone());
                        }
                    }
                }
                None => {
                    fit::label(&p, fl[2], "✔  Every step done. Hatch away!", b.font(13.0), theme::GREEN, Align2::LEFT_CENTER);
                }
            }
        }
        if let Some(k) = tick {
            let f = self.setup.breed.file();
            if !f.plan_done.contains(&k) {
                f.plan_done.push(k);
            }
            self.setup.breed.touch();
        }
        if open {
            self.open_breed(super::breed_home::BreedView::Plan);
        }
    }

    /// Eggs hatched, with the week as bars filling the rest.
    pub(super) fn w_hatches(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "hatches");
        let log = self.setup.breed.file().log.clone();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, mut open) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🐥", kit::C_GOLD, "Hatch tracker", Some("Log ›")) };
        let p = ui.painter_at(b.r);
        if log.is_empty() {
            fit::empty(&p, &b, body, "🐥", "Log your Eggs in Breeding and your hatches add up here.");
        } else {
            let today = crate::stats::local_day(crate::config::now_unix());
            let shinies = log.iter().filter(|h| h.shiny).count();
            let week: Vec<u64> = (0..7).map(|k| log.iter().filter(|h| crate::stats::local_day(h.when) == today - 6 + k).count() as u64).collect();
            let labels: Vec<String> = (0..7).map(|k| crate::stats::WEEKDAYS[crate::stats::weekday(today - 6 + k)][..1].to_string()).collect();
            let side = b.flat() || body.width() > body.height() * 2.2;
            let (num_r, chart) = if side { fit::split_left(body, (body.width() * 0.38).min(b.px(200.0)), b.px(10.0)) } else { fit::split_top(body, (body.height() * 0.4).clamp(b.px(36.0), b.px(70.0)), b.px(6.0)) };
            let fl = fit::vflow(num_r, &[(b.px(20.0), b.px(44.0)), (b.px(13.0), b.px(15.0))], b.px(2.0));
            let at = fit::hero(&p, Rect::from_min_max(fl[0].min, pos2(fl[0].max.x - b.px(70.0), fl[0].max.y)), &n(log.len() as u64), fl[0].height(), TEXT(), Align2::LEFT_CENTER);
            p.text(pos2(at.max.x + b.px(6.0), at.max.y - at.height() * 0.18), Align2::LEFT_BOTTOM, "Eggs hatched", b.font(11.5), MUTED());
            fit::label(&p, fl[1], &format!("✨ {shinies} shin{} · {} today", if shinies == 1 { "y" } else { "ies" }, week[6]), b.font(11.5), if shinies > 0 { theme::GOLD } else { MUTED() }, Align2::LEFT_CENTER);
            fit::bars(&p, chart, &week, Some(6), kit::C_GOLD, Some(&labels), b.font(10.0));
        }
        if small && ui.interact(b.r, ui.id().with(("hatch-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open the hatch log").clicked() {
            open = true;
        }
        if open {
            self.open_breed(super::breed_home::BreedView::Log);
        }
    }

    // ───────────────────────────── Damage Calc ─────────────────────────────

    /// Open the Damage Calc with these two.
    fn calc_open(&mut self, atk: &str, def: &str, world: World) {
        let c = &mut self.setup.team.calc;
        c.left = Some(Poke::new(default_set(atk, world), None));
        c.right = Some(Poke::new(default_set(def, world), None));
        self.go_to(Tab::Calc);
    }

    /// Two Pokémon side by side (with a swap button between them when
    /// `swap`), as big as `r` allows. Returns which side was clicked (1 or
    /// 2) and whether swap was.
    fn calc_pair(&mut self, ui: &mut egui::Ui, b: &WBox, r: Rect, a: &Member, d: &Member, editing: u8, swap: bool, id: u64) -> (u8, bool) {
        let p = ui.painter_at(r);
        let mid_w = if swap { b.px(36.0) } else { b.px(24.0) };
        let (l, rest) = fit::split_left(r, (r.width() - mid_w) / 2.0, 0.0);
        let (mid, rr) = fit::split_left(rest, mid_w, 0.0);
        let mut side = 0;
        for (k, (m, br)) in [(a, l), (d, rr)].into_iter().enumerate() {
            let br = br.shrink2(vec2(b.px(2.0), 0.0));
            let resp = ui.interact(br, ui.id().with(("qc-side", id, k)), Sense::click());
            let on = editing == k as u8 + 1;
            p.rect_filled(br, b.px(12.0), theme::blend(CARD(), if k == 0 { kit::C_ORANGE } else { kit::C_BLUE }, if on || resp.hovered() { 0.2 } else { 0.1 }));
            let with_name = br.height() > b.px(56.0);
            let (pic, name_r) = if with_name { fit::split_bottom(br.shrink(b.px(4.0)), b.px(15.0), 0.0) } else { (br.shrink(b.px(4.0)), Rect::NOTHING) };
            let s = pic.width().min(pic.height());
            self.team_sprite(&p, &m.species, false, pic.center(), vec2(s, s));
            if with_name {
                fit::label(&p, name_r, &m.species, b.bold(11.5), TEXT(), Align2::CENTER_CENTER);
            }
            if swap && resp.on_hover_text("Change").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                side = if on { 3 } else { k as u8 + 1 };
            }
        }
        let mut swapped = false;
        if swap {
            let sw = Rect::from_center_size(mid.center(), vec2(b.px(30.0), b.px(30.0)));
            let sresp = ui.interact(sw, ui.id().with(("qc-swap", id)), Sense::click());
            let ht = kit::anim(ui.ctx(), sresp.id.with("h"), sresp.hovered(), kit::QUICK);
            p.circle_filled(sw.center(), sw.width() / 2.0, theme::blend(kit::btn_fill(), kit::primary(), 0.3 * ht));
            p.circle_stroke(sw.center(), sw.width() / 2.0, Stroke::new(1.0f32, LINE()));
            // two arrows, drawn
            let st = Stroke::new(1.6f32, TEXT());
            let c = sw.center();
            let k = sw.width() / 30.0;
            for (y, dir) in [(-4.0, 1.0), (4.0, -1.0)] {
                let a = c + vec2(-7.0 * dir, y) * k;
                let e = c + vec2(7.0 * dir, y) * k;
                p.line_segment([a, e], st);
                p.line_segment([e, e + vec2(-3.5 * dir, -3.0) * k], st);
                p.line_segment([e, e + vec2(-3.5 * dir, 3.0) * k], st);
            }
            swapped = sresp.on_hover_text("Swap sides").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        } else {
            p.text(mid.center(), Align2::CENTER_CENTER, "vs", b.bold(12.0), MUTED());
        }
        (side, swapped)
    }

    /// Each side's best hit on the other, as lines down `r`.
    fn calc_lines(&self, p: &egui::Painter, b: &WBox, r: Rect, a: &Member, d: &Member, world: World) {
        let wiki = self.wiki.clone();
        let parts = fit::stack(r, 2, b.px(6.0));
        for ((x, y), pr) in [(a, d), (d, a)].into_iter().zip(parts.iter()) {
            let with_ko = pr.height() > b.px(32.0);
            let (top, ko) = if with_ko { fit::split_top(*pr, pr.height() * 0.55, 0.0) } else { (*pr, Rect::NOTHING) };
            match best_hit(wiki.as_deref(), x, y, world) {
                Some(h) => {
                    let pct = format!("{:.0}–{:.0}%", h.lo, h.hi);
                    let g = p.layout_no_wrap(pct.clone(), b.bold(13.0), super::charts::ink(hit_col(h.hi)));
                    let pr2 = Rect::from_min_max(pos2(top.max.x - g.size().x, top.min.y), top.max);
                    p.galley(pos2(pr2.min.x, top.center().y - g.size().y / 2.0), g, super::charts::ink(hit_col(h.hi)));
                    fit::label(p, Rect::from_min_max(top.min, pos2(pr2.min.x - b.px(8.0), top.max.y)), &format!("{}'s {}", x.display_name(), h.mv), b.font(12.0), TEXT(), Align2::LEFT_CENTER);
                    if with_ko && !h.ko.is_empty() {
                        fit::label(p, ko, &h.ko, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                    }
                }
                None => {
                    fit::label(p, top, &format!("{} has no damaging moves.", x.display_name()), b.font(11.5), MUTED(), Align2::LEFT_CENTER);
                }
            }
        }
    }

    /// One Pokémon's best hit on another, with their usual sets.
    pub(super) fn w_quickcalc(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "quickcalc");
        let world = World::Platinum;
        let (atk, def) = if preview || w.data.len() < 2 { ("Garchomp".to_string(), "Metagross".to_string()) } else { (w.data[0].clone(), w.data[1].clone()) };
        let editing = ui.ctx().data(|d| d.get_temp::<u8>(egui::Id::new(("w-qc-edit", w.id)))).unwrap_or(0);
        let wiki = self.wiki.clone();
        let (body, open) = fit::head(ui, &b, "🔢", kit::C_ORANGE, "Quick calc", Some("Damage Calc ›"));
        let p = ui.painter_at(b.r);
        let a = default_set(&atk, world);
        let d = default_set(&def, world);
        let mut new: Option<(String, String)> = None;
        let mut set_edit: Option<u8> = None;
        // the two at the top (beside the results when it's wide), the save link at the bottom
        let wide = body.width() > body.height() * 2.4;
        let (main, foot) = if body.height() > b.px(110.0) { fit::split_bottom(body, b.px(16.0), b.px(6.0)) } else { (body, Rect::NOTHING) };
        let (pair_r, rest) = if wide { fit::split_left(main, (main.width() * 0.42).min(main.height() * 2.6), b.px(12.0)) } else { fit::split_top(main, (main.height() * 0.45).clamp(b.px(44.0), b.px(120.0)), b.px(8.0)) };
        let (side, swapped) = self.calc_pair(ui, &b, pair_r, &a, &d, editing, !preview, w.id);
        if side > 0 {
            set_edit = Some(if side == 3 { 0 } else { side });
        }
        if swapped {
            new = Some((def.clone(), atk.clone()));
        }
        if editing > 0 && !preview {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(rest.intersect(ui.clip_rect()));
            if let Some(id) = self.mon_picker(&mut child, w.id, "qc", if editing == 1 { "🔍  Attacker" } else { "🔍  Defender" }) {
                let name = wiki.as_ref().and_then(|wk| wk.mon(&id)).map(|m| m.name.clone()).unwrap_or(id);
                new = Some(if editing == 1 { (name, def.clone()) } else { (atk.clone(), name) });
                set_edit = Some(0);
            }
        } else {
            self.calc_lines(&p, &b, rest, &a, &d, world);
        }
        let mut save = false;
        if foot.is_positive() {
            let lr = fit::label(&p, foot, "⭐  Save this calc", b.bold(11.5), kit::primary(), Align2::LEFT_CENTER);
            fit::label(&p, foot, "Usual sets, Lv 50", b.font(10.5), MUTED(), Align2::RIGHT_CENTER);
            save = ui.interact(lr.expand(3.0), ui.id().with(("qc-save", w.id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        }
        if let Some(e) = set_edit {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("w-qc-edit", w.id)), e));
        }
        if let Some((x, y)) = new {
            *out = Some(vec![x, y]);
        }
        if save && !preview {
            let k = format!("{atk}|{def}");
            if !self.cfg.home.saved_calcs.contains(&k) {
                self.cfg.home.saved_calcs.insert(0, k);
                self.cfg.home.saved_calcs.truncate(12);
                self.dirty = true;
            }
            self.app_toast("Saved to Saved calcs".into());
        }
        if open {
            self.calc_open(&atk, &def, world);
        }
    }

    /// Calcs you saved: click one for the Damage Calc.
    pub(super) fn w_savedcalcs(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "savedcalcs");
        let world = World::Platinum;
        let saved = self.cfg.home.saved_calcs.clone();
        let wiki = self.wiki.clone();
        let pairs: Vec<(String, String)> = saved.iter().filter_map(|k| k.split_once('|').map(|(a, d)| (a.to_string(), d.to_string()))).collect();
        let rows: Vec<Row> = pairs
            .iter()
            .map(|(a, d)| {
                let hit = best_hit(wiki.as_deref(), &default_set(a, world), &default_set(d, world), world);
                let row = Row::new(format!("{a} vs {d}")).lead(Lead::Wiki(a.clone(), false)).tip("Open in the Damage Calc · right-click to remove").menu("Remove");
                match hit {
                    Some(h) => row.sub(h.mv.clone()).right(format!("{:.0}–{:.0}%", h.lo, h.hi), Some(super::charts::ink(hit_col(h.hi)))),
                    None => row,
                }
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "⭐", col: kit::C_GOLD, title: "Saved calcs", link: None }, &rows, "Press “⭐ Save this calc” in a Quick calc widget and it's kept here.");
        if let Some(k) = c.menu {
            if let Some(pos) = saved.iter().position(|s| s == &format!("{}|{}", pairs[k].0, pairs[k].1)) {
                self.cfg.home.saved_calcs.remove(pos);
                self.dirty = true;
            }
        } else if let Some(k) = c.item {
            let (a, d) = pairs[k].clone();
            self.calc_open(&a, &d, world);
        }
    }

    /// Your main team's Speed against well-known fast Pokémon: bars that
    /// fill the box.
    pub(super) fn w_speedtiers(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "speedtiers");
        let team = self.dash_team();
        let (body, _) = fit::head(ui, &b, "💨", kit::C_CYAN, "Speed tiers", None);
        let Some(t) = team.filter(|t| !t.members.is_empty()) else { return self.no_team(ui, &b, body) };
        let p = ui.painter_at(b.r);
        let fs = self.team_facts(&t);
        let level = t.members.iter().map(|m| m.level).max().unwrap_or(50);
        let d = battle::data();
        let jolly = d.nature("Jolly");
        let mut list: Vec<(String, u16, Option<String>)> = Vec::new();
        for f in fs.iter().flatten() {
            let s = if f.item == "Choice Scarf" { f.stats[5] * 3 / 2 } else { f.stats[5] };
            list.push((f.name.clone(), s, Some(f.species.clone())));
        }
        for (name, bs) in BENCH.iter() {
            list.push((format!("{name} (max)"), battle::stat(5, *bs, 31, 252, level, jolly, false), None));
        }
        list.sort_by(|a, b| b.1.cmp(&a.1));
        let (rows_r, foot) = if body.height() > b.px(150.0) { fit::split_bottom(body, b.px(15.0), b.px(6.0)) } else { (body, Rect::NOTHING) };
        // your team always shows; the fast ones fill what's left
        let cap = fit::rows(rows_r, 64, b.px(20.0), b.px(3.0), 1.0).len().max(1);
        let mine = list.iter().filter(|r| r.2.is_some()).count();
        let mut bench_left = cap.saturating_sub(mine);
        let shown: Vec<&(String, u16, Option<String>)> = list
            .iter()
            .filter(|r| {
                if r.2.is_some() {
                    true
                } else if bench_left > 0 {
                    bench_left -= 1;
                    true
                } else {
                    false
                }
            })
            .collect();
        let top = shown.first().map(|r| r.1).unwrap_or(1).max(1) as f32;
        for ((name, s, sp), r) in shown.iter().zip(fit::rows(rows_r, shown.len(), b.px(20.0), b.px(3.0), 2.0).iter()) {
            let mine = sp.is_some();
            let name_w = (r.width() * 0.42).min(b.px(170.0));
            let (name_r, bar_r) = fit::split_left(*r, name_w, b.px(6.0));
            let (sp_r, nr) = if mine { fit::split_left(name_r, name_r.height().min(b.px(28.0)), b.px(4.0)) } else { (Rect::NOTHING, name_r) };
            if let Some(sp) = sp {
                self.team_sprite(&p, sp, false, sp_r.center(), vec2(sp_r.width(), sp_r.width()));
            }
            fit::label(&p, nr, name, if mine { b.bold(12.0) } else { b.font(11.0) }, if mine { TEXT() } else { MUTED() }, Align2::LEFT_CENTER);
            let (bar_r, num_r) = fit::split_right(bar_r, b.px(34.0), b.px(4.0));
            let bh = (r.height() * if mine { 0.45 } else { 0.3 }).clamp(4.0, b.px(14.0));
            let bar = Rect::from_min_size(pos2(bar_r.min.x, r.center().y - bh / 2.0), vec2((bar_r.width() * (*s as f32 / top)).max(2.0), bh));
            p.rect_filled(bar, bh / 2.0, if mine { kit::C_CYAN } else { theme::blend(kit::sunken(), MUTED(), 0.35) });
            fit::label(&p, num_r, &s.to_string(), b.font(11.0), if mine { TEXT() } else { MUTED() }, Align2::LEFT_CENTER);
        }
        if foot.is_positive() {
            fit::label(&p, foot, &format!("Fast ones with full Speed and a +Speed nature, Lv {level}."), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
    }

    /// The calculation open in the Damage Calc.
    pub(super) fn w_lastcalc(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "lastcalc");
        let c = &self.setup.team.calc;
        let (l, r) = (c.left.clone(), c.right.clone());
        let world = self.dash_team().map(|t| t.world).unwrap_or(World::Platinum);
        let (body, mut open) = fit::head(ui, &b, "📋", kit::C_ORANGE, "Last result", Some("Open ›"));
        let p = ui.painter_at(b.r);
        let (Some(l), Some(r)) = (l, r) else {
            fit::empty(&p, &b, body, "📋", "Pick two Pokémon in the Damage Calc and the result shows here.");
            if open {
                self.go_to(Tab::Calc);
            }
            return;
        };
        let wide = body.width() > body.height() * 2.4;
        let (pair_r, rest) = if wide { fit::split_left(body, (body.width() * 0.4).min(body.height() * 2.6), b.px(12.0)) } else { fit::split_top(body, (body.height() * 0.42).clamp(b.px(40.0), b.px(110.0)), b.px(8.0)) };
        self.calc_pair(ui, &b, pair_r, &l.set, &r.set, 0, false, 0);
        self.calc_lines(&p, &b, rest, &l.set, &r.set, world);
        if ui.interact(pair_r, ui.id().with(("lc-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open the Damage Calc").clicked() {
            open = true;
        }
        if open {
            self.go_to(Tab::Calc);
        }
    }

    /// Your main team against one Pokémon you pick: a row each, filling the
    /// box.
    pub(super) fn w_threat(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "threat");
        let team = self.dash_team();
        let world = team.as_ref().map(|t| t.world).unwrap_or(World::Platinum);
        let foe_name = if preview { Some("Garchomp".to_string()) } else { w.data.first().cloned() };
        let wiki = self.wiki.clone();
        let (body, change) = fit::head(ui, &b, "🚨", kit::C_RED, "Threat check", foe_name.as_ref().map(|_| "Change"));
        let Some(t) = team.filter(|t| !t.members.is_empty()) else { return self.no_team(ui, &b, body) };
        let p = ui.painter_at(b.r);
        let Some(foe) = foe_name else {
            let (note_r, rest) = fit::split_top(body, b.px(16.0), b.px(6.0));
            fit::label(&p, note_r, "Pick a Pokémon that gives you trouble.", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(rest.intersect(ui.clip_rect()));
            if let Some(id) = self.mon_picker(&mut child, w.id, "threat", "🔍  Garchomp, Gyarados…") {
                let name = self.wiki.as_ref().and_then(|wk| wk.mon(&id)).map(|m| m.name.clone()).unwrap_or(id);
                *out = Some(vec![name]);
            }
            return;
        };
        let fm = default_set(&foe, world);
        // the foe, the table (a row per member) and the verdict
        let (who, rest) = fit::split_top(body, b.px(34.0), b.px(6.0));
        let (sp_r, name_r) = fit::split_left(who, who.height(), b.px(8.0));
        self.team_sprite(&p, &fm.species, false, sp_r.center(), vec2(sp_r.width(), sp_r.width()));
        let nl = fit::stack(name_r, 2, 0.0);
        fit::label(&p, nl[0], &fm.species, b.bold(13.5), TEXT(), Align2::LEFT_CENTER);
        fit::label(&p, nl[1], &if fm.item.is_empty() { "Its usual set".to_string() } else { format!("{} · its usual set", fm.item) }, b.font(10.5), MUTED(), Align2::LEFT_CENTER);
        let (table, verdict) = if rest.height() > b.px(70.0) { fit::split_bottom(rest, b.px(15.0), b.px(5.0)) } else { (rest, Rect::NOTHING) };
        let (hdr, body_r) = fit::split_top(table, b.px(14.0), b.px(2.0));
        let col_w = b.px(70.0);
        let (_, us_r) = fit::split_right(hdr, col_w, 0.0);
        fit::label(&p, Rect::from_min_max(pos2(hdr.max.x - col_w * 2.0, hdr.min.y), pos2(hdr.max.x - col_w, hdr.max.y)), "it hits you", b.font(10.0), MUTED(), Align2::RIGHT_CENTER);
        fit::label(&p, us_r, "you hit it", b.font(10.0), MUTED(), Align2::RIGHT_CENTER);
        let mut answers = 0;
        let members: Vec<Member> = t.members.iter().take(6).cloned().collect();
        let slots = fit::rows(body_r, members.len(), b.px(22.0), b.px(3.0), 2.0);
        for (m, r) in members.iter().zip(slots.iter()) {
            let theirs = best_hit(wiki.as_deref(), &fm, m, world);
            let mine = best_hit(wiki.as_deref(), m, &fm, world);
            let (left, us) = fit::split_right(*r, col_w, 0.0);
            let (left, them) = fit::split_right(left, col_w, 0.0);
            let (sp, nm) = fit::split_left(left, left.height().min(b.px(30.0)), b.px(6.0));
            self.team_sprite(&p, &m.species, m.shiny, sp.center(), vec2(sp.width(), sp.width()));
            fit::label(&p, nm, &m.display_name(), b.bold(12.0), TEXT(), Align2::LEFT_CENTER);
            fit::label(&p, them, &theirs.as_ref().map_or("—".to_string(), |h| format!("{:.0}%", h.hi)), b.bold(12.0), theirs.as_ref().map_or(MUTED(), |h| super::charts::ink(hit_col(h.hi))), Align2::RIGHT_CENTER);
            fit::label(&p, us, &mine.as_ref().map_or("—".to_string(), |h| format!("{:.0}%", h.hi)), b.bold(12.0), mine.as_ref().map_or(MUTED(), |h| super::charts::ink(hit_col(100.0 - h.hi.min(100.0)))), Align2::RIGHT_CENTER);
            if mine.as_ref().is_some_and(|h| h.hi >= 50.0) && theirs.as_ref().map_or(true, |h| h.hi < 50.0) {
                answers += 1;
            }
        }
        if verdict.is_positive() {
            if answers > 0 {
                fit::label(&p, verdict, &format!("✔  {answers} of them take it on well (hit it for 50%+, take under 50%)."), b.font(11.5), theme::GREEN, Align2::LEFT_CENTER);
            } else {
                fit::label(&p, verdict, "⚠  Nobody takes it on well. Worth a plan!", b.font(11.5), kit::C_ORANGE, Align2::LEFT_CENTER);
            }
        }
        if change && !preview {
            *out = Some(Vec::new());
        }
    }
}
