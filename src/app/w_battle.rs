//! Dashboard widgets for the Team Planner, the Battle Sim, Breeding and
//! the Damage Calc. The team ones all show your "main team": the one you
//! pick in the Team switcher widget, or your newest.

use super::home::ago;
use super::team_analysis::{facts, weak_resist, Facts, BENCH};
use super::team_calc::{calc_mon, calc_move, default_set, Poke};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_explore::loading;
use super::w_kit::{self as wk, head, n, note, row};
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::battle::{self, World};
use crate::calc;
use crate::sim::{Format, Mode, Setup};
use crate::teams::{Member, Team};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

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

    /// "Make a team first" with a button.
    fn no_team(&mut self, ui: &mut egui::Ui) {
        note(ui, "Make a team in the Team Planner and it shows here.");
        ui.add_space(6.0);
        if wk::go_button(ui, "Open the Team Planner  ›", false) {
            self.go_to(Tab::Team);
        }
    }

    // ───────────────────────────── Team Planner ─────────────────────────────

    pub(super) fn w_mainteam(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        let mut open = false;
        kit::card(ui, |ui| {
            let title = team.as_ref().map(|t| if t.name.trim().is_empty() { "Untitled team".to_string() } else { t.name.clone() }).unwrap_or_else(|| "Main team".into());
            open = head(ui, "🛡", kit::C_BLUE, &title, team.as_ref().map(|_| "Open ›"));
            ui.add_space(8.0);
            let Some(t) = team.clone() else { return self.no_team(ui) };
            let w = ui.available_width();
            let cols = if w >= 520.0 { 6 } else { 3 };
            let cw = ((w - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for chunk in (0..6).collect::<Vec<_>>().chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for i in chunk {
                        let (r, resp) = ui.allocate_exact_size(vec2(cw, 104.0), Sense::click());
                        let p = ui.painter().clone();
                        match t.members.get(*i) {
                            Some(m) => {
                                let types = battle::data().species(&m.species).map(|s| s.types_in(t.world).to_vec()).unwrap_or_default();
                                let col = types.first().map(|x| super::pokedex::type_color(x)).unwrap_or(kit::C_BLUE);
                                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                                p.rect_filled(r, 12.0, theme::blend(CARD(), col, 0.08 + 0.08 * ht));
                                self.team_sprite(&p, &m.species, m.shiny, pos2(r.center().x, r.min.y + 40.0), vec2(62.0, 62.0));
                                p.with_clip_rect(r).text(pos2(r.center().x, r.max.y - 28.0), Align2::CENTER_CENTER, m.display_name(), crate::fonts::sans_bold_font(12.0), TEXT());
                                p.with_clip_rect(r).text(pos2(r.center().x, r.max.y - 12.0), Align2::CENTER_CENTER, if m.item.is_empty() { format!("Lv {}", m.level) } else { m.item.clone() }, crate::fonts::sans_font(10.5), MUTED());
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    open = true;
                                }
                            }
                            None => {
                                p.rect_stroke(r.shrink(0.5), 12.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_BLUE, 0.15)));
                                p.text(r.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(20.0), MUTED().gamma_multiply(0.5));
                            }
                        }
                    }
                });
                ui.add_space(8.0);
            }
        });
        if let (true, Some(t)) = (open, team) {
            self.open_team_page(t.id);
        }
    }

    fn team_facts(&self, t: &Team) -> Vec<Option<Facts>> {
        t.members.iter().map(|m| facts(self.wiki.as_deref(), t.world, m)).collect()
    }

    /// The types most of the team is weak to.
    pub(super) fn w_teamweak(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        kit::card(ui, |ui| {
            head(ui, "⚠", kit::C_ORANGE, "Team weaknesses", None);
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let fs = self.team_facts(&t);
            let mut rows: Vec<(&str, usize, usize)> = calc::types_in(t.world).iter().map(|at| {
                let (w, r) = weak_resist(&fs, at, t.world);
                (*at, w, r)
            }).filter(|x| x.1 >= 2 || (x.1 >= 1 && x.2 == 0 && t.members.len() <= 2)).collect();
            rows.sort_by(|a, b| (b.1 as i32 - b.2 as i32).cmp(&(a.1 as i32 - a.2 as i32)));
            ui.label(RichText::new(format!("For {}", if t.name.trim().is_empty() { "your team" } else { &t.name })).size(kit::fs(11.5)).color(MUTED()));
            ui.add_space(4.0);
            if rows.is_empty() {
                ui.label(RichText::new("✔  No type hits more than one of them hard.").size(kit::fs(12.5)).color(theme::GREEN));
            }
            for (at, weak, resist) in rows.iter().take(5) {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
                let p = ui.painter();
                wk::type_tag(p, pos2(r.min.x, r.center().y), at, "");
                let bad = *resist == 0;
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, format!("{weak} weak · {resist} resist"), crate::fonts::sans_bold_font(11.5), if bad { kit::C_RED } else { MUTED() });
            }
        });
    }

    /// Pick which team the Dashboard shows.
    pub(super) fn w_teamswitch(&mut self, ui: &mut egui::Ui) {
        let teams = self.team_choices();
        let cur = self.dash_team().map(|t| t.id);
        let mut pick: Option<u64> = None;
        let mut planner = false;
        kit::card(ui, |ui| {
            planner = head(ui, "🔁", kit::C_BLUE, "Team switcher", Some("Teams ›"));
            ui.add_space(4.0);
            note(ui, "The team widgets show this team.");
            ui.add_space(6.0);
            if teams.is_empty() {
                return self.no_team(ui);
            }
            for (id, name, count) in teams.iter().take(6) {
                let (r, resp) = row(ui, 32.0);
                let on = Some(*id) == cur;
                let p = ui.painter();
                let c = pos2(r.min.x + 12.0, r.center().y);
                p.circle_stroke(c, 7.0, Stroke::new(1.5f32, if on { kit::primary() } else { MUTED() }));
                if on {
                    p.circle_filled(c, 4.0, kit::primary());
                }
                let label = if name.trim().is_empty() { "Untitled team" } else { name };
                p.text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, label, if on { crate::fonts::sans_bold_font(13.0) } else { crate::fonts::sans_font(13.0) }, TEXT());
                p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, format!("{count}/6"), crate::fonts::sans_font(11.5), MUTED());
                if resp.clicked() {
                    pick = Some(*id);
                }
            }
        });
        if let Some(id) = pick {
            self.cfg.home.main_team = Some(id);
            self.dirty = true;
        }
        if planner {
            self.go_to(Tab::Team);
        }
    }

    /// How hard the team's moves hit every type.
    pub(super) fn w_coverage(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        kit::card(ui, |ui| {
            head(ui, "🎯", kit::C_VIOLET, "Coverage check", None);
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let fs = self.team_facts(&t);
            let types = calc::types_in(t.world);
            let best: Vec<(&str, f32)> = types.iter().map(|d| {
                let b = fs.iter().flatten().flat_map(|f| f.attacks.iter()).map(|a| calc::eff(&a.1, &[d.to_string()], t.world)).fold(-1.0f32, f32::max);
                (*d, b)
            }).collect();
            let se = best.iter().filter(|b| b.1 > 1.0).count();
            let walls: Vec<&str> = best.iter().filter(|b| b.1 >= 0.0 && b.1 < 1.0).map(|b| b.0).collect();
            ui.label(RichText::new(format!("Hits {se} of {} types super effectively", types.len())).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
            ui.add_space(8.0);
            let w = ui.available_width();
            let cols = ((w + 6.0) / (88.0 + 6.0)).floor().clamp(3.0, 9.0) as usize;
            let cw = ((w - 6.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for chunk in best.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (d, b) in chunk {
                        let (r, resp) = ui.allocate_exact_size(vec2(cw, 34.0), Sense::hover());
                        let p = ui.painter();
                        let (label, col) = match *b {
                            x if x < 0.0 => ("—", kit::C_SLATE),
                            x if x >= 2.0 => ("2×", theme::GREEN),
                            x if x > 1.0 => ("+", theme::GREEN),
                            x if x == 1.0 => ("1×", kit::C_SLATE),
                            x if x == 0.0 => ("0×", kit::C_RED),
                            _ => ("½×", kit::C_ORANGE),
                        };
                        p.rect_filled(r, 8.0, theme::blend(CARD(), col, 0.18));
                        p.rect_filled(Rect::from_min_size(r.min, vec2(4.0, r.height())), 2.0, super::pokedex::type_color(d));
                        p.with_clip_rect(r).text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, &d[..d.len().min(4)], crate::fonts::sans_bold_font(11.0), TEXT());
                        p.text(pos2(r.max.x - 6.0, r.center().y), Align2::RIGHT_CENTER, label, crate::fonts::sans_bold_font(11.5), super::charts::ink(col));
                        resp.on_hover_text(format!("Best hit on {d}: {label}"));
                    }
                });
                ui.add_space(6.0);
            }
            if !walls.is_empty() {
                note(ui, format!("Nothing hits these hard, and something resists: {}", walls.join(", ")));
            }
        });
    }

    /// A type that would cover the team's biggest gaps, with examples.
    pub(super) fn w_teamidea(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            head(ui, "💭", kit::C_TEAL, "Team ideas", None);
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let fs = self.team_facts(&t);
            let types = calc::types_in(t.world);
            let mut gaps: Vec<(&str, usize)> = types.iter().map(|at| (*at, weak_resist(&fs, at, t.world))).filter(|(_, (w, r))| *w >= 2 && *w > *r).map(|(a, (w, _))| (a, w)).collect();
            gaps.sort_by(|a, b| b.1.cmp(&a.1));
            gaps.truncate(3);
            if gaps.is_empty() {
                ui.label(RichText::new("✔  No big gaps: no type hits most of them hard.").size(kit::fs(12.5)).color(theme::GREEN));
                return;
            }
            // the type that takes little from the most of those
            let best = types.iter().map(|d| (*d, gaps.iter().filter(|(a, _)| calc::eff(a, &[d.to_string()], t.world) < 1.0).count())).max_by_key(|x| x.1).filter(|x| x.1 > 0);
            let Some((ty, covers)) = best else {
                note(ui, "No one type covers those. Try a Pokémon with a resisting Ability.");
                return;
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Add a").size(kit::fs(13.0)).color(TEXT()));
                super::pokedex::type_chip(ui, ty, "");
                ui.label(RichText::new("type").size(kit::fs(13.0)).color(TEXT()));
            });
            let names: Vec<&str> = gaps.iter().filter(|(a, _)| calc::eff(a, &[ty.to_string()], t.world) < 1.0).map(|g| g.0).collect();
            note(ui, format!("It takes little from {} ({covers} of your team's weak spots).", names.join(", ")));
            ui.add_space(6.0);
            // a few strong ones of that type the team doesn't have yet
            let d = battle::data();
            let mut picks: Vec<&battle::Species> = d.species.iter().filter(|s| s.num >= 1 && s.num <= 493 && !s.is_mega() && s.base.is_empty() && s.types_in(t.world).iter().any(|x| x == ty) && (t.world != World::Platinum || !s.modern_only()) && !t.members.iter().any(|m| battle::to_id(&m.species) == s.id)).collect();
            picks.sort_by_key(|s| std::cmp::Reverse(s.bst(t.world)));
            let wiki = self.wiki.clone();
            ui.horizontal(|ui| {
                for s in picks.iter().take(3) {
                    let (r, resp) = ui.allocate_exact_size(vec2(70.0, 76.0), Sense::click());
                    let p = ui.painter().clone();
                    p.rect_filled(r, 10.0, theme::blend(CARD(), super::pokedex::type_color(ty), 0.1));
                    self.team_sprite(&p, &s.name, false, pos2(r.center().x, r.min.y + 30.0), vec2(52.0, 52.0));
                    p.with_clip_rect(r).text(pos2(r.center().x, r.max.y - 12.0), Align2::CENTER_CENTER, &s.name, crate::fonts::sans_bold_font(11.0), TEXT());
                    if resp.on_hover_text("Its Pokédex page").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = wiki.as_ref().and_then(|w| w.by_dex(s.num)).map(|m| m.id.clone());
                    }
                }
            });
        });
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

    pub(super) fn w_lastbattle(&mut self, ui: &mut egui::Ui) {
        let last = self.setup.sim.file().history.first().cloned();
        let mut again = false;
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "⚔", kit::C_RED, "Last battle", Some("Battle Sim ›"));
            ui.add_space(6.0);
            let Some(b) = last.clone() else {
                note(ui, "Battle in the Battle Sim and how it went shows here.");
                return;
            };
            ui.horizontal(|ui| {
                let res = if b.result.is_empty() { "Left early" } else { &b.result };
                let col = result_col(&b.result);
                let g = ui.painter().layout_no_wrap(res.to_uppercase(), crate::fonts::sans_bold_font(11.0), Color32::WHITE);
                let (r, _) = ui.allocate_exact_size(g.size() + vec2(16.0, 8.0), Sense::hover());
                ui.painter().rect_filled(r, 8.0, col);
                ui.painter().galley(r.min + vec2(8.0, 4.0), g, Color32::WHITE);
                ui.label(RichText::new(ago(b.when)).size(kit::fs(11.5)).color(MUTED()));
            });
            ui.add_space(4.0);
            ui.label(RichText::new(&b.title).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
            ui.label(RichText::new(format!("{} · {} turns", b.team, b.turns)).size(kit::fs(11.5)).color(MUTED()));
            ui.add_space(6.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
            let p = ui.painter().clone();
            let mut x = r.min.x + 20.0;
            for (sp, shiny) in b.ours.iter().take(3) {
                self.team_sprite(&p, sp, *shiny, pos2(x, r.center().y), vec2(40.0, 40.0));
                x += 36.0;
            }
            p.text(pos2(r.center().x + 10.0, r.center().y), Align2::CENTER_CENTER, "vs", crate::fonts::sans_bold_font(12.0), MUTED());
            let mut x = r.max.x - 20.0;
            for (sp, shiny) in b.theirs.iter().take(3) {
                self.team_sprite(&p, sp, *shiny, pos2(x, r.center().y), vec2(40.0, 40.0));
                x -= 36.0;
            }
            ui.add_space(6.0);
            again = wk::go_button(ui, "↻  Rematch", true);
        });
        if again {
            if let Some(b) = last {
                self.sim_go(b.setup);
            }
        } else if open {
            self.go_to(Tab::Sim);
        }
    }

    pub(super) fn w_winrecord(&mut self, ui: &mut egui::Ui) {
        let hist = self.setup.sim.file().history.clone();
        kit::card(ui, |ui| {
            head(ui, "🏆", kit::C_GOLD, "Win record", None);
            ui.add_space(6.0);
            let played: Vec<&crate::sim::BattleRecord> = hist.iter().filter(|b| !b.result.is_empty()).collect();
            if played.is_empty() {
                note(ui, "Finish a battle in the Battle Sim and your record starts here.");
                return;
            }
            let won = played.iter().filter(|b| b.result == "Won" || b.result == "Caught").count();
            let lost = played.iter().filter(|b| b.result == "Lost").count();
            let other = played.len() - won - lost;
            let rate = won as f32 / played.len() as f32;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.0}%", rate * 100.0)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                ui.label(RichText::new(format!("won of {}", played.len())).size(kit::fs(12.5)).color(MUTED()));
            });
            ui.add_space(6.0);
            // one bar: won, lost, other
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 12.0), Sense::hover());
            let p = ui.painter();
            let tot = played.len() as f32;
            let mut x = r.min.x;
            for (k, col) in [(won, theme::GREEN), (lost, kit::C_RED), (other, kit::C_ORANGE)] {
                let wv = r.width() * k as f32 / tot;
                if wv > 0.0 {
                    p.rect_filled(Rect::from_min_size(pos2(x, r.min.y), vec2(wv, r.height())), 4.0, col);
                }
                x += wv;
            }
            ui.add_space(4.0);
            ui.label(RichText::new(format!("{won} won · {lost} lost · {other} ran or other")).size(kit::fs(11.5)).color(MUTED()));
            // the team with the most wins
            let mut by: Vec<(String, usize)> = Vec::new();
            for b in played.iter().filter(|b| b.result == "Won") {
                match by.iter_mut().find(|x| x.0 == b.team) {
                    Some(x) => x.1 += 1,
                    None => by.push((b.team.clone(), 1)),
                }
            }
            if let Some((team, k)) = by.into_iter().max_by_key(|x| x.1) {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("🏅  Best team: {team} ({k} win{})", if k == 1 { "" } else { "s" })).size(kit::fs(12.5)).color(TEXT()));
            }
        });
    }

    /// One click: your main team against a random wild Pokémon.
    pub(super) fn w_quickbattle(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        let mut go = false;
        kit::card(ui, |ui| {
            head(ui, "⚡", kit::C_GOLD, "Quick battle", None);
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let name = if t.name.trim().is_empty() { "Your team" } else { &t.name };
            ui.label(RichText::new(format!("{name} vs a random wild Pokémon")).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
            note(ui, "From anywhere in Sinnoh, at its own level.");
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
            let p = ui.painter().clone();
            let mut x = r.min.x + 22.0;
            for m in t.members.iter().take(6) {
                self.team_sprite(&p, &m.species, m.shiny, pos2(x, r.center().y), vec2(42.0, 42.0));
                x += 34.0;
            }
            p.text(pos2(r.max.x - 22.0, r.center().y), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(24.0), kit::C_GOLD);
            ui.add_space(6.0);
            go = wk::go_button(ui, "⚡  Battle!", true);
        });
        if !go {
            return;
        }
        let (Some(t), Some(w)) = (team, self.wiki.clone()) else { return };
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

    /// Trainers you made, a click from a battle with your main team.
    pub(super) fn w_trainers(&mut self, ui: &mut egui::Ui) {
        let trainers = self.setup.sim.file().trainers.clone();
        let team = self.dash_team();
        let mut fight: Option<u64> = None;
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "🎖", kit::C_ORANGE, "Your trainers", Some("Battle Sim ›"));
            ui.add_space(6.0);
            if trainers.is_empty() {
                note(ui, "Make trainers to battle (a gym leader, a rival…) in the Battle Sim and they show here.");
                return;
            }
            if let Some(t) = &team {
                note(ui, format!("Battle them with {}", if t.name.trim().is_empty() { "your team" } else { &t.name }));
            }
            ui.add_space(4.0);
            for tr in trainers.iter().take(4) {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
                let p = ui.painter().clone();
                let title = if tr.class.is_empty() { tr.name.clone() } else { format!("{} {}", tr.class, tr.name) };
                p.text(pos2(r.min.x + 4.0, r.center().y - 9.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(13.0), TEXT());
                p.text(pos2(r.min.x + 4.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{} Pokémon{}", tr.members.len(), if tr.doubles { " · doubles" } else { "" }), crate::fonts::sans_font(11.0), MUTED());
                let br = Rect::from_min_size(pos2(r.max.x - 80.0, r.center().y - 15.0), vec2(80.0, 30.0));
                let mut x = br.min.x - 22.0;
                for m in tr.members.iter().take(4) {
                    self.team_sprite(&p, &m.species, m.shiny, pos2(x, r.center().y), vec2(34.0, 34.0));
                    x -= 30.0;
                }
                let ok = team.as_ref().is_some_and(|t| !t.members.is_empty()) && !tr.members.is_empty();
                let b = ui.put(br, egui::Button::new(RichText::new("Battle").font(crate::fonts::sans_bold_font(12.5)).color(if ok { kit::on_primary() } else { MUTED() })).fill(if ok { kit::primary() } else { kit::btn_fill() }).rounding(15.0));
                if b.on_hover_text(if ok { "Battle this trainer with your main team" } else { "Needs a team with Pokémon on both sides" }).clicked() && ok {
                    fight = Some(tr.id);
                }
            }
        });
        if let (Some(id), Some(t)) = (fight, team) {
            let tr = trainers.iter().find(|x| x.id == id);
            let doubles = tr.is_some_and(|x| x.doubles) && t.members.len() >= 2;
            let s = Setup { mode: Mode::Trainer, format: if doubles { Format::Doubles } else { Format::Singles }, team: Some(t.id), picks: (0..t.members.len().min(6)).collect(), trainer: Some(id), world: t.world, ..Default::default() };
            self.sim_go(s);
        } else if open {
            self.go_to(Tab::Sim);
        }
    }

    pub(super) fn w_battlehistory(&mut self, ui: &mut egui::Ui) {
        let hist: Vec<crate::sim::BattleRecord> = self.setup.sim.file().history.iter().take(5).cloned().collect();
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "📜", kit::C_SLATE, "Battle history", Some("All ›"));
            ui.add_space(6.0);
            if hist.is_empty() {
                note(ui, "Your battles show here once you've played some in the Battle Sim.");
            }
            for b in &hist {
                let (r, resp) = row(ui, 40.0);
                let p = ui.painter().clone();
                let res = if b.result.is_empty() { "—" } else { &b.result };
                let col = result_col(&b.result);
                let g = p.layout_no_wrap(res.to_string(), crate::fonts::sans_bold_font(10.5), Color32::WHITE);
                let tr = Rect::from_min_size(pos2(r.min.x + 2.0, r.center().y - 10.0), vec2(58.0, 20.0));
                p.rect_filled(tr, 7.0, col);
                p.galley(tr.center() - g.size() / 2.0, g, Color32::WHITE);
                p.with_clip_rect(r.with_max_x(r.max.x - 70.0)).text(pos2(tr.max.x + 10.0, r.center().y - 8.0), Align2::LEFT_CENTER, &b.title, crate::fonts::sans_bold_font(12.5), TEXT());
                p.with_clip_rect(r.with_max_x(r.max.x - 70.0)).text(pos2(tr.max.x + 10.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{} · {} turns", b.team, b.turns), crate::fonts::sans_font(11.0), MUTED());
                p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, ago(b.when), crate::fonts::sans_font(11.0), MUTED());
                if resp.clicked() {
                    open = true;
                }
            }
        });
        if open {
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

    pub(super) fn w_breeders(&mut self, ui: &mut egui::Ui) {
        let breeders = self.setup.breed.file().breeders.clone();
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "📦", kit::C_PINK, "Breeders box", Some("All ›"));
            ui.add_space(6.0);
            if breeders.is_empty() {
                note(ui, "Add the Pokémon you breed with in Breeding → Breeders and they show here.");
                return;
            }
            ui.label(RichText::new(format!("{} breeder{}", breeders.len(), if breeders.len() == 1 { "" } else { "s" })).size(kit::fs(11.5)).color(MUTED()));
            for b in breeders.iter().rev().take(5) {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &b.parent.species, false, pos2(r.min.x + 18.0, r.center().y), vec2(36.0, 36.0));
                let name = if b.name.trim().is_empty() { self.mon_name(&b.parent.species) } else { b.name.clone() };
                let g = match b.parent.gender() {
                    crate::breed::Gender::Male => " ♂",
                    crate::breed::Gender::Female => " ♀",
                    crate::breed::Gender::Genderless => "",
                };
                p.text(pos2(r.min.x + 44.0, r.center().y - 8.0), Align2::LEFT_CENTER, format!("{name}{g}"), crate::fonts::sans_bold_font(12.5), TEXT());
                p.text(pos2(r.min.x + 44.0, r.center().y + 10.0), Align2::LEFT_CENTER, b.parent.nature.clone().unwrap_or_else(|| "Any nature".into()), crate::fonts::sans_font(11.0), MUTED());
                // its stats: a dot per stat, filled at 31
                for (i, iv) in b.parent.ivs.iter().enumerate() {
                    let c = pos2(r.max.x - 8.0 - (5 - i) as f32 * 12.0, r.center().y);
                    match iv {
                        Some(31) => {
                            p.circle_filled(c, 4.0, kit::C_PINK);
                        }
                        Some(_) => {
                            p.circle_stroke(c, 3.5, Stroke::new(1.2f32, MUTED()));
                        }
                        None => {
                            p.circle_stroke(c, 3.5, Stroke::new(1.0f32, MUTED().gamma_multiply(0.4)));
                        }
                    }
                }
            }
        });
        if open {
            self.open_breed(super::breed_home::BreedView::Breeders);
        }
    }

    /// The egg moves a Pokémon you pick can get.
    pub(super) fn w_eggmoves(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "💥", "Egg move finder") };
        let species = if preview { Some("GIBLE".to_string()) } else { w.data.first().cloned() };
        let modern = self.setup.breed.file().setup.world != World::Platinum;
        let mut change = false;
        let mut picked: Option<String> = None;
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            change = head(ui, "💥", kit::C_PINK, "Egg moves", species.as_ref().map(|_| "Change"));
            ui.add_space(6.0);
            let Some(id) = species.clone() else {
                note(ui, "Pick a Pokémon to see the moves it can get from an Egg.");
                ui.add_space(6.0);
                picked = self.mon_picker(ui, w.id, "egg", "🔍  Gible, Eevee…");
                return;
            };
            let baby = crate::breed::baby_of(&wiki, &id);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &baby, false, r.center(), vec2(38.0, 38.0));
                ui.label(RichText::new(self.mon_name(&baby)).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
            });
            ui.add_space(4.0);
            let moves: Vec<String> = wiki.learnset(&baby).map(|l| if modern && !l.megg.is_empty() { l.megg.clone() } else { l.egg.clone() }).unwrap_or_default();
            if moves.is_empty() {
                note(ui, "No egg moves.");
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                for k in &moves {
                    let Some(mv) = wiki.move_info(k) else { continue };
                    let col = super::pokedex::type_color(&mv.t);
                    let g = ui.painter().layout_no_wrap(mv.n.clone(), crate::fonts::sans_bold_font(11.5), TEXT());
                    let (r, resp) = ui.allocate_exact_size(g.size() + vec2(20.0, 10.0), Sense::click());
                    ui.painter().rect_filled(r, r.height() / 2.0, theme::blend(CARD(), col, 0.22));
                    ui.painter().circle_filled(pos2(r.min.x + 8.0, r.center().y), 3.0, col);
                    ui.painter().galley(r.min + vec2(14.0, 5.0), g, TEXT());
                    if resp.on_hover_text(format!("{} · {} · open its page", mv.t, mv.c)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(k.clone());
                    }
                }
            });
        });
        if change && !preview {
            *out = Some(Vec::new());
        }
        if let Some(p) = picked {
            *out = Some(vec![p]);
        }
        if let Some(k) = open {
            self.info_open_page(super::info_home::Page::Move(k));
        }
    }

    /// The next thing to do in your breeding plan, with a tick.
    pub(super) fn w_nextstep(&mut self, ui: &mut egui::Ui) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "👣", "Next step") };
        let (target, breeders, setup, done) = {
            let f = self.setup.breed.file();
            (f.target.clone(), f.breeders.clone(), f.setup, f.plan_done.clone())
        };
        let mut open = false;
        let mut tick: Option<String> = None;
        kit::card(ui, |ui| {
            open = head(ui, "👣", kit::C_TEAL, "Next step", Some("Plan ›"));
            ui.add_space(6.0);
            if target.species.is_empty() {
                note(ui, "Pick what you're breeding for in Breeding → Plan and its next step shows here.");
                return;
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
                if let Ok(p) = rx.try_recv() {
                    if *k == key {
                        self.setup.home.plan = Some((key.clone(), p));
                    }
                    self.setup.home.plan_rx = None;
                }
            }
            let Some((_, plan)) = self.setup.home.plan.clone() else { return };
            if let Some(pr) = &plan.problem {
                note(ui, pr.clone());
                return;
            }
            let total = plan.steps.len();
            let finished = plan.steps.iter().filter(|s| s.done || done.contains(&s.key)).count();
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &target.species, target.shiny, r.center(), vec2(34.0, 34.0));
                ui.label(RichText::new(format!("{} · step {} of {total}", self.mon_name(&target.species), (finished + 1).min(total))).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
            });
            ui.add_space(4.0);
            kit::bar(ui, ui.available_width(), 6.0, if total > 0 { finished as f32 / total as f32 } else { 1.0 });
            ui.add_space(8.0);
            match plan.steps.iter().find(|s| !s.done && !done.contains(&s.key)) {
                Some(s) => {
                    ui.label(RichText::new(&s.title).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
                    ui.add(egui::Label::new(RichText::new(&s.detail).size(kit::fs(12.0)).color(MUTED())).wrap());
                    ui.add_space(8.0);
                    if wk::go_button(ui, "✔  Done", true) {
                        tick = Some(s.key.clone());
                    }
                }
                None => {
                    ui.label(RichText::new("✔  Every step done. Hatch away!").size(kit::fs(13.0)).color(theme::GREEN));
                }
            }
        });
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

    pub(super) fn w_hatches(&mut self, ui: &mut egui::Ui) {
        let log = self.setup.breed.file().log.clone();
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "🐥", kit::C_GOLD, "Hatch tracker", Some("Log ›"));
            ui.add_space(6.0);
            if log.is_empty() {
                note(ui, "Log your Eggs in Breeding and your hatches add up here.");
                return;
            }
            let today = crate::stats::local_day(crate::config::now_unix());
            let shinies = log.iter().filter(|h| h.shiny).count();
            let week: Vec<usize> = (0..7).map(|k| log.iter().filter(|h| crate::stats::local_day(h.when) == today - 6 + k).count()).collect();
            ui.horizontal(|ui| {
                ui.label(RichText::new(n(log.len() as u64)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                ui.label(RichText::new("Eggs hatched").size(kit::fs(12.5)).color(MUTED()));
            });
            ui.label(RichText::new(format!("✨ {shinies} shin{} · {} today", if shinies == 1 { "y" } else { "ies" }, week[6])).size(kit::fs(12.5)).color(if shinies > 0 { theme::GOLD } else { MUTED() }));
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
            let p = ui.painter();
            let max = week.iter().copied().max().unwrap_or(0).max(1) as f32;
            let slot = r.width() / 7.0;
            for (k, v) in week.iter().enumerate() {
                let cx = r.min.x + slot * (k as f32 + 0.5);
                let h = ((r.height() - 16.0) * (*v as f32 / max)).max(3.0);
                p.rect_filled(Rect::from_min_max(pos2(cx - 7.0, r.max.y - 14.0 - h), pos2(cx + 7.0, r.max.y - 14.0)), 4.0, if k == 6 { kit::C_GOLD } else { theme::blend(CARD(), kit::C_GOLD, if *v > 0 { 0.5 } else { 0.15 }) });
                let wd = crate::stats::WEEKDAYS[crate::stats::weekday(today - 6 + k as i64)];
                p.text(pos2(cx, r.max.y - 4.0), Align2::CENTER_CENTER, &wd[..1], crate::fonts::sans_font(10.0), MUTED());
            }
        });
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

    /// One Pokémon's best hit on another, with their usual sets.
    pub(super) fn w_quickcalc(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let world = World::Platinum;
        let (atk, def) = if preview || w.data.len() < 2 { ("Garchomp".to_string(), "Metagross".to_string()) } else { (w.data[0].clone(), w.data[1].clone()) };
        let editing = ui.ctx().data(|d| d.get_temp::<u8>(egui::Id::new(("w-qc-edit", w.id)))).unwrap_or(0);
        let wiki = self.wiki.clone();
        let mut new: Option<(String, String)> = None;
        let mut save = false;
        let mut open = false;
        let mut set_edit: Option<u8> = None;
        kit::card(ui, |ui| {
            open = head(ui, "🔢", kit::C_ORANGE, "Quick calc", Some("Damage Calc ›"));
            ui.add_space(8.0);
            let a = default_set(&atk, world);
            let d = default_set(&def, world);
            // the two, with swap
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 74.0), Sense::hover());
            let p = ui.painter().clone();
            let half = r.width() / 2.0;
            for (k, (m, x)) in [(&a, r.min.x + half / 2.0), (&d, r.max.x - half / 2.0)].into_iter().enumerate() {
                let br = Rect::from_center_size(pos2(x, r.center().y), vec2(half - 40.0, 70.0));
                let resp = ui.interact(br, ui.id().with(("w-qc-side", w.id, k)), Sense::click());
                let on = editing == k as u8 + 1;
                p.rect_filled(br, 12.0, theme::blend(CARD(), if k == 0 { kit::C_ORANGE } else { kit::C_BLUE }, if on || resp.hovered() { 0.2 } else { 0.1 }));
                self.team_sprite(&p, &m.species, false, pos2(br.center().x, br.min.y + 28.0), vec2(48.0, 48.0));
                p.text(pos2(br.center().x, br.max.y - 12.0), Align2::CENTER_CENTER, &m.species, crate::fonts::sans_bold_font(12.0), TEXT());
                if resp.on_hover_text("Change").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !preview {
                    set_edit = Some(if on { 0 } else { k as u8 + 1 });
                }
            }
            // swap: a round button with two arrows
            let sw = Rect::from_center_size(r.center(), vec2(30.0, 30.0));
            let sresp = ui.interact(sw, ui.id().with(("w-qc-swap", w.id)), Sense::click());
            let ht = kit::anim(ui.ctx(), sresp.id.with("h"), sresp.hovered(), kit::QUICK);
            p.circle_filled(sw.center(), 15.0, theme::blend(kit::btn_fill(), kit::primary(), 0.3 * ht));
            p.circle_stroke(sw.center(), 15.0, Stroke::new(1.0f32, LINE()));
            let st = Stroke::new(1.6f32, TEXT());
            let c = sw.center();
            for (y, dir) in [(-4.0, 1.0), (4.0, -1.0)] {
                let a = c + vec2(-7.0 * dir, y);
                let b = c + vec2(7.0 * dir, y);
                p.line_segment([a, b], st);
                p.line_segment([b, b + vec2(-3.5 * dir, -3.0)], st);
                p.line_segment([b, b + vec2(-3.5 * dir, 3.0)], st);
            }
            if sresp.on_hover_text("Swap sides").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !preview {
                new = Some((def.clone(), atk.clone()));
            }
            if editing > 0 && !preview {
                ui.add_space(6.0);
                if let Some(id) = self.mon_picker(ui, w.id, "qc", if editing == 1 { "🔍  Attacker" } else { "🔍  Defender" }) {
                    let name = wiki.as_ref().and_then(|wk| wk.mon(&id)).map(|m| m.name.clone()).unwrap_or(id);
                    new = Some(if editing == 1 { (name, def.clone()) } else { (atk.clone(), name) });
                    set_edit = Some(0);
                }
            }
            ui.add_space(8.0);
            for (x, y) in [(&a, &d), (&d, &a)] {
                match best_hit(wiki.as_deref(), x, y, world) {
                    Some(h) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{} {} →", x.species, h.mv)).size(kit::fs(12.5)).color(TEXT()));
                            ui.label(RichText::new(format!("{:.0}–{:.0}%", h.lo, h.hi)).font(crate::fonts::sans_bold_font(13.5)).color(super::charts::ink(hit_col(h.hi))));
                        });
                        if !h.ko.is_empty() {
                            ui.label(RichText::new(&h.ko).size(kit::fs(11.5)).color(MUTED()));
                        }
                    }
                    None => note(ui, format!("{} has no damaging moves in its set.", x.species)),
                }
                ui.add_space(4.0);
            }
            note(ui, "With their usual sets, Lv 50.");
            ui.add_space(4.0);
            save = wk::link(ui, "⭐  Save this calc");
        });
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

    pub(super) fn w_savedcalcs(&mut self, ui: &mut egui::Ui) {
        let world = World::Platinum;
        let saved = self.cfg.home.saved_calcs.clone();
        let wiki = self.wiki.clone();
        let mut open: Option<(String, String)> = None;
        let mut remove: Option<usize> = None;
        kit::card(ui, |ui| {
            head(ui, "⭐", kit::C_GOLD, "Saved calcs", None);
            ui.add_space(6.0);
            if saved.is_empty() {
                note(ui, "Press “⭐ Save this calc” in a Quick calc widget and it's kept here.");
                return;
            }
            for (i, k) in saved.iter().enumerate().take(6) {
                let Some((a, d)) = k.split_once('|') else { continue };
                let hit = best_hit(wiki.as_deref(), &default_set(a, world), &default_set(d, world), world);
                let (r, resp) = row(ui, 40.0);
                let p = ui.painter().clone();
                self.team_sprite(&p, a, false, pos2(r.min.x + 16.0, r.center().y), vec2(32.0, 32.0));
                self.team_sprite(&p, d, false, pos2(r.min.x + 46.0, r.center().y), vec2(32.0, 32.0));
                p.text(pos2(r.min.x + 70.0, r.center().y - 8.0), Align2::LEFT_CENTER, format!("{a} vs {d}"), crate::fonts::sans_bold_font(12.0), TEXT());
                if let Some(h) = &hit {
                    p.text(pos2(r.min.x + 70.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{} {:.0}–{:.0}%", h.mv, h.lo, h.hi), crate::fonts::sans_font(11.0), super::charts::ink(hit_col(h.hi)));
                }
                let resp = resp.on_hover_text("Open in the Damage Calc · right-click to remove");
                if resp.clicked() {
                    open = Some((a.to_string(), d.to_string()));
                }
                resp.context_menu(|ui| {
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                        ui.close_menu();
                    }
                });
            }
        });
        if let Some(i) = remove {
            self.cfg.home.saved_calcs.remove(i);
            self.dirty = true;
        }
        if let Some((a, d)) = open {
            self.calc_open(&a, &d, world);
        }
    }

    /// Your main team's Speed against well-known fast Pokémon.
    pub(super) fn w_speedtiers(&mut self, ui: &mut egui::Ui) {
        let team = self.dash_team();
        kit::card(ui, |ui| {
            head(ui, "💨", kit::C_CYAN, "Speed tiers", None);
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let fs = self.team_facts(&t);
            let level = t.members.iter().map(|m| m.level).max().unwrap_or(50);
            let d = battle::data();
            let jolly = d.nature("Jolly");
            let mut rows: Vec<(String, u16, Option<String>)> = Vec::new();
            for f in fs.iter().flatten() {
                let s = if f.item == "Choice Scarf" { f.stats[5] * 3 / 2 } else { f.stats[5] };
                rows.push((f.name.clone(), s, Some(f.species.clone())));
            }
            for (name, b) in BENCH.iter().take(6) {
                rows.push((format!("{name} (max)"), battle::stat(5, *b, 31, 252, level, jolly, false), None));
            }
            rows.sort_by(|a, b| b.1.cmp(&a.1));
            let top = rows.first().map(|r| r.1).unwrap_or(1).max(1) as f32;
            for (name, s, sp) in &rows {
                let mine = sp.is_some();
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), if mine { 26.0 } else { 18.0 }), Sense::hover());
                let p = ui.painter().clone();
                let name_w = (r.width() * 0.45).min(150.0);
                if let Some(sp) = sp {
                    self.team_sprite(&p, sp, false, pos2(r.min.x + 12.0, r.center().y), vec2(24.0, 24.0));
                }
                p.with_clip_rect(r.with_max_x(r.min.x + name_w)).text(pos2(r.min.x + if mine { 28.0 } else { 4.0 }, r.center().y), Align2::LEFT_CENTER, name, if mine { crate::fonts::sans_bold_font(12.0) } else { crate::fonts::sans_font(11.0) }, if mine { TEXT() } else { MUTED() });
                let bw = (r.width() - name_w - 40.0) * (*s as f32 / top);
                let bar = Rect::from_min_size(pos2(r.min.x + name_w, r.center().y - if mine { 5.0 } else { 3.0 }), vec2(bw.max(2.0), if mine { 10.0 } else { 6.0 }));
                p.rect_filled(bar, 4.0, if mine { kit::C_CYAN } else { theme::blend(kit::sunken(), MUTED(), 0.35) });
                p.text(pos2(bar.max.x + 6.0, r.center().y), Align2::LEFT_CENTER, s.to_string(), crate::fonts::sans_font(11.0), if mine { TEXT() } else { MUTED() });
            }
            ui.add_space(4.0);
            note(ui, format!("Fast ones with full Speed and a +Speed nature, Lv {level}."));
        });
    }

    /// The calculation open in the Damage Calc.
    pub(super) fn w_lastcalc(&mut self, ui: &mut egui::Ui) {
        let c = &self.setup.team.calc;
        let (l, r) = (c.left.clone(), c.right.clone());
        let world = self.dash_team().map(|t| t.world).unwrap_or(World::Platinum);
        let wiki = self.wiki.clone();
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "📋", kit::C_ORANGE, "Last result", Some("Open ›"));
            ui.add_space(6.0);
            let (Some(l), Some(r)) = (l, r) else {
                note(ui, "Pick two Pokémon in the Damage Calc and the result shows here.");
                return;
            };
            let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
            let p = ui.painter().clone();
            self.team_sprite(&p, &l.set.species, l.set.shiny, pos2(rr.min.x + 26.0, rr.center().y), vec2(46.0, 46.0));
            self.team_sprite(&p, &r.set.species, r.set.shiny, pos2(rr.max.x - 26.0, rr.center().y), vec2(46.0, 46.0));
            p.text(rr.center(), Align2::CENTER_CENTER, "vs", crate::fonts::sans_bold_font(12.0), MUTED());
            for (x, y) in [(&l.set, &r.set), (&r.set, &l.set)] {
                match best_hit(wiki.as_deref(), x, y, world) {
                    Some(h) => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("{}'s {}", x.display_name(), h.mv)).size(kit::fs(12.0)).color(TEXT()));
                            ui.label(RichText::new(format!("{:.0}–{:.0}%", h.lo, h.hi)).font(crate::fonts::sans_bold_font(13.0)).color(super::charts::ink(hit_col(h.hi))));
                        });
                        if !h.ko.is_empty() {
                            ui.label(RichText::new(&h.ko).size(kit::fs(11.0)).color(MUTED()));
                        }
                    }
                    None => note(ui, format!("{} has no damaging moves.", x.display_name())),
                }
                ui.add_space(3.0);
            }
        });
        if open {
            self.go_to(Tab::Calc);
        }
    }

    /// Your main team against one Pokémon you pick.
    pub(super) fn w_threat(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let team = self.dash_team();
        let world = team.as_ref().map(|t| t.world).unwrap_or(World::Platinum);
        let foe_name = if preview { Some("Garchomp".to_string()) } else { w.data.first().cloned() };
        let wiki = self.wiki.clone();
        let mut change = false;
        let mut picked: Option<String> = None;
        kit::card(ui, |ui| {
            change = head(ui, "🚨", kit::C_RED, "Threat check", foe_name.as_ref().map(|_| "Change"));
            ui.add_space(6.0);
            let Some(t) = team.clone().filter(|t| !t.members.is_empty()) else { return self.no_team(ui) };
            let Some(foe) = foe_name.clone() else {
                note(ui, "Pick a Pokémon that gives you trouble and see how your team does against it.");
                ui.add_space(6.0);
                picked = self.mon_picker(ui, w.id, "threat", "🔍  Garchomp, Gyarados…");
                return;
            };
            let fm = default_set(&foe, world);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &fm.species, false, r.center(), vec2(42.0, 42.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&fm.species).font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
                    ui.label(RichText::new(if fm.item.is_empty() { "Its usual set".to_string() } else { format!("{} · its usual set", fm.item) }).size(kit::fs(11.0)).color(MUTED()));
                });
            });
            ui.add_space(6.0);
            let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
            ui.painter().text(pos2(hr.max.x - 90.0, hr.center().y), Align2::RIGHT_CENTER, "it hits you", crate::fonts::sans_font(10.5), MUTED());
            ui.painter().text(pos2(hr.max.x, hr.center().y), Align2::RIGHT_CENTER, "you hit it", crate::fonts::sans_font(10.5), MUTED());
            let mut answers = 0;
            for m in t.members.iter().take(6) {
                let theirs = best_hit(wiki.as_deref(), &fm, m, world);
                let mine = best_hit(wiki.as_deref(), m, &fm, world);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &m.species, m.shiny, pos2(r.min.x + 14.0, r.center().y), vec2(30.0, 30.0));
                p.with_clip_rect(r.with_max_x(r.max.x - 180.0)).text(pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, m.display_name(), crate::fonts::sans_bold_font(12.0), TEXT());
                let them = theirs.as_ref().map_or("—".to_string(), |h| format!("{:.0}%", h.hi));
                let us = mine.as_ref().map_or("—".to_string(), |h| format!("{:.0}%", h.hi));
                p.text(pos2(r.max.x - 90.0, r.center().y), Align2::RIGHT_CENTER, them, crate::fonts::sans_bold_font(12.0), theirs.as_ref().map_or(MUTED(), |h| super::charts::ink(hit_col(h.hi))));
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, us, crate::fonts::sans_bold_font(12.0), mine.as_ref().map_or(MUTED(), |h| super::charts::ink(theme::blend(hit_col(100.0 - h.hi.min(100.0)), theme::GREEN, 0.0))));
                if mine.as_ref().is_some_and(|h| h.hi >= 50.0) && theirs.as_ref().map_or(true, |h| h.hi < 50.0) {
                    answers += 1;
                }
            }
            ui.add_space(4.0);
            if answers > 0 {
                ui.label(RichText::new(format!("✔  {answers} of them take it on well (hit it for 50%+, take under 50%).")).size(kit::fs(12.0)).color(theme::GREEN));
            } else {
                ui.label(RichText::new("⚠  Nobody takes it on well. Worth a plan!").size(kit::fs(12.0)).color(kit::C_ORANGE));
            }
        });
        if change && !preview {
            *out = Some(Vec::new());
        }
        if let Some(id) = picked {
            let name = self.wiki.as_ref().and_then(|wk| wk.mon(&id)).map(|m| m.name.clone()).unwrap_or(id);
            *out = Some(vec![name]);
        }
    }
}
