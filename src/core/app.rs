use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::{Buffer, FrameStats, RenderContext, Terminal, keymap::KeyMap};

// Target frame rate and runtime options for [`App`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub target_fps: u64,
    // Keys that end the event loop before the frame callback sees them.
    // Empty by default so the application keeps every key; opt in with
    // [`AppConfig::quit_key`] or [`AppConfig::default_quit_keys`].
    pub quit_keys: Vec<(KeyCode, KeyModifiers)>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            quit_keys: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target_fps(mut self, fps: u64) -> Self {
        self.target_fps = fps.max(1);
        self
    }

    // Add an unmodified key that ends the event loop.
    pub fn quit_key(self, code: KeyCode) -> Self {
        self.quit_key_with(code, KeyModifiers::NONE)
    }

    // Add a key plus modifiers that ends the event loop.
    pub fn quit_key_with(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        let binding = (code, modifiers);
        if !self.quit_keys.contains(&binding) {
            self.quit_keys.push(binding);
        }
        self
    }

    // Replace the quit bindings with `keys`.
    pub fn quit_keys<I>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = (KeyCode, KeyModifiers)>,
    {
        self.quit_keys = Vec::new();
        for (code, modifiers) in keys {
            self = self.quit_key_with(code, modifiers);
        }
        self
    }

    // Convenience for the common `q` / `Esc` pair.
    pub fn default_quit_keys(self) -> Self {
        self.quit_key(KeyCode::Char('q')).quit_key(KeyCode::Esc)
    }

    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs(1) / self.target_fps.max(1) as u32
    }

    fn is_quit_key(&self, event: &AppEvent) -> bool {
        let AppEvent::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        else {
            return false;
        };

        self.quit_keys
            .iter()
            .any(|(c, m)| c == code && m == modifiers)
    }
}

// Input and timing events delivered to the frame callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    // Frame tick without input (used for animations / idle redraw).
    Tick,
    Key(KeyEvent),
    Resize { width: u16, height: u16 },
}

// Controls whether the event loop continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Continue,
    Break,
}

// Interactive terminal application with a sync event loop.
pub struct App {
    terminal: Terminal,
    config: AppConfig,
    previous: Buffer,
    current: Buffer,
    first_frame: bool,
    frame_stats: FrameStats,
}

impl App {
    pub fn new() -> io::Result<Self> {
        Self::with_config(AppConfig::default())
    }

    pub fn with_config(config: AppConfig) -> io::Result<Self> {
        let terminal = Terminal::new()?;
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

    pub fn frame_stats(&self) -> &FrameStats {
        &self.frame_stats
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    // Run the event loop until the frame callback returns [`ControlFlow::Break`] or the user presses `q` / `Esc`.
    // Each frame receives all input events collected since the last draw.
    // If none are pending, the slice contains a single [`AppEvent::Tick`].
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

            KeyMap::clear();

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
            if self.first_frame {
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

            self.frame_stats = FrameStats {
                flushed_cells: coords.len(),
                checked_cells: ctx.checked_cells(&self.current),
                dirty_regions: ctx.dirty_regions.len(),
                total_cells: self.current.width as u64 * self.current.height as u64,
                force_full: ctx.force_full,
                flushed_coords: coords.clone(),
            };

            self.terminal.flush_cells(&self.current, &coords)?;

            std::mem::swap(&mut self.current, &mut self.previous);
            self.first_frame = false;

            KeyMap::dispatch(&frame_events);

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
            .quit_keys([(KeyCode::Char('x'), KeyModifiers::NONE)]);

        assert_eq!(
            config.quit_keys,
            vec![(KeyCode::Char('x'), KeyModifiers::NONE)]
        );
    }

    #[test]
    fn quit_key_is_not_added_twice() {
        let config = AppConfig::new()
            .quit_key(KeyCode::Esc)
            .quit_key(KeyCode::Esc);

        assert_eq!(config.quit_keys.len(), 1);
    }
}
