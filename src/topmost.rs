//! Keeps the overlay above the game, including borderless / "fullscreen"
//! windowed games that push themselves to the top when they get focus.
//!
//! A background thread re-asserts HWND_TOPMOST every 100 ms, independent of
//! the UI loop (which Windows pauses when the overlay is covered). It never
//! activates our window, so the game keeps keyboard focus.
//!
//! Borderless fix: an NVIDIA/AMD OpenGL game that exactly fills its monitor
//! gets drawn straight to the screen, hiding every other window. When Platinum
//! Together does that on the overlay's monitor, we make its window 1 px taller,
//! with the extra row off the screen edge so there's no gap and Windows still
//! treats it as fullscreen (taskbar stays hidden)
//! (window size only; the game's files and memory are never touched) so Windows
//! keeps compositing it and the overlay stays visible.
//!
//! With SHINY_TRACKER_LOG=1 set it writes `topmost-log.txt` next to the .exe
//! describing the game's window (fullscreen? topmost?) for diagnosing problems.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::fmt::Write as _;
    use std::io::Write as _;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    type Hwnd = *mut c_void;

    #[repr(C)]
    #[derive(Default, Clone, Copy, PartialEq)]
    struct Rect {
        l: i32,
        t: i32,
        r: i32,
        b: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct MonitorInfo {
        cb: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn IsWindow(h: Hwnd) -> i32;
        fn IsWindowVisible(h: Hwnd) -> i32;
        fn IsIconic(h: Hwnd) -> i32;
        fn GetForegroundWindow() -> Hwnd;
        fn GetWindowTextW(h: Hwnd, buf: *mut u16, len: i32) -> i32;
        fn GetClassNameW(h: Hwnd, buf: *mut u16, len: i32) -> i32;
        fn GetWindowLongPtrW(h: Hwnd, index: i32) -> isize;
        fn GetWindowRect(h: Hwnd, r: *mut Rect) -> i32;
        fn MonitorFromWindow(h: Hwnd, flags: u32) -> *mut c_void;
        fn GetMonitorInfoW(m: *mut c_void, mi: *mut MonitorInfo) -> i32;
        fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
        fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
        fn MonitorFromPoint(pt: Point, flags: u32) -> *mut c_void;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
        fn GetCurrentProcessId() -> u32;
    }

    const HWND_TOPMOST: isize = -1;
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOPMOST: isize = 0x0008;
    const WS_POPUP: isize = 0x8000_0000u32 as i32 as isize;
    const WS_CAPTION: isize = 0x00C0_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_NOOWNERZORDER: u32 = 0x0200;
    const SWP_ASYNCWINDOWPOS: u32 = 0x4000;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const MONITOR_DEFAULTTONULL: u32 = 0;

    fn text(h: Hwnd, class: bool) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe {
            if class {
                GetClassNameW(h, buf.as_mut_ptr(), 256)
            } else {
                GetWindowTextW(h, buf.as_mut_ptr(), 256)
            }
        };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn describe(h: Hwnd) -> String {
        if h.is_null() {
            return "none".into();
        }
        unsafe {
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            let st = GetWindowLongPtrW(h, GWL_STYLE);
            let mut r = Rect::default();
            GetWindowRect(h, &mut r);
            let mut mi = MonitorInfo {
                cb: std::mem::size_of::<MonitorInfo>() as u32,
                ..Default::default()
            };
            GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi);
            let covers = r.l <= mi.monitor.l
                && r.t <= mi.monitor.t
                && r.r >= mi.monitor.r
                && r.b >= mi.monitor.b;
            let mut s = String::new();
            let _ = write!(
                s,
                "{:?} title={:?} class={:?} topmost={} popup={} caption={} covers_monitor={} rect=({},{})-({},{})",
                h,
                text(h, false),
                text(h, true),
                ex & WS_EX_TOPMOST != 0,
                st & WS_POPUP != 0,
                st & WS_CAPTION != 0,
                covers,
                r.l,
                r.t,
                r.r,
                r.b
            );
            s
        }
    }

    struct Log {
        file: Option<std::fs::File>,
        lines: u32,
        start: Instant,
    }

    impl Log {
        /// Logging is off unless the SHINY_TRACKER_LOG environment variable is set.
        fn new() -> Self {
            if std::env::var_os("SHINY_TRACKER_LOG").is_none() {
                return Self {
                    file: None,
                    lines: 0,
                    start: Instant::now(),
                };
            }
            let file = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("topmost-log.txt")))
                .and_then(|p| std::fs::File::create(p).ok());
            Self {
                file,
                lines: 0,
                start: Instant::now(),
            }
        }
        fn line(&mut self, msg: &str) {
            if self.lines > 400 {
                return;
            }
            self.lines += 1;
            if let Some(f) = &mut self.file {
                let _ = writeln!(f, "[{:>6.1}s] {msg}", self.start.elapsed().as_secs_f32());
                let _ = f.flush();
            }
        }
    }

    pub fn hwnd_of(frame: &eframe::Frame) -> Option<isize> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = frame.window_handle() {
            if let RawWindowHandle::Win32(w) = handle.as_raw() {
                return Some(w.hwnd.get());
            }
        }
        find_own_window()
    }

    /// Fallback: our process's visible top-level window titled "Shiny Tracker".
    fn find_own_window() -> Option<isize> {
        extern "system" fn cb(h: Hwnd, out: isize) -> i32 {
            unsafe {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, &mut pid);
                if pid == GetCurrentProcessId()
                    && IsWindowVisible(h) != 0
                    && text(h, false) == "Shiny Tracker"
                {
                    *(out as *mut isize) = h as isize;
                    return 0;
                }
            }
            1
        }
        let mut found: isize = 0;
        unsafe {
            EnumWindows(cb, &mut found as *mut isize as isize);
        }
        (found != 0).then_some(found)
    }

    fn monitor_of(h: Hwnd) -> (*mut c_void, Rect) {
        unsafe {
            let m = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
            let mut mi = MonitorInfo {
                cb: std::mem::size_of::<MonitorInfo>() as u32,
                ..Default::default()
            };
            GetMonitorInfoW(m, &mut mi);
            (m, mi.monitor)
        }
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(h: Hwnd, attr: u32, value: *const c_void, size: u32) -> i32;
    }

    /// Ask Windows 11 to round the settings window's corners (it has no
    /// title bar, and its corners would otherwise show as dark squares).
    pub fn round_settings_corners(done: &mut bool) {
        if *done {
            return;
        }
        extern "system" fn cb(h: Hwnd, out: isize) -> i32 {
            unsafe {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, &mut pid);
                if pid == GetCurrentProcessId() && text(h, false).starts_with(super::LAUNCHER_TITLE) {
                    *(out as *mut isize) = h as isize;
                    return 0;
                }
            }
            1
        }
        let mut found: isize = 0;
        unsafe {
            EnumWindows(cb, &mut found as *mut isize as isize);
        }
        if found == 0 {
            return;
        }
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWCP_ROUND: u32 = 2;
        let pref = DWMWCP_ROUND;
        unsafe {
            DwmSetWindowAttribute(
                found as Hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const u32 as *const c_void,
                4,
            );
        }
        // Done either way (older Windows just ignores it).
        *done = true;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> *mut c_void;
        fn CombineRgn(dst: *mut c_void, a: *mut c_void, b: *mut c_void, mode: i32) -> i32;
        fn DeleteObject(o: *mut c_void) -> i32;
        fn CreatePolygonRgn(pts: *const [i32; 2], n: i32, mode: i32) -> *mut c_void;
        fn CreateRectRgn(x1: i32, y1: i32, x2: i32, y2: i32) -> *mut c_void;
    }
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowRgn(h: Hwnd, rgn: *mut c_void, redraw: i32) -> i32;
    }

    /// Cut a window to a set of rounded boxes ([x, y, w, h, radius] in
    /// pixels). No boxes: the whole window shows again.
    pub fn clip_window(hwnd: isize, boxes: &[[i32; 5]]) {
        const RGN_OR: i32 = 2;
        unsafe {
            if IsWindow(hwnd as Hwnd) == 0 {
                return;
            }
            let mut all: *mut c_void = std::ptr::null_mut();
            for b in boxes {
                let r = CreateRoundRectRgn(b[0], b[1], b[0] + b[2] + 1, b[1] + b[3] + 1, b[4] * 2, b[4] * 2);
                if r.is_null() {
                    continue;
                }
                if all.is_null() {
                    all = r;
                } else {
                    CombineRgn(all, all, r, RGN_OR);
                    DeleteObject(r);
                }
            }
            // Windows owns the region from here on.
            SetWindowRgn(hwnd as Hwnd, all, 1);
        }
    }

    /// Cut a window to any shape: polygons, plain boxes [x, y, w, h] and
    /// rounded boxes [x, y, w, h, radius], all in pixels.
    pub fn clip_window_shapes(hwnd: isize, polys: &[Vec<[i32; 2]>], boxes: &[[i32; 4]], rounded: &[[i32; 5]]) {
        const RGN_OR: i32 = 2;
        const WINDING: i32 = 2;
        unsafe {
            if IsWindow(hwnd as Hwnd) == 0 {
                return;
            }
            let all: *mut c_void = CreateRectRgn(0, 0, 0, 0);
            if all.is_null() {
                return;
            }
            let mut parts: Vec<*mut c_void> = Vec::new();
            for p in polys {
                if p.len() >= 3 {
                    parts.push(CreatePolygonRgn(p.as_ptr(), p.len() as i32, WINDING));
                }
            }
            for b in boxes {
                parts.push(CreateRectRgn(b[0], b[1], b[0] + b[2], b[1] + b[3]));
            }
            for b in rounded {
                parts.push(CreateRoundRectRgn(b[0], b[1], b[0] + b[2] + 1, b[1] + b[3] + 1, b[4] * 2, b[4] * 2));
            }
            for r in parts {
                if !r.is_null() {
                    CombineRgn(all, all, r, RGN_OR);
                    DeleteObject(r);
                }
            }
            // Windows owns the region from here on.
            SetWindowRgn(hwnd as Hwnd, all, 1);
        }
    }

    /// One of our own windows, found by its exact title.
    pub fn own_window_titled(title: &str) -> Option<isize> {
        struct Find {
            title: String,
            out: isize,
        }
        extern "system" fn cb(h: Hwnd, lp: isize) -> i32 {
            unsafe {
                let f = &mut *(lp as *mut Find);
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, &mut pid);
                if pid == GetCurrentProcessId() && text(h, false) == f.title {
                    f.out = h as isize;
                    return 0;
                }
            }
            1
        }
        let mut f = Find { title: title.to_string(), out: 0 };
        unsafe {
            EnumWindows(cb, &mut f as *mut Find as isize);
        }
        (f.out != 0).then_some(f.out)
    }

    /// Is the game open (its window exists)?
    pub fn game_running() -> bool {
        find_game().is_some()
    }

    /// The game window's handle (for Shiny Replay's window capture).
    pub fn game_hwnd() -> Option<isize> {
        find_game().map(|h| h as isize)
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn QueryFullProcessImageNameW(process: *mut c_void, flags: u32, name: *mut u16, size: *mut u32) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    /// Full path of the program that owns a window.
    fn exe_of(h: Hwnd) -> Option<std::path::PathBuf> {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            if pid == 0 {
                return None;
            }
            let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if proc.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(proc, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(proc);
            (ok != 0).then(|| std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
        }
    }

    /// Where the running game's Platinum.exe is (its windows are checked,
    /// so the launcher, which also says "Platinum Together", is skipped).
    pub fn game_exe() -> Option<std::path::PathBuf> {
        extern "system" fn cb(h: Hwnd, out: isize) -> i32 {
            unsafe {
                if IsWindowVisible(h) != 0 && text(h, false).contains("Platinum Together") {
                    (*(out as *mut Vec<isize>)).push(h as isize);
                }
            }
            1
        }
        let mut found: Vec<isize> = Vec::new();
        unsafe {
            EnumWindows(cb, &mut found as *mut Vec<isize> as isize);
        }
        found.into_iter().filter_map(|h| exe_of(h as Hwnd)).find(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("Platinum.exe"))
        })
    }

    /// The Platinum Together game window (SDL window titled "Platinum Together").
    fn find_game() -> Option<Hwnd> {
        extern "system" fn cb(h: Hwnd, out: isize) -> i32 {
            unsafe {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, &mut pid);
                if pid != GetCurrentProcessId()
                    && IsWindowVisible(h) != 0
                    && text(h, true) == "SDL_app"
                    && text(h, false).contains("Platinum Together")
                {
                    *(out as *mut isize) = h as isize;
                    return 0;
                }
            }
            1
        }
        let mut found: isize = 0;
        unsafe {
            EnumWindows(cb, &mut found as *mut isize as isize);
        }
        (found != 0).then_some(found as Hwnd)
    }

    /// Keeps the borderless fix from fighting a game that resizes itself back.
    struct FixState {
        game: Option<Hwnd>,
        last_search: Instant,
        recent: std::collections::VecDeque<Instant>,
        paused_until: Option<Instant>,
    }

    impl FixState {
        fn new() -> Self {
            Self {
                game: None,
                last_search: Instant::now()
                    .checked_sub(Duration::from_secs(10))
                    .unwrap_or_else(Instant::now),
                recent: Default::default(),
                paused_until: None,
            }
        }

        fn tick(&mut self, ours: Hwnd, log: &mut Log) {
            if self.paused_until.is_some_and(|t| Instant::now() < t) {
                return;
            }
            let valid = self.game.is_some_and(|g| unsafe { IsWindow(g) != 0 });
            if !valid {
                if self.last_search.elapsed() < Duration::from_secs(2) {
                    return;
                }
                self.last_search = Instant::now();
                self.game = find_game();
                if let Some(g) = self.game {
                    log.line(&format!("game window found: {}", describe(g)));
                }
            }
            let Some(game) = self.game else { return };
            unsafe {
                if IsIconic(game) != 0 {
                    return;
                }
                let mut r = Rect::default();
                GetWindowRect(game, &mut r);
                let (gm, mon) = monitor_of(game);
                let (om, _) = monitor_of(ours);
                // Only when the game exactly fills the overlay's monitor (or is still
                // 1 px short from the older version of this fix).
                let left_short = Rect {
                    b: mon.b - 1,
                    ..mon
                };
                if gm != om || (r != mon && r != left_short) {
                    return;
                }
                let r = mon;
                // Back off if the game keeps snapping back (avoid a resize war).
                let now = Instant::now();
                while self
                    .recent
                    .front()
                    .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(10))
                {
                    self.recent.pop_front();
                }
                if self.recent.len() >= 8 {
                    log.line("game keeps resizing back to full screen; pausing the borderless fix for 30 s");
                    self.paused_until = Some(now + Duration::from_secs(30));
                    self.recent.clear();
                    return;
                }
                self.recent.push_back(now);
                // Grow 1 px past the bottom edge, or past the top edge if another
                // monitor sits directly below (so the extra row is never visible).
                let below = Point { x: r.l + 1, y: r.b };
                let grow_down = MonitorFromPoint(below, MONITOR_DEFAULTTONULL).is_null();
                let top = if grow_down { r.t } else { r.t - 1 };
                SetWindowPos(
                    game,
                    std::ptr::null_mut(),
                    r.l,
                    top,
                    r.r - r.l,
                    r.b - r.t + 1,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS,
                );
                log.line(&format!(
                    "borderless fix: game window made 1 px taller, off the {} edge",
                    if grow_down { "bottom" } else { "top" }
                ));
            }
        }
    }

    /// Where to open the settings window: beside the overlay on whichever
    /// side has more room, kept inside the overlay's monitor work area.
    /// Inputs/outputs are in egui points; `ppp` = physical pixels per point.
    pub fn settings_placement(
        frame: &eframe::Frame,
        want: [f32; 2],
        min: [f32; 2],
        ppp: f32,
    ) -> Option<([f32; 2], [f32; 2])> {
        let h = hwnd_of(frame)? as Hwnd;
        unsafe {
            let mut ov = Rect::default();
            if GetWindowRect(h, &mut ov) == 0 {
                return None;
            }
            let mut mi = MonitorInfo {
                cb: std::mem::size_of::<MonitorInfo>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) == 0 {
                return None;
            }
            let wa = mi.work;
            let s = ppp.max(0.5);
            let px = |v: f32| (v * s).round() as i32;
            let (margin, gap, title) = (px(12.0), px(10.0), px(32.0));
            let (area_w, area_h) = (wa.r - wa.l - 2 * margin, wa.b - wa.t - 2 * margin);
            if area_w < 100 || area_h < 100 {
                return None;
            }
            let space_left = ov.l - wa.l - gap - margin;
            let space_right = wa.r - ov.r - gap - margin;
            let right = space_right >= space_left;
            let space = space_right.max(space_left);
            // Prefer fitting beside the overlay (shrinking down to the minimum size).
            let mut w = px(want[0]);
            if w > space {
                w = space.max(px(min[0]));
            }
            let w = w.min(area_w);
            let hgt = (px(want[1]) + title)
                .min(area_h)
                .max(px(min[1]).min(area_h));
            let x = if right { ov.r + gap } else { ov.l - gap - w };
            let x = x.clamp(wa.l + margin, wa.r - margin - w);
            let y = ov.t.clamp(wa.t + margin, wa.b - margin - hgt);
            Some((
                [x as f32 / s, y as f32 / s],
                [w as f32 / s, (hgt - title) as f32 / s],
            ))
        }
    }

    /// Keep a window of `size` fully on a monitor: the one the game is on
    /// when it's open. With a position on that monitor: there, pulled in if it
    /// sticks out. Otherwise: that monitor's top-right corner (the primary
    /// monitor's when the game is closed and there's no position). Points in and out; `ppp` = pixels per point.
    pub fn fit_on_screen(pos: Option<[f32; 2]>, size: [f32; 2], ppp: f32, on_game: bool) -> [f32; 2] {
        const MONITOR_DEFAULTTOPRIMARY: u32 = 1;
        let s = ppp.max(0.5);
        let px = |v: f32| (v * s).round() as i32;
        unsafe {
            // The game's monitor wins: a remembered spot on another screen
            // is dropped for the top-right of the game's screen.
            let near = pos.map(|p| MonitorFromPoint(Point { x: px(p[0] + size[0] / 2.0), y: px(p[1] + 8.0) }, MONITOR_DEFAULTTONEAREST));
            let game = if on_game { find_game().map(|g| MonitorFromWindow(g, MONITOR_DEFAULTTONEAREST)).filter(|m| !m.is_null()) } else { None };
            let pos = match (near, game) {
                (Some(n), Some(g)) if n != g => None,
                _ => pos,
            };
            let m = match (pos, game, near) {
                (_, Some(g), _) => g,
                (Some(_), None, Some(n)) => n,
                _ => MonitorFromPoint(Point { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY),
            };
            let mut mi = MonitorInfo { cb: std::mem::size_of::<MonitorInfo>() as u32, ..Default::default() };
            if m.is_null() || GetMonitorInfoW(m, &mut mi) == 0 {
                return pos.unwrap_or([24.0, 120.0]);
            }
            let wa = mi.work;
            let margin = px(16.0);
            let (w, h) = (px(size[0]), px(size[1]));
            let (x, y) = match pos {
                Some(p) => (px(p[0]), px(p[1])),
                None => (wa.r - margin - w, wa.t + margin),
            };
            // Fully inside (or as much as fits when it's bigger than the screen).
            let x = x.min(wa.r - margin - w).max(wa.l + margin.min((wa.r - wa.l - w).max(0) / 2));
            let y = y.min(wa.b - margin - h).max(wa.t + margin.min((wa.b - wa.t - h).max(0) / 2));
            [x as f32 / s, y as f32 / s]
        }
    }

    /// Is this point (egui points; `ppp` = pixels per point) on any monitor?
    /// Used to check a remembered window position is still visible.
    pub fn point_on_screen(pos: [f32; 2], ppp: f32) -> bool {
        let s = ppp.max(0.5);
        let pt = Point { x: (pos[0] * s).round() as i32, y: (pos[1] * s).round() as i32 };
        unsafe { !MonitorFromPoint(pt, MONITOR_DEFAULTTONULL).is_null() }
    }

    pub fn spawn_keeper(hwnd: isize, fix: Arc<AtomicBool>) {
        let _ = std::thread::Builder::new()
            .name("stay-on-top".into())
            .spawn(move || {
                let ours = hwnd as Hwnd;
                let mut log = Log::new();
                log.line(&format!("keeper started for {}", describe(ours)));
                let mut last_fg: Hwnd = std::ptr::null_mut();
                let mut last_report = Instant::now();
                let mut failures = 0u32;
                let mut fix_state = FixState::new();
                loop {
                    unsafe {
                        if IsWindow(ours) == 0 {
                            log.line("overlay window closed; keeper stopping");
                            return;
                        }
                        let ok = SetWindowPos(
                            ours,
                            HWND_TOPMOST as Hwnd,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOSIZE
                                | SWP_NOMOVE
                                | SWP_NOACTIVATE
                                | SWP_NOOWNERZORDER
                                | SWP_ASYNCWINDOWPOS,
                        );
                        if ok == 0 {
                            failures += 1;
                            if failures <= 5 {
                                log.line(&format!("SetWindowPos failed, error {}", GetLastError()));
                            }
                        }
                        let fg = GetForegroundWindow();
                        if fg != last_fg || last_report.elapsed() > Duration::from_secs(10) {
                            last_fg = fg;
                            last_report = Instant::now();
                            let our_ex = GetWindowLongPtrW(ours, GWL_EXSTYLE);
                            log.line(&format!(
                                "foreground: {} | overlay topmost={} visible={} minimized={}",
                                describe(fg),
                                our_ex & WS_EX_TOPMOST != 0,
                                IsWindowVisible(ours) != 0,
                                IsIconic(ours) != 0
                            ));
                        }
                    }
                    if fix.load(Ordering::Relaxed) {
                        fix_state.tick(ours, &mut log);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            });
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn own_window_titled(_title: &str) -> Option<isize> {
        None
    }
    pub fn round_settings_corners(done: &mut bool) {
        *done = true;
    }
    pub fn clip_window(_hwnd: isize, _boxes: &[[i32; 5]]) {}
    pub fn clip_window_shapes(_hwnd: isize, _polys: &[Vec<[i32; 2]>], _boxes: &[[i32; 4]], _rounded: &[[i32; 5]]) {}
    pub fn game_running() -> bool {
        true
    }
    pub fn game_hwnd() -> Option<isize> {
        None
    }
    pub fn game_exe() -> Option<std::path::PathBuf> {
        None
    }
    pub fn hwnd_of(_frame: &eframe::Frame) -> Option<isize> {
        None
    }
    pub fn fit_on_screen(pos: Option<[f32; 2]>, _size: [f32; 2], _ppp: f32, _on_game: bool) -> [f32; 2] {
        pos.unwrap_or([24.0, 120.0])
    }
    pub fn point_on_screen(_pos: [f32; 2], _ppp: f32) -> bool {
        true
    }
    pub fn spawn_keeper(_hwnd: isize, _fix: std::sync::Arc<std::sync::atomic::AtomicBool>) {}
    pub fn settings_placement(
        _f: &eframe::Frame,
        _w: [f32; 2],
        _m: [f32; 2],
        _p: f32,
    ) -> Option<([f32; 2], [f32; 2])> {
        None
    }
}

/// Call every frame; starts the keeper thread once the window handle exists.
/// `borderless_fix` can be flipped at any time from the settings.
pub fn ensure_started(
    started: &mut bool,
    frame: &eframe::Frame,
    borderless_fix: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if *started {
        return;
    }
    if let Some(h) = imp::hwnd_of(frame) {
        imp::spawn_keeper(h, borderless_fix.clone());
        *started = true;
    }
}

/// Title of the launcher (main) window.
pub const LAUNCHER_TITLE: &str = "Poke Together Companion";
/// Title of the Shiny Hunter overlay window (how the keeper finds it).
pub const OVERLAY_TITLE: &str = "Shiny Hunter overlay";
/// The Card Collector's XP bar overlay.
pub const CARD_BAR_TITLE: &str = "Card Collector XP bar";
/// The Pokégotchi on the overlay.
pub const GOTCHI_TITLE: &str = "Pokegotchi overlay";

/// Like `ensure_overlay_keeper`, for another overlay window by its title.
pub fn ensure_keeper_titled(title: &str, last: &mut Option<isize>, borderless_fix: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
    if let Some(h) = imp::own_window_titled(title) {
        if *last != Some(h) {
            imp::spawn_keeper(h, borderless_fix.clone());
            *last = Some(h);
        }
    }
}

/// Call every frame while the overlay is open: starts a keeper for each new
/// overlay window (a closed one's keeper stops by itself).
pub fn ensure_overlay_keeper(
    last: &mut Option<isize>,
    borderless_fix: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if let Some(h) = imp::own_window_titled(OVERLAY_TITLE) {
        if *last != Some(h) {
            imp::spawn_keeper(h, borderless_fix.clone());
            *last = Some(h);
        }
    }
}

/// Position and size (in points) for the settings window beside the overlay.
pub fn settings_placement(
    frame: &eframe::Frame,
    want: [f32; 2],
    min: [f32; 2],
    ppp: f32,
) -> Option<([f32; 2], [f32; 2])> {
    imp::settings_placement(frame, want, min, ppp)
}

/// Where to put a window of `size` (points) so it's fully on a monitor:
/// near `pos` if given, else the primary monitor's top-right corner.
/// `on_game`: put it on the game's monitor (when the game is open).
pub fn fit_on_screen(pos: Option<[f32; 2]>, size: [f32; 2], ppp: f32, on_game: bool) -> [f32; 2] {
    imp::fit_on_screen(pos, size, ppp, on_game)
}

/// Whether a point (in egui points) is on a connected monitor, so a
/// remembered window position isn't restored somewhere off-screen.
pub fn point_on_screen(pos: [f32; 2], ppp: f32) -> bool {
    imp::point_on_screen(pos, ppp)
}

/// Whether the Platinum Together game window is open.
pub fn game_running() -> bool {
    imp::game_running()
}

/// The game window, if it's open (its handle as a number).
#[allow(dead_code)]
pub fn game_hwnd() -> Option<isize> {
    imp::game_hwnd()
}

/// Where the running game's Platinum.exe is, if the game is open.
pub fn game_exe() -> Option<std::path::PathBuf> {
    imp::game_exe()
}

/// Cut a window to a set of rounded boxes ([x, y, w, h, radius], pixels).
pub fn clip_window(hwnd: isize, boxes: &[[i32; 5]]) {
    imp::clip_window(hwnd, boxes)
}

/// Cut a window to polygons, boxes and rounded boxes (pixels), so only what's
/// drawn shows (on some PCs see-through window parts come out black).
pub fn clip_window_shapes(hwnd: isize, polys: &[Vec<[i32; 2]>], boxes: &[[i32; 4]], rounded: &[[i32; 5]]) {
    imp::clip_window_shapes(hwnd, polys, boxes, rounded)
}

/// Round the settings window's corners (Windows 11). Call each frame while
/// it's open; `done` stops repeat work.
pub fn round_settings_corners(done: &mut bool) {
    imp::round_settings_corners(done)
}
