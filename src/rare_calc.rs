//! Rare grass, worked out the way Platinum Together does it, so the app
//! knows every patch of the day without reading the game.
//!
//! - The day: the server's date (Central European time), made into a
//!   number: yy·372 + mm·31 + dd + 0x5EED, scrambled. So the grass changes
//!   at the daily reset, midnight CET, for everyone at once.
//! - Each 3×3 block of tiles gets a number from its position and the day. A
//!   tall-grass tile is rare grass when that number ends 000–024 (out of
//!   1000): about 2.5% of the grass each day.
//! - The Pokémon: picked from a list by the tile's number. 27 species
//!   anywhere, 25 more where the area's wild Pokémon go above level 15, 10
//!   more above 29, and with the MODERN POKéMON option on, up to 20 newer
//!   ones (each from its own level). The game's evolve roll may still evolve
//!   it when you meet it.
//!
//! Where the tall grass is (and each area's level) comes bundled with the
//! app (`rare_tiles.bin`, made once from the game's map files). If a game
//! update changes the maps or the formula, it needs making again.
//! Checked against the rare grass the game logged (every spot of 12 Nov 2026).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Every tall-grass tile: map (0 outdoors, else a side area's), x, y, area
/// (0xFFFF none) and the area's highest grass level. 9 bytes each.
static TILES: &[u8] = include_bytes!("rare_tiles.bin");

/// The game's scramble (an integer hash).
pub fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

/// The number for a date (two-digit year, month, day).
pub fn day_seed(yy: u32, mm: u32, dd: u32) -> u32 {
    mix(yy.wrapping_mul(372).wrapping_add(mm * 31).wrapping_add(dd).wrapping_add(0x5EED))
}

/// Division by 3 the way the game does it for tile blocks.
fn block(v: i32) -> i32 {
    if v >= 0 {
        v / 3
    } else {
        (v - 2) / 3
    }
}

/// A tile's number for the day.
pub fn tile_hash(seed: u32, x: i32, y: i32) -> u32 {
    let h = (block(x) as u32).wrapping_mul(0x0466_F45D) ^ (block(y) as u32).wrapping_mul(0x0127_409F);
    mix(mix(h) ^ seed)
}

/// Whether a tall-grass tile is rare grass that day.
pub fn is_rare(seed: u32, x: i32, y: i32) -> bool {
    tile_hash(seed, x, y) % 1000 < 25
}

// The game's lists (game species numbers; the newer ones are its own).
const BASE: [u16; 27] = [147, 246, 371, 374, 443, 133, 280, 1, 4, 7, 152, 155, 158, 252, 255, 258, 175, 172, 387, 390, 393, 63, 92, 328, 333, 240, 239];
const OVER_15: [u16; 25] = [447, 123, 127, 214, 228, 359, 227, 446, 58, 37, 361, 408, 410, 215, 200, 198, 236, 442, 207, 193, 355, 353, 95, 108, 114];
const OVER_29: [u16; 10] = [131, 142, 137, 132, 128, 115, 241, 113, 143, 357];
/// (species, from area level)
const MODERN: [(u16, u8); 20] = [
    (584, 0), (577, 0), (580, 0), (539, 0), (628, 0), (594, 0), (624, 0), (585, 17), (578, 16), (581, 16),
    (540, 16), (583, 16), (618, 25), (621, 30), (629, 35), (625, 35), (586, 45), (579, 45), (582, 45), (541, 45),
];

/// The Pokémon on a rare grass tile (game species number). `level` is the
/// area's highest grass encounter level.
pub fn species(seed: u32, x: i32, y: i32, level: u32, modern: bool) -> u16 {
    let nb = if level > 15 { OVER_15.len() } else { 0 };
    let nc = if level > 29 { OVER_29.len() } else { 0 };
    let base = BASE.len() + nb + nc;
    let newer: Vec<u16> = if modern { MODERN.iter().filter(|(_, m)| level >= *m as u32).map(|(s, _)| *s).collect() } else { Vec::new() };
    let n = base + newer.len();
    let i = (mix(tile_hash(seed, x, y) ^ 0xA_11CE) % n as u32) as usize;
    if i < BASE.len() {
        BASE[i]
    } else if i < BASE.len() + nb {
        OVER_15[i - BASE.len()]
    } else if i < base {
        OVER_29[i - BASE.len() - nb]
    } else {
        newer[i - base]
    }
}

/// A tall-grass tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub matrix: u16,
    pub x: u16,
    pub y: u16,
    pub zone: Option<u16>,
    pub level: u8,
}

fn tiles() -> impl Iterator<Item = Tile> {
    TILES.chunks_exact(9).map(|b| {
        let u = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let zone = u(6);
        Tile { matrix: u(0), x: u(2), y: u(4), zone: (zone != 0xFFFF).then_some(zone), level: b[8] }
    })
}

/// One rare grass tile on a day, and its Pokémon (game species number).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub tile: Tile,
    pub species: u16,
}

/// Whether the player has the MODERN POKéMON world option on (set by the
/// app from its settings; the Map's worker reads it).
static MODERN_ON: AtomicBool = AtomicBool::new(true);

pub fn set_modern(on: bool) {
    MODERN_ON.store(on, Ordering::Relaxed);
}

pub fn modern() -> bool {
    MODERN_ON.load(Ordering::Relaxed)
}

/// Every rare grass tile on a date (days since 1970 on the calendar).
pub fn spots_on(date: i64, modern: bool) -> Vec<Spot> {
    let (y, m, d) = crate::stats::civil_from_days(date);
    let seed = day_seed(y.rem_euclid(100) as u32, m, d);
    let mut out: Vec<Spot> = tiles()
        .filter(|t| is_rare(seed, t.x as i32, t.y as i32))
        .map(|t| Spot { tile: t, species: species(seed, t.x as i32, t.y as i32, t.level as u32, modern) })
        .collect();
    // The lakes have two maps each (before and after the story changes
    // them), on the same tiles: keep one, the later map.
    out.sort_by_key(|s| std::cmp::Reverse(s.tile.matrix));
    let mut seen = std::collections::HashSet::new();
    out.retain(|s| s.tile.matrix == 0 || seen.insert((s.tile.zone.and_then(zone_name), s.tile.x, s.tile.y)));
    out.sort_by_key(|s| (s.tile.matrix, s.tile.y, s.tile.x));
    out
}

/// The server's date at `now` (days since 1970): the rare grass day.
pub fn today(now: i64) -> i64 {
    crate::bingo::card_day(now)
}

/// When today's rare grass started (the daily reset, midnight CET).
pub fn day_start(now: i64) -> i64 {
    crate::gamemap::last_reset(now)
}

/// When it changes next (the next reset).
pub fn next_change(now: i64) -> i64 {
    crate::gamemap::next_reset(now)
}

/// Game species number → the wiki's id, for the newer Pokémon (the rest are
/// their National Dex number). From the bundled area list.
pub fn game_species_id(sp: u32) -> Option<&'static str> {
    static MAP: OnceLock<std::collections::HashMap<u32, String>> = OnceLock::new();
    let m = MAP.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Z {
            #[serde(rename = "gameSpecies", default)]
            game_species: std::collections::HashMap<String, String>,
        }
        serde_json::from_str::<Z>(crate::gamemap::ZONES_JSON)
            .map(|z| z.game_species.into_iter().filter_map(|(k, v)| Some((k.parse().ok()?, v))).collect())
            .unwrap_or_default()
    });
    m.get(&sp).map(|s| s.as_str())
}

/// An area's name from the bundled area list (when the game's map isn't
/// loaded).
pub fn zone_name(id: u16) -> Option<&'static str> {
    static MAP: OnceLock<std::collections::HashMap<u16, String>> = OnceLock::new();
    let m = MAP.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Zn {
            id: u16,
            name: String,
        }
        #[derive(serde::Deserialize)]
        struct Z {
            zones: Vec<Zn>,
        }
        serde_json::from_str::<Z>(crate::gamemap::ZONES_JSON).map(|z| z.zones.into_iter().map(|z| (z.id, z.name)).collect()).unwrap_or_default()
    });
    m.get(&id).map(|s| s.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_game_log() {
        // From the game's log: species and area level on 11 Nov 2026.
        let s = day_seed(26, 11, 11);
        assert!(is_rare(s, 313, 250));
        assert_eq!(species(s, 313, 250, 35, true), 374);
        assert_eq!(species(s, 281, 529, 15, true), 4);
        assert_eq!(species(s, 151, 857, 3, true), 580);
        assert_eq!(species(s, 178, 828, 4, true), 539);
        assert_eq!(species(s, 313, 250, 35, false), 236);
    }

    #[test]
    fn the_shared_list_of_12_nov_comes_out_exactly() {
        // Every spot met on 12 Nov 2026 (the shared list), tile and Pokémon.
        let met: [(u16, u16, u16); 8] = [(21, 18, 227), (23, 18, 227), (25, 55, 258), (31, 48, 252), (46, 33, 7), (47, 46, 175), (47, 47, 393), (176, 590, 175)];
        let date = crate::stats::days_from_civil(2026, 11, 12);
        let all = spots_on(date, true);
        for (x, y, sp) in met {
            assert!(all.iter().any(|s| s.tile.x == x && s.tile.y == y && s.species == sp), "{x},{y} {sp}");
        }
        assert!((150..400).contains(&all.len()), "{} spots", all.len());
        let tiles = TILES.len() / 9;
        assert_eq!(TILES.len() % 9, 0);
        assert!(tiles > 9000, "{tiles} tall-grass tiles bundled");
    }

    #[test]
    fn a_day_is_the_server_date() {
        // 14 Nov 2026 09:00 in Sydney is still 13 Nov on the server (CET).
        let now = 1_794_607_200;
        assert_eq!(crate::worldclock::civil_from_days(today(now)), (2026, 11, 13));
        assert_eq!(next_change(now) - day_start(now), 86_400);
        assert!(day_start(now) <= now && now < next_change(now));
        assert_eq!(game_species_id(577).is_some(), true);
        assert!(zone_name(65).is_some());
    }
}
