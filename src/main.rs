use anyhow::Result;
use colored::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use textwrap::{wrap, Options};
use tokio;
use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind, Alignment};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

struct MistralClient {
    client: reqwest::Client,
    api_key: String,
}

impl MistralClient {
    fn new(api_key: String) -> Self {
        let client = reqwest::Client::new();
        Self { client, api_key }
    }

    fn extract_language_hint(input: &str) -> Option<String> {
        let input = input.to_lowercase();
        let keywords = [
            // Systems Programming
            ("rust", "rust"),
            ("cpp", "cpp"),
            ("c++", "cpp"),
            ("c#", "cs"),
            ("csharp", "cs"),
            ("c lang", "c"),
            (" c ", "c"),
            ("objective-c", "objc"),
            ("objc", "objc"),
            ("assembly", "asm"),
            ("asm", "asm"),
            
            // Web Development
            ("javascript", "javascript"),
            ("js", "javascript"),
            ("typescript", "typescript"),
            ("ts", "typescript"),
            ("html", "html"),
            ("css", "css"),
            ("scss", "scss"),
            ("sass", "scss"),
            ("less", "less"),
            ("php", "php"),
            ("webassembly", "wasm"),
            ("wasm", "wasm"),
            
            // Scripting Languages
            ("python", "python"),
            ("py", "python"),
            ("ruby", "ruby"),
            ("perl", "perl"),
            ("lua", "lua"),
            ("powershell", "powershell"),
            ("ps1", "powershell"),
            ("shell", "shell"),
            ("bash", "shell"),
            ("zsh", "shell"),
            ("fish", "shell"),
            
            // JVM Languages
            ("java", "java"),
            ("kotlin", "kotlin"),
            ("scala", "scala"),
            ("groovy", "groovy"),
            ("clojure", "clojure"),
            
            // Mobile Development
            ("swift", "swift"),
            ("kotlin android", "kotlin"),
            ("objective-c", "objc"),
            ("dart", "dart"),
            ("flutter", "dart"),
            
            // Data & ML
            ("r lang", "r"),
            (" r ", "r"),
            ("julia", "julia"),
            ("matlab", "matlab"),
            ("octave", "matlab"),
            
            // Databases
            ("sql", "sql"),
            ("mysql", "sql"),
            ("postgresql", "sql"),
            ("postgres", "sql"),
            ("plsql", "sql"),
            ("oracle", "sql"),
            ("tsql", "sql"),
            ("mongodb", "javascript"),  // For MongoDB queries
            
            // Configuration & Data Formats
            ("json", "json"),
            ("yaml", "yaml"),
            ("yml", "yaml"),
            ("toml", "toml"),
            ("xml", "xml"),
            ("ini", "ini"),
            ("dockerfile", "dockerfile"),
            ("docker", "dockerfile"),
            
            // Modern Languages
            ("go", "go"),
            ("golang", "go"),
            ("elixir", "elixir"),
            ("erlang", "erlang"),
            ("haskell", "haskell"),
            ("ocaml", "ocaml"),
            ("f#", "fsharp"),
            ("fsharp", "fsharp"),
            ("nim", "nim"),
            ("crystal", "crystal"),
            ("zig", "zig"),
            
            // Build & Config
            ("makefile", "makefile"),
            ("cmake", "cmake"),
            ("gradle", "gradle"),
            ("maven", "xml"),
            ("pom", "xml"),
            
            // Version Control
            ("git", "git"),
            ("gitignore", "gitignore"),
            ("gitconfig", "gitconfig"),
            
            // Markup
            ("markdown", "markdown"),
            ("md", "markdown"),
            ("tex", "tex"),
            ("latex", "tex"),
            ("restructuredtext", "rst"),
            ("rst", "rst"),
            ("asciidoc", "asciidoc"),
            
            // Protocol & Schema
            ("protobuf", "protobuf"),
            ("proto", "protobuf"),
            ("thrift", "thrift"),
            ("graphql", "graphql"),
            ("gql", "graphql"),
        ];

        for (keyword, lang) in keywords.iter() {
            if input.contains(keyword) {
                return Some(lang.to_string());
            }
        }

        // Check for common programming questions
        if input.contains("code") || input.contains("function") || input.contains("program") 
            || input.contains("algorithm") || input.contains("class") || input.contains("method") {
            return Some("txt".to_string());
        }

        None
    }

    async fn send_message(&self, messages: Vec<ChatMessage>) -> Result<(String, Option<String>)> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Extract language hint from the last user message
        let language_hint = messages.last()
            .filter(|msg| msg.role == "user")
            .and_then(|msg| Self::extract_language_hint(&msg.content));

        let request = ChatRequest {
            model: "mistral-small".to_string(),
            messages,
        };

        let response = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .headers(headers)
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok((response.choices[0].message.content.clone(), language_hint))
    }
}

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
struct Table {
    headers: TableRow,
    rows: Vec<TableRow>,
    column_widths: Vec<usize>,
}

impl Table {
    fn new(headers: Vec<(String, Option<Alignment>)>) -> Self {
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

    fn add_row(&mut self, cells: Vec<String>) {
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

    fn calculate_column_widths(&mut self, max_width: usize) {
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

    fn render(&self) -> String {
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
        for (i, (&width, cell)) in self.column_widths.iter().zip(&self.headers.cells).enumerate() {
            match cell.alignment {
                Some(Alignment::Left) => {
                    output.push(':');
                    output.push_str(&"─".repeat(width + 1));
                },
                Some(Alignment::Right) => {
                    output.push_str(&"─".repeat(width + 1));
                    output.push(':');
                },
                Some(Alignment::Center) => {
                    output.push(':');
                    output.push_str(&"─".repeat(width));
                    output.push(':');
                },
                Some(Alignment::None) | None => {
                    output.push_str(&"─".repeat(width + 2));
                },
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
        let wrapped_contents: Vec<Vec<String>> = row.cells.iter().zip(&self.column_widths)
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
        let max_lines = wrapped_contents.iter().map(|lines| lines.len()).max().unwrap_or(1);

        // Render each line of the row
        for line_idx in 0..max_lines {
            output.push_str("  │ ");
            for (i, (cell, wrapped_content)) in row.cells.iter().zip(&wrapped_contents).enumerate() {
                let content = wrapped_content.get(line_idx).map_or("", |s| s);
                
                let formatted = match cell.alignment {
                    Some(Alignment::Left) | None => format!("{:<width$}", content, width = self.column_widths[i]),
                    Some(Alignment::Right) => format!("{:>width$}", content, width = self.column_widths[i]),
                    Some(Alignment::Center) => {
                        let spaces = self.column_widths[i] - content.chars().count();
                        let left_pad = spaces / 2;
                        let right_pad = spaces - left_pad;
                        format!("{}{}{}", " ".repeat(left_pad), content, " ".repeat(right_pad))
                    },
                    Some(Alignment::None) => format!("{:<width$}", content, width = self.column_widths[i]),
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

struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    wrap_options: Options<'static>,
    // Table state
    in_table: bool,
    table_headers: Vec<String>,
    current_row: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_alignments: Vec<Option<Alignment>>,
}

impl MarkdownRenderer {
    fn new(width: usize) -> Self {
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
        let headers: Vec<(String, Option<Alignment>)> = self.table_headers.iter().cloned()
            .zip(self.table_alignments.iter().cloned())
            .map(|(header, alignment)| (header.trim().to_string(), alignment))
            .collect();

        let mut table = Table::new(headers);

        // Add rows with proper trimming
        for row in &self.table_rows {
            let cleaned_row: Vec<String> = row.iter()
                .map(|cell| cell.trim().to_string())
                .collect();
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

    fn parse_markdown_table(text: &str) -> Option<(Vec<String>, Vec<Option<Alignment>>, Vec<Vec<String>>)> {
        // Split into lines and clean up
        let lines: Vec<_> = text.lines()
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
                    return Some(Alignment::Left);  // Default to left alignment
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
            let cells: Vec<String> = line
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

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

    fn render(&self, text: &str) -> String {
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
                            self.syntax_set.find_syntax_plain_text()
                        } else {
                            self.syntax_set
                                .find_syntax_by_token(&current_language)
                                .or_else(|| self.syntax_set.find_syntax_by_extension(&current_language))
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

    fn render_with_hint(&self, text: &str, language_hint: Option<&str>) -> String {
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
                                .or_else(|| language_hint.and_then(|lang| self.syntax_set.find_syntax_by_extension(lang)))
                                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
                        } else {
                            self.syntax_set
                                .find_syntax_by_token(&current_language)
                                .or_else(|| self.syntax_set.find_syntax_by_extension(&current_language))
                                .or_else(|| language_hint.and_then(|lang| self.syntax_set.find_syntax_by_token(lang)))
                                .or_else(|| language_hint.and_then(|lang| self.syntax_set.find_syntax_by_extension(lang)))
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
                    if i > 0 { // Skip the empty string before the first bullet
                        let trimmed_item = item.trim();
                        if !trimmed_item.is_empty() {
                            let mut list_options = self.wrap_options.clone();
                            list_options.initial_indent = "  • ";  // Indent with bullet
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

async fn chat_loop(client: MistralClient) -> Result<()> {
    let mut messages = Vec::new();
    
    // Get terminal width, default to 80 if unable to get it
    let width = match terminal_size::terminal_size() {
        Some((terminal_size::Width(w), _)) => w as usize - 2, // Subtract 2 for margin
        None => 80,
    };

    let renderer = MarkdownRenderer::new(width);

    // Define command box
    let command_box = "\
┌──────────────────────────────────────┐\n\
│          Available Commands          │\n\
├──────────────────────────────────────┤\n\
│    `exit`  - Quit the application    │\n\
├──────────────────────────────────────┤\n\
│    `clear` - Clear the screen        │\n\
├──────────────────────────────────────┤\n\
│    `new`   - Start a new chat        │\n\
└──────────────────────────────────────┘";

    // Function to show command box
    let show_command_box = || {
        println!("{}", command_box.green());
        println!();
    };

    // Show initial welcome message
    clearscreen::clear()?;
    if messages.is_empty() {
        let welcome_message = "I am Mistral Chat AI, a helpful and respectful assistant\npowered by Mistral. Here are some ways I can assist you:\n\n• Provide information and answer questions on a wide\nrange of topics\n• Generate ideas, suggestions, and recommendations\n\nI'm ready to help! How can I assist you today?";

        print!("{}", renderer.render(&welcome_message).cyan());
        println!("\n");
        show_command_box();
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;
    } else {
        show_command_box();
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;
    }

    // Configure rustyline editor with history
    let mut rl = DefaultEditor::new()?;
    rl.set_max_history_size(100)?;
    
    // Load history from file if it exists
    let history_file = dirs::home_dir()
        .map(|mut path| {
            path.push(".mistral_history");
            path
        })
        .unwrap_or_else(|| ".mistral_history".into());
    
    if history_file.exists() {
        let _ = rl.load_history(&history_file);
    }
    
    loop {
        let prompt = format!("{}", "> ".blue().bold());
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.eq_ignore_ascii_case("exit") {
                    // Save history before exiting
                    let _ = rl.save_history(&history_file);
                    break;
                } else if input.eq_ignore_ascii_case("clear") {
                    clearscreen::clear()?;
                    show_command_box();
                    continue;
                } else if input.eq_ignore_ascii_case("new") {
                    messages.clear();
                    clearscreen::clear()?;
                    show_command_box();
                    println!("{}", "Starting a fresh conversation...".green());
                    print!("{}", "> ".blue().bold());
                    io::stdout().flush()?;
                    continue;
                }

                // Add valid input to history
                if !input.is_empty() {
                    rl.add_history_entry(input)?;
                }

                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: input.to_string(),
                });

                print!("{}", "Thinking...".yellow());
                io::stdout().flush()?;

                match client.send_message(messages.clone()).await {
                    Ok((response, language_hint)) => {
                        clearscreen::clear()?;
                        show_command_box();
                        
                        print!("{}", "> ".blue().bold());
                        println!("{}", input);
                        println!();
                        
                        print!("{}", renderer.render_with_hint(&response, language_hint.as_deref()).cyan());
                        println!();
                        println!();

                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response,
                        });
                        
                        print!("{}", "> ".blue().bold());
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        print!("\r{}\r", " ".repeat(width)); // Clear "Thinking..." line
                        println!();
                        for line in wrap(&format!("Error: {}", e), &renderer.wrap_options) {
                            println!("{}", line.red());
                        }
                        println!();
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Use 'exit' to quit");
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    let api_key = env::var("MISTRAL_API_KEY")
        .expect("MISTRAL_API_KEY must be set in environment variables or .env file");

    let client = MistralClient::new(api_key);
    chat_loop(client).await?;

    Ok(())
}
