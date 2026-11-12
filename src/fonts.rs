//! The app's fonts. Inter and Inter Tight (Google Fonts, SIL Open Font
//! License) are built in; see the type system notes below. The optional
//! pixel font ("Press Start 2P", SIL Open Font License) is downloaded once
//! from Google Fonts' GitHub repo and cached; if it can't be fetched the
//! "pixel" family falls back to egui's font, so a missing font never breaks
//! the UI.

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

// ───────────────────────────── type system ─────────────────────────────
//
// The Companion's text uses two Google Fonts (SIL Open Font License, files
// and licences in assets/fonts) that are built to work together:
//
//   · Inter Tight Bold: every title. Page titles, section headings, card
//     titles, hero names and big numbers (anything bold at 14.5px or more).
//   · Inter: everything else. Regular for body text, descriptions and
//     inputs; Medium for buttons; SemiBold for small emphasis (labels,
//     values, the open tab, the open page in the sidebar).
//
// Digits are tabular in both, so counts don't shift as they tick up.
// Segoe UI and egui's own fonts stay behind them for any character they
// don't have (emoji, symbols). The overlay themes keep their own fonts.

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const INTER_TIGHT_BOLD: &[u8] = include_bytes!("../assets/fonts/InterTight-Bold.ttf");

/// Inter runs larger than Segoe UI at the same size; this keeps the
/// existing layouts' sizes reading as intended.
const INTER_SCALE: f32 = 0.93;

/// Titles at this size or larger use Inter Tight.
pub const TITLE_MIN: f32 = 14.5;

/// Body text: Inter Regular.
pub fn sans() -> FontFamily {
    FontFamily::Name("sans".into())
}

/// Buttons: Inter Medium.
pub fn sans_medium() -> FontFamily {
    FontFamily::Name("sans-medium".into())
}

/// Small emphasis (labels, values, tabs): Inter SemiBold.
pub fn sans_bold() -> FontFamily {
    FontFamily::Name("sans-bold".into())
}

/// Titles and big numbers: Inter Tight Bold.
pub fn title() -> FontFamily {
    FontFamily::Name("title".into())
}

pub fn title_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, title())
}

pub fn medium_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, sans_medium())
}

/// Overlay themes: Segoe UI Bold, Black, Black Italic and Light (each
/// falls back to the nearest one there is, then to egui's font).
pub fn bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-bold".into()))
}
pub fn black_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-black".into()))
}
pub fn display_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-display".into()))
}
pub fn light_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-light".into()))
}
/// Consolas (Terminal theme), Ink Free (Field journal) and Impact
/// (Broadcast), each falling back to a Segoe UI weight.
pub fn mono_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-mono".into()))
}
pub fn mono_bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-mono-bold".into()))
}
pub fn hand_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-hand".into()))
}
pub fn impact_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("ov-impact".into()))
}

pub fn sans_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, sans())
}

/// Bold text: Inter SemiBold when small, Inter Tight Bold from
/// `TITLE_MIN` up (a title).
pub fn sans_bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, if size >= TITLE_MIN { title() } else { sans_bold() })
}

/// Segoe UI Bold, Black, Black Italic and Light, Consolas, Ink Free and
/// Impact for the overlay themes.
fn theme_fonts() -> &'static [(&'static str, Option<Vec<u8>>); 8] {
    static FONTS: std::sync::OnceLock<[(&'static str, Option<Vec<u8>>); 8]> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let dir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"))
            .join("Fonts");
        let read = |name: &str| std::fs::read(dir.join(name)).ok().filter(|b| b.len() > 1000);
        let bold = read("segoeuib.ttf").or_else(|| read("seguisb.ttf"));
        let black = read("seguibl.ttf").or_else(|| bold.clone());
        let display = read("seguibli.ttf").or_else(|| black.clone());
        let light = read("segoeuil.ttf").or_else(|| read("segoeui.ttf"));
        let mono = read("consola.ttf").or_else(|| read("cour.ttf")).or_else(|| read("segoeui.ttf"));
        let mono_bold = read("consolab.ttf").or_else(|| read("courbd.ttf")).or_else(|| mono.clone());
        let hand = read("Inkfree.ttf").or_else(|| read("segoepr.ttf")).or_else(|| read("segoesc.ttf")).or_else(|| read("segoeui.ttf"));
        let impact = read("impact.ttf").or_else(|| black.clone());
        [("ov-bold", bold), ("ov-black", black), ("ov-display", display), ("ov-light", light), ("ov-mono", mono), ("ov-mono-bold", mono_bold), ("ov-hand", hand), ("ov-impact", impact)]
    })
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

fn inter(bytes: &'static [u8]) -> FontData {
    let mut d = FontData::from_static(bytes);
    d.tweak = egui::FontTweak { scale: INTER_SCALE, ..Default::default() };
    d
}

fn definitions(pixel_ttf: Option<Vec<u8>>) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("inter".into(), inter(INTER));
    defs.font_data.insert("inter-medium".into(), inter(INTER_MEDIUM));
    defs.font_data.insert("inter-semibold".into(), inter(INTER_SEMIBOLD));
    defs.font_data.insert("inter-tight-bold".into(), inter(INTER_TIGHT_BOLD));
    // egui's own fonts, kept last for emoji and symbols.
    let base = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let (regular, semibold) = system_fonts();
    if let Some(b) = regular {
        defs.font_data.insert("segoe-ui".into(), FontData::from_owned(b.clone()));
    }
    if let Some(b) = semibold.as_ref().or(regular.as_ref()) {
        defs.font_data.insert("segoe-ui-semibold".into(), FontData::from_owned(b.clone()));
    }
    let chain = |first: &[&str], fallback: &str| {
        let mut v: Vec<String> = first.iter().map(|s| s.to_string()).collect();
        if defs.font_data.contains_key(fallback) {
            v.push(fallback.to_string());
        }
        v.extend(base.iter().cloned());
        v
    };
    let body = chain(&["inter"], "segoe-ui");
    let medium = chain(&["inter-medium", "inter"], "segoe-ui");
    let semibold_fam = chain(&["inter-semibold"], "segoe-ui-semibold");
    let title_fam = chain(&["inter-tight-bold", "inter-semibold"], "segoe-ui-semibold");
    // Plain egui text (anything that doesn't name a font) is Inter too.
    defs.families.insert(FontFamily::Proportional, body.clone());
    defs.families.insert(sans(), body);
    defs.families.insert(sans_medium(), medium);
    defs.families.insert(sans_bold(), semibold_fam);
    defs.families.insert(title(), title_fam);
    // The overlay themes' own fonts (Segoe UI weights, Consolas, Ink Free, Impact).
    for (name, bytes) in theme_fonts() {
        let mut f = base.clone();
        if let Some(b) = bytes {
            defs.font_data.insert(name.to_string(), FontData::from_owned(b.clone()));
            f.insert(0, name.to_string());
        }
        defs.families.insert(FontFamily::Name((*name).into()), f);
    }
    let mut fam = base.clone();
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
