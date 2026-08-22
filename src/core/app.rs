use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::{Buffer, Terminal, keymap::KeyMap};

// Target frame rate and runtime options for [`App`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    pub target_fps: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { target_fps: 60 }
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

    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs(1) / self.target_fps.max(1) as u32
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
}

impl App {
    pub fn new() -> io::Result<Self> {
        Self::with_config(AppConfig::default())
    }

    pub fn with_config(config: AppConfig) -> io::Result<Self> {
        Ok(Self {
            terminal: Terminal::new()?,
            config,
        })
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
        F: FnMut(&mut Buffer, &[AppEvent]) -> ControlFlow,
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

            if frame_events.iter().any(Self::is_quit_key) {
                break;
            }

            KeyMap::clear();

            let mut control = ControlFlow::Continue;
            self.terminal.draw(|buf| {
                control = frame(buf, &frame_events);
            })?;

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

    fn is_quit_key(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::Key(KeyEvent {
                code: KeyCode::Char('q') | KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            })
        )
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
    fn quit_key_detection() {
        let quit = AppEvent::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let esc = AppEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let other = AppEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        assert!(App::is_quit_key(&quit));
        assert!(App::is_quit_key(&esc));
        assert!(!App::is_quit_key(&other));
        assert!(!App::is_quit_key(&AppEvent::Tick));
    }
}
