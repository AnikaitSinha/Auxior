use crate::{LayoutOptions, Text};

pub struct Table {
    layout: LayoutOptions,
    rows: Vec<Vec<Text>>,
    min_height: u16,
    num_of_cols: u16,
    min_len_per_col: Vec<u16>,
    header: bool,
    header_labels: Vec<Text>,
}

impl Table {
    pub fn new() -> Self {
        Table {
            layout: LayoutOptions::default(),
            rows: Vec::new(),
            min_height: 2,
            num_of_cols: 0,
            min_len_per_col: Vec::new(),
            header: false,
            header_labels: Vec::new(),
        }
    }

    pub fn num_of_cols(mut self, num_of_cols: u16) -> Self {
        self.num_of_cols = num_of_cols;
        self
    }

    pub fn add_row(mut self, row: Vec<Text>) -> Self {
        self.rows.push(row);
        self
    }

    pub fn add_row_at(mut self, index: u16, row: Vec<Text>) -> Self {
        self.rows.insert(index as usize, row);
        self
    }

    pub fn header(mut self, enable: bool) -> Self {
        self.header = enable;
        self
    }

    pub fn header_labels(mut self, labels: Vec<Text>) -> Self {
        self.header_labels = labels;
        self
    }

    pub fn min_height(mut self, min_height: u16) -> Self {
        self.min_height = min_height;
        self
    }

    pub fn min_len_per_col(mut self, min_len_per_col: Vec<u16>) -> Self {
        self.min_len_per_col = min_len_per_col;
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
}
