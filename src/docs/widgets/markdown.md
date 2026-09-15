# Markdown

[`Markdown`](crate::Markdown) renders [CommonMark](https://commonmark.org/) text:
headings, paragraphs, emphasis, code, lists, quotes, rules and links. It uses the
same parser as rustdoc, so a page reads the same here as on docs.rs. This guide is
written in it.

```rust
use auxior::{Area, Buffer, Canvas, Markdown};

let page = Markdown::new("# Shopping\n\n- milk\n- bread\n\n> Don't forget the eggs.");

let mut buf = Buffer::new(30, 6);
let area = Area::new_from_buffer(&buf);
page.render(&mut Canvas::new(&mut buf, area));

let row = |y: u16| {
    let text: String = (0..30).map(|x| buf.get(x, y).unwrap().ch).collect();
    text.trim_end().to_string()
};
assert_eq!(row(0), "Shopping");
assert_eq!(row(2), "• milk");
assert_eq!(row(3), "• bread");
assert_eq!(row(4), "");
assert_eq!(row(5), "│ Don't forget the eggs.");
```

## Options

| Method | Effect |
|---|---|
| [`new(source)`](crate::Markdown::new) | The Markdown text. |
| [`on_link(f)`](crate::Markdown::on_link) | Runs `f` with a link's destination when it's clicked. |
| [`headings(width)`](crate::Markdown::headings) | Lists the headings and the rows they land on. |
| `width`, `height`, `flex`, `x`, `y` | Layout. |

## What it renders

| Markdown | Rendered as |
|---|---|
| `# Heading` | Bold, underlined, cyan |
| `## Heading` | Bold, cyan |
| `###` to `######` | Bold |
| `*italic*`, `**bold**` | Italic, bold, or both |
| `` `code` `` | Yellow |
| Fenced or indented code | Green, indented two columns, not wrapped |
| `[text](url)` | Blue and underlined, clickable |
| `- item`, `1. item` | Bullets `•`, `◦`, `▪` by depth; numbers as written |
| `> quote` | A dark grey `│` bar on every row |
| `---` | A dark grey rule across the width |
| `![alt](image.png)` | `[alt]` in dark grey |

Tables, footnotes, strikethrough and task lists aren't enabled, so they show as
the plain text they're written in. HTML tags are left out, and the text between
them is kept.

## Layout rules

- Prose **wraps** to the width it's given, using the rules in
  [text and Unicode](../concepts/text.md#wrapping). Code blocks are
  **clipped** instead, because wrapping code changes its meaning.
- A blank row separates blocks: paragraphs, headings, code, quotes and whole
  lists.
- List items sit on consecutive rows, and nested lists sit directly under their
  item. Long items wrap under their own text rather than under the bullet.
- Tabs in code blocks become four spaces.

## Rust code blocks

As on docs.rs, lines starting with `# ` are **hidden** in Rust code blocks, so
examples can include setup lines that still compile as doctests. A line starting
with `##` shows as a single `#`. A code block counts as Rust when its fence has no
language, says `rust`, or has only rustdoc's own attributes such as `no_run` or
`ignore`.

## Links

Links are drawn blue and underlined. With [`on_link`](crate::Markdown::on_link),
clicking one runs your handler with its destination, which is how a documentation
browser follows links between pages. Clicks need mouse capture, and they work
inside a [`ScrollView`](crate::ScrollView).

## Headings and tables of contents

[`headings(width)`](crate::Markdown::headings) returns each heading's level, text,
and the row it starts on when drawn at `width`. Set a scroll state's offset to that
row to jump to a section:

```rust
use auxior::{Markdown, ScrollState};

let page = Markdown::new("# Intro\n\nHello.\n\n## Usage\n\nMore.");
let scroll = ScrollState::new();

let usage = page.headings(40).into_iter().find(|h| h.text == "Usage").unwrap();
assert_eq!(usage.row, 4);
scroll.set_offset(usage.row);
```

Rows depend on the width, because wrapping moves headings down. Ask for the width
the document is really drawn at. Inside a scroll view with a scrollbar, that's one
column less than the view.

## Size and performance

The document's height at a width is its number of rendered rows. It's parsed once
per `Markdown` value, and its layout is kept for the last width used, so measuring
and then drawing at the same width doesn't repeat the work.
