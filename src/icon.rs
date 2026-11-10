//! App icon (assets/icon-64.png), baked into the binary. Used for the window /
//! taskbar icon and drawn in the overlay header and hidden badge.

const PNG: &[u8] = include_bytes!("../assets/icon-64.png");

/// (RGBA pixels, width, height). Falls back to a blank icon if decoding fails.
pub fn rgba() -> (Vec<u8>, u32, u32) {
    match image::load_from_memory(PNG) {
        Ok(img) => {
            let img = img.to_rgba8();
            let (w, h) = (img.width(), img.height());
            (img.into_raw(), w, h)
        }
        Err(_) => (vec![0; 64 * 64 * 4], 64, 64),
    }
}
