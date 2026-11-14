//! Dashboard widget for Shiny Replay.

use super::replay_app::{day_label, ROSE};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_build::{Head, Stat};
use super::w_fit::{self as fit, wbox, Shape, WBox};
use super::{App, Tab};
use crate::replay::{ClipKind, RecState};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense};

impl App {
    fn replay_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: ROSE, title, link: Some("Open ›") }
    }

    /// One clip: its picture (with a play button) and, when there's room,
    /// its name and day under it or beside it. Returns true when clicked.
    fn clip_card(&mut self, ui: &mut egui::Ui, b: &WBox, r: Rect, c: &crate::replay::Clip, id: impl std::hash::Hash) -> bool {
        let resp = ui.interact(r, ui.id().with(id), Sense::click());
        let side_text = r.width() > r.height() * 2.6;
        let text_h = if side_text { 0.0 } else if r.height() > b.px(70.0) { b.px(34.0) } else { 0.0 };
        let (pic, text) = if side_text {
            fit::split_left(r, (r.height() * 16.0 / 9.0).min(r.width() * 0.5), b.px(10.0))
        } else {
            fit::split_bottom(r, text_h, b.px(4.0))
        };
        // the picture, 16:9 in the middle of its room
        let w = pic.width().min(pic.height() * 16.0 / 9.0);
        let pr = Rect::from_center_size(pic.center(), vec2(w, w * 9.0 / 16.0));
        self.replay_frame(ui, pr, c, c.alert_frame, b.px(8.0));
        let p = ui.painter_at(r);
        let rad = (pr.height() * 0.16).clamp(8.0, 26.0);
        p.circle_filled(pr.center(), rad, Color32::from_black_alpha(if resp.hovered() { 170 } else { 120 }));
        p.text(pr.center() + vec2(rad * 0.12, 0.0), Align2::CENTER_CENTER, "▶", egui::FontId::proportional(rad * 0.9), Color32::WHITE);
        if text.is_positive() && text.height() > b.px(20.0) {
            let name = if c.kind == ClipKind::Shiny { format!("✨ {}", c.name()) } else { c.name() };
            let mut sub = format!("{} · {} s", day_label(c.when), c.secs.round() as u32);
            if c.kind == ClipKind::Shiny && c.encounters > 0 {
                sub = format!("{} · {} enc.", day_label(c.when), crate::stats::thousands(c.encounters));
            }
            let fl = fit::vflow(text, &[(b.px(15.0), b.px(17.0)), (b.px(13.0), b.px(15.0))], b.px(2.0));
            fit::label(&p, fl[0], &name, b.bold(12.5), TEXT(), Align2::LEFT_CENTER);
            fit::label(&p, fl[1], &sub, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Watch it").clicked()
    }

    fn replay_idle_line(&self, st: &crate::replay::RecStatus) -> &'static str {
        if !self.cfg.replay.on {
            "Recording is off. Your next shiny won't be saved."
        } else if st.state == RecState::Recording {
            "Recording. Your next shiny is saved by itself."
        } else {
            "No clips yet. It records while the overlay runs."
        }
    }

    /// The newest clip, as big as the box allows.
    pub(super) fn w_replay_last(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "replay_last");
        let clip = self.setup.replay.file().clips.first().cloned();
        let st = self.setup.replay.status();
        let mini = b.shape() == Shape::Mini;
        let (body, mut go) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "🎬", ROSE, "Last Shiny Replay", Some("Open ›")) };
        let mut watch = None;
        match clip {
            None => {
                let p = ui.painter_at(b.r);
                fit::empty(&p, &b, body, "🎬", self.replay_idle_line(&st));
                if mini && ui.interact(b.r, ui.id().with(("rl-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).clicked() {
                    go = true;
                }
            }
            Some(c) => {
                if self.clip_card(ui, &b, body, &c, ("replay-last", b.r.min.x as i32, b.r.min.y as i32)) {
                    watch = Some(c.file.clone());
                }
            }
        }
        if let Some(f) = watch {
            open_clip(&f);
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// Seconds ready, the state, and Save replay now.
    pub(super) fn w_replay_rec(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "replay_rec");
        let st = self.setup.replay.status();
        let live = st.state == RecState::Recording && self.cfg.replay.on;
        let line = if !self.cfg.replay.on {
            "Recording is off".to_string()
        } else if live {
            format!("{}×{} · {:.0} fps", st.w, st.h, st.fps)
        } else if self.overlay_on {
            "Waiting for the game window".to_string()
        } else {
            "Starts with the overlay".to_string()
        };
        let secs = format!("{:.1} s", if live { st.ready } else { 0.0 });
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "⏺", ROSE, "Replay recorder", Some("Open ›")) };
        let p = ui.painter_at(b.r);
        let save;
        let id = (b.r.min.x as i32, b.r.min.y as i32);
        let dot = |p: &egui::Painter, c: egui::Pos2, r: f32| p.circle_filled(c, r, if live { ROSE } else { MUTED() });
        if b.flat() && b.cols >= 3 {
            let (left, btn) = fit::split_right(body, (body.width() * 0.4).min(b.px(170.0)), b.px(10.0));
            let fl = fit::vflow(left, &[(b.px(20.0), b.px(40.0)), (b.px(13.0), b.px(15.0))], b.px(2.0));
            let at = fit::hero(&p, Rect::from_min_max(pos2(fl[0].min.x + b.px(16.0), fl[0].min.y), fl[0].max), &secs, fl[0].height(), if live { TEXT() } else { MUTED() }, Align2::LEFT_CENTER);
            dot(&p, pos2(fl[0].min.x + b.px(5.0), at.center().y), b.px(5.0));
            fit::label(&p, fl[1], &line, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
            let br = Rect::from_center_size(btn.center(), vec2(btn.width(), b.px(32.0).min(btn.height())));
            save = fit::button(ui, br, if live { "⏺  Save now" } else { "Not recording" }, false, ("rec-save", id)) && live;
        } else {
            let mut parts = vec![(b.px(22.0), body.height()), (b.px(13.0), b.px(15.0))];
            let with_line = body.height() > b.px(90.0);
            if with_line {
                parts.push((b.px(13.0), b.px(15.0)));
            }
            parts.push((b.px(28.0), b.px(34.0)));
            let fl = fit::vflow(body, &parts, b.px(4.0));
            let at = fit::hero(&p, Rect::from_min_max(pos2(fl[0].min.x + b.px(16.0), fl[0].min.y), fl[0].max), &secs, fl[0].height().min(b.px(64.0)), if live { TEXT() } else { MUTED() }, Align2::LEFT_CENTER);
            dot(&p, pos2(fl[0].min.x + b.px(5.0), at.center().y), b.px(5.0));
            fit::label(&p, fl[1], "ready to save", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            if with_line {
                fit::label(&p, fl[2], &line, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
            }
            save = fit::button(ui, *fl.last().unwrap_or(&body), if live { "⏺  Save replay now" } else { "Not recording" }, false, ("rec-save", id)) && live;
        }
        if save {
            self.replay_save_now();
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// The latest clips, as many as the box holds.
    pub(super) fn w_replay_recent(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "replay_recent");
        let clips: Vec<_> = self.setup.replay.file().clips.clone();
        let (body, go) = fit::head(ui, &b, "🎞", ROSE, "Recent clips", Some("Open ›"));
        let mut watch = None;
        if clips.is_empty() {
            let p = ui.painter_at(b.r);
            fit::empty(&p, &b, body, "🎞", "No clips yet. Your next shiny is saved by itself.");
        } else {
            // wide rows when the box is narrow, else picture tiles
            let cells = if body.width() < b.px(300.0) { fit::rows(body, clips.len(), b.px(44.0), b.px(6.0), 1.6) } else { fit::tiles(body, clips.len(), 1.5, b.px(120.0), b.px(80.0), b.px(8.0)) };
            for (k, r) in cells.iter().enumerate() {
                if self.clip_card(ui, &b, *r, &clips[k], ("replay-recent", k, b.r.min.x as i32, b.r.min.y as i32)) {
                    watch = Some(clips[k].file.clone());
                }
            }
        }
        if let Some(f) = watch {
            open_clip(&f);
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// Clips, shinies, space.
    pub(super) fn w_replay_stats(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "replay_stats");
        let clips: Vec<_> = self.setup.replay.file().clips.clone();
        let shinies = clips.iter().filter(|c| c.kind == ClipKind::Shiny).count();
        let bytes: u64 = clips.iter().map(|c| c.bytes).sum();
        let secs: f32 = clips.iter().map(|c| c.secs).sum();
        let stats = [Stat::new(clips.len().to_string(), "clips", ROSE), Stat::new(shinies.to_string(), "shinies", kit::C_GOLD), Stat::new(format!("{:.0}", bytes as f64 / 1e6), "MB", kit::C_SLATE)];
        let note = format!("{} min of video in all", (secs / 60.0).ceil() as u32);
        if self.f_stats(ui, &b, Self::replay_head("📼", "Clips in numbers"), &stats, Some(&note)) {
            self.go_to(Tab::Replay);
        }
    }

    /// A big Save replay now button, filling the box.
    pub(super) fn w_replay_save(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "replay_save");
        let st = self.setup.replay.status();
        let live = st.state == RecState::Recording && self.cfg.replay.on;
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "💾", ROSE, "Save replay now", Some("Open ›")) };
        let hk = self.cfg.keybinds.get(crate::hotkeys::HotAction::SaveReplay).map(|k| k.text());
        let note = match (live, hk) {
            (false, _) => "Works while the recorder is on (with the overlay).".to_string(),
            (true, Some(k)) => format!("Or press {k} in the game."),
            (true, None) => "Set a hotkey for it on the Hotkeys page.".to_string(),
        };
        let (btn, note_r) = if body.height() > b.px(70.0) { fit::split_bottom(body, b.px(28.0), b.px(6.0)) } else { (body, Rect::NOTHING) };
        let btn = Rect::from_center_size(btn.center(), vec2(btn.width(), btn.height().min(b.px(90.0))));
        let resp = ui.interact(btn, ui.id().with(("replay-save", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && live, kit::QUICK);
        let p = ui.painter_at(b.r);
        let rr = (btn.height() / 2.0).min(b.px(22.0));
        p.rect_filled(btn, rr, if live { crate::theme::blend(ROSE, Color32::WHITE, 0.08 * ht) } else { kit::CARD_HOVER() });
        fit::hero(&p, btn.shrink2(vec2(b.px(14.0), btn.height() * 0.28)), "⏺  Save the last seconds", b.px(18.0), if live { Color32::WHITE } else { MUTED() }, Align2::CENTER_CENTER);
        if note_r.is_positive() {
            fit::note(&p, note_r, &note, b.font(11.0));
        }
        if live && resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(note).clicked() {
            self.replay_save_now();
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }
}

fn open_clip(f: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(f).spawn();
    }
    #[cfg(not(windows))]
    let _ = f;
}
