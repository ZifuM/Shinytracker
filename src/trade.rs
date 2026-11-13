//! Card trades between players, with no server. Each player has a Trainer
//! ID. To give a card, the sender types the friend's Trainer ID and gets a
//! code; the card leaves their binder. The code works once, and only in the
//! app with that Trainer ID.
//!
//! Every trade is written to a trade record, sealed (unreadable, and any
//! change to it is spotted) and kept in three places that repair each other.
//! The card save keeps a count of the trades it has seen, so restoring an
//! older save replays what happened since, and wiping the record shows up.
//!
//! Without a server nothing can stop someone who rolls back every copy at
//! once; this keeps honest players safe and makes the easy cheats useless.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ───────────────────────────── sealing ─────────────────────────────

/// The app's own key for codes and the record. (Anyone digging through the
/// program could find it: this is a lock against casual edits, not a vault.)
const KEY: [u64; 4] = [0x7f4a_9c21_d3e8_5b06, 0x1c3d_5e7f_8a9b_0c2d, 0xe5d4_c3b2_a190_8f7e, 0x2468_ace0_1357_9bdf];

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A keyed check-sum of `data` (`lane` keeps codes and the record apart).
fn mac(data: &[u8], lane: u64) -> u64 {
    let mut h = KEY[0] ^ lane;
    let mut g = KEY[1].wrapping_add(lane.rotate_left(17));
    for chunk in data.chunks(8) {
        let mut w = [0u8; 8];
        w[..chunk.len()].copy_from_slice(chunk);
        let v = u64::from_le_bytes(w) ^ (chunk.len() as u64) << 56;
        h = mix(h ^ v ^ KEY[2]);
        g = mix(g.wrapping_add(v).rotate_left(23) ^ KEY[3]);
    }
    mix(h ^ g.rotate_left(29) ^ data.len() as u64)
}

/// A stream of bytes from `seed`, to scramble with.
fn stream(seed: u64, n: usize) -> Vec<u8> {
    let mut s = seed ^ KEY[2];
    let mut out = Vec::with_capacity(n + 8);
    while out.len() < n {
        s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        out.extend_from_slice(&mix(s ^ KEY[3]).to_le_bytes());
    }
    out.truncate(n);
    out
}

fn xor(data: &mut [u8], seed: u64) {
    let ks = stream(seed, data.len());
    for (b, k) in data.iter_mut().zip(ks) {
        *b ^= k;
    }
}

/// A fresh random-ish number (time, the process and a counter, mixed).
pub fn fresh() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let local = 0u8;
    let addr = &local as *const u8 as u64;
    mix(t ^ mix(std::process::id() as u64 ^ addr.rotate_left(32)) ^ mix(N.fetch_add(1, Ordering::Relaxed) + 1))
}

// ───────────────────────────── base 32 ─────────────────────────────

/// Crockford's letters: no I, L, O or U, so codes are easy to read out.
const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn b32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn b32_value(c: char) -> Option<u32> {
    let c = match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        c => c,
    };
    B32.iter().position(|b| *b as char == c).map(|i| i as u32)
}

/// Decode `n` bytes, ignoring dashes and spaces (and a "PTC"/"TR" prefix).
fn b32_decode(s: &str, n: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.chars().filter(|c| c.is_ascii_alphanumeric()) {
        acc = (acc << 5) | b32_value(c)?;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    (out.len() == n).then_some(out)
}

fn grouped(s: &str) -> String {
    s.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join("-")
}

// ───────────────────────────── Trainer IDs ─────────────────────────────

/// 40 bits, never 0.
pub fn new_trainer_id() -> u64 {
    loop {
        let v = fresh() & 0xFF_FFFF_FFFF;
        if v != 0 {
            return v;
        }
    }
}

/// "TR-ABCD-EFGH"
pub fn fmt_id(id: u64) -> String {
    format!("TR-{}", grouped(&b32_encode(&id.to_be_bytes()[3..])))
}

/// Reads a Trainer ID however it was pasted ("tr-abcd-efgh", "ABCDEFGH"…).
pub fn parse_id(s: &str) -> Option<u64> {
    let t = s.trim().trim_start_matches(|c: char| c.is_whitespace());
    let t = t.strip_prefix("TR").or_else(|| t.strip_prefix("tr")).or_else(|| t.strip_prefix("Tr")).unwrap_or(t);
    let b = b32_decode(t, 5)?;
    let mut w = [0u8; 8];
    w[3..].copy_from_slice(&b);
    let v = u64::from_be_bytes(w);
    (v != 0).then_some(v)
}

// ───────────────────────────── trade codes ─────────────────────────────

/// One card on its way from one trainer to another.
#[derive(Clone, Debug, PartialEq)]
pub struct Gift {
    /// The card's id, e.g. "base1-4".
    pub card: String,
    pub from: u64,
    pub to: u64,
    /// The sender's trade number: (from, serial) names this one gift.
    pub serial: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodeError {
    /// Not a code at all (typo, cut short).
    Garbled,
    /// It was changed after it was made.
    Tampered,
    /// A card this app doesn't know.
    UnknownCard,
}

const CODE_LANE: u64 = 0xC0DE;

/// The code for a gift: "PTC-XXXX-…" (36 letters and numbers).
pub fn make_code(g: &Gift) -> Option<String> {
    let c = crate::cards::by_id(&g.card)?;
    let set = crate::cards::SETS.iter().position(|s| s.id == c.set)? as u8;
    let mut body = Vec::with_capacity(16);
    body.push(1u8); // version
    body.push(set);
    body.push(c.num as u8);
    body.extend_from_slice(&g.from.to_be_bytes()[3..]);
    body.extend_from_slice(&g.to.to_be_bytes()[3..]);
    body.extend_from_slice(&g.serial.to_be_bytes()[1..]);
    let tag = mac(&body, CODE_LANE);
    xor(&mut body, tag & 0xFFFF_FFFF_FFFF);
    body.extend_from_slice(&tag.to_be_bytes()[2..]);
    Some(format!("PTC-{}", grouped(&b32_encode(&body))))
}

/// Read a pasted code.
pub fn read_code(s: &str) -> Result<Gift, CodeError> {
    let t = s.trim();
    let t = t.strip_prefix("PTC").or_else(|| t.strip_prefix("ptc")).unwrap_or(t);
    let raw = b32_decode(t, 22).ok_or(CodeError::Garbled)?;
    let mut body = raw[..16].to_vec();
    let mut tb = [0u8; 8];
    tb[2..].copy_from_slice(&raw[16..]);
    let tag = u64::from_be_bytes(tb);
    xor(&mut body, tag);
    if mac(&body, CODE_LANE) & 0xFFFF_FFFF_FFFF != tag || body[0] != 1 {
        return Err(CodeError::Tampered);
    }
    let set = crate::cards::SETS.get(body[1] as usize).ok_or(CodeError::UnknownCard)?;
    let card = crate::cards::of_set(set.id).into_iter().find(|c| c.num == body[2] as u16).ok_or(CodeError::UnknownCard)?;
    let id5 = |b: &[u8]| {
        let mut w = [0u8; 8];
        w[3..].copy_from_slice(b);
        u64::from_be_bytes(w)
    };
    let mut sb = [0u8; 4];
    sb[1..].copy_from_slice(&body[13..16]);
    Ok(Gift { card: card.id.clone(), from: id5(&body[3..8]), to: id5(&body[8..13]), serial: u32::from_be_bytes(sb) })
}

// ───────────────────────────── the trade record ─────────────────────────────

/// One trade, in the order they happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entry {
    Sent { card: String, to: u64, serial: u32, when: u64, code: String },
    Got { card: String, from: u64, serial: u32, when: u64 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Book {
    /// This player's Trainer ID.
    pub me: u64,
    /// The last trade number used for a code.
    pub serial: u32,
    pub log: Vec<Entry>,
}

impl Book {
    pub fn new(me: u64) -> Self {
        Book { me, serial: 0, log: Vec::new() }
    }

    /// This gift was claimed here already.
    pub fn has(&self, from: u64, serial: u32) -> bool {
        self.log.iter().any(|e| matches!(e, Entry::Got { from: f, serial: s, .. } if *f == from && *s == serial))
    }
}

const BOOK_LANE: u64 = 0xB00C;
const MAGIC: &[u8; 6] = b"PTCTR1";

/// Sealed bytes for the record: magic, a fresh nonce, the scrambled JSON and
/// a check over it all.
pub fn seal(b: &Book) -> Vec<u8> {
    let mut body = serde_json::to_vec(b).unwrap_or_default();
    let nonce = fresh();
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&nonce.to_le_bytes());
    let tag = {
        let mut v = nonce.to_le_bytes().to_vec();
        v.extend_from_slice(&body);
        mac(&v, BOOK_LANE)
    };
    xor(&mut body, nonce ^ tag);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// The record from sealed bytes; None if it isn't one or was changed.
pub fn unseal(bytes: &[u8]) -> Option<Book> {
    if bytes.len() < 22 || &bytes[..6] != MAGIC {
        return None;
    }
    let nonce = u64::from_le_bytes(bytes[6..14].try_into().ok()?);
    let tag = u64::from_le_bytes(bytes[14..22].try_into().ok()?);
    let mut body = bytes[22..].to_vec();
    xor(&mut body, nonce ^ tag);
    let mut v = nonce.to_le_bytes().to_vec();
    v.extend_from_slice(&body);
    if mac(&v, BOOK_LANE) != tag {
        return None;
    }
    serde_json::from_slice(&body).ok()
}

/// The three places the record is kept: next to the card save, in the
/// backups folder, and in the download cache.
pub fn record_paths() -> Vec<PathBuf> {
    let data = crate::config::data_dir();
    vec![data.join("cards-trades.dat"), data.join("backups").join("cards-trades.dat"), crate::config::cache_dir().join("cards-trades.dat")]
}

/// What reading the record found.
#[derive(Clone, Debug, Default)]
pub struct Found {
    /// The longest good copy.
    pub book: Option<Book>,
    /// Some file was there (good or not).
    pub any_file: bool,
    /// A file was there that wasn't a good record (changed or damaged).
    pub bad_file: bool,
}

pub fn load() -> Found {
    load_from(&record_paths())
}

pub fn load_from(paths: &[PathBuf]) -> Found {
    let mut f = Found::default();
    for p in paths {
        let Ok(bytes) = std::fs::read(p) else { continue };
        f.any_file = true;
        match unseal(&bytes) {
            Some(b) => {
                if f.book.as_ref().map_or(true, |have| b.log.len() > have.log.len() || (b.log.len() == have.log.len() && b.serial > have.serial)) {
                    f.book = Some(b);
                }
            }
            None => f.bad_file = true,
        }
    }
    f
}

/// Write every copy (each written whole, then swapped in). Ok if at least
/// one copy was written.
pub fn save(b: &Book) -> Result<(), String> {
    save_to(&record_paths(), b)
}

pub fn save_to(paths: &[PathBuf], b: &Book) -> Result<(), String> {
    let bytes = seal(b);
    let mut ok = 0;
    let mut err = String::new();
    for p in paths {
        let Some(dir) = p.parent() else { continue };
        let _ = std::fs::create_dir_all(dir);
        let tmp = p.with_extension("dat.tmp");
        match std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, p)) {
            Ok(()) => ok += 1,
            Err(e) => err = e.to_string(),
        }
    }
    if ok > 0 {
        Ok(())
    } else {
        Err(format!("Couldn't save your trade record: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints a code for testing by hand: TRADE_TO=TR-… cargo test make_test_code -- --ignored --nocapture
    #[test]
    #[ignore]
    fn make_test_code() {
        let to = parse_id(&std::env::var("TRADE_TO").unwrap_or_default()).expect("TRADE_TO");
        let card = std::env::var("TRADE_CARD").unwrap_or_else(|_| "base4-10".into());
        println!("CODE {}", make_code(&Gift { card, from: 0x12_3456_789A, to, serial: 1 }).unwrap());
    }

    #[test]
    fn ids_round_trip() {
        for _ in 0..50 {
            let id = new_trainer_id();
            let s = fmt_id(id);
            assert!(s.starts_with("TR-") && s.len() == 12, "{s}");
            assert_eq!(parse_id(&s), Some(id));
            assert_eq!(parse_id(&s.to_lowercase()), Some(id));
            assert_eq!(parse_id(&s.replace('-', "")), Some(id));
        }
        assert_eq!(parse_id("hello"), None);
    }

    #[test]
    fn codes_round_trip_and_catch_edits() {
        let g = Gift { card: "base1-4".into(), from: 0x12_3456_789A, to: 0xAB_CDEF_0123, serial: 7 };
        let code = make_code(&g).unwrap();
        assert!(code.starts_with("PTC-"), "{code}");
        assert_eq!(read_code(&code), Ok(g.clone()));
        assert_eq!(read_code(&code.to_lowercase()), Ok(g.clone()));
        assert_eq!(read_code(&format!("  {code}  ")), Ok(g.clone()));
        // change one letter: it no longer reads
        let mut bad: Vec<char> = code.chars().collect();
        let i = 10;
        bad[i] = if bad[i] == 'A' { 'B' } else { 'A' };
        let bad: String = bad.into_iter().collect();
        assert!(read_code(&bad).is_err());
        assert_eq!(read_code("PTC-1234"), Err(CodeError::Garbled));
        // a Base Set 2 card too
        let g2 = Gift { card: "base4-130".into(), from: 1, to: 2, serial: 0xFF_FFFF };
        assert_eq!(read_code(&make_code(&g2).unwrap()), Ok(g2));
    }

    #[test]
    fn record_seals_and_picks_the_longest_copy() {
        let dir = std::env::temp_dir().join(format!("ptc-trade-test-{}", fresh()));
        let paths = vec![dir.join("a.dat"), dir.join("b").join("b.dat"), dir.join("c.dat")];
        let mut b = Book::new(42);
        b.log.push(Entry::Got { card: "base1-4".into(), from: 9, serial: 1, when: 1 });
        save_to(&paths, &b).unwrap();
        // one copy goes back to an older state, one is edited by hand
        std::fs::write(&paths[0], seal(&Book::new(42))).unwrap();
        let mut edited = std::fs::read(&paths[2]).unwrap();
        let n = edited.len();
        edited[n - 3] ^= 1;
        std::fs::write(&paths[2], edited).unwrap();
        let f = load_from(&paths);
        assert!(f.any_file && f.bad_file);
        assert_eq!(f.book, Some(b.clone()));
        assert!(f.book.unwrap().has(9, 1));
        // nothing readable to anyone: no plain text in the file
        let raw = std::fs::read(&paths[1]).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("base1-4"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
