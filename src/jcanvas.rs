//! A Journal canvas: cards on an endless board, joined by arrows. Saved as
//! a `.canvas` file in the JSON Canvas format (the one Obsidian uses), so a
//! board opens in Obsidian too. Our own extras (a live widget on a card)
//! ride along in `ptc…` fields that other apps keep or ignore.

use serde::{Deserialize, Serialize, Serializer};

fn int<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_i64(v.round() as i64)
}

fn none<T>(v: &Option<T>) -> bool {
    v.is_none()
}

/// A card.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    /// "text", "file", "link" or "group".
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(serialize_with = "int", default)]
    pub x: f32,
    #[serde(serialize_with = "int", default)]
    pub y: f32,
    #[serde(serialize_with = "int", default)]
    pub width: f32,
    #[serde(serialize_with = "int", default)]
    pub height: f32,
    /// "1"–"6" (red, orange, yellow, green, cyan, purple) or "#rrggbb".
    #[serde(default, skip_serializing_if = "none")]
    pub color: Option<String>,
    /// A text card's Markdown.
    #[serde(default, skip_serializing_if = "none")]
    pub text: Option<String>,
    /// A file card's note or picture, as a path in the Journal.
    #[serde(default, skip_serializing_if = "none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "none")]
    pub url: Option<String>,
    /// A group's name.
    #[serde(default, skip_serializing_if = "none")]
    pub label: Option<String>,
    /// A live widget on the card (its kind, as on the Dashboard).
    #[serde(rename = "ptcWidget", default, skip_serializing_if = "none")]
    pub widget: Option<String>,
    /// The widget's own settings.
    #[serde(rename = "ptcData", default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<String>,
    /// Anything else in the file, kept as it was.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// An arrow between two cards.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub from_node: String,
    /// "top", "right", "bottom" or "left".
    #[serde(default, skip_serializing_if = "none")]
    pub from_side: Option<String>,
    /// "none" or "arrow" (none when left out).
    #[serde(default, skip_serializing_if = "none")]
    pub from_end: Option<String>,
    pub to_node: String,
    #[serde(default, skip_serializing_if = "none")]
    pub to_side: Option<String>,
    /// "none" or "arrow" (an arrow when left out).
    #[serde(default, skip_serializing_if = "none")]
    pub to_end: Option<String>,
    #[serde(default, skip_serializing_if = "none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "none")]
    pub label: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Canvas {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A side of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];
    pub fn name(self) -> &'static str {
        match self {
            Side::Top => "top",
            Side::Right => "right",
            Side::Bottom => "bottom",
            Side::Left => "left",
        }
    }
    pub fn parse(s: Option<&str>) -> Option<Side> {
        Side::ALL.into_iter().find(|x| Some(x.name()) == s)
    }
    /// The way out of the card, as (x, y).
    pub fn dir(self) -> (f32, f32) {
        match self {
            Side::Top => (0.0, -1.0),
            Side::Right => (1.0, 0.0),
            Side::Bottom => (0.0, 1.0),
            Side::Left => (-1.0, 0.0),
        }
    }
}

impl Node {
    pub fn is_group(&self) -> bool {
        self.kind == "group"
    }
    /// (x, y, w, h)
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width, self.height)
    }
    /// The middle of a side.
    pub fn anchor(&self, s: Side) -> (f32, f32) {
        let (cx, cy) = (self.x + self.width / 2.0, self.y + self.height / 2.0);
        match s {
            Side::Top => (cx, self.y),
            Side::Right => (self.x + self.width, cy),
            Side::Bottom => (cx, self.y + self.height),
            Side::Left => (self.x, cy),
        }
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
    /// Is `o` wholly inside this card (a group's members)?
    pub fn holds(&self, o: &Node) -> bool {
        o.id != self.id && o.x >= self.x && o.y >= self.y && o.x + o.width <= self.x + self.width && o.y + o.height <= self.y + self.height
    }
}

/// The side of `a` that faces `b` best.
pub fn facing(a: &Node, b: &Node) -> Side {
    let (ax, ay) = (a.x + a.width / 2.0, a.y + a.height / 2.0);
    let (bx, by) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    let (dx, dy) = (bx - ax, by - ay);
    // scaled by the card's shape, so a wide card prefers its long sides less
    if (dx / a.width.max(1.0)).abs() > (dy / a.height.max(1.0)).abs() {
        if dx > 0.0 {
            Side::Right
        } else {
            Side::Left
        }
    } else if dy > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

impl Canvas {
    /// A canvas from a file's text. An empty file is an empty canvas.
    pub fn parse(text: &str) -> Result<Canvas, String> {
        let t = text.trim_start_matches('\u{feff}').trim();
        if t.is_empty() {
            return Ok(Canvas::default());
        }
        let mut c: Canvas = serde_json::from_str(t).map_err(|e| format!("This canvas file couldn't be read ({e})"))?;
        // a card with no size can't be seen or grabbed
        for n in &mut c.nodes {
            if n.width < 10.0 {
                n.width = 250.0;
            }
            if n.height < 10.0 {
                n.height = 60.0;
            }
        }
        // arrows to missing cards go
        let ids: std::collections::HashSet<String> = c.nodes.iter().map(|n| n.id.clone()).collect();
        c.edges.retain(|e| ids.contains(&e.from_node) && ids.contains(&e.to_node));
        Ok(c)
    }

    /// The file's text: tab-indented JSON, as Obsidian writes it.
    pub fn to_json(&self) -> String {
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        if self.serialize(&mut ser).is_err() {
            return "{\"nodes\":[],\"edges\":[]}".into();
        }
        String::from_utf8(buf).unwrap_or_default()
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// All the cards' bounds (x0, y0, x1, y1), or None when empty.
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut it = self.nodes.iter();
        let f = it.next()?;
        let mut b = (f.x, f.y, f.x + f.width, f.y + f.height);
        for n in it {
            b.0 = b.0.min(n.x);
            b.1 = b.1.min(n.y);
            b.2 = b.2.max(n.x + n.width);
            b.3 = b.3.max(n.y + n.height);
        }
        Some(b)
    }

    /// Where a new card of this size goes: right of everything, at the top.
    pub fn free_spot(&self, _w: f32, _h: f32) -> (f32, f32) {
        match self.bounds() {
            None => (0.0, 0.0),
            Some((_, y0, x1, _)) => (snap(x1 + 60.0), snap(y0)),
        }
    }

    /// Remove cards (and their arrows).
    pub fn remove(&mut self, ids: &[String]) {
        self.nodes.retain(|n| !ids.contains(&n.id));
        self.edges.retain(|e| !ids.contains(&e.from_node) && !ids.contains(&e.to_node) && !ids.contains(&e.id));
    }

    /// The topmost card under a point (groups last, so cards in them win).
    #[allow(dead_code)]
    pub fn hit(&self, x: f32, y: f32) -> Option<&Node> {
        self.nodes.iter().rev().filter(|n| !n.is_group()).find(|n| n.contains(x, y)).or_else(|| self.nodes.iter().rev().filter(|n| n.is_group()).find(|n| n.contains(x, y)))
    }
}

/// To the board's 20-point grid.
pub fn snap(v: f32) -> f32 {
    (v / 20.0).round() * 20.0
}

/// A new id for a card or arrow (16 hex characters, like Obsidian's).
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    let mut x = t ^ N.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed).rotate_left(17);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    format!("{x:016x}")
}

/// A text card.
pub fn text_node(x: f32, y: f32, w: f32, h: f32, text: &str) -> Node {
    Node { id: new_id(), kind: "text".into(), x, y, width: w, height: h, text: Some(text.into()), ..Default::default() }
}

/// A card showing a note or picture from the Journal.
pub fn file_node(x: f32, y: f32, w: f32, h: f32, rel: &str) -> Node {
    Node { id: new_id(), kind: "file".into(), x, y, width: w, height: h, file: Some(rel.into()), ..Default::default() }
}

/// A live widget card (a text card to other apps).
pub fn widget_node(x: f32, y: f32, w: f32, h: f32, kind: &str, name: &str) -> Node {
    Node {
        id: new_id(),
        kind: "text".into(),
        x,
        y,
        width: w,
        height: h,
        text: Some(format!("**{name}**\n\nA live widget from Poke Together Companion.")),
        widget: Some(kind.into()),
        ..Default::default()
    }
}

pub fn group_node(x: f32, y: f32, w: f32, h: f32, label: &str) -> Node {
    Node { id: new_id(), kind: "group".into(), x, y, width: w, height: h, label: Some(label.into()), ..Default::default() }
}

pub fn edge(from: &Node, to: &Node) -> Edge {
    Edge {
        id: new_id(),
        from_node: from.id.clone(),
        from_side: Some(facing(from, to).name().into()),
        to_node: to.id.clone(),
        to_side: Some(facing(to, from).name().into()),
        ..Default::default()
    }
}

/// A colour's RGB: JSON Canvas's six presets, or "#rrggbb".
pub fn color_rgb(c: &str) -> Option<(u8, u8, u8)> {
    Some(match c {
        "1" => (233, 73, 97),
        "2" => (236, 117, 0),
        "3" => (224, 172, 0),
        "4" => (8, 185, 78),
        "5" => (0, 191, 188),
        "6" => (120, 82, 238),
        h if h.starts_with('#') && h.len() == 7 => {
            let v = u32::from_str_radix(&h[1..], 16).ok()?;
            ((v >> 16) as u8, (v >> 8) as u8, v as u8)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBSIDIAN: &str = r##"{
	"nodes":[
		{"id":"a1","type":"text","text":"# Plan\n- [ ] Route 214","x":-200,"y":-100,"width":250,"height":140,"color":"6"},
		{"id":"b2","type":"file","file":"Hunts/Gible hunt.md","x":120,"y":-100,"width":400,"height":300,"subpath":"#Setup"},
		{"id":"g3","type":"group","label":"Tonight","x":-240,"y":-160,"width":800,"height":400},
		{"id":"w4","type":"text","text":"**Current hunt**","x":-200,"y":80,"width":224,"height":112,"ptcWidget":"hunt_ctrl"}
	],
	"edges":[
		{"id":"e1","fromNode":"a1","fromSide":"right","toNode":"b2","toSide":"left","label":"tracking"},
		{"id":"e2","fromNode":"a1","toNode":"gone"}
	]
}"##;

    #[test]
    fn reads_obsidian_canvases_and_keeps_what_it_doesnt_know() {
        let c = Canvas::parse(OBSIDIAN).unwrap();
        assert_eq!(c.nodes.len(), 4);
        assert_eq!(c.edges.len(), 1, "the arrow to a missing card goes");
        assert_eq!(c.nodes[1].file.as_deref(), Some("Hunts/Gible hunt.md"));
        assert_eq!(c.nodes[1].extra.get("subpath").and_then(|v| v.as_str()), Some("#Setup"));
        assert_eq!(c.nodes[3].widget.as_deref(), Some("hunt_ctrl"));
        assert_eq!(c.edges[0].label.as_deref(), Some("tracking"));
        // and back: whole numbers, tabs, the unknown field kept
        let out = c.to_json();
        assert!(out.contains("\t\"nodes\""));
        assert!(out.contains("\"x\": -200,"), "{out}");
        assert!(out.contains("\"subpath\": \"#Setup\""));
        assert!(out.contains("\"fromNode\": \"a1\""));
        assert!(!out.contains("ptcData"));
        assert_eq!(Canvas::parse(&out).unwrap(), c);
    }

    #[test]
    fn empty_and_broken_files() {
        assert_eq!(Canvas::parse("").unwrap(), Canvas::default());
        assert_eq!(Canvas::parse("{}").unwrap().nodes.len(), 0);
        assert!(Canvas::parse("{nope").is_err());
        // a card with no size gets one
        let c = Canvas::parse(r#"{"nodes":[{"id":"x","type":"text","x":0,"y":0}]}"#).unwrap();
        assert!(c.nodes[0].width >= 10.0 && c.nodes[0].height >= 10.0);
    }

    #[test]
    fn groups_hits_sides_and_new_cards() {
        let c = Canvas::parse(OBSIDIAN).unwrap();
        let g = c.node("g3").unwrap();
        assert!(g.holds(c.node("a1").unwrap()));
        assert!(!g.holds(g));
        // a card in a group wins over the group
        assert_eq!(c.hit(-150.0, -50.0).map(|n| n.id.as_str()), Some("a1"));
        assert_eq!(c.hit(500.0, 220.0).map(|n| n.id.as_str()), Some("g3"));
        assert_eq!(c.hit(5000.0, 0.0), None);
        let (a, b) = (c.node("a1").unwrap(), c.node("b2").unwrap());
        assert_eq!(facing(a, b), Side::Right);
        assert_eq!(facing(b, a), Side::Left);
        assert_eq!(a.anchor(Side::Right), (50.0, -30.0));
        // new cards go to the right of everything, on the grid
        assert_eq!(c.free_spot(200.0, 100.0), (620.0, -160.0));
        assert_eq!(Canvas::default().free_spot(1.0, 1.0), (0.0, 0.0));
        let mut c2 = c.clone();
        c2.remove(&["a1".to_string()]);
        assert!(c2.node("a1").is_none() && c2.edges.is_empty());
        assert_ne!(new_id(), new_id());
        assert_eq!(new_id().len(), 16);
        assert_eq!(color_rgb("#102030"), Some((16, 32, 48)));
        assert_eq!(color_rgb("6"), Some((120, 82, 238)));
        assert_eq!(color_rgb("x"), None);
    }
}
