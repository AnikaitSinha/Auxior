use std::io::{self, BufWriter, Write};
use std::sync::{Mutex, MutexGuard, Once, TryLockError};
use std::thread::{self, ThreadId};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};

use crate::{Buffer, Cell};

// A whole flush is written into one buffer and handed to the terminal in a
// single `write`, so cost here is escape-sequence bytes rather than syscalls.
const FLUSH_BUFFER_CAPACITY: usize = 64 * 1024;

// The terminal's last-known style and cursor position over the course of one
// flush, so repeated styles and redundant cursor moves can be skipped.
#[derive(Default)]
struct FlushState {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    // Where the cursor sits after the previous `Print`, when that is known.
    cursor: Option<(u16, u16)>,
}

impl FlushState {
    fn write_cell<W: Write>(
        &mut self,
        writer: &mut W,
        x: u16,
        y: u16,
        cell: &Cell,
        wrap_width: u16,
    ) -> io::Result<()> {
        if self.cursor != Some((x, y)) {
            queue!(writer, MoveTo(x, y))?;
        }

        if self.fg != Some(cell.fg) {
            queue!(writer, SetForegroundColor(cell.fg))?;
            self.fg = Some(cell.fg);
        }

        if self.bg != Some(cell.bg) {
            queue!(writer, SetBackgroundColor(cell.bg))?;
            self.bg = Some(cell.bg);
        }

        if self.bold != Some(cell.b) {
            queue!(
                writer,
                SetAttribute(if cell.b {
                    Attribute::Bold
                } else {
                    Attribute::NormalIntensity
                })
            )?;
            self.bold = Some(cell.b);
        }

        if self.italic != Some(cell.i) {
            queue!(
                writer,
                SetAttribute(if cell.i {
                    Attribute::Italic
                } else {
                    Attribute::NoItalic
                })
            )?;
            self.italic = Some(cell.i);
        }

        if self.underline != Some(cell.u) {
            queue!(
                writer,
                SetAttribute(if cell.u {
                    Attribute::Underlined
                } else {
                    Attribute::NoUnderline
                })
            )?;
            self.underline = Some(cell.u);
        }

        // A character with no width of its own would not move the cursor, so
        // every later cell in the run would land one column to the left.
        let (ch, advance) = match cell.width() {
            0 => (' ', 1),
            w => (cell.ch, w),
        };
        queue!(writer, Print(ch))?;

        // Printing into the final column may or may not wrap depending on the
        // terminal's autowrap mode, so the cursor is only tracked within a row.
        self.cursor = x
            .checked_add(advance)
            .filter(|&next| next < wrap_width)
            .map(|next| (next, y));

        Ok(())
    }

    fn write_buffer_cell<W: Write>(
        &mut self,
        writer: &mut W,
        buffer: &Buffer,
        x: u16,
        y: u16,
        wrap_width: u16,
    ) -> io::Result<()> {
        let Some(cell) = buffer.get(x, y) else {
            return Ok(());
        };

        if cell.is_continuation() {
            let covered = x
                .checked_sub(1)
                .and_then(|left| buffer.get(left, y))
                .is_some_and(|left| left.width() == 2);
            if covered {
                // Painted by the wide glyph to its left.
                return Ok(());
            }
            // Orphaned right half: nothing covers this column, so blank it.
            let blank = Cell { ch: ' ', ..*cell };
            return self.write_cell(writer, x, y, &blank, wrap_width);
        }

        self.write_cell(writer, x, y, cell, wrap_width)
    }
}
// Thread that owns the live terminal session, if one is active. The panic hook
// only restores the terminal for panics on this thread, so a panicking worker
// thread does not tear down a UI that is still running.
static SESSION_OWNER: Mutex<Option<ThreadId>> = Mutex::new(None);
static PANIC_HOOK: Once = Once::new();

fn lock_session_owner() -> MutexGuard<'static, Option<ThreadId>> {
    SESSION_OWNER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn claim_session() {
    *lock_session_owner() = Some(thread::current().id());
}

// Returns whether a session was active, so restoring happens at most once.
fn release_session() -> bool {
    lock_session_owner().take().is_some()
}

fn session_owned_by_current_thread() -> bool {
    // Never block inside a panic hook.
    let guard = match SESSION_OWNER.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return false,
    };
    *guard == Some(thread::current().id())
}

fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Leave raw mode and the alternate screen before the previous hook
            // prints, or the message goes to a screen that is about to vanish.
            if session_owned_by_current_thread() {
                let _ = restore_terminal();
            }
            previous(info);
        }));
    });
}

fn restore_terminal() -> io::Result<()> {
    if !release_session() {
        return Ok(());
    }

    let mut stdout = io::stdout();
    // Attempt both steps even if the first fails.
    let screen = execute!(stdout, Show, LeaveAlternateScreen);
    let raw = disable_raw_mode();
    screen.and(raw)
}

pub struct Terminal {
    width: u16,
    height: u16,
    // Whether crossterm raw mode / alternate screen are active.
    initialized: bool,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        install_panic_hook();

        enable_raw_mode()?;
        claim_session();

        let setup = (|| {
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, Hide)?;
            size()
        })();

        let (width, height) = match setup {
            Ok(size) => size,
            Err(err) => {
                // No `Terminal` exists yet, so `Drop` will not undo raw mode.
                let _ = restore_terminal();
                return Err(err);
            }
        };

        Ok(Self {
            width,
            height,
            initialized: true,
        })
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub(crate) fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    // Create a buffer, do the rendering, flush to screen.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        let mut buffer = Buffer::new(self.width, self.height);
        f(&mut buffer);
        self.flush(&buffer)
    }

    fn flush(&self, buffer: &Buffer) -> io::Result<()> {
        let stdout = io::stdout().lock();
        let mut writer = BufWriter::with_capacity(FLUSH_BUFFER_CAPACITY, stdout);
        Self::flush_to(buffer, self.width, &mut writer)
    }

    // Deprecated
    fn flush_to<W: Write>(buffer: &Buffer, wrap_width: u16, writer: &mut W) -> io::Result<()> {
        let mut state = FlushState::default();

        for y in 0..buffer.height {
            for x in 0..buffer.width {
                state.write_buffer_cell(writer, buffer, x, y, wrap_width)?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    fn flush_cells_to<W: Write>(
        buffer: &Buffer,
        coords: &[(u16, u16)],
        wrap_width: u16,
        writer: &mut W,
    ) -> io::Result<()> {
        if coords.is_empty() {
            return Ok(());
        }

        // Row-major order keeps runs of cells adjacent so the cursor can walk
        // them without a `MoveTo` per cell. Note the coordinates are `(x, y)`,
        // so a plain sort would order them by column.
        let owned: Vec<(u16, u16)>;
        let ordered: &[(u16, u16)] = if coords.is_sorted_by_key(|&(x, y)| (y, x)) {
            coords
        } else {
            let mut sorted = coords.to_vec();
            sorted.sort_unstable_by_key(|&(x, y)| (y, x));
            owned = sorted;
            &owned
        };

        let mut state = FlushState::default();

        for &(x, y) in ordered {
            state.write_buffer_cell(writer, buffer, x, y, wrap_width)?;
        }

        writer.flush()?;
        Ok(())
    }

    pub(crate) fn flush_cells(&self, buffer: &Buffer, coords: &[(u16, u16)]) -> io::Result<()> {
        let stdout = io::stdout().lock();
        let mut writer = BufWriter::with_capacity(FLUSH_BUFFER_CAPACITY, stdout);
        Self::flush_cells_to(buffer, coords, self.width, &mut writer)
    }

    fn restore(&mut self) -> io::Result<()> {
        restore_terminal()
    }

    #[cfg(test)]
    fn new_with_size(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            initialized: false,
        }
    }

    #[cfg(test)]
    fn draw_to_buffer<F>(&self, f: F) -> Buffer
    where
        F: FnOnce(&mut Buffer),
    {
        let mut buffer = Buffer::new(self.width, self.height);
        f(&mut buffer);
        buffer
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.restore();
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;
    use crossterm::style::Color;

    #[test]
    fn size_returns_width_and_height() {
        let term = Terminal::new_with_size(80, 24);
        assert_eq!(term.width(), 80);
        assert_eq!(term.height(), 24);
        assert_eq!(term.size(), (80, 24));
    }

    #[test]
    fn draw_passes_buffer_with_terminal_dimensions() {
        let term = Terminal::new_with_size(20, 10);

        let buf = term.draw_to_buffer(|buf| {
            assert_eq!(buf.width, 20);
            assert_eq!(buf.height, 10);
        });

        assert_eq!(buf.width, 20);
        assert_eq!(buf.height, 10);
    }

    #[test]
    fn draw_allows_writing_to_buffer() {
        let term = Terminal::new_with_size(5, 3);

        let buf = term.draw_to_buffer(|buf| {
            buf.set(2, 1, Cell::with_fg('X', Color::Red));
        });

        assert_eq!(buf.get(2, 1).unwrap().ch, 'X');
    }

    #[test]
    fn draw_flush_round_trip_without_stdout() {
        let term = Terminal::new_with_size(3, 2);
        let buf = term.draw_to_buffer(|buf| {
            buf.set(0, 0, Cell::new('A'));
        });

        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();
        assert!(String::from_utf8_lossy(&out).contains('A'));
    }

    #[test]
    fn flush_to_writes_characters() {
        let mut buf = Buffer::new(3, 2);
        buf.set(0, 0, Cell::new('A'));
        buf.set(1, 0, Cell::new('B'));
        buf.set(2, 1, Cell::new('C'));

        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains('A'));
        assert!(output.contains('B'));
        assert!(output.contains('C'));
    }

    #[test]
    fn flush_to_writes_spaces_for_empty_cells() {
        let mut buf = Buffer::new(2, 1);
        buf.fill(Cell::empty());

        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains(' '));
    }

    #[test]
    fn flush_to_handles_zero_size_buffer() {
        let buf = Buffer::new(0, 0);
        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();
    }

    #[test]
    fn flush_to_applies_foreground_color() {
        let mut buf = Buffer::new(1, 1);
        buf.set(0, 0, Cell::with_fg('Z', Color::Red));

        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains('Z'));
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn flush_to_turns_bold_off_for_non_bold_cells() {
        let mut buf = Buffer::new(2, 1);
        buf.set(0, 0, Cell::new('A').set_bold());
        buf.set(1, 0, Cell::new('B'));

        let mut out = Vec::new();
        Terminal::flush_to(&buf, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\x1b[1m"), "expected bold on for A");
        assert!(
            output.contains("\x1b[22m") || output.contains("\x1b[0m"),
            "expected bold reset before B"
        );
    }

    // `MoveTo` renders as CSI <row>;<col>H, so counting terminated sequences
    // counts cursor repositions.
    fn move_to_count(output: &str) -> usize {
        output
            .split("\x1b[")
            .skip(1)
            .filter(|seq| seq.starts_with(|c: char| c.is_ascii_digit()) && seq.contains('H'))
            .filter(|seq| {
                let end = seq.find(|c: char| !c.is_ascii_digit() && c != ';');
                end.map(|i| seq.as_bytes()[i] == b'H').unwrap_or(false)
            })
            .count()
    }

    #[test]
    fn flush_cells_emits_one_move_for_a_contiguous_run() {
        let mut buf = Buffer::new(8, 2);
        buf.set(0, 0, Cell::new('A'));
        buf.set(1, 0, Cell::new('B'));
        buf.set(2, 0, Cell::new('C'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (1, 0), (2, 0)], buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(
            output.contains("ABC"),
            "expected one uninterrupted run: {output:?}"
        );
        assert_eq!(
            move_to_count(&output),
            1,
            "adjacent cells should not each re-position the cursor: {output:?}"
        );
    }

    #[test]
    fn flush_cells_moves_for_each_disjoint_run() {
        let mut buf = Buffer::new(8, 2);
        buf.set(0, 0, Cell::new('A'));
        buf.set(4, 0, Cell::new('B'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (4, 0)], buf.width, &mut out).unwrap();

        assert_eq!(move_to_count(&String::from_utf8_lossy(&out)), 2);
    }

    #[test]
    fn flush_cells_repositions_after_the_final_column() {
        // Autowrap makes the cursor position ambiguous after the last column,
        // so the next cell must be addressed explicitly.
        let mut buf = Buffer::new(2, 2);
        buf.set(1, 0, Cell::new('A'));
        buf.set(0, 1, Cell::new('B'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(1, 0), (0, 1)], buf.width, &mut out).unwrap();

        assert_eq!(move_to_count(&String::from_utf8_lossy(&out)), 2);
    }

    #[test]
    fn flush_cells_walks_rows_not_columns() {
        // Coordinates are (x, y), so sorting them naively would order by column
        // and break every run.
        let mut buf = Buffer::new(3, 2);
        for (x, ch) in [(0, 'A'), (1, 'B'), (2, 'C')] {
            buf.set(x, 0, Cell::new(ch));
        }
        for (x, ch) in [(0, 'D'), (1, 'E'), (2, 'F')] {
            buf.set(x, 1, Cell::new(ch));
        }

        let coords = [(2, 1), (0, 0), (1, 1), (2, 0), (0, 1), (1, 0)];
        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &coords, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(
            output.contains("ABC"),
            "row 0 should print as a run: {output:?}"
        );
        assert!(
            output.contains("DEF"),
            "row 1 should print as a run: {output:?}"
        );
        assert_eq!(move_to_count(&output), 2, "one move per row: {output:?}");
    }

    #[test]
    fn flush_cells_writes_a_style_only_once_per_run() {
        let mut buf = Buffer::new(4, 1);
        for x in 0..3u16 {
            buf.set(x, 0, Cell::with_fg('x', Color::Red));
        }

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (1, 0), (2, 0)], buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert_eq!(
            output.matches("\x1b[38;5;9m").count() + output.matches("\x1b[31m").count(),
            1,
            "identical adjacent styles should not be re-sent: {output:?}"
        );
    }

    #[test]
    fn flush_cells_ignores_out_of_bounds_coords() {
        let mut buf = Buffer::new(2, 1);
        buf.set(0, 0, Cell::new('A'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (99, 99)], buf.width, &mut out).unwrap();

        assert!(String::from_utf8_lossy(&out).contains('A'));
    }

    #[test]
    fn flush_cells_writes_nothing_for_empty_coords() {
        let buf = Buffer::new(4, 4);
        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[], buf.width, &mut out).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn wide_glyph_advances_cursor_by_two() {
        let mut buf = Buffer::new(6, 1);
        buf.set(0, 0, Cell::new('a'));
        buf.set(1, 0, Cell::new('日'));
        buf.set(3, 0, Cell::new('b'));

        let coords: Vec<(u16, u16)> = (0..4).map(|x| (x, 0)).collect();
        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &coords, buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("a日b"), "one run: {output:?}");
        assert!(!out.contains(&0), "continuation cells must not be printed");
        assert_eq!(move_to_count(&output), 1, "{output:?}");
    }

    #[test]
    fn wide_glyph_ending_at_wrap_edge_forgets_cursor() {
        let mut buf = Buffer::new(3, 2);
        buf.set(1, 0, Cell::new('日'));
        buf.set(0, 1, Cell::new('b'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(1, 0), (2, 0), (0, 1)], buf.width, &mut out).unwrap();

        assert_eq!(move_to_count(&String::from_utf8_lossy(&out)), 2);
    }

    #[test]
    fn orphaned_continuation_prints_as_space() {
        let mut buf = Buffer::new(3, 1);
        *buf.get_mut(1, 0).unwrap() = Cell::continuation_of(Cell::new('x'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (1, 0), (2, 0)], buf.width, &mut out).unwrap();

        assert!(!out.contains(&0));
        assert_eq!(move_to_count(&String::from_utf8_lossy(&out)), 1);
    }

    #[test]
    fn zero_width_cell_prints_as_space_to_keep_columns_aligned() {
        let mut buf = Buffer::new(3, 1);
        buf.set(0, 0, Cell::new('\u{0301}'));
        buf.set(1, 0, Cell::new('b'));

        let mut out = Vec::new();
        Terminal::flush_cells_to(&buf, &[(0, 0), (1, 0)], buf.width, &mut out).unwrap();

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains(" b"), "{output:?}");
        assert!(!output.contains('\u{0301}'));
        assert_eq!(move_to_count(&output), 1);
    }

    #[test]
    fn session_is_owned_by_the_claiming_thread_and_released_once() {
        // One test so nothing else races on the global session owner.
        assert!(!session_owned_by_current_thread());

        claim_session();
        assert!(session_owned_by_current_thread());
        assert!(
            !std::thread::spawn(session_owned_by_current_thread)
                .join()
                .unwrap(),
            "a panic on another thread must not restore the terminal"
        );

        assert!(release_session());
        assert!(!release_session(), "restoring twice must be a no-op");
        assert!(!session_owned_by_current_thread());
    }
}
