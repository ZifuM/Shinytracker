//! Downloads, caches and animates sprites from pokemondb.
//! Gen 1–5 use the Black/White animated GIFs; later Pokémon (no B/W sprite
//! exists) fall back to pokemondb's HOME renders.

use crate::config::cache_dir;
use crate::species::Species;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BASE: &str = "https://img.pokemondb.net/sprites";
/// msikma/pokesprite: Gen 8-style box sprites, used for Gen 6+ (no animated
/// Black/White sprite exists for them).
const POKESPRITE: &str = "https://raw.githubusercontent.com/msikma/pokesprite/master/pokemon-gen8";
/// Last Pokémon with an animated Black/White sprite (Genesect).
pub const LAST_ANIMATED_DEX: u32 = 649;

/// Where a sprite is cached. Gen 6+ use a new name so older cached images
/// from the previous fallback get replaced by pokesprite ones.
pub fn cache_path(slug: &str, shiny: bool, dex: u32) -> PathBuf {
    let kind = if shiny { "shiny" } else { "normal" };
    let file = if dex > LAST_ANIMATED_DEX { format!("{slug}.v2.img") } else { format!("{slug}.img") };
    cache_dir().join("sprites").join(kind).join(file)
}

/// The cached sprite file for a slug, if downloaded (either naming).
pub fn cached_file(slug: &str, shiny: bool) -> Option<PathBuf> {
    let kind = if shiny { "shiny" } else { "normal" };
    let dir = cache_dir().join("sprites").join(kind);
    [format!("{slug}.v2.img"), format!("{slug}.img")]
        .into_iter()
        .map(|f| dir.join(f))
        .find(|p| p.exists())
}

pub struct Anim {
    frames: Vec<(TextureHandle, u32)>,
    total_ms: u64,
    pub size: egui::Vec2,
    /// A still image (Gen 6+ box sprite / HOME art), trimmed to the Pokémon,
    /// rather than an animated Gen 1-5 sprite.
    pub still: bool,
}

impl Anim {
    pub fn frame(&self, t_ms: u64) -> &TextureHandle {
        if self.frames.len() == 1 || self.total_ms == 0 {
            return &self.frames[0].0;
        }
        let mut t = t_ms % self.total_ms;
        for (tex, d) in &self.frames {
            if t < *d as u64 {
                return tex;
            }
            t -= *d as u64;
        }
        &self.frames[0].0
    }
}

enum Slot {
    Loading,
    Ready(Anim),
    Failed,
}

struct Request {
    key: String,
    urls: Vec<String>,
    cache: PathBuf,
}

enum Loaded {
    Ok {
        key: String,
        frames: Vec<(ColorImage, u32)>,
    },
    Failed {
        key: String,
    },
}

pub struct SpriteStore {
    req_tx: Sender<Request>,
    res_rx: Receiver<Loaded>,
    slots: HashMap<String, Slot>,
}

impl SpriteStore {
    pub fn new(ctx: &egui::Context) -> Self {
        let (req_tx, req_rx) = channel::<Request>();
        let (res_tx, res_rx) = channel::<Loaded>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        for i in 0..2 {
            let rx = req_rx.clone();
            let tx = res_tx.clone();
            let ctx = ctx.clone();
            std::thread::Builder::new()
                .name(format!("sprite-loader-{i}"))
                .spawn(move || loop {
                    let req = match rx.lock().ok().and_then(|r| r.recv().ok()) {
                        Some(r) => r,
                        None => return,
                    };
                    let msg = match load(&req) {
                        Ok(frames) => Loaded::Ok {
                            key: req.key,
                            frames,
                        },
                        Err(_) => Loaded::Failed { key: req.key },
                    };
                    let _ = tx.send(msg);
                    ctx.request_repaint();
                })
                .expect("spawn sprite loader");
        }
        Self {
            req_tx,
            res_rx,
            slots: HashMap::new(),
        }
    }

    /// Turn finished downloads into GPU textures. Call once per frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.res_rx.try_recv() {
            match msg {
                Loaded::Ok { key, frames } => {
                    let size = frames
                        .first()
                        .map(|(img, _)| egui::vec2(img.size[0] as f32, img.size[1] as f32))
                        .unwrap_or(egui::vec2(1.0, 1.0));
                    let total_ms = frames.iter().map(|(_, d)| *d as u64).sum();
                    let still = frames.len() == 1;
                    // big HOME artwork is scaled down: smooth it; pixel sprites stay crisp
                    let opts = if still && size.x.max(size.y) > 160.0 {
                        TextureOptions::LINEAR
                    } else {
                        TextureOptions::NEAREST
                    };
                    let frames = frames
                        .into_iter()
                        .enumerate()
                        .map(|(i, (img, d))| {
                            (
                                ctx.load_texture(
                                    format!("{key}#{i}"),
                                    img,
                                    opts,
                                ),
                                d,
                            )
                        })
                        .collect();
                    self.slots.insert(
                        key,
                        Slot::Ready(Anim {
                            frames,
                            total_ms,
                            size,
                            still,
                        }),
                    );
                }
                Loaded::Failed { key } => {
                    self.slots.insert(key, Slot::Failed);
                }
            }
        }
    }

    /// The sprite if it's ready; otherwise starts loading it and returns None.
    pub fn get(&mut self, sp: &Species, shiny: bool) -> Option<&Anim> {
        let kind = if shiny { "shiny" } else { "normal" };
        let key = format!("{kind}/{}", sp.slug);
        if !self.slots.contains_key(&key) {
            let slugs = sp.sprite_slugs();
            let mut urls = Vec::new();
            if sp.dex <= LAST_ANIMATED_DEX {
                // Gen 1-5: the animated Black/White sprites come first.
                for s in &slugs {
                    urls.push(format!("{BASE}/black-white/anim/{kind}/{s}.gif"));
                }
            } else {
                // Gen 6+: pokesprite box sprites (Gen 6-8 and Legends: Arceus).
                let folder = if shiny { "shiny" } else { "regular" };
                for s in &slugs {
                    urls.push(format!("{POKESPRITE}/{folder}/{s}.png"));
                }
            }
            // Last resort (and Gen 9, which pokesprite doesn't cover).
            for s in &slugs {
                urls.push(format!("{BASE}/home/{kind}/1x/{s}.png"));
            }
            for s in &slugs {
                urls.push(format!("{BASE}/home/{kind}/{s}.png"));
            }
            let cache = cache_path(&sp.slug, shiny, sp.dex);
            self.slots.insert(key.clone(), Slot::Loading);
            let _ = self.req_tx.send(Request {
                key: key.clone(),
                urls,
                cache,
            });
        }
        match self.slots.get(&key) {
            Some(Slot::Ready(a)) => Some(a),
            _ => None,
        }
    }

    pub fn failed(&self, sp: &Species, shiny: bool) -> bool {
        let kind = if shiny { "shiny" } else { "normal" };
        matches!(
            self.slots.get(&format!("{kind}/{}", sp.slug)),
            Some(Slot::Failed)
        )
    }
}

fn load(req: &Request) -> Result<Vec<(ColorImage, u32)>, String> {
    if let Ok(bytes) = std::fs::read(&req.cache) {
        if let Ok(frames) = decode(&bytes) {
            return Ok(frames);
        }
    }
    for url in &req.urls {
        if let Ok(bytes) = fetch(url) {
            if let Ok(frames) = decode(&bytes) {
                if let Some(dir) = req.cache.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&req.cache, &bytes);
                return Ok(frames);
            }
        }
    }
    Err("no sprite found".into())
}

pub fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let resp = ureq::get(url)
        .set("User-Agent", "ShinyTracker/0.1 (personal hunting overlay)")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_reader()
        .take(20_000_000)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Crop away fully transparent borders (keeps a 1 px margin).
fn trim_transparent(img: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 8 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 > x1 || y0 > y1 {
        return img;
    }
    let (x0, y0) = (x0.saturating_sub(1), y0.saturating_sub(1));
    let (x1, y1) = ((x1 + 1).min(w - 1), (y1 + 1).min(h - 1));
    image::imageops::crop_imm(&img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image()
}

fn decode(bytes: &[u8]) -> Result<Vec<(ColorImage, u32)>, String> {
    if bytes.starts_with(b"GIF8") {
        use image::AnimationDecoder;
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
            .map_err(|e| e.to_string())?;
        let frames = dec
            .into_frames()
            .collect_frames()
            .map_err(|e| e.to_string())?;
        if frames.is_empty() {
            return Err("empty gif".into());
        }
        Ok(frames
            .into_iter()
            .map(|f| {
                let (n, d) = f.delay().numer_denom_ms();
                let ms = if d == 0 || n == 0 {
                    100
                } else {
                    (n / d).max(20)
                };
                let buf = f.into_buffer();
                let size = [buf.width() as usize, buf.height() as usize];
                (ColorImage::from_rgba_unmultiplied(size, buf.as_raw()), ms)
            })
            .collect())
    } else {
        let img = image::load_from_memory(bytes)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        // Still sprites (box sprites) have lots of empty space around the
        // Pokémon: trim it so they can be shown at a sensible size.
        let img = trim_transparent(img);
        let size = [img.width() as usize, img.height() as usize];
        Ok(vec![(
            ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
            0,
        )])
    }
}
