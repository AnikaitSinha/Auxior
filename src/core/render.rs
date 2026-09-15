use super::{Area, Buffer};

/// Statistics about one frame sent to the terminal, for tuning and debugging.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    /// Cells that changed and were written to the terminal.
    pub flushed_cells: usize,
    /// Cells compared with the previous frame: those inside dirty regions, each counted once.
    pub checked_cells: usize,
    /// Areas marked as redrawn.
    pub dirty_regions: usize,
    /// Cells on screen.
    pub total_cells: u64,
    /// Whether every cell was compared, as on the first frame and after a resize.
    pub force_full: bool,
    /// Coordinates of the cells written to the terminal.
    pub flushed_coords: Vec<(u16, u16)>,
}

/// Tracks which parts of the screen were redrawn this frame, so only those are compared with
/// the previous frame.
///
/// [`App::run`](crate::App::run) passes one to each frame. Widgets drawn with
/// [`Widget::render_with_context`](crate::Widget::render_with_context) mark their area dirty;
/// areas nothing marks are assumed unchanged.
pub struct RenderContext<'a> {
    /// The previous frame.
    pub previous: &'a Buffer,
    /// Areas marked as redrawn this frame.
    pub dirty_regions: Vec<Area>,
    /// Compare every cell, ignoring the dirty regions.
    pub force_full: bool,
}

impl<'a> RenderContext<'a> {
    /// A context comparing against `previous`, with nothing marked yet.
    pub fn new(previous: &'a Buffer) -> Self {
        Self {
            previous,
            dirty_regions: Vec::new(),
            force_full: false,
        }
    }

    /// Marks `area` as redrawn this frame. Empty areas are ignored.
    pub fn mark_dirty(&mut self, area: Area) {
        if area.width > 0 && area.height > 0 {
            self.dirty_regions.push(area);
        }
    }

    /// The cells in the dirty regions (or everywhere, with [`force_full`](Self::force_full))
    /// that differ from the previous frame, row by row.
    pub fn diff_coords(&self, current: &Buffer) -> Vec<(u16, u16)> {
        if self.force_full {
            return current.diff_region(
                self.previous,
                Area::new(0, 0, current.width, current.height),
            );
        }

        if self.dirty_regions.is_empty() {
            return Vec::new();
        }

        let mut coords = Vec::new();
        for area in &self.dirty_regions {
            coords.extend(current.diff_region(self.previous, *area));
        }
        // Row-major, matching what the flush path walks. Coordinates are
        // `(x, y)`, so the derived ordering would sort them by column.
        coords.sort_unstable_by_key(|&(x, y)| (y, x));
        coords.dedup();
        coords
    }

    /// How many cells the dirty regions cover, counting overlaps once.
    pub fn checked_cells(&self, buffer: &Buffer) -> usize {
        if self.force_full {
            return buffer.width as usize * buffer.height as usize;
        }

        if self.dirty_regions.is_empty() {
            return 0;
        }

        // Mark covered cells instead of collecting and sorting coordinates.
        let width = buffer.width as usize;
        let mut covered = vec![false; width * buffer.height as usize];
        let mut count = 0;
        for area in &self.dirty_regions {
            let x_end = area.x.saturating_add(area.width).min(buffer.width);
            let y_end = area.y.saturating_add(area.height).min(buffer.height);
            for y in area.y..y_end {
                for x in area.x..x_end {
                    let seen = &mut covered[y as usize * width + x as usize];
                    if !*seen {
                        *seen = true;
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    #[test]
    fn mark_dirty_ignores_zero_width_and_height() {
        let previous = Buffer::new(4, 4);
        let mut ctx = RenderContext::new(&previous);

        ctx.mark_dirty(Area::new(0, 0, 0, 2));
        ctx.mark_dirty(Area::new(0, 0, 2, 0));

        assert!(ctx.dirty_regions.is_empty());
    }

    #[test]
    fn mark_dirty_records_positive_area() {
        let previous = Buffer::new(4, 4);
        let mut ctx = RenderContext::new(&previous);
        let area = Area::new(1, 2, 3, 1);

        ctx.mark_dirty(area);

        assert_eq!(ctx.dirty_regions, vec![area]);
    }

    #[test]
    fn diff_coords_empty_when_no_dirty_regions() {
        let previous = Buffer::new(3, 3);
        let mut current = Buffer::new(3, 3);
        current.set(1, 1, Cell::new('x'));

        let ctx = RenderContext::new(&previous);

        assert!(ctx.diff_coords(&current).is_empty());
    }

    #[test]
    fn diff_coords_force_full_diffs_entire_buffer() {
        let previous = Buffer::new(2, 2);
        let mut current = Buffer::new(2, 2);
        current.set(1, 1, Cell::new('a'));

        let mut ctx = RenderContext::new(&previous);
        ctx.force_full = true;

        assert_eq!(ctx.diff_coords(&current), vec![(1, 1)]);
    }

    #[test]
    fn diff_coords_scoped_to_dirty_regions_only() {
        let previous = Buffer::new(4, 4);
        let mut current = Buffer::new(4, 4);
        current.set(0, 0, Cell::new('a'));
        current.set(3, 3, Cell::new('z'));

        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 1, 1));

        assert_eq!(ctx.diff_coords(&current), vec![(0, 0)]);
    }

    #[test]
    fn diff_coords_dedupes_overlapping_dirty_regions() {
        let previous = Buffer::new(3, 3);
        let mut current = Buffer::new(3, 3);
        current.set(1, 1, Cell::new('x'));

        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 2, 2));
        ctx.mark_dirty(Area::new(1, 1, 2, 2));

        let coords = ctx.diff_coords(&current);
        assert_eq!(coords, vec![(1, 1)]);
    }

    #[test]
    fn diff_coords_returns_empty_when_dirty_region_unchanged() {
        let mut previous = Buffer::new(2, 2);
        previous.set(0, 0, Cell::new('h'));

        let mut current = Buffer::new(2, 2);
        current.set(0, 0, Cell::new('h'));

        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 1, 1));

        assert!(ctx.diff_coords(&current).is_empty());
    }

    #[test]
    fn checked_cells_force_full_equals_buffer_size() {
        let previous = Buffer::new(5, 4);
        let mut ctx = RenderContext::new(&previous);
        ctx.force_full = true;

        assert_eq!(ctx.checked_cells(&previous), 20);
    }

    #[test]
    fn checked_cells_counts_non_overlapping_regions() {
        let previous = Buffer::new(10, 10);
        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 2, 3));
        ctx.mark_dirty(Area::new(4, 0, 3, 2));

        assert_eq!(ctx.checked_cells(&previous), 6 + 6);
    }

    #[test]
    fn checked_cells_dedupes_overlapping_regions() {
        let previous = Buffer::new(10, 10);
        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 4, 4));
        ctx.mark_dirty(Area::new(2, 2, 4, 4));

        assert_eq!(ctx.checked_cells(&previous), 28);
    }

    #[test]
    fn checked_cells_zero_when_no_dirty_regions_and_not_force_full() {
        let previous = Buffer::new(8, 8);
        let ctx = RenderContext::new(&previous);

        assert_eq!(ctx.checked_cells(&previous), 0);
    }

    #[test]
    fn diff_coords_are_row_major() {
        let previous = Buffer::new(4, 3);
        let mut current = Buffer::new(4, 3);
        for (x, y) in [(3, 0), (0, 0), (2, 2), (1, 1)] {
            current.set(x, y, Cell::new('x'));
        }

        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 4, 3));

        let coords = ctx.diff_coords(&current);
        assert_eq!(coords, vec![(0, 0), (3, 0), (1, 1), (2, 2)]);
        assert!(coords.is_sorted_by_key(|&(x, y)| (y, x)));
    }
}
