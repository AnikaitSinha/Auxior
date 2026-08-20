use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Flex, ScrollGraph, Text};
use crossterm::style::Color;

fn trim(history: &mut Vec<f32>, window: usize) {
    let keep = window.saturating_mul(2).max(window);
    if history.len() > keep {
        let drop = history.len() - keep;
        history.drain(..drop);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().target_fps(1))?;
    let mut cpu_history: Vec<f32> = Vec::new();
    let mut load_history: Vec<f32> = Vec::new();
    let mut tick = 0_u64;
    let window = 120;

    app.run(move |buf, _events| {
        tick = tick.wrapping_add(1);

        // Synthetic CPU-like signal: slow wave + bursts of noise.
        let t = tick as f32 * 0.08;
        let base = 35.0 + 25.0 * (t * 0.35).sin();
        let burst = 20.0 * ((t * 2.1).sin() * 0.5 + 0.5).powi(3);
        let noise = ((tick.wrapping_mul(1103515245).wrapping_add(12345) % 1000) as f32 / 1000.0
            - 0.5)
            * 12.0;
        let cpu = (base + burst + noise).clamp(0.0, 100.0);

        // Separate series with its own stable timeline (phase from tick, not buffer index).
        let load =
            (40.0 + 30.0 * (t * 0.55 + 1.2).sin() + 10.0 * (t * 1.7).cos()).clamp(0.0, 100.0);

        cpu_history.push(cpu);
        load_history.push(load);
        trim(&mut cpu_history, window);
        trim(&mut load_history, window);

        buf.fill(Cell::empty());
        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        let latest = cpu_history.last().copied().unwrap_or(0.0);

        Div::new()
            .border(true)
            .title(
                Text::new(format!("CPU  {latest:5.1}%"))
                    .fg(Color::Rgb {
                        r: 120,
                        g: 220,
                        b: 160,
                    })
                    .bold(true),
            )
            .padding(1)
            .child(
                Flex::column()
                    .gap(1)
                    .child(
                        Text::new(format!(
                            "window={window} samples across width · braille · q to quit"
                        ))
                        .fg(Color::DarkGrey),
                    )
                    .child(
                        ScrollGraph::new()
                            .window(window)
                            .range(0.0, 100.0)
                            .start_color(Color::Rgb {
                                r: 80,
                                g: 255,
                                b: 140,
                            })
                            .end_color(Color::Rgb {
                                r: 20,
                                g: 60,
                                b: 140,
                            })
                            .values(cpu_history.iter().copied())
                            .flex(1),
                    )
                    .child(
                        ScrollGraph::new()
                            .window(window)
                            .range(0.0, 100.0)
                            .height(4)
                            .start_color(Color::Rgb {
                                r: 255,
                                g: 180,
                                b: 60,
                            })
                            .end_color(Color::Rgb {
                                r: 180,
                                g: 40,
                                b: 40,
                            })
                            .values(load_history.iter().copied()),
                    ),
            )
            .render(&mut canvas);

        ControlFlow::Continue
    })?;

    Ok(())
}
