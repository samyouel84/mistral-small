use pulldown_cmark::Alignment;

#[derive(Debug)]
struct TableCell {
    content: String,
    alignment: Option<Alignment>,
}

#[derive(Debug)]
struct TableRow {
    cells: Vec<TableCell>,
}

#[derive(Debug)]
pub(crate) struct Table {
    headers: TableRow,
    rows: Vec<TableRow>,
    column_widths: Vec<usize>,
}

impl Table {
    pub(crate) fn new(headers: Vec<(String, Option<Alignment>)>) -> Self {
        let header_row = TableRow {
            cells: headers
                .into_iter()
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

    pub(crate) fn add_row(&mut self, cells: Vec<String>) {
        let row = TableRow {
            cells: cells
                .into_iter()
                .enumerate()
                .map(|(i, content)| {
                    let alignment = self
                        .headers
                        .cells
                        .get(i)
                        .and_then(|header| header.alignment);
                    TableCell { content, alignment }
                })
                .collect(),
        };
        self.rows.push(row);
    }

    pub(crate) fn calculate_column_widths(&mut self, max_width: usize) {
        let num_columns = self.headers.cells.len();
        let min_column_width = 15;
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

        // Second pass: Distribute remaining space proportionally
        let total_content_width: usize = self.column_widths.iter().sum();
        if total_content_width > available_width {
            // Scale down if content is too wide
            let scale_factor = available_width as f64 / total_content_width as f64;
            for width in self.column_widths.iter_mut() {
                *width = (*width as f64 * scale_factor).max(min_column_width as f64) as usize;
            }
        } else {
            // Distribute extra space proportionally
            let extra_space = available_width - total_content_width;
            let base_extra = extra_space / num_columns;
            for width in self.column_widths.iter_mut() {
                *width += base_extra;
            }
        }
    }

    pub(crate) fn render(&self) -> String {
        let mut output = String::new();
        output.push('\n');

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
        self.render_row(&mut output, &self.headers, true);

        // Separator after headers
        output.push_str("  ├");
        for (i, (&width, cell)) in self
            .column_widths
            .iter()
            .zip(&self.headers.cells)
            .enumerate()
        {
            match cell.alignment {
                Some(Alignment::Left) => {
                    output.push(':');
                    output.push_str(&"─".repeat(width + 1));
                }
                Some(Alignment::Right) => {
                    output.push_str(&"─".repeat(width + 1));
                    output.push(':');
                }
                Some(Alignment::Center) => {
                    output.push(':');
                    output.push_str(&"─".repeat(width));
                    output.push(':');
                }
                Some(Alignment::None) | None => {
                    output.push_str(&"─".repeat(width + 2));
                }
            }
            if i < self.column_widths.len() - 1 {
                output.push('┼');
            }
        }
        output.push_str("┤\n");

        // Rows with separators between them
        for (i, row) in self.rows.iter().enumerate() {
            self.render_row(&mut output, row, false);

            // Add separator between rows (except for the last row)
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

    fn render_row(&self, output: &mut String, row: &TableRow, _is_header: bool) {
        // First, wrap the content of each cell
        let wrapped_contents: Vec<Vec<String>> = row
            .cells
            .iter()
            .zip(&self.column_widths)
            .map(|(cell, &width)| {
                let words = cell.content.split_whitespace().collect::<Vec<_>>();
                let mut lines = Vec::new();
                let mut current_line = String::new();

                for word in words {
                    let test_line = if current_line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{} {}", current_line, word)
                    };

                    if test_line.chars().count() <= width {
                        current_line = test_line;
                    } else {
                        if !current_line.is_empty() {
                            lines.push(current_line);
                        }
                        current_line = word.to_string();
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                if lines.is_empty() {
                    lines.push(String::new());
                }
                lines
            })
            .collect();

        // Find the maximum number of lines in any cell
        let max_lines = wrapped_contents
            .iter()
            .map(|lines| lines.len())
            .max()
            .unwrap_or(1);

        // Render each line of the row
        for line_idx in 0..max_lines {
            output.push_str("  │ ");
            for (i, (cell, wrapped_content)) in row.cells.iter().zip(&wrapped_contents).enumerate()
            {
                let content = wrapped_content.get(line_idx).map_or("", |s| s);

                let formatted = match cell.alignment {
                    Some(Alignment::Left) | None => {
                        format!("{:<width$}", content, width = self.column_widths[i])
                    }
                    Some(Alignment::Right) => {
                        format!("{:>width$}", content, width = self.column_widths[i])
                    }
                    Some(Alignment::Center) => {
                        let spaces = self.column_widths[i] - content.chars().count();
                        let left_pad = spaces / 2;
                        let right_pad = spaces - left_pad;
                        format!(
                            "{}{}{}",
                            " ".repeat(left_pad),
                            content,
                            " ".repeat(right_pad)
                        )
                    }
                    Some(Alignment::None) => {
                        format!("{:<width$}", content, width = self.column_widths[i])
                    }
                };

                output.push_str(&formatted);
                if i < self.column_widths.len() - 1 {
                    output.push_str(" │ ");
                }
            }
            output.push_str(" │\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Table;
    use pulldown_cmark::Alignment;

    #[test]
    fn render_contains_header_and_row_content() {
        let mut table = Table::new(vec![
            ("Name".to_string(), Some(Alignment::Left)),
            ("Score".to_string(), Some(Alignment::Right)),
        ]);
        table.add_row(vec!["Alice".to_string(), "10".to_string()]);
        table.add_row(vec!["Bob".to_string(), "7".to_string()]);
        table.calculate_column_widths(60);

        let out = table.render();
        assert!(out.contains("Name"), "output should contain header: {out}");
        assert!(
            out.contains("Alice"),
            "output should contain row data: {out}"
        );
        assert!(out.contains("Bob"), "output should contain row data: {out}");
    }

    #[test]
    fn render_uses_box_drawing_chars() {
        let mut table = Table::new(vec![("A".to_string(), None)]);
        table.add_row(vec!["x".to_string()]);
        table.calculate_column_widths(40);
        let out = table.render();
        // Box-drawing characters indicate a rendered border table
        assert!(out.contains('│'), "expected box-drawing border in: {out}");
        assert!(out.contains('─'), "expected horizontal rule in: {out}");
    }
}
