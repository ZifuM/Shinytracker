//! Little sound effects, synthesised in code (no audio files) and played
//! through Windows' waveOut, on the default or a chosen output device.

use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    /// Achievement unlocked: a quick sparkly arpeggio.
    Achievement,
    /// Level up: a rising fanfare.
    LevelUp,
    /// Milestone (1,000s, odds): a bright two-note ding.
    Milestone,
    /// Caught!: a happy jingle.
    Caught,
    /// Daily goal / weekly challenge done.
    Goal,
    /// A shiny was spotted on screen: two glittering runs and a bright
    /// held chord, longer and louder than the others so it can't be missed.
    Shiny,
}

impl Sfx {
    pub const ALL: [Sfx; 6] = [Sfx::Shiny, Sfx::Achievement, Sfx::LevelUp, Sfx::Milestone, Sfx::Caught, Sfx::Goal];
    /// Stable name used to store a custom sound for this effect.
    pub fn key(self) -> &'static str {
        match self {
            Sfx::Achievement => "achievement",
            Sfx::LevelUp => "level_up",
            Sfx::Milestone => "milestone",
            Sfx::Caught => "caught",
            Sfx::Goal => "goal",
            Sfx::Shiny => "shiny",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Sfx::Achievement => "Achievements",
            Sfx::LevelUp => "Level ups",
            Sfx::Milestone => "Milestones",
            Sfx::Caught => "Caught!",
            Sfx::Goal => "Goals and challenges",
            Sfx::Shiny => "Shiny spotted!",
        }
    }
}

const RATE: u32 = 22_050;

fn note(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// (start s, length s, midi note, loudness)
fn score(s: Sfx) -> Vec<(f32, f32, f32, f32)> {
    match s {
        Sfx::Achievement => vec![
            (0.00, 0.35, 84.0, 0.7),
            (0.07, 0.35, 88.0, 0.6),
            (0.14, 0.35, 91.0, 0.6),
            (0.21, 0.55, 96.0, 0.7),
        ],
        Sfx::LevelUp => vec![
            (0.00, 0.18, 72.0, 0.6),
            (0.12, 0.18, 76.0, 0.6),
            (0.24, 0.18, 79.0, 0.6),
            (0.36, 0.70, 84.0, 0.8),
            (0.36, 0.70, 88.0, 0.4),
        ],
        Sfx::Milestone => vec![(0.00, 0.30, 88.0, 0.7), (0.12, 0.60, 93.0, 0.7)],
        Sfx::Caught => vec![
            (0.00, 0.14, 79.0, 0.6),
            (0.10, 0.14, 83.0, 0.6),
            (0.20, 0.14, 86.0, 0.6),
            (0.30, 0.14, 91.0, 0.6),
            (0.42, 0.80, 95.0, 0.7),
            (0.42, 0.80, 88.0, 0.3),
        ],
        Sfx::Goal => vec![(0.00, 0.25, 81.0, 0.6), (0.10, 0.25, 85.0, 0.6), (0.20, 0.50, 88.0, 0.7)],
        Sfx::Shiny => vec![
            // first glittering run
            (0.00, 0.22, 88.0, 0.55),
            (0.05, 0.22, 91.0, 0.55),
            (0.10, 0.22, 96.0, 0.55),
            (0.15, 0.30, 100.0, 0.6),
            // second, a step higher
            (0.32, 0.22, 91.0, 0.55),
            (0.37, 0.22, 96.0, 0.55),
            (0.42, 0.22, 100.0, 0.55),
            (0.47, 0.30, 103.0, 0.6),
            // bright held chord with a little shimmer on top
            (0.66, 1.10, 84.0, 0.45),
            (0.66, 1.10, 88.0, 0.40),
            (0.66, 1.10, 91.0, 0.40),
            (0.66, 1.10, 96.0, 0.55),
            (0.80, 0.22, 100.0, 0.30),
            (0.92, 0.22, 103.0, 0.28),
            (1.04, 0.30, 105.0, 0.26),
        ],
    }
}

/// 16-bit mono WAV bytes for a sound at `volume` (0..1).
pub fn wav(s: Sfx, volume: f32) -> Vec<u8> {
    let notes = score(s);
    let len_s = notes.iter().map(|n| n.0 + n.1).fold(0.0, f32::max) + 0.05;
    let n = (len_s * RATE as f32) as usize;
    let mut buf = vec![0f32; n];
    for &(start, dur, midi, amp) in &notes {
        let f = note(midi);
        let i0 = (start * RATE as f32) as usize;
        let len = (dur * RATE as f32) as usize;
        for k in 0..len {
            let i = i0 + k;
            if i >= n {
                break;
            }
            let t = k as f32 / RATE as f32;
            // soft attack, exponential decay; a sine plus a little octave for sparkle
            let env = (t / 0.006).min(1.0) * (-t * 7.0 / dur.max(0.1)).exp();
            let ph = std::f32::consts::TAU * f * t;
            buf[i] += amp * env * (ph.sin() + 0.25 * (2.0 * ph).sin() + 0.08 * (3.0 * ph).sin());
        }
    }
    let peak = buf.iter().fold(0f32, |m, v| m.max(v.abs())).max(1e-6);
    let gain = volume.clamp(0.0, 1.0) * 0.8 / peak;
    let data: Vec<u8> = buf
        .iter()
        .flat_map(|v| (((v * gain).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// Sounds stay in memory while Windows plays them, so they're kept here
/// (and never freed while the app runs).
static CACHE: Mutex<Vec<(Sfx, u32, &'static [u8])>> = Mutex::new(Vec::new());

/// Play a sound on `device` (an output device name from `devices()`;
/// empty or not found = the Windows default).
pub fn play(s: Sfx, volume: f32, device: &str) {
    let vkey = (volume.clamp(0.0, 1.0) * 100.0).round() as u32;
    if vkey == 0 {
        return;
    }
    let bytes: &'static [u8] = {
        let Ok(mut cache) = CACHE.lock() else { return };
        match cache.iter().find(|(k, v, _)| *k == s && *v == vkey) {
            Some((_, _, b)) => b,
            None => {
                let b: &'static [u8] = Box::leak(wav(s, vkey as f32 / 100.0).into_boxed_slice());
                cache.push((s, vkey, b));
                b
            }
        }
    };
    imp::play(bytes, device);
}

/// Play a song file (MP3, WAV, WMA, M4A…) on the default output device, for
/// the "Alert song". Stops any song already playing. Returns false if Windows
/// couldn't open the file.
pub fn play_song(path: &str, volume: f32) -> bool {
    imp::play_file("shinysong", path, volume)
}

/// Stop the alert song if it's playing.
pub fn stop_song() {
    imp::stop_file("shinysong")
}

/// Play the user's own file for a sound effect (default speakers). Returns
/// false if it couldn't be played, so the built-in sound can be used instead.
pub fn play_custom(s: Sfx, path: &str, volume: f32) -> bool {
    if volume <= 0.0 {
        return true; // muted: nothing to play, but don't fall back either
    }
    imp::play_file(&format!("sfx_{}", s.key()), path, volume)
}

/// Is this a file the custom sounds can use?
pub fn usable_file(path: &str) -> bool {
    !path.is_empty() && !path.contains('"') && std::path::Path::new(path).is_file()
}

/// Names of the audio output devices (cached; `refresh` re-reads them).
pub fn devices(refresh: bool) -> Vec<String> {
    static LIST: Mutex<Option<Vec<String>>> = Mutex::new(None);
    let Ok(mut l) = LIST.lock() else { return Vec::new() };
    if refresh || l.is_none() {
        *l = Some(imp::devices());
    }
    l.clone().unwrap_or_default()
}

#[cfg(windows)]
mod imp {
    use std::ptr::null_mut;

    // Windows' media player commands (MCI): plays MP3 / WAV / WMA / M4A files.
    #[link(name = "winmm")]
    extern "system" {
        fn mciSendStringW(command: *const u16, ret: *mut u16, ret_len: u32, hwnd: isize) -> u32;
    }

    fn mci(cmd: &str) -> u32 {
        let w: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { mciSendStringW(w.as_ptr(), null_mut(), 0, 0) }
    }

    /// Open and play a file under `alias` (restarting it if already playing).
    /// Each alias plays independently, so different sounds can overlap.
    pub fn play_file(alias: &str, path: &str, volume: f32) -> bool {
        if !super::usable_file(path) {
            return false;
        }
        mci(&format!("close {alias}"));
        if mci(&format!("open \"{path}\" type mpegvideo alias {alias}")) != 0 {
            return false;
        }
        let v = (volume.clamp(0.0, 1.0) * 1000.0).round() as u32;
        mci(&format!("setaudio {alias} volume to {v}"));
        mci(&format!("play {alias}")) == 0
    }

    pub fn stop_file(alias: &str) {
        mci(&format!("close {alias}"));
    }

    #[repr(C)]
    struct WaveFormatEx {
        format_tag: u16,
        channels: u16,
        samples_per_sec: u32,
        avg_bytes_per_sec: u32,
        block_align: u16,
        bits_per_sample: u16,
        cb_size: u16,
    }
    #[repr(C)]
    struct WaveHdr {
        data: *mut u8,
        buffer_length: u32,
        bytes_recorded: u32,
        user: usize,
        flags: u32,
        loops: u32,
        next: *mut WaveHdr,
        reserved: usize,
    }
    #[repr(C)]
    struct WaveOutCapsW {
        mid: u16,
        pid: u16,
        driver_version: u32,
        pname: [u16; 32],
        formats: u32,
        channels: u16,
        reserved1: u16,
        support: u32,
    }
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(sound: *const u16, module: isize, flags: u32) -> i32;
        fn waveOutGetNumDevs() -> u32;
        fn waveOutGetDevCapsW(id: usize, caps: *mut WaveOutCapsW, size: u32) -> u32;
        fn waveOutOpen(h: *mut isize, id: u32, fmt: *const WaveFormatEx, cb: usize, inst: usize, flags: u32) -> u32;
        fn waveOutPrepareHeader(h: isize, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutUnprepareHeader(h: isize, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutWrite(h: isize, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutClose(h: isize) -> u32;
    }
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    const WAVE_MAPPER: u32 = u32::MAX;
    const WHDR_DONE: u32 = 1;

    pub fn devices() -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            for id in 0..waveOutGetNumDevs() {
                let mut caps: WaveOutCapsW = std::mem::zeroed();
                if waveOutGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<WaveOutCapsW>() as u32) == 0 {
                    let n = caps.pname.iter().position(|c| *c == 0).unwrap_or(32);
                    out.push(String::from_utf16_lossy(&caps.pname[..n]));
                }
            }
        }
        out
    }

    pub fn play(bytes: &'static [u8], device: &str) {
        let id = if device.is_empty() {
            WAVE_MAPPER
        } else {
            devices().iter().position(|d| d == device).map_or(WAVE_MAPPER, |i| i as u32)
        };
        // Our WAVs are 44-byte header + 16-bit mono PCM.
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let pcm: &'static [u8] = &bytes[44..];
        std::thread::spawn(move || unsafe {
            let fmt = WaveFormatEx {
                format_tag: 1,
                channels: 1,
                samples_per_sec: rate,
                avg_bytes_per_sec: rate * 2,
                block_align: 2,
                bits_per_sample: 16,
                cb_size: 0,
            };
            let mut h: isize = 0;
            if waveOutOpen(&mut h, id, &fmt, 0, 0, 0) != 0 {
                // fall back to the default device
                PlaySoundW(bytes.as_ptr() as *const u16, 0, SND_ASYNC | SND_NODEFAULT | SND_MEMORY);
                return;
            }
            let size = std::mem::size_of::<WaveHdr>() as u32;
            let mut hdr = Box::new(WaveHdr {
                data: pcm.as_ptr() as *mut u8,
                buffer_length: pcm.len() as u32,
                bytes_recorded: 0,
                user: 0,
                flags: 0,
                loops: 0,
                next: null_mut(),
                reserved: 0,
            });
            let hp: *mut WaveHdr = &mut *hdr;
            if waveOutPrepareHeader(h, hp, size) == 0 {
                if waveOutWrite(h, hp, size) == 0 {
                    // wait for it to finish (at most ~5 s)
                    for _ in 0..500 {
                        if std::ptr::read_volatile(&(*hp).flags) & WHDR_DONE != 0 {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
                waveOutUnprepareHeader(h, hp, size);
            }
            waveOutClose(h);
        });
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn play_file(_alias: &str, _path: &str, _volume: f32) -> bool {
        false
    }
    pub fn stop_file(_alias: &str) {}
    pub fn devices() -> Vec<String> {
        Vec::new()
    }
    pub fn play(_bytes: &'static [u8], _device: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wavs_are_valid() {
        for s in Sfx::ALL {
            let w = wav(s, 0.8);
            assert_eq!(&w[0..4], b"RIFF");
            assert_eq!(&w[8..12], b"WAVE");
            assert!(w.len() > 5_000 && w.len() < 200_000, "{s:?} {}", w.len());
        }
    }
}
