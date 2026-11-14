//! Small cards over the overlay for things that need a moment of attention:
//! break reminder, session goal reached, "nothing counted yet?", and update
//! download progress / "restart to update". One is shown at a time.

use super::{App, Tab};
use crate::config::CountSource;
use crate::stats::{fmt_duration, now_unix};
use crate::theme::{self, GOLD};
use crate::update::{self, State};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Stroke};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum NoticeAction {
    RestartToUpdate,
    OpenCounting,
    /// The game runs from another folder: use that one.
    UseRunningGame,
    /// A preview from Settings: the button just closes it.
    Preview,
    /// Shiny Replay saved a clip: open it.
    WatchReplay,
}

pub(super) struct Notice {
    pub id: &'static str,
    pub icon: &'static str,
    pub title: String,
    pub body: String,
    /// 0..1 progress bar (downloads).
    pub progress: Option<f32>,
    pub action: Option<(&'static str, NoticeAction)>,
    /// Goes away by itself at this time (None = until dismissed / resolved).
    pub expires: Option<Instant>,
    pub accent: Color32,
}

/// How long after start-up the first update check runs, and how often after.
const FIRST_CHECK: Duration = Duration::from_secs(8);
const RECHECK: Duration = Duration::from_secs(6 * 3600);
/// Game open this long with nothing counted → "nothing counted yet?".
const STUCK_AFTER: Duration = Duration::from_secs(5 * 60);

impl App {
    /// Show a notice (one with the same id is replaced).
    pub(super) fn notify(&mut self, n: Notice) {
        // All pop-ups switched off (Start overlay picker): only Settings'
        // previews still come through.
        if !self.cfg.all_toasts && !matches!(n.action, Some((_, NoticeAction::Preview))) {
            return;
        }
        if let Some(x) = self.notices.iter_mut().find(|x| x.id == n.id) {
            *x = n;
        } else {
            self.notices.push(n);
        }
    }

    /// Switch the hunting style when the game log makes it clear (Rare
    /// grass / Soft resets), with a short notice saying so.
    pub(super) fn apply_method_hint(&mut self, h: crate::gamelog::MethodHint) {
        use crate::config::HuntMethod;
        use crate::gamelog::MethodHint;
        if !self.cfg.auto_method {
            return;
        }
        let (m, why) = match h {
            MethodHint::RareGrass => (HuntMethod::RareGrassSweetScent, "Sweet Scent on rare grass spotted."),
            MethodHint::Starter => (HuntMethod::SoftReset, "You picked a starter."),
            MethodHint::StaticAfterReset => (HuntMethod::SoftReset, "A reset, then a single Pokémon."),
        };
        if self.cfg.hunt_method == m {
            return;
        }
        self.cfg.hunt_method = m;
        self.dirty = true;
        let name = match m {
            HuntMethod::RareGrassSweetScent => "Rare grass",
            _ => "Soft resets",
        };
        self.notify(Notice {
            id: "auto-method",
            icon: "🔁",
            title: format!("Switched to {name}"),
            body: format!("{why} Turn this off in Settings > Hunt."),
            progress: None,
            action: None,
            expires: Some(Instant::now() + Duration::from_secs(8)),
            accent: theme::GREEN,
        });
    }

    pub(super) fn clear_notice(&mut self, id: &str) {
        self.notices.retain(|n| n.id != id);
    }

    /// Called every frame: break / goal / stuck checks and update progress.
    pub(super) fn tick_notices(&mut self, ctx: &egui::Context) {
        let now_i = Instant::now();
        self.notices.retain(|n| n.expires.map_or(true, |t| now_i < t));

        // Break reminder and session goal (once per session each).
        let now = now_unix();
        if let Some(s) = self.cfg.run_stats.current_session(now).cloned() {
            if self.cfg.break_reminder
                && self.cfg.break_minutes > 0
                && s.secs >= self.cfg.break_minutes as u64 * 60
                && self.break_nudged != s.start
            {
                self.break_nudged = s.start;
                self.notify(Notice {
                    id: "break",
                    icon: "☕",
                    title: "Time for a break?".into(),
                    body: format!("{} of hunting this session. Stretch and grab a drink, the shiny will wait.", fmt_duration(s.secs)),
                    progress: None,
                    action: None,
                    expires: Some(now_i + Duration::from_secs(25)),
                    accent: theme::GREEN,
                });
            }
            if self.cfg.session_goal > 0 && s.encounters >= self.cfg.session_goal && self.goal_nudged != s.start {
                self.goal_nudged = s.start;
                self.notify(Notice {
                    id: "goal",
                    icon: "🏁",
                    title: "Session goal reached!".into(),
                    body: format!("{} encounters this session. Nice work.", theme::thousands(s.encounters)),
                    progress: None,
                    action: None,
                    expires: Some(now_i + Duration::from_secs(20)),
                    accent: GOLD,
                });
            }
        }

        // "Nothing counted yet?": the game has been open a while this time,
        // counting isn't paused, and nothing has been counted since start-up.
        match self.game_open {
            Some(true) => {
                if self.game_open_since.is_none() {
                    self.game_open_since = Some(now_i);
                }
            }
            _ => self.game_open_since = None,
        }
        if self.counted_since_start {
            self.clear_notice("stuck");
        } else if !self.stuck_shown
            && !self.cfg.paused
            && self.game_open_since.is_some_and(|t| t.elapsed() > STUCK_AFTER)
        {
            self.stuck_shown = true;
            let body = if let Some(e) = &self.status.error {
                e.clone()
            } else if self.cfg.count_source == CountSource::GameLog {
                "The game's open but no encounters have come through. Check the game log in Settings > Counting.".to_string()
            } else {
                "The game's open but no encounters have been read. Check the name boxes in Settings > Counting.".to_string()
            };
            self.notify(Notice {
                id: "stuck",
                icon: "🔎",
                title: "Nothing counted yet".into(),
                body,
                progress: None,
                action: Some(("Open Counting", NoticeAction::OpenCounting)),
                expires: None,
                accent: theme::HP_YELLOW,
            });
        }

        // Updates: check soon after start-up, then every few hours.
        if self.cfg.auto_update {
            let due = match self.last_update_check {
                None => self.started.elapsed() > FIRST_CHECK,
                Some(t) => t.elapsed() > RECHECK,
            };
            if due {
                self.last_update_check = Some(now_i);
                update::check(self.updater.clone(), ctx.clone(), self.cfg.installed_update.clone(), self.cfg.prerelease_updates);
            }
        }
        match update::get(&self.updater) {
            State::Downloading { version, got, total } if !self.update_dismissed => {
                let mb = |b: u64| b as f64 / 1_048_576.0;
                self.notify(Notice {
                    id: "update",
                    icon: "⬇",
                    title: format!("Downloading update v{version}"),
                    body: if total > 0 {
                        format!("{:.1} of {:.1} MB", mb(got), mb(total))
                    } else {
                        format!("{:.1} MB", mb(got))
                    },
                    progress: (total > 0).then(|| (got as f32 / total as f32).clamp(0.0, 1.0)),
                    action: None,
                    expires: None,
                    accent: theme::BUTTON(),
                });
            }
            State::Ready { version, .. } => {
                if !self.update_dismissed {
                    self.notify(Notice {
                        id: "update",
                        icon: "✨",
                        title: format!("Update ready: v{version}"),
                        body: "Restart to switch to the new version. Your hunts are kept.".into(),
                        progress: Some(1.0),
                        action: Some(("Restart", NoticeAction::RestartToUpdate)),
                        expires: None,
                        accent: theme::BUTTON(),
                    });
                }
            }
            // Up to date, checking, failed, or dismissed: no update card.
            _ => self.clear_notice("update"),
        }
    }

    /// Install the downloaded update: save, swap the exe, start it, quit.
    pub(super) fn restart_to_update(&mut self, ctx: &egui::Context) {
        let State::Ready { path, version } = update::get(&self.updater) else { return };
        let before = self.cfg.installed_update.replace(version);
        self.cfg.save();
        self.dirty = false;
        match update::install(&path) {
            Ok(()) => ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close),
            Err(e) => {
                self.cfg.installed_update = before;
                self.cfg.save();
                self.notify(Notice {
                id: "update",
                icon: "⚠",
                title: "Couldn't update".into(),
                body: e,
                progress: None,
                action: None,
                expires: Some(Instant::now() + Duration::from_secs(15)),
                accent: theme::HP_RED,
                })
            }
        }
    }

    /// Draw the first notice as a card in `area` (overlay, under the header).
    pub(super) fn draw_notice(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, area: Rect, theme: crate::config::OverlayTheme) {
        let Some(n) = self.notices.first() else { return };
        let h = if n.progress.is_some() || n.action.is_some() { 84.0 } else { 64.0 };
        let r = Rect::from_min_size(area.min, egui::vec2(area.width(), h));
        let p = ui.painter().clone();
        if let Some(sk) = super::ov_toast::skin(theme) {
            self.themed_notice(ui, ctx, r, theme, &sk);
            return;
        }
        p.rect_filled(r.translate(egui::vec2(0.0, 3.0)), 10.0, Color32::from_black_alpha(110));
        p.rect_filled(r, 10.0, Color32::from_rgb(22, 19, 36));
        p.rect_stroke(r, 10.0, Stroke::new(2.0f32, n.accent));
        p.text(egui::pos2(r.min.x + 20.0, r.min.y + 22.0), Align2::CENTER_CENTER, n.icon, egui::FontId::proportional(17.0), n.accent);
        let tx = r.min.x + 38.0;
        let tw = r.max.x - tx - 24.0;
        let clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(tx, r.min.y), egui::pos2(r.max.x - 22.0, r.max.y)));
        clip.text(egui::pos2(tx, r.min.y + 15.0), Align2::LEFT_CENTER, &n.title, egui::FontId::proportional(13.0), Color32::WHITE);
        let body = clip.layout(n.body.clone(), egui::FontId::proportional(11.0), Color32::from_gray(200), tw);
        clip.galley(egui::pos2(tx, r.min.y + 26.0), body, Color32::from_gray(200));
        if let Some(f) = n.progress {
            let bar = Rect::from_min_size(egui::pos2(tx, r.max.y - 22.0), egui::vec2(tw * 0.55, 6.0));
            p.rect_filled(bar, 3.0, Color32::from_gray(60));
            p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * f, 6.0)), 3.0, n.accent);
        }
        let id = n.id;
        let action = n.action;
        // close ✖
        let xr = Rect::from_center_size(egui::pos2(r.max.x - 12.0, r.min.y + 12.0), egui::vec2(18.0, 18.0));
        if ui
            .put(xr, egui::Button::new(RichText::new("✖").size(11.0).color(Color32::from_gray(190))).frame(false))
            .on_hover_text("Dismiss")
            .clicked()
        {
            if id == "update" {
                self.update_dismissed = true;
            }
            self.clear_notice(id);
            return;
        }
        if let Some((label, act)) = action {
            let br = Rect::from_min_size(egui::pos2(r.max.x - 108.0, r.max.y - 30.0), egui::vec2(96.0, 22.0));
            let ink = if act == NoticeAction::RestartToUpdate { theme::button_ink() } else { Color32::WHITE };
            let fill = if act == NoticeAction::RestartToUpdate { theme::BUTTON() } else { theme::PLATE() };
            if ui
                .put(br, egui::Button::new(RichText::new(label).size(11.5).strong().color(ink)).fill(fill).rounding(11.0))
                .clicked()
            {
                match act {
                    NoticeAction::RestartToUpdate => self.restart_to_update(ctx),
                    NoticeAction::OpenCounting => {
                        self.show_setup = true;
                        self.setup.tab = Tab::Counting;
                        self.clear_notice(id);
                    }
                    NoticeAction::UseRunningGame => self.use_running_game_folder(),
                    NoticeAction::Preview => self.clear_notice(id),
                    NoticeAction::WatchReplay => self.replay_watch_latest(),
                }
            }
        }
    }
}

impl App {
    /// A notice in the overlay's theme (see ov_toast).
    fn themed_notice(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, r: Rect, theme: crate::config::OverlayTheme, sk: &super::ov_toast::ToastSkin) {
        use super::ov_common::Kind;
        let Some(n) = self.notices.first() else { return };
        let lk = super::ov_common::look(theme);
        let p = ui.painter().clone();
        let inner = (sk.inner)(r);
        let xr = Rect::from_center_size(egui::pos2(inner.max.x - 8.0, inner.min.y + 9.0), egui::vec2(18.0, 18.0));
        let close = ui.interact(xr, ui.id().with(("notice-x", n.id)), egui::Sense::click()).on_hover_text("Dismiss");
        let view = super::ov_toast::NoticeView {
            icon: n.icon,
            title: &n.title,
            body: &n.body,
            progress: n.progress,
            has_action: n.action.is_some(),
            accent: n.accent,
            close: xr,
            close_hot: close.hovered(),
        };
        super::ov_toast::notice(&p, r, &view, sk, &lk);
        let id = n.id;
        let action = n.action;
        if close.clicked() {
            if id == "update" {
                self.update_dismissed = true;
            }
            self.clear_notice(id);
            return;
        }
        if let Some((label, act)) = action {
            let br = Rect::from_min_size(egui::pos2(inner.max.x - 100.0, r.max.y - 30.0), egui::vec2(96.0, 22.0));
            let b = ui.interact(br, ui.id().with(("notice-act", id)), egui::Sense::click());
            let kind = if act == NoticeAction::RestartToUpdate { Kind::Main } else { Kind::Plain };
            (lk.button)(&p, br, label, kind, b.hovered(), b.is_pointer_button_down_on());
            if b.clicked() {
                match act {
                    NoticeAction::RestartToUpdate => self.restart_to_update(ctx),
                    NoticeAction::OpenCounting => {
                        self.show_setup = true;
                        self.setup.tab = Tab::Counting;
                        self.clear_notice(id);
                    }
                    NoticeAction::UseRunningGame => self.use_running_game_folder(),
                    NoticeAction::Preview => self.clear_notice(id),
                    NoticeAction::WatchReplay => self.replay_watch_latest(),
                }
            }
        }
    }
}

impl App {
    /// Settings > Overlay: show a sample notice (`which` 0..3): a plain one,
    /// one with a progress bar and one with a button.
    pub(super) fn preview_notice(&mut self, which: usize) {
        let now = Instant::now();
        let n = match which {
            0 => Notice {
                id: "preview",
                icon: "☕",
                title: "Time for a break?".into(),
                body: "2h 05m of hunting this session. Stretch and grab a drink, the shiny will wait.".into(),
                progress: None,
                action: None,
                expires: Some(now + Duration::from_secs(10)),
                accent: theme::GREEN,
            },
            1 => Notice {
                id: "preview",
                icon: "⬇",
                title: "Downloading update v9.9.9".into(),
                body: "12.4 of 20.0 MB (this is only a preview)".into(),
                progress: Some(0.62),
                action: None,
                expires: Some(now + Duration::from_secs(10)),
                accent: theme::BUTTON(),
            },
            _ => Notice {
                id: "preview",
                icon: "🔎",
                title: "Nothing counted yet".into(),
                body: "The game's open but no encounters have come through.".into(),
                progress: None,
                action: Some(("Got it", NoticeAction::Preview)),
                expires: Some(now + Duration::from_secs(12)),
                accent: theme::HP_YELLOW,
            },
        };
        self.notify(n);
    }
}
