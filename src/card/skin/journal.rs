//! Field journal: a notebook page. Ruled paper with a red margin and
//! spiral holes, the shiny in a taped-on polaroid, everything in
//! handwriting, stats on sticky notes, a hatched pencil gauge, circled
//! labels and a signature in the footer.

use super::*;

pub(in super::super) struct Journal;

const PAPER: Col = Col(251, 246, 233, 255);
const PEN_C: Col = Col(32, 44, 96, 255);
const PENCIL: Col = Col(98, 98, 106, 255);
const RED: Col = Col(206, 52, 52, 255);
const RULE: Col = Col(160, 192, 228, 255);
const MARGIN: Col = Col(232, 128, 128, 255);
const STICKY: Col = Col(255, 232, 128, 255);
const HIGHLIGHT: Col = Col(255, 232, 80, 255);

fn wob(seed: u32, i: u32) -> f32 {
    let mut s = seed.wrapping_mul(2654435761) ^ i.wrapping_mul(40503).wrapping_add(0x9e37);
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    (s % 2001) as f32 / 1000.0 - 1.0
}

/// A hand-drawn line.
pub(super) fn scribble(pen: &mut Pen, a: (f32, f32), b: (f32, f32), w: f32, col: Col) {
    let seed = (a.0 * 7.0 + a.1 * 13.0 + b.0 * 3.0) as i32 as u32;
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let n = (len / 22.0).ceil().max(2.0) as u32;
    let (px, py) = (-dy / len, dx / len);
    let mut prev = a;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let k = if i == n { 0.3 } else { 1.0 };
        let q = (a.0 + dx * t + px * wob(seed, i) * 1.6 * k, a.1 + dy * t + py * wob(seed, i) * 1.6 * k);
        pen.line(prev.0, prev.1, q.0, q.1, w, col);
        prev = q;
    }
}

fn scribble_box(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, t: f32, col: Col) {
    let o = 3.0;
    scribble(pen, (x - o, y), (x + w + o * 0.5, y), t, col);
    scribble(pen, (x + w, y - o * 0.5), (x + w, y + h + o), t, col);
    scribble(pen, (x + w + o, y + h), (x - o * 0.5, y + h), t, col);
    scribble(pen, (x, y + h + o * 0.5), (x, y - o), t, col);
}

fn scribble_oval(pen: &mut Pen, cx: f32, cy: f32, rx: f32, ry: f32, t: f32, col: Col) {
    let seed = (cx * 3.0 + cy) as i32 as u32;
    let n = 36;
    let mut prev: Option<(f32, f32)> = None;
    for i in 0..=n + 4 {
        let a = i as f32 / n as f32 * std::f32::consts::TAU - 0.5;
        let k = 1.0 + wob(seed, i) * 0.03 + i as f32 * 0.0025;
        let q = (cx + a.cos() * rx * k, cy + a.sin() * ry * k);
        if let Some(p) = prev {
            pen.line(p.0, p.1, q.0, q.1, t, col);
        }
        prev = Some(q);
    }
}

/// A turned rectangle with a soft shadow.
fn card(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, deg: f32, fill: Col) {
    pen.poly(&turned(x + 5.0, y + 7.0, w, h, deg), Col(0, 0, 0, 34));
    pen.poly(&turned(x, y, w, h, deg), fill);
}

fn tape(pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, w: f32, deg: f32) {
    pen.poly(&turned(cx - w / 2.0, cy - 14.0, w, 28.0, deg), o.theme.type_colour().mix(WHITE, 0.45).alpha(0.65));
}

impl Skin for Journal {
    fn bg(&self, _o: &CardOptions) -> Col {
        PAPER
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        let mut y = 70.0;
        while y < h - 10.0 {
            pen.line(0.0, y, w, y, 1.4, RULE.alpha(0.6));
            y += 36.0;
        }
        pen.line(54.0, 0.0, 54.0, h, 1.6, MARGIN.alpha(0.7));
        let mut hy = 40.0;
        while hy < h - 20.0 {
            pen.circle(24.0, hy, 8.0, Col(60, 58, 66, 255).alpha(0.85));
            hy += 52.0;
        }
        // a coffee ring
        pen.ring(w - 150.0, 120.0, 70.0, 7.0, Col(150, 100, 60, 255).alpha(0.07));
        pen.ring(w - 146.0, 124.0, 66.0, 2.0, Col(150, 100, 60, 255).alpha(0.08));
    }
    fn inset(&self) -> f32 {
        44.0
    }
    fn text(&self) -> Col {
        PEN_C
    }
    fn soft(&self) -> Col {
        PENCIL
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        RED
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Hand, num: Face::Hand, label: Face::Hand, body: Face::Hand }
    }
    fn case(&self, s: &str) -> String {
        s.to_string()
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(PEN_C, 0.35)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        match role {
            Role::Tile => card(pen, x, y, w, h, if (x as i32 / 7) % 2 == 0 { -1.2 } else { 1.0 }, STICKY),
            Role::Slot => card(pen, x, y, w, h, if (x as i32 / 5) % 2 == 0 { -1.5 } else { 1.3 }, WHITE),
            Role::Card => {
                pen.rrect(x, y, w, h, 4.0, PAPER.mix(WHITE, 0.5));
                scribble_box(pen, x, y, w, h, 2.0, PEN_C.alpha(0.75));
            }
            Role::Hero => card(pen, x, y, w, h, -2.0, WHITE),
        }
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        // a polaroid, taped on
        let (px, py, pw, ph) = (x + 10.0, y + 14.0, w - 20.0, h - 24.0);
        card(pen, px, py, pw, ph, -3.0, WHITE);
        let (fx, fy, fw, fh) = (px + 22.0, py + 22.0, pw - 44.0, ph - 100.0);
        pen.poly(&turned(fx, fy, fw, fh, -3.0), Col(222, 232, 242, 255));
        hero_sprites(pen, d, o, fx, fy, fw, fh, Col(240, 180, 40, 255));
        let cap = d.nickname.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| d.name.clone());
        let cs = pen.fit(&cap, 34.0, fw, Face::Hand);
        pen.text(&cap, px + pw / 2.0 + 2.0, fy + fh + (ph - fh - 22.0 - cs) / 2.0 - 4.0, cs, PEN_C, Face::Hand, Align::Center, false);
        tape(pen, o, px + pw / 2.0, py + 4.0, 130.0, 5.0);
        if o.show_normal {
            if let Some(img) = &d.normal {
                let (nx, ny) = (x + w - 70.0, y + h - 112.0);
                card(pen, nx - 50.0, ny - 46.0, 100.0, 104.0, 4.0, WHITE);
                sprite(pen, img, None, nx, ny - 4.0, 80.0, 70.0, true);
                pen.text("normal", nx, ny + 32.0, 16.0, PENCIL, Face::Hand, Align::Center, false);
                tape(pen, o, nx, ny - 46.0, 60.0, -8.0);
            }
        }
    }
    fn chip_w(&self, pen: &Pen, text: &str) -> f32 {
        let w = pen.width(text, 24.0, Face::Hand) + 30.0;
        w
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let w = pen.width(text, 24.0, Face::Hand) + 30.0;
        let x0 = aligned(x, w, align);
        if let Some(c) = fill {
            let c = if c == GOOD { GOOD } else { HIGHLIGHT };
            pen.rrect(x0 + 4.0, y + 6.0, w - 8.0, 24.0, 2.0, c.alpha(0.45));
        }
        let col = if fill.is_some() { RED } else { PEN_C };
        pen.text(text, x0 + 15.0, y + 17.0 - pen.mid(24.0, Face::Hand), 24.0, col, Face::Hand, Align::Left, false);
        scribble_oval(pen, x0 + w / 2.0, y + 17.0, w / 2.0, 19.0, 1.8, col.alpha(0.85));
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let w = pen.text(text, x, y, size, PEN_C, Face::Hand, align, false);
        let x0 = aligned(x, w, align);
        scribble(pen, (x0, y + size * 1.12), (x0 + w + 8.0, y + size * 1.1), 3.0, RED);
        scribble(pen, (x0 + 10.0, y + size * 1.2), (x0 + w - 6.0, y + size * 1.19), 2.0, RED);
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let w = pen.text(text, x, y - size * 0.06, size * 1.1, PEN_C, Face::Hand, align, false);
        let x0 = aligned(x, w, align);
        scribble_oval(pen, x0 + w / 2.0, y + size * 0.55, w / 2.0 + 24.0, size * 0.58, 2.4, RED.alpha(0.8));
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        let t = text.to_lowercase();
        pen.text(&t, x, y - size * 0.15, size * 1.35, col, Face::Hand, align, false)
    }
    fn label_mid(&self, pen: &Pen, size: f32) -> f32 {
        pen.mid(size * 1.35, Face::Hand) - size * 0.15
    }
    fn gauge_h(&self) -> f32 {
        22.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        let end = x + w * frac.clamp(0.0, 1.0);
        let h = 22.0;
        let mut k = x - h;
        while k < end {
            let (mut a, mut b) = ((k, y + h), (k + h, y));
            if a.0 < x {
                a = (x, y + h - (x - a.0));
            }
            if b.0 > end {
                b = (end, y + (b.0 - end));
            }
            if b.0 > a.0 {
                pen.line(a.0, a.1, b.0, b.1, 2.0, col.alpha(0.85));
            }
            k += 7.0;
        }
        scribble_box(pen, x, y, w, h, 2.2, PEN_C);
    }
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        pen.text(&label.to_lowercase(), x + 16.0, y + 8.0, 21.0, PENCIL, Face::Hand, Align::Left, false);
        let vs = pen.fit(value, 32.0, w - 30.0, Face::Hand);
        pen.text(value, x + 16.0, y + h - vs * 1.25 - 6.0, vs, PEN_C, Face::Hand, Align::Left, false);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        let sig = match (name.is_empty(), level) {
            (true, _) => String::new(),
            (false, Some(lv)) => format!("— {name}  (lv {lv})"),
            (false, None) => format!("— {name}"),
        };
        let mut x = x0;
        if !left.is_empty() && !sig.is_empty() {
            x += pen.text(&format!("{}:", left.to_lowercase()), x, cy - pen.mid(22.0, Face::Hand), 22.0, PENCIL, Face::Hand, Align::Left, false) + 10.0;
        }
        let bw = pen.width(BRAND, 21.0, Face::Hand);
        if !sig.is_empty() {
            x += pen.avatar(x, cy, 34.0, PEN_C);
            // a long name gets smaller rather than running into the app's name
            let ss = pen.fit_name(&sig, 32.0, Face::Hand, x1 - bw - 60.0 - x);
            let ty = cy - pen.mid(ss, Face::Hand);
            let sw = pen.width(&sig, ss, Face::Hand);
            let off = pen.plate_under_k(x, ty, sw, ss, Face::Hand, 1.5);
            let ink = if off > 0.0 { WHITE } else { PEN_C };
            pen.text(&sig, x + off, ty, ss, ink, Face::Hand, Align::Left, false);
        }
        let bw = pen.text(BRAND, x1, cy - pen.mid(21.0, Face::Hand), 21.0, PENCIL, Face::Hand, Align::Right, false).max(bw);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 36.0, cy - 13.0, 26.0, 26.0);
        }
        let _ = o;
    }
    fn badge(&self, pen: &mut Pen, _o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, metal: Col, _light: Col) {
        // a gold sticker star
        let pts: Vec<(f32, f32)> = (0..10)
            .map(|i| {
                let a = (i as f32 * 36.0 - 90.0).to_radians();
                let rr = if i % 2 == 0 { r } else { r * 0.52 };
                (cx + a.cos() * rr, cy + a.sin() * rr)
            })
            .collect();
        let shadow: Vec<(f32, f32)> = pts.iter().map(|p| (p.0 + 4.0, p.1 + 6.0)).collect();
        pen.poly(&shadow, Col(0, 0, 0, 34));
        // a star isn't convex: fill it as a centre fan
        for i in 0..10 {
            pen.poly(&[(cx, cy), pts[i], pts[(i + 1) % 10]], metal);
        }
        let n = level.to_string();
        let s = r * 0.6;
        pen.text(&n, cx, cy - s * 0.62, s, PEN_C, Face::Hand, Align::Center, false);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, _col: Col) {
        card(pen, x, y, w, h, -2.0, STICKY);
    }
    fn map_px(&self, c: Col) -> Col {
        let l = (0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32) / 255.0;
        Col(120, 90, 60, 255).mix(Col(250, 240, 220, 255), l).mix(c, 0.35)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        halo(pen, name, cx, y - 4.0, size * 1.3, PAPER, PEN_C, Face::Hand, Align::Center, 2.5);
    }
}
