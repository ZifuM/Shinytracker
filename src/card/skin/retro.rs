//! Retro DS: a Gen 4 battle screen. Sky over striped grass, the shiny on
//! a battle platform, white text boxes with a two-tone border, an HP-bar
//! luck gauge and the pixel font; the footer is a text box with a blinking
//! "more" arrow.

use super::*;

pub(in super::super) struct Retro;

const INK: Col = Col(48, 48, 56, 255);
const SOFT: Col = Col(104, 104, 120, 255);
const BOX: Col = Col(248, 248, 248, 255);
const EDGE: Col = Col(64, 64, 76, 255);
const EDGE_IN: Col = Col(200, 208, 224, 255);
const SKY_A: Col = Col(136, 200, 248, 255);
const SKY_B: Col = Col(224, 242, 255, 255);
const GRASS: Col = Col(120, 196, 96, 255);
const GRASS_D: Col = Col(104, 180, 84, 255);

/// Where the grass starts (share of the height).
const HORIZON: f32 = 0.6;

impl Retro {
    fn textbox(&self, pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, r: f32) {
        pen.rrect(x + 4.0, y + 5.0, w, h, r, Col(0, 0, 0, 50));
        pen.rrect(x, y, w, h, r, EDGE);
        pen.rrect(x + 4.0, y + 4.0, w - 8.0, h - 8.0, (r - 3.0).max(2.0), EDGE_IN);
        pen.rrect(x + 7.0, y + 7.0, w - 14.0, h - 14.0, (r - 5.0).max(2.0), BOX);
    }
}

impl Skin for Retro {
    fn bg(&self, _o: &CardOptions) -> Col {
        SKY_A
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        let hz = (h * HORIZON).min(h - 220.0).max(h * 0.4);
        pen.rrect_vgrad(0.0, 0.0, w, hz, 0.0, SKY_A, SKY_B);
        // clouds
        for (cx, cy, s) in [(w * 0.18, hz * 0.18, 1.0), (w * 0.62, hz * 0.1, 0.8), (w * 0.86, hz * 0.32, 0.7)] {
            for (dx, dy, r) in [(-40.0, 6.0, 26.0), (0.0, 0.0, 36.0), (42.0, 8.0, 24.0), (16.0, 16.0, 26.0)] {
                pen.circle(cx + dx * s, cy + dy * s, r * s, WHITE.alpha(0.85));
            }
        }
        pen.rrect(0.0, hz, w, h - hz, 0.0, GRASS);
        let mut y = hz;
        let mut k = 0;
        while y < h {
            if k % 2 == 1 {
                pen.rrect(0.0, y, w, 16.0, 0.0, GRASS_D);
            }
            y += 16.0;
            k += 1;
        }
        pen.rrect(0.0, hz - 3.0, w, 6.0, 0.0, Col(88, 160, 72, 255));
    }
    fn inset(&self) -> f32 {
        40.0
    }
    fn text(&self) -> Col {
        INK
    }
    fn soft(&self) -> Col {
        SOFT
    }
    fn accent(&self, o: &CardOptions) -> Col {
        o.theme.type_colour().mix(BLACK, 0.25)
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Pixel, num: Face::Pixel, label: Face::Pixel, body: Face::Pixel }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(BLACK, 0.3)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let r = match role {
            Role::Card | Role::Hero => 14.0,
            _ => 10.0,
        };
        self.textbox(pen, x, y, w, h, r);
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        // A battle platform on the grass (no box): the shiny stands on it.
        let (cx, floor) = (x + w / 2.0, y + h - h * 0.16);
        let rx = w * 0.46;
        let ry = (h * 0.1).min(48.0);
        pen.ellipse(cx + 6.0, floor + 8.0, rx, ry, Col(0, 0, 0, 40));
        pen.ellipse(cx, floor, rx, ry, Col(88, 160, 72, 255));
        pen.ellipse(cx, floor - 4.0, rx * 0.94, ry * 0.8, Col(168, 224, 128, 255));
        pen.ellipse(cx, floor - 5.0, rx * 0.7, ry * 0.5, Col(192, 236, 152, 255));
        match &d.shiny {
            Some(img) => {
                let (sx, sy, sw, sh) = sprite(pen, img, d.sprite_box, cx, floor + ry * 0.2, w * 0.78, h * 0.78, false);
                if o.sparkles {
                    sparkles(pen, sx, sy, sw, sh, Col(255, 236, 120, 255));
                }
            }
            None => {
                pen.text("?", cx, floor - 160.0, 120.0, INK, Face::Pixel, Align::Center, false);
            }
        }
        if o.show_normal && d.normal.is_some() {
            self.textbox(pen, x, y, 112.0, 132.0, 10.0);
            normal_inset(pen, d, o, x + 12.0, y + 10.0, 88.0);
            pen.text("NORMAL", x + 56.0, y + 106.0, 10.0, SOFT, Face::Pixel, Align::Center, false);
        }
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = pen.width(&t, 13.0, Face::Pixel) + 28.0;
        let x0 = aligned(x, w, align);
        let c = fill.map(|c| c.mix(BLACK, 0.15)).unwrap_or(EDGE);
        pen.rrect(x0, y, w, 34.0, 6.0, INK);
        pen.rrect(x0 + 3.0, y + 3.0, w - 6.0, 28.0, 4.0, c);
        pen.rrect(x0 + 6.0, y + 5.0, w - 12.0, 3.0, 1.5, WHITE.alpha(0.35));
        pen.text(&t, x0 + 14.0, y + 11.0, 13.0, WHITE, Face::Pixel, Align::Left, false);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let s = size * 0.8;
        let y = y + size * 0.1;
        pen.text(text, x + 3.0, y + 3.0, s, Col(0, 0, 0, 60), Face::Pixel, align, false);
        pen.text(text, x, y, s, INK, Face::Pixel, align, false);
    }
    fn number(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let s = size * 0.78;
        let y = y + size * 0.1;
        pen.text(text, x + 4.0, y + 4.0, s, Col(0, 0, 0, 60), Face::Pixel, align, false);
        pen.text(text, x, y, s, self.accent(o), Face::Pixel, align, false);
    }
    fn number_w(&self, pen: &Pen, text: &str, size: f32) -> f32 {
        pen.width(text, size * 0.78, Face::Pixel)
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        pen.text(&text.to_uppercase(), x, y, size * 0.8, col, Face::Pixel, align, false)
    }
    fn gauge_h(&self) -> f32 {
        22.0
    }
    /// An HP bar: a dark frame, the yellow "HP" tab, green / yellow / red fill.
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        let tab = 48.0;
        pen.rrect(x, y, w, 22.0, 6.0, INK);
        pen.rrect(x + 3.0, y + 3.0, tab - 6.0, 16.0, 3.0, Col(248, 184, 64, 255));
        pen.text("HP", x + tab / 2.0, y + 6.0, 10.0, INK, Face::Pixel, Align::Center, false);
        let (bx, bw) = (x + tab, w - tab - 4.0);
        pen.rrect(bx, y + 4.0, bw, 14.0, 3.0, Col(84, 84, 96, 255));
        let fw = bw * frac.clamp(0.0, 1.0);
        if fw > 1.0 {
            let c = col.mix(WHITE, 0.15);
            pen.rrect(bx, y + 4.0, fw.max(6.0), 14.0, 3.0, c);
            pen.rrect(bx + 2.0, y + 6.0, (fw - 4.0).max(2.0), 3.0, 1.5, WHITE.alpha(0.45));
        }
    }
    fn tile(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.textbox(pen, x, y, w, h, 10.0);
        pen.text(&label.to_uppercase(), x + 18.0, y + 18.0, 11.0, SOFT, Face::Pixel, Align::Left, false);
        let vs = pen.fit(value, 20.0, w - 36.0, Face::Pixel);
        pen.text(value, x + 18.0, y + h - vs - 18.0, vs, INK, Face::Pixel, Align::Left, false);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        self.textbox(pen, x0, y - 4.0, x1 - x0, h + 4.0, 12.0);
        let cy = y + h / 2.0 - 2.0;
        let mut x = x0 + 24.0;
        if !left.is_empty() {
            x += pen.text(&left.to_uppercase(), x, cy - 6.0, 12.0, SOFT, Face::Pixel, Align::Left, false) + 14.0;
        }
        if !name.is_empty() {
            x += pen.text(&name.to_uppercase(), x, cy - 8.0, 15.0, INK, Face::Pixel, Align::Left, false) + 14.0;
        }
        if let Some(lv) = level {
            pen.text(&format!("Lv{lv}"), x, cy - 6.0, 12.0, self.accent(o), Face::Pixel, Align::Left, false);
        }
        let bw = pen.text(&BRAND.to_uppercase(), x1 - 54.0, cy - 5.0, 10.0, SOFT, Face::Pixel, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - 54.0 - bw - 34.0, cy - 12.0, 24.0, 24.0);
        }
        // the blinking "more" arrow
        let on = pen.phase.map_or(true, |p| (p * 4.0).fract() < 0.6);
        if on {
            let (ax, ay) = (x1 - 30.0, cy - 4.0);
            pen.poly(&[(ax - 8.0, ay), (ax + 8.0, ay), (ax, ay + 10.0)], Col(224, 72, 64, 255));
        }
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, metal: Col, light: Col) {
        // a Poké Ball-style emblem
        pen.circle(cx + 4.0, cy + 5.0, r, Col(0, 0, 0, 50));
        pen.circle(cx, cy, r, INK);
        pen.circle(cx, cy, r - 5.0, BOX);
        pen.rrect_with(cx - r + 5.0, cy - r + 5.0, 2.0 * r - 10.0, r - 8.0, 0.0, |px, py| if ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() < r - 5.0 { metal } else { Col(0, 0, 0, 0) });
        pen.rrect(cx - r + 5.0, cy - 5.0, 2.0 * r - 10.0, 10.0, 0.0, INK);
        pen.circle(cx, cy, r * 0.36, INK);
        pen.circle(cx, cy, r * 0.28, light.mix(WHITE, 0.5));
        let _ = (o, level);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, _col: Col) {
        self.textbox(pen, x - 4.0, y - 4.0, w + 8.0, h + 8.0, 8.0);
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        let t = name.to_uppercase();
        let w = pen.width(&t, size * 0.6, Face::Pixel) + 14.0;
        pen.rrect(cx - w / 2.0, y, w, size * 0.6 + 12.0, 4.0, INK);
        pen.text(&t, cx, y + 6.0, size * 0.6, WHITE, Face::Pixel, Align::Center, false);
    }
}
