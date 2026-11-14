//! ReportLab `platypus.Paragraph` for the subset this report uses: a single font,
//! `<br/>` line breaks, left/centre alignment, greedy wrapping with ReportLab's
//! 5 % space shrinkage, long-word splitting, and `Paragraph.split` for page breaks.
//!
//! Mirrors `reportlab/platypus/paragraph.py` (`cleanBlockQuotedText`, `_getFragWords`,
//! `Paragraph.breakLines` (multi-fragment branch), `wrap`, `drawPara`, `split`) and
//! `paraparser.ParaParser` (entities, `<br/>`, ignored unknown tags).

use std::collections::VecDeque;

use super::fonts::{string_width, Face};
use super::writer::{Color, Content};
use super::PdfError;

/// `rl_config.spaceShrinkage`.
const SPACE_SHRINKAGE: f64 = 0.05;
/// `rl_config._FUZZ`.
pub const FUZZ: f64 = 1e-6;

/// Paragraph alignment (`TA_LEFT`, `TA_CENTER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// `TA_LEFT`
    Left,
    /// `TA_CENTER`
    Center,
}

/// The `ParagraphStyle` attributes that matter here, with the values ReportLab
/// resolves after parent inheritance (`getSampleStyleSheet()["Normal"]` defaults:
/// Helvetica 10 / leading 12, black, no space before/after).
#[derive(Debug, Clone, PartialEq)]
pub struct ParaStyle {
    /// `fontName` (Helvetica or Helvetica-Bold).
    pub font: Face,
    /// `fontSize`.
    pub font_size: f64,
    /// `leading`.
    pub leading: f64,
    /// `textColor`.
    pub text_color: Color,
    /// `alignment`.
    pub alignment: Align,
    /// `spaceBefore`.
    pub space_before: f64,
    /// `spaceAfter`.
    pub space_after: f64,
}

impl ParaStyle {
    /// `getSampleStyleSheet()["Normal"]`.
    pub fn normal() -> Self {
        ParaStyle {
            font: Face::Helvetica,
            font_size: 10.0,
            leading: 12.0,
            text_color: Color::BLACK,
            alignment: Align::Left,
            space_before: 0.0,
            space_after: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Seg {
    Text(String),
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitKind {
    /// Ordinary word.
    None,
    /// `_SplitFrag` (non-final piece of a split long word).
    Piece,
    /// `_SplitFragH` (final piece).
    Head,
}

#[derive(Debug, Clone)]
enum Item {
    Word {
        text: String,
        width: f64,
        hs: bool,
        split: SplitKind,
    },
    Break,
}

/// One wrapped line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    text: String,
    extra_space: f64,
    word_count: usize,
}

/// A wrapped (or wrappable) paragraph.
#[derive(Debug, Clone)]
pub struct Paragraph {
    style: ParaStyle,
    segs: Vec<Seg>,
    lines: Vec<Line>,
    wrapped: bool,
    fixed: bool,
    width: f64,
    height: f64,
}

/// Python `str.isspace`-compatible whitespace (`str.split()` with no argument).
fn is_py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `paragraph._wsc` (no-break space deliberately excluded).
fn is_wsc(c: char) -> bool {
    matches!(c,
        '\u{9}'..='\u{d}' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{1680}'
        | '\u{2000}'..='\u{200b}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

/// `paragraph.split(text)` with no delimiter.
fn split_words(text: &str) -> Vec<&str> {
    if text.contains('\u{a0}') {
        text.split(is_wsc).filter(|s| !s.is_empty()).collect()
    } else {
        text.split(is_py_space).filter(|s| !s.is_empty()).collect()
    }
}

/// `cleanBlockQuotedText(text)`: each `\n`-separated line is stripped, internal
/// whitespace runs collapse to one space, empty lines are dropped, rest joined by ' '.
fn clean_block_quoted_text(text: &str) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|l| split_words(l.trim_matches(is_wsc)).join(" "))
        .filter(|l| !l.is_empty())
        .collect();
    lines.join(" ")
}

fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '\u{a9}',
        "reg" => '\u{ae}',
        "deg" => '\u{b0}',
        "eacute" => '\u{e9}',
        "egrave" => '\u{e8}',
        "agrave" => '\u{e0}',
        "ccedil" => '\u{e7}',
        "Eacute" => '\u{c9}',
        _ => return None,
    })
}

/// Tags `ParaParser` knows (they change formatting, which this port does not support).
const FORMATTING_TAGS: &[&str] = &[
    "a",
    "b",
    "bullet",
    "em",
    "font",
    "greek",
    "i",
    "img",
    "index",
    "link",
    "nobr",
    "ondraw",
    "seq",
    "seqchain",
    "seqdefault",
    "seqformat",
    "seqreset",
    "span",
    "strike",
    "strong",
    "sub",
    "super",
    "u",
    "unichar",
];

/// Port of the relevant part of `ParaParser.parse` (HTMLParser based).
fn parse_markup(text: &str) -> Result<Vec<Seg>, PdfError> {
    let chars: Vec<char> = text.chars().collect();
    let mut segs: Vec<Seg> = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    let flush = |segs: &mut Vec<Seg>, cur: &mut String| {
        if !cur.is_empty() {
            segs.push(Seg::Text(std::mem::take(cur)));
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '<'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '/')
        {
            let Some(end) = chars[i..].iter().position(|&x| x == '>') else {
                return Err(PdfError::Markup(format!("unterminated tag in {text:?}")));
            };
            let inner: String = chars[i + 1..i + end].iter().collect();
            i += end + 1;
            let closing = inner.starts_with('/');
            let body = inner.trim_start_matches('/').trim_end_matches('/');
            let name: String = body
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if name == "br" {
                if !closing {
                    flush(&mut segs, &mut cur);
                    segs.push(Seg::Break);
                }
            } else if FORMATTING_TAGS.contains(&name.as_str()) {
                return Err(PdfError::Markup(format!(
                    "unsupported markup <{name}> in {text:?}"
                )));
            }
            continue;
        }
        if c == '&' {
            let rest: String = chars[i + 1..].iter().take(12).collect();
            if let Some(num) = rest.strip_prefix('#') {
                let digits: String = num
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                let parsed = if let Some(hex) = digits.strip_prefix('x') {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    digits.parse::<u32>().ok()
                };
                if let Some(ch) = parsed.and_then(char::from_u32) {
                    cur.push(ch);
                    let consumed = 1
                        + digits.chars().count()
                        + usize::from(num[digits.len()..].starts_with(';'));
                    i += 1 + consumed;
                    continue;
                }
            } else {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if !name.is_empty() && rest[name.len()..].starts_with(';') {
                    if let Some(ch) = named_entity(&name) {
                        cur.push(ch);
                    } else {
                        cur.push('&');
                        cur.push_str(&name);
                        cur.push(';');
                    }
                    i += 1 + name.chars().count() + 1;
                    continue;
                }
            }
        }
        cur.push(c);
        i += 1;
    }
    flush(&mut segs, &mut cur);
    Ok(segs)
}

/// `_getFragWords` for one font: words split on whitespace; every word is followed
/// by a space except the last of a text segment; `<br/>` becomes a break item.
fn frag_words(segs: &[Seg], style: &ParaStyle) -> Vec<Item> {
    let mut items = Vec::new();
    let mut hanging_strip = true;
    for seg in segs {
        match seg {
            Seg::Break => {
                items.push(Item::Break);
                hanging_strip = true;
            }
            Seg::Text(t) => {
                let mut text: &str = t;
                if hanging_strip {
                    text = text.trim_start_matches(is_wsc);
                    if text.is_empty() {
                        continue;
                    }
                    hanging_strip = false;
                }
                let words = split_words(text);
                let ends_ws = text
                    .chars()
                    .last()
                    .is_some_and(|c| " \t\n\r\x0b\x0c".contains(c));
                let n = words.len();
                for (k, w) in words.iter().enumerate() {
                    let hs = k + 1 < n || ends_ws;
                    items.push(Item::Word {
                        text: (*w).to_string(),
                        width: string_width(w, style.font, style.font_size),
                        hs,
                        split: SplitKind::None,
                    });
                }
            }
        }
    }
    items
}

/// `_splitFragWord`: split an over-long word into pieces.
fn split_long_word(
    text: &str,
    hs: bool,
    style: &ParaStyle,
    max_width: f64,
    max_widths: [f64; 2],
    lineno: usize,
) -> Vec<Item> {
    let maxlineno = 1;
    let mw = |n: usize| max_widths[n.min(maxlineno)];
    let mut lineno = lineno;
    let mut max_width = max_width;
    let mut max_width_next = mw(lineno + 1);
    let mut out = Vec::new();
    let mut line_width = 0.0;
    let mut word_width = 0.0;
    let mut piece = String::new();
    for ch in text.chars() {
        let mut buf = [0u8; 4];
        let cw = string_width(ch.encode_utf8(&mut buf), style.font, style.font_size);
        let mut new_line_width = line_width + cw;
        let too_long = new_line_width > max_width && (word_width != 0.0 || cw <= max_width_next);
        if too_long {
            out.push(Item::Word {
                text: std::mem::take(&mut piece),
                width: word_width,
                hs: false,
                split: SplitKind::Piece,
            });
            lineno += 1;
            max_width = max_width_next;
            max_width_next = mw(lineno + 1);
            new_line_width = cw;
            word_width = 0.0;
        }
        word_width += cw;
        piece.push(ch);
        line_width = new_line_width;
    }
    out.push(Item::Word {
        text: piece,
        width: word_width,
        hs,
        split: SplitKind::Head,
    });
    out
}

impl Paragraph {
    /// `Paragraph(text, style)`: cleans and parses the markup.
    pub fn new(text: &str, style: &ParaStyle) -> Result<Paragraph, PdfError> {
        let cleaned = clean_block_quoted_text(text);
        let segs = parse_markup(&cleaned)?;
        Ok(Paragraph {
            style: style.clone(),
            segs,
            lines: Vec::new(),
            wrapped: false,
            fixed: false,
            width: 0.0,
            height: 0.0,
        })
    }

    /// The style.
    pub fn style(&self) -> &ParaStyle {
        &self.style
    }

    /// Lines of text after wrapping (joined words per line).
    pub fn line_texts(&self) -> Vec<&str> {
        self.lines.iter().map(|l| l.text.as_str()).collect()
    }

    /// `Paragraph.breakLines` (multi-fragment branch, single font).
    fn break_lines(&self, widths: [f64; 2]) -> Vec<Line> {
        let st = &self.style;
        let sw = string_width(" ", st.font, st.font_size);
        let mut items: VecDeque<Item> = frag_words(&self.segs, st).into();
        let mut lines: Vec<Line> = Vec::new();
        let maxlineno = 1usize;
        let mw = |n: usize| widths[n.min(maxlineno)];
        let mut lineno = 0usize;
        let mut max_width = mw(0);
        let mut has_words = false;
        let mut line_text = String::new();
        let (mut n, mut space_width, mut current_width) = (0usize, 0.0f64, 0.0f64);
        let mut spaces = 0usize;
        while let Some(w) = items.pop_front() {
            if !has_words {
                n = 0;
                space_width = 0.0;
                current_width = 0.0;
                spaces = 0;
            }
            let (word_width, is_break) = match &w {
                Item::Word { width, .. } => (*width, false),
                Item::Break => (0.0, true),
            };
            let new_width = if word_width > 0.0 {
                current_width + space_width + word_width
            } else {
                current_width
            };
            let spacing = (space_width + (spaces as f64) * sw) * SPACE_SHRINKAGE;
            let lim_width = max_width + spacing;
            if !is_break && new_width > lim_width {
                if let Item::Word {
                    text, hs, split, ..
                } = &w
                {
                    if *split == SplitKind::None && word_width > mw(lineno.min(maxlineno)) {
                        let pieces = split_long_word(
                            text,
                            *hs,
                            st,
                            max_width - space_width - current_width,
                            widths,
                            lineno,
                        );
                        for (k, p) in pieces.into_iter().enumerate() {
                            items.insert(k, p);
                        }
                        items.insert(1, Item::Break);
                        continue;
                    }
                }
            }
            let end_line = (new_width > lim_width && n > 0) || is_break;
            if !end_line {
                if let Item::Word { text, hs, .. } = &w {
                    if !text.is_empty() {
                        n += 1;
                    }
                    if !has_words {
                        line_text = text.clone();
                        has_words = true;
                    } else if space_width != 0.0 {
                        line_text.push(' ');
                        line_text.push_str(text);
                        spaces += 1;
                    } else {
                        line_text.push_str(text);
                    }
                    space_width = if *hs { sw } else { 0.0 };
                    current_width = new_width;
                }
            } else {
                lines.push(Line {
                    text: std::mem::take(&mut line_text),
                    extra_space: max_width - current_width,
                    word_count: n,
                });
                lineno += 1;
                max_width = mw(lineno);
                if is_break {
                    has_words = false;
                    continue;
                }
                if let Item::Word {
                    text, hs, width, ..
                } = &w
                {
                    space_width = if *hs { sw } else { 0.0 };
                    current_width = *width;
                    n = 1;
                    line_text = text.clone();
                    has_words = true;
                    spaces = 0;
                }
            }
        }
        if has_words {
            lines.push(Line {
                text: line_text,
                extra_space: max_width - current_width,
                word_count: n,
            });
        }
        lines
    }

    /// `Paragraph.wrap(availWidth, availHeight)`; returns `(width, height)`.
    pub fn wrap(&mut self, avail_width: f64, _avail_height: f64) -> (f64, f64) {
        if avail_width < FUZZ {
            return (0.0, f64::from(0x7fff_ffffu32));
        }
        self.width = avail_width;
        if !(self.fixed && self.wrapped) {
            self.lines = self.break_lines([avail_width, avail_width]);
        }
        self.wrapped = true;
        self.height = (self.lines.len() as f64) * self.style.leading;
        (self.width, self.height)
    }

    /// `Paragraph.split(availWidth, availHeight)` (no widows control, orphans not
    /// allowed: ReportLab defaults). Returns an empty vector when it cannot split.
    pub fn split(&mut self, avail_width: f64, avail_height: f64) -> Vec<Paragraph> {
        if self.segs.is_empty() && !self.fixed || avail_width < FUZZ || avail_height < FUZZ {
            return Vec::new();
        }
        if !self.wrapped {
            self.wrap(avail_width, avail_height);
        }
        let leading = self.style.leading;
        let s = (avail_height / leading) as usize;
        if s <= 1 {
            self.wrapped = false;
            return Vec::new();
        }
        let n = self.lines.len();
        if n <= s {
            return vec![self.clone()];
        }
        let mk = |lines: Vec<Line>, h: f64| Paragraph {
            style: self.style.clone(),
            segs: self.segs.clone(),
            height: h,
            width: avail_width,
            lines,
            wrapped: true,
            fixed: true,
        };
        let p1 = mk(self.lines[..s].to_vec(), s as f64 * leading);
        let p2 = mk(self.lines[s..].to_vec(), (n - s) as f64 * leading);
        vec![p1, p2]
    }

    /// `Paragraph.drawPara` with the canvas translated to the paragraph origin
    /// (bottom-left corner).
    pub fn draw(&self, out: &mut Content) {
        if self.lines.is_empty() {
            return;
        }
        let st = &self.style;
        out.save_state();
        out.set_fill(st.text_color);
        let cur_y = self.height - st.font_size;
        for (i, line) in self.lines.iter().enumerate() {
            let nbsp = line.text.chars().filter(|&c| c == '\u{a0}').count();
            let simple = line.extra_space > -1e-8;
            let n_spaces = (line.word_count + nbsp) as i64 - 1;
            let (x, ws) = if simple || n_spaces <= 0 {
                match st.alignment {
                    Align::Left => (0.0, 0.0),
                    Align::Center => (0.5 * line.extra_space, 0.0),
                }
            } else {
                (0.0, line.extra_space / n_spaces as f64)
            };
            let y = cur_y - (i as f64) * st.leading;
            out.show_text(x, y, st.font, st.font_size, ws, &line.text);
        }
        out.restore_state();
    }
}
