//! The battle pass share image looks: Distortion World (Giratina's
//! upside-down world: void violet, crimson rifts, Griseous gold, floating
//! islands and shadow tendrils) and Snowpoint Aurora (night sky over the
//! snowy peaks, dancing teal-green-violet lights, falling snow). Each comes
//! still, or "live": then the whole picture moves in the GIF.

use super::*;
use crate::cosmetics;
use std::f32::consts::TAU;

// ───────────────────────────── Distortion World ─────────────────────────────

pub(in super::super) struct Distortion {
    pub live: bool,
}

const VOID: Col = Col(14, 6, 26, 255);
const DEEP: Col = Col(40, 16, 66, 255);
const VIOLET: Col = Col(112, 54, 178, 255);
const CRIMSON: Col = Col(222, 40, 74, 255);
const GOLD: Col = Col(236, 186, 80, 255);
const RIFT_GREEN: Col = Col(86, 186, 150, 255);
const LILAC_SOFT: Col = Col(198, 182, 226, 255);

/// Where the loop is (0..1); still images sit at 0.
fn t_of(pen: &Pen, live: bool) -> f32 {
    if live {
        pen.phase.unwrap_or(0.0)
    } else {
        0.0
    }
}

/// A crimson rift: a jagged crack with a glow.
fn rift(pen: &mut Pen, x: f32, y: f32, len: f32, ang: f32, open: f32) {
    let (dx, dy) = (ang.cos(), ang.sin());
    let (nx, ny) = (-dy, dx);
    let n = 7;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let f = i as f32 / n as f32;
        let j = if i == 0 || i == n { 0.0 } else if i % 2 == 0 { 9.0 } else { -9.0 };
        pts.push((x + dx * len * f + nx * j, y + dy * len * f + ny * j));
    }
    pen.glow(x + dx * len / 2.0, y + dy * len / 2.0, len * 0.6, CRIMSON.alpha(0.18 + 0.2 * open));
    for w in pts.windows(2) {
        pen.line(w[0].0, w[0].1, w[1].0, w[1].1, 4.0 + 5.0 * open, CRIMSON.alpha(0.85));
        pen.line(w[0].0, w[0].1, w[1].0, w[1].1, 1.4 + 1.6 * open, Col(255, 190, 200, 255));
    }
}

/// An upside-down floating island.
fn island(pen: &mut Pen, x: f32, y: f32, w: f32, flip: bool) {
    let d = if flip { -1.0 } else { 1.0 };
    let h = w * 0.32;
    pen.poly(&[(x, y), (x + w, y), (x + w * 0.72, y + d * h), (x + w * 0.28, y + d * h)], Col(52, 40, 82, 235));
    pen.poly(&[(x + w * 0.28, y + d * h), (x + w * 0.72, y + d * h), (x + w * 0.5, y + d * h * 1.9)], Col(36, 26, 60, 235));
    pen.line(x, y, x + w, y, 3.0, RIFT_GREEN.alpha(0.8));
}

/// A swaying shadow tendril from (x, y) towards `ang`.
fn tendril(pen: &mut Pen, x: f32, y: f32, len: f32, ang: f32, t: f32, thick: f32) {
    // one smooth tapering stroke (solid, so the joins don't show as beads)
    let n = 64;
    let col = Col(6, 1, 12, 255);
    let at = |f: f32| {
        let sway = (f * TAU * 1.3 + t * TAU).sin() * len * 0.08 * f;
        let a = ang + (f * 2.0 + t * TAU).sin() * 0.25 * f;
        (x + a.cos() * len * f - a.sin() * sway, y + a.sin() * len * f + a.cos() * sway)
    };
    let mut prev = at(0.0);
    for i in 1..=n {
        let f = i as f32 / n as f32;
        let p = at(f);
        let r = thick * (1.0 - f * 0.9);
        pen.line(prev.0, prev.1, p.0, p.1, r * 2.0, col);
        pen.circle(p.0, p.1, r, col);
        prev = p;
    }
}

impl Skin for Distortion {
    fn bg(&self, _o: &CardOptions) -> Col {
        VOID
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_with(0.0, 0.0, w, h, 0.0, |x, y| DEEP.mix(VOID, ((x / w) * 0.4 + (y / h) * 0.6).clamp(0.0, 1.0)));
        pen.glow(w * 0.78, h * 0.18, w.min(h) * 0.7, VIOLET.alpha(0.45));
        pen.glow(w * 0.12, h * 0.9, w.min(h) * 0.55, CRIMSON.alpha(0.22));
        // the far, upside-down horizon
        pen.rrect_vgrad(0.0, h * 0.36, w, h * 0.08, 0.0, RIFT_GREEN.alpha(0.0), RIFT_GREEN.alpha(0.09));
        let mut r = Rng(0x61A7);
        for _ in 0..70 {
            pen.circle(r.next() * w, r.next() * h, 0.8 + r.next() * 1.6, LILAC_SOFT.alpha(0.15 + r.next() * 0.35));
        }
    }
    fn chrome(&self, pen: &mut Pen, _o: &CardOptions, _what: &str) {
        let (w, h) = (pen.w, pen.h);
        let t = t_of(pen, self.live);
        let bob = |k: f32| (t * TAU + k).sin();
        // floating islands drifting up and down
        island(pen, w * 0.05, h * 0.1 + bob(0.0) * 10.0, 120.0, true);
        island(pen, w * 0.8, h * 0.07 + bob(2.0) * 12.0, 90.0, true);
        island(pen, w * 0.55, h * 0.93 + bob(4.0) * 8.0, 140.0, false);
        island(pen, w * 0.9, h * 0.72 + bob(1.0) * 10.0, 70.0, false);
        // rifts that open and close
        // (kept to the edges, clear of the title)
        rift(pen, w * 0.9, h * 0.03, 140.0, 1.75, (bob(0.5) * 0.5 + 0.5) * 0.8);
        rift(pen, w * 0.02, h * 0.62, 120.0, -0.4, (bob(3.0) * 0.5 + 0.5) * 0.8);
        // shadow tendrils in the corners
        tendril(pen, 0.0, h, h * 0.55, -0.9, t, 18.0);
        tendril(pen, 0.0, h, h * 0.4, -0.4, t + 0.3, 14.0);
        tendril(pen, w, 0.0, h * 0.5, 2.3, t + 0.6, 16.0);
        tendril(pen, w, 0.0, h * 0.36, 2.75, t + 0.15, 12.0);
    }
    fn overlay(&self, pen: &mut Pen, _o: &CardOptions) {
        if !self.live {
            return;
        }
        // embers drifting upward
        let (w, h) = (pen.w, pen.h);
        let t = t_of(pen, true);
        let mut r = Rng(0xE3B);
        for _ in 0..26 {
            let x = r.next() * w + (t * TAU + r.next() * 6.0).sin() * 14.0;
            let y = (r.next() * h - t * h).rem_euclid(h);
            let c = if r.next() > 0.5 { CRIMSON } else { GOLD };
            pen.circle(x, y, 1.6 + r.next() * 2.2, c.alpha(0.55));
        }
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        LILAC_SOFT
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        GOLD
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Display, num: Face::Black, label: Face::Bold, body: Face::Body }
    }
    fn case(&self, s: &str) -> String {
        s.to_uppercase()
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let r = match role {
            Role::Hero | Role::Card => 22.0,
            Role::Tile => 14.0,
            Role::Slot => 12.0,
        };
        pen.rrect(x + 4.0, y + 6.0, w, h, r, Col(0, 0, 0, 120));
        pen.rrect_vgrad(x, y, w, h, r, Col(46, 22, 74, 225), Col(22, 10, 38, 235));
        pen.rrect_stroke(x, y, w, h, r, 1.5, VIOLET.alpha(0.7));
        pen.line(x + r, y + 1.5, x + w - r, y + 1.5, 2.0, CRIMSON.alpha(0.7));
        // Griseous-gold corner ticks
        let k = (r * 0.9).min(18.0);
        for (cx, cy, sx, sy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
            pen.line(cx + sx * 4.0, cy + sy * 4.0, cx + sx * (4.0 + k), cy + sy * 4.0, 2.0, GOLD.alpha(0.85));
            pen.line(cx + sx * 4.0, cy + sy * 4.0, cx + sx * 4.0, cy + sy * (4.0 + k), 2.0, GOLD.alpha(0.85));
        }
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = w.min(h) * 0.4;
        let t = t_of(pen, self.live);
        // a portal: a violet vortex with a turning gold ring
        pen.glow(cx, cy, r * 1.4, VIOLET.alpha(0.5));
        for (k, a) in [(1.0, 0.12), (0.8, 0.16), (0.6, 0.2)] {
            pen.circle(cx, cy, r * k, VOID.alpha(a * 3.0));
        }
        let spin = t * TAU;
        {
            let s = pen.s;
            cosmetics::arc(&mut pen.c, cx * s, cy * s, r * s, 3.5 * s, spin, spin + TAU * 0.7, |f| GOLD.alpha(0.2 + 0.7 * f));
            cosmetics::arc(&mut pen.c, cx * s, cy * s, r * 0.82 * s, 2.0 * s, -spin * 1.5 + 2.0, -spin * 1.5 + 2.0 + TAU * 0.45, |f| CRIMSON.alpha(0.2 + 0.6 * f));
        }
        hero_sprites(pen, d, o, x, y, w, h, GOLD);
        if normal_inset(pen, d, o, x + 22.0, y + h - 112.0, 90.0) {
            pen.ring(x + 67.0, y + h - 67.0, 50.0, 2.0, GOLD.alpha(0.6));
            tracked(pen, "NORMAL", x + 67.0, y + h - 24.0, 11.0, LILAC_SOFT, Face::Bold, Align::Center, 2.0);
        }
        tracked(pen, "SHINY", x + w - 26.0, y + 24.0, 13.0, GOLD, Face::Bold, Align::Right, 3.0);
    }
    fn chip_w(&self, pen: &Pen, text: &str) -> f32 {
        let t = text.to_uppercase();
        let w = tracked_w(pen, &t, 14.0, Face::Bold, 2.0) + 30.0;
        w
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = tracked_w(pen, &t, 14.0, Face::Bold, 2.0) + 30.0;
        let x0 = aligned(x, w, align);
        let c = fill.unwrap_or(CRIMSON);
        pen.poly(&[(x0 + 8.0, y), (x0 + w, y), (x0 + w - 8.0, y + 34.0), (x0, y + 34.0)], c.mix(VOID, 0.35));
        pen.line(x0 + 8.0, y + 1.0, x0 + w, y + 1.0, 2.0, GOLD.alpha(0.8));
        tracked(pen, &t, x0 + 15.0, y + 17.0 - pen.mid(14.0, Face::Bold), 14.0, WHITE, Face::Bold, Align::Left, 2.0);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        // a crimson echo behind, like the world tearing
        pen.text(text, x + size * 0.05, y + size * 0.04, size, CRIMSON.alpha(0.75), Face::Display, align, false);
        pen.text(text, x, y, size, WHITE, Face::Display, align, false);
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        tracked(pen, &text.to_uppercase(), x, y, size, col, Face::Bold, align, size * 0.15)
    }
    fn label_mid(&self, pen: &Pen, size: f32) -> f32 {
        pen.mid(size, Face::Bold)
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y, size, WHITE, VIOLET.alpha(0.7), Face::Black, align);
    }
    fn gauge_h(&self) -> f32 {
        20.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        pen.rrect(x, y + 4.0, w, 12.0, 6.0, Col(8, 2, 16, 255));
        pen.rrect_stroke(x, y + 4.0, w, 12.0, 6.0, 1.2, VIOLET.alpha(0.8));
        // filled in the status colour (under / at / over the odds), like every style
        let fw = (w * frac.clamp(0.0, 1.0)).max(12.0);
        hgrad(pen, x, y + 4.0, fw, 12.0, 6.0, col.mix(VOID, 0.45), col);
        pen.glow(x + fw, y + 10.0, 22.0, col.alpha(0.5));
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        pen.glow(cx, cy, r * 1.4, VIOLET.alpha(0.5));
        pen.circle(cx, cy, r, VOID);
        pen.ring(cx, cy, r, 4.0, GOLD);
        pen.ring(cx, cy, r * 0.84, 2.0, CRIMSON);
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.62 } else { r * 0.82 };
        self.number(pen, o, &n, cx, cy - size * 0.62, size, Align::Center);
    }
}

// ───────────────────────────── Snowpoint Aurora ─────────────────────────────

pub(in super::super) struct Aurora {
    pub live: bool,
}

const NIGHT: Col = Col(8, 18, 44, 255);
const NAVY: Col = Col(18, 40, 86, 255);
const TEAL: Col = Col(50, 220, 200, 255);
const GREEN: Col = Col(118, 244, 166, 255);
const LILAC: Col = Col(164, 132, 255, 255);
const ICE: Col = Col(234, 247, 255, 255);
const FROST: Col = Col(160, 214, 242, 255);

impl Skin for Aurora {
    fn bg(&self, _o: &CardOptions) -> Col {
        NIGHT
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_vgrad(0.0, 0.0, w, h, 0.0, NIGHT, NAVY);
        let mut r = Rng(0x5A0A);
        for _ in 0..110 {
            pen.circle(r.next() * w, r.next() * h * 0.7, 0.6 + r.next() * 1.3, WHITE.alpha(0.2 + r.next() * 0.6));
        }
        // Snowpoint's peaks low along the bottom: a soft silhouette with frost on the
        // ridge line (the panels are see-through, so nothing bright sits under them)
        let base = h;
        let peaks = [(0.0, -0.06), (0.12, -0.13), (0.24, -0.05), (0.38, -0.16), (0.52, -0.06), (0.66, -0.14), (0.8, -0.04), (0.92, -0.11), (1.0, -0.03)];
        for wnd in peaks.windows(2) {
            let ((ax, ay), (bx, by)) = (wnd[0], wnd[1]);
            pen.poly(&[(ax * w, base + ay * h), (bx * w, base + by * h), (bx * w, h), (ax * w, h)], Col(36, 64, 112, 255));
        }
        for wnd in peaks.windows(2) {
            let ((ax, ay), (bx, by)) = (wnd[0], wnd[1]);
            pen.line(ax * w, base + ay * h, bx * w, base + by * h, 2.0, ICE.alpha(0.28));
        }
        pen.rrect_vgrad(0.0, h * 0.9, w, h * 0.1, 0.0, ICE.alpha(0.0), ICE.alpha(0.15));
    }
    fn chrome(&self, pen: &mut Pen, _o: &CardOptions, _what: &str) {
        let (w, h) = (pen.w, pen.h);
        let t = t_of(pen, self.live) * TAU;
        let s = pen.s;
        cosmetics::curtain(&mut pen.c, -20.0 * s, h * 0.02 * s, (w + 40.0) * s, h * 0.42 * s, t, h * 0.06 * s, GREEN, TEAL, 0.5);
        cosmetics::curtain(&mut pen.c, -20.0 * s, h * 0.1 * s, (w + 40.0) * s, h * 0.36 * s, t * 2.0 + 2.0, h * 0.05 * s, TEAL, LILAC, 0.38);
        cosmetics::curtain(&mut pen.c, w * 0.3 * s, h * 0.0, w * 0.75 * s, h * 0.3 * s, -t + 4.0, h * 0.04 * s, LILAC, GREEN, 0.25);
    }
    fn overlay(&self, pen: &mut Pen, _o: &CardOptions) {
        // snow falling over everything
        let (w, h) = (pen.w, pen.h);
        let t = t_of(pen, self.live);
        let mut r = Rng(0x5404);
        let n = if self.live { 60 } else { 34 };
        for i in 0..n {
            let size = 2.0 + r.next() * 5.0;
            let speed = 0.6 + size / 7.0;
            let x = r.next() * w + (t * TAU * 2.0 + i as f32).sin() * 10.0;
            let y = (r.next() * h + t * h * speed.round().max(1.0)).rem_euclid(h);
            let c = ICE.alpha(0.35 + r.next() * 0.45);
            if size > 5.0 {
                let s = pen.s;
                cosmetics::snowflake(&mut pen.c, x * s, y * s, size * s, c);
            } else {
                pen.circle(x, y, size * 0.45, c);
            }
        }
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        FROST
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        TEAL
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Bold, num: Face::Light, label: Face::Bold, body: Face::Body }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(WHITE, 0.2)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let r = match role {
            Role::Hero | Role::Card => 26.0,
            Role::Tile => 18.0,
            Role::Slot => 16.0,
        };
        pen.rrect_vgrad(x, y, w, h, r, Col(200, 236, 255, 42), Col(120, 190, 255, 18));
        pen.rrect_stroke(x, y, w, h, r, 1.6, ICE.alpha(0.42));
        pen.rrect_vgrad(x + 2.0, y + 2.0, w - 4.0, (h * 0.35).min(40.0), r, WHITE.alpha(0.12), WHITE.alpha(0.0));
        // a little frost in the top corner
        let s = pen.s;
        cosmetics::snowflake(&mut pen.c, (x + w - 16.0) * s, (y + 16.0) * s, 6.0 * s, ICE.alpha(0.6));
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = w.min(h) * 0.42;
        pen.glow(cx, cy, r * 1.3, TEAL.alpha(0.3));
        let t = t_of(pen, self.live) * TAU;
        {
            let s = pen.s;
            cosmetics::arc(&mut pen.c, cx * s, cy * s, r * s, 3.0 * s, 0.0, TAU, |f| {
                let k = (f * TAU + t).sin() * 0.5 + 0.5;
                GREEN.mix(LILAC, k).alpha(0.75)
            });
        }
        hero_sprites(pen, d, o, x, y, w, h, ICE);
        if normal_inset(pen, d, o, x + 22.0, y + h - 112.0, 90.0) {
            pen.ring(x + 67.0, y + h - 67.0, 50.0, 1.5, ICE.alpha(0.5));
            tracked(pen, "NORMAL", x + 67.0, y + h - 24.0, 11.0, FROST, Face::Bold, Align::Center, 2.0);
        }
        tracked(pen, "SHINY", x + w - 28.0, y + 26.0, 13.0, TEAL, Face::Bold, Align::Right, 3.0);
    }
    fn chip_w(&self, pen: &Pen, text: &str) -> f32 {
        let t = text.to_uppercase();
        let w = tracked_w(pen, &t, 14.0, Face::Bold, 2.5) + 32.0;
        w
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = tracked_w(pen, &t, 14.0, Face::Bold, 2.5) + 32.0;
        let x0 = aligned(x, w, align);
        let c = fill.unwrap_or(TEAL);
        pen.rrect(x0, y, w, 34.0, 17.0, c.alpha(0.2));
        pen.rrect_stroke(x0, y, w, 34.0, 17.0, 1.5, c.mix(WHITE, 0.4));
        tracked(pen, &t, x0 + 16.0, y + 17.0 - pen.mid(14.0, Face::Bold), 14.0, WHITE, Face::Bold, Align::Left, 2.5);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y, size, WHITE, TEAL.alpha(0.6), Face::Bold, align);
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        tracked(pen, &text.to_uppercase(), x, y, size, col, Face::Bold, align, size * 0.16)
    }
    fn label_mid(&self, pen: &Pen, size: f32) -> f32 {
        pen.mid(size, Face::Bold)
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y - size * 0.06, size, WHITE, TEAL.alpha(0.55), Face::Light, align);
    }
    fn gauge_h(&self) -> f32 {
        18.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        let cy = y + 9.0;
        pen.rrect(x, cy - 3.0, w, 6.0, 3.0, WHITE.alpha(0.14));
        // filled in the status colour (under / at / over the odds), like every style
        let fx = x + w * frac.clamp(0.0, 1.0);
        if fx > x + 2.0 {
            hgrad(pen, x, cy - 3.0, fx - x, 6.0, 3.0, col.alpha(0.45), col);
        }
        pen.glow(fx.max(x + 3.0), cy, 20.0, col.alpha(0.45));
        let s = pen.s;
        cosmetics::snowflake(&mut pen.c, fx.max(x + 3.0) * s, cy * s, 8.0 * s, WHITE);
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        pen.glow(cx, cy, r * 1.3, TEAL.alpha(0.35));
        let s = pen.s;
        cosmetics::arc(&mut pen.c, cx * s, cy * s, r * s, 5.0 * s, 0.0, TAU, |f| GREEN.mix(LILAC, (f * TAU).sin() * 0.5 + 0.5));
        pen.ring(cx, cy, r * 0.86, 1.2, ICE.alpha(0.5));
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.62 } else { r * 0.82 };
        self.number(pen, o, &n, cx, cy - size * 0.62, size, Align::Center);
    }
}
