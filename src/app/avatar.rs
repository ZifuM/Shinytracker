//! Profile picture: pick a picture (PNG, JPG or GIF, or drop one on the
//! window), move it and zoom it inside a round frame in a floating window,
//! and save. The result is a 256 × 256 PNG, `avatar.png`, next to the other
//! saves; a copy of the picture (at most 1024 px) is kept as
//! `avatar-source.png` so "Adjust" can reopen it at full quality.
//! Shown on the Profile page and on the trainer card in the sidebar.

use super::charts as ch;
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::App;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureId, TextureOptions, Vec2};
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

const FILE: &str = "avatar.png";
const SOURCE: &str = "avatar-source.png";
/// Saved picture size.
pub const OUT: u32 = 256;
/// Pictures bigger than this are scaled down first (plenty for a 256 px crop).
const MAX_SRC: u32 = 1024;
/// Refuse files bigger than this.
const MAX_BYTES: u64 = 60 * 1024 * 1024;
/// The crop stage and the round frame inside it, in points.
const STAGE: f32 = 300.0;
const FRAME: f32 = 244.0;
const MAX_ZOOM: f32 = 8.0;

fn file() -> PathBuf {
    crate::config::data_dir().join(FILE)
}
fn source_file() -> PathBuf {
    crate::config::data_dir().join(SOURCE)
}

/// The saved picture made round, for the share images (None without one).
/// Kept until the file changes.
pub fn card_avatar() -> Option<crate::canvas::Img> {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(Option<std::time::SystemTime>, Option<crate::canvas::Img>)>> = Mutex::new(None);
    let path = file();
    let stamp = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let mut c = CACHE.lock().ok()?;
    if let Some((s, img)) = c.as_ref() {
        if *s == stamp {
            return img.clone();
        }
    }
    let img = stamp.and_then(|_| std::fs::read(&path).ok()).and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
        let i = i.to_rgba8();
        let side = i.width().min(i.height()).min(512);
        let i = image::imageops::resize(&image::imageops::crop_imm(&i, (i.width() - i.width().min(i.height())) / 2, (i.height() - i.width().min(i.height())) / 2, i.width().min(i.height()), i.width().min(i.height())).to_image(), side, side, image::imageops::FilterType::Triangle);
        let mut px = i.into_raw();
        // round, with a soft edge
        let r = side as f32 / 2.0;
        for y in 0..side {
            for x in 0..side {
                let d = ((x as f32 + 0.5 - r).powi(2) + (y as f32 + 0.5 - r).powi(2)).sqrt();
                let k = (r - d + 0.5).clamp(0.0, 1.0);
                let a = &mut px[((y * side + x) * 4 + 3) as usize];
                *a = (*a as f32 * k).round() as u8;
            }
        }
        crate::canvas::Img { w: side, h: side, px }
    });
    *c = Some((stamp, img.clone()));
    img
}

/// The saved picture as a texture, loaded the first time it's needed.
#[derive(Default)]
pub struct AvatarCache {
    tried: bool,
    tex: Option<TextureHandle>,
}

/// The "Profile picture" window.
pub struct AvatarEditor {
    img: Option<RgbaImage>,
    tex: Option<TextureHandle>,
    /// Picture pixel that sits in the middle of the frame.
    center: Vec2,
    /// 1 = the short side just fills the frame.
    zoom: f32,
    picker: Option<Receiver<Option<PathBuf>>>,
    loading: Option<Receiver<Result<RgbaImage, String>>>,
    msg: Option<(bool, String)>,
    /// "Remove picture" was clicked once; the next click removes it.
    confirm_remove: bool,
}

impl AvatarEditor {
    fn new() -> Self {
        Self { img: None, tex: None, center: Vec2::ZERO, zoom: 1.0, picker: None, loading: None, msg: None, confirm_remove: false }
    }

    fn set_image(&mut self, ctx: &egui::Context, img: RgbaImage, crop: Option<[f32; 3]>) {
        let (w, h) = (img.width() as f32, img.height() as f32);
        self.center = vec2(w / 2.0, h / 2.0);
        self.zoom = 1.0;
        if let Some([x, y, z]) = crop {
            self.center = vec2(x, y);
            self.zoom = z.clamp(1.0, MAX_ZOOM);
        }
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
        self.tex = Some(ctx.load_texture("avatar-edit", ci, TextureOptions::LINEAR));
        self.img = Some(img);
        self.clamp();
    }

    /// Screen points per picture pixel.
    fn scale(&self) -> f32 {
        match &self.img {
            Some(i) => FRAME / i.width().min(i.height()).max(1) as f32 * self.zoom,
            None => 1.0,
        }
    }

    /// Keep the frame inside the picture.
    fn clamp(&mut self) {
        let Some(i) = &self.img else { return };
        let (w, h) = (i.width() as f32, i.height() as f32);
        self.zoom = self.zoom.clamp(1.0, MAX_ZOOM);
        let half = FRAME / 2.0 / self.scale();
        self.center.x = self.center.x.clamp(half.min(w / 2.0), (w - half).max(w / 2.0));
        self.center.y = self.center.y.clamp(half.min(h / 2.0), (h - half).max(h / 2.0));
    }

    /// Zoom around the middle of the frame.
    fn zoom_to(&mut self, z: f32) {
        self.zoom = z;
        self.clamp();
    }

    fn rotate(&mut self, ctx: &egui::Context) {
        if let Some(i) = self.img.take() {
            let r = image::imageops::rotate90(&i);
            self.set_image(ctx, r, None);
        }
    }

    /// The square that ends up in the saved picture, in picture pixels.
    fn crop_rect(&self) -> Option<(u32, u32, u32)> {
        let i = self.img.as_ref()?;
        let side = (FRAME / self.scale()).round().max(1.0) as u32;
        let side = side.min(i.width()).min(i.height());
        let x = (self.center.x - side as f32 / 2.0).round().clamp(0.0, (i.width() - side) as f32) as u32;
        let y = (self.center.y - side as f32 / 2.0).round().clamp(0.0, (i.height() - side) as f32) as u32;
        Some((x, y, side))
    }

    fn render(&self) -> Option<RgbaImage> {
        let i = self.img.as_ref()?;
        let (x, y, side) = self.crop_rect()?;
        let part = image::imageops::crop_imm(i, x, y, side, side).to_image();
        Some(image::imageops::resize(&part, OUT, OUT, image::imageops::FilterType::CatmullRom))
    }
}

/// Read a picture file: PNG and GIF (first frame) through `image`, JPG
/// through `jpeg-decoder` (turned upright by its EXIF rotation).
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    let img = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        decode_jpeg(bytes)?
    } else if bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"GIF8") {
        image::load_from_memory(bytes).map_err(|_| "That picture looks damaged".to_string())?.to_rgba8()
    } else {
        return Err("Pick a PNG, JPG or GIF picture".into());
    };
    if img.width() < 16 || img.height() < 16 {
        return Err("That picture is too small".into());
    }
    let long = img.width().max(img.height());
    Ok(if long > MAX_SRC {
        let k = MAX_SRC as f32 / long as f32;
        let (w, h) = (((img.width() as f32 * k).round() as u32).max(1), ((img.height() as f32 * k).round() as u32).max(1));
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    })
}

fn decode_jpeg(bytes: &[u8]) -> Result<RgbaImage, String> {
    let bad = || "That JPG looks damaged".to_string();
    let mut d = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let px = d.decode().map_err(|_| bad())?;
    let info = d.info().ok_or_else(bad)?;
    let (w, h) = (info.width as u32, info.height as u32);
    let n = (w * h) as usize;
    let mut out = Vec::with_capacity(n * 4);
    use jpeg_decoder::PixelFormat as F;
    match info.pixel_format {
        F::L8 => px.iter().take(n).for_each(|&v| out.extend_from_slice(&[v, v, v, 255])),
        F::L16 => px.chunks_exact(2).take(n).for_each(|c| out.extend_from_slice(&[c[0], c[0], c[0], 255])),
        F::RGB24 => px.chunks_exact(3).take(n).for_each(|c| out.extend_from_slice(&[c[0], c[1], c[2], 255])),
        F::CMYK32 => px.chunks_exact(4).take(n).for_each(|c| {
            let k = 1.0 - c[3] as f32 / 255.0;
            let f = |v: u8| ((1.0 - v as f32 / 255.0) * k * 255.0).round() as u8;
            out.extend_from_slice(&[f(c[0]), f(c[1]), f(c[2]), 255]);
        }),
    }
    let img = RgbaImage::from_raw(w, h, out).ok_or_else(bad)?;
    Ok(match d.exif_data().and_then(exif_orientation) {
        Some(3) => image::imageops::rotate180(&img),
        Some(6) => image::imageops::rotate90(&img),
        Some(8) => image::imageops::rotate270(&img),
        _ => img,
    })
}

/// The EXIF "Orientation" value (1–8), if there is one.
fn exif_orientation(t: &[u8]) -> Option<u16> {
    let le = match t.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |o: usize| -> Option<u16> {
        let b = t.get(o..o + 2)?;
        Some(if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) })
    };
    let u32_at = |o: usize| -> Option<u32> {
        let b = t.get(o..o + 4)?;
        Some(if le { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) } else { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) })
    };
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    (0..count.min(64)).find_map(|k| {
        let e = ifd + 2 + k * 12;
        (u16_at(e)? == 0x0112).then(|| u16_at(e + 8)).flatten()
    })
}

fn load_file(path: &Path) -> Result<RgbaImage, String> {
    let size = std::fs::metadata(path).map(|m| m.len()).map_err(|_| "Couldn't open that file".to_string())?;
    if size > MAX_BYTES {
        return Err("That picture is too big (over 60 MB)".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Couldn't open that file".to_string())?;
    decode(&bytes)
}

fn png_bytes(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

/// Write through a temporary file so a crash can't leave half a picture.
fn write_safely(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("png.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// A picture drawn as a circle (a triangle fan with the texture on it).
pub fn circle_image(p: &egui::Painter, tex: TextureId, c: Pos2, r: f32, uv: Rect) {
    let n = 72;
    let mut mesh = egui::Mesh::with_texture(tex);
    let uv_at = |d: Vec2| pos2(uv.center().x + d.x * uv.width() / 2.0, uv.center().y + d.y * uv.height() / 2.0);
    mesh.vertices.push(egui::epaint::Vertex { pos: c, uv: uv.center(), color: Color32::WHITE });
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        let d = vec2(a.cos(), a.sin());
        mesh.vertices.push(egui::epaint::Vertex { pos: c + d * r, uv: uv_at(d), color: Color32::WHITE });
    }
    for k in 0..n {
        mesh.add_triangle(0, 1 + k, 1 + (k + 1) % n);
    }
    p.add(egui::Shape::mesh(mesh));
}

/// The whole-picture uv rect.
pub fn full_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0))
}

/// Darken everything in `outer` except a circle (the crop frame's surround).
fn dim_outside(p: &egui::Painter, c: Pos2, r: f32, outer: f32, col: Color32) {
    let n = 96u32;
    let mut mesh = egui::Mesh::default();
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        let d = vec2(a.cos(), a.sin());
        mesh.colored_vertex(c + d * r, col);
        mesh.colored_vertex(c + d * outer, col);
    }
    for k in 0..n {
        let (i0, o0, i1, o1) = (2 * k, 2 * k + 1, 2 * ((k + 1) % n), 2 * ((k + 1) % n) + 1);
        mesh.add_triangle(i0, o0, o1);
        mesh.add_triangle(i0, o1, i1);
    }
    p.add(egui::Shape::mesh(mesh));
}

impl App {
    /// The saved profile picture, if there is one.
    pub(super) fn avatar_tex(&mut self, ctx: &egui::Context) -> Option<TextureId> {
        let c = &mut self.setup.avatar;
        if !c.tried {
            c.tried = true;
            c.tex = std::fs::read(file()).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                let i = i.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw());
                ctx.load_texture("avatar", ci, TextureOptions::LINEAR)
            });
        }
        c.tex.as_ref().map(|t| t.id())
    }

    /// Open the "Profile picture" window: on the saved picture when there is
    /// one, otherwise straight to choosing a file.
    pub(super) fn open_avatar_editor(&mut self, ctx: &egui::Context) {
        let mut ed = AvatarEditor::new();
        let saved = self.avatar_tex(ctx).is_some();
        let src = source_file();
        if saved && src.is_file() {
            if let Ok(img) = load_file(&src) {
                ed.set_image(ctx, img, self.cfg.avatar_crop);
            }
        } else if saved {
            if let Ok(img) = load_file(&file()) {
                ed.set_image(ctx, img, None);
            }
        }
        self.setup.avatar_edit = Some(ed);
    }

    /// A picture in a ring, with the level badge on its corner when `level`
    /// is given; falls back to the plain level badge without a picture.
    pub(super) fn draw_avatar(&mut self, ctx: &egui::Context, p: &egui::Painter, c: Pos2, r: f32, level: u32, ring: Color32) -> bool {
        // a battle pass frame, when one is worn, takes the place of the plain ring
        let frame = self.cfg.frame.clone();
        match self.avatar_tex(ctx) {
            Some(tex) => {
                p.circle_filled(c + vec2(0.0, r * 0.06), r * 1.06, Color32::from_black_alpha(80));
                circle_image(p, tex, c, r, full_uv());
                if frame.is_empty() {
                    p.circle_stroke(c, r, Stroke::new((r * 0.07).max(2.0), ring));
                    p.circle_stroke(c, r + (r * 0.035).max(1.0) + 0.5, Stroke::new(1.0f32, theme::OUTLINE.gamma_multiply(0.6)));
                } else {
                    self.draw_frame_ui(ctx, p, &frame, c, r);
                }
                theme::level_badge(p, c + vec2(r * 0.72, r * 0.70), (r * 0.36).max(8.0), level);
                true
            }
            None => {
                theme::level_badge(p, c, r * 0.94, level);
                if !frame.is_empty() {
                    self.draw_frame_ui(ctx, p, &frame, c, r);
                }
                false
            }
        }
    }

    pub(super) fn avatar_window(&mut self, ctx: &egui::Context) {
        let Some(mut ed) = self.setup.avatar_edit.take() else { return };

        // a file box that closed, a picture that finished loading
        if let Some(pick) = ed.picker.as_ref().and_then(|rx| rx.try_recv().ok()) {
            ed.picker = None;
            if let Some(path) = pick {
                ed.loading = Some(start_load(path, ctx.clone()));
            }
        }
        // a picture dropped on the app while the window is open
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
        if let Some(path) = dropped {
            ed.loading = Some(start_load(path, ctx.clone()));
        }
        if let Some(done) = ed.loading.as_ref().and_then(|rx| rx.try_recv().ok()) {
            ed.loading = None;
            match done {
                Ok(img) => {
                    ed.set_image(ctx, img, None);
                    ed.msg = None;
                }
                Err(e) => ed.msg = Some((false, e)),
            }
        }
        let hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());

        let level = self.cfg.level_seen.max(1);
        let has_saved = self.avatar_tex(ctx).is_some();
        let saved_tex = self.setup.avatar.tex.as_ref().map(|t| t.id());
        let name = if self.cfg.trainer_name.trim().is_empty() { "Trainer".to_string() } else { self.cfg.trainer_name.clone() };
        let (_, main, _) = crate::progress::badge_metal(level);
        let ring = Color32::from_rgb(main[0], main[1], main[2]);

        // dim the app behind the window
        ctx.layer_painter(egui::LayerId::new(egui::Order::PanelResizeLine, egui::Id::new("avatar-dim")))
            .rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha(120));

        let mut open = true;
        let (mut choose, mut save, mut remove, mut cancel) = (false, false, false, false);
        egui::Window::new(RichText::new("📷  Profile picture").font(ch::bold(15.0)))
            .id(egui::Id::new("avatar-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(STAGE + 14.0 + 210.0);
                ui.label(RichText::new("Drag to move, zoom to resize. Only what's inside the circle is used.").size(kit::fs(12.5)).color(TEXT()));
                ui.label(RichText::new("The picture stays on this PC.").size(kit::fs(11.5)).color(MUTED()));
                ui.add_space(10.0);
                ui.horizontal_top(|ui| {
                    // ---- the crop stage ----
                    ui.vertical(|ui| {
                        let (stage, resp) = ui.allocate_exact_size(vec2(STAGE, STAGE), Sense::click_and_drag());
                        let p = ui.painter_at(stage);
                        let fc = stage.center();
                        p.rect_filled(stage, 16.0, theme::blend(kit::CARD(), TEXT(), 0.06));
                        if let (Some(tex), Some(img)) = (&ed.tex, &ed.img) {
                            let s = ed.scale();
                            let size = vec2(img.width() as f32, img.height() as f32) * s;
                            let r = Rect::from_min_size(fc - ed.center * s, size);
                            p.image(tex.id(), r, full_uv(), Color32::WHITE);
                            dim_outside(&p, fc, FRAME / 2.0, STAGE, Color32::from_black_alpha(150));
                            p.circle_stroke(fc, FRAME / 2.0, Stroke::new(2.0f32, Color32::WHITE));
                            // rule-of-thirds guides while moving
                            if resp.dragged() {
                                let g = Stroke::new(1.0f32, Color32::from_white_alpha(70));
                                let (x0, y0, d) = (fc.x - FRAME / 2.0, fc.y - FRAME / 2.0, FRAME / 3.0);
                                for k in 1..3 {
                                    p.line_segment([pos2(x0 + d * k as f32, y0 + 8.0), pos2(x0 + d * k as f32, y0 + FRAME - 8.0)], g);
                                    p.line_segment([pos2(x0 + 8.0, y0 + d * k as f32), pos2(x0 + FRAME - 8.0, y0 + d * k as f32)], g);
                                }
                            }
                            if resp.dragged() {
                                ed.center -= resp.drag_delta() / s;
                                ed.clamp();
                            }
                            if resp.hovered() {
                                let wheel = ui.input(|i| i.smooth_scroll_delta.y);
                                if wheel != 0.0 {
                                    ed.zoom_to(ed.zoom * (wheel * 0.003).exp());
                                }
                            }
                            resp.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text("Drag to move · scroll to zoom");
                        } else {
                            let dash = if resp.hovered() { kit::primary() } else { theme::blend(kit::CARD(), TEXT(), 0.28) };
                            p.circle_stroke(fc, FRAME / 2.0, Stroke::new(2.0f32, dash));
                            p.text(fc - vec2(0.0, 34.0), Align2::CENTER_CENTER, "🖼", egui::FontId::proportional(40.0), MUTED());
                            let busy = ed.loading.is_some() || ed.picker.is_some();
                            let t = if ed.loading.is_some() { "Opening the picture…" } else { "Choose a picture" };
                            p.text(fc + vec2(0.0, 14.0), Align2::CENTER_CENTER, t, ch::bold(14.0), TEXT());
                            p.text(fc + vec2(0.0, 36.0), Align2::CENTER_CENTER, "PNG, JPG or GIF · or drop one here", ch::reg(11.5), MUTED());
                            if !busy && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                choose = true;
                            }
                        }
                        if hovering_file {
                            p.rect_filled(stage, 16.0, ch::faint(kit::primary(), 0.25));
                            p.rect_stroke(stage.shrink(2.0), 14.0, Stroke::new(2.0f32, kit::primary()));
                            p.text(fc, Align2::CENTER_CENTER, "Drop to use this picture", ch::bold(15.0), Color32::WHITE);
                        }
                        ui.add_space(8.0);
                        // zoom and rotate
                        ui.add_enabled_ui(ed.img.is_some(), |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.spacing_mut().slider_width = STAGE - 196.0;
                                let sq = vec2(26.0, 26.0);
                                let tool = |t: &str| egui::Button::new(RichText::new(t).size(14.0)).fill(kit::btn_fill()).min_size(sq);
                                if ui.add(tool("−")).on_hover_text("Zoom out").clicked() {
                                    ed.zoom_to(ed.zoom / 1.25);
                                }
                                let mut z = ed.zoom;
                                if ui.add(egui::Slider::new(&mut z, 1.0..=MAX_ZOOM).logarithmic(true).show_value(false)).on_hover_text("Zoom (or scroll on the picture)").changed() {
                                    ed.zoom_to(z);
                                }
                                if ui.add(tool("+")).on_hover_text("Zoom in").clicked() {
                                    ed.zoom_to(ed.zoom * 1.25);
                                }
                                ui.label(RichText::new(format!("{:.1}×", ed.zoom)).size(kit::fs(11.5)).color(MUTED()));
                                if ui.add(tool("↻")).on_hover_text("Turn the picture a quarter turn").clicked() {
                                    ed.rotate(ctx);
                                }
                                if ui.add(egui::Button::new(RichText::new("Reset").size(kit::fs(12.0))).fill(kit::btn_fill()).min_size(vec2(0.0, 26.0))).on_hover_text("The whole picture again, centred").clicked() {
                                    if let Some(i) = &ed.img {
                                        ed.center = vec2(i.width() as f32, i.height() as f32) / 2.0;
                                    }
                                    ed.zoom_to(1.0);
                                }
                            });
                        });
                    });
                    ui.add_space(14.0);
                    // ---- previews ----
                    ui.vertical(|ui| {
                        ui.set_width(210.0);
                        ui.label(RichText::new("PREVIEW").font(ch::bold(11.0)).color(MUTED()));
                        ui.add_space(6.0);
                        let crop = ed.crop_rect().zip(ed.img.as_ref()).map(|((x, y, s), i)| {
                            let (w, h) = (i.width() as f32, i.height() as f32);
                            Rect::from_min_size(pos2(x as f32 / w, y as f32 / h), vec2(s as f32 / w, s as f32 / h))
                        });
                        let shown = match (&ed.tex, crop) {
                            (Some(t), Some(uv)) => Some((t.id(), uv)),
                            _ => saved_tex.map(|t| (t, full_uv())),
                        };
                        // profile size
                        let (r, _) = ui.allocate_exact_size(vec2(210.0, 132.0), Sense::hover());
                        let p = ui.painter_at(r.expand(4.0));
                        p.rect_filled(r, 14.0, kit::CARD());
                        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, kit::LINE()));
                        let c = r.center();
                        let rad = 50.0;
                        match shown {
                            Some((tex, uv)) => {
                                circle_image(&p, tex, c, rad, uv);
                                p.circle_stroke(c, rad, Stroke::new(3.5f32, ring));
                                theme::level_badge(&p, c + vec2(rad * 0.72, rad * 0.70), rad * 0.36, level);
                            }
                            None => theme::level_badge(&p, c, rad * 0.94, level),
                        }
                        ui.add_space(3.0);
                        ui.label(RichText::new("Profile page").size(kit::fs(11.5)).color(MUTED()));
                        ui.add_space(6.0);
                        // sidebar size
                        let (r, _) = ui.allocate_exact_size(vec2(210.0, 58.0), Sense::hover());
                        let p = ui.painter_at(r.expand(2.0));
                        p.rect_filled(r, 12.0, theme::blend(kit::SIDEBAR(), TEXT(), 0.04));
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(kit::SIDEBAR(), TEXT(), 0.08)));
                        let c = r.left_center() + vec2(28.0, 0.0);
                        match shown {
                            Some((tex, uv)) => {
                                circle_image(&p, tex, c, 17.0, uv);
                                p.circle_stroke(c, 17.0, Stroke::new(2.0f32, ring));
                                theme::level_badge(&p, c + vec2(13.0, 12.0), 8.0, level);
                            }
                            None => theme::level_badge(&p, c, 17.0, level),
                        }
                        let pc = p.with_clip_rect(r.shrink(4.0));
                        pc.text(pos2(c.x + 28.0, r.center().y - 8.0), Align2::LEFT_CENTER, &name, crate::fonts::sans_bold_font(13.5), TEXT());
                        pc.text(pos2(c.x + 28.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("Level {level}"), crate::fonts::sans_font(12.0), MUTED());
                        ui.add_space(4.0);
                        ui.label(RichText::new("Sidebar").size(kit::fs(11.5)).color(MUTED()));
                        ui.add_space(10.0);
                        let label = if ed.img.is_some() { "🖼  Choose another…" } else { "🖼  Choose a picture…" };
                        let busy = ed.picker.is_some() || ed.loading.is_some();
                        if ui.add_enabled(!busy, egui::Button::new(label).fill(kit::btn_fill()).min_size(vec2(210.0, 30.0))).clicked() {
                            choose = true;
                        }
                        if has_saved {
                            ui.add_space(4.0);
                            let (t, tip) = if ed.confirm_remove {
                                ("🗑  Click again to remove", "Your picture is deleted from this PC and the level badge comes back")
                            } else {
                                ("🗑  Remove picture", "Go back to the level badge")
                            };
                            let fill = if ed.confirm_remove { ch::faint(theme::HP_RED, 0.18) } else { kit::btn_fill() };
                            let b = ui.add(egui::Button::new(RichText::new(t).color(theme::HP_RED)).fill(fill).min_size(vec2(210.0, 30.0))).on_hover_text(tip);
                            if b.clicked() {
                                if ed.confirm_remove {
                                    remove = true;
                                } else {
                                    ed.confirm_remove = true;
                                }
                            }
                            if ed.confirm_remove && !b.hovered() && ui.input(|i| i.pointer.any_click()) {
                                ed.confirm_remove = false;
                            }
                        }
                    });
                });
                if let Some((ok, m)) = &ed.msg {
                    ui.add_space(8.0);
                    kit::pill(ui, m, if *ok { PillKind::Ok } else { PillKind::Bad });
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let b = egui::Button::new(RichText::new("  ✔  Save picture  ").font(ch::bold(13.5)).color(kit::on_primary()));
                    if ui.add_enabled_ui(ed.img.is_some(), |ui| kit::accent_button(ui, b, 6.0)).inner.clicked() {
                        save = true;
                    }
                    if ui.add(egui::Button::new("  Cancel  ").fill(kit::btn_fill()).min_size(vec2(0.0, 30.0))).clicked() {
                        cancel = true;
                    }
                    if ed.img.is_some() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("Saved at {OUT} × {OUT}")).size(kit::fs(11.0)).color(MUTED()));
                        });
                    }
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if choose && ed.picker.is_none() {
            let start = dirs::picture_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("."));
            ed.picker = Some(crate::share::pick_image(start, ctx.clone()));
        }
        if save {
            match self.save_avatar(&ed) {
                Ok(()) => {
                    return; // closes the window; the new picture shows right away
                }
                Err(e) => ed.msg = Some((false, format!("Couldn't save the picture: {e}"))),
            }
        }
        if remove {
            let _ = std::fs::remove_file(file());
            let _ = std::fs::remove_file(source_file());
            self.cfg.avatar_crop = None;
            self.dirty = true;
            self.setup.avatar = AvatarCache::default();
            return;
        }
        if open && !cancel {
            if ed.picker.is_some() || ed.loading.is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            self.setup.avatar_edit = Some(ed);
        }
    }

    fn save_avatar(&mut self, ed: &AvatarEditor) -> Result<(), String> {
        let out = ed.render().ok_or("no picture")?;
        if let Some(src) = &ed.img {
            // the copy for "Adjust" (it may have been turned); the profile
            // picture still saves if this fails
            let _ = png_bytes(src).and_then(|b| write_safely(&source_file(), &b));
        }
        write_safely(&file(), &png_bytes(&out)?)?;
        self.cfg.avatar_crop = Some([ed.center.x, ed.center.y, ed.zoom]);
        self.dirty = true;
        self.setup.avatar = AvatarCache::default();
        Ok(())
    }
}

fn start_load(path: PathBuf, ctx: egui::Context) -> Receiver<Result<RgbaImage, String>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(load_file(&path));
        ctx.request_repaint();
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(w: u32, h: u32) -> AvatarEditor {
        let mut e = AvatarEditor::new();
        e.img = Some(RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 255])));
        e.center = vec2(w as f32 / 2.0, h as f32 / 2.0);
        e
    }

    #[test]
    fn crop_stays_inside_the_picture() {
        let mut e = editor(800, 400);
        assert_eq!(e.crop_rect(), Some((200, 0, 400)));
        e.center = vec2(-500.0, 9999.0);
        e.clamp();
        assert_eq!(e.crop_rect(), Some((0, 0, 400)));
        e.zoom_to(4.0);
        let (x, y, s) = e.crop_rect().unwrap();
        assert_eq!(s, 100);
        assert!(x + s <= 800 && y + s <= 400);
        e.zoom_to(100.0);
        assert_eq!(e.zoom, MAX_ZOOM);
    }

    #[test]
    fn renders_a_square_png() {
        let e = editor(640, 480);
        let out = e.render().unwrap();
        assert_eq!((out.width(), out.height()), (OUT, OUT));
        let png = png_bytes(&out).unwrap();
        assert_eq!(decode(&png).unwrap().dimensions(), (OUT, OUT));
    }

    #[test]
    fn big_pictures_are_scaled_down_and_junk_is_refused() {
        let big = RgbaImage::from_pixel(3000, 1500, image::Rgba([1, 2, 3, 255]));
        assert_eq!(decode(&png_bytes(&big).unwrap()).unwrap().dimensions(), (MAX_SRC, MAX_SRC / 2));
        assert!(decode(b"not a picture").is_err());
    }

    #[test]
    fn reads_exif_rotation() {
        // little-endian TIFF header, one IFD entry: Orientation = 6
        let mut t = b"II*\0\x08\0\0\0".to_vec();
        t.extend_from_slice(&[1, 0, 0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0]);
        assert_eq!(exif_orientation(&t), Some(6));
        assert_eq!(exif_orientation(b"MM"), None);
    }
}
