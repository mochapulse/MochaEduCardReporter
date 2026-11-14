//! ReportLab `SimpleDocTemplate` + `Frame` + `handle_flowable`: places a story of
//! flowables into the page frame, splitting tables / paragraphs and starting new
//! pages exactly like `doctemplate.py` / `frames.py`.

use std::collections::VecDeque;

use super::paragraph::{Paragraph, FUZZ};
use super::table::Table;
use super::writer::Content;
use super::PdfError;

/// `reportlab.lib.units.mm` (`cm = inch / 2.54`, `mm = cm * 0.1`, `inch = 72.0`).
pub fn mm() -> f64 {
    (72.0 / 2.54) * 0.1
}

/// `reportlab.lib.pagesizes.A4` = `(210 * mm, 297 * mm)`.
pub fn a4() -> (f64, f64) {
    (210.0 * mm(), 297.0 * mm())
}

/// A flowable of the story.
#[derive(Debug, Clone)]
pub enum Flowable {
    /// `Paragraph`
    Para(Paragraph),
    /// `Spacer(1, height)`
    Spacer(f64),
    /// `Table` (boxed: it is much larger than the other variants)
    Table(Box<Table>),
}

impl Flowable {
    fn space_before(&self) -> f64 {
        match self {
            Flowable::Para(p) => p.style().space_before,
            _ => 0.0,
        }
    }

    fn space_after(&self) -> f64 {
        match self {
            Flowable::Para(p) => p.style().space_after,
            _ => 0.0,
        }
    }

    fn wrap(&mut self, aw: f64, ah: f64) -> (f64, f64) {
        match self {
            Flowable::Para(p) => p.wrap(aw, ah),
            Flowable::Spacer(h) => (1.0, *h),
            Flowable::Table(t) => t.wrap(aw, ah),
        }
    }

    fn split(&mut self, aw: f64, ah: f64) -> Vec<Flowable> {
        match self {
            Flowable::Para(p) => p.split(aw, ah).into_iter().map(Flowable::Para).collect(),
            Flowable::Spacer(_) => Vec::new(),
            Flowable::Table(t) => t
                .split(aw, ah)
                .into_iter()
                .map(|t| Flowable::Table(Box::new(t)))
                .collect(),
        }
    }

    /// `Flowable.drawOn(canvas, x, y, _sW)`.
    fn draw_on(&mut self, out: &mut Content, x: f64, y: f64, spare_width: f64) {
        match self {
            Flowable::Para(p) => {
                out.save_state();
                out.translate(x, y);
                p.draw(out);
                out.restore_state();
            }
            Flowable::Spacer(_) => {}
            Flowable::Table(t) => {
                // Table.hAlign == 'CENTER'
                let x = x + 0.5 * spare_width;
                out.save_state();
                out.translate(x, y);
                t.draw(out);
                out.restore_state();
            }
        }
    }
}

/// Page geometry (`SimpleDocTemplate` arguments).
#[derive(Debug, Clone, Copy)]
pub struct PageSetup {
    /// Page width.
    pub width: f64,
    /// Page height.
    pub height: f64,
    /// `leftMargin`
    pub left: f64,
    /// `rightMargin`
    pub right: f64,
    /// `topMargin`
    pub top: f64,
    /// `bottomMargin`
    pub bottom: f64,
}

/// `frames.Frame` with its default 6 pt padding on every side.
struct Frame {
    x: f64,
    y: f64,
    y1p: f64,
    aw: f64,
    top: f64,
    at_top: bool,
    prev_a_space: f64,
}

impl Frame {
    fn new(setup: &PageSetup) -> Frame {
        let (x1, y1) = (setup.left, setup.bottom);
        let width = setup.width - setup.left - setup.right;
        let height = setup.height - setup.top - setup.bottom;
        let pad = 6.0;
        let x2 = x1 + width;
        let y2 = y1 + height;
        Frame {
            x: x1 + pad,
            y: y2 - pad,
            y1p: y1 + pad,
            aw: x2 - x1 - pad - pad,
            top: y2 - pad,
            at_top: true,
            prev_a_space: 0.0,
        }
    }

    /// `Frame._add` (overlapAttachedSpace on).
    fn add(&mut self, f: &mut Flowable, out: &mut Content) -> bool {
        let mut y = self.y;
        let p = self.y1p;
        let mut s = 0.0;
        if !self.at_top {
            s = f.space_before();
            s = (s - self.prev_a_space).max(0.0);
        }
        let h = y - p - s;
        let (w, mut h) = if h > 0.0 {
            f.wrap(self.aw, h)
        } else {
            return false;
        };
        h += s;
        y -= h;
        if y < p - FUZZ {
            return false;
        }
        let sa = f.space_after();
        f.draw_on(out, self.x, y, self.aw - w);
        y -= sa;
        self.prev_a_space = sa;
        if y != self.y {
            self.at_top = false;
        }
        self.y = y;
        true
    }

    /// `Frame.split`.
    fn split(&mut self, f: &mut Flowable) -> Vec<Flowable> {
        let mut s = 0.0;
        if !self.at_top {
            s = f.space_before();
            s = (s - self.prev_a_space).max(0.0);
        }
        let h = self.y - self.y1p - s;
        if h <= 0.0 {
            return Vec::new();
        }
        f.split(self.aw, h)
    }

    #[allow(dead_code)]
    fn top(&self) -> f64 {
        self.top
    }
}

/// Run the story through the template (`BaseDocTemplate.build` +
/// `handle_flowable`), returning one content stream per page.
pub fn build(setup: &PageSetup, story: Vec<Flowable>) -> Result<Vec<Content>, PdfError> {
    let mut queue: VecDeque<(Flowable, bool)> = story.into_iter().map(|f| (f, false)).collect();
    let mut pages: Vec<Content> = Vec::new();
    let mut canv = Content::new();
    let mut frame = Frame::new(setup);
    while let Some((mut f, postponed)) = queue.pop_front() {
        if frame.add(&mut f, &mut canv) {
            continue;
        }
        let mut parts = frame.split(&mut f);
        if parts.is_empty() {
            if postponed {
                return Err(PdfError::Layout("flowable too large for the frame".into()));
            }
            queue.push_front((f, true));
            pages.push(std::mem::take(&mut canv));
            frame = Frame::new(setup);
            continue;
        }
        let mut first = parts.remove(0);
        if !frame.add(&mut first, &mut canv) {
            return Err(PdfError::Layout("splitting error".into()));
        }
        for rest in parts.into_iter().rev() {
            queue.push_front((rest, false));
        }
    }
    pages.push(canv);
    Ok(pages)
}
