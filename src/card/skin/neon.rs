//! Neon: a synthwave night. A glowing horizon and a perspective grid,
//! dark panels with glowing magenta / cyan edges, the shiny in a neon
//! hexagon and glowing numbers.

use super::*;

pub(in super::super) struct Neon;

pub(super) const MAGENTA: Col = Col(255, 46, 196, 255);
pub(super) const CYAN: Col = Col(0, 232, 255, 255);
const SOFT: Col = Col(176, 168, 220, 255);
const PANEL: Col = Col(16, 11, 36, 230);

/// A rounded box edge that glows.
#[allow(clippy::too_many_arguments)]
fn glow_box(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, r: f32, col: Col) {
    for (k, a) in [(9.0, 0.05), (6.0, 0.08), (3.5, 0.14)] {
        pen.rrect_stroke(x - k, y - k, w + 2.0 * k, h + 2.0 * k, r + k, k, col.alpha(a));
    }
    pen.rrect_stroke(x, y, w, h, r, 2.0, col);
}

fn hexagon(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
    (0..6)
        .map(|i| {
            let a = (i as f32 * 60.0 - 90.0).to_radians();
            (cx + a.cos() * r, cy + a.sin() * r)
        })
        .collect()
}

fn glow_poly(pen: &mut Pen, pts: &[(f32, f32)], col: Col) {
    let n = pts.len();
    for (wd, a) in [(14.0, 0.06), (8.0, 0.12), (4.0, 0.25), (2.5, 1.0)] {
        for i in 0..n {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            pen.line(p.0, p.1, q.0, q.1, wd, col.alpha(a));
        }
    }
}

impl Skin for Neon {
    fn bg(&self, _o: &CardOptions) -> Col {
        Col(8, 6, 20, 255)
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        let hz = h * 0.66;
        pen.rrect_vgrad(0.0, 0.0, w, hz, 0.0, Col(10, 6, 28, 255), Col(40, 10, 60, 255));
        // stars
        let mut rng = Rng(0x5eed_0f_57a2);
        for _ in 0..((w * hz / 9000.0) as usize) {
            let (sx, sy) = (rng.next() * w, rng.next() * hz * 0.9);
            pen.circle(sx, sy, 0.8 + rng.next() * 1.4, WHITE.alpha(0.2 + rng.next() * 0.5));
        }
        pen.glow(w / 2.0, hz, w * 0.55, MAGENTA.alpha(0.35));
        pen.glow(w / 2.0, hz, w * 0.25, Col(255, 140, 60, 255).alpha(0.25));
        pen.rrect(0.0, hz, w, h - hz, 0.0, Col(10, 4, 22, 255));
        // perspective grid
        let mut d = 6.0;
        while hz + d < h {
            pen.line(0.0, hz + d, w, hz + d, 1.6, MAGENTA.alpha(0.38));
            d *= 1.32;
        }
        let vx = w / 2.0;
        for i in -14..=14 {
            let bx = vx + i as f32 * w / 10.0;
            pen.line(vx + (bx - vx) * 0.02, hz, bx, h, 1.6, MAGENTA.alpha(0.3));
        }
        pen.line(0.0, hz, w, hz, 2.5, MAGENTA.alpha(0.7));
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        SOFT
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        CYAN
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Display, num: Face::Black, label: Face::Bold, body: Face::Body }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(WHITE, 0.2)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let (r, col) = match role {
            Role::Hero => (18.0, MAGENTA),
            Role::Card => (18.0, MAGENTA),
            Role::Tile => (12.0, CYAN),
            Role::Slot => (12.0, CYAN),
        };
        pen.rrect(x, y, w, h, r, PANEL);
        glow_box(pen, x, y, w, h, r, col);
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        pen.rrect(x, y, w, h, 18.0, PANEL);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = w.min(h) * 0.46;
        let pulse = pen.phase.map_or(1.0, |p| 0.75 + 0.25 * (p * std::f32::consts::TAU * 2.0).sin().abs());
        pen.glow(cx, cy, r * 1.1, MAGENTA.alpha(0.25 * pulse));
        pen.poly(&hexagon(cx, cy, r), Col(20, 8, 44, 255));
        glow_poly(pen, &hexagon(cx, cy, r), CYAN);
        glow_poly(pen, &hexagon(cx, cy, r * 0.86), MAGENTA.alpha(0.6));
        hero_sprites(pen, d, o, x, y, w, h, CYAN);
        if normal_inset(pen, d, o, x + 16.0, y + h - 104.0, 88.0) {
            pen.text("NORMAL", x + 60.0, y + h - 26.0, 11.0, SOFT, Face::Bold, Align::Center, false);
        }
        glow_box(pen, x, y, w, h, 18.0, MAGENTA);
        glow_text(pen, "LIVE", x + w - 26.0, y + 18.0, 15.0, WHITE, MAGENTA, Face::Bold, Align::Right);
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = pen.width(&t, 15.0, Face::Bold) + 32.0;
        let x0 = aligned(x, w, align);
        let c = fill.map(|c| c.mix(WHITE, 0.2)).unwrap_or(MAGENTA);
        pen.rrect(x0, y, w, 34.0, 17.0, c.alpha(0.16));
        glow_box(pen, x0, y, w, 34.0, 17.0, c);
        pen.text(&t, x0 + 16.0, y + 8.5, 15.0, WHITE, Face::Bold, Align::Left, false);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y, size, WHITE, MAGENTA, Face::Display, align);
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y, size, Col(220, 252, 255, 255), CYAN, Face::Black, align);
    }
    fn gauge_h(&self) -> f32 {
        16.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, _col: Col) {
        pen.rrect(x, y, w, 16.0, 8.0, WHITE.alpha(0.08));
        let fw = (w * frac.clamp(0.0, 1.0)).max(16.0);
        for (k, a) in [(8.0, 0.06), (4.0, 0.12)] {
            pen.rrect(x - k, y - k, fw + 2.0 * k, 16.0 + 2.0 * k, 8.0 + k, MAGENTA.alpha(a));
        }
        hgrad(pen, x, y, fw, 16.0, 8.0, MAGENTA, CYAN);
        pen.rrect(x + 6.0, y + 3.0, (fw - 12.0).max(0.0), 3.0, 1.5, WHITE.alpha(0.5));
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        pen.glow(cx, cy, r * 1.3, MAGENTA.alpha(0.3));
        pen.poly(&hexagon(cx, cy, r), Col(20, 8, 44, 255));
        glow_poly(pen, &hexagon(cx, cy, r), CYAN);
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.55 } else { r * 0.72 };
        self.number(pen, o, &n, cx, cy - size * 0.62, size, Align::Center);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, col: Col) {
        pen.rrect(x, y, w, h, h / 2.0, Col(16, 8, 36, 235));
        glow_box(pen, x, y, w, h, h / 2.0, col);
    }
    fn area_col(&self, _o: &CardOptions, i: usize) -> Col {
        [CYAN, MAGENTA, Col(150, 92, 255, 255), Col(255, 200, 60, 255)][i % 4]
    }
    fn map_px(&self, c: Col) -> Col {
        let l = (0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32) / 255.0;
        Col(20, 8, 44, 255).mix(Col(120, 60, 200, 255), l).mix(c, 0.25)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        glow_text(pen, name, cx, y, size, WHITE, CYAN, Face::Bold, Align::Center);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        hgrad(pen, x0, y + 2.0, x1 - x0, 2.0, 1.0, MAGENTA, CYAN);
        let mut x = x0;
        if !left.is_empty() {
            x += pen.text(&left.to_uppercase(), x, cy - 7.0, 13.0, SOFT, Face::Bold, Align::Left, false) + 12.0;
        }
        if !name.is_empty() {
            x += glow_text(pen, name, x, cy - 12.0, 21.0, WHITE, MAGENTA, Face::Bold, Align::Left) + 14.0;
        }
        if let Some(lv) = level {
            self.chip(pen, o, &format!("LV {lv}"), x, cy - 15.0, Some(CYAN), Align::Left);
        }
        let bw = pen.text(&BRAND.to_uppercase(), x1, cy - 7.0, 13.0, SOFT, Face::Bold, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 38.0, cy - 14.0, 28.0, 28.0);
        }
    }
}
