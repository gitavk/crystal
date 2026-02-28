use ratatui::prelude::*;

use super::QueryPane;

impl QueryPane {
    pub fn set_editor_content(&mut self, sql: &str) {
        self.editor_lines = sql.split('\n').map(|s| s.to_string()).collect();
        if self.editor_lines.is_empty() {
            self.editor_lines = vec![String::new()];
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.editor_scroll = 0;
    }

    pub fn editor_push(&mut self, c: char) {
        let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col);
        self.editor_lines[self.cursor_row].insert(byte, c);
        self.cursor_col += 1;
    }

    pub fn editor_pop(&mut self) {
        if self.cursor_col > 0 {
            let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col - 1);
            self.editor_lines[self.cursor_row].remove(byte);
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            let current = self.editor_lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
            self.editor_lines[self.cursor_row].push_str(&current);
            self.adjust_editor_scroll();
        }
    }

    pub fn editor_newline(&mut self) {
        let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col);
        let tail = self.editor_lines[self.cursor_row].split_off(byte);
        self.cursor_row += 1;
        self.editor_lines.insert(self.cursor_row, tail);
        self.cursor_col = 0;
        self.adjust_editor_scroll();
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let max_col = self.editor_lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(max_col);
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor_row + 1 < self.editor_lines.len() {
            self.cursor_row += 1;
            let max_col = self.editor_lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(max_col);
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_right(&mut self) {
        let line_len = self.editor_lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.editor_lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.adjust_editor_scroll();
        }
    }

    pub fn editor_indent(&mut self) {
        self.editor_lines[self.cursor_row].insert_str(0, "  ");
        self.cursor_col += 2;
    }

    pub fn editor_deindent(&mut self) {
        let spaces = self.editor_lines[self.cursor_row].chars().take(2).take_while(|&c| c == ' ').count();
        if spaces == 0 {
            return;
        }
        self.editor_lines[self.cursor_row].drain(..spaces);
        self.cursor_col = self.cursor_col.saturating_sub(spaces);
    }

    pub fn editor_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn editor_end(&mut self) {
        self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
    }

    pub fn editor_content(&self) -> String {
        self.editor_lines.join("\n")
    }

    pub(super) fn adjust_editor_scroll(&mut self) {
        let h = self.editor_area_height.get().max(1);
        if self.cursor_row < self.editor_scroll {
            self.editor_scroll = self.cursor_row;
        } else if self.cursor_row >= self.editor_scroll + h {
            self.editor_scroll = self.cursor_row + 1 - h;
        }
    }
}

pub(super) fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

pub(super) fn render_cursor_line(
    line: &str,
    cursor_col: usize,
    normal_style: Style,
    cursor_style: Style,
) -> Line<'static> {
    let char_count = line.chars().count();
    let byte = char_to_byte(line, cursor_col);
    let before = line[..byte].to_string();
    let (cursor_ch, after) = if cursor_col < char_count {
        let ch = line[byte..].chars().next().unwrap();
        (ch.to_string(), line[byte + ch.len_utf8()..].to_string())
    } else {
        (" ".to_string(), String::new())
    };
    Line::from(vec![
        Span::styled(before, normal_style),
        Span::styled(cursor_ch, cursor_style),
        Span::styled(after, normal_style),
    ])
}
