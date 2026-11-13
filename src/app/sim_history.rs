//! Battle Sim → the battles you've played: a list on the left, the one
//! you pick on the right with its whole battle log.

use super::sim_home::{OURS, RED};
use super::team_home::{caps, chip, now_secs, small, soft};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::sim::{BattleRecord, Format, Mode};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// The colour and words for how a battle ended.
fn result_meta(r: &str) -> (Color32, &'static str) {
    match r {
        "Won" => (kit::C_GREEN, "Won"),
        "Lost" => (kit::C_RED, "Lost"),
        "Caught" => (kit::C_GOLD, "Caught"),
        "Ran" => (kit::C_SLATE, "Got away"),
        "Fled" => (kit::C_SLATE, "It fled"),
        "Draw" => (kit::C_SLATE, "Draw"),
        _ => (kit::C_INDIGO, "Unfinished"),
    }
}

fn ago(t: u64) -> String {
    let d = now_secs().saturating_sub(t);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        86_400..=172_799 => "yesterday".into(),
        _ => format!("{} days ago", d / 86_400),
    }
}

/// "Kanto Starters vs Gym Leader Roark", "Kanto Starters vs Wild Bidoof".
fn full_title(r: &BattleRecord) -> String {
    let them = r.title.strip_prefix("vs ").unwrap_or(&r.title);
    if r.team.is_empty() {
        format!("vs {them}")
    } else {
        format!("{} vs {them}", r.team)
    }
}

/// A small pill with the result in it; returns its width.
fn pill(p: &egui::Painter, right_center: egui::Pos2, r: &str) -> f32 {
    let (col, label) = result_meta(r);
    let f = crate::fonts::sans_bold_font(11.0);
    let w = p.layout_no_wrap(label.to_string(), f.clone(), TEXT()).size().x + 18.0;
    let rect = Rect::from_center_size(right_center - egui::vec2(w / 2.0, 0.0), egui::vec2(w, 20.0));
    p.rect_filled(rect, 10.0, theme::blend(CARD(), col, 0.22));
    p.text(rect.center(), Align2::CENTER_CENTER, label, f, super::charts::ink(col));
    w
}

impl App {
    /// The battle history: every battle you've played, and its log.
    pub(super) fn sim_history(&mut self, ui: &mut egui::Ui) {
        let all: Vec<BattleRecord> = self.setup.sim.file().history.clone();
        if all.is_empty() {
            kit::card(ui, |ui| {
                ui.label(RichText::new("No battles yet").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.label(small("Every battle you play is kept here with its whole battle log: who won, how many turns, and everything that happened, turn by turn."));
            });
            return;
        }
        let sel_id = egui::Id::new("sim-history-sel");
        let filt_id = egui::Id::new("sim-history-filter");
        let mut sel: Option<u64> = ui.ctx().data(|d| d.get_temp(sel_id)).flatten();
        let mut filt: u8 = ui.ctx().data(|d| d.get_temp(filt_id)).unwrap_or(0);
        let shown: Vec<&BattleRecord> = all
            .iter()
            .filter(|r| match filt {
                1 => r.result == "Won",
                2 => r.result == "Lost",
                3 => r.result == "Caught",
                4 => !matches!(r.result.as_str(), "Won" | "Lost" | "Caught"),
                _ => true,
            })
            .collect();
        if !sel.is_some_and(|id| all.iter().any(|r| r.id == id)) {
            sel = shown.first().map(|r| r.id);
        }
        let mut delete: Option<u64> = None;
        let mut clear = false;
        let mut replay: Option<BattleRecord> = None;
        let mut load: Option<BattleRecord> = None;
        let full = ui.available_width();
        let left = 380.0;
        let gap = 16.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            // the list
            ui.allocate_ui_with_layout(egui::vec2(left, 4000.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(left);
                kit::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("BATTLES  ·  {}", all.len())).font(crate::fonts::sans_bold_font(11.0)).color(super::charts::ink(RED)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.link(RichText::new("Clear all").size(12.0)).on_hover_text("Forget every battle here").clicked() {
                                clear = true;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for (k, l) in [(0u8, "All"), (1, "Won"), (2, "Lost"), (3, "Caught"), (4, "Other")] {
                            if chip(ui, l, filt == k, RED).clicked() {
                                filt = k;
                            }
                        }
                    });
                    ui.add_space(10.0);
                    if shown.is_empty() {
                        ui.label(small("None like that yet."));
                    }
                    egui::ScrollArea::vertical().id_salt("sim-history-list").max_height(640.0).auto_shrink([false, true]).show(ui, |ui| {
                        for r in &shown {
                            if self.history_row(ui, r, sel == Some(r.id)).clicked() {
                                sel = Some(r.id);
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
            });
            // the one picked
            let w = (full - left - gap).max(300.0);
            ui.allocate_ui_with_layout(egui::vec2(w, 4000.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(w);
                if let Some(r) = sel.and_then(|id| all.iter().find(|r| r.id == id)) {
                    kit::card(ui, |ui| self.history_detail(ui, r, &mut replay, &mut load, &mut delete));
                }
            });
        });
        ui.ctx().data_mut(|d| {
            d.insert_temp(sel_id, sel);
            d.insert_temp(filt_id, filt);
        });
        if let Some(id) = delete {
            self.setup.sim.file().history.retain(|r| r.id != id);
            self.setup.sim.touch();
        }
        if clear {
            self.setup.sim.file().history.clear();
            self.setup.sim.touch();
        }
        if let Some(r) = load {
            self.setup.sim.setup = r.setup;
            self.setup.sim.note = None;
            self.setup.sim.drawer = 0;
        }
        if let Some(r) = replay {
            self.setup.sim.setup = r.setup;
            self.setup.sim.note = None;
            if let Err(e) = self.sim_begin() {
                self.setup.sim.note = Some(e);
            }
        }
    }

    /// One battle in the list.
    fn history_row(&mut self, ui: &mut egui::Ui, r: &BattleRecord, on: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 92.0), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let (col, _) = result_meta(&r.result);
        let p = ui.painter().clone();
        p.rect_filled(rect, 12.0, if on { theme::blend(CARD(), RED, 0.14) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
        if on {
            p.rect_stroke(rect, 12.0, Stroke::new(1.5f32, RED));
        }
        p.rect_filled(Rect::from_min_size(rect.min + egui::vec2(0.0, 16.0), egui::vec2(4.0, rect.height() - 32.0)), 2.0, col);
        // title and result
        let pw = pill(&p, egui::pos2(rect.max.x - 12.0, rect.min.y + 20.0), &r.result);
        let fb = crate::fonts::sans_bold_font(13.0);
        let title = full_title(r);
        p.text(egui::pos2(rect.min.x + 16.0, rect.min.y + 20.0), Align2::LEFT_CENTER, super::charts::clip(ui, &title, &fb, rect.width() - pw - 36.0), fb, TEXT());
        // turns and when
        let fmt = if r.setup.format == Format::Doubles { " · doubles" } else { "" };
        let meta = format!("{} turn{}{fmt} · {}", r.turns, if r.turns == 1 { "" } else { "s" }, ago(r.when));
        p.text(egui::pos2(rect.min.x + 16.0, rect.min.y + 39.0), Align2::LEFT_CENTER, meta, crate::fonts::sans_font(11.0), MUTED());
        // both sides, small
        let y = rect.min.y + 66.0;
        let mut x = rect.min.x + 28.0;
        // squeeze them closer when both sides are big, so they stay in the card
        let count = (r.ours.len().min(6) + r.theirs.len().min(6)).max(1) as f32;
        let step = ((rect.width() - 28.0 - 32.0 - 24.0) / count).min(26.0);
        for (sp, shiny) in r.ours.iter().take(6) {
            if !self.team_sprite(&p, sp, *shiny, egui::pos2(x, y), egui::vec2(30.0, 30.0)) {
                p.circle_filled(egui::pos2(x, y), 8.0, theme::blend(CARD(), OURS, 0.2));
            }
            x += step;
        }
        let x0 = x - step + 20.0;
        x = x0;
        p.text(egui::pos2(x, y), Align2::LEFT_CENTER, "vs", crate::fonts::sans_bold_font(10.5), MUTED());
        x += 34.0;
        for (sp, shiny) in r.theirs.iter().take(6) {
            if !self.team_sprite(&p, sp, *shiny, egui::pos2(x, y), egui::vec2(30.0, 30.0)) {
                p.circle_filled(egui::pos2(x, y), 8.0, theme::blend(CARD(), RED, 0.2));
            }
            x += step;
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// The battle picked: how it went, both teams, and its log.
    fn history_detail(&mut self, ui: &mut egui::Ui, r: &BattleRecord, replay: &mut Option<BattleRecord>, load: &mut Option<BattleRecord>, delete: &mut Option<u64>) {
        let (col, label) = result_meta(&r.result);
        // header
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 6.0, col);
            ui.label(RichText::new(label).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
            ui.label(RichText::new(format!("  {}", r.title)).font(crate::fonts::sans_bold_font(16.0)).color(MUTED()));
        });
        let mode = match r.setup.mode {
            Mode::Trainer => "Trainer battle",
            _ => "Wild battle",
        };
        let fmt = if r.setup.format == Format::Doubles { "Doubles" } else { "Singles" };
        ui.label(small(format!("{mode} · {fmt} · {} turn{} · {} · {}", r.turns, if r.turns == 1 { "" } else { "s" }, r.sub, ago(r.when))));
        ui.add_space(12.0);
        // both teams
        let (band, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 84.0), Sense::hover());
        let p = ui.painter().clone();
        let mid = band.center().x;
        p.rect_filled(Rect::from_min_max(band.min, egui::pos2(mid, band.max.y)), egui::Rounding { nw: 12.0, sw: 12.0, ne: 0.0, se: 0.0 }, theme::blend(kit::sunken(), OURS, 0.12));
        p.rect_filled(Rect::from_min_max(egui::pos2(mid, band.min.y), band.max), egui::Rounding { nw: 0.0, sw: 0.0, ne: 12.0, se: 12.0 }, theme::blend(kit::sunken(), RED, 0.12));
        let f = crate::fonts::sans_bold_font(11.0);
        p.text(band.min + egui::vec2(14.0, 14.0), Align2::LEFT_CENTER, if r.team.is_empty() { "YOUR TEAM".to_string() } else { r.team.to_uppercase() }, f.clone(), super::charts::ink(OURS));
        p.text(egui::pos2(band.max.x - 14.0, band.min.y + 14.0), Align2::RIGHT_CENTER, "THEIR SIDE", f, super::charts::ink(RED));
        let step = ((mid - band.min.x - 60.0) / 6.0).min(52.0);
        for (i, (sp, shiny)) in r.ours.iter().take(6).enumerate() {
            let c = egui::pos2(band.min.x + 36.0 + step * i as f32, band.min.y + 52.0);
            if !self.team_sprite(&p, sp, *shiny, c, egui::vec2(50.0, 50.0)) {
                p.circle_filled(c, 14.0, theme::blend(CARD(), OURS, 0.2));
            }
        }
        for (i, (sp, shiny)) in r.theirs.iter().take(6).enumerate() {
            let c = egui::pos2(band.max.x - 36.0 - step * i as f32, band.min.y + 52.0);
            if !self.team_sprite(&p, sp, *shiny, c, egui::vec2(50.0, 50.0)) {
                p.circle_filled(c, 14.0, theme::blend(CARD(), RED, 0.2));
            }
        }
        p.circle_filled(egui::pos2(mid, band.center().y), 17.0, CARD());
        p.circle_stroke(egui::pos2(mid, band.center().y), 17.0, Stroke::new(1.0f32, LINE()));
        p.text(egui::pos2(mid, band.center().y), Align2::CENTER_CENTER, "VS", crate::fonts::sans_bold_font(12.0), TEXT());
        ui.add_space(12.0);
        // what you can do with it
        ui.horizontal(|ui| {
            if kit::accent_button_w(ui, egui::Button::new(RichText::new("▶  Play it again").size(13.0)), 8.0, 140.0).on_hover_text("The same teams, a new battle").clicked() {
                *replay = Some(r.clone());
            }
            if soft(ui, "Load this setup").on_hover_text("Put these teams back in the setup to change something first").clicked() {
                *load = Some(r.clone());
            }
            if soft(ui, "Copy the log").clicked() {
                let text: Vec<String> = r.lines.iter().map(|(n, t)| if t.is_empty() { format!("\nTurn {n}") } else { t.clone() }).collect();
                ui.ctx().copy_text(format!("{}\n{}", full_title(r), text.join("\n")).trim().to_string());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.link(RichText::new("Delete").size(12.0)).clicked() {
                    *delete = Some(r.id);
                }
            });
        });
        ui.add_space(14.0);
        let (lr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(lr, 0.0, LINE());
        ui.add_space(12.0);
        // the log
        let qid = egui::Id::new("sim-history-q");
        let mut q: String = ui.ctx().data(|d| d.get_temp(qid)).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(caps("BATTLE LOG"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut q).hint_text("Find in the log…").desired_width(200.0));
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(qid, q.clone()));
        ui.add_space(8.0);
        if r.lines.is_empty() {
            ui.label(small("Nothing happened in this one yet."));
            return;
        }
        let ql = q.trim().to_lowercase();
        // the log in turns: what happened before turn 1 (sending out,
        // abilities) is "Start"
        let mut turns: Vec<(u32, Vec<&String>)> = vec![(0, Vec::new())];
        for (n, t) in &r.lines {
            if t.is_empty() {
                turns.push((*n, Vec::new()));
            } else if let Some(last) = turns.last_mut() {
                last.1.push(t);
            }
        }
        turns.retain(|(_, l)| !l.is_empty());
        // searching: every matching line, with its turn
        if !ql.is_empty() {
            let hits: Vec<(u32, &String)> = turns.iter().flat_map(|(n, l)| l.iter().filter(|t| t.to_lowercase().contains(&ql)).map(move |t| (*n, *t))).collect();
            if hits.is_empty() {
                ui.label(small("Nothing in the log matches that."));
                return;
            }
            ui.label(small(format!("{} line{} found", hits.len(), if hits.len() == 1 { "" } else { "s" })));
            ui.add_space(6.0);
            let page_id = egui::Id::new(("sim-history-page", r.id));
            for (n, t) in hits {
                ui.horizontal(|ui| {
                    let tag = if n == 0 { "Start".to_string() } else { format!("Turn {n}") };
                    if ui.add(egui::Button::new(RichText::new(tag).size(11.0)).min_size(egui::vec2(58.0, 20.0))).on_hover_text("Open this turn").clicked() {
                        let idx = turns.iter().position(|(m, _)| *m == n).unwrap_or(0);
                        ui.ctx().data_mut(|d| d.insert_temp(page_id, idx));
                        ui.ctx().data_mut(|d| d.insert_temp(qid, String::new()));
                    }
                    ui.add(egui::Label::new(RichText::new(t.as_str()).size(13.0).color(TEXT())).wrap());
                });
            }
            return;
        }
        // one turn at a time, with the turns as page numbers in one row
        let page_id = egui::Id::new(("sim-history-page", r.id));
        let mut page: usize = ui.ctx().data(|d| d.get_temp(page_id)).unwrap_or(0);
        page = page.min(turns.len() - 1);
        let label = |i: usize| if turns[i].0 == 0 { "Start".to_string() } else { turns[i].0.to_string() };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let bw = 34.0;
            let start_w = 52.0;
            // which page buttons fit: all of them, or the first, the last
            // and the ones around this page
            let room = ((ui.available_width() - 2.0 * (bw + 4.0) - start_w) / (bw + 4.0)).floor().max(5.0) as usize;
            let n = turns.len();
            let mut shown: Vec<Option<usize>> = Vec::new();
            if n <= room + 1 {
                shown = (0..n).map(Some).collect();
            } else {
                let side = (room.saturating_sub(4)) / 2;
                let lo = page.saturating_sub(side).max(1);
                let hi = (lo + room.saturating_sub(4)).min(n - 2);
                let lo = hi.saturating_sub(room.saturating_sub(4)).max(1);
                shown.push(Some(0));
                if lo > 1 {
                    shown.push(None);
                }
                shown.extend((lo..=hi).map(Some));
                if hi < n - 2 {
                    shown.push(None);
                }
                shown.push(Some(n - 1));
            }
            let pager = |ui: &mut egui::Ui, text: &str, w: f32, on: bool, enabled: bool, col: Color32| -> bool {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 30.0), if enabled { Sense::click() } else { Sense::hover() });
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && enabled, kit::QUICK);
                let p = ui.painter();
                let fill = if on { theme::blend(CARD(), col, 0.55) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
                p.rect_filled(r, 8.0, fill);
                p.rect_stroke(r, 8.0, Stroke::new(1.0f32, if on { col } else { theme::blend(LINE(), col, 0.5 * ht) }));
                let tc = if !enabled { MUTED().gamma_multiply(0.5) } else if on { Color32::WHITE } else { TEXT() };
                p.text(r.center(), Align2::CENTER_CENTER, text, crate::fonts::sans_bold_font(12.0), tc);
                enabled && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
            };
            if pager(ui, "‹", bw, false, page > 0, kit::C_INDIGO) {
                page -= 1;
            }
            for s in &shown {
                match s {
                    Some(i) => {
                        let last = turns[*i].0 == r.turns && !r.result.is_empty();
                        let col = if last { result_meta(&r.result).0 } else { kit::C_INDIGO };
                        let w = if turns[*i].0 == 0 { start_w } else { bw };
                        if pager(ui, &label(*i), w, page == *i, true, col) {
                            page = *i;
                        }
                    }
                    None => {
                        ui.label(RichText::new("…").size(12.0).color(MUTED()));
                    }
                }
            }
            if pager(ui, "›", bw, false, page + 1 < turns.len(), kit::C_INDIGO) {
                page += 1;
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(page_id, page));
        ui.add_space(10.0);
        // the page
        let (n, lines) = &turns[page];
        let last = *n == r.turns && !r.result.is_empty();
        let col = if *n == 0 { kit::C_SLATE } else if last { result_meta(&r.result).0 } else { kit::C_INDIGO };
        egui::Frame::none()
            .fill(theme::blend(kit::sunken(), col, 0.06))
            .stroke(Stroke::new(1.0f32, theme::blend(LINE(), col, 0.3)))
            .rounding(12.0)
            .inner_margin(egui::Margin::symmetric(16.0, 14.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let title = if *n == 0 { "BEFORE TURN 1".to_string() } else { format!("TURN {n}") };
                    ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(12.0)).color(super::charts::ink(col)));
                    if last {
                        ui.label(RichText::new(format!("·  {}", result_meta(&r.result).1)).font(crate::fonts::sans_bold_font(12.0)).color(super::charts::ink(col)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(small(format!("page {} of {}", page + 1, turns.len())));
                    });
                });
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.y = 6.0;
                for t in lines {
                    let low = t.to_lowercase();
                    let c = if low.contains("fainted") {
                        super::charts::ink(kit::C_RED)
                    } else if low.contains("critical hit") || low.contains("super effective") {
                        super::charts::ink(kit::C_GOLD)
                    } else if low.contains("not very effective") || low.contains("missed") || low.contains("doesn't affect") {
                        MUTED()
                    } else {
                        TEXT()
                    };
                    ui.add(egui::Label::new(RichText::new(t.as_str()).size(13.5).color(c)).wrap());
                }
            });
    }
}
