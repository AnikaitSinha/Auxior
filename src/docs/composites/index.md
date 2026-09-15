# Composite widgets

Ready-made widgets built from the base ones.

A composite is an ordinary [`Widget`](crate::Widget) whose drawing is done by other
widgets: a [`StatusBar`](crate::StatusBar) draws a [`Text`](crate::Text), a
[`Bar`](crate::Bar) and two more `Text`s side by side. They save you assembling
common pieces by hand, and they're good examples of how to build
[your own widgets](../concepts/custom-widgets.md).

- [List](list.md): lines of text stacked top to bottom
- [Table](table.md): rows of cells in columns, with an optional header
- [StatusBar](status-bar.md): a label, a progress bar and a value
- [SparklineGraph](sparkline-graph.md): a label, a sparkline and a value

Several composites have a **minimum size**, and draw nothing (or an error marker)
when given less. Each page says what the minimum is.
