//! Dashboard widget for Shiny Replay.

use super::replay_app::{day_label, ROSE};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{head, note};
use super::{App, Tab};
use crate::replay::{ClipKind, RecState};
use eframe::egui::{self, vec2, Align2, Color32, RichText, Sense};

impl App {
    /// The newest clip, and whether the recorder is on.
    pub(super) fn w_replay_last(&mut self, ui: &mut egui::Ui) {
        let mut go = false;
        let mut watch = None;
        let clip = self.setup.replay.file().clips.first().cloned();
        let st = self.setup.replay.status();
        kit::card(ui, |ui| {
            go = head(ui, "🎬", ROSE, "Last Shiny Replay", Some("Open ›"));
            ui.add_space(6.0);
            let Some(c) = clip else {
                let line = if !self.cfg.replay.on {
                    "Recording is off. Your next shiny won't be saved."
                } else if st.state == RecState::Recording {
                    "Recording. Your next shiny is saved by itself."
                } else {
                    "No clips yet. It records while the overlay runs."
                };
                note(ui, line);
                return;
            };
            let w = ui.available_width();
            let room = kit::take_card_fill().map(|h| (h - 92.0).max(60.0)).unwrap_or(w * 9.0 / 16.0);
            let h = (w * 9.0 / 16.0).min(room);
            let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            self.replay_frame(ui, r, &c, c.alert_frame, 8.0);
            let p = ui.painter();
            p.circle_filled(r.center(), 18.0, Color32::from_black_alpha(if resp.hovered() { 170 } else { 120 }));
            p.text(r.center() + vec2(2.0, 0.0), Align2::CENTER_CENTER, "▶", egui::FontId::proportional(16.0), Color32::WHITE);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Watch it").clicked() {
                watch = Some(c.file.clone());
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let name = if c.kind == ClipKind::Shiny { format!("✨ {}", c.name()) } else { c.name() };
                ui.label(RichText::new(name).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                let mut sub = day_label(c.when);
                if c.kind == ClipKind::Shiny && c.encounters > 0 {
                    sub = format!("{sub} · {} enc.", crate::stats::thousands(c.encounters));
                }
                ui.add(egui::Label::new(RichText::new(sub).size(11.5).color(MUTED())).truncate());
            });
        });
        if let Some(f) = watch {
            open_clip(&f);
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    fn w_replay_head(&mut self, ui: &mut egui::Ui, icon: &str, title: &str) -> bool {
        head(ui, icon, ROSE, title, Some("Open ›"))
    }

    /// Seconds ready, the state, and Save replay now.
    pub(super) fn w_replay_rec(&mut self, ui: &mut egui::Ui) {
        let st = self.setup.replay.status();
        let mut go = false;
        let mut save = false;
        kit::card(ui, |ui| {
            go = self.w_replay_head(ui, "⏺", "Replay recorder");
            ui.add_space(6.0);
            let live = st.state == RecState::Recording && self.cfg.replay.on;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.1} s", if live { st.ready } else { 0.0 })).font(crate::fonts::sans_bold_font(24.0)).color(if live { TEXT() } else { MUTED() }));
                ui.label(RichText::new("ready to save").size(12.0).color(MUTED()));
            });
            let line = if !self.cfg.replay.on {
                "Recording is off".to_string()
            } else if live {
                format!("{}×{} · {:.0} fps", st.w, st.h, st.fps)
            } else if self.overlay_on {
                "Waiting for the game window".to_string()
            } else {
                "Starts with the overlay".to_string()
            };
            note(ui, line);
            ui.add_space(4.0);
            let b = egui::Button::new(RichText::new("⏺  Save replay now").size(12.5).color(TEXT())).rounding(14.0);
            if ui.add_enabled(live, b).clicked() {
                save = true;
            }
        });
        if save {
            self.replay_save_now();
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// The latest few clips.
    pub(super) fn w_replay_recent(&mut self, ui: &mut egui::Ui) {
        let clips: Vec<_> = self.setup.replay.file().clips.clone();
        let mut go = false;
        let mut watch = None;
        kit::card(ui, |ui| {
            go = self.w_replay_head(ui, "🎞", "Recent clips");
            ui.add_space(6.0);
            if clips.is_empty() {
                note(ui, "No clips yet. Your next shiny is saved by itself.");
                return;
            }
            let room = kit::take_card_fill().map(|h| ((h - 60.0) / 46.0).floor() as usize).unwrap_or(4).clamp(1, 10);
            for c in clips.iter().take(room) {
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(vec2(64.0, 36.0), Sense::click());
                    self.replay_frame(ui, r, c, c.alert_frame, 5.0);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Watch it").clicked() {
                        watch = Some(c.file.clone());
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let name = if c.kind == ClipKind::Shiny { format!("✨ {}", c.name()) } else { c.name() };
                        ui.label(RichText::new(name).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                        ui.label(RichText::new(format!("{} · {} s", day_label(c.when), c.secs.round() as u32)).size(11.0).color(MUTED()));
                    });
                });
                ui.add_space(4.0);
            }
            if clips.len() > room {
                note(ui, format!("+{} more", clips.len() - room));
            }
        });
        if let Some(f) = watch {
            open_clip(&f);
        }
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// Clips, shinies, space.
    pub(super) fn w_replay_stats(&mut self, ui: &mut egui::Ui) {
        let clips: Vec<_> = self.setup.replay.file().clips.clone();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_replay_head(ui, "📼", "Clips in numbers");
            ui.add_space(6.0);
            let shinies = clips.iter().filter(|c| c.kind == ClipKind::Shiny).count();
            let bytes: u64 = clips.iter().map(|c| c.bytes).sum();
            let secs: f32 = clips.iter().map(|c| c.secs).sum();
            ui.horizontal(|ui| {
                for (n, what) in [(clips.len().to_string(), "clips"), (shinies.to_string(), "shinies"), (format!("{:.0}", bytes as f64 / 1e6), "MB")] {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(n).font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
                        ui.label(RichText::new(what).size(11.5).color(MUTED()));
                    });
                    ui.add_space(14.0);
                }
            });
            ui.add_space(4.0);
            note(ui, format!("{} min of video in all", (secs / 60.0).ceil() as u32));
        });
        if go {
            self.go_to(Tab::Replay);
        }
    }

    /// A big Save replay now button.
    pub(super) fn w_replay_save(&mut self, ui: &mut egui::Ui) {
        let st = self.setup.replay.status();
        let live = st.state == RecState::Recording && self.cfg.replay.on;
        let mut save = false;
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_replay_head(ui, "💾", "Save replay now");
            ui.add_space(6.0);
            let w = ui.available_width();
            let b = egui::Button::new(RichText::new("⏺  Save the last seconds").size(14.0).color(Color32::WHITE)).fill(if live { ROSE } else { kit::CARD_HOVER() }).rounding(18.0).min_size(vec2(w, 44.0));
            if ui.add_enabled(live, b).clicked() {
                save = true;
            }
            let hk = self.cfg.keybinds.get(crate::hotkeys::HotAction::SaveReplay).map(|k| k.text());
            note(ui, match (live, hk) {
                (false, _) => "Works while the recorder is on (with the overlay).".to_string(),
                (true, Some(k)) => format!("Or press {k} in the game."),
                (true, None) => "Set a hotkey for it on the Hotkeys page.".to_string(),
            });
        });
        if save {
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
