//! A moving picture, drawn three different ways.
//!
//! The frames here are generated rather than decoded, so the example runs
//! without any image file: a coloured pattern that rolls across the picture.
//!
//! Keys: 1, 2 and 3 switch how pixels become characters, f switches how the
//! picture is fitted, space pauses, - and + change the speed, q quits.

use std::cell::Cell as StdCell;
use std::rc::Rc;

use auxior::{
    Animation, AnimationState, App, AppConfig, AppEvent, Area, Canvas, Cell, Color, ControlFlow,
    Div, Fit, Image, KeyBinding, KeyCode, Picture, PixelMode, Text, Widget,
};

const FRAMES: u16 = 30;
const SIZE: u16 = 120;

/// One frame of a rolling pattern: rings of colour moving outwards.
fn frame(step: u16) -> Picture {
    let middle = f32::from(SIZE) / 2.0;
    let phase = f32::from(step) / f32::from(FRAMES);

    Picture::from_fn(SIZE, SIZE, |x, y| {
        let dx = f32::from(x) - middle;
        let dy = f32::from(y) - middle;
        let distance = (dx * dx + dy * dy).sqrt() / middle;
        let wave = ((distance * 6.0 - phase * std::f32::consts::TAU).sin() + 1.0) / 2.0;

        let angle = dy.atan2(dx) / std::f32::consts::PI * 0.5 + 0.5;
        (
            (wave * 255.0) as u8,
            (angle * 200.0) as u8,
            ((1.0 - wave) * 255.0) as u8,
        )
    })
    .expect("a square picture with pixels for every position")
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(
        AppConfig::new()
            .target_fps(30)
            .quit_key(KeyBinding::new(KeyCode::Char('q')))
            .quit_key(KeyBinding::ctrl(KeyCode::Char('c'))),
    )?;

    // Decoded once, held for the whole run: only the clock changes per frame.
    let animation = Animation::with_frame_rate((0..FRAMES).map(frame), 15);
    let playing = AnimationState::new();

    let mode = Rc::new(StdCell::new(PixelMode::HalfBlock));
    let fit = Rc::new(StdCell::new(Fit::Contain));

    app.run(move |buf, _previous, events, ctx, stats| {
        for event in events {
            let AppEvent::Key(key) = event else { continue };
            match key.code {
                KeyCode::Char('1') => mode.set(PixelMode::HalfBlock),
                KeyCode::Char('2') => mode.set(PixelMode::Braille),
                KeyCode::Char('3') => mode.set(PixelMode::Ascii),
                KeyCode::Char('f') => fit.set(match fit.get() {
                    Fit::Contain => Fit::Cover,
                    Fit::Cover => Fit::Stretch,
                    Fit::Stretch => Fit::Contain,
                }),
                KeyCode::Char(' ') => playing.toggle(),
                KeyCode::Char('-') => playing.set_speed((playing.speed() - 0.25).max(0.25)),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    playing.set_speed(playing.speed() + 0.25)
                }
                _ => {}
            }
        }

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let status = format!(
            " {:?}   {:?}   {}x speed{}   {} cells written ",
            mode.get(),
            fit.get(),
            playing.speed(),
            if playing.is_paused() { "   paused" } else { "" },
            stats.flushed_cells,
        );

        Div::new()
            .border(true)
            .title(Text::new(
                " animation — 1/2/3 mode, f fit, space pause, -/+ speed, q quit ",
            ))
            .child(
                Image::animation(&animation, &playing)
                    .mode(mode.get())
                    .fit(fit.get())
                    .flex(1),
            )
            .child(Text::new(status).fg(Color::DarkGrey).height(1))
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })
}
