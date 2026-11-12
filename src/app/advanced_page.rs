//! The Advanced page: a health check at the top (version, game folder,
//! backups, data), then every card on one page: updates, backups (with daily
//! automatic copies), data folder, game folder, the counter, screen reading
//! and help.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::settings::AdvTab;
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::App;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};

/// How many daily copies to keep.
/// How many automatic copies are kept; the newest replaces the oldest.
pub(crate) const AUTO_KEEP: usize = 5;
/// How often a copy is made while the app is open (only if something changed).
const AUTO_EVERY: std::time::Duration = std::time::Duration::from_secs(60 * 60);

impl App {
    /// Automatic copies: one when the app opens and one an hour while it's
    /// open, only when something changed, never of an empty save. The last
    /// `AUTO_KEEP` are kept: the newest replaces the oldest.
    pub(super) fn auto_backup_tick(&mut self) {
        if !self.cfg.auto_backup {
            return;
        }
        if self.setup.auto_backup_at.is_some_and(|t| t.elapsed() < AUTO_EVERY) {
            return;
        }
        self.setup.auto_backup_at = Some(std::time::Instant::now());
        let _ = write_auto_backup(&self.cfg);
    }

    pub(super) fn page_advanced(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "ADVANCED", "Updates, backups, folders and fine-tuning. Everything here is safe to look at; nothing changes until you click.");
        self.adv_poll_pickers();
        self.adv_health(ui);
        ui.add_space(18.0);
        let w = ui.available_width();
        let jump = self.setup.adv_jump.take();
        let mut jump_to: Option<Rect> = None;
        let mut card = |this: &mut App, ui: &mut egui::Ui, which: AdvTab| {
            let r = ui
                .scope(|ui| match which {
                    AdvTab::Updates => this.adv_updates_card(ui),
                    AdvTab::Data => {
                        this.adv_backup_card(ui);
                        ui.add_space(18.0);
                        this.adv_data_card(ui);
                    }
                    AdvTab::Game => this.adv_game_card(ui),
                    AdvTab::Counter => this.adv_counter_card(ui),
                    AdvTab::Reading => this.adv_reading_card(ui),
                    AdvTab::Help => this.adv_help_card(ui),
                })
                .response
                .rect;
            if jump == Some(which) {
                jump_to = Some(r);
            }
            ui.add_space(18.0);
        };
        if w >= 1000.0 {
            let gap = 18.0;
            let cw = ((w - gap) / 2.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    for c in [AdvTab::Updates, AdvTab::Data, AdvTab::Counter] {
                        card(self, ui, c);
                    }
                });
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    for c in [AdvTab::Game, AdvTab::Reading, AdvTab::Help] {
                        card(self, ui, c);
                    }
                });
            });
        } else {
            for c in [AdvTab::Updates, AdvTab::Data, AdvTab::Game, AdvTab::Counter, AdvTab::Reading, AdvTab::Help] {
                card(self, ui, c);
            }
        }
        if let Some(r) = jump_to {
            ui.scroll_to_rect(r, Some(egui::Align::Min));
            // a short glow so the eye finds it
            // read the time first: asking for it inside data_mut would wait on
            // the lock data_mut already holds, and the app would freeze
            let now = ui.input(|i| i.time);
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("adv-glow"), (r, now)));
        }
        if let Some((r, t)) = ui.ctx().data(|d| d.get_temp::<(Rect, f64)>(egui::Id::new("adv-glow"))) {
            let age = (ui.input(|i| i.time) - t) as f32;
            if age < 1.6 {
                let k = 1.0 - age / 1.6;
                ui.painter().rect_stroke(r.expand(3.0), 26.0, Stroke::new(2.5f32, ch::faint(kit::primary(), k)));
                ui.ctx().request_repaint();
            }
        }
        ui.add_space(12.0);
    }

    /// File and folder boxes that answered since last frame.
    fn adv_poll_pickers(&mut self) {
        if let Some(rx) = &self.setup.log_picker {
            if let Ok(res) = rx.try_recv() {
                self.setup.log_picker = None;
                if let Some(dir) = res {
                    self.cfg.game_dir = Some(dir.to_string_lossy().to_string());
                    self.game_folder_changed();
                }
            }
        }
        if let Some(rx) = &self.setup.data_picker {
            if let Ok(res) = rx.try_recv() {
                self.setup.data_picker = None;
                if let Some(target) = res {
                    self.move_data(Some(target));
                }
            }
        }
        if let Some((save, rx)) = &self.setup.backup_picker {
            if let Ok(res) = rx.try_recv() {
                let save = *save;
                self.setup.backup_picker = None;
                if let Some(path) = res {
                    if save {
                        self.setup.backup_msg = Some(match serde_json::to_string_pretty(&self.cfg) {
                            Ok(json) => match std::fs::write(&path, json) {
                                Ok(()) => {
                                    self.cfg.last_backup = Some(crate::stats::now_unix());
                                    self.dirty = true;
                                    (true, format!("✔ Backup saved: {}", path.display()))
                                }
                                Err(e) => (false, format!("Couldn't save the backup: {e}")),
                            },
                            Err(e) => (false, format!("Couldn't save the backup: {e}")),
                        });
                    } else {
                        self.load_backup(path);
                    }
                }
            }
        }
    }

    /// Read a backup and ask "Replace everything?".
    fn load_backup(&mut self, path: std::path::PathBuf) {
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<crate::config::Config>(&t).map_err(|_| "That file isn't a Companion backup".to_string()))
        {
            Ok(c) => self.setup.restore_pending = Some((path, Box::new(c))),
            Err(e) => self.setup.backup_msg = Some((false, e)),
        }
    }

    // ───────────────────────────── health ─────────────────────────────

    fn adv_health(&mut self, ui: &mut egui::Ui) {
        use crate::update::{self, State};
        let state = update::get(&self.updater);
        let (up_val, up_c) = match &state {
            State::Idle => ("Not checked yet".to_string(), MUTED()),
            State::Checking => ("Checking…".to_string(), CYAN),
            State::UpToDate => ("Up to date".to_string(), GREEN),
            State::Downloading { version, .. } => (format!("Getting v{version}"), CYAN),
            State::Ready { version, .. } => (format!("v{version} ready"), ORANGE),
            State::Failed(_) => ("Couldn't check".to_string(), ORANGE),
        };
        let dir = self.game_folder();
        let map_ok = crate::gamemap::has_map_files(&dir);
        let logs_ok = dir.join("logs").is_dir();
        let (game_val, game_c) = match (map_ok, logs_ok) {
            (true, true) => ("Found".to_string(), GREEN),
            (false, false) => ("Not found".to_string(), theme::HP_RED),
            _ => ("Partly found".to_string(), ORANGE),
        };
        let newest = auto_backups().first().map(|(_, t)| *t);
        let last = [newest, self.cfg.last_backup].into_iter().flatten().max();
        let (bk_val, bk_c) = match last {
            Some(t) => (ago(t), if self.cfg.auto_backup { GREEN } else { CYAN }),
            None if self.cfg.auto_backup => ("Soon".to_string(), CYAN),
            None => ("None yet".to_string(), ORANGE),
        };
        let moved = crate::config::data_dir_moved();
        let tiles = [
            (AdvTab::Updates, "⬆", PURPLE, "Version", format!("v{}", update::VERSION), up_val, up_c),
            (AdvTab::Game, "📁", GOLD, "Game folder", game_val.clone(), short_path(&dir), game_c),
            (AdvTab::Data, "💾", PINK, "Last backup", bk_val, if self.cfg.auto_backup { "Automatic copies on".into() } else { "Automatic copies off".into() }, bk_c),
            (AdvTab::Counter, "🎯", CYAN, "This run", theme::thousands(self.cfg.count), format!("{} battles", theme::thousands(self.cfg.battles)), MUTED()),
        ];
        let _ = moved;
        let avail = ui.available_width();
        let cols = if avail >= 900.0 { 4 } else { 2 };
        let gap = 14.0;
        let tw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut go: Option<AdvTab> = None;
        for row in tiles.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (tab, icon, col, title, value, sub, dot) in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw, 84.0), Sense::click());
                    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter();
                    let rr = r.translate(vec2(0.0, -2.0 * t));
                    if t > 0.0 {
                        p.add(kit::shadow_tier(2).as_shape(rr, 18.0));
                    }
                    p.rect_filled(rr, 18.0, theme::blend(kit::CARD(), kit::CARD_HOVER(), t));
                    p.rect_stroke(rr, 18.0, Stroke::new(1.0f32, kit::LINE()));
                    let chip = Rect::from_min_size(rr.min + vec2(14.0, 14.0), vec2(30.0, 30.0));
                    kit::icon_chip(p, chip, icon, *col, false);
                    p.text(pos2(chip.max.x + 10.0, chip.center().y), egui::Align2::LEFT_CENTER, *title, ch::bold(11.5), MUTED());
                    let g = p.layout_no_wrap(value.clone(), ch::heavy(17.0), TEXT());
                    let vx = rr.min.x + 14.0;
                    let vy = rr.max.y - 24.0;
                    let gw = g.size().x;
                    p.galley(pos2(vx, vy - g.size().y / 2.0), g, TEXT());
                    p.circle_filled(pos2(vx + gw + 10.0, vy), 4.0, *dot);
                    let sg = p.layout_no_wrap(sub.clone(), ch::reg(11.0), MUTED());
                    let sx = (vx + gw + 20.0).max(rr.max.x - 14.0 - sg.size().x);
                    if sx + sg.size().x <= rr.max.x - 10.0 {
                        p.galley(pos2(sx, vy - sg.size().y / 2.0), sg, MUTED());
                    }
                    if resp.on_hover_text(format!("{sub}\nClick to go to {title}")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        go = Some(*tab);
                    }
                }
            });
            ui.add_space(gap);
        }
        if go.is_some() {
            self.setup.adv_jump = go;
        }
    }

    // ───────────────────────────── cards ─────────────────────────────

    fn adv_updates_card(&mut self, ui: &mut egui::Ui) {
        use crate::update::{self, State};
        kit::card(ui, |ui| {
            card_head(ui, "⬆", PURPLE, "Updates", "New versions, straight from GitHub.");
            ui.add_space(14.0);
            let state = update::get(&self.updater);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("v{}", update::VERSION)).font(ch::heavy(22.0)).color(TEXT()));
                ui.add_space(6.0);
                let (text, kind) = match &state {
                    State::Idle => ("Not checked yet".to_string(), PillKind::Info),
                    State::Checking => ("Checking…".to_string(), PillKind::Info),
                    State::UpToDate => ("✔ Up to date".to_string(), PillKind::Ok),
                    State::Downloading { version, got, total } => (
                        if *total > 0 { format!("Downloading v{version}… {}%", got * 100 / (*total).max(1)) } else { format!("Downloading v{version}…") },
                        PillKind::Info,
                    ),
                    State::Ready { version, .. } => (format!("v{version} ready to install"), PillKind::Ok),
                    State::Failed(e) => (e.clone(), PillKind::Warn),
                };
                kit::pill(ui, &text, kind);
            });
            if let State::Downloading { got, total, .. } = &state {
                if *total > 0 {
                    ui.add_space(6.0);
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
                    ui.painter().rect_filled(r, 3.0, theme::blend(kit::CARD(), TEXT(), 0.09));
                    let f = (*got as f32 / *total as f32).clamp(0.0, 1.0);
                    ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(r.width() * f, 6.0)), 3.0, kit::primary());
                }
            }
            ui.add_space(12.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Update automatically",
                "Checks GitHub for a new version now and then and downloads it in the background. You choose when to restart.",
                &mut self.cfg.auto_update,
            );
            ui.add_space(10.0);
            let mut pre = self.cfg.prerelease_updates;
            setting_row(ui, "Builds", "Pre-release builds are test versions: newest features first, but they may have bugs.", |ui| {
                kit::segmented(ui, &mut pre, &[(false, "Releases"), (true, "Pre-releases too")]);
            });
            if pre != self.cfg.prerelease_updates {
                self.cfg.prerelease_updates = pre;
                self.dirty = true;
                // look again straight away with the new choice
                if matches!(state, State::Idle | State::UpToDate | State::Failed(_)) {
                    if let Ok(mut g) = self.updater.lock() {
                        *g = State::Idle;
                    }
                    self.last_update_check = Some(std::time::Instant::now());
                    self.update_dismissed = false;
                    update::check(self.updater.clone(), ui.ctx().clone(), self.cfg.installed_update.clone(), pre);
                }
            }
            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let state = update::get(&self.updater);
                if matches!(state, State::Ready { .. }) {
                    if kit::accent_button(ui, egui::Button::new(RichText::new("Restart to update").font(ch::bold(13.0)).color(kit::on_primary())), 6.0).clicked() {
                        let ctx = ui.ctx().clone();
                        self.restart_to_update(&ctx);
                    }
                } else if ui
                    .add_enabled(!matches!(state, State::Checking | State::Downloading { .. }), egui::Button::new("Check now").fill(kit::btn_fill()))
                    .clicked()
                {
                    if matches!(state, State::UpToDate | State::Failed(_)) {
                        if let Ok(mut g) = self.updater.lock() {
                            *g = State::Idle;
                        }
                    }
                    self.last_update_check = Some(std::time::Instant::now());
                    self.update_dismissed = false;
                    update::check(self.updater.clone(), ui.ctx().clone(), self.cfg.installed_update.clone(), self.cfg.prerelease_updates);
                }
                if ui.add(egui::Button::new("What's new ↗").fill(kit::btn_fill())).on_hover_text("The releases page on GitHub").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://github.com/{}/releases", update::REPO)));
                }
            });
        });
    }

    fn adv_backup_card(&mut self, ui: &mut egui::Ui) {
        let autos = auto_backups();
        kit::card(ui, |ui| {
            card_head(ui, "💾", PINK, "Backups", "Hunts, runs, stats, achievements and settings, all in one file.");
            ui.add_space(14.0);
            let busy = self.setup.backup_picker.is_some();
            let start = dirs::document_dir().unwrap_or_default();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                if kit::accent_button(ui, egui::Button::new(RichText::new("Save a backup…").font(ch::bold(13.0)).color(kit::on_primary())), 6.0).clicked() && !busy {
                    let (y, m, d) = crate::stats::civil_from_days(crate::stats::local_day(crate::stats::now_unix()));
                    let name = format!("Shiny Tracker backup {y:04}-{m:02}-{d:02}.json");
                    self.setup.backup_picker = Some((true, crate::share::pick_backup(true, start.clone(), name, ui.ctx().clone())));
                }
                if ui.add_enabled(!busy, egui::Button::new("Restore from a file…").fill(kit::btn_fill())).clicked() {
                    self.setup.backup_picker = Some((false, crate::share::pick_backup(false, start.clone(), String::new(), ui.ctx().clone())));
                }
            });
            if let Some(t) = self.cfg.last_backup {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("You last saved one {}.", ago(t).to_lowercase())).size(12.0).color(MUTED()));
            }
            self.adv_restore_question(ui);
            if let Some((ok, msg)) = &self.setup.backup_msg {
                ui.add_space(8.0);
                kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            }
            ui.add_space(14.0);
            divider(ui);
            ui.add_space(12.0);
            kit::toggle_row(
                ui,
                "Automatic copies",
                "A copy of everything when the Companion opens and every hour while it's open, only when something changed. The 5 newest are kept: each new copy replaces the oldest. An empty save is never copied, so it can't push out your good ones. They stay on this PC.",
                &mut self.cfg.auto_backup,
            );
            if self.cfg.auto_backup {
                ui.add_space(8.0);
                if autos.is_empty() {
                    ui.label(RichText::new("The first copy is made once there is something to keep.").size(12.0).color(MUTED()));
                } else {
                    let mut restore: Option<std::path::PathBuf> = None;
                    for (i, (path, t)) in autos.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(8.0, 18.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 3.5, if i == 0 { GREEN } else { kit::LINE() });
                            ui.label(RichText::new(day_label(*t)).size(12.5).color(TEXT()));
                            if i == 0 {
                                ui.label(RichText::new("newest").size(11.0).color(MUTED()));
                            }
                            ui.add_space((ui.available_width() - 70.0).max(4.0));
                            if ui.add(egui::Button::new(RichText::new("Restore").size(12.0)).fill(kit::btn_fill())).on_hover_text(path.display().to_string()).clicked() {
                                restore = Some(path.clone());
                            }
                        });
                    }
                    if let Some(p) = restore {
                        self.load_backup(p);
                    }
                }
            }
        });
    }

    /// "Replace everything with this backup?"
    fn adv_restore_question(&mut self, ui: &mut egui::Ui) {
        let Some((path, c)) = &self.setup.restore_pending else { return };
        ui.add_space(10.0);
        let run = self.target_name(c.target.as_deref());
        let text = format!(
            "Replace everything with this backup? It has {} caught shinies and a {} run at {} encounters. What you have now is saved first as before-restore.json.",
            c.history.len(),
            run,
            theme::thousands(c.count)
        );
        let path_s = path.display().to_string();
        let mut go = false;
        let mut cancel = false;
        egui::Frame::none().fill(ch::faint(GOLD, 0.14)).rounding(12.0).inner_margin(egui::Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(text).size(12.5).color(TEXT())).wrap()).on_hover_text(path_s);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                go = kit::accent_button(ui, egui::Button::new(RichText::new("Replace").font(ch::bold(13.0)).color(kit::on_primary())), 6.0).clicked();
                cancel = ui.add(egui::Button::new("Cancel").fill(kit::btn_fill())).clicked();
            });
        });
        if go {
            if let Some((_, c)) = self.setup.restore_pending.take() {
                let safety = crate::config::data_dir().join("before-restore.json");
                let _ = std::fs::create_dir_all(crate::config::data_dir());
                if let Ok(json) = serde_json::to_string_pretty(&self.cfg) {
                    let _ = std::fs::write(&safety, json);
                }
                self.cfg = *c;
                self.reset_transient();
                self.push_settings(true);
                self.apply_keybinds();
                self.dirty = true;
                self.setup.backup_msg = Some((true, "✔ Restored from the backup".into()));
            }
        } else if cancel {
            self.setup.restore_pending = None;
        }
    }

    fn adv_data_card(&mut self, ui: &mut egui::Ui) {
        let here = crate::config::data_dir();
        let moved = crate::config::data_dir_moved();
        kit::card(ui, |ui| {
            card_head(ui, "📂", CYAN, "Data folder", "Where your saves and downloaded sprites live.");
            ui.add_space(14.0);
            path_box(ui, &here.display().to_string(), if moved { "Moved by you" } else { "The usual place" });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                if ui.add(egui::Button::new("Open folder").fill(kit::btn_fill())).clicked() {
                    crate::share::open_dir(&here);
                }
                let busy = self.setup.data_picker.is_some();
                if ui
                    .add_enabled(!busy, egui::Button::new("Move to…").fill(kit::btn_fill()))
                    .on_hover_text("Pick a folder (another drive, a synced folder): a ShinyTracker folder is made inside it and everything comes along")
                    .clicked()
                {
                    let start = dirs::document_dir().unwrap_or_default();
                    self.setup.data_picker = Some(crate::share::pick_folder(start, ui.ctx().clone()));
                }
                if moved && ui.add_enabled(!busy, egui::Button::new("Back to default").fill(kit::btn_fill())).on_hover_text("Move everything back to AppData").clicked() {
                    self.move_data(None);
                }
            });
            if let Some((ok, msg)) = &self.setup.data_msg {
                ui.add_space(8.0);
                kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            }
        });
    }

    fn adv_game_card(&mut self, ui: &mut egui::Ui) {
        let chosen = self.cfg.game_dir.is_some();
        let dir = self.game_folder();
        let map_ok = crate::gamemap::has_map_files(&dir);
        let logs_ok = dir.join("logs").is_dir();
        kit::card(ui, |ui| {
            card_head(ui, "📁", GOLD, "Game folder", "Counting from the game log and the Map read from here.");
            ui.add_space(14.0);
            let how = if chosen {
                "Chosen by you"
            } else if self.cfg.game_dir_found.is_some() {
                "Found where the game runs"
            } else {
                "The usual place"
            };
            path_box(ui, &dir.display().to_string(), how);
            ui.add_space(10.0);
            check_row(ui, map_ok, "Map files", if map_ok { "The Map can show where you are" } else { "The Map needs these" });
            check_row(ui, logs_ok, "Game logs", if logs_ok { "Counting from the game log works" } else { "Counting from the game log needs these" });
            if !(map_ok && logs_ok) {
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(
                        RichText::new("Normally AppData\\Local\\PlatinumTogether\\Game. Moved the game? Start it and it's found by itself, or choose its folder (the one with Platinum.exe).")
                            .size(11.5)
                            .color(MUTED()),
                    )
                    .wrap(),
                );
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let busy = self.setup.log_picker.is_some();
                if ui.add_enabled(!busy, egui::Button::new("Choose folder…").fill(kit::btn_fill())).clicked() {
                    let start = if dir.is_dir() { dir.clone() } else { dirs::data_local_dir().unwrap_or_default() };
                    self.setup.log_picker = Some(crate::share::pick_folder(start, ui.ctx().clone()));
                }
                if chosen
                    && ui
                        .add_enabled(!busy, egui::Button::new("Find automatically").fill(kit::btn_fill()))
                        .on_hover_text("Forget the chosen folder and look in the usual place")
                        .clicked()
                {
                    self.cfg.game_dir = None;
                    self.game_folder_changed();
                }
                if dir.is_dir() && ui.add(egui::Button::new("Open folder").fill(kit::btn_fill())).clicked() {
                    crate::share::open_dir(&dir);
                }
            });
        });
    }

    fn adv_counter_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "🎯", CYAN, "Edit the counter", "Fix this run's numbers by hand.");
            ui.add_space(14.0);
            let mut changed = false;
            for (label, help, v) in [
                ("Encounters", "Every Pokémon you've met on this run.", &mut self.cfg.count),
                ("Battles", "Wild battles (a double battle is one).", &mut self.cfg.battles),
                ("Target seen", "How often your target showed up.", &mut self.cfg.target_seen),
            ] {
                setting_row(ui, label, help, |ui| {
                    changed |= ui.add(egui::DragValue::new(v).speed(1.0).range(0..=u64::MAX / 2)).changed();
                });
                ui.add_space(8.0);
            }
            if changed {
                self.dirty = true;
            }
            ui.add_space(4.0);
            if self.setup.confirm_reset {
                egui::Frame::none().fill(ch::faint(theme::HP_RED, 0.12)).rounding(12.0).inner_margin(egui::Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Set this run back to 0? This can't be undone.").size(12.5).color(TEXT()));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("Set to 0").color(Color32::WHITE)).fill(theme::HP_RED)).clicked() {
                            self.cfg.count = 0;
                            self.cfg.battles = 0;
                            self.cfg.target_seen = 0;
                            self.dirty = true;
                            self.setup.confirm_reset = false;
                        }
                        if ui.add(egui::Button::new("Cancel").fill(kit::btn_fill())).clicked() {
                            self.setup.confirm_reset = false;
                        }
                    });
                });
            } else if ui.add(egui::Button::new("Set to 0…").fill(kit::btn_fill())).clicked() {
                self.setup.confirm_reset = true;
            }
        });
    }

    fn adv_reading_card(&mut self, ui: &mut egui::Ui) {
        let d = crate::config::Config::default();
        let mut changed = false;
        kit::card(ui, |ui| {
            card_head(ui, "👁", PURPLE, "Screen reading", "For counting from names on screen. The usual values suit most PCs.");
            ui.add_space(14.0);
            ui.spacing_mut().slider_width = (ui.available_width() - 200.0).clamp(120.0, 260.0);
            changed |= slider_row(ui, "Matching reads", "How many times a name must be read the same before it counts. Higher is safer, lower is quicker.", &mut self.cfg.stable_reads, 1..=5, d.stable_reads, "");
            changed |= slider_row(ui, "Empty reads", "How many reads with no name before a battle counts as over.", &mut self.cfg.rearm_reads, 1..=10, d.rearm_reads, "");
            let mut iv = self.cfg.interval_ms as u32;
            if slider_row(ui, "Scan every", "How often the screen is read. Lower notices faster but uses more CPU.", &mut iv, 100..=1000, d.interval_ms as u32, " ms") {
                self.cfg.interval_ms = iv as u64;
                changed = true;
            }
            ui.add_space(4.0);
            divider(ui);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("SHINY STAR ALERT").font(ch::bold(10.5)).color(MUTED()));
                kit::pill(ui, "Experimental", PillKind::Warn);
            });
            ui.add_space(8.0);
            changed |= slider_row(
                ui,
                "Star size",
                "It looks for the ★ next to the name and only alerts when it stays put for a moment. Lower this if your game window is small and real stars are missed.",
                &mut self.cfg.star_threshold,
                48..=400,
                d.star_threshold,
                "",
            );
            let off = self.cfg.stable_reads != d.stable_reads || self.cfg.rearm_reads != d.rearm_reads || self.cfg.interval_ms != d.interval_ms || self.cfg.star_threshold != d.star_threshold;
            if off {
                ui.add_space(4.0);
                if ui.add(egui::Button::new("Back to the usual values").fill(kit::btn_fill())).clicked() {
                    self.cfg.stable_reads = d.stable_reads;
                    self.cfg.rearm_reads = d.rearm_reads;
                    self.cfg.interval_ms = d.interval_ms;
                    self.cfg.star_threshold = d.star_threshold;
                    changed = true;
                }
            }
        });
        if changed {
            self.dirty = true;
            self.push_settings(false);
        }
    }

    fn adv_help_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "💡", GREEN, "Help", "Start over with the tour, or send the dev what they need to help.");
            ui.add_space(14.0);
            setting_row(ui, "Welcome tour", "The step-by-step setup from your first start: game, look, overlay, target and odds. Your hunt and settings are kept.", |ui| {
                if ui.add(egui::Button::new("Start the tour").fill(kit::btn_fill())).clicked() {
                    self.cfg.onboarding_seen = 0;
                    self.setup.welcome_step = 0;
                    self.dirty = true;
                }
            });
            ui.add_space(10.0);
            let info = self.app_info();
            let cid = egui::Id::new("adv-info-copied");
            let now = ui.input(|i| i.time);
            let copied = ui.ctx().data(|d| d.get_temp::<f64>(cid)).is_some_and(|t| now - t < 2.0);
            setting_row(ui, "App info for a bug report", "Version, Windows and your main settings: no hunts, names or logs. Paste it into your message to the dev.", |ui| {
                if ui.add(egui::Button::new(if copied { "Copied ✔" } else { "Copy" }).fill(kit::btn_fill())).on_hover_text(info.clone()).clicked() {
                    ui.ctx().copy_text(info.clone());
                    ui.ctx().data_mut(|d| d.insert_temp(cid, now));
                }
            });
            if copied {
                ui.ctx().request_repaint();
            }
            ui.add_space(10.0);
            setting_row(ui, "Report a problem", "Opens the issues page on GitHub.", |ui| {
                if ui.add(egui::Button::new("Open ↗").fill(kit::btn_fill())).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://github.com/{}/issues", crate::update::REPO)));
                }
            });
        });
    }

    /// A short, private-data-free description of the setup.
    fn app_info(&self) -> String {
        let a = &self.cfg.appearance;
        let style = if a.modern_ui { "Modern" } else if a.clean_settings { "Clean" } else { "Classic" };
        format!(
            "Poke Together Companion v{}\nOS: {} {}\nLook: {} / {:?}\nOverlay: {} theme, {:?}, {}\nCounting: {:?}, method {:?}\nHotkeys: {} set{}",
            crate::update::VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH,
            style,
            a.mode,
            self.cfg.overlay.theme.name(),
            self.cfg.overlay.size,
            if self.cfg.overlay.legacy() { "legacy" } else { "modular" },
            self.cfg.count_source,
            self.cfg.hunt_method,
            crate::hotkeys::HotAction::ALL.iter().filter(|x| self.cfg.keybinds.get(**x).is_some()).count(),
            if self.cfg.keybinds.paused { " (all off)" } else { "" },
        )
    }
}

/// Nothing worth keeping: no runs, no catches, no count, no hunt time.
pub(crate) fn is_blank(c: &crate::config::Config) -> bool {
    c.is_blank()
}

/// Write one automatic copy (unless the save is empty or unchanged since the
/// newest copy) and remove the oldest beyond `AUTO_KEEP`. Returns the new
/// file, if one was written.
pub(crate) fn write_auto_backup(c: &crate::config::Config) -> Option<std::path::PathBuf> {
    if is_blank(c) {
        return None;
    }
    let json = serde_json::to_string_pretty(c).ok()?;
    let list = auto_backups();
    if list.first().and_then(|(p, _)| std::fs::read_to_string(p).ok()).is_some_and(|old| old == json) {
        return None;
    }
    let dir = crate::config::data_dir().join("backups");
    let _ = std::fs::create_dir_all(&dir);
    let now = crate::stats::now_unix();
    let (y, m, d) = crate::stats::civil_from_days(crate::stats::local_day(now));
    let secs = (now as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400);
    let name = format!("auto-{y:04}-{m:02}-{d:02}_{:02}-{:02}-{:02}.json", secs / 3600, secs / 60 % 60, secs % 60);
    let path = dir.join(name);
    // write then rename, so a half-written copy never counts as the newest
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).ok()?;
    std::fs::rename(&tmp, &path).ok()?;
    for (p, _) in auto_backups().into_iter().skip(AUTO_KEEP) {
        let _ = std::fs::remove_file(p);
    }
    Some(path)
}

/// The automatic copies, newest first (path, when).
pub(crate) fn auto_backups() -> Vec<(std::path::PathBuf, u64)> {
    let dir = crate::config::data_dir().join("backups");
    let mut v: Vec<(std::path::PathBuf, u64)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("auto-") && n.ends_with(".json")
        })
        .filter_map(|e| {
            let t = e.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            Some((e.path(), t))
        })
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    v
}

/// "Today", "Yesterday", "3 days ago".
fn ago(t: u64) -> String {
    let today = crate::stats::local_day(crate::stats::now_unix());
    match today - crate::stats::local_day(t) {
        d if d <= 0 => "Today".into(),
        1 => "Yesterday".into(),
        d => format!("{d} days ago"),
    }
}

/// "Today · 12 Nov".
fn day_label(t: u64) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, d) = crate::stats::civil_from_days(crate::stats::local_day(t));
    let secs = (t as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400);
    let (h, mi) = (secs / 3600, secs / 60 % 60);
    let (h12, ap) = (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "am" } else { "pm" });
    format!("{} · {d} {} · {h12}:{mi:02} {ap}", ago(t), M[(m as usize).saturating_sub(1).min(11)])
}

/// The last two parts of a path, for a tile.
fn short_path(p: &std::path::Path) -> String {
    let parts: Vec<String> = p.components().rev().take(2).map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let mut parts = parts;
    parts.reverse();
    format!("…\\{}", parts.join("\\"))
}

fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
            ui.label(RichText::new(sub).size(11.5).color(MUTED()));
        });
    });
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

/// A folder path in a box, with how it was picked.
fn path_box(ui: &mut egui::Ui, path: &str, how: &str) {
    egui::Frame::none()
        .fill(kit::CARD_HOVER())
        .stroke(Stroke::new(1.0f32, kit::LINE()))
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(how.to_uppercase()).font(ch::bold(9.5)).color(MUTED()));
            ui.add(egui::Label::new(RichText::new(path).size(12.5).color(TEXT())).truncate()).on_hover_text(path);
        });
}

/// ✔ / ! and what it means.
fn check_row(ui: &mut egui::Ui, ok: bool, what: &str, note: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        let c = if ok { GREEN } else { ORANGE };
        let p = ui.painter();
        p.circle_filled(r.center(), 8.0, ch::faint(c, 0.2));
        if ok {
            p.add(egui::Shape::line(vec![r.center() + vec2(-3.5, 0.0), r.center() + vec2(-1.0, 2.8), r.center() + vec2(3.8, -2.8)], Stroke::new(1.8f32, ch::ink(c))));
        } else {
            p.text(r.center(), egui::Align2::CENTER_CENTER, "!", ch::heavy(11.0), ch::ink(c));
        }
        ui.label(RichText::new(if ok { format!("{what} found") } else { format!("{what} not found") }).font(ch::bold(12.5)).color(TEXT()));
        ui.label(RichText::new(note).size(11.5).color(MUTED()));
    });
}

/// Label and help on the left, the control flush right.
fn setting_row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    super::appearance_page::setting_row(ui, label, help, control);
}

/// A slider with its label and a mark at the usual value.
fn slider_row(ui: &mut egui::Ui, label: &str, help: &str, v: &mut u32, range: std::ops::RangeInclusive<u32>, usual: u32, suffix: &str) -> bool {
    let mut changed = false;
    setting_row(ui, label, help, |ui| {
        let text = format!("{v}{suffix}");
        changed = kit::slider(ui, egui::Slider::new(v, range), &text).changed();
    });
    if *v != usual {
        ui.label(RichText::new(format!("Usually {usual}{suffix}")).size(11.0).color(MUTED()));
    }
    ui.add_space(8.0);
    changed
}
