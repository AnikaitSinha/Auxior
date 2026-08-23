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
