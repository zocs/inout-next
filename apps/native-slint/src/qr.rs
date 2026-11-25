//! QR symbols for share addresses. The view draws the modules itself, so the
//! code stays sharp at any display scale and needs no image file.
use qrcodegen::{QrCode, QrCodeEcc};

/// Light modules required around a symbol on every side.
pub const QUIET_ZONE: i32 = 4;
const FINDER: i32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Module {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    size: i32,
    dark: Vec<bool>,
}

impl Symbol {
    /// Encode with quartile error correction. Returns `None` when the text
    /// cannot fit any QR version.
    pub fn encode(text: &str) -> Option<Self> {
        let code = QrCode::encode_text(text, QrCodeEcc::Quartile).ok()?;
        let size = code.size();
        let mut dark = Vec::with_capacity((size * size) as usize);
        for y in 0..size {
            for x in 0..size {
                dark.push(code.get_module(x, y));
            }
        }
        Some(Self { size, dark })
    }

    /// Modules per side, without the quiet zone.
    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn is_dark(&self, x: i32, y: i32) -> bool {
        (0..self.size).contains(&x)
            && (0..self.size).contains(&y)
            && self.dark[(y * self.size + x) as usize]
    }

    /// Top-left corners of the three 7×7 finder patterns.
    pub fn finders(&self) -> [Module; 3] {
        let far = self.size - FINDER;
        [
            Module { x: 0, y: 0 },
            Module { x: far, y: 0 },
            Module { x: 0, y: far },
        ]
    }

    fn in_finder(&self, x: i32, y: i32) -> bool {
        self.finders().iter().any(|corner| {
            (corner.x..corner.x + FINDER).contains(&x) && (corner.y..corner.y + FINDER).contains(&y)
        })
    }

    /// Dark modules outside the finder patterns, row by row. Views draw the
    /// finders as solid rings and these as individual marks.
    pub fn data_modules(&self) -> Vec<Module> {
        let mut modules = Vec::new();
        for y in 0..self.size {
            for x in 0..self.size {
                if self.is_dark(x, y) && !self.in_finder(x, y) {
                    modules.push(Module { x, y });
                }
            }
        }
        modules
    }
}
