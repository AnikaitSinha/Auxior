use std::cell::{OnceCell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use crossterm::style::Color;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use unicode_width::UnicodeWidthChar;

use super::text::wrap_ranges;
use crate::core::{MouseMap, text_width};
use crate::{Area, Canvas, Cell, LayoutOptions, Widget};

const HEADING: Color = Color::Cyan;
const INLINE_CODE: Color = Color::Yellow;
const CODE_BLOCK: Color = Color::Green;
const LINK: Color = Color::Blue;
const MUTED: Color = Color::DarkGrey;
// Rules fill the width, but an unmeasured width must not make them huge.
const MAX_RULE: u16 = 256;

// A heading in a rendered document, for a table of contents or for jumping to
// a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    // 1 for `#`, up to 6.
    pub level: u8,
    pub text: String,
    // The row the heading starts on, at the width it was measured for.
    pub row: u16,
}

type LinkHandler = Box<dyn FnMut(&str)>;

// Renders CommonMark text: headings, paragraphs, emphasis, inline and fenced
// code, lists, block quotes, rules, images (as their alt text) and links.
//
// Text wraps to the width it is given; code blocks do not wrap. Put it in a
// `ScrollView` for documents longer than the screen.
pub struct Markdown {
    source: String,
    layout: LayoutOptions,
    on_link: RefCell<Option<LinkHandler>>,
    document: OnceCell<Document>,
    // The last layout, reused while the width stays the same.
    laid_out: RefCell<Option<(u16, Rc<Layout>)>>,
}

impl Markdown {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            layout: LayoutOptions::default(),
            on_link: RefCell::new(None),
            document: OnceCell::new(),
            laid_out: RefCell::new(None),
        }
    }

    // Called with a link's destination when the user clicks it.
    pub fn on_link(self, handler: impl FnMut(&str) + 'static) -> Self {
        *self.on_link.borrow_mut() = Some(Box::new(handler));
        self
    }

    pub fn x(mut self, n: u16) -> Self {
        self.layout.x = Some(n);
        self
    }

    pub fn y(mut self, n: u16) -> Self {
        self.layout.y = Some(n);
        self
    }

    pub fn width(mut self, n: u16) -> Self {
        self.layout.width = Some(n);
        self
    }

    pub fn height(mut self, n: u16) -> Self {
        self.layout.height = Some(n);
        self
    }

    pub fn flex(mut self, n: u16) -> Self {
        self.layout.flex = Some(n);
        self
    }

    pub fn render(&self, canvas: &mut Canvas) {
        <Self as Widget>::render(self, canvas);
    }

    // The document's headings and the rows they land on when drawn `width`
    // columns wide.
    pub fn headings(&self, width: u16) -> Vec<Heading> {
        self.lay_out(width).headings.clone()
    }

    fn document(&self) -> &Document {
        self.document.get_or_init(|| parse(&self.source))
    }

    fn lay_out(&self, width: u16) -> Rc<Layout> {
        if let Some((cached_width, layout)) = &*self.laid_out.borrow() {
            if *cached_width == width {
                return Rc::clone(layout);
            }
        }

        let layout = Rc::new(lay_out(self.document(), width));
        *self.laid_out.borrow_mut() = Some((width, Rc::clone(&layout)));
        layout
    }
}

impl Widget for Markdown {
    fn render(&self, canvas: &mut Canvas) {
        let (width, height) = (canvas.width(), canvas.height());
        if width == 0 || height == 0 {
            return;
        }

        let layout = self.lay_out(width);
        let links = &self.document().links;
        let on_link = self
            .on_link
            .borrow_mut()
            .take()
            .map(|handler| Rc::new(RefCell::new(handler)));
        let origin = canvas.global_area();

        for (y, row) in layout.rows.iter().enumerate() {
            let Ok(y) = u16::try_from(y) else {
                break;
            };
            if y >= height {
                break;
            }

            let mut x = 0_u16;
            for span in row {
                let used = canvas.set_str(x, y, &span.text, span.style);
                if let (Some(link), Some(handler)) = (span.link, &on_link) {
                    let target = links[link].clone();
                    let handler = Rc::clone(handler);
                    let area = Area::new(
                        origin.x.saturating_add(x),
                        origin.y.saturating_add(y),
                        used,
                        1,
                    );
                    MouseMap::region(area, move || (*handler.borrow_mut())(&target));
                }
                x = x.saturating_add(text_width(&span.text));
            }
        }
    }

    fn layout(&self) -> &LayoutOptions {
        &self.layout
    }

    fn default_height(&self) -> u16 {
        row_count(&self.lay_out(u16::MAX))
    }

    fn default_width(&self) -> u16 {
        self.lay_out(u16::MAX)
            .rows
            .iter()
            .map(|row| spans_width(row))
            .max()
            .unwrap_or(1)
            .max(1)
    }

    fn height_for_width(&self, width: u16) -> u16 {
        row_count(&self.lay_out(width))
    }
}

fn row_count(layout: &Layout) -> u16 {
    layout.rows.len().clamp(1, usize::from(u16::MAX)) as u16
}

// --- Parsing: markdown to width-independent blocks ---------------------------

#[derive(Debug, Clone, PartialEq)]
struct Span {
    text: String,
    // Colors and attributes; `ch` is unused.
    style: Cell,
    // Index into `Document::links`.
    link: Option<usize>,
}

impl Span {
    fn new(text: impl Into<String>, style: Cell) -> Self {
        Self {
            text: text.into(),
            style,
            link: None,
        }
    }
}

enum Block {
    // Lines of text after a prefix of list markers, quote bars and indents.
    // `first_prefix` starts the block; `rest_prefix` starts every later row.
    Flow {
        first_prefix: Vec<Span>,
        rest_prefix: Vec<Span>,
        // Split at hard line breaks.
        lines: Vec<Vec<Span>>,
        // Code is clipped rather than wrapped.
        wrap: bool,
        heading: Option<u8>,
    },
    Blank {
        prefix: Vec<Span>,
    },
    Rule {
        prefix: Vec<Span>,
    },
}

struct Document {
    blocks: Vec<Block>,
    links: Vec<String>,
}

enum Container {
    Quote,
    Item {
        marker: String,
        // The marker has been drawn; later rows indent instead.
        used: bool,
        first_in_list: bool,
    },
}

struct Code {
    rust: bool,
    text: String,
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    links: Vec<String>,
    containers: Vec<Container>,
    // Each open list: the next number (bullet lists have none), and items seen.
    lists: Vec<(Option<u64>, u32)>,
    // Inline content of the block being read.
    lines: Vec<Vec<Span>>,
    bold: u32,
    italic: u32,
    image: u32,
    link: Option<usize>,
    heading: Option<u8>,
    code: Option<Code>,
    // A blank row separates the next block from the previous one.
    gap: bool,
}

fn parse(source: &str) -> Document {
    let mut builder = Builder::default();

    for event in Parser::new(source) {
        match event {
            Event::Start(tag) => builder.start(tag),
            Event::End(tag) => builder.end(tag),
            Event::Text(text) => builder.text(&text),
            Event::Code(code) => {
                let mut style = builder.style();
                if builder.link.is_none() {
                    style.fg = INLINE_CODE;
                }
                builder.push(&code, style);
            }
            Event::SoftBreak => builder.text(" "),
            Event::HardBreak => builder.lines.push(Vec::new()),
            Event::Rule => {
                builder.flush();
                let (prefix, _) = builder.begin_block();
                builder.blocks.push(Block::Rule { prefix });
            }
            // HTML, math and footnotes are left out.
            _ => {}
        }
    }

    builder.flush();
    Document {
        blocks: builder.blocks,
        links: builder.links,
    }
}

impl Builder {
    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.flush(),
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.containers.push(Container::Quote);
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                self.code = Some(Code {
                    rust: is_rust(&kind),
                    text: String::new(),
                });
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push((start, 0));
            }
            Tag::Item => {
                self.flush();
                let depth = self.lists.len();
                let Some((number, items)) = self.lists.last_mut() else {
                    return;
                };
                let marker = match number {
                    Some(n) => {
                        *n += 1;
                        format!("{}. ", *n - 1)
                    }
                    None => format!("{} ", ["•", "◦", "▪"][(depth - 1) % 3]),
                };
                *items += 1;
                let first_in_list = *items == 1;
                self.containers.push(Container::Item {
                    marker,
                    used: false,
                    first_in_list,
                });
            }
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Link { dest_url, .. } => {
                self.links.push(dest_url.to_string());
                self.link = Some(self.links.len() - 1);
            }
            Tag::Image { .. } => {
                self.image += 1;
                self.push("[", self.style());
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                self.flush();
                self.heading = None;
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.containers.pop();
                self.gap = true;
            }
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.emit_code(code);
                }
            }
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                self.gap = true;
            }
            TagEnd::Item => {
                self.flush();
                self.containers.pop();
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Link => self.link = None,
            TagEnd::Image => {
                self.push("]", self.style());
                self.image = self.image.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn style(&self) -> Cell {
        let mut style = Cell::default();
        if let Some(level) = self.heading {
            style.b = true;
            if level <= 2 {
                style.fg = HEADING;
            }
            style.u = level == 1;
        }
        if self.bold > 0 {
            style.b = true;
        }
        if self.italic > 0 {
            style.i = true;
        }
        if self.image > 0 {
            style.fg = MUTED;
        }
        if self.link.is_some() {
            style.fg = LINK;
            style.u = true;
        }
        style
    }

    fn text(&mut self, text: &str) {
        if let Some(code) = &mut self.code {
            code.text.push_str(text);
            return;
        }
        self.push(text, self.style());
    }

    fn push(&mut self, text: &str, style: Cell) {
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        let mut span = Span::new(text, style);
        span.link = self.link;
        if let Some(line) = self.lines.last_mut() {
            line.push(span);
        }
    }

    // Ends the block of inline content being read, if it has any.
    fn flush(&mut self) {
        let lines = std::mem::take(&mut self.lines);
        if lines.iter().flatten().all(|span| span.text.is_empty()) {
            return;
        }

        let heading = self.heading;
        let (first_prefix, rest_prefix) = self.begin_block();
        self.blocks.push(Block::Flow {
            first_prefix,
            rest_prefix,
            lines,
            wrap: true,
            heading,
        });
    }

    fn emit_code(&mut self, code: Code) {
        let style = Cell::with_fg(' ', CODE_BLOCK);
        let lines: Vec<Vec<Span>> = code
            .text
            .lines()
            .filter_map(|line| {
                if code.rust {
                    shown_rust_line(line)
                } else {
                    Some(line.to_string())
                }
            })
            // Tabs have no width of their own and would vanish.
            .map(|line| vec![Span::new(line.replace('\t', "    "), style)])
            .collect();
        if lines.is_empty() {
            return;
        }

        let (mut first_prefix, mut rest_prefix) = self.begin_block();
        // Code sits indented from the text around it.
        first_prefix.push(Span::new("  ", Cell::default()));
        rest_prefix.push(Span::new("  ", Cell::default()));
        self.blocks.push(Block::Flow {
            first_prefix,
            rest_prefix,
            lines,
            wrap: false,
            heading: None,
        });
    }

    // Adds the blank row before a new block if one is due, and returns the
    // block's first-row and later-row prefixes.
    fn begin_block(&mut self) -> (Vec<Span>, Vec<Span>) {
        // Items in a list sit on consecutive rows, and so does a nested list
        // under its item's text. A list still gets a gap from what precedes it.
        let nested = self.containers.len() > 1
            && self.containers[..self.containers.len() - 1]
                .iter()
                .any(|container| matches!(container, Container::Item { .. }));
        let starts_item = matches!(
            self.containers.last(),
            Some(Container::Item { used: false, first_in_list, .. }) if !first_in_list || nested
        );
        if self.gap && !starts_item {
            let prefix = self.prefix(false);
            self.blocks.push(Block::Blank { prefix });
        }

        let first = self.prefix(true);
        let rest = self.prefix(false);
        for container in &mut self.containers {
            if let Container::Item { used, .. } = container {
                *used = true;
            }
        }
        self.gap = true;
        (first, rest)
    }

    fn prefix(&self, first_row: bool) -> Vec<Span> {
        self.containers
            .iter()
            .map(|container| match container {
                Container::Quote => Span::new("│ ", Cell::with_fg(' ', MUTED)),
                Container::Item { marker, used, .. } if first_row && !used => {
                    Span::new(marker.as_str(), Cell::default())
                }
                Container::Item { marker, .. } => {
                    Span::new(" ".repeat(usize::from(text_width(marker))), Cell::default())
                }
            })
            .collect()
    }
}

// Rustdoc treats a fence with no language, or with only its own attributes, as
// Rust.
fn is_rust(kind: &CodeBlockKind) -> bool {
    match kind {
        CodeBlockKind::Indented => true,
        CodeBlockKind::Fenced(info) => info
            .split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter(|token| !token.is_empty())
            .all(|token| {
                matches!(
                    token,
                    "rust" | "ignore" | "no_run" | "should_panic" | "compile_fail" | "test_harness"
                ) || token.starts_with("edition")
            }),
    }
}

// Rustdoc hides lines starting with `# ` in Rust examples (they still compile
// as doctests) and shows `##` as a literal `#`. `None` means hidden.
fn shown_rust_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if let Some(rest) = trimmed.strip_prefix("##") {
        return Some(format!("{indent}#{rest}"));
    }
    if trimmed == "#" || trimmed.starts_with("# ") || trimmed.starts_with("#\t") {
        return None;
    }
    Some(line.to_string())
}

// --- Layout: blocks to rows at a width ----------------------------------------

#[derive(Default)]
struct Layout {
    rows: Vec<Vec<Span>>,
    headings: Vec<Heading>,
}

fn lay_out(document: &Document, width: u16) -> Layout {
    let mut layout = Layout::default();

    for block in &document.blocks {
        match block {
            Block::Blank { prefix } => layout.rows.push(prefix.clone()),
            Block::Rule { prefix } => {
                let length = width.saturating_sub(spans_width(prefix)).min(MAX_RULE);
                let mut row = prefix.clone();
                row.push(Span::new(
                    "─".repeat(usize::from(length)),
                    Cell::with_fg(' ', MUTED),
                ));
                layout.rows.push(row);
            }
            Block::Flow {
                first_prefix,
                rest_prefix,
                lines,
                wrap,
                heading,
            } => {
                let start = layout.rows.len();
                // Markers and their indents are the same width.
                let available = width.saturating_sub(spans_width(rest_prefix));

                for line in lines {
                    for piece in split_line(line, available, *wrap) {
                        let prefix = if layout.rows.len() == start {
                            first_prefix
                        } else {
                            rest_prefix
                        };
                        let mut row = prefix.clone();
                        row.extend(piece);
                        layout.rows.push(row);
                    }
                }

                if let Some(level) = heading {
                    layout.headings.push(Heading {
                        level: *level,
                        text: lines
                            .iter()
                            .map(|line| {
                                line.iter()
                                    .map(|span| span.text.as_str())
                                    .collect::<String>()
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                        row: u16::try_from(start).unwrap_or(u16::MAX),
                    });
                }
            }
        }
    }

    layout
}

// One line of styled text as rows no wider than `width`: wrapped between words
// like `Text`, or clipped.
fn split_line(line: &[Span], width: u16, wrap: bool) -> Vec<Vec<Span>> {
    let plain: String = line.iter().map(|span| span.text.as_str()).collect();
    let ranges = if wrap {
        wrap_ranges(&plain, width)
    } else {
        std::iter::once(0..clip_len(&plain, width)).collect()
    };
    ranges
        .into_iter()
        .map(|range| slice_spans(line, range))
        .collect()
}

// The spans covering `range` of the line's concatenated text.
fn slice_spans(line: &[Span], range: Range<usize>) -> Vec<Span> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for span in line {
        let end = start + span.text.len();
        let (from, to) = (range.start.max(start), range.end.min(end));
        if from < to {
            pieces.push(Span {
                text: span.text[from - start..to - start].to_string(),
                ..span.clone()
            });
        }
        start = end;
    }
    pieces
}

// Bytes of `text` that fit in `width` columns.
fn clip_len(text: &str, width: u16) -> usize {
    let mut used = 0;
    for (index, ch) in text.char_indices() {
        used += ch.width().unwrap_or(0);
        if used > usize::from(width) {
            return index;
        }
    }
    text.len()
}

fn spans_width(spans: &[Span]) -> u16 {
    spans.iter().fold(0_u16, |total, span| {
        total.saturating_add(text_width(&span.text))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Buffer;
    use crate::core::{AppEvent, begin_frame, dispatch_input};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn rows(source: &str, width: u16) -> Vec<String> {
        Markdown::new(source)
            .lay_out(width)
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn render(markdown: &Markdown, width: u16, height: u16) -> Buffer {
        let mut buf = Buffer::new(width, height);
        markdown.render(&mut Canvas::new(&mut buf, Area::new(0, 0, width, height)));
        buf
    }

    fn cell(buf: &Buffer, x: u16, y: u16) -> Cell {
        *buf.get(x, y).unwrap()
    }

    #[test]
    fn paragraphs_wrap_and_are_separated_by_a_blank_row() {
        assert_eq!(
            rows("one two three\n\nfour", 7),
            ["one two", "three", "", "four"]
        );
    }

    #[test]
    fn soft_breaks_join_and_hard_breaks_split() {
        assert_eq!(rows("a\nb", 10), ["a b"]);
        assert_eq!(rows("a  \nb", 10), ["a", "b"]);
        assert_eq!(rows("a\\\nb", 10), ["a", "b"]);
    }

    #[test]
    fn headings_are_styled_and_listed() {
        let markdown = Markdown::new("# Title\n\ntext\n\n## Sub\n\n### Small");

        assert_eq!(
            rows("# Title\n\ntext\n\n## Sub\n\n### Small", 20),
            ["Title", "", "text", "", "Sub", "", "Small"]
        );
        assert_eq!(
            markdown.headings(20),
            [
                Heading {
                    level: 1,
                    text: "Title".into(),
                    row: 0
                },
                Heading {
                    level: 2,
                    text: "Sub".into(),
                    row: 4
                },
                Heading {
                    level: 3,
                    text: "Small".into(),
                    row: 6
                },
            ]
        );

        let buf = render(&markdown, 20, 7);
        let (title, sub, small) = (cell(&buf, 0, 0), cell(&buf, 0, 4), cell(&buf, 0, 6));
        assert!(title.b && title.u && title.fg == HEADING);
        assert!(sub.b && !sub.u && sub.fg == HEADING);
        assert!(small.b && small.fg == Color::Reset);
    }

    #[test]
    fn heading_rows_follow_wrapping() {
        let markdown = Markdown::new("one two three\n\n## Next");
        assert_eq!(markdown.headings(20)[0].row, 2);
        assert_eq!(markdown.headings(7)[0].row, 3);
    }

    #[test]
    fn inline_styles() {
        let markdown = Markdown::new("*i* **b** `c` [l](x)");
        assert_eq!(rows("*i* **b** `c` [l](x)", 20), ["i b c l"]);

        let buf = render(&markdown, 20, 1);
        assert!(cell(&buf, 0, 0).i);
        assert!(cell(&buf, 2, 0).b);
        assert_eq!(cell(&buf, 4, 0).fg, INLINE_CODE);
        let link = cell(&buf, 6, 0);
        assert!(link.u && link.fg == LINK);
        assert!(!cell(&buf, 1, 0).i, "the space after is plain");
    }

    #[test]
    fn nested_emphasis_combines() {
        let buf = render(&Markdown::new("***x***"), 5, 1);
        let x = cell(&buf, 0, 0);
        assert!(x.b && x.i);
    }

    #[test]
    fn lists_are_tight_but_set_apart_from_other_blocks() {
        assert_eq!(
            rows("before\n\n- a\n- b\n\nafter", 20),
            ["before", "", "• a", "• b", "", "after"]
        );
    }

    #[test]
    fn ordered_lists_keep_their_numbers() {
        assert_eq!(rows("3. x\n4. y", 20), ["3. x", "4. y"]);
    }

    #[test]
    fn nested_lists_indent_and_change_bullets() {
        assert_eq!(
            rows("- a\n  - b\n    - c\n- d", 20),
            ["• a", "  ◦ b", "    ▪ c", "• d"]
        );
    }

    #[test]
    fn list_items_wrap_under_their_text() {
        assert_eq!(rows("- aaa bbb", 5), ["• aaa", "  bbb"]);
        assert_eq!(rows("10. aaa bbb", 7), ["10. aaa", "    bbb"]);
    }

    #[test]
    fn paragraphs_within_an_item_are_separated() {
        assert_eq!(
            rows("- a\n\n  second\n- b", 20),
            ["• a", "", "  second", "• b"]
        );
    }

    #[test]
    fn block_quotes_mark_every_row() {
        assert_eq!(
            rows("> aaa bbb\n>\n> ccc", 5),
            ["│ aaa", "│ bbb", "│", "│ ccc"]
        );
    }

    #[test]
    fn code_blocks_are_indented_clipped_and_colored() {
        let source = "```\nlet long_line = 1;\n```";
        assert_eq!(rows(source, 10), ["  let long"]);
        assert_eq!(
            cell(&render(&Markdown::new(source), 10, 1), 2, 0).fg,
            CODE_BLOCK
        );
    }

    #[test]
    fn rustdoc_hidden_lines_are_hidden_in_rust_code() {
        let source = "```rust\n# use std::fmt;\nlet a = 1;\n    # indented\n## shown\n#\n```";
        assert_eq!(rows(source, 40), ["  let a = 1;", "  # shown"]);
        assert_eq!(rows("```rust,ignore\n# hidden\nx\n```", 40), ["  x"]);
        assert_eq!(rows("    # indented code is rust\n    y", 40), ["  y"]);
    }

    #[test]
    fn other_languages_keep_every_line() {
        assert_eq!(rows("```text\n# kept\n```", 40), ["  # kept"]);
        assert_eq!(rows("```sh\n# comment\n```", 40), ["  # comment"]);
    }

    #[test]
    fn tabs_in_code_become_spaces() {
        assert_eq!(rows("```text\n\tx\n```", 20), ["      x"]);
    }

    #[test]
    fn rules_span_the_width() {
        assert_eq!(rows("a\n\n---\n\nb", 5), ["a", "", "─────", "", "b"]);
    }

    #[test]
    fn images_show_their_alt_text() {
        assert_eq!(rows("![a cat](cat.png)", 20), ["[a cat]"]);
    }

    #[test]
    fn html_tags_are_left_out() {
        assert_eq!(rows("a <b>bold</b> c", 20), ["a bold c"]);
    }

    #[test]
    fn wide_characters_wrap_by_columns() {
        assert_eq!(rows("日本語 テキスト", 8), ["日本語", "テキスト"]);
    }

    #[test]
    fn measures_by_rows() {
        let markdown = Markdown::new("one two three");
        assert_eq!(markdown.height_for_width(7), 2);
        assert_eq!(markdown.height_for_width(20), 1);
        assert_eq!(markdown.height_for_width(7), 2, "cache follows the width");
        assert_eq!(markdown.default_height(), 1);
        assert_eq!(markdown.default_width(), 13);

        assert_eq!(Markdown::new("").height_for_width(10), 1);
    }

    #[test]
    fn clicking_a_link_reports_its_destination() {
        crate::Focus::clear();
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut buf = Buffer::new(20, 1);
        // "see docs or api": "docs" at columns 4..8, "api" at 12..15.
        let draw = |events: &[AppEvent], buf: &mut Buffer| {
            dispatch_input(events);
            begin_frame();
            let record = seen.clone();
            Markdown::new("see [docs](page.md) or [api](api.md)")
                .on_link(move |target| record.borrow_mut().push(target.to_string()))
                .render(&mut Canvas::new(buf, Area::new(0, 0, 20, 1)));
        };
        let click = |column| {
            AppEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row: 0,
                modifiers: KeyModifiers::NONE,
            })
        };

        draw(&[], &mut buf);
        draw(&[click(5)], &mut buf);
        draw(&[click(13)], &mut buf);
        draw(&[click(0), click(10)], &mut buf);

        assert_eq!(*seen.borrow(), ["page.md", "api.md"]);
    }
}
