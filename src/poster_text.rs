//! Fonts for the rare grass infographic: Bahnschrift, set condensed and
//! bold, for titles and numbers, and Segoe UI for text. Both come with
//! Windows 10/11; egui's own font is used for any that are missing.

use crate::canvas::{Canvas, Col};
use crate::poster::{PFace, PosterText};
use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont, VariableFont};
use eframe::egui;
use std::path::PathBuf;

pub struct PosterFonts {
    /// One per `PFace`, in its order.
    faces: Vec<Option<FontVec>>,
}

fn index(face: PFace) -> usize {
    match face {
        PFace::D800 => 0,
        PFace::D700 => 1,
        PFace::D600 => 2,
        PFace::T600 => 3,
        PFace::T500 => 4,
    }
}

impl PosterFonts {
    pub fn load() -> Self {
        let dir = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
            .join("Fonts");
        let read = |name: &str| -> Option<FontVec> {
            let bytes = std::fs::read(dir.join(name)).ok().filter(|b| b.len() > 1000)?;
            FontVec::try_from_vec(bytes).ok()
        };
        let egui_font = || -> Option<FontVec> {
            let defs = egui::FontDefinitions::default();
            let f = defs.font_data.get("Ubuntu-Light")?;
            FontVec::try_from_vec(f.font.to_vec()).ok()
        };
        // Bahnschrift is a variable font: narrow it and pick the weight.
        let display = |weight: f32| -> Option<FontVec> {
            let mut f = read("bahnschrift.ttf")?;
            f.set_variation(b"wdth", 75.0);
            f.set_variation(b"wght", weight);
            Some(f)
        };
        let bold = || read("segoeuib.ttf").or_else(|| read("seguisb.ttf")).or_else(egui_font);
        let faces = vec![
            display(700.0).or_else(|| read("seguibl.ttf")).or_else(bold),
            display(640.0).or_else(bold),
            display(520.0).or_else(|| read("seguisb.ttf")).or_else(bold),
            read("seguisb.ttf").or_else(bold),
            read("segoeui.ttf").or_else(egui_font),
        ];
        Self { faces }
    }

    fn font(&self, face: PFace) -> Option<&FontVec> {
        self.faces[index(face)].as_ref().or_else(|| self.faces.iter().flatten().next())
    }
}

impl PosterText for PosterFonts {
    fn width(&self, s: &str, px: f32, face: PFace) -> f32 {
        let Some(f) = self.font(face) else {
            return s.chars().count() as f32 * px * 0.55;
        };
        let sf = f.as_scaled(PxScale::from(px));
        let mut w = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                w += sf.kern(p, id);
            }
            w += sf.h_advance(id);
            prev = Some(id);
        }
        w
    }

    fn draw(&self, c: &mut Canvas, s: &str, x: f32, baseline: f32, px: f32, col: Col, face: PFace) {
        let Some(f) = self.font(face) else { return };
        let scale = PxScale::from(px);
        let sf = f.as_scaled(scale);
        let mut pen = x;
        let mut prev = None;
        for ch in s.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                pen += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(scale, point(pen, baseline.round()));
            pen += sf.h_advance(id);
            prev = Some(id);
            if let Some(og) = f.outline_glyph(g) {
                let b = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    c.blend(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, col, cov);
                });
            }
        }
    }
}
