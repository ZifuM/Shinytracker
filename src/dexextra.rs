//! Extra Pokédex facts from pokemondb.net: Pokédex data (category, height,
//! weight, abilities), training (EV yield, catch rate…), breeding, base
//! stats and type defenses. The Platinum Together Wiki stays the source for
//! anything both have (types, moves, where to find, evolutions).
//!
//! A Pokémon's page is fetched the first time its entry is opened, then kept
//! in the cache folder, like the sprites.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

const BASE: &str = "https://pokemondb.net/pokedex/";
/// Bump when the parser changes so old cache files are fetched again.
const FORMAT: u32 = 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Extra {
    pub format: u32,
    /// "Mantis Pokémon"
    pub category: String,
    /// "1.5 m (4′11″)"
    pub height: String,
    /// "56.0 kg (123.5 lbs)"
    pub weight: String,
    pub abilities: Vec<String>,
    pub hidden_ability: String,
    /// "1 Attack"
    pub ev_yield: String,
    /// "45 (5.9% with PokéBall, full HP)"
    pub catch_rate: String,
    pub base_friendship: String,
    pub base_exp: String,
    pub growth_rate: String,
    pub egg_groups: Vec<String>,
    /// "50% male, 50% female"
    pub gender: String,
    /// "25 (6,169–6,425 steps)"
    pub egg_cycles: String,
    /// HP, Attack, Defense, Sp. Atk, Sp. Def, Speed, as in Platinum.
    pub stats: Vec<u32>,
    pub total: u32,
    /// The latest games' stats, when they differ from Platinum's.
    pub modern_stats: Vec<u32>,
    /// What the page's "changes" list put back to Platinum values.
    pub platinum_notes: Vec<String>,
    /// Attacking type → damage ×100 (0, 25, 50, 100, 200, 400).
    pub type_fx: Vec<(String, u32)>,
}

impl Extra {
    pub fn page_url(slug: &str) -> String {
        format!("{BASE}{slug}")
    }
}

// ───────────────────────────── parsing ─────────────────────────────

fn decode(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&#8470;", "№")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&#8242;", "′")
        .replace("&#8243;", "″")
        .replace("&#8594;", "→")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&eacute;", "é")
        .replace("&amp;", "&")
}

/// Text of an HTML fragment: tags dropped, entities decoded, spaces collapsed.
pub fn text(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    let s = decode(&out);
    let mut clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
    // "45 ( 5.9% … )" → "45 (5.9% …)"
    for (a, b) in [("( ", "("), (" )", ")"), (" ,", ","), (" .", ".")] {
        clean = clean.replace(a, b);
    }
    clean
}

/// The first `<h2>title</h2>` section, up to the next `<h2`.
fn section<'a>(html: &'a str, title: &str) -> Option<&'a str> {
    let start = html.find(&format!("<h2>{title}</h2>"))?;
    let rest = &html[start + 4..];
    let end = rest.find("<h2").map(|i| start + 4 + i).unwrap_or(html.len());
    Some(&html[start..end])
}

/// (header, cell html) pairs of a vitals table: `<th>…</th>` then `<td…>…</td>`.
fn rows(sec: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = sec;
    while let Some(th) = rest.find("<th") {
        let after = &rest[th..];
        let Some(th_end) = after.find("</th>") else { break };
        let head = text(&after[..th_end]);
        let after_th = &after[th_end + 5..];
        let Some(td) = after_th.find("<td") else { break };
        // a <th> before the next <td> means this <th> had no cell
        if after_th[..td].contains("<th") {
            rest = &after_th[after_th[..td].find("<th").unwrap_or(0)..];
            continue;
        }
        let cell_start = &after_th[td..];
        let Some(open_end) = cell_start.find('>') else { break };
        let Some(td_end) = cell_start.find("</td>") else { break };
        out.push((head, cell_start[open_end + 1..td_end].to_string()));
        rest = &cell_start[td_end + 5..];
    }
    out
}

fn first_number(s: &str) -> u32 {
    s.split(|c: char| !c.is_ascii_digit()).find(|p| !p.is_empty()).and_then(|p| p.parse().ok()).unwrap_or(0)
}

/// Does "Generation 4" / "Generations 1-5" (as written on the page) include Gen 4?
fn covers_gen4(gens: &str) -> bool {
    let nums: Vec<u32> = gens.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse().ok()).collect();
    match nums.as_slice() {
        [a] => *a == 4,
        [a, b, ..] => *a <= 4 && 4 <= *b,
        _ => false,
    }
}

/// pokemondb shows the latest games' values and lists older ones under
/// "<Name> changes". Put back whatever was different in Generation 4.
fn apply_changes(html: &str, e: &mut Extra) {
    let Some(h) = html.find(" changes</h2>") else { return };
    let rest = &html[h..];
    let end = rest.find("</ul>").unwrap_or(rest.len());
    let names = ["HP", "Attack", "Defense", "Special Attack", "Special Defense", "Speed"];
    let before = e.stats.clone();
    for li in rest[..end].split("<li>").skip(1) {
        let t = text(li);
        // "In Generations 1-5, Pikachu has a base Defense of 30."
        let Some(after_in) = t.strip_prefix("In Generation") else { continue };
        let Some((gens, claim)) = after_in.split_once(',') else { continue };
        if !covers_gen4(gens) {
            continue;
        }
        let claim = claim.trim().trim_end_matches('.');
        if ["Mega ", "Alolan ", "Galarian ", "Hisuian ", "Paldean ", "Primal ", "Gigantamax"].iter().any(|f| claim.contains(f)) {
            continue;
        }
        if let Some((_, v)) = claim.split_once(" has a base experience yield of ") {
            e.base_exp = v.trim().to_string();
            e.platinum_notes.push("Base Exp.".into());
        } else if let Some((_, v)) = claim.split_once(" has a base Friendship value of ") {
            e.base_friendship = v.trim().to_string();
            e.platinum_notes.push("Base Friendship".into());
        } else if let Some((_, v)) = claim.split_once(" has a base ") {
            // "Defense of 30"
            if let Some((stat, n)) = v.rsplit_once(" of ") {
                if let (Some(i), Ok(n)) = (names.iter().position(|s| *s == stat), n.trim().parse::<u32>()) {
                    if let Some(slot) = e.stats.get_mut(i) {
                        *slot = n;
                    }
                }
            }
        } else if let Some((_, v)) = claim.split_once(" does not have the ") {
            if let Some(a) = v.strip_suffix(" ability") {
                e.abilities.retain(|x| x != a);
                if e.hidden_ability == a {
                    e.hidden_ability.clear();
                }
                e.platinum_notes.push("Abilities".into());
            }
        } else if let Some((_, v)) = claim.split_once(" has the ") {
            if let Some(a) = v.strip_suffix(" ability") {
                if e.abilities.len() <= 1 {
                    e.abilities = vec![a.to_string()];
                } else if !e.abilities.iter().any(|x| x == a) {
                    e.abilities.push(a.to_string());
                }
                e.platinum_notes.push("Abilities".into());
            }
        }
    }
    if e.stats != before && !before.is_empty() {
        e.modern_stats = before;
        e.total = e.stats.iter().sum();
        e.platinum_notes.push("Base stats".into());
    }
    e.platinum_notes.dedup();
}

/// Parse a pokemondb Pokémon page (the first form only).
pub fn parse(html: &str) -> Extra {
    let mut e = Extra { format: FORMAT, ..Default::default() };
    if let Some(sec) = section(html, "Pokédex data") {
        for (h, cell) in rows(sec) {
            match h.as_str() {
                "Species" => e.category = text(&cell),
                "Height" => e.height = text(&cell),
                "Weight" => e.weight = text(&cell),
                "Abilities" => {
                    for part in cell.split("<br>") {
                        let t = text(part);
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(name) = t.strip_suffix("(hidden ability)") {
                            e.hidden_ability = name.trim().to_string();
                        } else {
                            // "1. Swarm" → "Swarm"
                            let name = t.split_once(". ").map(|(n, rest)| if n.chars().all(|c| c.is_ascii_digit()) { rest } else { t.as_str() }).unwrap_or(&t);
                            e.abilities.push(name.trim().to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(sec) = section(html, "Training") {
        for (h, cell) in rows(sec) {
            let v = text(&cell);
            match h.as_str() {
                "EV yield" => e.ev_yield = v,
                "Catch rate" => e.catch_rate = v,
                "Base Friendship" => e.base_friendship = v,
                "Base Exp." => e.base_exp = v,
                "Growth Rate" => e.growth_rate = v,
                _ => {}
            }
        }
    }
    if let Some(sec) = section(html, "Breeding") {
        for (h, cell) in rows(sec) {
            match h.as_str() {
                "Egg Groups" => {
                    e.egg_groups = cell.split(',').map(text).filter(|s| !s.is_empty()).collect();
                }
                "Gender" => e.gender = text(&cell),
                "Egg cycles" => e.egg_cycles = text(&cell),
                _ => {}
            }
        }
    }
    if let Some(sec) = section(html, "Base stats") {
        let order = ["HP", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed"];
        let mut stats = [0u32; 6];
        for (h, cell) in rows(sec) {
            if let Some(i) = order.iter().position(|o| *o == h) {
                if stats[i] == 0 {
                    stats[i] = first_number(&text(&cell));
                }
            } else if h == "Total" && e.total == 0 {
                e.total = first_number(&text(&cell));
            }
        }
        if stats.iter().any(|&s| s > 0) {
            e.stats = stats.to_vec();
            if e.total == 0 {
                e.total = stats.iter().sum();
            }
        }
    }
    apply_changes(html, &mut e);
    if let Some(sec) = section(html, "Type defenses") {
        let mut rest = sec;
        while let Some(i) = rest.find("<td title=\"") {
            let cell = &rest[i + 11..];
            let Some(q) = cell.find('"') else { break };
            let title = decode(&cell[..q]);
            let after = &cell[q..];
            let class_fx = after
                .find("type-fx-")
                .and_then(|k| {
                    let s = &after[k + 8..];
                    // skip "type-fx-cell"
                    if s.starts_with("cell") {
                        let k2 = s.find("type-fx-")?;
                        Some(first_number(&s[k2 + 8..]))
                    } else {
                        Some(first_number(s))
                    }
                })
                .unwrap_or(100);
            if let Some((attacker, _)) = title.split_once('→') {
                let a = attacker.trim().to_string();
                if !a.is_empty() && !e.type_fx.iter().any(|(t, _)| *t == a) {
                    e.type_fx.push((a, class_fx));
                }
            }
            rest = &cell[q..];
        }
    }
    e
}

// ───────────────────────────── type chart ─────────────────────────────

pub const TYPES: [&str; 18] = [
    "Normal", "Fire", "Water", "Electric", "Grass", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug",
    "Rock", "Ghost", "Dragon", "Dark", "Steel", "Fairy",
];

/// Attacking type vs one defending type: damage ×2 (0 = none, 1 = half,
/// 2 = normal, 4 = double). The current chart, which has Fairy.
fn hit(atk: &str, def: &str) -> u32 {
    let (double, half, none): (&[&str], &[&str], &[&str]) = match atk {
        "Normal" => (&[], &["Rock", "Steel"], &["Ghost"]),
        "Fire" => (&["Grass", "Ice", "Bug", "Steel"], &["Fire", "Water", "Rock", "Dragon"], &[]),
        "Water" => (&["Fire", "Ground", "Rock"], &["Water", "Grass", "Dragon"], &[]),
        "Electric" => (&["Water", "Flying"], &["Electric", "Grass", "Dragon"], &["Ground"]),
        "Grass" => (&["Water", "Ground", "Rock"], &["Fire", "Grass", "Poison", "Flying", "Bug", "Dragon", "Steel"], &[]),
        "Ice" => (&["Grass", "Ground", "Flying", "Dragon"], &["Fire", "Water", "Ice", "Steel"], &[]),
        "Fighting" => (&["Normal", "Ice", "Rock", "Dark", "Steel"], &["Poison", "Flying", "Psychic", "Bug", "Fairy"], &["Ghost"]),
        "Poison" => (&["Grass", "Fairy"], &["Poison", "Ground", "Rock", "Ghost"], &["Steel"]),
        "Ground" => (&["Fire", "Electric", "Poison", "Rock", "Steel"], &["Grass", "Bug"], &["Flying"]),
        "Flying" => (&["Grass", "Fighting", "Bug"], &["Electric", "Rock", "Steel"], &[]),
        "Psychic" => (&["Fighting", "Poison"], &["Psychic", "Steel"], &["Dark"]),
        "Bug" => (&["Grass", "Psychic", "Dark"], &["Fire", "Fighting", "Poison", "Flying", "Ghost", "Steel", "Fairy"], &[]),
        "Rock" => (&["Fire", "Ice", "Flying", "Bug"], &["Fighting", "Ground", "Steel"], &[]),
        "Ghost" => (&["Psychic", "Ghost"], &["Dark"], &["Normal"]),
        "Dragon" => (&["Dragon"], &["Steel"], &["Fairy"]),
        "Dark" => (&["Psychic", "Ghost"], &["Fighting", "Dark", "Fairy"], &[]),
        "Steel" => (&["Ice", "Rock", "Fairy"], &["Fire", "Water", "Electric", "Steel"], &[]),
        "Fairy" => (&["Fighting", "Dragon", "Dark"], &["Fire", "Poison", "Steel"], &[]),
        _ => (&[], &[], &[]),
    };
    if none.contains(&def) {
        0
    } else if double.contains(&def) {
        4
    } else if half.contains(&def) {
        1
    } else {
        2
    }
}

/// How much damage each attacking type does to a Pokémon of these types,
/// ×100 (0, 25, 50, 100, 200, 400). Worked out from the wiki's types, so
/// Pokémon that kept their Platinum typing get the right chart.
pub fn defenses(types: &[String]) -> Vec<(&'static str, u32)> {
    TYPES
        .iter()
        .map(|a| {
            let mut x = 100u32;
            for d in types.iter().take(2) {
                x = x * hit(a, d) / 2;
            }
            (*a, x)
        })
        .collect()
}

// ───────────────────────────── fetching ─────────────────────────────

fn cache_path(name: &str) -> PathBuf {
    crate::config::cache_dir().join("pokemondb").join(format!("{name}.json"))
}

/// The cached facts for a Pokémon, if fetched before (and still current).
pub fn cached(slug: &str) -> Option<Extra> {
    let bytes = std::fs::read(cache_path(slug)).ok()?;
    let e: Extra = serde_json::from_slice(&bytes).ok()?;
    (e.format == FORMAT).then_some(e)
}

fn get_html(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", concat!("PokeTogetherCompanion/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn save<T: Serialize>(name: &str, v: &T) {
    let path = cache_path(name);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_vec(v) {
        let _ = std::fs::write(path, json);
    }
}

fn fetch(slug: &str) -> Result<Extra, String> {
    let e = parse(&get_html(&Extra::page_url(slug))?);
    if e.stats.is_empty() && e.category.is_empty() {
        return Err("the pokemondb page didn't have the expected sections".into());
    }
    save(slug, &e);
    Ok(e)
}

/// Where a Pokémon's extra facts are up to.
#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Ready(Box<Extra>),
    Failed(String),
}

/// Fetches pages one at a time on a background thread.
pub struct Fetcher {
    known: HashMap<String, State>,
    tx: Sender<String>,
    rx: Receiver<(String, Result<Extra, String>)>,
}

impl Default for Fetcher {
    fn default() -> Self {
        let (tx, jobs) = std::sync::mpsc::channel::<String>();
        let (done_tx, rx) = std::sync::mpsc::channel();
        let _ = std::thread::Builder::new().name("pokemondb".into()).spawn(move || {
            for slug in jobs {
                let r = fetch(&slug);
                if done_tx.send((slug, r)).is_err() {
                    return;
                }
                // Be gentle with the site.
                std::thread::sleep(Duration::from_millis(400));
            }
        });
        Self { known: HashMap::new(), tx, rx }
    }
}

impl Fetcher {
    fn poll(&mut self) {
        while let Ok((s, r)) = self.rx.try_recv() {
            let st = match r {
                Ok(e) => State::Ready(Box::new(e)),
                Err(e) => State::Failed(e),
            };
            self.known.insert(s, st);
        }
    }

    /// Facts for a Pokémon: from memory, the cache, or start fetching.
    pub fn get(&mut self, slug: &str) -> State {
        self.poll();
        if let Some(s) = self.known.get(slug) {
            return s.clone();
        }
        let st = match cached(slug) {
            Some(e) => State::Ready(Box::new(e)),
            None => {
                let _ = self.tx.send(slug.to_string());
                State::Loading
            }
        };
        self.known.insert(slug.to_string(), st.clone());
        st
    }

    /// Try a failed one again.
    pub fn retry(&mut self, slug: &str) {
        self.known.remove(slug);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<h1>Scyther</h1>
<h2>Pokédex data</h2>
<table class="vitals-table"><tbody>
<tr><th>National &#8470;</th><td><strong>0123</strong></td></tr>
<tr><th>Type</th><td><a class="type-icon type-bug" href="/type/bug" >Bug</a> <a href="/type/flying">Flying</a></td></tr>
<tr><th>Species</th><td>Mantis Pokémon</td></tr>
<tr><th>Height</th><td>1.5&nbsp;m (4&#8242;11&#8243;)</td></tr>
<tr><th>Weight</th><td>56.0&nbsp;kg (123.5&nbsp;lbs)</td></tr>
<tr><th>Abilities</th><td><span class="text-muted">1. <a href="/ability/swarm" title="x">Swarm</a></span><br><span class="text-muted">2. <a href="/ability/technician">Technician</a></span><br><small class="text-muted"><a href="/ability/steadfast">Steadfast</a> (hidden ability)</small><br></td></tr>
</tbody></table>
<h2>Training</h2>
<table class="vitals-table"><tbody>
<tr><th>EV yield</th><td class="text">
  1 Attack  </td></tr>
<tr><th>Catch rate</th><td>
  45 <small class="text-muted">(5.9% with PokéBall, full HP)</small></td></tr>
<tr><th>Base <a href="/glossary#def-friendship">Friendship</a></th><td>50 <small class="text-muted">(normal)</small></td></tr>
<tr><th>Base Exp.</th><td>100</td></tr>
<tr><th>Growth Rate</th><td>Medium Fast</td></tr>
</tbody></table>
<h2>Breeding</h2>
<table class="vitals-table"><tbody>
<tr><th>Egg Groups</th><td><a href="/egg-group/bug">Bug</a>, <a href="/egg-group/x">Flying</a></td></tr>
<tr><th>Gender</th><td><span class="text-blue">50% male</span>, <span class="text-pink">50% female</span></td></tr>
<tr><th><a href="/glossary#def-eggcycle">Egg cycles</a></th><td>25 <small class="text-muted">(6,169&ndash;6,425 steps)</small></td></tr>
</tbody></table>
<h2>Base stats</h2>
<table class="vitals-table"><tbody>
<tr><th>HP</th><td class="cell-num">70</td><td class="cell-barchart"><div></div></td><td class="cell-num">250</td><td class="cell-num">344</td></tr>
<tr><th>Attack</th><td class="cell-num">110</td><td class="cell-barchart"></td><td class="cell-num">202</td><td class="cell-num">350</td></tr>
<tr><th>Defense</th><td class="cell-num">80</td></tr>
<tr><th>Sp. Atk</th><td class="cell-num">55</td></tr>
<tr><th>Sp. Def</th><td class="cell-num">80</td></tr>
<tr><th>Speed</th><td class="cell-num">105</td></tr>
</tbody><tfoot><tr><th>Total</th><td class="cell-num cell-total">500</td><th class="cell-barchart"></th><th>Min</th><th>Max</th></tr></tfoot></table>
<h2>Type defenses</h2>
<table class="type-table"><tr><th><a title="Normal">Nor</a></th></tr>
<tr><td title="Normal &#8594; Bug/Flying = normal effectiveness" class="type-fx-cell type-fx-100"></td><td title="Fire &#8594; Bug/Flying = super-effective" class="type-fx-cell type-fx-200">2</td><td title="Ground &#8594; Bug/Flying = no effect" class="type-fx-cell type-fx-0">0</td><td title="Grass &#8594; Bug/Flying = not very effective" class="type-fx-cell type-fx-25">¼</td><td title="Rock &#8594; Bug/Flying = super-effective" class="type-fx-cell type-fx-400">4</td></tr></table>
<h2>Evolution chart</h2>
<h2>Pokédex data</h2><table><tr><th>Species</th><td>Second form</td></tr></table>"#;

    #[test]
    fn parses_a_page() {
        let e = parse(PAGE);
        assert_eq!(e.category, "Mantis Pokémon");
        assert_eq!(e.height, "1.5 m (4′11″)");
        assert_eq!(e.weight, "56.0 kg (123.5 lbs)");
        assert_eq!(e.abilities, vec!["Swarm", "Technician"]);
        assert_eq!(e.hidden_ability, "Steadfast");
        assert_eq!(e.ev_yield, "1 Attack");
        assert_eq!(e.catch_rate, "45 (5.9% with PokéBall, full HP)");
        assert_eq!(e.base_friendship, "50 (normal)");
        assert_eq!(e.base_exp, "100");
        assert_eq!(e.growth_rate, "Medium Fast");
        assert_eq!(e.egg_groups, vec!["Bug", "Flying"]);
        assert_eq!(e.gender, "50% male, 50% female");
        assert_eq!(e.egg_cycles, "25 (6,169–6,425 steps)");
        assert_eq!(e.stats, vec![70, 110, 80, 55, 80, 105]);
        assert_eq!(e.total, 500);
        assert_eq!(
            e.type_fx,
            vec![
                ("Normal".to_string(), 100),
                ("Fire".to_string(), 200),
                ("Ground".to_string(), 0),
                ("Grass".to_string(), 25),
                ("Rock".to_string(), 400)
            ]
        );
    }

    #[test]
    fn type_chart_matches_pokemondb() {
        let fx = defenses(&["Bug".to_string(), "Flying".to_string()]);
        let get = |t: &str| fx.iter().find(|(a, _)| *a == t).unwrap().1;
        assert_eq!(get("Normal"), 100);
        assert_eq!(get("Fire"), 200);
        assert_eq!(get("Ground"), 0);
        assert_eq!(get("Grass"), 25);
        assert_eq!(get("Rock"), 400);
        let ghost = defenses(&["Normal".to_string()]);
        assert_eq!(ghost.iter().find(|(a, _)| *a == "Ghost").unwrap().1, 0);
        assert_eq!(ghost.iter().find(|(a, _)| *a == "Fighting").unwrap().1, 200);
    }

    #[test]
    fn puts_back_platinum_values() {
        let page = r#"<h2>Pokédex data</h2><table><tr><th>Abilities</th><td><span>1. <a>Cursed Body</a></span><br></td></tr></table>
<h2>Training</h2><table><tr><th>Base Exp.</th><td>250</td></tr><tr><th>Base <a>Friendship</a></th><td>50 <small>(normal)</small></td></tr></table>
<h2>Base stats</h2><table><tr><th>HP</th><td class="cell-num">35</td></tr><tr><th>Attack</th><td class="cell-num">55</td></tr>
<tr><th>Defense</th><td class="cell-num">40</td></tr><tr><th>Sp. Atk</th><td class="cell-num">50</td></tr><tr><th>Sp. Def</th><td class="cell-num">50</td></tr><tr><th>Speed</th><td class="cell-num">90</td></tr>
<tfoot><tr><th>Total</th><td class="cell-num cell-total">320</td></tr></tfoot></table>
<h2>Pikachu changes</h2>
<ul>
<li>In <abbr title="Pokémon Red">Generation 1</abbr>, <em>Pikachu</em> has a base Special stat of 50.</li>
<li>In <abbr title="x">Generations 1-5</abbr>, <em>Pikachu</em> has a base Defense of 30.</li>
<li>In <abbr title="x">Generations 2-5</abbr>, <em>Pikachu</em> has a base Special Defense of 40.</li>
<li>In <abbr title="x">Generations 1-4</abbr>, <em>Pikachu</em> has a base experience yield of 82.</li>
<li>In <abbr title="x">Generation 5</abbr>, <em>Pikachu</em> has a base experience yield of 105.</li>
<li>In Pokémon Yellow, <em>Pikachu</em> has a base Friendship value of 90.</li>
<li>In <abbr title="x">Generations 2-7</abbr>, <em>Pikachu</em> has a base Friendship value of 70.</li>
<li>In <abbr title="x">Generations 3-6</abbr>, <em>Pikachu</em> has the Levitate ability.</li>
<li>In <abbr title="x">Generation 6</abbr>, Mega <em>Pikachu</em> has a base Speed of 1.</li>
</ul><h2>Pokédex entries</h2>"#;
        let e = parse(page);
        assert_eq!(e.stats, vec![35, 55, 30, 50, 40, 90]);
        assert_eq!(e.total, 300);
        assert_eq!(e.modern_stats, vec![35, 55, 40, 50, 50, 90]);
        assert_eq!(e.base_exp, "82");
        assert_eq!(e.base_friendship, "70");
        assert_eq!(e.abilities, vec!["Levitate"]);
        assert!(covers_gen4(" 4") && covers_gen4("s 1-5") && !covers_gen4("s 5-7") && !covers_gen4(" 1"));
    }

    #[test]
    fn missing_sections_are_empty() {
        let e = parse("<html>nothing</html>");
        assert!(e.stats.is_empty());
        assert!(e.category.is_empty());
    }
}
