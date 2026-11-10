//! Global hotkeys (work while the game has focus). Bindings are configurable
//! in Setup → Hotkeys and saved in the config.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotAction {
    ToggleHide,
    Plus,
    Minus,
    Pause,
    Caught,
    Undo,
    ClickThrough,
}

impl HotAction {
    pub const ALL: [HotAction; 7] = [
        HotAction::ToggleHide,
        HotAction::Plus,
        HotAction::Minus,
        HotAction::Pause,
        HotAction::Caught,
        HotAction::Undo,
        HotAction::ClickThrough,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HotAction::ToggleHide => "Hide / show overlay",
            HotAction::Plus => "Add 1",
            HotAction::Minus => "Remove 1",
            HotAction::Pause => "Pause / resume auto count",
            HotAction::Caught => "Caught! (asks to confirm)",
            HotAction::Undo => "Undo last encounter",
            HotAction::ClickThrough => "Click-through on / off (modular overlay)",
        }
    }
}

/// One key combination, e.g. Ctrl+Shift+H. `key` uses egui's key names
/// ("H", "7", "F5", "Up", "PageDown", "Space").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyBind {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: String,
}

impl KeyBind {
    fn ctrl_shift(key: &str) -> Self {
        Self {
            ctrl: true,
            shift: true,
            alt: false,
            key: key.into(),
        }
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(match self.key.as_str() {
            "Up" | "ArrowUp" => "↑",
            "Down" | "ArrowDown" => "↓",
            "Left" | "ArrowLeft" => "←",
            "Right" | "ArrowRight" => "→",
            k => k,
        });
        s
    }

    pub fn has_modifier(&self) -> bool {
        self.ctrl || self.shift || self.alt
    }

    fn to_hotkey(&self) -> Option<HotKey> {
        let code = code_for(&self.key)?;
        let mut m = Modifiers::empty();
        if self.ctrl {
            m |= Modifiers::CONTROL;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        Some(HotKey::new(if m.is_empty() { None } else { Some(m) }, code))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBinds {
    pub hide: Option<KeyBind>,
    pub plus: Option<KeyBind>,
    pub minus: Option<KeyBind>,
    pub pause: Option<KeyBind>,
    pub caught: Option<KeyBind>,
    pub undo: Option<KeyBind>,
    pub click_through: Option<KeyBind>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        Self {
            hide: Some(KeyBind::ctrl_shift("H")),
            plus: Some(KeyBind::ctrl_shift("Up")),
            minus: Some(KeyBind::ctrl_shift("Down")),
            pause: Some(KeyBind::ctrl_shift("P")),
            caught: Some(KeyBind::ctrl_shift("C")),
            undo: Some(KeyBind::ctrl_shift("Z")),
            click_through: Some(KeyBind::ctrl_shift("T")),
        }
    }
}

impl KeyBinds {
    pub fn get(&self, a: HotAction) -> Option<&KeyBind> {
        match a {
            HotAction::ToggleHide => self.hide.as_ref(),
            HotAction::Plus => self.plus.as_ref(),
            HotAction::Minus => self.minus.as_ref(),
            HotAction::Pause => self.pause.as_ref(),
            HotAction::Caught => self.caught.as_ref(),
            HotAction::Undo => self.undo.as_ref(),
            HotAction::ClickThrough => self.click_through.as_ref(),
        }
    }

    pub fn set(&mut self, a: HotAction, b: Option<KeyBind>) {
        match a {
            HotAction::ToggleHide => self.hide = b,
            HotAction::Plus => self.plus = b,
            HotAction::Minus => self.minus = b,
            HotAction::Pause => self.pause = b,
            HotAction::Caught => self.caught = b,
            HotAction::Undo => self.undo = b,
            HotAction::ClickThrough => self.click_through = b,
        }
    }

    /// " (Ctrl+Shift+H)" for tooltips, or "" when unbound.
    pub fn hint(&self, a: HotAction) -> String {
        self.get(a)
            .map(|b| format!(" ({})", b.text()))
            .unwrap_or_default()
    }
}

/// Keys that can be used in a hotkey (egui key name → keyboard code).
pub fn code_for(key: &str) -> Option<Code> {
    const LETTERS: [Code; 26] = [
        Code::KeyA,
        Code::KeyB,
        Code::KeyC,
        Code::KeyD,
        Code::KeyE,
        Code::KeyF,
        Code::KeyG,
        Code::KeyH,
        Code::KeyI,
        Code::KeyJ,
        Code::KeyK,
        Code::KeyL,
        Code::KeyM,
        Code::KeyN,
        Code::KeyO,
        Code::KeyP,
        Code::KeyQ,
        Code::KeyR,
        Code::KeyS,
        Code::KeyT,
        Code::KeyU,
        Code::KeyV,
        Code::KeyW,
        Code::KeyX,
        Code::KeyY,
        Code::KeyZ,
    ];
    const DIGITS: [Code; 10] = [
        Code::Digit0,
        Code::Digit1,
        Code::Digit2,
        Code::Digit3,
        Code::Digit4,
        Code::Digit5,
        Code::Digit6,
        Code::Digit7,
        Code::Digit8,
        Code::Digit9,
    ];
    const FKEYS: [Code; 12] = [
        Code::F1,
        Code::F2,
        Code::F3,
        Code::F4,
        Code::F5,
        Code::F6,
        Code::F7,
        Code::F8,
        Code::F9,
        Code::F10,
        Code::F11,
        Code::F12,
    ];
    let k = key.trim();
    let mut chars = k.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        let c = c.to_ascii_uppercase();
        if c.is_ascii_uppercase() {
            return Some(LETTERS[(c as u8 - b'A') as usize]);
        }
        if c.is_ascii_digit() {
            return Some(DIGITS[(c as u8 - b'0') as usize]);
        }
    }
    if let Some(n) = k.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()) {
        if (1..=12).contains(&n) {
            return Some(FKEYS[n - 1]);
        }
    }
    Some(match k {
        "Up" | "ArrowUp" => Code::ArrowUp,
        "Down" | "ArrowDown" => Code::ArrowDown,
        "Left" | "ArrowLeft" => Code::ArrowLeft,
        "Right" | "ArrowRight" => Code::ArrowRight,
        "Home" => Code::Home,
        "End" => Code::End,
        "PageUp" => Code::PageUp,
        "PageDown" => Code::PageDown,
        "Insert" => Code::Insert,
        "Delete" => Code::Delete,
        "Space" => Code::Space,
        _ => return None,
    })
}

pub struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, HotAction)>,
    /// Problems from the last `apply` (e.g. a combo taken by another app).
    pub errors: Vec<String>,
}

impl Hotkeys {
    pub fn new(binds: &KeyBinds) -> Self {
        let mut me = match GlobalHotKeyManager::new() {
            Ok(m) => Self {
                manager: Some(m),
                registered: Vec::new(),
                errors: Vec::new(),
            },
            Err(e) => Self {
                manager: None,
                registered: Vec::new(),
                errors: vec![format!("Hotkeys unavailable: {e}")],
            },
        };
        me.apply(binds);
        me
    }

    /// Stop listening (used while recording a new combination).
    pub fn suspend(&mut self) {
        if let Some(m) = &self.manager {
            for (hk, _) in self.registered.drain(..) {
                let _ = m.unregister(hk);
            }
        }
    }

    /// Register the given bindings, replacing the current ones.
    pub fn apply(&mut self, binds: &KeyBinds) {
        self.suspend();
        let Some(m) = &self.manager else { return };
        self.errors.clear();
        for action in HotAction::ALL {
            let Some(bind) = binds.get(action) else {
                continue;
            };
            let Some(hk) = bind.to_hotkey() else {
                self.errors
                    .push(format!("{} can't be used as a hotkey", bind.text()));
                continue;
            };
            match m.register(hk.clone()) {
                Ok(()) => self.registered.push((hk, action)),
                Err(_) => self.errors.push(format!(
                    "{} ({}) is already used by another app — pick a different one",
                    bind.text(),
                    action.label()
                )),
            }
        }
    }

    pub fn poll(&self) -> Vec<HotAction> {
        let mut out = Vec::new();
        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state == HotKeyState::Pressed {
                if let Some((_, a)) = self.registered.iter().find(|(hk, _)| hk.id() == ev.id) {
                    out.push(*a);
                }
            }
        }
        out
    }
}
