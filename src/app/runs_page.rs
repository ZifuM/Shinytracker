//! The Runs page: every hunt in one place. A summary strip, the run being
//! hunted now, then every saved run and caught shiny as cards or a list,
//! with filters, search, sorting, pinning and undo on delete.

use super::charts::{self as ch, BLUE, CYAN, GOLD, GREEN, PURPLE};
use super::hunt_pages::RunInfo;
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::{App, RunRef, Tab};
use crate::config::{HuntRecord, SavedRun};
use crate::stats::{self, fmt_duration, local_day, now_unix};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// How long the "Deleted · Undo" bar stays.
const UNDO_SECS: f64 = 8.0;

/// A run just removed, so it can be put back.
#[derive(Clone)]
pub(super) enum Undo {
    Saved(SavedRun),
    Finished(usize, HuntRecord),
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Saved,
    Caught,
    Phase,
}

/// One run on the page, copied out so drawing can borrow `self`.
struct Row {
    r: RunRef,
    info: RunInfo,
    kind: Kind,
    /// Last time it was hunted (or when it was caught).
    last: u64,
    pinned: bool,
    secs: u64,
}

impl Row {
    fn odds(&self) -> Option<f64> {
        self.info.odds.filter(|o| *o > 1.0)
    }
    fn mult(&self) -> Option<f64> {
        self.odds().map(|o| self.info.count as f64 / o)
    }
    fn col(&self) -> Color32 {
        self.odds().map_or(PURPLE, |o| ch::luck_col(stats::found_by(self.info.count, o)))
    }
    fn id(&self) -> u64 {
        match self.r {
            RunRef::Saved(id) => id,
            _ => self.info.run_id,
        }
    }
}

/// What a click asked for; applied once drawing is done.
enum Act {
    View(RunRef),
    Share(RunRef),
    Resume(u64),
    Pin(u64),
    Delete(RunRef),
    SaveNew,
    PickTarget,
}

const SORTS: [&str; 4] = ["Recently hunted", "Most encounters", "Furthest past odds", "Name"];

impl App {
    fn run_rows(&self) -> Vec<Row> {
        let mut v = vec![];
        for s in self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id) {
            if let Some(info) = self.run_info(RunRef::Saved(s.id)) {
                v.push(Row {
                    r: RunRef::Saved(s.id),
                    kind: Kind::Saved,
                    last: s.stats.last_unix.max(s.saved_unix),
                    pinned: self.cfg.pinned_runs.contains(&s.id),
                    secs: s.stats.hunt_secs,
                    info,
                });
            }
        }
        for (i, h) in self.cfg.history.iter().enumerate() {
            if let Some(info) = self.run_info(RunRef::Finished(i)) {
                v.push(Row {
                    r: RunRef::Finished(i),
                    kind: if h.phase { Kind::Phase } else { Kind::Caught },
                    last: h.when_unix,
                    pinned: false,
                    secs: h.stats.as_ref().map_or(0, |s| s.hunt_secs),
                    info,
                });
            }
        }
        v
    }

    pub(super) fn page_runs(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "RUNS", "Every hunt you've started, saved or caught. Pick any of them back up.");
        let rows = self.run_rows();
        let mut act: Option<Act> = None;

        self.runs_summary(ui, &rows);
        ui.add_space(18.0);
        self.runs_now(ui, &mut act);
        ui.add_space(26.0);

        // ── toolbar ──
        let n_saved = rows.iter().filter(|r| r.kind == Kind::Saved).count();
        let n_caught = rows.iter().filter(|r| r.kind == Kind::Caught).count();
        let n_phase = rows.iter().filter(|r| r.kind == Kind::Phase).count();
        let labels = [format!("All  {}", rows.len()), format!("In progress  {n_saved}"), format!("Caught  {n_caught}"), format!("Phases  {n_phase}")];
        let mut opts: Vec<(u8, &str)> = vec![(0, &labels[0]), (1, &labels[1]), (2, &labels[2])];
        if n_phase > 0 {
            opts.push((3, &labels[3]));
        } else if self.setup.runs_filter == 3 {
            self.setup.runs_filter = 0;
        }
        let wide = ui.available_width() >= 860.0;
        let mut filter = self.setup.runs_filter;
        let mut sort = self.cfg.runs_sort.min(3);
        let mut list = self.cfg.runs_list;
        let mut query = std::mem::take(&mut self.setup.runs_query);
        ui.label(RichText::new("Your runs").font(ch::bold(18.0)).color(TEXT()));
        ui.add_space(10.0);
        // width of the sort box and view switch, measured last frame
        let rest_id = egui::Id::new("runs-tools-rest");
        let rest_w = ui.ctx().data(|d| d.get_temp::<f32>(rest_id)).unwrap_or(420.0);
        // everything in the tools group except the search box's text width
        let mut tools = |ui: &mut egui::Ui, search_w: f32| {
            let start = ui.cursor().min.x;
            kit::search_box(ui, &mut query, "Search runs", search_w);
            kit::Dropdown::from_id_salt("runs-sort")
                .selected_text(SORTS[sort as usize])
                .width(170.0)
                .show_ui(ui, |ui| {
                    for (i, s) in SORTS.iter().enumerate() {
                        ui.selectable_value(&mut sort, i as u8, *s);
                    }
                });
            kit::segmented(ui, &mut list, &[(false, "Cards"), (true, "List")]);
            let used = ui.cursor().min.x - start - ui.spacing().item_spacing.x - search_w;
            if (used - rest_w).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(rest_id, used));
                ui.ctx().request_repaint();
            }
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            kit::segmented(ui, &mut filter, &opts);
            if wide {
                ui.add_space((ui.available_width() - (170.0 + rest_w)).max(8.0));
                tools(ui, 170.0);
            }
        });
        if !wide {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let w = (ui.available_width() - rest_w - 2.0).max(120.0);
                tools(ui, w);
            });
        }
        self.setup.runs_query = query;
        self.setup.runs_filter = filter;
        if sort != self.cfg.runs_sort || list != self.cfg.runs_list {
            self.cfg.runs_sort = sort;
            self.cfg.runs_list = list;
            self.dirty = true;
        }
        ui.add_space(14.0);

        // ── filtered, sorted runs ──
        let q = self.setup.runs_query.trim().to_lowercase();
        let mut shown: Vec<&Row> = rows
            .iter()
            .filter(|r| match filter {
                1 => r.kind == Kind::Saved,
                2 => r.kind == Kind::Caught,
                3 => r.kind == Kind::Phase,
                _ => true,
            })
            .filter(|r| q.is_empty() || r.info.name.to_lowercase().contains(&q) || r.info.method.to_lowercase().contains(&q))
            .collect();
        shown.sort_by(|a, b| {
            b.pinned.cmp(&a.pinned).then_with(|| match sort {
                1 => b.info.count.cmp(&a.info.count),
                2 => b.mult().unwrap_or(0.0).total_cmp(&a.mult().unwrap_or(0.0)),
                3 => a.info.name.to_lowercase().cmp(&b.info.name.to_lowercase()),
                _ => b.last.cmp(&a.last),
            })
        });

        if shown.is_empty() {
            let msg = if !q.is_empty() {
                format!("No runs match \u{201c}{}\u{201d}.", self.setup.runs_query.trim())
            } else if rows.is_empty() {
                "No saved runs yet. When you change target, the run you were on is saved here so you can come back to it.".to_string()
            } else {
                match filter {
                    1 => "Nothing in progress. Runs you put aside show up here.".to_string(),
                    2 => "No shinies yet. Press Caught! on the overlay when you get one.".to_string(),
                    _ => "Nothing here yet.".to_string(),
                }
            };
            kit::card(ui, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(msg).size(13.0).color(MUTED()));
                    if !q.is_empty() && ui.link("Clear search").clicked() {
                        self.setup.runs_query.clear();
                    }
                });
                ui.add_space(10.0);
            });
        } else if self.cfg.runs_list {
            self.runs_list(ui, &shown, &mut act);
        } else {
            // Cards stay between MIN and MAX wide: as many columns as fit,
            // so wide windows get more cards per row instead of wider ones.
            const MIN: f32 = 310.0;
            const MAX: f32 = 400.0;
            let gap = 14.0;
            let avail = ui.available_width();
            let mut per = (((avail + gap) / (MAX + gap)).ceil() as usize).max(1);
            while per > 1 && (avail - gap * (per as f32 - 1.0)) / (per as f32) < MIN {
                per -= 1;
            }
            let w = ((avail - gap * (per as f32 - 1.0)) / per as f32).floor().min(MAX);
            for chunk in shown.chunks(per) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for r in chunk {
                        self.runs_card(ui, w, r, &mut act);
                    }
                });
                ui.add_space(gap);
            }
        }
        ui.add_space(40.0);
        self.runs_undo_bar(ui);

        match act {
            Some(Act::View(r)) => self.setup.view_run = Some(r),
            Some(Act::Share(r)) => self.open_share(r, None),
            Some(Act::Resume(id)) => self.resume_run(id),
            Some(Act::Pin(id)) => {
                if let Some(i) = self.cfg.pinned_runs.iter().position(|x| *x == id) {
                    self.cfg.pinned_runs.remove(i);
                } else {
                    self.cfg.pinned_runs.push(id);
                    self.add_event("favourites");
                }
                self.dirty = true;
            }
            Some(Act::Delete(r)) => {
                let now = ui.input(|i| i.time);
                match r {
                    RunRef::Saved(id) => {
                        if let Some(i) = self.cfg.saved_runs.iter().position(|x| x.id == id) {
                            let run = self.cfg.saved_runs.remove(i);
                            self.setup.runs_undo = Some((Undo::Saved(run), now));
                        }
                    }
                    RunRef::Finished(i) if i < self.cfg.history.len() => {
                        let rec = self.cfg.history.remove(i);
                        self.setup.runs_undo = Some((Undo::Finished(i, rec), now));
                    }
                    _ => {}
                }
                self.dirty = true;
            }
            Some(Act::SaveNew) => {
                self.park_current();
                self.start_fresh(self.cfg.target.clone());
            }
            Some(Act::PickTarget) => self.go_to(Tab::Hunt),
            None => {}
        }
    }

    /// Lifetime totals across every run.
    fn runs_summary(&mut self, ui: &mut egui::Ui, rows: &[Row]) {
        let active = (self.cfg.target.is_some() || self.cfg.count > 0) as usize;
        let open = rows.iter().filter(|r| r.kind == Kind::Saved).count() + active;
        let caught: Vec<&Row> = rows.iter().filter(|r| r.kind == Kind::Caught).collect();
        let phases = rows.iter().filter(|r| r.kind == Kind::Phase).count();
        let not_phase = || rows.iter().filter(|r| r.kind != Kind::Phase);
        let enc = self.cfg.count + not_phase().map(|r| r.info.count).sum::<u64>();
        let secs = self.cfg.run_stats.hunt_secs + not_phase().map(|r| r.secs).sum::<u64>();
        let best = caught.iter().filter(|r| r.info.count > 0).filter_map(|r| r.mult().map(|m| (m, r.info.name.clone()))).min_by(|a, b| a.0.total_cmp(&b.0));
        let shinies = if phases > 0 { format!("{} + {phases} phase{}", caught.len(), if phases == 1 { "" } else { "s" }) } else { caught.len().to_string() };
        let cells: Vec<(&str, Color32, String, String)> = vec![
            ("📜", BLUE, open.to_string(), "Runs on the go".into()),
            ("✨", GOLD, shinies, "Shinies caught".into()),
            ("⚔", PURPLE, theme::thousands(enc), "Encounters".into()),
            ("⏱", CYAN, fmt_duration(secs), "Time hunted".into()),
            ("🍀", GREEN, best.as_ref().map_or("—".into(), |b| format!("{:.2}×", b.0)), best.as_ref().map_or("Luckiest catch".into(), |b| format!("Luckiest: {}", b.1))),
        ];
        let w = ui.available_width();
        let per = if w >= 880.0 { 5 } else if w >= 560.0 { 3 } else { 2 };
        let lines = cells.len().div_ceil(per);
        let cell_h = 70.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, cell_h * lines as f32 + 16.0), Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        p.add(kit::card_shadow().as_shape(rect, 22.0));
        p.rect_filled(rect, 22.0, kit::CARD());
        p.rect_stroke(rect, 22.0, Stroke::new(1.0f32, kit::LINE()));
        for (k, (icon, col, value, label)) in cells.iter().enumerate() {
            let (cx, cy) = (k % per, k / per);
            let in_row = (cells.len() - cy * per).min(per);
            let cw = (w - 16.0) / in_row as f32;
            let cell = Rect::from_min_size(pos2(rect.min.x + 8.0 + cw * cx as f32, rect.min.y + 8.0 + cell_h * cy as f32), vec2(cw, cell_h));
            if cx > 0 {
                p.line_segment([pos2(cell.min.x, cell.min.y + 16.0), pos2(cell.min.x, cell.max.y - 16.0)], Stroke::new(1.0f32, kit::LINE()));
            }
            let chip = Rect::from_center_size(pos2(cell.min.x + 32.0, cell.center().y), vec2(36.0, 36.0));
            kit::icon_chip(&p, chip, icon, *col, false);
            let x = chip.max.x + 12.0;
            let room = cell.max.x - 10.0 - x;
            let vf = ch::fit_font(ui, value, ch::heavy, 20.0, 12.0, room);
            p.text(pos2(x, cell.center().y - 9.0), Align2::LEFT_CENTER, value, vf, TEXT());
            p.text(pos2(x, cell.center().y + 13.0), Align2::LEFT_CENTER, ch::clip(ui, label, &ch::reg(12.0), room), ch::reg(12.0), MUTED());
        }
    }

    /// The run being hunted now: big and first.
    fn runs_now(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let w = ui.available_width();
        if self.cfg.target.is_none() && self.cfg.count == 0 {
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Not hunting anything right now").font(ch::bold(16.0)).color(TEXT()));
                        ui.label(RichText::new("Pick a target to start a run, or resume one of your saved runs below.").size(13.0).color(MUTED()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if kit::accent_button(ui, egui::Button::new(RichText::new("Pick a target").font(ch::bold(13.5)).color(kit::on_primary())), 4.0).clicked() {
                            *act = Some(Act::PickTarget);
                        }
                    });
                });
            });
            return;
        }
        let Some(info) = self.run_info(RunRef::Current) else { return };
        let st = self.cfg.run_stats.clone();
        let odds = info.odds.filter(|o| *o > 1.0);
        let found = odds.map(|o| stats::found_by(info.count, o));
        let col = found.map_or(PURPLE, ch::luck_col);
        let side = w >= 960.0;
        let h = 288.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter_at(rect.expand(16.0));
        p.add(kit::card_shadow().as_shape(rect, 26.0));
        p.rect_filled(rect, 26.0, kit::CARD());
        p.rect_stroke(rect, 26.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), col, 0.35)));

        // sprite on a soft glow of the luck colour
        let sc = pos2(rect.min.x + 104.0, rect.center().y);
        p.circle_filled(sc, 78.0, ch::faint(col, 0.06));
        p.circle_filled(sc, 58.0, ch::faint(col, 0.12));
        // the hunt on now: several targets take turns
        let shown = if info.current { self.shown_target_idx(ui.ctx()).map(|i| (i, self.species[i].clone())) } else { self.species_of(info.slug.as_deref()) };
        if let Some((_, sp)) = shown {
            self.draw_sprite_centered(&p, &sp, true, sc, vec2(112.0, 112.0));
        }
        let x = rect.min.x + 208.0;
        let right = if side { rect.max.x - 340.0 } else { rect.max.x - 28.0 };
        let room = right - x;
        chip(&p, pos2(x, rect.min.y + 24.0), if self.overlay_on { "● HUNTING NOW" } else { "● CURRENT RUN" }, GREEN);
        let name = if info.name.is_empty() { "No target".to_string() } else { info.name.clone() };
        let nf = ch::fit_font(ui, &name, ch::heavy, 28.0, 16.0, room);
        p.text(pos2(x, rect.min.y + 52.0), Align2::LEFT_TOP, &name, nf, TEXT());
        let started = if info.started > 0 { format!(" · started {}", super::days_ago(info.started)) } else { String::new() };
        p.text(pos2(x, rect.min.y + 88.0), Align2::LEFT_TOP, ch::clip(ui, &format!("{}{started}", info.method), &ch::reg(12.5), room), ch::reg(12.5), MUTED());
        let num = theme::thousands(info.count);
        let nr = p.text(pos2(x, rect.min.y + 108.0), Align2::LEFT_TOP, &num, ch::heavy(40.0), ch::ink(col));
        p.text(pos2(nr.max.x + 8.0, nr.max.y - 9.0), Align2::LEFT_BOTTOM, "encounters", ch::reg(13.0), MUTED());
        let track_w = room.min(560.0);
        if let Some(o) = odds {
            let m = info.count as f64 / o;
            let mx = x + track_w;
            p.text(pos2(mx, rect.min.y + 112.0), Align2::RIGHT_TOP, format!("{m:.2}×"), ch::heavy(22.0), ch::ink(col));
            p.text(pos2(mx, rect.min.y + 139.0), Align2::RIGHT_TOP, "of the odds", ch::reg(12.0), MUTED());
            // luck track: 0 to 3× the odds
            let bar = Rect::from_min_size(pos2(x, rect.min.y + 168.0), vec2(track_w, 8.0));
            luck_track(ui, &p, bar, m, col, true, egui::Id::new("runs-now-luck"));
            let f = found.unwrap_or(0.0);
            let note = format!("{:.0}% of hunters would have found it by now", f * 100.0);
            p.text(pos2(x, bar.max.y + 32.0), Align2::LEFT_TOP, ch::clip(ui, &note, &ch::reg(12.5), room), ch::reg(12.5), TEXT().gamma_multiply(0.8));
        }
        // buttons
        let br = Rect::from_min_size(pos2(x, rect.max.y - 50.0), vec2(room, 34.0));
        let empty = self.run_is_empty();
        child(ui, br, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if kit::accent_button(ui, egui::Button::new(RichText::new("📊  Hunt stats").font(ch::bold(13.0)).color(kit::on_primary())), 4.0).clicked() {
                *act = Some(Act::View(RunRef::Current));
            }
            if ui.add(egui::Button::new("🖼  Share").fill(kit::btn_fill())).on_hover_text("Make a share image of this run for Discord").clicked() {
                *act = Some(Act::Share(RunRef::Current));
            }
            if ui
                .add_enabled(!empty, egui::Button::new("Save and start over").fill(kit::btn_fill()))
                .on_hover_text("Saves this run below and starts again from 0 with the same target")
                .on_disabled_hover_text("Nothing to save yet: this run has no encounters")
                .clicked()
            {
                *act = Some(Act::SaveNew);
            }
        });

        // right side: the last two weeks and a few numbers
        if side {
            let sx = rect.max.x - 312.0;
            p.line_segment([pos2(sx - 24.0, rect.min.y + 24.0), pos2(sx - 24.0, rect.max.y - 24.0)], Stroke::new(1.0f32, kit::LINE()));
            p.text(pos2(sx, rect.min.y + 26.0), Align2::LEFT_TOP, "LAST 14 DAYS", ch::bold(11.0), MUTED());
            let today = local_day(now_unix());
            let vals: Vec<u64> = (0..14).map(|k| today - 13 + k).map(|d| st.days.iter().find(|x| x.day == d).map_or(0, |x| x.encounters)).collect();
            let max = vals.iter().copied().max().unwrap_or(0).max(1) as f32;
            let area = Rect::from_min_size(pos2(sx, rect.min.y + 48.0), vec2(286.0, 60.0));
            let bw = (area.width() - 13.0 * 5.0) / 14.0;
            let grow = kit::grow(ui.ctx(), egui::Id::new("runs-now-days"), 1.0);
            for (k, v) in vals.iter().enumerate() {
                let bx = area.min.x + k as f32 * (bw + 5.0);
                if *v == 0 {
                    p.circle_filled(pos2(bx + bw / 2.0, area.max.y - 2.0), 2.0, theme::blend(kit::CARD(), TEXT(), 0.14));
                    continue;
                }
                let bh = (area.height() * (*v as f32 / max) * grow).max(5.0);
                let br = Rect::from_min_max(pos2(bx, area.max.y - bh), pos2(bx + bw, area.max.y));
                p.rect_filled(br, 3.0, if k == 13 { col } else { ch::faint(col, 0.5) });
            }
            let total: u64 = vals.iter().sum();
            p.text(pos2(area.max.x, rect.min.y + 26.0), Align2::RIGHT_TOP, format!("{} enc.", theme::thousands(total)), ch::bold(11.0), MUTED());
            p.text(pos2(area.max.x, area.max.y + 6.0), Align2::RIGHT_TOP, "today", ch::reg(10.5), MUTED());
            p.text(pos2(area.min.x, area.max.y + 6.0), Align2::LEFT_TOP, "2 weeks ago", ch::reg(10.5), MUTED());
            let facts = [
                ("Hunted", fmt_duration(st.hunt_secs)),
                ("Per hour", st.per_hour().map_or("—".into(), |r| theme::thousands(r.round() as u64))),
                ("Days hunted", st.days_hunted().to_string()),
                ("Sessions", st.sessions.len().to_string()),
            ];
            for (k, (l, v)) in facts.iter().enumerate() {
                let fx = sx + (k % 2) as f32 * 150.0;
                let fy = rect.min.y + 142.0 + (k / 2) as f32 * 56.0;
                p.text(pos2(fx, fy), Align2::LEFT_TOP, *l, ch::reg(12.0), MUTED());
                p.text(pos2(fx, fy + 16.0), Align2::LEFT_TOP, v, ch::heavy(19.0), TEXT());
            }
        }
    }

    /// A run as a card.
    fn runs_card(&mut self, ui: &mut egui::Ui, w: f32, row: &Row, act: &mut Option<Act>) {
        let h = 240.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let hov = ui.rect_contains_pointer(rect);
        let lift = kit::anim(ui.ctx(), resp.id.with("lift"), hov, kit::QUICK);
        let rect = rect.translate(vec2(0.0, -2.0 * lift));
        let p = ui.painter_at(rect.expand(16.0));
        let col = row.col();
        p.add(kit::card_shadow().as_shape(rect, 22.0));
        if lift > 0.0 {
            p.add(kit::shadow_tier(2).as_shape(rect, 22.0));
        }
        p.rect_filled(rect, 22.0, kit::CARD());
        p.rect_stroke(rect, 22.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), col, 0.55 * lift)));

        let sc = pos2(rect.min.x + 56.0, rect.min.y + 58.0);
        p.circle_filled(sc, 38.0, ch::faint(col, 0.14));
        if let Some((_, sp)) = self.species_of(row.info.slug.as_deref()) {
            self.draw_sprite_centered(&p, &sp, true, sc, vec2(70.0, 70.0));
        }
        if row.kind != Kind::Saved {
            theme::sparkle(&p, sc + vec2(30.0, -28.0), 7.0, GOLD);
        }
        let x = rect.min.x + 110.0;
        let room = rect.max.x - 20.0 - x;
        let (st, sc_col) = match row.kind {
            Kind::Saved if row.pinned => ("📌 PINNED", kit::primary()),
            Kind::Saved => ("SAVED", MUTED()),
            Kind::Caught => ("★ CAUGHT", GOLD),
            Kind::Phase => ("✨ PHASE", CYAN),
        };
        chip(&p, pos2(x, rect.min.y + 20.0), st, sc_col);
        let name = if row.info.name.is_empty() { "No target".to_string() } else { row.info.name.clone() };
        p.text(pos2(x, rect.min.y + 46.0), Align2::LEFT_TOP, ch::clip(ui, &name, &ch::bold(17.0), room - 64.0 * lift), ch::bold(17.0), TEXT());
        let method = if row.info.method.is_empty() { "—" } else { &row.info.method };
        p.text(pos2(x, rect.min.y + 70.0), Align2::LEFT_TOP, ch::clip(ui, method, &ch::reg(12.0), room), ch::reg(12.0), MUTED());

        // numbers
        let num = theme::thousands(row.info.count);
        let nr = p.text(pos2(rect.min.x + 22.0, rect.min.y + 108.0), Align2::LEFT_TOP, &num, ch::heavy(28.0), TEXT());
        p.text(pos2(nr.max.x + 6.0, nr.max.y - 6.0), Align2::LEFT_BOTTOM, "enc.", ch::reg(12.0), MUTED());
        if let Some(m) = row.mult() {
            p.text(pos2(rect.max.x - 22.0, rect.min.y + 112.0), Align2::RIGHT_TOP, format!("{m:.2}×"), ch::bold(16.0), ch::ink(col));
            let sub = if row.kind == Kind::Saved { "of the odds".to_string() } else { ch::LUCK_BANDS[ch::luck_band(stats::found_by(row.info.count, row.odds().unwrap_or(1.0)))].1.to_string() };
            p.text(pos2(rect.max.x - 22.0, rect.min.y + 133.0), Align2::RIGHT_TOP, sub, ch::reg(11.5), MUTED());
            let bar = Rect::from_min_max(pos2(rect.min.x + 22.0, rect.min.y + 156.0), pos2(rect.max.x - 22.0, rect.min.y + 162.0));
            luck_track(ui, &p, bar, m, col, false, egui::Id::new(("runs-card-luck", row.id(), row.last, row.info.name.as_str())));
        } else if row.kind == Kind::Phase {
            p.text(pos2(rect.min.x + 22.0, rect.min.y + 152.0), Align2::LEFT_CENTER, ch::clip(ui, "A surprise shiny, found while hunting something else", &ch::reg(12.0), w - 44.0), ch::reg(12.0), MUTED());
        }
        let when = match row.kind {
            Kind::Saved => format!("last hunted {}", super::days_ago(row.last)),
            _ => format!("caught {}", super::days_ago(row.last)),
        };
        let meta = if row.secs > 0 { format!("{} hunted · {when}", fmt_duration(row.secs)) } else { when };
        p.text(pos2(rect.min.x + 22.0, rect.min.y + 180.0), Align2::LEFT_CENTER, ch::clip(ui, &meta, &ch::reg(12.0), w - 44.0), ch::reg(12.0), MUTED());

        // pinned mark / hover tools
        let tools = self.card_tools(ui, rect, row, lift, act);
        let _ = tools;

        // buttons
        let br = Rect::from_min_max(pos2(rect.min.x + 16.0, rect.max.y - 50.0), pos2(rect.max.x - 16.0, rect.max.y - 16.0));
        child(ui, br, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let side_w = 86.0;
            let n_side = if row.kind == Kind::Saved { 2.0 } else { 1.0 };
            let main_w = br.width() - n_side * (side_w + 8.0);
            if let (Kind::Saved, RunRef::Saved(id)) = (row.kind, row.r) {
                let who = self.target_name(self.cfg.target.as_deref());
                let tip = if self.run_is_empty() { "Pick this run back up".to_string() } else { format!("Saves your {who} run and picks this one back up") };
                if kit::accent_button_w(ui, egui::Button::new(RichText::new("▶  Resume").font(ch::bold(13.0)).color(kit::on_primary())), 4.0, main_w).on_hover_text(tip).clicked() {
                    *act = Some(Act::Resume(id));
                }
            } else if ui.add(egui::Button::new(RichText::new("📊  Stats").font(ch::bold(13.0))).fill(kit::btn_fill()).min_size(vec2(main_w, 0.0))).clicked() {
                *act = Some(Act::View(row.r));
            }
            if row.kind == Kind::Saved && ui.add(egui::Button::new("📊  Stats").fill(kit::btn_fill()).min_size(vec2(side_w, 0.0))).clicked() {
                *act = Some(Act::View(row.r));
            }
            if ui.add(egui::Button::new("🖼  Share").fill(kit::btn_fill()).min_size(vec2(side_w, 0.0))).on_hover_text("Make a share image for Discord").clicked() {
                *act = Some(Act::Share(row.r));
            }
        });
        if resp.clicked() && act.is_none() {
            *act = Some(Act::View(row.r));
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    /// Pin and delete, top-right, fading in on hover. Returns how visible.
    fn card_tools(&mut self, ui: &mut egui::Ui, rect: Rect, row: &Row, lift: f32, act: &mut Option<Act>) -> f32 {
        if lift <= 0.01 {
            return 0.0;
        }
        let mut x = rect.max.x - 16.0;
        let y = rect.min.y + 16.0;
        let mut tool = |ui: &mut egui::Ui, icon: &str, tip: &str, danger: bool, on: bool| -> bool {
            let r = Rect::from_min_size(pos2(x - 30.0, y), vec2(30.0, 30.0));
            x -= 36.0;
            let resp = ui.interact(r, ui.id().with(("run-tool", row.id(), icon, row.last)), Sense::click()).on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
            let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
            let base = if on { kit::primary() } else { MUTED() };
            let fg = if danger { theme::blend(base, theme::HP_RED, ht) } else { theme::blend(base, TEXT(), ht) };
            let p = ui.painter();
            p.circle_filled(r.center(), 15.0, theme::blend(kit::sunken(), fg, 0.12 * ht).gamma_multiply(lift));
            p.text(r.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(14.0), fg.gamma_multiply(lift));
            lift > 0.5 && resp.clicked()
        };
        if tool(ui, "🗑", if row.kind == Kind::Saved { "Delete this run" } else { "Remove from the list" }, true, false) {
            *act = Some(Act::Delete(row.r));
        }
        if row.kind == Kind::Saved && tool(ui, "📌", if row.pinned { "Unpin" } else { "Pin to the top" }, false, row.pinned) {
            *act = Some(Act::Pin(row.id()));
        }
        lift
    }

    /// Runs as a compact table.
    fn runs_list(&mut self, ui: &mut egui::Ui, rows: &[&Row], act: &mut Option<Act>) {
        let w = ui.available_width();
        let cols_wide = w >= 820.0;
        let row_h = 64.0;
        let head_h = 36.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, head_h + row_h * rows.len() as f32 + 8.0), Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        p.add(kit::card_shadow().as_shape(rect, 22.0));
        p.rect_filled(rect, 22.0, kit::CARD());
        p.rect_stroke(rect, 22.0, Stroke::new(1.0f32, kit::LINE()));
        // columns, right to left: actions, last, hunted, odds, encounters
        let act_w = 252.0;
        let mut cx = rect.max.x - 18.0 - act_w;
        let mut col_x = vec![];
        let spec: Vec<(&str, f32)> = if cols_wide { vec![("LAST ACTIVE", 120.0), ("HUNTED", 90.0), ("ODDS", 80.0), ("ENCOUNTERS", 100.0)] } else { vec![("ODDS", 72.0), ("ENCOUNTERS", 96.0)] };
        for (l, cw) in &spec {
            col_x.push((*l, cx));
            cx -= cw;
        }
        let name_room = cx - (rect.min.x + 76.0) - 12.0;
        let hy = rect.min.y + head_h / 2.0 + 4.0;
        p.text(pos2(rect.min.x + 22.0, hy), Align2::LEFT_CENTER, "RUN", ch::bold(10.5), MUTED());
        for (l, x) in &col_x {
            p.text(pos2(*x - 10.0, hy), Align2::RIGHT_CENTER, *l, ch::bold(10.5), MUTED());
        }
        for (k, row) in rows.iter().enumerate() {
            let r = Rect::from_min_size(pos2(rect.min.x + 6.0, rect.min.y + head_h + 4.0 + k as f32 * row_h), vec2(w - 12.0, row_h));
            let resp = ui.interact(r, ui.id().with(("run-row", row.id(), row.last)), Sense::click());
            let hov = ui.rect_contains_pointer(r);
            let ht = kit::anim(ui.ctx(), resp.id, hov, kit::QUICK);
            if ht > 0.0 {
                p.rect_filled(r, 16.0, theme::blend(kit::CARD(), TEXT(), 0.05 * ht));
            } else if k > 0 {
                p.line_segment([pos2(r.min.x + 16.0, r.min.y), pos2(r.max.x - 16.0, r.min.y)], Stroke::new(1.0f32, kit::LINE()));
            }
            let col = row.col();
            let sc = pos2(r.min.x + 38.0, r.center().y);
            p.circle_filled(sc, 23.0, ch::faint(col, 0.14));
            if let Some((_, sp)) = self.species_of(row.info.slug.as_deref()) {
                self.draw_sprite_centered(&p, &sp, true, sc, vec2(44.0, 44.0));
            }
            let x = r.min.x + 72.0;
            let name = if row.info.name.is_empty() { "No target".to_string() } else { row.info.name.clone() };
            let nr = p.text(pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, ch::clip(ui, &name, &ch::bold(14.5), name_room - 90.0), ch::bold(14.5), TEXT());
            let (st, stc) = match row.kind {
                Kind::Saved if row.pinned => ("📌 PINNED", kit::primary()),
                Kind::Saved => ("SAVED", MUTED()),
                Kind::Caught => ("★ CAUGHT", GOLD),
                Kind::Phase => ("✨ PHASE", CYAN),
            };
            chip(&p, pos2(nr.max.x + 8.0, r.center().y - 19.0), st, stc);
            p.text(pos2(x, r.center().y + 11.0), Align2::LEFT_CENTER, ch::clip(ui, &row.info.method, &ch::reg(12.0), name_room), ch::reg(12.0), MUTED());
            for (l, cx) in &col_x {
                let (v, c) = match *l {
                    "ENCOUNTERS" => (theme::thousands(row.info.count), TEXT()),
                    "ODDS" => (row.mult().map_or("—".into(), |m| format!("{m:.2}×")), ch::ink(col)),
                    "HUNTED" => (if row.secs > 0 { fmt_duration(row.secs) } else { "—".into() }, TEXT()),
                    _ => (super::days_ago(row.last), MUTED()),
                };
                p.text(pos2(*cx - 10.0, r.center().y), Align2::RIGHT_CENTER, v, if *l == "LAST ACTIVE" { ch::reg(12.5) } else { ch::bold(14.0) }, c);
            }
            // actions
            let ar = Rect::from_min_max(pos2(r.max.x - 12.0 - act_w, r.center().y - 16.0), pos2(r.max.x - 12.0, r.center().y + 16.0));
            child(ui, ar, egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let small = |ui: &mut egui::Ui, icon: &str, tip: &str| ui.add(egui::Button::new(icon).fill(kit::btn_fill()).min_size(vec2(32.0, 30.0))).on_hover_text(tip).clicked();
                if small(ui, "🗑", if row.kind == Kind::Saved { "Delete this run" } else { "Remove from the list" }) {
                    *act = Some(Act::Delete(row.r));
                }
                if small(ui, "🖼", "Share image") {
                    *act = Some(Act::Share(row.r));
                }
                if let RunRef::Saved(id) = row.r {
                    if small(ui, "📌", if row.pinned { "Unpin" } else { "Pin to the top" }) {
                        *act = Some(Act::Pin(id));
                    }
                    if kit::accent_button(ui, egui::Button::new(RichText::new("▶ Resume").font(ch::bold(12.5)).color(kit::on_primary())), 4.0).clicked() {
                        *act = Some(Act::Resume(id));
                    }
                }
            });
            if resp.clicked() && act.is_none() {
                *act = Some(Act::View(row.r));
            }
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }

    /// "Deleted the Beldum run · Undo", floating at the bottom for a few seconds.
    fn runs_undo_bar(&mut self, ui: &mut egui::Ui) {
        let Some((u, at)) = self.setup.runs_undo.clone() else { return };
        let now = ui.input(|i| i.time);
        let age = now - at;
        if age > UNDO_SECS || age < 0.0 {
            self.setup.runs_undo = None;
            return;
        }
        ui.ctx().request_repaint();
        let name = match &u {
            Undo::Saved(r) => format!("Deleted the {} run", self.target_name(r.target.as_deref())),
            Undo::Finished(_, h) => format!("Removed {}", h.target),
        };
        let fade = (age.min(UNDO_SECS - age) / 0.2).clamp(0.0, 1.0) as f32;
        let clip = ui.clip_rect();
        let pos = pos2(clip.center().x, clip.max.y - 22.0 + 10.0 * (1.0 - fade));
        let mut undo = false;
        let mut close = false;
        egui::Area::new(egui::Id::new("runs-undo"))
            .order(egui::Order::Foreground)
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                ui.set_opacity(fade);
                let ink = kit::CARD();
                egui::Frame::none()
                    .fill(TEXT())
                    .rounding(22.0)
                    .shadow(kit::shadow_tier(2))
                    .inner_margin(egui::Margin { left: 18.0, right: 8.0, top: 6.0, bottom: 6.0 })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.label(RichText::new(name).size(13.0).color(ink));
                            let b = egui::Button::new(RichText::new("Undo").font(ch::bold(13.0)).color(ink)).fill(theme::blend(TEXT(), ink, 0.18)).rounding(16.0);
                            if ui.add(b).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                undo = true;
                            }
                            if ui.add(egui::Button::new(RichText::new("×").size(18.0).color(ink.gamma_multiply(0.7))).frame(false)).clicked() {
                                close = true;
                            }
                        });
                        // time left
                        let r = ui.min_rect();
                        let left = 1.0 - (age / UNDO_SECS) as f32;
                        let line = Rect::from_min_size(pos2(r.min.x, r.max.y + 4.0), vec2(r.width() * left, 2.0));
                        ui.painter().rect_filled(line, 1.0, ink.gamma_multiply(0.35));
                    });
            });
        if undo {
            match u {
                Undo::Saved(r) => {
                    if !self.cfg.saved_runs.iter().any(|x| x.id == r.id) {
                        self.cfg.saved_runs.push(r);
                    }
                }
                Undo::Finished(i, h) => {
                    let i = i.min(self.cfg.history.len());
                    self.cfg.history.insert(i, h);
                }
            }
            self.dirty = true;
            self.setup.runs_undo = None;
        } else if close {
            self.setup.runs_undo = None;
        }
    }
}

/// A ui over `r` that doesn't take up room in the parent's layout.
fn child<R>(ui: &mut egui::Ui, r: Rect, layout: egui::Layout, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(layout));
    f(&mut c)
}

/// A small status chip ("SAVED", "★ CAUGHT") with its top-left at `at`.
fn chip(p: &egui::Painter, at: egui::Pos2, text: &str, c: Color32) -> Rect {
    let g = p.layout_no_wrap(text.to_string(), ch::bold(10.5), ch::ink(c));
    let r = Rect::from_min_size(at, vec2(g.size().x + 14.0, 20.0));
    p.rect_filled(r, 10.0, ch::faint(c, 0.18));
    p.galley(pos2(at.x + 7.0, r.center().y - g.size().y / 2.0), g, ch::ink(c));
    r
}

/// The odds track: 0 to 3× the odds, ticks at 1× and 2×, filled to `mult`.
fn luck_track(ui: &egui::Ui, p: &egui::Painter, bar: Rect, mult: f64, col: Color32, labels: bool, key: egui::Id) {
    let r = bar.height() / 2.0;
    p.rect_filled(bar, r, theme::blend(kit::CARD(), TEXT(), 0.09));
    // keyed by the run, not the position: hovering lifts the card and scrolling moves it
    let frac = kit::grow(ui.ctx(), key, (mult / 3.0).clamp(0.0, 1.0) as f32);
    if frac > 0.0 {
        let fill = Rect::from_min_size(bar.min, vec2((bar.width() * frac).max(bar.height()), bar.height()));
        p.rect_filled(fill, r, col);
    }
    for k in 1..3 {
        let tx = bar.min.x + bar.width() * k as f32 / 3.0;
        p.line_segment([pos2(tx, bar.min.y - 3.0), pos2(tx, bar.max.y + 3.0)], Stroke::new(1.5f32, kit::CARD()));
    }
    if labels {
        for k in 1..=3 {
            let tx = bar.min.x + bar.width() * k as f32 / 3.0;
            let al = if k == 3 { Align2::RIGHT_TOP } else { Align2::CENTER_TOP };
            p.text(pos2(tx, bar.max.y + 10.0), al, format!("{k}×"), ch::reg(11.0), MUTED());
        }
    }
}
