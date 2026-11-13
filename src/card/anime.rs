//! The anime-style share images: flat bold colours, thick ink outlines,
//! speed lines and halftone dots, the Pokémon as a sticker on a star burst,
//! and the numbers as an infographic (stickers, a segmented luck gauge).
//! Same data and options as the classic look; the theme picks the colours.

use super::*;

/// Flat colours for one theme.
#[derive(Clone, Copy)]
struct Ink {
    /// The theme's type colour.
    main: Col,
    /// Darker and lighter versions of it.
    deep: Col,
    soft: Col,
    /// Cream paper, near-black ink.
    paper: Col,
    ink: Col,
    /// Three bright pop colours that sit well beside `main`.
    pop: [Col; 3],
}

const PAPER: Col = Col::rgb(255, 247, 232);
const INK: Col = Col::rgb(28, 24, 48);
const WHITE: Col = Col::rgb(255, 255, 255);
const YELLOW: Col = Col::rgb(255, 210, 63);
const PINK: Col = Col::rgb(255, 104, 154);
const CYAN: Col = Col::rgb(60, 203, 232);
const MINT: Col = Col::rgb(91, 224, 160);
const ORANGE: Col = Col::rgb(255, 146, 72);
const GREEN: Col = Col::rgb(76, 205, 110);
const RED: Col = Col::rgb(255, 92, 82);

fn hue(c: Col) -> f32 {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    if d < 1e-4 {
        return 0.0;
    }
    let h = if mx == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h * 60.0
}

fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

fn ink_for(o: &CardOptions) -> Ink {
    let main = o.theme.type_colour();
    // Pop colours: the three furthest in hue from the theme colour.
    let mut pops = [YELLOW, PINK, CYAN, MINT, ORANGE];
    let h = hue(main);
    pops.sort_by(|a, b| hue_gap(hue(*b), h).partial_cmp(&hue_gap(hue(*a), h)).unwrap_or(std::cmp::Ordering::Equal));
    Ink {
        main,
        deep: main.mix(INK, 0.45),
        soft: main.mix(WHITE, 0.45),
        paper: PAPER,
        ink: INK,
        pop: [pops[0], pops[1], pops[2]],
    }
}

/// Readable text on a flat fill.
fn on(fill: Col) -> Col {
    ink_on(fill, INK)
}

/// The cream background (the parts that never move).
pub(super) fn base(o: &CardOptions) -> Canvas {
    let (w, h, _) = o.output_size();
    Canvas::new(w, h, PAPER)
}

// ───────────────────────────── drawing helpers ─────────────────────────────

impl Pen<'_> {
    /// Text with a thick outline and an optional hard drop shadow (comic
    /// lettering). Returns the width.
    #[allow(clippy::too_many_arguments)]
    fn lettering(&mut self, s: &str, x: f32, y: f32, size: f32, fill: Col, outline: Col, t: f32, face: Face, align: Align, shadow: Option<Col>) -> f32 {
        let w = self.width(s, size, face);
        let x = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let k = self.s;
        let ring = |t: f32| -> Vec<(f32, f32)> {
            let n = 16;
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                    (a.cos() * t, a.sin() * t)
                })
                .collect()
        };
        if let Some(sc) = shadow {
            let off = (size / 12.0).clamp(3.0, 9.0);
            for (dx, dy) in ring(t) {
                self.t.draw(&mut self.c, s, (x + off + dx) * k, (y + off + dy) * k, size * k, sc, face);
            }
        }
        if t > 0.0 {
            for (dx, dy) in ring(t) {
                self.t.draw(&mut self.c, s, (x + dx) * k, (y + dy) * k, size * k, outline, face);
            }
            for (dx, dy) in ring(t * 0.5) {
                self.t.draw(&mut self.c, s, (x + dx) * k, (y + dy) * k, size * k, outline, face);
            }
        }
        self.t.draw(&mut self.c, s, x * k, y * k, size * k, fill, face);
        w
    }

    /// A flat sticker: ink outline, flat fill and a hard ink shadow.
    #[allow(clippy::too_many_arguments)]
    fn sticker(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Col, ink: Col) {
        self.rrect(x + 5.0, y + 6.0, w, h, r, ink);
        self.rrect(x - 3.0, y - 3.0, w + 6.0, h + 6.0, r + 3.0, ink);
        self.rrect(x, y, w, h, r, fill);
    }

    /// A small label: ink outline round a flat pill. Returns its width.
    #[allow(clippy::too_many_arguments)]
    fn tag(&mut self, text: &str, x: f32, y: f32, size: f32, fill: Col, ink: Col, align: Align) -> f32 {
        let pad = size * 0.8;
        let w = self.width(text, size, Face::Bold) + pad * 2.0;
        let h = size * 1.75;
        let x = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        self.rrect(x + 3.0, y + 4.0, w, h, h / 2.0, ink);
        self.rrect(x - 2.5, y - 2.5, w + 5.0, h + 5.0, h / 2.0 + 2.5, ink);
        self.rrect(x, y, w, h, h / 2.0, fill);
        let ty = y + h / 2.0 - self.mid(size, Face::Bold);
        self.text(text, x + pad, ty, size, on(fill), Face::Bold, Align::Left, false);
        w
    }

    /// A parallelogram ribbon with text (leans right).
    fn ribbon(&mut self, text: &str, x: f32, y: f32, size: f32, fill: Col, ink: Col) -> f32 {
        let tw = self.width(text, size, Face::Display);
        let h = size * 1.6;
        let lean = h * 0.35;
        let w = tw + size * 1.6 + lean;
        let shape = |dx: f32, dy: f32, grow: f32| -> Vec<(f32, f32)> {
            vec![
                (x + lean + dx - grow, y + dy - grow),
                (x + w + dx + grow, y + dy - grow),
                (x + w - lean + dx + grow, y + h + dy + grow),
                (x + dx - grow, y + h + dy + grow),
            ]
        };
        self.poly(&shape(5.0, 6.0, 0.0), ink);
        self.poly(&shape(0.0, 0.0, 3.0), ink);
        self.poly(&shape(0.0, 0.0, 0.0), fill);
        self.lettering(text, x + lean / 2.0 + size * 0.8, y + (h - size * 1.2) / 2.0, size, WHITE, ink, 2.5, Face::Display, Align::Left, None);
        w
    }

    /// Speed lines: thin wedges radiating from (cx, cy), between radii r0..r1.
    #[allow(clippy::too_many_arguments)]
    /// `ph` (0..1, loops) makes each line flick in and out a little.
    fn speed_lines(&mut self, cx: f32, cy: f32, r0: f32, r1: f32, n: usize, col: Col, ph: Option<f32>) {
        let mut rng = Rng(0x5eed_1a7e_0c0f_fee5);
        for i in 0..n {
            let a = (i as f32 + rng.next() * 0.6) / n as f32 * std::f32::consts::TAU;
            let half = 0.012 + rng.next() * 0.02;
            let off = rng.next();
            let flick = ph.map_or(0.0, |ph| 0.12 * ((ph * 2.0 + off) * std::f32::consts::TAU).sin());
            let inner = r0 * (0.85 + rng.next() * 0.3 + flick);
            let (c0, s0) = (a.cos(), a.sin());
            let (ca, sa) = ((a - half).cos(), (a - half).sin());
            let (cb, sb) = ((a + half).cos(), (a + half).sin());
            self.poly(&[(cx + c0 * inner, cy + s0 * inner), (cx + ca * r1, cy + sa * r1), (cx + cb * r1, cy + sb * r1)], col);
        }
    }

    /// Halftone dots in a box, growing towards (fx, fy) (0..1 inside the box).
    #[allow(clippy::too_many_arguments)]
    fn halftone(&mut self, x: f32, y: f32, w: f32, h: f32, step: f32, fx: f32, fy: f32, col: Col) {
        let (ox, oy) = (x + w * fx, y + h * fy);
        let reach = (w * w + h * h).sqrt() * 0.75;
        let mut row = 0;
        let mut yy = y;
        while yy <= y + h {
            let shift = if row % 2 == 1 { step / 2.0 } else { 0.0 };
            let mut xx = x + shift;
            while xx <= x + w {
                let d = ((xx - ox).powi(2) + (yy - oy).powi(2)).sqrt() / reach;
                let r = step * 0.42 * (1.0 - d).clamp(0.0, 1.0);
                if r > 0.6 {
                    self.circle(xx, yy, r, col);
                }
                xx += step;
            }
            yy += step;
            row += 1;
        }
    }

    /// A spiky star burst with an ink outline.
    #[allow(clippy::too_many_arguments)]
    fn burst(&mut self, cx: f32, cy: f32, r_in: f32, r_out: f32, n: usize, fill: Col, ink: Col, turn: f32) {
        let spikes = |grow: f32| -> Vec<Vec<(f32, f32)>> {
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU + turn;
                    let half = std::f32::consts::PI / n as f32;
                    let ro = r_out + grow * 1.6;
                    let ri = r_in + grow;
                    vec![
                        (cx + (a - half).cos() * ri, cy + (a - half).sin() * ri),
                        (cx + a.cos() * ro, cy + a.sin() * ro),
                        (cx + (a + half).cos() * ri, cy + (a + half).sin() * ri),
                        (cx, cy),
                    ]
                })
                .collect()
        };
        for sp in spikes(5.0) {
            self.poly(&sp, ink);
        }
        self.circle(cx, cy, r_in + 5.0, ink);
        for sp in spikes(0.0) {
            self.poly(&sp, fill);
        }
        self.circle(cx, cy, r_in, fill);
    }

    /// A four-point twinkle with an ink outline.
    fn twinkle(&mut self, cx: f32, cy: f32, size: f32, fill: Col, ink: Col) {
        self.sparkle(cx, cy, size + 4.0, ink);
        self.sparkle(cx, cy, size, fill);
    }

    /// Pixel-art Pokémon as a sticker (white border, ink edge), standing on
    /// `floor`, centred on `cx`. Returns the sprite's box.
    #[allow(clippy::too_many_arguments)]
    fn sprite_sticker(&mut self, img: &Img, tb: Option<(u32, u32, u32, u32)>, cx: f32, floor: f32, max_w: f32, max_h: f32, ink: Col) -> (f32, f32, f32, f32) {
        let tb = tb.unwrap_or_else(|| Canvas::trim_box(img));
        let sp = crop(img, tb);
        let k = ((max_w / sp.w as f32).min(max_h / sp.h as f32)).floor().clamp(1.0, 9.0);
        let (sw, sh) = (sp.w as f32 * k, sp.h as f32 * k);
        let (sx, sy) = ((cx - sw / 2.0).round(), (floor - sh).round());
        let white = silhouette(&sp, WHITE);
        let dark = silhouette(&sp, ink);
        let ring = |r: f32| -> Vec<(f32, f32)> {
            (0..16)
                .map(|i| {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU;
                    (a.cos() * r, a.sin() * r)
                })
                .collect()
        };
        // hard shadow, ink edge, white border, then the sprite
        self.blit_px(&dark, sx + 8.0, sy + 9.0, k);
        for (dx, dy) in ring(11.0) {
            self.blit_px(&dark, sx + dx, sy + dy, k);
        }
        for (dx, dy) in ring(7.0).into_iter().chain(ring(4.0)) {
            self.blit_px(&white, sx + dx, sy + dy, k);
        }
        self.blit_px(&sp, sx, sy, k);
        (sx, sy, sw, sh)
    }
}

/// Every opaque pixel of `img` in one colour.
fn silhouette(img: &Img, c: Col) -> Img {
    let mut px = img.px.clone();
    for p in px.chunks_mut(4) {
        let a = p[3];
        p[0] = c.0;
        p[1] = c.1;
        p[2] = c.2;
        p[3] = if a > 40 { 255 } else { 0 };
    }
    Img { w: img.w, h: img.h, px }
}

// ───────────────────────────── hunt card ─────────────────────────────

pub(super) fn hunt(pen: &mut Pen, d: &CardData, o: &CardOptions) {
    let k = ink_for(o);
    match o.layout {
        CardLayout::Wide => hunt_wide(pen, d, o, k),
        CardLayout::Square => hunt_square(pen, d, o, k),
    }
}

/// How far the speed lines and bursts have turned in the animated version.
/// Where the loop is (0..1); 0 for a still picture.
fn turn(pen: &Pen) -> f32 {
    pen.phase.unwrap_or(0.0)
}

/// The colour block with speed lines, burst, sprite and stickers on it.
#[allow(clippy::too_many_arguments)]
fn sprite_scene(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink, cx: f32, cy: f32, size: f32) {
    let t = turn(pen);
    // Burst behind the Pokémon.
    let pulse = pen.phase.map_or(1.0, |ph| 1.0 + 0.04 * (ph * std::f32::consts::TAU * 2.0).sin());
    pen.burst(cx, cy, size * 0.62 * pulse, size * 0.9 * pulse, 14, k.pop[0], k.ink, t * std::f32::consts::TAU / 14.0);
    pen.circle(cx, cy, size * 0.56, k.paper);
    pen.ring(cx, cy, size * 0.56, 5.0, k.ink);
    pen.halftone(cx - size * 0.56, cy - size * 0.56, size * 1.12, size * 1.12, 14.0, 0.15, 0.1, k.soft.alpha(0.55));
    pen.circle(cx, cy, size * 0.56 + 0.5, k.paper.alpha(0.0));

    match &d.shiny {
        Some(img) => {
            let (sx, sy, sw, sh) = pen.sprite_sticker(img, d.sprite_box, cx, cy + size * 0.48, size * 1.15, size * 1.0, k.ink);
            if o.sparkles {
                let tw = |base: f32, off: f32| -> f32 { pen.phase.map_or(base, |ph| base * (0.65 + 0.35 * ((ph + off) * std::f32::consts::TAU).sin().abs())) };
                let (a, b, c) = (tw(24.0, 0.0), tw(15.0, 0.33), tw(11.0, 0.66));
                pen.twinkle(sx + sw + 10.0, sy + 18.0, a, YELLOW, k.ink);
                pen.twinkle(sx - 18.0, sy + sh * 0.4, b, WHITE, k.ink);
                pen.twinkle(sx + sw - 6.0, sy + sh * 0.78, c, k.pop[1], k.ink);
            }
        }
        None => {
            pen.lettering("?", cx, cy - 70.0, 140.0, WHITE, k.ink, 6.0, Face::Display, Align::Center, Some(k.deep));
        }
    }
}

/// The normal-colour sprite in a small round sticker.
fn normal_badge(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink, cx: f32, cy: f32, r: f32) {
    if !o.show_normal {
        return;
    }
    let Some(img) = &d.normal else { return };
    pen.circle(cx + 5.0, cy + 6.0, r, k.ink);
    pen.circle(cx, cy, r + 4.0, k.ink);
    pen.circle(cx, cy, r, k.paper);
    let sp = crop(img, Canvas::trim_box(img));
    let kk = ((r * 1.45) / sp.w.max(sp.h) as f32).floor().clamp(1.0, 4.0);
    let (sw, sh) = (sp.w as f32 * kk, sp.h as f32 * kk);
    pen.blit_px(&sp, (cx - sw / 2.0).round(), (cy + r * 0.62 - sh).round(), kk);
    pen.tag("NORMAL", cx, cy + r - 8.0, 13.0, k.ink, k.ink, Align::Center);
}

/// "SHINY EEVEE!" with a gender sign. Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn title(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink, x: f32, y: f32, w: f32, align: Align) -> f32 {
    let title = if o.title.trim().is_empty() { auto_title(d) } else { o.title.trim().to_uppercase() };
    let sign = d.gender != Gender::None;
    let room = |s: f32| if sign { s * 1.0 } else { 0.0 };
    let mut size = pen.fit(&title, 66.0, w, Face::Display);
    while size > 12.0 && pen.width(&title, size, Face::Display) + room(size) > w {
        size -= 1.0;
    }
    let fill = if d.caught { WHITE } else { k.paper };
    let tw = pen.width(&title, size, Face::Display);
    let tx = match align {
        Align::Center => x - room(size) / 2.0,
        _ => x,
    };
    pen.lettering(&title, tx, y, size, fill, k.ink, (size / 14.0).clamp(3.0, 6.0), Face::Display, align, Some(k.main));
    if sign {
        let left = match align {
            Align::Center => tx - tw / 2.0,
            Align::Right => tx - tw,
            Align::Left => tx,
        };
        let c = if d.gender == Gender::Male { CYAN } else { PINK };
        let (gx, gy, gr) = (left + tw + size * 0.55, y + size * 0.62, size * 0.3);
        pen.circle(gx, gy, gr + 4.0, k.ink);
        pen.circle(gx, gy, gr, c);
        // ♂ / ♀ drawn as shapes (fonts may not have them).
        let (r, t) = (gr * 0.36, (gr * 0.16).max(2.0));
        if d.gender == Gender::Male {
            let (ox, oy) = (gx - r * 0.35, gy + r * 0.35);
            pen.ring(ox, oy, r, t, WHITE);
            let q = std::f32::consts::FRAC_1_SQRT_2;
            let (ex, ey) = (ox + r * 1.9 * q + r * 0.3, oy - r * 1.9 * q - r * 0.3);
            pen.line(ox + r * q, oy - r * q, ex, ey, t, WHITE);
            pen.line(ex, ey, ex - r * 0.8, ey, t, WHITE);
            pen.line(ex, ey, ex, ey + r * 0.8, t, WHITE);
        } else {
            let oy = gy - r * 0.45;
            pen.ring(gx, oy, r, t, WHITE);
            pen.line(gx, oy + r, gx, oy + r * 2.2, t, WHITE);
            pen.line(gx - r * 0.6, oy + r * 1.6, gx + r * 0.6, oy + r * 1.6, t, WHITE);
        }
    }
    y + size * 1.25
}

/// "#0133 · Egg hatching · 1 in 4,096", the note and phases. Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn subline(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink, x: f32, y: f32, w: f32, align: Align, room: f32) -> f32 {
    let mut y = y;
    let mut sub = Vec::new();
    if d.nickname.as_ref().is_some_and(|n| !n.trim().is_empty()) && o.title.trim().is_empty() {
        sub.push(d.name.clone());
    }
    if let Some(n) = d.dex {
        sub.push(format!("#{n:04}"));
    }
    if o.show_odds {
        if !d.method.is_empty() {
            sub.push(d.method.clone());
        }
        if let Some(odds) = d.odds_one_in {
            sub.push(format!("1 in {}", thousands(odds.round() as u64)));
        }
    }
    if !sub.is_empty() {
        let s = sub.join("  ·  ");
        let fs = pen.fit(&s, 23.0, w, Face::Bold);
        pen.text(&s, x, y, fs, k.ink, Face::Bold, align, false);
        y += fs + 14.0;
    }
    // The note as a speech bubble.
    if let (true, Some(note)) = (o.show_note, d.note.as_ref().filter(|n| !n.trim().is_empty())) {
        if room >= 60.0 {
            let mut q = note.trim().to_string();
            if q.chars().count() > 60 {
                q = format!("{}…", q.chars().take(57).collect::<String>());
            }
            let fs = pen.fit(&q, 20.0, w - 50.0, Face::Bold);
            let bw = pen.width(&q, fs, Face::Bold) + 36.0;
            let bh = fs + 22.0;
            let bx = match align {
                Align::Center => x - bw / 2.0,
                Align::Right => x - bw,
                Align::Left => x + 14.0,
            };
            let tail = [(bx + 18.0, y + 4.0), (bx - 6.0, y - 8.0), (bx + 34.0, y + 6.0)];
            pen.poly(&[(tail[0].0 - 3.0, tail[0].1 + 3.0), (tail[1].0 - 4.0, tail[1].1 - 4.0), (tail[2].0 + 4.0, tail[2].1 + 3.0)], k.ink);
            pen.rrect(bx - 3.0, y - 3.0 + 4.0, bw + 6.0, bh + 6.0, bh / 2.0 + 3.0, k.ink);
            pen.rrect(bx, y + 4.0, bw, bh, bh / 2.0, WHITE);
            pen.poly(&tail, WHITE);
            pen.text(&q, bx + 18.0, y + 4.0 + (bh - fs * 1.2) / 2.0, fs, k.ink, Face::Bold, Align::Left, false);
            y += bh + 16.0;
        }
    }
    if !d.phases.is_empty() && room >= 100.0 {
        let mut names: Vec<&str> = d.phases.iter().map(|s| s.as_str()).take(3).collect();
        names.dedup();
        let more = d.phases.len().saturating_sub(3);
        let t = format!("PHASE{}: {}{}", if d.phases.len() == 1 { "" } else { "S" }, names.join(", ").to_uppercase(), if more > 0 { format!(" +{more}") } else { String::new() });
        pen.tag(&t, x, y, 14.0, k.pop[2], k.ink, align);
        y += 14.0 * 1.75 + 14.0;
    }
    y
}

/// The big encounter count. Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn count(pen: &mut Pen, d: &CardData, k: Ink, x: f32, y: f32, w: f32, align: Align, size: f32) -> f32 {
    let n = thousands(d.encounters);
    let label = if d.encounters == 1 { "ENCOUNTER" } else { "ENCOUNTERS" };
    let lw = pen.width(label, 16.0, Face::Bold) + 30.0;
    let size = pen.fit(&n, size, (w - lw - 20.0).max(80.0), Face::Display);
    let nw = pen.width(&n, size, Face::Display);
    let total = nw + 20.0 + lw;
    let x0 = match align {
        Align::Left => x,
        Align::Center => x - total / 2.0,
        Align::Right => x - total,
    };
    pen.lettering(&n, x0, y, size, k.main, k.ink, (size / 16.0).clamp(3.0, 6.0), Face::Display, Align::Left, Some(k.ink));
    pen.tag(label, x0 + nw + 26.0, y + size * 0.55, 16.0, k.ink, k.ink, Align::Left);
    if let Some(ts) = d.target_seen {
        let t = format!("{} {} SEEN", thousands(ts), d.name.to_uppercase());
        pen.text(&t, x0 + nw + 28.0, y + size * 0.55 - 26.0, 15.0, k.deep, Face::Bold, Align::Left, false);
    }
    y + size * 1.22
}

/// The luck gauge: three flat segments (under / at / over the odds) with a
/// pointer at the hunt's place. Returns the bottom y.
fn gauge(pen: &mut Pen, d: &CardData, k: Ink, x: f32, y: f32, w: f32) -> f32 {
    let Some(odds) = d.odds_one_in else { return y };
    let n = match (d.odds_on_target, d.target_seen) {
        (true, Some(ts)) => ts,
        _ => d.encounters,
    };
    let mult = n as f64 / odds;
    let (label, _) = luck(mult);
    let col = if mult < 0.9 {
        GREEN
    } else if mult <= 1.1 {
        YELLOW
    } else {
        RED
    };
    let bh = 26.0;
    let by = y + 46.0;
    // Labels above.
    pen.tag(label, x, y, 14.0, col, k.ink, Align::Left);
    pen.lettering(&format!("{} THE ODDS", fmt_mult(mult)), x + w, y - 2.0, 24.0, k.ink, k.paper, 0.0, Face::Display, Align::Right, None);
    // Bar: ink frame, three flat segments.
    pen.rrect(x + 5.0, by + 6.0, w, bh, bh / 2.0, k.ink);
    pen.rrect(x - 3.0, by - 3.0, w + 6.0, bh + 6.0, bh / 2.0 + 3.0, k.ink);
    let segs = [(0.0, 1.0, GREEN), (1.0, 2.0, YELLOW), (2.0, 3.0, RED)];
    for (i, (a, b, c)) in segs.iter().enumerate() {
        let sx = x + w * a / 3.0;
        let sw = w * (b - a) / 3.0;
        let r = if i == 0 || i == 2 { bh / 2.0 } else { 0.0 };
        pen.rrect(sx, by, sw, bh, r, c.mix(WHITE, 0.45));
        if i == 1 {
            pen.rrect(sx, by, sw, bh, 0.0, c.mix(WHITE, 0.45));
        }
    }
    // Filled part, flat and bold.
    let frac = (mult / 3.0).clamp(0.0, 1.0) as f32;
    if frac > 0.0 {
        let fw = (w * frac).max(bh);
        for (i, (a, b, c)) in segs.iter().enumerate() {
            let sx = x + w * a / 3.0;
            let ex = (x + w * b / 3.0).min(x + fw);
            if ex <= sx {
                continue;
            }
            let r = if i == 0 { bh / 2.0 } else { 0.0 };
            pen.rrect(sx, by, ex - sx, bh, r, *c);
        }
        pen.rrect(x + 10.0, by + 5.0, (fw - 20.0).max(0.0), 5.0, 2.5, WHITE.alpha(0.55));
    }
    for i in 1..3 {
        let tx = x + w * i as f32 / 3.0;
        pen.rrect(tx - 2.0, by, 4.0, bh, 0.0, k.ink);
    }
    // Pointer.
    let px = x + w * frac;
    pen.poly(&[(px - 13.0, by + bh + 15.0), (px + 13.0, by + bh + 15.0), (px, by + bh - 4.0)], k.ink);
    pen.poly(&[(px - 7.0, by + bh + 10.0), (px + 7.0, by + bh + 10.0), (px, by + bh + 2.0)], WHITE);
    for i in 1..=3 {
        let tx = x + w * i as f32 / 3.0;
        let a = if i == 3 { Align::Right } else { Align::Center };
        pen.text(&format!("{i}×"), if i == 3 { x + w } else { tx }, by + bh + 12.0, 14.0, k.deep, Face::Bold, a, false);
    }
    by + bh + 34.0
}

/// Stat stickers in a row. Returns the bottom y.
#[allow(clippy::too_many_arguments)]
fn stickers(pen: &mut Pen, list: &[(&str, String)], k: Ink, x: f32, y: f32, w: f32, per_row: usize, th: f32) -> f32 {
    if list.is_empty() {
        return y;
    }
    let gap = 22.0;
    let n = list.len().min(per_row);
    let cols = per_row.min(n);
    let tw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let mut yy = y;
    for (row_i, row) in list[..n].chunks(cols).enumerate() {
        for (i, (label, value)) in row.iter().enumerate() {
            let fill = k.pop[(i + row_i) % 3].mix(WHITE, 0.25);
            let tx = x + i as f32 * (tw + gap);
            pen.sticker(tx, yy, tw, th, 16.0, fill, k.ink);
            // A little ink dot "icon" and the label.
            pen.circle(tx + 22.0, yy + 24.0, 7.0, k.ink);
            pen.circle(tx + 22.0, yy + 24.0, 3.0, fill);
            let ls = pen.fit(&label.to_uppercase(), 14.0, tw - 50.0, Face::Bold);
            pen.text(&label.to_uppercase(), tx + 36.0, yy + 15.0, ls, k.ink, Face::Bold, Align::Left, false);
            let vs = pen.fit(value, 32.0, tw - 32.0, Face::Display);
            pen.text(value, tx + 16.0, yy + th - vs * 1.25 - 8.0, vs, k.ink, Face::Display, Align::Left, false);
        }
        yy += th + gap;
        if row_i == 0 && n <= cols {
            break;
        }
    }
    yy - gap
}

/// Ink band along the bottom: trainer and the app's name.
#[allow(clippy::too_many_arguments)]
fn footer_band(pen: &mut Pen, k: Ink, label: &str, trainer: &str, level: Option<u32>, right: &str, icon: Option<&Img>, y: f32, h: f32) {
    let w = pen.w;
    pen.rrect(0.0, y, w, h, 0.0, k.ink);
    pen.rrect(0.0, y, w, 6.0, 0.0, k.main);
    let m = 40.0;
    let cy = y + 6.0 + (h - 6.0) / 2.0;
    let mut x = m;
    if !trainer.trim().is_empty() {
        pen.text(label, x, cy - pen.mid(15.0, Face::Bold), 15.0, k.main.mix(WHITE, 0.4), Face::Bold, Align::Left, false);
        x += pen.width(label, 15.0, Face::Bold) + 12.0;
        x += pen.avatar(x, cy, 36.0, WHITE);
        let t = trainer.trim().to_uppercase();
        let nw = pen.width(&t, 24.0, Face::Display);
        let ny = cy - pen.mid(24.0, Face::Display);
        let off = pen.plate_under(x, ny, nw, 24.0, Face::Display);
        x += pen.text(&t, x + off, ny, 24.0, WHITE, Face::Display, Align::Left, false) + 12.0 + off * 2.0;
        if let Some(lv) = level {
            // a tag is 1.75 × its text size tall
            pen.tag(&format!("LV {lv}"), x, cy - 13.0 * 0.875, 13.0, YELLOW, k.ink, Align::Left);
        }
    }
    let bw = pen.text(right, w - m, cy - pen.mid(15.0, Face::Bold), 15.0, WHITE, Face::Bold, Align::Right, false);
    if let Some(icon) = icon {
        pen.blit_smooth(icon, w - m - bw - 38.0, cy - 15.0, 30.0, 30.0);
    }
}

fn hunt_wide(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink) {
    let (w, h) = (pen.w, pen.h);
    let band = 74.0;
    let bottom = h - band;
    // Colour block on the left with a slanted edge; speed lines from the sprite.
    let (cx, cy) = (262.0, 300.0);
    let (top_x, bot_x) = (540.0, 450.0);
    pen.poly(&[(0.0, 0.0), (top_x, 0.0), (bot_x, bottom), (0.0, bottom)], k.main);

    pen.speed_lines(cx, cy, 200.0, 760.0, 54, k.soft.alpha(0.55), pen.phase);
    // Paper back over the spill on the right, then the slanted edge stripes.
    pen.poly(&[(top_x, 0.0), (w, 0.0), (w, bottom), (bot_x, bottom)], k.paper);
    pen.halftone(w - 330.0, 0.0, 330.0, 220.0, 16.0, 1.0, 0.0, k.soft.alpha(0.7));
    pen.halftone(0.0, bottom - 200.0, 240.0, 200.0, 16.0, 0.0, 1.0, k.deep.alpha(0.35));
    let stripe = |off: f32, wid: f32| -> Vec<(f32, f32)> { vec![(top_x + off, 0.0), (top_x + off + wid, 0.0), (bot_x + off + wid, bottom), (bot_x + off, bottom)] };
    pen.poly(&stripe(-3.0, 9.0), k.ink);
    pen.poly(&stripe(22.0, 14.0), k.pop[0]);
    pen.poly(&stripe(19.0, 3.0), k.ink);
    pen.poly(&stripe(36.0, 3.0), k.ink);

    sprite_scene(pen, d, o, k, cx, cy, 230.0);
    pen.ribbon("SHINY!", 30.0, 28.0, 28.0, k.pop[1], k.ink);
    normal_badge(pen, d, o, k, 92.0, bottom - 96.0, 58.0);

    // Right column.
    let x0 = 610.0;
    let cw = w - 44.0 - x0;
    let mut y = 36.0;
    let (status, fill) = if d.caught { ("CAUGHT!", GREEN) } else { ("HUNTING", k.pop[2]) };
    let sw = pen.tag(status, x0, y, 15.0, fill, k.ink, Align::Left);
    if let Some(dex) = d.dex {
        pen.tag(&format!("#{dex:04}"), x0 + sw + 16.0, y, 15.0, WHITE, k.ink, Align::Left);
    }
    y += 50.0;
    y = title(pen, d, o, k, x0, y, cw, Align::Left);
    let room = bottom - 26.0 - 96.0 - 120.0 - 140.0 - y;
    y = subline(pen, d, o, k, x0, y, cw, Align::Left, room) + 4.0;
    y = count(pen, d, k, x0, y, cw, Align::Left, 92.0);
    if o.show_meter && d.odds_one_in.is_some() && y + 110.0 + 96.0 <= bottom - 20.0 {
        y = gauge(pen, d, k, x0, y + 4.0, cw) + 6.0;
    }
    let t = tiles(d, o);
    let th = 92.0;
    let sy = (bottom - 28.0 - th).max(y);
    if sy + th <= bottom - 16.0 {
        stickers(pen, &t, k, x0, sy, cw, 3, th);
    }
    // bigger than the trainer card's: the hunt image's footer has room to spare
    let fk = 1.3;
    pen.scaled(fk, |pen| footer_band(pen, k, "TRAINER", &o.trainer, d.trainer_level, "POKE TOGETHER COMPANION", d.icon.as_ref(), bottom / fk, band / fk));
}

fn hunt_square(pen: &mut Pen, d: &CardData, o: &CardOptions, k: Ink) {
    let (w, h) = (pen.w, pen.h);
    let band = 78.0;
    let bottom = h - band;
    // Colour block across the top with a slanted bottom edge.
    let (cx, cy) = (w / 2.0, 222.0);
    let (left_y, right_y) = (452.0, 392.0);
    pen.poly(&[(0.0, 0.0), (w, 0.0), (w, right_y), (0.0, left_y)], k.main);

    pen.speed_lines(cx, cy, 190.0, 900.0, 64, k.soft.alpha(0.55), pen.phase);
    pen.poly(&[(0.0, left_y), (w, right_y), (w, h), (0.0, h)], k.paper);
    pen.halftone(w - 300.0, right_y + 10.0, 300.0, 240.0, 16.0, 1.0, 0.0, k.soft.alpha(0.7));
    pen.halftone(0.0, 0.0, 260.0, 220.0, 16.0, 0.0, 0.0, k.deep.alpha(0.35));
    let stripe = |off: f32, wid: f32| -> Vec<(f32, f32)> { vec![(0.0, left_y + off), (w, right_y + off), (w, right_y + off + wid), (0.0, left_y + off + wid)] };
    pen.poly(&stripe(-3.0, 9.0), k.ink);
    pen.poly(&stripe(22.0, 14.0), k.pop[0]);
    pen.poly(&stripe(19.0, 3.0), k.ink);
    pen.poly(&stripe(36.0, 3.0), k.ink);

    sprite_scene(pen, d, o, k, cx, cy, 205.0);
    pen.ribbon("SHINY!", 34.0, 30.0, 30.0, k.pop[1], k.ink);
    normal_badge(pen, d, o, k, w - 100.0, 100.0, 58.0);

    let m = 56.0;
    let cw = w - 2.0 * m;
    // Fixed from the bottom: stickers, then the gauge.
    let t_list = tiles(d, o);
    let th = 92.0;
    let stick_y = bottom - 26.0 - th;
    let gauge_h = if o.show_meter && d.odds_one_in.is_some() { 106.0 } else { 0.0 };
    let gauge_y = stick_y - 18.0 - gauge_h;
    // From the top: status tags, title, details, count.
    let mut y = left_y + 34.0;
    let (status, fill) = if d.caught { ("CAUGHT!", GREEN) } else { ("HUNTING", k.pop[2]) };
    let dex = d.dex.map(|n| format!("#{n:04}"));
    let sw = pen.width(status, 15.0, Face::Bold) + 24.0;
    let dw = dex.as_ref().map_or(0.0, |t| pen.width(t, 15.0, Face::Bold) + 24.0 + 16.0);
    let x0 = w / 2.0 - (sw + dw) / 2.0;
    pen.tag(status, x0, y, 15.0, fill, k.ink, Align::Left);
    if let Some(t) = &dex {
        pen.tag(t, x0 + sw + 16.0, y, 15.0, WHITE, k.ink, Align::Left);
    }
    y += 44.0;
    y = title(pen, d, o, k, w / 2.0, y, cw, Align::Center);
    let count_h = 110.0;
    let room = gauge_y - count_h - y - 40.0;
    y = subline(pen, d, o, k, w / 2.0, y, cw, Align::Center, room);
    let cs = (gauge_y - 10.0 - y).clamp(60.0, 100.0) / 1.22;
    count(pen, d, k, w / 2.0, y, cw, Align::Center, cs.max(56.0));
    if gauge_h > 0.0 {
        gauge(pen, d, k, m, gauge_y, cw);
    }
    stickers(pen, &t_list, k, m, stick_y, cw, 4, th);
    // bigger than the trainer card's: the hunt image's footer has room to spare
    let fk = 1.3;
    pen.scaled(fk, |pen| footer_band(pen, k, "TRAINER", &o.trainer, d.trainer_level, "POKE TOGETHER COMPANION", d.icon.as_ref(), bottom / fk, band / fk));
}

// ───────────────────────────── trainer profile ─────────────────────────────

pub(super) fn profile(pen: &mut Pen, d: &ProfileData, o: &CardOptions) {
    let k = ink_for(o);
    match o.layout {
        CardLayout::Wide => profile_wide(pen, d, k),
        CardLayout::Square => profile_square(pen, d, k),
    }
}

/// Flat hexagon level badge with an ink outline.
fn flat_badge(pen: &mut Pen, d: &ProfileData, k: Ink, cx: f32, cy: f32, r: f32) {
    // the player's picture in a sticker frame instead, when there is one
    if pen.av.is_some() {
        pen.burst(cx, cy, r * 1.08, r * 1.42, 16, k.pop[0], k.ink, 0.1);
        pen.circle(cx + 8.0, cy + 10.0, r + 8.0, k.ink);
        pen.circle(cx, cy, r + 8.0, k.ink);
        pen.circle(cx, cy, r + 3.0, rgb(d.metal.1));
        pen.avatar(cx - r, cy, r * 2.0, k.ink);
        pen.twinkle(cx + r * 0.9, cy - r * 0.75, r * 0.2, WHITE, k.ink);
        return;
    }
    let main = rgb(d.metal.1);
    let light = rgb(d.metal.2);
    let hex = |r: f32, dx: f32, dy: f32| -> Vec<(f32, f32)> {
        (0..6)
            .map(|i| {
                let a = std::f32::consts::FRAC_PI_6 + i as f32 * std::f32::consts::FRAC_PI_3;
                (cx + dx + a.cos() * r, cy + dy + a.sin() * r)
            })
            .collect()
    };
    pen.burst(cx, cy, r * 1.08, r * 1.42, 16, k.pop[0], k.ink, 0.1);
    pen.poly(&hex(r + 6.0, 8.0, 10.0), k.ink);
    pen.poly(&hex(r + 6.0, 0.0, 0.0), k.ink);
    pen.poly(&hex(r, 0.0, 0.0), main);
    pen.poly(&hex(r * 0.78, 0.0, 0.0), k.ink);
    pen.poly(&hex(r * 0.72, 0.0, 0.0), light);
    pen.poly(&[(cx - r * 0.6, cy - r * 0.32), (cx, cy - r * 0.68), (cx - r * 0.14, cy - r * 0.46)], WHITE.alpha(0.7));
    let n = d.level.to_string();
    let size = if n.len() >= 3 { r * 0.58 } else { r * 0.78 };
    pen.lettering(&n, cx, cy - size * 0.62, size, WHITE, k.ink, (size / 12.0).max(3.0), Face::Display, Align::Center, None);
    pen.twinkle(cx + r * 0.9, cy - r * 0.75, r * 0.2, WHITE, k.ink);
}

/// The name, title tag and XP bar. `align` Left or Center.
#[allow(clippy::too_many_arguments)]
fn identity_text(pen: &mut Pen, d: &ProfileData, k: Ink, x: f32, y: f32, w: f32, align: Align, on_colour: bool) -> f32 {
    let name = if d.name.trim().is_empty() { "TRAINER".to_string() } else { d.name.trim().to_uppercase() };
    let fill = if on_colour { WHITE } else { k.paper };
    // with the player's picture in the badge's place, the level sits
    // next to the name
    let ns = if pen.av.is_some() {
        let lv = format!("LV {}", d.level);
        let ts = 20.0;
        let tw = pen.width(&lv, ts, Face::Bold) + ts * 1.6;
        let base = if pen.plate.is_empty() { 64.0 } else { 64.0 * PLATE_SHRINK };
        let (ns, _, _) = pen.name_inline(&name, x, w, base, Face::Display, align, tw + pen.plate_room(base, PLATE_K_BIG));
        let nw = pen.width(&name, ns, Face::Display);
        // name (on its plate) and tag placed as one group at the size used
        let (gap, edge) = (14.0, if pen.plate.is_empty() { 0.0 } else { 4.0 });
        let room = pen.plate_room(ns, PLATE_K_BIG);
        let all = edge + nw + room + gap + tw;
        let start = match align {
            Align::Center => x - all / 2.0,
            Align::Right => x - all,
            Align::Left => x,
        } + edge;
        let nx = start + pen.plate_under_k(start, y, nw, ns, Face::Display, PLATE_K_BIG);
        let tag_x = start + nw + room + gap;
        pen.lettering(&name, nx, y, ns, fill, k.ink, 5.0, Face::Display, Align::Left, Some(k.ink));
        // the tag's middle on the name's middle
        pen.tag(&lv, tag_x, y + pen.mid(ns, Face::Display) - ts * 0.875, ts, YELLOW, k.ink, Align::Left);
        ns
    } else {
        let base = if pen.plate.is_empty() { 64.0 } else { 64.0 * PLATE_SHRINK };
        let ns = pen.fit(&name, base, w - pen.plate_room(base, PLATE_K_BIG), Face::Display);
        let nw = pen.width(&name, ns, Face::Display);
        let all = nw + pen.plate_room(ns, PLATE_K_BIG);
        let left = match align {
            Align::Center => x - all / 2.0,
            Align::Right => x - all,
            Align::Left => x,
        };
        let off = pen.plate_under_k(left, y, nw, ns, Face::Display, PLATE_K_BIG);
        pen.lettering(&name, left + off, y, ns, fill, k.ink, 5.0, Face::Display, Align::Left, Some(k.ink));
        ns
    };
    // below the name (and its plate)
    let mut y = (y + ns * 1.3).max(pen.plate_bottom(y, ns, Face::Display, PLATE_K_BIG) + 12.0);
    let title = d.title.to_uppercase();
    // a battle pass title on a tag in its look's colour
    let tfill = crate::pass::title_theme(&d.title).map_or(YELLOW, |t| {
        let [r, g, b] = t.rgb();
        Col(r, g, b, 255)
    });
    pen.tag(&title, x, y, 17.0, tfill, k.ink, align);
    y += 17.0 * 1.75 + 16.0;
    // XP bar.
    let bw = w.min(420.0);
    let bx = match align {
        Align::Center => x - bw / 2.0,
        _ => x,
    };
    let frac = if d.max_level { 1.0 } else { d.xp_into as f32 / d.xp_need.max(1) as f32 };
    let bh = 22.0;
    pen.rrect(bx + 5.0, y + 6.0, bw, bh, bh / 2.0, k.ink);
    pen.rrect(bx - 3.0, y - 3.0, bw + 6.0, bh + 6.0, bh / 2.0 + 3.0, k.ink);
    pen.rrect(bx, y, bw, bh, bh / 2.0, WHITE);
    if frac > 0.0 {
        pen.rrect(bx, y, (bw * frac).max(bh), bh, bh / 2.0, k.pop[2]);
        pen.rrect(bx + 8.0, y + 4.0, ((bw * frac) - 16.0).max(0.0), 4.0, 2.0, WHITE.alpha(0.6));
    }
    y += bh + 12.0;
    let xp = if d.max_level {
        format!("{} XP · MAX LEVEL", thousands(d.total_xp))
    } else {
        format!("{} / {} XP TO LEVEL {}", thousands(d.xp_into), thousands(d.xp_need), d.level + 1)
    };
    let xs = pen.fit(&xp, 16.0, w, Face::Bold);
    pen.text(&xp, x, y, xs, if on_colour { WHITE } else { k.ink }, Face::Bold, align, false);
    y + xs + 8.0
}

/// Section heading: a small ink tag.
fn heading(pen: &mut Pen, k: Ink, text: &str, x: f32, y: f32) -> f32 {
    pen.tag(text, x, y, 14.0, k.ink, k.ink, Align::Left);
    y + 14.0 * 1.75 + 14.0
}

/// Shinies as polaroid-style stickers.
fn shinies_row(pen: &mut Pen, d: &ProfileData, k: Ink, x: f32, y: f32, w: f32, h: f32) {
    let y = heading(pen, k, "SHINY SHOWCASE", x, y);
    if d.shinies.is_empty() {
        pen.sticker(x, y, w, h, 16.0, WHITE, k.ink);
        pen.text("No shinies yet: the hunt continues!", x + w / 2.0, y + h / 2.0 - 12.0, 22.0, k.ink, Face::Bold, Align::Center, false);
        return;
    }
    let n = d.shinies.len().clamp(4, 6);
    let gap = 18.0;
    let sw = ((w - gap * (n as f32 - 1.0)) / n as f32).min(h * 0.95);
    for (i, (img, name, enc)) in d.shinies.iter().take(n).enumerate() {
        let sx = x + i as f32 * (sw + gap);
        pen.sticker(sx, y, sw, h, 12.0, WHITE, k.ink);
        let inner = (sx + 8.0, y + 8.0, sw - 16.0, h - 52.0);
        pen.rrect(inner.0, inner.1, inner.2, inner.3, 8.0, k.pop[i % 3].mix(WHITE, 0.35));
        pen.halftone(inner.0, inner.1, inner.2, inner.3, 10.0, 1.0, 0.0, WHITE.alpha(0.5));
        if let Some(img) = img {
            sprite_in(pen, img, inner.0 + 4.0, inner.1 + 4.0, inner.2 - 8.0, inner.3 - 6.0);
        }
        pen.twinkle(sx + sw - 14.0, y + 16.0, 8.0, YELLOW, k.ink);
        let ns = pen.fit(&name.to_uppercase(), 15.0, sw - 12.0, Face::Bold);
        pen.text(&name.to_uppercase(), sx + sw / 2.0, y + h - 40.0, ns, k.ink, Face::Bold, Align::Center, false);
        let e = format!("{} ENC.", thousands(*enc));
        let es = pen.fit(&e, 13.0, sw - 12.0, Face::Bold);
        pen.text(&e, sx + sw / 2.0, y + h - 20.0, es, k.deep, Face::Bold, Align::Center, false);
    }
}

/// Recent achievements as round medal stickers with names.
fn achievements_row(pen: &mut Pen, d: &ProfileData, k: Ink, x: f32, y: f32, w: f32, h: f32) {
    let label = format!("ACHIEVEMENTS {} / {}", d.ach_count.0, d.ach_count.1);
    let y = heading(pen, k, &label, x, y);
    if d.achievements.is_empty() {
        pen.sticker(x, y, w, h, 16.0, WHITE, k.ink);
        pen.text("Achievements unlock as you hunt", x + w / 2.0, y + h / 2.0 - 11.0, 20.0, k.ink, Face::Bold, Align::Center, false);
        return;
    }
    let n = d.achievements.len().clamp(4, 6);
    let gap = 18.0;
    let aw = (w - gap * (n as f32 - 1.0)) / n as f32;
    for (i, (name, tier, img)) in d.achievements.iter().take(n).enumerate() {
        let ax = x + i as f32 * (aw + gap);
        let tc = rgb(*tier);
        pen.sticker(ax, y, aw, h, 12.0, tc.mix(WHITE, 0.55), k.ink);
        let r = (h * 0.24).min(aw * 0.28);
        let (mx, my) = (ax + aw / 2.0, y + 12.0 + r);
        pen.circle(mx, my, r + 4.0, k.ink);
        pen.circle(mx, my, r, tc);
        match img {
            Some(img) => sprite_in(pen, img, mx - r * 0.85, my - r * 0.9, r * 1.7, r * 1.6),
            None => pen.twinkle(mx, my, r * 0.55, WHITE, k.ink),
        }
        let ns = pen.fit(&name.to_uppercase(), 13.0, aw - 12.0, Face::Bold);
        pen.text(&name.to_uppercase(), ax + aw / 2.0, y + h - 26.0, ns, k.ink, Face::Bold, Align::Center, false);
    }
}

fn profile_wide(pen: &mut Pen, d: &ProfileData, k: Ink) {
    let (w, h) = (pen.w, pen.h);
    let band = 56.0;
    let bottom = h - band;
    let (top_x, bot_x) = (440.0, 380.0);
    let (cx, cy) = (205.0, 190.0);
    pen.poly(&[(0.0, 0.0), (top_x, 0.0), (bot_x, bottom), (0.0, bottom)], k.main);
    pen.speed_lines(cx, cy, 150.0, 700.0, 50, k.soft.alpha(0.55), None);
    pen.poly(&[(top_x, 0.0), (w, 0.0), (w, bottom), (bot_x, bottom)], k.paper);
    pen.halftone(w - 300.0, 0.0, 300.0, 180.0, 16.0, 1.0, 0.0, k.soft.alpha(0.7));
    let stripe = |off: f32, wid: f32| -> Vec<(f32, f32)> { vec![(top_x + off, 0.0), (top_x + off + wid, 0.0), (bot_x + off + wid, bottom), (bot_x + off, bottom)] };
    pen.poly(&stripe(-3.0, 9.0), k.ink);
    pen.poly(&stripe(22.0, 14.0), k.pop[0]);
    pen.poly(&stripe(19.0, 3.0), k.ink);
    pen.poly(&stripe(36.0, 3.0), k.ink);

    flat_badge(pen, d, k, cx, cy, 92.0);
    identity_text(pen, d, k, cx, cy + 150.0, 330.0, Align::Center, true);

    let x0 = 500.0;
    let cw = w - 40.0 - x0;
    pen.ribbon("TRAINER CARD", x0, 30.0, 24.0, k.pop[1], k.ink);
    // Stats: 4 × 2 stickers.
    let gap = 18.0;
    let tw = (cw - gap * 3.0) / 4.0;
    let th = 70.0;
    let mut y = 98.0;
    for (r, row) in d.stats.chunks(4).take(2).enumerate() {
        for (i, (label, value)) in row.iter().enumerate() {
            let fill = k.pop[(i + r) % 3].mix(WHITE, 0.3);
            let tx = x0 + i as f32 * (tw + gap);
            pen.sticker(tx, y, tw, th, 12.0, fill, k.ink);
            let ls = pen.fit(&label.to_uppercase(), 12.5, tw - 20.0, Face::Bold);
            pen.text(&label.to_uppercase(), tx + 12.0, y + 9.0, ls, k.ink, Face::Bold, Align::Left, false);
            let vs = pen.fit(value, 26.0, tw - 24.0, Face::Display);
            pen.text(value, tx + 12.0, y + th - vs * 1.25 - 4.0, vs, k.ink, Face::Display, Align::Left, false);
        }
        y += th + gap;
    }
    y += 2.0;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { 140.0 } else { 230.0 };
        shinies_row(pen, d, k, x0, y, cw, sh);
        y += 40.0 + sh + 18.0;
    }
    if d.show_achievements {
        let ah = (bottom - 22.0 - y - 40.0).clamp(90.0, 200.0);
        achievements_row(pen, d, k, x0, y, cw, ah);
    }
    // the picture goes by the name, not the date
    let av = pen.av.take();
    let plate = std::mem::take(&mut pen.plate);
    footer_band(pen, k, "CARD MADE", &d.date, None, "POKE TOGETHER COMPANION", d.icon.as_ref(), bottom, band);
    pen.av = av;
    pen.plate = plate;
}

fn profile_square(pen: &mut Pen, d: &ProfileData, k: Ink) {
    let (w, h) = (pen.w, pen.h);
    let band = 60.0;
    let bottom = h - band;
    let (left_y, right_y) = (390.0, 330.0);
    let (cx, cy) = (210.0, 190.0);
    pen.poly(&[(0.0, 0.0), (w, 0.0), (w, right_y), (0.0, left_y)], k.main);
    pen.speed_lines(cx, cy, 150.0, 900.0, 60, k.soft.alpha(0.55), None);
    pen.poly(&[(0.0, left_y), (w, right_y), (w, h), (0.0, h)], k.paper);
    let stripe = |off: f32, wid: f32| -> Vec<(f32, f32)> { vec![(0.0, left_y + off), (w, right_y + off), (w, right_y + off + wid), (0.0, left_y + off + wid)] };
    pen.poly(&stripe(-3.0, 9.0), k.ink);
    pen.poly(&stripe(22.0, 14.0), k.pop[0]);
    pen.poly(&stripe(19.0, 3.0), k.ink);
    pen.poly(&stripe(36.0, 3.0), k.ink);

    flat_badge(pen, d, k, cx, cy, 100.0);
    pen.ribbon("TRAINER CARD", 400.0, 44.0, 22.0, k.pop[1], k.ink);
    identity_text(pen, d, k, 400.0, 110.0, w - 440.0, Align::Left, true);

    let m = 46.0;
    let cw = w - 2.0 * m;
    let gap = 18.0;
    let tw = (cw - gap * 3.0) / 4.0;
    let th = 80.0;
    let mut y = left_y + 54.0;
    for (r, row) in d.stats.chunks(4).take(2).enumerate() {
        for (i, (label, value)) in row.iter().enumerate() {
            let fill = k.pop[(i + r) % 3].mix(WHITE, 0.3);
            let tx = m + i as f32 * (tw + gap);
            pen.sticker(tx, y, tw, th, 12.0, fill, k.ink);
            let ls = pen.fit(&label.to_uppercase(), 13.0, tw - 20.0, Face::Bold);
            pen.text(&label.to_uppercase(), tx + 12.0, y + 10.0, ls, k.ink, Face::Bold, Align::Left, false);
            let vs = pen.fit(value, 30.0, tw - 24.0, Face::Display);
            pen.text(value, tx + 12.0, y + th - vs * 1.25 - 4.0, vs, k.ink, Face::Display, Align::Left, false);
        }
        y += th + gap;
    }
    y += 4.0;
    let both = d.show_shinies && d.show_achievements;
    if d.show_shinies {
        let sh = if both { 160.0 } else { 280.0 };
        shinies_row(pen, d, k, m, y, cw, sh);
        y += 40.0 + sh + 20.0;
    }
    if d.show_achievements {
        let ah = (bottom - 24.0 - y - 40.0).clamp(100.0, 240.0);
        achievements_row(pen, d, k, m, y, cw, ah);
    }
    // the picture goes by the name, not the date
    let av = pen.av.take();
    let plate = std::mem::take(&mut pen.plate);
    footer_band(pen, k, "CARD MADE", &d.date, None, "POKE TOGETHER COMPANION", d.icon.as_ref(), bottom, band);
    pen.av = av;
    pen.plate = plate;
}
