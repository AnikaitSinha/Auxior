use auxior::{
    App, Area, Bar, Button, Canvas, Cell, ControlFlow, Div, Flex, List, StatusBar, Table, Text,
    Widget,
};
use crossterm::style::Color;

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;

    app.run(|buf, _previous, _events, ctx, _stats| {
        buf.fill(Cell::empty());

        let area = Area::new_from_buffer(buf);
        let mut canvas = Canvas::new(buf, area);

        Div::new()
            .border(true)
            .title(
                Text::new("Flex Demo")
                    .fg(crossterm::style::Color::Rgb {
                        r: 123,
                        g: 11,
                        b: 166,
                    })
                    .x(10)
                    .bold(true)
                    .underline(true)
                    .italic(true),
            )
            .border_button(
                Button::border_button("click")
                    .side(auxior::BorderSide::Top)
                    .align(auxior::BorderAlign::End),
            )
            .border_button(
                Button::border_button("click2")
                    .side(auxior::BorderSide::Top)
                    .align(auxior::BorderAlign::End),
            )
            .border_button(
                Button::border_button("click3")
                    .side(auxior::BorderSide::Right)
                    .align(auxior::BorderAlign::End),
            )
            .padding(1)
            .child(
                Flex::row()
                    .child(
                        Flex::column()
                            .gap(1)
                            .flex(1)
                            .child(Text::new("Header").fg(Color::Cyan))
                            .child(
                                Flex::row()
                                    .gap(5)
                                    .flex(1)
                                    .child(
                                        Div::new()
                                            .border(true)
                                            .title(Text::new("Left"))
                                            .flex(1)
                                            .child(Text::new("Panel A")),
                                    )
                                    .child(
                                        Div::new()
                                            .border(true)
                                            .title(Text::new("Right"))
                                            .flex(1)
                                            .child(Text::new("Panel B")),
                                    ),
                            )
                            .child(Text::new("Footer — q to quit"))
                            .child(Bar::new().width(6).fill(0.98).bg(Color::Black))
                            .child(
                                StatusBar::new()
                                    .fill(0.5)
                                    .bg(Color::Black)
                                    .label(Text::new("CPU")),
                            ),
                    )
                    .child(
                        Table::new()
                            .num_of_cols(2)
                            .min_len_per_col(vec![8, 6])
                            .header(true)
                            .header_labels(vec![Text::new("Name"), Text::new("Score")])
                            .add_row(vec![Text::new("Alice"), Text::new("98")])
                            .add_row(vec![Text::new("Bob"), Text::new("87")])
                            .width(20),
                    ),
            )
            .render_with_context(&mut canvas, ctx);

        ControlFlow::Continue
    })?;

    Ok(())
}
