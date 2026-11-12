//! Broadcast: an esports scoreboard. A dark arena with light beams,
//! slanted plates with an accent top edge, the shiny in a team-logo box,
//! the title on an angled name plate, the count in a score box, a
//! possession-style gauge and a white ticker strip as the footer.

use super::*;

pub(in super::super) struct Broadcast;

const DARK: Col = Col(12, 14, 24, 255);
const PLATE: Col = Col(28, 32, 50, 255);
const SOFT: Col = Col(160, 168, 196, 255);
const LIVE: Col = Col(232, 36, 52, 255);
const LEAN: f32 = 14.0;

fn slant(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, lean: f32, c: Col) {
    pen.poly(&[(x + lean, y), (x + w, y), (x + w - lean, y + h), (x, y + h)], c);
}

fn slant_grad(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, lean: f32, a: Col, b: Col) {
    let n = 12;
    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let t1 = (i + 1) as f32 / n as f32;
        let (ya, yb) = (y + h * t0, y + h * t1);
        let la = lean * (1.0 - t0);
        let lb = lean * (1.0 - t1);
        pen.poly(&[(x + la, ya), (x + w - lean + la, ya), (x + w - lean + lb, yb + 0.5), (x + lb, yb + 0.5)], a.mix(b, (t0 + t1) / 2.0));
    }
}

/// Impact with a hard shadow.
#[allow(clippy::too_many_arguments)]
fn sport(pen: &mut Pen, s: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
    pen.text(s, x + 3.0, y + 3.0, size, Col(0, 0, 0, 150), Face::Impact, align, false);
    pen.text(s, x, y, size, col, Face::Impact, align, false)
}

impl Skin for Broadcast {
    fn bg(&self, _o: &CardOptions) -> Col {
        DARK
    }
    fn backdrop(&self, pen: &mut Pen, o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_vgrad(0.0, 0.0, w, h, 0.0, Col(30, 34, 58, 255), Col(8, 10, 18, 255));
        for (x0, wd) in [(w * 0.1, 160.0), (w * 0.45, 110.0), (w * 0.78, 200.0)] {
            pen.poly(&[(x0, 0.0), (x0 + wd, 0.0), (x0 + wd - h * 0.5, h), (x0 - h * 0.5 - wd, h)], WHITE.alpha(0.025));
        }
        let mut x = -h;
        while x < w {
            pen.line(x, h, x + h, 0.0, 10.0, WHITE.alpha(0.012));
            x += 44.0;
        }
        pen.glow(w * 0.85, 0.0, w * 0.5, o.theme.type_colour().alpha(0.22));
        pen.rrect(0.0, 0.0, w, 6.0, 0.0, o.theme.type_colour());
    }
    fn inset(&self) -> f32 {
        44.0
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        SOFT
    }
    fn accent(&self, o: &CardOptions) -> Col {
        o.theme.type_colour().mix(WHITE, 0.08)
    }
    fn title_pad(&self) -> f32 {
        34.0
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Impact, num: Face::Impact, label: Face::Impact, body: Face::Bold }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(WHITE, 0.12)
    }
    fn panel(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let lean = if role == Role::Tile || role == Role::Slot { LEAN } else { 0.0 };
        slant(pen, x + 4.0, y + 4.0, w, h, lean, Col(0, 0, 0, 110));
        slant_grad(pen, x, y, w, h, lean, PLATE, PLATE.mix(DARK, 0.6));
        slant(pen, x, y, w, 4.0, lean * 4.0 / h.max(4.0), self.accent(o));
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        let a = self.accent(o);
        pen.rrect(x + 5.0, y + 6.0, w, h, 6.0, Col(0, 0, 0, 120));
        pen.rrect_vgrad(x, y, w, h, 6.0, a.mix(DARK, 0.3), a.mix(DARK, 0.75));
        let mut k = x - h;
        while k < x + w {
            let (x0, x1) = (k.max(x), (k + h).min(x + w));
            if x1 > x0 {
                let (y0, y1) = (y + h - (x0 - k), y + h - (x1 - k));
                pen.line(x0, y0, x1, y1, 12.0, WHITE.alpha(0.04));
            }
            k += 34.0;
        }
        pen.rrect_stroke(x, y, w, h, 6.0, 3.0, a);
        hero_sprites(pen, d, o, x, y, w, h, WHITE);
        if normal_inset(pen, d, o, x + 14.0, y + h - 106.0, 92.0) {
            sport(pen, "NORMAL", x + 60.0, y + h - 24.0, 15.0, SOFT, Align::Center);
        }
        // LIVE tag
        slant(pen, x + w - 128.0, y + 16.0, 112.0, 34.0, 10.0, LIVE);
        let on = pen.phase.map_or(true, |p| (p * 2.0).fract() < 0.5);
        pen.circle(x + w - 104.0, y + 33.0, 6.0, if on { WHITE } else { WHITE.alpha(0.3) });
        pen.text("LIVE", x + w - 92.0, y + 19.0, 22.0, WHITE, Face::Impact, Align::Left, false);
    }
    fn chip(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = pen.width(&t, 20.0, Face::Impact) + 40.0;
        let x0 = aligned(x, w, align);
        let c = fill.unwrap_or(WHITE);
        slant(pen, x0, y, w, 34.0, 10.0, c);
        let ink = if fill.is_none() || c == GOOD { DARK } else { WHITE };
        let _ = o;
        pen.text(&t, x0 + w / 2.0, y + 4.0, 20.0, ink, Face::Impact, Align::Center, false);
        w
    }
    fn title(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let w = pen.width(text, size, Face::Impact) + 44.0;
        let x0 = aligned(x, w, align) - if align == Align::Left { 4.0 } else { 0.0 };
        let a = self.accent(o);
        slant(pen, x0 + 5.0, y + 5.0, w, size * 1.22, LEAN, Col(0, 0, 0, 130));
        slant_grad(pen, x0, y, w, size * 1.22, LEAN, a.mix(WHITE, 0.15), a);
        sport(pen, text, x0 + w / 2.0, y + size * 0.08, size, WHITE, Align::Center);
    }
    fn number(&self, pen: &mut Pen, o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let w = pen.width(text, size, Face::Impact) + 48.0;
        let x0 = aligned(x, w, align);
        pen.rrect_vgrad(x0, y - 4.0, w, size * 1.08, 4.0, PLATE, DARK);
        pen.rrect(x0, y - 4.0 + size * 1.08 - 4.0, w, 4.0, 0.0, self.accent(o));
        sport(pen, text, x0 + w / 2.0, y + size * 0.05, size, WHITE, Align::Center);
    }
    fn number_w(&self, pen: &Pen, text: &str, size: f32) -> f32 {
        pen.width(text, size, Face::Impact) + 48.0
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        pen.text(&text.to_uppercase(), x, y - size * 0.15, size * 1.25, col, Face::Impact, align, false)
    }
    fn gauge_h(&self) -> f32 {
        22.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        slant(pen, x, y, w, 22.0, 12.0, PLATE);
        let fw = (w * frac.clamp(0.0, 1.0)).max(26.0);
        slant_grad(pen, x, y, fw, 22.0, 12.0, col.mix(WHITE, 0.25), col);
    }
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        pen.text(&label.to_uppercase(), x + w / 2.0, y + 14.0, 17.0, SOFT, Face::Impact, Align::Center, false);
        let vs = pen.fit(value, 32.0, w - 40.0, Face::Impact).min((h - 44.0) / 1.2);
        sport(pen, value, x + w / 2.0, y + h - vs * 1.25 - 6.0, vs, WHITE, Align::Center);
    }
    /// The ticker: a white strip with an INFO tag.
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let (y, h) = (y + 8.0, h - 8.0);
        let _ = x0;
        let (sx0, sx1) = (0.0, pen.w);
        pen.rrect(sx0, y, sx1 - sx0, h, 0.0, WHITE.alpha(0.95));
        let tag_w = 150.0;
        slant(pen, sx0, y, tag_w + 20.0, h, 0.0, self.accent(o));
        pen.poly(&[(sx0 + tag_w, y), (sx0 + tag_w + 24.0, y), (sx0 + tag_w, y + h)], self.accent(o));
        pen.text(&if left.is_empty() { "INFO".to_string() } else { left.to_uppercase() }, sx0 + tag_w / 2.0 + 8.0, y + h / 2.0 - 14.0, 24.0, WHITE, Face::Impact, Align::Center, false);
        let mut text = String::new();
        if !name.is_empty() {
            text.push_str(&name.to_uppercase());
        }
        if let Some(lv) = level {
            text.push_str(&format!("  ·  LV {lv}"));
        }
        let x = sx0 + tag_w + 44.0;
        pen.text(&text, x, y + h / 2.0 - 14.0, 24.0, DARK, Face::Impact, Align::Left, false);
        let bw = pen.text(&BRAND.to_uppercase(), x1, y + h / 2.0 - 11.0, 19.0, DARK.alpha(0.7), Face::Impact, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 38.0, y + h / 2.0 - 14.0, 28.0, 28.0);
        }
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        let a = self.accent(o);
        let s = r * 1.6;
        slant(pen, cx - s / 2.0 + 6.0, cy - s / 2.0 + 6.0, s, s, 22.0, Col(0, 0, 0, 120));
        slant_grad(pen, cx - s / 2.0, cy - s / 2.0, s, s, 22.0, a.mix(WHITE, 0.2), a.mix(DARK, 0.4));
        sport(pen, "LV", cx, cy - s / 2.0 + 10.0, 24.0, WHITE.alpha(0.8), Align::Center);
        let n = level.to_string();
        let size = s * 0.5;
        sport(pen, &n, cx, cy - size * 0.35, size, WHITE, Align::Center);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, col: Col) {
        slant(pen, x - 8.0, y, w + 16.0, h, 10.0, PLATE);
        slant(pen, x - 8.0, y, w + 16.0, 4.0, 1.0, col);
    }
    fn map_px(&self, c: Col) -> Col {
        c.mix(Col(14, 18, 34, 255), 0.4)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        let t = name.to_uppercase();
        let w = pen.width(&t, size, Face::Impact) + 18.0;
        slant(pen, cx - w / 2.0, y, w, size * 1.3, 6.0, WHITE);
        pen.text(&t, cx, y + size * 0.05, size, DARK, Face::Impact, Align::Center, false);
    }
}
