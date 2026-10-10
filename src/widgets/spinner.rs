use crate::widgets::layout_builders;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crossterm::style::Color;

use crate::core::text_width;
use crate::{Canvas, Cell, LayoutOptions, Widget};

// Fast enough to read as motion, slow enough not to blur.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(80);

// One clock for every spinner in the process, so they all turn in step and none of them needs
// state of its own. Started the first time a spinner is drawn.
fn clock() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// The characters a [`Spinner`] turns through.
///
/// ```
/// use auxior::SpinnerStyle;
///
/// assert_eq!(SpinnerStyle::Line.frames(), &['|', '/', '-', '\\']);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// Braille dots: `⠋⠙⠹⠸⠼⠴⠦⠧`. The default.
    #[default]
    Dots,
    /// A turning line: `|/-\`, for terminals without braille.
    Line,
    /// A turning arc: `◜◠◝◞◡◟`.
    Arc,
    /// A growing and shrinking bar: `▁▃▄▅▆▇▆▅▄▃`.
    Bar,
    /// Characters of your own, turned through in order.
    Custom(&'static [char]),
}

impl SpinnerStyle {
    /// The characters this style turns through.
    pub fn frames(self) -> &'static [char] {
        match self {
            SpinnerStyle::Dots => &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'],
            SpinnerStyle::Line => &['|', '/', '-', '\\'],
            SpinnerStyle::Arc => &['◜', '◠', '◝', '◞', '◡', '◟'],
            SpinnerStyle::Bar => &['▁', '▃', '▄', '▅', '▆', '▇', '▆', '▅', '▄', '▃'],
            SpinnerStyle::Custom(frames) => frames,
        }
    }
}

/// A one-character animation for work that is going on, with an optional label beside it.
///
/// ```
/// use auxior::{Spinner, SpinnerStyle};
/// use auxior::testing::render_to_text;
///
/// let spinner = Spinner::new().style(SpinnerStyle::Line).label("Fetching");
///
/// // Which frame shows depends on the clock, but the label does not.
/// assert!(render_to_text(&spinner, 12, 1).ends_with(" Fetching  "));
/// ```
///
/// A spinner needs no state, because it keeps no position of its own: which frame to draw is
/// worked out from the clock each time it is drawn. That means it turns at the same speed
/// whatever frame rate the app runs at, and skipping frames cannot make it stutter — it is
/// never behind, because there is nothing to be behind.
///
/// Every spinner in the process reads one clock, so several on screen turn in step.
///
/// An app showing a spinner should draw frames while it turns: give
/// [`AppConfig::target_fps`](crate::AppConfig::target_fps) a rate at least as fast as
/// [`interval`](Spinner::interval), or the spinner will only move when something else causes a
/// frame.
pub struct Spinner {
    style: SpinnerStyle,
    interval: Duration,
    label: Option<String>,
    fg: Color,
    layout: LayoutOptions,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// A spinner of braille dots, turning every 80 milliseconds.
    pub fn new() -> Self {
        Self {
            style: SpinnerStyle::default(),
            interval: DEFAULT_INTERVAL,
            label: None,
            fg: Color::Reset,
            layout: LayoutOptions::default(),
        }
    }

    /// Sets the characters it turns through.
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets how long each character shows for. A zero interval holds the first character still.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets text shown one column to the right of the spinner.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the color of the spinner and its label.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Sets the column offset within the container.
    pub fn x(mut self, n: u16) -> Self {
        self.layout.x = Some(n);
        self
    }

    /// Sets the row offset within the container.
    pub fn y(mut self, n: u16) -> Self {
        self.layout.y = Some(n);
        self
    }

    /// Sets a fixed width in columns.
    pub fn width(mut self, n: u16) -> Self {
        self.layout.width = Some(n);
        self
    }

    /// Sets a fixed height in rows.
    pub fn height(mut self, n: u16) -> Self {
        self.layout.height = Some(n);
        self
    }

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex) or
    /// [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    layout_builders!(layout);

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    /// The character showing right now.
    ///
    /// Worked out from how long the process has been running, so it does not depend on how
    /// often the spinner is drawn.
    pub fn frame(&self) -> char {
        let frames = self.style.frames();
        if frames.is_empty() {
            return ' ';
        }

        let index = if self.interval.is_zero() {
            0
        } else {
            let turns = clock().elapsed().as_nanos() / self.interval.as_nanos();
            (turns % frames.len() as u128) as usize
        };

        frames[index]
    }
}

impl Widget for Spinner {
    fn render(&self, canvas: &mut Canvas) {
        if canvas.width() == 0 || canvas.height() == 0 {
            return;
        }

        let style = Cell::with_fg(' ', self.fg);
        canvas.set(
            0,
            0,
            Cell {
                ch: self.frame(),
                ..style
            },
        );

        if let Some(label) = &self.label {
            canvas.set_str(2, 0, label, style);
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        1
    }

    fn default_width(&self) -> u16 {
        match &self.label {
            // The spinner, a blank column, then the label.
            Some(label) => text_width(label).saturating_add(2),
            None => 1,
        }
    }
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestTerminal, render_to_text};

    #[test]
    fn a_spinner_draws_one_of_its_frames() {
        let spinner = Spinner::new();
        let drawn = render_to_text(&spinner, 1, 1);
        let ch = drawn.chars().next().unwrap();

        assert!(
            SpinnerStyle::Dots.frames().contains(&ch),
            "drew {ch:?}, which is not one of the frames",
        );
    }

    #[test]
    fn a_zero_interval_holds_the_first_frame() {
        let spinner = Spinner::new().interval(Duration::ZERO);
        assert_eq!(spinner.frame(), '⠋');
        assert_eq!(render_to_text(&spinner, 1, 1), "⠋");
    }

    #[test]
    fn each_style_has_its_own_frames() {
        assert_eq!(SpinnerStyle::Line.frames(), &['|', '/', '-', '\\']);
        assert_eq!(SpinnerStyle::Dots.frames().len(), 8);
        assert_eq!(SpinnerStyle::Arc.frames().len(), 6);
        assert_eq!(SpinnerStyle::Bar.frames().len(), 10);
        assert_eq!(SpinnerStyle::default(), SpinnerStyle::Dots);
    }

    #[test]
    fn frames_of_your_own_are_turned_through() {
        let spinner = Spinner::new()
            .style(SpinnerStyle::Custom(&['a', 'b']))
            .interval(Duration::ZERO);

        assert_eq!(spinner.frame(), 'a');
    }

    #[test]
    fn a_style_with_no_frames_draws_a_blank() {
        let spinner = Spinner::new().style(SpinnerStyle::Custom(&[]));
        assert_eq!(spinner.frame(), ' ');
    }

    #[test]
    fn a_label_sits_one_column_to_the_right() {
        let spinner = Spinner::new().interval(Duration::ZERO).label("Loading");

        assert_eq!(render_to_text(&spinner, 10, 1), "⠋ Loading ");
    }

    #[test]
    fn the_width_covers_the_spinner_and_its_label() {
        assert_eq!(Spinner::new().default_width(), 1);
        assert_eq!(Spinner::new().label("abc").default_width(), 5);
        // Measured in columns, so wide characters count twice.
        assert_eq!(Spinner::new().label("日本").default_width(), 6);
        assert_eq!(Spinner::new().default_height(), 1);
    }

    #[test]
    fn a_zero_sized_canvas_draws_nothing() {
        let mut term = TestTerminal::new(0, 1);
        term.draw(&Spinner::new());
        assert_eq!(term.to_text(), "");
    }

    #[test]
    fn which_frame_shows_is_worked_out_from_the_clock() {
        // Two spinners drawn at the same moment agree, because neither keeps a position of
        // its own: the frame comes from the clock, not from how often it was drawn.
        let a = Spinner::new();
        let b = Spinner::new();
        assert_eq!(a.frame(), b.frame());

        // Drawing it many times does not move it along: a long interval means the frame can
        // only change when that much time has really passed.
        let slow = Spinner::new().interval(Duration::from_secs(600));
        let before = slow.frame();
        for _ in 0..50 {
            let _ = render_to_text(&slow, 1, 1);
        }
        assert_eq!(slow.frame(), before);
    }
}
