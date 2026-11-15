//! Reading Platinum Together's save file, READ ONLY. The game's rules
//! (Kai's four) are kept: nothing here writes to the game or its files,
//! patches or decompiles anything, or touches the game's memory. The file
//! is read into memory as plain bytes and decoded in the standard, publicly
//! documented Pokémon Platinum save format (its own checksums say whether a
//! record is whole). `Platinum.exe` is never read.
//!
//! Also read here: the game's English text archive (msgdata/pl_msg.narc),
//! for the names of the original moves, abilities and items.

use std::path::Path;

/// A half of the file (the game keeps two copies and writes them in turn).
const HALF: usize = 0x40000;
/// Where things are in this game's save (Platinum's layout, 12 bytes later).
const NAME: usize = 0x74;
const TID: usize = 0x84;
const MONEY: usize = 0x88;
const BADGES: usize = 0x8E;
const PARTY_COUNT: usize = 0xA8;
const PARTY: usize = 0xAC;
const PARTY_SIZE: usize = 236;
const BOX_SIZE: usize = 136;
const DAYCARE: usize = 0x1660;
const DAYCARE_STRIDE: usize = 240;
const GENERAL_FOOTER_COUNT: usize = 0xCF30;
const BOXES: usize = 0xCF48;
const BOX_NAMES: usize = BOXES + 18 * 30 * BOX_SIZE;
/// This game's extra boxes (19 on): their names, the data just before.
const MORE_BOX_NAMES: usize = 0x35F70;
pub const SLOTS: usize = 30;
/// The bag's pockets: (name, offset, slots).
const POCKETS: [(&str, usize, usize); 8] = [
    ("Items", 0x63C, 165),
    ("Key Items", 0x8D0, 50),
    ("TMs & HMs", 0x998, 100),
    ("Mail", 0xB28, 12),
    ("Medicine", 0xB58, 40),
    ("Berries", 0xBF8, 64),
    ("Poké Balls", 0xCF8, 15),
    ("Battle Items", 0xD34, 30),
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mon {
    pub pid: u32,
    /// The game's species number (the Pokédex number up to #493).
    pub species: u16,
    pub item: u16,
    pub tid: u16,
    pub sid: u16,
    pub exp: u32,
    /// Friendship (for an Egg: the cycles left before it hatches).
    pub friendship: u8,
    pub ability: u8,
    /// The game language it came from (2 English, 1 Japanese, 3 French,
    /// 4 Italian, 5 German, 7 Spanish, 8 Korean): different parents'
    /// languages make the Masuda Method.
    pub language: u8,
    pub evs: [u8; 6],
    pub ivs: [u8; 6],
    pub moves: [u16; 4],
    pub egg: bool,
    pub nicknamed: bool,
    pub female: bool,
    pub genderless: bool,
    pub form: u8,
    pub nickname: String,
    pub ot: String,
    pub ball: u8,
    pub met_level: u8,
    /// (year since 2000, month, day)
    pub met_date: Option<(u8, u8, u8)>,
    pub met_location: u16,
    /// Party Pokémon only (box ones: worked out from `exp`).
    pub level: Option<u8>,
    /// Party Pokémon only: the stats the game shows, as HP (max), Atk, Def,
    /// SpA, SpD, Spe. Box ones are worked out.
    pub stats: Option<[u16; 6]>,
}

impl Mon {
    pub fn shiny(&self) -> bool {
        (self.tid ^ self.sid ^ (self.pid >> 16) as u16 ^ (self.pid & 0xFFFF) as u16) < 8
    }
    /// 0-24, Hardy … Quirky.
    pub fn nature(&self) -> u8 {
        (self.pid % 25) as u8
    }
}

pub const NATURES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty", "Bold", "Docile", "Relaxed", "Impish", "Lax", "Timid", "Hasty", "Serious", "Jolly", "Naive", "Modest", "Mild", "Quiet", "Bashful", "Rash", "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];

#[derive(Debug, Clone, Default)]
pub struct GameBox {
    pub name: String,
    pub slots: Vec<Option<Mon>>,
}

#[derive(Debug, Clone, Default)]
pub struct Save {
    pub trainer: String,
    pub tid: u16,
    pub money: u32,
    pub badges: u8,
    pub party: Vec<Mon>,
    pub boxes: Vec<GameBox>,
    /// Pokédex (the first 493): seen and caught.
    pub dex_seen: Vec<u16>,
    pub dex_caught: Vec<u16>,
    pub daycare: Vec<Mon>,
    /// An Egg is waiting at the Day-Care.
    pub egg_waiting: bool,
    /// The Day-Care's step counter (it checks for an Egg, and Eggs you carry
    /// get a step closer to hatching, each time it comes round to 255).
    pub daycare_steps: u8,
    /// (pocket, item number, how many)
    pub bag: Vec<(&'static str, u16, u16)>,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// The game's text: "{x}" for a character this doesn't know.
pub fn text(codes: impl IntoIterator<Item = u16>) -> String {
    let mut s = String::new();
    for c in codes {
        match c {
            0xFFFF => break,
            0x121..=0x12A => s.push((b'0' + (c - 0x121) as u8) as char),
            0x12B..=0x144 => s.push((b'A' + (c - 0x12B) as u8) as char),
            0x145..=0x15E => s.push((b'a' + (c - 0x145) as u8) as char),
            0x1DE | 0x1E2 => s.push(' '),
            0x1AB => s.push('!'),
            0x1AC => s.push('?'),
            0x1AD => s.push(','),
            0x1AE => s.push('.'),
            0x1B3 => s.push('\''),
            0x1BE => s.push('-'),
            0x1C4 => s.push(':'),
            0x188 => s.push('é'),
            0x1BB => s.push('♂'),
            0x1BC => s.push('♀'),
            0 => {}
            _ => {}
        }
    }
    s
}

fn text_at(d: &[u8], o: usize, max_chars: usize) -> String {
    text((0..max_chars).map(|i| u16_at(d, o + 2 * i)))
}

/// Pokémon records' scrambling (the standard Gen 4 one).
fn crypt(data: &mut [u8], seed: u32) {
    let mut s = seed;
    for ch in data.chunks_exact_mut(2) {
        s = s.wrapping_mul(0x41C6_4E6D).wrapping_add(0x6073);
        let k = (s >> 16) as u16;
        let w = u16::from_le_bytes([ch[0], ch[1]]) ^ k;
        ch.copy_from_slice(&w.to_le_bytes());
    }
}

const ORDERS: [&str; 24] = [
    "ABCD", "ABDC", "ACBD", "ACDB", "ADBC", "ADCB", "BACD", "BADC", "BCAD", "BCDA", "BDAC", "BDCA", "CABD", "CADB", "CBAD", "CBDA", "CDAB", "CDBA", "DABC", "DACB", "DBAC", "DBCA", "DCAB", "DCBA",
];

/// One Pokémon record (136 bytes, or 236 with battle stats). None: empty or
/// not whole (its checksum doesn't match).
pub fn decode(raw: &[u8]) -> Option<Mon> {
    if raw.len() < BOX_SIZE {
        return None;
    }
    let pid = u32_at(raw, 0);
    let chk = u16_at(raw, 6);
    if pid == 0 && chk == 0 {
        return None;
    }
    let mut body = raw[8..136].to_vec();
    crypt(&mut body, chk as u32);
    let sum = body.chunks_exact(2).fold(0u16, |a, c| a.wrapping_add(u16::from_le_bytes([c[0], c[1]])));
    if sum != chk {
        return None;
    }
    let order = ORDERS[(((pid & 0x3E000) >> 13) % 24) as usize].as_bytes();
    let block = |name: u8| {
        let i = order.iter().position(|b| *b == name).unwrap_or(0);
        &body[i * 32..i * 32 + 32]
    };
    let (a, b, c, d) = (block(b'A'), block(b'B'), block(b'C'), block(b'D'));
    let species = u16_at(a, 0);
    if species == 0 {
        return None;
    }
    let iv = u32_at(b, 0x10);
    let fl = b[0x18];
    let md = (d[0x13], d[0x14], d[0x15]);
    let mut m = Mon {
        pid,
        species,
        item: u16_at(a, 2),
        tid: u16_at(a, 4),
        sid: u16_at(a, 6),
        exp: u32_at(a, 8),
        friendship: a[0x0C],
        ability: a[0x0D],
        language: a[0x0F],
        evs: [a[0x10], a[0x11], a[0x12], a[0x13], a[0x14], a[0x15]],
        ivs: std::array::from_fn(|i| ((iv >> (5 * i)) & 31) as u8),
        moves: [u16_at(b, 0), u16_at(b, 2), u16_at(b, 4), u16_at(b, 6)],
        egg: (iv >> 30) & 1 == 1,
        nicknamed: (iv >> 31) & 1 == 1,
        female: fl & 2 != 0,
        genderless: fl & 4 != 0,
        form: fl >> 3,
        nickname: text((0..11).map(|i| u16_at(c, 2 * i))),
        ot: text((0..8).map(|i| u16_at(d, 2 * i))),
        ball: d[0x1B],
        met_level: d[0x1C] & 0x7F,
        met_date: (md.1 >= 1 && md.1 <= 12 && md.2 >= 1).then_some(md),
        met_location: u16_at(d, 0x18),
        level: None,
        stats: None,
    };
    if raw.len() >= PARTY_SIZE {
        let mut st = raw[136..236].to_vec();
        crypt(&mut st, pid);
        if (1..=100).contains(&st[4]) {
            m.level = Some(st[4]);
            // max HP, Atk, Def, Spe, SpA, SpD
            let v = |o: usize| u16::from_le_bytes([st[o], st[o + 1]]);
            m.stats = Some([v(8), v(10), v(12), v(16), v(18), v(14)]);
        }
    }
    Some(m)
}

/// The newer half of the file (the game writes the two halves in turn).
fn newest_half(d: &[u8]) -> &[u8] {
    let a = &d[..HALF.min(d.len())];
    if d.len() < 2 * HALF {
        return a;
    }
    let b = &d[HALF..2 * HALF];
    if u32_at(b, GENERAL_FOOTER_COUNT) > u32_at(a, GENERAL_FOOTER_COUNT) {
        b
    } else {
        a
    }
}

/// A box name that reads as one (letters, digits, spaces).
fn plausible_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.chars().all(|c| c.is_alphanumeric() || " .'!?-:é♂♀,".contains(c))
}

/// Read a save file (a copy of its bytes; the file itself is only opened to
/// read).
/// The newest save in the game folder: `save.bin` (written when you save)
/// or `save-auto.bin` (the game's own snapshot every few minutes, so Egg
/// steps and the like keep up while you play). With its time (unix ms).
pub fn newest_save(game_dir: &Path) -> Option<(std::path::PathBuf, u64)> {
    ["save.bin", "save-auto.bin"]
        .iter()
        .filter_map(|n| {
            let p = game_dir.join(n);
            let ms = std::fs::metadata(&p).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
            // a half-written file isn't a save
            (std::fs::metadata(&p).ok()?.len() >= (2 * HALF) as u64).then_some((p, ms))
        })
        .max_by_key(|(_, ms)| *ms)
}

pub fn read(path: &Path) -> Result<Save, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<Save, String> {
    if bytes.len() < HALF {
        return Err("This doesn't look like a Platinum Together save.".into());
    }
    let d = newest_half(bytes);
    let mut s = Save { trainer: text_at(d, NAME, 8), tid: u16_at(d, TID), money: u32_at(d, MONEY), badges: d[BADGES].count_ones() as u8, ..Default::default() };
    let n = (u32_at(d, PARTY_COUNT) as usize).min(6);
    for i in 0..n {
        let o = PARTY + i * PARTY_SIZE;
        if let Some(m) = decode(&d[o..o + PARTY_SIZE]) {
            s.party.push(m);
        }
    }
    if s.party.is_empty() && s.trainer.is_empty() {
        return Err("This doesn't look like a Platinum Together save (no trainer or party found).".into());
    }
    let read_box = |data_at: usize, name: String| GameBox {
        name,
        slots: (0..SLOTS).map(|k| d.get(data_at + k * BOX_SIZE..data_at + (k + 1) * BOX_SIZE).and_then(decode)).collect(),
    };
    for b in 0..18 {
        let name = text_at(d, BOX_NAMES + b * 0x28, 20);
        let name = if plausible_name(&name) { name } else { format!("BOX {}", b + 1) };
        s.boxes.push(read_box(BOXES + b * SLOTS * BOX_SIZE, name));
    }
    // this game's extra boxes: as many names as read as names
    let more = (0..64).take_while(|i| plausible_name(&text_at(d, MORE_BOX_NAMES + i * 0x28, 20))).count();
    if more > 0 {
        let start = MORE_BOX_NAMES - more * SLOTS * BOX_SIZE;
        for b in 0..more {
            s.boxes.push(read_box(start + b * SLOTS * BOX_SIZE, text_at(d, MORE_BOX_NAMES + b * 0x28, 20)));
        }
    }
    // Pokédex: after its marker, caught then seen (493 bits each)
    let magic = 0xBEEF_CAFEu32.to_le_bytes();
    if let Some(i) = d[..0xCF00].windows(4).position(|w| w == magic) {
        let bits = |o: usize| -> Vec<u16> { (0..493usize).filter(|n| d.get(o + n / 8).is_some_and(|b| b >> (n % 8) & 1 == 1)).map(|n| n as u16 + 1).collect() };
        s.dex_caught = bits(i + 4);
        s.dex_seen = bits(i + 4 + 0x40);
    }
    for k in 0..2 {
        let o = DAYCARE + k * DAYCARE_STRIDE;
        if let Some(m) = d.get(o..o + PARTY_SIZE).and_then(decode) {
            s.daycare.push(m);
        }
    }
    s.egg_waiting = s.daycare.len() == 2 && u32_at(d, DAYCARE + 2 * DAYCARE_STRIDE) != 0;
    s.daycare_steps = d.get(DAYCARE + 2 * DAYCARE_STRIDE + 4).copied().unwrap_or(0);
    for (pocket, at, n) in POCKETS {
        for k in 0..n {
            let (item, qty) = (u16_at(d, at + 4 * k), u16_at(d, at + 4 * k + 2));
            if item == 0 {
                break;
            }
            s.bag.push((pocket, item, qty));
        }
    }
    Ok(s)
}

// ───────────────────────────── levels ─────────────────────────────

/// Experience needed for `n` with a growth rate (sim data's names).
pub fn exp_for(growth: &str, n: u32) -> u64 {
    if n <= 1 {
        return 0;
    }
    let n = n as i64;
    let c = n * n * n;
    let v = match growth {
        "fast" => 4 * c / 5,
        "medium" => c,
        "medium-slow" => 6 * c / 5 - 15 * n * n + 100 * n - 140,
        "slow" => 5 * c / 4,
        "slow-then-very-fast" => match n {
            0..=50 => c * (100 - n) / 50,
            51..=68 => c * (150 - n) / 100,
            69..=98 => c * ((1911 - 10 * n) / 3) / 500,
            _ => c * (160 - n) / 100,
        },
        "fast-then-very-slow" => match n {
            0..=15 => c * ((n + 1) / 3 + 24) / 50,
            16..=36 => c * (n + 14) / 50,
            _ => c * (n / 2 + 32) / 50,
        },
        _ => c,
    };
    v.max(0) as u64
}

pub fn level_from_exp(growth: &str, exp: u32) -> u8 {
    let mut lv = 1;
    for n in 2..=100 {
        if exp_for(growth, n) <= exp as u64 {
            lv = n;
        } else {
            break;
        }
    }
    lv as u8
}

// ───────────────────────────── the game's text ─────────────────────────────

/// The newer moves (past #467) by the game's own number. The game's English
/// text stops at #467; these were lined up in order with the Platinum Together
/// wiki's move list by type, category, power, accuracy and PP from the game's
/// move table, and checked against the game's German names (608 Kulleraugen
/// is Baby-Doll Eyes, 644 Überheblichkeit is Power Trip). Moves no Pokémon in
/// the game learns aren't on the wiki, so they aren't here either.
pub const NEW_MOVES: &[(u16, &str)] = &[
    (468, "Fairy Wind"), (469, "Disarming Voice"), (470, "Draining Kiss"), (471, "Play Rough"),
    (472, "Dazzling Gleam"), (473, "Moonblast"), (474, "Hone Claws"), (475, "Wide Guard"),
    (476, "Guard Split"), (477, "Power Split"), (478, "Wonder Room"), (479, "Psyshock"), (480, "Venoshock"),
    (481, "Autotomize"), (482, "Rage Powder"), (484, "Magic Room"), (485, "Smack Down"),
    (486, "Storm Throw"), (488, "Sludge Wave"), (489, "Quiver Dance"), (490, "Heavy Slam"),
    (492, "Electro Ball"), (493, "Soak"), (494, "Flame Charge"), (495, "Coil"), (496, "Low Sweep"),
    (497, "Acid Spray"), (498, "Foul Play"), (499, "Simple Beam"), (500, "Entrainment"), (501, "After You"),
    (502, "Round"), (503, "Echoed Voice"), (505, "Clear Smog"), (506, "Stored Power"), (507, "Quick Guard"),
    (508, "Ally Switch"), (510, "Shell Smash"), (511, "Heal Pulse"), (512, "Hex"), (514, "Shift Gear"),
    (515, "Circle Throw"), (516, "Incinerate"), (517, "Quash"), (518, "Acrobatics"), (519, "Reflect Type"),
    (520, "Retaliate"), (521, "Final Gambit"), (523, "Inferno"), (527, "Volt Switch"), (528, "Struggle Bug"),
    (529, "Bulldoze"), (530, "Frost Breath"), (531, "Dragon Tail"), (532, "Work Up"), (533, "Electroweb"),
    (534, "Wild Charge"), (535, "Drill Run"), (536, "Dual Chop"), (538, "Horn Leech"), (539, "Sacred Sword"),
    (540, "Razor Shell"), (541, "Heat Crash"), (542, "Leaf Tornado"), (544, "Cotton Guard"),
    (545, "Night Daze"), (546, "Psystrike"), (548, "Hurricane"), (551, "Searing Shot"), (556, "Bolt Strike"),
    (557, "Blue Flare"), (558, "Fiery Dance"), (561, "Snarl"), (562, "Icicle Crash"), (563, "V-create"),
    (564, "Fusion Flare"), (565, "Fusion Bolt"), (566, "Flying Press"), (568, "Belch"), (570, "Sticky Web"),
    (571, "Fell Stinger"), (572, "Phantom Force"), (574, "Noble Roar"), (578, "Petal Blizzard"),
    (579, "Freeze-Dry"), (580, "Parting Shot"), (581, "Topsy-Turvy"), (582, "Flower Shield"),
    (584, "Grassy Terrain"), (585, "Misty Terrain"), (587, "Boomburst"), (589, "King’s Shield"),
    (590, "Play Nice"), (591, "Confide"), (592, "Diamond Storm"), (595, "Water Shuriken"),
    (596, "Mystical Fire"), (597, "Spiky Shield"), (598, "Aromatic Mist"), (599, "Eerie Impulse"),
    (600, "Venom Drench"), (602, "Geomancy"), (603, "Magnetic Flux"), (605, "Electric Terrain"),
    (608, "Baby-Doll Eyes"), (609, "Nuzzle"), (611, "Infestation"), (612, "Power-Up Punch"),
    (613, "Oblivion Wing"), (614, "Thousand Arrows"), (615, "Thousand Waves"), (616, "Land’s Wrath"),
    (618, "Origin Pulse"), (619, "Precipice Blades"), (620, "Dragon Ascent"), (623, "First Impression"),
    (627, "Sparkling Aria"), (628, "Ice Hammer"), (630, "High Horsepower"), (631, "Strength Sap"),
    (633, "Leafage"), (635, "Toxic Thread"), (636, "Laser Focus"), (638, "Throat Chop"), (642, "Lunge"),
    (644, "Power Trip"), (647, "Smart Strike"), (650, "Core Enforcer"), (656, "Brutal Swing"),
    (657, "Aurora Veil"), (659, "Fleur Cannon"), (661, "Stomping Tantrum"), (664, "Liquidation"),
    (669, "Tearful Look"), (674, "Plasma Fists"), (693, "Stuff Cheeks"), (694, "No Retreat"),
    (696, "Magic Powder"), (704, "Body Press"), (712, "Breaking Swipe"), (719, "Life Dew"),
    (734, "Skitter Smack"), (740, "Flip Turn"), (742, "Dual Wingbeat"), (745, "Wicked Blow"),
    (756, "Psyshield Bash"), (760, "Mystical Power"), (761, "Raging Fury"), (762, "Wave Crash"),
    (766, "Headlong Rush"), (767, "Barb Barrage"), (777, "Lunar Blessing"), (778, "Take Heart"),
    (781, "Axe Kick"), (784, "Order Up"), (786, "Spicy Extract"), (790, "Glaive Rush"), (794, "Mortal Spin"),
    (798, "Flower Trick"), (801, "Raging Bull"), (808, "Shed Tail"), (809, "Chilly Reception"),
    (810, "Tidy Up"), (811, "Snowscape"), (815, "Hyper Drill"), (816, "Twin Beam"), (817, "Rage Fist"),
    (821, "Gigaton Hammer"), (822, "Comeuppance"), (823, "Aqua Cutter"),
];

/// English names the game itself uses: moves, abilities, items (index = the
/// game's number). Only the original ones are in its text archive.
#[derive(Debug, Clone, Default)]
pub struct GameText {
    pub moves: Vec<String>,
    pub abilities: Vec<String>,
    pub items: Vec<String>,
    pub places: Vec<String>,
    /// Each move's type, category and power, from the game's own move
    /// table (the one the launcher builds), when it's there.
    pub move_info: Vec<MoveInfo>,
    /// The game's own English item list (`kit-item-text.txt`), which has
    /// the newer items too (Egg Warmer, Shiny Charm, Mega Stones…).
    pub kit_items: std::collections::HashMap<u16, String>,
}

/// `kit-item-text.txt`: number, name, description… a line.
fn kit_items(game_dir: &Path) -> std::collections::HashMap<u16, String> {
    let Ok(t) = std::fs::read_to_string(game_dir.join("kit-item-text.txt")) else { return Default::default() };
    t.lines()
        .filter_map(|l| {
            let mut it = l.split('\t');
            let n = it.next()?.trim().parse().ok()?;
            let name = it.next()?.trim();
            (!name.is_empty() && name != "None").then(|| (n, name.to_string()))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MoveInfo {
    pub kind: &'static str,
    /// "Physical", "Special" or "Status".
    pub cat: &'static str,
    pub power: u8,
}

const TYPES: [&str; 19] = [
    "Normal", "Fighting", "Flying", "Poison", "Ground", "Rock", "Bug", "Ghost", "Steel", "???", "Fire", "Water", "Grass", "Electric", "Psychic", "Ice", "Dragon", "Dark", "Fairy",
];

/// The game's move table: 16 bytes a move (category at 2, power 3, type 4).
fn move_table(game_dir: &Path) -> Vec<MoveInfo> {
    let path = game_dir.join("patch-built").join("poketool").join("waza").join("pl_waza_tbl.narc");
    let Ok(d) = std::fs::read(path) else { return Vec::new() };
    let Some(files) = narc_files(&d) else { return Vec::new() };
    files
        .iter()
        .map(|e| match e.get(..5) {
            Some(e) => MoveInfo {
                kind: TYPES.get(e[4] as usize).copied().unwrap_or("???"),
                cat: ["Physical", "Special", "Status"].get(e[2] as usize).copied().unwrap_or(""),
                power: e[3],
            },
            None => MoveInfo::default(),
        })
        .collect()
}

fn narc_files(d: &[u8]) -> Option<Vec<&[u8]>> {
    if d.get(..4)? != b"NARC" {
        return None;
    }
    let off = u16_at(d, 0xC) as usize;
    if d.get(off..off + 4)? != b"BTAF" {
        return None;
    }
    let size = u32_at(d, off + 4) as usize;
    let n = u32_at(d, off + 8) as usize;
    let fat: Vec<(usize, usize)> = (0..n).map(|i| (u32_at(d, off + 12 + 8 * i) as usize, u32_at(d, off + 16 + 8 * i) as usize)).collect();
    let fnt = off + size;
    if d.get(fnt..fnt + 4)? != b"BTNF" {
        return None;
    }
    let img = fnt + u32_at(d, fnt + 4) as usize;
    if d.get(img..img + 4)? != b"GMIF" {
        return None;
    }
    let base = img + 8;
    fat.into_iter().map(|(a, b)| d.get(base + a..base + b)).collect()
}

/// One message file's strings (the standard Gen 4 text scrambling).
fn messages(f: &[u8]) -> Vec<String> {
    let num = u16_at(f, 0) as usize;
    let seed = u16_at(f, 2) as u32;
    (0..num)
        .map(|i| {
            let key = (seed.wrapping_mul(0x2FD).wrapping_mul(i as u32 + 1)) & 0xFFFF;
            let k2 = key | (key << 16);
            let o = (u32_at(f, 4 + 8 * i) ^ k2) as usize;
            let l = (u32_at(f, 8 + 8 * i) ^ k2) as usize;
            let mut k = (0x91BD3u32.wrapping_mul(i as u32 + 1)) & 0xFFFF;
            text((0..l.min(200)).map(|j| {
                let c = u16_at(f, o + 2 * j) ^ k as u16;
                k = (k + 0x493D) & 0xFFFF;
                c
            }))
        })
        .collect()
}

/// From the game folder's `msgdata/pl_msg.narc` (read only).
pub fn load_text(game_dir: &Path) -> Option<GameText> {
    let d = std::fs::read(game_dir.join("msgdata").join("pl_msg.narc")).ok()?;
    let files = narc_files(&d)?;
    Some(GameText {
        moves: messages(files.get(647)?),
        abilities: messages(files.get(610)?),
        items: messages(files.get(392)?),
        places: messages(files.get(433)?),
        move_info: move_table(game_dir),
        kit_items: kit_items(game_dir),
    })
}

impl GameText {
    pub fn move_name(&self, n: u16) -> String {
        match self.moves.get(n as usize).filter(|s| !s.is_empty()) {
            Some(s) => s.clone(),
            None => match NEW_MOVES.binary_search_by_key(&n, |m| m.0) {
                Ok(i) => NEW_MOVES[i].1.to_string(),
                Err(_) => format!("Move #{n}"),
            },
        }
    }
    /// Type, category and power, as the game has them.
    pub fn move_info(&self, n: u16) -> Option<MoveInfo> {
        self.move_info.get(n as usize).copied().filter(|m| !m.cat.is_empty() && m.kind != "???")
    }
    pub fn ability_name(&self, n: u8) -> String {
        match self.abilities.get(n as usize).filter(|s| !s.is_empty()) {
            Some(s) => s.clone(),
            None => format!("Ability #{n}"),
        }
    }
    pub fn place_name(&self, n: u16) -> Option<String> {
        self.places.get(n as usize).filter(|s| !s.is_empty() && s.as_str() != "Mystery Zone").cloned()
    }
    pub fn item_name(&self, n: u16) -> String {
        if let Some(s) = self.kit_items.get(&n) {
            return s.clone();
        }
        match self.items.get(n as usize).filter(|s| !s.is_empty() && s.as_str() != "???") {
            Some(s) => s.clone(),
            None => format!("Item #{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "/mnt/user-data/uploads/PlatinumTogether/Game";

    /// Your save, if it's on this machine.
    #[test]
    fn reads_your_save() {
        let p = Path::new(GAME).join("save.bin");
        if !p.exists() {
            return;
        }
        let s = read(&p).unwrap();
        assert_eq!(s.trainer, "Zifu");
        assert_eq!(s.badges, 8);
        assert_eq!(s.party.len(), 6);
        assert_eq!(s.party[0].species, 260);
        assert_eq!(s.party[0].level, Some(87));
        assert_eq!(s.party[0].ivs, [31; 6]);
        assert!(s.boxes.len() >= 18);
        let n: usize = s.boxes.iter().map(|b| b.slots.iter().flatten().count()).sum();
        assert!(n > 100, "{n} in boxes");
        assert!(s.dex_caught.len() > 50 && s.dex_seen.len() > s.dex_caught.len());
        assert!(s.bag.iter().any(|b| b.0 == "Poké Balls" && b.1 == 15));
        let t = load_text(Path::new(GAME)).unwrap();
        assert_eq!(t.move_name(89), "Earthquake");
        assert_eq!(t.ability_name(1), "Stench");
        assert_eq!(t.item_name(4), "Poké Ball");
        // the stats worked out from the species, IVs, EVs and nature match
        // what the game keeps for the party (all but a buffed Swampert)
        let mut same = 0;
        for m in &s.party {
            let nat = crate::species::national_from_game(m.species as u32).unwrap();
            let sp = crate::battle::data().species.iter().find(|x| x.num == nat && x.base.is_empty()).unwrap();
            let ord = |v: [u8; 6]| [v[0], v[1], v[2], v[4], v[5], v[3]].map(|x| x as u16);
            let nature = crate::battle::data().nature(NATURES[m.nature() as usize]);
            let worked = crate::battle::stats(sp.base_stats(crate::battle::World::Platinum), ord(m.ivs), ord(m.evs), m.level.unwrap(), nature, false);
            same += (Some(worked) == m.stats) as usize;
        }
        assert!(same >= 5, "{same} of 6");
        // the newer Pokémon and moves have names
        assert_eq!(crate::species::national_from_game(541), Some(658));
        assert_eq!(t.move_name(608), "Baby-Doll Eyes");
        assert_eq!(t.move_name(644), "Power Trip");
        let fw = t.move_info(608).unwrap();
        assert_eq!((fw.kind, fw.cat), ("Fairy", "Status"));
        assert_eq!(t.move_info(89).unwrap().power, 100);
        assert_eq!(t.item_name(658), "Egg Warmer");
        assert_eq!(t.item_name(134), "Shiny Charm");
        assert!(s.party.iter().all(|m| m.language == 2));
        eprintln!("boxes: {:?}", s.boxes.iter().map(|b| (b.name.clone(), b.slots.iter().flatten().count())).collect::<Vec<_>>());
    }

    /// A backup save from while three Eevee Eggs were being carried.
    #[test]
    fn reads_eggs_and_the_day_care() {
        let p = Path::new(GAME).join("save-backup-1.bin");
        if !p.exists() {
            return;
        }
        let s = read(&p).unwrap();
        let eggs: Vec<&Mon> = s.party.iter().filter(|m| m.egg).collect();
        assert_eq!(eggs.len(), 3);
        assert!(eggs.iter().all(|m| m.species == 133 && m.friendship == 25));
        // Talonflame's Flame Body
        assert!(s.party.iter().any(|m| m.species == 544 && m.ability == 49));
        assert_eq!(s.daycare.len(), 2);
        assert!(s.daycare.iter().all(|m| m.language == 2));
        let t = load_text(Path::new(GAME)).unwrap();
        assert_eq!(crate::breed::hatch_steps(25, false, true), 3188);
        assert!(!s.bag.iter().any(|b| t.item_name(b.1) == "Egg Warmer"));
    }

    #[test]
    fn levels() {
        assert_eq!(level_from_exp("medium", 1_000_000), 100);
        assert_eq!(level_from_exp("medium", 125), 5);
        assert_eq!(level_from_exp("medium", 124), 4);
        assert_eq!(level_from_exp("medium-slow", 0), 1);
        assert_eq!(level_from_exp("fast", 800_000), 100);
    }
}
