//! Overlay theme "Terminal": a console window. A title bar, a command
//! prompt, the shiny in a box-drawing frame, stats printed as log lines
//! with dot leaders, ASCII progress bars, a block-character sparkline,
//! [bracket] buttons and a blinking cursor. A shiny prints "MATCH FOUND".

use super::ov_common::*;
use super::App;
use crate::fonts::{mono_bold_font, mono_font};
use crate::theme::{self};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const BG: Color32 = Color32::from_rgb(8, 12, 10);
pub(super) const FG: Color32 = Color32::from_rgb(96, 255, 146);
pub(super) const DIM: Color32 = Color32::from_rgb(58, 156, 92);
pub(super) const FAINT: Color32 = Color32::from_rgb(34, 84, 52);
pub(super) const AMBER: Color32 = Color32::from_rgb(255, 196, 76);
const BAR: Color32 = Color32::from_rgb(24, 30, 28);
const RAD: f32 = 7.0;
const TITLE_H: f32 = 24.0;

/// Status colours as terminal colours.
fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        DIM
    } else if c == theme::GREEN {
        FG
    } else {
        mix(c, Color32::WHITE, 0.15)
    }
}

/// Monospace text.
fn tx(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    p.text(pos, anchor, s, mono_font(size), col)
}

fn txb(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    p.text(pos, anchor, s, mono_bold_font(size), col)
}

/// Width of one character at `size`.
fn cw(p: &Painter, size: f32) -> f32 {
    text_w(p, "0", mono_font(size)).max(1.0)
}

/// "[#########.........]" filling `chars` characters.
pub(super) fn ascii_bar(frac: f32, chars: usize) -> String {
    let n = chars.saturating_sub(2).max(1);
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    format!("[{}{}]", "#".repeat(lit), ".".repeat(n - lit))
}

/// Block characters for a list of values.
pub(super) fn blocks(vals: &[u64]) -> String {
    const B: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = vals.iter().copied().max().unwrap_or(0).max(1) as f32;
    vals.iter().map(|&v| if v == 0 { ' ' } else { B[((v as f32 / max) * 7.0).round() as usize] }).collect()
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    p.text(pos, anchor, s, mono_bold_font(font.size), col)
}

/// The window: title bar, dark screen, scan lines.
fn window(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.6 + 0.4 * opacity;
    rrect(s, r, RAD, fade(BG, a));
    s.put(Shape::rect_filled(Rect::from_min_size(r.min, vec2(r.width(), TITLE_H)), egui::Rounding { nw: RAD, ne: RAD, sw: 0.0, se: 0.0 }, fade(BAR, a.max(0.85))));
    line(s, pos2(r.min.x, r.min.y + TITLE_H), pos2(r.max.x, r.min.y + TITLE_H), Stroke::new(1.0f32, FAINT));
    screen_lines(s, Rect::from_min_max(pos2(r.min.x + 1.0, r.min.y + TITLE_H + 1.0), r.max - vec2(1.0, 1.0)));
    rstroke(s, r, RAD, Stroke::new(1.0f32, if alert { AMBER } else { FAINT }));
}

fn screen_lines(s: &dyn Sink, r: Rect) {
    for (k, a) in [(1.0, 0.035), (0.6, 0.04)] {
        circle(s, pos2(r.center().x, r.min.y), r.width() * 0.5 * k, fade(FG, a * 0.5));
    }
    let mut y = r.min.y + 1.0;
    while y < r.max.y {
        line(s, pos2(r.min.x, y), pos2(r.max.x, y), Stroke::new(1.0f32, fade(Color32::BLACK, 0.28)));
        y += 3.0;
    }
}

fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    // blocks: one per 6 px, lit ones bright
    let n = (r.width() / 6.0).floor().max(4.0) as usize;
    let w = r.width() / n as f32;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    for i in 0..n {
        let b = Rect::from_min_size(pos2(r.min.x + i as f32 * w, r.min.y), vec2(w - 1.5, r.height()));
        rrect(s, b, 0.0, if i < lit { col } else { fade(FAINT, 0.7) });
    }
}

/// [ bracket ] buttons: hover turns them solid.
fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let col = match kind {
        Kind::Main => FG,
        Kind::Good => FG,
        Kind::Gold => AMBER,
        Kind::Plain => DIM,
    };
    let solid = hovered || down;
    if solid {
        rrect(p, r, 0.0, col);
    } else if kind == Kind::Main {
        rrect(p, r, 0.0, fade(FG, 0.12));
    }
    let text_col = if solid { BG } else if kind == Kind::Plain { FG } else { col };
    let size = if kind == Kind::Main { 13.0 } else { 12.0 };
    let inner = match label {
        "⏸" => "||".to_string(),
        "▶" => ">".to_string(),
        "−1" => "-1".to_string(),
        "✖" => "x".to_string(),
        l => l.to_uppercase(),
    };
    let font: fn(f32) -> FontId = if kind == Kind::Main { mono_bold_font } else { mono_font };
    let room = r.width() - 2.0 * text_w(p, "[", font(size)) - 6.0;
    let f = fit(p, &inner, font, size, 8.5, room);
    let s = clip_text(p, &inner, &f, room);
    p.text(r.center(), Align2::CENTER_CENTER, &s, f.clone(), text_col);
    p.text(pos2(r.min.x + 2.0, r.center().y), Align2::LEFT_CENTER, "[", f.clone(), text_col);
    p.text(pos2(r.max.x - 2.0, r.center().y), Align2::RIGHT_CENTER, "]", f, text_col);
}

pub(super) fn look() -> Look {
    Look {
        text: FG,
        muted: DIM,
        accent: FG,
        radius: RAD,
        num: mono_bold_font,
        label: mono_font,
        body: mono_font,
        panel: |s, r, o, a| {
            rrect(s, r, RAD, fade(BG, 0.6 + 0.4 * o));
            screen_lines(s, r.shrink(1.0));
            rstroke(s, r, RAD, Stroke::new(1.0f32, if a { AMBER } else { FAINT }));
        },
        big,
        button,
        gauge,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, FAINT)),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn terminal_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let (x0, x1) = (left + 12.0, left + w - 12.0);
        let bg = p.add(Shape::Noop);
        let fs = if v.compact { 11.0 } else { 12.0 };
        let lh = fs + 6.0;
        let c1 = cw(&p, fs);
        let cols = ((x1 - x0) / c1).floor() as usize;

        // ── Title bar
        let mid = top + TITLE_H / 2.0;
        let mut x = left + 10.0;
        if m.status_dot {
            let r = Rect::from_center_size(pos2(x + 4.0, mid), vec2(8.0, 8.0));
            rrect(&p, r, 0.0, tone(v.status.0));
            tip(ui, r.expand(3.0), "tm-dot", &v.status.1);
            x += 14.0;
        }
        let who = match &v.target {
            Some(sp) => format!("shiny@tracker: ~/hunt/{}", sp.slug),
            None => "shiny@tracker: ~/hunt".to_string(),
        };
        let mut cx = left + w - 6.0;
        let mut ctrl_rects = Vec::new();
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 10.0, mid), vec2(22.0, TITLE_H - 2.0));
            ctrl_rects.push((c, r));
            cx -= 22.0;
        }
        let f = mono_font(10.0);
        tx(&p, pos2(x, mid), Align2::LEFT_CENTER, &clip_text(&p, &who, &f, cx - x - 6.0), 10.0, DIM);
        for (c, r) in ctrl_rects {
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                rrect(&p, r, 0.0, if c == Ctrl::Close { Color32::from_rgb(196, 43, 28) } else { FAINT });
            }
            ctrl_icon(&p, r, c, false, if on { AMBER } else if hov { Color32::WHITE } else { DIM }, 11.0);
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 2.0, top + TITLE_H)), "tm-drag");

        let mut y = top + TITLE_H + 10.0;
        // the command line
        let mut cmd = match &v.target {
            Some(sp) => format!("hunt --target {}", sp.slug),
            None => "hunt --target ?".to_string(),
        };
        if let Some(lv) = v.level {
            cmd.push_str(&format!(" --lv {lv}"));
        }
        let pr = txb(&p, pos2(x0, y + lh / 2.0), Align2::LEFT_CENTER, "$", fs, FG);
        tx(&p, pos2(pr.max.x + c1, y + lh / 2.0), Align2::LEFT_CENTER, &clip_text(&p, &cmd, &mono_font(fs), x1 - pr.max.x - c1), fs, Color32::from_rgb(210, 236, 218));
        y += lh;
        if v.ghost {
            tx(&p, pos2(x0, y + lh / 2.0), Align2::LEFT_CENTER, "# click-through is on", fs, AMBER);
            y += lh;
        }
        y += 4.0;

        // ── Hero: box-drawn frame, name, the count
        let art = v.sprites();
        if art || m.target || m.counter {
            let bs: f32 = if !art { 0.0 } else if v.compact { 70.0 } else { 88.0 };
            let hero_h = bs.max(if m.counter { 70.0 } else { lh * 2.0 });
            if art {
                let br = Rect::from_min_size(pos2(x0, y), vec2(bs, bs));
                let k = 6.0;
                let st = Stroke::new(1.2f32, DIM);
                for (a, b) in [
                    (br.left_top(), br.left_top() + vec2(k * 2.0, 0.0)),
                    (br.left_top(), br.left_top() + vec2(0.0, k * 2.0)),
                    (br.right_top(), br.right_top() - vec2(k * 2.0, 0.0)),
                    (br.right_top(), br.right_top() + vec2(0.0, k * 2.0)),
                    (br.left_bottom(), br.left_bottom() + vec2(k * 2.0, 0.0)),
                    (br.left_bottom(), br.left_bottom() - vec2(0.0, k * 2.0)),
                    (br.right_bottom(), br.right_bottom() - vec2(k * 2.0, 0.0)),
                    (br.right_bottom(), br.right_bottom() - vec2(0.0, k * 2.0)),
                ] {
                    line(&p, a, b, st);
                }
                let mut d = br.min.x + k * 3.0;
                while d < br.max.x - k * 3.0 {
                    line(&p, pos2(d, br.min.y), pos2(d + 3.0, br.min.y), Stroke::new(1.0f32, FAINT));
                    line(&p, pos2(d, br.max.y), pos2(d + 3.0, br.max.y), Stroke::new(1.0f32, FAINT));
                    d += 6.0;
                }
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = br.center().x + (k as f32 - (n - 1.0) / 2.0) * bs * 0.28;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, br.center().y), vec2(bs * 0.5, bs * 0.8));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, br.center(), vec2(bs - 12.0, bs - 12.0));
                    }
                }
                tx(&p, pos2(br.min.x + 4.0, br.max.y - 7.0), Align2::LEFT_CENTER, "shiny.gif", 8.5, FAINT);
            }
            let rx = if art { x0 + bs + 14.0 } else { x0 };
            let room = x1 - rx;
            if let Some((_, text)) = &v.alert {
                let r = Rect::from_min_size(pos2(rx - 2.0, y + 2.0), vec2(room + 2.0, 22.0));
                if v.blink {
                    rrect(&p, r, 0.0, FG);
                }
                txb(&p, pos2(rx + 2.0, r.center().y), Align2::LEFT_CENTER, &clip_text(&p, ">>> MATCH FOUND", &mono_bold_font(fs + 1.0), room), fs + 1.0, if v.blink { BG } else { FG });
                let f = mono_font(fs - 1.0);
                tx(&p, pos2(rx, y + 36.0), Align2::LEFT_CENTER, &clip_text(&p, &format!("shiny: {}", text.to_lowercase()), &f, room), fs - 1.0, Color32::WHITE);
                let b = ActBtn { act: Act::Dismiss, label: "dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Main, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, y + 52.0), vec2(100.0, 22.0)), &b, &button);
            } else {
                let mut ry = y;
                if m.target {
                    let name = v.title.clone().unwrap_or_else(|| "none (open setup)".into());
                    let lr = tx(&p, pos2(rx, ry + lh / 2.0), Align2::LEFT_CENTER, "target:", fs - 1.0, DIM);
                    let col = if v.flash && v.blink { AMBER } else { FG };
                    let f = mono_bold_font(fs);
                    let nr = p.text(pos2(lr.max.x + c1 * 0.6, ry + lh / 2.0), Align2::LEFT_CENTER, clip_text(&p, &name.to_lowercase(), &f, x1 - lr.max.x - c1), f, col);
                    if v.multi {
                        tip(ui, nr, "tm-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                    }
                    ry += lh;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let f = fit(&p, &num, mono_bold_font, if v.compact { 32.0 } else { 38.0 }, 16.0, room);
                    let nr = p.text(pos2(rx - 1.0, ry + f.size * 0.6), Align2::LEFT_CENTER, &num, f.clone(), FG);
                    // the cursor after the number
                    if v.blink {
                        rrect(&p, Rect::from_min_size(pos2(nr.max.x + 4.0, nr.min.y + f.size * 0.18), vec2(f.size * 0.5, f.size * 0.85)), 0.0, fade(FG, 0.8));
                    }
                    let mut lab = v.counter_label.to_lowercase();
                    if let (Some((t, _, _)), false) = (&v.timer, v.compact) {
                        lab = format!("{lab} | today {t}");
                    }
                    let f2 = mono_font(fs - 2.0);
                    let lr = p.text(pos2(rx, nr.max.y + 6.0), Align2::LEFT_CENTER, clip_text(&p, &lab, &f2, room), f2, if v.counter_label == "ENCOUNTERS" { DIM } else { tone(v.counter_col) });
                    if let Some((_, _, tp)) = &v.timer {
                        tip(ui, lr, "tm-timer", tp);
                    }
                }
                if let Some((text, dry, t)) = &v.watch {
                    let wr = tx(&p, pos2(x1, y + hero_h - 6.0), Align2::RIGHT_CENTER, &text.to_lowercase(), fs - 3.0, if *dry { AMBER } else { DIM });
                    tip(ui, wr, "tm-watch", t);
                }
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let r = Rect::from_min_max(pos2(x0 - 4.0, y + hero_h / 2.0 - 13.0), pos2(x1 + 4.0, y + hero_h / 2.0 + 13.0));
                rrect(&p, r, 0.0, fade(FG, a));
                let f = mono_bold_font(fs);
                p.text(r.center(), Align2::CENTER_CENTER, clip_text(&p, &format!("*** milestone: {} ***", text.to_lowercase()), &f, r.width() - 8.0), f, fade(BG, a));
            }
            y += hero_h + 10.0;
        }

        let kv = |ui: &mut egui::Ui, y: &mut f32, key: &str, val: &str, col: Color32, t: &str| {
            let kr = tx(&p, pos2(x0, *y + lh / 2.0), Align2::LEFT_CENTER, &format!("> {key} "), fs, DIM);
            let f = mono_font(fs);
            let vtxt = clip_text(&p, val, &f, x1 - kr.max.x - c1 * 2.0);
            let vr = p.text(pos2(x1, *y + lh / 2.0), Align2::RIGHT_CENTER, &vtxt, f.clone(), col);
            let dots = ((vr.min.x - kr.max.x - c1) / c1).floor().max(0.0) as usize;
            p.text(pos2(kr.max.x, *y + lh / 2.0), Align2::LEFT_CENTER, ".".repeat(dots), f, FAINT);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + lh)), ("tm-kv", key.to_string()), t);
            *y += lh;
        };

        // ── Odds: the multiple and an ASCII bar
        if m.odds_bar && v.alert.is_none() {
            let col = tone(v.odds.color);
            let top_y = y;
            kv(ui, &mut y, "odds", &format!("{} {}", v.mult_text().replace('×', "x"), v.odds_word().to_lowercase()), col, &v.odds.tip);
            let right = v.odds.right.to_lowercase();
            let bar_chars = cols.saturating_sub(right.chars().count() + 3).max(8);
            let bar = ascii_bar(v.odds.frac, bar_chars);
            let br = tx(&p, pos2(x0 + c1 * 2.0, y + lh / 2.0), Align2::LEFT_CENTER, &bar, fs, col);
            tx(&p, pos2(x1, y + lh / 2.0), Align2::RIGHT_CENTER, &right, fs - 1.0, DIM);
            let _ = br;
            y += lh;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "tm-odds", &v.odds.tip);
        }
        if m.method_line && v.alert.is_none() {
            let f = mono_font(fs - 1.0);
            p.text(pos2(x0, y + lh / 2.0), Align2::LEFT_CENTER, clip_text(&p, &format!("# {}", v.method.to_lowercase()), &f, x1 - x0), f, FAINT.gamma_multiply(1.6));
            y += lh;
        }
        if v.alert.is_none() {
            if m.luck {
                match &v.luck {
                    Some((pct, col, t)) => kv(ui, &mut y, "luck", &format!("{pct:.0}% would have it"), tone(*col), t),
                    None => kv(ui, &mut y, "luck", "after 1st battle", DIM, "Needs the odds first (start a battle)."),
                }
            }
            if m.milestone {
                let (what, when, t) = v.milestone.clone();
                kv(ui, &mut y, "next", &format!("{what} {when}").to_lowercase(), FG, &t);
            }
            if m.session {
                kv(ui, &mut y, "session", &format!("{} | {} | {}", theme::thousands(v.session.0), v.session_time(), v.session_rate()), FG, "Encounters | hunting time | pace, this session");
            }
            if m.last_seen {
                let txt = match &v.last.1 {
                    Some(a) => format!("{} ({a})", v.last.0),
                    None => v.last.0.clone(),
                };
                kv(ui, &mut y, "last", &txt.to_lowercase(), FG, "");
            }
            if m.sparkline {
                let rate = format!("{}/h", theme::thousands(v.pace_hr));
                let kr = tx(&p, pos2(x0, y + lh / 2.0), Align2::LEFT_CENTER, "> pace ", fs, DIM);
                tx(&p, pos2(x1, y + lh / 2.0), Align2::RIGHT_CENTER, &rate, fs, FG);
                let room = ((x1 - kr.max.x - text_w(&p, &rate, mono_font(fs)) - c1) / c1).floor().max(4.0) as usize;
                let vals = &v.pace[v.pace.len().saturating_sub(room)..];
                let g = tx(&p, pos2(kr.max.x, y + lh / 2.0), Align2::LEFT_CENTER, &blocks(vals), fs, FG);
                tip(ui, g, "tm-pace", "Encounters each minute for the last 20 minutes.");
                y += lh;
            }
            if !v.chips.is_empty() {
                let mut x = x0;
                y += 2.0;
                for (text, col, _f, t) in &v.chips {
                    let s = format!("[{}]", text.to_lowercase());
                    let f = mono_font(fs - 1.0);
                    let wdt = text_w(&p, &s, f.clone());
                    if x + wdt > x1 && x > x0 {
                        x = x0;
                        y += lh;
                    }
                    let r = p.text(pos2(x, y + lh / 2.0), Align2::LEFT_CENTER, &s, f, if *col == theme::MUTED() { AMBER } else { tone(*col) });
                    tip(ui, r, ("tm-chip", text.clone()), t);
                    x += wdt + c1;
                }
                y += lh + 2.0;
            }
            let bar = |ui: &mut egui::Ui, y: &mut f32, key: &str, right: String, frac: f32, col: Color32, t: &str| {
                let kr = tx(&p, pos2(x0, *y + lh / 2.0), Align2::LEFT_CENTER, &format!("> {key} "), fs, DIM);
                let rr = tx(&p, pos2(x1, *y + lh / 2.0), Align2::RIGHT_CENTER, &right, fs - 1.0, DIM);
                let n = ((rr.min.x - kr.max.x - c1) / c1).floor().max(6.0) as usize;
                tx(&p, pos2(kr.max.x, *y + lh / 2.0), Align2::LEFT_CENTER, &ascii_bar(frac, n), fs, col);
                tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + lh)), ("tm-bar", key.to_string()), t);
                *y += lh;
            };
            if m.xp {
                bar(ui, &mut y, &format!("lv{}", v.st.level.max(1)), format!("{}/{}xp", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, Color32::from_rgb(110, 190, 255), "Trainer level progress");
            }
            if m.daily_goal {
                let right = if v.goal > 0 { format!("{}/{}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
                bar(ui, &mut y, "goal", right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, FG, "Encounters today across every run");
            }
        }

        // ── [ buttons ]
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() && v.alert.is_none() {
            y += 4.0;
            let h = 24.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += tx(&p, pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, &format!("? {}", q.to_lowercase()), fs, AMBER).width() + c1;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(44.0, h)), b, &button);
                    x += 48.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x, y), pos2(x1, y + h)), &acts.btns[3], &button);
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
            y += h + 6.0;
        }
        // the prompt, waiting
        let pr = txb(&p, pos2(x0, y + lh / 2.0), Align2::LEFT_CENTER, "$", fs, FG);
        if v.blink {
            rrect(&p, Rect::from_min_size(pos2(pr.max.x + c1 * 0.6, y + 3.0), vec2(c1 * 0.9, lh - 6.0)), 0.0, FG);
        }
        y += lh + 8.0;

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, window);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, FG);
    }
}

/// For the theme picker.
pub(super) fn preview(p: &Painter, card: Rect) {
    window(p, card, 1.0, false);
    p.text(card.min + vec2(6.0, 6.0), Align2::LEFT_CENTER, "~/hunt", mono_font(7.0), DIM);
    p.text(card.min + vec2(6.0, 30.0), Align2::LEFT_CENTER, "$ 42", mono_bold_font(13.0), FG);
    p.text(card.min + vec2(6.0, 46.0), Align2::LEFT_CENTER, "[###...]", mono_font(8.5), FG);
    p.text(card.min + vec2(6.0, 58.0), Align2::LEFT_CENTER, "▂▃▅▇▅", mono_font(8.5), DIM);
    rrect(p, Rect::from_min_size(card.min + vec2(6.0, 66.0), vec2(6.0, 8.0)), 0.0, FG);
}

// ───────────────────────────── pop-up cards ─────────────────────────────

const TOAST_BAR: f32 = 15.0;

/// A small console window: title bar, dark screen, scan lines.
fn toast_card(p: &Painter, r: Rect, _tint: Color32) {
    let rad = RAD.min(r.height() / 2.0);
    rrect(p, r, rad, BG);
    if r.height() > TOAST_BAR + 2.0 {
        p.add(Shape::rect_filled(Rect::from_min_size(r.min, vec2(r.width(), TOAST_BAR)), egui::Rounding { nw: rad, ne: rad, sw: 0.0, se: 0.0 }, BAR));
        line(p, pos2(r.min.x, r.min.y + TOAST_BAR), pos2(r.max.x, r.min.y + TOAST_BAR), Stroke::new(1.0f32, FAINT));
        for (i, c) in [Color32::from_rgb(232, 84, 72), AMBER, FG].into_iter().enumerate() {
            circle(p, pos2(r.min.x + 10.0 + i as f32 * 10.0, r.min.y + TOAST_BAR / 2.0), 2.8, fade(c, 0.8));
        }
        tx(p, pos2(r.center().x, r.min.y + TOAST_BAR / 2.0), Align2::CENTER_CENTER, "notify", 9.0, DIM);
        screen_lines(p, Rect::from_min_max(pos2(r.min.x + 1.0, r.min.y + TOAST_BAR + 1.0), r.max - vec2(1.0, 1.0)));
    }
    rstroke(p, r, rad, Stroke::new(1.0f32, FAINT));
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    use super::ov_toast::What;
    let x0 = r.min.x + 9.0;
    let room = r.max.x - x0 - 10.0;
    let tag = match v.what {
        What::Achievement => "[ACH]",
        What::LevelUp => "[LVL]",
        What::Welcome => "[HI!]",
        What::Goal => "[GOAL]",
        What::Milestone => "[MILE]",
    };
    let col = if v.what == What::Achievement || v.what == What::Milestone { AMBER } else { tone(v.tint()) };
    // line 1: the tag and the caption
    let y1 = r.min.y + TOAST_BAR + 10.0;
    let tw = txb(p, pos2(x0, y1), Align2::LEFT_CENTER, tag, 10.0, v.a(col)).width();
    let cap = v.caption.to_lowercase().replace('!', "");
    tx(p, pos2(x0 + tw + 6.0, y1), Align2::LEFT_CENTER, &clip_text(p, &cap, &mono_font(10.0), room - tw - 6.0), 10.0, v.a(DIM));
    // line 2: "> title" typed out with the cursor
    let y2 = y1 + 15.0;
    let pw = tx(p, pos2(x0, y2), Align2::LEFT_CENTER, ">", 11.5, v.a(DIM)).width();
    let title = clip_text(p, &v.title, &mono_bold_font(11.5), room - pw - 14.0);
    let shown = v.typed(&title, 0.25, 34.0);
    let w = txb(p, pos2(x0 + pw + 6.0, y2), Align2::LEFT_CENTER, &shown, 11.5, v.a(FG)).width();
    if (v.t * 2.2) as u32 % 2 == 0 {
        rrect(p, Rect::from_min_size(pos2(x0 + pw + 7.0 + w, y2 - 6.0), vec2(7.0, 12.0)), 0.0, v.a(FG));
    }
    // line 3: the details, once the title is in
    if shown.chars().count() >= title.chars().count() {
        let line_s = clip_text(p, &format!("  {}", v.line), &mono_font(9.0), room);
        tx(p, pos2(x0, y2 + 13.0), Align2::LEFT_CENTER, &line_s, 9.0, v.a(DIM));
    }
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: RAD,
        card: toast_card,
        inner: |r| Rect::from_min_max(r.min + vec2(4.0, TOAST_BAR + 1.0), r.max - vec2(6.0, 2.0)),
        toast: toast_body,
        title_font: mono_bold_font(11.0),
        body_font: mono_font(9.5),
    }
}
