//! Screen capture via `xcap`. Only the most basic xcap API is used
//! (`Monitor::all`, `capture_image`), so it works across xcap versions.

use crate::imageproc::Rgba;

pub fn monitor_count() -> usize {
    xcap::Monitor::all().map(|m| m.len()).unwrap_or(0)
}

/// One-off capture of a whole monitor.
pub fn grab(monitor: usize) -> Result<Rgba, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Can't list screens: {e}"))?;
    let m = monitors
        .get(monitor)
        .or_else(|| monitors.first())
        .ok_or("No screens found")?;
    let img = m
        .capture_image()
        .map_err(|e| format!("Screen capture failed: {e}"))?;
    let (w, h) = (img.width(), img.height());
    Ok(Rgba {
        w,
        h,
        px: img.into_raw(),
    })
}

/// Keeps the monitor list around between captures (used by the worker).
pub struct Capturer {
    monitors: Vec<xcap::Monitor>,
}

impl Capturer {
    pub fn new() -> Self {
        Self {
            monitors: xcap::Monitor::all().unwrap_or_default(),
        }
    }

    pub fn grab(&mut self, monitor: usize) -> Result<Rgba, String> {
        if self.monitors.is_empty() {
            self.monitors = xcap::Monitor::all().map_err(|e| format!("Can't list screens: {e}"))?;
        }
        let idx = if monitor < self.monitors.len() {
            monitor
        } else {
            0
        };
        let m = self.monitors.get(idx).ok_or("No screens found")?;
        match m.capture_image() {
            Ok(img) => {
                let (w, h) = (img.width(), img.height());
                Ok(Rgba {
                    w,
                    h,
                    px: img.into_raw(),
                })
            }
            Err(e) => {
                // Monitor setup may have changed; re-enumerate next time.
                self.monitors.clear();
                Err(format!("Screen capture failed: {e}"))
            }
        }
    }
}
