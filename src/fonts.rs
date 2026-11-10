//! Optional pixel font ("Press Start 2P", SIL Open Font License) downloaded
//! once from Google Fonts' GitHub repo and cached. If it can't be fetched the
//! app keeps egui's default font. Everything uses the "pixel" family name,
//! which always exists, so a missing font never breaks the UI.

use crate::config::cache_dir;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::mpsc::{channel, Receiver};

const URL: &str =
    "https://raw.githubusercontent.com/google/fonts/main/ofl/pressstart2p/PressStart2P-Regular.ttf";

pub fn pixel() -> FontFamily {
    FontFamily::Name("pixel".into())
}

pub fn pixel_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, pixel())
}

/// Clean-style text font (Segoe UI from Windows; egui's font if missing).
pub fn sans() -> FontFamily {
    FontFamily::Name("sans".into())
}

/// Clean-style heading font (Segoe UI Semibold; falls back like `sans`).
pub fn sans_bold() -> FontFamily {
    FontFamily::Name("sans-bold".into())
}

pub fn sans_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, sans())
}

pub fn sans_bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, sans_bold())
}

/// Segoe UI regular + semibold, read once from the Windows fonts folder.
fn system_fonts() -> &'static (Option<Vec<u8>>, Option<Vec<u8>>) {
    static FONTS: std::sync::OnceLock<(Option<Vec<u8>>, Option<Vec<u8>>)> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let dir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"))
            .join("Fonts");
        let read = |name: &str| std::fs::read(dir.join(name)).ok().filter(|b| b.len() > 1000);
        (read("segoeui.ttf"), read("seguisb.ttf").or_else(|| read("segoeuib.ttf")))
    })
}

fn definitions(pixel_ttf: Option<Vec<u8>>) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut fam = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    // Clean-style families: Segoe UI first, then egui's own fonts (emoji
    // and symbols still come from those).
    let (regular, semibold) = system_fonts();
    let mut sans_fam = fam.clone();
    if let Some(b) = regular {
        defs.font_data.insert("segoe-ui".into(), FontData::from_owned(b.clone()));
        sans_fam.insert(0, "segoe-ui".into());
    }
    let mut bold_fam = fam.clone();
    if let Some(b) = semibold.as_ref().or(regular.as_ref()) {
        defs.font_data.insert("segoe-ui-semibold".into(), FontData::from_owned(b.clone()));
        bold_fam.insert(0, "segoe-ui-semibold".into());
    }
    defs.families.insert(sans(), sans_fam);
    defs.families.insert(sans_bold(), bold_fam);
    if let Some(bytes) = pixel_ttf {
        defs.font_data
            .insert("press-start".into(), FontData::from_owned(bytes));
        fam.insert(0, "press-start".into());
    }
    defs.families.insert(pixel(), fam);
    defs
}

pub struct FontLoader {
    rx: Option<Receiver<Vec<u8>>>,
}

impl FontLoader {
    pub fn start(ctx: &egui::Context) -> Self {
        ctx.set_fonts(definitions(None));
        let (tx, rx) = channel();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let path = cache_dir().join("PressStart2P-Regular.ttf");
            let bytes = std::fs::read(&path).ok().or_else(|| {
                let b = crate::sprites::fetch(URL).ok()?;
                // TrueType files start with 0x00010000.
                if b.len() > 1000 && b.starts_with(&[0, 1, 0, 0]) {
                    let _ = std::fs::create_dir_all(cache_dir());
                    let _ = std::fs::write(&path, &b);
                    Some(b)
                } else {
                    None
                }
            });
            if let Some(b) = bytes {
                let _ = tx.send(b);
                ctx2.request_repaint();
            }
        });
        Self { rx: Some(rx) }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.rx {
            if let Ok(bytes) = rx.try_recv() {
                ctx.set_fonts(definitions(Some(bytes)));
                self.rx = None;
            }
        }
    }
}
