use super::{Area, Buffer};

// Flush/diff statistics from the previous frame.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub flushed_cells: usize,
    pub checked_cells: usize,
    pub dirty_regions: usize,
    pub total_cells: u64,
    pub force_full: bool,
    pub flushed_coords: Vec<(u16, u16)>,
}

pub struct RenderContext<'a> {
    pub previous: &'a Buffer,
    pub dirty_regions: Vec<Area>,
    pub force_full: bool,
}

impl<'a> RenderContext<'a> {
    pub fn new(previous: &'a Buffer) -> Self {
        Self {
            previous,
            dirty_regions: Vec::new(),
            force_full: false,
        }
    }

    pub fn mark_dirty(&mut self, area: Area) {
        if area.width > 0 && area.height > 0 {
            self.dirty_regions.push(area);
        }
    }

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
        coords.sort_unstable();
        coords.dedup();
        coords
    }

    // Cells compared during diff (full buffer when `force_full`, else dirty regions).
    pub fn checked_cells(&self, buffer: &Buffer) -> usize {
        if self.force_full {
            return buffer.width as usize * buffer.height as usize;
        }

        self.dirty_regions
            .iter()
            .map(|area| area.width as usize * area.height as usize)
            .sum()
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
    fn checked_cells_sums_dirty_region_areas() {
        let previous = Buffer::new(10, 10);
        let mut ctx = RenderContext::new(&previous);
        ctx.mark_dirty(Area::new(0, 0, 2, 3));
        ctx.mark_dirty(Area::new(4, 0, 3, 2));

        assert_eq!(ctx.checked_cells(&previous), 6 + 6);
    }

    #[test]
    fn checked_cells_zero_when_no_dirty_regions_and_not_force_full() {
        let previous = Buffer::new(8, 8);
        let ctx = RenderContext::new(&previous);

        assert_eq!(ctx.checked_cells(&previous), 0);
    }
}
