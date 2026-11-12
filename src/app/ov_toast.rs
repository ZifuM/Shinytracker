//! Pop-up cards under the overlay in the overlay's theme: the toasts
//! (achievement, level up, goal, milestone, welcome) and the notices
//! (break reminder, update, "nothing counted yet?"…). Each theme gives a
//! card (frame and background), its own toast layout, and where a notice's
//! text sits on the card; Classic keeps the original card.

use super::ov_common::*;
use crate::config::OverlayTheme;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect};

/// What a toast is about (picks its icon).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum What {
    Achievement,
    LevelUp,
    Welcome,
    Goal,
    Milestone,
}

/// One toast, ready to draw.
pub(super) struct ToastView {
    pub what: What,
    /// Tier / metal / goal colour.
    pub rgb: [u8; 3],
    /// "Gold achievement", "Level up!", "Daily goal"…
    pub caption: String,
    /// The achievement's name, "Level 12"…
    pub title: String,
    /// The small line under it.
    pub line: String,
    pub level: u32,
    /// Seconds since it opened, and how visible what's on it is (0..1).
    pub t: f32,
    pub alpha: f32,
}

impl ToastView {
    pub fn tint(&self) -> Color32 {
        Color32::from_rgb(self.rgb[0], self.rgb[1], self.rgb[2])
    }
    /// A colour faded with the toast.
    pub fn a(&self, c: Color32) -> Color32 {
        c.gamma_multiply(self.alpha)
    }
    /// How far a typed-out text has got: characters shown `delay` seconds
    /// in, at `rate` characters a second.
    pub fn typed(&self, s: &str, delay: f32, rate: f32) -> String {
        let n = ((self.t - delay).max(0.0) * rate) as usize;
        s.chars().take(n).collect()
    }
}

/// One notice, ready to draw.
pub(super) struct NoticeView<'a> {
    pub icon: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub progress: Option<f32>,
    pub has_action: bool,
    pub accent: Color32,
    /// Where the close ✖ is, and whether the pointer is on it.
    pub close: Rect,
    pub close_hot: bool,
}

/// A theme's pop-up cards.
pub(super) struct ToastSkin {
    /// Corner rounding of the card (the window is cut to it).
    pub radius: f32,
    /// The card's frame and background in `rect` (the opening card too),
    /// with the toast's or notice's colour.
    pub card: fn(&Painter, Rect, Color32),
    /// Where a notice's icon and text go on a card.
    pub inner: fn(Rect) -> Rect,
    /// Everything on a toast, laid out on the full card.
    pub toast: fn(&Painter, Rect, &ToastView),
    /// A notice's title and body fonts.
    pub title_font: FontId,
    pub body_font: FontId,
}

pub(super) fn skin(theme: OverlayTheme) -> Option<ToastSkin> {
    Some(match theme {
        OverlayTheme::Classic => return None,
        OverlayTheme::Anime => super::ov_anime::toast_skin(),
        OverlayTheme::Glass => super::ov_glass::toast_skin(),
        OverlayTheme::Retro => super::ov_retro::toast_skin(),
        OverlayTheme::Neon => super::ov_neon::toast_skin(),
        OverlayTheme::Poketch => super::ov_poketch::toast_skin(),
        OverlayTheme::Journal => super::ov_journal::toast_skin(),
        OverlayTheme::Holo => super::ov_holo::toast_skin(),
        OverlayTheme::Broadcast => super::ov_broadcast::toast_skin(),
        OverlayTheme::Terminal => super::ov_terminal::toast_skin(),
    })
}

/// The icon grows a little as it lands.
pub(super) fn pop(t: f32) -> f32 {
    1.0 + 0.25 * (1.0 - ((t - 0.2) / 0.35).clamp(0.0, 1.0))
}

/// The toast's icon: a medal, a level badge or a big star (`size` is about
/// the radius of a medal).
pub(super) fn badge(p: &Painter, c: Pos2, size: f32, v: &ToastView) {
    let k = pop(v.t);
    match v.what {
        What::Achievement | What::Goal => theme::medal(p, c, size * k, v.rgb, false),
        What::Milestone => theme::sparkle(p, c, (size * 0.75 + size * 0.13 * (v.t * 5.0).sin().abs()) * k, v.a(GOLD)),
        What::LevelUp | What::Welcome => theme::level_badge(p, c, size * 1.2 * k, v.level),
    }
}

/// A themed notice: the card, then the icon, title, wrapped body, progress
/// and the close ✖ in the theme's look (buttons are drawn by the caller).
pub(super) fn notice(p: &Painter, r: Rect, n: &NoticeView, sk: &ToastSkin, lk: &Look) {
    let accent = (lk.tone)(n.accent);
    (sk.card)(p, r, accent);
    let inner = (sk.inner)(r);
    let tx = inner.min.x + 28.0;
    let tw = inner.max.x - tx - 18.0;
    p.text(pos2(inner.min.x + 10.0, inner.min.y + 11.0), Align2::CENTER_CENTER, n.icon, egui::FontId::proportional(16.0), accent);
    let clip = p.with_clip_rect(Rect::from_min_max(pos2(tx, inner.min.y), pos2(inner.max.x - 16.0, inner.max.y)));
    let title = clip_text(p, n.title, &sk.title_font, tw);
    clip.text(pos2(tx, inner.min.y + 10.0), Align2::LEFT_CENTER, title, sk.title_font.clone(), lk.text);
    // the body: as many lines as fit above the progress bar / button / edge
    let by = inner.min.y + 21.0;
    let bottom = if n.progress.is_some() || n.has_action { r.max.y - 30.0 } else { inner.max.y - 2.0 };
    let row = p.fonts(|f| f.row_height(&sk.body_font)).max(1.0);
    let rows = (((bottom - by) / row).floor() as usize).max(1);
    let mut job = egui::text::LayoutJob::simple(n.body.to_string(), sk.body_font.clone(), lk.muted, tw);
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    let body = p.layout_job(job);
    clip.galley(pos2(tx, by), body, lk.muted);
    if let Some(f) = n.progress {
        let bar = Rect::from_min_size(pos2(tx, r.max.y - 22.0), vec2(tw * 0.55, 6.0));
        (lk.gauge)(p, bar, f, accent);
    }
    let x = n.close.center();
    let col = if n.close_hot { lk.text } else { lk.muted };
    let s = 3.5;
    p.line_segment([x + vec2(-s, -s), x + vec2(s, s)], egui::Stroke::new(1.6f32, col));
    p.line_segment([x + vec2(-s, s), x + vec2(s, -s)], egui::Stroke::new(1.6f32, col));
}
