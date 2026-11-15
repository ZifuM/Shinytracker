//! The stats share window: your stats as an infographic, in one of three
//! layouts (Poster, Dashboard or Wrapped), to copy into Discord or save.
//! Opened from the Stats page (the Overview for the range shown, the
//! current hunt, a Month in Review) and from a hunt's stats under Runs.

use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::{App, RunRef};
use crate::canvas::Canvas;
use crate::statshare::{self as ss, Layout};
use crate::stats::{self, local_day, now_unix};
use crate::theme;
use eframe::egui::{self, RichText, TextureHandle, TextureOptions, Vec2};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

const MONTH_NAMES: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// What the image is about.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum ShareWhat {
    /// The Stats Overview, for the range picked there.
    Overview,
    /// One hunt.
    Hunt(RunRef),
    /// One month: (year, month 1..12).
    Month(i64, u32),
}

/// A sprite still downloading when the window opened, for this spot.
#[derive(Clone)]
enum Slot {
    Hunt,
    Shelf(usize),
}

pub(super) struct StatShare {
    what: ShareWhat,
    title: String,
    /// For the file name.
    file: String,
    data: ss::Data,
    who: ss::Who,
    /// Sprites to fill in once they're downloaded: (where, slug, shiny).
    wants: Vec<(Slot, String, bool)>,
    /// The drawing, and the numbers back with any sprites filled in.
    job: Option<((Layout, bool), Receiver<(Canvas, ss::Data)>)>,
    image: Option<((Layout, bool), Canvas)>,
    tex: Option<TextureHandle>,
    msg: Option<(bool, String)>,
    saved: Option<PathBuf>,
    picker: Option<(Vec<u8>, Receiver<Option<PathBuf>>)>,
}

/// A small copy of the image for the window.
fn preview(c: &Canvas, ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::RgbaImage::from_raw(c.w, c.h, c.px.clone())?;
    let scale = (720.0 / c.w.max(c.h) as f32).min(1.0);
    let (w, h) = (((c.w as f32 * scale).round() as u32).max(1), ((c.h as f32 * scale).round() as u32).max(1));
    let small = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], small.as_raw());
    Some(ctx.load_texture("stat-share-preview", ci, TextureOptions::LINEAR))
}

/// First and last day of a month (the last no later than today).
pub(super) fn month_days(y: i64, m: u32) -> (i64, i64) {
    let first = stats::days_from_civil(y, m, 1);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let last = stats::days_from_civil(ny, nm, 1) - 1;
    (first, last.min(local_day(now_unix())))
}

pub(super) fn month_name(y: i64, m: u32) -> String {
    format!("{} {y}", MONTH_NAMES[(m.clamp(1, 12) - 1) as usize])
}

impl App {
    /// Open the share window for `what` (and start drawing).
    pub(super) fn open_stat_share(&mut self, what: ShareWhat, ctx: &egui::Context) {
        use super::stats_overview::StatsPeriod as P;
        let today = local_day(now_unix());
        let mut wants: Vec<(Slot, String, bool)> = Vec::new();
        let (data, title, file) = match what {
            ShareWhat::Overview => {
                let period = self.setup.stats_period;
                let (days, chip, span) = match period {
                    P::Week => (Some(7), "LAST 7 DAYS", "last 7 days"),
                    P::Month => (Some(30), "LAST 30 DAYS", "last 30 days"),
                    P::Year => (Some(365), "LAST 12 MONTHS", "last 12 months"),
                    P::All => (None, "ALL TIME", "all time"),
                };
                let from = match days {
                    Some(n) => today - n + 1,
                    None => self.stats_first_day().unwrap_or(today).min(today),
                };
                let o = self.share_overview(from, today, chip, span, days.is_none());
                for (i, c) in self.share_shelf_slugs(from, today, days.is_none()).into_iter().enumerate().take(3) {
                    if let Some(s) = c {
                        wants.push((Slot::Shelf(i), s, true));
                    }
                }
                (ss::Data::Overview(o), format!("Share your stats · {span}"), format!("stats-{}", span.replace(' ', "-")))
            }
            ShareWhat::Month(y, m) => {
                let (from, to) = month_days(y, m);
                let name = month_name(y, m);
                let o = self.share_overview(from, to, &name.to_uppercase(), &name, false);
                for (i, c) in self.share_shelf_slugs(from, to, false).into_iter().enumerate().take(3) {
                    if let Some(s) = c {
                        wants.push((Slot::Shelf(i), s, true));
                    }
                }
                (ss::Data::Overview(o), format!("Share {name}"), format!("month-{y}-{m:02}"))
            }
            ShareWhat::Hunt(r) => {
                let Some(info) = self.run_info(r) else { return };
                if let Some(s) = info.slug.clone() {
                    wants.push((Slot::Hunt, s, info.caught));
                }
                let h = self.share_hunt(&info);
                let title = format!("Share your {} hunt", info.name);
                (ss::Data::Hunt(h), title, format!("{}-hunt-stats", crate::share::clean_name(&info.name)))
            }
        };
        // start any sprite downloads now; the drawing waits a moment for them
        for (_, slug, shiny) in &wants {
            if let Some((_, sp)) = self.species_of(Some(slug)) {
                let _ = self.sprites.get(&sp, *shiny);
            }
        }
        let level = self.cfg.level_seen.max(1);
        let who = ss::Who { name: self.cfg.trainer_name.trim().to_string(), level: format!("Level {level} {}", self.player_title(level)), avatar: super::avatar::card_avatar() };
        self.setup.stat_share = Some(StatShare { what, title, file, data, who, wants, job: None, image: None, tex: None, msg: None, saved: None, picker: None });
        self.start_stat_share(ctx);
    }

    /// The first day anything was hunted.
    fn stats_first_day(&self) -> Option<i64> {
        let mut all: Vec<&stats::RunStats> = vec![&self.cfg.run_stats];
        all.extend(self.cfg.saved_runs.iter().map(|r| &r.stats));
        all.extend(self.cfg.history.iter().filter_map(|h| h.stats.as_ref()));
        all.iter().flat_map(|st| st.days.iter().map(|d| d.day)).min()
    }

    /// The shelf's shinies (newest first) as sprite names, matching the
    /// order `share_overview` uses.
    fn share_shelf_slugs(&self, from: i64, to: i64, all_time: bool) -> Vec<Option<String>> {
        let mut v: Vec<(u64, Option<String>)> = self
            .cfg
            .history
            .iter()
            .filter(|h| all_time || (from..=to).contains(&local_day(h.when_unix)))
            .map(|h| (h.when_unix, h.slug.clone()))
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.into_iter().map(|x| x.1).collect()
    }

    /// Draw the image for the chosen layout in the background.
    fn start_stat_share(&mut self, ctx: &egui::Context) {
        let layout = self.cfg.stat_share_layout;
        let me = self.cfg.stat_share_me;
        let Some(v) = self.setup.stat_share.as_mut() else { return };
        if v.image.as_ref().is_some_and(|(k, _)| *k == (layout, me)) || v.job.as_ref().is_some_and(|(k, _)| *k == (layout, me)) {
            return;
        }
        let mut data = v.data.clone();
        let who = if me { v.who.clone() } else { ss::Who::default() };
        let wants = v.wants.clone();
        let (tx, rx) = channel();
        let ctx2 = ctx.clone();
        std::thread::Builder::new()
            .name("stat-share".into())
            .spawn(move || {
                // wait (a little) for sprites still downloading, then use them
                let until = Instant::now() + Duration::from_secs(5);
                while Instant::now() < until && wants.iter().any(|(_, s, shiny)| crate::sprites::cached_file(s, *shiny).is_none()) {
                    std::thread::sleep(Duration::from_millis(200));
                }
                for (slot, slug, shiny) in &wants {
                    let Some(img) = crate::share::sprite_frame(slug, *shiny) else { continue };
                    match (slot, &mut data) {
                        (Slot::Hunt, ss::Data::Hunt(h)) => h.sprite = Some(img),
                        (Slot::Shelf(i), ss::Data::Overview(o)) => {
                            if let Some(p) = o.shelf.get_mut(*i) {
                                p.sprite = Some(img);
                            }
                        }
                        _ => {}
                    }
                }
                let img = ss::render(&data, layout, &who);
                let _ = tx.send((img, data));
                ctx2.request_repaint();
            })
            .ok();
        v.job = Some(((layout, me), rx));
        v.msg = None;
    }

    /// The share window, while it's open.
    pub(super) fn stat_share_window(&mut self, ctx: &egui::Context) {
        let Some(v) = self.setup.stat_share.as_mut() else { return };
        // a finished drawing
        let done = v.job.as_ref().and_then(|(k, rx)| rx.try_recv().ok().map(|r| (*k, r)));
        if let Some((k, (img, data))) = done {
            v.tex = preview(&img, ctx);
            v.image = Some((k, img));
            v.job = None;
            // sprites are in now (or never coming): don't wait for them again
            v.data = data;
            v.wants.clear();
        } else if v.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
        // a "Save as" box that closed
        let picked = v.picker.as_ref().and_then(|(_, rx)| rx.try_recv().ok());
        if let Some(pick) = picked {
            let bytes = v.picker.take().map(|p| p.0).unwrap_or_default();
            if let Some(path) = pick {
                v.msg = Some(match crate::share::save_to(&bytes, &path, "png") {
                    Ok(p) => {
                        v.saved = Some(p.clone());
                        (true, format!("✔ Saved to {}", p.display()))
                    }
                    Err(e) => (false, format!("Couldn't save: {e}")),
                });
            }
        }

        let mut open = true;
        let mut layout = self.cfg.stat_share_layout;
        let mut me = self.cfg.stat_share_me;
        let (mut copy, mut save) = (false, false);
        let mut reveal: Option<PathBuf> = None;
        let busy = v.job.is_some();
        let shown = v.image.as_ref().is_some_and(|(k, _)| *k == (layout, me));
        let what = v.what;
        egui::Window::new(format!("🖼  {}", v.title))
            .id(egui::Id::new("stat-share-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(600.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let about = match what {
                    ShareWhat::Overview => "Your numbers and charts for the range on the Stats page, as one image.",
                    ShareWhat::Month(..) => "The month in one image: totals, activity, luck, highlights and your shinies.",
                    ShareWhat::Hunt(_) => "This hunt in one image: the count, your luck against the odds, your pace and your days.",
                };
                ui.label(RichText::new(about).size(kit::fs(12.0)).color(MUTED()));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let opts: Vec<(Layout, String)> = Layout::ALL.iter().map(|l| (*l, l.label().to_string())).collect();
                    let refs: Vec<(Layout, &str)> = opts.iter().map(|(l, s)| (*l, s.as_str())).collect();
                    kit::segmented(ui, &mut layout, &refs);
                    ui.add_space(8.0);
                    ui.label(RichText::new(layout.shape()).size(kit::fs(11.5)).color(MUTED()));
                });
                ui.add_space(6.0);
                kit::toggle_row(ui, "Show my name and picture", "Your trainer name, level and profile picture on the image.", &mut me);
                ui.add_space(8.0);
                // the preview, in a fixed area so the window doesn't jump
                let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(560.0), 520.0), egui::Sense::hover());
                match (&v.tex, shown && !busy) {
                    (Some(tex), true) => {
                        let size = tex.size_vec2();
                        let k = (area.width() / size.x).min(area.height() / size.y);
                        let r = egui::Rect::from_center_size(area.center(), size * k);
                        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));
                        ui.painter().image(tex.id(), r, uv, egui::Color32::WHITE);
                    }
                    _ => {
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
                        child.add_space(area.height() / 2.0 - 24.0);
                        child.vertical_centered(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Drawing your stats…").size(kit::fs(12.0)).color(MUTED()));
                        });
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let ready = shown && !busy;
                    let b = egui::Button::new(RichText::new("  📋  Copy for Discord  ").family(kit::strong_family(13.5)).strong().color(kit::on_primary()));
                    if ui.add_enabled_ui(ready, |ui| kit::accent_button(ui, b, 6.0)).inner.on_hover_text("Then paste it in Discord with Ctrl+V").clicked() {
                        copy = true;
                    }
                    if ui.add_enabled(ready && v.picker.is_none(), egui::Button::new("  💾  Save image  ")).clicked() {
                        save = true;
                    }
                    if let Some(p) = &v.saved {
                        if ui.add(egui::Button::new(RichText::new("Show in folder").color(theme::LINK())).frame(false)).clicked() {
                            reveal = Some(p.clone());
                        }
                    }
                });
                if let Some((ok, m)) = &v.msg {
                    ui.add_space(6.0);
                    kit::pill(ui, m, if *ok { PillKind::Ok } else { PillKind::Bad });
                }
                ui.add_space(2.0);
                ui.label(RichText::new(format!("{} · PNG", layout.shape())).size(kit::fs(11.0)).color(TEXT().gamma_multiply(0.6)));
            });

        if let Some(p) = reveal {
            crate::share::reveal(&p);
        }
        if copy {
            if let Some(v) = self.setup.stat_share.as_mut() {
                v.msg = Some(match v.image.as_ref().map(|(_, c)| crate::share::copy_image(c)) {
                    Some(Ok(())) => (true, "✔ Copied! Paste it into Discord with Ctrl+V".into()),
                    Some(Err(e)) => (false, format!("Couldn't copy: {e}")),
                    None => (false, "Still drawing, try again".into()),
                });
            }
        }
        if save {
            let folder = self.share_folder();
            let asking = self.cfg.share_ask;
            if let Some(v) = self.setup.stat_share.as_mut() {
                if let Some(((l, _), c)) = v.image.as_ref() {
                    let name = format!("{}-{}", v.file, l.label().to_lowercase());
                    match crate::share::png_bytes(c) {
                        None => v.msg = Some((false, "Couldn't make the PNG".into())),
                        Some(bytes) if asking => {
                            let rx = crate::share::pick_save(folder, format!("{name}.png"), "png", ctx.clone());
                            v.picker = Some((bytes, rx));
                        }
                        Some(bytes) => {
                            v.msg = Some(match crate::share::save_bytes(&bytes, &name, "png", &folder) {
                                Ok(p) => {
                                    v.saved = Some(p.clone());
                                    (true, format!("✔ Saved to {}", p.display()))
                                }
                                Err(e) => (false, format!("Couldn't save: {e}")),
                            });
                        }
                    }
                }
            }
        }
        if layout != self.cfg.stat_share_layout || me != self.cfg.stat_share_me {
            self.cfg.stat_share_layout = layout;
            self.cfg.stat_share_me = me;
            self.dirty = true;
            self.start_stat_share(ctx);
        }
        if !open {
            self.setup.stat_share = None;
        }
    }
}
