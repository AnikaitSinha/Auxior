use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use super::{Buffer, FrameStats, KeyBinding, RenderContext, Terminal, begin_frame, dispatch_input};

/// Options for an [`App`]: frame rate, quit keys and mouse capture.
///
/// ```
/// use auxior::{AppConfig, KeyBinding, KeyCode};
///
/// let config = AppConfig::new()
///     .target_fps(30)
///     .quit_key(KeyBinding::ctrl(KeyCode::Char('c')))
///     .mouse_capture(true);
/// assert_eq!(config.target_fps, 30);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// The most frames drawn per second. See [`AppConfig::target_fps()`].
    pub target_fps: u64,
    /// Keys that end [`App::run`] before the frame callback sees them. Empty by default, so the
    /// application receives every key.
    pub quit_keys: Vec<KeyBinding>,
    /// Whether the terminal reports mouse events. See [`AppConfig::mouse_capture()`].
    pub mouse_capture: bool,
    /// Whether only the areas widgets marked as redrawn are compared with the
    /// previous frame. See [`AppConfig::incremental()`].
    pub incremental: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            quit_keys: Vec::new(),
            mouse_capture: false,
            incremental: false,
        }
    }
}

impl AppConfig {
    /// The default configuration: 60 frames per second, no quit keys and no mouse capture.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the frame rate the event loop aims for. Values below 1 are raised to 1.
    pub fn target_fps(mut self, fps: u64) -> Self {
        self.target_fps = fps.max(1);
        self
    }

    /// Adds a key that ends the event loop, such as `'q'` or `KeyCode::Esc`.
    ///
    /// Adding the same key twice has no further effect.
    pub fn quit_key(mut self, key: impl Into<KeyBinding>) -> Self {
        let binding = key.into();
        if !self.quit_keys.contains(&binding) {
            self.quit_keys.push(binding);
        }
        self
    }

    /// Adds a key that ends the event loop only while `modifiers` are held.
    pub fn quit_key_with(self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.quit_key(KeyBinding::with(code, modifiers))
    }

    /// Replaces every quit key with `keys`.
    pub fn quit_keys<I>(mut self, keys: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<KeyBinding>,
    {
        self.quit_keys = Vec::new();
        for key in keys {
            self = self.quit_key(key);
        }
        self
    }

    /// Reports mouse events as [`AppEvent::Mouse`], and makes widgets such as
    /// [`Button`](crate::Button) clickable and [`ScrollView`](crate::ScrollView) scrollable
    /// with the wheel.
    ///
    /// Off by default: while the terminal captures the mouse, users cannot select text with it.
    pub fn mouse_capture(mut self, on: bool) -> Self {
        self.mouse_capture = on;
        self
    }

    /// Compares only the areas widgets marked as redrawn with the previous
    /// frame, instead of the whole screen.
    ///
    /// Off by default, and worth turning on only for large screens that change
    /// very little, together with [`Div::dirty`](crate::Div::dirty). With it on,
    /// anything drawn without marking its area — by calling
    /// [`Widget::render`](crate::Widget::render) instead of
    /// [`render_with_context`](crate::Widget::render_with_context), or by writing
    /// into the buffer directly — will not reach the screen.
    pub fn incremental(mut self, on: bool) -> Self {
        self.incremental = on;
        self
    }
    /// Adds the common pair of quit keys, `q` and `Esc`.
    pub fn default_quit_keys(self) -> Self {
        self.quit_key(KeyCode::Char('q')).quit_key(KeyCode::Esc)
    }

    /// The time budget for one frame at the configured frame rate.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs(1) / self.target_fps.max(1) as u32
    }

    fn is_quit_key(&self, event: &AppEvent) -> bool {
        let AppEvent::Key(key) = event else {
            return false;
        };

        self.quit_keys.iter().any(|binding| binding.matches(key))
    }
}

/// Something that happened since the last frame, passed to the [`App::run`] callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// No input arrived. The frame is drawn on schedule anyway, for animation or live data.
    Tick,
    /// A key was pressed, repeated or released. Test it with [`KeyBinding::matches`], which
    /// ignores releases.
    Key(KeyEvent),
    /// A mouse event. Only delivered while [`AppConfig::mouse_capture()`] is on.
    Mouse(MouseEvent),
    /// The terminal was resized.
    Resize {
        /// New width in columns.
        width: u16,
        /// New height in rows.
        height: u16,
    },
}

/// Whether [`App::run`] keeps going after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    /// Draw another frame.
    Continue,
    /// Return from [`App::run`].
    Break,
}

/// Runs an interactive terminal application.
///
/// Creating an `App` switches the terminal into raw mode and the alternate screen. Dropping it
/// restores the terminal, and so does a panic on the thread that created it.
///
/// ```no_run
/// use auxior::{App, AppConfig, ControlFlow};
///
/// let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
/// app.run(|buf, _previous, _events, _ctx, _stats| {
///     // Draw into `buf` here.
///     ControlFlow::Continue
/// })?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct App {
    terminal: Terminal,
    config: AppConfig,
    previous: Buffer,
    current: Buffer,
    first_frame: bool,
    frame_stats: FrameStats,
}

impl App {
    /// Starts an app with the default [`AppConfig`].
    ///
    /// # Errors
    ///
    /// Fails if the terminal cannot be set up, for example when standard output is not a
    /// terminal.
    pub fn new() -> io::Result<Self> {
        Self::with_config(AppConfig::default())
    }

    /// Starts an app with `config`.
    ///
    /// # Errors
    ///
    /// Fails if the terminal cannot be set up or mouse capture cannot be turned on.
    pub fn with_config(config: AppConfig) -> io::Result<Self> {
        let mut terminal = Terminal::new()?;
        if config.mouse_capture {
            terminal.enable_mouse_capture()?;
        }
        let (w, h) = terminal.size();
        let previous = Buffer::new(w, h);
        let current = Buffer::new(w, h);

        Ok(Self {
            terminal,
            config,
            previous,
            current,
            first_frame: true,
            frame_stats: FrameStats::default(),
        })
    }

    /// Statistics about the last frame sent to the terminal.
    pub fn frame_stats(&self) -> &FrameStats {
        &self.frame_stats
    }

    /// The configuration the app started with.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// The terminal the app draws to.
    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Mutable access to the terminal the app draws to.
    pub fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    /// Runs the event loop until `frame` returns [`ControlFlow::Break`] or a quit key is
    /// pressed.
    ///
    /// Each frame, `frame` receives:
    ///
    /// - the buffer to draw into, still holding what the previous frame drew;
    /// - the previous frame's buffer;
    /// - every event since the last frame, or a single [`AppEvent::Tick`] if there were none;
    /// - a [`RenderContext`] for incremental drawing with
    ///   [`Widget::render_with_context`](crate::Widget::render_with_context);
    /// - statistics about the previous frame.
    ///
    /// Input is handled before `frame` runs, so a button pressed this frame already shows its
    /// effect in what `frame` draws.
    ///
    /// # Errors
    ///
    /// Fails if reading input or writing to the terminal fails.
    pub fn run<F>(&mut self, mut frame: F) -> io::Result<()>
    where
        F: FnMut(&mut Buffer, &Buffer, &[AppEvent], &mut RenderContext, &FrameStats) -> ControlFlow,
    {
        let frame_duration = self.config.frame_duration();
        let mut last_frame = Instant::now();
        let mut pending_events: Vec<AppEvent> = Vec::new();

        loop {
            let poll_timeout = Self::poll_timeout(frame_duration, last_frame);
            if event::poll(poll_timeout)? {
                Self::drain_events(&mut self.terminal, &mut pending_events)?;
            }

            let now = Instant::now();
            if now.duration_since(last_frame) < frame_duration {
                continue;
            }

            let frame_events = if pending_events.is_empty() {
                vec![AppEvent::Tick]
            } else {
                std::mem::take(&mut pending_events)
            };

            if frame_events.iter().any(|e| self.config.is_quit_key(e)) {
                break;
            }

            // Route input against the bindings, regions and focus order of the
            // frame that was on screen when the user acted, before rendering
            // replaces them, so a handler's effect shows up in the frame
            // rendered below.
            dispatch_input(&frame_events);
            begin_frame();

            // let mut control = ControlFlow::Continue;

            if frame_events
                .iter()
                .any(|e| matches!(e, AppEvent::Resize { .. }))
            {
                let (w, h) = self.terminal.size();
                self.current = Buffer::new(w, h);
                self.previous = Buffer::new(w, h);
                self.first_frame = true;
            }

            if !self.first_frame {
                self.current.copy_buffer_from(&self.previous);
            }

            let mut ctx = RenderContext::new(&self.previous);
            // The whole screen is compared unless the application asked for
            // incremental drawing, so a widget drawn without marking its area
            // still reaches the terminal. Comparing cells is cheap; sending them
            // is what costs.
            if self.first_frame || !self.config.incremental {
                ctx.force_full = true;
            }

            let control = frame(
                &mut self.current,
                &self.previous,
                &frame_events,
                &mut ctx,
                &self.frame_stats,
            );

            let coords = if self.first_frame {
                self.current.all_coords()
            } else {
                ctx.diff_coords(&self.current)
            };

            self.terminal.flush_cells(&self.current, &coords)?;

            // Built after flushing, so the coordinates move in instead of being
            // copied every frame.
            self.frame_stats = FrameStats {
                flushed_cells: coords.len(),
                checked_cells: ctx.checked_cells(&self.current),
                dirty_regions: ctx.dirty_regions.len(),
                total_cells: self.current.width as u64 * self.current.height as u64,
                force_full: ctx.force_full,
                flushed_coords: coords,
            };

            std::mem::swap(&mut self.current, &mut self.previous);
            self.first_frame = false;

            last_frame = now;

            if control == ControlFlow::Break {
                break;
            }
        }

        Ok(())
    }

    fn poll_timeout(frame_duration: Duration, last_frame: Instant) -> Duration {
        let elapsed = last_frame.elapsed();
        if elapsed >= frame_duration {
            Duration::ZERO
        } else {
            frame_duration - elapsed
        }
    }

    fn drain_events(terminal: &mut Terminal, pending: &mut Vec<AppEvent>) -> io::Result<()> {
        loop {
            match event::read()? {
                Event::Key(key) => pending.push(AppEvent::Key(key)),
                Event::Mouse(mouse) => pending.push(AppEvent::Mouse(mouse)),
                Event::Resize(width, height) => {
                    terminal.set_size(width, height);
                    pending.push(AppEvent::Resize { width, height });
                }
                _ => {}
            }

            if !event::poll(Duration::ZERO)? {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_target_fps_is_60() {
        assert_eq!(AppConfig::default().target_fps, 60);
    }

    #[test]
    fn frame_duration_from_target_fps() {
        let config = AppConfig::default().target_fps(30);
        assert_eq!(
            config.frame_duration(),
            Duration::from_nanos(1_000_000_000 / 30)
        );
    }

    #[test]
    fn target_fps_minimum_is_one() {
        let config = AppConfig::default().target_fps(0);
        assert_eq!(config.target_fps, 1);
    }

    #[test]
    fn no_quit_keys_by_default() {
        let config = AppConfig::default();
        assert!(config.quit_keys.is_empty());

        let q = AppEvent::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let esc = AppEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(!config.is_quit_key(&q));
        assert!(!config.is_quit_key(&esc));
    }

    #[test]
    fn default_quit_keys_match_q_and_esc() {
        let config = AppConfig::new().default_quit_keys();

        let quit = AppEvent::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let esc = AppEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let other = AppEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        assert!(config.is_quit_key(&quit));
        assert!(config.is_quit_key(&esc));
        assert!(!config.is_quit_key(&other));
        assert!(!config.is_quit_key(&AppEvent::Tick));
    }

    #[test]
    fn quit_key_respects_modifiers() {
        let config = AppConfig::new().quit_key_with(KeyCode::Char('c'), KeyModifiers::CONTROL);

        let ctrl_c = AppEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        let plain_c = AppEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert!(config.is_quit_key(&ctrl_c));
        assert!(!config.is_quit_key(&plain_c));
    }

    #[test]
    fn quit_keys_replaces_existing_bindings() {
        let config = AppConfig::new()
            .default_quit_keys()
            .quit_keys([KeyCode::Char('x')]);

        assert_eq!(config.quit_keys, vec![KeyBinding::from('x')]);
    }

    #[test]
    fn quit_key_is_not_added_twice() {
        let config = AppConfig::new()
            .quit_key(KeyCode::Esc)
            .quit_key(KeyCode::Esc);

        assert_eq!(config.quit_keys.len(), 1);
    }

    #[test]
    fn mouse_capture_is_off_by_default() {
        assert!(!AppConfig::default().mouse_capture);
        assert!(AppConfig::new().mouse_capture(true).mouse_capture);
    }

    #[test]
    fn quit_keys_accept_chars_and_named_keys() {
        let config = AppConfig::new().quit_key('q').quit_key(KeyCode::F(10));

        let f10 = AppEvent::Key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
        assert!(config.is_quit_key(&f10));
        assert!(config.is_quit_key(&AppEvent::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE
        ))));
    }

    #[test]
    fn releasing_a_quit_key_does_not_quit() {
        use crossterm::event::{KeyEventKind, KeyEventState};

        let config = AppConfig::new().default_quit_keys();
        let release = AppEvent::Key(KeyEvent::new_with_kind_and_state(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Release,
            KeyEventState::NONE,
        ));

        assert!(!config.is_quit_key(&release));
    }

    #[test]
    fn incremental_drawing_is_off_by_default() {
        assert!(!AppConfig::default().incremental);
        assert!(AppConfig::new().incremental(true).incremental);
    }
}
