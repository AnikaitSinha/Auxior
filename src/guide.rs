// The pages live in `src/docs` as ordinary Markdown. `build.rs` copies them
// into `OUT_DIR`, turning links between pages into rustdoc paths.

#![doc = include_str!(concat!(env!("OUT_DIR"), "/docs/index.md"))]

pub mod engine {
    #![doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/index.md"))]

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/overview.md"))]
    pub mod overview {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/app.md"))]
    pub mod app {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/frame-loop.md"))]
    pub mod frame_loop {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/cells-and-buffers.md"))]
    pub mod cells_and_buffers {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/rendering.md"))]
    pub mod rendering {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/terminal.md"))]
    pub mod terminal {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/engine/input.md"))]
    pub mod input {}
}

pub mod concepts {
    #![doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/index.md"))]

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/layout.md"))]
    pub mod layout {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/text.md"))]
    pub mod text {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/input-and-focus.md"))]
    pub mod input_and_focus {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/scrolling.md"))]
    pub mod scrolling {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/concepts/custom-widgets.md"))]
    pub mod custom_widgets {}
}

pub mod widgets {
    #![doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/index.md"))]

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/text.md"))]
    pub mod text {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/button.md"))]
    pub mod button {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/input.md"))]
    pub mod input {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/bar.md"))]
    pub mod bar {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/scroll-graph.md"))]
    pub mod scroll_graph {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/div.md"))]
    pub mod div {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/flex.md"))]
    pub mod flex {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/grid.md"))]
    pub mod grid {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/scroll-view.md"))]
    pub mod scroll_view {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/widgets/markdown.md"))]
    pub mod markdown {}
}

pub mod composites {
    #![doc = include_str!(concat!(env!("OUT_DIR"), "/docs/composites/index.md"))]

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/composites/list.md"))]
    pub mod list {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/composites/table.md"))]
    pub mod table {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/composites/status-bar.md"))]
    pub mod status_bar {}

    #[doc = include_str!(concat!(env!("OUT_DIR"), "/docs/composites/sparkline-graph.md"))]
    pub mod sparkline_graph {}
}
