//! Small, dependency-free image helpers working on raw RGBA buffers.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(not(offline_test), derive(serde::Serialize, serde::Deserialize))]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Region {
    pub fn is_valid(&self) -> bool {
        self.w >= 4 && self.h >= 4
    }
    /// Clamp to an image of size `iw`×`ih`.
    pub fn clamped(&self, iw: u32, ih: u32) -> Option<Region> {
        if self.x >= iw || self.y >= ih {
            return None;
        }
        let w = self.w.min(iw - self.x);
        let h = self.h.min(ih - self.y);
        let r = Region {
            x: self.x,
            y: self.y,
            w,
            h,
        };
        r.is_valid().then_some(r)
    }
}

/// An owned RGBA8 image.
#[derive(Clone)]
pub struct Rgba {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

/// An owned 8-bit grayscale image.
#[derive(Clone)]
pub struct Gray {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Rgba {
    pub fn crop(&self, r: Region) -> Option<Rgba> {
        let r = r.clamped(self.w, self.h)?;
        let mut px = Vec::with_capacity((r.w * r.h * 4) as usize);
        for y in r.y..r.y + r.h {
            let start = ((y * self.w + r.x) * 4) as usize;
            px.extend_from_slice(&self.px[start..start + (r.w * 4) as usize]);
        }
        Some(Rgba { w: r.w, h: r.h, px })
    }

    pub fn luma(&self) -> Gray {
        let px = self
            .px
            .chunks_exact(4)
            .map(|p| ((p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000) as u8)
            .collect();
        Gray {
            w: self.w,
            h: self.h,
            px,
        }
    }

    /// Number of "shiny star yellow" pixels. Name text is white and the
    /// level text sits outside the star zone, so yellow here means ★.
    pub fn count_yellow(&self) -> u32 {
        self.px
            .chunks_exact(4)
            .filter(|p| {
                let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
                r >= 200 && g >= 150 && b <= 110 && r - b > 100
            })
            .count() as u32
    }

    /// Looks for one shiny ★ in this crop: a single solid yellow shape that
    /// is roughly square, about half filled (a star, not a bar, a block or a
    /// thin streak) and mirror-symmetric left to right. Returns its centre and
    /// pixel count. Scattered yellow (sparkles, effects, the level text)
    /// doesn't pass.
    pub fn find_star(&self, min_px: u32) -> Option<(f32, f32, u32)> {
        let (w, h) = (self.w as i32, self.h as i32);
        let n = (w * h) as usize;
        let mask: Vec<bool> = self
            .px
            .chunks_exact(4)
            .map(|p| {
                let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
                r >= 200 && g >= 150 && b <= 110 && r - b > 100
            })
            .collect();
        if mask.len() != n {
            return None;
        }
        let mut seen = vec![false; n];
        let mut best: Option<(f32, f32, u32)> = None;
        let mut comp: Vec<(i32, i32)> = Vec::new();
        for start in 0..n {
            if !mask[start] || seen[start] {
                continue;
            }
            // flood fill one shape (8-connected)
            comp.clear();
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(i) = stack.pop() {
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                comp.push((x, y));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let j = (ny * w + nx) as usize;
                        if mask[j] && !seen[j] {
                            seen[j] = true;
                            stack.push(j);
                        }
                    }
                }
            }
            let count = comp.len() as u32;
            if count < min_px.max(12) {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for &(x, y) in &comp {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
            let (bw, bh) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
            if bw < 5.0 || bh < 5.0 {
                continue;
            }
            let aspect = bw / bh;
            let fill = count as f32 / (bw * bh);
            if !(0.7..=1.45).contains(&aspect) || !(0.3..=0.72).contains(&fill) {
                continue;
            }
            // mirror symmetry (a ★ is symmetric left to right)
            let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && mask[(y * w + x) as usize];
            let mirrored = comp.iter().filter(|&&(x, y)| inside(x0 + x1 - x, y)).count() as f32;
            if mirrored / (count as f32) < 0.75 {
                continue;
            }
            // a ★ has one narrow tip on top, in the middle (letters like the
            // yellow "v" of "Lv." or digits are open or wide at the top)
            let (mut tx0, mut tx1) = (i32::MAX, i32::MIN);
            for &(x, y) in &comp {
                if y == y0 {
                    tx0 = tx0.min(x);
                    tx1 = tx1.max(x);
                }
            }
            let tip_w = (tx1 - tx0 + 1) as f32;
            let tip_c = (tx0 + tx1) as f32 / 2.0;
            let cx = (x0 + x1) as f32 / 2.0;
            if tip_w > bw * 0.4 || (tip_c - cx).abs() > bw * 0.15 {
                continue;
            }
            // and its widest row (the arms) is in the upper part
            let mut rows = vec![0u32; bh as usize];
            for &(_, y) in &comp {
                rows[(y - y0) as usize] += 1;
            }
            let most = rows.iter().copied().max().unwrap_or(0);
            let widest = rows.iter().position(|n| *n == most).unwrap_or(0);
            if widest as f32 > bh * 0.6 {
                continue;
            }
            let cy = (y0 + y1) as f32 / 2.0;
            if best.map_or(true, |b| count > b.2) {
                best = Some((cx, cy, count));
            }
        }
        best
    }
}

/// Otsu's threshold for an 8-bit histogram.
pub fn otsu(px: &[u8]) -> u8 {
    let mut hist = [0u64; 256];
    for &p in px {
        hist[p as usize] += 1;
    }
    let total = px.len() as f64;
    let sum_all: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &h)| i as f64 * h as f64)
        .sum();
    let (mut wb, mut sb, mut best, mut th) = (0.0f64, 0.0f64, -1.0f64, 128u8);
    for t in 0..256 {
        wb += hist[t] as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sb += t as f64 * hist[t] as f64;
        let mb = sb / wb;
        let mf = (sum_all - sb) / wf;
        let between = wb * wf * (mb - mf) * (mb - mf);
        if between > best {
            best = between;
            th = t as u8;
        }
    }
    th
}

impl Gray {
    /// Binarise so text ends up black on white, whatever the game's colours.
    /// The text is assumed to be the minority class (it nearly always is).
    pub fn binarize_text_dark(&self) -> Gray {
        let th = otsu(&self.px);
        let bright = self.px.iter().filter(|&&p| p > th).count();
        let text_is_bright = bright * 2 < self.px.len();
        let px = self
            .px
            .iter()
            .map(|&p| {
                let is_text = if text_is_bright { p > th } else { p <= th };
                if is_text {
                    0
                } else {
                    255
                }
            })
            .collect();
        Gray {
            w: self.w,
            h: self.h,
            px,
        }
    }

    /// Nearest-neighbour integer upscale (keeps pixel fonts crisp).
    pub fn upscale(&self, k: u32) -> Gray {
        if k <= 1 {
            return self.clone();
        }
        let (w, h) = (self.w * k, self.h * k);
        let mut px = vec![0u8; (w * h) as usize];
        for y in 0..h {
            let sy = y / k;
            for x in 0..w {
                px[(y * w + x) as usize] = self.px[(sy * self.w + x / k) as usize];
            }
        }
        Gray { w, h, px }
    }

    /// Stack several images vertically on white with padding between them,
    /// so all name plates can be read with a single OCR call.
    pub fn stack(parts: &[Gray], pad: u32) -> Gray {
        let w = parts.iter().map(|g| g.w).max().unwrap_or(1) + pad * 2;
        let h = parts.iter().map(|g| g.h + pad).sum::<u32>() + pad;
        let mut px = vec![255u8; (w * h) as usize];
        let mut y0 = pad;
        for g in parts {
            for y in 0..g.h {
                let dst = ((y0 + y) * w + pad) as usize;
                let src = (y * g.w) as usize;
                px[dst..dst + g.w as usize].copy_from_slice(&g.px[src..src + g.w as usize]);
            }
            y0 += g.h + pad;
        }
        Gray { w, h, px }
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        self.px.iter().flat_map(|&v| [v, v, v, 255]).collect()
    }

    /// Cheap fingerprint used to skip OCR when nothing on screen changed.
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &self.px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^ ((self.w as u64) << 32 | self.h as u64)
    }
}

/// Crop each name region, binarise it, upscale small text, and stack it all
/// into one OCR-ready image.
pub fn prepare_names(frame: &Rgba, regions: &[Region]) -> Option<Gray> {
    let parts: Vec<Gray> = regions
        .iter()
        .filter_map(|r| frame.crop(*r))
        .map(|c| {
            let bin = c.luma().binarize_text_dark();
            // Aim for text roughly 40px+ tall: small regions (720p, windowed
            // games) get doubled or tripled.
            let k = if c.h < 30 {
                3
            } else if c.h < 60 {
                2
            } else {
                1
            };
            bin.upscale(k)
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(Gray::stack(&parts, 16))
}

#[cfg(test)]
mod star_tests {
    use super::Rgba;

    fn blank(w: u32, h: u32) -> Rgba {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p.copy_from_slice(&[40, 60, 90, 255]);
        }
        Rgba { w, h, px }
    }
    fn put(img: &mut Rgba, x: i32, y: i32) {
        if x >= 0 && y >= 0 && (x as u32) < img.w && (y as u32) < img.h {
            let i = ((y as u32 * img.w + x as u32) * 4) as usize;
            img.px[i..i + 4].copy_from_slice(&[250, 210, 40, 255]);
        }
    }
    /// A filled 5-point star centred at (cx, cy).
    fn star(img: &mut Rgba, cx: f32, cy: f32, r: f32) {
        let pts: Vec<(f32, f32)> = (0..10)
            .map(|k| {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                let rr = if k % 2 == 0 { r } else { r * 0.45 };
                (cx + a.cos() * rr, cy + a.sin() * rr)
            })
            .collect();
        for y in 0..img.h as i32 {
            for x in 0..img.w as i32 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let mut inside = false;
                let mut j = pts.len() - 1;
                for i in 0..pts.len() {
                    let (xi, yi) = pts[i];
                    let (xj, yj) = pts[j];
                    if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                        inside = !inside;
                    }
                    j = i;
                }
                if inside {
                    put(img, x, y);
                }
            }
        }
    }

    #[test]
    fn finds_a_star() {
        for r in [6.0f32, 9.0, 14.0] {
            let mut img = blank(120, 40);
            star(&mut img, 60.0, 20.0, r);
            assert!(img.find_star(12).is_some(), "star r={r}");
        }
    }

    #[test]
    fn ignores_blocks_bars_and_streaks() {
        let mut img = blank(120, 40);
        for y in 10..30 {
            for x in 10..30 {
                put(&mut img, x, y); // solid block
            }
            for x in 50..110 {
                if y < 14 {
                    put(&mut img, x, y); // wide bar
                }
            }
        }
        for k in 0..30 {
            put(&mut img, 40 + k, 5 + k); // diagonal streak
            put(&mut img, 41 + k, 5 + k);
        }
        assert!(img.find_star(12).is_none());
    }

    #[test]
    fn ignores_level_text_v() {
        // a chunky pixel "v", like the yellow "Lv." next to the level
        let mut img = blank(60, 40);
        for y in 0..15 {
            let off = y / 2;
            for t in 0..4 {
                put(&mut img, 10 + off + t, 10 + y);
                put(&mut img, 40 - off - t, 10 + y);
            }
        }
        assert!(img.find_star(12).is_none());
    }

    #[test]
    fn ignores_scattered_sparkles() {
        let mut img = blank(120, 40);
        for (x, y) in [(5, 5), (30, 12), (70, 30), (100, 8), (55, 22)] {
            for d in -1..=1 {
                put(&mut img, x + d, y);
                put(&mut img, x, y + d);
            }
        }
        assert!(img.find_star(12).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otsu_splits_two_levels() {
        let mut px = vec![20u8; 900];
        px.extend(vec![230u8; 100]);
        let t = otsu(&px);
        assert!(t >= 20 && t < 230);
    }

    #[test]
    fn bright_text_on_dark_becomes_black_on_white() {
        // 10x10 dark image with a bright 2x2 "glyph"
        let mut g = Gray {
            w: 10,
            h: 10,
            px: vec![30; 100],
        };
        for (x, y) in [(4, 4), (5, 4), (4, 5), (5, 5)] {
            g.px[y * 10 + x] = 250;
        }
        let b = g.binarize_text_dark();
        assert_eq!(b.px[44], 0);
        assert_eq!(b.px[0], 255);
    }

    #[test]
    fn crop_and_stack_shapes() {
        let img = Rgba {
            w: 8,
            h: 8,
            px: vec![255; 8 * 8 * 4],
        };
        let c = img.crop(Region {
            x: 6,
            y: 6,
            w: 10,
            h: 10,
        });
        assert!(c.is_none(), "2x2 after clamping is too small");
        let c = img
            .crop(Region {
                x: 0,
                y: 0,
                w: 5,
                h: 4,
            })
            .unwrap();
        assert_eq!((c.w, c.h, c.px.len()), (5, 4, 80));
        let s = Gray::stack(&[c.luma(), c.luma().upscale(2)], 2);
        assert_eq!((s.w, s.h), (14, 2 + 4 + 2 + 8 + 2));
    }
}
