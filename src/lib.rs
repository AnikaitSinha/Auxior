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
//! Widgets draw onto a [`Canvas`] over any [`Buffer`], so a screen can be drawn and checked in
//! a test:
//!
//! ```
//! use auxior::{Area, Buffer, Canvas, Text, Widget};
//!
//! let mut buf = Buffer::new(12, 1);
//! let area = Area::new_from_buffer(&buf);
//! Text::new("Hello").render(&mut Canvas::new(&mut buf, area));
//!
//! assert_eq!(buf.get(0, 0).unwrap().ch, 'H');
//! ```
//!
//! # What's here
//!
//! - Running an app: [`App`], [`AppConfig`], [`AppEvent`], [`ControlFlow`]
//! - Drawing: [`Buffer`], [`Canvas`], [`Area`], [`Cell`], [`Color`]
//! - Layout: [`Div`], [`Flex`], [`Grid`], [`ScrollView`]
//! - Content: [`Text`], [`Markdown`], [`Button`], [`Bar`], [`ScrollGraph`]
//! - Composites: [`List`], [`Table`], [`StatusBar`], [`SparklineGraph`]
//! - Input: [`KeyBinding`], [`Focus`], and mouse events through [`AppConfig::mouse_capture()`]
//! - Your own widgets: implement [`Widget`]

#![warn(missing_docs)]

mod composites;
mod core;
pub mod guide;
mod widgets;

// Re-exported so downstream crates can use the exact `crossterm` version Auxior
// was built against without declaring their own (possibly mismatched) dependency.
pub use crossterm;
pub use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crossterm::style::Color;

pub use composites::{List, SparklineGraph, StatusBar, StatusType, Table};
pub use core::{
    App, AppConfig, AppEvent, Area, Buffer, Canvas, Cell, ControlFlow, Focus, FocusId, FrameStats,
    KeyBinding, RenderContext, Terminal,
};
pub use widgets::{
    Bar, BorderAlign, BorderSide, Button, Div, DivOptions, Flex, FlexDirection, Grid, Heading,
    LayoutOptions, Markdown, ScrollGraph, ScrollState, ScrollView, Text, Widget,
};
