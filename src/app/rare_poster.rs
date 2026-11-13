//! Today's rare grass as an infographic (the Sparkle Map's share image,
//! now drawn from the Map's own tiles): opened from the Map, saved as a PNG
//! or copied for Discord. Drawing is in `crate::poster`.

use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::App;
use crate::canvas::{Canvas, Img};
use crate::gamemap::{self, GameMap, CHUNK};
use crate::poster::{self, PosterArea, PosterData, PosterLabel, PosterMap, PosterMode, PosterMon, PosterStyle};
use crate::theme;
use eframe::egui::{self, RichText, TextureHandle, TextureOptions, Vec2};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// The share window's state.
pub(super) struct PosterView {
    pub mode: PosterMode,
    job: Option<(PosterMode, Receiver<Canvas>)>,
    image: Option<(PosterMode, Canvas)>,
    tex: Option<TextureHandle>,
    msg: Option<(bool, String)>,
    saved: Option<PathBuf>,
    picker: Option<(Vec<u8>, Receiver<Option<PathBuf>>)>,
}

/// A spot's tile on the Sinnoh map (side areas through their place on it).
fn world_tile(map: &GameMap, matrix: u16, x: usize, y: usize) -> Option<(f32, f32)> {
    if matrix == 0 {
        return Some((x as f32 + 0.5, y as f32 + 0.5));
    }
    let o = map.overlays.iter().find(|o| o.matrix == matrix && o.x.is_some() && o.y.is_some())?;
    Some(((o.x? + x as i32) as f32 + 0.5, (o.y? + y as i32) as f32 + 0.5))
}

/// The Sparkle Map's drawing of Sinnoh (rendered from its own map data),
/// 80 pixels to a 32-tile map square, with open sea around it.
const SINNOH_ART: &[u8] = include_bytes!("../sinnoh_art.png");

/// City and town names as the Sparkle Map places them: (name, x, y) in tiles
/// at the top middle of the place, and whether it's a city (bigger text).
const PLACE_LABELS: &[(&str, f32, f32, bool)] = &[
    ("Snowpoint", 368.0, 192.0, true),
    ("Survival Area", 656.0, 320.0, false),
    ("Fight Area", 640.0, 416.0, true),
    ("Resort Area", 816.0, 448.0, false),
    ("Eterna", 320.0, 512.0, true),
    ("Celestic", 464.0, 512.0, false),
    ("Pokémon League", 848.0, 544.0, true),
    ("Veilstone", 704.0, 576.0, true),
    ("Floaroma", 176.0, 608.0, false),
    ("Solaceon", 576.0, 640.0, false),
    ("Hearthome", 480.0, 672.0, true),
    ("Canalave", 48.0, 704.0, true),
    ("Jubilife", 160.0, 736.0, true),
    ("Oreburgh", 288.0, 736.0, true),
    ("Sunyshore", 864.0, 736.0, true),
    ("Pastoria", 608.0, 800.0, true),
    ("Sandgem", 176.0, 832.0, false),
    ("Twinleaf", 112.0, 864.0, false),
];

fn place_labels() -> Vec<PosterLabel> {
    PLACE_LABELS.iter().map(|&(name, x, y, city)| PosterLabel { name: name.to_string(), x, y, city }).collect()
}

/// The map picture, decoded once.
fn sinnoh_art() -> Option<PosterMap> {
    static ART: std::sync::OnceLock<Option<Img>> = std::sync::OnceLock::new();
    let img = ART
        .get_or_init(|| {
            let i = image::load_from_memory(SINNOH_ART).ok()?.to_rgba8();
            let (w, h) = (i.width(), i.height());
            Some(Img { w, h, px: i.into_raw() })
        })
        .clone()?;
    // The drawing spans map squares -11.5..37.5 across and -1.5..35.5 down
    // (the Sparkle Map's view, 6 and 3 squares of sea added each side);
    // squares -5.5..31.5 by 1.5..32.5 fill the height of the top.
    let sq = CHUNK as f32;
    Some(PosterMap {
        img,
        x0: -11.5 * sq,
        y0: -1.5 * sq,
        px_per_tile: 80.0 / sq,
        fit: (-5.5 * sq, 1.5 * sq, 37.0 * sq, 31.0 * sq),
    })
}

/// A small copy of the poster for the window.
fn preview(c: &Canvas, ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::RgbaImage::from_raw(c.w, c.h, c.px.clone())?;
    let w = 900u32.min(c.w);
    let h = ((c.h as f32 * w as f32 / c.w as f32).round() as u32).max(1);
    let small = image::imageops::thumbnail(&img, w, h);
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], small.as_raw());
    Some(ctx.load_texture("rare-poster-preview", ci, TextureOptions::LINEAR))
}

impl App {
    /// Open the share window (and start drawing the full infographic).
    pub(super) fn open_rare_poster(&mut self, ctx: &egui::Context) {
        if self.mapui.poster.is_none() {
            self.mapui.poster = Some(PosterView {
                mode: PosterMode::Full,
                job: None,
                image: None,
                tex: None,
                msg: None,
                saved: None,
                picker: None,
            });
        }
        let mode = self.mapui.poster.as_ref().map(|v| v.mode).unwrap_or_default();
        self.start_rare_poster(mode, ctx);
    }

    /// Draw the poster in the background.
    fn start_rare_poster(&mut self, mode: PosterMode, ctx: &egui::Context) {
        let Some(map) = self.gamemap.as_ref().and_then(|m| m.as_ref().ok()).cloned() else { return };
        // Every tile a Pokémon was met on today (yours plus the shared
        // list). Scans with no Pokémon and species we can't name are left
        // out, so every pin on the poster has a Pokémon.
        self.mapui.refresh_rare();
        let rare = self.mapui.rare.clone();
        struct Draft {
            name: String,
            tiles: Vec<(f32, f32, usize)>,
            mons: Vec<(String, String, usize)>,
        }
        let mut drafts: Vec<Draft> = Vec::new();
        let mut slugs: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for r in &rare {
            let Some(sp) = r.species else { continue };
            let Some(species) = self.rare_species(&map, sp) else { continue };
            if !seen.insert((r.matrix, r.x, r.y, species.slug.clone())) {
                continue;
            }
            // start the sprite download now; the drawing waits a moment for it
            let _ = self.sprites.get(&species, false);
            if !slugs.contains(&species.slug) {
                slugs.push(species.slug.clone());
            }
            let place = map.rare_zone(r.matrix, r.x, r.y).map(|z| self.zone_label(&map, z)).unwrap_or_else(|| "Outdoors".into());
            let d = match drafts.iter().position(|d| d.name == place) {
                Some(i) => &mut drafts[i],
                None => {
                    drafts.push(Draft { name: place, tiles: Vec::new(), mons: Vec::new() });
                    drafts.last_mut().unwrap()
                }
            };
            let mi = match d.mons.iter().position(|m| m.0 == species.slug) {
                Some(i) => {
                    d.mons[i].2 += 1;
                    i
                }
                None => {
                    d.mons.push((species.slug.clone(), species.name.to_string(), 1));
                    d.mons.len() - 1
                }
            };
            if let Some((x, y)) = world_tile(&map, r.matrix, r.x, r.y) {
                d.tiles.push((x, y, mi));
            }
        }
        let labels = place_labels();
        let look = self.poster_look();
        let style = if look == crate::card::CardStyle::Classic { PosterStyle::Classic } else { PosterStyle::Anime };
        let theme = self.cfg.card.theme;
        let now = crate::stats::now_unix() as i64;
        let (reset, next) = (gamemap::last_reset(now), gamemap::next_reset(now));
        let (tx, rx) = channel();
        let ctx2 = ctx.clone();
        std::thread::Builder::new()
            .name("rare-poster".into())
            .spawn(move || {
                // wait (a little) for sprites still downloading
                let until = Instant::now() + Duration::from_secs(8);
                while Instant::now() < until && slugs.iter().any(|s| crate::sprites::cached_file(s, false).is_none()) {
                    std::thread::sleep(Duration::from_millis(200));
                }
                let areas: Vec<PosterArea> = drafts
                    .into_iter()
                    .map(|d| {
                        let mons = d
                            .mons
                            .into_iter()
                            .map(|(slug, name, spots)| PosterMon { sprite: crate::share::sprite_frame(&slug, false), key: slug, name, spots })
                            .collect();
                        PosterArea { name: d.name, mons, tiles: d.tiles }
                    })
                    .collect();
                let data = PosterData { areas, labels, map: sinnoh_art(), reset, next };
                let img = match crate::card::render_poster(&data, mode, look, theme, &crate::share::AbText::load()) {
                    Some(img) => img,
                    None => poster::render(&data, mode, style, &crate::poster_text::PosterFonts::load()),
                };
                let _ = tx.send(img);
                ctx2.request_repaint();
            })
            .ok();
        if let Some(v) = self.mapui.poster.as_mut() {
            v.mode = mode;
            v.job = Some((mode, rx));
            v.msg = None;
        }
    }

    /// The poster's look (older settings only had Sticker or Classic).
    fn poster_look(&self) -> crate::card::CardStyle {
        self.cfg.poster_look.unwrap_or(if self.cfg.poster_classic { crate::card::CardStyle::Classic } else { crate::card::CardStyle::Anime })
    }

    /// The share window, while it's open.
    pub(super) fn rare_poster_window(&mut self, ctx: &egui::Context) {
        let Some(v) = self.mapui.poster.as_mut() else { return };
        // a finished drawing
        let done = v.job.as_ref().and_then(|(m, rx)| rx.try_recv().ok().map(|img| (*m, img)));
        if let Some((mode, img)) = done {
            v.tex = preview(&img, ctx);
            v.image = Some((mode, img));
            v.job = None;
        } else if v.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
        // a "Save as" box that closed
        let picked = v.picker.as_ref().and_then(|(_, rx)| rx.try_recv().ok());
        if let Some(pick) = picked {
            let bytes = v.picker.take().map(|p| p.0).unwrap_or_default();
            if let Some(path) = pick {
                v.msg = Some(match crate::share::save_to(&bytes, &path, "png") {
                    Ok(p) => {
                        v.saved = Some(p.clone());
                        (true, format!("✔ Saved to {}", p.display()))
                    }
                    Err(e) => (false, format!("Couldn't save: {e}")),
                });
            }
        }

        let mut open = true;
        let mut redo: Option<PosterMode> = None;
        let mut look = self.cfg.poster_look.unwrap_or(if self.cfg.poster_classic { crate::card::CardStyle::Classic } else { crate::card::CardStyle::Anime });
        let mut copy = false;
        let mut save = false;
        let mut reveal: Option<PathBuf> = None;
        let busy = v.job.is_some();
        egui::Window::new("✨ Share today's rare grass")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("The infographic: today's rare grass on the map at the exact tiles, the reset time around the world, the top 3 and where to find everything.")
                        .size(kit::fs(12.0))
                        .color(MUTED()),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    for m in [PosterMode::Full, PosterMode::MapOnly, PosterMode::Lists] {
                        if ui.selectable_label(v.mode == m, RichText::new(m.label()).size(kit::fs(12.5))).on_hover_text(m.tip()).clicked() && v.mode != m {
                            redo = Some(m);
                        }
                    }
                    ui.separator();
                    kit::Dropdown::from_id_salt("poster-look")
                        .selected_text(RichText::new(format!("{} {}", look.icon(), look.name())).size(kit::fs(12.5)))
                        .show_ui(ui, |ui| {
                            for c in crate::card::CardStyle::ALL {
                                let tip = if c == crate::card::CardStyle::Classic { "The original sea-blue infographic" } else { c.about() };
                                if ui.selectable_label(look == c, format!("{} {}", c.icon(), c.name())).on_hover_text(tip).clicked() && look != c {
                                    look = c;
                                    redo = Some(v.mode);
                                }
                            }
                        })
                        .response
                        .on_hover_text("The poster's look");
                });
                ui.add_space(8.0);
                let shown = v.image.as_ref().is_some_and(|(m, _)| *m == v.mode);
                match (&v.tex, shown && !busy) {
                    (Some(tex), true) => {
                        let size = tex.size_vec2();
                        let w = 540.0f32;
                        let h = (size.y * w / size.x).min(560.0);
                        let w = size.x * h / size.y;
                        ui.vertical_centered(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::hover());
                            let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));
                            ui.painter().image(tex.id(), r, uv, egui::Color32::WHITE);
                        });
                    }
                    _ => {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Drawing the infographic…").size(kit::fs(12.0)).color(MUTED()));
                        });
                        ui.add_space(20.0);
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let ready = shown && !busy;
                    let b = egui::Button::new(RichText::new("  📋  Copy for Discord  ").family(kit::strong_family(13.5)).strong().color(kit::on_primary()));
                    if ui.add_enabled_ui(ready, |ui| kit::accent_button(ui, b, 6.0)).inner.on_hover_text("Then paste it in Discord with Ctrl+V").clicked() {
                        copy = true;
                    }
                    if ui.add_enabled(ready && v.picker.is_none(), egui::Button::new("  💾  Save image  ")).clicked() {
                        save = true;
                    }
                    if let Some(p) = &v.saved {
                        if ui.add(egui::Button::new(RichText::new("Show in folder").color(theme::LINK())).frame(false)).clicked() {
                            reveal = Some(p.clone());
                        }
                    }
                });
                if let Some((ok, m)) = &v.msg {
                    ui.add_space(6.0);
                    kit::pill(ui, m, if *ok { PillKind::Ok } else { PillKind::Bad });
                }
                ui.add_space(4.0);
                ui.label(RichText::new("Pokémon from your game log plus the shared list. New patches at midnight Central European time.").size(kit::fs(11.0)).color(TEXT().gamma_multiply(0.7)));
            });

        if let Some(p) = reveal {
            crate::share::reveal(&p);
        }
        if copy {
            if let Some(v) = self.mapui.poster.as_mut() {
                v.msg = Some(match v.image.as_ref().map(|(_, c)| crate::share::copy_image(c)) {
                    Some(Ok(())) => (true, "✔ Copied! Paste it into Discord with Ctrl+V".into()),
                    Some(Err(e)) => (false, format!("Couldn't copy: {e}")),
                    None => (false, "Still drawing, try again".into()),
                });
            }
        }
        if save {
            let folder = self.share_folder();
            let asking = self.cfg.share_ask;
            if let Some(v) = self.mapui.poster.as_mut() {
                if let Some((mode, c)) = v.image.as_ref() {
                    let name = format!("{}-{}", mode.file_tag(), poster::date_slug(gamemap::last_reset(crate::stats::now_unix() as i64)));
                    match crate::share::png_bytes(c) {
                        None => v.msg = Some((false, "Couldn't make the PNG".into())),
                        Some(bytes) if asking => {
                            let rx = crate::share::pick_save(folder, format!("{name}.png"), "png", ctx.clone());
                            v.picker = Some((bytes, rx));
                        }
                        Some(bytes) => {
                            v.msg = Some(match crate::share::save_bytes(&bytes, &name, "png", &folder) {
                                Ok(p) => {
                                    v.saved = Some(p.clone());
                                    (true, format!("✔ Saved to {}", p.display()))
                                }
                                Err(e) => (false, format!("Couldn't save: {e}")),
                            });
                        }
                    }
                }
            }
        }
        if look != self.poster_look() {
            self.cfg.poster_look = Some(look);
            self.cfg.poster_classic = look == crate::card::CardStyle::Classic;
            self.dirty = true;
        }
        if let Some(m) = redo {
            self.start_rare_poster(m, ctx);
        }
        if !open {
            self.mapui.poster = None;
        }
    }
}
