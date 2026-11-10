//! Background thread: capture → crop name plates → OCR → match species →
//! encounter state machine. Results go to the UI over a channel.

use crate::capture::Capturer;
use crate::config::CountSource;
use crate::detect::{NameMatcher, Tracker, TrackerConfig, TrackerEvent};
use crate::gamelog::LogTail;
use crate::imageproc::{prepare_names, Region};
use crate::ocr::Ocr;
use crate::species::Species;
use eframe::egui;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long after a battle starts to keep looking for the shiny star.
const STAR_WINDOW_MS: u64 = 8000;
/// Skip the battle intro (boxes sliding in, grass sparkles).
const STAR_IGNORE_MS: u64 = 600;
/// A ★ must be seen this many looks in a row, in the same place...
const STAR_FRAMES: u32 = 3;
/// ...over at least this long.
const STAR_HOLD_MS: u64 = 300;

/// The sensitivity setting -> smallest ★ size in pixels (default 60 -> 15).
fn star_min_px(threshold: u32) -> u32 {
    (threshold / 4).max(12)
}

/// Where a possible ★ was seen in each name box: (x, y, looks in a row, first seen).
type StarTrack = Vec<Option<(f32, f32, u32, Instant)>>;

/// One look at name box `i`. True once a ★ has stayed in place long enough.
fn star_check(track: &mut StarTrack, i: usize, found: Option<(f32, f32, u32)>) -> bool {
    if track.len() <= i {
        track.resize(i + 1, None);
    }
    match (found, track[i]) {
        (Some((x, y, _)), Some((px, py, n, t)))
            if (x - px).abs() <= 3.0 && (y - py).abs() <= 3.0 =>
        {
            track[i] = Some((x, y, n + 1, t));
            n + 1 >= STAR_FRAMES && t.elapsed() >= Duration::from_millis(STAR_HOLD_MS)
        }
        (Some((x, y, _)), _) => {
            track[i] = Some((x, y, 1, Instant::now()));
            false
        }
        (None, _) => {
            track[i] = None;
            false
        }
    }
}

#[derive(Clone)]
pub struct WorkerSettings {
    pub paused: bool,
    pub monitor: usize,
    pub name_regions: Vec<Region>,
    pub tracker: TrackerConfig,
    pub interval_ms: u64,
    pub shiny_star_alert: bool,
    pub star_threshold: u32,
    pub source: CountSource,
    /// Bumped whenever regions/monitor change so the worker starts fresh.
    pub generation: u64,
}

#[derive(Clone, Default)]
pub struct Status {
    pub text: String,
    pub names: Vec<usize>,
    pub in_battle: bool,
    pub error: Option<String>,
    pub loop_ms: u32,
    pub idle_reason: Option<&'static str>,
}

pub enum WorkerMsg {
    Encounter {
        names: Vec<usize>,
        added: u32,
        late: bool,
    },
    BattleOver,
    ShinyStar {
        plate: usize,
        yellow: u32,
    },
    /// The starter picked from the briefcase came out shiny.
    ShinyGift,
    /// Shiny chance the game reported for the latest encounter.
    LogOdds {
        num: u32,
        den: u32,
    },
    Status(Status),
}

pub fn spawn(
    settings: Arc<Mutex<WorkerSettings>>,
    tx: Sender<WorkerMsg>,
    ctx: egui::Context,
    species: Arc<Vec<Species>>,
) {
    std::thread::Builder::new()
        .name("shiny-detector".into())
        .spawn(move || run(settings, tx, ctx, species))
        .expect("spawn detector thread");
}

fn run(
    settings: Arc<Mutex<WorkerSettings>>,
    tx: Sender<WorkerMsg>,
    ctx: egui::Context,
    species: Arc<Vec<Species>>,
) {
    let matcher = NameMatcher::new(&species);
    let mut ocr: Option<Ocr> = None;
    let mut ocr_error: Option<String> = None;
    let mut capturer = Capturer::new();
    let mut tracker = Tracker::new(TrackerConfig::default());
    let mut generation = u64::MAX;
    let mut last_fp: Option<u64> = None;
    let mut last_names: Vec<usize> = Vec::new();
    let mut last_text = String::new();
    let mut star_done = false;
    let mut star_track: StarTrack = Vec::new();
    let clock = Instant::now();
    let mut logtail: Option<LogTail> = None;
    let mut last_log_try: Option<Instant> = None;
    let mut log_battle_at: Option<Instant> = None;

    let send = |msg: WorkerMsg| {
        let _ = tx.send(msg);
        ctx.request_repaint();
    };

    loop {
        let s = settings
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|p| p.into_inner().clone());
        if s.generation != generation {
            generation = s.generation;
            tracker.reset();
            last_fp = None;
            last_names.clear();
            star_done = false;
            star_track.clear();
        }
        tracker.cfg = s.tracker;

        // ── Game log: always followed (it reports the exact odds); it only
        // drives the counter when "count from the game log" is chosen.
        if logtail.is_none() && last_log_try.map_or(true, |t| t.elapsed() > Duration::from_secs(5))
        {
            last_log_try = Some(Instant::now());
            logtail = crate::gamelog::default_log_dir().map(LogTail::new);
        }
        let mut log_error = None;
        match logtail.as_mut() {
            Some(tail) => match tail.poll() {
                Ok(groups) => {
                    for g in groups {
                        if let Some(w) = g.first() {
                            send(WorkerMsg::LogOdds { num: w.chance_num, den: w.chance_den });
                        }
                        if s.source == CountSource::GameLog && !s.paused {
                            let names: Vec<usize> = g
                                .iter()
                                .filter_map(|w| (w.species as usize).checked_sub(1))
                                .filter(|&i| i < species.len())
                                .collect();
                            send(WorkerMsg::Encounter { names, added: g.len() as u32, late: false });
                            log_battle_at = Some(Instant::now());
                            star_done = false;
                star_track.clear();
                        }
                    }
                    // Starter picked from the briefcase: one soft reset, and
                    // the game says whether it's shiny. Screen reading can't
                    // see this, so it's counted whichever way counting is set.
                    for gift in tail.take_gifts() {
                        if s.paused {
                            continue;
                        }
                        let names: Vec<usize> = (gift.species as usize)
                            .checked_sub(1)
                            .filter(|&i| i < species.len())
                            .into_iter()
                            .collect();
                        send(WorkerMsg::Encounter { names, added: 1, late: false });
                        if gift.shiny {
                            send(WorkerMsg::ShinyGift);
                        }
                    }
                }
                Err(e) => log_error = Some(e),
            },
            None => {
                log_error = Some(
                    "Can't find Platinum Together's log folder (AppData\\Local\\PlatinumTogether\\Game\\logs)".into(),
                )
            }
        }

        if s.source == CountSource::GameLog {
            let since = log_battle_at.map(|t| t.elapsed());
            // Shiny star alert still uses the name boxes (if set up), early in each battle.
            if s.shiny_star_alert
                && !star_done
                && !s.name_regions.is_empty()
                && since.is_some_and(|d| {
                    d >= Duration::from_millis(STAR_IGNORE_MS)
                        && d <= Duration::from_millis(STAR_WINDOW_MS)
                })
            {
                if let Ok(frame) = capturer.grab(s.monitor) {
                    for (i, r) in s.name_regions.iter().enumerate() {
                        let found = frame
                            .crop(*r)
                            .and_then(|c| c.find_star(star_min_px(s.star_threshold)));
                        if star_check(&mut star_track, i, found) {
                            star_done = true;
                            send(WorkerMsg::ShinyStar {
                                plate: i,
                                yellow: found.map_or(0, |f| f.2),
                            });
                            break;
                        }
                    }
                }
            }
            send(WorkerMsg::Status(Status {
                text: logtail
                    .as_ref()
                    .and_then(|t| t.file_name())
                    .unwrap_or_default(),
                in_battle: since.is_some_and(|d| d < Duration::from_secs(6)),
                error: log_error,
                idle_reason: if s.paused { Some("Paused") } else { None },
                ..Default::default()
            }));
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }

        let idle = if s.paused {
            Some("Paused")
        } else if s.name_regions.is_empty() {
            Some("Set up name boxes in ⚙ Setup")
        } else {
            None
        };
        if let Some(reason) = idle {
            send(WorkerMsg::Status(Status {
                idle_reason: Some(reason),
                ..Default::default()
            }));
            std::thread::sleep(Duration::from_millis(300));
            continue;
        }

        if ocr.is_none() {
            match Ocr::new() {
                Ok(o) => {
                    ocr = Some(o);
                    ocr_error = None;
                }
                Err(e) => {
                    ocr_error = Some(e);
                }
            }
        }
        let Some(engine) = ocr.as_ref() else {
            send(WorkerMsg::Status(Status {
                error: ocr_error.clone(),
                ..Default::default()
            }));
            std::thread::sleep(Duration::from_secs(3));
            continue;
        };

        let t0 = Instant::now();
        let mut error = None;

        let frame = match capturer.grab(s.monitor) {
            Ok(f) => f,
            Err(e) => {
                send(WorkerMsg::Status(Status {
                    error: Some(e),
                    ..Default::default()
                }));
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        let names = match prepare_names(&frame, &s.name_regions) {
            None => {
                error = Some(
                    "Name boxes are outside the captured screen. Re-draw them in Setup.".into(),
                );
                Vec::new()
            }
            Some(img) => {
                let fp = img.fingerprint();
                if last_fp == Some(fp) {
                    // Nothing changed on screen: reuse the last reading.
                    last_names.clone()
                } else {
                    last_fp = Some(fp);
                    match engine.read(&img) {
                        Ok(text) => {
                            last_text = text;
                            matcher.find(&last_text)
                        }
                        Err(e) => {
                            error = Some(e);
                            Vec::new()
                        }
                    }
                }
            }
        };
        last_names = names.clone();

        let now = clock.elapsed().as_millis() as u64;
        match tracker.feed(names.clone(), now) {
            Some(TrackerEvent::Encounter { names, added }) => {
                star_done = false;
                star_track.clear();
                send(WorkerMsg::Encounter {
                    names,
                    added,
                    late: false,
                });
            }
            Some(TrackerEvent::LateNames { names, added }) => {
                send(WorkerMsg::Encounter {
                    names,
                    added,
                    late: true,
                });
            }
            Some(TrackerEvent::BattleOver) => send(WorkerMsg::BattleOver),
            None => {}
        }

        // Shiny ★ check: yellow pixels inside the name boxes early in a battle.
        if s.shiny_star_alert
            && !star_done
            && tracker
                .battle_age_ms(now)
                .is_some_and(|age| (STAR_IGNORE_MS..=STAR_WINDOW_MS).contains(&age))
        {
            for (i, r) in s.name_regions.iter().enumerate() {
                let found = frame
                    .crop(*r)
                    .and_then(|c| c.find_star(star_min_px(s.star_threshold)));
                if star_check(&mut star_track, i, found) {
                    star_done = true;
                    send(WorkerMsg::ShinyStar {
                        plate: i,
                        yellow: found.map_or(0, |f| f.2),
                    });
                    break;
                }
            }
        }

        let loop_ms = t0.elapsed().as_millis() as u32;
        send(WorkerMsg::Status(Status {
            text: last_text.clone(),
            names,
            in_battle: tracker.in_battle(),
            error,
            loop_ms,
            idle_reason: None,
        }));

        let wait = s.interval_ms.saturating_sub(loop_ms as u64).max(30);
        std::thread::sleep(Duration::from_millis(wait));
    }
}
