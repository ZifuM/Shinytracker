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
// Modern Plus's type pass borrows the share images' heavy Inter: ExtraBold
// for page titles, Black with tabular digits for big numbers.
const INTER_EXTRABOLD: &[u8] = include_bytes!("../assets/fonts/share/Inter-ExtraBold.otf");
const INTER_BLACK_TNUM: &[u8] = include_bytes!("../assets/fonts/share/Inter-Black-Tnum.otf");
// The game themes' pixel face (Pokédex, Pocket, Retro DS): Pixelify Sans
// (Google Fonts, SIL Open Font License), picked because it stays readable,
// unlike the very wide Press Start 2P.
const PIXELIFY: &[u8] = include_bytes!("../assets/fonts/PixelifySans-Medium.ttf");
// The Modern Plus themes' heading faces (Google Fonts, SIL Open Font
// License, licences in assets/fonts/themes).
const TH_OUTFIT: &[u8] = include_bytes!("../assets/fonts/themes/Outfit-Bold.ttf");
const TH_BRICOLAGE: &[u8] = include_bytes!("../assets/fonts/themes/BricolageGrotesque-Bold.ttf");
const TH_JURA: &[u8] = include_bytes!("../assets/fonts/themes/Jura-Medium.ttf");
const TH_YOUNG: &[u8] = include_bytes!("../assets/fonts/themes/YoungSerif-Regular.ttf");
const TH_TEKTUR: &[u8] = include_bytes!("../assets/fonts/themes/Tektur-Medium.ttf");
const TH_GLOOCK: &[u8] = include_bytes!("../assets/fonts/themes/Gloock-Regular.ttf");
const TH_ERICA: &[u8] = include_bytes!("../assets/fonts/themes/EricaOne-Regular.ttf");
const TH_BOLDONSE: &[u8] = include_bytes!("../assets/fonts/themes/Boldonse-Regular.ttf");
const TH_INSTRUMENT: &[u8] = include_bytes!("../assets/fonts/themes/InstrumentSans-Bold.ttf");
const TH_ITALIANA: &[u8] = include_bytes!("../assets/fonts/themes/Italiana-Regular.ttf");
const TH_SCRAWL: &[u8] = include_bytes!("../assets/fonts/themes/NothingYouCouldDo-Regular.ttf");

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

/// Modern Plus page titles: Inter ExtraBold (as on the share images).
pub fn heavy_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("heavy".into()))
}

/// Companion's headings.
pub fn outfit_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-outfit".into()))
}
/// Studio's headings.
pub fn bricolage_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-bricolage".into()))
}
/// Aurora glass's headings.
pub fn jura_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-jura".into()))
}
/// Field guide's headings.
pub fn young_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-young".into()))
}
/// Synthwave's headings.
pub fn tektur_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-tektur".into()))
}
/// Gold's headings.
pub fn gloock_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-gloock".into()))
}
/// Candy's headings.
pub fn erica_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-erica".into()))
}
/// Brutal's headings.
pub fn boldonse_font(size: f32) -> egui::FontId {
    // (it runs very big: a little smaller keeps it in line with the others)
    egui::FontId::new(size * 0.8, FontFamily::Name("th-boldonse".into()))
}
/// Liquid's headings.
pub fn instrument_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-instrument".into()))
}
/// Winter's headings.
pub fn italiana_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-italiana".into()))
}
/// Spooky's headings.
pub fn scrawl_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("th-scrawl".into()))
}

/// The game themes' readable pixel face (headers, big numbers, buttons).
pub fn pixel_ui_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("pixel-ui".into()))
}

/// Modern Plus big numbers: Inter Black, tabular digits (as on the share
/// images), so counts don't shift as they tick up.
pub fn number_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name("number".into()))
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

/// A theme's face on everything: "pixelify" (the pixel themes) or "ov-hand"
/// (Field journal's pen), or none.
static FACE_EVERYWHERE: std::sync::Mutex<Option<&'static str>> = std::sync::Mutex::new(None);
/// The downloaded Press Start 2P, kept so the fonts can be rebuilt.
static PIXEL_TTF: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);

/// Put a theme's face on all the Companion's text (the pixel themes' pixel
/// face, Field journal's handwriting), or take it off again. Rebuilds the
/// fonts only when it changes.
pub fn set_face_everywhere(ctx: &egui::Context, face: Option<&'static str>) {
    let changed = match FACE_EVERYWHERE.lock() {
        Ok(mut g) if *g != face => {
            *g = face;
            true
        }
        _ => false,
    };
    if changed {
        let ttf = PIXEL_TTF.lock().map(|g| g.clone()).unwrap_or(None);
        ctx.set_fonts(definitions(ttf));
    }
}

fn definitions(pixel_ttf: Option<Vec<u8>>) -> FontDefinitions {
    if let (Some(b), Ok(mut g)) = (pixel_ttf.as_ref(), PIXEL_TTF.lock()) {
        *g = Some(b.clone());
    }
    let everywhere = FACE_EVERYWHERE.lock().map(|g| *g).unwrap_or(None);
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("inter".into(), inter(INTER));
    defs.font_data.insert("inter-medium".into(), inter(INTER_MEDIUM));
    defs.font_data.insert("inter-semibold".into(), inter(INTER_SEMIBOLD));
    defs.font_data.insert("inter-tight-bold".into(), inter(INTER_TIGHT_BOLD));
    defs.font_data.insert("inter-extrabold".into(), inter(INTER_EXTRABOLD));
    defs.font_data.insert("inter-black-tnum".into(), inter(INTER_BLACK_TNUM));
    defs.font_data.insert("pixelify".into(), FontData::from_static(PIXELIFY));
    defs.font_data.insert("th-outfit".into(), FontData::from_static(TH_OUTFIT));
    defs.font_data.insert("th-bricolage".into(), FontData::from_static(TH_BRICOLAGE));
    defs.font_data.insert("th-jura".into(), FontData::from_static(TH_JURA));
    defs.font_data.insert("th-young".into(), FontData::from_static(TH_YOUNG));
    defs.font_data.insert("th-tektur".into(), FontData::from_static(TH_TEKTUR));
    defs.font_data.insert("th-gloock".into(), FontData::from_static(TH_GLOOCK));
    defs.font_data.insert("th-erica".into(), FontData::from_static(TH_ERICA));
    defs.font_data.insert("th-boldonse".into(), FontData::from_static(TH_BOLDONSE));
    defs.font_data.insert("th-instrument".into(), FontData::from_static(TH_INSTRUMENT));
    defs.font_data.insert("th-italiana".into(), FontData::from_static(TH_ITALIANA));
    defs.font_data.insert("th-scrawl".into(), FontData::from_static(TH_SCRAWL));
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
    // (the share fonts only carry Latin; anything else falls back to the title face)
    let heavy_fam = chain(&["inter-extrabold", "inter-tight-bold", "inter-semibold"], "segoe-ui-semibold");
    let number_fam = chain(&["inter-black-tnum", "inter-extrabold", "inter-tight-bold"], "segoe-ui-semibold");
    // Plain egui text (anything that doesn't name a font) is Inter too.
    defs.families.insert(FontFamily::Proportional, body.clone());
    defs.families.insert(sans(), body);
    defs.families.insert(sans_medium(), medium);
    defs.families.insert(sans_bold(), semibold_fam);
    defs.families.insert(title(), title_fam);
    defs.families.insert(FontFamily::Name("heavy".into()), heavy_fam);
    defs.families.insert(FontFamily::Name("number".into()), number_fam);
    let pixel_ui_fam = chain(&["pixelify", "inter-semibold"], "segoe-ui-semibold");
    defs.families.insert(FontFamily::Name("th-outfit".into()), chain(&["th-outfit", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-bricolage".into()), chain(&["th-bricolage", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-jura".into()), chain(&["th-jura", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-young".into()), chain(&["th-young", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-tektur".into()), chain(&["th-tektur", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-gloock".into()), chain(&["th-gloock", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-erica".into()), chain(&["th-erica", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-boldonse".into()), chain(&["th-boldonse", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-instrument".into()), chain(&["th-instrument", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-italiana".into()), chain(&["th-italiana", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("th-scrawl".into()), chain(&["th-scrawl", "inter-semibold"], "segoe-ui-semibold"));
    defs.families.insert(FontFamily::Name("pixel-ui".into()), pixel_ui_fam);
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
    // a theme's face in front of every face the Companion uses (only when
    // that font is there: Ink Free comes from Windows)
    if let Some(face) = everywhere.filter(|f| defs.font_data.contains_key(*f)) {
        let mut names: Vec<FontFamily> = vec![FontFamily::Proportional, sans(), sans_medium(), sans_bold(), title()];
        names.push(FontFamily::Name("heavy".into()));
        names.push(FontFamily::Name("number".into()));
        for f in names {
            if let Some(list) = defs.families.get_mut(&f) {
                list.insert(0, face.to_string());
            }
        }
    }
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
