//! Overlay theme "Retro DS": a handheld with two screens. The top screen is
//! a Gen 4 battle: the shiny on its platform, an enemy box with the name
//! and an HP-style bar for the odds, and a player box with the count and an
//! EXP bar. A hinge in between holds the power light and the controls. The
//! bottom screen is the touch screen: a message box with the stats and the
//! battle menu as buttons (Caught! where FIGHT would be).

use super::ov_common::*;
use super::App;
use crate::fonts::pixel_font;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

const SHELL_A: Color32 = Color32::from_rgb(62, 66, 78);
const SHELL_B: Color32 = Color32::from_rgb(34, 36, 44);
const BEZEL: Color32 = Color32::from_rgb(14, 14, 18);
const BOX: Color32 = Color32::from_rgb(248, 248, 232);
const BOX_EDGE: Color32 = Color32::from_rgb(72, 72, 80);
const TXT: Color32 = Color32::from_rgb(64, 64, 72);
const TXT_SH: Color32 = Color32::from_rgb(208, 208, 196);
const HP_GREEN: Color32 = Color32::from_rgb(88, 208, 128);
const HP_YEL: Color32 = Color32::from_rgb(248, 216, 56);
const HP_RED: Color32 = Color32::from_rgb(240, 88, 64);
const EXP: Color32 = Color32::from_rgb(72, 160, 248);
const RAD: f32 = 16.0;

/// Gen 4 text: dark grey with a light shadow.
fn ds(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    let sh = (size / 7.0).clamp(1.0, 2.0);
    p.text(pos + vec2(sh, sh), anchor, s, pixel_font(size), if col == Color32::WHITE { Color32::from_rgb(40, 40, 48) } else { TXT_SH });
    p.text(pos, anchor, s, pixel_font(size), col)
}

/// Status colours that read on the white boxes.
fn tone(c: Color32) -> Color32 {
    if c == theme::GREEN {
        Color32::from_rgb(40, 152, 72)
    } else if c == theme::HP_YELLOW || c == GOLD {
        Color32::from_rgb(184, 128, 8)
    } else if c == theme::HP_RED {
        Color32::from_rgb(208, 56, 48)
    } else if c == theme::MUTED() || c == Color32::WHITE {
        TXT
    } else {
        mix(c, Color32::BLACK, 0.25)
    }
}

/// The white message box with its double border.
fn text_box(s: &dyn Sink, r: Rect) {
    rrect(s, r.expand(3.0), 8.0, Color32::from_rgb(88, 96, 120));
    rrect(s, r.expand(1.0), 6.5, Color32::from_rgb(184, 192, 216));
    rrect(s, r, 6.0, Color32::from_rgb(250, 250, 250));
}

/// Gen 4 HP bar: dark frame, coloured fill with a bright top line.
fn hp(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    rrect(s, r.expand(2.0), 3.0, Color32::from_rgb(64, 64, 72));
    rrect(s, r, 2.0, Color32::from_rgb(96, 96, 104));
    let w = r.width() * frac.clamp(0.0, 1.0);
    if w > 0.5 {
        let f = Rect::from_min_size(r.min, vec2(w, r.height()));
        rrect(s, f, 2.0, col);
        rrect(s, Rect::from_min_size(f.min, vec2(w, (r.height() * 0.35).max(1.5))), 1.0, mix(col, Color32::WHITE, 0.45));
    }
}

fn hp_col(frac: f32) -> Color32 {
    if frac > 0.5 {
        HP_GREEN
    } else if frac > 0.2 {
        HP_YEL
    } else {
        HP_RED
    }
}

/// The battle-menu buttons: rounded, two-tone, white inner edge.
fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let base = match kind {
        Kind::Main => Color32::from_rgb(232, 88, 72),
        Kind::Good => Color32::from_rgb(96, 184, 88),
        Kind::Gold => Color32::from_rgb(232, 184, 64),
        Kind::Plain => match label {
            "−1" => Color32::from_rgb(232, 184, 64),
            "+1" => Color32::from_rgb(96, 184, 88),
            "⏸" | "▶" => Color32::from_rgb(88, 136, 216),
            _ => Color32::from_rgb(128, 140, 168),
        },
    };
    let base = if hovered { mix(base, Color32::WHITE, 0.15) } else { base };
    let r = if down { r.translate(vec2(0.0, 2.0)) } else { r };
    let rad = (r.height() * 0.28).min(12.0);
    if !down {
        rrect(p, r.translate(vec2(0.0, 3.0)), rad, mix(base, Color32::BLACK, 0.55));
    }
    rrect(p, r, rad, mix(base, Color32::BLACK, 0.45));
    let inner = r.shrink(2.0);
    vgrad(p, &rrect_pts(inner, rad - 2.0), mix(base, Color32::WHITE, 0.25), base);
    rstroke(p, inner.shrink(1.5), rad - 3.0, Stroke::new(1.5f32, fade(Color32::WHITE, 0.55)));
    let size = if kind == Kind::Main { (r.height() * 0.22).clamp(8.0, 11.0) } else if label.chars().count() <= 2 { 9.5 } else { 7.5 };
    if label == "⏸" {
        for dx in [-3.5f32, 3.5] {
            rrect(p, Rect::from_center_size(r.center() + vec2(dx + 1.0, 1.0), vec2(4.0, 12.0)), 0.0, Color32::from_rgb(40, 40, 48));
            rrect(p, Rect::from_center_size(r.center() + vec2(dx, 0.0), vec2(4.0, 12.0)), 0.0, Color32::WHITE);
        }
    } else if label == "▶" {
        let c = r.center();
        poly(p, vec![c + vec2(-3.0, -6.0), c + vec2(8.0, 1.0), c + vec2(-3.0, 8.0)], Color32::from_rgb(40, 40, 48));
        poly(p, vec![c + vec2(-4.0, -7.0), c + vec2(7.0, 0.0), c + vec2(-4.0, 7.0)], Color32::WHITE);
    } else {
        ds(p, r.center(), Align2::CENTER_CENTER, &label.to_uppercase(), size, Color32::WHITE);
    }
}

fn panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    rrect(s, r, 10.0, if alert { GOLD } else { Color32::from_rgb(88, 96, 120) });
    rrect(s, r.shrink(2.0), 8.0, Color32::from_rgb(184, 192, 216));
    rrect(s, r.shrink(3.0), 7.0, fade(Color32::from_rgb(250, 250, 250), 0.6 + 0.4 * opacity));
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    ds(p, pos, anchor, s, (font.size * 0.62).clamp(7.0, 16.0), if col == Color32::WHITE { TXT } else { col })
}

fn hp_gauge(s: &dyn Sink, r: Rect, frac: f32, _col: Color32) {
    let left = 1.0 - frac.clamp(0.0, 1.0);
    hp(s, r, left, hp_col(left));
}

pub(super) fn look() -> Look {
    Look {
        text: TXT,
        muted: Color32::from_rgb(128, 128, 140),
        accent: HP_RED,
        radius: 10.0,
        num: pixel_font,
        label: |s| pixel_font(s * 0.62),
        body: |s| pixel_font((s * 0.6).max(6.5)),
        panel,
        big,
        button,
        gauge: hp_gauge,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, Color32::from_rgb(200, 200, 212))),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn retro_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 14.0;
        let pad = 12.0;
        let (sx0, sx1) = (left + pad, left + w - pad);
        let bg = p.add(Shape::Noop);
        let mut y = top + pad;

        // ── Top screen: the battle
        let screen_h = if v.compact { 124.0 } else { 152.0 };
        let scr = Rect::from_min_max(pos2(sx0, y), pos2(sx1, y + screen_h));
        rrect(&p, scr.expand(4.0), 6.0, BEZEL);
        let sky = Rect::from_min_max(scr.min, pos2(scr.max.x, scr.min.y + screen_h * 0.52));
        vgrad(&p, &[sky.left_top(), sky.right_top(), sky.right_bottom(), sky.left_bottom()], Color32::from_rgb(168, 212, 240), Color32::from_rgb(236, 244, 236));
        let ground = Rect::from_min_max(pos2(scr.min.x, sky.max.y), scr.max);
        vgrad(&p, &[ground.left_top(), ground.right_top(), ground.right_bottom(), ground.left_bottom()], Color32::from_rgb(184, 216, 144), Color32::from_rgb(120, 176, 104));
        line(&p, pos2(scr.min.x, sky.max.y), pos2(scr.max.x, sky.max.y), Stroke::new(2.0f32, Color32::from_rgb(208, 232, 184)));
        // stripes in the grass
        for k in 0..4 {
            let yy = ground.min.y + 8.0 + k as f32 * (ground.height() / 4.0);
            line(&p, pos2(scr.min.x, yy), pos2(scr.max.x, yy), Stroke::new(1.0f32, fade(Color32::WHITE, 0.12)));
        }
        let sw = scr.width();
        // the scene stays inside the screen
        let p = p.with_clip_rect(scr);
        // enemy platform and the shiny
        let ec = pos2(scr.min.x + sw * 0.7, scr.min.y + screen_h * 0.62);
        let plat = |c: Pos2, rx: f32, ry: f32| {
            p.add(Shape::ellipse_filled(c, vec2(rx, ry), Color32::from_rgb(112, 160, 88)));
            p.add(Shape::ellipse_filled(c - vec2(0.0, ry * 0.25), vec2(rx * 0.92, ry * 0.7), Color32::from_rgb(152, 200, 120)));
        };
        plat(ec, sw * 0.22, screen_h * 0.075);
        if m.sprites {
            if v.multi {
                let n = v.all.len().min(3) as f32;
                for (k, sp) in v.all.iter().take(3).enumerate() {
                    let fx = ec.x + (k as f32 - (n - 1.0) / 2.0) * sw * 0.12;
                    self.draw_sprite(&p, sp, true, pos2(fx, ec.y + 2.0), vec2(sw * 0.16, screen_h * 0.45));
                }
            } else if let Some(sp) = v.target.clone() {
                self.draw_sprite(&p, &sp, true, pos2(ec.x, ec.y + 2.0), vec2(sw * 0.38, screen_h * 0.56));
                if !m.single_sprite && !v.compact {
                    let pc = pos2(scr.min.x + sw * 0.2, scr.max.y - 6.0);
                    plat(pc, sw * 0.24, screen_h * 0.07);
                    // Your side, seen from behind like in battle: the animated
                    // Gen 5 back sprite (cut off by the bottom of the screen),
                    // or the front one where there's no back sprite.
                    if !self.draw_back_sprite(&p, &sp, false, pos2(pc.x, scr.max.y + screen_h * 0.1), vec2(sw * 0.44, screen_h * 0.66)) && self.sprites.back_failed(&sp, false) {
                        self.draw_sprite(&p, &sp, false, pos2(pc.x, pc.y + 2.0), vec2(sw * 0.3, screen_h * 0.48));
                    }
                }
            }
            if v.alert.is_some() {
                for k in 0..6 {
                    let a = k as f32 / 6.0 * std::f32::consts::TAU + v.t * 1.5;
                    let c = ec + vec2(a.cos() * sw * 0.16, a.sin() * screen_h * 0.22 - screen_h * 0.18);
                    theme::sparkle(&p, c, 5.0 + 2.0 * (v.t * 4.0 + k as f32).sin().abs(), Color32::WHITE);
                }
            }
        }
        // enemy box: name and the odds as an HP bar
        let eb_w = sw * 0.58;
        let eb_h = if m.odds_bar { 38.0 } else { 22.0 };
        let eb = Rect::from_min_size(pos2(scr.min.x + 6.0, scr.min.y + 8.0), vec2(eb_w, eb_h));
        poly(&p, vec![eb.left_top() + vec2(-2.0, -2.0), eb.right_top() + vec2(10.0, -2.0), eb.right_bottom() + vec2(2.0, 2.0), eb.left_bottom() + vec2(-2.0, 2.0)], BOX_EDGE);
        poly(&p, vec![eb.left_top(), eb.right_top() + vec2(8.0, 0.0), eb.right_bottom(), eb.left_bottom()], BOX);
        let name = if v.flash && v.blink {
            "SPOTTED!".to_string()
        } else {
            v.title.clone().map(|t| t.to_uppercase()).unwrap_or_else(|| if m.target { "PICK A TARGET".into() } else { "SHINY HUNT".into() })
        };
        let nf = pixel_font(7.0);
        let name = clip_text(&p, &name, &nf, eb_w - 22.0);
        ds(&p, pos2(eb.min.x + 6.0, eb.min.y + 10.0), Align2::LEFT_CENTER, &name, 7.0, if v.flash { tone(GOLD) } else { TXT });
        theme::sparkle(&p, pos2(eb.max.x - 4.0, eb.min.y + 10.0), 5.0, GOLD);
        if v.multi {
            tip(ui, eb, "ds-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
        } else if let Some((_, _, t)) = &v.watch {
            tip(ui, eb, "ds-watch", t);
        }
        if m.odds_bar {
            let left_frac = 1.0 - v.odds.frac;
            let tg = Rect::from_min_size(pos2(eb.min.x + 6.0, eb.min.y + 21.0), vec2(26.0, 9.0));
            rrect(&p, tg, 2.0, Color32::from_rgb(64, 64, 72));
            p.text(tg.center(), Align2::CENTER_CENTER, "ODDS", pixel_font(5.0), Color32::from_rgb(248, 200, 72));
            hp(&p, Rect::from_min_max(pos2(tg.max.x + 3.0, tg.min.y + 2.0), pos2(eb.max.x - 4.0, tg.max.y - 2.0)), left_frac, hp_col(left_frac));
            ds(&p, pos2(eb.max.x - 2.0, eb.max.y + 8.0), Align2::RIGHT_CENTER, &format!("{} · {}", v.mult_text(), v.odds.right.to_uppercase()), 5.5, TXT);
            tip(ui, eb.expand(4.0), "ds-odds", &v.odds.tip);
        }
        // player box: the count and an EXP bar
        if m.counter {
            let pb_w = sw * 0.6;
            let pb_h = if m.xp || m.daily_goal { 44.0 } else { 36.0 };
            let pb = Rect::from_min_size(pos2(scr.max.x - pb_w - 6.0, scr.max.y - pb_h - 6.0), vec2(pb_w, pb_h));
            poly(&p, vec![pb.left_top() + vec2(-10.0, -2.0), pb.right_top() + vec2(2.0, -2.0), pb.right_bottom() + vec2(2.0, 2.0), pb.left_bottom() + vec2(-2.0, 2.0)], BOX_EDGE);
            poly(&p, vec![pb.left_top() + vec2(-8.0, 0.0), pb.right_top(), pb.right_bottom(), pb.left_bottom()], BOX);
            let label = if v.counter_label == "ENCOUNTERS" { "ENCOUNTERS" } else { v.counter_label };
            ds(&p, pos2(pb.min.x + 4.0, pb.min.y + 9.0), Align2::LEFT_CENTER, label, 5.5, tone(v.counter_col));
            if let Some((t, idle, tp)) = &v.timer {
                let r = ds(&p, pos2(pb.max.x - 5.0, pb.min.y + 9.0), Align2::RIGHT_CENTER, &t.to_uppercase(), 5.5, if *idle { Color32::from_rgb(150, 150, 160) } else { TXT });
                tip(ui, r.expand(3.0), "ds-timer", tp);
            }
            let num = theme::thousands(v.count);
            let size = if text_w(&p, &num, pixel_font(14.0)) < pb_w - 12.0 { 14.0 } else { 10.0 };
            ds(&p, pos2(pb.max.x - 6.0, pb.min.y + 23.0), Align2::RIGHT_CENTER, &num, size, TXT);
            if m.xp || m.daily_goal {
                let (frac, tipx) = if m.xp {
                    (v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, format!("Trainer LV {}: {} / {} XP", v.st.level.max(1), theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)))
                } else {
                    (if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, format!("Daily goal: {} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)))
                };
                let eg = Rect::from_min_max(pos2(pb.min.x + 26.0, pb.max.y - 7.0), pos2(pb.max.x - 6.0, pb.max.y - 4.0));
                p.text(pos2(pb.min.x + 4.0, eg.center().y), Align2::LEFT_CENTER, if m.xp { "EXP" } else { "GOAL" }, pixel_font(4.5), EXP);
                hp(&p, eg, frac, EXP);
                tip(ui, pb, "ds-exp", &tipx);
            }
        }
        // a message along the bottom of the screen for alerts
        if v.alert.is_some() || v.celebrate.is_some() {
            let mb = Rect::from_min_max(pos2(scr.min.x + 6.0, scr.max.y - 30.0), pos2(scr.max.x - 6.0, scr.max.y - 6.0));
            text_box(&p, mb);
            let (msg, col) = match (&v.alert, &v.celebrate) {
                (Some(_), _) => (format!("A shiny {} appeared!", v.title.clone().unwrap_or_else(|| "Pokémon".into())), if v.blink { tone(GOLD) } else { TXT }),
                (_, Some((_, t))) => (format!("{t}! Keep going!"), TXT),
                _ => (String::new(), TXT),
            };
            let f = pixel_font(6.5);
            ds(&p, pos2(mb.min.x + 8.0, mb.center().y), Align2::LEFT_CENTER, &clip_text(&p, &msg, &f, mb.width() - 16.0), 6.5, col);
            if let Some((el, _)) = &v.celebrate {
                confetti(&p, scr, *el, &[HP_RED, HP_YEL, EXP, HP_GREEN, Color32::WHITE], celebrate_fade(*el));
            }
        }
        y = scr.max.y + 4.0;
        let p = ui.painter().clone();

        // ── Hinge: power light, title, controls
        let hinge = Rect::from_min_max(pos2(left + 4.0, y + 2.0), pos2(left + w - 4.0, y + 22.0));
        rrect(&p, hinge, 5.0, Color32::from_rgb(28, 30, 36));
        line(&p, hinge.left_top() + vec2(6.0, 0.0), hinge.right_top() - vec2(6.0, 0.0), Stroke::new(1.0f32, fade(Color32::WHITE, 0.08)));
        let mid = hinge.center().y;
        let mut x = hinge.min.x + 10.0;
        if m.status_dot {
            let col = v.status.0;
            let led = Rect::from_center_size(pos2(x + 3.0, mid), vec2(6.0, 4.0));
            rrect(&p, led.expand(3.0), 4.0, fade(col, 0.25));
            rrect(&p, led, 2.0, col);
            tip(ui, led.expand(5.0), "ds-led", &v.status.1);
            x += 14.0;
        }
        let title = if v.ghost { "CLICK-THROUGH" } else { "SHINY HUNT" };
        let tr = p.text(pos2(x, mid + 0.5), Align2::LEFT_CENTER, title, pixel_font(6.0), if v.ghost { GOLD } else { Color32::from_rgb(170, 172, 186) });
        if let (Some(lv), false) = (v.level, v.ghost || v.compact) {
            p.text(pos2(tr.max.x + 8.0, mid + 0.5), Align2::LEFT_CENTER, format!("LV{lv}"), pixel_font(6.0), Color32::from_rgb(248, 200, 72));
        }
        let mut cx = hinge.max.x - 12.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx, mid), vec2(18.0, 16.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                rrect(&p, r, 3.0, if c == Ctrl::Close { Color32::from_rgb(196, 43, 28) } else { fade(Color32::WHITE, 0.12) });
            }
            ctrl_icon(&p, r, c, false, if on { Color32::from_rgb(248, 200, 72) } else { Color32::from_rgb(190, 192, 206) }, 11.0);
            cx -= 20.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 6.0, hinge.max.y)), "ds-drag");
        y = hinge.max.y + 4.0;

        // ── Bottom screen: message box with the stats, then the battle menu
        let mut lines: Vec<(String, Color32, String)> = Vec::new();
        if m.method_line {
            lines.push((v.method.to_uppercase(), TXT, String::new()));
        }
        if m.luck {
            if let Some((pct, col, t)) = &v.luck {
                lines.push((format!("LUCK  {pct:.0}% WOULD HAVE IT"), tone(*col), t.clone()));
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            lines.push((format!("NEXT  {} {}", what.to_uppercase(), when.to_uppercase()), TXT, t));
        }
        if m.session {
            lines.push((format!("SESSION  {} · {} · {}", theme::thousands(v.session.0), v.session_time().to_uppercase(), v.session_rate().to_uppercase()), TXT, "Encounters, time and pace this session".into()));
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("LAST  {} · {}", v.last.0.to_uppercase(), a.to_uppercase()),
                None => format!("LAST  {}", v.last.0.to_uppercase()),
            };
            lines.push((txt, TXT, String::new()));
        }
        for (text, col, _f, t) in &v.chips {
            lines.push((text.clone(), tone(*col), t.clone()));
        }
        if m.daily_goal && m.xp {
            lines.push((format!("GOAL  {} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)), TXT, "Encounters today towards your daily goal".into()));
        }
        let acts = self.ov_actions(m.buttons);
        let any_bottom = !lines.is_empty() || m.sparkline || !acts.btns.is_empty();
        if any_bottom {
            let bs_top = y + 4.0;
            let bs_slot = p.add(Shape::Noop);
            let mut by = bs_top + 8.0;
            let (bx0, bx1) = (sx0 + 8.0, sx1 - 8.0);
            if !lines.is_empty() || m.sparkline || acts.prompt.is_some() {
                let lh = 14.0;
                let n = lines.len() + acts.prompt.is_some() as usize;
                let box_h = n as f32 * lh + if m.sparkline { 30.0 } else { 0.0 } + 10.0;
                let mb = Rect::from_min_max(pos2(bx0, by), pos2(bx1, by + box_h));
                text_box(&p, mb);
                let mut ly = mb.min.y + 12.0;
                let f = pixel_font(6.0);
                if let Some(q) = &acts.prompt {
                    ds(&p, pos2(mb.min.x + 8.0, ly), Align2::LEFT_CENTER, &q.to_uppercase(), 6.5, tone(GOLD));
                    ly += lh;
                }
                for (i, (text, col, t)) in lines.iter().enumerate() {
                    let s = clip_text(&p, text, &f, mb.width() - 16.0);
                    let r = ds(&p, pos2(mb.min.x + 8.0, ly), Align2::LEFT_CENTER, &s, 6.0, *col);
                    tip(ui, Rect::from_min_max(pos2(mb.min.x, r.min.y - 3.0), pos2(mb.max.x, r.max.y + 3.0)), ("ds-line", i), t);
                    ly += lh;
                }
                if m.sparkline {
                    let g = Rect::from_min_max(pos2(mb.min.x + 8.0, ly - 2.0), pos2(mb.max.x - 60.0, ly + 20.0));
                    sparkline(&p, g, &v.pace, HP_RED, Color32::from_rgb(120, 136, 168));
                    ds(&p, pos2(mb.max.x - 8.0, g.center().y), Align2::RIGHT_CENTER, &format!("{}/H", theme::thousands(v.pace_hr)), 6.0, TXT);
                    tip(ui, g, "ds-pace", "Encounters each minute for the last 20 minutes.");
                }
                by = mb.max.y + 12.0;
            }
            if !acts.btns.is_empty() {
                if acts.normal {
                    // FIGHT → Caught!, then BAG / RUN / POKéMON → −1 / pause / +1
                    let fr = Rect::from_min_max(pos2(bx0, by), pos2(bx1, by + 44.0));
                    self.ov_button(ui, &p, fr, &acts.btns[3], &button);
                    by = fr.max.y + 8.0;
                    let third = (bx1 - bx0 - 16.0) / 3.0;
                    let row = [(0usize, bx0, third, 0.0), (2, bx0 + third + 8.0, third, 6.0), (1, bx1 - third, third, 0.0)];
                    for (i, x, bw, dy) in row {
                        self.ov_button(ui, &p, Rect::from_min_size(pos2(x, by + dy), vec2(bw, 30.0 - dy)), &acts.btns[i], &button);
                    }
                    by += 38.0;
                } else {
                    let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                    let gap = 8.0;
                    let room = bx1 - bx0 - gap * (acts.btns.len() as f32 - 1.0);
                    let mut x = bx0;
                    for b in &acts.btns {
                        let bw = room * b.weight / total;
                        self.ov_button(ui, &p, Rect::from_min_size(pos2(x, by), vec2(bw, 34.0)), b, &button);
                        x += bw + gap;
                    }
                    by += 42.0;
                }
            }
            let bs = Rect::from_min_max(pos2(sx0, bs_top), pos2(sx1, by));
            let rec = Rec::default();
            rrect(&rec, bs.expand(4.0), 6.0, BEZEL);
            rrect(&rec, bs, 2.0, Color32::from_rgb(48, 92, 104));
            let pc = p.with_clip_rect(bs);
            let mut k = -bs.height();
            let stripes = Rec::default();
            while k < bs.width() {
                line(&stripes, pos2(bs.min.x + k, bs.max.y), pos2(bs.min.x + k + bs.height(), bs.min.y), Stroke::new(3.0f32, Color32::from_rgb(58, 104, 116)));
                k += 10.0;
            }
            let _ = pc;
            let mut all = rec.0.into_inner();
            all.push(Shape::Vec(stripes.0.into_inner()).clip(bs));
            p.set(bs_slot, Shape::Vec(all));
            y = bs.max.y + 6.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y + pad - 4.0));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        let op = self.cfg.opacity;
        let rec = Rec::default();
        vgrad(&rec, &rrect_pts(full, RAD), fade(SHELL_A, 0.75 + 0.25 * op), fade(SHELL_B, 0.75 + 0.25 * op));
        rstroke(&rec, full.shrink(0.5), RAD, Stroke::new(if alert { 2.5f32 } else { 1.0f32 }, if alert { GOLD } else { Color32::from_rgb(12, 12, 16) }));
        line(&rec, pos2(full.min.x + RAD, full.min.y + 1.5), pos2(full.max.x - RAD, full.min.y + 1.5), Stroke::new(1.0f32, fade(Color32::WHITE, 0.14)));
        for sx in [full.min.x + 8.0, full.max.x - 8.0] {
            circle(&rec, pos2(sx, hinge.center().y), 2.5, Color32::from_rgb(20, 20, 24));
        }
        p.set(bg, rec.shape());
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, Color32::from_rgb(248, 200, 72));
    }
}

trait ClipShape {
    fn clip(self, r: Rect) -> Shape;
}

impl ClipShape for Shape {
    /// Keep only the line pieces inside `r` (stripes on the touch screen).
    fn clip(self, r: Rect) -> Shape {
        match self {
            Shape::Vec(v) => Shape::Vec(v.into_iter().map(|s| s.clip(r)).collect()),
            Shape::LineSegment { points: [a, b], stroke } => {
                // clip the segment to the rectangle (Liang–Barsky)
                let d = b - a;
                let (mut t0, mut t1) = (0.0f32, 1.0f32);
                for (pp, q) in [(-d.x, a.x - r.min.x), (d.x, r.max.x - a.x), (-d.y, a.y - r.min.y), (d.y, r.max.y - a.y)] {
                    if pp.abs() < 1e-6 {
                        if q < 0.0 {
                            return Shape::Noop;
                        }
                    } else {
                        let t = q / pp;
                        if pp < 0.0 {
                            t0 = t0.max(t);
                        } else {
                            t1 = t1.min(t);
                        }
                    }
                }
                if t0 >= t1 {
                    return Shape::Noop;
                }
                Shape::LineSegment { points: [a + d * t0, a + d * t1], stroke }
            }
            other => other,
        }
    }
}

// ───────────────────────────── pop-up cards ─────────────────────────────

/// The Gen 4 message box.
fn toast_card(p: &Painter, r: Rect, _tint: Color32) {
    let rad = 8.0f32.min(r.height() / 2.0);
    rrect(p, r, rad, Color32::from_rgb(88, 96, 120));
    if r.height() > 6.0 {
        rrect(p, r.shrink(2.0), (rad - 2.0).max(0.0), Color32::from_rgb(184, 192, 216));
        rrect(p, r.shrink(3.5), (rad - 3.5).max(0.0), Color32::from_rgb(250, 250, 250));
    }
}

/// A message being typed out, with the red ▼ blinking when it's done.
fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let ic = pos2(r.min.x + 28.0, r.center().y);
    // the icon on a little battle platform
    p.add(Shape::ellipse_filled(ic + vec2(0.0, 15.0), vec2(20.0, 5.0), Color32::from_rgb(208, 216, 200)));
    p.add(Shape::ellipse_stroke(ic + vec2(0.0, 15.0), vec2(20.0, 5.0), Stroke::new(1.0f32, Color32::from_rgb(160, 168, 152))));
    super::ov_toast::badge(p, ic - vec2(0.0, 2.0), 12.0, v);
    let tx = r.min.x + 56.0;
    let room = r.max.x - tx - 18.0;
    ds(p, pos2(tx, r.min.y + 14.0), Align2::LEFT_CENTER, &clip_text(p, &v.caption.to_uppercase(), &pixel_font(7.0), room), 7.0, tone(v.tint()));
    let title_s = clip_text(p, &v.title, &pixel_font(9.0), room);
    let line_s = clip_text(p, &v.line, &pixel_font(7.0), room);
    let n1 = title_s.chars().count();
    let shown = v.typed(&format!("{title_s}{line_s}"), 0.3, 40.0);
    let (a, b): (String, String) = (shown.chars().take(n1).collect(), shown.chars().skip(n1).collect());
    ds(p, pos2(tx, r.min.y + 31.0), Align2::LEFT_CENTER, &a, 9.0, TXT);
    ds(p, pos2(tx, r.min.y + 47.0), Align2::LEFT_CENTER, &b, 7.0, Color32::from_rgb(104, 104, 116));
    if shown.chars().count() >= n1 + line_s.chars().count() && (v.t * 2.5) as u32 % 2 == 0 {
        let c = pos2(r.max.x - 14.0, r.max.y - 13.0);
        poly(p, vec![c + vec2(-4.0, -3.0), c + vec2(4.0, -3.0), c + vec2(0.0, 3.0)], HP_RED);
    }
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: 8.0,
        card: toast_card,
        inner: |r| r.shrink2(vec2(6.0, 5.0)),
        toast: toast_body,
        title_font: pixel_font(8.5),
        body_font: pixel_font(7.0),
    }
}

/// The Card Collector's XP bar in the Retro DS look: a Gen 4 status box
/// with a "PACK" label, an EXP-blue bar and pixel numbers. `msg` replaces
/// the numbers for a moment (+XP, a pack, snapping); `glow` lights the
/// edge gold (0 to 1); `near` lights it blue (about to snap).
#[allow(clippy::too_many_arguments)]
pub(super) fn card_xp_strip(p: &Painter, r: Rect, frac: f32, xp: u64, need: u64, packs: u32, msg: Option<&str>, glow: f32, near: bool) {
    // the box: dark outer edge, light inner edge, cream inside
    let edge = if near { Color32::from_rgb(96, 200, 248) } else if glow > 0.0 { mix(Color32::from_rgb(88, 96, 120), GOLD, glow) } else { Color32::from_rgb(88, 96, 120) };
    rrect(p, r, 6.0, edge);
    rrect(p, r.shrink(1.5), 5.0, Color32::from_rgb(184, 192, 216));
    let inner = r.shrink(2.5);
    rrect(p, inner, 4.0, BOX);
    // left: PACK, with how many wait
    let ly = inner.center().y;
    let label = ds(p, pos2(inner.min.x + 6.0, ly), Align2::LEFT_CENTER, "PACK", 8.0, TXT);
    let mut x = label.max.x + 4.0;
    if packs > 0 {
        let b = Rect::from_min_size(pos2(x, ly - 6.0), vec2(if packs > 9 { 20.0 } else { 14.0 }, 12.0));
        rrect(p, b, 3.0, HP_RED);
        p.text(b.center(), Align2::CENTER_CENTER, if packs > 9 { "9+".to_string() } else { packs.to_string() }, pixel_font(7.0), Color32::WHITE);
        x = b.max.x + 4.0;
    }
    // right: the numbers (or the message)
    let right = msg.map(|m| m.to_string()).unwrap_or_else(|| format!("{xp}/{need}"));
    let g = p.layout_no_wrap(right.clone(), pixel_font(8.0), TXT);
    let rx = inner.max.x - 6.0;
    let col = if msg.is_some() { Color32::from_rgb(184, 128, 8) } else { TXT };
    ds(p, pos2(rx, ly), Align2::RIGHT_CENTER, &right, 8.0, col);
    // the EXP bar between them
    let bar = Rect::from_min_max(pos2(x + 2.0, ly - 2.5), pos2((rx - g.size().x - 8.0).max(x + 20.0), ly + 2.5));
    rrect(p, bar.expand(1.5), 2.0, Color32::from_rgb(64, 64, 72));
    rrect(p, bar, 1.0, Color32::from_rgb(96, 96, 104));
    let w = bar.width() * frac.clamp(0.0, 1.0);
    if w > 0.5 {
        let f = Rect::from_min_size(bar.min, vec2(w, bar.height()));
        rrect(p, f, 1.0, EXP);
        rrect(p, Rect::from_min_size(f.min, vec2(w, 1.5)), 0.5, mix(EXP, Color32::WHITE, 0.45));
    }
}
