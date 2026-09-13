mod composites;
mod core;
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
    Bar, BorderAlign, BorderSide, Button, Div, DivOptions, Flex, FlexDirection, Grid,
    LayoutOptions, ScrollGraph, Text, Widget,
};
