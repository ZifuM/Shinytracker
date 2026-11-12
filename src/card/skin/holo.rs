//! Holo card: the shiny as a collectible card on dark velvet. A gold
//! border, a name bar with the odds as "HP", an art window with a rainbow
//! foil that sweeps across in the animated version, two "moves" and the
//! count as the card number ("6,642 / 4,096"); gold-edged panels round it.
//! An original card design.

use super::*;

pub(in super::super) struct Holo;

const GOLD: Col = Col(244, 196, 64, 255);
const GOLD_HI: Col = Col(255, 232, 140, 255);
const GOLD_LO: Col = Col(184, 132, 34, 255);
const CARD_INK: Col = Col(28, 26, 34, 255);
const SOFT: Col = Col(232, 214, 168, 255);

fn rainbow(h: f32) -> Col {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let s = |v: f32| (150.0 + v * 105.0) as u8;
    Col(s(r), s(g), s(b), 255)
}

fn gold_frame(pen: &mut Pen, x: f32, y: f32, w: f32, h: f32, r: f32) {
    pen.rrect(x + 5.0, y + 8.0, w, h, r, Col(0, 0, 0, 90));
    pen.rrect_vgrad(x, y, w, h, r, GOLD_HI, GOLD_LO);
    pen.rrect_stroke(x, y, w, h, r, 1.5, GOLD_LO.mix(BLACK, 0.3));
}

impl Holo {
    /// The card face in `x, y, w, h`.
    #[allow(clippy::too_many_arguments)]
    fn card(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        let type_c = o.theme.type_colour();
        gold_frame(pen, x, y, w, h, 22.0);
        let b = 14.0;
        let (ix, iy, iw, ih) = (x + b, y + b, w - 2.0 * b, h - 2.0 * b);
        pen.rrect_vgrad(ix, iy, iw, ih, 12.0, type_c.mix(WHITE, 0.82), type_c.mix(WHITE, 0.62));
        // name bar
        let name = d.nickname.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| d.name.clone());
        let mut nx = ix + 14.0;
        if let Some(dex) = d.dex {
            let t = format!("No.{dex}");
            let tw = pen.width(&t, 13.0, Face::Bold) + 14.0;
            pen.rrect(nx, iy + 14.0, tw, 22.0, 4.0, type_c.mix(BLACK, 0.3));
            pen.text(&t, nx + 7.0, iy + 17.0, 13.0, WHITE, Face::Bold, Align::Left, false);
            nx += tw + 10.0;
        }
        let (mult, col) = match d.odds_one_in {
            Some(odds) => {
                let n = match (d.odds_on_target, d.target_seen) {
                    (true, Some(ts)) => ts,
                    _ => d.encounters,
                };
                let m = n as f64 / odds;
                (Some(m), if m < 0.9 { GOOD } else if m <= 1.1 { OK } else { BAD })
            }
            None => (None, GOOD),
        };
        let mut right = ix + iw - 14.0;
        if let Some(m) = mult {
            pen.circle(right - 13.0, iy + 25.0, 14.0, Col(0, 0, 0, 110));
            pen.circle(right - 13.0, iy + 24.0, 13.0, col);
            pen.sparkle(right - 13.0, iy + 24.0, 7.0, WHITE);
            right -= 34.0;
            let mt = fmt_mult(m);
            let mw = pen.text(&mt, right, iy + 10.0, 30.0, CARD_INK, Face::Black, Align::Right, false);
            right -= mw + 4.0;
            right -= pen.text("ODDS", right, iy + 22.0, 12.0, CARD_INK.alpha(0.75), Face::Bold, Align::Right, false) + 8.0;
        }
        let ns = pen.fit(&name, 30.0, right - nx, Face::Black);
        pen.text(&clip(pen, &name, ns, Face::Black, right - nx), nx, iy + 10.0 + (30.0 - ns) * 0.6, ns, CARD_INK, Face::Black, Align::Left, false);
        // art window with foil
        let (ax, ay, aw) = (ix + 16.0, iy + 54.0, iw - 32.0);
        let ah = (ih * 0.48).min(aw * 0.78);
        pen.rrect(ax - 4.0, ay - 4.0, aw + 8.0, ah + 8.0, 4.0, GOLD_LO);
        pen.rrect(ax - 2.0, ay - 2.0, aw + 4.0, ah + 4.0, 3.0, GOLD_HI);
        let shift = pen.phase.unwrap_or(0.0);
        pen.rrect_with(ax, ay, aw, ah, 2.0, |px, py| {
            let base = type_c.mix(WHITE, 0.45).mix(Col(30, 24, 60, 255), (py - ay) / ah * 0.45);
            let t = ((px - ax) + (ah - (py - ay))) / 120.0 + shift;
            base.mix(rainbow(t / 3.0), 0.3)
        });
        // a glint that sweeps across in the animation
        let gx = ax - 80.0 + ((shift * 1.0) % 1.0) * (aw + 160.0);
        let glint: Vec<(f32, f32)> = vec![(gx.max(ax), ay + ah), ((gx + 30.0).clamp(ax, ax + aw), ay + ah), ((gx + 30.0 + ah * 0.5).clamp(ax, ax + aw), ay), ((gx + ah * 0.5).clamp(ax, ax + aw), ay)];
        if pen.phase.is_some() || shift == 0.0 {
            pen.poly(&glint, WHITE.alpha(0.28));
        }
        pen.glow(ax + aw / 2.0, ay + ah / 2.0, ah * 0.45, WHITE.alpha(0.3));
        hero_sprites(pen, d, o, ax, ay, aw, ah, WHITE);
        if o.show_normal {
            if let Some(img) = &d.normal {
                let (cx, cy) = (ax + aw - 40.0, ay + ah - 40.0);
                pen.circle(cx, cy, 32.0, WHITE.alpha(0.6));
                pen.ring(cx, cy, 32.0, 2.0, GOLD_LO);
                sprite(pen, img, None, cx, cy, 52.0, 52.0, true);
            }
        }
        // info strip
        let mut yy = ay + ah + 12.0;
        if !d.method.is_empty() {
            pen.rrect_vgrad(ax + 10.0, yy, aw - 20.0, 24.0, 3.0, GOLD_HI, GOLD.mix(GOLD_LO, 0.5));
            let ms = pen.fit(&d.method, 13.0, aw - 40.0, Face::Body);
            pen.text(&d.method, ax + aw / 2.0, yy + (24.0 - ms * 1.2) / 2.0, ms, CARD_INK, Face::Body, Align::Center, false);
            yy += 34.0;
        }
        // two "moves"
        let mut moves: Vec<(usize, &str, String)> = Vec::new();
        if d.hunt_secs >= 60 {
            moves.push((1, "Patience", fmt_duration(d.hunt_secs)));
        }
        if let Some(r) = d.per_hour {
            moves.push((2, "Quick Search", format!("{}/h", thousands(r.round() as u64))));
        }
        let num_y = iy + ih - 50.0;
        for (cost, mv, val) in moves {
            if yy + 40.0 > num_y {
                break;
            }
            for k in 0..cost {
                pen.circle(ax + 12.0 + k as f32 * 22.0, yy + 18.0, 9.0, type_c.mix(BLACK, 0.1));
                pen.circle(ax + 9.0 + k as f32 * 22.0, yy + 15.0, 3.0, WHITE.alpha(0.6));
            }
            pen.text(mv, ax + 62.0, yy + 6.0, 21.0, CARD_INK, Face::Black, Align::Left, false);
            pen.text(&val, ax + aw, yy + 6.0, 21.0, CARD_INK, Face::Black, Align::Right, false);
            pen.line(ax, yy + 40.0, ax + aw, yy + 40.0, 1.2, CARD_INK.alpha(0.15));
            yy += 46.0;
        }
        // the card number
        let (by, bh) = (iy + ih - 44.0, 34.0);
        pen.rrect_vgrad(ix + 10.0, by, iw - 20.0, bh, 6.0, type_c.mix(BLACK, 0.25), type_c.mix(BLACK, 0.45));
        let total = d.odds_one_in.map(|o| thousands(o.round() as u64)).unwrap_or_else(|| "?".into());
        let nw = pen.text(&thousands(d.encounters), ix + 24.0, by + 3.0, 25.0, WHITE, Face::Black, Align::Left, false);
        pen.text(&format!("/ {total}"), ix + 30.0 + nw, by + 10.0, 15.0, WHITE.alpha(0.75), Face::Bold, Align::Left, false);
        pen.sparkle(ix + iw - 32.0, by + bh / 2.0, 9.0, GOLD_HI);
    }
}

impl Skin for Holo {
    fn bg(&self, o: &CardOptions) -> Col {
        o.theme.type_colour().mix(Col(12, 8, 22, 255), 0.82)
    }
    fn backdrop(&self, pen: &mut Pen, o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        let base = self.bg(o);
        pen.rrect_with(0.0, 0.0, w, h, 0.0, |x, y| base.mix(Col(4, 2, 10, 255), (((x - w / 2.0) / w).powi(2) + ((y - h / 2.0) / h).powi(2)).sqrt()));
        // a faint diamond pattern
        let mut yy = 0.0;
        let mut row = 0;
        while yy < h {
            let mut xx = if row % 2 == 0 { 0.0 } else { 30.0 };
            while xx < w {
                pen.poly(&[(xx, yy - 6.0), (xx + 6.0, yy), (xx, yy + 6.0), (xx - 6.0, yy)], GOLD.alpha(0.05));
                xx += 60.0;
            }
            yy += 30.0;
            row += 1;
        }
        pen.glow(w * 0.3, h * 0.4, w.min(h) * 0.7, o.theme.type_colour().alpha(0.18));
    }
    fn text(&self) -> Col {
        WHITE
    }
    fn soft(&self) -> Col {
        SOFT
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        GOLD
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Black, num: Face::Display, label: Face::Bold, body: Face::Body }
    }
    fn portrait(&self) -> bool {
        true
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let r = if role == Role::Slot || role == Role::Tile { 12.0 } else { 18.0 };
        pen.rrect(x, y, w, h, r, Col(0, 0, 0, 90));
        pen.rrect_stroke(x, y, w, h, r, 3.0, GOLD_LO);
        pen.rrect_stroke(x + 3.0, y + 3.0, w - 6.0, h - 6.0, r - 3.0, 1.2, GOLD_HI.alpha(0.7));
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        // card proportions (5:7), centred in the space
        let cw = w.min(h * 5.0 / 7.0);
        let ch = (cw * 7.0 / 5.0).min(h);
        self.card(pen, d, o, x + (w - cw) / 2.0, y + (h - ch) / 2.0, cw, ch);
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = pen.width(&t, 15.0, Face::Bold) + 30.0;
        let x0 = aligned(x, w, align);
        match fill {
            Some(c) if c != GOLD => {
                pen.rrect(x0, y, w, 34.0, 8.0, c.mix(BLACK, 0.15));
                pen.rrect_stroke(x0, y, w, 34.0, 8.0, 2.0, GOLD_HI);
            }
            _ => {
                pen.rrect_vgrad(x0, y, w, 34.0, 8.0, GOLD_HI, GOLD_LO);
            }
        }
        let ink = if fill.is_some_and(|c| c != GOLD) { WHITE } else { CARD_INK };
        pen.text(&t, x0 + 15.0, y + 8.5, 15.0, ink, Face::Bold, Align::Left, false);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        pen.text(text, x + 3.0, y + 4.0, size, GOLD_LO.mix(BLACK, 0.3), Face::Black, align, false);
        pen.text(text, x, y, size, WHITE, Face::Black, align, false);
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        // gold lettering: a dark drop, a deep rim, a bright face
        pen.text(text, x + 3.0, y + 5.0, size, Col(0, 0, 0, 140), Face::Display, align, false);
        pen.text(text, x + 1.5, y + 2.0, size, GOLD_LO, Face::Display, align, false);
        pen.text(text, x, y, size, GOLD, Face::Display, align, false);
        pen.text(text, x - 0.8, y - 1.2, size, GOLD_HI.alpha(0.55), Face::Display, align, false);
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        pen.rrect(x - 3.0, y - 3.0, w + 6.0, 30.0, 15.0, GOLD_LO);
        pen.rrect(x, y, w, 24.0, 12.0, Col(20, 14, 30, 255));
        let fw = (w * frac.clamp(0.0, 1.0)).max(24.0);
        let shift = pen.phase.unwrap_or(0.0);
        pen.rrect_with(x, y, fw, 24.0, 12.0, |px, _| col.mix(rainbow((px - x) / 400.0 + shift), 0.35));
        pen.rrect(x + 10.0, y + 4.0, (fw - 20.0).max(0.0), 5.0, 2.5, WHITE.alpha(0.45));
    }
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        pen.text(&label.to_uppercase(), x + 16.0, y + 14.0, 13.0, GOLD, Face::Bold, Align::Left, false);
        let vs = pen.fit(value, 30.0, w - 32.0, Face::Black);
        pen.text(value, x + 16.0, y + h - vs * 1.2 - 10.0, vs, WHITE, Face::Black, Align::Left, false);
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, metal: Col, light: Col) {
        pen.circle(cx + 4.0, cy + 6.0, r, Col(0, 0, 0, 100));
        pen.circle(cx, cy, r, GOLD_LO);
        pen.circle(cx, cy, r - 5.0, metal.mix(light, 0.3));
        let shift = pen.phase.unwrap_or(0.0);
        let rr = r - 10.0;
        pen.rrect_with(cx - rr, cy - rr, 2.0 * rr, 2.0 * rr, rr, |px, py| metal.mix(rainbow((px + py) / 300.0 + shift), 0.3));
        let n = level.to_string();
        let size = if n.len() >= 3 { r * 0.6 } else { r * 0.8 };
        let _ = o;
        pen.text(&n, cx + 3.0, cy - size * 0.58 + 4.0, size, Col(0, 0, 0, 120), Face::Black, Align::Center, false);
        pen.text(&n, cx, cy - size * 0.58, size, WHITE, Face::Black, Align::Center, false);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, _col: Col) {
        pen.rrect_vgrad(x - 3.0, y - 3.0, w + 6.0, h + 6.0, h / 2.0 + 3.0, GOLD_HI, GOLD_LO);
        pen.rrect(x, y, w, h, h / 2.0, Col(30, 20, 44, 240));
    }
    fn area_col(&self, o: &CardOptions, i: usize) -> Col {
        [GOLD, o.theme.type_colour().mix(WHITE, 0.2), Col(220, 120, 255, 255), Col(120, 210, 255, 255)][i % 4]
    }
    fn map_px(&self, c: Col) -> Col {
        c.mix(Col(30, 16, 50, 255), 0.25)
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        halo(pen, name, cx, y, size, Col(30, 16, 50, 230), GOLD_HI, Face::Bold, Align::Center, 3.0);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let cy = y + h / 2.0;
        hgrad(pen, x0, y + 2.0, x1 - x0, 2.0, 1.0, GOLD_LO.alpha(0.2), GOLD);
        let mut x = x0;
        if !left.is_empty() {
            x += pen.text(&left.to_uppercase(), x, cy - 7.0, 13.0, GOLD, Face::Bold, Align::Left, false) + 12.0;
        }
        if !name.is_empty() {
            x += pen.text(name, x, cy - 12.0, 21.0, WHITE, Face::Bold, Align::Left, false) + 14.0;
        }
        if let Some(lv) = level {
            self.chip(pen, o, &format!("LV {lv}"), x, cy - 15.0, None, Align::Left);
        }
        let bw = pen.text(&BRAND.to_uppercase(), x1, cy - 7.0, 13.0, SOFT, Face::Bold, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - bw - 38.0, cy - 14.0, 28.0, 28.0);
        }
    }
}
