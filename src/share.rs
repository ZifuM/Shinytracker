//! Share images: real font drawing for the card (ab_glyph, the same font
//! engine egui uses), loading sprite frames from the sprite cache, saving PNGs
//! and copying the image to the clipboard so it pastes straight into Discord.

use crate::canvas::{Canvas, Col, Img};
use crate::card::{CardData, CardOptions, Face, TextDraw};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use crate::config::cache_dir;
use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use eframe::egui;
use std::path::PathBuf;

pub struct AbText {
    pixel: Option<FontArc>,
    body: Option<FontArc>,
    /// Segoe UI Bold, for the anime style's labels.
    bold: Option<FontArc>,
    /// Segoe UI Black Italic, for the anime style's titles and numbers.
    display: Option<FontArc>,
    /// Segoe UI Black, Light, Consolas (+ bold), Ink Free and Impact for
    /// the themed styles.
    black: Option<FontArc>,
    light: Option<FontArc>,
    mono: Option<FontArc>,
    mono_bold: Option<FontArc>,
    hand: Option<FontArc>,
    impact: Option<FontArc>,
}

impl AbText {
    pub fn load() -> Self {
        let defs = egui::FontDefinitions::default();
        let body = defs
            .font_data
            .get("Ubuntu-Light")
            .and_then(|f| FontArc::try_from_vec(f.font.to_vec()).ok());
        let pixel = std::fs::read(cache_dir().join("PressStart2P-Regular.ttf"))
            .ok()
            .and_then(|b| FontArc::try_from_vec(b).ok());
        let dir = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
            .join("Fonts");
        let read = |name: &str| -> Option<FontArc> {
            let bytes = std::fs::read(dir.join(name)).ok().filter(|b| b.len() > 1000)?;
            FontArc::try_from_vec(bytes).ok()
        };
        let bold = read("segoeuib.ttf").or_else(|| read("seguisb.ttf"));
        let display = read("seguibli.ttf").or_else(|| read("seguibl.ttf")).or_else(|| bold.clone());
        let black = read("seguibl.ttf").or_else(|| bold.clone());
        let light = read("segoeuil.ttf").or_else(|| read("segoeui.ttf"));
        let mono = read("consola.ttf").or_else(|| read("cour.ttf"));
        let mono_bold = read("consolab.ttf").or_else(|| read("courbd.ttf")).or_else(|| mono.clone());
        let hand = read("Inkfree.ttf").or_else(|| read("segoepr.ttf")).or_else(|| read("segoesc.ttf"));
        let impact = read("impact.ttf").or_else(|| black.clone());
        Self { pixel, body, bold, display, black, light, mono, mono_bold, hand, impact }
    }

    pub fn has_pixel(&self) -> bool {
        self.pixel.is_some()
    }

    fn font(&self, face: Face) -> Option<&FontArc> {
        match face {
            Face::Pixel => self.pixel.as_ref().or(self.body.as_ref()),
            Face::Body => self.body.as_ref().or(self.pixel.as_ref()),
            Face::Bold => self.bold.as_ref().or(self.body.as_ref()).or(self.pixel.as_ref()),
            Face::Display => self.display.as_ref().or(self.bold.as_ref()).or(self.body.as_ref()),
            Face::Black => self.black.as_ref().or(self.bold.as_ref()).or(self.body.as_ref()),
            Face::Light => self.light.as_ref().or(self.body.as_ref()),
            Face::Mono => self.mono.as_ref().or(self.body.as_ref()),
            Face::MonoBold => self.mono_bold.as_ref().or(self.mono.as_ref()).or(self.bold.as_ref()),
            Face::Hand => self.hand.as_ref().or(self.body.as_ref()),
            Face::Impact => self.impact.as_ref().or(self.black.as_ref()).or(self.bold.as_ref()),
        }
    }
}

impl TextDraw for AbText {
    fn width(&self, s: &str, size: f32, face: Face) -> f32 {
        let Some(f) = self.font(face) else {
            return s.chars().count() as f32 * size * 0.6;
        };
        let sf = f.as_scaled(PxScale::from(size));
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

    fn draw(&self, c: &mut Canvas, s: &str, x: f32, y: f32, size: f32, col: Col, face: Face) {
        let Some(f) = self.font(face) else { return };
        let scale = PxScale::from(size);
        let sf = f.as_scaled(scale);
        let base = (y + sf.ascent()).round();
        let mut pen = x;
        let mut prev = None;
        for ch in s.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                pen += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(scale, point(pen.round(), base));
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

/// First frame of a cached sprite (normal or shiny), if it has been downloaded.
pub fn sprite_frame(slug: &str, shiny: bool) -> Option<Img> {
    let path = crate::sprites::cached_file(slug, shiny)?;
    let bytes = std::fs::read(path).ok()?;
    let img = if bytes.starts_with(b"GIF8") {
        use image::AnimationDecoder;
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&bytes)).ok()?;
        let frame = dec.into_frames().next()?.ok()?;
        frame.into_buffer()
    } else {
        image::load_from_memory(&bytes).ok()?.to_rgba8()
    };
    let (w, h) = (img.width(), img.height());
    Some(Img { w, h, px: img.into_raw() })
}

/// Every frame of a cached sprite with its delay in ms.
pub fn sprite_frames(slug: &str, shiny: bool) -> Vec<(Img, u32)> {
    let Some(path) = crate::sprites::cached_file(slug, shiny) else { return Vec::new() };
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    if !bytes.starts_with(b"GIF8") {
        return sprite_frame(slug, shiny).map(|i| vec![(i, 0)]).unwrap_or_default();
    }
    use image::AnimationDecoder;
    let Ok(dec) = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&bytes)) else {
        return Vec::new();
    };
    let Ok(frames) = dec.into_frames().collect_frames() else { return Vec::new() };
    frames
        .into_iter()
        .map(|f| {
            let (n, d) = f.delay().numer_denom_ms();
            let ms = if d == 0 || n == 0 { 100 } else { (n / d).max(20) };
            let b = f.into_buffer();
            let (w, h) = (b.width(), b.height());
            (Img { w, h, px: b.into_raw() }, ms)
        })
        .collect()
}

/// One crop box covering the sprite in every frame (so it doesn't jiggle).
fn union_box(frames: &[(Img, u32)]) -> Option<(u32, u32, u32, u32)> {
    let mut acc: Option<(u32, u32, u32, u32)> = None;
    for (img, _) in frames {
        let (x, y, w, h) = Canvas::trim_box(img);
        acc = Some(match acc {
            None => (x, y, x + w, y + h),
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x + w), d.max(y + h)),
        });
    }
    acc.map(|(a, b, c, d)| (a, b, c - a, d - b))
}

pub struct GifOut {
    pub bytes: Vec<u8>,
    /// Frames at the GIF's size, for the animated preview.
    pub frames: Vec<(Img, u32)>,
}

/// An animated share image being made in the background.
pub struct GifJob {
    pub done: Arc<AtomicUsize>,
    pub total: usize,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Result<GifOut, String>>,
}

impl GifJob {
    pub fn poll(&self) -> Option<Result<GifOut, String>> {
        self.rx.try_recv().ok()
    }
}

impl Drop for GifJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Render the animated card (moving sprite, floating stars) at the chosen
/// resolution and encode a looping GIF. After the first frame, each frame only
/// stores the pixels that changed, which keeps big GIFs small.
pub fn start_gif(data: CardData, opts: CardOptions, slug: Option<String>, ctx: egui::Context) -> GifJob {
    let frames = slug.as_deref().map(|s| sprite_frames(s, true)).unwrap_or_default();
    let sprite_ms: u32 = if frames.len() > 1 { frames.iter().map(|f| f.1).sum() } else { 0 };
    // Loop length: a whole number of sprite loops, about 2-4 seconds.
    let loop_ms = if sprite_ms == 0 {
        3000
    } else if sprite_ms >= 2000 {
        sprite_ms.min(8000)
    } else {
        sprite_ms * 2000u32.div_ceil(sprite_ms).max(1)
    };
    // GIF delays are in 1/100 s: use 6 cs (about 16 fps), keeping the loop exact.
    let n = ((loop_ms as f32 / 60.0).round() as usize).clamp(16, 120);
    let step = loop_ms as f32 / n as f32;
    let done = Arc::new(AtomicUsize::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel();
    let job = GifJob { done: done.clone(), total: n, cancel: cancel.clone(), rx };
    std::thread::Builder::new()
        .name("share-gif".into())
        .spawn(move || {
            let result = (|| -> Result<GifOut, String> {
                let fonts = AbText::load();
                let mut data = data;
                data.sprite_box = union_box(&frames);
                let (w, h, _) = opts.output_size();
                let base = crate::card::render_base(&opts);
                let pw = w.min(640);
                let ph = (h as f32 * pw as f32 / w as f32).round() as u32;
                let mut bytes = Vec::new();
                let mut preview = Vec::with_capacity(n);
                {
                    let mut enc = gif::Encoder::new(&mut bytes, w as u16, h as u16, &[]).map_err(|e| e.to_string())?;
                    enc.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                    let mut prev: Option<Vec<u8>> = None;
                    let mut t_cs = 0u32; // keeps the total exact with whole-cs delays
                    for i in 0..n {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".into());
                        }
                        let t = i as f32 * step;
                        if sprite_ms > 0 {
                            let mut tt = (t as u32) % sprite_ms;
                            let mut pick = &frames[0].0;
                            for (img, d) in &frames {
                                if tt < *d {
                                    pick = img;
                                    break;
                                }
                                tt -= d;
                            }
                            data.shiny = Some(pick.clone());
                        } else if let Some((img, _)) = frames.first() {
                            data.shiny = Some(img.clone());
                        }
                        let c = crate::card::render_on(base.clone(), &data, &opts, &fonts, Some(i as f32 / n as f32));
                        let end_cs = (((i + 1) as f32 * step) / 10.0).round() as u32;
                        let delay = (end_cs - t_cs).max(1) as u16;
                        t_cs = end_cs;
                        let mut frame = match &prev {
                            None => {
                                let mut px = c.px.clone();
                                gif::Frame::from_rgba_speed(w as u16, h as u16, &mut px, 10)
                            }
                            Some(p) => diff_frame(p, &c.px, w, h),
                        };
                        frame.delay = delay;
                        frame.dispose = gif::DisposalMethod::Keep;
                        enc.write_frame(&frame).map_err(|e| e.to_string())?;
                        // small copy for the in-app preview
                        let big = image::RgbaImage::from_raw(c.w, c.h, c.px.clone()).ok_or("image error")?;
                        let small = image::imageops::resize(&big, pw, ph, image::imageops::FilterType::Triangle);
                        preview.push((Img { w: pw, h: ph, px: small.into_raw() }, delay as u32 * 10));
                        prev = Some(c.px);
                        done.fetch_add(1, Ordering::Relaxed);
                        ctx.request_repaint();
                    }
                }
                Ok(GifOut { bytes, frames: preview })
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        })
        .expect("spawn gif thread");
    job
}

/// A GIF frame holding only the pixels that differ from the previous frame
/// (cropped to where they are; unchanged pixels are transparent).
fn diff_frame(prev: &[u8], cur: &[u8], w: u32, h: u32) -> gif::Frame<'static> {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    for y in 0..h {
        let row = (y * w * 4) as usize;
        for x in 0..w {
            let i = row + (x * 4) as usize;
            if prev[i..i + 4] != cur[i..i + 4] {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 {
        // nothing moved: a single transparent pixel keeps the timing
        let mut px = vec![0u8; 4];
        return gif::Frame::from_rgba_speed(1, 1, &mut px, 10);
    }
    let (fw, fh) = (x1 - x0, y1 - y0);
    let mut px = Vec::with_capacity((fw * fh * 4) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 4) as usize;
            if prev[i..i + 4] == cur[i..i + 4] {
                px.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                px.extend_from_slice(&[cur[i], cur[i + 1], cur[i + 2], 255]);
            }
        }
    }
    let mut f = gif::Frame::from_rgba_speed(fw as u16, fh as u16, &mut px, 10);
    f.left = x0 as u16;
    f.top = y0 as u16;
    f
}

pub fn icon() -> Img {
    let (px, w, h) = crate::icon::rgba();
    Img { w, h, px }
}

fn rgba_image(c: &Canvas) -> Option<image::RgbaImage> {
    image::RgbaImage::from_raw(c.w, c.h, c.px.clone())
}

pub fn png_bytes(c: &Canvas) -> Option<Vec<u8>> {
    let img = rgba_image(c)?;
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

pub fn default_share_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(crate::config::data_dir)
        .join("Shiny Tracker")
}

/// Save image bytes (PNG or GIF) in `dir` with a name + date. Returns the path.
pub fn save_bytes(bytes: &[u8], name: &str, ext: &str, dir: &std::path::Path) -> Result<PathBuf, String> {
    let path = out_path(name, ext, dir)?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Write to an exact path (from a "Save as" box), adding the extension if missing.
pub fn save_to(bytes: &[u8], path: &std::path::Path, ext: &str) -> Result<PathBuf, String> {
    let mut path = path.to_path_buf();
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        let mut s = path.into_os_string();
        s.push(format!(".{ext}"));
        path = PathBuf::from(s);
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

/// A file name like "porygon-2322" made safe for Windows.
pub fn clean_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect();
    clean.trim_matches('-').to_string()
}

/// Open a folder in File Explorer.
pub fn open_dir(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(dir).spawn();
    }
}

/// Windows' "choose a folder" box, on a background thread.
pub fn pick_folder(start: PathBuf, ctx: egui::Context) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(imp::pick_folder(&start));
        ctx.request_repaint();
    });
    rx
}

/// Windows' "open file" box for a song (alert song), on a background thread.
pub fn pick_audio(start: PathBuf, ctx: egui::Context) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(imp::pick_audio(&start));
        ctx.request_repaint();
    });
    rx
}

/// Backup file box on a background thread: "Save as" (`save`) or "Open".
pub fn pick_backup(save: bool, start: PathBuf, name: String, ctx: egui::Context) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(imp::pick_backup(save, &start, &name));
        ctx.request_repaint();
    });
    rx
}

/// Windows' "Save as" box, on a background thread.
pub fn pick_save(start: PathBuf, file_name: String, ext: &'static str, ctx: egui::Context) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(imp::pick_save(&start, &file_name, ext));
        ctx.request_repaint();
    });
    rx
}

fn out_path(name: &str, ext: &str, dir: &std::path::Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let now = crate::stats::now_unix();
    let local = now as i64 + crate::stats::utc_offset_secs();
    let (y, m, d) = crate::stats::civil_from_days(local.div_euclid(86_400));
    let t = local.rem_euclid(86_400);
    let file = format!(
        "{}-{y:04}{m:02}{d:02}-{:02}{:02}{:02}.{ext}",
        clean_name(name),
        t / 3600,
        (t / 60) % 60,
        t % 60
    );
    Ok(dir.join(file))
}

/// Put a file on the clipboard (like Copy in Explorer), so Ctrl+V in Discord
/// uploads it, animated GIFs included.
pub fn copy_file(path: &std::path::Path) -> Result<(), String> {
    imp::copy_file(path)
}

/// Open File Explorer with the file selected.
pub fn reveal(path: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn();
    }
    #[cfg(not(windows))]
    let _ = path;
}

/// Copy the image to the clipboard (PNG + bitmap) for pasting into Discord.
pub fn copy_image(c: &Canvas) -> Result<(), String> {
    let png = png_bytes(c).ok_or("couldn't encode the image")?;
    imp::copy(c, &png)
}

#[cfg(windows)]
mod imp {
    use crate::canvas::Canvas;
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(owner: isize) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
        fn CloseClipboard() -> i32;
        fn RegisterClipboardFormatW(name: *const u16) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
        fn GlobalLock(mem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(mem: *mut c_void) -> i32;
        fn GlobalFree(mem: *mut c_void) -> *mut c_void;
    }
    const GMEM_MOVEABLE: u32 = 0x0002;
    const CF_DIB: u32 = 8;

    unsafe fn put(format: u32, data: &[u8]) -> bool {
        let mem = GlobalAlloc(GMEM_MOVEABLE, data.len());
        if mem.is_null() {
            return false;
        }
        let dst = GlobalLock(mem) as *mut u8;
        if dst.is_null() {
            GlobalFree(mem);
            return false;
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        GlobalUnlock(mem);
        if SetClipboardData(format, mem).is_null() {
            GlobalFree(mem);
            return false;
        }
        true
    }

    fn dib(c: &Canvas) -> Vec<u8> {
        let (w, h) = (c.w as usize, c.h as usize);
        let mut out = Vec::with_capacity(40 + w * h * 4);
        let hdr: [u32; 10] = [40, w as u32, h as u32, 0, 0, (w * h * 4) as u32, 0, 0, 0, 0];
        for (i, v) in hdr.iter().enumerate() {
            if i == 3 {
                // biPlanes = 1 (u16), biBitCount = 32 (u16)
                out.extend_from_slice(&1u16.to_le_bytes());
                out.extend_from_slice(&32u16.to_le_bytes());
            } else {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for y in (0..h).rev() {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let p = &c.px[i..i + 4];
                out.extend_from_slice(&[p[2], p[1], p[0], 255]);
            }
        }
        out
    }

    pub fn pick_folder(start: &std::path::Path) -> Option<std::path::PathBuf> {
        rfd::FileDialog::new()
            .set_title("Where should share images be saved?")
            .set_directory(start)
            .pick_folder()
    }

    pub fn pick_backup(save: bool, start: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
        let d = rfd::FileDialog::new()
            .set_directory(start)
            .add_filter("Shiny Tracker backup", &["json"]);
        if save {
            d.set_title("Save a backup of Shiny Tracker").set_file_name(name).save_file()
        } else {
            d.set_title("Restore Shiny Tracker from a backup").pick_file()
        }
    }

    pub fn pick_audio(start: &std::path::Path) -> Option<std::path::PathBuf> {
        rfd::FileDialog::new()
            .set_title("Choose your alert song")
            .set_directory(start)
            .add_filter("Audio", &["mp3", "wav", "wma", "m4a"])
            .pick_file()
    }

    pub fn pick_save(start: &std::path::Path, name: &str, ext: &str) -> Option<std::path::PathBuf> {
        let kind = if ext == "gif" { "Animated GIF" } else { "PNG image" };
        rfd::FileDialog::new()
            .set_title("Save share image")
            .set_directory(start)
            .set_file_name(name)
            .add_filter(kind, &[ext])
            .save_file()
    }

    unsafe fn open() -> bool {
        for _ in 0..10 {
            if OpenClipboard(0) != 0 {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        false
    }

    pub fn copy_file(path: &std::path::Path) -> Result<(), String> {
        const CF_HDROP: u32 = 15;
        // DROPFILES header: offset to the file list, point, non-client flag, wide chars.
        let mut data = Vec::new();
        for v in [20u32, 0, 0, 0, 1] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let wide: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().chain([0, 0]).collect();
        for u in wide {
            data.extend_from_slice(&u.to_le_bytes());
        }
        unsafe {
            if !open() {
                return Err("the clipboard is busy, try again".into());
            }
            EmptyClipboard();
            let ok = put(CF_HDROP, &data);
            let name: Vec<u16> = "Preferred DropEffect".encode_utf16().chain(Some(0)).collect();
            let fmt = RegisterClipboardFormatW(name.as_ptr());
            if fmt != 0 {
                put(fmt, &1u32.to_le_bytes()); // DROPEFFECT_COPY
            }
            CloseClipboard();
            if ok {
                Ok(())
            } else {
                Err("couldn't copy the file".into())
            }
        }
    }

    pub fn copy(c: &Canvas, png: &[u8]) -> Result<(), String> {
        unsafe {
            let mut opened = false;
            for _ in 0..10 {
                if OpenClipboard(0) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            if !opened {
                return Err("the clipboard is busy, try again".into());
            }
            EmptyClipboard();
            let name: Vec<u16> = "PNG".encode_utf16().chain(Some(0)).collect();
            let png_fmt = RegisterClipboardFormatW(name.as_ptr());
            let a = png_fmt != 0 && put(png_fmt, png);
            let b = put(CF_DIB, &dib(c));
            CloseClipboard();
            if a || b {
                Ok(())
            } else {
                Err("couldn't copy the image".into())
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn pick_folder(_s: &std::path::Path) -> Option<std::path::PathBuf> {
        None
    }
    pub fn pick_audio(_s: &std::path::Path) -> Option<std::path::PathBuf> {
        None
    }
    pub fn pick_backup(_save: bool, _s: &std::path::Path, _n: &str) -> Option<std::path::PathBuf> {
        None
    }
    pub fn pick_save(_s: &std::path::Path, _n: &str, _e: &str) -> Option<std::path::PathBuf> {
        None
    }
    pub fn copy_file(_p: &std::path::Path) -> Result<(), String> {
        Err("only supported on Windows".into())
    }
    pub fn copy(_c: &crate::canvas::Canvas, _png: &[u8]) -> Result<(), String> {
        Err("copying images is only supported on Windows".into())
    }
}
