//! A waterfall (masonry) layout: items go, in order, into whichever columns
//! are shortest, so cards of different heights pack tightly with no gaps
//! across a row. An item can span several columns (it then starts below the
//! tallest of the columns it covers).

/// Where one item goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// First column it covers.
    pub col: usize,
    /// How many columns it covers.
    pub span: usize,
    /// Its top, from the grid's top.
    pub y: f32,
}

/// Lay out items with these column spans and heights in `cols` columns with
/// `gap` between them. Returns each item's place and the grid's height.
#[cfg_attr(not(test), allow(dead_code))]
pub fn layout(spans: &[usize], heights: &[f32], cols: usize, gap: f32) -> (Vec<Place>, f32) {
    let cols = cols.max(1);
    let mut tops = vec![0.0f32; cols];
    let mut out = Vec::with_capacity(spans.len());
    for (i, s) in spans.iter().enumerate() {
        let span = (*s).clamp(1, cols);
        let h = heights.get(i).copied().unwrap_or(0.0).max(0.0);
        // the start column whose covered columns end highest (leftmost on ties)
        let mut best = (0usize, f32::MAX);
        for c in 0..=cols - span {
            let y = tops[c..c + span].iter().copied().fold(0.0, f32::max);
            if y < best.1 - 0.5 {
                best = (c, y);
            }
        }
        let (col, y) = best;
        for t in &mut tops[col..col + span] {
            *t = y + h + gap;
        }
        out.push(Place { col, span, y });
    }
    let total = tops.iter().copied().fold(0.0, f32::max);
    (out, if spans.is_empty() { 0.0 } else { total - gap })
}

/// Like `layout`, but each item goes into the highest open space it fits,
/// holes included (a short card can tuck in beside a tall one, under a wide
/// one). Order still decides who picks first.
pub fn pack(spans: &[usize], heights: &[f32], cols: usize, gap: f32) -> (Vec<Place>, f32) {
    let cols = cols.max(1);
    // placed: (first col, span, top, bottom)
    let mut placed: Vec<(usize, usize, f32, f32)> = Vec::with_capacity(spans.len());
    let mut out = Vec::with_capacity(spans.len());
    for (i, s) in spans.iter().enumerate() {
        let span = (*s).clamp(1, cols);
        let h = heights.get(i).copied().unwrap_or(0.0).max(0.0);
        let mut ys: Vec<f32> = std::iter::once(0.0).chain(placed.iter().map(|p| p.3 + gap)).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        let free = |c: usize, y: f32| {
            placed.iter().all(|&(pc, ps, py0, py1)| {
                let cols_meet = c < pc + ps && pc < c + span;
                let rows_meet = y < py1 + gap - 0.01 && py0 < y + h + gap - 0.01;
                !(cols_meet && rows_meet)
            })
        };
        let mut best: Option<(usize, f32)> = None;
        for c in 0..=cols - span {
            if let Some(y) = ys.iter().copied().find(|y| free(c, *y)) {
                if best.map_or(true, |(_, by)| y < by - 0.5) {
                    best = Some((c, y));
                }
            }
        }
        let (col, y) = best.unwrap_or((0, placed.iter().map(|p| p.3 + gap).fold(0.0, f32::max)));
        placed.push((col, span, y, y + h));
        out.push(Place { col, span, y });
    }
    let total = placed.iter().map(|p| p.3).fold(0.0, f32::max);
    (out, total)
}

/// How many columns fit `width` (each at least `min_col` wide), at most `max`.
pub fn columns(width: f32, min_col: f32, gap: f32, max: usize) -> usize {
    (((width + gap) / (min_col + gap)).floor() as usize).clamp(1, max.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_the_shortest_column_first() {
        // three columns: a tall, then two short; the 4th goes under a short one
        let (p, h) = layout(&[1, 1, 1, 1], &[300.0, 100.0, 100.0, 50.0], 3, 10.0);
        assert_eq!(p[0], Place { col: 0, span: 1, y: 0.0 });
        assert_eq!(p[1].col, 1);
        assert_eq!(p[2].col, 2);
        assert_eq!(p[3], Place { col: 1, span: 1, y: 110.0 });
        assert_eq!(h, 300.0);
    }

    #[test]
    fn wide_items_start_under_everything_they_cover() {
        let (p, h) = layout(&[1, 1, 2], &[100.0, 200.0, 50.0], 2, 10.0);
        assert_eq!(p[2], Place { col: 0, span: 2, y: 210.0 });
        assert_eq!(h, 260.0);
    }

    #[test]
    fn spans_clamp_to_the_columns_and_one_column_stacks() {
        let (p, h) = layout(&[3, 1], &[40.0, 60.0], 1, 8.0);
        assert_eq!(p[0], Place { col: 0, span: 1, y: 0.0 });
        assert_eq!(p[1].y, 48.0);
        assert_eq!(h, 108.0);
        assert_eq!(layout(&[], &[], 3, 8.0).1, 0.0);
    }

    #[test]
    fn pack_fills_holes_beside_tall_items() {
        // a tall card on the left, a short one on the right, then a wide one:
        // the last small card tucks into the hole under the short one
        let (p, h) = pack(&[1, 1, 2, 1], &[300.0, 100.0, 100.0, 100.0], 2, 10.0);
        assert_eq!(p[0], Place { col: 0, span: 1, y: 0.0 });
        assert_eq!(p[1], Place { col: 1, span: 1, y: 0.0 });
        assert_eq!(p[2], Place { col: 0, span: 2, y: 310.0 });
        assert_eq!(p[3], Place { col: 1, span: 1, y: 110.0 });
        assert_eq!(h, 410.0);
        // with no holes it matches the plain waterfall
        let a = layout(&[1, 1, 1, 2], &[120.0, 80.0, 60.0, 40.0], 3, 8.0);
        let b = pack(&[1, 1, 1, 2], &[120.0, 80.0, 60.0, 40.0], 3, 8.0);
        assert_eq!(a, b);
    }

    #[test]
    fn column_count() {
        assert_eq!(columns(1200.0, 360.0, 18.0, 3), 3);
        assert_eq!(columns(800.0, 360.0, 18.0, 3), 2);
        assert_eq!(columns(300.0, 360.0, 18.0, 3), 1);
    }
}
