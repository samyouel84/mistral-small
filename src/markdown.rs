use crate::table::Table;
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Parser, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use textwrap::{wrap, Options};

/// Parsed table: (headers, per-column alignments, rows).
type ParsedTable = Option<(Vec<String>, Vec<Option<Alignment>>, Vec<Vec<String>>)>;

pub(crate) struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    pub(crate) wrap_options: Options<'static>,
    // Table state
    in_table: bool,
    table_headers: Vec<String>,
    current_row: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_alignments: Vec<Option<Alignment>>,
}

impl MarkdownRenderer {
    pub(crate) fn new(width: usize) -> Self {
        let wrap_options = Options::new(width)
            .initial_indent("  ")
            .subsequent_indent("  ");

        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            wrap_options,
            in_table: false,
            table_headers: Vec::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_alignments: Vec::new(),
        }
    }

    fn render_table(&self) -> String {
        if self.table_headers.is_empty() && self.table_rows.is_empty() {
            return String::new();
        }

        // Create table with headers and alignments
        let headers: Vec<(String, Option<Alignment>)> = self
            .table_headers
            .iter()
            .cloned()
            .zip(self.table_alignments.iter().cloned())
            .map(|(header, alignment)| (header.trim().to_string(), alignment))
            .collect();

        let mut table = Table::new(headers);

        // Add rows with proper trimming
        for row in &self.table_rows {
            let cleaned_row: Vec<String> = row.iter().map(|cell| cell.trim().to_string()).collect();
            table.add_row(cleaned_row);
        }

        // Calculate column widths based on terminal size
        let terminal_width = match terminal_size::terminal_size() {
            Some((terminal_size::Width(w), _)) => w as usize - 4,
            None => 76,
        };
        table.calculate_column_widths(terminal_width);

        table.render()
    }

    fn flush_table(&mut self, output: &mut String) {
        if self.in_table {
            output.push_str(&self.render_table());
            self.table_headers.clear();
            self.table_rows.clear();
            self.current_row.clear();
            self.table_alignments.clear();
            self.in_table = false;
        }
    }

    fn parse_markdown_table(text: &str) -> ParsedTable {
        // Split into lines and clean up
        let lines: Vec<_> = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && l.contains('|'))
            .collect();

        if lines.len() < 3 {
            return None;
        }

        // Parse header row
        let header_line = lines[0].trim_matches('|');
        let headers: Vec<String> = header_line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if headers.is_empty() {
            return None;
        }

        // Parse alignment row
        let align_line = lines[1].trim_matches('|');
        let mut alignments: Vec<Option<Alignment>> = align_line
            .split('|')
            .map(|s| {
                let s = s.trim();
                if !s.contains('-') {
                    return Some(Alignment::Left); // Default to left alignment
                }
                match (s.starts_with(':'), s.ends_with(':')) {
                    (true, true) => Some(Alignment::Center),
                    (true, false) => Some(Alignment::Left),
                    (false, true) => Some(Alignment::Right),
                    (false, false) => Some(Alignment::Left),
                }
            })
            .collect();

        // Ensure alignments match header count
        while alignments.len() < headers.len() {
            alignments.push(Some(Alignment::Left));
        }
        alignments.truncate(headers.len());

        // Parse data rows with validation
        let mut rows = Vec::new();
        for line in &lines[2..] {
            let line = line.trim_matches('|');
            let cells: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();

            // Skip empty rows or rows with no content
            if cells.iter().all(|cell| cell.is_empty()) {
                continue;
            }

            // Ensure each row has the correct number of columns
            let mut padded_row = cells;
            while padded_row.len() < headers.len() {
                padded_row.push(String::new());
            }
            padded_row.truncate(headers.len());

            rows.push(padded_row);
        }

        // Validate final table structure
        if rows.is_empty() || rows.iter().any(|row| row.len() != headers.len()) {
            return None;
        }

        Some((headers, alignments, rows))
    }

    fn preprocess_table_text(text: &str) -> String {
        let mut result = String::new();
        let mut in_table = false;
        let mut table_lines = Vec::new();
        let mut column_count = 0;

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.contains('|') {
                // Count columns in the first table line to establish expected width
                if !in_table {
                    in_table = true;
                    table_lines.clear();
                    column_count = trimmed.matches('|').count() - 1;
                }

                // Clean up and normalize the line
                let mut cleaned = trimmed.to_string();
                if !cleaned.starts_with('|') {
                    cleaned.insert(0, '|');
                }
                if !cleaned.ends_with('|') {
                    cleaned.push('|');
                }

                // Ensure consistent column count
                let current_columns = cleaned.matches('|').count() - 1;
                if current_columns < column_count {
                    // Add missing columns
                    cleaned.push_str(&"|".repeat(column_count - current_columns));
                }

                table_lines.push(cleaned);
            } else if in_table {
                if !trimmed.is_empty() {
                    in_table = false;
                    // Add collected table lines
                    for table_line in &table_lines {
                        result.push_str(table_line);
                        result.push('\n');
                    }
                    result.push_str(trimmed);
                    result.push('\n');
                }
            } else {
                result.push_str(trimmed);
                result.push('\n');
            }
        }

        // Add any remaining table lines
        if in_table {
            for table_line in &table_lines {
                result.push_str(table_line);
                result.push('\n');
            }
        }

        result
    }

    pub(crate) fn render(&self, text: &str) -> String {
        // `render` is `render_with_hint` without a language hint; delegating
        // avoids ~175 lines of duplicated markdown-parsing logic.
        self.render_with_hint(text, None)
    }

    pub(crate) fn render_with_hint(&self, text: &str, language_hint: Option<&str>) -> String {
        // Preprocess text to fix table formatting
        let processed_text = Self::preprocess_table_text(text);

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut output = String::new();
        let mut in_code_block = false;
        let mut in_list = false;
        let mut current_paragraph = String::new();
        let mut current_language = String::new();
        let mut renderer = Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            wrap_options: self.wrap_options.clone(),
            in_table: false,
            table_headers: Vec::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_alignments: Vec::new(),
        };

        // Try to parse as a table first
        if let Some((headers, alignments, rows)) = Self::parse_markdown_table(&processed_text) {
            let mut table = Table::new(headers.into_iter().zip(alignments).collect());
            for row in rows {
                table.add_row(row);
            }

            let terminal_width = match terminal_size::terminal_size() {
                Some((terminal_size::Width(w), _)) => w as usize - 4,
                None => 76,
            };
            table.calculate_column_widths(terminal_width);
            return table.render();
        }

        // If not a table, proceed with normal markdown parsing
        let parser = Parser::new(&processed_text);

        for event in parser {
            match event {
                Event::Start(Tag::Table(alignments)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    renderer.in_table = true;
                    renderer.table_alignments = alignments.into_iter().map(Some).collect();
                }
                Event::End(Tag::Table(_)) => {
                    renderer.flush_table(&mut output);
                }
                Event::Start(Tag::TableHead) => {
                    renderer.current_row.clear();
                }
                Event::End(Tag::TableHead) => {
                    renderer.table_headers = renderer.current_row.clone();
                    renderer.current_row.clear();
                }
                Event::Start(Tag::TableRow) => {
                    renderer.current_row.clear();
                }
                Event::End(Tag::TableRow) => {
                    if !renderer.current_row.is_empty() {
                        renderer.table_rows.push(renderer.current_row.clone());
                        renderer.current_row.clear();
                    }
                }
                Event::Start(Tag::TableCell) => {
                    current_paragraph.clear();
                }
                Event::End(Tag::TableCell) => {
                    if renderer.in_table {
                        renderer.current_row.push(current_paragraph.clone());
                        current_paragraph.clear();
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    in_code_block = true;
                    current_language = match kind {
                        CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_string(),
                        _ => language_hint.unwrap_or("txt").to_string(),
                    };
                    output.push('\n');
                }
                Event::End(Tag::CodeBlock(_)) => {
                    in_code_block = false;
                    current_language.clear();
                    output.push('\n');
                }
                Event::Start(Tag::List(_)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    in_list = true;
                }
                Event::End(Tag::List(_)) => {
                    in_list = false;
                    output.push('\n');
                }
                Event::Start(Tag::Item) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    current_paragraph.push_str("• ");
                }
                Event::End(Tag::Item) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                }
                Event::Start(Tag::Paragraph) => {
                    if !current_paragraph.is_empty() {
                        renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    }
                }
                Event::End(Tag::Paragraph) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    if !in_list {
                        output.push('\n');
                    }
                }
                Event::Start(Tag::Heading(level, _, _)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    // Bold heading marker so headings read as headings in plain text
                    current_paragraph.push_str(&"#".repeat(level as usize));
                    current_paragraph.push(' ');
                }
                Event::End(Tag::Heading(..)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    output.push('\n');
                }
                Event::Start(Tag::Emphasis) => {
                    current_paragraph.push_str("\x1B[3m"); // Italic
                }
                Event::End(Tag::Emphasis) => {
                    current_paragraph.push_str("\x1B[23m"); // Reset italic
                }
                Event::Start(Tag::Strong) => {
                    current_paragraph.push_str("\x1B[1m"); // Bold
                }
                Event::End(Tag::Strong) => {
                    current_paragraph.push_str("\x1B[22m"); // Reset bold
                }
                Event::Code(text) => {
                    current_paragraph.push('`');
                    current_paragraph.push_str(&text);
                    current_paragraph.push('`');
                }
                Event::Text(text) => {
                    if in_code_block {
                        let syntax = if current_language.is_empty() {
                            language_hint
                                .and_then(|lang| self.syntax_set.find_syntax_by_token(lang))
                                .or_else(|| {
                                    language_hint.and_then(|lang| {
                                        self.syntax_set.find_syntax_by_extension(lang)
                                    })
                                })
                                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
                        } else {
                            self.syntax_set
                                .find_syntax_by_token(&current_language)
                                .or_else(|| {
                                    self.syntax_set.find_syntax_by_extension(&current_language)
                                })
                                .or_else(|| {
                                    language_hint
                                        .and_then(|lang| self.syntax_set.find_syntax_by_token(lang))
                                })
                                .or_else(|| {
                                    language_hint.and_then(|lang| {
                                        self.syntax_set.find_syntax_by_extension(lang)
                                    })
                                })
                                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
                        };

                        let mut highlighter = HighlightLines::new(syntax, theme);

                        for line in LinesWithEndings::from(&text) {
                            match highlighter.highlight_line(line, &self.syntax_set) {
                                Ok(ranges) => {
                                    output.push_str("    "); // Indent
                                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                                    output.push_str(&escaped);
                                }
                                Err(_) => {
                                    output.push_str("    ");
                                    output.push_str(line);
                                }
                            }
                        }
                    } else {
                        current_paragraph.push_str(&text);
                    }
                }
                Event::SoftBreak => {
                    current_paragraph.push(' ');
                }
                Event::HardBreak => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    output.push('\n');
                }
                _ => {}
            }
        }

        renderer.flush_table(&mut output);
        output.trim_end().to_string()
    }

    fn flush_paragraph(&self, output: &mut String, current: &mut String) {
        if !current.is_empty() {
            if current.starts_with('•') {
                // Split the current paragraph by bullet points
                let items: Vec<&str> = current.split("•").collect();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        // Skip the empty string before the first bullet
                        let trimmed_item = item.trim();
                        if !trimmed_item.is_empty() {
                            let mut list_options = self.wrap_options.clone();
                            list_options.initial_indent = "  • "; // Indent with bullet
                            list_options.subsequent_indent = "    "; // 4 spaces for wrapped lines

                            // Wrap each list item separately
                            for line in wrap(trimmed_item, &list_options) {
                                output.push_str(&line);
                                output.push('\n');
                            }
                        }
                    }
                }
            } else {
                // For normal paragraphs
                for line in wrap(current, &self.wrap_options) {
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            current.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownRenderer;

    fn renderer() -> MarkdownRenderer {
        MarkdownRenderer::new(80)
    }

    #[test]
    fn renders_plain_text() {
        let out = renderer().render("hello world");
        assert!(out.contains("hello world"), "got: {out}");
    }

    #[test]
    fn renders_heading() {
        let out = renderer().render("# Title");
        assert!(out.contains("Title"), "heading text should appear: {out}");
    }

    #[test]
    fn render_and_render_with_hint_are_equivalent_without_hint() {
        let md = "some **bold** and `code` text";
        assert_eq!(
            renderer().render(md),
            renderer().render_with_hint(md, None),
            "render() should delegate to render_with_hint(None)"
        );
    }

    #[test]
    fn renders_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let out = renderer().render(md);
        // "main" is a single syntax-highlight token, so it stays contiguous even
        // though ANSI escape codes are interleaved between tokens.
        assert!(out.contains("main"), "code content should appear: {out}");
    }
}
