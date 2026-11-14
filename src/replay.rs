//! Shiny Replay: keeps the last few seconds of the game window and saves
//! them as an MP4 when the shiny alert fires (or when asked by hand).
//!
//! How it works:
//! - A recorder thread captures the game window at 30 or 60 fps, scales it
//!   to the chosen size and hands the frames to an encoder thread.
//! - The encoder thread writes short H.264 segments (a couple of seconds
//!   each, in memory) and keeps just enough of them for a clip.
//! - On a save, the recorder waits for the seconds after, then joins the
//!   segments that cover the clip into one MP4 file (on its own thread).
//!
//! Counting always comes first: the recorder runs at a lower priority and
//! drops frames rather than waiting, and nothing here ever touches the
//! game's files or memory: it only looks at the window, like a screenshot.
//!
//! The capture and the encoder are Windows code (`replay_win`), switched on
//! by the `replay-rec` cargo feature. Without it the app shows the recorder
//! as not available. `SHINY_REPLAY_DEMO=1` runs the whole pipeline with a
//! made-up picture and a stand-in encoder (for testing anywhere).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ───────────────────────────── settings ─────────────────────────────

/// Recording size (the height; the width follows the game window's shape).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Res {
    P480,
    #[default]
    P720,
    P1080,
}

impl Res {
    pub const ALL: [Res; 3] = [Res::P480, Res::P720, Res::P1080];
    pub fn height(self) -> u32 {
        match self {
            Res::P480 => 480,
            Res::P720 => 720,
            Res::P1080 => 1080,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Res::P480 => "480p",
            Res::P720 => "720p",
            Res::P1080 => "1080p",
        }
    }
}

/// Bitrate: a preset (scaled to the size and frame rate) or a set number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Quality {
    Low,
    Medium,
    #[default]
    High,
    VeryHigh,
    Custom,
}

impl Quality {
    pub const ALL: [Quality; 5] = [Quality::Low, Quality::Medium, Quality::High, Quality::VeryHigh, Quality::Custom];
    pub fn label(self) -> &'static str {
        match self {
            Quality::Low => "Low",
            Quality::Medium => "Medium",
            Quality::High => "High",
            Quality::VeryHigh => "Very high",
            Quality::Custom => "Custom",
        }
    }
    /// Bits per pixel per frame for the presets.
    fn bpp(self) -> f64 {
        match self {
            Quality::Low => 0.05,
            Quality::Medium => 0.08,
            Quality::High | Quality::Custom => 0.11,
            Quality::VeryHigh => 0.16,
        }
    }
}

pub const MBPS_MIN: f32 = 1.0;
pub const MBPS_MAX: f32 = 50.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplaySettings {
    /// Keep the last seconds ready while the overlay is running.
    pub on: bool,
    pub res: Res,
    pub fps: u32,
    pub quality: Quality,
    /// Used when `quality` is Custom (Mbps).
    pub custom_mbps: f32,
    /// Seconds kept before the alert and recorded after it.
    pub before: u32,
    pub after: u32,
    /// Where clips go (None: Videos\Shiny Replays).
    pub folder: Option<PathBuf>,
    /// Hand-saved clips kept (0 = all). Shiny clips are never removed.
    pub keep: u32,
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self { on: true, res: Res::P720, fps: 30, quality: Quality::High, custom_mbps: 8.0, before: 15, after: 5, folder: None, keep: 0 }
    }
}

impl ReplaySettings {
    pub fn fps(&self) -> u32 {
        if self.fps >= 45 {
            60
        } else {
            30
        }
    }

    /// The recording size for a game window of `src` (never bigger than the
    /// window, never above the chosen height; both even, as H.264 needs).
    pub fn size_for(&self, src: (u32, u32)) -> (u32, u32) {
        target_size(src, self.res.height())
    }

    /// Bitrate in kbit/s for a recording size.
    pub fn kbps(&self, size: (u32, u32)) -> u32 {
        let k = match self.quality {
            Quality::Custom => self.custom_mbps.clamp(MBPS_MIN, MBPS_MAX) as f64 * 1000.0,
            q => size.0 as f64 * size.1 as f64 * self.fps() as f64 * q.bpp() / 1000.0,
        };
        (k.round() as u32).clamp(300, 80_000)
    }

    /// Seconds in a clip.
    pub fn clip_secs(&self) -> u32 {
        self.before + self.after
    }

    /// About how big a clip is (MB), for a 16:9 game window.
    #[cfg(test)]
    pub fn clip_mb(&self) -> f32 {
        let size = self.size_for((3840, 2160));
        self.kbps(size) as f32 * self.clip_secs() as f32 / 8.0 / 1000.0
    }

    pub fn folder(&self) -> PathBuf {
        self.folder.clone().unwrap_or_else(default_folder)
    }
}

pub fn default_folder() -> PathBuf {
    dirs::video_dir().unwrap_or_else(|| crate::config::data_dir().join("videos")).join("Shiny Replays")
}

/// `src` scaled down to at most `max_h` tall, kept in shape, both even.
pub fn target_size(src: (u32, u32), max_h: u32) -> (u32, u32) {
    let (sw, sh) = (src.0.max(2), src.1.max(2));
    let h = sh.min(max_h);
    let w = ((sw as u64 * h as u64 + sh as u64 / 2) / sh as u64) as u32;
    (w.max(2) & !1, h.max(2) & !1)
}

// ───────────────────────────── the clip list ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClipKind {
    #[default]
    Shiny,
    /// Saved with "Save replay now".
    Hand,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Clip {
    /// "20261114-134201" (also the preview's file name).
    pub id: String,
    /// The MP4.
    pub file: PathBuf,
    pub kind: ClipKind,
    /// The shiny (empty for most hand-saved clips).
    pub mon: String,
    pub dex: u32,
    /// What the hunt was (target and method), for the line under the name.
    pub hunt: String,
    pub encounters: u64,
    /// Unix time it was saved.
    pub when: u64,
    pub secs: f32,
    /// Where in the clip the alert was (seconds from the start).
    pub alert_at: f32,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub bytes: u64,
    /// Preview frames in the preview sheet, and which one is the alert.
    pub frames: u32,
    pub alert_frame: u32,
    /// Size of one preview frame.
    pub fw: u32,
    pub fh: u32,
    /// Renamed by hand (shown instead of the Pokémon).
    pub title: String,
}

impl Clip {
    pub fn name(&self) -> String {
        if !self.title.trim().is_empty() {
            self.title.clone()
        } else if !self.mon.is_empty() {
            self.mon.clone()
        } else if self.kind == ClipKind::Hand {
            "Saved by hand".into()
        } else {
            "Shiny".into()
        }
    }
    pub fn exists(&self) -> bool {
        self.file.is_file()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayFile {
    /// Newest first.
    pub clips: Vec<Clip>,
}

impl ReplayFile {
    pub fn has_data(&self) -> bool {
        !self.clips.is_empty()
    }

    pub fn add(&mut self, c: Clip) {
        self.clips.retain(|x| x.id != c.id);
        self.clips.insert(0, c);
    }

    /// Hand-saved clips past `keep` (oldest first). Shinies are never on it.
    pub fn over_keep(&self, keep: u32) -> Vec<Clip> {
        if keep == 0 {
            return Vec::new();
        }
        self.clips.iter().filter(|c| c.kind == ClipKind::Hand).skip(keep as usize).cloned().collect()
    }

    pub fn remove(&mut self, id: &str) -> Option<Clip> {
        let i = self.clips.iter().position(|c| c.id == id)?;
        Some(self.clips.remove(i))
    }
}

pub fn previews_dir() -> PathBuf {
    crate::config::data_dir().join("replay-previews")
}

pub fn preview_path(id: &str) -> PathBuf {
    previews_dir().join(format!("{id}.png"))
}

pub fn load_from(dir: &Path) -> (ReplayFile, Option<String>) {
    crate::safefile::load(dir, "shiny-replays", "Shiny Replay")
}

pub fn save_to(dir: &Path, f: &ReplayFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "shiny-replays", f, first_this_run, ReplayFile::has_data)
}

/// "Shiny Eevee 2026-11-14 13-42-01.mp4" style name and the clip id.
pub fn clip_names(kind: ClipKind, mon: &str, unix: u64) -> (String, String) {
    let local = unix as i64 + crate::stats::utc_offset_secs();
    let day = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (y, m, d) = crate::stats::civil_from_days(day);
    let (hh, mm, ss) = (secs / 3600, secs % 3600 / 60, secs % 60);
    let id = format!("{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}");
    let what = match (kind, mon.is_empty()) {
        (ClipKind::Shiny, false) => format!("Shiny {mon}"),
        (ClipKind::Shiny, true) => "Shiny".to_string(),
        (ClipKind::Hand, _) => "Replay".to_string(),
    };
    let clean: String = what.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-').collect();
    (format!("{clean} {y:04}-{m:02}-{d:02} {hh:02}-{mm:02}-{ss:02}.mp4"), id)
}

// ───────────────────────────── frames ─────────────────────────────

/// A picture of the game window: BGRA, rows packed.
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

/// A small RGBA picture for the preview.
#[derive(Clone)]
pub struct Thumb {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

pub const THUMB_W: u32 = 256;
/// A preview picture this often.
pub const THUMB_EVERY: Duration = Duration::from_millis(500);
pub const PREVIEW_MAX: usize = 48;

/// Scale a BGRA picture to `w`×`h` (box filter when shrinking, so the game's
/// pixel art stays clean; plain copy when the size already matches).
pub fn scale_bgra(src: &Frame, w: u32, h: u32, out: &mut Vec<u8>) {
    out.clear();
    if src.w == w && src.h == h {
        out.extend_from_slice(&src.px);
        return;
    }
    out.resize((w * h * 4) as usize, 0);
    let (sw, sh) = (src.w as usize, src.h as usize);
    // source span per output pixel, in 1/256ths
    let fx = ((sw as u64) << 8) / w as u64;
    let fy = ((sh as u64) << 8) / h as u64;
    for oy in 0..h as usize {
        let y0 = ((oy as u64 * fy) >> 8) as usize;
        let y1 = ((((oy + 1) as u64 * fy) >> 8) as usize).clamp(y0 + 1, sh);
        for ox in 0..w as usize {
            let x0 = ((ox as u64 * fx) >> 8) as usize;
            let x1 = ((((ox + 1) as u64 * fx) >> 8) as usize).clamp(x0 + 1, sw);
            // up to 2×2 samples is plenty for the sizes used here
            let xs = [x0, (x0 + x1 - 1) / 2 + ((x1 - x0) > 1) as usize].map(|x| x.min(sw - 1));
            let ys = [y0, (y0 + y1 - 1) / 2 + ((y1 - y0) > 1) as usize].map(|y| y.min(sh - 1));
            let mut acc = [0u32; 4];
            for &yy in &ys {
                for &xx in &xs {
                    let i = (yy * sw + xx) * 4;
                    for c in 0..4 {
                        acc[c] += src.px[i + c] as u32;
                    }
                }
            }
            let o = (oy * w as usize + ox) * 4;
            for c in 0..4 {
                out[o + c] = (acc[c] / 4) as u8;
            }
        }
    }
}

/// A preview picture (RGBA, `THUMB_W` wide) from a BGRA picture.
pub fn thumb_of(px: &[u8], w: u32, h: u32) -> Thumb {
    let tw = THUMB_W.min(w).max(2);
    let th = ((h as u64 * tw as u64) / w.max(1) as u64).max(2) as u32;
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for y in 0..th {
        let sy = (y as u64 * h as u64 / th as u64) as u32;
        for x in 0..tw {
            let sx = (x as u64 * w as u64 / tw as u64) as u32;
            let i = ((sy * w + sx) * 4) as usize;
            let o = ((y * tw + x) * 4) as usize;
            if i + 3 < px.len() {
                out[o] = px[i + 2];
                out[o + 1] = px[i + 1];
                out[o + 2] = px[i];
                out[o + 3] = 255;
            }
        }
    }
    Thumb { w: tw, h: th, px: out }
}

/// Up to `max` previews spread evenly, and which one is nearest `alert`.
pub fn pick_previews(thumbs: &[(Instant, Thumb)], from: Instant, to: Instant, alert: Instant, max: usize) -> (Vec<Thumb>, usize) {
    let inside: Vec<&(Instant, Thumb)> = thumbs.iter().filter(|(t, _)| *t >= from && *t <= to).collect();
    if inside.is_empty() {
        return (Vec::new(), 0);
    }
    let n = inside.len().min(max.max(1));
    let picks: Vec<&(Instant, Thumb)> = (0..n).map(|i| inside[i * inside.len() / n]).collect();
    let alert_i = picks
        .iter()
        .enumerate()
        .min_by_key(|(_, (t, _))| if *t > alert { *t - alert } else { alert - *t })
        .map(|(i, _)| i)
        .unwrap_or(0);
    (picks.into_iter().map(|(_, th)| th.clone()).collect(), alert_i)
}

/// The previews side by side, 8 to a row, as one PNG.
pub fn preview_sheet_png(frames: &[Thumb]) -> Option<Vec<u8>> {
    let f = frames.first()?;
    let (fw, fh) = (f.w, f.h);
    let cols = 8u32.min(frames.len() as u32);
    let rows = (frames.len() as u32).div_ceil(cols);
    let mut img = image::RgbaImage::new(fw * cols, fh * rows);
    for (i, t) in frames.iter().enumerate() {
        if t.w != fw || t.h != fh {
            continue;
        }
        let (ox, oy) = ((i as u32 % cols) * fw, (i as u32 / cols) * fh);
        for y in 0..fh {
            for x in 0..fw {
                let s = ((y * fw + x) * 4) as usize;
                img.put_pixel(ox + x, oy + y, image::Rgba([t.px[s], t.px[s + 1], t.px[s + 2], 255]));
            }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

// ───────────────────────────── platform parts ─────────────────────────────

/// Gets pictures of the game window.
pub trait Capture {
    /// Find the game window and start. Err: why not (e.g. the game isn't open).
    fn open(&mut self) -> Result<(), String>;
    /// The newest picture, or None if nothing new since the last one.
    fn grab(&mut self) -> Result<Option<Frame>, String>;
    fn close(&mut self);
}

/// Turns pictures into one short H.264/MP4 segment at a time.
pub trait Encoder {
    fn begin(&mut self, w: u32, h: u32, fps: u32, kbps: u32) -> Result<(), String>;
    /// A BGRA picture of the size given to `begin`, `t` after the segment began.
    fn frame(&mut self, bgra: &[u8], t: Duration) -> Result<(), String>;
    /// End the segment: its bytes (a complete little MP4).
    fn finish(&mut self) -> Result<Vec<u8>, String>;
}

type MakeCapture = fn() -> Result<Box<dyn Capture>, String>;
type MakeEncoder = fn() -> Result<Box<dyn Encoder>, String>;
type Mux = fn(&[Vec<u8>], &Path) -> Result<(), String>;

struct Platform {
    capture: MakeCapture,
    encoder: MakeEncoder,
    mux: Mux,
    /// Lower this thread's priority (Windows).
    background: fn(),
}

fn platform() -> Result<Platform, String> {
    if std::env::var_os("SHINY_REPLAY_DEMO").is_some() {
        return Ok(demo::platform());
    }
    #[cfg(all(windows, feature = "replay-rec"))]
    {
        Ok(Platform { capture: crate::replay_win::capture, encoder: crate::replay_win::encoder, mux: crate::replay_win::mux, background: crate::replay_win::background })
    }
    #[cfg(not(all(windows, feature = "replay-rec")))]
    {
        Err("The recorder isn't in this build yet. It's coming in the next step.".into())
    }
}

// ───────────────────────────── the recorder ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
pub enum RecState {
    /// Switched off, or the overlay isn't running.
    #[default]
    Off,
    /// Waiting for the game window.
    NoGame(String),
    Recording,
    /// Can't record in this build / on this PC.
    Unavailable(String),
    /// Something went wrong; it tries again by itself.
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct RecStatus {
    pub state: RecState,
    /// Seconds ready to save.
    pub ready: f32,
    /// Frames per second actually recorded.
    pub fps: f32,
    pub w: u32,
    pub h: u32,
    pub kbps: u32,
    /// Memory the kept seconds use (bytes).
    pub mem: u64,
    /// Clips being written right now.
    pub saving: u32,
}

pub struct SaveReq {
    pub id: String,
    pub path: PathBuf,
}

pub enum RecEvent {
    Saved { id: String, path: PathBuf, secs: f32, alert_at: f32, w: u32, h: u32, fps: u32, bytes: u64, previews: Vec<Thumb>, alert_frame: usize },
    Failed { id: String, msg: String },
}

enum Cmd {
    Set(ReplaySettings, bool),
    Save(SaveReq, Instant),
}

/// The app's handle on the recorder thread.
pub struct Recorder {
    cmd: Sender<Cmd>,
    events: Receiver<RecEvent>,
    status: Arc<Mutex<RecStatus>>,
    last_set: Option<(ReplaySettings, bool)>,
}

impl Recorder {
    pub fn spawn(repaint: Option<eframe::egui::Context>) -> Self {
        let (cmd, cmd_rx) = channel();
        let (ev_tx, events) = channel();
        let status = Arc::new(Mutex::new(RecStatus::default()));
        let st = status.clone();
        let _ = std::thread::Builder::new().name("shiny-replay".into()).spawn(move || {
            // A problem in the recorder must never take the app (or the count) down.
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(cmd_rx, ev_tx, st.clone(), repaint)));
            if ok.is_err() {
                if let Ok(mut s) = st.lock() {
                    s.state = RecState::Error("The recorder stopped after a problem. Restart the app to try again.".into());
                }
            }
        });
        Self { cmd, events, status, last_set: None }
    }

    /// What to do: these settings, and whether to record now (overlay running).
    pub fn set(&mut self, s: &ReplaySettings, running: bool) {
        let want = (s.clone(), running);
        if self.last_set.as_ref() != Some(&want) {
            let _ = self.cmd.send(Cmd::Set(want.0.clone(), want.1));
            self.last_set = Some(want);
        }
    }

    /// Save a clip around now (the seconds after are still recorded first).
    pub fn save(&self, req: SaveReq) {
        let _ = self.cmd.send(Cmd::Save(req, Instant::now()));
    }

    pub fn status(&self) -> RecStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn poll(&self) -> Vec<RecEvent> {
        self.events.try_iter().collect()
    }
}

/// One finished segment.
struct Segment {
    start: Instant,
    end: Instant,
    bytes: Vec<u8>,
}

/// How long each segment is (a clip starts on a segment edge, so it starts
/// up to this much earlier than asked).
pub const SEGMENT: Duration = Duration::from_secs(2);

/// The segments to join for a clip around `alert`: from the last one that
/// starts at or before `alert - before`, to the end. (start, end) of each.
pub fn pick_segments(segs: &[(Instant, Instant)], alert: Instant, before: Duration) -> std::ops::Range<usize> {
    if segs.is_empty() {
        return 0..0;
    }
    let want = alert.checked_sub(before).unwrap_or(alert);
    let first = segs.iter().rposition(|(s, _)| *s <= want).unwrap_or(0);
    first..segs.len()
}

enum EncMsg {
    Frame(Vec<u8>, Instant),
    /// Close the segment being written now (for a save), then answer.
    Cut(Sender<()>),
}

struct EncoderPipe {
    tx: SyncSender<EncMsg>,
    segs: Arc<Mutex<VecDeque<Segment>>>,
    started: Arc<Mutex<Option<Instant>>>,
    err: Arc<Mutex<Option<String>>>,
    size: (u32, u32),
}

impl EncoderPipe {
    fn start(make: MakeEncoder, background: fn(), size: (u32, u32), fps: u32, kbps: u32, keep: Duration) -> Self {
        let (tx, rx) = sync_channel::<EncMsg>(6);
        let segs = Arc::new(Mutex::new(VecDeque::new()));
        let started = Arc::new(Mutex::new(None));
        let err = Arc::new(Mutex::new(None));
        let (s2, st2, e2) = (segs.clone(), started.clone(), err.clone());
        let _ = std::thread::Builder::new().name("shiny-replay-enc".into()).spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                background();
                encode_loop(rx, make, size, fps, kbps, keep, &s2, &st2)
            }));
            let msg = match r {
                Ok(Ok(())) => return,
                Ok(Err(e)) => e,
                Err(_) => "The video encoder stopped after a problem".into(),
            };
            if let Ok(mut e) = e2.lock() {
                *e = Some(msg);
            }
        });
        Self { tx, segs, started, err, size }
    }

    /// Seconds ready, and the memory they use.
    fn ready(&self, now: Instant) -> (f32, u64) {
        let segs = self.segs.lock().map(|s| (s.front().map(|x| x.start), s.iter().map(|x| x.bytes.len() as u64).sum::<u64>())).unwrap_or((None, 0));
        let first = segs.0.or_else(|| self.started.lock().ok().and_then(|s| *s));
        (first.map_or(0.0, |f| now.saturating_duration_since(f).as_secs_f32()), segs.1)
    }

    fn error(&self) -> Option<String> {
        self.err.lock().ok().and_then(|e| e.clone())
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_loop(
    rx: Receiver<EncMsg>,
    make: MakeEncoder,
    size: (u32, u32),
    fps: u32,
    kbps: u32,
    keep: Duration,
    segs: &Mutex<VecDeque<Segment>>,
    started: &Mutex<Option<Instant>>,
) -> Result<(), String> {
    let mut enc = make()?;
    // the segment being written: when it began, its last frame
    let mut cur: Option<(Instant, Instant)> = None;
    let push = |enc: &mut Box<dyn Encoder>, cur: &mut Option<(Instant, Instant)>| -> Result<(), String> {
        if let Some((start, last)) = cur.take() {
            let bytes = enc.finish()?;
            let end = last + Duration::from_secs_f64(1.0 / fps as f64);
            let mut s = segs.lock().map_err(|_| "segments")?;
            s.push_back(Segment { start, end, bytes });
            // keep just enough for a clip
            while s.len() > 2 && s.get(1).is_some_and(|x| end.saturating_duration_since(x.start) >= keep) {
                s.pop_front();
            }
        }
        Ok(())
    };
    for msg in rx.iter() {
        match msg {
            EncMsg::Frame(px, at) => {
                if cur.is_some_and(|(s, _)| at.saturating_duration_since(s) >= SEGMENT) {
                    push(&mut enc, &mut cur)?;
                }
                let (start, _) = match cur {
                    Some(c) => c,
                    None => {
                        enc.begin(size.0, size.1, fps, kbps)?;
                        if let Ok(mut s) = started.lock() {
                            s.get_or_insert(at);
                        }
                        (at, at)
                    }
                };
                enc.frame(&px, at.saturating_duration_since(start))?;
                cur = Some((start, at));
            }
            EncMsg::Cut(done) => {
                push(&mut enc, &mut cur)?;
                let _ = done.send(());
            }
        }
    }
    push(&mut enc, &mut cur)
}

fn set_state(st: &Mutex<RecStatus>, f: impl FnOnce(&mut RecStatus)) {
    if let Ok(mut s) = st.lock() {
        f(&mut s);
    }
}

fn run(cmd_rx: Receiver<Cmd>, ev_tx: Sender<RecEvent>, st: Arc<Mutex<RecStatus>>, repaint: Option<eframe::egui::Context>) {
    let mut settings = ReplaySettings::default();
    let mut want = false;
    let mut plat: Option<Platform> = None;
    let mut cap: Option<Box<dyn Capture>> = None;
    let mut pipe: Option<EncoderPipe> = None;
    let mut thumbs: VecDeque<(Instant, Thumb)> = VecDeque::new();
    let mut last_thumb: Option<Instant> = None;
    let mut pending: Vec<(SaveReq, Instant)> = Vec::new();
    let mut retry_at: Option<Instant> = None;
    let mut next_frame = Instant::now();
    let mut scaled: Vec<u8> = Vec::new();
    let mut last_px: Option<(u32, u32, Vec<u8>)> = None;
    let mut fps_win: VecDeque<Instant> = VecDeque::new();
    let saving = Arc::new(Mutex::new(0u32));
    let mut prioritised = false;

    let stop = |cap: &mut Option<Box<dyn Capture>>, pipe: &mut Option<EncoderPipe>, thumbs: &mut VecDeque<(Instant, Thumb)>| {
        if let Some(mut c) = cap.take() {
            c.close();
        }
        *pipe = None;
        thumbs.clear();
    };

    loop {
        // ── commands
        let recording = want && settings.on && cap.is_some();
        let msg = if recording {
            match cmd_rx.try_recv() {
                Ok(m) => Some(m),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        } else {
            match cmd_rx.recv_timeout(Duration::from_millis(250)) {
                Ok(m) => Some(m),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(_) => return,
            }
        };
        match msg {
            Some(Cmd::Set(s, w)) => {
                let restart = s.res != settings.res || s.fps() != settings.fps() || s.kbps((1920, 1080)) != settings.kbps((1920, 1080)) || s.before != settings.before || s.after != settings.after;
                settings = s;
                want = w;
                if restart || !(want && settings.on) {
                    pipe = None;
                    if !(want && settings.on) {
                        stop(&mut cap, &mut pipe, &mut thumbs);
                        retry_at = None;
                    }
                }
                continue;
            }
            Some(Cmd::Save(req, at)) => {
                if pipe.is_some() {
                    pending.push((req, at));
                } else {
                    let _ = ev_tx.send(RecEvent::Failed { id: req.id, msg: "Nothing recorded yet: the recorder only runs while the overlay is on and the game is open.".into() });
                    if let Some(c) = &repaint {
                        c.request_repaint();
                    }
                }
                continue;
            }
            None => {}
        }

        if !(want && settings.on) {
            set_state(&st, |s| *s = RecStatus { saving: *saving.lock().unwrap_or_else(|e| e.into_inner()), ..Default::default() });
            continue;
        }

        // ── the platform and the game window
        if plat.is_none() {
            match platform() {
                Ok(p) => plat = Some(p),
                Err(e) => {
                    set_state(&st, |s| s.state = RecState::Unavailable(e));
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
        let p = plat.as_ref().expect("set above");
        if !prioritised {
            (p.background)();
            prioritised = true;
        }
        if cap.is_none() {
            if retry_at.is_some_and(|t| Instant::now() < t) {
                continue;
            }
            match (p.capture)().and_then(|mut c| c.open().map(|_| c)) {
                Ok(c) => {
                    cap = Some(c);
                    next_frame = Instant::now();
                    retry_at = None;
                }
                Err(e) => {
                    set_state(&st, |s| *s = RecStatus { state: RecState::NoGame(e), ..Default::default() });
                    retry_at = Some(Instant::now() + Duration::from_secs(3));
                    continue;
                }
            }
        }

        // ── one frame
        let fps = settings.fps();
        let period = Duration::from_secs_f64(1.0 / fps as f64);
        let now = Instant::now();
        if now < next_frame {
            std::thread::sleep((next_frame - now).min(Duration::from_millis(20)));
            continue;
        }
        next_frame += period;
        if now.saturating_duration_since(next_frame) > period * 4 {
            next_frame = now + period; // fell behind: don't try to catch up
        }
        let grabbed = match cap.as_mut().expect("open").grab() {
            Ok(f) => f,
            Err(e) => {
                stop(&mut cap, &mut pipe, &mut thumbs);
                set_state(&st, |s| *s = RecStatus { state: RecState::NoGame(e), ..Default::default() });
                retry_at = Some(Instant::now() + Duration::from_secs(3));
                continue;
            }
        };
        // nothing new on screen: repeat the last picture so time keeps going
        let src = match grabbed {
            Some(f) => {
                let size = settings.size_for((f.w, f.h));
                scale_bgra(&f, size.0, size.1, &mut scaled);
                last_px = Some((size.0, size.1, scaled.clone()));
                Some(size)
            }
            None => last_px.as_ref().map(|(w, h, _)| (*w, *h)),
        };
        let Some(size) = src else { continue };
        let at = Instant::now();
        // encoder for this size
        if pipe.as_ref().is_some_and(|pp| pp.size != size || pp.error().is_some()) {
            if let Some(e) = pipe.as_ref().and_then(|pp| pp.error()) {
                set_state(&st, |s| s.state = RecState::Error(e));
                std::thread::sleep(Duration::from_secs(2));
            }
            pipe = None;
        }
        if pipe.is_none() {
            let keep = Duration::from_secs((settings.before + settings.after) as u64) + SEGMENT * 2;
            pipe = Some(EncoderPipe::start(p.encoder, p.background, size, fps, settings.kbps(size), keep));
            thumbs.clear();
        }
        let pp = pipe.as_ref().expect("started");
        let px = &last_px.as_ref().expect("have a picture").2;
        // previews
        if last_thumb.map_or(true, |t| at.saturating_duration_since(t) >= THUMB_EVERY) {
            last_thumb = Some(at);
            thumbs.push_back((at, thumb_of(px, size.0, size.1)));
            let keep = Duration::from_secs((settings.before + settings.after + 6) as u64);
            while thumbs.front().is_some_and(|(t, _)| at.saturating_duration_since(*t) > keep) {
                thumbs.pop_front();
            }
        }
        match pp.tx.try_send(EncMsg::Frame(px.clone(), at)) {
            Ok(()) => {
                fps_win.push_back(at);
                while fps_win.front().is_some_and(|t| at.saturating_duration_since(*t) > Duration::from_secs(1)) {
                    fps_win.pop_front();
                }
            }
            Err(TrySendError::Full(_)) => {} // encoder busy: drop this one
            Err(TrySendError::Disconnected(_)) => pipe = None,
        }

        // ── saves that are due
        let after = Duration::from_secs(settings.after as u64);
        let before = Duration::from_secs(settings.before as u64);
        let mut i = 0;
        while i < pending.len() {
            if at.saturating_duration_since(pending[i].1) < after {
                i += 1;
                continue;
            }
            let (req, alert) = pending.remove(i);
            let Some(pp) = pipe.as_ref() else {
                let _ = ev_tx.send(RecEvent::Failed { id: req.id, msg: "The recording stopped before the clip was done.".into() });
                continue;
            };
            let (done_tx, done_rx) = channel();
            if pp.tx.send(EncMsg::Cut(done_tx)).is_err() || done_rx.recv_timeout(Duration::from_secs(5)).is_err() {
                let _ = ev_tx.send(RecEvent::Failed { id: req.id, msg: "The video encoder didn't answer.".into() });
                continue;
            }
            let (bytes, first, last): (Vec<Vec<u8>>, Instant, Instant) = {
                let segs = pp.segs.lock().map(|s| s.iter().map(|x| (x.start, x.end)).collect::<Vec<_>>()).unwrap_or_default();
                let r = pick_segments(&segs, alert, before);
                let b = pp.segs.lock().map(|s| s.range(r.clone()).map(|x| x.bytes.clone()).collect()).unwrap_or_default();
                match (segs.get(r.start), segs.get(r.end.saturating_sub(1))) {
                    (Some(f), Some(l)) => (b, f.0, l.1),
                    _ => (Vec::new(), at, at),
                }
            };
            if bytes.is_empty() {
                let _ = ev_tx.send(RecEvent::Failed { id: req.id, msg: "Nothing was recorded for that moment.".into() });
                continue;
            }
            let (previews, alert_frame) = pick_previews(thumbs.make_contiguous(), first, last, alert, PREVIEW_MAX);
            let (mux, tx, rp, sv) = (p.mux, ev_tx.clone(), repaint.clone(), saving.clone());
            let (w, h) = size;
            if let Ok(mut n) = sv.lock() {
                *n += 1;
            }
            let _ = std::thread::Builder::new().name("shiny-replay-save".into()).spawn(move || {
                let _ = std::fs::create_dir_all(req.path.parent().unwrap_or(Path::new(".")));
                let r = std::panic::catch_unwind(|| mux(&bytes, &req.path)).unwrap_or_else(|_| Err("Writing the clip failed".into()));
                let ev = match r {
                    Ok(()) => RecEvent::Saved {
                        id: req.id,
                        bytes: std::fs::metadata(&req.path).map(|m| m.len()).unwrap_or(0),
                        path: req.path,
                        secs: last.saturating_duration_since(first).as_secs_f32(),
                        alert_at: alert.saturating_duration_since(first).as_secs_f32(),
                        w,
                        h,
                        fps,
                        previews,
                        alert_frame,
                    },
                    Err(msg) => RecEvent::Failed { id: req.id, msg },
                };
                let _ = tx.send(ev);
                if let Ok(mut n) = sv.lock() {
                    *n = n.saturating_sub(1);
                }
                if let Some(c) = rp {
                    c.request_repaint();
                }
            });
        }

        // ── status
        if let Some(pp) = pipe.as_ref() {
            let (ready, mem) = pp.ready(at);
            let shown = ready.min((settings.before) as f32);
            let n = *saving.lock().unwrap_or_else(|e| e.into_inner());
            let kbps = settings.kbps(size);
            set_state(&st, |s| {
                *s = RecStatus { state: RecState::Recording, ready: shown, fps: fps_win.len() as f32, w: size.0, h: size.1, kbps, mem: mem + thumbs.len() as u64 * (THUMB_W as u64 * 144 * 4), saving: n }
            });
        }
    }
}

/// Unix time now.
pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ───────────────────────────── demo / test stand-ins ─────────────────────────────

/// A made-up game picture and a stand-in encoder, so the whole recorder can
/// run (and be tested) without Windows. The "MP4" it writes is just the
/// joined stand-in bytes.
mod demo {
    use super::*;

    pub(super) fn platform() -> Platform {
        Platform { capture: || Ok(Box::new(DemoCapture { n: 0 })), encoder: || Ok(Box::new(DemoEncoder::default())), mux: demo_mux, background: || {} }
    }

    struct DemoCapture {
        n: u32,
    }

    impl Capture for DemoCapture {
        fn open(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn grab(&mut self) -> Result<Option<Frame>, String> {
            self.n += 1;
            let (w, h) = (640u32, 360u32);
            let mut px = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    let sky = y < h * 44 / 100;
                    px[i] = if sky { 214 } else { 107 };
                    px[i + 1] = if sky { 143 } else { 194 };
                    px[i + 2] = if sky { 93 } else { 124 };
                    px[i + 3] = 255;
                    // a moving block, so frames differ
                    if (x + self.n * 4) % w < 60 && y > h / 5 && y < h / 5 + 60 {
                        px[i] = 230;
                        px[i + 1] = 226;
                        px[i + 2] = 208;
                    }
                }
            }
            Ok(Some(Frame { w, h, px }))
        }
        fn close(&mut self) {}
    }

    #[derive(Default)]
    struct DemoEncoder {
        frames: u32,
    }

    impl Encoder for DemoEncoder {
        fn begin(&mut self, _w: u32, _h: u32, _fps: u32, _kbps: u32) -> Result<(), String> {
            self.frames = 0;
            Ok(())
        }
        fn frame(&mut self, _bgra: &[u8], _t: Duration) -> Result<(), String> {
            self.frames += 1;
            Ok(())
        }
        fn finish(&mut self) -> Result<Vec<u8>, String> {
            Ok(format!("segment of {} frames\n", self.frames).into_bytes())
        }
    }

    fn demo_mux(parts: &[Vec<u8>], path: &Path) -> Result<(), String> {
        std::fs::write(path, parts.concat()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_bitrates() {
        // a 16:9 window is shrunk to the chosen height, never grown
        assert_eq!(target_size((2560, 1440), 1080), (1920, 1080));
        assert_eq!(target_size((1280, 720), 1080), (1280, 720));
        assert_eq!(target_size((1366, 768), 720), (1280, 720));
        // odd sizes become even
        let (w, h) = target_size((1001, 777), 1080);
        assert!(w % 2 == 0 && h % 2 == 0);
        let mut s = ReplaySettings::default();
        assert_eq!(s.fps(), 30);
        s.fps = 60;
        assert_eq!(s.fps(), 60);
        s.fps = 24; // an old value
        assert_eq!(s.fps(), 30);
        s.quality = Quality::Custom;
        s.custom_mbps = 12.0;
        assert_eq!(s.kbps((1920, 1080)), 12_000);
        s.quality = Quality::High;
        s.fps = 30;
        let k720 = s.kbps((1280, 720));
        assert!((2500..4000).contains(&k720), "{k720}");
        assert!(s.kbps((1920, 1080)) > k720);
        assert!(Quality::Low.bpp() < Quality::VeryHigh.bpp());
        s.res = Res::P720;
        assert!(s.clip_mb() > 4.0 && s.clip_mb() < 12.0, "{}", s.clip_mb());
    }

    #[test]
    fn clips_start_on_a_segment_before_the_wanted_time() {
        let t0 = Instant::now();
        let s = |a: u64, b: u64| (t0 + Duration::from_secs(a), t0 + Duration::from_secs(b));
        let segs = vec![s(0, 2), s(2, 4), s(4, 6), s(6, 8), s(8, 10), s(10, 12), s(12, 14), s(14, 16), s(16, 18), s(18, 20), s(20, 22), s(22, 24)];
        // alert at 19 s, 15 s before → 4 s: starts with the segment at 4
        let r = pick_segments(&segs, t0 + Duration::from_secs(19), Duration::from_secs(15));
        assert_eq!(r, 2..12);
        // alert at 17.5: wants 2.5 → the segment at 2
        let r = pick_segments(&segs, t0 + Duration::from_millis(17_500), Duration::from_secs(15));
        assert_eq!(r.start, 1);
        // not enough yet: everything there is
        let r = pick_segments(&segs[5..], t0 + Duration::from_secs(12), Duration::from_secs(15));
        assert_eq!(r, 0..7);
        assert_eq!(pick_segments(&[], t0, Duration::from_secs(15)), 0..0);
    }

    #[test]
    fn previews_and_scaling() {
        let t0 = Instant::now();
        let th = |c: u8| Thumb { w: 4, h: 2, px: vec![c; 32] };
        let thumbs: Vec<(Instant, Thumb)> = (0..60).map(|i| (t0 + Duration::from_millis(500 * i), th(i as u8))).collect();
        let (p, a) = pick_previews(&thumbs, t0, t0 + Duration::from_secs(20), t0 + Duration::from_secs(15), 40);
        assert_eq!(p.len(), 40);
        assert!((28..=31).contains(&a), "{a}");
        let sheet = preview_sheet_png(&p).unwrap();
        assert!(sheet.starts_with(&[0x89, b'P', b'N', b'G']));
        // scaling keeps a flat colour flat
        let f = Frame { w: 8, h: 6, px: [10u8, 20, 30, 255].repeat(48) };
        let mut out = Vec::new();
        scale_bgra(&f, 4, 2, &mut out);
        assert_eq!(out.len(), 4 * 2 * 4);
        assert!(out.chunks(4).all(|c| c == [10, 20, 30, 255]));
        scale_bgra(&f, 8, 6, &mut out);
        assert_eq!(out, f.px);
        let t = thumb_of(&f.px, 8, 6);
        assert_eq!((t.w, t.h), (8, 6));
        assert_eq!(&t.px[..4], &[30, 20, 10, 255]);
    }

    #[test]
    fn keep_never_removes_shinies() {
        let mut f = ReplayFile::default();
        for i in 0..30 {
            f.add(Clip { id: format!("{i}"), kind: if i % 3 == 0 { ClipKind::Shiny } else { ClipKind::Hand }, ..Default::default() });
        }
        let over = f.over_keep(5);
        assert_eq!(over.len(), 20 - 5);
        assert!(over.iter().all(|c| c.kind == ClipKind::Hand));
        assert!(f.over_keep(0).is_empty());
        // the newest is first
        assert_eq!(f.clips[0].id, "29");
        assert!(f.remove("29").is_some());
        assert!(f.remove("29").is_none());
        let (name, id) = clip_names(ClipKind::Shiny, "Mr. Mime", 1_794_627_000);
        assert!(name.starts_with("Shiny Mr Mime 20") && name.ends_with(".mp4"), "{name}");
        assert_eq!(id.len(), 15);
    }

    #[test]
    fn the_demo_recorder_saves_a_clip() {
        std::env::set_var("SHINY_REPLAY_DEMO", "1");
        let mut r = Recorder::spawn(None);
        let mut s = ReplaySettings { before: 2, after: 1, fps: 30, ..Default::default() };
        s.on = true;
        r.set(&s, true);
        // let it fill up
        let t = Instant::now();
        while r.status().ready < 2.0 && t.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(r.status().state, RecState::Recording, "{:?}", r.status().state);
        let dir = std::env::temp_dir().join(format!("replay-test-{}", std::process::id()));
        let path = dir.join("clip.mp4");
        r.save(SaveReq { id: "x".into(), path: path.clone() });
        let t = Instant::now();
        let mut got = None;
        while got.is_none() && t.elapsed() < Duration::from_secs(10) {
            got = r.poll().into_iter().next();
            std::thread::sleep(Duration::from_millis(50));
        }
        match got {
            Some(RecEvent::Saved { secs, alert_at, previews, w, h, .. }) => {
                assert!(secs >= 3.0 && secs < 6.5, "{secs}");
                assert!(alert_at >= 2.0 && alert_at <= secs, "{alert_at}");
                assert!(!previews.is_empty());
                assert_eq!((w, h), (640, 360));
                let text = std::fs::read_to_string(&path).unwrap();
                assert!(text.contains("segment of"));
            }
            Some(RecEvent::Failed { msg, .. }) => panic!("{msg}"),
            None => panic!("no clip"),
        }
        r.set(&s, false);
        let _ = std::fs::remove_dir_all(dir);
    }
}
