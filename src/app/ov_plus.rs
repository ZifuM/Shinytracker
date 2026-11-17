//! Overlay themes made to go with the Modern Plus app themes that had none:
//! Studio, Pokédex, Aurora glass, Field guide, Synthwave, Gold, Companion,
//! Pocket, Candy, Brutal, Liquid, Winter and Spooky.
//!
//! They share one layout (Full and Compact here; the Bar and the hidden
//! badge come from ov_common through `look`), and each brings its own
//! style: its panel, buttons, odds gauge, the stage behind the sprite,
//! little decorations, chips, type and colours, and its pop-up cards.

use super::ov_common::*;
use super::App;
use crate::config::OverlayTheme;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

/// One overlay theme's style.
#[derive(Clone, Copy)]
pub(super) struct Style {
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    /// The window's corner (0: square).
    pub radius: f32,
    /// Big numbers, small labels, everything else, the title.
    pub num: fn(f32) -> FontId,
    pub label: fn(f32) -> FontId,
    pub body: fn(f32) -> FontId,
    /// Small labels in capitals.
    pub caps: bool,
    /// The background and frame (rect, opacity, alert blink).
    pub panel: fn(&dyn Sink, Rect, f32, bool),
    pub button: fn(&Painter, Rect, &str, Kind, bool, bool),
    /// The odds / XP / goal bar (rect, filled share, colour).
    pub gauge: fn(&dyn Sink, Rect, f32, Color32),
    /// Behind the sprite (centre, radius, accent).
    pub stage: fn(&Painter, Pos2, f32, Color32),
    /// Little touches over the panel (the whole card, time in seconds).
    pub decor: fn(&dyn Sink, Rect, f32),
    /// A chip's background (rect, its colour).
    pub chip: fn(&Painter, Rect, Color32),
    /// A status colour made readable on the panel.
    pub tone: fn(Color32) -> Color32,
    /// A hard shadow under the window (Brutal).
    pub shadow: Option<egui::Vec2>,
    /// Its decorations move (snow, ribbons, a scrolling grid…).
    pub animated: bool,
}

/// The style of a theme made here (None: another file's theme).
pub(super) fn style_of(t: OverlayTheme) -> Option<Style> {
    use OverlayTheme as O;
    Some(match t {
        O::Studio => studio::style(),
        O::Pokedex => dex::style(),
        O::Aurora => aurora::style(),
        O::FieldGuide => field::style(),
        O::Synthwave => synth::style(),
        O::Gold => gold::style(),
        O::Companion => companion::style(),
        O::Pocket => pocket::style(),
        O::Candy => candy::style(),
        O::Brutal => brutal::style(),
        O::Liquid => liquid::style(),
        O::Winter => winter::style(),
        O::Spooky => spooky::style(),
        _ => return None,
    })
}

fn big_plain(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    p.text(pos, anchor, s, font, col)
}

/// The shared `Look` (for the Bar, the badge, the strips under it).
pub(super) fn look_for(t: OverlayTheme) -> Look {
    let st = style_of(t).unwrap_or_else(studio::style);
    Look {
        text: st.text,
        muted: st.muted,
        accent: st.accent,
        radius: st.radius,
        num: st.num,
        label: st.label,
        body: st.body,
        panel: st.panel,
        big: big_plain,
        button: st.button,
        gauge: st.gauge,
        divider: match t {
            OverlayTheme::Pokedex | OverlayTheme::Pocket | OverlayTheme::Brutal => ink_divider,
            OverlayTheme::FieldGuide | OverlayTheme::Companion | OverlayTheme::Candy | OverlayTheme::Winter => dark_divider,
            _ => soft_divider,
        },
        tone: st.tone,
        shadow: st.shadow,
    }
}

/// A small label, in capitals if the theme likes them.
fn label(p: &Painter, st: &Style, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    let text = if st.caps { s.to_uppercase() } else { s.to_string() };
    p.text(pos, anchor, text, (st.label)(size), col)
}

impl App {
    /// A Modern Plus theme's overlay, Full or Compact.
    pub(super) fn plus_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView, st: &Style) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let off = st.shadow.unwrap_or(egui::Vec2::ZERO);
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0 - off.x;
        let (x0, x1) = (left + 18.0, left + w - 18.0);
        let acc = st.accent;
        let (text, soft) = (st.text, st.muted);
        let bg = p.add(Shape::Noop);
        let deco = p.add(Shape::Noop);
        let time = ui.input(|i| i.time) as f32;

        // ── Top line: status, title, level; the controls on the right
        let mut y = top + 8.0;
        let mid = y + 11.0;
        let mut x = x0;
        if m.status_dot {
            circle(&p, pos2(x + 4.0, mid), 4.0, (st.tone)(v.status.0));
            tip(ui, Rect::from_center_size(pos2(x + 4.0, mid), vec2(14.0, 14.0)), "pl-dot", &v.status.1);
            x += 14.0;
        }
        let tr = label(&p, st, pos2(x, mid), Align2::LEFT_CENTER, if v.ghost { "Click-through" } else { "Shiny hunt" }, 10.0, if v.ghost { GOLD } else { soft });
        if let (Some(lv), false) = (v.level, v.ghost) {
            label(&p, st, pos2(tr.max.x + 10.0, mid), Align2::LEFT_CENTER, &format!("Lv {lv}"), 9.5, acc);
        }
        let mut cx = x1 + 4.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, mid), vec2(20.0, 20.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                rrect(&p, r, st.radius.min(10.0), if c == Ctrl::Close { rgba(220, 60, 60, 0.85) } else { fade(text, 0.14) });
            }
            ctrl_icon(&p, r, c, false, if on { acc } else if hov { text } else { fade(text, 0.55) }, 12.0);
            cx -= 22.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 6.0, mid + 12.0)), "pl-drag");
        y = mid + 16.0;

        // ── Hero: the sprite on the theme's stage, the name, a big count
        let art = v.sprites();
        if art || m.target || m.counter {
            let disc_r = if !art { 0.0 } else if v.compact { 38.0 } else { 46.0 };
            let hero_h = if art { disc_r * 2.0 + 8.0 } else if m.counter { 70.0 } else { 34.0 };
            let c = pos2(x0 + disc_r, y + hero_h / 2.0);
            if art {
                (st.stage)(&p, c, disc_r, acc);
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = c.x + (k as f32 - (n - 1.0) / 2.0) * disc_r * 0.55;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, c.y), vec2(disc_r * 0.9, disc_r * 1.2));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, c, vec2(disc_r * 1.6, disc_r * 1.6));
                    }
                }
            }
            let rx = if art { x0 + disc_r * 2.0 + 16.0 } else { x0 };
            let mut ry = y + if art { 6.0 } else { 2.0 };
            if let Some((_, msg)) = &v.alert {
                theme::sparkle(&p, pos2(rx + 5.0, ry + 6.0), 5.5, GOLD);
                label(&p, st, pos2(rx + 14.0, ry + 6.0), Align2::LEFT_CENTER, "Shiny!", 10.0, GOLD);
                p.text(pos2(rx, ry + 34.0), Align2::LEFT_CENTER, "Found it!", (st.num)(28.0), if v.blink { GOLD } else { text });
                let f = (st.body)(11.5);
                p.text(pos2(rx, ry + 60.0), Align2::LEFT_CENTER, clip_text(&p, msg, &f, x1 - rx), f, soft);
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, ry + 74.0), vec2(84.0, 26.0)), &b, &st.button);
            } else {
                if m.target {
                    label(&p, st, pos2(rx, ry + 4.0), Align2::LEFT_CENTER, if v.flash && v.blink { "Spotted!" } else { "Hunting" }, 9.0, if v.flash { GOLD } else { soft });
                    let name = v.title.clone().unwrap_or_else(|| "Pick a target ⚙".into());
                    let f = fit(&p, &name, st.label, 18.0, 12.0, x1 - rx);
                    let nr = p.text(pos2(rx, ry + 21.0), Align2::LEFT_CENTER, clip_text(&p, &name, &f, x1 - rx), f, if v.title.is_some() { text } else { soft });
                    if v.multi {
                        tip(ui, nr, "pl-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                    }
                    ry += 34.0;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let size = if v.compact { 36.0 } else if m.target { 38.0 } else { 46.0 };
                    let f = fit(&p, &num, st.num, size, 20.0, x1 - rx);
                    let nr = p.text(pos2(rx - 1.0, ry + f.size * 0.55), Align2::LEFT_CENTER, &num, f.clone(), text);
                    let mut line_txt = v.counter_label.to_lowercase();
                    if let Some((t, _, _)) = &v.timer {
                        if !v.compact {
                            line_txt.push_str(&format!(" · {t} today"));
                        }
                    }
                    let lr = p.text(pos2(rx, nr.max.y + 6.0), Align2::LEFT_CENTER, &line_txt, (st.body)(11.0), if super::overlay_mod::plain_counter(v.counter_label) { soft } else { (st.tone)(v.counter_col) });
                    if let Some((_, _, tp)) = &v.timer {
                        tip(ui, lr, "pl-timer", tp);
                    }
                }
                if let Some((t, dry, tp)) = &v.watch {
                    let wr = p.text(pos2(x1, y + 12.0), Align2::RIGHT_CENTER, t.to_lowercase(), (st.body)(10.0), if *dry { GOLD } else { soft });
                    tip(ui, wr, "pl-watch", tp);
                }
            }
            if let Some((el, msg)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let area = Rect::from_min_max(pos2(x0 - 6.0, y), pos2(x1 + 6.0, y + hero_h));
                confetti(&p, area, *el, &[GOLD, acc, st.accent2], a * 0.8);
                let br = Rect::from_center_size(area.center(), vec2(area.width(), 54.0));
                let rec = Rec::default();
                (st.panel)(&rec, br, 1.0, false);
                p.add(rec.shape());
                p.text(br.center() - vec2(0.0, 7.0), Align2::CENTER_CENTER, msg, (st.num)(19.0), fade(GOLD, a));
                label(&p, st, br.center() + vec2(0.0, 15.0), Align2::CENTER_CENTER, "Milestone", 9.0, fade(text, 0.7 * a));
            }
            y += hero_h + 10.0;
        }

        // ── Odds: the multiple and the theme's gauge
        if m.odds_bar {
            let col = (st.tone)(v.odds.color);
            let top_y = y;
            let mr = p.text(pos2(x0, y + 11.0), Align2::LEFT_CENTER, v.mult_text(), (st.num)(22.0), text);
            p.text(pos2(mr.max.x + 6.0, y + 15.0), Align2::LEFT_CENTER, "the odds", (st.body)(11.0), soft);
            label(&p, st, pos2(x1, y + 6.0), Align2::RIGHT_CENTER, v.odds_word(), 9.0, col);
            p.text(pos2(x1, y + 20.0), Align2::RIGHT_CENTER, &v.odds.right, (st.body)(10.5), soft);
            y += 32.0;
            (st.gauge)(&p, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 10.0)), v.odds.frac, col);
            y += 18.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "pl-odds", &v.odds.tip);
        }
        if m.method_line {
            let f = (st.body)(10.5);
            p.text(pos2(x0, y + 5.0), Align2::LEFT_CENTER, clip_text(&p, &v.method, &f, x1 - x0), f, soft);
            y += 18.0;
        }

        // ── Numbers in a row, split by the theme's dividers
        let divider = look_for(self.cfg.overlay.theme).divider;
        let mut first = true;
        let mut metrics = |ui: &mut egui::Ui, y: &mut f32, cells: Vec<(&str, String, Color32, String)>| {
            if first {
                divider(&p, pos2(x0, *y), pos2(x1, *y));
                *y += 8.0;
                first = false;
            }
            let n = cells.len().max(1) as f32;
            let cw = (x1 - x0) / n;
            for (i, (lab, value, col, t)) in cells.into_iter().enumerate() {
                let cxl = x0 + i as f32 * cw + if i > 0 { 12.0 } else { 0.0 };
                if i > 0 {
                    divider(&p, pos2(x0 + i as f32 * cw, *y + 2.0), pos2(x0 + i as f32 * cw, *y + 32.0));
                }
                label(&p, st, pos2(cxl, *y + 7.0), Align2::LEFT_CENTER, lab, 8.5, soft);
                let f = (st.body)(14.0);
                let room = cw - if i > 0 { 16.0 } else { 4.0 };
                p.text(pos2(cxl, *y + 24.0), Align2::LEFT_CENTER, clip_text(&p, &value, &f, room), f, (st.tone)(col));
                tip(ui, Rect::from_min_size(pos2(x0 + i as f32 * cw, *y), vec2(cw, 34.0)), ("pl-m", lab.to_string()), &t);
            }
            *y += 40.0;
        };
        let mut pair: Vec<(&str, String, Color32, String)> = Vec::new();
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => pair.push(("Luck", format!("{pct:.0}% would have it"), *col, t.clone())),
                None => pair.push(("Luck", "after 1st battle".into(), theme::MUTED(), "Needs the odds first (start a battle).".into())),
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            pair.push(("Next", format!("{what} {when}"), text, t));
        }
        if !pair.is_empty() {
            metrics(ui, &mut y, pair);
        }
        if m.session {
            metrics(
                ui,
                &mut y,
                vec![
                    ("Session", theme::thousands(v.session.0), text, "Encounters this session".into()),
                    ("Time", v.session_time(), text, "Hunting time this session".into()),
                    ("Pace", v.session_rate(), text, "Encounters per hour this session".into()),
                ],
            );
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("{} · {a}", v.last.0),
                None => v.last.0.clone(),
            };
            metrics(ui, &mut y, vec![("Last battle", txt, text, String::new())]);
        }
        if m.sparkline {
            label(&p, st, pos2(x0, y + 5.0), Align2::LEFT_CENTER, "Pace · 20 min", 8.5, soft);
            p.text(pos2(x1, y + 5.0), Align2::RIGHT_CENTER, format!("{}/h", theme::thousands(v.pace_hr)), (st.body)(11.0), text);
            let g = Rect::from_min_max(pos2(x0, y + 14.0), pos2(x1, y + 38.0));
            sparkline(&p, g, &v.pace, acc, fade(acc, 0.4));
            tip(ui, g, "pl-pace", "Encounters each minute for the last 20 minutes.");
            y += 46.0;
        }

        // ── Chips
        if !v.chips.is_empty() {
            let mut x = x0;
            for (t, col, _f, tp) in &v.chips {
                let f = (st.label)(10.0);
                let cw = text_w(&p, t, f.clone()) + 18.0;
                if x + cw > x1 && x > x0 {
                    x = x0;
                    y += 25.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 20.0));
                (st.chip)(&p, r, if *col == theme::MUTED() { soft } else { *col });
                p.text(r.center(), Align2::CENTER_CENTER, t, f, (st.tone)(*col));
                tip(ui, r, ("pl-chip", t.clone()), tp);
                x += cw + 6.0;
            }
            y += 29.0;
        }

        // ── XP and the daily goal
        let bar = |ui: &mut egui::Ui, y: &mut f32, lab: String, right: String, frac: f32, col: Color32, t: &str| {
            label(&p, st, pos2(x0, *y + 5.0), Align2::LEFT_CENTER, &lab, 8.5, col);
            p.text(pos2(x1, *y + 5.0), Align2::RIGHT_CENTER, right, (st.body)(10.5), soft);
            (st.gauge)(&p, Rect::from_min_max(pos2(x0, *y + 13.0), pos2(x1, *y + 21.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 22.0)), ("pl-bar", lab.clone()), t);
            *y += 30.0;
        };
        if m.xp {
            bar(ui, &mut y, format!("Trainer Lv {}", v.st.level.max(1)), format!("{} / {} XP", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, (st.tone)(Color32::from_rgb(120, 190, 255)), "Trainer level progress");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            bar(ui, &mut y, if done { "Daily goal ✔".into() } else { "Daily goal".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { GOLD } else { (st.tone)(theme::GREEN) }, "Encounters today across every run");
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 32.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += p.text(pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q, (st.body)(12.0), text).width() + 10.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h + 4.0, h)), b, &st.button);
                    x += h + 10.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x + 4.0, y), pos2(x1, y + h)), &acts.btns[3], &st.button);
            } else {
                let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                let gap = 8.0;
                let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
                for b in &acts.btns {
                    let bw = room * b.weight / total;
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &st.button);
                    x += bw + gap;
                }
            }
            y += h + 16.0;
        } else {
            y += 8.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, st.panel);
        let rec = Rec::default();
        (st.decor)(&rec, full, time);
        p.set(deco, rec.shape());
        let mut shapes = vec![[full.min.x, full.min.y, full.width(), full.height(), st.radius]];
        if let Some(o) = st.shadow {
            shapes.push([full.min.x + o.x, full.min.y + o.y, full.width(), full.height(), st.radius]);
        }
        self.ov_finish(ui, ctx, v, full, shapes, full.max.y + off.y, acc);
        if st.animated && theme::motion() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
}

fn no_decor(_: &dyn Sink, _: Rect, _: f32) {}

fn ink_divider(s: &dyn Sink, a: Pos2, b: Pos2) {
    line(s, a, b, Stroke::new(2.0f32, fade(Color32::BLACK, 0.35)));
}

fn soft_divider(s: &dyn Sink, a: Pos2, b: Pos2) {
    line(s, a, b, Stroke::new(1.0f32, fade(Color32::WHITE, 0.14)));
}

fn dark_divider(s: &dyn Sink, a: Pos2, b: Pos2) {
    line(s, a, b, Stroke::new(1.0f32, fade(Color32::BLACK, 0.14)));
}

// ───────────────────────────── pop-up cards, previews ─────────────────────────────

fn g_toast_card(st: &Style, p: &Painter, r: Rect, tint: Color32) {
    (st.panel)(p, r, 1.0, false);
    let strip = Rect::from_min_size(r.min + vec2(0.0, 6.0), vec2(4.0, (r.height() - 12.0).max(0.0)));
    rrect(p, strip, 2.0f32.min(st.radius), tint);
}

fn g_toast_body(st: &Style, p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let ic = pos2(r.min.x + 32.0, r.center().y);
    (st.stage)(p, ic, 17.0, v.tint());
    super::ov_toast::badge(p, ic, 12.0, v);
    let tx = r.min.x + 62.0;
    let room = r.max.x - tx - 12.0;
    let cap = if st.caps { v.caption.to_uppercase() } else { v.caption.clone() };
    p.text(pos2(tx, r.min.y + 14.0), Align2::LEFT_CENTER, cap, (st.label)(9.0), v.a(mix(v.tint(), st.text, 0.35)));
    let f = fit(p, &v.title, st.num, 18.0, 12.0, room);
    let title = clip_text(p, &v.title, &f, room);
    p.text(pos2(tx - 1.0, r.min.y + 32.0), Align2::LEFT_CENTER, title, f, v.a(st.text));
    let lf = (st.body)(10.5);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, r.min.y + 49.0), Align2::LEFT_CENTER, line_s, lf, v.a(st.muted));
}

fn g_preview(st: &Style, p: &Painter, r: Rect) {
    let card = Rect::from_center_size(r.center(), vec2(r.width() * 0.62, r.height() * 0.84));
    if let Some(o) = st.shadow {
        rrect(p, card.translate(o * 0.5), st.radius.min(8.0), Color32::BLACK);
    }
    (st.panel)(p, card, 1.0, false);
    (st.decor)(p, card, 0.0);
    (st.stage)(p, card.min + vec2(20.0, 26.0), 13.0, st.accent);
    p.text(card.min + vec2(38.0, 26.0), Align2::LEFT_CENTER, "42", (st.num)(15.0), st.text);
    (st.gauge)(p, Rect::from_min_size(card.min + vec2(7.0, 46.0), vec2(card.width() - 14.0, 5.0)), 0.6, st.accent);
    (st.button)(p, Rect::from_min_size(card.min + vec2(card.width() - 32.0, card.height() - 17.0), vec2(26.0, 12.0)), "", Kind::Main, false, false);
}

/// Each theme's pop-up card skin and preview (fn pointers need a fn per theme).
macro_rules! plus_fns {
    ($m:ident) => {
        pub(super) fn toast_card(p: &Painter, r: Rect, tint: Color32) {
            super::g_toast_card(&style(), p, r, tint)
        }
        pub(super) fn toast_body(p: &Painter, r: Rect, v: &super::super::ov_toast::ToastView) {
            super::g_toast_body(&style(), p, r, v)
        }
        pub(super) fn toast_skin() -> super::super::ov_toast::ToastSkin {
            let st = style();
            super::super::ov_toast::ToastSkin { radius: st.radius.min(16.0), card: toast_card, inner: |r| r.shrink2(vec2(8.0, 4.0)), toast: toast_body, title_font: (st.label)(13.0), body_font: (st.body)(10.5) }
        }
        pub(super) fn preview(p: &Painter, r: Rect) {
            super::g_preview(&style(), p, r)
        }
    };
}

pub(super) fn toast_skin_for(t: OverlayTheme) -> Option<super::ov_toast::ToastSkin> {
    use OverlayTheme as O;
    Some(match t {
        O::Studio => studio::toast_skin(),
        O::Pokedex => dex::toast_skin(),
        O::Aurora => aurora::toast_skin(),
        O::FieldGuide => field::toast_skin(),
        O::Synthwave => synth::toast_skin(),
        O::Gold => gold::toast_skin(),
        O::Companion => companion::toast_skin(),
        O::Pocket => pocket::toast_skin(),
        O::Candy => candy::toast_skin(),
        O::Brutal => brutal::toast_skin(),
        O::Liquid => liquid::toast_skin(),
        O::Winter => winter::toast_skin(),
        O::Spooky => spooky::toast_skin(),
        _ => return None,
    })
}

pub(super) fn preview_for(p: &Painter, r: Rect, t: OverlayTheme) {
    use OverlayTheme as O;
    match t {
        O::Studio => studio::preview(p, r),
        O::Pokedex => dex::preview(p, r),
        O::Aurora => aurora::preview(p, r),
        O::FieldGuide => field::preview(p, r),
        O::Synthwave => synth::preview(p, r),
        O::Gold => gold::preview(p, r),
        O::Companion => companion::preview(p, r),
        O::Pocket => pocket::preview(p, r),
        O::Candy => candy::preview(p, r),
        O::Brutal => brutal::preview(p, r),
        O::Liquid => liquid::preview(p, r),
        O::Winter => winter::preview(p, r),
        O::Spooky => spooky::preview(p, r),
        _ => {}
    }
}

// ───────────────────────────── shared bits ─────────────────────────────

/// A button's label or icon, centred, in `col`.
fn btn_label(p: &Painter, r: Rect, label: &str, font: FontId, col: Color32) {
    if label == "⏸" {
        for dx in [-3.0f32, 3.0] {
            p.rect_filled(Rect::from_center_size(r.center() + vec2(dx, 0.0), vec2(2.5, 11.0)), 1.0, col);
        }
    } else if label == "▶" {
        let c = r.center();
        poly(p, vec![c + vec2(-3.5, -6.0), c + vec2(6.0, 0.0), c + vec2(-3.5, 6.0)], col);
    } else if !label.is_empty() {
        p.text(r.center(), Align2::CENTER_CENTER, label, font, col);
    }
}

/// The fill for a button kind: the main colour, green, gold, or plain.
fn kind_fill(kind: Kind, main: Color32, plain: Color32) -> Color32 {
    match kind {
        Kind::Main => main,
        Kind::Good => theme::GREEN,
        Kind::Gold => GOLD,
        Kind::Plain => plain,
    }
}

/// Readable text on a fill: white or near-black.
fn on(fill: Color32) -> Color32 {
    if theme::contrast(Color32::WHITE, fill) >= theme::contrast(Color32::from_rgb(24, 22, 30), fill) {
        Color32::WHITE
    } else {
        Color32::from_rgb(24, 22, 30)
    }
}

/// A simple filled gauge: track, fill, in `rad` corners.
fn simple_gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32, track: Color32, rad: f32) {
    rrect(s, r, rad, track);
    let w = r.width() * frac.clamp(0.0, 1.0);
    if w > 0.5 {
        rrect(s, Rect::from_min_size(r.min, vec2(w.max(rad * 2.0).min(r.width()), r.height())), rad, col);
    }
}

fn hash01(i: u32, k: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x % 10_000) as f32 / 10_000.0
}

fn keep(c: Color32) -> Color32 {
    c
}

fn light_tone(c: Color32) -> Color32 {
    // status colours on light panels: a touch darker
    if c == theme::MUTED() {
        Color32::from_rgb(100, 100, 112)
    } else {
        mix(c, Color32::BLACK, 0.25)
    }
}

fn dark_tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        fade(Color32::WHITE, 0.6)
    } else {
        mix(c, Color32::WHITE, 0.15)
    }
}

// ───────────────────────────── the themes ─────────────────────────────

/// Studio: deep slate, a violet-to-orange edge, soft gradient pills.
mod studio {
    use super::*;
    const V: Color32 = Color32::from_rgb(139, 92, 246);
    const O: Color32 = Color32::from_rgb(249, 115, 22);
    pub(super) fn style() -> Style {
        Style { text: Color32::WHITE, muted: Color32::from_rgb(176, 172, 196), accent: V, accent2: O, radius: 18.0, num: crate::fonts::bricolage_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: false, panel, button, gauge, stage, decor: no_decor, chip, tone: dark_tone, shadow: None, animated: false }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.5 + 0.5 * op;
        vgrad(s, &rrect_pts(r, 18.0), rgba(36, 32, 52, a), rgba(20, 18, 30, a));
        hgrad(s, &[r.min + vec2(18.0, 0.0), pos2(r.max.x - 18.0, r.min.y), pos2(r.max.x - 18.0, r.min.y + 2.5), r.min + vec2(18.0, 2.5)], V, O);
        rstroke(s, r.shrink(0.5), 18.0, Stroke::new(1.0f32, if alert { GOLD } else { fade(Color32::WHITE, 0.12) }));
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = 10.0f32.min(r.height() / 2.0);
        if kind == Kind::Main {
            hgrad(p, &rrect_pts(r, rad), mix(V, Color32::WHITE, if hov { 0.15 } else { 0.0 }), mix(O, Color32::WHITE, if hov { 0.15 } else { 0.0 }));
            btn_label(p, r, label, crate::fonts::sans_bold_font(13.5), Color32::WHITE);
        } else {
            let fill = kind_fill(kind, V, fade(Color32::WHITE, if down { 0.16 } else if hov { 0.11 } else { 0.06 }));
            rrect(p, r, rad, fill);
            rstroke(p, r, rad, Stroke::new(1.0f32, fade(Color32::WHITE, 0.18)));
            btn_label(p, r, label, crate::fonts::sans_bold_font(12.5), if kind == Kind::Plain { Color32::WHITE } else { on(fill) });
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        let rad = r.height() / 2.0;
        rrect(s, r, rad, fade(Color32::WHITE, 0.08));
        let w = (r.width() * frac.clamp(0.0, 1.0)).max(r.height());
        hgrad(s, &rrect_pts(Rect::from_min_size(r.min, vec2(w.min(r.width()), r.height())), rad), col, mix(col, O, 0.5));
    }
    fn stage(p: &Painter, c: Pos2, r: f32, acc: Color32) {
        for (k, a) in [(1.0, 0.08), (0.8, 0.1), (0.6, 0.14)] {
            circle(p, c, r * k, fade(acc, a));
        }
        circle(p, c + vec2(r * 0.35, r * 0.3), r * 0.5, fade(O, 0.12));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 10.0, fade(col, 0.16));
    }
    plus_fns!(studio);
}

/// Pokédex: the red device, a black hinge, an LCD screen for the sprite,
/// pixel type and segment gauges.
mod dex {
    use super::*;
    const RED: Color32 = Color32::from_rgb(214, 36, 40);
    const INK: Color32 = Color32::from_rgb(28, 28, 32);
    const YEL: Color32 = Color32::from_rgb(250, 204, 21);
    pub(super) fn style() -> Style {
        Style { text: Color32::WHITE, muted: Color32::from_rgb(255, 214, 210), accent: YEL, accent2: Color32::from_rgb(90, 170, 255), radius: 0.0, num: crate::fonts::pixel_ui_font, label: crate::fonts::pixel_ui_font, body: crate::fonts::sans_bold_font, caps: true, panel, button, gauge, stage, decor, chip, tone, shadow: None, animated: false }
    }
    fn tone(c: Color32) -> Color32 {
        if c == theme::MUTED() {
            Color32::from_rgb(255, 214, 210)
        } else {
            mix(c, Color32::WHITE, 0.5)
        }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.6 + 0.4 * op;
        vgrad(s, &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], fade(RED, a), fade(mix(RED, Color32::BLACK, 0.25), a));
        rstroke(s, r.shrink(1.5), 0.0, Stroke::new(3.0f32, if alert { YEL } else { INK }));
    }
    fn decor(s: &dyn Sink, r: Rect, _t: f32) {
        // the hinge under the top line, and screws in the corners
        let y = r.min.y + 32.0;
        rrect(s, Rect::from_min_max(pos2(r.min.x + 3.0, y), pos2(r.max.x - 3.0, y + 4.0)), 0.0, INK);
        for c in [r.left_bottom() + vec2(9.0, -9.0), r.right_bottom() + vec2(-9.0, -9.0)] {
            rrect(s, Rect::from_center_size(c, vec2(5.0, 5.0)), 0.0, mix(RED, Color32::BLACK, 0.45));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let fill = kind_fill(kind, YEL, Color32::from_rgb(52, 52, 58));
        let fill = if hov { mix(fill, Color32::WHITE, 0.12) } else { fill };
        let r = if down { r.translate(vec2(0.0, 1.0)) } else { r };
        rrect(p, r.translate(vec2(0.0, 3.0)), 0.0, INK);
        rrect(p, r, 0.0, fill);
        rrect(p, Rect::from_min_size(r.min, vec2(r.width(), 2.0)), 0.0, mix(fill, Color32::WHITE, 0.3));
        btn_label(p, r, label, crate::fonts::pixel_ui_font(12.5), on(fill));
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r.expand(2.0), 0.0, INK);
        let n = 20;
        let w = r.width() / n as f32;
        for i in 0..n {
            let lit = (i as f32 + 0.5) / n as f32 <= frac;
            rrect(s, Rect::from_min_size(pos2(r.min.x + i as f32 * w + 1.0, r.min.y), vec2(w - 2.0, r.height())), 0.0, if lit { mix(col, Color32::WHITE, 0.2) } else { Color32::from_rgb(70, 20, 22) });
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        // a little screen in a black bezel, its corner cut
        let sr = Rect::from_center_size(c, vec2(r * 2.0, r * 2.0));
        rrect(p, sr.expand(4.0), 0.0, INK);
        poly(p, vec![sr.left_top(), sr.right_top(), sr.right_bottom(), pos2(sr.min.x + r * 0.35, sr.max.y), pos2(sr.min.x, sr.max.y - r * 0.35)], Color32::from_rgb(196, 214, 196));
        let mut y = sr.min.y + 3.0;
        while y < sr.max.y {
            line(p, pos2(sr.min.x, y), pos2(sr.max.x, y), Stroke::new(1.0f32, fade(Color32::BLACK, 0.05)));
            y += 3.0;
        }
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 0.0, fade(INK, 0.45));
        rstroke(p, r, 0.0, Stroke::new(1.5f32, mix(col, Color32::WHITE, 0.4)));
    }
    plus_fns!(dex);
}

/// Aurora glass: dark glass with aurora ribbons rippling over the top.
mod aurora {
    use super::*;
    const T: Color32 = Color32::from_rgb(94, 234, 212);
    const P: Color32 = Color32::from_rgb(167, 139, 250);
    pub(super) fn style() -> Style {
        Style { text: Color32::WHITE, muted: Color32::from_rgb(176, 198, 214), accent: T, accent2: P, radius: 22.0, num: crate::fonts::jura_font, label: crate::fonts::jura_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: dark_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.3 + 0.6 * op;
        vgrad(s, &rrect_pts(r, 22.0), rgba(14, 24, 44, a), rgba(6, 10, 22, a));
        rstroke(s, r.shrink(0.5), 22.0, Stroke::new(1.0f32, if alert { GOLD } else { fade(Color32::WHITE, 0.2) }));
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        let tt = if theme::motion() { t } else { 0.0 };
        for (j, col) in [T, P, Color32::from_rgb(125, 211, 252)].iter().enumerate() {
            let mut prev: Option<Pos2> = None;
            let mut x = r.min.x + 22.0;
            while x <= r.max.x - 22.0 {
                let y = r.min.y + 6.0 + j as f32 * 5.0 + ((x * 0.03) + tt * 1.2 + j as f32).sin() * 4.0;
                let q = pos2(x, y);
                if let Some(a) = prev {
                    line(s, a, q, Stroke::new(6.0f32, fade(*col, 0.08)));
                    line(s, a, q, Stroke::new(1.5f32, fade(*col, 0.45)));
                }
                prev = Some(q);
                x += 8.0;
            }
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = r.height() / 2.0;
        if kind == Kind::Main {
            hgrad(p, &rrect_pts(r, rad), mix(T, Color32::WHITE, if hov { 0.2 } else { 0.0 }), mix(P, Color32::WHITE, if hov { 0.2 } else { 0.0 }));
            btn_label(p, r, label, crate::fonts::sans_bold_font(13.5), Color32::from_rgb(10, 20, 36));
        } else {
            let fill = kind_fill(kind, T, fade(Color32::WHITE, if down { 0.18 } else if hov { 0.12 } else { 0.05 }));
            rrect(p, r, rad, fill);
            rstroke(p, r, rad, Stroke::new(1.0f32, fade(Color32::WHITE, if hov { 0.5 } else { 0.28 })));
            btn_label(p, r, label, crate::fonts::sans_bold_font(12.5), if kind == Kind::Plain { Color32::WHITE } else { on(fill) });
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        let y = r.center().y;
        line(s, pos2(r.min.x, y), pos2(r.max.x, y), Stroke::new(3.0f32, fade(Color32::WHITE, 0.12)));
        let x = r.min.x + r.width() * frac.clamp(0.0, 1.0);
        if x > r.min.x + 1.0 {
            hgrad(s, &[pos2(r.min.x, y - 1.5), pos2(x, y - 1.5), pos2(x, y + 1.5), pos2(r.min.x, y + 1.5)], fade(T, 0.6), col);
        }
        for (k, a) in [(3.0, 0.1), (2.0, 0.2), (1.0, 1.0)] {
            circle(s, pos2(x.max(r.min.x + 2.0), y), 3.0 * k, fade(col, a));
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, acc: Color32) {
        for (k, a) in [(1.0, 0.05), (0.8, 0.08), (0.6, 0.12)] {
            circle(p, c, r * k, fade(acc, a));
        }
        p.circle_stroke(c, r, Stroke::new(1.0f32, fade(P, 0.4)));
        p.circle_stroke(c, r * 0.8, Stroke::new(1.0f32, fade(T, 0.3)));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, r.height() / 2.0, fade(col, 0.12));
        rstroke(p, r, r.height() / 2.0, Stroke::new(1.0f32, fade(col, 0.45)));
    }
    plus_fns!(aurora);
}

/// Field guide: a printed guide page: cream paper, a ruled header, a
/// specimen stamp for the sprite, ruler gauges and square buttons.
mod field {
    use super::*;
    const PAPER: Color32 = Color32::from_rgb(247, 242, 228);
    const GREEN: Color32 = Color32::from_rgb(15, 108, 100);
    const RUST: Color32 = Color32::from_rgb(194, 65, 12);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(28, 40, 32), muted: Color32::from_rgb(92, 102, 92), accent: GREEN, accent2: RUST, radius: 0.0, num: crate::fonts::young_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: light_tone, shadow: None, animated: false }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        rrect(s, r, 0.0, fade(PAPER, 0.75 + 0.25 * op));
        rstroke(s, r.shrink(1.0), 0.0, Stroke::new(2.0f32, if alert { GOLD } else { GREEN }));
        rstroke(s, r.shrink(4.0), 0.0, Stroke::new(1.0f32, fade(GREEN, 0.45)));
    }
    fn decor(s: &dyn Sink, r: Rect, _t: f32) {
        // a double rule under the top line, registration marks in the corners
        let y = r.min.y + 32.0;
        line(s, pos2(r.min.x + 10.0, y), pos2(r.max.x - 10.0, y), Stroke::new(1.5f32, GREEN));
        line(s, pos2(r.min.x + 10.0, y + 3.0), pos2(r.max.x - 10.0, y + 3.0), Stroke::new(0.8f32, GREEN));
        for (c, dx, dy) in [(r.left_bottom(), 1.0f32, -1.0f32), (r.right_bottom(), -1.0, -1.0)] {
            let q = c + vec2(dx * 9.0, dy * 9.0);
            line(s, q, q + vec2(dx * 8.0, 0.0), Stroke::new(1.0f32, RUST));
            line(s, q, q + vec2(0.0, dy * 8.0), Stroke::new(1.0f32, RUST));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let r = if down { r.translate(vec2(0.0, 1.0)) } else { r };
        if kind == Kind::Plain {
            rrect(p, r, 0.0, if hov { mix(PAPER, GREEN, 0.12) } else { PAPER });
            rstroke(p, r, 0.0, Stroke::new(1.5f32, GREEN));
            btn_label(p, r, label, crate::fonts::sans_bold_font(12.5), GREEN);
        } else {
            let fill = kind_fill(kind, GREEN, PAPER);
            let fill = if hov { mix(fill, Color32::WHITE, 0.12) } else { fill };
            rrect(p, r, 0.0, fill);
            rstroke(p, r.shrink(3.0), 0.0, Stroke::new(1.0f32, fade(Color32::WHITE, 0.45)));
            btn_label(p, r, label, crate::fonts::sans_bold_font(13.0), on(fill));
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r, 0.0, fade(GREEN, 0.08));
        rrect(s, Rect::from_min_size(r.min, vec2(r.width() * frac.clamp(0.0, 1.0), r.height())), 0.0, col);
        rstroke(s, r, 0.0, Stroke::new(1.0f32, GREEN));
        let mut x = r.min.x;
        let mut k = 0;
        while x <= r.max.x {
            let h = if k % 5 == 0 { 4.0 } else { 2.0 };
            line(s, pos2(x, r.min.y - h), pos2(x, r.min.y), Stroke::new(1.0f32, fade(GREEN, 0.7)));
            x += r.width() / 20.0;
            k += 1;
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        // a specimen stamp: a dashed ring, compass ticks
        circle(p, c, r * 0.92, Color32::from_rgb(236, 229, 206));
        let n = 36;
        for i in 0..n {
            if i % 2 == 0 {
                let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
                let a1 = (i as f32 + 1.0) / n as f32 * std::f32::consts::TAU;
                line(p, c + vec2(a0.cos(), a0.sin()) * r, c + vec2(a1.cos(), a1.sin()) * r, Stroke::new(1.5f32, GREEN));
            }
        }
        for k in 0..4 {
            let a = k as f32 * std::f32::consts::FRAC_PI_2;
            let d = vec2(a.cos(), a.sin());
            line(p, c + d * (r - 6.0), c + d * (r + 3.0), Stroke::new(1.5f32, RUST));
        }
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 0.0, fade(col, 0.08));
        rstroke(p, r, 0.0, Stroke::new(1.2f32, col));
    }
    plus_fns!(field);
}

/// Synthwave: a dusk sky, a neon grid rolling in at the bottom, a striped
/// sun behind the sprite, neon tube buttons.
mod synth {
    use super::*;
    const C: Color32 = Color32::from_rgb(34, 211, 238);
    const PK: Color32 = Color32::from_rgb(244, 114, 182);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(240, 248, 255), muted: Color32::from_rgb(206, 176, 232), accent: C, accent2: PK, radius: 6.0, num: crate::fonts::tektur_font, label: crate::fonts::tektur_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: dark_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.55 + 0.45 * op;
        vgrad(s, &rrect_pts(r, 6.0), rgba(18, 6, 38, a), rgba(64, 12, 72, a));
        rstroke(s, r.expand(1.5), 7.5, Stroke::new(3.0f32, fade(PK, 0.18)));
        rstroke(s, r.shrink(0.5), 6.0, Stroke::new(1.5f32, if alert { GOLD } else { PK }));
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        // the grid along the bottom, rolling towards you
        let h = 34.0f32.min(r.height() * 0.25);
        let horizon = r.max.y - h;
        line(s, pos2(r.min.x + 4.0, horizon), pos2(r.max.x - 4.0, horizon), Stroke::new(1.0f32, fade(PK, 0.6)));
        let ph = if theme::motion() { (t * 0.8).fract() } else { 0.0 };
        for k in 0..6 {
            let u = (k as f32 + ph) / 6.0;
            let y = horizon + h * u * u;
            if y < r.max.y - 2.0 {
                line(s, pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y), Stroke::new(1.0f32, fade(PK, 0.15 + 0.3 * u)));
            }
        }
        let cx = r.center().x;
        for i in -8..=8 {
            let xb = cx + i as f32 * r.width() / 9.0;
            let xt = cx + i as f32 * r.width() / 40.0;
            line(s, pos2(xt, horizon), pos2(xb.clamp(r.min.x + 4.0, r.max.x - 4.0), r.max.y - 2.0), Stroke::new(1.0f32, fade(PK, 0.25)));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let col = match kind {
            Kind::Main => PK,
            Kind::Good => theme::GREEN,
            Kind::Gold => GOLD,
            Kind::Plain => C,
        };
        rrect(p, r.expand(2.0), 8.0, fade(col, if hov { 0.25 } else { 0.12 }));
        rrect(p, r, 6.0, fade(col, if down { 0.4 } else if kind == Kind::Main { 0.28 } else { 0.1 }));
        rstroke(p, r, 6.0, Stroke::new(1.5f32, mix(col, Color32::WHITE, 0.2)));
        btn_label(p, r, label, crate::fonts::tektur_font(13.0), Color32::WHITE);
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r, 3.0, fade(PK, 0.15));
        let w = r.width() * frac.clamp(0.0, 1.0);
        if w > 1.0 {
            let f = Rect::from_min_size(r.min, vec2(w, r.height()));
            rrect(s, f.expand(2.0), 5.0, fade(col, 0.2));
            hgrad(s, &rrect_pts(f, 3.0), C, mix(PK, col, 0.3));
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        // a striped sun
        let rr = r * 0.95;
        let mut y = c.y - rr;
        while y < c.y + rr {
            let dy = (y - c.y) / rr;
            let half = rr * (1.0 - dy * dy).max(0.0).sqrt();
            let gapped = dy > 0.1 && ((dy * 7.0).fract() < 0.3);
            if !gapped {
                line(p, pos2(c.x - half, y), pos2(c.x + half, y), Stroke::new(2.0f32, mix(Color32::from_rgb(250, 204, 21), PK, (dy + 1.0) / 2.0)));
            }
            y += 2.0;
        }
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 3.0, fade(col, 0.12));
        rstroke(p, r, 3.0, Stroke::new(1.0f32, col));
    }
    plus_fns!(synth);
}

/// Gold: black with a double gold frame and corner filigree, gold type
/// and a medallion behind the sprite; glints twinkle.
mod gold {
    use super::*;
    const G: Color32 = Color32::from_rgb(242, 193, 78);
    const PALE: Color32 = Color32::from_rgb(255, 240, 179);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(250, 240, 215), muted: Color32::from_rgb(196, 176, 128), accent: G, accent2: PALE, radius: 6.0, num: crate::fonts::gloock_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: dark_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.6 + 0.4 * op;
        vgrad(s, &rrect_pts(r, 6.0), rgba(22, 19, 14, a), rgba(10, 9, 7, a));
        rstroke(s, r.shrink(1.0), 6.0, Stroke::new(1.5f32, if alert { PALE } else { G }));
        rstroke(s, r.shrink(5.0), 3.0, Stroke::new(0.8f32, fade(G, 0.55)));
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        // little diamonds on the corners, and a few glints
        for c in [r.left_top() + vec2(5.0, 5.0), r.right_top() + vec2(-5.0, 5.0), r.left_bottom() + vec2(5.0, -5.0), r.right_bottom() + vec2(-5.0, -5.0)] {
            poly(s, vec![c + vec2(0.0, -4.0), c + vec2(4.0, 0.0), c + vec2(0.0, 4.0), c + vec2(-4.0, 0.0)], G);
        }
        let tt = if theme::motion() { t } else { 0.0 };
        for i in 0..5u32 {
            let k = ((tt * (0.8 + hash01(i, 1)) + hash01(i, 2) * 6.0).sin()).max(0.0).powi(8);
            if k > 0.05 {
                let c = pos2(r.min.x + r.width() * (0.1 + 0.8 * hash01(i, 3)), r.min.y + r.height() * (0.1 + 0.8 * hash01(i, 4)));
                let l = 4.0 * k + 1.0;
                line(s, c - vec2(l, 0.0), c + vec2(l, 0.0), Stroke::new(1.0f32, fade(PALE, k)));
                line(s, c - vec2(0.0, l), c + vec2(0.0, l), Stroke::new(1.0f32, fade(PALE, k)));
            }
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = 6.0f32.min(r.height() / 2.0);
        if kind == Kind::Plain {
            rrect(p, r, rad, fade(G, if down { 0.2 } else if hov { 0.12 } else { 0.04 }));
            rstroke(p, r, rad, Stroke::new(1.0f32, G));
            btn_label(p, r, label, crate::fonts::sans_bold_font(12.5), G);
        } else {
            let base = kind_fill(kind, G, G);
            vgrad(p, &rrect_pts(r, rad), mix(base, Color32::WHITE, if hov { 0.45 } else { 0.3 }), mix(base, Color32::BLACK, 0.2));
            rstroke(p, r, rad, Stroke::new(1.0f32, mix(base, Color32::BLACK, 0.45)));
            btn_label(p, r, label, crate::fonts::sans_bold_font(13.0), Color32::from_rgb(40, 28, 6));
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r, 3.0, Color32::from_rgb(42, 36, 22));
        let w = r.width() * frac.clamp(0.0, 1.0);
        if w > 1.0 {
            let f = Rect::from_min_size(r.min, vec2(w, r.height()));
            vgrad(s, &rrect_pts(f, 3.0), mix(col, Color32::WHITE, 0.35), mix(col, Color32::BLACK, 0.2));
            circle(s, pos2(f.max.x - 1.0, f.center().y), 2.0, PALE);
        }
        rstroke(s, r, 3.0, Stroke::new(0.8f32, fade(G, 0.5)));
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        // a medallion
        circle(p, c, r, fade(G, 0.18));
        p.circle_stroke(c, r, Stroke::new(3.0f32, G));
        p.circle_stroke(c, r - 5.0, Stroke::new(1.0f32, fade(PALE, 0.6)));
        circle(p, c, r - 6.0, Color32::from_rgb(20, 17, 12));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 4.0, fade(col, 0.1));
        rstroke(p, r, 4.0, Stroke::new(1.0f32, fade(G, 0.6)));
    }
    plus_fns!(gold);
}

/// Companion: soft pastel, round everything, little hearts.
mod companion {
    use super::*;
    const PK: Color32 = Color32::from_rgb(236, 110, 180);
    const LI: Color32 = Color32::from_rgb(139, 106, 216);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(64, 40, 84), muted: Color32::from_rgb(132, 110, 150), accent: PK, accent2: LI, radius: 24.0, num: crate::fonts::outfit_font, label: crate::fonts::outfit_font, body: crate::fonts::sans_font, caps: false, panel, button, gauge, stage, decor, chip, tone: light_tone, shadow: None, animated: false }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.75 + 0.25 * op;
        vgrad(s, &rrect_pts(r, 24.0), rgba(255, 244, 251, a), rgba(244, 232, 252, a));
        rstroke(s, r.shrink(1.0), 24.0, Stroke::new(2.0f32, if alert { GOLD } else { fade(PK, 0.45) }));
    }
    fn heart(s: &dyn Sink, c: Pos2, k: f32, col: Color32) {
        circle(s, c + vec2(-k * 0.5, -k * 0.2), k * 0.6, col);
        circle(s, c + vec2(k * 0.5, -k * 0.2), k * 0.6, col);
        poly(s, vec![c + vec2(-k * 1.05, 0.05 * k), c + vec2(k * 1.05, 0.05 * k), c + vec2(0.0, k * 1.15)], col);
    }
    fn decor(s: &dyn Sink, r: Rect, _t: f32) {
        heart(s, r.right_bottom() + vec2(-18.0, -16.0), 4.0, fade(PK, 0.5));
        heart(s, r.right_bottom() + vec2(-30.0, -24.0), 2.6, fade(LI, 0.45));
        heart(s, r.left_bottom() + vec2(16.0, -14.0), 3.0, fade(PK, 0.35));
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = r.height() / 2.0;
        if kind == Kind::Plain {
            rrect(p, r, rad, if down { mix(Color32::WHITE, PK, 0.2) } else if hov { mix(Color32::WHITE, PK, 0.1) } else { Color32::WHITE });
            rstroke(p, r, rad, Stroke::new(1.5f32, fade(PK, 0.5)));
            btn_label(p, r, label, crate::fonts::outfit_font(13.0), PK);
        } else {
            let fill = kind_fill(kind, PK, PK);
            let fill = if hov { mix(fill, Color32::WHITE, 0.12) } else { fill };
            rrect(p, r, rad, fill);
            circle(p, pos2(r.min.x + rad * 0.8, r.min.y + rad * 0.55), 2.5, fade(Color32::WHITE, 0.7));
            btn_label(p, r, label, crate::fonts::outfit_font(13.5), on(fill));
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        simple_gauge(s, r, frac, col, Color32::from_rgb(242, 222, 240), r.height() / 2.0);
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        circle(p, c, r, Color32::from_rgb(255, 230, 245));
        p.circle_stroke(c, r, Stroke::new(2.0f32, fade(LI, 0.5)));
        heart(p, c + vec2(r * 0.8, -r * 0.75), 5.0, PK);
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, r.height() / 2.0, fade(col, 0.16));
    }
    plus_fns!(companion);
}

/// Pocket: a Game Boy screen: four greens, a bezel, pixel type.
mod pocket {
    use super::*;
    const D0: Color32 = Color32::from_rgb(15, 56, 15);
    const D1: Color32 = Color32::from_rgb(48, 98, 48);
    const L1: Color32 = Color32::from_rgb(139, 172, 15);
    const L0: Color32 = Color32::from_rgb(155, 188, 15);
    pub(super) fn style() -> Style {
        Style { text: D0, muted: D1, accent: D0, accent2: D1, radius: 0.0, num: crate::fonts::pixel_ui_font, label: crate::fonts::pixel_ui_font, body: crate::fonts::pixel_ui_font, caps: true, panel, button, gauge, stage, decor, chip, tone, shadow: None, animated: false }
    }
    fn tone(c: Color32) -> Color32 {
        if c == theme::MUTED() {
            D1
        } else {
            D0
        }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        rrect(s, r, 0.0, fade(L0, 0.8 + 0.2 * op));
        rstroke(s, r.shrink(2.0), 0.0, Stroke::new(4.0f32, if alert { D0 } else { D1 }));
    }
    fn decor(s: &dyn Sink, r: Rect, _t: f32) {
        let mut y = r.min.y + 4.0;
        while y < r.max.y - 4.0 {
            line(s, pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y), Stroke::new(1.0f32, fade(L1, 0.35)));
            y += 3.0;
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let fill = if kind == Kind::Plain { D1 } else { D0 };
        let r = if down { r.translate(vec2(0.0, 1.0)) } else { r };
        rrect(p, r, 0.0, if hov { mix(fill, L1, 0.15) } else { fill });
        btn_label(p, r, label, crate::fonts::pixel_ui_font(12.5), L0);
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, _col: Color32) {
        rrect(s, r.expand(1.0), 0.0, D1);
        rrect(s, r, 0.0, L1);
        let n = 16;
        let w = r.width() / n as f32;
        for i in 0..n {
            if (i as f32 + 0.5) / n as f32 <= frac {
                rrect(s, Rect::from_min_size(pos2(r.min.x + i as f32 * w + 1.0, r.min.y + 1.0), vec2(w - 2.0, r.height() - 2.0)), 0.0, D0);
            }
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        let sr = Rect::from_center_size(c, vec2(r * 2.0, r * 2.0));
        rrect(p, sr.expand(3.0), 0.0, D1);
        rrect(p, sr, 0.0, L1);
    }
    fn chip(p: &Painter, r: Rect, _col: Color32) {
        rrect(p, r, 0.0, L1);
        rstroke(p, r, 0.0, Stroke::new(1.5f32, D1));
    }
    plus_fns!(pocket);
}

/// Candy: white, candy stripes along the bottom, sprinkles, jelly buttons
/// and a lollipop swirl behind the sprite.
mod candy {
    use super::*;
    const PK: Color32 = Color32::from_rgb(236, 72, 153);
    const VI: Color32 = Color32::from_rgb(139, 92, 246);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(92, 30, 72), muted: Color32::from_rgb(150, 92, 130), accent: PK, accent2: VI, radius: 20.0, num: crate::fonts::erica_font, label: crate::fonts::black_font, body: crate::fonts::sans_font, caps: false, panel, button, gauge, stage, decor, chip, tone: light_tone, shadow: None, animated: false }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        rrect(s, r, 20.0, fade(Color32::from_rgb(255, 250, 252), 0.8 + 0.2 * op));
        rstroke(s, r.shrink(1.0), 20.0, Stroke::new(2.5f32, if alert { GOLD } else { PK }));
    }
    fn decor(s: &dyn Sink, r: Rect, _t: f32) {
        // sprinkles
        let cols = [PK, VI, Color32::from_rgb(56, 189, 248), Color32::from_rgb(250, 204, 21), Color32::from_rgb(52, 211, 153)];
        for i in 0..14u32 {
            let c = pos2(r.min.x + 14.0 + (r.width() - 28.0) * hash01(i, 1), r.max.y - 10.0 - 26.0 * hash01(i, 2));
            let a = hash01(i, 3) * 3.0;
            let d = vec2(a.cos(), a.sin()) * 3.0;
            line(s, c - d, c + d, Stroke::new(2.0f32, cols[i as usize % cols.len()]));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = r.height() / 2.0;
        if kind == Kind::Plain {
            rrect(p, r, rad, if down { mix(Color32::WHITE, PK, 0.18) } else if hov { mix(Color32::WHITE, PK, 0.08) } else { Color32::WHITE });
            rstroke(p, r, rad, Stroke::new(2.0f32, PK));
            btn_label(p, r, label, crate::fonts::black_font(12.5), PK);
        } else {
            let fill = kind_fill(kind, PK, PK);
            hgrad(p, &rrect_pts(r, rad), mix(fill, Color32::WHITE, if hov { 0.15 } else { 0.0 }), mix(fill, VI, if kind == Kind::Main { 0.6 } else { 0.0 }));
            rrect(p, Rect::from_min_size(r.min + vec2(rad * 0.6, 2.5), vec2(r.width() - rad * 1.2, r.height() * 0.32)), r.height() * 0.16, fade(Color32::WHITE, 0.35));
            btn_label(p, r, label, crate::fonts::black_font(13.0), Color32::WHITE);
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        let rad = r.height() / 2.0;
        rrect(s, r, rad, Color32::from_rgb(252, 226, 240));
        let w = (r.width() * frac.clamp(0.0, 1.0)).max(r.height());
        let f = Rect::from_min_size(r.min, vec2(w.min(r.width()), r.height()));
        rrect(s, f, rad, col);
        let mut x = f.min.x - f.height();
        while x < f.max.x - rad {
            poly(s, vec![pos2(x.max(f.min.x + rad * 0.5), f.max.y), pos2((x + 3.0).max(f.min.x + rad * 0.5), f.max.y), pos2((x + 3.0 + f.height()).min(f.max.x - rad * 0.5), f.min.y), pos2((x + f.height()).min(f.max.x - rad * 0.5), f.min.y)], fade(Color32::WHITE, 0.45));
            x += 9.0;
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        circle(p, c, r, Color32::WHITE);
        let pts: Vec<Pos2> = (0..=140).map(|i| {
            let a = i as f32 / 140.0 * 3.0 * std::f32::consts::TAU;
            c + vec2(a.cos(), a.sin()) * (3.0 + a * r / 20.0).min(r - 2.0)
        }).collect();
        p.add(Shape::line(pts, Stroke::new(5.0f32, fade(PK, 0.35))));
        p.circle_stroke(c, r, Stroke::new(2.0f32, PK));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, r.height() / 2.0, fade(col, 0.14));
        rstroke(p, r, r.height() / 2.0, Stroke::new(1.0f32, fade(col, 0.5)));
    }
    plus_fns!(candy);
}

/// Brutal: cream, thick black outlines, a hard shadow, square everything.
mod brutal {
    use super::*;
    const INK: Color32 = Color32::from_rgb(18, 18, 18);
    const PK: Color32 = Color32::from_rgb(255, 92, 138);
    const YE: Color32 = Color32::from_rgb(255, 210, 63);
    pub(super) fn style() -> Style {
        Style { text: INK, muted: Color32::from_rgb(78, 78, 78), accent: PK, accent2: YE, radius: 0.0, num: crate::fonts::boldonse_font, label: crate::fonts::black_font, body: crate::fonts::sans_bold_font, caps: true, panel, button, gauge, stage, decor: no_decor, chip, tone: light_tone, shadow: Some(vec2(6.0, 6.0)), animated: false }
    }
    fn panel(s: &dyn Sink, r: Rect, _op: f32, alert: bool) {
        rrect(s, r.translate(vec2(6.0, 6.0)), 0.0, INK);
        rrect(s, r, 0.0, Color32::from_rgb(255, 250, 235));
        rstroke(s, r.shrink(1.5), 0.0, Stroke::new(3.0f32, if alert { PK } else { INK }));
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let fill = kind_fill(kind, PK, if hov { YE } else { Color32::WHITE });
        let lift = if down { 0.0 } else { 3.0 };
        let r = if down { r.translate(vec2(3.0, 3.0)) } else { r };
        if lift > 0.0 {
            rrect(p, r.translate(vec2(lift, lift)), 0.0, INK);
        }
        rrect(p, r, 0.0, fill);
        rstroke(p, r, 0.0, Stroke::new(2.5f32, INK));
        btn_label(p, r, label, crate::fonts::black_font(12.5), on(fill));
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r, 0.0, Color32::WHITE);
        rrect(s, Rect::from_min_size(r.min, vec2(r.width() * frac.clamp(0.0, 1.0), r.height())), 0.0, col);
        rstroke(s, r, 0.0, Stroke::new(2.0f32, INK));
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        let sr = Rect::from_center_size(c, vec2(r * 2.0, r * 2.0));
        rrect(p, sr.translate(vec2(4.0, 4.0)), 0.0, INK);
        rrect(p, sr, 0.0, YE);
        rstroke(p, sr, 0.0, Stroke::new(3.0f32, INK));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 0.0, fade(col, 0.25));
        rstroke(p, r, 0.0, Stroke::new(1.5f32, INK));
    }
    plus_fns!(brutal);
}

/// Liquid: glossy deep blue, a wave rolling along the bottom, bubbles,
/// aqua pills and a bubble behind the sprite.
mod liquid {
    use super::*;
    const AQ: Color32 = Color32::from_rgb(125, 211, 252);
    const MG: Color32 = Color32::from_rgb(219, 39, 119);
    pub(super) fn style() -> Style {
        Style { text: Color32::WHITE, muted: Color32::from_rgb(200, 222, 255), accent: AQ, accent2: MG, radius: 24.0, num: crate::fonts::instrument_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: false, panel, button, gauge, stage, decor, chip, tone: dark_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.5 + 0.5 * op;
        vgrad(s, &rrect_pts(r, 24.0), rgba(40, 104, 232, a), rgba(14, 38, 118, a));
        rrect(s, Rect::from_min_size(r.min + vec2(24.0, 2.0), vec2(r.width() - 48.0, 10.0)), 5.0, fade(Color32::WHITE, 0.12));
        rstroke(s, r.shrink(0.5), 24.0, Stroke::new(1.0f32, if alert { GOLD } else { fade(Color32::WHITE, 0.4) }));
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        let tt = if theme::motion() { t } else { 0.0 };
        let base = r.max.y - 12.0;
        let mut pts = vec![];
        let mut x = r.min.x + 20.0;
        while x <= r.max.x - 20.0 {
            pts.push(pos2(x, base + ((x * 0.05) + tt * 2.0).sin() * 3.0));
            x += 6.0;
        }
        if pts.len() > 1 {
            s.put(Shape::line(pts, Stroke::new(2.0f32, fade(AQ, 0.35))));
        }
        for i in 0..6u32 {
            let travel = r.height() * 0.6;
            let y = r.max.y - 14.0 - ((hash01(i, 1) * travel + tt * (10.0 + 8.0 * hash01(i, 2))) % travel);
            let c = pos2(r.min.x + 16.0 + (r.width() - 32.0) * hash01(i, 3), y);
            s.put(Shape::circle_stroke(c, 2.0 + 3.0 * hash01(i, 4), Stroke::new(1.0f32, fade(Color32::WHITE, 0.3))));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = r.height() / 2.0;
        let fill = kind_fill(kind, AQ, fade(Color32::WHITE, if down { 0.26 } else if hov { 0.2 } else { 0.12 }));
        rrect(p, r, rad, fill);
        rrect(p, Rect::from_min_size(r.min + vec2(rad * 0.6, 2.0), vec2(r.width() - rad * 1.2, r.height() * 0.35)), r.height() * 0.17, fade(Color32::WHITE, if kind == Kind::Plain { 0.12 } else { 0.35 }));
        rstroke(p, r, rad, Stroke::new(1.0f32, fade(Color32::WHITE, 0.5)));
        btn_label(p, r, label, crate::fonts::sans_bold_font(13.0), if kind == Kind::Plain { Color32::WHITE } else { Color32::from_rgb(8, 30, 70) });
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        let rad = r.height() / 2.0;
        rrect(s, r, rad, fade(Color32::WHITE, 0.14));
        let w = (r.width() * frac.clamp(0.0, 1.0)).max(r.height());
        let f = Rect::from_min_size(r.min, vec2(w.min(r.width()), r.height()));
        rrect(s, f, rad, col);
        rrect(s, Rect::from_min_size(f.min + vec2(rad, 1.0), vec2((f.width() - rad * 2.0).max(0.0), f.height() * 0.35)), 1.5, fade(Color32::WHITE, 0.35));
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        circle(p, c, r, fade(Color32::WHITE, 0.1));
        p.circle_stroke(c, r, Stroke::new(1.5f32, fade(Color32::WHITE, 0.5)));
        let arc: Vec<Pos2> = (0..=16).map(|i| {
            let a = 3.6 + i as f32 / 16.0 * 1.1;
            c + vec2(a.cos(), a.sin()) * (r - 6.0)
        }).collect();
        p.add(Shape::line(arc, Stroke::new(3.0f32, fade(Color32::WHITE, 0.55))));
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, r.height() / 2.0, fade(Color32::WHITE, 0.12));
        rstroke(p, r, r.height() / 2.0, Stroke::new(1.0f32, fade(col, 0.6)));
    }
    plus_fns!(liquid);
}

/// Winter: frosty light blue, snow piled on the top edge, snowflakes
/// drifting, icicle gauges, an ice crystal behind the sprite.
mod winter {
    use super::*;
    const BL: Color32 = Color32::from_rgb(56, 140, 220);
    const IC: Color32 = Color32::from_rgb(147, 197, 253);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(28, 48, 80), muted: Color32::from_rgb(88, 110, 142), accent: BL, accent2: IC, radius: 10.0, num: crate::fonts::italiana_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: light_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.75 + 0.25 * op;
        vgrad(s, &rrect_pts(r, 10.0), rgba(240, 248, 255, a), rgba(204, 226, 248, a));
        rstroke(s, r.shrink(0.5), 10.0, Stroke::new(1.5f32, if alert { GOLD } else { fade(BL, 0.45) }));
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        // snow piled along the top edge
        let mut x = r.min.x + 12.0;
        let mut i = 0u32;
        while x < r.max.x - 12.0 {
            circle(s, pos2(x, r.min.y + 1.0), 5.0 + 4.0 * hash01(i, 9), Color32::WHITE);
            x += 9.0;
            i += 1;
        }
        // flakes drifting down
        let tt = if theme::motion() { t } else { 0.0 };
        for i in 0..10u32 {
            let travel = r.height() - 20.0;
            let y = r.min.y + 14.0 + ((hash01(i, 1) * travel + tt * (8.0 + 8.0 * hash01(i, 2))) % travel);
            let x = r.min.x + 10.0 + (r.width() - 20.0) * hash01(i, 3) + (tt * 0.8 + i as f32).sin() * 4.0;
            circle(s, pos2(x, y), 1.0 + 1.2 * hash01(i, 4), fade(BL, 0.35));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        let rad = 10.0f32.min(r.height() / 2.0);
        if kind == Kind::Plain {
            rrect(p, r, rad, fade(Color32::WHITE, if down { 0.95 } else if hov { 0.85 } else { 0.65 }));
            rstroke(p, r, rad, Stroke::new(1.0f32, fade(BL, 0.5)));
            btn_label(p, r, label, crate::fonts::sans_bold_font(12.5), BL);
        } else {
            let fill = kind_fill(kind, BL, BL);
            rrect(p, r, rad, if hov { mix(fill, Color32::WHITE, 0.15) } else { fill });
            rrect(p, Rect::from_min_size(r.min - vec2(0.0, 1.5), vec2(r.width(), 4.0)), 2.0, fade(Color32::WHITE, 0.9));
            btn_label(p, r, label, crate::fonts::sans_bold_font(13.0), on(fill));
        }
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        let rad = r.height() / 2.0;
        rrect(s, r, rad, Color32::from_rgb(220, 234, 250));
        let w = (r.width() * frac.clamp(0.0, 1.0)).max(r.height());
        let f = Rect::from_min_size(r.min, vec2(w.min(r.width()), r.height()));
        hgrad(s, &rrect_pts(f, rad), col, mix(col, Color32::WHITE, 0.55));
        // a few icicles under it
        let mut x = f.min.x + 6.0;
        let mut i = 0u32;
        while x < f.max.x - 4.0 {
            let l = 2.0 + 4.0 * hash01(i, 5);
            poly(s, vec![pos2(x - 1.5, f.max.y), pos2(x + 1.5, f.max.y), pos2(x, f.max.y + l)], fade(IC, 0.9));
            x += 11.0;
            i += 1;
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        circle(p, c, r, fade(Color32::WHITE, 0.7));
        for i in 0..6 {
            let a = i as f32 * std::f32::consts::TAU / 6.0;
            let d = vec2(a.cos(), a.sin());
            line(p, c + d * r * 0.3, c + d * r, Stroke::new(2.0f32, fade(IC, 0.9)));
            for (k, sgn) in [(0.7f32, 1.0f32), (0.7, -1.0)] {
                let b = a + sgn * 0.7;
                let q = c + d * r * k;
                line(p, q, q + vec2(b.cos(), b.sin()) * r * 0.22, Stroke::new(1.5f32, fade(IC, 0.9)));
            }
        }
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 10.0f32.min(r.height() / 2.0), fade(Color32::WHITE, 0.7));
        rstroke(p, r, 10.0f32.min(r.height() / 2.0), Stroke::new(1.0f32, fade(col, 0.6)));
    }
    plus_fns!(winter);
}

/// Spooky: a dark night, cobwebs in the corners, a pale moon behind the
/// sprite, tombstone buttons, an ember gauge and flickering sparks.
mod spooky {
    use super::*;
    const PUMP: Color32 = Color32::from_rgb(249, 115, 22);
    const PURP: Color32 = Color32::from_rgb(168, 85, 247);
    pub(super) fn style() -> Style {
        Style { text: Color32::from_rgb(246, 232, 214), muted: Color32::from_rgb(184, 164, 176), accent: PUMP, accent2: PURP, radius: 8.0, num: crate::fonts::scrawl_font, label: crate::fonts::sans_bold_font, body: crate::fonts::sans_font, caps: true, panel, button, gauge, stage, decor, chip, tone: dark_tone, shadow: None, animated: true }
    }
    fn panel(s: &dyn Sink, r: Rect, op: f32, alert: bool) {
        let a = 0.6 + 0.4 * op;
        vgrad(s, &rrect_pts(r, 8.0), rgba(24, 12, 30, a), rgba(10, 6, 14, a));
        rstroke(s, r.shrink(0.5), 8.0, Stroke::new(1.5f32, if alert { PUMP } else { fade(PURP, 0.5) }));
    }
    fn web(s: &dyn Sink, c: Pos2, dir: egui::Vec2, size: f32) {
        let col = fade(Color32::WHITE, 0.22);
        let base = dir.y.atan2(dir.x);
        let spokes: Vec<egui::Vec2> = (0..5).map(|i| {
            let a = base - 0.75 + i as f32 * 0.375;
            vec2(a.cos(), a.sin())
        }).collect();
        for d in &spokes {
            line(s, c, c + *d * size, Stroke::new(1.0f32, col));
        }
        for k in 1..4 {
            let rr = size * k as f32 / 4.0;
            for w in spokes.windows(2) {
                line(s, c + w[0] * rr, c + w[1] * rr, Stroke::new(0.8f32, col));
            }
        }
    }
    fn decor(s: &dyn Sink, r: Rect, t: f32) {
        web(s, r.right_top() + vec2(-2.0, 2.0), vec2(-1.0, 1.0), 34.0);
        web(s, r.left_bottom() + vec2(2.0, -2.0), vec2(1.0, -1.0), 26.0);
        let tt = if theme::motion() { t } else { 0.0 };
        for i in 0..6u32 {
            let k = (0.5 + 0.5 * (tt * (3.0 + 2.0 * hash01(i, 1)) + i as f32).sin()) * 0.7;
            let c = pos2(r.min.x + 16.0 + (r.width() - 32.0) * hash01(i, 2), r.max.y - 8.0 - 20.0 * hash01(i, 3));
            circle(s, c, 1.4, fade(PUMP, k));
        }
    }
    fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hov: bool, down: bool) {
        // a tombstone: a round top on a square base
        let fill = kind_fill(kind, PUMP, Color32::from_rgb(62, 52, 68));
        let fill = if down { mix(fill, Color32::BLACK, 0.15) } else if hov { mix(fill, Color32::WHITE, 0.12) } else { fill };
        let rad = 8.0f32.min(r.height() / 2.0);
        p.add(Shape::rect_filled(r, egui::Rounding { nw: rad, ne: rad, sw: 2.0, se: 2.0 }, fill));
        p.add(Shape::rect_stroke(r, egui::Rounding { nw: rad, ne: rad, sw: 2.0, se: 2.0 }, Stroke::new(1.0f32, fade(Color32::BLACK, 0.5))));
        btn_label(p, r, label, crate::fonts::sans_bold_font(13.0), on(fill));
    }
    fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
        rrect(s, r, 3.0, Color32::from_rgb(40, 24, 30));
        let w = r.width() * frac.clamp(0.0, 1.0);
        if w > 1.0 {
            let f = Rect::from_min_size(r.min, vec2(w, r.height()));
            rrect(s, f.expand(2.0), 4.0, fade(col, 0.2));
            hgrad(s, &rrect_pts(f, 3.0), mix(col, Color32::from_rgb(200, 30, 30), 0.3), col);
        }
    }
    fn stage(p: &Painter, c: Pos2, r: f32, _acc: Color32) {
        // a pale moon with a few craters
        circle(p, c, r, fade(PUMP, 0.08));
        circle(p, c, r * 0.86, Color32::from_rgb(236, 226, 196));
        for (dx, dy, k) in [(-0.3f32, -0.25f32, 0.14f32), (0.3, 0.1, 0.1), (-0.05, 0.4, 0.08)] {
            circle(p, c + vec2(dx, dy) * r, r * k, Color32::from_rgb(214, 202, 170));
        }
    }
    fn chip(p: &Painter, r: Rect, col: Color32) {
        rrect(p, r, 8.0f32.min(r.height() / 2.0), fade(col, 0.15));
        rstroke(p, r, 8.0f32.min(r.height() / 2.0), Stroke::new(1.0f32, fade(col, 0.5)));
    }
    plus_fns!(spooky);
}

#[allow(dead_code)]
fn _unused() -> fn(Color32) -> Color32 {
    keep
}
