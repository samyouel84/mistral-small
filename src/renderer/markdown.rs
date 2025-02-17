use crate::renderer::{Table, SyntaxCache};
use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind, Alignment};
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use textwrap::{Options, wrap};
use std::fmt::Write;

pub struct MarkdownRenderer {
    wrap_options: Options<'static>,
    // Table state
    in_table: bool,
    table_headers: Vec<String>,
    current_row: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_alignments: Vec<Option<Alignment>>,
}

impl MarkdownRenderer {
    pub fn new(width: usize) -> Self {
        let wrap_options = Options::new(width)
            .initial_indent("  ")
            .subsequent_indent("  ");
            
        Self {
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

        let headers: Vec<(String, Option<Alignment>)> = self.table_headers.iter().cloned()
            .zip(self.table_alignments.iter().cloned())
            .map(|(header, alignment)| (header.trim().to_string(), alignment))
            .collect();

        let mut table = Table::new(headers);

        for row in &self.table_rows {
            let cleaned_row: Vec<String> = row.iter()
                .map(|cell| cell.trim().to_string())
                .collect();
            table.add_row(cleaned_row);
        }

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

    fn parse_markdown_table(text: &str) -> Option<(Vec<String>, Vec<Option<Alignment>>, Vec<Vec<String>>)> {
        let lines: Vec<_> = text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && l.contains('|'))
            .collect();

        if lines.len() < 3 {
            return None;
        }

        let header_line = lines[0].trim_matches('|');
        let headers: Vec<String> = header_line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if headers.is_empty() {
            return None;
        }

        let align_line = lines[1].trim_matches('|');
        let mut alignments: Vec<Option<Alignment>> = align_line
            .split('|')
            .map(|s| {
                let s = s.trim();
                if !s.contains('-') {
                    return Some(Alignment::Left);
                }
                match (s.starts_with(':'), s.ends_with(':')) {
                    (true, true) => Some(Alignment::Center),
                    (true, false) => Some(Alignment::Left),
                    (false, true) => Some(Alignment::Right),
                    (false, false) => Some(Alignment::Left),
                }
            })
            .collect();

        while alignments.len() < headers.len() {
            alignments.push(Some(Alignment::Left));
        }
        alignments.truncate(headers.len());

        let mut rows = Vec::new();
        for line in &lines[2..] {
            let line = line.trim_matches('|');
            let cells: Vec<String> = line
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            if cells.iter().all(|cell| cell.is_empty()) {
                continue;
            }

            let mut padded_row = cells;
            while padded_row.len() < headers.len() {
                padded_row.push(String::new());
            }
            padded_row.truncate(headers.len());

            rows.push(padded_row);
        }

        if rows.is_empty() || rows.iter().any(|row| row.len() != headers.len()) {
            return None;
        }

        Some((headers, alignments, rows))
    }

    fn preprocess_table_text(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut in_table = false;
        let mut table_lines = Vec::new();
        let mut column_count = 0;

        for line in text.lines() {
            let trimmed = line.trim();
            
            if trimmed.contains('|') {
                if !in_table {
                    in_table = true;
                    table_lines.clear();
                    column_count = trimmed.matches('|').count() - 1;
                }
                
                let mut cleaned = String::with_capacity(trimmed.len() + 2);
                if !trimmed.starts_with('|') {
                    cleaned.push('|');
                }
                cleaned.push_str(trimmed);
                if !trimmed.ends_with('|') {
                    cleaned.push('|');
                }

                let current_columns = cleaned.matches('|').count() - 1;
                if current_columns < column_count {
                    cleaned.reserve(column_count - current_columns);
                    cleaned.push_str(&"|".repeat(column_count - current_columns));
                }

                table_lines.push(cleaned);
            } else if in_table {
                if !trimmed.is_empty() {
                    in_table = false;
                    for table_line in &table_lines {
                        writeln!(result, "{}", table_line).unwrap();
                    }
                    writeln!(result, "{}", trimmed).unwrap();
                }
            } else {
                writeln!(result, "{}", trimmed).unwrap();
            }
        }

        if in_table {
            for table_line in &table_lines {
                writeln!(result, "{}", table_line).unwrap();
            }
        }

        result
    }

    pub fn render(&self, text: &str) -> String {
        let processed_text = Self::preprocess_table_text(text);
        let syntax_cache = SyntaxCache::global();
        let theme = syntax_cache.get_theme();
        
        let mut output = String::with_capacity(processed_text.len() * 2);
        let mut in_code_block = false;
        let mut in_list = false;
        let mut current_paragraph = String::with_capacity(256);
        let mut current_language = String::new();
        let mut renderer = Self {
            wrap_options: self.wrap_options.clone(),
            in_table: false,
            table_headers: Vec::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_alignments: Vec::new(),
        };

        if let Some((headers, alignments, rows)) = Self::parse_markdown_table(&processed_text) {
            let mut table = Table::new(headers.into_iter().zip(alignments.into_iter()).collect());
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
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        _ => "txt".to_string(),
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
                Event::Start(Tag::Emphasis) => {
                    current_paragraph.push_str("\x1B[3m");
                }
                Event::End(Tag::Emphasis) => {
                    current_paragraph.push_str("\x1B[23m");
                }
                Event::Start(Tag::Strong) => {
                    current_paragraph.push_str("\x1B[1m");
                }
                Event::End(Tag::Strong) => {
                    current_paragraph.push_str("\x1B[22m");
                }
                Event::Code(text) => {
                    current_paragraph.push('`');
                    current_paragraph.push_str(&text);
                    current_paragraph.push('`');
                }
                Event::Text(text) => {
                    if in_code_block {
                        let syntax = if current_language.is_empty() {
                            syntax_cache.get_syntax("txt")
                        } else {
                            syntax_cache.get_syntax(&current_language)
                        };

                        let mut highlighter = HighlightLines::new(syntax, theme);
                        
                        for line in LinesWithEndings::from(&text) {
                            match highlighter.highlight_line(line, &syntax_cache.syntax_set) {
                                Ok(ranges) => {
                                    output.push_str("    ");
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

    pub fn render_with_hint(&self, text: &str, language_hint: Option<&str>) -> String {
        let processed_text = Self::preprocess_table_text(text);
        let syntax_cache = SyntaxCache::global();
        let theme = syntax_cache.get_theme();
        
        let mut output = String::with_capacity(processed_text.len() * 2);
        let mut in_code_block = false;
        let mut in_list = false;
        let mut current_paragraph = String::with_capacity(256);
        let mut current_language = String::new();
        let mut renderer = Self {
            wrap_options: self.wrap_options.clone(),
            in_table: false,
            table_headers: Vec::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_alignments: Vec::new(),
        };

        if let Some((headers, alignments, rows)) = Self::parse_markdown_table(&processed_text) {
            let mut table = Table::new(headers.into_iter().zip(alignments.into_iter()).collect());
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

        let parser = Parser::new(&processed_text);

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    renderer.flush_paragraph(&mut output, &mut current_paragraph);
                    in_code_block = true;
                    current_language = match kind {
                        CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_string(),
                        _ => language_hint.unwrap_or("txt").to_string(),
                    };
                    output.push('\n');
                }
                Event::Text(text) if in_code_block => {
                    let syntax = if current_language.is_empty() {
                        language_hint
                            .map(|lang| syntax_cache.get_syntax(lang))
                            .unwrap_or_else(|| syntax_cache.get_syntax("txt"))
                    } else {
                        syntax_cache.get_syntax(&current_language)
                    };

                    let mut highlighter = HighlightLines::new(syntax, theme);
                    
                    for line in LinesWithEndings::from(&text) {
                        match highlighter.highlight_line(line, &syntax_cache.syntax_set) {
                            Ok(ranges) => {
                                output.push_str("    ");
                                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                                output.push_str(&escaped);
                            }
                            Err(_) => {
                                output.push_str("    ");
                                output.push_str(line);
                            }
                        }
                    }
                }
                // Handle other events same as render()
                event => self.handle_markdown_event(event, &mut output, &mut current_paragraph, 
                    &mut in_code_block, &mut in_list, &mut current_language, &mut renderer),
            }
        }

        renderer.flush_table(&mut output);
        output.trim_end().to_string()
    }

    fn handle_markdown_event(&self, event: Event, output: &mut String, current_paragraph: &mut String,
        in_code_block: &mut bool, in_list: &mut bool, current_language: &mut String, 
        renderer: &mut MarkdownRenderer) {
        match event {
            Event::Start(Tag::Table(alignments)) => {
                renderer.flush_paragraph(output, current_paragraph);
                renderer.in_table = true;
                renderer.table_alignments = alignments.into_iter().map(Some).collect();
            }
            Event::End(Tag::Table(_)) => {
                renderer.flush_table(output);
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
            Event::End(Tag::CodeBlock(_)) => {
                *in_code_block = false;
                current_language.clear();
                output.push('\n');
            }
            Event::Start(Tag::List(_)) => {
                renderer.flush_paragraph(output, current_paragraph);
                *in_list = true;
            }
            Event::End(Tag::List(_)) => {
                *in_list = false;
                output.push('\n');
            }
            Event::Start(Tag::Item) => {
                renderer.flush_paragraph(output, current_paragraph);
                current_paragraph.push_str("• ");
            }
            Event::End(Tag::Item) => {
                renderer.flush_paragraph(output, current_paragraph);
            }
            Event::Start(Tag::Paragraph) => {
                if !current_paragraph.is_empty() {
                    renderer.flush_paragraph(output, current_paragraph);
                }
            }
            Event::End(Tag::Paragraph) => {
                renderer.flush_paragraph(output, current_paragraph);
                if !*in_list {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Emphasis) => {
                current_paragraph.push_str("\x1B[3m");
            }
            Event::End(Tag::Emphasis) => {
                current_paragraph.push_str("\x1B[23m");
            }
            Event::Start(Tag::Strong) => {
                current_paragraph.push_str("\x1B[1m");
            }
            Event::End(Tag::Strong) => {
                current_paragraph.push_str("\x1B[22m");
            }
            Event::Code(text) => {
                current_paragraph.push('`');
                current_paragraph.push_str(&text);
                current_paragraph.push('`');
            }
            Event::Text(text) if !*in_code_block => {
                current_paragraph.push_str(&text);
            }
            Event::SoftBreak => {
                current_paragraph.push(' ');
            }
            Event::HardBreak => {
                renderer.flush_paragraph(output, current_paragraph);
                output.push('\n');
            }
            _ => {}
        }
    }

    fn flush_paragraph(&self, output: &mut String, current: &mut String) {
        if !current.is_empty() {
            if current.starts_with('•') {
                let items: Vec<&str> = current.split('•').collect();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        let trimmed_item = item.trim();
                        if !trimmed_item.is_empty() {
                            let mut list_options = self.wrap_options.clone();
                            list_options.initial_indent = "  • ";
                            list_options.subsequent_indent = "    ";

                            for line in wrap(trimmed_item, &list_options) {
                                writeln!(output, "{}", line).unwrap();
                            }
                        }
                    }
                }
            } else {
                for line in wrap(current, &self.wrap_options) {
                    writeln!(output, "{}", line).unwrap();
                }
            }
            current.clear();
        }
    }
} 