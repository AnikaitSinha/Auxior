# Auxior

A terminal UI library for Rust.

## Usage

```rust
use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Text, Widget};

fn main() -> std::io::Result<()> {
    // Auxior takes no keys for itself. Opt in to `q` / `Esc` quitting, pick your
    // own bindings with `quit_key`, or leave them unset and return
    // `ControlFlow::Break` yourself.
    let mut app = App::with_config(AppConfig::new().target_fps(60).default_quit_keys())?;

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title(Text::new("Hello"))
            .padding(1)
            .child(Text::new("q or Esc to quit"))
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })
}
```

Auxior re-exports the `crossterm` version it was built against, so you do not need
to depend on `crossterm` yourself: use `auxior::{Color, KeyCode, KeyEvent, KeyModifiers}`
for the common types, or `auxior::crossterm::...` for the rest.

## License

Copyright © 2026 Anikait Sinha

Licensed under the [GNU Lesser General Public License v3.0 or later](LICENSE) (LGPL-3.0-or-later).

You may use this library in your own projects (including commercial ones). If you modify and distribute Auxior itself, those changes must be made available under the same license. See [LICENSE](LICENSE) for the full text.
