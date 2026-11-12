//! The rare grass infographic in the anime style: flat bold colours, thick
//! ink outlines, speed lines and halftone dots, comic lettering, the map in
//! a sticker frame with bubble pins, a podium for the top 3 and every area
//! as a sticker card. Same data and modes as the classic look.

use super::*;

const PAPER_A: Col = hex(0xfff7e8);
const INK_A: Col = hex(0x1c1830);
const GREEN: Col = hex(0x4ccd6e);
const YELLOW: Col = hex(0xffd23f);
const PINK: Col = hex(0xff689a);
const CYAN: Col = hex(0x3ccbe8);
const MINT: Col = hex(0x5be0a0);
const ORANGE: Col = hex(0xff9248);
const NIGHT: Col = hex(0x343b78);

/// Card and pin colours, one per area in turn.
const AREA_COLS: [Col; 6] = [GREEN, CYAN, PINK, YELLOW, ORANGE, MINT];

/// Podium colours: gold, silver, bronze.
const PODIUM: [Col; 3] = [hex(0xffc83a), hex(0xc9d4e2), hex(0xe08a4c)];

fn deep(c: Col) -> Col {
    c.mix(INK_A, 0.45)
}
fn soft(c: Col) -> Col {
    c.mix(WHITE, 0.45)
}

impl Pen<'_> {
    /// Comic lettering at a baseline: thick outline and a hard shadow.
    /// Returns the width.
    #[allow(clippy::too_many_arguments)]
    fn letter(&mut self, s: &str, x: f32, y: f32, size: f32, face: PFace, fill: Col, outline: Col, t: f32, al: Al, shadow: Option<Col>) -> f32 {
        let w = self.tw(s, size, face);
        let x = match al {
            Al::L => x,
            Al::C => x - w / 2.0,
            Al::R => x - w,
        };
        let r = self.r;
        let ring = |t: f32| -> Vec<(f32, f32)> {
            (0..16)
                .map(|i| {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU;
                    (a.cos() * t, a.sin() * t)
                })
                .collect()
        };
        if let Some(sc) = shadow {
            let off = (size / 14.0).clamp(3.0, 10.0);
            for (dx, dy) in ring(t) {
                self.t.draw(&mut self.c, s, (x + off + dx) * r, (y + off + dy) * r, size * r, sc, face);
            }
        }
        if t > 0.0 {
            for (dx, dy) in ring(t).into_iter().chain(ring(t * 0.5)) {
                self.t.draw(&mut self.c, s, (x + dx) * r, (y + dy) * r, size * r, outline, face);
            }
        }
        self.t.draw(&mut self.c, s, x * r, y * r, size * r, fill, face);
        w
    }

    /// The biggest size up to `size` at which `s` fits `maxw`.
    fn fit_size(&self, s: &str, size: f32, face: PFace, maxw: f32, min: f32) -> f32 {
        let w = self.tw(s, size, face);
        if w <= maxw {
            size
        } else {
            (size * maxw / w.max(1.0)).max(min)
        }
    }

    /// A flat sticker: ink outline, flat fill and a hard ink shadow.
    fn stk(&mut self, x: f32, y: f32, w: f32, h: f32, rad: f32, fill: Col) {
        self.rrect(x + 6.0, y + 7.0, w, h, rad, INK_A);
        self.rrect(x - 3.0, y - 3.0, w + 6.0, h + 6.0, rad + 3.0, INK_A);
        self.rrect(x, y, w, h, rad, fill);
    }

    /// A pill label with an ink outline, centred on `cy`. Returns its width.
    #[allow(clippy::too_many_arguments)]
    fn pill(&mut self, s: &str, x: f32, cy: f32, size: f32, fill: Col, text: Col, al: Al) -> f32 {
        let pad = size * 0.75;
        let w = self.tw(s, size, PFace::Bold) + pad * 2.0;
        let h = size * 1.7;
        let x = match al {
            Al::L => x,
            Al::C => x - w / 2.0,
            Al::R => x - w,
        };
        self.rrect(x - 2.5, cy - h / 2.0 - 2.5, w + 5.0, h + 5.0, h / 2.0 + 2.5, INK_A);
        self.rrect(x, cy - h / 2.0, w, h, h / 2.0, fill);
        self.text(s, x + pad, cy + size * 0.36, size, PFace::Bold, text, Al::L);
        w
    }

    /// A slanted ribbon with lettering. Returns its width.
    fn ribbon(&mut self, s: &str, x: f32, y: f32, size: f32, fill: Col) -> f32 {
        let tw = self.tw(s, size, PFace::Black);
        let h = size * 1.6;
        let lean = h * 0.35;
        let w = tw + size * 1.6 + lean;
        let shape = |dx: f32, dy: f32, g: f32| -> Vec<(f32, f32)> {
            vec![(x + lean + dx - g, y + dy - g), (x + w + dx + g, y + dy - g), (x + w - lean + dx + g, y + h + dy + g), (x + dx - g, y + h + dy + g)]
        };
        self.poly(&shape(5.0, 6.0, 0.0), INK_A);
        self.poly(&shape(0.0, 0.0, 3.0), INK_A);
        self.poly(&shape(0.0, 0.0, 0.0), fill);
        self.letter(s, x + lean / 2.0 + size * 0.8, y + h / 2.0 + size * 0.36, size, PFace::Black, WHITE, INK_A, 2.5, Al::L, None);
        w
    }

    /// Thin wedges radiating from (cx, cy).
    #[allow(clippy::too_many_arguments)]
    fn speed_lines(&mut self, cx: f32, cy: f32, r0: f32, r1: f32, n: usize, col: Col, seed: u64) {
        let mut rng = crate::canvas::Rng(seed);
        for i in 0..n {
            let a = (i as f32 + rng.next() * 0.6) / n as f32 * std::f32::consts::TAU;
            let half = 0.01 + rng.next() * 0.018;
            let inner = r0 * (0.8 + rng.next() * 0.4);
            self.poly(
                &[
                    (cx + a.cos() * inner, cy + a.sin() * inner),
                    (cx + (a - half).cos() * r1, cy + (a - half).sin() * r1),
                    (cx + (a + half).cos() * r1, cy + (a + half).sin() * r1),
                ],
                col,
            );
        }
    }

    /// Dots that grow towards (x + w·fx, y + h·fy).
    #[allow(clippy::too_many_arguments)]
    fn halftone(&mut self, x: f32, y: f32, w: f32, h: f32, step: f32, fx: f32, fy: f32, col: Col) {
        let (ox, oy) = (x + w * fx, y + h * fy);
        let reach = (w * w + h * h).sqrt() * 0.75;
        let mut row = 0;
        let mut yy = y;
        while yy <= y + h {
            let mut xx = x + if row % 2 == 1 { step / 2.0 } else { 0.0 };
            while xx <= x + w {
                let d = ((xx - ox).powi(2) + (yy - oy).powi(2)).sqrt() / reach;
                let rad = step * 0.42 * (1.0 - d).clamp(0.0, 1.0);
                if rad > 0.6 {
                    self.circle(xx, yy, rad, col);
                }
                xx += step;
            }
            yy += step;
            row += 1;
        }
    }

    /// A four-point twinkle with an ink outline.
    fn twinkle(&mut self, cx: f32, cy: f32, size: f32, fill: Col) {
        self.star(cx, cy, size + 4.0, INK_A);
        self.star(cx, cy, size, fill);
    }

    /// A Pokémon as a sticker: white border, ink edge and a hard shadow,
    /// standing on `floor` (or centred on it when `centred`). Returns the
    /// size drawn, in layout units.
    fn mon(&mut self, img: Option<&Img>, cx: f32, floor: f32, box_: f32, edge: f32, centred: bool) -> Option<(f32, f32)> {
        let img = img?;
        let (bx, by, bw, bh) = Canvas::trim_box(img);
        if bw == 0 || bh == 0 {
            return None;
        }
        let spr = crop(img, (bx, by, bw, bh));
        let k = fit_scale(bw as f32, bh as f32, box_);
        let kr0 = k * self.r;
        let kr = if kr0 >= 1.0 { kr0.round().max(1.0) } else { kr0 };
        let (pw, ph) = ((bw as f32 * kr).round().max(1.0), (bh as f32 * kr).round().max(1.0));
        let x = cx * self.r - pw / 2.0;
        let y = if centred { floor * self.r - ph / 2.0 } else { floor * self.r - ph };
        let o = edge * self.r;
        let ring = |rad: f32| -> Vec<(f32, f32)> {
            (0..12)
                .map(|i| {
                    let a = i as f32 / 12.0 * std::f32::consts::TAU;
                    (a.cos() * rad, a.sin() * rad)
                })
                .collect()
        };
        let sh = (edge * 0.9).max(2.0) * self.r;
        self.mask(&spr, x + sh, y + sh * 1.1, kr, INK_A.alpha(0.9));
        for (dx, dy) in ring(o * 1.7) {
            self.mask(&spr, x + dx, y + dy, kr, INK_A);
        }
        for (dx, dy) in ring(o).into_iter().chain(ring(o * 0.5)) {
            self.mask(&spr, x + dx, y + dy, kr, WHITE);
        }
        if kr >= 1.0 {
            self.c.blit_nearest(&spr, x, y, kr);
        } else {
            self.c.blit_smooth(&spr, x, y, pw, ph);
        }
        Some((pw / self.r, ph / self.r))
    }

    /// A flat sun or moon with an ink outline.
    fn sky(&mut self, cx: f32, cy: f32, day: bool) {
        if day {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                self.line(cx + a.cos() * 7.0, cy + a.sin() * 7.0, cx + a.cos() * 11.5, cy + a.sin() * 11.5, 2.6, INK_A);
            }
            self.circle(cx, cy, 7.5, INK_A);
            self.circle(cx, cy, 5.5, YELLOW);
        } else {
            self.circle(cx, cy, 8.0, INK_A);
            self.circle(cx, cy, 6.0, hex(0xf3f6fb));
            self.circle(cx + 3.4, cy - 2.6, 4.6, NIGHT);
        }
    }

    /// Paint everything outside a rounded rectangle (inside the rectangle's
    /// box) with `col`: rounds the corners of something already drawn.
    fn round_corners(&mut self, x: f32, y: f32, w: f32, h: f32, rad: f32, col: Col) {
        let r = self.r;
        let (x0, y0, x1, y1, rr) = (x * r, y * r, (x + w) * r, (y + h) * r, rad * r);
        for (cx, cy, sx, sy) in [(x0 + rr, y0 + rr, -1.0, -1.0), (x1 - rr, y0 + rr, 1.0, -1.0), (x0 + rr, y1 - rr, -1.0, 1.0), (x1 - rr, y1 - rr, 1.0, 1.0)] {
            let (bx0, bx1) = if sx < 0.0 { (x0, cx) } else { (cx, x1) };
            let (by0, by1) = if sy < 0.0 { (y0, cy) } else { (cy, y1) };
            self.c.fill_with(bx0.floor() as i32, by0.floor() as i32, bx1.ceil() as i32, by1.ceil() as i32, |px, py| {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                if (fx - cx) * sx < 0.0 || (fy - cy) * sy < 0.0 {
                    return None;
                }
                let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
                let cov = (d - rr + 0.5).clamp(0.0, 1.0);
                (cov > 0.0).then_some((col, cov))
            });
        }
    }
}

/// A pin or card colour for an area.
fn area_col(i: usize) -> Col {
    AREA_COLS[i % AREA_COLS.len()]
}

/// Draw the infographic in the anime style.
pub(super) fn render(data: &PosterData, mode: PosterMode, t: &dyn PosterText) -> Canvas {
    let map_only = mode == PosterMode::MapOnly;
    let lists_only = mode == PosterMode::Lists;
    const W: f32 = 1920.0;
    const M: f32 = 64.0;

    let (areas, tally, top3) = prepare(data);
    let name_of = |key: &str| -> String {
        areas.iter().flat_map(|a| a.mons.iter()).find(|m| m.key == key).map(|m| m.name.clone()).unwrap_or_default()
    };
    let sprite_of = |key: &str| -> Option<&Img> {
        areas.iter().flat_map(|a| a.mons.iter()).find(|m| m.key == key).and_then(|m| m.sprite.as_ref())
    };

    // ── measure
    const HEAD_H: f32 = 270.0;
    let hero_h = if lists_only { HEAD_H } else { 1180.0 };
    let band_h = if top3.is_empty() { 0.0 } else { 540.0 };
    let rows_total: usize = areas.iter().map(|a| a.mons.len().max(1)).sum();
    // row, sprite box, name size
    let (row, sbox, fname): (f32, f32, f32) = if rows_total <= 10 {
        (68.0, 54.0, 21.0)
    } else if rows_total <= 28 {
        (60.0, 48.0, 19.0)
    } else {
        (52.0, 42.0, 17.0)
    };
    const COLS: usize = 4;
    const GAP: f32 = 40.0;
    const VGAP: f32 = 34.0;
    const CARD_HEAD: f32 = 60.0;
    const CARD_PAD: f32 = 12.0;
    let cw = (W - 2.0 * M - (COLS as f32 - 1.0) * GAP) / COLS as f32;
    let mut col_h = [0.0f32; COLS];
    let mut placed: Vec<Entry> = Vec::new();
    for a in &areas {
        let h = CARD_HEAD + a.mons.len().max(1) as f32 * row + CARD_PAD * 2.0;
        let ci = (0..COLS).min_by(|&i, &j| col_h[i].total_cmp(&col_h[j])).unwrap_or(0);
        placed.push(Entry { area: a, col: ci, y: col_h[ci] });
        col_h[ci] += h + VGAP;
    }
    let index_y = hero_h + band_h;
    const INDEX_HEAD: f32 = 190.0;
    let tallest = if areas.is_empty() { 90.0 } else { col_h.iter().cloned().fold(0.0, f32::max) };
    let index_h = INDEX_HEAD + tallest + 30.0;
    const FOOT_H: f32 = 72.0;
    let h = if map_only { hero_h + FOOT_H } else { index_y + index_h + FOOT_H };

    let r = pick_scale(h);
    let mut p = Pen { c: Canvas::new((W * r).round() as u32, (h * r).round() as u32, PAPER_A), r, t };

    let zone = worldclock::server_zone_name(data.next);

    if lists_only {
        header(&mut p, data, zone, W, M, HEAD_H);
    } else {
        hero(&mut p, data, &areas, zone, W, hero_h);
    }

    if !map_only {
        // ── most widespread: a podium for the top 3
        if !top3.is_empty() {
            let by = hero_h;
            p.rect(0.0, by, W, band_h, YELLOW);
            p.halftone(0.0, by, W * 0.45, band_h, 20.0, 0.0, 1.0, ORANGE.alpha(0.35));
            p.halftone(W * 0.6, by, W * 0.4, band_h, 20.0, 1.0, 0.0, ORANGE.alpha(0.35));
            p.rect(0.0, by, W, 8.0, INK_A);
            let rw = p.ribbon("MOST WIDESPREAD", M, by + 40.0, 34.0, PINK);
            p.text("Pokémon found in the most areas today. Ties share a place.", M + rw + 26.0, by + 84.0, 20.0, PFace::Bold, INK_A, Al::L);

            const PW: f32 = 520.0;
            const PGAP: f32 = 34.0;
            let x0 = (W - 3.0 * PW - 2.0 * PGAP) / 2.0;
            let floor = by + band_h - 46.0;
            // left to right: 2nd, 1st, 3rd
            for (slot, rank) in [1usize, 0, 2].into_iter().enumerate() {
                let bh = [196.0, 160.0, 136.0][rank];
                let (x, top) = (x0 + slot as f32 * (PW + PGAP), floor - bh);
                let col = PODIUM[rank];
                match top3.get(rank) {
                    Some((n, keys)) => {
                        // the Pokémon standing on the block
                        let k = keys.len().min(4);
                        let big = [150.0, 128.0, 118.0][rank];
                        let size = match k {
                            1 => big,
                            2 => big * 0.8,
                            _ => big * 0.62,
                        };
                        let step = size * 0.82;
                        for (j, key) in keys.iter().take(4).enumerate() {
                            let cx = x + PW / 2.0 + (j as f32 - (k as f32 - 1.0) / 2.0) * step;
                            if p.mon(sprite_of(key), cx, top - 10.0, size, 3.0, false).is_none() {
                                p.circle(cx, top - 10.0 - size * 0.35, size * 0.3, WHITE);
                                p.ring(cx, top - 10.0 - size * 0.35, size * 0.3, 4.0, INK_A);
                            }
                        }
                        if rank == 0 {
                            p.twinkle(x + PW / 2.0 + big * 0.62, top - big * 0.9, 18.0, WHITE);
                            p.twinkle(x + PW / 2.0 - big * 0.66, top - big * 0.55, 12.0, PINK);
                        }
                        p.stk(x, top, PW, bh, 18.0, col);
                        p.halftone(x, top, PW, bh, 14.0, 1.0, 1.0, deep(col).alpha(0.18));
                        // big rank number
                        p.letter(&(rank + 1).to_string(), x + 62.0, top + bh * 0.5 + 42.0, 112.0, PFace::Black, WHITE, INK_A, 6.0, Al::C, Some(INK_A));
                        let tx = x + 128.0;
                        let maxw = x + PW - 22.0 - tx;
                        let names: Vec<String> = keys.iter().map(|k| name_of(k)).collect();
                        let label = if names.len() == 2 { names.join(" & ") } else { names.join(", ") };
                        let fs = p.fit_size(&label, if k == 1 { 40.0 } else { 32.0 }, PFace::Black, maxw, 22.0);
                        let fitted = p.fit(&label, fs, PFace::Black, maxw);
                        let ty = top + 30.0 + fs * 0.85;
                        p.text(&fitted, tx, ty, fs, PFace::Black, INK_A, Al::L);
                        let count = format!(
                            "{}{}",
                            if *n == 1 { "1 AREA".to_string() } else { format!("{n} AREAS") },
                            if keys.len() > 1 { " EACH" } else { "" }
                        );
                        p.pill(&count, tx, ty + 30.0, 15.0, WHITE, INK_A, Al::L);
                        let wher: Vec<String> = areas
                            .iter()
                            .filter(|a| a.mons.iter().any(|m| keys.contains(&m.key)))
                            .map(|a| a.name.strip_prefix("Route ").unwrap_or(&a.name).to_string())
                            .collect();
                        let lines = p.wrap(&wher.join(", "), 15.5, PFace::Bold, maxw);
                        let room = ((top + bh - 14.0 - (ty + 62.0)) / 21.0).floor().max(0.0) as usize + 1;
                        for (li, l) in lines.iter().take(room).enumerate() {
                            let l = if li + 1 == room && lines.len() > room { p.fit(&format!("{l} …"), 15.5, PFace::Bold, maxw) } else { l.clone() };
                            p.text(&l, tx, ty + 72.0 + li as f32 * 21.0, 15.5, PFace::Bold, INK_A.mix(col, 0.22), Al::L);
                        }
                    }
                    None => {
                        let faded = col.mix(YELLOW, 0.6);
                        p.stk(x, top, PW, bh, 18.0, faded);
                        p.letter(&(rank + 1).to_string(), x + 62.0, top + bh * 0.5 + 42.0, 112.0, PFace::Black, faded.mix(WHITE, 0.4), INK_A.alpha(0.5), 4.0, Al::C, None);
                        p.text("NOBODY YET", x + 128.0, top + bh / 2.0 + 6.0, 30.0, PFace::Black, INK_A.alpha(0.5), Al::L);
                        p.circle(x + PW / 2.0, top - 60.0, 44.0, faded.mix(WHITE, 0.3));
                        p.ring(x + PW / 2.0, top - 60.0, 44.0, 4.0, INK_A.alpha(0.5));
                        p.letter("?", x + PW / 2.0, top - 38.0, 64.0, PFace::Black, WHITE, INK_A.alpha(0.5), 3.0, Al::C, None);
                    }
                }
            }
            p.rect(0.0, by + band_h - 8.0, W, 8.0, INK_A);
        }

        // ── where to find them: every area as a sticker card
        let iy0 = index_y;
        p.rect(0.0, iy0, W, h - iy0, PAPER_A);
        p.halftone(W - 520.0, iy0, 520.0, 220.0, 18.0, 1.0, 0.0, GREEN.alpha(0.35));
        p.letter("WHERE TO FIND THEM", M, iy0 + 108.0, 70.0, PFace::Black, WHITE, INK_A, 5.0, Al::L, Some(GREEN));
        p.text(
            "In route order, from your game log plus the shared list. Use Sweet Scent on the rare grass!",
            M,
            iy0 + 150.0,
            20.0,
            PFace::Bold,
            INK_A.mix(PAPER_A, 0.3),
            Al::L,
        );
        let iy = iy0 + INDEX_HEAD;
        if areas.is_empty() {
            p.text("Nothing met on rare grass yet since the reset.", M, iy + 40.0, 22.0, PFace::Bold, INK_A, Al::L);
        }
        for e in &placed {
            let ai = areas.iter().position(|a| std::ptr::eq(a, e.area)).unwrap_or(0);
            let (x, y, a) = (M + e.col as f32 * (cw + GAP), iy + e.y, e.area);
            let col = area_col(ai);
            let ch = CARD_HEAD + a.mons.len().max(1) as f32 * row + CARD_PAD * 2.0;
            p.stk(x, y, cw, ch, 16.0, WHITE);
            // coloured head with the route badge and name
            p.rrect(x, y, cw, CARD_HEAD, 16.0, col);
            p.rect(x, y + CARD_HEAD - 16.0, cw, 16.0, col);
            p.rect(x, y + CARD_HEAD, cw, 3.0, INK_A);
            let (bx, bcy, br) = (x + 34.0, y + CARD_HEAD / 2.0, 23.0);
            p.circle(bx, bcy, br + 3.0, INK_A);
            p.circle(bx, bcy, br, INK_A.mix(col, 0.15));
            match route_no(&a.name) {
                Some(n) => {
                    let s = n.to_string();
                    let fs = p.fit_size(&s, 24.0, PFace::Black, br * 1.7, 12.0);
                    p.text(&s, bx - 1.0, bcy + fs * 0.36, fs, PFace::Black, WHITE, Al::C);
                }
                None => {
                    let (gx, gy, gr) = (bx, bcy - 2.0, 8.0);
                    p.poly(&[(gx - gr * 0.86, gy), (gx + gr * 0.86, gy), (gx, gy + gr * 1.45)], WHITE);
                    p.circle(gx, gy - gr * 0.35, gr, WHITE);
                    p.circle(gx, gy - gr * 0.35, gr * 0.42, INK_A);
                }
            }
            let tx = bx + br + 14.0;
            let maxw = x + cw - 14.0 - tx;
            let title = a.name.to_uppercase();
            let fs = p.fit_size(&title, 21.0, PFace::Black, maxw, 15.0);
            let title = p.fit(&title, fs, PFace::Black, maxw);
            p.text(&title, tx, bcy + fs * 0.36, fs, PFace::Black, INK_A, Al::L);
            // the Pokémon
            let rows_y = y + CARD_HEAD + CARD_PAD;
            for (i, m) in a.mons.iter().enumerate() {
                let cy = rows_y + i as f32 * row + row / 2.0;
                if i > 0 {
                    let ly = rows_y + i as f32 * row;
                    p.line(x + 16.0, ly, x + cw - 16.0, ly, 1.5, col.mix(WHITE, 0.4));
                }
                let sx = x + 16.0 + sbox / 2.0;
                if p.mon(m.sprite.as_ref(), sx, cy, sbox, 2.0, true).is_none() {
                    p.circle(sx, cy, sbox * 0.3, col);
                    p.ring(sx, cy, sbox * 0.3, 3.0, INK_A);
                }
                let nx = sx + sbox / 2.0 + 14.0;
                let tag = if m.spots > 1 { format!("{} TILES", m.spots) } else { "1 TILE".to_string() };
                let tag_w = p.tw(&tag, 12.5, PFace::Bold) + 20.0;
                let nmax = x + cw - 16.0 - tag_w - 12.0 - nx;
                let nm = p.fit(&m.name, fname, PFace::Bold, nmax);
                p.text(&nm, nx, cy + fname * 0.36, fname, PFace::Bold, INK_A, Al::L);
                p.pill(&tag, x + cw - 16.0, cy, 12.5, soft(col), INK_A, Al::R);
            }
        }
    }

    // ── footer band
    let fy = h - FOOT_H;
    p.rect(0.0, fy, W, FOOT_H, INK_A);
    p.rect(0.0, fy, W, 6.0, GREEN);
    let cy = fy + 6.0 + (FOOT_H - 6.0) / 2.0;
    p.star(M + 10.0, cy, 13.0, YELLOW);
    p.text("POKE TOGETHER COMPANION", M + 34.0, cy + 7.0, 20.0, PFace::Black, WHITE, Al::L);
    if map_only {
        let (nm, na) = (tally.len(), areas.len());
        let mid = format!("{na} {} WITH RARE GRASS · {nm} POKÉMON", if na == 1 { "AREA" } else { "AREAS" });
        p.pill(&mid, W / 2.0, cy, 15.0, GREEN, INK_A, Al::C);
    }
    let zone_long = if zone == "CEST" { "Central European Summer Time" } else if zone == "UTC" { "Coordinated Universal Time" } else { "Central European Time" };
    p.text(&format!("Server time is {zone} ({zone_long})"), W - M, cy + 6.0, 16.0, PFace::Bold, WHITE.mix(INK_A, 0.35), Al::R);
    p.c
}

/// The title band of the lists-only picture.
fn header(p: &mut Pen, data: &PosterData, zone: &str, w: f32, m: f32, head_h: f32) {
    p.rect(0.0, 0.0, w, head_h, GREEN);
    p.speed_lines(260.0, head_h * 0.5, 160.0, 1400.0, 70, soft(GREEN).alpha(0.55), 0x7a11_57ad);
    p.halftone(w - 700.0, 0.0, 700.0, head_h, 18.0, 1.0, 0.0, deep(GREEN).alpha(0.35));
    p.pill("POKÉMON PLATINUM TOGETHER", m, 52.0, 15.0, INK_A, WHITE, Al::L);
    let tw = p.letter("RARE GRASS!", m, 190.0, 118.0, PFace::Black, WHITE, INK_A, 7.0, Al::L, Some(INK_A));
    p.twinkle(m + tw + 36.0, 92.0, 26.0, YELLOW);
    p.twinkle(m + tw + 72.0, 140.0, 13.0, WHITE);
    // the date as a sticker on the right
    let date = date_label(data.reset).to_uppercase();
    let fs = 40.0;
    let dw = p.tw(&date, fs, PFace::Black) + 56.0;
    let (dx, dy) = (w - m - dw, 62.0);
    p.stk(dx, dy, dw, 76.0, 16.0, YELLOW);
    p.text(&date, dx + 28.0, dy + 38.0 + fs * 0.36, fs, PFace::Black, INK_A, Al::L);
    let span = format!("{}, 12:00 am → {}, 12:00 am ({zone})", short_date(data.reset), short_date(data.next));
    p.pill(&span, w - m, dy + 120.0, 15.0, WHITE, INK_A, Al::R);
    p.rect(0.0, head_h - 8.0, w, 8.0, INK_A);
}

/// The top: title and reset times on the left, the map in a sticker frame
/// on the right with a bubble pin for every rare grass patch.
fn hero(p: &mut Pen, data: &PosterData, areas: &[PosterArea], zone: &str, w: f32, hero_h: f32) {
    p.rect(0.0, 0.0, w, hero_h, GREEN);
    p.speed_lines(290.0, 300.0, 230.0, 1900.0, 90, soft(GREEN).alpha(0.5), 0x5eed_9a55);
    p.halftone(0.0, hero_h - 420.0, 640.0, 420.0, 18.0, 0.0, 1.0, deep(GREEN).alpha(0.3));

    // ── the map frame
    let (fy, fh) = (40.0, hero_h - 80.0);
    let (fw, scale) = match &data.map {
        Some(mp) => {
            let (_, _, tw, th) = mp.fit;
            let scale = fh / th;
            (tw * scale, scale)
        }
        None => (1300.0, 1.0),
    };
    let fx = w - 46.0 - fw;
    let sea = hex(0x56b8e0);
    p.stk(fx, fy, fw, fh, 26.0, sea);
    let to_px = |tx: f32, ty: f32| -> (f32, f32) {
        match &data.map {
            Some(mp) => (fx + (tx - mp.fit.0) * scale, fy + (ty - mp.fit.1) * scale),
            None => (fx, fy),
        }
    };
    match &data.map {
        Some(mp) => {
            // the picture, cut to the frame
            let r = p.r;
            let (pw, ph) = ((fw * r).round() as u32, (fh * r).round() as u32);
            let mut tmp = Canvas::new(pw, ph, sea);
            let (ix, iy) = ((mp.x0 - mp.fit.0) * scale, (mp.y0 - mp.fit.1) * scale);
            let (iw, ih) = (mp.img.w as f32 / mp.px_per_tile * scale, mp.img.h as f32 / mp.px_per_tile * scale);
            tmp.blit_smooth(&mp.img, ix * r, iy * r, iw * r, ih * r);
            let (ox, oy) = ((fx * r).round() as i32, (fy * r).round() as i32);
            p.c.fill_with(ox, oy, ox + pw as i32, oy + ph as i32, |x, y| {
                let (tx, ty) = ((x - ox) as u32, (y - oy) as u32);
                if tx >= pw || ty >= ph {
                    return None;
                }
                let i = ((ty * pw + tx) * 4) as usize;
                Some((Col(tmp.px[i], tmp.px[i + 1], tmp.px[i + 2], 255), 1.0))
            });
            p.round_corners(fx, fy, fw, fh, 26.0, INK_A);
        }
        None => {
            p.text("MAP UNAVAILABLE", fx + fw / 2.0, fy + fh / 2.0, 40.0, PFace::Black, WHITE, Al::C);
        }
    }
    p.rrect_stroke(fx, fy, fw, fh, 26.0, 6.0, INK_A);

    if data.map.is_some() {
        // city and town names
        let mut taken: Vec<(f32, f32, f32, f32)> = Vec::new();
        for l in &data.labels {
            let (cx, cy) = to_px(l.x, l.y);
            let name = l.name.trim_end_matches(" City").trim_end_matches(" Town");
            let fs = if l.city { 21.0 } else { 17.0 };
            p.letter(name, cx, cy - 6.0, fs, PFace::Bold, WHITE, INK_A, 3.2, Al::C, None);
            let tw = p.tw(name, fs, PFace::Bold);
            taken.push((cx - tw / 2.0 - 4.0, cy - 6.0 - fs, tw + 8.0, fs + 8.0));
        }
        let (pins, sb) = place_pins(areas, &to_px, &taken, fx + 8.0, fx + fw - 8.0, fy + 8.0, fy + fh - 8.0);
        for pin in &pins {
            let moved = ((pin.x - pin.ox).powi(2) + (pin.y - pin.oy).powi(2)).sqrt() > 6.0;
            if moved {
                p.line(pin.ox, pin.oy, pin.x, pin.y, 3.0, INK_A);
            }
            p.circle(pin.ox, pin.oy, 6.0, INK_A);
            p.circle(pin.ox, pin.oy, 3.0, YELLOW);
        }
        for pin in &pins {
            let a = &areas[pin.area];
            let col = area_col(pin.area);
            let mons: Vec<&PosterMon> = pin.mons.iter().take(4).filter_map(|&i| a.mons.get(i)).collect();
            let b = sb * 0.72;
            let step = b * 0.7;
            let n = mons.len().max(1) as f32;
            let (bw, bh) = (b + (n - 1.0) * step + 14.0, b + 8.0);
            let (bx, by) = (pin.x - bw / 2.0, pin.y - bh / 2.0);
            p.rrect(bx + 3.0, by + 4.0, bw, bh, bh / 2.0, INK_A);
            p.rrect(bx - 3.0, by - 3.0, bw + 6.0, bh + 6.0, bh / 2.0 + 3.0, INK_A);
            p.rrect(bx, by, bw, bh, bh / 2.0, soft(col).mix(WHITE, 0.35));
            for (i, mon) in mons.iter().enumerate() {
                let ix = pin.x + (i as f32 - (n - 1.0) / 2.0) * step;
                if p.sticker(mon.sprite.as_ref(), ix, pin.y, b, 0.0).is_none() {
                    p.circle(ix, pin.y, b * 0.28, col);
                    p.ring(ix, pin.y, b * 0.28, 2.5, INK_A);
                }
            }
        }
    }
    // a little "SINNOH" tag on the frame
    p.pill("SINNOH", fx + 30.0, fy + 4.0, 16.0, YELLOW, INK_A, Al::L);

    // ── title column
    let (x, cw) = (48.0, fx - 48.0 - 40.0);
    p.pill("POKÉMON PLATINUM TOGETHER", x, 70.0, 15.0, INK_A, WHITE, Al::L);
    let big = p.fit_size("GRASS!", 150.0, PFace::Black, cw - 20.0, 80.0);
    p.letter("RARE", x, 112.0 + big * 0.86, big, PFace::Black, WHITE, INK_A, 7.0, Al::L, Some(INK_A));
    let gw = p.letter("GRASS!", x, 112.0 + big * 1.86, big, PFace::Black, WHITE, INK_A, 7.0, Al::L, Some(INK_A));
    let rw = p.tw("RARE", big, PFace::Black);
    p.twinkle(x + rw + 44.0, 112.0 + big * 0.4, 26.0, YELLOW);
    p.twinkle(x + gw.min(cw - 30.0) - 10.0, 112.0 + big * 0.98, 12.0, WHITE);
    let mut y = 112.0 + big * 1.86 + 46.0;

    // the date sticker
    let date = date_label(data.reset).to_uppercase();
    let fs = p.fit_size(&date, 34.0, PFace::Black, cw - 44.0, 20.0);
    p.stk(x, y, cw, 64.0, 16.0, YELLOW);
    p.text(&date, x + 22.0, y + 32.0 + fs * 0.36, fs, PFace::Black, INK_A, Al::L);
    y += 64.0 + 34.0;
    let span = format!("{}, 12:00 am → {}, 12:00 am", short_date(data.reset), short_date(data.next));
    let sfs = p.fit_size(&span, 18.0, PFace::Bold, cw, 13.0);
    p.text(&span, x, y, sfs, PFace::Bold, INK_A, Al::L);
    p.text(&format!("Server time ({zone})"), x, y + 25.0, 15.0, PFace::Bold, deep(GREEN), Al::L);
    y += 64.0;

    // ── reset time around the world
    let rows = worldclock::rows(data.next);
    let card_y = y;
    let card_h = hero_h - 40.0 - card_y;
    p.stk(x, card_y, cw, card_h, 18.0, PAPER_A);
    p.ribbon("RESET AROUND THE WORLD", x + 14.0, card_y - 22.0, 19.0, PINK);
    let head = 46.0;
    let foot = 34.0;
    let rowh = ((card_h - head - foot) / rows.len().max(1) as f32).min(36.0);
    let rx0 = x + 18.0;
    let barw = (cw * 0.34).clamp(110.0, 190.0);
    let barx = x + cw - 18.0 - barw;
    let timex = barx - 56.0;
    let y0 = card_y + head;
    for (l, hr) in [("12a", 0.0), ("6a", 6.0), ("12p", 12.0), ("6p", 18.0), ("12a", 24.0)] {
        p.text(l, barx + barw * hr / 24.0, y0 - 6.0, 12.0, PFace::Bold, INK_A.mix(PAPER_A, 0.45), Al::C);
    }
    for (i, rw) in rows.iter().enumerate() {
        let cy = y0 + i as f32 * rowh + rowh / 2.0;
        if rw.server {
            p.rrect(x + 8.0, cy - rowh / 2.0 + 2.0, cw - 16.0, rowh - 4.0, 9.0, YELLOW);
        }
        let t12 = rw.time12();
        let tfs = 20.0;
        let room = timex - p.tw(&t12, tfs, PFace::Black) - 12.0 - rx0;
        let lfs = p.fit_size(rw.label, 16.0, PFace::Bold, room, 12.5);
        let label = p.fit(rw.label, lfs, PFace::Bold, room);
        p.text(&label, rx0, cy + 5.5, lfs, PFace::Bold, INK_A, Al::L);
        p.text(&t12, timex, cy + 7.0, tfs, PFace::Black, INK_A, Al::R);
        p.text(rw.weekday, timex + 8.0, cy + 5.0, 12.5, PFace::Bold, INK_A.mix(PAPER_A, 0.4), Al::L);
        // flat 24 hour bar: night and day
        let (bh, by) = (12.0, cy - 6.0);
        p.rrect(barx - 2.0, by - 2.0, barw + 4.0, bh + 4.0, bh / 2.0 + 2.0, INK_A);
        p.rrect(barx, by, barw, bh, bh / 2.0, NIGHT);
        p.rect(barx + barw * 0.27, by, barw * 0.46, bh, hex(0x8fd8f5));
        let frac = (rw.hour as f32 + rw.minute as f32 / 60.0) / 24.0;
        p.sky(barx + barw * frac, cy, (6..18).contains(&rw.hour));
    }
    p.text(
        &format!("Local time of the daily 12am {zone} reset"),
        rx0,
        card_y + card_h - 14.0,
        13.5,
        PFace::Bold,
        INK_A.mix(PAPER_A, 0.4),
        Al::L,
    );
}
