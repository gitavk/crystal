use std::collections::HashMap;

use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

use super::QueryPane;

pub(super) struct CompletionState {
    pub(super) items: Vec<String>,
    pub(super) selected: usize,
    pub(super) prefix_len: usize,
}

pub(super) static PG_KEYWORDS: &[&str] = &[
    "ALL",
    "ANALYSE",
    "ANALYZE",
    "AND",
    "ANY",
    "ARRAY",
    "AS",
    "ASC",
    "ASYMMETRIC",
    "BETWEEN",
    "BOTH",
    "BY",
    "CALLED",
    "CASE",
    "CAST",
    "CHECK",
    "CLUSTER",
    "COALESCE",
    "COLLATE",
    "COLUMN",
    "COMMENT",
    "COMMIT",
    "CONCURRENTLY",
    "CONSTRAINT",
    "COPY",
    "CREATE",
    "CROSS",
    "CURRENT",
    "CURRENT_DATE",
    "CURRENT_ROLE",
    "CURRENT_TIME",
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "DATABASE",
    "DEFAULT",
    "DELETE",
    "DESC",
    "DISTINCT",
    "DO",
    "DROP",
    "ELSE",
    "END",
    "ENUM",
    "ESCAPE",
    "EXCEPT",
    "EXISTS",
    "EXPLAIN",
    "EXTENSION",
    "FALSE",
    "FETCH",
    "FOLLOWING",
    "FOR",
    "FOREIGN",
    "FORMAT",
    "FROM",
    "FULL",
    "FUNCTION",
    "GRANT",
    "GROUP",
    "HAVING",
    "ILIKE",
    "IMMUTABLE",
    "IN",
    "INDEX",
    "INNER",
    "INSERT",
    "INTERSECT",
    "INTO",
    "IS",
    "JOIN",
    "JSON",
    "KEY",
    "LANGUAGE",
    "LATERAL",
    "LEADING",
    "LEFT",
    "LIKE",
    "LIMIT",
    "LOCK",
    "MATERIALIZED",
    "NATURAL",
    "NOT",
    "NULL",
    "NULLIF",
    "OFFSET",
    "ON",
    "ONLY",
    "OR",
    "ORDER",
    "OUTER",
    "OVER",
    "OVERLAPS",
    "PARTITION",
    "PRECEDING",
    "PRIMARY",
    "PROCEDURE",
    "RANGE",
    "RECURSIVE",
    "REFERENCES",
    "REINDEX",
    "RETURNING",
    "REVOKE",
    "RIGHT",
    "ROLE",
    "ROLLBACK",
    "ROW",
    "ROWS",
    "SAVEPOINT",
    "SCHEMA",
    "SELECT",
    "SEQUENCE",
    "SESSION",
    "SET",
    "SIMILAR",
    "SOME",
    "STABLE",
    "STRICT",
    "TABLE",
    "TABLESAMPLE",
    "TEXT",
    "THEN",
    "TO",
    "TRAILING",
    "TRANSACTION",
    "TRIGGER",
    "TRUE",
    "TRUNCATE",
    "TYPE",
    "UNBOUNDED",
    "UNION",
    "UNIQUE",
    "UPDATE",
    "USER",
    "VACUUM",
    "VALUES",
    "VERBOSE",
    "VIEW",
    "VOLATILE",
    "WHEN",
    "WHERE",
    "WINDOW",
    "WITH",
    "YAML",
];

enum CompletionContext {
    Keyword,
    TableName,
    TableColumn { table: String },
    ColumnName { from_tables: Vec<String> },
}

impl QueryPane {
    pub fn trigger_completion(&mut self) -> bool {
        let ctx = completion_context(&self.editor_lines, self.cursor_row, self.cursor_col);
        let prefix = token_before_cursor(&self.editor_lines[self.cursor_row], self.cursor_col);
        let prefix_len = prefix.chars().count();
        let items = build_completion_items(ctx, &prefix, &self.schema_tables, &self.column_cache);
        if items.is_empty() {
            self.completion = None;
            return false;
        }
        self.completion = Some(CompletionState { items, selected: 0, prefix_len });
        true
    }

    pub fn update_completion(&mut self) {
        let ctx = completion_context(&self.editor_lines, self.cursor_row, self.cursor_col);
        let prefix = token_before_cursor(&self.editor_lines[self.cursor_row], self.cursor_col);
        let prefix_len = prefix.chars().count();
        let items = build_completion_items(ctx, &prefix, &self.schema_tables, &self.column_cache);
        if items.is_empty() {
            self.completion = None;
        } else if let Some(ref mut c) = self.completion {
            c.selected = c.selected.min(items.len().saturating_sub(1));
            c.prefix_len = prefix_len;
            c.items = items;
        }
    }

    pub fn complete_next(&mut self) {
        if let Some(ref mut c) = self.completion {
            if c.selected + 1 < c.items.len() {
                c.selected += 1;
            }
        }
    }

    pub fn complete_prev(&mut self) {
        if let Some(ref mut c) = self.completion {
            c.selected = c.selected.saturating_sub(1);
        }
    }

    pub fn complete_accept(&mut self) {
        let Some(ref c) = self.completion else { return };
        let Some(word) = c.items.get(c.selected) else { return };
        let word = word.clone();
        let prefix_len = c.prefix_len;
        self.completion = None;

        let line = &mut self.editor_lines[self.cursor_row];
        let end_byte = super::editor::char_to_byte(line, self.cursor_col);
        let start_byte = super::editor::char_to_byte(line, self.cursor_col.saturating_sub(prefix_len));
        line.replace_range(start_byte..end_byte, &word);
        self.cursor_col = self.cursor_col.saturating_sub(prefix_len) + word.chars().count();
    }

    pub fn complete_dismiss(&mut self) {
        self.completion = None;
    }

    pub fn completion_is_open(&self) -> bool {
        self.completion.is_some()
    }
}

fn token_before_cursor(line: &str, cursor_col: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let end = cursor_col.min(chars.len());
    let mut start = end;
    while start > 0 && (chars[start - 1].is_ascii_alphabetic() || chars[start - 1] == '_') {
        start -= 1;
    }
    chars[start..end].iter().collect()
}

fn completion_context(lines: &[String], cursor_row: usize, cursor_col: usize) -> CompletionContext {
    let full_query = lines.join("\n");

    let mut before = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i < cursor_row {
            before.push_str(line);
            before.push('\n');
        } else if i == cursor_row {
            before.extend(line.chars().take(cursor_col));
        }
    }

    let prefix_len = token_before_cursor(&lines[cursor_row], cursor_col).chars().count();
    let before_char_count = before.chars().count();
    let text_before_prefix: String = before.chars().take(before_char_count.saturating_sub(prefix_len)).collect();

    let trimmed = text_before_prefix.trim_end();
    if let Some(before_dot) = trimmed.strip_suffix('.') {
        let chars: Vec<char> = before_dot.chars().collect();
        let mut start = chars.len();
        let mut i = chars.len();
        while i > 0 {
            let c = chars[i - 1];
            if c.is_ascii_alphanumeric() || c == '_' {
                start = i - 1;
                i -= 1;
            } else {
                break;
            }
        }
        let raw: String = chars[start..].iter().collect();
        if !raw.is_empty() && (raw.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')) {
            let aliases = extract_alias_map(&full_query);
            let table = aliases.get(&raw.to_ascii_lowercase()).cloned().unwrap_or(raw);
            return CompletionContext::TableColumn { table };
        }
    }

    match last_context_keyword(&text_before_prefix).as_deref() {
        Some("FROM") | Some("JOIN") | Some("UPDATE") | Some("INTO") | Some("TABLE") => CompletionContext::TableName,
        Some("SELECT") | Some("WHERE") | Some("HAVING") | Some("SET") | Some("ON") | Some("AND") | Some("OR")
        | Some("BY") => CompletionContext::ColumnName { from_tables: extract_from_tables(&full_query) },
        _ => CompletionContext::Keyword,
    }
}

fn build_completion_items(
    ctx: CompletionContext,
    prefix: &str,
    schema_tables: &[(String, String)],
    column_cache: &HashMap<String, Vec<(String, String)>>,
) -> Vec<String> {
    match ctx {
        CompletionContext::Keyword => {
            if prefix.is_empty() {
                return Vec::new();
            }
            let prefix_upper = prefix.to_ascii_uppercase();
            PG_KEYWORDS
                .iter()
                .filter(|kw| kw.starts_with(prefix_upper.as_str()))
                .map(|s| s.to_string())
                .take(8)
                .collect()
        }
        CompletionContext::TableName => {
            let prefix_lower = prefix.to_ascii_lowercase();
            schema_tables
                .iter()
                .filter(|(name, _)| prefix_lower.is_empty() || name.to_ascii_lowercase().starts_with(&prefix_lower))
                .map(|(name, _)| name.clone())
                .take(8)
                .collect()
        }
        CompletionContext::TableColumn { table } => {
            let prefix_lower = prefix.to_ascii_lowercase();
            let table_lower = table.to_ascii_lowercase();
            let cols = column_cache.iter().find(|(k, _)| k.to_ascii_lowercase() == table_lower).map(|(_, v)| v);
            match cols {
                Some(cols) => cols
                    .iter()
                    .filter(|(name, _)| prefix_lower.is_empty() || name.to_ascii_lowercase().starts_with(&prefix_lower))
                    .map(|(name, _)| name.clone())
                    .take(8)
                    .collect(),
                None => Vec::new(),
            }
        }
        CompletionContext::ColumnName { from_tables } => {
            let prefix_lower = prefix.to_ascii_lowercase();
            let prefix_upper = prefix.to_ascii_uppercase();
            let mut items: Vec<String> = Vec::new();
            for table in &from_tables {
                let table_lower = table.to_ascii_lowercase();
                if let Some(cols) =
                    column_cache.iter().find(|(k, _)| k.to_ascii_lowercase() == table_lower).map(|(_, v)| v)
                {
                    for (name, _) in cols {
                        if (prefix_lower.is_empty() || name.to_ascii_lowercase().starts_with(&prefix_lower))
                            && !items.contains(name)
                        {
                            items.push(name.clone());
                        }
                        if items.len() >= 8 {
                            break;
                        }
                    }
                }
            }
            if !prefix.is_empty() {
                for kw in PG_KEYWORDS {
                    if kw.starts_with(prefix_upper.as_str()) && !items.contains(&kw.to_string()) {
                        items.push(kw.to_string());
                    }
                    if items.len() >= 8 {
                        break;
                    }
                }
            }
            items
        }
    }
}

fn last_context_keyword(text: &str) -> Option<String> {
    const CTX_KW: &[&str] = &[
        "SELECT", "FROM", "WHERE", "JOIN", "HAVING", "UPDATE", "INSERT", "INTO", "SET", "ON", "AND", "OR", "TABLE",
        "BY",
    ];
    let mut last: Option<String> = None;
    for token in sql_tokens(text) {
        let upper = token.to_ascii_uppercase();
        if CTX_KW.contains(&upper.as_str()) {
            last = Some(upper);
        }
    }
    last
}

fn extract_from_tables(query: &str) -> Vec<String> {
    const TABLE_CTX: &[&str] = &["FROM", "JOIN"];
    const SKIP: &[&str] = &[
        "SELECT",
        "WHERE",
        "ON",
        "AND",
        "OR",
        "HAVING",
        "GROUP",
        "ORDER",
        "LIMIT",
        "OFFSET",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "CROSS",
        "FULL",
        "NATURAL",
        "SET",
        "INTO",
        "VALUES",
        "UPDATE",
        "INSERT",
        "DELETE",
        "CREATE",
        "WITH",
        "UNION",
        "INTERSECT",
        "EXCEPT",
        "AS",
    ];
    let mut tables: Vec<String> = Vec::new();
    let tokens: Vec<&str> = sql_tokens(query).collect();
    let mut i = 0;
    while i < tokens.len() {
        if TABLE_CTX.contains(&tokens[i].to_ascii_uppercase().as_str()) && i + 1 < tokens.len() {
            let candidate = tokens[i + 1];
            let upper = candidate.to_ascii_uppercase();
            if !SKIP.contains(&upper.as_str()) && !candidate.starts_with('(') {
                let table = candidate.split('.').next_back().unwrap_or(candidate);
                if !table.is_empty() && !tables.iter().any(|t: &String| t.eq_ignore_ascii_case(table)) {
                    tables.push(table.to_string());
                }
                i += 1;
            }
        }
        i += 1;
    }
    tables
}

fn extract_alias_map(query: &str) -> HashMap<String, String> {
    const TABLE_CTX: &[&str] = &["FROM", "JOIN"];
    const STOP: &[&str] = &[
        "SELECT",
        "FROM",
        "WHERE",
        "JOIN",
        "ON",
        "HAVING",
        "GROUP",
        "ORDER",
        "LIMIT",
        "OFFSET",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "CROSS",
        "FULL",
        "NATURAL",
        "SET",
        "INTO",
        "VALUES",
        "UPDATE",
        "INSERT",
        "DELETE",
        "CREATE",
        "WITH",
        "UNION",
        "INTERSECT",
        "EXCEPT",
        "AND",
        "OR",
    ];
    let mut aliases: HashMap<String, String> = HashMap::new();
    let tokens: Vec<&str> = sql_tokens(query).collect();
    let mut i = 0;
    while i < tokens.len() {
        if TABLE_CTX.contains(&tokens[i].to_ascii_uppercase().as_str()) {
            if let Some(table_tok) = tokens.get(i + 1) {
                let table_upper = table_tok.to_ascii_uppercase();
                if !STOP.contains(&table_upper.as_str()) && !table_tok.starts_with('(') {
                    let table_name = table_tok.split('.').next_back().unwrap_or(table_tok).to_string();
                    let alias_pos = if tokens.get(i + 2).map(|t| t.eq_ignore_ascii_case("AS")).unwrap_or(false) {
                        i + 3
                    } else {
                        i + 2
                    };
                    if let Some(alias_tok) = tokens.get(alias_pos) {
                        let alias_upper = alias_tok.to_ascii_uppercase();
                        if !STOP.contains(&alias_upper.as_str())
                            && !TABLE_CTX.contains(&alias_upper.as_str())
                            && alias_tok.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            aliases.insert(alias_tok.to_ascii_lowercase(), table_name);
                        }
                    }
                }
            }
        }
        i += 1;
    }
    aliases
}

fn sql_tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|s| !s.is_empty())
}

pub(super) fn render_completion_popup(
    frame: &mut Frame,
    full_area: Rect,
    popup_x: u16,
    popup_y: u16,
    state: &CompletionState,
    theme: &kubetile_tui::theme::Theme,
) {
    if popup_y >= full_area.y + full_area.height {
        return;
    }

    let max_item_len = state.items.iter().map(|s| s.len()).max().unwrap_or(4);
    let popup_w = ((max_item_len + 2) as u16).clamp(10, 40);
    let popup_h = (state.items.len() as u16).min((full_area.y + full_area.height).saturating_sub(popup_y));

    if popup_h == 0 {
        return;
    }

    let max_x = full_area.x + full_area.width;
    let popup_x = popup_x.min(max_x.saturating_sub(popup_w));

    let popup = Rect { x: popup_x, y: popup_y, width: popup_w, height: popup_h };
    frame.render_widget(Clear, popup);

    let inner_w = popup_w as usize;
    let lines: Vec<Line> = state
        .items
        .iter()
        .enumerate()
        .take(popup_h as usize)
        .map(|(i, item)| {
            let style = if i == state.selected { theme.selection } else { theme.overlay };
            let text = format!(" {:<width$}", item, width = inner_w.saturating_sub(1));
            Line::from(Span::styled(text, style))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).style(theme.overlay), popup);
}
