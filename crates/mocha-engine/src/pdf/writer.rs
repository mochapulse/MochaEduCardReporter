//! PDF object emission with `pdf-writer` plus a small content-stream builder that
//! plays the role of the ReportLab canvas calls this report uses
//! (`saveState`, `translate`, `setFillColor`, `line`, `rect`, `drawString`, ...).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pdf_writer::{Name, Pdf, Rect, Ref, TextStr};

use super::fonts::{encode_runs, Face};

/// An RGB colour with components in 0..=1 (ReportLab `Color`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red.
    pub r: f64,
    /// Green.
    pub g: f64,
    /// Blue.
    pub b: f64,
}

impl Color {
    /// `colors.black`
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    /// `colors.red`
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
    };
    /// `colors.lightgrey` (ReportLab: `Color(0.827451, ...)`, i.e. 211/255).
    pub const LIGHTGREY: Color = Color {
        r: 0.827451,
        g: 0.827451,
        b: 0.827451,
    };

    /// `colors.HexColor("#RRGGBB")`.
    pub fn hex(rgb: u32) -> Color {
        Color {
            r: f64::from((rgb >> 16) & 0xFF) / 255.0,
            g: f64::from((rgb >> 8) & 0xFF) / 255.0,
            b: f64::from(rgb & 0xFF) / 255.0,
        }
    }
}

/// Format a number like ReportLab's `fp_str` (up to 6 decimals, trimmed).
pub fn fp(v: f64) -> String {
    let mut s = format!("{v:.6}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Content stream of one page.
#[derive(Debug, Default, Clone)]
pub struct Content {
    buf: String,
    faces: BTreeSet<u8>,
}

fn face_index(f: Face) -> u8 {
    match f {
        Face::Helvetica => 0,
        Face::HelveticaBold => 1,
        Face::Symbol => 2,
        Face::ZapfDingbats => 3,
    }
}

fn face_from_index(i: u8) -> Face {
    match i {
        0 => Face::Helvetica,
        1 => Face::HelveticaBold,
        2 => Face::Symbol,
        _ => Face::ZapfDingbats,
    }
}

impl Content {
    /// Empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// The operators written so far.
    pub fn as_str(&self) -> &str {
        &self.buf
    }

    fn op(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    /// `canvas.saveState()`
    pub fn save_state(&mut self) {
        self.op("q");
    }

    /// `canvas.restoreState()`
    pub fn restore_state(&mut self) {
        self.op("Q");
    }

    /// `canvas.translate(x, y)`
    pub fn translate(&mut self, x: f64, y: f64) {
        self.op(&format!("1 0 0 1 {} {} cm", fp(x), fp(y)));
    }

    /// `canvas.setFillColor`
    pub fn set_fill(&mut self, c: Color) {
        self.op(&format!("{} {} {} rg", fp(c.r), fp(c.g), fp(c.b)));
    }

    /// `canvas.setStrokeColor`
    pub fn set_stroke(&mut self, c: Color) {
        self.op(&format!("{} {} {} RG", fp(c.r), fp(c.g), fp(c.b)));
    }

    /// `canvas.setLineWidth`
    pub fn set_line_width(&mut self, w: f64) {
        self.op(&format!("{} w", fp(w)));
    }

    /// `canvas.setLineCap`
    pub fn set_line_cap(&mut self, cap: i32) {
        self.op(&format!("{cap} J"));
    }

    /// `canvas.setLineJoin`
    pub fn set_line_join(&mut self, join: i32) {
        self.op(&format!("{join} j"));
    }

    /// `canvas.line(x1, y1, x2, y2)`
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.op(&format!(
            "{} {} m {} {} l S",
            fp(x1),
            fp(y1),
            fp(x2),
            fp(y2)
        ));
    }

    /// `canvas.rect(x, y, w, h, stroke=0, fill=1)`
    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.op(&format!("{} {} {} {} re f", fp(x), fp(y), fp(w), fp(h)));
    }

    /// Show `text` with its baseline origin at `(x, y)` (user space), in base face
    /// `face` at `size`, with optional word spacing (`Tw`). Characters outside the
    /// base encoding switch to the substitution faces exactly like ReportLab.
    pub fn show_text(
        &mut self,
        x: f64,
        y: f64,
        face: Face,
        size: f64,
        word_space: f64,
        text: &str,
    ) {
        if text.is_empty() {
            return;
        }
        let mut s = String::new();
        let _ = write!(s, "BT 1 0 0 1 {} {} Tm ", fp(x), fp(y));
        if word_space != 0.0 {
            let _ = write!(s, "{} Tw ", fp(word_space));
        }
        let _ = write!(s, "/{} {} Tf ", face.resource_name(), fp(size));
        self.faces.insert(face_index(face));
        let mut cur = face;
        for (f, bytes) in encode_runs(text, face) {
            if f != cur {
                let _ = write!(s, "/{} {} Tf ", f.resource_name(), fp(size));
                self.faces.insert(face_index(f));
                cur = f;
            }
            s.push('(');
            for b in bytes {
                match b {
                    b'(' | b')' | b'\\' => {
                        s.push('\\');
                        s.push(char::from(b));
                    }
                    0x20..=0x7e => s.push(char::from(b)),
                    _ => {
                        let _ = write!(s, "\\{b:03o}");
                    }
                }
            }
            s.push_str(") Tj ");
        }
        s.push_str("ET");
        self.op(&s);
    }
}

/// Document-level data for [`write_pdf`].
#[derive(Debug, Clone)]
pub struct DocInfo<'a> {
    /// `/Title` (ReportLab `title=`).
    pub title: &'a str,
    /// `/Author` (ReportLab `author=`).
    pub author: &'a str,
}

/// Assemble a PDF from page streams. Standard Type 1 fonts, `WinAnsiEncoding`
/// on the text faces, not embedded (exactly like ReportLab).
pub fn write_pdf(pages: &[Content], page_w: f64, page_h: f64, info: &DocInfo<'_>) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let pages_id = Ref::new(2);
    let info_id = Ref::new(3);
    let font_ids: [Ref; 4] = [Ref::new(4), Ref::new(5), Ref::new(6), Ref::new(7)];
    let mut next = 8;
    let mut page_ids = Vec::new();
    let mut content_ids = Vec::new();
    for _ in pages {
        page_ids.push(Ref::new(next));
        content_ids.push(Ref::new(next + 1));
        next += 2;
    }

    pdf.catalog(catalog_id).pages(pages_id);
    pdf.document_info(info_id)
        .title(TextStr(info.title))
        .author(TextStr(info.author))
        .producer(TextStr("MochaEduCardReporter"));
    pdf.pages(pages_id)
        .kids(page_ids.iter().copied())
        .count(pages.len() as i32);

    let all_faces: BTreeSet<u8> = pages.iter().flat_map(|p| p.faces.iter().copied()).collect();
    for &i in &all_faces {
        let face = face_from_index(i);
        let mut font = pdf.type1_font(font_ids[usize::from(i)]);
        font.base_font(Name(face.base_font().as_bytes()));
        if matches!(face, Face::Helvetica | Face::HelveticaBold) {
            font.encoding_predefined(Name(b"WinAnsiEncoding"));
        }
    }

    for (idx, page) in pages.iter().enumerate() {
        let mut p = pdf.page(page_ids[idx]);
        p.media_box(Rect::new(0.0, 0.0, page_w as f32, page_h as f32))
            .parent(pages_id)
            .contents(content_ids[idx]);
        let mut res = p.resources();
        let mut fonts = res.fonts();
        for &i in &page.faces {
            let face = face_from_index(i);
            fonts.pair(
                Name(face.resource_name().as_bytes()),
                font_ids[usize::from(i)],
            );
        }
        drop(fonts);
        drop(res);
        drop(p);
        pdf.stream(content_ids[idx], page.buf.as_bytes());
    }
    pdf.finish()
}
