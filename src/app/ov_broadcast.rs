//! Overlay theme "Broadcast": an esports scoreboard. A LIVE tag and an
//! angled name plate, the shiny in a team-logo box, the count in a big
//! score box with the odds as the other side's score, a match clock, a stat
//! strip with slanted dividers, a possession-style odds bar and a scrolling
//! ticker along the bottom. A shiny wipes a banner across.

use super::ov_common::*;
use super::App;
use crate::fonts::{bold_font, impact_font, sans_bold_font};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const DARK: Color32 = Color32::from_rgb(12, 14, 24);
pub(super) const PLATE: Color32 = Color32::from_rgb(28, 32, 50);
pub(super) const LIVE: Color32 = Color32::from_rgb(232, 36, 52);
const WHITE: Color32 = Color32::WHITE;
const SOFT: Color32 = Color32::from_rgb(160, 168, 196);
const RAD: f32 = 6.0;
const LEAN: f32 = 8.0;

pub(super) fn accent() -> Color32 {
    let a = theme::pal().accent;
    mix(a, WHITE, 0.08)
}

fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        SOFT
    } else {
        mix(c, WHITE, 0.12)
    }
}

/// A parallelogram leaning right by `lean`.
pub(super) fn slant(s: &dyn Sink, r: Rect, lean: f32, fill: Color32) {
    poly(s, vec![pos2(r.min.x + lean, r.min.y), r.right_top(), pos2(r.max.x - lean, r.max.y), r.left_bottom()], fill);
}

fn slant_grad(s: &dyn Sink, r: Rect, lean: f32, a: Color32, b: Color32) {
    vgrad(s, &[pos2(r.min.x + lean, r.min.y), r.right_top(), pos2(r.max.x - lean, r.max.y), r.left_bottom()], a, b);
}

/// Italic-looking heavy text (Impact), with a hard shadow.
fn sport(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    p.text(pos + vec2(1.5, 1.5), anchor, s, impact_font(size), fade(Color32::BLACK, 0.55));
    p.text(pos, anchor, s, impact_font(size), col)
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    sport(p, pos, anchor, s, font.size * 1.1, col)
}

fn panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.55 + 0.45 * opacity;
    vgrad(s, &rrect_pts(r, RAD), fade(Color32::from_rgb(30, 34, 56), a), fade(DARK, a));
    // faint diagonal stripes
    let mut x = r.min.x - r.height();
    while x < r.max.x {
        let (mut a0, mut b0) = (pos2(x, r.max.y - 1.0), pos2(x + r.height(), r.min.y + 1.0));
        if a0.x < r.min.x + 1.0 {
            a0 = pos2(r.min.x + 1.0, r.max.y - 1.0 - (r.min.x + 1.0 - a0.x));
        }
        if b0.x > r.max.x - 1.0 {
            b0 = pos2(r.max.x - 1.0, r.min.y + 1.0 + (b0.x - r.max.x + 1.0));
        }
        if b0.x > a0.x {
            line(s, a0, b0, Stroke::new(6.0f32, fade(WHITE, 0.018 * a)));
        }
        x += 22.0;
    }
    rrect(s, Rect::from_min_size(r.min, vec2(r.width(), 3.0)), RAD, if alert { GOLD } else { accent() });
    rstroke(s, r, RAD, Stroke::new(1.0f32, fade(if alert { GOLD } else { WHITE }, if alert { 0.9 } else { 0.08 })));
}

fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    slant(s, r, r.height() * 0.6, PLATE);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fr = Rect::from_min_size(r.min, vec2((r.width() * f).max(r.height()), r.height()));
        slant_grad(s, fr, r.height() * 0.6, mix(col, WHITE, 0.25), col);
    }
}

/// Angled buttons; Caught! is a big accent plate.
fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let r = if down { r.translate(vec2(0.0, 1.0)) } else { r };
    let (fill, text) = match kind {
        Kind::Main => (accent(), WHITE),
        Kind::Good => (theme::GREEN, DARK),
        Kind::Gold => (GOLD, DARK),
        Kind::Plain => (PLATE, WHITE),
    };
    let fill = if hovered { mix(fill, WHITE, 0.18) } else { fill };
    slant(p, r.translate(vec2(2.0, 2.0)), LEAN, fade(Color32::BLACK, 0.5));
    slant(p, r, LEAN, fill);
    if kind == Kind::Plain {
        line(p, pos2(r.min.x + LEAN, r.min.y + 0.5), pos2(r.max.x, r.min.y + 0.5), Stroke::new(1.0f32, fade(WHITE, 0.25)));
    }
    let c = r.center();
    if label == "⏸" {
        for dx in [-3.0f32, 3.0] {
            rrect(p, Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.0, 12.0)), 0.0, text);
        }
    } else if label == "▶" {
        poly(p, vec![c + vec2(-3.5, -6.0), c + vec2(6.0, 0.0), c + vec2(-3.5, 6.0)], text);
    } else {
        let t = label.replace('−', "-").to_uppercase();
        let room = r.width() - LEAN - 4.0;
        let f = fit(p, &t, impact_font, if kind == Kind::Main { 17.0 } else { 14.0 }, 9.0, room);
        let s = clip_text(p, &t, &f, room);
        if kind == Kind::Main {
            sport(p, c, Align2::CENTER_CENTER, &s, f.size, text);
        } else {
            p.text(c, Align2::CENTER_CENTER, s, f, text);
        }
    }
}

pub(super) fn look() -> Look {
    Look {
        text: WHITE,
        muted: SOFT,
        accent: accent(),
        radius: RAD,
        num: impact_font,
        label: |s| impact_font(s * 1.1),
        body: sans_bold_font,
        panel,
        big,
        button,
        gauge,
        divider: |s, a, b| line(s, a + vec2(3.0, 0.0), b - vec2(3.0, 0.0), Stroke::new(2.0f32, fade(accent(), 0.6))),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn broadcast_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let (x0, x1) = (left + 10.0, left + w - 10.0);
        let bg = p.add(Shape::Noop);
        let acc = accent();

        // ── Top plates: LIVE, the show name and level, the controls
        let mut y = top + 10.0;
        let ph = 24.0;
        let mut x = x0;
        let (live_txt, live_col) = if v.ghost {
            ("CLICK-THROUGH", GOLD)
        } else if self.cfg.paused {
            ("PAUSED", Color32::from_rgb(110, 116, 140))
        } else {
            ("LIVE", LIVE)
        };
        let lw = text_w(&p, live_txt, impact_font(14.0)) + 30.0;
        let lr = Rect::from_min_size(pos2(x, y), vec2(lw, ph));
        slant(&p, lr, LEAN, live_col);
        if m.status_dot || live_txt == "LIVE" {
            circle(&p, pos2(lr.min.x + 13.0, lr.center().y), 3.5, if v.blink || live_txt != "LIVE" { WHITE } else { fade(WHITE, 0.35) });
        }
        p.text(pos2(lr.min.x + 21.0, lr.center().y), Align2::LEFT_CENTER, live_txt, impact_font(14.0), if live_col == GOLD { DARK } else { WHITE });
        tip(ui, lr, "bc-live", &v.status.1);
        x = lr.max.x - 2.0;
        let ctrl_left = x1 + 2.0 - 4.0 * 21.0 - 4.0;
        let show = if w > 330.0 { "SHINY HUNT" } else { "HUNT" };
        let sw = text_w(&p, show, impact_font(14.0)) + 26.0;
        if x + sw <= ctrl_left {
            let sr = Rect::from_min_size(pos2(x, y), vec2(sw, ph));
            slant(&p, sr, LEAN, WHITE);
            p.text(sr.center(), Align2::CENTER_CENTER, show, impact_font(14.0), DARK);
            x = sr.max.x - 2.0;
        }
        if let Some(lv) = v.level {
            let t = format!("LV {lv}");
            let tw = text_w(&p, &t, impact_font(13.0)) + 22.0;
            if x + tw <= ctrl_left {
                let r = Rect::from_min_size(pos2(x, y), vec2(tw, ph));
                slant(&p, r, LEAN, acc);
                p.text(r.center(), Align2::CENTER_CENTER, t, impact_font(13.0), WHITE);
            }
        }
        let mut cx = x1 + 2.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, y + ph / 2.0), vec2(20.0, 20.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                rrect(&p, r, 3.0, if c == Ctrl::Close { LIVE } else { PLATE });
            }
            ctrl_icon(&p, r, c, on, if on { GOLD } else if hov { WHITE } else { SOFT }, 12.0);
            cx -= 21.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 4.0, y + ph)), "bc-drag");
        y += ph + 10.0;

        // ── Scoreboard: logo box, name + score, odds box
        let art = v.sprites();
        if art || m.target || m.counter {
            let sh = if v.compact { 62.0 } else { 76.0 };
            let mut sx = x0;
            if art {
                let lb = Rect::from_min_size(pos2(sx, y), vec2(sh, sh));
                vgrad(&p, &rrect_pts(lb, 4.0), mix(acc, DARK, 0.35), mix(acc, DARK, 0.75));
                let clip = p.with_clip_rect(lb);
                let mut k = lb.min.x - sh;
                while k < lb.max.x {
                    line(&clip, pos2(k, lb.max.y), pos2(k + sh, lb.min.y), Stroke::new(5.0f32, fade(WHITE, 0.05)));
                    k += 14.0;
                }
                rstroke(&p, lb, 4.0, Stroke::new(1.5f32, fade(acc, 0.8)));
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = lb.center().x + (k as f32 - (n - 1.0) / 2.0) * sh * 0.28;
                            self.draw_sprite_centered(&clip, sp, true, pos2(fx, lb.center().y), vec2(sh * 0.5, sh * 0.8));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&clip, &sp, true, lb.center(), vec2(sh - 6.0, sh - 6.0));
                    }
                }
                sx = lb.max.x + 6.0;
            }
            let odds_w = if m.odds_bar && !v.compact { 78.0 } else { 0.0 };
            let score_r = Rect::from_min_max(pos2(sx, y), pos2(x1 - if odds_w > 0.0 { odds_w + 6.0 } else { 0.0 }, y + sh));
            // name plate on top of the score box
            let name_h = if m.target { 22.0 } else { 0.0 };
            if m.target {
                let nr = Rect::from_min_size(score_r.min, vec2(score_r.width(), name_h));
                slant_grad(&p, nr, 0.0, mix(acc, WHITE, 0.15), acc);
                let name = v.title.clone().unwrap_or_else(|| "PICK A TARGET".into()).to_uppercase();
                let f = impact_font(15.0);
                let col = if v.flash && v.blink { GOLD } else { WHITE };
                let tr = p.text(pos2(nr.min.x + 8.0, nr.center().y), Align2::LEFT_CENTER, clip_text(&p, &name, &f, nr.width() - 16.0), f, col);
                if v.multi {
                    tip(ui, tr, "bc-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                }
            }
            let br = Rect::from_min_max(pos2(score_r.min.x, score_r.min.y + name_h), score_r.max);
            vgrad(&p, &rrect_pts(br, 2.0), PLATE, DARK);
            line(&p, br.left_bottom(), br.right_bottom(), Stroke::new(2.0f32, acc));
            if m.counter {
                let num = theme::thousands(v.count);
                let size = (br.height() * 0.78).min(48.0);
                let f = fit(&p, &num, impact_font, size, 18.0, br.width() - 70.0);
                sport(&p, pos2(br.min.x + 8.0, br.center().y + 1.0), Align2::LEFT_CENTER, &num, f.size, WHITE);
                let lab = if v.counter_label == "ENCOUNTERS" { "ENC" } else { v.counter_label };
                p.text(pos2(br.max.x - 6.0, br.min.y + 9.0), Align2::RIGHT_CENTER, lab, impact_font(10.0), if v.counter_label == "ENCOUNTERS" { SOFT } else { tone(v.counter_col) });
            }
            if let Some((text, dry, t)) = &v.watch {
                let wr = p.text(pos2(br.max.x - 6.0, br.max.y - 9.0), Align2::RIGHT_CENTER, text, impact_font(10.0), if *dry { GOLD } else { SOFT });
                tip(ui, wr, "bc-watch", t);
            }
            if odds_w > 0.0 {
                let orr = Rect::from_min_max(pos2(x1 - odds_w, y), pos2(x1, y + sh));
                vgrad(&p, &rrect_pts(orr, 2.0), PLATE, DARK);
                let col = tone(v.odds.color);
                rrect(&p, Rect::from_min_size(orr.min, vec2(orr.width(), 18.0)), 2.0, fade(col, 0.9));
                p.text(pos2(orr.center().x, orr.min.y + 9.0), Align2::CENTER_CENTER, "ODDS", impact_font(12.0), DARK);
                let mt = v.mult_text();
                let f = fit(&p, &mt, impact_font, 26.0, 14.0, orr.width() - 8.0);
                sport(&p, pos2(orr.center().x, orr.min.y + 18.0 + (sh - 18.0) * 0.45), Align2::CENTER_CENTER, &mt, f.size, WHITE);
                p.text(pos2(orr.center().x, orr.max.y - 8.0), Align2::CENTER_CENTER, v.odds_word().replace(" ODDS", ""), impact_font(9.5), col);
                tip(ui, orr, "bc-odds-box", &v.odds.tip);
            }
            // a shiny: banner wipe across the scoreboard
            if let Some((_, text)) = &v.alert {
                let band = Rect::from_min_max(pos2(x0 - 4.0, y + 4.0), pos2(x1 + 4.0, y + sh - 4.0));
                let wipe = ((v.t * 1.8) % 1.0).min(1.0);
                slant_grad(&p, band, 18.0, mix(GOLD, WHITE, 0.2), mix(GOLD, LIVE, 0.25));
                let clip = p.with_clip_rect(band);
                let gx = band.min.x - 40.0 + wipe * (band.width() + 80.0);
                poly(&clip, vec![pos2(gx, band.max.y), pos2(gx + 18.0, band.max.y), pos2(gx + 48.0, band.min.y), pos2(gx + 30.0, band.min.y)], fade(WHITE, 0.6));
                sport(&p, pos2(band.min.x + 22.0, band.center().y - 6.0), Align2::LEFT_CENTER, "SHINY!", 34.0, if v.blink { WHITE } else { DARK });
                let f = sans_bold_font(11.0);
                p.text(pos2(band.min.x + 24.0, band.max.y - 10.0), Align2::LEFT_CENTER, clip_text(&p, text, &f, band.width() - 140.0), f, DARK);
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(band.max.x - 98.0, band.center().y - 13.0), vec2(90.0, 26.0)), &b, &button);
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let band = Rect::from_min_max(pos2(x0, y + 8.0), pos2(x1, y + sh - 8.0));
                confetti(&p, band.expand(10.0), *el, &[GOLD, acc, WHITE], a);
                slant(&p, band, 16.0, fade(acc, 0.95 * a));
                sport(&p, band.center() - vec2(0.0, 7.0), Align2::CENTER_CENTER, "MILESTONE", 20.0, fade(WHITE, a));
                p.text(band.center() + vec2(0.0, 13.0), Align2::CENTER_CENTER, text.to_uppercase(), impact_font(12.0), fade(GOLD, a));
            }
            y += sh + 8.0;
        }

        // ── Match clock and the possession-style odds bar
        if m.odds_bar && v.alert.is_none() {
            let col = tone(v.odds.color);
            let top_y = y;
            let clock_w = if v.timer.is_some() { 82.0 } else { 0.0 };
            if let Some((t, idle, tp)) = &v.timer {
                let cr = Rect::from_min_size(pos2(x0, y), vec2(clock_w - 6.0, 22.0));
                rrect(&p, cr, 2.0, if *idle { PLATE } else { WHITE });
                p.text(cr.center(), Align2::CENTER_CENTER, t, impact_font(14.0), if *idle { SOFT } else { DARK });
                tip(ui, cr, "bc-clock", tp);
            }
            let bx = x0 + clock_w;
            let lab = if v.compact { v.mult_text() } else { format!("{} · {}", v.odds_word(), v.odds.right.to_uppercase()) };
            p.text(pos2(bx, y + 5.0), Align2::LEFT_CENTER, lab, impact_font(10.5), col);
            gauge(&p, Rect::from_min_max(pos2(bx, y + 12.0), pos2(x1, y + 22.0)), v.odds.frac, col);
            y += 30.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "bc-odds", &v.odds.tip);
        }

        // ── Stat strip with slanted dividers
        let mut cells: Vec<(&str, String, Color32, String)> = Vec::new();
        if m.session {
            cells.push(("SESSION", theme::thousands(v.session.0), WHITE, "Encounters this session".into()));
            cells.push(("TIME", v.session_time(), WHITE, "Hunting time this session".into()));
            cells.push(("PACE", v.session_rate(), WHITE, "Encounters per hour this session".into()));
        }
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => cells.push(("LUCK", format!("{pct:.0}%"), tone(*col), t.clone())),
                None => cells.push(("LUCK", "—".into(), SOFT, "Needs the odds first (start a battle).".into())),
            }
        }
        if !cells.is_empty() && v.alert.is_none() {
            let per_row = if v.compact { 2 } else { 4 };
            for row in cells.chunks(per_row) {
                let sr = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 36.0));
                rrect(&p, sr, 2.0, fade(PLATE, 0.85));
                let cw = sr.width() / row.len() as f32;
                for (i, (lab, val, col, t)) in row.iter().enumerate() {
                    let cx0 = sr.min.x + i as f32 * cw;
                    if i > 0 {
                        line(&p, pos2(cx0 + 4.0, sr.min.y + 5.0), pos2(cx0 - 4.0, sr.max.y - 5.0), Stroke::new(2.0f32, fade(acc, 0.7)));
                    }
                    p.text(pos2(cx0 + cw / 2.0, sr.min.y + 10.0), Align2::CENTER_CENTER, *lab, impact_font(9.5), SOFT);
                    let f = impact_font(15.0);
                    p.text(pos2(cx0 + cw / 2.0, sr.min.y + 25.0), Align2::CENTER_CENTER, clip_text(&p, val, &f, cw - 10.0), f, *col);
                    tip(ui, Rect::from_min_size(pos2(cx0, sr.min.y), vec2(cw, 36.0)), ("bc-s", *lab), t);
                }
                y += 40.0;
            }
        }
        if m.sparkline && v.alert.is_none() {
            p.text(pos2(x0, y + 5.0), Align2::LEFT_CENTER, "MOMENTUM · 20 MIN", impact_font(10.0), SOFT);
            p.text(pos2(x1, y + 5.0), Align2::RIGHT_CENTER, format!("{}/H", theme::thousands(v.pace_hr)), impact_font(11.0), WHITE);
            let g = Rect::from_min_max(pos2(x0, y + 12.0), pos2(x1, y + 32.0));
            sparkline(&p, g, &v.pace, acc, fade(SOFT, 0.5));
            tip(ui, g, "bc-pace", "Encounters each minute for the last 20 minutes.");
            y += 38.0;
        }
        let bar = |ui: &mut egui::Ui, y: &mut f32, lab: String, right: String, frac: f32, col: Color32, t: &str| {
            p.text(pos2(x0, *y + 5.0), Align2::LEFT_CENTER, &lab, impact_font(10.5), WHITE);
            p.text(pos2(x1, *y + 5.0), Align2::RIGHT_CENTER, &right, impact_font(10.0), SOFT);
            gauge(&p, Rect::from_min_max(pos2(x0, *y + 12.0), pos2(x1, *y + 19.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 20.0)), ("bc-bar", lab.clone()), t);
            *y += 26.0;
        };
        if m.xp && v.alert.is_none() {
            bar(ui, &mut y, format!("TRAINER LV {}", v.st.level.max(1)), format!("{} / {} XP", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, Color32::from_rgb(90, 170, 255), "Trainer level progress");
        }
        if m.daily_goal && v.alert.is_none() {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "OFF".into() };
            bar(ui, &mut y, if done { "DAILY GOAL ✔".into() } else { "DAILY GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { GOLD } else { theme::GREEN }, "Encounters today across every run");
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() && v.alert.is_none() {
            let h = 30.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += p.text(pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q.to_uppercase(), impact_font(13.0), WHITE).width() + 10.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h + 10.0, h)), b, &button);
                    x += h + 12.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x + 2.0, y), pos2(x1, y + h)), &acts.btns[3], &button);
            } else {
                let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                let gap = 4.0;
                let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
                for b in &acts.btns {
                    let bw = room * b.weight / total;
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &button);
                    x += bw + gap;
                }
            }
            y += h + 8.0;
        }

        // ── The ticker
        let mut items: Vec<String> = Vec::new();
        if m.method_line {
            items.push(v.method.to_uppercase());
        }
        if m.milestone {
            let (what, when, _) = &v.milestone;
            items.push(format!("NEXT: {what} {when}").to_uppercase());
        }
        if m.last_seen {
            items.push(match &v.last.1 {
                Some(a) => format!("LAST: {} · {a}", v.last.0).to_uppercase(),
                None => format!("LAST: {}", v.last.0).to_uppercase(),
            });
        }
        for (text, _, _, _) in &v.chips {
            items.push(text.to_uppercase());
        }
        if !items.is_empty() {
            let tr = Rect::from_min_max(pos2(left + 1.0, y), pos2(left + w - 1.0, y + 22.0));
            rrect(&p, tr, 0.0, fade(WHITE, 0.94));
            let tag = Rect::from_min_size(tr.min, vec2(52.0, tr.height()));
            slant(&p, Rect::from_min_max(tag.min, pos2(tag.max.x + 8.0, tag.max.y)), 0.0, acc);
            p.text(tag.center(), Align2::CENTER_CENTER, "INFO", impact_font(12.0), WHITE);
            let text = items.join("   ◆   ") + "   ◆   ";
            let f = bold_font(10.5);
            let tw = text_w(&p, &text, f.clone()).max(1.0);
            let lane = Rect::from_min_max(pos2(tag.max.x + 10.0, tr.min.y), tr.max);
            let clip = p.with_clip_rect(lane);
            let off = (v.t * 34.0) % tw;
            let mut tx = lane.min.x - off;
            while tx < lane.max.x {
                clip.text(pos2(tx, tr.center().y), Align2::LEFT_CENTER, &text, f.clone(), DARK);
                tx += tw;
            }
            tip(ui, tr, "bc-ticker", &items.join("\n"));
            y = tr.max.y;
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else {
            y += 4.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, panel);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, acc);
    }
}

/// For the theme picker.
pub(super) fn preview(p: &Painter, card: Rect) {
    let r = Rect::from_center_size(card.center(), vec2(card.width() - 12.0, 58.0));
    panel(p, r, 1.0, false);
    slant(p, Rect::from_min_size(r.min + vec2(5.0, 7.0), vec2(22.0, 10.0)), 3.0, LIVE);
    slant(p, Rect::from_min_size(r.min + vec2(26.0, 7.0), vec2(36.0, 10.0)), 3.0, WHITE);
    let lb = Rect::from_min_size(r.min + vec2(5.0, 21.0), vec2(22.0, 22.0));
    rrect(p, lb, 2.0, mix(accent(), DARK, 0.4));
    let sr = Rect::from_min_size(r.min + vec2(30.0, 21.0), vec2(r.width() - 58.0, 22.0));
    rrect(p, sr, 1.0, PLATE);
    p.text(sr.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, "42", impact_font(17.0), WHITE);
    rrect(p, Rect::from_min_size(pos2(sr.max.x + 3.0, sr.min.y), vec2(22.0, 22.0)), 1.0, PLATE);
    rrect(p, Rect::from_min_max(pos2(r.min.x + 1.0, r.max.y - 10.0), pos2(r.max.x - 1.0, r.max.y - 2.0)), 0.0, WHITE);
}

// ───────────────────────────── pop-up cards ─────────────────────────────

/// A lower-third plate: dark, faint stripes, a stripe of the toast's colour.
fn toast_card(p: &Painter, r: Rect, tint: Color32) {
    let rad = RAD.min(r.height() / 2.0);
    vgrad(p, &rrect_pts(r, rad), Color32::from_rgb(30, 34, 56), DARK);
    if r.height() > 6.0 {
        p.add(Shape::rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 3.0)), egui::Rounding { nw: rad, ne: rad, sw: 0.0, se: 0.0 }, tone(tint)));
    }
    rstroke(p, r, rad, Stroke::new(1.0f32, fade(WHITE, 0.08)));
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let col = tone(v.tint());
    // the logo box
    let logo = Rect::from_min_size(r.min + vec2(8.0, 10.0), vec2(48.0, r.height() - 18.0));
    slant(p, logo.translate(vec2(2.0, 2.0)), 6.0, fade(Color32::BLACK, 0.5));
    slant(p, logo, 6.0, PLATE);
    super::ov_toast::badge(p, logo.center(), 11.0, v);
    let tx = logo.max.x + 8.0;
    let room = r.max.x - tx - 10.0;
    // the caption on a plate in the toast's colour
    let cap = v.caption.to_uppercase().replace('!', "");
    let cf = impact_font(11.0);
    let cap = clip_text(p, &cap, &cf, room - 18.0);
    let cw = text_w(p, &cap, cf.clone()) + 18.0;
    let plate = Rect::from_min_size(pos2(tx, r.min.y + 9.0), vec2(cw, 15.0));
    slant(p, plate, 5.0, col);
    let ink = if col.r() as u32 + col.g() as u32 + col.b() as u32 > 480 { DARK } else { WHITE };
    p.text(plate.center(), Align2::CENTER_CENTER, cap, cf, v.a(ink));
    let title = v.title.to_uppercase();
    let f = fit(p, &title, impact_font, 18.0, 12.0, room);
    let title = clip_text(p, &title, &f, room);
    sport(p, pos2(tx, r.min.y + 35.0), Align2::LEFT_CENTER, &title, f.size, v.a(WHITE));
    let lf = bold_font(9.5);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, r.min.y + 52.0), Align2::LEFT_CENTER, line_s, lf, v.a(SOFT));
    // a wipe in the toast's colour as it opens
    let w = ((v.t - 0.1) / 0.45).clamp(0.0, 1.0);
    if w > 0.0 && w < 1.0 {
        let x = r.min.x - 30.0 + w * (r.width() + 60.0);
        slant(p, Rect::from_min_max(pos2(x - 40.0, r.min.y), pos2(x, r.max.y)), 18.0, col);
        slant(p, Rect::from_min_max(pos2(x - 52.0, r.min.y), pos2(x - 44.0, r.max.y)), 18.0, fade(WHITE, 0.6));
    }
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: RAD,
        card: toast_card,
        inner: |r| Rect::from_min_max(r.min + vec2(6.0, 6.0), r.max - vec2(6.0, 4.0)),
        toast: toast_body,
        title_font: impact_font(14.0),
        body_font: bold_font(10.0),
    }
}
