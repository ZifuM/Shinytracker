//! Battle Sim's setup pages: the three activity cards on top (wild battle,
//! trainer battle, catch test) and the versus page wild and trainer battles
//! use. The catch test's page is in `sim_catch`.

use super::sim_home::{thousands, SimView, OURS, RED};
use super::team_home::{chip, small};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::sim::{Format, Mode};
use crate::teams::Member;
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// The three things you can do here.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Activity {
    Wild,
    Trainer,
    Catch,
}

/// 2 → "2nd", 3 → "3rd"…
pub(super) fn nth(n: usize) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        _ => format!("{n}th"),
    }
}

/// Who's battling, for the summaries: your picks in order, and theirs.
struct Sides {
    team: String,
    ours: Vec<Member>,
    them: String,
    theirs: Vec<Member>,
}

impl App {
    fn sim_sides(&mut self) -> Sides {
        let s = self.setup.sim.setup.clone();
        let team = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned());
        let (team_name, ours) = match &team {
            Some(t) => (if t.name.trim().is_empty() { "Untitled team".to_string() } else { t.name.clone() }, s.picks.iter().filter_map(|i| t.members.get(*i).cloned()).collect()),
            None => ("No team picked".to_string(), Vec::new()),
        };
        let (them, theirs) = match s.mode {
            Mode::Trainer => match s.trainer.and_then(|id| self.setup.sim.file().trainers.iter().find(|t| t.id == id).cloned()) {
                Some(t) => (t.name.clone(), t.members.clone()),
                None => ("No trainer picked".to_string(), Vec::new()),
            },
            _ => {
                let n = if s.format == Format::Doubles { 2 } else { 1 };
                let f: Vec<Member> = s.foes.iter().take(n).cloned().collect();
                let name = if f.len() > 1 { "Two wild Pokémon".to_string() } else { f.first().map(|m| format!("Wild {} · Lv {}", m.display_name(), m.level)).unwrap_or_else(|| "A wild Pokémon".into()) };
                (name, f)
            }
        };
        Sides { team: team_name, ours, them, theirs }
    }

    /// A row of sprites, `size` points each (faces left when `flip` is set,
    /// with the leads lit and the rest dimmer).
    fn sim_lineup(&mut self, ui: &mut egui::Ui, mons: &[Member], size: f32, leads: usize, col: Color32, align_right: bool) {
        let gap = 6.0;
        let n = mons.len().max(1);
        let w = (size + gap) * n as f32 - gap;
        let full = ui.available_width();
        let (row, _) = ui.allocate_exact_size(egui::vec2(full, size + 4.0), Sense::hover());
        let x0 = if align_right { row.max.x - w } else { row.min.x };
        let p = ui.painter().clone();
        if mons.is_empty() {
            let r = Rect::from_min_size(egui::pos2(x0, row.min.y), egui::vec2(size, size));
            p.rect_stroke(r, 12.0, Stroke::new(1.0f32, LINE()));
            p.text(r.center(), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(size * 0.35), MUTED());
            return;
        }
        for (i, m) in mons.iter().enumerate() {
            let r = Rect::from_min_size(egui::pos2(x0 + (size + gap) * i as f32, row.min.y), egui::vec2(size, size));
            let lead = i < leads;
            p.rect_filled(r, 12.0, if lead { theme::blend(CARD(), col, 0.16) } else { kit::sunken() });
            if lead {
                p.rect_stroke(r, 12.0, Stroke::new(1.5f32, theme::blend(LINE(), col, 0.8)));
            }
            if !self.team_sprite(&p, &m.species, m.shiny, r.center(), egui::vec2(size - 6.0, size - 6.0)) {
                p.text(r.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(12.0), MUTED());
            }
            if !lead {
                p.rect_filled(r, 12.0, CARD().gamma_multiply(0.35));
            }
        }
    }

    /// Your whole team in the banner: the ones coming along are lit with
    /// their order (the first `leads` lead), the rest dimmed; click one to
    /// bring it along or leave it out.
    fn sim_pick_lineup(&mut self, ui: &mut egui::Ui, members: &[Member], size: f32, leads: usize) {
        let gap = 6.0;
        if members.is_empty() {
            let (r, _) = ui.allocate_exact_size(egui::vec2(size, size + 4.0), Sense::hover());
            let r = Rect::from_min_size(r.min, egui::vec2(size, size));
            ui.painter().rect_stroke(r, 12.0, Stroke::new(1.0f32, LINE()));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(size * 0.35), MUTED());
            return;
        }
        let picks = self.setup.sim.setup.picks.clone();
        let mut toggle: Option<usize> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (i, m) in members.iter().enumerate().take(6) {
                let order = picks.iter().position(|p| *p == i);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(size, size + 4.0), Sense::click());
                let r = Rect::from_min_size(r.min, egui::vec2(size, size));
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                let lead = order.is_some_and(|o| o < leads);
                let edge = match order {
                    Some(_) if lead => OURS,
                    Some(_) => theme::blend(OURS, MUTED(), 0.45),
                    None => theme::blend(LINE(), OURS, 0.5 * ht),
                };
                p.rect_filled(r, 12.0, if order.is_some() { theme::blend(CARD(), OURS, 0.16) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                p.rect_stroke(r, 12.0, Stroke::new(if lead { 1.5f32 } else { 1.0 }, edge));
                if !self.team_sprite(&p, &m.species, m.shiny, r.center(), egui::vec2(size - 6.0, size - 6.0)) {
                    p.text(r.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(12.0), MUTED());
                }
                if order.is_none() {
                    p.rect_filled(r, 12.0, CARD().gamma_multiply(0.55 - 0.25 * ht));
                }
                if let Some(o) = order {
                    let rad = if size < 70.0 { 8.0 } else { 10.0 };
                    let c = egui::pos2(r.max.x - rad - 2.0, r.min.y + rad + 2.0);
                    p.circle_filled(c, rad, if lead { OURS } else { theme::blend(CARD(), MUTED(), 0.5) });
                    p.text(c, Align2::CENTER_CENTER, (o + 1).to_string(), crate::fonts::sans_bold_font(rad + 1.0), Color32::WHITE);
                }
                let tip = match order {
                    Some(o) if o < leads => format!("{} leads. Click to leave it out.", m.display_name()),
                    Some(o) => format!("{} comes out {}. Click to leave it out.", m.display_name(), super::sim_layouts::nth(o + 1)),
                    None => format!("Click to bring {} along", m.display_name()),
                };
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
                    toggle = Some(i);
                }
            }
        });
        if let Some(i) = toggle {
            let picks = &mut self.setup.sim.setup.picks;
            if let Some(p) = picks.iter().position(|x| *x == i) {
                picks.remove(p);
            } else {
                picks.push(i);
            }
        }
    }

    /// The Auto-sim tab while it's switched off: what's coming.
    fn sim_auto_soon(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            let col = kit::C_INDIGO;
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, "⚡", col, false);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Auto-sim").font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
                        // the "coming soon" pill
                        let f = crate::fonts::sans_bold_font(10.5);
                        let g = ui.painter().layout_no_wrap("COMING SOON".into(), f, Color32::WHITE);
                        let (pr, _) = ui.allocate_exact_size(g.size() + egui::vec2(18.0, 8.0), Sense::hover());
                        ui.painter().rect_filled(pr, pr.height() / 2.0, theme::blend(CARD(), kit::C_GOLD, 0.25));
                        ui.painter().rect_stroke(pr, pr.height() / 2.0, Stroke::new(1.0f32, kit::C_GOLD));
                        ui.painter().galley(pr.min + egui::vec2(9.0, 4.0), g, super::charts::ink(kit::C_GOLD));
                    });
                    ui.label(small("The AI plays your setup on both sides hundreds of times over, and tells you how it really goes."));
                });
            });
            ui.add_space(16.0);
            // what it'll show, as faded preview tiles
            let parts = [
                ("🏆", kit::C_GREEN, "Your win rate", "How often you win, and how sure that number is."),
                ("⏱", kit::C_CYAN, "How long it takes", "Turns per battle, and how many of your Pokémon are left standing."),
                ("⭐", kit::C_GOLD, "Who does the work", "Damage and KOs for each of yours, and which of theirs is the problem."),
            ];
            let gap = 12.0;
            let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (icon, c, title, text) in parts {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(w, 112.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(r, 12.0, theme::blend(kit::sunken(), c, 0.06));
                    p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), c, 0.2)));
                    kit::icon_chip(p, Rect::from_min_size(r.min + egui::vec2(14.0, 14.0), egui::vec2(30.0, 30.0)), icon, c, false);
                    p.text(r.min + egui::vec2(52.0, 29.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(14.0), TEXT());
                    let g = p.layout(text.to_string(), crate::fonts::sans_font(12.0), MUTED(), w - 28.0);
                    p.galley(r.min + egui::vec2(14.0, 56.0), g, MUTED());
                }
            });
            ui.add_space(12.0);
            ui.label(small("Until then, press Start battle to play it yourself: every battle you play is kept under Battles."));
            ui.add_space(4.0);
        });
    }

    /// Start and simulate: stacked full width (`stack`) or in a row.
    fn sim_actions(&mut self, ui: &mut egui::Ui, stack: bool) {
        let ready = self.sim_ready();
        let n_auto = super::sim_auto::auto_count(self.setup.sim.auto_n);
        let (mut start, mut auto) = (false, false);
        let w = if stack { ui.available_width() } else { 150.0 };
        let mut lay = |ui: &mut egui::Ui| {
            let b = ui.add_enabled_ui(ready.is_ok(), |ui| kit::accent_button_w(ui, egui::Button::new(RichText::new("▶  Start battle").size(14.0)), 8.0, w)).inner;
            start = match &ready {
                Err(why) => b.on_disabled_hover_text(why.as_str()),
                Ok(()) => b.on_hover_text("Play this battle yourself"),
            }
            .clicked();
            let live = super::sim_auto::READY;
            let a = ui.add_enabled(ready.is_ok() && live, egui::Button::new(RichText::new(format!("⚡  Simulate {}", thousands(n_auto))).size(13.0)).fill(kit::btn_fill()).min_size(egui::vec2(w, 34.0)));
            auto = if live { a.on_hover_text("Both sides played by the AI, many times over: your win rate and who does what") } else { a.on_disabled_hover_text("Coming soon") }.clicked();
        };
        if stack {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                lay(ui);
            });
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                lay(ui);
            });
        }
        if let Err(why) = &ready {
            ui.add_space(4.0);
            ui.label(small(format!("To start: {}", why.to_lowercase())));
        } else if let Some(n) = &self.setup.sim.note {
            ui.add_space(4.0);
            ui.add(egui::Label::new(small(n.clone())).wrap());
        }
        if start {
            self.sim_start();
        }
        if auto {
            self.sim_auto_start();
        }
    }

    /// Singles or doubles.
    fn sim_format_chips(&mut self, ui: &mut egui::Ui) {
        let mut format = self.setup.sim.setup.format;
        // exactly as wide as the two chips, so a centred layout centres them
        let f = crate::fonts::sans_bold_font(12.0);
        let wid = |t: &str| ui.painter().layout_no_wrap(t.to_string(), f.clone(), TEXT()).size().x + 20.0;
        let w = wid("Singles") + 6.0 + wid("Doubles");
        ui.allocate_ui_with_layout(egui::vec2(w, 28.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (v, l) in [(Format::Singles, "Singles"), (Format::Doubles, "Doubles")] {
                if chip(ui, l, format == v, OURS).clicked() {
                    format = v;
                }
            }
        });
        if format != self.setup.sim.setup.format {
            self.setup.sim.setup.format = format;
            self.setup.sim.note = None;
        }
    }

    /// Which activity is open.
    pub(super) fn sim_activity(&self) -> Activity {
        if self.setup.sim.view == SimView::Catch {
            Activity::Catch
        } else if self.setup.sim.setup.mode == Mode::Trainer {
            Activity::Trainer
        } else {
            Activity::Wild
        }
    }

    /// The three activity cards: the open one lit in its colour.
    pub(super) fn sim_activity_cards(&mut self, ui: &mut egui::Ui) {
        let cur = self.sim_activity();
        let acts = [
            (Activity::Wild, "🌿", "Wild battle", "Pick a route, roll a wild Pokémon by the game's odds and battle it with your team.", kit::C_GREEN),
            (Activity::Trainer, "🏅", "Trainer battle", "Build a trainer or turn one of your teams into one, then fight their whole team.", kit::C_GOLD),
            (Activity::Catch, "🎯", "Catch test", "See every ball's odds as its HP drops, then weaken it and throw for real.", kit::C_VIOLET),
        ];
        let gap = 16.0;
        let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
        let mut pick: Option<Activity> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (a, icon, title, sub, col) in acts {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 150.0), Sense::click());
                let on = cur == a;
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && !on, kit::QUICK);
                let r = r.translate(egui::vec2(0.0, -2.0 * ht));
                let p = ui.painter();
                p.add(kit::card_shadow().as_shape(r, 18.0));
                let fill = if on { theme::blend(CARD(), col, 0.16) } else { theme::blend(theme::blend(CARD(), CARD_HOVER(), ht), col, 0.04 + 0.06 * ht) };
                p.rect_filled(r, 18.0, fill);
                p.rect_stroke(r, 18.0, Stroke::new(if on { 2.0f32 } else { 1.0 + ht }, if on { col } else { theme::blend(LINE(), col, 0.25 + 0.5 * ht) }));
                kit::icon_chip(p, Rect::from_min_size(r.min + egui::vec2(20.0, 20.0), egui::vec2(42.0, 42.0)), icon, col, true);
                p.text(r.min + egui::vec2(20.0, 84.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(18.0), TEXT());
                let g = p.layout(sub.to_string(), crate::fonts::sans_font(12.5), MUTED(), w - 40.0);
                p.galley(r.min + egui::vec2(20.0, 100.0), g, MUTED());
                let tag = if on { "●  Open" } else { "Start  ›" };
                p.text(egui::pos2(r.max.x - 20.0, r.min.y + 41.0), Align2::RIGHT_CENTER, tag, crate::fonts::sans_bold_font(12.5), super::charts::ink(col));
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(a);
                }
            }
        });
        if let Some(a) = pick {
            match a {
                Activity::Catch => self.setup.sim.view = SimView::Catch,
                Activity::Trainer => {
                    self.setup.sim.view = SimView::Battle;
                    self.setup.sim.setup.mode = Mode::Trainer;
                }
                Activity::Wild => {
                    self.setup.sim.view = SimView::Battle;
                    self.setup.sim.setup.mode = Mode::Wild;
                }
            }
            self.setup.sim.note = None;
        }
    }

    // ---------------------------------------------------------------
    // The versus page: a banner with both sides, one drawer of details.

    pub(super) fn sim_versus(&mut self, ui: &mut egui::Ui) {
        let sd = self.sim_sides();
        let leads = if self.setup.sim.setup.format == Format::Doubles { 2 } else { 1 };
        let mut open: Option<u8> = None;
        let full = ui.available_width();
        let h = 262.0;
        let (banner, _) = ui.allocate_exact_size(egui::vec2(full, h), Sense::hover());
        {
            let p = ui.painter();
            p.add(kit::card_shadow().as_shape(banner, 18.0));
            p.rect_filled(banner, 18.0, CARD());
            // the two halves, tinted
            let half = banner.width() / 2.0;
            let l = Rect::from_min_size(banner.min, egui::vec2(half, h));
            let r = Rect::from_min_size(egui::pos2(banner.min.x + half, banner.min.y), egui::vec2(half, h));
            p.rect_filled(l, egui::Rounding { nw: 18.0, sw: 18.0, ne: 0.0, se: 0.0 }, theme::blend(CARD(), OURS, 0.08));
            p.rect_filled(r, egui::Rounding { nw: 0.0, sw: 0.0, ne: 18.0, se: 18.0 }, theme::blend(CARD(), RED, 0.08));
            p.rect_stroke(banner, 18.0, Stroke::new(1.0f32, LINE()));
        }
        let half = banner.width() / 2.0;
        let mid = 190.0;
        for side in 0..2 {
            let ours = side == 0;
            let x = if ours { banner.min.x } else { banner.min.x + half + mid / 2.0 };
            let area = Rect::from_min_size(egui::pos2(x + 24.0, banner.min.y + 20.0), egui::vec2(half - mid / 2.0 - 48.0, h - 40.0));
            // their side opens its drawer tab; on yours, the Pokémon are buttons
            let resp = ui.interact(Rect::from_min_size(egui::pos2(if ours { banner.min.x } else { banner.min.x + half + mid / 2.0 }, banner.min.y), egui::vec2(half - mid / 2.0, h)), ui.id().with(("banner", side)), if ours { Sense::hover() } else { Sense::click() });
            let hv = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area).layout(egui::Layout::top_down(if ours { egui::Align::LEFT } else { egui::Align::RIGHT })));
            let col = if ours { OURS } else { RED };
            child.label(RichText::new(if ours { "YOUR SIDE" } else { "THEIR SIDE" }).font(crate::fonts::sans_bold_font(11.0)).color(super::charts::ink(col)));
            child.add_space(2.0);
            child.label(RichText::new(if ours { &sd.team } else { &sd.them }).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
            child.add_space(10.0);
            let mons = if ours { sd.ours.clone() } else { sd.theirs.clone() };
            if ours {
                // the whole team: click one to bring it along or leave it out
                let team = self.setup.sim.setup.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned());
                let members = team.map(|t| t.members).unwrap_or_default();
                let size = if members.len() > 4 { 60.0 } else { 92.0 };
                self.sim_pick_lineup(&mut child, &members, size, leads);
            } else {
                let size = if mons.len() > 4 { 60.0 } else { 92.0 };
                self.sim_lineup(&mut child, &mons, size, leads, col, !ours);
            }
            child.add_space(8.0);
            let info = if mons.is_empty() {
                if ours { "Nobody picked yet".to_string() } else { "Nobody to battle yet".to_string() }
            } else if ours {
                let lead: Vec<String> = mons.iter().take(leads).map(|m| m.display_name().to_string()).collect();
                format!("{} battling · {} {}", mons.len(), lead.join(" & "), if lead.len() > 1 { "lead" } else { "leads" })
            } else if self.setup.sim.setup.mode == Mode::Trainer {
                format!("{} Pokémon", mons.len())
            } else {
                super::sim_foes::describe(&mons[0])
            };
            if ours {
                // what's picked, with All / None and the way to another team
                let n = self.setup.sim.setup.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.members.len())).unwrap_or(0);
                child.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(RichText::new(info).size(12.5).color(TEXT()));
                    if n > 0 {
                        if ui.link(RichText::new("All").size(12.0)).on_hover_text("Bring them all").clicked() {
                            self.setup.sim.setup.picks = (0..n.min(6)).collect();
                        }
                        if ui.link(RichText::new("None").size(12.0)).clicked() {
                            self.setup.sim.setup.picks.clear();
                        }
                    }
                });
                child.add_space(2.0);
                if child.link(RichText::new("Change team  ›").size(12.5).color(MUTED())).clicked() {
                    open = Some(0);
                }
            } else {
                child.label(RichText::new(info).size(12.5).color(TEXT()));
                child.add_space(2.0);
                child.label(RichText::new("Change the opponent  ›").size(12.5).color(if hv > 0.5 { TEXT() } else { MUTED() }));
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(1);
                }
            }
        }
        // the middle: VS, singles or doubles and the buttons, centred on the
        // line between the two colours (and in the banner's height)
        let mid_id = ui.id().with("versus-mid-h");
        let mid_h: f32 = ui.ctx().data(|d| d.get_temp(mid_id)).unwrap_or(h - 24.0);
        let top = (banner.center().y - mid_h / 2.0).max(banner.min.y + 8.0);
        let mr = Rect::from_min_max(egui::pos2(banner.center().x - mid / 2.0, top), egui::pos2(banner.center().x + mid / 2.0, banner.max.y - 8.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(mr).layout(egui::Layout::top_down(egui::Align::Center)));
        {
            let (vr, _) = child.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
            let p = child.painter();
            let c = egui::pos2(banner.center().x, vr.center().y);
            // the circle split like the banner: green on your side, red on theirs
            let half = |from: f32, col: Color32| {
                let n = 24;
                let pts: Vec<egui::Pos2> = (0..=n).map(|i| from + std::f32::consts::PI * i as f32 / n as f32).map(|a| c + egui::vec2(a.cos(), a.sin()) * 30.0).collect();
                egui::Shape::convex_polygon(pts, col, Stroke::NONE)
            };
            p.add(half(std::f32::consts::FRAC_PI_2, theme::blend(CARD(), OURS, 0.22)));
            p.add(half(-std::f32::consts::FRAC_PI_2, theme::blend(CARD(), RED, 0.22)));
            p.circle_stroke(c, 30.0, Stroke::new(2.0f32, theme::blend(LINE(), OURS, 0.6)));
            p.text(c, Align2::CENTER_CENTER, "VS", crate::fonts::sans_bold_font(20.0), TEXT());
        }
        child.add_space(10.0);
        self.sim_format_chips(&mut child);
        child.add_space(10.0);
        let bw = 190.0;
        child.allocate_ui_with_layout(egui::vec2(bw, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.set_width(bw);
            self.sim_actions(ui, true);
        });
        let used = child.min_rect().height();
        if (used - mid_h).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(mid_id, used));
            ui.ctx().request_repaint();
        }
        ui.add_space(14.0);
        // the drawer
        let wild = self.setup.sim.setup.mode == Mode::Wild;
        let mut tab = if open.is_some() { open.unwrap_or(0) } else { self.setup.sim.drawer };
        if tab == 4 {
            tab = 2;
        }
        let rec = self.setup.sim.file().recent.len();
        let rec_l = format!("Recent ({rec})");
        let played = self.setup.sim.file().history.len();
        let hist_l = if played > 0 { format!("Battles ({played})") } else { "Battles".to_string() };
        let tabs: Vec<(u8, &str)> = vec![(0, "Your team"), (1, if wild { "Wild Pokémon" } else { "Trainer" }), (2, hist_l.as_str()), (3, if super::sim_auto::READY { "Auto-sim" } else { "Auto-sim · soon" }), (5, rec_l.as_str())];
        kit::tab_bar(ui, &mut tab, &tabs);
        self.setup.sim.drawer = tab;
        ui.add_space(14.0);
        match tab {
            0 => self.sim_ours(ui),
            1 => {
                if wild {
                    self.ws_final(ui);
                } else {
                    self.tr_final(ui);
                }
            }
            3 if !super::sim_auto::READY => self.sim_auto_soon(ui),
            3 => {
                if self.setup.sim.auto.is_some() || self.setup.sim.auto_done.is_some() {
                    self.sim_auto_card(ui);
                } else {
                    kit::card(ui, |ui| {
                        ui.label(RichText::new("No results yet").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                        ui.label(small("Press Simulate above: the AI plays both sides many times over and you get your win rate, how long it takes and who does the work."));
                    });
                }
            }
            5 => self.sim_recent(ui),
            _ => self.sim_history(ui),
        }
    }
}
