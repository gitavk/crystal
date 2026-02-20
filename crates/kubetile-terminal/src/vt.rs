pub struct VtParser {
    parser: vt100::Parser,
}

impl VtParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { parser: vt100::Parser::new(rows, cols, 0) }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_parser_with_given_size() {
        let vt = VtParser::new(24, 80);
        let (rows, cols) = vt.screen().size();
        assert_eq!(rows, 24);
        assert_eq!(cols, 80);
    }

    #[test]
    fn process_updates_screen_contents() {
        let mut vt = VtParser::new(24, 80);
        vt.process(b"Hello, world!");
        let contents = vt.screen().contents();
        assert!(contents.starts_with("Hello, world!"));
    }

    #[test]
    fn resize_changes_screen_dimensions() {
        let mut vt = VtParser::new(24, 80);
        vt.resize(40, 120);
        let (rows, cols) = vt.screen().size();
        assert_eq!(rows, 40);
        assert_eq!(cols, 120);
    }
}
