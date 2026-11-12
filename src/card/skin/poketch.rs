//! Pokétch: the whole picture is the watch. A red casing round a green
//! LCD screen with a faint pixel grid; dot-matrix digits with their unlit
//! dots showing, ink-outlined boxes, segmented gauges and inverted chips.

use super::*;

pub(in super::super) struct Poketch;

const LCD: Col = Col(158, 188, 108, 255);
const LCD_HI: Col = Col(178, 204, 128, 255);
const INK: Col = Col(30, 46, 26, 255);
const MID: Col = Col(78, 102, 52, 255);
const CASE: Col = Col(206, 70, 62, 255);
const CASE_HI: Col = Col(234, 112, 98, 255);
const CASE_DARK: Col = Col(132, 38, 38, 255);
/// The casing's width round the screen.
const RIM: f32 = 26.0;

fn ghost() -> Col {
    LCD.mix(INK, 0.1)
}

/// A 5×7 (or narrower) glyph: width and one row of bits per line.
fn glyph(c: char) -> Option<(u8, [u8; 7])> {
    Some(match c {
        '0' => (5, [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => (5, [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => (5, [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
        '3' => (5, [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110]),
        '4' => (5, [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => (5, [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => (5, [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => (5, [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => (5, [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => (5, [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
        ',' => (2, [0, 0, 0, 0, 0, 0b01, 0b10]),
        '.' => (1, [0, 0, 0, 0, 0, 0, 1]),
        '/' => (5, [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
        ' ' => (3, [0; 7]),
        _ => return None,
    })
}

fn dots_w(s: &str, cell: f32) -> Option<f32> {
    let mut cols = 0.0;
    let mut n = 0.0;
    for c in s.chars() {
        cols += glyph(c)?.0 as f32;
        n += 1.0;
    }
    Some((cols + (n - 1.0f32).max(0.0)) * cell)
}

/// Dot-matrix text with its unlit dots; `y` is the top.
fn dots(pen: &mut Pen, s: &str, x: f32, y: f32, cell: f32, on: Col) {
    let mut cx = x;
    let d = cell * 0.82;
    for c in s.chars() {
        let Some((w, rows)) = glyph(c) else { continue };
        for (ry, bits) in rows.iter().enumerate() {
            for col in 0..w {
                let lit = bits >> (w - 1 - col) & 1 == 1;
                if lit || w >= 3 {
                    pen.rrect(cx + col as f32 * cell, y + ry as f32 * cell, d, d, 0.0, if lit { on } else { ghost() });
                }
            }
        }
        cx += (w as f32 + 1.0) * cell;
    }
}

impl Poketch {
    fn screen(pen: &Pen) -> (f32, f32, f32, f32) {
        (RIM, RIM, pen.w - 2.0 * RIM, pen.h - 2.0 * RIM)
    }
}

impl Skin for Poketch {
    fn bg(&self, _o: &CardOptions) -> Col {
        CASE
    }
    fn backdrop(&self, pen: &mut Pen, _o: &CardOptions) {
        let (w, h) = (pen.w, pen.h);
        pen.rrect_vgrad(0.0, 0.0, w, h, 0.0, CASE_HI, CASE);
        pen.rrect_stroke(4.0, 4.0, w - 8.0, h - 8.0, 22.0, 3.0, CASE_DARK);
        let (x, y, sw, sh) = Poketch::screen(pen);
        pen.rrect(x - 6.0, y - 6.0, sw + 12.0, sh + 12.0, 16.0, CASE_DARK);
        pen.rrect(x - 2.0, y - 2.0, sw + 4.0, sh + 4.0, 13.0, Col(40, 44, 34, 255));
        pen.rrect_vgrad(x, y, sw, sh, 11.0, LCD_HI, LCD);
        // the pixel grid
        let grid = INK.alpha(0.05);
        let mut gx = x + 4.0;
        while gx < x + sw {
            pen.line(gx, y + 2.0, gx, y + sh - 2.0, 1.0, grid);
            gx += 4.0;
        }
        let mut gy = y + 4.0;
        while gy < y + sh {
            pen.line(x + 2.0, gy, x + sw - 2.0, gy, 1.0, grid);
            gy += 4.0;
        }
        // screws in the corners of the casing
        for (sx, sy) in [(12.0, 12.0), (w - 12.0, 12.0), (12.0, h - 12.0), (w - 12.0, h - 12.0)] {
            pen.circle(sx, sy, 5.0, CASE_DARK);
            pen.line(sx - 3.0, sy, sx + 3.0, sy, 1.5, CASE_HI);
        }
    }
    fn inset(&self) -> f32 {
        RIM + 30.0
    }
    fn text(&self) -> Col {
        INK
    }
    fn soft(&self) -> Col {
        MID
    }
    fn accent(&self, _o: &CardOptions) -> Col {
        INK
    }
    fn faces(&self) -> Faces {
        Faces { title: Face::Pixel, num: Face::Pixel, label: Face::Pixel, body: Face::Pixel }
    }
    fn tone(&self, c: Col) -> Col {
        c.mix(INK, 0.5)
    }
    fn panel(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, role: Role) {
        let t = if role == Role::Slot { 2.5 } else { 3.5 };
        if role == Role::Hero {
            pen.rrect(x, y, w, h, 0.0, LCD_HI);
        }
        pen.rrect_stroke(x, y, w, h, 0.0, t, INK);
        if matches!(role, Role::Hero | Role::Card) {
            for (dx, dy) in [(0.0, 0.0), (w - 10.0, 0.0), (0.0, h - 10.0), (w - 10.0, h - 10.0)] {
                pen.rrect(x + dx, y + dy, 10.0, 10.0, 0.0, INK);
            }
        }
    }
    fn hero(&self, pen: &mut Pen, d: &CardData, o: &CardOptions, x: f32, y: f32, w: f32, h: f32) {
        self.panel(pen, o, x, y, w, h, Role::Hero);
        hero_sprites(pen, d, o, x, y, w, h, INK);
        if o.show_normal && d.normal.is_some() {
            let s = 92.0;
            let (nx, ny) = (x + w - s - 14.0, y + h - s - 14.0);
            pen.rrect(nx, ny, s, s, 0.0, LCD);
            pen.rrect_stroke(nx, ny, s, s, 0.0, 2.5, INK);
            normal_inset(pen, d, o, nx + 4.0, ny + 4.0, s - 8.0);
        }
        pen.rrect(x + 14.0, y + 14.0, 96.0, 26.0, 0.0, INK);
        pen.text("SHINY", x + 62.0, y + 21.0, 12.0, LCD, Face::Pixel, Align::Center, false);
    }
    fn chip(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, _fill: Option<Col>, align: Align) -> f32 {
        let t = text.to_uppercase();
        let w = pen.width(&t, 13.0, Face::Pixel) + 26.0;
        let x0 = aligned(x, w, align);
        pen.rrect(x0, y, w, 34.0, 0.0, INK);
        pen.text(&t, x0 + 13.0, y + 11.0, 13.0, LCD, Face::Pixel, Align::Left, false);
        w
    }
    fn title(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        pen.text(text, x, y + size * 0.1, size * 0.8, INK, Face::Pixel, align, false);
    }
    fn number(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, align: Align) {
        let cell = size / 7.5;
        match dots_w(text, cell) {
            Some(w) => {
                // unlit 8s behind (same shape), as on a real LCD
                let x0 = aligned(x, w, align);
                let ghost_s: String = text.chars().map(|c| if c.is_ascii_digit() { '8' } else { c }).collect();
                dots(pen, &ghost_s, x0, y, cell, ghost());
                dots(pen, text, x0, y, cell, INK);
            }
            None => {
                pen.text(text, x, y, size * 0.6, INK, Face::Pixel, align, false);
            }
        }
    }
    fn number_w(&self, pen: &Pen, text: &str, size: f32) -> f32 {
        dots_w(text, size / 7.5).unwrap_or_else(|| pen.width(text, size * 0.6, Face::Pixel))
    }
    fn label(&self, pen: &mut Pen, _o: &CardOptions, text: &str, x: f32, y: f32, size: f32, col: Col, align: Align) -> f32 {
        pen.text(&text.to_uppercase(), x, y, size * 0.78, col, Face::Pixel, align, false)
    }
    fn gauge_h(&self) -> f32 {
        20.0
    }
    fn gauge(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, frac: f32, col: Col) {
        pen.rrect_stroke(x - 4.0, y - 4.0, w + 8.0, 28.0, 0.0, 2.5, INK);
        let n = ((w / 18.0) as usize).clamp(8, 48);
        let gap = 3.0;
        let sw = (w - gap * (n as f32 - 1.0)) / n as f32;
        let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
        for i in 0..n {
            pen.rrect(x + i as f32 * (sw + gap), y, sw, 20.0, 0.0, if i < lit { col } else { ghost() });
        }
    }
    fn tile(&self, pen: &mut Pen, o: &CardOptions, x: f32, y: f32, w: f32, h: f32, label: &str, value: &str) {
        self.panel(pen, o, x, y, w, h, Role::Tile);
        pen.rrect(x, y, w, 26.0, 0.0, INK);
        let l = clip(pen, &label.to_uppercase(), 11.0, Face::Pixel, w - 16.0);
        pen.text(&l, x + w / 2.0, y + 8.0, 11.0, LCD, Face::Pixel, Align::Center, false);
        let vs = pen.fit(value, 20.0, w - 24.0, Face::Pixel);
        pen.text(value, x + w / 2.0, y + 26.0 + (h - 26.0 - vs) / 2.0, vs, INK, Face::Pixel, Align::Center, false);
    }
    fn footer(&self, pen: &mut Pen, o: &CardOptions, x0: f32, x1: f32, y: f32, h: f32, left: &str, name: &str, level: Option<u32>, icon: Option<&Img>) {
        let (y, h) = (y + 6.0, h - 6.0);
        pen.rrect(x0, y, x1 - x0, h, 0.0, INK);
        let cy = y + h / 2.0;
        let mut x = x0 + 18.0;
        if !left.is_empty() {
            x += pen.text(&left.to_uppercase(), x, cy - 6.0, 11.0, LCD.mix(INK, 0.35), Face::Pixel, Align::Left, false) + 14.0;
        }
        if !name.is_empty() {
            x += pen.text(&name.to_uppercase(), x, cy - 8.0, 15.0, LCD, Face::Pixel, Align::Left, false) + 14.0;
        }
        if let Some(lv) = level {
            pen.text(&format!("LV{lv}"), x, cy - 6.0, 12.0, LCD_HI, Face::Pixel, Align::Left, false);
        }
        let bw = pen.text(&BRAND.to_uppercase(), x1 - 18.0, cy - 5.0, 10.0, LCD.mix(INK, 0.25), Face::Pixel, Align::Right, false);
        if let Some(icon) = icon {
            pen.blit_smooth(icon, x1 - 18.0 - bw - 32.0, cy - 12.0, 24.0, 24.0);
        }
        let _ = o;
    }
    fn badge(&self, pen: &mut Pen, o: &CardOptions, cx: f32, cy: f32, r: f32, level: u32, _metal: Col, _light: Col) {
        let s = r * 1.7;
        pen.rrect(cx - s / 2.0, cy - s / 2.0, s, s, 0.0, LCD_HI);
        pen.rrect_stroke(cx - s / 2.0, cy - s / 2.0, s, s, 0.0, 4.0, INK);
        pen.rrect(cx - s / 2.0, cy - s / 2.0, s, 24.0, 0.0, INK);
        pen.text("LEVEL", cx, cy - s / 2.0 + 7.0, 11.0, LCD, Face::Pixel, Align::Center, false);
        let n = level.to_string();
        let size = s * 0.5;
        self.number(pen, o, &n, cx, cy - size * 0.38, size, Align::Center);
    }
    fn pin(&self, pen: &mut Pen, _o: &CardOptions, x: f32, y: f32, w: f32, h: f32, _col: Col) {
        pen.rrect(x, y, w, h, 0.0, LCD_HI);
        pen.rrect_stroke(x, y, w, h, 0.0, 3.0, INK);
    }
    fn area_col(&self, _o: &CardOptions, i: usize) -> Col {
        [INK, MID][i % 2]
    }
    fn map_px(&self, c: Col) -> Col {
        let l = (0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32) / 255.0;
        // four LCD shades
        match (l * 4.0) as u32 {
            0 => INK,
            1 => MID,
            2 => LCD,
            _ => LCD_HI,
        }
    }
    fn map_label(&self, pen: &mut Pen, _o: &CardOptions, name: &str, cx: f32, y: f32, size: f32) {
        let t = name.to_uppercase();
        let w = pen.width(&t, size * 0.6, Face::Pixel) + 12.0;
        pen.rrect(cx - w / 2.0, y, w, size * 0.6 + 10.0, 0.0, INK);
        pen.text(&t, cx, y + 5.0, size * 0.6, LCD, Face::Pixel, Align::Center, false);
    }
}
