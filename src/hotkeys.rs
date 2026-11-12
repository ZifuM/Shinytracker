//! Global hotkeys (work while the game has focus). Bindings are configurable
//! in Setup → Hotkeys and saved in the config.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotAction {
    ToggleHide,
    Plus,
    Minus,
    Pause,
    Caught,
    Undo,
    ClickThrough,
    StartStop,
    AddMany,
    MuteSounds,
    DismissAlert,
    ShowApp,
    CycleSize,
    HotkeysToggle,
}

/// The groups the Hotkeys page shows them in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotGroup {
    Counting,
    Overlay,
    Hunt,
    App,
}

impl HotGroup {
    pub const ALL: [HotGroup; 4] = [HotGroup::Counting, HotGroup::Overlay, HotGroup::Hunt, HotGroup::App];
    pub fn label(self) -> &'static str {
        match self {
            HotGroup::Counting => "Counting",
            HotGroup::Overlay => "Overlay",
            HotGroup::Hunt => "Hunt",
            HotGroup::App => "App",
        }
    }
}

impl HotAction {
    pub const ALL: [HotAction; 14] = [
        HotAction::Plus,
        HotAction::Minus,
        HotAction::AddMany,
        HotAction::Undo,
        HotAction::Pause,
        HotAction::ToggleHide,
        HotAction::StartStop,
        HotAction::ClickThrough,
        HotAction::CycleSize,
        HotAction::Caught,
        HotAction::DismissAlert,
        HotAction::MuteSounds,
        HotAction::ShowApp,
        HotAction::HotkeysToggle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HotAction::ToggleHide => "Hide / show overlay",
            HotAction::Plus => "Add 1",
            HotAction::Minus => "Remove 1",
            HotAction::Pause => "Pause / resume auto count",
            HotAction::Caught => "Caught! (asks to confirm)",
            HotAction::Undo => "Undo last encounter",
            HotAction::ClickThrough => "Click-through on / off",
            HotAction::StartStop => "Start / stop overlay",
            HotAction::AddMany => "Add several",
            HotAction::MuteSounds => "Sounds on / off",
            HotAction::DismissAlert => "Dismiss shiny alert",
            HotAction::ShowApp => "Open the Companion",
            HotAction::CycleSize => "Next overlay size",
            HotAction::HotkeysToggle => "All hotkeys off / on",
        }
    }

    /// One line on what it does.
    pub fn about(self) -> &'static str {
        match self {
            HotAction::ToggleHide => "Shrinks the overlay to its badge and back. If it's stopped, starts it.",
            HotAction::Plus => "One more encounter, for when the auto count missed one.",
            HotAction::Minus => "One encounter less.",
            HotAction::Pause => "Stops counting for a moment (cutscenes, menus) and starts again.",
            HotAction::Caught => "Opens \"Log hunt & reset?\" on the overlay. Nothing happens until you confirm.",
            HotAction::Undo => "Takes back the last encounter that was counted.",
            HotAction::ClickThrough => "Lets clicks go through the overlay to the game, or not.",
            HotAction::StartStop => "Opens or closes the overlay completely.",
            HotAction::AddMany => "Adds a set number at once: handy for hordes and double battles.",
            HotAction::MuteSounds => "Turns every sound effect off or back on.",
            HotAction::DismissAlert => "Closes the shiny alert without touching the mouse.",
            HotAction::ShowApp => "Brings this window to the front, even from the game.",
            HotAction::CycleSize => "Switches the overlay between Full, Compact and Bar.",
            HotAction::HotkeysToggle => "Turns every other hotkey off (when you're typing in chat) and back on. This one keeps working.",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            HotAction::ToggleHide => "👁",
            HotAction::Plus => "⬆",
            HotAction::Minus => "⬇",
            HotAction::Pause => "⏸",
            HotAction::Caught => "✨",
            HotAction::Undo => "⟲",
            HotAction::ClickThrough => "🖱",
            HotAction::StartStop => "▶",
            HotAction::AddMany => "⏫",
            HotAction::MuteSounds => "🔊",
            HotAction::DismissAlert => "🔕",
            HotAction::ShowApp => "🏠",
            HotAction::CycleSize => "↔",
            HotAction::HotkeysToggle => "⌨",
        }
    }

    pub fn group(self) -> HotGroup {
        match self {
            HotAction::Plus | HotAction::Minus | HotAction::AddMany | HotAction::Undo | HotAction::Pause => HotGroup::Counting,
            HotAction::ToggleHide | HotAction::StartStop | HotAction::ClickThrough | HotAction::CycleSize => HotGroup::Overlay,
            HotAction::Caught | HotAction::DismissAlert => HotGroup::Hunt,
            HotAction::MuteSounds | HotAction::ShowApp | HotAction::HotkeysToggle => HotGroup::App,
        }
    }

    /// Is it new in this version (off until you set it)?
    pub fn is_new(self) -> bool {
        matches!(
            self,
            HotAction::StartStop | HotAction::AddMany | HotAction::MuteSounds | HotAction::DismissAlert | HotAction::ShowApp | HotAction::CycleSize | HotAction::HotkeysToggle
        )
    }

    /// A good combination to suggest when it's off.
    pub fn suggested(self) -> KeyBind {
        let cs = KeyBind::ctrl_shift;
        match self {
            HotAction::ToggleHide => cs("H"),
            HotAction::Plus => cs("Up"),
            HotAction::Minus => cs("Down"),
            HotAction::Pause => cs("P"),
            HotAction::Caught => cs("C"),
            HotAction::Undo => cs("Z"),
            HotAction::ClickThrough => cs("T"),
            HotAction::StartStop => cs("O"),
            HotAction::AddMany => cs("Right"),
            HotAction::MuteSounds => cs("M"),
            HotAction::DismissAlert => cs("D"),
            HotAction::ShowApp => cs("Home"),
            HotAction::CycleSize => cs("L"),
            HotAction::HotkeysToggle => KeyBind { ctrl: true, shift: true, alt: true, key: "H".into() },
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
    pub fn ctrl_shift(key: &str) -> Self {
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
        s.push_str(key_label(&self.key));
        s
    }

    /// The keys one by one, for drawing as key caps.
    pub fn parts(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.ctrl {
            v.push("Ctrl");
        }
        if self.alt {
            v.push("Alt");
        }
        if self.shift {
            v.push("Shift");
        }
        v
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
    pub start_stop: Option<KeyBind>,
    pub add_many: Option<KeyBind>,
    pub mute: Option<KeyBind>,
    pub dismiss: Option<KeyBind>,
    pub show_app: Option<KeyBind>,
    pub cycle_size: Option<KeyBind>,
    pub toggle_all: Option<KeyBind>,
    /// How many "Add several" adds.
    pub add_many_n: u64,
    /// Every hotkey but "All hotkeys off / on" is switched off.
    pub paused: bool,
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
            // The newer ones start off, so an update never takes a key
            // another app uses (the Hotkeys page suggests one for each).
            start_stop: None,
            add_many: None,
            mute: None,
            dismiss: None,
            show_app: None,
            cycle_size: None,
            toggle_all: None,
            add_many_n: 5,
            paused: false,
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
            HotAction::StartStop => self.start_stop.as_ref(),
            HotAction::AddMany => self.add_many.as_ref(),
            HotAction::MuteSounds => self.mute.as_ref(),
            HotAction::DismissAlert => self.dismiss.as_ref(),
            HotAction::ShowApp => self.show_app.as_ref(),
            HotAction::CycleSize => self.cycle_size.as_ref(),
            HotAction::HotkeysToggle => self.toggle_all.as_ref(),
        }
    }

    pub fn set(&mut self, a: HotAction, b: Option<KeyBind>) {
        let slot = match a {
            HotAction::ToggleHide => &mut self.hide,
            HotAction::Plus => &mut self.plus,
            HotAction::Minus => &mut self.minus,
            HotAction::Pause => &mut self.pause,
            HotAction::Caught => &mut self.caught,
            HotAction::Undo => &mut self.undo,
            HotAction::ClickThrough => &mut self.click_through,
            HotAction::StartStop => &mut self.start_stop,
            HotAction::AddMany => &mut self.add_many,
            HotAction::MuteSounds => &mut self.mute,
            HotAction::DismissAlert => &mut self.dismiss,
            HotAction::ShowApp => &mut self.show_app,
            HotAction::CycleSize => &mut self.cycle_size,
            HotAction::HotkeysToggle => &mut self.toggle_all,
        };
        *slot = b;
    }

    /// The action already using this combination, other than `except`.
    pub fn used_by(&self, b: &KeyBind, except: Option<HotAction>) -> Option<HotAction> {
        HotAction::ALL.into_iter().find(|&a| Some(a) != except && self.get(a) == Some(b))
    }

    /// " (Ctrl+Shift+H)" for tooltips, or "" when unbound (or all paused).
    pub fn hint(&self, a: HotAction) -> String {
        if self.paused && a != HotAction::HotkeysToggle {
            return String::new();
        }
        self.get(a)
            .map(|b| format!(" ({})", b.text()))
            .unwrap_or_default()
    }

    /// Give every unset action its suggested keys, unless they're taken.
    /// Returns how many were set.
    pub fn fill_suggested(&mut self) -> usize {
        let mut n = 0;
        for a in HotAction::ALL {
            if self.get(a).is_none() {
                let s = a.suggested();
                if self.used_by(&s, None).is_none() {
                    self.set(a, Some(s));
                    n += 1;
                }
            }
        }
        n
    }
}

/// How a key is shown ("Up" → "↑", "OpenBracket" → "[").
pub fn key_label(key: &str) -> &str {
    match key {
        "Up" | "ArrowUp" => "↑",
        "Down" | "ArrowDown" => "↓",
        "Left" | "ArrowLeft" => "←",
        "Right" | "ArrowRight" => "→",
        "PageUp" => "Page Up",
        "PageDown" => "Page Down",
        "Minus" => "-",
        "Equals" => "=",
        "OpenBracket" => "[",
        "CloseBracket" => "]",
        "Comma" => ",",
        "Period" => ".",
        "Semicolon" => ";",
        "Slash" => "/",
        "Backslash" => "\\",
        "Backtick" => "`",
        "Quote" => "'",
        k => k,
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
    // F13–F24 are handy for macro pads and stream decks.
    const FKEYS: [Code; 24] = [
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
        Code::F13,
        Code::F14,
        Code::F15,
        Code::F16,
        Code::F17,
        Code::F18,
        Code::F19,
        Code::F20,
        Code::F21,
        Code::F22,
        Code::F23,
        Code::F24,
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
        if (1..=24).contains(&n) {
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
        "Minus" => Code::Minus,
        "Equals" => Code::Equal,
        "OpenBracket" => Code::BracketLeft,
        "CloseBracket" => Code::BracketRight,
        "Comma" => Code::Comma,
        "Period" => Code::Period,
        "Semicolon" => Code::Semicolon,
        "Slash" => Code::Slash,
        "Backslash" => Code::Backslash,
        "Backtick" => Code::Backquote,
        "Quote" => Code::Quote,
        _ => return None,
    })
}

pub struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, HotAction)>,
    /// Problems from the last `apply` (e.g. a combo taken by another app).
    pub errors: Vec<String>,
    /// Actions whose keys couldn't be registered last time.
    pub failed: Vec<HotAction>,
}

impl Hotkeys {
    pub fn new(binds: &KeyBinds) -> Self {
        let mut me = match GlobalHotKeyManager::new() {
            Ok(m) => Self {
                manager: Some(m),
                registered: Vec::new(),
                errors: Vec::new(),
                failed: Vec::new(),
            },
            Err(e) => Self {
                manager: None,
                registered: Vec::new(),
                errors: vec![format!("Hotkeys unavailable: {e}")],
                failed: Vec::new(),
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
        self.failed.clear();
        for action in HotAction::ALL {
            // With every hotkey off, only the switch itself listens.
            if binds.paused && action != HotAction::HotkeysToggle {
                continue;
            }
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
                Err(_) => {
                    self.errors.push(format!(
                        "{} ({}) couldn't be set up: another app is probably using it. Pick a different one.",
                        bind.text(),
                        action.label()
                    ));
                    self.failed.push(action);
                }
            }
        }
    }

    /// Could hotkeys be set up at all?
    pub fn available(&self) -> bool {
        self.manager.is_some()
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
