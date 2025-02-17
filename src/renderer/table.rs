use pulldown_cmark::Alignment;
use std::fmt::Write;

#[derive(Debug)]
pub struct TableCell {
    pub content: String,
    pub alignment: Option<Alignment>,
}

#[derive(Debug)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug)]
pub struct Table {
    headers: TableRow,
    rows: Vec<TableRow>,
    column_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<(String, Option<Alignment>)>) -> Self {
        let header_row = TableRow {
            cells: headers.into_iter()
                .map(|(content, alignment)| TableCell { content, alignment })
                .collect(),
        };
        let num_columns = header_row.cells.len();
        Table {
            headers: header_row,
            rows: Vec::new(),
            column_widths: vec![0; num_columns],
        }
    }

    pub fn add_row(&mut self, cells: Vec<String>) {
        let row = TableRow {
            cells: cells.into_iter()
                .enumerate()
                .map(|(i, content)| {
                    let alignment = self.headers.cells.get(i)
                        .and_then(|header| header.alignment);
                    TableCell { content, alignment }
                })
                .collect(),
        };
        self.rows.push(row);
    }

    pub fn calculate_column_widths(&mut self, max_width: usize) {
        let num_columns = self.headers.cells.len();
        let min_column_width = 3;
        let padding = 2; // Space on each side of content
        let borders = 1 + num_columns + 1; // Left border + column separators + right border
        let total_padding = padding * 2 * num_columns; // Padding for each column
        let available_width = max_width.saturating_sub(borders + total_padding);

        // Calculate initial width per column
        let base_width = (available_width / num_columns).max(min_column_width);
        self.column_widths = vec![base_width; num_columns];

        // First pass: Calculate required width for each column
        for (i, cell) in self.headers.cells.iter().enumerate() {
            let content_width = cell.content.chars().count();
            self.column_widths[i] = self.column_widths[i].max(content_width);
        }

        for row in &self.rows {
            for (i, cell) in row.cells.iter().enumerate() {
                if i < self.column_widths.len() {
                    let content_width = cell.content.chars().count();
                    self.column_widths[i] = self.column_widths[i].max(content_width);
                }
            }
        }

        // Second pass: Scale down if total width exceeds available width
        let total_content_width: usize = self.column_widths.iter().sum();
        if total_content_width > available_width {
            let scale_factor = available_width as f64 / total_content_width as f64;
            for width in self.column_widths.iter_mut() {
                *width = (*width as f64 * scale_factor).max(min_column_width as f64) as usize;
            }
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::with_capacity(
            (self.rows.len() + 3) * // Number of rows plus headers and borders
            (self.column_widths.iter().sum::<usize>() + // Total content width
             self.column_widths.len() * 4 + 4) // Padding and borders
        );

        // Top border
        output.push_str("  ┌");
        for (i, &width) in self.column_widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < self.column_widths.len() - 1 {
                output.push('┬');
            }
        }
        output.push_str("┐\n");

        // Headers
        self.render_row(&mut output, &self.headers);

        // Separator after headers
        output.push_str("  ├");
        for (i, (&width, cell)) in self.column_widths.iter().zip(&self.headers.cells).enumerate() {
            match cell.alignment {
                Some(Alignment::Left) => write!(output, ":{0}─", "─".repeat(width + 1)).unwrap(),
                Some(Alignment::Right) => write!(output, "{0}─:", "─".repeat(width + 1)).unwrap(),
                Some(Alignment::Center) => write!(output, ":{0}:", "─".repeat(width)).unwrap(),
                Some(Alignment::None) | None => write!(output, "{0}", "─".repeat(width + 2)).unwrap(),
            }
            if i < self.column_widths.len() - 1 {
                output.push('┼');
            }
        }
        output.push_str("┤\n");

        // Data rows
        for (i, row) in self.rows.iter().enumerate() {
            self.render_row(&mut output, row);
            if i < self.rows.len() - 1 {
                output.push_str("  ├");
                for (j, &width) in self.column_widths.iter().enumerate() {
                    output.push_str(&"─".repeat(width + 2));
                    if j < self.column_widths.len() - 1 {
                        output.push('┼');
                    }
                }
                output.push_str("┤\n");
            }
        }

        // Bottom border
        output.push_str("  └");
        for (i, &width) in self.column_widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < self.column_widths.len() - 1 {
                output.push('┴');
            }
        }
        output.push_str("┘\n");

        output
    }

    fn render_row(&self, output: &mut String, row: &TableRow) {
        output.push_str("  │ ");
        for (i, (cell, &width)) in row.cells.iter().zip(&self.column_widths).enumerate() {
            let formatted = match cell.alignment {
                Some(Alignment::Left) | None => format!("{:<width$}", cell.content, width = width),
                Some(Alignment::Right) => format!("{:>width$}", cell.content, width = width),
                Some(Alignment::Center) => {
                    let spaces = width - cell.content.chars().count();
                    let left_pad = spaces / 2;
                    let right_pad = spaces - left_pad;
                    format!("{}{}{}", " ".repeat(left_pad), cell.content, " ".repeat(right_pad))
                },
                Some(Alignment::None) => format!("{:<width$}", cell.content, width = width),
            };
            output.push_str(&formatted);
            if i < self.column_widths.len() - 1 {
                output.push_str(" │ ");
            }
        }
        output.push_str(" │\n");
    }
} 