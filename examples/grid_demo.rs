use auxior::Color;
use auxior::{App, AppConfig, Area, Canvas, Cell, ControlFlow, Div, Grid, StatusBar, Text, Widget};

const GRID_ROWS: u16 = 3;
const ROW_GAP: u16 = 1;
const COL_GAP: u16 = 2;

fn grid_cell(row_h: u16, child: impl Widget + 'static) -> Div {
    Div::new().border(true).height(row_h).flex(1).child(child)
}

fn main() -> std::io::Result<()> {
    let mut app = App::with_config(AppConfig::new().default_quit_keys())?;

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        // Bordered cells need at least 3 rows; give each grid row enough height to show a frame.
        let row_h = 6u16;
        let grid_h = row_h
            .saturating_mul(GRID_ROWS)
            .saturating_add(ROW_GAP.saturating_mul(GRID_ROWS.saturating_sub(1)));

        Div::new()
            .border(true)
            .title(
                Text::new("Grid Demo")
                    .fg(Color::Rgb {
                        r: 123,
                        g: 11,
                        b: 166,
                    })
                    .x(10)
                    .bold(true),
            )
            .padding(1)
            .child(Text::new(
                "3-column grid — col_gap(2), row_gap(1), flex columns",
            ))
            .child(
                Grid::new()
                    .cols(3)
                    .col_gap(COL_GAP)
                    .row_gap(ROW_GAP)
                    .height(grid_h)
                    .child(grid_cell(row_h, Text::new("CPU").fg(Color::Yellow)))
                    .child(grid_cell(row_h, Text::new("RAM").fg(Color::Yellow)))
                    .child(grid_cell(row_h, Text::new("Disk").fg(Color::Yellow)))
                    .child(grid_cell(
                        row_h,
                        StatusBar::new()
                            .label(Text::new("CPU"))
                            .fill(0.72)
                            .bg(Color::Black),
                    ))
                    .child(grid_cell(
                        row_h,
                        StatusBar::new()
                            .label(Text::new("RAM"))
                            .fill(0.45)
                            .bg(Color::Black),
                    ))
                    .child(grid_cell(
                        row_h,
                        StatusBar::new()
                            .label(Text::new("Disk"))
                            .fill(0.18)
                            .bg(Color::Black),
                    ))
                    .child(
                        grid_cell(row_h, Text::new("Row-major cell 0")).title(Text::new("Panel A")),
                    )
                    .child(
                        grid_cell(row_h, Text::new("Row-major cell 1")).title(Text::new("Panel B")),
                    )
                    .child(
                        grid_cell(row_h, Text::new("Row-major cell 2")).title(Text::new("Panel C")),
                    ),
            )
            .child(Text::new("q to quit"))
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
