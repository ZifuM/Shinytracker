//! Text recognition. On Windows this uses the OCR engine built into
//! Windows 10/11 (nothing to install). Elsewhere it shells out to `tesseract`.

use crate::imageproc::Gray;

pub struct Ocr {
    inner: imp::Engine,
}

impl Ocr {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            inner: imp::Engine::new()?,
        })
    }

    pub fn read(&self, img: &Gray) -> Result<String, String> {
        self.inner.read(img)
    }
}

#[cfg(windows)]
mod imp {
    use crate::imageproc::Gray;
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    fn err(e: windows::core::Error) -> String {
        format!("{e}")
    }

    pub struct Engine {
        engine: OcrEngine,
        max_dim: u32,
    }

    impl Engine {
        pub fn new() -> Result<Self, String> {
            let engine = OcrEngine::TryCreateFromUserProfileLanguages()
                .or_else(|_| {
                    let lang = Language::CreateLanguage(&HSTRING::from("en-US"))?;
                    OcrEngine::TryCreateFromLanguage(&lang)
                })
                .map_err(|e| {
                    format!(
                        "Windows OCR is unavailable ({e}). Add an English language pack in \
                         Settings > Time & language > Language."
                    )
                })?;
            let max_dim = OcrEngine::MaxImageDimension().unwrap_or(2600);
            Ok(Self { engine, max_dim })
        }

        pub fn read(&self, img: &Gray) -> Result<String, String> {
            if img.w > self.max_dim || img.h > self.max_dim {
                return Err(format!(
                    "Name boxes are too large to read ({}x{}); draw tighter boxes",
                    img.w, img.h
                ));
            }
            // Grey pixels, so BGRA vs RGBA channel order doesn't matter.
            let bgra = img.to_rgba();
            let writer = DataWriter::new().map_err(err)?;
            writer.WriteBytes(&bgra).map_err(err)?;
            let buffer = writer.DetachBuffer().map_err(err)?;
            let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
                &buffer,
                BitmapPixelFormat::Bgra8,
                img.w as i32,
                img.h as i32,
            )
            .map_err(err)?;
            let result = self
                .engine
                .RecognizeAsync(&bitmap)
                .map_err(err)?
                .get()
                .map_err(err)?;
            Ok(result.Text().map_err(err)?.to_string())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::imageproc::Gray;
    use std::process::Command;

    pub struct Engine;

    impl Engine {
        pub fn new() -> Result<Self, String> {
            Command::new("tesseract")
                .arg("--version")
                .output()
                .map_err(|_| {
                    "tesseract not found; install it (e.g. `apt install tesseract-ocr`)".to_string()
                })?;
            Ok(Self)
        }

        pub fn read(&self, img: &Gray) -> Result<String, String> {
            let path =
                std::env::temp_dir().join(format!("shiny-tracker-{}.pgm", std::process::id()));
            let mut pgm = format!("P5\n{} {}\n255\n", img.w, img.h).into_bytes();
            pgm.extend_from_slice(&img.px);
            std::fs::write(&path, pgm).map_err(|e| e.to_string())?;
            let out = Command::new("tesseract")
                .arg(&path)
                .args(["stdout", "--psm", "6"])
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        }
    }
}
