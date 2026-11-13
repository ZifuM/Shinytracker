//! The battle pass name plates and picture frames, drawn on a Canvas (in
//! pixels). The share images draw them straight onto the card; the app
//! draws them into small pictures and shows those.
//!
//! Two looks: Distortion World (void violet, crimson rifts, Griseous gold)
//! and Snowpoint Aurora (night navy, teal-green-violet lights, ice white).

use crate::canvas::{Canvas, Col};
use std::f32::consts::{PI, TAU};

// Distortion World
const VOID: Col = Col(18, 8, 32, 255);
const VIOLET: Col = Col(88, 40, 140, 255);
const DEEP: Col = Col(44, 18, 74, 255);
const CRIMSON: Col = Col(214, 38, 70, 255);
const GOLD: Col = Col(232, 182, 76, 255);
const GOLD_LO: Col = Col(150, 104, 36, 255);
const RIFT_GREEN: Col = Col(80, 170, 140, 255);
// Snowpoint Aurora
const NIGHT: Col = Col(10, 24, 54, 255);
const NAVY: Col = Col(20, 44, 92, 255);
const TEAL: Col = Col(46, 214, 196, 255);
const GREEN: Col = Col(110, 240, 160, 255);
const LILAC: Col = Col(156, 124, 255, 255);
const ICE: Col = Col(232, 246, 255, 255);
const FROST: Col = Col(150, 210, 240, 255);
const WHITE: Col = Col(255, 255, 255, 255);

/// Every name plate: (id, name).
pub const PLATES: [&str; 9] = [
    "plate_rift",
    "plate_snowpoint",
    "plate_reverse",
    "plate_northern_lights",
    "plate_tendril",
    "plate_polar_night",
    "plate_griseous_crown",
    "plate_crystal_veil",
    "plate_twin_worlds",
];

/// How far in from a plate's end the name starts, as a share of the plate's height
/// (room for the decoration at each end).
pub const PLATE_PAD: f32 = 0.55;

pub const FRAMES: [&str; 9] = [
    "frame_aurora_ring",
    "frame_shadow_ring",
    "frame_frost_crystal",
    "frame_griseous",
    "frame_diamond_dust",
    "frame_rift_eye",
    "frame_borealis",
    "frame_eclipse",
    "frame_origin_halo",
];

/// The colour text should be on a plate (they're all dark in the middle).
pub fn plate_text(id: &str) -> Col {
    match id {
        "plate_griseous_crown" => Col(255, 236, 190, 255),
        "plate_snowpoint" | "plate_crystal_veil" => Col(250, 253, 255, 255),
        _ => WHITE,
    }
}

/// The main colour of a plate or frame (for borders next to it).
pub fn accent(id: &str) -> Col {
    match id {
        "plate_rift" | "frame_rift_eye" | "plate_tendril" => CRIMSON,
        "plate_reverse" => RIFT_GREEN,
        "plate_griseous_crown" | "frame_griseous" | "frame_eclipse" => GOLD,
        "frame_shadow_ring" => VIOLET,
        "plate_snowpoint" | "frame_frost_crystal" | "plate_crystal_veil" => FROST,
        "plate_northern_lights" | "frame_aurora_ring" | "frame_borealis" => TEAL,
        "plate_polar_night" | "frame_diamond_dust" => ICE,
        _ => LILAC,
    }
}

/// Small repeatable random numbers.
struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % 10_000) as f32 / 10_000.0
    }
}

fn hgrad(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, r: f32, a: Col, b: Col) {
    let x0 = x;
    c.rrect_with(x, y, w, h, r, move |px, _| a.mix(b, ((px as f32 - x0) / w).clamp(0.0, 1.0)));
}

/// A thick line made of a quad (any angle), so polygons stay convex.
fn bar(c: &mut Canvas, (x0, y0): (f32, f32), (x1, y1): (f32, f32), w: f32, col: Col) {
    c.line(x0, y0, x1, y1, w, col);
}

pub fn snowflake(c: &mut Canvas, cx: f32, cy: f32, r: f32, col: Col) {
    let t = (r * 0.16).max(0.8);
    for k in 0..3 {
        let a = k as f32 * PI / 3.0;
        let (dx, dy) = (a.cos() * r, a.sin() * r);
        bar(c, (cx - dx, cy - dy), (cx + dx, cy + dy), t, col);
    }
    c.circle(cx, cy, t * 1.2, col);
}

fn star(c: &mut Canvas, cx: f32, cy: f32, r: f32, col: Col) {
    c.sparkle(cx, cy, r, col);
}

/// A ring from angle `a0` to `a1` (radians, 0 = right, clockwise on screen),
/// coloured along its length by `col(t)`.
pub fn arc(c: &mut Canvas, cx: f32, cy: f32, r: f32, t: f32, a0: f32, a1: f32, col: impl Fn(f32) -> Col) {
    let steps = (((a1 - a0).abs() * r) / 3.0).ceil().max(6.0) as usize;
    let (ri, ro) = (r - t / 2.0, r + t / 2.0);
    for i in 0..steps {
        let f0 = i as f32 / steps as f32;
        let f1 = (i as f32 + 1.06) / steps as f32;
        let (b0, b1) = (a0 + (a1 - a0) * f0, a0 + (a1 - a0) * f1.min(1.0));
        let p = |a: f32, rr: f32| (cx + a.cos() * rr, cy + a.sin() * rr);
        c.poly(&[p(b0, ri), p(b0, ro), p(b1, ro), p(b1, ri)], col((f0 + f1) / 2.0));
    }
}

/// A wavy band across `w` (aurora curtain): vertical strokes whose top and
/// bottom follow sine waves.
#[allow(clippy::too_many_arguments)]
pub fn curtain(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, phase: f32, amp: f32, a: Col, b: Col, alpha: f32) {
    let n = (w / 1.5).ceil() as usize;
    for i in 0..n {
        let f = i as f32 / n as f32;
        let px = x + f * w;
        let wave = (f * TAU * 1.4 + phase).sin() * amp + (f * TAU * 3.1 + phase * 1.7).sin() * amp * 0.35;
        let top = y + wave;
        let col = a.mix(b, f).alpha(alpha * (0.55 + 0.45 * (f * TAU * 2.0 + phase).sin().abs()));
        c.rrect_vgrad(px, top, 1.8, h, 0.0, col, col.alpha(0.0));
    }
}

// ───────────────────────────── name plates ─────────────────────────────

/// Draw name plate `id` filling (x, y, w, h). Text goes on top, centre-left.
pub fn draw_plate(c: &mut Canvas, id: &str, x: f32, y: f32, w: f32, h: f32) {
    let r = h * 0.28;
    let k = h / 48.0;
    let mut rng = Rng(0xC0FFEE ^ id.len() as u64 * 7919);
    // a soft shadow under every plate
    c.rrect(x + 2.0 * k, y + 3.0 * k, w, h, r, Col(0, 0, 0, 90));
    match id {
        "plate_rift" => {
            hgrad(c, x, y, w, h, r, DEEP, VOID);
            c.glow(x + w * 0.08, y + h * 0.5, h * 1.1, CRIMSON.alpha(0.45));
            c.glow(x + w * 0.95, y + h * 0.5, h * 0.9, VIOLET.alpha(0.6));
            // crimson rift cracks: zigzags at both ends
            // (kept narrow so they frame the name rather than cross it)
            for (sx, dir) in [(x + h * 0.14, 1.0f32), (x + w - h * 0.14, -1.0)] {
                let mut p = (sx, y + 2.0 * k);
                for i in 0..5 {
                    let nx = sx + dir * (if i % 2 == 0 { h * 0.17 } else { 0.0 }) + dir * i as f32 * h * 0.02;
                    let np = (nx, y + (i as f32 + 1.0) * h / 5.0);
                    bar(c, p, np, 2.6 * k, CRIMSON);
                    bar(c, p, np, 1.0 * k, Col(255, 170, 170, 255));
                    p = np;
                }
            }
            c.rrect_stroke(x, y, w, h, r, 2.0 * k, GOLD.alpha(0.85));
        }
        "plate_reverse" => {
            hgrad(c, x, y, w, h, r, Col(30, 44, 52, 255), DEEP);
            // the mirrored horizon
            c.rrect(x + h * 0.4, y + h * 0.5 - 0.6 * k, w - h * 0.8, 1.2 * k, 0.0, RIFT_GREEN.alpha(0.35));
            // floating upside-down islands along the top and bottom
            for i in 0..((w / (h * 0.9)) as usize).max(3) {
                let fx = x + h * 0.4 + rng.f() * (w - h * 0.8);
                let fw = h * (0.25 + rng.f() * 0.3);
                let top = rng.f() > 0.5;
                let (yy, d) = if top { (y + 3.0 * k, 1.0) } else { (y + h - 3.0 * k, -1.0) };
                let th = h * (0.12 + rng.f() * 0.1);
                c.poly(&[(fx, yy), (fx + fw, yy), (fx + fw * 0.7, yy + d * th), (fx + fw * 0.3, yy + d * th)], if i % 3 == 0 { RIFT_GREEN.alpha(0.8) } else { Col(70, 60, 110, 230) });
            }
            c.rrect_stroke(x, y, w, h, r, 1.8 * k, RIFT_GREEN.alpha(0.8));
        }
        "plate_tendril" => {
            hgrad(c, x, y, w, h, r, Col(8, 4, 14, 255), Col(36, 12, 50, 255));
            // shadow tendrils creeping in from both ends
            for (side, base) in [(0.0f32, x), (1.0, x + w)] {
                for t in 0..3 {
                    let yy = y + h * (0.25 + t as f32 * 0.25);
                    let len = h * (1.1 + t as f32 * 0.25);
                    let n = 16;
                    for i in 0..n {
                        let f = i as f32 / n as f32;
                        let dx = (h * 0.12 + f * len) * if side == 0.0 { 1.0 } else { -1.0 };
                        let wy = (f * TAU * 1.2 + t as f32).sin() * h * 0.12;
                        c.circle(base + dx, yy + wy, h * 0.13 * (1.0 - f * 0.8), Col(4, 0, 8, 235));
                    }
                }
                // Giratina's red stripes
                let sx = if side == 0.0 { x + h * 0.24 } else { x + w - h * 0.24 };
                for j in 0..3 {
                    let yy = y + h * (0.3 + j as f32 * 0.2);
                    c.rrect(sx - h * 0.13, yy - 1.4 * k, h * 0.26, 2.8 * k, 1.4 * k, CRIMSON.alpha(0.9));
                }
            }
            c.rrect_stroke(x, y, w, h, r, 1.6 * k, CRIMSON.alpha(0.55));
        }
        "plate_griseous_crown" => {
            // crown points along the top, behind the plate
            let n = ((w / (h * 0.55)) as usize).max(4);
            for i in 0..n {
                let fx = x + h * 0.6 + (w - h * 1.2) * (i as f32 + 0.5) / n as f32;
                let tall = if i % 2 == 0 { h * 0.32 } else { h * 0.2 };
                c.poly(&[(fx - h * 0.12, y + 4.0 * k), (fx, y - tall), (fx + h * 0.12, y + 4.0 * k)], GOLD);
                c.circle(fx, y - tall, 2.0 * k, Col(255, 240, 200, 255));
            }
            hgrad(c, x, y, w, h, r, Col(40, 22, 10, 255), Col(20, 12, 24, 255));
            c.rrect_stroke(x, y, w, h, r, 3.0 * k, GOLD);
            c.rrect_stroke(x + 4.0 * k, y + 4.0 * k, w - 8.0 * k, h - 8.0 * k, (r - 4.0 * k).max(1.0), 1.0 * k, GOLD_LO);
            // the Griseous Orb at the right end
            let (ox, oy, or) = (x + w - h * 0.33, y + h * 0.5, h * 0.22);
            c.glow(ox, oy, or * 2.2, GOLD.alpha(0.5));
            c.circle(ox, oy, or, GOLD);
            c.circle(ox, oy, or * 0.7, Col(110, 120, 140, 255));
            c.circle(ox - or * 0.25, oy - or * 0.25, or * 0.22, Col(220, 225, 235, 255));
        }
        "plate_snowpoint" => {
            hgrad(c, x, y, w, h, r, Col(56, 120, 186, 255), Col(30, 72, 140, 255));
            for _ in 0..((w / (h * 0.4)) as usize) {
                let fx = x + rng.f() * w;
                let fy = y + h * 0.3 + rng.f() * h * 0.65;
                snowflake(c, fx, fy, h * (0.06 + rng.f() * 0.08), ICE.alpha(0.35 + rng.f() * 0.4));
            }
            // snow resting on the top edge
            let n = (w / (h * 0.32)) as usize;
            for i in 0..n {
                let fx = x + r * 0.6 + (w - r * 1.2) * i as f32 / n.max(1) as f32;
                c.circle(fx, y + 2.0 * k, h * (0.1 + rng.f() * 0.07), ICE);
            }
            c.rrect_stroke(x, y, w, h, r, 2.0 * k, ICE.alpha(0.9));
        }
        "plate_northern_lights" => {
            hgrad(c, x, y, w, h, r, NIGHT, Col(16, 30, 70, 255));
            for _ in 0..((w / 9.0) as usize) {
                c.circle(x + rng.f() * w, y + rng.f() * h, 0.7 * k, WHITE.alpha(0.3 + rng.f() * 0.5));
            }
            curtain(c, x + r * 0.3, y + h * 0.12, w - r * 0.6, h * 0.55, 0.3, h * 0.12, GREEN, TEAL, 0.65);
            curtain(c, x + r * 0.3, y + h * 0.32, w - r * 0.6, h * 0.45, 2.1, h * 0.1, TEAL, LILAC, 0.45);
            c.rrect_stroke(x, y, w, h, r, 1.6 * k, TEAL.alpha(0.8));
        }
        "plate_polar_night" => {
            hgrad(c, x, y, w, h, r, Col(6, 12, 30, 255), Col(14, 26, 58, 255));
            c.rrect_vgrad(x + 2.0, y + h * 0.6, w - 4.0, h * 0.38, 0.0, TEAL.alpha(0.0), TEAL.alpha(0.18));
            for _ in 0..((w / 7.0) as usize) {
                c.circle(x + rng.f() * w, y + rng.f() * h * 0.8, (0.5 + rng.f() * 0.6) * k, WHITE.alpha(0.25 + rng.f() * 0.6));
            }
            for _ in 0..3 {
                star(c, x + h * 0.4 + rng.f() * (w - h * 1.6), y + h * (0.2 + rng.f() * 0.4), h * 0.09, ICE);
            }
            // the moon
            let (mx, my, mr) = (x + w - h * 0.38, y + h * 0.48, h * 0.21);
            c.glow(mx, my, mr * 2.4, ICE.alpha(0.35));
            c.circle(mx, my, mr, ICE);
            c.circle(mx + mr * 0.42, my - mr * 0.2, mr * 0.88, Col(10, 20, 46, 255));
            c.rrect_stroke(x, y, w, h, r, 1.4 * k, FROST.alpha(0.6));
        }
        "plate_crystal_veil" => {
            hgrad(c, x, y, w, h, r, Col(70, 140, 200, 255), Col(110, 90, 200, 255));
            // faceted crystal: triangles in a strip
            let n = ((w / (h * 0.5)) as usize).max(4);
            let step = w / n as f32;
            for i in 0..n {
                let fx = x + i as f32 * step;
                let a = 0.06 + rng.f() * 0.16;
                c.poly(&[(fx, y), (fx + step, y), (fx + step * 0.5, y + h * 0.55)], ICE.alpha(a));
                c.poly(&[(fx + step * 0.5, y + h * 0.55), (fx + step * 1.5, y + h * 0.55), (fx + step, y + h)], ICE.alpha(a * 0.8));
            }
            c.rrect_vgrad(x, y, w, h * 0.45, r, WHITE.alpha(0.25), WHITE.alpha(0.0));
            for _ in 0..4 {
                star(c, x + rng.f() * w, y + rng.f() * h, h * 0.08, WHITE);
            }
            c.rrect_stroke(x, y, w, h, r, 2.0 * k, WHITE.alpha(0.9));
        }
        _ => {
            // Twin Worlds: Distortion on the left, Aurora on the right
            let mid = x + w * 0.5;
            hgrad(c, x, y, w, h, r, DEEP, NAVY);
            c.glow(x + h * 0.3, y + h * 0.5, h * 1.2, CRIMSON.alpha(0.4));
            curtain(c, mid + h * 0.2, y + h * 0.15, w * 0.5 - r, h * 0.55, 1.0, h * 0.1, GREEN, LILAC, 0.5);
            let mut p = (x + h * 0.3, y + 2.0 * k);
            for i in 0..5 {
                let np = (x + h * 0.3 + if i % 2 == 0 { h * 0.25 } else { 0.0 }, y + (i as f32 + 1.0) * h / 5.0);
                bar(c, p, np, 2.2 * k, CRIMSON);
                p = np;
            }
            snowflake(c, x + w - h * 0.3, y + h * 0.5, h * 0.14, ICE);
            // the seam between the worlds
            c.poly(&[(mid - 1.5 * k, y + 1.0), (mid + 1.5 * k + h * 0.15, y + 1.0), (mid + 1.5 * k, y + h - 1.0), (mid - 1.5 * k - h * 0.15, y + h - 1.0)], GOLD.alpha(0.6));
            c.rrect_stroke(x, y, w, h, r, 2.0 * k, GOLD.alpha(0.8));
        }
    }
}

// ───────────────────────────── picture frames ─────────────────────────────

/// How far a frame reaches past the picture, as a share of its radius.
pub const FRAME_REACH: f32 = 0.42;

/// Draw frame `id` around a round picture of radius `r` at (cx, cy).
pub fn draw_frame(c: &mut Canvas, id: &str, cx: f32, cy: f32, r: f32) {
    let t = r * 0.15;
    let ring = r + t * 0.45;
    let mut rng = Rng(0xF2A3E ^ id.len() as u64 * 104_729);
    match id {
        "frame_aurora_ring" => {
            // a soft halo outside the ring only (a full glow would tint the photo)
            for i in 0..10 {
                let f = i as f32 / 10.0;
                c.ring(cx, cy, ring + t * 0.5 + f * r * 0.36, r * 0.04, TEAL.alpha(0.2 * (1.0 - f) * (1.0 - f)));
            }
            arc(c, cx, cy, ring, t, 0.0, TAU, |f| {
                let k = (f * TAU).sin() * 0.5 + 0.5;
                GREEN.mix(LILAC, k).mix(TEAL, ((f * TAU * 2.0).cos() * 0.5 + 0.5) * 0.5)
            });
            c.ring(cx, cy, ring + t * 0.5, 1.2, WHITE.alpha(0.6));
        }
        "frame_shadow_ring" => {
            // spikes of shadow
            for i in 0..10 {
                let a = i as f32 * TAU / 10.0 - PI / 2.0;
                let len = r * if i % 2 == 0 { 0.4 } else { 0.26 };
                let (s, w) = (ring, 0.17);
                c.poly(&[(cx + (a - w).cos() * s, cy + (a - w).sin() * s), (cx + a.cos() * (s + len), cy + a.sin() * (s + len)), (cx + (a + w).cos() * s, cy + (a + w).sin() * s)], VOID);
            }
            c.ring(cx, cy, ring, t * 1.2, DEEP);
            c.ring(cx, cy, r + 1.0, t * 0.25, CRIMSON);
            c.ring(cx, cy, ring + t * 0.55, 1.5, VIOLET);
        }
        "frame_frost_crystal" => {
            for i in 0..6 {
                let a = i as f32 * TAU / 6.0 - PI / 2.0;
                let (p0, p1) = (ring - t * 0.3, ring + r * 0.4);
                let w = r * 0.09;
                let (nx, ny) = (-a.sin() * w, a.cos() * w);
                let mid = (p0 + p1) * 0.45;
                let (mx, my) = (cx + a.cos() * mid, cy + a.sin() * mid);
                c.poly(&[(cx + a.cos() * p0, cy + a.sin() * p0), (mx + nx, my + ny), (cx + a.cos() * p1, cy + a.sin() * p1), (mx - nx, my - ny)], FROST);
                c.poly(&[(cx + a.cos() * p0, cy + a.sin() * p0), (mx + nx * 0.4, my + ny * 0.4), (cx + a.cos() * p1, cy + a.sin() * p1)], WHITE.alpha(0.8));
            }
            c.ring(cx, cy, ring, t, Col(120, 190, 235, 255));
            c.ring(cx, cy, ring - t * 0.3, t * 0.25, WHITE.alpha(0.9));
        }
        "frame_griseous" => {
            c.ring(cx, cy, ring, t * 1.1, GOLD);
            c.ring(cx, cy, ring + t * 0.55, 1.4, GOLD_LO);
            c.ring(cx, cy, ring - t * 0.55, 1.4, GOLD_LO);
            // notches
            for i in 0..24 {
                let a = i as f32 * TAU / 24.0;
                bar(c, (cx + a.cos() * (ring - t * 0.4), cy + a.sin() * (ring - t * 0.4)), (cx + a.cos() * (ring + t * 0.4), cy + a.sin() * (ring + t * 0.4)), 1.2, GOLD_LO);
            }
            // three Griseous orbs
            for i in 0..3 {
                let a = i as f32 * TAU / 3.0 - PI / 2.0;
                let (ox, oy, or) = (cx + a.cos() * (ring + t * 0.2), cy + a.sin() * (ring + t * 0.2), r * 0.17);
                c.glow(ox, oy, or * 2.2, GOLD.alpha(0.5));
                c.circle(ox, oy, or, GOLD);
                c.circle(ox, oy, or * 0.68, Col(110, 120, 140, 255));
                c.circle(ox - or * 0.25, oy - or * 0.25, or * 0.22, Col(225, 230, 240, 255));
            }
        }
        "frame_diamond_dust" => {
            c.ring(cx, cy, ring, t * 0.45, ICE);
            c.ring(cx, cy, ring + t * 0.5, 1.0, FROST.alpha(0.8));
            for _ in 0..22 {
                let a = rng.f() * TAU;
                let d = ring + (rng.f() - 0.3) * r * 0.35;
                let s = r * (0.04 + rng.f() * 0.07);
                star(c, cx + a.cos() * d, cy + a.sin() * d, s, if rng.f() > 0.4 { WHITE } else { FROST });
            }
        }
        "frame_rift_eye" => {
            // short cracks around the ring
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0 + 0.2;
                let mut p = (cx + a.cos() * ring, cy + a.sin() * ring);
                for j in 1..4 {
                    let aa = a + if j % 2 == 0 { 0.08 } else { -0.08 };
                    let d = ring + j as f32 * r * 0.1;
                    let np = (cx + aa.cos() * d, cy + aa.sin() * d);
                    bar(c, p, np, 2.0, CRIMSON);
                    p = np;
                }
            }
            c.ring(cx, cy, ring, t, VIOLET);
            c.ring(cx, cy, ring, t * 0.3, CRIMSON.alpha(0.8));
            // the eye at the top
            let (ex, ey) = (cx, cy - ring);
            c.glow(ex, ey, r * 0.4, CRIMSON.alpha(0.55));
            c.ellipse(ex, ey, r * 0.28, r * 0.14, Col(250, 220, 120, 255));
            c.ellipse(ex, ey, r * 0.06, r * 0.13, Col(30, 0, 10, 255));
            c.ring(ex, ey, r * 0.2, 1.5, CRIMSON);
        }
        "frame_borealis" => {
            // curtains of light rising from the ring
            for i in 0..48 {
                let a = i as f32 * TAU / 48.0;
                let k = (a * 3.0).sin() * 0.5 + 0.5;
                let len = r * (0.18 + 0.22 * ((a * 5.0).sin() * 0.5 + 0.5));
                let col = GREEN.mix(LILAC, k).alpha(0.55);
                bar(c, (cx + a.cos() * ring, cy + a.sin() * ring), (cx + a.cos() * (ring + len), cy + a.sin() * (ring + len)), 2.4, col);
            }
            c.ring(cx, cy, ring, t * 0.55, TEAL);
            c.ring(cx, cy, ring, t * 0.2, WHITE.alpha(0.9));
        }
        "frame_eclipse" => {
            // the corona: rays all round, brightest at the top right
            for i in 0..36 {
                let a = i as f32 * TAU / 36.0;
                let k = ((a + PI * 0.25).sin() * 0.5 + 0.5).powf(2.0);
                let len = r * (0.12 + 0.28 * k) * (0.8 + rng.f() * 0.4);
                bar(c, (cx + a.cos() * ring, cy + a.sin() * ring), (cx + a.cos() * (ring + len), cy + a.sin() * (ring + len)), 2.0, GOLD.mix(CRIMSON, 1.0 - k).alpha(0.4 + 0.5 * k));
            }
            c.ring(cx, cy, ring, t * 1.1, Col(10, 4, 14, 255));
            arc(c, cx, cy, ring + t * 0.35, t * 0.45, -PI * 0.75, PI * 0.25, |f| GOLD.alpha((1.0 - (f - 0.5).abs() * 2.0).max(0.0)));
        }
        _ => {
            // Origin Halo: violet and gold one side, teal and ice the other
            for i in 0..16 {
                let a = i as f32 * TAU / 16.0;
                let left = a.cos() < 0.0;
                let len = r * if i % 2 == 0 { 0.34 } else { 0.2 };
                bar(c, (cx + a.cos() * ring, cy + a.sin() * ring), (cx + a.cos() * (ring + len), cy + a.sin() * (ring + len)), 2.4, if left { GOLD } else { TEAL });
            }
            arc(c, cx, cy, ring, t, PI * 0.5, PI * 1.5, |f| VIOLET.mix(CRIMSON, (f * PI).sin() * 0.6));
            arc(c, cx, cy, ring, t, -PI * 0.5, PI * 0.5, |f| TEAL.mix(LILAC, (f * PI).sin() * 0.5));
            c.ring(cx, cy, ring + t * 0.55, 1.4, GOLD.alpha(0.9));
            star(c, cx, cy - ring - r * 0.12, r * 0.12, WHITE);
            star(c, cx, cy + ring + r * 0.12, r * 0.1, ICE);
        }
    }
}

/// A frame by itself on a clear square `size` pixels across, for showing in
/// the app (the picture goes in the middle, radius `size / 2 / (1 + reach)`).
pub fn frame_image(id: &str, size: u32) -> Canvas {
    let mut c = Canvas::new(size, size, Col(0, 0, 0, 0));
    let r = size as f32 / 2.0 / (1.0 + FRAME_REACH);
    draw_frame(&mut c, id, size as f32 / 2.0, size as f32 / 2.0, r);
    c
}

/// A plate by itself on a clear background, `w` × `h` pixels (with a little
/// room above for crowns and snow).
pub fn plate_image(id: &str, w: u32, h: u32) -> Canvas {
    let mut c = Canvas::new(w, h, Col(0, 0, 0, 0));
    let top = h as f32 * 0.22;
    let (px, py, pw, ph) = (2.0, top, w as f32 - 6.0, h as f32 - top - 4.0);
    draw_plate(&mut c, id, px, py, pw, ph);
    // glows spill past the plate; on their own picture they would end in a hard box, so
    // fade everything outside the plate (crowns and snow above it stay, just softened)
    let fade = (top * 0.9).max(2.0);
    for yy in 0..h {
        for xx in 0..w {
            let (fx, fy) = (xx as f32 + 0.5, yy as f32 + 0.5);
            let dx = (px - fx).max(fx - (px + pw)).max(0.0);
            let dy = (py - fy).max(fy - (py + ph)).max(0.0);
            let d = (dx * dx + dy * dy).sqrt();
            if d > 0.0 {
                let i = ((yy * w + xx) * 4 + 3) as usize;
                let edge_x = (fx.min(w as f32 - fx) / 3.0).min(1.0);
                let k = ((1.0 - d / fade).clamp(0.0, 1.0)).powf(0.7) * edge_x;
                // solid decoration (a crown, a crystal) keeps its weight; soft glow fades out
                let a = c.px[i] as f32 / 255.0;
                let k = k + (1.0 - k) * a * a * a * edge_x;
                c.px[i] = (c.px[i] as f32 * k) as u8;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    #[test]
    fn plate_pictures_dump() {
        let Ok(dir) = std::env::var("PLATE_DUMP") else { return };
        for id in super::PLATES {
            let c = super::plate_image(id, 360, 80);
            image::RgbaImage::from_raw(c.w, c.h, c.px.clone()).unwrap().save(format!("{dir}/{id}.png")).unwrap();
        }
        for id in super::FRAMES {
            let c = super::frame_image(id, 240);
            image::RgbaImage::from_raw(c.w, c.h, c.px.clone()).unwrap().save(format!("{dir}/{id}.png")).unwrap();
        }
    }
}
