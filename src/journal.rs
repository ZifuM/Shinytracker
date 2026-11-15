//! The Journal: notes kept as plain Markdown files in a `Journal` folder in
//! the data folder (folders inside it are the note folders), like an
//! Obsidian vault. This holds the files and what's read out of a note
//! (properties, tags, links, headings); the app's pages draw them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The Journal's folder.
pub fn root() -> PathBuf {
    crate::config::data_dir().join("Journal")
}

/// Deleted notes go here (inside the Journal, so nothing is lost by a click).
pub const TRASH: &str = ".trash";

/// Pictures added to the Journal (share images, imported files) go here.
pub const ATTACH: &str = "Attachments";

/// One entry in the folder tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Folder { name: String, rel: String, children: Vec<Entry> },
    Note { name: String, rel: String },
    /// A `.canvas` board.
    Canvas { name: String, rel: String },
    /// A picture (png, gif, jpg).
    Image { name: String, rel: String },
}

impl Entry {
    pub fn rel(&self) -> &str {
        match self {
            Entry::Folder { rel, .. } | Entry::Note { rel, .. } | Entry::Canvas { rel, .. } | Entry::Image { rel, .. } => rel,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Entry::Folder { name, .. } | Entry::Note { name, .. } | Entry::Canvas { name, .. } | Entry::Image { name, .. } => name,
        }
    }
}

/// A picture the Journal can show.
pub fn is_image(name: &str) -> bool {
    let n = name.to_lowercase();
    [".png", ".gif", ".jpg", ".jpeg"].iter().any(|e| n.ends_with(e))
}

pub fn is_canvas(rel: &str) -> bool {
    rel.to_lowercase().ends_with(".canvas")
}

/// A note's path inside the Journal, with `/` between folders ("Hunts/Gible.md").
pub fn rel_of(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/")
}

/// The full path of a note's `rel`. None for anything that would leave the Journal.
pub fn path_of(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.starts_with('/') || rel.contains('\\') || rel.split('/').any(|s| s == ".." || s.is_empty() || s == ".") {
        return None;
    }
    Some(root.join(rel))
}

/// A note's title: its file name without `.md` (or a canvas's without `.canvas`).
pub fn title_of(rel: &str) -> String {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    name.strip_suffix(".md").or_else(|| name.strip_suffix(".canvas")).unwrap_or(name).to_string()
}

/// The folders and notes under `dir`, folders first, both by name. Hidden
/// entries (the trash) are left out.
pub fn tree(root: &Path) -> Vec<Entry> {
    fn walk(root: &Path, dir: &Path, depth: usize) -> Vec<Entry> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
        let mut folders = Vec::new();
        let mut notes = Vec::new();
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                if depth < 12 {
                    folders.push(Entry::Folder { rel: rel_of(root, &p), children: walk(root, &p, depth + 1), name });
                }
            } else if name.to_lowercase().ends_with(".md") {
                notes.push(Entry::Note { rel: rel_of(root, &p), name: title_of(&name) });
            } else if is_canvas(&name) {
                notes.push(Entry::Canvas { rel: rel_of(root, &p), name: title_of(&name) });
            } else if is_image(&name) {
                notes.push(Entry::Image { rel: rel_of(root, &p), name });
            }
        }
        let key = |e: &Entry| e.name().to_lowercase();
        folders.sort_by_key(key);
        notes.sort_by_key(key);
        folders.extend(notes);
        folders
    }
    walk(root, root, 0)
}

/// Every note's `rel`, in tree order.
pub fn all_notes(tree: &[Entry]) -> Vec<String> {
    all_where(tree, |e| matches!(e, Entry::Note { .. }))
}

/// Every canvas's `rel`, in tree order.
pub fn all_canvases(tree: &[Entry]) -> Vec<String> {
    all_where(tree, |e| matches!(e, Entry::Canvas { .. }))
}

/// Every picture's `rel`, in tree order.
pub fn all_images(tree: &[Entry]) -> Vec<String> {
    all_where(tree, |e| matches!(e, Entry::Image { .. }))
}

fn all_where(tree: &[Entry], want: fn(&Entry) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    fn go(es: &[Entry], want: fn(&Entry) -> bool, out: &mut Vec<String>) {
        for e in es {
            match e {
                Entry::Folder { children, .. } => go(children, want, out),
                e if want(e) => out.push(e.rel().to_string()),
                _ => {}
            }
        }
    }
    go(tree, want, &mut out);
    out
}

/// Characters a file name can't have on Windows, swapped for a space.
pub fn clean_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { ' ' } else { c }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_matches('.').trim().to_string();
    if s.is_empty() {
        "Untitled".into()
    } else {
        s.chars().take(80).collect()
    }
}

/// A `rel` in `folder` for `name` that isn't taken ("Untitled 2.md"…).
pub fn free_rel(root: &Path, folder: &str, name: &str) -> String {
    free_rel_ext(root, folder, name, "md")
}

/// The same with another file type ("canvas", "png").
pub fn free_rel_ext(root: &Path, folder: &str, name: &str, ext: &str) -> String {
    let base = clean_name(name);
    let join = |n: &str| if folder.is_empty() { format!("{n}.{ext}") } else { format!("{folder}/{n}.{ext}") };
    let mut rel = join(&base);
    let mut i = 2;
    while root.join(&rel).exists() {
        rel = join(&format!("{base} {i}"));
        i += 1;
    }
    rel
}

/// Write a note (making its folders), through a temp file so a crash never
/// leaves half a note.
pub fn write(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    let p = path_of(root, rel).ok_or("Not a note in the Journal")?;
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    write_bytes_at(&p, text.as_bytes())
}

/// Write a picture or any other file into the Journal.
pub fn write_bytes(root: &Path, rel: &str, bytes: &[u8]) -> Result<(), String> {
    let p = path_of(root, rel).ok_or("Not a file in the Journal")?;
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    write_bytes_at(&p, bytes)
}

fn write_bytes_at(p: &Path, bytes: &[u8]) -> Result<(), String> {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = p.with_file_name(format!("{name}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, p).map_err(|e| e.to_string())
}

/// The picture an embed (`![[x.png]]`, `![](Attachments/x.png)`) shows.
pub fn resolve_image(link: &str, images: &[String]) -> Option<String> {
    let l = link.trim().replace("%20", " ").to_lowercase();
    images.iter().find(|r| r.to_lowercase() == l).or_else(|| images.iter().find(|r| r.rsplit('/').next().is_some_and(|n| n.to_lowercase() == l.rsplit('/').next().unwrap_or("")))).cloned()
}

pub fn read(root: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(path_of(root, rel)?).ok()
}

/// Move a note or folder to the trash (kept, not deleted).
pub fn trash(root: &Path, rel: &str) -> Result<String, String> {
    let from = path_of(root, rel).ok_or("Not in the Journal")?;
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let tdir = root.join(TRASH);
    std::fs::create_dir_all(&tdir).map_err(|e| e.to_string())?;
    let mut to = tdir.join(name);
    let mut i = 2;
    while to.exists() {
        to = tdir.join(format!("{i} {name}"));
        i += 1;
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    Ok(rel_of(root, &to))
}

/// Rename or move a note or folder. Returns its new `rel`.
pub fn rename(root: &Path, rel: &str, new_rel: &str) -> Result<String, String> {
    let from = path_of(root, rel).ok_or("Not in the Journal")?;
    let to = path_of(root, new_rel).ok_or("Not a name the Journal can use")?;
    if to.exists() {
        return Err("Something already has that name".into());
    }
    if let Some(d) = to.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    Ok(new_rel.to_string())
}

// ───────────────────────────── reading a note ─────────────────────────────

/// A note's properties (the `---` block at its top) and where its body starts.
pub fn front_matter(text: &str) -> (Vec<(String, String)>, usize) {
    let mut props = Vec::new();
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let skip = text.len() - t.len();
    if !(t.starts_with("---\n") || t.starts_with("---\r\n")) {
        return (props, 0);
    }
    let mut at = t.find('\n').map_or(t.len(), |i| i + 1);
    let mut key: Option<String> = None;
    while at < t.len() {
        let end = t[at..].find('\n').map_or(t.len(), |i| at + i + 1);
        let line = t[at..end].trim_end_matches(['\n', '\r']);
        if line.trim() == "---" {
            return (props, skip + end);
        }
        if let Some(item) = line.trim_start().strip_prefix("- ").filter(|_| line.starts_with(' ') || line.starts_with('-')) {
            if let Some(k) = &key {
                if let Some(last) = props.iter_mut().rev().find(|(pk, _)| pk == k) {
                    if !last.1.is_empty() {
                        last.1.push_str(", ");
                    }
                    last.1.push_str(item.trim());
                }
            }
        } else if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_string();
            let v = v.trim().trim_matches(['[', ']']).to_string();
            key = Some(k.clone());
            props.push((k, v));
        }
        at = end;
    }
    // no closing line: not front matter
    (Vec::new(), 0)
}

/// `#tags` in a note (body and `tags:` property), without the `#`.
pub fn tags(text: &str) -> BTreeSet<String> {
    let (props, body_at) = front_matter(text);
    let mut out = BTreeSet::new();
    for (k, v) in props {
        if k.eq_ignore_ascii_case("tags") || k.eq_ignore_ascii_case("tag") {
            for t in v.split([',', ' ']).map(|t| t.trim().trim_start_matches('#')).filter(|t| !t.is_empty()) {
                out.insert(t.to_string());
            }
        }
    }
    let mut in_code = false;
    for line in text[body_at..].lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let start_ok = i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(';
            if chars[i] == '#' && start_ok {
                let mut j = i + 1;
                while j < chars.len() && (chars[j].is_alphanumeric() || matches!(chars[j], '_' | '-' | '/')) {
                    j += 1;
                }
                let tag: String = chars[i + 1..j].iter().collect();
                // a heading ("# Title") or a number ("#1") isn't a tag
                if !tag.is_empty() && !tag.chars().all(|c| c.is_ascii_digit()) {
                    out.insert(tag);
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }
    out
}

/// `[[links]]` in a note: the note named, without `|alias` or `#heading`.
pub fn links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("[[") {
        let after = &rest[i + 2..];
        let Some(j) = after.find("]]") else { break };
        let inner = &after[..j];
        if !inner.contains('\n') {
            let target = inner.split('|').next().unwrap_or(inner).split('#').next().unwrap_or("").trim();
            if !target.is_empty() {
                out.push(target.to_string());
            }
        }
        rest = &after[j + 2..];
    }
    out
}

/// The headings in a note: (level 1–6, text, line number).
pub fn headings(text: &str) -> Vec<(u8, String, usize)> {
    let (_, body_at) = front_matter(text);
    let skip = text[..body_at].lines().count();
    let mut out = Vec::new();
    let mut in_code = false;
    for (n, line) in text[body_at..].lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if let Some((lvl, t)) = heading(line) {
            out.push((lvl, t.to_string(), n + skip));
        }
    }
    out
}

/// "## Text" -> (2, "Text").
pub fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        Some((hashes as u8, line[hashes..].trim()))
    } else {
        None
    }
}

/// Words in a note's body (properties left out).
pub fn words(text: &str) -> usize {
    let (_, body_at) = front_matter(text);
    text[body_at..].split_whitespace().filter(|w| w.chars().any(|c| c.is_alphanumeric())).count()
}

/// Does `link` (as written in `[[…]]`) point at the note `rel`?
pub fn link_hits(link: &str, rel: &str) -> bool {
    let l = link.trim().trim_end_matches(".md").to_lowercase();
    let r = rel.trim_end_matches(".md").to_lowercase();
    l == r || r.rsplit('/').next() == Some(l.as_str())
}

/// The note a `[[link]]` opens: the one with that name (a path wins).
pub fn resolve(link: &str, notes: &[String]) -> Option<String> {
    let l = link.trim().trim_end_matches(".md").to_lowercase();
    notes.iter().find(|r| r.trim_end_matches(".md").to_lowercase() == l).or_else(|| notes.iter().find(|r| link_hits(link, r))).cloned()
}

/// The text of today's note when it's first made.
pub fn daily_template(day_label: &str) -> String {
    format!("# {day_label}\n\n## Hunting\n\n- \n\n## Notes\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_tags_links_and_headings() {
        let t = "---\ntags: [hunt, gible]\nrun: 12\nalso:\n  - one\n  - two\n---\n# Gible on Route 206\nStarted #shiny-hunt and [[Route 214]] then [[Hunts/Beldum|Beldum]].\n## Tonight\n- [ ] #todo\n```\n# not a heading #nottag\n```\nIssue #1 isn't a tag\n";
        let (props, at) = front_matter(t);
        assert_eq!(props[0], ("tags".to_string(), "hunt, gible".to_string()));
        assert_eq!(props[1], ("run".to_string(), "12".to_string()));
        assert_eq!(props[2], ("also".to_string(), "one, two".to_string()));
        assert!(t[at..].starts_with("# Gible"));
        let tg = tags(t);
        assert!(tg.contains("hunt") && tg.contains("gible") && tg.contains("shiny-hunt") && tg.contains("todo"));
        assert!(!tg.contains("nottag") && !tg.contains("1"));
        assert_eq!(links(t), vec!["Route 214".to_string(), "Hunts/Beldum".to_string()]);
        let h = headings(t);
        assert_eq!(h.iter().map(|x| (x.0, x.1.as_str())).collect::<Vec<_>>(), vec![(1, "Gible on Route 206"), (2, "Tonight")]);
        assert!(words(t) > 5);
    }

    #[test]
    fn no_front_matter_and_unclosed_block() {
        assert_eq!(front_matter("# Hi\n").1, 0);
        assert_eq!(front_matter("---\ntags: a\n# never closed\n"), (Vec::new(), 0));
    }

    #[test]
    fn links_find_notes_by_name_or_path() {
        let notes = vec!["Hunts/Gible.md".to_string(), "Gible.md".to_string(), "Daily/2026-11-14.md".to_string()];
        assert_eq!(resolve("Hunts/Gible", &notes).as_deref(), Some("Hunts/Gible.md"));
        assert_eq!(resolve("gible", &notes).as_deref(), Some("Gible.md"));
        assert_eq!(resolve("2026-11-14", &notes).as_deref(), Some("Daily/2026-11-14.md"));
        assert_eq!(resolve("Nope", &notes), None);
    }

    #[test]
    fn paths_stay_inside_the_journal() {
        let r = Path::new("/j");
        assert!(path_of(r, "../x.md").is_none());
        assert!(path_of(r, "a/../../x.md").is_none());
        assert!(path_of(r, "/abs.md").is_none());
        assert!(path_of(r, "a\\b.md").is_none());
        assert_eq!(path_of(r, "Hunts/Gible.md"), Some(PathBuf::from("/j/Hunts/Gible.md")));
        assert_eq!(clean_name("a/b:c?"), "a b c");
        assert_eq!(clean_name("  ..  "), "Untitled");
    }

    #[test]
    fn files_tree_write_rename_and_trash() {
        let dir = std::env::temp_dir().join(format!("journal-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "Hunts/Gible.md", "# Gible").unwrap();
        write(&dir, "Ideas.md", "x").unwrap();
        let rel = free_rel(&dir, "Hunts", "Gible");
        assert_eq!(rel, "Hunts/Gible 2.md");
        let t = tree(&dir);
        assert_eq!(all_notes(&t), vec!["Hunts/Gible.md".to_string(), "Ideas.md".to_string()]);
        assert_eq!(read(&dir, "Hunts/Gible.md").as_deref(), Some("# Gible"));
        rename(&dir, "Ideas.md", "Hunts/Ideas.md").unwrap();
        assert!(rename(&dir, "Hunts/Ideas.md", "Hunts/Gible.md").is_err());
        let gone = trash(&dir, "Hunts/Ideas.md").unwrap();
        assert!(gone.starts_with(".trash/"));
        assert_eq!(all_notes(&tree(&dir)), vec!["Hunts/Gible.md".to_string()]);
        // canvases and pictures are listed, but aren't notes
        write(&dir, "Boards/Plan.canvas", "{}").unwrap();
        write_bytes(&dir, "Attachments/Gible 2412.png", &[1, 2, 3]).unwrap();
        let t = tree(&dir);
        assert_eq!(all_canvases(&t), vec!["Boards/Plan.canvas".to_string()]);
        assert_eq!(all_images(&t), vec!["Attachments/Gible 2412.png".to_string()]);
        assert_eq!(all_notes(&t), vec!["Hunts/Gible.md".to_string()]);
        assert_eq!(title_of("Boards/Plan.canvas"), "Plan");
        assert_eq!(free_rel_ext(&dir, "Boards", "Plan", "canvas"), "Boards/Plan 2.canvas");
        let imgs = all_images(&t);
        assert_eq!(resolve_image("Gible%202412.png", &imgs).as_deref(), Some("Attachments/Gible 2412.png"));
        assert_eq!(resolve_image("attachments/gible 2412.png", &imgs).as_deref(), Some("Attachments/Gible 2412.png"));
        assert!(!dir.join("Attachments/Gible 2412.png.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
