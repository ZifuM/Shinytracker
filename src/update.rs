//! Auto-update from the GitHub releases of ZifuM/Shinytracker.
//!
//! A background thread asks GitHub for the latest release. If its version
//! (the tag, e.g. "v0.3.0") is newer than this build, it downloads the
//! release's .exe next to the running one as "Shiny Tracker.update.exe",
//! reporting progress. "Restart to update" then swaps the files: the running
//! exe is renamed to "*.old.exe" (Windows allows renaming a running exe), the
//! new one takes its name and is started, and this copy quits. The old file
//! is deleted on the next start.

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const REPO: &str = "ZifuM/Shinytracker";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    /// Downloading `version`: bytes so far and total (0 = unknown).
    Downloading { version: String, got: u64, total: u64 },
    /// Downloaded and ready to install.
    Ready { version: String, path: PathBuf },
    Failed(String),
}

pub type Shared = Arc<Mutex<State>>;

pub fn new_shared() -> Shared {
    Arc::new(Mutex::new(State::Idle))
}

pub fn get(s: &Shared) -> State {
    s.lock().map(|g| g.clone()).unwrap_or(State::Idle)
}

fn set(s: &Shared, v: State) {
    if let Ok(mut g) = s.lock() {
        *g = v;
    }
}

/// "v1.2.3" / "1.2" → (1, 2, 3). Anything unreadable → None.
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let mut it = v.split(|c: char| c == '.' || c == '-' || c == '+');
    let a = it.next()?.parse().ok()?;
    let b = it.next().map_or(Some(0), |x| x.parse().ok())?;
    let c = it.next().map_or(Some(0), |x| x.parse().ok())?;
    Some((a, b, c))
}

/// Is `latest` a newer version than `current`?
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// The file the update downloads to (next to the running exe).
fn update_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.with_file_name("Shiny Tracker.update.exe"))
}

/// Delete leftovers from a previous update (call at start-up).
pub fn cleanup() {
    if let Ok(exe) = std::env::current_exe() {
        let old = exe.with_extension("old.exe");
        // The old copy may still be closing: try a few times in the background.
        std::thread::spawn(move || {
            for _ in 0..10 {
                if !old.exists() || std::fs::remove_file(&old).is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

/// Check GitHub (and download if there's a newer build) on a background thread.
pub fn check(shared: Shared, ctx: eframe::egui::Context) {
    {
        let Ok(mut g) = shared.lock() else { return };
        if matches!(*g, State::Checking | State::Downloading { .. } | State::Ready { .. }) {
            return; // already busy or done
        }
        *g = State::Checking;
    }
    std::thread::spawn(move || {
        let result = run(&shared, &ctx);
        if let Err(e) = result {
            set(&shared, State::Failed(e));
        }
        ctx.request_repaint();
    });
}

fn run(shared: &Shared, ctx: &eframe::egui::Context) -> Result<(), String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", "ShinyTracker-updater")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "No releases published yet".to_string(),
            other => format!("Couldn't reach GitHub ({other})"),
        })?;
    let body = resp.into_string().map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let tag = json["tag_name"].as_str().unwrap_or("").to_string();
    if !is_newer(&tag, VERSION) {
        set(shared, State::UpToDate);
        return Ok(());
    }
    // Find the .exe asset (prefer one named like the app).
    let assets = json["assets"].as_array().cloned().unwrap_or_default();
    let exe = assets
        .iter()
        .filter(|a| a["name"].as_str().is_some_and(|n| n.to_lowercase().ends_with(".exe")))
        .max_by_key(|a| a["name"].as_str().is_some_and(|n| n.to_lowercase().contains("shiny")))
        .ok_or_else(|| format!("Release {tag} has no .exe to download"))?;
    let dl = exe["browser_download_url"].as_str().ok_or("Release asset has no download link")?;
    let total_hint = exe["size"].as_u64().unwrap_or(0);
    let version = tag.trim_start_matches(['v', 'V']).to_string();

    let dest = update_path().ok_or("Couldn't find where the app is")?;
    let tmp = dest.with_extension("part");
    set(shared, State::Downloading { version: version.clone(), got: 0, total: total_hint });
    ctx.request_repaint();

    let resp = ureq::get(dl)
        .set("User-Agent", "ShinyTracker-updater")
        .timeout(Duration::from_secs(300))
        .call()
        .map_err(|e| format!("Download failed ({e})"))?;
    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(total_hint);
    let mut reader = resp.into_reader().take(200_000_000);
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("Couldn't save the update ({e})"))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut got = 0u64;
    let mut first = Vec::new();
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("Download interrupted ({e})"))?;
        if n == 0 {
            break;
        }
        if first.len() < 2 {
            first.extend_from_slice(&buf[..n.min(2)]);
        }
        std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| format!("Couldn't save the update ({e})"))?;
        got += n as u64;
        set(shared, State::Downloading { version: version.clone(), got, total });
        ctx.request_repaint();
    }
    drop(file);
    // Sanity checks: a real Windows program, complete.
    if got < 500_000 || !first.starts_with(b"MZ") || (total > 0 && got != total) {
        let _ = std::fs::remove_file(&tmp);
        return Err("The downloaded update looks incomplete; will try again later".into());
    }
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&tmp, &dest).map_err(|e| format!("Couldn't save the update ({e})"))?;
    set(shared, State::Ready { version, path: dest });
    ctx.request_repaint();
    Ok(())
}

/// Swap in the downloaded build and start it. The caller saves settings
/// first and closes this copy right after this returns Ok.
pub fn install(new_exe: &std::path::Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = exe.with_extension("old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("Couldn't replace the app ({e})"))?;
    if let Err(e) = std::fs::rename(new_exe, &exe) {
        // put things back the way they were
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("Couldn't replace the app ({e})"));
    }
    std::process::Command::new(&exe)
        .current_dir(exe.parent().unwrap_or(std::path::Path::new(".")))
        .spawn()
        .map_err(|e| format!("Couldn't start the new version ({e})"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.10"), Some((1, 10, 0)));
        assert!(is_newer("v0.3.0", "0.2.9"));
        assert!(is_newer("v1.0", "0.9.9"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("garbage", "0.2.0"));
        assert!(!is_newer("v0.1.9", "0.2.0"));
    }
}
