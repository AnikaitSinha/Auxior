use std::cell::Cell as StdCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crossterm::style::Color;

use crate::{Canvas, Cell, LayoutOptions, Widget};

// Braille dots, top row first, left column first in each row.
const BRAILLE_DOTS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
// Characters from darkest to lightest, for `PixelMode::Ascii`.
const RAMP: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
// Terminal cells are about twice as tall as they are wide.
const CELL_ASPECT: f32 = 2.0;
// How far apart the lightest and darkest parts of a cell must be before braille
// treats it as an edge to trace rather than as flat shading.
const EDGE_CONTRAST: f32 = 64.0;
// The most samples one point averages, per axis. Sixteen lookups a cell at the
// worst, which is nothing next to the cost of sending the result.
const MAX_TAPS: u16 = 4;
// Where each sample sits inside its share of the area, as a fraction of it.
//
// Sampling the middle of each share sounds right and is a trap: the samples
// then sit at a regular spacing, and a picture with a regular pattern of its
// own can line up with it, so that every sample lands on the same part of the
// pattern. A finely chequered picture comes out solid white rather than grey,
// and the dithering that GIFs are full of turns into bands of flat colour.
// These
// offsets come from the golden ratio, which lines up with nothing.
const TAP_OFFSETS: [f32; MAX_TAPS as usize] = [0.618_034, 0.236_068, 0.854_102, 0.472_136];
// As fast as an animation may be played. Far beyond any real use, and low
// enough that multiplying the clock by it cannot overflow.
const MAX_SPEED: f32 = 1000.0;

/// A decoded picture: red, green and blue for every pixel.
///
/// This is the hand-off point between a decoder and the screen. Build one from
/// pixels a decoder produced, or from a function, and hand it to
/// [`Image`]. Cloning is cheap: the pixels are shared.
///
/// ```
/// use auxior::Picture;
///
/// // A two by two picture: red, green / blue, white.
/// let pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
/// let picture = Picture::from_rgb(2, 2, pixels).unwrap();
///
/// assert_eq!(picture.size(), (2, 2));
/// assert_eq!(picture.pixel(1, 0), (0, 255, 0));
/// ```
#[derive(Debug, Clone)]
pub struct Picture {
    inner: Rc<Pixels>,
}

#[derive(Debug)]
struct Pixels {
    width: u16,
    height: u16,
    // Red, green and blue per pixel, row by row.
    rgb: Vec<u8>,
}

impl Picture {
    /// A picture from `width` × `height` pixels, three bytes each.
    ///
    /// Returns `None` if the size is empty or the pixels don't match it.
    pub fn from_rgb(width: u16, height: u16, rgb: Vec<u8>) -> Option<Self> {
        let expected = usize::from(width) * usize::from(height) * 3;
        if width == 0 || height == 0 || rgb.len() != expected {
            return None;
        }

        Some(Self {
            inner: Rc::new(Pixels { width, height, rgb }),
        })
    }

    /// A picture whose pixels come from `pixel`, called with each position.
    pub fn from_fn(
        width: u16,
        height: u16,
        mut pixel: impl FnMut(u16, u16) -> (u8, u8, u8),
    ) -> Option<Self> {
        let mut rgb = Vec::with_capacity(usize::from(width) * usize::from(height) * 3);
        for y in 0..height {
            for x in 0..width {
                let (r, g, b) = pixel(x, y);
                rgb.extend_from_slice(&[r, g, b]);
            }
        }
        Self::from_rgb(width, height, rgb)
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u16, u16) {
        (self.inner.width, self.inner.height)
    }

    /// The pixel at `(x, y)`, clamped to the edges.
    pub fn pixel(&self, x: u16, y: u16) -> (u8, u8, u8) {
        let x = x.min(self.inner.width - 1);
        let y = y.min(self.inner.height - 1);
        let at = (usize::from(y) * usize::from(self.inner.width) + usize::from(x)) * 3;
        (
            self.inner.rgb[at],
            self.inner.rgb[at + 1],
            self.inner.rgb[at + 2],
        )
    }
}

/// What happens when an [`Animation`] reaches its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Start again from the first picture.
    #[default]
    Loop,
    /// Hold the last picture.
    Once,
    /// Play backwards, then forwards, and so on.
    PingPong,
}

/// A sequence of pictures with a duration each: a decoded GIF, or anything
/// else that moves.
///
/// Cloning is cheap; the pictures are shared. Playback position is kept
/// separately, in an [`AnimationState`], so several views can play the same
/// animation at different points.
#[derive(Debug, Clone)]
pub struct Animation {
    inner: Rc<Sequence>,
}

#[derive(Debug)]
struct Sequence {
    pictures: Vec<Picture>,
    // When each picture stops being shown, measured from the start.
    ends: Vec<Duration>,
    total: Duration,
}

impl Animation {
    /// An animation from pictures and how long each is shown.
    ///
    /// A duration of zero is treated as 100 ms, which is what browsers do with
    /// GIFs that ask for no delay.
    pub fn new(pictures: impl IntoIterator<Item = (Picture, Duration)>) -> Self {
        let mut frames = Vec::new();
        let mut ends = Vec::new();
        let mut total = Duration::ZERO;

        for (picture, delay) in pictures {
            let delay = if delay.is_zero() {
                Duration::from_millis(100)
            } else {
                delay
            };
            total = total.saturating_add(delay);
            frames.push(picture);
            ends.push(total);
        }

        Self {
            inner: Rc::new(Sequence {
                pictures: frames,
                ends,
                total,
            }),
        }
    }

    /// Every picture shown for the same length of time.
    pub fn with_frame_rate(pictures: impl IntoIterator<Item = Picture>, fps: u32) -> Self {
        let delay = Duration::from_secs(1) / fps.max(1);
        Self::new(pictures.into_iter().map(|picture| (picture, delay)))
    }

    /// How many pictures it has.
    pub fn len(&self) -> usize {
        self.inner.pictures.len()
    }

    /// Whether it has no pictures at all.
    pub fn is_empty(&self) -> bool {
        self.inner.pictures.is_empty()
    }

    /// How long one play through takes.
    pub fn duration(&self) -> Duration {
        self.inner.total
    }

    /// Which picture is showing `elapsed` into playback.
    pub fn index_at(&self, elapsed: Duration, repeat: Repeat) -> Option<usize> {
        let last = self.inner.pictures.len().checked_sub(1)?;
        if self.inner.total.is_zero() {
            return Some(0);
        }

        let position = match repeat {
            Repeat::Loop => div_rem(elapsed, self.inner.total),
            Repeat::Once if elapsed >= self.inner.total => return Some(last),
            Repeat::Once => elapsed,
            Repeat::PingPong => {
                let cycle = self.inner.total * 2;
                let position = div_rem(elapsed, cycle);
                if position >= self.inner.total {
                    // Second half of the cycle: the same moment, counted from
                    // the end, so each picture is shown for its own duration.
                    return Some(self.index_for(cycle - position - Duration::from_nanos(1)));
                }
                position
            }
        };

        Some(self.index_for(position))
    }

    /// The picture showing `elapsed` into playback.
    pub fn picture_at(&self, elapsed: Duration, repeat: Repeat) -> Option<&Picture> {
        self.inner.pictures.get(self.index_at(elapsed, repeat)?)
    }

    // The picture covering `position`, which must be within one play through.
    fn index_for(&self, position: Duration) -> usize {
        self.inner
            .ends
            .partition_point(|end| *end <= position)
            .min(self.inner.pictures.len() - 1)
    }
}

// `Duration` has no remainder operator.
fn div_rem(value: Duration, span: Duration) -> Duration {
    let value = value.as_nanos();
    let span = span.as_nanos().max(1);
    Duration::from_nanos((value % span) as u64)
}

/// How far into an [`Animation`] playback has reached, kept by the application
/// between frames.
///
/// Widgets are rebuilt every frame and cannot remember anything, so the clock
/// lives here. Playback follows the wall clock rather than counting frames: if
/// the application draws faster than the animation, pictures repeat; if it
/// draws slower, pictures are skipped and the animation still takes the time it
/// should.
///
/// ```
/// use std::time::Duration;
/// use auxior::AnimationState;
///
/// let playing = AnimationState::new();
/// assert!(!playing.is_paused());
///
/// // A stopped clock stays where it is put.
/// playing.pause();
/// playing.set_elapsed(Duration::from_millis(250));
/// assert_eq!(playing.elapsed(), Duration::from_millis(250));
/// ```
#[derive(Debug, Clone)]
pub struct AnimationState {
    inner: Rc<Clock>,
}

#[derive(Debug)]
struct Clock {
    // Time already counted, before the current run.
    base: StdCell<Duration>,
    // When the current run started, or `None` while paused.
    running_since: StdCell<Option<Instant>>,
    speed: StdCell<f32>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationState {
    /// A clock that starts now.
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Clock {
                base: StdCell::new(Duration::ZERO),
                running_since: StdCell::new(Some(Instant::now())),
                speed: StdCell::new(1.0),
            }),
        }
    }

    /// How long playback has been going, leaving out any paused time.
    pub fn elapsed(&self) -> Duration {
        let base = self.inner.base.get();
        match self.inner.running_since.get() {
            Some(since) => base + since.elapsed().mul_f32(self.inner.speed.get()),
            None => base,
        }
    }

    /// Jumps to a moment in the animation.
    pub fn set_elapsed(&self, elapsed: Duration) {
        self.inner.base.set(elapsed);
        if self.inner.running_since.get().is_some() {
            self.inner.running_since.set(Some(Instant::now()));
        }
    }

    /// Starts again from the beginning.
    pub fn restart(&self) {
        self.set_elapsed(Duration::ZERO);
    }

    /// Stops the clock, leaving the current picture on screen.
    pub fn pause(&self) {
        if self.inner.running_since.get().is_some() {
            self.inner.base.set(self.elapsed());
            self.inner.running_since.set(None);
        }
    }

    /// Starts the clock again after [`pause`](AnimationState::pause).
    pub fn resume(&self) {
        if self.inner.running_since.get().is_none() {
            self.inner.running_since.set(Some(Instant::now()));
        }
    }

    /// Pauses if running, resumes if paused.
    pub fn toggle(&self) {
        if self.is_paused() {
            self.resume();
        } else {
            self.pause();
        }
    }

    /// Whether the clock is stopped.
    pub fn is_paused(&self) -> bool {
        self.inner.running_since.get().is_none()
    }

    /// How fast time passes: 1.0 is normal, 2.0 is twice as fast.
    ///
    /// Clamped to between zero and a thousand. Nothing sensible asks for more,
    /// and a wild value would otherwise overflow the clock.
    pub fn set_speed(&self, speed: f32) {
        // Bank the time spent at the old speed first.
        self.inner.base.set(self.elapsed());
        if self.inner.running_since.get().is_some() {
            self.inner.running_since.set(Some(Instant::now()));
        }
        // Deliberately not `clamp`, which passes a NaN straight through and
        // would leave the clock to panic on it later. `max` returns the other
        // side of the comparison for a NaN, turning it into zero.
        #[allow(clippy::manual_clamp)]
        self.inner.speed.set(speed.max(0.0).min(MAX_SPEED));
    }

    /// The current speed.
    pub fn speed(&self) -> f32 {
        self.inner.speed.get()
    }
}

/// How pixels are turned into characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelMode {
    /// Two pixels per cell, one above the other, drawn as `▀` with a colour
    /// for each half. The most faithful of the three.
    #[default]
    HalfBlock,
    /// Eight pixels per cell as braille dots, in one colour. The most detail,
    /// the least colour.
    Braille,
    /// One character per cell, chosen by brightness, in one colour. The
    /// classic ASCII-art look.
    Ascii,
}

impl PixelMode {
    // How many sample points one cell needs, across and down.
    fn cell_samples(self) -> (u16, u16) {
        match self {
            PixelMode::HalfBlock => (1, 2),
            PixelMode::Braille => (2, 4),
            PixelMode::Ascii => (1, 1),
        }
    }

    // How tall one sample point is on screen, relative to its width.
    fn sample_aspect(self) -> f32 {
        let (across, down) = self.cell_samples();
        CELL_ASPECT * f32::from(across) / f32::from(down)
    }
}

/// How a picture is fitted to the space it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// Keep the shape, fit the whole picture, leave the rest untouched.
    #[default]
    Contain,
    /// Keep the shape, fill the space, crop the overflow.
    Cover,
    /// Stretch to fill, changing the shape.
    Stretch,
}

/// Draws a [`Picture`] or an [`Animation`] as coloured characters.
///
/// Nothing is fixed to a particular size: the picture is sampled afresh for
/// whatever space the widget is given, so it keeps working when the terminal is
/// resized.
///
/// ```
/// use auxior::{Area, Buffer, Canvas, Image, Picture};
///
/// // Top half red, bottom half blue.
/// let picture = Picture::from_fn(1, 2, |_, y| if y == 0 { (255, 0, 0) } else { (0, 0, 255) }).unwrap();
///
/// let mut buf = Buffer::new(1, 1);
/// let area = Area::new_from_buffer(&buf);
/// Image::picture(&picture).render(&mut Canvas::new(&mut buf, area));
///
/// // One cell holds both: the upper half block is the top pixel, its
/// // background the bottom one.
/// let cell = buf.get(0, 0).unwrap();
/// assert_eq!(cell.ch, '▀');
/// assert_eq!(cell.fg, auxior::Color::Rgb { r: 255, g: 0, b: 0 });
/// assert_eq!(cell.bg, auxior::Color::Rgb { r: 0, g: 0, b: 255 });
/// ```
pub struct Image {
    source: Source,
    mode: PixelMode,
    fit: Fit,
    repeat: Repeat,
    // Only set when the caller supplied one; the default costs nothing.
    ramp: Option<Vec<char>>,
    layout: LayoutOptions,
}

enum Source {
    Still(Picture),
    Playing(Animation, AnimationState),
}

impl Image {
    fn new(source: Source) -> Self {
        Self {
            source,
            mode: PixelMode::default(),
            fit: Fit::default(),
            repeat: Repeat::default(),
            ramp: None,
            layout: LayoutOptions::default(),
        }
    }

    /// Draws a single picture.
    pub fn picture(picture: &Picture) -> Self {
        Self::new(Source::Still(picture.clone()))
    }

    /// Plays an animation, at the point `state` has reached.
    pub fn animation(animation: &Animation, state: &AnimationState) -> Self {
        Self::new(Source::Playing(animation.clone(), state.clone()))
    }

    /// How pixels become characters. Defaults to
    /// [`HalfBlock`](PixelMode::HalfBlock).
    pub fn mode(mut self, mode: PixelMode) -> Self {
        self.mode = mode;
        self
    }

    /// How the picture is fitted to its space. Defaults to
    /// [`Contain`](Fit::Contain).
    pub fn fit(mut self, fit: Fit) -> Self {
        self.fit = fit;
        self
    }

    /// What happens at the end of an animation. Defaults to
    /// [`Loop`](Repeat::Loop).
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// The characters [`Ascii`](PixelMode::Ascii) draws with, darkest first.
    ///
    /// Defaults to `` .:-=+*#%@``. Give it the same characters backwards for a
    /// terminal with a light background, where dark ink means a bright pixel:
    ///
    /// ```
    /// # use auxior::{Image, Picture};
    /// # let picture = Picture::from_fn(1, 1, |_, _| (0, 0, 0)).unwrap();
    /// let light = Image::picture(&picture).ramp("@%#*+=-:. ".chars());
    /// ```
    ///
    /// An empty set of characters is ignored, and the default kept.
    pub fn ramp(mut self, ramp: impl IntoIterator<Item = char>) -> Self {
        let ramp: Vec<char> = ramp.into_iter().collect();
        if !ramp.is_empty() {
            self.ramp = Some(ramp);
        }
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

    /// Sets the share of leftover space this takes in a [`Flex`](crate::Flex)
    /// or [`Grid`](crate::Grid), relative to its flexible siblings.
    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    /// Draws this widget; the same as [`Widget::render`](crate::Widget::render).
    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // The picture to draw this frame.
    fn current(&self) -> Option<Picture> {
        match &self.source {
            Source::Still(picture) => Some(picture.clone()),
            Source::Playing(animation, state) => {
                animation.picture_at(state.elapsed(), self.repeat).cloned()
            }
        }
    }
}

// Which part of the picture goes where on the sample grid.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Plan {
    // The sample rectangle that gets drawn.
    left: u16,
    top: u16,
    across: u16,
    down: u16,
    // The part of the picture it comes from.
    source_x: f32,
    source_y: f32,
    source_width: f32,
    source_height: f32,
}

// Works out that mapping. Cheap enough to redo every frame: it is a handful of
// divisions, not a pass over the pixels.
fn plan(
    fit: Fit,
    cells: (u16, u16),
    per_cell: (u16, u16),
    aspect: f32,
    picture: (u16, u16),
) -> Option<Plan> {
    let grid_across = cells.0.saturating_mul(per_cell.0);
    let grid_down = cells.1.saturating_mul(per_cell.1);
    let (width, height) = picture;
    if grid_across == 0 || grid_down == 0 || width == 0 || height == 0 {
        return None;
    }

    let full = Plan {
        left: 0,
        top: 0,
        across: grid_across,
        down: grid_down,
        source_x: 0.0,
        source_y: 0.0,
        source_width: f32::from(width),
        source_height: f32::from(height),
    };

    // How many sample points wide the picture wants to be for every one it is
    // tall. Tall sample points need more of them across, hence the multiply.
    let shape = (f32::from(width) / f32::from(height)) * aspect;
    let grid_shape = f32::from(grid_across) / f32::from(grid_down);

    match fit {
        Fit::Stretch => Some(full),
        Fit::Contain => {
            let (across, down) = if shape > grid_shape {
                // Wider than the space: full width, short of height.
                (
                    grid_across,
                    ((f32::from(grid_across) / shape).round() as u16).max(1),
                )
            } else {
                (
                    ((f32::from(grid_down) * shape).round() as u16).max(1),
                    grid_down,
                )
            };
            let across = across.min(grid_across);
            let down = down.min(grid_down);
            // Start on a cell boundary, so no cell is left half inside the
            // picture and half outside it.
            Some(Plan {
                left: (grid_across - across) / 2 / per_cell.0 * per_cell.0,
                top: (grid_down - down) / 2 / per_cell.1 * per_cell.1,
                across,
                down,
                ..full
            })
        }
        Fit::Cover => {
            // Crop the picture to the shape of the space.
            let (source_width, source_height) = if shape > grid_shape {
                // Wider than the space: keep the height, trim the sides.
                (f32::from(height) * grid_shape / aspect, f32::from(height))
            } else {
                // Taller than the space: keep the width, trim top and bottom.
                (f32::from(width), f32::from(width) * aspect / grid_shape)
            };
            Some(Plan {
                source_x: (f32::from(width) - source_width) / 2.0,
                source_y: (f32::from(height) - source_height) / 2.0,
                source_width,
                source_height,
                ..full
            })
        }
    }
}

// The average colour of the part of the picture one sample point covers.
// Costs a fixed number of lookups however large the picture is, so the work
// depends on the size of the terminal alone.
fn sample(picture: &Picture, plan: &Plan, x: u16, y: u16, taps: (u16, u16)) -> (u8, u8, u8) {
    let (taps_x, taps_y) = taps;
    let step_x = plan.source_width / f32::from(plan.across);
    let step_y = plan.source_height / f32::from(plan.down);
    let origin_x = plan.source_x + f32::from(x) * step_x;
    let origin_y = plan.source_y + f32::from(y) * step_y;
    let (width, height) = picture.size();

    let (mut red, mut green, mut blue) = (0_u32, 0_u32, 0_u32);
    for row in 0..taps_y {
        for column in 0..taps_x {
            let across = f32::from(column) + TAP_OFFSETS[usize::from(column)];
            let down = f32::from(row) + TAP_OFFSETS[usize::from(row)];
            let at_x = origin_x + across * step_x / f32::from(taps_x);
            let at_y = origin_y + down * step_y / f32::from(taps_y);
            let (r, g, b) = picture.pixel(
                (at_x.max(0.0) as u16).min(width - 1),
                (at_y.max(0.0) as u16).min(height - 1),
            );
            red += u32::from(r);
            green += u32::from(g);
            blue += u32::from(b);
        }
    }

    let count = u32::from(taps_x) * u32::from(taps_y);
    (
        (red / count) as u8,
        (green / count) as u8,
        (blue / count) as u8,
    )
}

// How many samples each point averages, per axis: more where the picture is
// being shrunk a lot, one where it is being enlarged. Capped, so the work
// depends on the size of the screen and never on the size of the picture.
fn taps(plan: &Plan) -> (u16, u16) {
    let across = plan.source_width / f32::from(plan.across);
    let down = plan.source_height / f32::from(plan.down);
    (
        (across.round() as u16).clamp(1, MAX_TAPS),
        (down.round() as u16).clamp(1, MAX_TAPS),
    )
}

fn rgb(colour: (u8, u8, u8)) -> Color {
    Color::Rgb {
        r: colour.0,
        g: colour.1,
        b: colour.2,
    }
}

fn brightness(colour: (u8, u8, u8)) -> f32 {
    0.2126 * f32::from(colour.0) + 0.7152 * f32::from(colour.1) + 0.0722 * f32::from(colour.2)
}

impl Widget for Image {
    fn render(&self, canvas: &mut Canvas) {
        let (columns, rows) = (canvas.width(), canvas.height());
        let Some(picture) = self.current() else {
            return;
        };
        let per_cell @ (per_column, per_row) = self.mode.cell_samples();
        let Some(plan) = plan(
            self.fit,
            (columns, rows),
            per_cell,
            self.mode.sample_aspect(),
            picture.size(),
        ) else {
            return;
        };
        let taps = taps(&plan);

        // Only the cells the picture reaches are drawn, so anything behind an
        // uncovered edge shows through.
        for row in 0..rows {
            for column in 0..columns {
                let first_x = column.saturating_mul(per_column);
                let first_y = row.saturating_mul(per_row);
                // Skip the cell only if the picture misses it entirely; a cell
                // the far edge lands inside draws with its edge pixels.
                if first_x.saturating_add(per_column) <= plan.left
                    || first_y.saturating_add(per_row) <= plan.top
                    || first_x >= plan.left.saturating_add(plan.across)
                    || first_y >= plan.top.saturating_add(plan.down)
                {
                    continue;
                }

                let at = |x: u16, y: u16| {
                    // Samples past the far edge repeat the edge, which only
                    // happens in the last cell of a picture that does not
                    // divide evenly into cells.
                    let x = first_x
                        .saturating_add(x)
                        .saturating_sub(plan.left)
                        .min(plan.across - 1);
                    let y = first_y
                        .saturating_add(y)
                        .saturating_sub(plan.top)
                        .min(plan.down - 1);
                    sample(&picture, &plan, x, y, taps)
                };

                let cell = match self.mode {
                    PixelMode::HalfBlock => {
                        let (top, bottom) = (at(0, 0), at(0, 1));
                        let mut cell = if top == bottom {
                            // Flat cells are common in drawn pictures, and a
                            // blank costs one colour to send instead of two.
                            Cell::new(' ')
                        } else {
                            Cell::with_fg('▀', rgb(top))
                        };
                        cell.bg = rgb(bottom);
                        cell
                    }
                    PixelMode::Braille => braille_cell(&at),
                    PixelMode::Ascii => {
                        let colour = at(0, 0);
                        let ramp = self.ramp.as_deref().unwrap_or(RAMP);
                        let step = brightness(colour) / 255.0 * (ramp.len() - 1) as f32;
                        Cell::with_fg(ramp[step.round() as usize], rgb(colour))
                    }
                };
                canvas.set(column, row, cell);
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        self.height_for_width(self.default_width())
    }

    fn default_width(&self) -> u16 {
        self.current().map_or(1, |picture| picture.size().0.min(40))
    }

    // Tall enough to keep the picture's shape at this width.
    fn height_for_width(&self, width: u16) -> u16 {
        let Some(picture) = self.current() else {
            return 1;
        };
        let (picture_width, picture_height) = picture.size();
        let (per_column, per_row) = self.mode.cell_samples();

        let across = f32::from(width.saturating_mul(per_column));
        let shape =
            (f32::from(picture_width) / f32::from(picture_height)) * self.mode.sample_aspect();
        let down = across / shape.max(f32::EPSILON);
        ((down / f32::from(per_row)).ceil() as u16).max(1)
    }
}

// Eight dots, lit where they are brighter than the middle of this cell's range,
// so detail survives in dark and light areas alike.
fn braille_cell(at: &impl Fn(u16, u16) -> (u8, u8, u8)) -> Cell {
    let mut colours = [(0, 0, 0); 8];
    let mut levels = [0.0_f32; 8];
    for (row, masks) in BRAILLE_DOTS.iter().enumerate() {
        for column in 0..masks.len() {
            let index = row * 2 + column;
            colours[index] = at(column as u16, row as u16);
            levels[index] = brightness(colours[index]);
        }
    }

    let low = levels.iter().copied().fold(f32::MAX, f32::min);
    let high = levels.iter().copied().fold(f32::MIN, f32::max);
    // Where a cell holds a real edge, splitting it at the middle of its own
    // range traces that edge finely. Where it does not, doing so would pick out
    // whatever faint variation is there and draw it at full contrast, turning
    // the dithering in a GIF into a field of noise. Such a cell is compared
    // against plain mid grey instead, so that a dim one stays dark.
    let threshold = if high - low < EDGE_CONTRAST {
        128.0
    } else {
        (low + high) / 2.0
    };

    let mut bits = 0_u8;
    let (mut red, mut green, mut blue, mut lit) = (0_u32, 0_u32, 0_u32, 0_u32);
    for (row, masks) in BRAILLE_DOTS.iter().enumerate() {
        for (column, mask) in masks.iter().enumerate() {
            let index = row * 2 + column;
            if levels[index] >= threshold {
                bits |= mask;
                red += u32::from(colours[index].0);
                green += u32::from(colours[index].1);
                blue += u32::from(colours[index].2);
                lit += 1;
            }
        }
    }

    // With no dot lit there is no colour to average.
    let colour = match (
        red.checked_div(lit),
        green.checked_div(lit),
        blue.checked_div(lit),
    ) {
        (Some(red), Some(green), Some(blue)) => (red as u8, green as u8, blue as u8),
        _ => (0, 0, 0),
    };
    let dots = char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' ');
    Cell::with_fg(dots, rgb(colour))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Buffer};

    fn solid(width: u16, height: u16, colour: (u8, u8, u8)) -> Picture {
        Picture::from_fn(width, height, |_, _| colour).unwrap()
    }

    fn draw(image: Image, columns: u16, rows: u16) -> Buffer {
        let mut buffer = Buffer::new(columns, rows);
        let area = Area::new_from_buffer(&buffer);
        image.render(&mut Canvas::new(&mut buffer, area));
        buffer
    }

    fn colours(buffer: &Buffer, x: u16, y: u16) -> (char, Color, Color) {
        let cell = buffer.get(x, y).unwrap();
        (cell.ch, cell.fg, cell.bg)
    }

    #[test]
    fn rejects_pixels_that_do_not_match_the_size() {
        assert!(Picture::from_rgb(2, 2, vec![0; 11]).is_none());
        assert!(Picture::from_rgb(0, 2, vec![]).is_none());
        assert!(Picture::from_rgb(2, 2, vec![0; 12]).is_some());
    }

    #[test]
    fn pixels_are_read_back_by_position() {
        let picture = Picture::from_fn(3, 2, |x, y| (x as u8, y as u8, 7)).unwrap();
        assert_eq!(picture.pixel(2, 1), (2, 1, 7));
        // Out of range reads clamp to the edge rather than panicking.
        assert_eq!(picture.pixel(9, 9), (2, 1, 7));
    }

    #[test]
    fn half_block_packs_two_pixels_into_one_cell() {
        let picture =
            Picture::from_fn(1, 2, |_, y| if y == 0 { (255, 0, 0) } else { (0, 0, 255) }).unwrap();
        let buffer = draw(Image::picture(&picture), 1, 1);

        assert_eq!(
            colours(&buffer, 0, 0),
            (
                '▀',
                Color::Rgb { r: 255, g: 0, b: 0 },
                Color::Rgb { r: 0, g: 0, b: 255 }
            )
        );
    }

    #[test]
    fn a_cell_with_matching_halves_is_sent_as_a_blank() {
        // Both halves the same colour: a background alone says it, and costs
        // half as much to send as a half block with two colours.
        let buffer = draw(
            Image::picture(&solid(1, 2, (4, 5, 6))).fit(Fit::Stretch),
            1,
            1,
        );

        let (ch, fg, bg) = colours(&buffer, 0, 0);
        assert_eq!(ch, ' ');
        assert_eq!(fg, Color::Reset);
        assert_eq!(bg, Color::Rgb { r: 4, g: 5, b: 6 });
    }

    #[test]
    fn stretch_covers_every_cell() {
        let buffer = draw(
            Image::picture(&solid(1, 1, (10, 20, 30))).fit(Fit::Stretch),
            3,
            2,
        );

        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(
                    buffer.get(x, y).unwrap().bg,
                    Color::Rgb {
                        r: 10,
                        g: 20,
                        b: 30
                    },
                    "at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn contain_leaves_the_rest_of_the_space_untouched() {
        // Four times as wide as it is tall, in a square space.
        let buffer = draw(Image::picture(&solid(4, 1, (255, 255, 255))), 4, 4);

        let drawn: Vec<u16> = (0..4)
            .filter(|y| *buffer.get(0, *y).unwrap() != Cell::empty())
            .collect();
        assert_eq!(drawn.len(), 1, "one row of cells holds the picture");

        for y in 0..4 {
            if drawn.contains(&y) {
                continue;
            }
            for x in 0..4 {
                assert_eq!(*buffer.get(x, y).unwrap(), Cell::empty(), "at {x},{y}");
            }
        }
    }

    #[test]
    fn a_square_picture_stays_square_in_ascii() {
        // A cell is twice as tall as it is wide and ASCII uses one per cell, so
        // a square picture over eight columns takes four rows, not eight.
        let buffer = draw(
            Image::picture(&solid(8, 8, (255, 255, 255))).mode(PixelMode::Ascii),
            8,
            8,
        );

        let drawn = (0..8)
            .filter(|y| *buffer.get(0, *y).unwrap() != Cell::empty())
            .count();
        assert_eq!(drawn, 4);
    }

    #[test]
    fn cover_fills_the_space_by_cropping() {
        // Far taller than it is wide, in a wide space: every cell is drawn.
        let buffer = draw(
            Image::picture(&solid(1, 8, (9, 9, 9))).fit(Fit::Cover),
            4,
            2,
        );

        for y in 0..2 {
            for x in 0..4 {
                assert_ne!(*buffer.get(x, y).unwrap(), Cell::empty(), "at {x},{y}");
            }
        }
    }

    #[test]
    fn ascii_picks_characters_by_brightness() {
        let picture = Picture::from_fn(
            2,
            1,
            |x, _| if x == 0 { (0, 0, 0) } else { (255, 255, 255) },
        )
        .unwrap();
        let buffer = draw(
            Image::picture(&picture)
                .mode(PixelMode::Ascii)
                .fit(Fit::Stretch),
            2,
            1,
        );

        assert_eq!(buffer.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buffer.get(1, 0).unwrap().ch, '@');
    }

    #[test]
    fn ascii_averages_the_pixels_a_cell_covers() {
        // Two pixels, one cell: the character comes from their average.
        let picture = Picture::from_fn(
            2,
            1,
            |x, _| if x == 0 { (0, 0, 0) } else { (255, 255, 255) },
        )
        .unwrap();
        let buffer = draw(
            Image::picture(&picture)
                .mode(PixelMode::Ascii)
                .fit(Fit::Stretch),
            1,
            1,
        );

        let (ch, fg, _) = colours(&buffer, 0, 0);
        assert_eq!(
            fg,
            Color::Rgb {
                r: 127,
                g: 127,
                b: 127
            }
        );
        assert!(ch != ' ' && ch != '@', "a middle character, got {ch:?}");
    }

    #[test]
    fn braille_lights_the_dots_over_the_bright_pixels() {
        // Left half white, right half black.
        let picture = Picture::from_fn(
            2,
            4,
            |x, _| {
                if x == 0 { (255, 255, 255) } else { (0, 0, 0) }
            },
        )
        .unwrap();
        let buffer = draw(
            Image::picture(&picture)
                .mode(PixelMode::Braille)
                .fit(Fit::Stretch),
            1,
            1,
        );

        // The four left-hand dots: 0x01, 0x02, 0x04 and 0x40.
        let expected = char::from_u32(0x2800 + 0x47).unwrap();
        assert_eq!(buffer.get(0, 0).unwrap().ch, expected);
    }

    #[test]
    fn braille_leaves_a_flat_dark_cell_blank() {
        let buffer = draw(
            Image::picture(&solid(2, 4, (0, 0, 0)))
                .mode(PixelMode::Braille)
                .fit(Fit::Stretch),
            1,
            1,
        );

        assert_eq!(buffer.get(0, 0).unwrap().ch, '\u{2800}');
    }

    #[test]
    fn the_same_picture_draws_at_any_size() {
        let picture = Picture::from_fn(16, 16, |x, y| ((x * 16) as u8, (y * 16) as u8, 0)).unwrap();

        // Nothing is baked for one terminal size: each size samples afresh.
        for (columns, rows) in [(4, 2), (13, 7), (80, 24)] {
            let buffer = draw(Image::picture(&picture).fit(Fit::Stretch), columns, rows);
            assert_ne!(
                *buffer.get(columns - 1, rows - 1).unwrap(),
                Cell::empty(),
                "the far corner is drawn at {columns}x{rows}"
            );
        }
    }

    #[test]
    fn height_for_width_keeps_the_shape() {
        // A square picture over 20 columns: 10 rows, because a cell is twice
        // as tall as it is wide, so 20 by 10 cells is a square on screen.
        let image = Image::picture(&solid(10, 10, (1, 2, 3)));
        assert_eq!(image.height_for_width(20), 10);

        // Twice as wide as it is tall: half the rows again.
        let wide = Image::picture(&solid(20, 10, (1, 2, 3)));
        assert_eq!(wide.height_for_width(20), 5);

        // In ASCII each cell is one sample, so the same picture needs the same
        // rows: the shape is kept whatever the mode.
        let ascii = Image::picture(&solid(10, 10, (1, 2, 3))).mode(PixelMode::Ascii);
        assert_eq!(ascii.height_for_width(20), 10);
    }

    const MODES: [PixelMode; 3] = [PixelMode::HalfBlock, PixelMode::Braille, PixelMode::Ascii];

    #[test]
    fn contain_keeps_the_shape_and_stays_inside_the_space() {
        for mode in MODES {
            let per_cell = mode.cell_samples();
            let aspect = mode.sample_aspect();

            for (columns, rows) in [(1, 1), (3, 2), (17, 5), (80, 24), (200, 60)] {
                for picture in [
                    (1, 1),
                    (4, 1),
                    (1, 4),
                    (16, 9),
                    (9, 16),
                    (618, 618),
                    (1000, 3),
                ] {
                    let plan = plan(Fit::Contain, (columns, rows), per_cell, aspect, picture)
                        .expect("a space and a picture both have a size");
                    let where_ = format!("{mode:?} {columns}x{rows} cells, {picture:?} pixels");

                    // Inside the space, on cell boundaries, and never empty.
                    let grid = (columns * per_cell.0, rows * per_cell.1);
                    assert!(plan.across >= 1 && plan.down >= 1, "{where_}");
                    assert!(plan.left + plan.across <= grid.0, "{where_}");
                    assert!(plan.top + plan.down <= grid.1, "{where_}");
                    assert_eq!(plan.left % per_cell.0, 0, "{where_}");
                    assert_eq!(plan.top % per_cell.1, 0, "{where_}");

                    // The whole picture is shown: contain crops nothing.
                    assert_eq!(plan.source_x, 0.0, "{where_}");
                    assert_eq!(plan.source_y, 0.0, "{where_}");
                    assert_eq!(plan.source_width, f32::from(picture.0), "{where_}");
                    assert_eq!(plan.source_height, f32::from(picture.1), "{where_}");

                    // And it keeps its shape, up to rounding to whole samples.
                    // Where the picture is thinner than one sample there is
                    // nothing left to round to, so that case is skipped.
                    let shape = (f32::from(picture.0) / f32::from(picture.1)) * aspect;
                    if plan.across > 1 && plan.down > 1 {
                        let wanted = f32::from(plan.down) * shape;
                        let slack = 0.5 * shape.max(1.0) + 0.001;
                        assert!(
                            (f32::from(plan.across) - wanted).abs() <= slack,
                            "{where_}: {} across, wanted about {wanted}",
                            plan.across
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn cover_fills_the_space_and_crops_inside_the_picture() {
        for mode in MODES {
            let per_cell = mode.cell_samples();
            let aspect = mode.sample_aspect();

            for (columns, rows) in [(1, 1), (3, 2), (17, 5), (80, 24), (200, 60)] {
                for picture in [
                    (1, 1),
                    (4, 1),
                    (1, 4),
                    (16, 9),
                    (9, 16),
                    (618, 618),
                    (1000, 3),
                ] {
                    let plan = plan(Fit::Cover, (columns, rows), per_cell, aspect, picture)
                        .expect("a space and a picture both have a size");
                    let where_ = format!("{mode:?} {columns}x{rows} cells, {picture:?} pixels");

                    // Every cell is covered.
                    assert_eq!(plan.left, 0, "{where_}");
                    assert_eq!(plan.top, 0, "{where_}");
                    assert_eq!(plan.across, columns * per_cell.0, "{where_}");
                    assert_eq!(plan.down, rows * per_cell.1, "{where_}");

                    // The crop lies within the picture.
                    assert!(plan.source_x >= -0.001, "{where_}");
                    assert!(plan.source_y >= -0.001, "{where_}");
                    assert!(
                        plan.source_x + plan.source_width <= f32::from(picture.0) + 0.001,
                        "{where_}"
                    );
                    assert!(
                        plan.source_y + plan.source_height <= f32::from(picture.1) + 0.001,
                        "{where_}"
                    );
                    assert!(
                        plan.source_width > 0.0 && plan.source_height > 0.0,
                        "{where_}"
                    );

                    // And the crop is the shape of the space, so nothing is
                    // squashed: this is the whole point of cover.
                    let shown = (plan.source_width / plan.source_height) * aspect;
                    let space = f32::from(plan.across) / f32::from(plan.down);
                    assert!(
                        (shown - space).abs() <= space * 0.001,
                        "{where_}: crop shape {shown}, space shape {space}"
                    );
                }
            }
        }
    }

    #[test]
    fn stretch_uses_all_of_both() {
        for mode in MODES {
            let per_cell = mode.cell_samples();
            let plan = plan(
                Fit::Stretch,
                (9, 4),
                per_cell,
                mode.sample_aspect(),
                (16, 9),
            )
            .expect("a space and a picture both have a size");

            assert_eq!((plan.left, plan.top), (0, 0));
            assert_eq!(plan.across, 9 * per_cell.0);
            assert_eq!(plan.down, 4 * per_cell.1);
            assert_eq!(plan.source_width, 16.0);
            assert_eq!(plan.source_height, 9.0);
        }
    }

    #[test]
    fn nothing_panics_at_any_size() {
        // Sizes that have caught edge cases before: one cell, one pixel, and
        // pictures far thinner than the space they are drawn in.
        for mode in MODES {
            for fit in [Fit::Contain, Fit::Cover, Fit::Stretch] {
                for (columns, rows) in [(1, 1), (1, 7), (7, 1), (13, 4), (80, 24)] {
                    for size in [(1, 1), (1, 9), (9, 1), (3, 7), (256, 256)] {
                        let picture = Picture::from_fn(size.0, size.1, |x, y| {
                            ((x % 256) as u8, (y % 256) as u8, 0)
                        })
                        .unwrap();
                        let buffer =
                            draw(Image::picture(&picture).mode(mode).fit(fit), columns, rows);

                        // Something is always drawn: a picture never vanishes.
                        let drawn = (0..rows)
                            .flat_map(|y| (0..columns).map(move |x| (x, y)))
                            .filter(|(x, y)| *buffer.get(*x, *y).unwrap() != Cell::empty())
                            .count();
                        assert!(drawn > 0, "{mode:?} {fit:?} {columns}x{rows} from {size:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn cover_crops_from_the_middle() {
        // Four pixels across: the ends are trimmed and the middle kept.
        let picture = Picture::from_fn(4, 2, |x, _| match x {
            0 => (255, 0, 0),
            3 => (0, 0, 255),
            _ => (0, 255, 0),
        })
        .unwrap();

        let buffer = draw(Image::picture(&picture).fit(Fit::Cover), 1, 1);
        assert_eq!(
            buffer.get(0, 0).unwrap().bg,
            Color::Rgb { r: 0, g: 255, b: 0 },
            "the middle of the picture, not an end"
        );
    }

    #[test]
    fn contain_centres_what_it_draws() {
        // A sliver four times taller than it is wide, in a square-ish space.
        let buffer = draw(Image::picture(&solid(1, 4, (255, 255, 255))), 4, 2);

        let drawn: Vec<u16> = (0..4)
            .filter(|x| *buffer.get(*x, 0).unwrap() != Cell::empty())
            .collect();
        assert_eq!(drawn, vec![1], "one column, in the middle");
    }

    #[test]
    fn a_cell_averages_everything_under_it() {
        // A fine checkerboard shrunk into one cell comes out grey, rather than
        // whichever square a single sample happened to land on.
        let picture = Picture::from_fn(6, 6, |x, y| {
            if (x + y) % 2 == 0 {
                (255, 255, 255)
            } else {
                (0, 0, 0)
            }
        })
        .unwrap();

        let buffer = draw(
            Image::picture(&picture)
                .mode(PixelMode::Ascii)
                .fit(Fit::Stretch),
            1,
            1,
        );

        let Color::Rgb { r, .. } = buffer.get(0, 0).unwrap().fg else {
            panic!("a sampled cell is always a true colour");
        };
        assert!((80..=175).contains(&r), "a middling grey, got {r}");
    }

    #[test]
    fn braille_does_not_turn_dithering_into_noise() {
        // Two dark shades alternating, as a GIF dithers a dark background.
        // Faint shading is not an edge, and drawing it at full contrast would
        // fill the cell with dots that are not really there.
        let picture = Picture::from_fn(16, 16, |x, y| {
            if (x + y) % 2 == 0 {
                (60, 60, 60)
            } else {
                (20, 20, 20)
            }
        })
        .unwrap();

        let buffer = draw(
            Image::picture(&picture)
                .mode(PixelMode::Braille)
                .fit(Fit::Stretch),
            4,
            2,
        );

        for y in 0..2 {
            for x in 0..4 {
                assert_eq!(
                    buffer.get(x, y).unwrap().ch,
                    '\u{2800}',
                    "at {x},{y}: dark dithering should stay dark"
                );
            }
        }

        // A real edge is still traced: half white against half black.
        let edge = Picture::from_fn(
            2,
            4,
            |x, _| if x == 0 { (255, 255, 255) } else { (0, 0, 0) },
        )
        .unwrap();
        let buffer = draw(
            Image::picture(&edge)
                .mode(PixelMode::Braille)
                .fit(Fit::Stretch),
            1,
            1,
        );
        assert_eq!(
            buffer.get(0, 0).unwrap().ch,
            char::from_u32(0x2800 + 0x47).unwrap()
        );
    }

    #[test]
    fn a_dithered_picture_averages_rather_than_banding() {
        // Patterns with a two-pixel period, which is what dithering in a GIF
        // looks like. Shrunk down they should all come out mid-grey.
        type Pattern = (&'static str, fn(u16, u16) -> bool);
        let patterns: [Pattern; 3] = [
            ("columns", |x, _| x % 2 == 0),
            ("rows", |_, y| y % 2 == 0),
            ("chequers", |x, y| (x + y) % 2 == 0),
        ];

        for (name, lit) in patterns {
            let picture = Picture::from_fn(64, 64, |x, y| {
                if lit(x, y) {
                    (255, 255, 255)
                } else {
                    (0, 0, 0)
                }
            })
            .unwrap();

            let buffer = draw(
                Image::picture(&picture)
                    .mode(PixelMode::Ascii)
                    .fit(Fit::Stretch),
                8,
                4,
            );

            for y in 0..4 {
                for x in 0..8 {
                    let Color::Rgb { r, .. } = buffer.get(x, y).unwrap().fg else {
                        panic!("a sampled cell is always a true colour");
                    };
                    assert!(
                        (60..=195).contains(&r),
                        "{name} at {x},{y}: {r}, which is a band rather than a blend"
                    );
                }
            }
        }
    }

    #[test]
    fn a_custom_ramp_replaces_the_characters() {
        let picture = Picture::from_fn(
            2,
            1,
            |x, _| if x == 0 { (0, 0, 0) } else { (255, 255, 255) },
        )
        .unwrap();
        let image = || {
            Image::picture(&picture)
                .mode(PixelMode::Ascii)
                .fit(Fit::Stretch)
        };

        let buffer = draw(image().ramp("ab".chars()), 2, 1);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'a');
        assert_eq!(buffer.get(1, 0).unwrap().ch, 'b');

        // Backwards, for a terminal with a light background.
        let buffer = draw(image().ramp("@%#*+=-:. ".chars()), 2, 1);
        assert_eq!(buffer.get(0, 0).unwrap().ch, '@');
        assert_eq!(buffer.get(1, 0).unwrap().ch, ' ');

        // An empty ramp is ignored rather than breaking the drawing.
        let buffer = draw(image().ramp(std::iter::empty()), 2, 1);
        assert_eq!(buffer.get(1, 0).unwrap().ch, '@');
    }

    #[test]
    fn a_wild_speed_does_not_break_the_clock() {
        let state = AnimationState::new();
        state.set_speed(f32::MAX);
        assert_eq!(state.speed(), MAX_SPEED);

        state.set_speed(-5.0);
        assert_eq!(state.speed(), 0.0);

        state.set_speed(f32::NAN);
        assert_eq!(state.speed(), 0.0);

        // A stopped clock, whatever the speed.
        let held = state.elapsed();
        assert_eq!(state.elapsed(), held);
    }

    #[test]
    fn a_very_long_animation_still_has_a_duration() {
        let animation = Animation::new(
            (0..4).map(|_| (solid(1, 1, (0, 0, 0)), Duration::from_secs(u64::MAX / 2))),
        );

        assert_eq!(animation.len(), 4);
        // Saturated rather than overflowed, and still answers questions.
        assert!(
            animation
                .index_at(Duration::from_secs(1), Repeat::Loop)
                .is_some()
        );
    }

    #[test]
    fn an_empty_animation_draws_nothing() {
        let animation = Animation::new(std::iter::empty());
        let state = AnimationState::new();
        let buffer = draw(Image::animation(&animation, &state), 4, 2);

        assert_eq!(*buffer.get(0, 0).unwrap(), Cell::empty());
    }

    fn three_frames() -> Animation {
        Animation::new((0..3).map(|n| (solid(1, 2, (n * 10, 0, 0)), Duration::from_millis(100))))
    }

    #[test]
    fn frames_follow_the_clock() {
        let animation = three_frames();
        assert_eq!(animation.len(), 3);
        assert_eq!(animation.duration(), Duration::from_millis(300));

        let at = |ms| animation.index_at(Duration::from_millis(ms), Repeat::Loop);
        assert_eq!(at(0), Some(0));
        assert_eq!(at(99), Some(0));
        assert_eq!(at(100), Some(1));
        assert_eq!(at(250), Some(2));
        // Round and round.
        assert_eq!(at(300), Some(0));
        assert_eq!(at(1150), Some(2));
        assert_eq!(at(1250), Some(0));
    }

    #[test]
    fn once_holds_the_last_frame() {
        let animation = three_frames();
        let at = |ms| animation.index_at(Duration::from_millis(ms), Repeat::Once);

        assert_eq!(at(150), Some(1));
        assert_eq!(at(300), Some(2));
        assert_eq!(at(9_000), Some(2));
    }

    #[test]
    fn ping_pong_turns_around_at_the_end() {
        let animation = three_frames();
        let at = |ms| animation.index_at(Duration::from_millis(ms), Repeat::PingPong);

        assert_eq!(at(0), Some(0));
        assert_eq!(at(250), Some(2));
        // The way back: the last frame again, then down to the first.
        assert_eq!(at(300), Some(2));
        assert_eq!(at(400), Some(1));
        assert_eq!(at(550), Some(0));
        // And off again.
        assert_eq!(at(600), Some(0));
    }

    #[test]
    fn a_missing_delay_becomes_a_tenth_of_a_second() {
        let animation = Animation::new([(solid(1, 1, (0, 0, 0)), Duration::ZERO)]);
        assert_eq!(animation.duration(), Duration::from_millis(100));
    }

    #[test]
    fn a_frame_rate_spreads_the_frames_evenly() {
        let animation = Animation::with_frame_rate((0..4).map(|_| solid(1, 1, (0, 0, 0))), 4);
        assert_eq!(animation.duration(), Duration::from_secs(1));
        assert_eq!(
            animation.index_at(Duration::from_millis(600), Repeat::Loop),
            Some(2)
        );
    }

    #[test]
    fn pausing_stops_the_clock() {
        let state = AnimationState::new();
        state.pause();
        state.set_elapsed(Duration::from_millis(500));

        assert!(state.is_paused());
        let held = state.elapsed();
        assert_eq!(held, Duration::from_millis(500));
        // Still the same moment later on.
        assert_eq!(state.elapsed(), held);

        state.toggle();
        assert!(!state.is_paused());
        assert!(state.elapsed() >= held);
    }

    #[test]
    fn speed_changes_keep_the_time_already_spent() {
        let state = AnimationState::new();
        state.pause();
        state.set_elapsed(Duration::from_millis(200));
        state.set_speed(3.0);

        assert_eq!(state.speed(), 3.0);
        assert_eq!(state.elapsed(), Duration::from_millis(200));
    }

    #[test]
    fn a_paused_animation_draws_the_frame_it_stopped_on() {
        let animation = three_frames();
        let state = AnimationState::new();
        state.pause();
        state.set_elapsed(Duration::from_millis(150));

        let buffer = draw(Image::animation(&animation, &state), 1, 1);
        assert_eq!(
            buffer.get(0, 0).unwrap().bg,
            Color::Rgb { r: 10, g: 0, b: 0 },
            "the second frame"
        );
    }

    #[test]
    fn an_empty_space_draws_nothing() {
        let mut buffer = Buffer::new(0, 0);
        let area = Area::new(0, 0, 0, 0);
        Image::picture(&solid(2, 2, (0, 0, 0))).render(&mut Canvas::new(&mut buffer, area));
    }
}
