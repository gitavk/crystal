use ratatui::prelude::*;
use ratatui::style::Modifier;

fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::new();

    let (fg, bg) = if cell.inverse() {
        (convert_color(cell.bgcolor()), convert_color(cell.fgcolor()))
    } else {
        (convert_color(cell.fgcolor()), convert_color(cell.bgcolor()))
    };
    style = style.fg(fg).bg(bg);

    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    style
}

/// Convert vt100::Screen state to ratatui drawable content.
/// Pure function: takes screen state and area, renders into the frame.
pub fn render_terminal_screen(screen: &vt100::Screen, area: Rect, buf: &mut Buffer) {
    let (screen_rows, screen_cols) = screen.size();
    let visible_rows = area.height.min(screen_rows);
    let visible_cols = area.width.min(screen_cols);

    for row in 0..visible_rows {
        let mut col: u16 = 0;
        let mut spans: Vec<Span> = Vec::new();

        while col < visible_cols {
            if let Some(cell) = screen.cell(row, col) {
                if cell.is_wide_continuation() {
                    col += 1;
                    continue;
                }
                let content = cell.contents();
                let style = cell_style(cell);
                let text = if content.is_empty() { " ".to_string() } else { content.to_string() };
                spans.push(Span::styled(text, style));
                if cell.is_wide() {
                    col += 2;
                } else {
                    col += 1;
                }
            } else {
                spans.push(Span::raw(" "));
                col += 1;
            }
        }

        let line = Line::from(spans);
        let x = area.x;
        let y = area.y + row;
        if y < area.y + area.height {
            buf.set_line(x, y, &line, area.width);
        }
    }

    if !screen.hide_cursor() {
        let (cursor_row, cursor_col) = screen.cursor_position();
        let x = area.x + cursor_col;
        let y = area.y + cursor_row;
        if x < area.x + area.width && y < area.y + area.height {
            let cell_ref = buf.cell_mut((x, y));
            if let Some(buf_cell) = cell_ref {
                buf_cell.set_style(Style::new().add_modifier(Modifier::REVERSED));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_screen(rows: u16, cols: u16, input: &[u8]) -> vt100::Parser {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(input);
        parser
    }

    fn render_to_buf(screen: &vt100::Screen, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_terminal_screen(screen, area, &mut buf);
        buf
    }

    #[test]
    fn empty_screen_renders_blank_spans() {
        let parser = make_screen(24, 80, b"");
        let buf = render_to_buf(parser.screen(), 80, 24);
        for y in 0..24 {
            for x in 0..80u16 {
                let cell = &buf[(x, y)];
                assert_eq!(cell.symbol(), " ");
            }
        }
    }

    #[test]
    fn single_line_text_produces_correct_spans() {
        let parser = make_screen(24, 80, b"Hello");
        let buf = render_to_buf(parser.screen(), 80, 24);
        assert_eq!(buf[(0, 0)].symbol(), "H");
        assert_eq!(buf[(1, 0)].symbol(), "e");
        assert_eq!(buf[(2, 0)].symbol(), "l");
        assert_eq!(buf[(3, 0)].symbol(), "l");
        assert_eq!(buf[(4, 0)].symbol(), "o");
        assert_eq!(buf[(5, 0)].symbol(), " ");
    }

    #[test]
    fn bold_red_foreground_maps_correctly() {
        // ESC[1;31m = bold + red fg
        let parser = make_screen(24, 80, b"\x1b[1;31mHi\x1b[0m");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "H");
        assert_eq!(cell.fg, Color::Indexed(1));
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn color_256_index_maps_correctly() {
        // ESC[38;5;142m = 256-color fg index 142
        let parser = make_screen(24, 80, b"\x1b[38;5;142mX\x1b[0m");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "X");
        assert_eq!(cell.fg, Color::Indexed(142));
    }

    #[test]
    fn truecolor_rgb_maps_correctly() {
        // ESC[38;2;100;200;50m = truecolor fg
        let parser = make_screen(24, 80, b"\x1b[38;2;100;200;50mR\x1b[0m");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "R");
        assert_eq!(cell.fg, Color::Rgb(100, 200, 50));
    }

    #[test]
    fn inverse_attribute_swaps_fg_bg() {
        // ESC[7m = inverse, ESC[31m = red fg
        let parser = make_screen(24, 80, b"\x1b[31;7mI\x1b[0m");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "I");
        // With inverse: fg becomes bg, bg becomes fg
        // Red fg + default bg + inverse → fg=default(Reset), bg=red
        assert_eq!(cell.bg, Color::Indexed(1));
        assert_eq!(cell.fg, Color::Reset);
    }

    #[test]
    fn cursor_position_renders_highlight() {
        // Write "AB" — cursor ends at (0, 2)
        let parser = make_screen(24, 80, b"AB");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(2, 0)];
        assert!(cell.modifier.contains(Modifier::REVERSED));
        // Non-cursor cell should not have REVERSED
        let other = &buf[(0, 0)];
        assert!(!other.modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn red_hello_default_world() {
        // ESC[31mHello ESC[0m World
        let parser = make_screen(24, 80, b"\x1b[31mHello\x1b[0m World");
        let buf = render_to_buf(parser.screen(), 80, 24);
        // "Hello" in red
        for i in 0..5u16 {
            assert_eq!(buf[(i, 0)].fg, Color::Indexed(1));
        }
        // " World" in default
        for i in 5..11u16 {
            assert_eq!(buf[(i, 0)].fg, Color::Reset);
        }
    }

    #[test]
    fn bold_and_underline_modifiers() {
        // ESC[1;4m = bold + underline
        let parser = make_screen(24, 80, b"\x1b[1;4mBU\x1b[0m");
        let buf = render_to_buf(parser.screen(), 80, 24);
        let cell = &buf[(0, 0)];
        assert!(cell.modifier.contains(Modifier::BOLD));
        assert!(cell.modifier.contains(Modifier::UNDERLINED));
    }
}
