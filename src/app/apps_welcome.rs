//! The first thing someone updating from an older version sees: their whole
//! save folder is copied to a backup, every file is checked to load, and the
//! screen shows what came across (profile, runs, encounters, Shiny Dex,
//! trophies, battle pass, teams, settings). Then they keep every app (as
//! before the update) or choose which to keep. Nothing is deleted.

use super::store::{app_col, app_icon, pill_button};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::apps::{self, AppId, APPS_WELCOME_VERSION};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// What the first launch found and did.
pub(super) struct WelcomeRun {
    pub started: Instant,
    /// Where the copy went, and how many files.
    pub backup: Result<(PathBuf, usize), String>,
    /// Files that didn't read as JSON (kept as they were).
    pub unreadable: Vec<String>,
    pub teams: usize,
    pub battles: usize,
    pub breeding: bool,
    /// Keep every app (true) or choose.
    pub keep_all: bool,
    pub picks: Vec<AppId>,
}

/// Copy every file in the save folder (not the folders in it) to
/// `backups/before-<version>-apps-<time>/`, and check each .json loads.
fn back_up_saves() -> (Result<(PathBuf, usize), String>, Vec<String>) {
    let dir = crate::config::data_dir();
    let stamp = crate::config::now_unix();
    let to = dir.join("backups").join(format!("before-v{}-apps-{stamp}", crate::config::update_version()));
    let mut bad = Vec::new();
    let res = (|| {
        std::fs::create_dir_all(&to).map_err(|e| format!("Couldn't make the backup folder: {e}"))?;
        let mut n = 0;
        for e in std::fs::read_dir(&dir).map_err(|e| format!("Couldn't read your save folder: {e}"))?.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            std::fs::copy(&p, to.join(&name)).map_err(|e| format!("Couldn't copy {name}: {e}"))?;
            n += 1;
            if name.ends_with(".json") {
                let ok = std::fs::read_to_string(&p).ok().is_some_and(|s| serde_json::from_str::<serde_json::Value>(&s).is_ok());
                if !ok {
                    bad.push(name);
                }
            }
        }
        Ok((to.clone(), n))
    })();
    (res, bad)
}

impl App {
    /// Is the first-launch screen showing? (People updating, once.)
    pub(super) fn apps_welcome_open(&self) -> bool {
        self.cfg.welcome_done && self.cfg.apps.welcomed < APPS_WELCOME_VERSION
    }

    /// Onboarding or the first-launch screen: no sidebar while either shows.
    pub(super) fn takeover_open(&self) -> bool {
        self.onboarding_open() || self.apps_welcome_open()
    }

    pub(super) fn apps_welcome(&mut self, ctx: &egui::Context) {
        if self.setup.store.welcome.is_none() {
            let (backup, unreadable) = back_up_saves();
            let teams = self.setup.team.teams().teams.len();
            let battles = self.setup.sim.file().history.len();
            let breeding = crate::config::data_dir().join("breeding.json").exists();
            self.setup.store.welcome = Some(WelcomeRun {
                started: Instant::now(),
                backup,
                unreadable,
                teams,
                battles,
                breeding,
                keep_all: true,
                picks: apps::ALL.to_vec(),
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            kit::settings_style(ui);
            let area = ui.max_rect();
            let p = ui.painter().clone();
            // a soft glow behind
            for (c, r, col) in [(pos2(area.max.x - 160.0, area.min.y - 40.0), 420.0, kit::C_VIOLET), (pos2(area.min.x + 80.0, area.max.y + 40.0), 360.0, kit::C_ORANGE)] {
                for k in 0..10 {
                    let f = k as f32 / 10.0;
                    p.circle_filled(c, r * (1.0 - f), col.gamma_multiply(0.012 + 0.01 * f));
                }
            }
            let w = 780.0f32.min(area.width() - 48.0);
            let inner = Rect::from_center_size(area.center(), vec2(w, area.height() - 40.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                    ui.set_width(w);
                    self.apps_welcome_body(ui, w);
                });
            });
        });
    }

    fn apps_welcome_body(&mut self, ui: &mut egui::Ui, w: f32) {
        let run = self.setup.store.welcome.as_ref().expect("set above");
        let lt = self.lifetime();
        let level = self.cfg.level_seen.max(1);
        let name = if self.cfg.trainer_name.trim().is_empty() { "Trainer".to_string() } else { self.cfg.trainer_name.clone() };
        let runs = self.cfg.saved_runs.len() + 1;
        let today = crate::stats::local_day(crate::config::now_unix());
        let season = crate::pass::current(today);
        let xp = self.pass_xp_in(season);
        let tier = crate::pass::tier(xp);
        let hours = lt.hunt_secs / 3600;
        let mins = (lt.hunt_secs % 3600) / 60;
        let backup_line = match &run.backup {
            Ok((p, n)) => {
                let folder = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                (true, "Backed up your save".to_string(), format!("{n} file{} → backups\\{folder}", if *n == 1 { "" } else { "s" }))
            }
            Err(e) => (false, "Couldn't back up your save".to_string(), e.clone()),
        };
        let mut steps: Vec<(bool, String, String)> = vec![
            backup_line,
            (true, format!("Profile: {name} · Level {level}"), "Avatar, plate, title and trainer card".into()),
            (true, format!("{runs} runs · {} encounters", theme::thousands(lt.encounters)), format!("Every run and its stats · {hours}h {mins}m hunted")),
            (true, format!("Shiny Dex: {} shinies · {} trophies", lt.shinies, self.cfg.achievements.len()), "Everything you've caught and earned".into()),
            (true, format!("Battle pass: Season {} · tier {tier}", season.num), format!("{} XP and every reward you've claimed", theme::thousands(xp))),
            (true, format!("{} teams · {} battles{}", run.teams, run.battles, if run.breeding { " · breeding plans" } else { "" }), "Team Planner, Battle Sim and Breeding".into()),
            (true, "Settings, overlay & hotkeys".into(), "Exactly as you left them".into()),
        ];
        if let Some(old) = self.setup.store.old_look {
            steps.push((true, "Your look is Modern now".into(), format!("{old} is retired; your colours are kept")));
        }
        for f in &run.unreadable {
            steps.push((false, format!("{f} didn't read cleanly"), "Kept exactly as it was, and in the backup".into()));
        }
        let at = (run.started.elapsed().as_millis() / 280) as usize;
        let ready = at >= steps.len();
        if !ready {
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }
        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("UPDATED TO v{} · FIRST LAUNCH", crate::config::update_version())).font(crate::fonts::sans_bold_font(12.0)).color(Color32::from_rgb(196, 176, 255)));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(if ready { "Everything came across ✓" } else { "Bringing your stuff over…" }).font(crate::fonts::title_font(30.0)).color(TEXT()));
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("This happens before anything else, and nothing is deleted. Apps are new in this version: you keep every one you already had, and can add or remove them any time.")
                .size(kit::fs(14.0))
                .color(MUTED()),
        );
        ui.add_space(18.0);
        // the checklist, two across
        let cw = (w - 10.0) / 2.0;
        for pair in steps.chunks(2).enumerate() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for (k, (ok, t, sub)) in pair.1.iter().enumerate() {
                    let i = pair.0 * 2 + k;
                    let (r, _) = ui.allocate_exact_size(vec2(cw, 58.0), Sense::hover());
                    let p = ui.painter();
                    let shown = i < at;
                    let a: f32 = if shown { 1.0 } else if i == at { 0.6 } else { 0.35 };
                    p.rect_filled(r, 14.0, CARD().gamma_multiply(a.max(0.6)));
                    p.rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
                    let c = pos2(r.min.x + 26.0, r.center().y);
                    let (fill, ink, mark) = if !shown {
                        (kit::sunken(), MUTED(), format!("{}", i + 1))
                    } else if *ok {
                        (theme::blend(CARD(), theme::GREEN, 0.25), theme::GREEN, "✓".into())
                    } else {
                        (theme::blend(CARD(), kit::C_GOLD, 0.25), kit::C_GOLD, "!".into())
                    };
                    p.circle_filled(c, 13.0, fill);
                    p.text(c, Align2::CENTER_CENTER, mark, crate::fonts::sans_bold_font(13.0), ink);
                    let clip = p.with_clip_rect(r.shrink(2.0));
                    clip.text(pos2(r.min.x + 50.0, r.center().y - 9.0), Align2::LEFT_CENTER, t, crate::fonts::sans_bold_font(13.5), TEXT().gamma_multiply(a));
                    clip.text(pos2(r.min.x + 50.0, r.center().y + 10.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(12.0), MUTED().gamma_multiply(a));
                }
            });
            ui.add_space(10.0);
        }
        if !ready {
            return;
        }
        ui.add_space(8.0);
        let keep_all = run.keep_all;
        let mut set_keep: Option<bool> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (k, (t, sub, rec)) in [(true, ("Keep all my apps", "Same as before the update: every app installed. Remove any later in your Library.", true)), (false, ("Choose my apps", "Only keep what you use. Apps you remove keep their saved data.", false))] {
                let (r, resp) = ui.allocate_exact_size(vec2(cw, 86.0), Sense::click());
                let on = keep_all == k;
                let p = ui.painter();
                p.rect_filled(r, 16.0, if on { theme::blend(CARD(), kit::primary(), 0.14) } else { CARD() });
                p.rect_stroke(r, 16.0, Stroke::new(if on { 1.5f32 } else { 1.0f32 }, if on { kit::primary() } else { LINE() }));
                let g = p.text(pos2(r.min.x + 16.0, r.min.y + 16.0), Align2::LEFT_TOP, t, crate::fonts::sans_bold_font(14.5), TEXT());
                if rec {
                    super::store::tag(p, pos2(g.max.x + 8.0, r.min.y + 15.0), "RECOMMENDED", kit::C_BLUE);
                }
                let gal = p.layout(sub.to_string(), crate::fonts::sans_font(12.5), MUTED(), cw - 32.0);
                p.galley(pos2(r.min.x + 16.0, r.min.y + 42.0), gal, MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    set_keep = Some(k);
                }
            }
        });
        if let Some(k) = set_keep {
            self.setup.store.welcome.as_mut().expect("set").keep_all = k;
        }
        let run = self.setup.store.welcome.as_ref().expect("set");
        let mut toggle: Option<AppId> = None;
        if !run.keep_all {
            ui.add_space(10.0);
            let pw = (w - 30.0) / 4.0;
            for row in apps::ALL.chunks(4) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for a in row {
                        let a = *a;
                        let on = a.meta().core || run.picks.contains(&a);
                        let hint = match a {
                            _ if a.meta().core => "Always installed".to_string(),
                            AppId::Team if run.teams > 0 => format!("You have {} teams", run.teams),
                            AppId::Sim if run.battles > 0 => format!("{} battles saved", run.battles),
                            AppId::Sim => "Needs Team Planner".into(),
                            AppId::Breed if run.breeding => "You have breeding plans".into(),
                            _ => a.meta().cat.name().to_string(),
                        };
                        let (r, resp) = ui.allocate_exact_size(vec2(pw, 50.0), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(r, 12.0, if on { theme::blend(CARD(), kit::primary(), 0.12) } else { CARD() });
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, if on { kit::primary() } else { LINE() }));
                        kit::icon_chip(p, Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), vec2(26.0, 26.0)), app_icon(a), app_col(a), false);
                        let clip = p.with_clip_rect(r.shrink2(vec2(30.0, 0.0)).with_min_x(r.min.x + 40.0));
                        clip.text(pos2(r.min.x + 42.0, r.center().y - 8.0), Align2::LEFT_CENTER, a.name(), crate::fonts::sans_bold_font(12.5), TEXT());
                        clip.text(pos2(r.min.x + 42.0, r.center().y + 9.0), Align2::LEFT_CENTER, &hint, crate::fonts::sans_font(11.0), MUTED());
                        let cb = Rect::from_center_size(pos2(r.max.x - 18.0, r.center().y), vec2(18.0, 18.0));
                        p.rect_filled(cb, 5.0, if on { kit::primary() } else { Color32::TRANSPARENT });
                        p.rect_stroke(cb, 5.0, Stroke::new(1.5f32, if on { kit::primary() } else { kit::CONTROL() }));
                        if on {
                            p.text(cb.center(), Align2::CENTER_CENTER, "✓", crate::fonts::sans_bold_font(11.0), kit::on_primary());
                        }
                        if !a.meta().core && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            toggle = Some(a);
                        }
                    }
                });
                ui.add_space(8.0);
            }
        }
        if let Some(a) = toggle {
            let run = self.setup.store.welcome.as_mut().expect("set");
            if run.picks.contains(&a) {
                run.picks.retain(|x| *x != a);
                // nothing can keep what it needs out
                for d in apps::needed_by(a, &run.picks.clone()) {
                    run.picks.retain(|x| *x != d);
                }
            } else {
                for x in apps::with_requirements(a, &run.picks.clone()) {
                    run.picks.push(x);
                }
            }
        }
        ui.add_space(14.0);
        let run = self.setup.store.welcome.as_ref().expect("set");
        let where_ = match &run.backup {
            Ok((p, _)) => format!("Backup kept in {}", p.display()),
            Err(_) => "Your files weren't changed".to_string(),
        };
        let mut go = false;
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(where_).size(kit::fs(12.0)).color(MUTED())).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(150.0, 44.0), Sense::hover());
                go = pill_button(ui, r, "Let's go  ›", true, "apps-welcome-go").clicked();
            });
        });
        ui.add_space(20.0);
        if go {
            let run = self.setup.store.welcome.take().expect("set");
            if run.keep_all {
                self.cfg.apps.set_installed(&apps::ALL);
            } else {
                self.cfg.apps.set_installed(&run.picks);
            }
            self.cfg.apps.welcomed = APPS_WELCOME_VERSION;
            if self.cfg.apps.pins.is_empty() {
                self.cfg.apps.pin(AppId::Shiny);
            }
            if self.cfg.apps.open.is_empty() {
                self.cfg.apps.open(AppId::Shiny, crate::config::now_unix());
            }
            self.dirty = true;
            self.go_to(super::Tab::Home);
            self.app_toast(format!("All your data is here. Welcome to v{}", crate::config::update_version()));
        }
    }
}
