//! Settings pages added later: per-run statistics, the shiny trophy case,
//! the share image maker and appearance (light/dark + accent colours).

use super::ui_kit::{self as kit, MUTED, PillKind, CARD, CARD_HOVER, LINE, TEXT};
use super::{App, GifReady, RunRef, ShareView, Tab};
use crate::card::{CardData, CardLayout, CardOptions, CardTheme};
use crate::config::{Appearance, UiMode};
use crate::stats::{fmt_date, fmt_duration, RunStats};
use crate::theme::{self, GOLD};
use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

/// What an open file box is for.
pub(super) enum Pick {
    Folder,
    /// Save these bytes with this extension where the user chooses.
    SaveAs(Vec<u8>, &'static str),
}

/// Everything about one run, copied out so the UI can borrow `self` freely.
pub(super) struct RunInfo {
    pub name: String,
    pub slug: Option<String>,
    pub count: u64,
    pub battles: u64,
    pub started: u64,
    pub ended: Option<u64>,
    pub caught: bool,
    pub current: bool,
    pub stats: Option<RunStats>,
    pub method: String,
    pub odds: Option<f64>,
    pub target_seen: u64,
    pub run_id: u64,
    /// A phase shiny (found during another hunt).
    pub phase: bool,
}

/// Which hunt a share image's nickname and gender belong to.
fn card_key(info: &RunInfo) -> String {
    format!("{}:{}:{}", info.run_id, info.started, info.slug.as_deref().unwrap_or(&info.name))
}

/// Which signs a species can have, from its Pokédex gender line
/// ("Genderless", "100% male", "50% male, 50% female"…). Unknown = both.
fn gender_choices(line: Option<&str>) -> (bool, bool) {
    let Some(l) = line.map(|l| l.to_lowercase()) else { return (true, true) };
    if l.contains("genderless") {
        return (false, false);
    }
    // The percentage written just before "% male" / "% female".
    let pct = |word: &str| -> Option<f32> {
        let at = l.find(word)?;
        let num: String = l[..at].chars().rev().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>().chars().rev().collect();
        num.parse().ok()
    };
    match (pct("% male"), pct("% female")) {
        (None, None) => (true, true),
        (m, f) => (m.unwrap_or(0.0) > 0.0, f.unwrap_or(0.0) > 0.0),
    }
}

const ACCENTS: [(&str, [u8; 3]); 9] = [
    ("Red", [214, 52, 58]),
    ("Orange", [236, 112, 36]),
    ("Gold", [214, 162, 24]),
    ("Green", [44, 160, 86]),
    ("Teal", [22, 150, 160]),
    ("Blue", [52, 116, 218]),
    ("Purple", [128, 78, 208]),
    ("Pink", [222, 68, 146]),
    ("Slate", [86, 92, 120]),
];

fn c3(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub(super) fn back_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(format!("‹  {label}")).size(kit::fs(12.5)).color(MUTED()))
            .frame(false),
    )
    .clicked()
}

pub(super) fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let text = if kit::clean() { text.trim() } else { text };
    kit::accent_button(
        ui,
        egui::Button::new(RichText::new(text).family(kit::strong_family(13.5)).strong().color(kit::on_primary())).min_size(egui::vec2(0.0, 34.0)),
        8.0,
    )
}

pub(super) fn secondary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(TEXT()))
            .fill(kit::btn_fill())
            .rounding(kit::btn_round(8.0))
            .min_size(egui::vec2(0.0, if kit::clean() { kit::CONTROL_H } else { 34.0 })),
    )
}

impl App {
    pub(super) fn run_info(&self, r: RunRef) -> Option<RunInfo> {
        match r {
            RunRef::Current => Some(RunInfo {
                name: self.target_name(self.cfg.target.as_deref()),
                slug: self.cfg.target.clone(),
                count: self.cfg.count,
                battles: self.cfg.battles,
                started: self.cfg.run_started_unix,
                ended: None,
                caught: false,
                current: true,
                stats: Some(self.cfg.run_stats.clone()),
                method: self.cfg.odds_caption(),
                odds: self.cfg.odds_one_in(),
                target_seen: self.cfg.target_seen,
                run_id: self.cfg.run_id,
                phase: false,
            }),
            RunRef::Saved(id) => {
                let s = self.cfg.saved_runs.iter().find(|x| x.id == id)?;
                Some(RunInfo {
                    name: self.target_name(s.target.as_deref()),
                    slug: s.target.clone(),
                    count: s.count,
                    battles: s.battles,
                    started: s.started_unix,
                    ended: None,
                    caught: false,
                    current: false,
                    stats: Some(s.stats.clone()),
                    method: self.cfg.odds_caption(),
                    odds: self.cfg.odds_one_in(),
                    target_seen: s.target_seen,
                    run_id: s.id,
                    phase: false,
                })
            }
            RunRef::Finished(i) => {
                let h = self.cfg.history.get(i)?;
                let slug = h.slug.clone().or_else(|| {
                    self.species
                        .iter()
                        .find(|s| s.name.eq_ignore_ascii_case(&h.target))
                        .map(|s| s.slug.clone())
                });
                Some(RunInfo {
                    name: h.target.clone(),
                    slug,
                    count: h.encounters,
                    battles: h.battles,
                    started: h.started_unix,
                    ended: Some(h.when_unix),
                    caught: true,
                    current: false,
                    stats: h.stats.clone(),
                    method: h.method.clone(),
                    odds: h.odds_one_in,
                    target_seen: h.target_seen,
                    run_id: h.run_id,
                    phase: h.phase,
                })
            }
        }
    }

    /// Where share images are saved.
    pub(super) fn share_folder(&self) -> std::path::PathBuf {
        self.cfg
            .share_folder
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(crate::share::default_share_dir)
    }

    pub(super) fn species_of(&self, slug: Option<&str>) -> Option<(usize, crate::species::Species)> {
        let slug = slug?;
        self.species
            .iter()
            .position(|s| s.slug == slug)
            .map(|i| (i, self.species[i].clone()))
    }

    /// Make sure both sprites are downloading/cached for the share image.
    fn request_sprites(&mut self, slug: Option<&str>) {
        if let Some((_, sp)) = self.species_of(slug) {
            let _ = self.sprites.get(&sp, true);
            let _ = self.sprites.get(&sp, false);
        }
    }

    pub(super) fn pass_card_data(&self, info: &RunInfo) -> CardData {
        self.card_data(info)
    }

    fn card_data(&self, info: &RunInfo) -> CardData {
        let st = info.stats.clone().unwrap_or_default();
        let dex = self.species_of(info.slug.as_deref()).map(|(i, _)| i as u32 + 1);
        let (shiny, normal) = match &info.slug {
            Some(s) => (crate::share::sprite_frame(s, true), crate::share::sprite_frame(s, false)),
            None => (None, None),
        };
        CardData {
            name: info.name.clone(),
            dex,
            caught: info.caught,
            encounters: info.count,
            odds_one_in: info.odds,
            method: info.method.clone(),
            hunt_secs: st.hunt_secs,
            per_hour: st.per_hour(),
            days_hunted: st.days_hunted(),
            started: (info.started > 0).then(|| fmt_date(info.started)),
            finished: info.ended.map(fmt_date),
            shiny,
            normal,
            icon: Some(crate::share::icon()),
            sprite_box: None,
            trainer_level: Some(self.cfg.level_seen.max(1)),
            note: st.notes.last().map(|n| n.text.clone()),
            target_seen: (!info.phase && info.target_seen > 0).then_some(info.target_seen),
            odds_on_target: self.cfg.odds_target_only && !info.phase && info.target_seen > 0,
            phases: if info.phase { Vec::new() } else { self.phases_of(info.run_id).iter().map(|&i| self.cfg.history[i].target.clone()).collect() },
            nickname: self.cfg.card_names.get(&card_key(info)).map(|n| n.0.trim().to_string()).filter(|n| !n.is_empty()),
            gender: self.cfg.card_names.get(&card_key(info)).map(|n| n.1).unwrap_or_default(),
            avatar: super::avatar::card_avatar(),
            plate: self.cfg.plate.clone(),
            frame: self.cfg.frame.clone(),
        }
    }

    pub(super) fn card_fonts_pub(&mut self) -> &crate::share::AbText {
        self.card_fonts()
    }

    fn card_fonts(&mut self) -> &crate::share::AbText {
        let reload = match &self.card_text {
            None => true,
            Some(t) => !t.has_pixel() && crate::config::cache_dir().join("PressStart2P-Regular.ttf").exists(),
        };
        if reload {
            self.card_text = Some(crate::share::AbText::load());
        }
        self.card_text.as_ref().expect("fonts loaded")
    }

    /// Share options with the profile name filled in when none is set.
    fn card_opts(&self) -> CardOptions {
        let mut o = self.cfg.card.clone();
        if o.trainer.trim().is_empty() {
            o.trainer = self.cfg.trainer_name.clone();
        }
        o
    }

    fn render_card(&mut self, info: &RunInfo) -> crate::canvas::Canvas {
        let data = self.card_data(info);
        let opts = self.card_opts();
        let fonts = self.card_fonts();
        crate::card::render(&data, &opts, fonts)
    }

    pub(super) fn open_share(&mut self, src: RunRef, back_to: Option<RunRef>) {
        if let Some(info) = self.run_info(src) {
            self.request_sprites(info.slug.as_deref());
            // This hunt's nickname and gender, if it has them.
            let (nick, gender) = self.cfg.card_names.get(&card_key(&info)).cloned().unwrap_or_default();
            self.cfg.card.nickname = nick;
            self.cfg.card.gender = gender;
        }
        self.setup.share = Some(ShareView {
            src,
            tex: None,
            image: None,
            drawn: None,
            msg: None,
            saved: None,
            back_to,
            gif_job: None,
            gif: None,
            changed_at: Instant::now() - Duration::from_secs(5),
            picker: None,
        });
    }

    /// One click: render with the saved options and copy to the clipboard.
    fn quick_copy(&mut self, src: RunRef) -> Result<(), String> {
        let info = self.run_info(src).ok_or("run not found")?;
        let img = self.render_card(&info);
        crate::share::copy_image(&img)
    }

    // ───────────────────────────── run stats ─────────────────────────────

    pub(super) fn page_run_stats(&mut self, ui: &mut egui::Ui, run: RunRef) {
        if back_button(ui, "Runs") {
            self.setup.view_run = None;
            return;
        }
        let Some(info) = self.run_info(run) else {
            self.setup.view_run = None;
            return;
        };
        ui.add_space(4.0);
        let st = info.stats.clone().unwrap_or_default();
        let mut tab = if self.setup.run_tab == RunTab::Journal { RunTab::Journal } else { RunTab::Overview };
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 210.0).max(240.0);
            ui.allocate_ui_with_layout(egui::vec2(w, 44.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(w);
                kit::tab_bar(ui, &mut tab, &[(RunTab::Overview, "Stats"), (RunTab::Journal, "Journal")]);
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary(ui, "  🖼  Make share image  ").clicked() {
                    self.open_share(run, Some(run));
                }
            });
        });
        self.setup.run_tab = tab;
        ui.add_space(14.0);
        match tab {
            RunTab::Journal => self.run_journal(ui, run, &info, &st),
            _ => self.hunt_dashboard(ui, &info, &st),
        }
    }

    /// Run stats → Journal: notes, milestones and phases.
    fn run_journal(&mut self, ui: &mut egui::Ui, run: RunRef, info: &RunInfo, st: &RunStats) {
        self.journal_section(ui, run, info.count);
        // Milestones
        if !st.milestones.is_empty() {
            kit::section(ui, "Milestones");
            kit::card(ui, |ui| {
                for m in st.milestones.iter().rev().take(8) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✨").color(GOLD));
                        ui.label(RichText::new(&m.label).size(kit::fs(12.5)).color(TEXT()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(fmt_date(m.unix)).size(kit::fs(11.5)).color(MUTED()));
                        });
                    });
                }
            });
        }

        if !info.phase {
            self.phases_section(ui, info.run_id);
        }
    }

    // ───────────────────────────── trophies ─────────────────────────────

    pub(super) fn page_trophies(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "TROPHIES", "Every shiny you've caught. Share any of them as an image.");
        self.add_event("trophy_room");
        let n = self.cfg.history.len();
        if n == 0 {
            kit::card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new("🏆").size(kit::fs(31.5)).color(GOLD));
                    ui.label(RichText::new("No shinies yet").size(kit::fs(14.5)).family(kit::strong_family(kit::fs(14.5))).strong().color(TEXT()));
                    ui.label(
                        RichText::new("When you get one, press Caught! on the overlay and it's added here.")
                            .size(kit::fs(12.0))
                            .color(MUTED()),
                    );
                    ui.add_space(10.0);
                });
            });
            return;
        }
        let total: u64 = self.cfg.history.iter().filter(|h| !h.phase).map(|h| h.encounters).sum();
        let luckiest = self.cfg.history.iter().filter(|h| !h.phase).min_by_key(|h| h.encounters).cloned();
        let w3 = ((ui.available_width() - 16.0) / 3.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            kit::stat(ui, w3, "Shinies caught", &n.to_string());
            kit::stat(ui, w3, "Average encounters", &theme::thousands(total / n as u64));
            kit::stat(
                ui,
                w3,
                "Luckiest",
                &luckiest.map_or("—".into(), |h| format!("{} · {}", h.target, theme::thousands(h.encounters))),
            );
        });
        ui.add_space(12.0);
        let order: Vec<usize> = (0..n).rev().collect();
        self.trophy_grid(ui, &order);
    }

    /// Trophy cards for these finished hunts (indices into history), with
    /// Share / Copy image buttons; clicking the picture opens the run's stats.
    pub(super) fn trophy_grid(&mut self, ui: &mut egui::Ui, order: &[usize]) {
        if let Some((at, ok, msg)) = self.setup.toast.clone() {
            if at.elapsed() < Duration::from_secs(4) {
                kit::pill(ui, &msg, if ok { PillKind::Ok } else { PillKind::Bad });
                ui.add_space(8.0);
                ui.ctx().request_repaint_after(Duration::from_millis(250));
            }
        }
        let gap = 10.0;
        let cols = (((ui.available_width() + gap) / (220.0 + gap)).floor() as usize).clamp(2, 8);
        let tw = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut share = None;
        let mut copy = None;
        let mut stats_of = None;
        for row in order.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    let Some(info) = self.run_info(RunRef::Finished(i)) else { continue };
                    // Top-down layout: the row is horizontal, and without this the
                    // card's picture, name, stats and buttons all got laid out in
                    // one line on top of each other.
                    ui.allocate_ui_with_layout(egui::vec2(tw, 262.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::Frame::none()
                            .fill(CARD())
                            .rounding(kit::r(12.0))
                            .stroke(Stroke::new(1.5f32, GOLD.gamma_multiply(0.55)))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.set_width(tw - 20.0);
                                let (r, resp) = ui.allocate_exact_size(egui::vec2(tw - 20.0, 112.0), Sense::click());
                                let p = ui.painter_at(r);
                                p.rect_filled(r, kit::r(10.0), CARD_HOVER());
                                theme::platform(&p, r.center_bottom() - egui::vec2(0.0, 14.0), r.width() * 0.3, 8.0);
                                if let Some((_, sp)) = self.species_of(info.slug.as_deref()) {
                                    self.draw_sprite(&p, &sp, true, r.center_bottom() - egui::vec2(0.0, 12.0), egui::vec2(r.width() - 20.0, 96.0));
                                    let _ = self.sprites.get(&sp, false);
                                }
                                theme::sparkle(&p, r.right_top() + egui::vec2(-14.0, 14.0), 6.0, GOLD);
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("View stats").clicked() {
                                    stats_of = Some(i);
                                }
                                ui.add_space(4.0);
                                ui.label(RichText::new(&info.name).size(kit::fs(14.5)).family(kit::strong_family(kit::fs(14.5))).strong().color(TEXT()));
                                ui.label(
                                    RichText::new(format!("{} encounters", theme::thousands(info.count)))
                                        .size(kit::fs(12.0))
                                        .color(theme::LINK()),
                                );
                                let mut line = Vec::new();
                                if !info.method.is_empty() {
                                    line.push(info.method.clone());
                                }
                                if let Some(st) = &info.stats {
                                    if st.hunt_secs >= 60 {
                                        line.push(format!("{} hunting", fmt_duration(st.hunt_secs)));
                                    }
                                }
                                ui.label(RichText::new(line.join(" · ")).size(kit::fs(10.5)).color(MUTED()));
                                ui.label(
                                    RichText::new(format!("Caught {}", info.ended.map(fmt_date).unwrap_or_default()))
                                        .size(kit::fs(10.5))
                                        .color(MUTED()),
                                );
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if kit::accent_button(ui, egui::Button::new(RichText::new("Share").family(kit::strong_family(13.5)).strong().color(kit::on_primary())), 6.0)
                                        .on_hover_text("Make a share image with your own theme and options")
                                        .clicked()
                                    {
                                        share = Some(i);
                                    }
                                    if ui
                                        .add(egui::Button::new("Copy image").fill(kit::btn_fill()).rounding(kit::btn_round(6.0)))
                                        .on_hover_text("Copies the image so you can paste it straight into Discord (Ctrl+V)")
                                        .clicked()
                                    {
                                        copy = Some(i);
                                    }
                                });
                            });
                    });
                }
            });
            ui.add_space(gap);
        }
        if let Some(i) = share {
            self.open_share(RunRef::Finished(i), None);
        }
        if let Some(i) = stats_of {
            self.setup.tab = Tab::Runs;
            self.setup.view_run = Some(RunRef::Finished(i));
        }
        if let Some(i) = copy {
            let res = self.quick_copy(RunRef::Finished(i));
            if res.is_ok() {
                self.add_event("show_off");
            }
            self.setup.toast = Some(match res {
                Ok(()) => (Instant::now(), true, "✔ Copied! Paste it into Discord with Ctrl+V".to_string()),
                Err(e) => (Instant::now(), false, format!("Couldn't copy: {e}")),
            });
        }
    }

    // ───────────────────────────── share maker ─────────────────────────────

    /// Handle a folder / "Save as" box that has finished.
    pub(super) fn poll_picker(&mut self) {
        let mut picked = None;
        if let Some(sv) = self.setup.share.as_mut() {
            if let Some((_, rx)) = &sv.picker {
                if let Ok(res) = rx.try_recv() {
                    picked = sv.picker.take().map(|(pick, _)| (pick, res));
                }
            }
        }
        match picked {
            Some((Pick::Folder, Some(dir))) => {
                self.cfg.share_folder = Some(dir.display().to_string());
                self.dirty = true;
                if let Some(sv) = self.setup.share.as_mut() {
                    sv.msg = Some((true, format!("✔ Share images will be saved in {}", dir.display())));
                }
            }
            Some((Pick::SaveAs(bytes, ext), Some(path))) => {
                if let Some(sv) = self.setup.share.as_mut() {
                    match crate::share::save_to(&bytes, &path, ext) {
                        Ok(p) => {
                            sv.msg = Some((true, format!("✔ Saved to {}", p.display())));
                            sv.saved = Some(p);
                        }
                        Err(e) => sv.msg = Some((false, format!("Couldn't save: {e}"))),
                    }
                }
            }
            _ => {}
        }
    }

    /// The "Save location" box (folder, open, default, ask each time).
    pub(super) fn save_location_card(&mut self, ui: &mut egui::Ui) {
        let folder = self.share_folder();
        let picker_open = self.setup.share.as_ref().is_some_and(|sv| sv.picker.is_some());
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁").size(kit::fs(16.0)).color(GOLD));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Save location").size(kit::fs(12.0)).color(MUTED()));
                    ui.add(
                        egui::Label::new(RichText::new(folder.display().to_string()).size(kit::fs(13.0)).color(TEXT()))
                            .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(folder.display().to_string());
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let change = ui.add_enabled(
                    !picker_open,
                    egui::Button::new(RichText::new("Change folder…").size(kit::fs(12.5))).fill(kit::btn_fill()).rounding(kit::btn_round(6.0)),
                );
                if change.clicked() {
                    let rx = crate::share::pick_folder(folder.clone(), ui.ctx().clone());
                    if let Some(sv) = self.setup.share.as_mut() {
                        sv.picker = Some((Pick::Folder, rx));
                    }
                }
                if ui
                    .add(egui::Button::new(RichText::new("Open folder").size(kit::fs(12.5))).fill(kit::btn_fill()).rounding(kit::btn_round(6.0)))
                    .clicked()
                {
                    crate::share::open_dir(&folder);
                }
                if self.cfg.share_folder.is_some()
                    && ui
                        .add(egui::Button::new(RichText::new("Use default").size(kit::fs(12.5)).color(theme::LINK())).frame(false))
                        .on_hover_text(crate::share::default_share_dir().display().to_string())
                        .clicked()
                {
                    self.cfg.share_folder = None;
                    self.dirty = true;
                }
            });
            ui.add_space(4.0);
            if ui
                .checkbox(&mut self.cfg.share_ask, "Ask where to save each time")
                .on_hover_text("Shows a Save As box when you press Save, so you can pick the folder and file name")
                .changed()
            {
                self.dirty = true;
            }
            if picker_open {
                ui.label(RichText::new("A Windows file box is open (it may be behind this window).").size(kit::fs(12.0)).color(MUTED()));
            }
        });
    }

    pub(super) fn page_share(&mut self, ui: &mut egui::Ui, src: RunRef) {
        let back_label = match self.setup.share.as_ref().and_then(|s| s.back_to) {
            Some(_) => "Stats",
            None => "Back",
        };
        if back_button(ui, back_label) {
            let back = self.setup.share.take().and_then(|s| s.back_to);
            if back.is_some() {
                self.setup.view_run = back;
            }
            return;
        }
        let Some(info) = self.run_info(src) else {
            self.setup.share = None;
            return;
        };
        kit::page_header(ui, "SHARE IMAGE", "Make it yours, then copy it into Discord or save it as a picture.");
        if info.caught {
            kit::pill(ui, "✔ Caught: the image is labelled CAUGHT!", PillKind::Ok);
        } else {
            kit::pill(
                ui,
                "This hunt is labelled IN PROGRESS. It changes to CAUGHT! once you press Caught! on the run.",
                PillKind::Info,
            );
        }
        ui.add_space(6.0);

        // Re-draw when the options change or sprites/fonts arrive.
        let has = |slug: &Option<String>, shiny: bool| {
            slug.as_ref().is_some_and(|s| crate::sprites::cached_file(s, shiny).is_some())
        };
        let key = (self.card_opts(), has(&info.slug, true), has(&info.slug, false));
        let needs = self.setup.share.as_ref().is_some_and(|s| s.drawn.as_ref() != Some(&key))
            || self.card_text.as_ref().is_some_and(|t| !t.has_pixel() && crate::config::cache_dir().join("PressStart2P-Regular.ttf").exists());
        if needs {
            self.request_sprites(info.slug.as_deref());
            let img = self.render_card(&info);
            // Preview copy at a sensible size (the real image can be 2560 wide).
            let pw = img.w.min(1280);
            let ph = (img.h as f32 * pw as f32 / img.w as f32).round() as u32;
            let preview = match image::RgbaImage::from_raw(img.w, img.h, img.px.clone()) {
                Some(big) if pw < img.w => image::imageops::resize(&big, pw, ph, image::imageops::FilterType::Triangle).into_raw(),
                _ => img.px.clone(),
            };
            let (tw, th) = if pw < img.w { (pw, ph) } else { (img.w, img.h) };
            let tex = ui.ctx().load_texture(
                "share-preview",
                egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], &preview),
                egui::TextureOptions::LINEAR,
            );
            if let Some(s) = self.setup.share.as_mut() {
                s.tex = Some(tex);
                s.image = Some(img);
                s.drawn = Some(key.clone());
            }
        }
        if !key.1 || !key.2 {
            // still downloading a sprite: check again shortly
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }

        // Animated version: build it in the background a moment after the
        // options stop changing.
        let animated = self.cfg.card.animated;
        if animated {
            let mut finished = None;
            if let Some(sv) = self.setup.share.as_mut() {
                if let Some((job, k)) = &sv.gif_job {
                    if let Some(res) = job.poll() {
                        finished = Some((res, k.clone()));
                        sv.gif_job = None;
                    }
                }
            }
            if let Some((res, k)) = finished {
                match res {
                    Ok(out) => {
                        let frames = out
                            .frames
                            .iter()
                            .enumerate()
                            .map(|(i, (img, d))| {
                                (
                                    ui.ctx().load_texture(
                                        format!("share-gif-{i}"),
                                        egui::ColorImage::from_rgba_unmultiplied([img.w as usize, img.h as usize], &img.px),
                                        egui::TextureOptions::LINEAR,
                                    ),
                                    *d,
                                )
                            })
                            .collect();
                        if let Some(sv) = self.setup.share.as_mut() {
                            sv.gif = Some(GifReady { key: k, bytes: out.bytes, frames, saved: None });
                        }
                        self.add_event("gif_maker");
                    }
                    Err(e) if e != "cancelled" => {
                        if let Some(sv) = self.setup.share.as_mut() {
                            sv.msg = Some((false, format!("Couldn't make the GIF: {e}")));
                        }
                    }
                    Err(_) => {}
                }
            }
            let fresh = self.setup.share.as_ref().is_some_and(|sv| sv.gif.as_ref().is_some_and(|g| g.key == key));
            let running_for_key = self.setup.share.as_ref().is_some_and(|sv| sv.gif_job.as_ref().is_some_and(|(_, k)| *k == key));
            let settled = self.setup.share.as_ref().is_some_and(|sv| sv.changed_at.elapsed() > Duration::from_millis(700));
            if !fresh && !running_for_key && settled && key.1 {
                let data = self.card_data(&info);
                let job = crate::share::start_gif(data, self.card_opts(), info.slug.clone(), ui.ctx().clone());
                if let Some(sv) = self.setup.share.as_mut() {
                    sv.gif_job = Some((job, key.clone()));
                }
            } else if !fresh && !settled {
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
        }

        // Preview (animated once the GIF is ready)
        let gif_frames: Option<Vec<(egui::TextureHandle, u32)>> = if animated {
            self.setup
                .share
                .as_ref()
                .and_then(|sv| sv.gif.as_ref().filter(|g| g.key == key).map(|g| g.frames.clone()))
        } else {
            None
        };
        let still = self.setup.share.as_ref().and_then(|s| s.tex.clone());
        let shown = match &gif_frames {
            Some(fr) if !fr.is_empty() => {
                let total: u64 = fr.iter().map(|f| f.1 as u64).sum::<u64>().max(1);
                let mut t = (self.started.elapsed().as_millis() as u64) % total;
                let mut pick = fr[0].0.clone();
                for (tex, d) in fr {
                    if t < *d as u64 {
                        pick = tex.clone();
                        break;
                    }
                    t -= *d as u64;
                }
                ui.ctx().request_repaint_after(Duration::from_millis(30));
                Some(pick)
            }
            _ => still,
        };
        if let Some(tex) = shown {
            let size = tex.size_vec2();
            let w = ui.available_width().min(if self.cfg.card.layout == CardLayout::Square { 420.0 } else { 600.0 });
            let h = w * size.y / size.x;
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(w, h), Sense::hover());
                ui.painter().rect_filled(r.translate(egui::vec2(3.0, 5.0)), 12.0, Color32::from_black_alpha(70));
                egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), size))
                    .rounding(kit::r(12.0))
                    .paint_at(ui, r);
            });
        }
        if let (true, Some(g)) = (animated, self.setup.share.as_ref().and_then(|sv| sv.gif.as_ref().filter(|g| g.key == key))) {
            let mb = g.bytes.len() as f64 / 1_048_576.0;
            let (w, h, _) = self.cfg.card.output_size();
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("GIF ready · {w} × {h} · {mb:.1} MB")).size(kit::fs(11.5)).color(MUTED()));
            });
            if mb > 10.0 {
                kit::pill(
                    ui,
                    "Over Discord's 10 MB limit for free accounts. Try a lower resolution (1080p or 720p).",
                    PillKind::Warn,
                );
            }
        }
        if animated && gif_frames.is_none() {
            ui.add_space(8.0);
            match self.setup.share.as_ref().and_then(|sv| sv.gif_job.as_ref()) {
                Some((job, _)) => {
                    let d = job.done.load(std::sync::atomic::Ordering::Relaxed);
                    let frac = d as f32 / job.total.max(1) as f32;
                    let (w, h, _) = self.cfg.card.output_size();
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(format!("Making your {w} × {h} GIF…  frame {d} of {}", job.total))
                                .size(kit::fs(11.5))
                                .color(MUTED()),
                        );
                    });
                    kit::progress(ui, frac);
                }
                None if !key.1 => {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Loading the sprite…").size(kit::fs(11.5)).color(MUTED()));
                    });
                }
                None => {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Getting ready…").size(kit::fs(11.5)).color(MUTED()));
                    });
                }
            }
        }
        ui.add_space(12.0);

        self.poll_picker();
        let folder = self.share_folder();
        let asking = self.cfg.share_ask;
        let picker_open = self.setup.share.as_ref().is_some_and(|sv| sv.picker.is_some());

        // Actions
        let msg_before = self.setup.share.as_ref().and_then(|s| s.msg.clone());
        let gif_ready = gif_frames.is_some();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let copy_tip = if animated {
                "Copies the GIF file. In Discord press Ctrl+V and it uploads, animated"
            } else {
                "Then paste in Discord with Ctrl+V"
            };
            let busy = animated && !gif_ready;
            let copy = ui.add_enabled_ui(!busy, |ui| primary(ui, "  📋  Copy for Discord  ")).inner;
            if copy.on_hover_text(copy_tip).clicked() {
                let name = format!("{}-{}", info.name, info.count);
                if let Some(sv) = self.setup.share.as_mut() {
                    sv.msg = Some(if animated {
                        match sv.gif.as_mut() {
                            Some(g) => {
                                let path = match &g.saved {
                                    Some(p) if p.exists() => Ok(p.clone()),
                                    _ => crate::share::save_bytes(&g.bytes, &name, "gif", &folder),
                                };
                                match path.and_then(|p| {
                                    g.saved = Some(p.clone());
                                    crate::share::copy_file(&p).map(|_| p)
                                }) {
                                    Ok(p) => {
                                        sv.saved = Some(p);
                                        (true, "✔ Copied! In Discord press Ctrl+V to upload the GIF".into())
                                    }
                                    Err(e) => (false, format!("Couldn't copy: {e}")),
                                }
                            }
                            None => (false, "Still making the GIF, one moment".into()),
                        }
                    } else {
                        match sv.image.as_ref().map(crate::share::copy_image) {
                            Some(Ok(())) => (true, "✔ Copied! Paste it into Discord with Ctrl+V".into()),
                            Some(Err(e)) => (false, format!("Couldn't copy: {e}")),
                            None => (false, "Still drawing, try again".into()),
                        }
                    });
                }
            }
            let save_label = if animated { "  💾  Save GIF  " } else { "  💾  Save image  " };
            let save = ui.add_enabled_ui(!busy && !picker_open, |ui| secondary(ui, save_label)).inner;
            let save_tip = if asking {
                "Choose where to save it".to_string()
            } else {
                format!("Saves it in {}", folder.display())
            };
            if save.on_hover_text(save_tip).clicked() {
                let name = format!("{}-{}", info.name, info.count);
                let ctx = ui.ctx().clone();
                if let Some(s) = self.setup.share.as_mut() {
                    let (bytes, ext): (Option<Vec<u8>>, &'static str) = if animated {
                        (s.gif.as_ref().map(|g| g.bytes.clone()), "gif")
                    } else {
                        (s.image.as_ref().and_then(crate::share::png_bytes), "png")
                    };
                    match bytes {
                        None => s.msg = Some((false, "Still drawing, try again".into())),
                        Some(bytes) if asking => {
                            let file = format!("{}.{ext}", crate::share::clean_name(&name));
                            let rx = crate::share::pick_save(folder.clone(), file, ext, ctx);
                            s.picker = Some((Pick::SaveAs(bytes, ext), rx));
                            s.msg = None;
                        }
                        Some(bytes) => match crate::share::save_bytes(&bytes, &name, ext, &folder) {
                            Ok(path) => {
                                if let (true, Some(g)) = (animated, s.gif.as_mut()) {
                                    g.saved = Some(path.clone());
                                }
                                s.msg = Some((true, format!("✔ Saved to {}", path.display())));
                                s.saved = Some(path);
                            }
                            Err(e) => s.msg = Some((false, format!("Couldn't save: {e}"))),
                        },
                    }
                }
            }
            if let Some(path) = self.setup.share.as_ref().and_then(|s| s.saved.clone()) {
                if ui.add(egui::Button::new(RichText::new("Show in folder").color(theme::LINK())).frame(false)).clicked() {
                    crate::share::reveal(&path);
                }
            }
        });
        // Sharing or saving counts for the "Show Off" achievement.
        let msg_after = self.setup.share.as_ref().and_then(|s| s.msg.clone());
        if msg_after != msg_before && msg_after.as_ref().is_some_and(|(ok, m)| *ok && (m.contains("Copied") || m.contains("Saved"))) {
            self.add_event("show_off");
        }
        if let Some((ok, msg)) = self.setup.share.as_ref().and_then(|s| s.msg.clone()) {
            ui.add_space(6.0);
            kit::pill(ui, &msg, if ok { PillKind::Ok } else { PillKind::Bad });
        }
        ui.add_space(10.0);

        self.save_location_card(ui);
        ui.add_space(14.0);

        // Options
        let before = self.cfg.card.clone();
        kit::section(ui, "Style");
        {
            let has_sprite = info.slug.as_ref().is_some_and(|sl| crate::sprites::cached_file(sl, true).is_some());
            let o = self.card_opts();
            let key = format!("hunt|{:?}|{:?}|{}|{}|{}|{}|{:?}|{}|{}", o.theme, o.layout, info.name, info.count, info.caught, has_sprite, o.nickname, o.trainer, o.show_avatar);
            let ctx = ui.ctx().clone();
            // a battle pass style you no longer have (a copied settings file) falls back
            if !self.style_owned(self.cfg.card.style) {
                self.cfg.card.style = crate::card::CardOptions::default().style;
            }
            let styles = self.style_choices(true);
            self.style_thumbs(&ctx, key, &styles, |a| (ThumbOf::Hunt(a.card_data(&info)), a.card_opts()));
            self.card_style_picker(ui, &styles);
        }
        // the animated battle pass styles only come as a GIF
        let live = self.cfg.card.style.live();
        if live {
            self.cfg.card.animated = true;
        }
        kit::section(ui, "Format");
        kit::card(ui, |ui| {
            ui.add_enabled_ui(!live, |ui| {
                ui.horizontal(|ui| {
                    kit::segmented(
                        ui,
                        &mut self.cfg.card.animated,
                        &[(false, "  Picture (PNG)  "), (true, "  ✨ Animated (GIF)  ")],
                    );
                });
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(if live {
                    "This style is always animated: the whole scene moves. Loops forever. Takes a few seconds to make."
                } else if self.cfg.card.animated {
                    "Your Pokémon moves and little stars float up. Loops forever. Takes a few seconds to make."
                } else {
                    "A sharp still picture. Copies straight into Discord."
                })
                .size(kit::fs(11.0))
                .color(MUTED()),
            );
        });
        kit::section(ui, "Theme (pick a type)");
        kit::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                for t in CardTheme::ALL {
                    if theme_swatch(ui, t, self.cfg.card.theme == t).clicked() {
                        self.cfg.card.theme = t;
                    }
                }
            });
        });
        kit::section(ui, "Size");
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                kit::segmented(
                    ui,
                    &mut self.cfg.card.layout,
                    &[(CardLayout::Wide, "  Wide 16:9 (Discord)  "), (CardLayout::Square, "  Square 1:1  ")],
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if !crate::card::RESOLUTIONS.contains(&self.cfg.card.res) {
                    self.cfg.card.res = 1080;
                }
                kit::segmented(
                    ui,
                    &mut self.cfg.card.res,
                    &[(480, "  480p  "), (720, "  720p  "), (1080, "  1080p  "), (1440, "  1440p  ")],
                );
                let (w, h, _) = self.cfg.card.output_size();
                ui.label(RichText::new(format!("{w} × {h} px")).size(kit::fs(11.5)).color(MUTED()));
            });
            if self.cfg.card.animated && self.cfg.card.res >= 1440 {
                ui.label(
                    RichText::new("1440p GIFs take longer to make and can be large files.")
                        .size(kit::fs(11.0))
                        .color(MUTED()),
                );
            }
        });
        kit::section(ui, "Text");
        kit::card(ui, |ui| {
            let auto = if info.caught { format!("SHINY {}!", info.name.to_uppercase()) } else { info.name.to_uppercase() };
            egui::Grid::new("share-text").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
                ui.label(RichText::new("Title").size(kit::fs(12.5)).color(TEXT()));
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.card.title)
                        .hint_text(format!("Automatic: {auto}"))
                        .desired_width(ui.available_width())
                        .char_limit(28),
                );
                ui.end_row();
                ui.label(RichText::new("Nickname").size(kit::fs(12.5)).color(TEXT()));
                let nick_changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.cfg.card.nickname)
                            .hint_text(format!("Optional: shown as the title, with {} underneath", info.name))
                            .desired_width(ui.available_width())
                            .char_limit(12),
                    )
                    .changed();
                ui.end_row();
                // Gender: only the signs this species can have.
                let line = info.slug.as_deref().and_then(|slug| match self.dexdb.get(slug) {
                    crate::dexextra::State::Ready(e) => Some(e.gender.clone()),
                    _ => None,
                });
                let (can_m, can_f) = gender_choices(line.as_deref());
                use crate::card::Gender;
                let mut gender_changed = false;
                if !can_m && self.cfg.card.gender == Gender::Male || !can_f && self.cfg.card.gender == Gender::Female {
                    self.cfg.card.gender = Gender::None;
                    gender_changed = true;
                }
                ui.label(RichText::new("Gender").size(kit::fs(12.5)).color(TEXT()));
                if !can_m && !can_f {
                    ui.label(RichText::new("Genderless: no sign on the image").size(kit::fs(12.0)).color(MUTED()));
                } else {
                    let mut opts: Vec<(Gender, &str)> = vec![(Gender::None, "  None  ")];
                    if can_m {
                        opts.push((Gender::Male, "  ♂ Male  "));
                    }
                    if can_f {
                        opts.push((Gender::Female, "  ♀ Female  "));
                    }
                    let mut g = self.cfg.card.gender;
                    if kit::segmented(ui, &mut g, &opts) {
                        self.cfg.card.gender = g;
                        gender_changed = true;
                    }
                }
                ui.end_row();
                if nick_changed || gender_changed {
                    let entry = (self.cfg.card.nickname.trim().to_string(), self.cfg.card.gender);
                    let key = card_key(&info);
                    if entry.0.is_empty() && entry.1 == Gender::None {
                        self.cfg.card_names.remove(&key);
                    } else {
                        self.cfg.card_names.insert(key, entry);
                    }
                    self.dirty = true;
                }
                ui.label(RichText::new("Trainer name").size(kit::fs(12.5)).color(TEXT()));
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.card.trainer)
                        .hint_text(if self.cfg.trainer_name.trim().is_empty() {
                            "Optional".to_string()
                        } else {
                            format!("From your profile: {}", self.cfg.trainer_name.trim())
                        })
                        .desired_width(ui.available_width())
                        .char_limit(16),
                );
                ui.end_row();
            });
        });
        kit::section(ui, "Show on the image");
        kit::card(ui, |ui| {
            let c = &mut self.cfg.card;
            egui::Grid::new("share-show").num_columns(2).spacing([24.0, 8.0]).show(ui, |ui| {
                kit::checkbox(ui, &mut c.show_normal, "Normal sprite too");
                kit::checkbox(ui, &mut c.show_odds, "Method and odds");
                ui.end_row();
                kit::checkbox(ui, &mut c.show_meter, "Luck meter");
                kit::checkbox(ui, &mut c.show_time, "Hunt time and days");
                ui.end_row();
                kit::checkbox(ui, &mut c.show_rate, "Encounters per hour");
                kit::checkbox(ui, &mut c.show_dates, "Dates");
                ui.end_row();
                kit::checkbox(ui, &mut c.sparkles, "Sparkles");
                kit::checkbox(ui, &mut c.show_note, "Latest journal note");
                ui.end_row();
                kit::checkbox(ui, &mut c.show_avatar, "Profile picture by your name").on_hover_text("Shows next to the trainer name, when you've set a profile picture");
                ui.end_row();
            });
        });
        if ui.add(egui::Button::new(RichText::new("Reset options").size(kit::fs(11.5)).color(MUTED())).frame(false)).clicked() {
            let keep = self.cfg.card.trainer.clone();
            self.cfg.card = CardOptions { trainer: keep, ..CardOptions::default() };
        }
        if self.cfg.card != before {
            self.dirty = true;
            if let Some(s) = self.setup.share.as_mut() {
                s.msg = None;
                s.saved = None;
                s.changed_at = Instant::now();
            }
        }
    }

    // ───────────────────────────── appearance ─────────────────────────────
}

/// A clickable swatch showing a share-image theme.
/// Small pictures of the share image in each style, drawn in the
/// background whenever what's on the image changes.
#[derive(Default)]
pub(super) struct StyleThumbs {
    key: Option<String>,
    tex: Vec<Option<egui::TextureHandle>>,
    rx: Option<std::sync::mpsc::Receiver<(usize, usize, usize, Vec<u8>)>>,
}

/// What a style thumbnail shows.
pub(super) enum ThumbOf {
    Hunt(CardData),
    Profile(crate::card::ProfileData),
}

impl App {
    /// Make sure the thumbnails for `key` are drawn (or being drawn), and
    /// pick up any that have finished.
    pub(super) fn style_thumbs(&mut self, ctx: &egui::Context, key: String, styles: &[crate::card::CardStyle], what: impl FnOnce(&App) -> (ThumbOf, CardOptions)) {
        let key = format!("{key}|{styles:?}");
        let styles = styles.to_vec();
        let stale = self.style_thumbs.key.as_deref() != Some(key.as_str());
        let job = stale.then(|| what(self));
        let th = &mut self.style_thumbs;
        if let Some((data, opts)) = job {
            let (tx, rx) = std::sync::mpsc::channel();
            let ctx2 = ctx.clone();
            th.key = Some(key);
            th.rx = Some(rx);
            if th.tex.len() != styles.len() {
                th.tex = vec![None; styles.len()];
            }
            std::thread::Builder::new()
                .name("style-thumbs".into())
                .spawn(move || {
                    let fonts = crate::share::AbText::load();
                    for (i, style) in styles.into_iter().enumerate() {
                        let o = CardOptions { style, res: 480, animated: false, ..opts.clone() };
                        let c = match &data {
                            ThumbOf::Hunt(d) => crate::card::render(d, &o, &fonts),
                            ThumbOf::Profile(d) => crate::card::render_profile(d, &o, &fonts),
                        };
                        let tw = 400u32.min(c.w);
                        let th = (c.h as f32 * tw as f32 / c.w as f32).round().max(1.0) as u32;
                        let Some(big) = image::RgbaImage::from_raw(c.w, c.h, c.px) else { continue };
                        let small = image::imageops::resize(&big, tw, th, image::imageops::FilterType::Triangle);
                        if tx.send((i, tw as usize, th as usize, small.into_raw())).is_err() {
                            return;
                        }
                        ctx2.request_repaint();
                    }
                })
                .ok();
        }
        if let Some(rx) = &th.rx {
            while let Ok((i, w, h, px)) = rx.try_recv() {
                let tex = ctx.load_texture(format!("style-thumb-{i}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &px), egui::TextureOptions::LINEAR);
                if let Some(slot) = th.tex.get_mut(i) {
                    *slot = Some(tex);
                }
            }
        }
    }

    /// The share image styles you can pick: the usual ones, then the battle pass
    /// ones you've claimed (the animated ones only where a GIF can be made).
    pub(super) fn style_choices(&self, animated_ok: bool) -> Vec<crate::card::CardStyle> {
        use crate::card::CardStyle;
        let mut v = CardStyle::ALL.to_vec();
        v.extend(CardStyle::PASS.into_iter().filter(|s| (animated_ok || !s.live()) && self.style_owned(*s)));
        v
    }

    /// The style picker with the thumbnails.
    pub(super) fn card_style_picker(&mut self, ui: &mut egui::Ui, styles: &[crate::card::CardStyle]) {
        let thumbs = self.style_thumbs.tex.clone();
        card_style_picker(ui, &mut self.cfg.card.style, &thumbs, styles);
    }
}

/// Every share-image look as a tile with a picture of it.
pub(super) fn card_style_picker(ui: &mut egui::Ui, style: &mut crate::card::CardStyle, thumbs: &[Option<egui::TextureHandle>], styles: &[crate::card::CardStyle]) {
    let avail = ui.available_width();
    let gap = 10.0;
    let per = ((avail + gap) / 200.0).floor().clamp(2.0, 5.0) as usize;
    let w = ((avail - gap * (per as f32 - 1.0)) / per as f32).floor();
    // two rows a page; more looks (claimed battle pass ones) go on more pages
    let page_size = per * 2;
    let pages = styles.len().div_ceil(page_size).max(1);
    let id = ui.id().with("style-page");
    let mut page = ui.data(|d| d.get_temp::<usize>(id)).unwrap_or_else(|| styles.iter().position(|s| s == style).unwrap_or(0) / page_size);
    page = page.min(pages - 1);
    let first = page * page_size;
    for (r, row) in styles[first..(first + page_size).min(styles.len())].chunks(per).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (k, &v) in row.iter().enumerate() {
                let tex = thumbs.get(first + r * per + k).and_then(|t| t.as_ref());
                if style_tile(ui, v, *style == v, w, tex).clicked() {
                    *style = v;
                }
            }
        });
        ui.add_space(gap);
    }
    if pages > 1 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if ui.add_enabled(page > 0, egui::Button::new("‹").min_size(egui::vec2(30.0, 28.0))).clicked() {
                page -= 1;
            }
            for n in 0..pages {
                let on = n == page;
                // the page with the look in use gets a dot
                let has = styles.iter().skip(n * page_size).take(page_size).any(|s| s == style);
                let label = if has && !on { format!("{} •", n + 1) } else { format!("{}", n + 1) };
                let b = egui::Button::new(RichText::new(label).size(kit::fs(12.0)).color(if on { kit::on_primary() } else { kit::TEXT() }))
                    .fill(if on { kit::primary() } else { kit::btn_fill() })
                    .min_size(egui::vec2(34.0, 28.0));
                if ui.add(b).clicked() {
                    page = n;
                }
            }
            if ui.add_enabled(page + 1 < pages, egui::Button::new("›").min_size(egui::vec2(30.0, 28.0))).clicked() {
                page += 1;
            }
            ui.label(RichText::new(format!("{} looks", styles.len())).size(kit::fs(11.0)).color(MUTED()));
        });
        ui.add_space(gap);
    }
    ui.data_mut(|d| d.insert_temp(id, page));
}

/// A clickable tile: a picture of the share image in this style, its name
/// and what it is (the same look as the overlay theme tiles).
fn style_tile(ui: &mut egui::Ui, v: crate::card::CardStyle, selected: bool, w: f32, tex: Option<&egui::TextureHandle>) -> egui::Response {
    let pic_h = ((w - 16.0) * 9.0 / 16.0).round();
    let h = pic_h + 78.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    let p = ui.painter_at(rect.expand(12.0));
    let lift = kit::anim(ui.ctx(), resp.id.with("lift"), resp.hovered() && !selected, kit::QUICK);
    let rect = rect.translate(egui::vec2(0.0, -2.0 * lift));
    if lift > 0.0 {
        p.add(kit::shadow_tier(2).as_shape(rect, 12.0));
    }
    let fill = if selected { kit::SELECTED_BG() } else if resp.hovered() { kit::CARD_HOVER() } else { kit::CARD() };
    p.rect_filled(rect, 12.0, fill);
    p.rect_stroke(rect, 12.0, if selected { Stroke::new(2.0f32, kit::ACCENT()) } else { Stroke::new(1.0f32, kit::LINE()) });
    let pr = Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(w - 16.0, pic_h));
    p.rect_filled(pr, 8.0, Color32::from_rgb(20, 20, 28));
    match tex {
        Some(t) => {
            let s = t.size_vec2();
            let k = (pr.width() / s.x).min(pr.height() / s.y);
            let ir = Rect::from_center_size(pr.center(), s * k);
            egui::Image::new(t).rounding(if (ir.width() - pr.width()).abs() < 1.0 { 8.0 } else { 4.0 }).paint_at(ui, ir);
        }
        None => {
            p.text(pr.center(), egui::Align2::CENTER_CENTER, "drawing…", crate::fonts::sans_font(11.5), MUTED());
        }
    }
    p.text(egui::pos2(rect.min.x + 12.0, pr.max.y + 15.0), egui::Align2::LEFT_CENTER, format!("{} {}", v.icon(), v.name()), crate::fonts::title_font(14.0), kit::TEXT());
    if selected {
        let c = egui::pos2(rect.max.x - 18.0, pr.max.y + 15.0);
        p.circle_filled(c, 8.0, kit::ACCENT());
        p.add(egui::Shape::line(vec![c + egui::vec2(-3.5, 0.0), c + egui::vec2(-1.0, 2.8), c + egui::vec2(3.8, -2.8)], Stroke::new(1.8f32, Color32::WHITE)));
    }
    let galley = p.layout(v.about().to_string(), crate::fonts::sans_font(11.5), MUTED(), w - 24.0);
    p.galley(egui::pos2(rect.min.x + 12.0, pr.max.y + 26.0), galley, MUTED());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn theme_swatch(ui: &mut egui::Ui, t: CardTheme, selected: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(98.0, 66.0), Sense::click());
    let p = ui.painter_at(r.expand(2.0));
    let pal = t.palette();
    let c = |x: crate::canvas::Col| Color32::from_rgb(x.0, x.1, x.2);
    let top = Rect::from_min_size(r.min, egui::vec2(r.width(), 42.0));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(top.left_top(), c(pal.bg_a));
    mesh.colored_vertex(top.right_top(), c(pal.bg_a.mix(pal.bg_b, 0.5)));
    mesh.colored_vertex(top.right_bottom(), c(pal.bg_b));
    mesh.colored_vertex(top.left_bottom(), c(pal.bg_a.mix(pal.bg_b, 0.5)));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    // type-colour badge + a mini card
    p.rect_filled(Rect::from_min_size(top.min + egui::vec2(8.0, 9.0), egui::vec2(24.0, 24.0)), 6.0, c(pal.panel_a));
    theme::sparkle(&p, top.min + egui::vec2(20.0, 21.0), 6.0, c(pal.highlight));
    p.rect_filled(Rect::from_min_size(top.min + egui::vec2(40.0, 11.0), egui::vec2(46.0, 7.0)), 3.0, c(pal.text));
    p.rect_filled(Rect::from_min_size(top.min + egui::vec2(40.0, 24.0), egui::vec2(30.0, 7.0)), 3.0, c(pal.highlight));
    let tc = t.type_colour();
    let label_rect = Rect::from_min_max(egui::pos2(r.min.x, top.max.y), r.max);
    p.rect_filled(
        label_rect,
        egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 },
        c(tc),
    );
    let lum = (0.2126 * tc.0 as f32 + 0.7152 * tc.1 as f32 + 0.0722 * tc.2 as f32) / 255.0;
    let ink = if lum > 0.62 { Color32::from_rgb(30, 28, 44) } else { Color32::WHITE };
    p.text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        t.label().to_uppercase(),
        crate::fonts::pixel_font(7.5),
        ink,
    );
    let stroke = if selected { Stroke::new(2.5f32, TEXT()) } else { Stroke::new(1.0f32, LINE()) };
    p.rect_stroke(r, 8.0, stroke);
    resp.on_hover_text(format!("{} type", t.label())).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Light / semi-dark / dark choice drawn as a tiny window.
pub(super) fn mode_tile(ui: &mut egui::Ui, w: f32, mode: UiMode, label: &str, a: &Appearance) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 96.0), Sense::click());
    let p = ui.painter_at(r.expand(2.0));
    let (body, line, text) = match mode {
        UiMode::Light => (Color32::from_rgb(246, 244, 251), Color32::from_rgb(214, 210, 228), Color32::from_rgb(40, 38, 56)),
        UiMode::SemiDark => (Color32::from_rgb(54, 51, 76), Color32::from_rgb(92, 89, 120), Color32::from_rgb(236, 234, 245)),
        UiMode::Dark => (Color32::from_rgb(20, 19, 30), Color32::from_rgb(58, 55, 80), Color32::from_rgb(236, 234, 245)),
    };
    let win = Rect::from_min_size(r.min + egui::vec2(10.0, 10.0), egui::vec2(r.width() - 20.0, 52.0));
    p.rect_filled(win, 6.0, body);
    p.rect_filled(
        Rect::from_min_size(win.min, egui::vec2(win.width(), 12.0)),
        egui::Rounding { nw: 6.0, ne: 6.0, sw: 0.0, se: 0.0 },
        c3(a.accent),
    );
    if a.gradient {
        let hr = Rect::from_min_size(win.min, egui::vec2(win.width(), 12.0));
        let mut m = egui::Mesh::default();
        m.colored_vertex(hr.left_top() + egui::vec2(6.0, 0.0), c3(a.accent));
        m.colored_vertex(hr.right_top() - egui::vec2(6.0, 0.0), c3(a.accent2));
        m.colored_vertex(hr.right_bottom(), c3(a.accent2));
        m.colored_vertex(hr.left_bottom(), c3(a.accent));
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        p.add(egui::Shape::mesh(m));
    }
    p.rect_filled(Rect::from_min_size(win.min + egui::vec2(8.0, 20.0), egui::vec2(win.width() * 0.5, 6.0)), 3.0, text);
    p.rect_filled(Rect::from_min_size(win.min + egui::vec2(8.0, 33.0), egui::vec2(win.width() * 0.32, 6.0)), 3.0, line);
    p.rect_stroke(win, 6.0, Stroke::new(1.0f32, line));
    let selected = a.mode == mode;
    p.text(
        egui::pos2(r.center().x, r.max.y - 16.0),
        egui::Align2::CENTER_CENTER,
        label,
        kit::font(12.5),
        if selected { TEXT() } else { MUTED() },
    );
    p.rect_filled(r, kit::r(10.0), Color32::TRANSPARENT);
    p.rect_stroke(r, kit::r(10.0), if selected { Stroke::new(2.5f32, kit::ACCENT()) } else { Stroke::new(1.0f32, LINE()) });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Preset colour dots plus a custom colour picker.
pub(super) fn swatch_row(ui: &mut egui::Ui, value: &mut [u8; 3], id: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (name, c) in ACCENTS {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
            let p = ui.painter();
            let selected = *value == c;
            p.circle_filled(r.center(), 13.0, c3(c));
            if selected {
                p.circle_stroke(r.center(), 15.0, Stroke::new(2.0f32, TEXT()));
                p.line_segment([r.center() + egui::vec2(-5.0, 0.0), r.center() + egui::vec2(-1.0, 4.0)], Stroke::new(2.0f32, Color32::WHITE));
                p.line_segment([r.center() + egui::vec2(-1.0, 4.0), r.center() + egui::vec2(6.0, -4.0)], Stroke::new(2.0f32, Color32::WHITE));
            } else if resp.hovered() {
                p.circle_stroke(r.center(), 15.0, Stroke::new(1.5f32, MUTED()));
            }
            if resp.on_hover_text(name).clicked() {
                *value = c;
            }
        }
        ui.add_space(6.0);
        ui.push_id(id, |ui| {
            ui.label(RichText::new("Custom").size(kit::fs(11.5)).color(MUTED()));
            ui.color_edit_button_srgb(value);
        });
    });
}

/// Tabs of a run's stats page.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum RunTab {
    #[default]
    Overview,
    Luck,
    Activity,
    Encounters,
    Journal,
}

/// Which encounters chart the run stats page shows.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum ChartView {
    #[default]
    Day,
    SixHours,
    Hour,
}

impl ChartView {
    fn title(self) -> &'static str {
        match self {
            ChartView::Day => "Encounters per day",
            ChartView::SixHours => "Encounters per 6 hours",
            ChartView::Hour => "Encounters per hour",
        }
    }
    fn empty_text(self) -> &'static str {
        match self {
            ChartView::Day => "No encounters in these two weeks",
            ChartView::SixHours => "No encounters in this week",
            ChartView::Hour => "No encounters in these 24 hours",
        }
    }
}



