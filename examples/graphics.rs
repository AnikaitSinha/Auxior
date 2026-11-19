//! A bar chart drawn as a real picture, next to the same picture in half blocks.
//!
//! The chart is rendered at exactly the pixel size of the cells it fills, using
//! `Graphics::cell_pixels()`, so the terminal shows it without scaling. On a terminal that
//! cannot show pictures (or inside tmux), the left side quietly falls back to half blocks too.
//!
//! Run it in Kitty, WezTerm, Ghostty or Konsole, outside tmux. Keys: space shuffles the bars,
//! q quits.

use std::cell::Cell as StdCell;
use std::rc::Rc;

use auxior::{
    App, AppConfig, AppEvent, Area, Canvas, Cell, Color, ControlFlow, Div, Fit, Flex, Graphics,
    Image, KeyBinding, KeyCode, Picture, PixelMode, Text, Widget,
};

const BARS: usize = 12;

// A cheap, repeatable stream of numbers, so the example needs no dependency.
fn heights(seed: u32) -> Vec<f32> {
    let mut state = seed.wrapping_mul(2_654_435_761).max(1);
    (0..BARS)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            0.15 + (state % 1000) as f32 / 1000.0 * 0.8
        })
        .collect()
}

// Draws the chart into an RGB picture of exactly `width` × `height` pixels.
fn chart(width: u16, height: u16, bars: &[f32]) -> Picture {
    let (w, h) = (f32::from(width), f32::from(height));
    let slot = w / bars.len() as f32;
    let gap = (slot * 0.2).max(1.0);

    Picture::from_fn(width.max(1), height.max(1), |x, y| {
        let (fx, fy) = (f32::from(x), f32::from(y));
        // Faint grid lines every tenth of the height.
        let grid = (fy / h * 10.0).fract() < 10.0 / h;
        let index = ((fx / slot) as usize).min(bars.len() - 1);
        let inside = fx - index as f32 * slot;
        let top = h * (1.0 - bars[index]);

        if inside >= gap / 2.0 && inside < slot - gap / 2.0 && fy >= top {
            // A vertical gradient inside each bar: lighter at the top.
            let t = (fy - top) / (h - top).max(1.0);
            let hue = index as f32 / bars.len() as f32;
            (
                (80.0 + 150.0 * (1.0 - hue) * (1.0 - t * 0.5)) as u8,
                (120.0 + 100.0 * hue * (1.0 - t * 0.4)) as u8,
                (220.0 - 60.0 * t) as u8,
            )
        } else if grid {
            (45, 48, 58)
        } else {
            (22, 24, 30)
        }
    })
    .expect("a picture at least one pixel each way")
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(
        AppConfig::new()
            .target_fps(30)
            .quit_key(KeyBinding::new(KeyCode::Char('q')))
            .quit_key(KeyBinding::ctrl(KeyCode::Char('c'))),
    )?;

    let seed = Rc::new(StdCell::new(1_u32));
    // The picture is only redrawn when the bars or the space for it change.
    let mut cached: Option<((u16, u16, u32), Picture)> = None;

    app.run(move |buf, _previous, events, ctx, _stats| {
        for event in ctx.unhandled(events) {
            if let AppEvent::Key(key) = event {
                if KeyBinding::from(' ').matches(key) {
                    seed.set(seed.get().wrapping_add(1));
                }
            }
        }

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        // Each half gets the inside of a bordered box: half the width less two columns, and
        // the height less the borders and the status row.
        let columns = (area.width / 2).saturating_sub(2).max(1);
        let rows = area.height.saturating_sub(3).max(1);
        let (px_w, px_h) = Graphics::cell_pixels().unwrap_or((8, 16));
        let key = (columns * px_w, rows * px_h, seed.get());
        let picture = match &cached {
            Some((at, picture)) if *at == key => picture.clone(),
            _ => {
                let picture = chart(key.0, key.1, &heights(key.2));
                cached = Some((key, picture.clone()));
                picture
            }
        };

        let status = if Graphics::supported() {
            format!(
                " pictures on · {}×{} px cells · {}×{} px chart · space shuffles, q quits ",
                px_w, px_h, key.0, key.1
            )
        } else {
            " this terminal cannot show pictures, so both sides are half blocks · q quits "
                .to_string()
        };

        Div::new()
            .child(
                Flex::row()
                    .flex(1)
                    .child(
                        Div::new()
                            .border(true)
                            .title(Text::new(" PixelMode::Graphics "))
                            .flex(1)
                            .child(
                                Image::picture(&picture)
                                    .mode(PixelMode::Graphics)
                                    .fit(Fit::Stretch)
                                    .flex(1),
                            ),
                    )
                    .child(
                        Div::new()
                            .border(true)
                            .title(Text::new(" PixelMode::HalfBlock "))
                            .flex(1)
                            .child(
                                Image::picture(&picture)
                                    .mode(PixelMode::HalfBlock)
                                    .fit(Fit::Stretch)
                                    .flex(1),
                            ),
                    ),
            )
            .child(Text::new(status).fg(Color::DarkGrey).height(1))
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })
}
