//! The shiny ★ alert reading the game window itself (Windows Graphics
//! Capture, like OBS's "Window Capture") instead of the screen, so a window
//! on top of the game doesn't hide the ★. Experimental, behind its own
//! switch. It only looks at the window: the game's files and memory are
//! never touched. A minimised game draws nothing, so nothing is seen then.

use crate::imageproc::{Region, Rgba};
#[cfg(all(windows, feature = "replay-rec"))]
use crate::replay::Capture;
use std::time::{Duration, Instant};

/// The game's picture, kept open while the alert wants it.
#[derive(Default)]
pub struct GameEye {
    #[cfg(all(windows, feature = "replay-rec"))]
    cap: Option<Box<dyn crate::replay::Capture>>,
    /// The last picture (the window only sends one when something changed).
    last: Option<Rgba>,
    /// Don't try to open it again before this (the game isn't open, say).
    retry: Option<Instant>,
    /// The last look (it's let go of a while after battles).
    used: Option<Instant>,
}

impl GameEye {
    /// The game's picture and where it sits on its screen now (to move the
    /// name boxes into it). None: it can't be seen right now; use the screen.
    #[cfg(all(windows, feature = "replay-rec"))]
    pub fn look(&mut self) -> Option<(Rgba, (i32, i32))> {
        self.used = Some(Instant::now());
        if self.cap.is_none() {
            if self.retry.is_some_and(|t| Instant::now() < t) {
                return None;
            }
            let mut c = crate::replay_win::capture().ok()?;
            if c.open().is_err() {
                self.retry = Some(Instant::now() + Duration::from_secs(3));
                return None;
            }
            self.cap = Some(c);
        }
        let at = crate::replay_win::game_on_its_screen()?;
        match self.cap.as_mut()?.grab() {
            Ok(Some(f)) => {
                // BGRA to RGBA
                let mut px = f.px;
                for p in px.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
                self.last = Some(Rgba { w: f.w, h: f.h, px });
            }
            Ok(None) => {}
            Err(_) => {
                // closed or minimised: start again in a bit
                self.close();
                self.retry = Some(Instant::now() + Duration::from_secs(3));
                return None;
            }
        }
        self.last.clone().map(|f| (f, at))
    }

    #[cfg(not(all(windows, feature = "replay-rec")))]
    pub fn look(&mut self) -> Option<(Rgba, (i32, i32))> {
        let _ = (&self.last, &self.retry, &self.used, Duration::ZERO, Instant::now());
        None
    }

    /// Let go of the window once no battle has wanted it for a while (the
    /// window is only watched around battles; on Windows 10 a watched
    /// window gets a yellow border).
    pub fn rest(&mut self) {
        if self.used.is_some_and(|t| t.elapsed() > Duration::from_secs(8)) {
            self.close();
            self.used = None;
        }
    }

    /// Let go of the window (the switch went off).
    pub fn close(&mut self) {
        #[cfg(all(windows, feature = "replay-rec"))]
        if let Some(mut c) = self.cap.take() {
            c.close();
        }
        self.last = None;
    }
}

/// Where the game's picture is on its screen right now (None: the game
/// isn't open, or not on Windows).
pub fn game_at() -> Option<(i32, i32)> {
    #[cfg(all(windows, feature = "replay-rec"))]
    {
        crate::replay_win::game_on_its_screen()
    }
    #[cfg(not(all(windows, feature = "replay-rec")))]
    {
        None
    }
}

/// A name box drawn on the screen, as a place in the game's picture.
/// `at`: where the game's picture is on that screen. None: the box is
/// outside the game.
pub fn into_game(r: Region, at: (i32, i32)) -> Option<Region> {
    let x = r.x as i32 - at.0;
    let y = r.y as i32 - at.1;
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = (x + r.w as i32, y + r.h as i32);
    (x1 > x0 && y1 > y0).then(|| Region { x: x0 as u32, y: y0 as u32, w: (x1 - x0) as u32, h: (y1 - y0) as u32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_boxes_move_into_the_game_picture() {
        let r = Region { x: 140, y: 90, w: 300, h: 60 };
        // a game window whose picture starts at 100,50 on its screen
        let g = into_game(r, (100, 50)).unwrap();
        assert_eq!((g.x, g.y, g.w, g.h), (40, 40, 300, 60));
        // fullscreen: nothing moves
        let g = into_game(r, (0, 0)).unwrap();
        assert_eq!((g.x, g.y), (140, 90));
        // a box half off the game's left edge keeps the half on it
        let g = into_game(Region { x: 80, y: 90, w: 40, h: 20 }, (100, 50)).unwrap();
        assert_eq!((g.x, g.w), (0, 20));
        // a box wholly off the game
        assert!(into_game(Region { x: 10, y: 10, w: 40, h: 20 }, (100, 50)).is_none());
    }
}
