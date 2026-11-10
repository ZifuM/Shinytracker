//! A tiny software canvas for drawing the share images: anti-aliased rounded
//! rectangles, circles, gradients, sparkles and pixel-art sprite blits.
//! No dependencies, so it can be tested and previewed on its own.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Col(pub u8, pub u8, pub u8, pub u8);

impl Col {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Col(r, g, b, 255)
    }
    pub fn hex(v: u32) -> Self {
        Col((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
    }
    pub fn alpha(self, a: f32) -> Self {
        Col(self.0, self.1, self.2, (self.3 as f32 * a.clamp(0.0, 1.0)).round() as u8)
    }
    pub fn mix(self, o: Col, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Col(l(self.0, o.0), l(self.1, o.1), l(self.2, o.2), l(self.3, o.3))
    }
}

/// Straight-alpha image with any size.
#[derive(Clone)]
pub struct Img {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

#[derive(Clone)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    /// RGBA, straight alpha.
    pub px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32, bg: Col) -> Self {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p.copy_from_slice(&[bg.0, bg.1, bg.2, bg.3]);
        }
        Self { w, h, px }
    }

    /// Paint `c` over pixel (x, y) with extra coverage `cov` (0..1).
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Col, cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let a = (c.3 as f32 / 255.0) * cov.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        let d = &mut self.px[i..i + 4];
        let da = d[3] as f32 / 255.0;
        let oa = a + da * (1.0 - a);
        if oa <= 0.0 {
            return;
        }
        for k in 0..3 {
            let s = [c.0, c.1, c.2][k] as f32;
            d[k] = ((s * a + d[k] as f32 * da * (1.0 - a)) / oa).round().clamp(0.0, 255.0) as u8;
        }
        d[3] = (oa * 255.0).round() as u8;
    }

    /// Fill a whole area by asking `f` for the colour of each pixel.
    pub fn fill_with(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32) -> Option<(Col, f32)>) {
        let (x0, y0) = (x0.max(0), y0.max(0));
        let (x1, y1) = (x1.min(self.w as i32), y1.min(self.h as i32));
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some((c, cov)) = f(x, y) {
                    self.blend(x, y, c, cov);
                }
            }
        }
    }

    /// Background gradient from `a` (top-left) to `b` (bottom-right).
    pub fn diagonal_gradient(&mut self, a: Col, b: Col) {
        let (w, h) = (self.w as f32, self.h as f32);
        let len = w * w + h * h;
        self.fill_with(0, 0, self.w as i32, self.h as i32, |x, y| {
            let t = (x as f32 * w + y as f32 * h) / len;
            Some((a.mix(b, t), 1.0))
        });
    }

    /// Rounded rectangle, filled by `paint(x, y)` (lets panels have gradients).
    pub fn rrect_with(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, paint: impl Fn(i32, i32) -> Col) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        self.fill_with(x.floor() as i32, y.floor() as i32, (x + w).ceil() as i32, (y + h).ceil() as i32, |px, py| {
            let cov = rrect_cov(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r);
            (cov > 0.0).then(|| (paint(px, py), cov))
        });
    }

    pub fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        self.rrect_with(x, y, w, h, r, |_, _| c);
    }

    /// Vertical gradient inside a rounded rectangle.
    pub fn rrect_vgrad(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, top: Col, bottom: Col) {
        self.rrect_with(x, y, w, h, r, |_, py| top.mix(bottom, (py as f32 - y) / h.max(1.0)));
    }

    /// Outline of a rounded rectangle, `t` pixels thick (drawn inside the box).
    pub fn rrect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, c: Col) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        let ri = (r - t).max(0.0);
        self.fill_with(x.floor() as i32, y.floor() as i32, (x + w).ceil() as i32, (y + h).ceil() as i32, |px, py| {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let outer = rrect_cov(fx, fy, x, y, w, h, r);
            let inner = rrect_cov(fx, fy, x + t, y + t, w - 2.0 * t, h - 2.0 * t, ri);
            let cov = (outer - inner).max(0.0);
            (cov > 0.0).then_some((c, cov))
        });
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Col) {
        self.fill_with((cx - rx).floor() as i32, (cy - ry).floor() as i32, (cx + rx).ceil() as i32 + 1, (cy + ry).ceil() as i32 + 1, |px, py| {
            let dx = (px as f32 + 0.5 - cx) / rx;
            let dy = (py as f32 + 0.5 - cy) / ry;
            let d = (dx * dx + dy * dy).sqrt();
            // distance from the edge in pixels (approximate for ellipses)
            let edge = (1.0 - d) * rx.min(ry);
            let cov = (edge + 0.5).clamp(0.0, 1.0);
            (cov > 0.0).then_some((c, cov))
        });
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        self.ellipse(cx, cy, r, r, c);
    }

    /// Circle outline (bubble), anti-aliased.
    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, c: Col) {
        let e = r + t;
        self.fill_with((cx - e) as i32 - 1, (cy - e) as i32 - 1, (cx + e) as i32 + 2, (cy + e) as i32 + 2, |px, py| {
            let d = ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
            let cov = (t / 2.0 + 0.5 - (d - r).abs()).clamp(0.0, 1.0);
            (cov > 0.0).then_some((c, cov))
        });
    }

    /// Thick anti-aliased line with round ends.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: Col) {
        let h = w / 2.0 + 1.0;
        let (minx, maxx) = (x0.min(x1) - h, x0.max(x1) + h);
        let (miny, maxy) = (y0.min(y1) - h, y0.max(y1) + h);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        self.fill_with(minx as i32, miny as i32, maxx as i32 + 1, maxy as i32 + 1, |px, py| {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let t = (((fx - x0) * dx + (fy - y0) * dy) / len2).clamp(0.0, 1.0);
            let d = ((fx - x0 - t * dx).powi(2) + (fy - y0 - t * dy).powi(2)).sqrt();
            let cov = (w / 2.0 + 0.5 - d).clamp(0.0, 1.0);
            (cov > 0.0).then_some((c, cov))
        });
    }

    /// Filled convex polygon (points in order), anti-aliased.
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Col) {
        if pts.len() < 3 {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        // orientation, so "inside" is the same side of every edge
        let mut area = 0.0;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            area += a.0 * b.1 - b.0 * a.1;
        }
        let sign = if area >= 0.0 { 1.0 } else { -1.0 };
        let edges: Vec<(f32, f32, f32)> = (0..pts.len())
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = (dx * dx + dy * dy).sqrt().max(1e-6);
                // outward normal
                let (nx, ny) = (sign * dy / len, -sign * dx / len);
                (nx, ny, -(nx * a.0 + ny * a.1))
            })
            .collect();
        self.fill_with(x0 as i32 - 1, y0 as i32 - 1, x1 as i32 + 2, y1 as i32 + 2, |px, py| {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let d = edges.iter().map(|(nx, ny, c0)| nx * fx + ny * fy + c0).fold(f32::MIN, f32::max);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            (cov > 0.0).then_some((c, cov))
        });
    }

    /// Soft round glow fading out to nothing.
    pub fn glow(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        self.fill_with((cx - r) as i32, (cy - r) as i32, (cx + r) as i32 + 1, (cy + r) as i32 + 1, |px, py| {
            let d = ((px as f32 - cx).powi(2) + (py as f32 - cy).powi(2)).sqrt() / r;
            (d < 1.0).then(|| (c, (1.0 - d).powf(1.8)))
        });
    }

    /// Four-pointed sparkle (like the shiny star burst).
    pub fn sparkle(&mut self, cx: f32, cy: f32, size: f32, c: Col) {
        const N: i32 = 3; // 3x3 supersampling
        self.fill_with((cx - size) as i32 - 1, (cy - size) as i32 - 1, (cx + size) as i32 + 2, (cy + size) as i32 + 2, |px, py| {
            let mut hit = 0;
            for sy in 0..N {
                for sx in 0..N {
                    let dx = ((px as f32 + (sx as f32 + 0.5) / N as f32) - cx).abs() / size;
                    let dy = ((py as f32 + (sy as f32 + 0.5) / N as f32) - cy).abs() / size;
                    if dx.sqrt() + dy.sqrt() <= 1.0 {
                        hit += 1;
                    }
                }
            }
            (hit > 0).then(|| (c, hit as f32 / (N * N) as f32))
        });
    }

    /// Pixel art scaled by any factor with nearest-neighbour sampling (crisp
    /// edges), top-left at (x, y).
    pub fn blit_nearest(&mut self, img: &Img, x: f32, y: f32, scale: f32) {
        if img.w == 0 || img.h == 0 || scale <= 0.0 {
            return;
        }
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let dw = (img.w as f32 * scale).round() as i32;
        let dh = (img.h as f32 * scale).round() as i32;
        for dy in 0..dh {
            let sy = (((dy as f32 + 0.5) / scale) as u32).min(img.h - 1);
            for dx in 0..dw {
                let sx = (((dx as f32 + 0.5) / scale) as u32).min(img.w - 1);
                let i = ((sy * img.w + sx) * 4) as usize;
                let p = &img.px[i..i + 4];
                if p[3] == 0 {
                    continue;
                }
                self.blend(x0 + dx, y0 + dy, Col(p[0], p[1], p[2], p[3]), 1.0);
            }
        }
    }

    /// Smooth (bilinear) scaled draw for non-pixel-art images like the icon.
    pub fn blit_smooth(&mut self, img: &Img, x: f32, y: f32, w: f32, h: f32) {
        if img.w == 0 || img.h == 0 {
            return;
        }
        let get = |sx: i32, sy: i32| -> [f32; 4] {
            let sx = sx.clamp(0, img.w as i32 - 1) as u32;
            let sy = sy.clamp(0, img.h as i32 - 1) as u32;
            let i = ((sy * img.w + sx) * 4) as usize;
            let a = img.px[i + 3] as f32 / 255.0;
            // premultiply for correct filtering
            [img.px[i] as f32 * a, img.px[i + 1] as f32 * a, img.px[i + 2] as f32 * a, a]
        };
        self.fill_with(x as i32, y as i32, (x + w).ceil() as i32, (y + h).ceil() as i32, |px, py| {
            let u = (px as f32 + 0.5 - x) / w * img.w as f32 - 0.5;
            let v = (py as f32 + 0.5 - y) / h * img.h as f32 - 0.5;
            let (x0, y0) = (u.floor() as i32, v.floor() as i32);
            let (fx, fy) = (u - x0 as f32, v - y0 as f32);
            let mut acc = [0f32; 4];
            for (dx, dy, wgt) in [(0, 0, (1.0 - fx) * (1.0 - fy)), (1, 0, fx * (1.0 - fy)), (0, 1, (1.0 - fx) * fy), (1, 1, fx * fy)] {
                let s = get(x0 + dx, y0 + dy);
                for k in 0..4 {
                    acc[k] += s[k] * wgt;
                }
            }
            if acc[3] <= 0.001 {
                return None;
            }
            let c = Col((acc[0] / acc[3]) as u8, (acc[1] / acc[3]) as u8, (acc[2] / acc[3]) as u8, 255);
            Some((c, acc[3]))
        });
    }

    /// Bounding box of non-transparent pixels in an image (sprites have
    /// empty margins), as (x, y, w, h).
    pub fn trim_box(img: &Img) -> (u32, u32, u32, u32) {
        let (mut x0, mut y0, mut x1, mut y1) = (img.w, img.h, 0, 0);
        for y in 0..img.h {
            for x in 0..img.w {
                if img.px[((y * img.w + x) * 4 + 3) as usize] > 8 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
        }
        if x1 <= x0 {
            (0, 0, img.w, img.h)
        } else {
            (x0, y0, x1 - x0, y1 - y0)
        }
    }
}

/// Crop helper.
pub fn crop(img: &Img, (x, y, w, h): (u32, u32, u32, u32)) -> Img {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let s = ((row * img.w + x) * 4) as usize;
        px.extend_from_slice(&img.px[s..s + (w * 4) as usize]);
    }
    Img { w, h, px }
}

/// Coverage of a pixel centre (fx, fy) by a rounded rectangle.
fn rrect_cov(fx: f32, fy: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
    // signed distance to a rounded box
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let qx = (fx - cx).abs() - (w / 2.0 - r);
    let qy = (fy - cy).abs() - (h / 2.0 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    let d = outside + inside - r;
    (0.5 - d).clamp(0.0, 1.0)
}

/// Small deterministic random numbers (for star fields).
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}
