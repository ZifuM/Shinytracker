//! Glass: frosted panels on a dark night glow, big light numbers, tracked
//! capitals and hairline gauges with a glowing tip.

use super::*;

pub(in super::super) struct Glass;

const SOFT: Col = Col(196, 202, 228, 255);

impl Skin for Glass {
    fn bg(&self, _o: &CardOptions) -> Col {
        Col(10, 12, 28, 255)
    }
    fn backdrop(&self, pen: &mut Pen, o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_with(0.0, 0.0, w, h, 0.0, |x, y| Col(16, 18, 44, 255).mix(Col(28, 14, 48, 255), (x / w + y / h) / 2.0));
        let a = o.theme.type_colour();
        pen.glow(w * 0.92, h * 0.05, w.min(h) * 0.75, a.alpha(0.42));
        pen.glow(w * 0.05, h * 0.95, w.min(h) * 0.6, Col(255, 92, 170, 255).alpha(0.26));
        pen.glow(w * 0.45, h * 0.55, w.min(h) * 0.5, Col(70, 200, 255, 255).alpha(0.12));
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        SOFT
    }
    fn accent(&self, o: &CardOptions) -> Col {
        o.theme.type_colour().mix(WHITE, 0.3)
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Bold, num: Face::Light, label: Face::Bold, body: Face::Body }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(WHITE, 0.15)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let r = match role {
            Role::Hero | Role::Card => 28.0,
            Role::Tile => 20.0,
            Role::Slot => 18.0,
        };
        pen.rrect_vgrad(x, y, w, h, r, WHITE.alpha(0.11), WHITE.alpha(0.04));
        pen.rrect_stroke(x, y, w, h, r, 1.5, WHITE.alpha(0.2));
        pen.line(x + r, y + 1.5, x + w - r, y + 1.5, 1.2, WHITE.alpha(0.4));
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = w.min(h) * 0.42;
        let a = self.accent(o);
        for (k, al) in [(1.0, 0.06), (0.85, 0.08), (0.7, 0.1), (0.55, 0.13)] {
            pen.circle(cx, cy, r * k, a.alpha(al));
        }
        pen.ring(cx, cy, r, 1.5, WHITE.alpha(0.16));
        hero_sprites(pen, d, o, x, y, w, h, WHITE);
        if normal_inset(pen, d, o, x + 22.0, y + h - 112.0, 90.0) {
            pen.ring(x + 67.0, y + h - 67.0, 50.0, 1.2, WHITE.alpha(0.3));
            tracked(pen, "NORMAL", x + 67.0, y + h - 24.0, 11.0, SOFT, Face::Bold, Align::Center, 2.0);
        }
        tracked(pen, "SHINY", x + w - 28.0, y + 26.0, 13.0, a, Face::Bold, Align::Right, 3.0);
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
        let c = fill.unwrap_or(WHITE);
        pen.rrect(x0, y, w, 34.0, 17.0, c.alpha(0.18));
        pen.rrect_stroke(x0, y, w, 34.0, 17.0, 1.5, c.alpha(0.6));
        tracked(pen, &t, x0 + 16.0, y + 17.0 - pen.mid(14.0, Face::Bold), 14.0, WHITE, Face::Bold, Align::Left, 2.5);
        w
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        tracked(pen, &text.to_uppercase(), x, y, size, col, Face::Bold, align, size * 0.16)
    }
    fn label_mid(&self, pen: &Pen, size: f32) -> f32 {
        pen.mid(size, Face::Bold)
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        glow_text(pen, text, x, y - size * 0.06, size, WHITE, WHITE.alpha(0.5), Face::Light, align);
    }
    fn gauge_h(&self) -> f32 {
        18.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        let cy = y + 9.0;
        pen.rrect(x, cy - 2.5, w, 5.0, 2.5, WHITE.alpha(0.13));
        let fx = x + w * frac.clamp(0.0, 1.0);
        if fx > x + 2.0 {
            hgrad(pen, x, cy - 2.5, fx - x, 5.0, 2.5, col.alpha(0.25), col);
        }
        for (k, a) in [(3.4, 0.07), (2.4, 0.14), (1.6, 0.25)] {
            pen.circle(fx.max(x + 3.0), cy, 5.0 * k, col.alpha(a));
        }
        pen.circle(fx.max(x + 3.0), cy, 5.0, WHITE);
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, metal: Col, light: Col) {
        for (k, a) in [(1.25, 0.08), (1.05, 0.12)] {
            pen.circle(cx, cy, r * k, light.alpha(a));
        }
        pen.ring(cx, cy, r, 3.0, metal);
        pen.ring(cx, cy, r * 0.84, 1.2, WHITE.alpha(0.3));
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.62 } else { r * 0.82 };
        self.number(pen, o, &n, cx, cy - size * 0.62, size, Align::Center);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, col: Col) {
        pen.rrect(x, y, w, h, h / 2.0, Col(20, 22, 44, 230));
        pen.rrect_stroke(x, y, w, h, h / 2.0, 2.0, col.mix(WHITE, 0.3));
    }
    fn map_px(&self, c: Col) -> Col {
        c.mix(Col(20, 24, 60, 255), 0.35)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        halo(pen, name, cx, y, size, Col(10, 12, 30, 200), WHITE, Face::Bold, Align::Center, 2.5);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        pen.line(x0, y + 4.0, x1, y + 4.0, 1.0, WHITE.alpha(0.18));
        let mut x = x0;
        if !left.is_empty() {
            x += self.label(pen, o, left, x, cy - self.label_mid(pen, 12.0), 12.0, SOFT, Align::Left) + 14.0;
        }
        if !name.is_empty() {
            x += pen.avatar(x, cy, 32.0, WHITE);
            let pw = pen.width(name, 21.0, Face::Body);
            let ny = cy - pen.mid(21.0, Face::Body);
            let off = pen.plate_under(x, ny, pw, 21.0, Face::Body);
            x += off;
            x += pen.text(name, x, ny, 21.0, WHITE, Face::Body, Align::Left, false) + 14.0;
            x += off;
        }
        if let Some(lv) = level {
            self.chip(pen, o, &format!("LV {lv}"), x, cy - 17.0, Some(self.accent(o)), Align::Left);
        }
        let bw = self.label(pen, o, BRAND, x1, cy - self.label_mid(pen, 12.0), 12.0, SOFT, Align::Right);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 38.0, cy - 14.0, 28.0, 28.0);
        }
    }
}
