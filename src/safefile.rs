//! Saving the planner files (teams, battle setups) safely: written whole to
//! a temporary file and swapped in, a copy kept in `backups/<stem>-<time>.json`
//! before the first save of each run (the newest 5 are kept), and a file that
//! can't be read is moved aside, never overwritten.

use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// Rolling copies kept of each file.
const KEEP: usize = 5;

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn file_in(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.json"))
}

pub fn backups_in(dir: &Path, stem: &str) -> Vec<PathBuf> {
    let pre = format!("{stem}-");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir.join("backups"))
        .map(|r| r.flatten().map(|e| e.path()).filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&pre) && n.ends_with(".json"))).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Load a file, and a note when it couldn't be read: then it's moved aside
/// (kept) and the newest good backup is used. `what` names it in the note.
pub fn load<T: DeserializeOwned + Default>(dir: &Path, stem: &str, what: &str) -> (T, Option<String>) {
    let path = file_in(dir, stem);
    let Ok(text) = std::fs::read_to_string(&path) else { return (T::default(), None) };
    match serde_json::from_str::<T>(&text) {
        Ok(f) => (f, None),
        Err(_) => {
            let aside = dir.join(format!("{stem}.unreadable-{}.json", now()));
            let _ = std::fs::rename(&path, &aside);
            let back = backups_in(dir, stem).pop().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<T>(&t).ok());
            let note = match &back {
                Some(_) => format!("Your {what} file couldn't be read, so the newest backup was loaded. The old file is kept as {}.", aside.display()),
                None => format!("Your {what} file couldn't be read. It's kept as {}.", aside.display()),
            };
            (back.unwrap_or_default(), Some(note))
        }
    }
}

/// Save a file. `has_data` says whether a file's contents hold anything
/// worth keeping: an empty save never replaces a full file without a copy.
pub fn save<T: Serialize + DeserializeOwned>(dir: &Path, stem: &str, f: &T, first_this_run: bool, has_data: impl Fn(&T) -> bool) -> Result<(), String> {
    let path = file_in(dir, stem);
    let _ = std::fs::create_dir_all(dir);
    if let Ok(old) = std::fs::read_to_string(&path) {
        let had = serde_json::from_str::<T>(&old).is_ok_and(|o| has_data(&o));
        if first_this_run || (!has_data(f) && had) {
            let bk = dir.join("backups");
            let _ = std::fs::create_dir_all(&bk);
            let mut name = bk.join(format!("{stem}-{}.json", now()));
            let mut n = 1;
            while name.exists() {
                name = bk.join(format!("{stem}-{}-{n}.json", now()));
                n += 1;
            }
            let _ = std::fs::write(name, &old);
            let mut all = backups_in(dir, stem);
            while all.len() > KEEP {
                let _ = std::fs::remove_file(all.remove(0));
            }
        }
    }
    let json = serde_json::to_string_pretty(f).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
