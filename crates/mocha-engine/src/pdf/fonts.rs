//! Font selection and text measurement. Mirrors ReportLab `pdfmetrics.stringWidth`
//! (`instanceStringWidthT1`) and `rl_accel.unicode2T1` (per-character font fallback
//! Helvetica -> Symbol -> ZapfDingbats -> ZapfDingbats `n`).

use super::metrics;

/// One of the four standard Type 1 faces ReportLab may emit for this report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// `Helvetica`
    Helvetica,
    /// `Helvetica-Bold`
    HelveticaBold,
    /// `Symbol` (substitution font)
    Symbol,
    /// `ZapfDingbats` (substitution / notdef font)
    ZapfDingbats,
}

impl Face {
    /// PDF `BaseFont` name.
    pub fn base_font(self) -> &'static str {
        match self {
            Face::Helvetica => "Helvetica",
            Face::HelveticaBold => "Helvetica-Bold",
            Face::Symbol => "Symbol",
            Face::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Resource name used in content streams.
    pub fn resource_name(self) -> &'static str {
        match self {
            Face::Helvetica => "F1",
            Face::HelveticaBold => "F2",
            Face::Symbol => "F3",
            Face::ZapfDingbats => "F4",
        }
    }

    fn widths(self) -> &'static [u16; 256] {
        match self {
            Face::Helvetica => &metrics::HELVETICA_WIDTHS,
            Face::HelveticaBold => &metrics::HELVETICA_BOLD_WIDTHS,
            Face::Symbol => &metrics::SYMBOL_WIDTHS,
            Face::ZapfDingbats => &metrics::ZAPF_DINGBATS_WIDTHS,
        }
    }

    /// Face ascent in 1/1000 em (ReportLab `getAscentDescent`).
    pub fn ascent(self) -> i32 {
        match self {
            Face::Helvetica => metrics::HELVETICA_ASCENT,
            Face::HelveticaBold => metrics::HELVETICA_BOLD_ASCENT,
            Face::Symbol => metrics::SYMBOL_ASCENT,
            Face::ZapfDingbats => metrics::ZAPF_DINGBATS_ASCENT,
        }
    }

    /// Face descent in 1/1000 em (negative).
    pub fn descent(self) -> i32 {
        match self {
            Face::Helvetica => metrics::HELVETICA_DESCENT,
            Face::HelveticaBold => metrics::HELVETICA_BOLD_DESCENT,
            Face::Symbol => metrics::SYMBOL_DESCENT,
            Face::ZapfDingbats => metrics::ZAPF_DINGBATS_DESCENT,
        }
    }
}

fn lookup(table: &[(u32, u8)], c: char) -> Option<u8> {
    let key = c as u32;
    table
        .binary_search_by_key(&key, |e| e.0)
        .ok()
        .map(|i| table[i].1)
}

/// Mirrors `unicode2T1(text, [font] + font.substitutionFonts)`: returns runs of
/// `(face, bytes)`; characters the base face cannot encode go to Symbol, then
/// ZapfDingbats, then become ZapfDingbats `n`.
pub fn encode_runs(text: &str, base: Face) -> Vec<(Face, Vec<u8>)> {
    let mut runs: Vec<(Face, Vec<u8>)> = Vec::new();
    for c in text.chars() {
        let (face, byte) = if let Some(b) = lookup(&metrics::WINANSI, c) {
            (base, b)
        } else if let Some(b) = lookup(&metrics::SYMBOL_ENC, c) {
            (Face::Symbol, b)
        } else if let Some(b) = lookup(&metrics::ZAPF_ENC, c) {
            (Face::ZapfDingbats, b)
        } else {
            (Face::ZapfDingbats, b'n')
        };
        match runs.last_mut() {
            Some((f, bytes)) if *f == face => bytes.push(byte),
            _ => runs.push((face, vec![byte])),
        }
    }
    runs
}

/// Mirrors `pdfmetrics.stringWidth(text, fontName, fontSize)`:
/// `sum(widths) * 0.001 * size` (same operation order as ReportLab).
pub fn string_width(text: &str, face: Face, size: f64) -> f64 {
    let mut sum: i64 = 0;
    for (f, bytes) in encode_runs(text, face) {
        let w = f.widths();
        for b in bytes {
            sum += i64::from(w[usize::from(b)]);
        }
    }
    (sum as f64) * 0.001 * size
}
