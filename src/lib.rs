//! A terminal UI library.
//!
//! Auxior draws widgets into a [`Buffer`] each frame and sends only the cells that changed to
//! the terminal. Widgets are rebuilt every frame from your application's state, so there is no
//! widget tree to keep in sync: describe what the screen should show, and Auxior works out what
//! to redraw.
//!
//! # A first app
//!
//! ```no_run
//! use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Text, Widget};
//!
//! fn main() -> std::io::Result<()> {
//!     let mut app = App::with_config(AppConfig::new().default_quit_keys())?;
//!
//!     app.run(|buf, _previous, _events, ctx, _stats| {
//!         buf.fill(Cell::empty());
//!         let area = Area::new_from_buffer(buf);
//!         let mut canvas = Canvas::new(buf, area);
//!
//!         Div::new()
//!             .border(true)
//!             .title(Text::new("Hello"))
//!             .child(Text::new("Press q to quit"))
//!             .render_with_context(&mut canvas, ctx);
//!
//!         ControlFlow::Continue
//!     })
//! }
//! ```
//!
//! # Without a terminal
//!
//! Widgets draw onto a [`Canvas`] over any [`Buffer`], neither of which needs a terminal, so a
//! screen can be drawn and checked in a test:
//!
//! ```
//! use auxior::Text;
//! use auxior::testing::TestTerminal;
//!
//! let mut term = TestTerminal::new(12, 1);
//! term.draw(&Text::new("Hello"));
//!
//! term.assert_text("Hello");
//! ```
//!
//! See [`testing`] for reading colors back, comparing frames, and drawing onto a canvas
//! directly.
//!
//! # What's here
//!
//! - Running an app: [`App`], [`AppConfig`], [`AppEvent`], [`ControlFlow`]
//! - Drawing: [`Buffer`], [`Canvas`], [`Area`], [`Cell`], [`Color`]
//! - Layout: [`Div`], [`Flex`], [`Grid`], [`ScrollView`]
//! - Content: [`Text`], [`Markdown`], [`Button`], [`Input`], [`Bar`], [`ScrollGraph`],
//!   [`Image`]
//! - Composites: [`List`], [`Table`], [`StatusBar`], [`SparklineGraph`]
//! - Input: [`KeyBinding`], [`Focus`], and mouse events through [`AppConfig::mouse_capture()`]
//! - Your own widgets: implement [`Widget`]
//! - Testing: [`testing::TestTerminal`] draws widgets without a terminal

#![warn(missing_docs)]

mod composites;
mod core;
pub mod guide;
mod media;
mod widgets;

// Re-exported so downstream crates can use the exact `crossterm` version Auxior
// was built against without declaring their own (possibly mismatched) dependency.
pub use crossterm;
pub use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crossterm::style::Color;

pub use core::testing;

pub use composites::{List, SparklineGraph, StatusBar, StatusType, Table};
pub use core::{
    App, AppConfig, AppEvent, Area, Buffer, Canvas, Cell, ControlFlow, Focus, FocusId, FrameStats,
    KeyBinding, RenderContext, Terminal,
};
pub use widgets::{
    Align, Animation, AnimationState, Bar, BorderAlign, BorderChars, BorderSide, BorderSides,
    BorderStyle, Button, Direction, Div, DivOptions, Filter, Fit, Flex, FlexDirection, Grid,
    Heading, Image, Input, InputState, LayoutOptions, Markdown, Picture, PixelMode, Repeat,
    ScrollGraph, ScrollState, ScrollView, Text, Widget,
};

pub use media::{GifFrame, RawGif};
