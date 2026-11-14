//! ReportLab `platypus.Table` for the features this report uses: fixed column widths,
//! row heights from cell content, `SPAN`, `BACKGROUND`, `GRID`/`LINEABOVE` (and the
//! other line operators), per-cell styles, `repeatRows` and row-wise splitting.
//!
//! Mirrors `reportlab/platypus/tables.py`: `_calc_height`, `spanFixDim`,
//! `_calcSpanRanges`, `_calcSpanRects`, `_drawBkgrnd`, `_drawCell`, `_drawLines`,
//! `_hLine`, `_splitLineCmds`, `_cr_0`, `_cr_1_1`, `_getFirstPossibleSplitRowPosition`,
//! `_splitRows` (`splitInRow=0`), `split`, `wrap`.

use std::collections::{BTreeMap, HashMap};

use super::fonts::{string_width, Face};
use super::paragraph::{Paragraph, FUZZ};
use super::writer::{Color, Content};

/// Horizontal alignment of a cell (`ALIGN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    /// `LEFT`
    Left,
    /// `CENTER`
    Center,
    /// `RIGHT`
    Right,
}

/// Vertical alignment of a cell (`VALIGN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    /// `TOP`
    Top,
    /// `MIDDLE`
    Middle,
    /// `BOTTOM`
    Bottom,
}

/// Cell content: a plain string (drawn with the cell style) or a `Paragraph` flowable.
#[derive(Debug, Clone)]
pub enum Cell {
    /// A string cell.
    Text(String),
    /// A flowable cell.
    Para(Paragraph),
}

/// `tables.CellStyle` (defaults from the class attributes).
#[derive(Debug, Clone)]
pub struct CellStyle {
    /// `alignment`
    pub alignment: HAlign,
    /// `valign`
    pub valign: VAlign,
    /// `leftPadding`
    pub left_padding: f64,
    /// `rightPadding`
    pub right_padding: f64,
    /// `topPadding`
    pub top_padding: f64,
    /// `bottomPadding`
    pub bottom_padding: f64,
    /// `fontname`
    pub font: Face,
    /// `fontsize`
    pub font_size: f64,
    /// `leading`
    pub leading: f64,
    /// `color`
    pub color: Color,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle {
            alignment: HAlign::Left,
            valign: VAlign::Bottom,
            left_padding: 6.0,
            right_padding: 6.0,
            top_padding: 3.0,
            bottom_padding: 3.0,
            font: Face::Helvetica,
            font_size: 10.0,
            leading: 12.0,
            color: Color::BLACK,
        }
    }
}

/// A `(col, row)` coordinate; negative values count from the end.
pub type Coord = (i32, i32);
/// A `(start, stop)` cell range.
pub type Range = (Coord, Coord);

/// Line operators (`LINECOMMANDS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOp {
    /// `GRID`
    Grid,
    /// `BOX` / `OUTLINE`
    Box,
    /// `INNERGRID`
    InnerGrid,
    /// `LINEBELOW`
    LineBelow,
    /// `LINEABOVE`
    LineAbove,
    /// `LINEBEFORE`
    LineBefore,
    /// `LINEAFTER`
    LineAfter,
}

/// A `TableStyle` command.
#[derive(Debug, Clone)]
pub enum Cmd {
    /// `BACKGROUND`
    Background(Range, Color),
    /// `SPAN`
    Span(Range),
    /// A line command with weight and colour (cap 1, join 1, count 1 as defaults).
    Line(LineOp, Range, f64, Color),
    /// `ALIGN`
    Align(Range, HAlign),
    /// `VALIGN`
    VAlign(Range, VAlign),
    /// `FONTNAME`
    FontName(Range, Face),
    /// `FONTSIZE`
    FontSize(Range, f64),
    /// `TEXTCOLOR`
    TextColor(Range, Color),
    /// `LEFTPADDING`
    LeftPadding(Range, f64),
    /// `RIGHTPADDING`
    RightPadding(Range, f64),
    /// `TOPPADDING`
    TopPadding(Range, f64),
    /// `BOTTOMPADDING`
    BottomPadding(Range, f64),
}

#[derive(Debug, Clone)]
struct BgCmd {
    range: Range,
    color: Color,
}

#[derive(Debug, Clone)]
struct LineCmd {
    op: LineOp,
    range: Range,
    weight: f64,
    color: Color,
}

type SpanRange = (usize, usize, usize, usize);

/// `(col, row) -> (x, y, w, h)` of each span rectangle; `None` for occluded cells.
type SpanRects = HashMap<(usize, usize), Option<(f64, f64, f64, f64)>>;

/// Line blocks: `(coordinate, extents)` pairs, one entry per coordinate.
type Blocks = Vec<(f64, Vec<(f64, f64)>)>;

#[derive(Debug, Clone, Default)]
struct Calc {
    row_heights: Vec<f64>,
    row_positions: Vec<f64>,
    col_positions: Vec<f64>,
    width: f64,
    height: f64,
    span_ranges: HashMap<(usize, usize), Option<SpanRange>>,
    col_span_cells: Vec<(usize, usize)>,
    row_span_cells: Vec<(usize, usize)>,
    /// `(col, row) -> (x, y, w, h)`; `None` for occluded cells.
    span_rects: SpanRects,
    h_blocks: Blocks,
    v_blocks: Blocks,
}

/// A table flowable.
#[derive(Debug, Clone)]
pub struct Table {
    cells: Vec<Vec<Cell>>,
    styles: Vec<Vec<CellStyle>>,
    col_widths: Vec<f64>,
    arg_heights: Vec<Option<f64>>,
    repeat_rows: usize,
    bkgrnd: Vec<BgCmd>,
    spans: Vec<Range>,
    lines: Vec<LineCmd>,
    calc: Calc,
}

fn norm_idx(v: i32, n: usize) -> i64 {
    if v < 0 {
        i64::from(v) + n as i64
    } else {
        i64::from(v)
    }
}

impl Table {
    /// `Table(data, colWidths=..., repeatRows=...)` (row heights computed).
    pub fn new(cells: Vec<Vec<Cell>>, col_widths: Vec<f64>, repeat_rows: usize) -> Table {
        let nrows = cells.len();
        let ncols = col_widths.len();
        Table {
            styles: vec![vec![CellStyle::default(); ncols]; nrows],
            arg_heights: vec![None; nrows],
            cells,
            col_widths,
            repeat_rows,
            bkgrnd: Vec::new(),
            spans: Vec::new(),
            lines: Vec::new(),
            calc: Calc::default(),
        }
    }

    fn nrows(&self) -> usize {
        self.cells.len()
    }

    fn ncols(&self) -> usize {
        self.col_widths.len()
    }

    /// `Table.setStyle(TableStyle(cmds))` (`_addCommand` for each command, in order).
    pub fn set_style(&mut self, cmds: &[Cmd]) {
        for c in cmds {
            match c {
                Cmd::Background(r, col) => self.bkgrnd.push(BgCmd {
                    range: *r,
                    color: *col,
                }),
                Cmd::Span(r) => self.spans.push(*r),
                Cmd::Line(op, r, w, col) => self.lines.push(LineCmd {
                    op: *op,
                    range: *r,
                    weight: *w,
                    color: *col,
                }),
                Cmd::Align(r, v) => self.each_style(*r, |s| s.alignment = *v),
                Cmd::VAlign(r, v) => self.each_style(*r, |s| s.valign = *v),
                Cmd::FontName(r, v) => self.each_style(*r, |s| s.font = *v),
                Cmd::FontSize(r, v) => self.each_style(*r, |s| s.font_size = *v),
                Cmd::TextColor(r, v) => self.each_style(*r, |s| s.color = *v),
                Cmd::LeftPadding(r, v) => self.each_style(*r, |s| s.left_padding = *v),
                Cmd::RightPadding(r, v) => self.each_style(*r, |s| s.right_padding = *v),
                Cmd::TopPadding(r, v) => self.each_style(*r, |s| s.top_padding = *v),
                Cmd::BottomPadding(r, v) => self.each_style(*r, |s| s.bottom_padding = *v),
            }
        }
    }

    /// `normCellRange` followed by the cell loop of `_addCommand`.
    fn each_style(&mut self, r: Range, f: impl Fn(&mut CellStyle)) {
        let ((sc, sr), (ec, er)) = r;
        let (nr, nc) = (self.nrows(), self.ncols());
        let sc = norm_idx(sc, nc).max(0);
        let ec = norm_idx(ec, nc).min(nc as i64 - 1);
        let sr = norm_idx(sr, nr).max(0);
        let er = norm_idx(er, nr).min(nr as i64 - 1);
        for i in sr..=er {
            for j in sc..=ec {
                f(&mut self.styles[i as usize][j as usize]);
            }
        }
    }

    // ---------------------------------------------------------------- geometry

    /// `_calcSpanRanges`.
    fn calc_span_ranges(&mut self) {
        let (nc, nr) = (self.ncols(), self.nrows());
        let mut ranges: HashMap<(usize, usize), Option<SpanRange>> = HashMap::new();
        for x in 0..nc {
            for y in 0..nr {
                ranges.insert((x, y), Some((x, y, x, y)));
            }
        }
        let mut csa = Vec::new();
        let mut rsa = Vec::new();
        for &((x0, y0), (x1, y1)) in &self.spans {
            let mut x0 = norm_idx(x0, nc);
            let mut x1 = norm_idx(x1, nc);
            let mut y0 = norm_idx(y0, nr);
            let mut y1 = norm_idx(y1, nr);
            if x0 > x1 {
                std::mem::swap(&mut x0, &mut x1);
            }
            if y0 > y1 {
                std::mem::swap(&mut y0, &mut y1);
            }
            if x0 != x1 || y0 != y1 {
                if x0 != x1 {
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            csa.push((x as usize, y as usize));
                        }
                    }
                }
                if y0 != y1 {
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            rsa.push((x as usize, y as usize));
                        }
                    }
                }
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        ranges.insert((x as usize, y as usize), None);
                    }
                }
                ranges.insert(
                    (x0 as usize, y0 as usize),
                    Some((x0 as usize, y0 as usize, x1 as usize, y1 as usize)),
                );
            }
        }
        self.calc.span_ranges = ranges;
        self.calc.col_span_cells = csa;
        self.calc.row_span_cells = rsa;
    }

    /// `_calc_width` with all widths given.
    fn calc_width(&mut self) {
        let mut pos = vec![0.0];
        let mut width = 0.0;
        for w in &self.col_widths {
            width += w;
            pos.push(width);
        }
        self.calc.col_positions = pos;
        self.calc.width = width;
    }

    /// `_listCellGeom` for a single flowable: wraps and returns its height.
    fn para_geom(p: &mut Paragraph, w: f64, s: &CellStyle) -> f64 {
        let aw = w - s.left_padding - s.right_padding;
        let (_, h) = p.wrap(aw, 72000.0 - s.top_padding - s.bottom_padding);
        h
    }

    /// `spanFixDim`.
    fn span_fix_dim(v0: &[Option<f64>], v: &mut [f64], cons: &BTreeMap<(usize, usize), f64>) {
        let mut m: HashMap<usize, f64> = HashMap::new();
        let mut order: Vec<(f64, (usize, usize))> =
            cons.iter().map(|(k, val)| (*val, *k)).collect();
        order.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        order.reverse();
        let lim = v0.len();
        for (val, (x0, x1)) in order {
            if x0 >= lim {
                continue;
            }
            let x1 = x1 + 1;
            let t: f64 = (x0..x1)
                .map(|x| v[x] + m.get(&x).copied().unwrap_or(0.0))
                .sum();
            if t >= val - FUZZ {
                continue;
            }
            let xs: Vec<usize> = (x0..x1).filter(|&x| v0[x].is_none()).collect();
            if xs.is_empty() {
                continue;
            }
            let share = (val - t) / xs.len() as f64;
            for x in xs {
                *m.entry(x).or_insert(0.0) += share;
            }
        }
        for (x, add) in m {
            v[x] += add;
        }
    }

    /// `_calc_height` and the row-position computation.
    fn calc_height(&mut self) {
        let (nr, nc) = (self.nrows(), self.ncols());
        let mut h: Vec<Option<f64>> = self.arg_heights.clone();
        if h.iter().any(Option::is_none) {
            let col_pos = self.calc.col_positions.clone();
            let mut span_cons: BTreeMap<(usize, usize), f64> = BTreeMap::new();
            for i in 0..nr {
                if h[i].is_some() {
                    continue;
                }
                let mut hh = 0.0f64;
                for j in 0..nc {
                    let ji = (j, i);
                    let span = self.calc.span_ranges.get(&ji).copied().flatten();
                    if self.calc.row_span_cells.contains(&ji) && span.is_none() {
                        continue;
                    }
                    let style = self.styles[i][j].clone();
                    let mut t;
                    match &mut self.cells[i][j] {
                        Cell::Para(p) => {
                            let mut w = self.col_widths[j];
                            if self.calc.col_span_cells.contains(&ji) {
                                match span {
                                    None => continue,
                                    Some(sp) => w = (col_pos[sp.2 + 1] - col_pos[sp.0]).max(w),
                                }
                            }
                            t = Self::para_geom(p, w, &style);
                        }
                        Cell::Text(s) => {
                            let n = s.split('\n').count();
                            t = (if style.leading != 0.0 {
                                style.leading
                            } else {
                                1.2 * style.font_size
                            }) * n as f64;
                        }
                    }
                    t += style.bottom_padding + style.top_padding;
                    if let Some(sp) = span {
                        if sp.1 != sp.3 {
                            let e = span_cons.entry((sp.1, sp.3)).or_insert(t);
                            if t > *e {
                                *e = t;
                            }
                            t = 0.0;
                        }
                    }
                    if t > hh {
                        hh = t;
                    }
                }
                h[i] = Some(hh);
            }
            let mut hv: Vec<f64> = h.iter().map(|x| x.unwrap_or(0.0)).collect();
            if !span_cons.is_empty() {
                Self::span_fix_dim(&self.arg_heights, &mut hv, &span_cons);
            }
            self.calc.row_heights = hv;
        } else {
            self.calc.row_heights = h.iter().map(|x| x.unwrap_or(0.0)).collect();
        }
        // Row positions with ReportLab's compensated summation.
        let mut pos = Vec::new();
        let (mut height, mut c) = (0.0f64, 0.0f64);
        for i in (0..nr).rev() {
            pos.push(height);
            let y = self.calc.row_heights[i] - c;
            let t = height + y;
            c = (t - height) - y;
            height = t;
        }
        pos.push(height);
        pos.reverse();
        self.calc.height = height;
        self.calc.row_positions = pos;
    }

    /// `_calcSpanRects`.
    fn calc_span_rects(&mut self) {
        let cp = self.calc.col_positions.clone();
        let rp = self.calc.row_positions.clone();
        let mut rects = HashMap::new();
        let mut v_blocks: Blocks = Vec::new();
        let mut h_blocks: Blocks = Vec::new();
        fn add(blocks: &mut Blocks, key: f64, val: (f64, f64)) {
            if let Some(e) = blocks.iter_mut().find(|e| e.0 == key) {
                e.1.push(val);
            } else {
                blocks.push((key, vec![val]));
            }
        }
        let rlim = rp.len() - 1;
        let mut keys: Vec<_> = self.calc.span_ranges.keys().copied().collect();
        keys.sort();
        for coord in keys {
            match self.calc.span_ranges[&coord] {
                None => {
                    rects.insert(coord, None);
                }
                Some((col0, row0, col1, row1)) => {
                    if row1 >= rlim {
                        continue;
                    }
                    if col1 > col0 {
                        for &x in &cp[col0 + 1..=col1] {
                            add(&mut v_blocks, x, (rp[row1 + 1], rp[row0]));
                        }
                    }
                    if row1 > row0 {
                        for &y in &rp[row0 + 1..=row1] {
                            add(&mut h_blocks, y, (cp[col0], cp[col1 + 1]));
                        }
                    }
                    let x = cp[col0];
                    let y = rp[row1 + 1];
                    rects.insert(coord, Some((x, y, cp[col1 + 1] - x, rp[row0] - y)));
                }
            }
        }
        for b in h_blocks.iter_mut().chain(v_blocks.iter_mut()) {
            b.1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        self.calc.span_rects = rects;
        self.calc.h_blocks = h_blocks;
        self.calc.v_blocks = v_blocks;
    }

    /// `_calc`.
    fn do_calc(&mut self) {
        self.calc.span_ranges.clear();
        self.calc.col_span_cells.clear();
        self.calc.row_span_cells.clear();
        self.calc.span_rects.clear();
        self.calc.h_blocks.clear();
        self.calc.v_blocks.clear();
        self.calc_span_ranges();
        self.calc_width();
        self.calc_height();
        if !self.spans.is_empty() {
            self.calc_span_rects();
        }
    }

    /// `Table.wrap`; returns `(width, height)`.
    pub fn wrap(&mut self, _avail_width: f64, _avail_height: f64) -> (f64, f64) {
        self.do_calc();
        (self.calc.width, self.calc.height)
    }

    /// Row heights computed by the last `wrap`/`split`.
    pub fn row_heights(&self) -> &[f64] {
        &self.calc.row_heights
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.nrows()
    }

    // ---------------------------------------------------------------- splitting

    /// `_getFirstPossibleSplitRowPosition(availHeight)`.
    fn first_possible_split(&self, avail_height: f64) -> usize {
        let mut impossible: Vec<usize> = Vec::new();
        if !self.spans.is_empty() {
            for xy in &self.calc.row_span_cells {
                if let Some(Some(r)) = self.calc.span_ranges.get(xy) {
                    let (y1, y2) = (r.1, r.3);
                    if y1 != y2 {
                        let (lo, hi) = (y1.min(y2), y1.max(y2));
                        for y in lo + 1..=hi {
                            impossible.push(y);
                        }
                    }
                }
            }
        }
        let mut h = 0.0;
        let mut split_at = 0usize;
        for (i, rh) in self.calc.row_heights.iter().enumerate() {
            if h + rh > avail_height {
                break;
            }
            let n = i + 1;
            if !impossible.contains(&n) {
                split_at = n;
            }
            h += rh;
        }
        split_at
    }

    /// `_splitLineCmds(n)`.
    fn split_line_cmds(&self, n: i64) -> Vec<LineCmd> {
        let (nr, nc) = (self.nrows(), self.ncols());
        let mut out = Vec::new();
        for l in &self.lines {
            let ((sc, sr), (ec, er)) = l.range;
            let sc = norm_idx(sc, nc) as i32;
            let ec = norm_idx(ec, nc) as i32;
            let sr = norm_idx(sr, nr);
            let er = norm_idx(er, nr);
            let mk = |op: LineOp, a: (i32, i64), b: (i32, i64)| LineCmd {
                op,
                range: ((a.0, a.1 as i32), (b.0, b.1 as i32)),
                weight: l.weight,
                color: l.color,
            };
            match l.op {
                LineOp::Grid | LineOp::Box => {
                    if sr < n && er >= n {
                        out.push(mk(LineOp::LineAbove, (sc, sr), (ec, sr)));
                        out.push(mk(LineOp::LineBefore, (sc, sr), (sc, er)));
                        out.push(mk(LineOp::LineAfter, (ec, sr), (ec, er)));
                        out.push(mk(LineOp::LineBelow, (sc, er), (ec, er)));
                        if l.op == LineOp::Grid {
                            out.push(mk(LineOp::LineBelow, (sc, n - 1), (ec, n - 1)));
                            out.push(mk(LineOp::LineAbove, (sc, n), (ec, n)));
                            out.push(mk(LineOp::InnerGrid, (sc, sr), (ec, er)));
                        }
                    } else {
                        out.push(mk(l.op, (sc, sr), (ec, er)));
                    }
                }
                LineOp::InnerGrid => {
                    if sr < n && er >= n {
                        out.push(mk(LineOp::LineBelow, (sc, n - 1), (ec, n - 1)));
                        out.push(mk(LineOp::LineAbove, (sc, n), (ec, n)));
                    }
                    out.push(mk(l.op, (sc, sr), (ec, er)));
                }
                LineOp::LineBelow => {
                    if sr < n && er >= n - 1 {
                        out.push(mk(LineOp::LineAbove, (sc, n), (ec, n)));
                    }
                    out.push(mk(l.op, (sc, sr), (ec, er)));
                }
                LineOp::LineAbove => {
                    if sr <= n && er >= n {
                        out.push(mk(LineOp::LineBelow, (sc, n - 1), (ec, n - 1)));
                    }
                    out.push(mk(l.op, (sc, sr), (ec, er)));
                }
                _ => out.push(mk(l.op, (sc, sr), (ec, er))),
            }
        }
        out
    }

    /// `_cr_0` for one range: clip a command to the first `n` rows.
    fn cr0(range: Range, n: i64, nr0: usize) -> Option<Range> {
        let ((sc, sr), (ec, er)) = range;
        let mut sr = i64::from(sr);
        let mut er = i64::from(er);
        if sr < 0 {
            sr += nr0 as i64;
        }
        if sr >= n {
            return None;
        }
        if er >= n {
            er = n - 1;
        }
        Some(((sc, sr as i32), (ec, er as i32)))
    }

    /// `_cr_1_1` for one range: ranges for the second table (repeat rows kept).
    fn cr11(range: Range, n: i64, nrows: usize, repeat: &[usize]) -> Vec<Range> {
        let ((sc, sr), (ec, er)) = range;
        let mut out = Vec::new();
        let mut sr = i64::from(sr);
        let mut er = i64::from(er);
        if sr < 0 {
            sr += nrows as i64;
        }
        if er < 0 {
            er += nrows as i64;
        }
        let in_repeat: Vec<usize> = (sr.max(0)..=er)
            .map(|v| v as usize)
            .filter(|v| repeat.contains(v))
            .collect();
        if let (Some(lo), Some(hi)) = (in_repeat.iter().min(), in_repeat.iter().max()) {
            let a = repeat.iter().position(|v| v == lo).unwrap_or(0) as i32;
            let b = repeat.iter().position(|v| v == hi).unwrap_or(0) as i32;
            out.push(((sc, a), (ec, b)));
        }
        if er < n {
            return out;
        }
        let nrr = repeat.len() as i64;
        let sr2 = (sr - n).max(0) + nrr;
        let er2 = (er - n).max(0) + nrr;
        out.push(((sc, sr2 as i32), (ec, er2 as i32)));
        out
    }

    /// `Table.split(availWidth, availHeight)` for `splitByRow=1, splitInRow=0`.
    /// Returns an empty vector when no split is possible.
    pub fn split(&mut self, _avail_width: f64, avail_height: f64) -> Vec<Table> {
        self.do_calc();
        let n = self.first_possible_split(avail_height);
        let maxrepeat = self.repeat_rows;
        if n <= maxrepeat {
            return Vec::new();
        }
        let lim = self.calc.row_heights.len();
        if n == lim {
            return vec![self.clone()];
        }
        let nrows = self.nrows();
        let ni = n as i64;
        let heights: Vec<Option<f64>> = self.calc.row_heights.iter().map(|h| Some(*h)).collect();
        let repeat: Vec<usize> = (0..self.repeat_rows).collect();

        let line_cmds = self.split_line_cmds(ni);

        // R0: first n rows.
        let mut r0 = Table {
            cells: self.cells[..n].to_vec(),
            styles: self.styles[..n].to_vec(),
            col_widths: self.col_widths.clone(),
            arg_heights: heights[..n].to_vec(),
            repeat_rows: self.repeat_rows,
            bkgrnd: Vec::new(),
            spans: Vec::new(),
            lines: Vec::new(),
            calc: Calc::default(),
        };
        for l in &line_cmds {
            if let Some(r) = Self::cr0(l.range, ni, nrows) {
                r0.lines.push(LineCmd {
                    range: r,
                    ..l.clone()
                });
            }
        }
        for b in &self.bkgrnd {
            if let Some(r) = Self::cr0(b.range, ni, nrows) {
                r0.bkgrnd.push(BgCmd {
                    range: r,
                    color: b.color,
                });
            }
        }
        for s in &self.spans {
            if let Some(r) = Self::cr0(*s, ni, nrows) {
                r0.spans.push(r);
            }
        }

        // R1: repeat rows + rows from n.
        let mut cells: Vec<Vec<Cell>> = self.cells[..self.repeat_rows].to_vec();
        cells.extend(self.cells[n..].iter().cloned());
        let mut styles: Vec<Vec<CellStyle>> = self.styles[..self.repeat_rows].to_vec();
        styles.extend(self.styles[n..].iter().cloned());
        let mut hs: Vec<Option<f64>> = heights[..self.repeat_rows].to_vec();
        hs.extend(heights[n..].iter().cloned());
        let mut r1 = Table {
            cells,
            styles,
            col_widths: self.col_widths.clone(),
            arg_heights: hs,
            repeat_rows: self.repeat_rows,
            bkgrnd: Vec::new(),
            spans: Vec::new(),
            lines: Vec::new(),
            calc: Calc::default(),
        };
        for l in &line_cmds {
            for r in Self::cr11(l.range, ni, nrows, &repeat) {
                r1.lines.push(LineCmd {
                    range: r,
                    ..l.clone()
                });
            }
        }
        for b in &self.bkgrnd {
            for r in Self::cr11(b.range, ni, nrows, &repeat) {
                r1.bkgrnd.push(BgCmd {
                    range: r,
                    color: b.color,
                });
            }
        }
        for s in &self.spans {
            for r in Self::cr11(*s, ni, nrows, &repeat) {
                r1.spans.push(r);
            }
        }
        vec![r0, r1]
    }

    // ------------------------------------------------------------------ drawing

    /// `Table.draw` with the canvas translated to the table's bottom-left corner.
    pub fn draw(&mut self, out: &mut Content) {
        self.do_calc();
        out.save_state();
        self.draw_background(out);
        let (nr, nc) = (self.nrows(), self.ncols());
        for r in 0..nr {
            for c in 0..nc {
                let rect = if self.spans.is_empty() {
                    Some((
                        self.calc.col_positions[c],
                        self.calc.row_positions[r + 1],
                        self.col_widths[c],
                        self.calc.row_heights[r],
                    ))
                } else {
                    self.calc.span_rects.get(&(c, r)).copied().flatten()
                };
                if let Some((x, y, w, h)) = rect {
                    self.draw_cell(out, r, c, (x, y), (w, h));
                }
            }
        }
        self.draw_lines(out);
        out.restore_state();
    }

    fn draw_background(&self, out: &mut Content) {
        let (nr, nc) = (self.nrows(), self.ncols());
        for b in &self.bkgrnd {
            let ((sc, sr), (ec, er)) = b.range;
            let sc = norm_idx(sc, nc) as usize;
            let ec = norm_idx(ec, nc) as usize;
            let sr = norm_idx(sr, nr) as usize;
            let er = norm_idx(er, nr) as usize;
            let x0 = self.calc.col_positions[sc];
            let y0 = self.calc.row_positions[sr];
            let x1 = self.calc.col_positions[(ec + 1).min(nc)];
            let y1 = self.calc.row_positions[(er + 1).min(nr)];
            out.set_fill(b.color);
            out.fill_rect(x0, y0, x1 - x0, y1 - y0);
        }
    }

    fn draw_cell(
        &mut self,
        out: &mut Content,
        r: usize,
        c: usize,
        pos: (f64, f64),
        size: (f64, f64),
    ) {
        let (colpos, rowpos) = pos;
        let (colwidth, rowheight) = size;
        let st = self.styles[r][c].clone();
        out.set_fill(st.color);
        match &mut self.cells[r][c] {
            Cell::Para(p) => {
                let h = Self::para_geom(p, colwidth, &st);
                let w = colwidth - st.left_padding - st.right_padding;
                let mut y = match st.valign {
                    VAlign::Top => rowpos + rowheight - st.top_padding,
                    VAlign::Bottom => rowpos + st.bottom_padding + h,
                    VAlign::Middle => {
                        rowpos + (rowheight + st.bottom_padding - st.top_padding + h) / 2.0
                    }
                };
                let x = match st.alignment {
                    HAlign::Left => colpos + st.left_padding,
                    HAlign::Right => colpos + colwidth - st.right_padding - w,
                    HAlign::Center => {
                        colpos + (colwidth + st.left_padding - st.right_padding - w) / 2.0
                    }
                };
                y -= h;
                out.save_state();
                out.translate(x, y);
                p.draw(out);
                out.restore_state();
            }
            Cell::Text(s) => {
                let x = match st.alignment {
                    HAlign::Left => colpos + st.left_padding,
                    HAlign::Center => {
                        colpos + (colwidth + st.left_padding - st.right_padding) * 0.5
                    }
                    HAlign::Right => colpos + colwidth - st.right_padding,
                };
                let vals: Vec<&str> = s.split('\n').collect();
                let n = vals.len() as f64;
                let mut y = match st.valign {
                    VAlign::Bottom => rowpos + st.bottom_padding + n * st.leading - st.font_size,
                    VAlign::Top => rowpos + rowheight - st.top_padding - st.font_size,
                    VAlign::Middle => {
                        rowpos
                            + (st.bottom_padding + rowheight - st.top_padding + n * st.leading)
                                / 2.0
                            - st.font_size
                    }
                };
                for v in vals {
                    let w = string_width(v, st.font, st.font_size);
                    let dx = match st.alignment {
                        HAlign::Left => 0.0,
                        HAlign::Center => -0.5 * w,
                        HAlign::Right => -w,
                    };
                    out.show_text(x + dx, y, st.font, st.font_size, 0.0, v);
                    y -= st.leading;
                }
            }
        }
    }

    /// `_hLine`: draw a line avoiding spanned blocks.
    fn h_line(
        mut canv: impl FnMut(f64, f64, f64, f64),
        scp: f64,
        ecp: f64,
        y: f64,
        blocks: &[(f64, Vec<(f64, f64)>)],
    ) {
        let hb = blocks.iter().find(|e| e.0 == y).map(|e| &e.1);
        match hb {
            Some(hb) if !(scp >= hb[hb.len() - 1].1 - FUZZ || ecp <= hb[0].0 + FUZZ) => {
                let mut scp = scp;
                let mut i = 0;
                while scp < ecp - FUZZ && i < hb.len() {
                    let (x0, x1) = hb[i];
                    if x1 <= scp + FUZZ || x0 >= ecp - FUZZ {
                        i += 1;
                        continue;
                    }
                    let i0 = scp.max(x0);
                    let i1 = ecp.min(x1);
                    if i0 > scp {
                        canv(scp, y, i0, y);
                    }
                    scp = i1;
                }
                if scp < ecp - FUZZ {
                    canv(scp, y, ecp, y);
                }
            }
            _ => canv(scp, y, ecp, y),
        }
    }

    fn draw_hlines(&self, out: &mut Content, start: (i64, i64), end: (i64, i64)) {
        let (sc, sr) = start;
        let (ec, er) = end;
        let cp = &self.calc.col_positions;
        let rp = &self.calc.row_positions;
        let ecp = slice(cp, sc, ec + 2);
        let rps = slice(rp, sr, er + 1);
        if ecp.len() <= 1 || rps.is_empty() {
            return;
        }
        let scp = ecp[0];
        let ecp_end = ecp[ecp.len() - 1];
        for y in rps {
            Self::h_line(
                |a, b, c, d| out.line(a, b, c, d),
                scp,
                ecp_end,
                y,
                &self.calc.h_blocks,
            );
        }
    }

    fn draw_vlines(&self, out: &mut Content, start: (i64, i64), end: (i64, i64)) {
        let (sc, sr) = start;
        let (ec, er) = end;
        let rp = &self.calc.row_positions;
        let erp = slice(rp, sr, er + 2);
        let cps = slice(&self.calc.col_positions, sc, ec + 1);
        if erp.len() <= 1 || cps.is_empty() {
            return;
        }
        let srp = erp[0];
        let erp_end = erp[erp.len() - 1];
        for x in cps {
            // `_hLine(canvLine, erp, srp, x, vBlocks)` with swapped coordinates.
            Self::h_line(
                |y0, x0, y1, x1| out.line(x0, y0, x1, y1),
                erp_end,
                srp,
                x,
                &self.calc.v_blocks,
            );
        }
    }

    fn draw_lines(&self, out: &mut Content) {
        out.save_state();
        let mut first = true;
        for l in &self.lines {
            if first {
                out.set_line_cap(1);
                out.set_line_join(1);
                first = false;
            }
            out.set_stroke(l.color);
            out.set_line_width(l.weight);
            let ((sc, sr), (ec, er)) = l.range;
            let (nr, nc) = (self.nrows(), self.ncols());
            let sc = norm_idx(sc, nc).max(0);
            let ec = norm_idx(ec, nc).min(nc as i64 - 1);
            let sr = norm_idx(sr, nr).max(0);
            let er = norm_idx(er, nr).min(nr as i64 - 1);
            match l.op {
                LineOp::Grid => {
                    self.draw_box(out, (sc, sr), (ec, er));
                    self.draw_inner_grid(out, (sc, sr), (ec, er));
                }
                LineOp::Box => self.draw_box(out, (sc, sr), (ec, er)),
                LineOp::InnerGrid => self.draw_inner_grid(out, (sc, sr), (ec, er)),
                LineOp::LineAbove => self.draw_hlines(out, (sc, sr), (ec, er)),
                LineOp::LineBelow => self.draw_hlines(out, (sc, sr + 1), (ec, er + 1)),
                LineOp::LineBefore => self.draw_vlines(out, (sc, sr), (ec, er)),
                LineOp::LineAfter => self.draw_vlines(out, (sc + 1, sr), (ec + 1, er)),
            }
        }
        out.restore_state();
    }

    fn draw_box(&self, out: &mut Content, s: (i64, i64), e: (i64, i64)) {
        let (sc, sr) = s;
        let (ec, er) = e;
        self.draw_hlines(out, (sc, sr), (ec, sr));
        self.draw_hlines(out, (sc, er + 1), (ec, er + 1));
        self.draw_vlines(out, (sc, sr), (sc, er));
        self.draw_vlines(out, (ec + 1, sr), (ec + 1, er));
    }

    fn draw_inner_grid(&self, out: &mut Content, s: (i64, i64), e: (i64, i64)) {
        let (sc, sr) = s;
        let (ec, er) = e;
        self.draw_hlines(out, (sc, sr + 1), (ec, er));
        self.draw_vlines(out, (sc + 1, sr), (ec, er));
    }
}

/// Python-style `list[a:b]` clamped slice copy.
fn slice(v: &[f64], a: i64, b: i64) -> Vec<f64> {
    let a = a.clamp(0, v.len() as i64) as usize;
    let b = b.clamp(0, v.len() as i64) as usize;
    if a >= b {
        Vec::new()
    } else {
        v[a..b].to_vec()
    }
}
