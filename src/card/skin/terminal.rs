//! Terminal: a console window. A title bar with the path, green
//! monospace text on black with scan lines, bracket-cornered boxes, the
//! count with a blinking cursor, ASCII progress bars and a waiting prompt
//! as the footer.

use super::*;

pub(in super::super) struct Terminal;

const BG: Col = Col(8, 12, 10, 255);
const FG: Col = Col(96, 255, 146, 255);
const DIM: Col = Col(58, 156, 92, 255);
const FAINT: Col = Col(34, 84, 52, 255);
const AMBER: Col = Col(255, 196, 76, 255);
const BAR: Col = Col(24, 30, 28, 255);
const TITLE_H: f32 = 46.0;
const WIN: f32 = 18.0;

/// Corner brackets with a dotted edge between.
fn brackets(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, col: Col) {
    let k = 18.0f32.min(w / 4.0).min(h / 4.0);
    let t = 2.0;
    for (cx, cy, sx, sy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
        pen.line(cx, cy, cx + sx * k, cy, t, col);
        pen.line(cx, cy, cx, cy + sy * k, t, col);
    }
    let mut d = x + k + 8.0;
    while d < x + w - k - 8.0 {
        pen.rrect(d, y - 1.0, 4.0, 2.0, 0.0, FAINT);
        pen.rrect(d, y + h - 1.0, 4.0, 2.0, 0.0, FAINT);
        d += 10.0;
    }
}

impl Skin for Terminal {
    fn bg(&self, _o: &CardOptions) -> Col {
        Col(20, 22, 24, 255)
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_vgrad(0.0, 0.0, w, h, 0.0, Col(34, 38, 44, 255), Col(14, 16, 20, 255));
        let (x, y, ww, wh) = (WIN, WIN, w - 2.0 * WIN, h - 2.0 * WIN);
        pen.rrect(x + 6.0, y + 10.0, ww, wh, 14.0, Col(0, 0, 0, 120));
        pen.rrect(x, y, ww, wh, 14.0, BG);
        pen.rrect(x, y, ww, TITLE_H, 14.0, BAR);
        pen.rrect(x, y + TITLE_H - 14.0, ww, 14.0, 0.0, BAR);
        pen.line(x, y + TITLE_H, x + ww, y + TITLE_H, 1.5, FAINT);
        for (i, c) in [Col(255, 95, 86, 255), Col(255, 189, 46, 255), Col(39, 201, 63, 255)].iter().enumerate() {
            pen.circle(x + 26.0 + i as f32 * 26.0, y + TITLE_H / 2.0, 8.0, *c);
        }
        pen.glow(w / 2.0, y + TITLE_H, w * 0.6, FG.alpha(0.05));
        pen.rrect_stroke(x, y, ww, wh, 14.0, 1.5, FAINT);
    }
    fn chrome(&self, pen: &mut Pen, _o: &CardOptions, what: &str) {
        let t = format!("shiny@tracker: ~/{what}");
        pen.text(&t, pen.w / 2.0, WIN + TITLE_H / 2.0 - 10.0, 18.0, DIM, Face::Mono, Align::Center, false);
    }
    fn overlay(&self, pen: &mut Pen, _o: &CardOptions) {
        // scan lines over the screen
        let (x0, y0, x1, y1) = (WIN, WIN + TITLE_H, pen.w - WIN, pen.h - WIN);
        let s = pen.s;
        let step = (3.0 * s).max(2.0) as i32;
        pen.c.fill_with((x0 * s) as i32, (y0 * s) as i32, (x1 * s) as i32, (y1 * s) as i32, |_, py| (py % step == 0).then_some((BLACK, 0.22)));
    }
    fn inset(&self) -> f32 {
        WIN + 26.0
    }
    fn top(&self) -> f32 {
        TITLE_H
    }
    fn text(&self) -> Col {
        FG
    }
    fn soft(&self) -> Col {
        DIM
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        FG
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::MonoBold, num: Face::MonoBold, label: Face::Mono, body: Face::Mono }
    }
    fn case(&self, s: &str) -> String {
        s.to_lowercase()
    }
    fn tone(&self, c: Col) -> Col {
        if c == GOOD {
            FG
        } else if c == OK {
            AMBER
        } else {
            Col(255, 110, 100, 255)
        }
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        pen.rrect(x, y, w, h, 0.0, FG.alpha(0.03));
        brackets(pen, x, y, w, h, if role == Role::Hero || role == Role::Card { FG.alpha(0.8) } else { DIM });
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        hero_sprites(pen, d, o, x, y, w, h, FG);
        pen.text("shiny.gif", x + 14.0, y + h - 30.0, 15.0, DIM, Face::Mono, Align::Left, false);
        if normal_inset(pen, d, o, x + w - 100.0, y + h - 112.0, 86.0) {
            pen.text("normal.gif", x + w - 14.0, y + h - 30.0, 15.0, FAINT, Face::Mono, Align::Right, false);
        }
        pen.text("-rw-r--r--  1 zifu  shinies", x + 14.0, y + 12.0, 13.0, FAINT, Face::Mono, Align::Left, false);
    }
    fn chip_w(&self, pen: &Pen, text: &str) -> f32 {
        let t = format!("[{}]", text.to_lowercase());
        let w = pen.width(&t, 20.0, Face::Mono);
        w
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = format!("[{}]", text.to_lowercase());
        let w = pen.width(&t, 20.0, Face::Mono);
        let x0 = aligned(x, w, align);
        let col = match fill {
            Some(c) if c == GOOD => FG,
            Some(_) => AMBER,
            None => DIM,
        };
        pen.text(&t, x0, y + 17.0 - pen.mid(20.0, Face::Mono), 20.0, col, Face::Mono, Align::Left, false)
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let t = format!("# {}", text.to_lowercase());
        let s = pen.fit(&t, size, pen.width(text, size, Face::MonoBold) + 10.0, Face::MonoBold);
        pen.text(&t, x, y + (size - s) * 0.5, s, Col(214, 255, 226, 255), Face::MonoBold, align, false);
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let w = pen.width(text, size, Face::MonoBold);
        let x0 = aligned(x, w + size * 0.6, align);
        glow_text(pen, text, x0, y, size, FG, FG.alpha(0.6), Face::MonoBold, Align::Left);
        let on = pen.phase.map_or(true, |p| (p * 3.0).fract() < 0.5);
        if on {
            pen.rrect(x0 + w + size * 0.12, y + size * 0.18, size * 0.45, size * 0.9, 0.0, FG.alpha(0.85));
        }
    }
    fn number_w(&self, pen: &Pen, text: &str, size: f32) -> f32 {
        pen.width(text, size, Face::MonoBold) + size * 0.6
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        pen.text(&text.to_lowercase(), x, y - size * 0.1, size * 1.2, col, Face::Mono, align, false)
    }
    fn gauge_h(&self) -> f32 {
        24.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        let cw = pen.width("#", 22.0, Face::Mono).max(1.0);
        let n = ((w / cw).floor() as usize).max(6);
        let inner = n - 2;
        let lit = (frac.clamp(0.0, 1.0) * inner as f32).round() as usize;
        let bar = format!("[{}{}]", "#".repeat(lit), ".".repeat(inner - lit));
        pen.text(&bar, x, y, 22.0, col, Face::Mono, Align::Left, false);
    }
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        pen.text(&format!("$ {}", label.to_lowercase()), x + 16.0, y + 12.0, 16.0, DIM, Face::Mono, Align::Left, false);
        let vs = pen.fit(value, 28.0, w - 32.0, Face::MonoBold);
        pen.text(value, x + 16.0, y + h - vs * 1.2 - 10.0, vs, FG, Face::MonoBold, Align::Left, false);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        let mut t = String::from("$ ");
        if !name.is_empty() {
            t.push_str(&format!("whoami → {}", name.to_lowercase()));
            if let Some(lv) = level {
                t.push_str(&format!(" (lv {lv})"));
            }
        } else if !left.is_empty() {
            t.push_str(&left.to_lowercase());
        }
        let x0 = if name.is_empty() { x0 } else { x0 + pen.avatar(x0, cy, 28.0, FG) };
        let pw = pen.width(&t, 20.0, Face::Mono);
        let ty = cy - pen.mid(20.0, Face::Mono);
        let x0 = if name.is_empty() { x0 } else { x0 + pen.plate_under(x0, ty, pw, 20.0, Face::Mono) };
        let tw = pen.text(&t, x0, ty, 20.0, FG, Face::Mono, Align::Left, false);
        let on = pen.phase.map_or(true, |p| (p * 3.0).fract() < 0.5);
        if on {
            pen.rrect(x0 + tw + 6.0, cy - 11.0, 11.0, 22.0, 0.0, FG);
        }
        let bw = pen.text(&BRAND.to_lowercase(), x1, cy - pen.mid(16.0, Face::Mono), 16.0, DIM, Face::Mono, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 36.0, cy - 13.0, 26.0, 26.0);
        }
        let _ = o;
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        let s = r * 1.7;
        brackets(pen, cx - s / 2.0, cy - s / 2.0, s, s, FG);
        pen.text("lvl", cx, cy - s / 2.0 + 12.0, 16.0, DIM, Face::Mono, Align::Center, false);
        let n = level.to_string();
        let size = s * 0.42;
        let w = pen.width(&n, size, Face::MonoBold);
        let _ = o;
        glow_text(pen, &n, cx - w / 2.0, cy - size * 0.35, size, FG, FG.alpha(0.6), Face::MonoBold, Align::Left);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, _col: Col) {
        pen.rrect(x, y, w, h, 0.0, BG.alpha(0.92));
        pen.rrect_stroke(x, y, w, h, 0.0, 2.0, FG);
    }
    fn area_col(&self, _o: &CardOptions, i: usize) -> Col {
        [FG, AMBER, Col(120, 200, 255, 255)][i % 3]
    }
    fn map_px(&self, c: Col) -> Col {
        let l = (0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32) / 255.0;
        BG.mix(FG, l * 0.75)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        let t = name.to_lowercase();
        let w = pen.width(&t, size, Face::Mono) + 12.0;
        pen.rrect(cx - w / 2.0, y, w, size * 1.3, 0.0, BG);
        pen.text(&t, cx, y + size * 0.1, size, FG, Face::Mono, Align::Center, false);
    }
}
