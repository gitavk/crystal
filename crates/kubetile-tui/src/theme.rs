use kubetile_config::ThemeConfig;
use ratatui::style::{Color, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub bg: Color,
    pub fg: Color,
    pub header: Style,
    pub status_bar: Style,
    pub selection: Style,
    pub border: Style,
    pub border_active: Style,
    pub text_dim: Style,
    pub overlay: Style,
    pub status_running: Style,
    pub status_pending: Style,
    pub status_failed: Style,
    pub status_unknown: Style,
    pub yaml_key: Style,
    pub yaml_string: Style,
    pub yaml_number: Style,
    pub yaml_boolean: Style,
    pub yaml_null: Style,
    pub insert_mode: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        let accent = parse_color_or_default(&config.accent);
        let bg = parse_color_or_default(&config.bg);
        let fg = parse_color_or_default(&config.fg);
        let header_bg = parse_color_or_default(&config.header_bg);
        let header_fg = parse_color_or_default(&config.header_fg);
        let selection_bg = parse_color_or_default(&config.selection_bg);
        let selection_fg = parse_color_or_default(&config.selection_fg);
        let border_color = parse_color_or_default(&config.border);
        let border_active_color = parse_color_or_default(&config.border_active);
        let text_dim_color = parse_color_or_default(&config.text_dim);
        let overlay_bg = parse_color_or_default(&config.overlay_bg);

        let status_running = parse_color_or_default(&config.status_running);
        let status_pending = parse_color_or_default(&config.status_pending);
        let status_failed = parse_color_or_default(&config.status_failed);
        let status_unknown = parse_color_or_default(&config.status_unknown);

        let yaml_key = parse_color_or_default(&config.yaml_key);
        let yaml_string = parse_color_or_default(&config.yaml_string);
        let yaml_number = parse_color_or_default(&config.yaml_number);
        let yaml_boolean = parse_color_or_default(&config.yaml_boolean);
        let yaml_null = parse_color_or_default(&config.yaml_null);

        let insert_mode_bg = parse_color_or_default(&config.insert_mode_bg);
        let insert_mode_fg = parse_color_or_default(&config.insert_mode_fg);

        Self {
            accent,
            bg,
            fg,
            header: Style::default().fg(header_fg).bg(header_bg),
            status_bar: Style::default().fg(header_fg).bg(header_bg),
            selection: Style::default().fg(selection_fg).bg(selection_bg),
            border: Style::default().fg(border_color),
            border_active: Style::default().fg(border_active_color),
            text_dim: Style::default().fg(text_dim_color),
            overlay: Style::default().bg(overlay_bg),
            status_running: Style::default().fg(status_running),
            status_pending: Style::default().fg(status_pending),
            status_failed: Style::default().fg(status_failed),
            status_unknown: Style::default().fg(status_unknown),
            yaml_key: Style::default().fg(yaml_key),
            yaml_string: Style::default().fg(yaml_string),
            yaml_number: Style::default().fg(yaml_number),
            yaml_boolean: Style::default().fg(yaml_boolean),
            yaml_null: Style::default().fg(yaml_null),
            insert_mode: Style::default().fg(insert_mode_fg).bg(insert_mode_bg),
        }
    }
}

fn parse_color_or_default(s: &str) -> Color {
    parse_color(s).unwrap_or(Color::Reset)
}

/// Parse a color string into a ratatui `Color`.
///
/// Supported formats:
/// - `"#89b4fa"` — hex RGB
/// - `"rgb(137,180,250)"` — functional RGB
/// - `"red"`, `"blue"`, etc. — named colors
/// - `"default"` — terminal default (`Color::Reset`)
pub fn parse_color(s: &str) -> anyhow::Result<Color> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("default") {
        return Ok(Color::Reset);
    }

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            anyhow::bail!("invalid hex color \"{s}\": expected 6 hex digits after '#'");
        }
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| anyhow::anyhow!("invalid hex color \"{s}\": bad red component"))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| anyhow::anyhow!("invalid hex color \"{s}\": bad green component"))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| anyhow::anyhow!("invalid hex color \"{s}\": bad blue component"))?;
        return Ok(Color::Rgb(r, g, b));
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() != 3 {
            anyhow::bail!("invalid rgb color \"{s}\": expected rgb(r,g,b)");
        }
        let r: u8 = parts[0].trim().parse().map_err(|_| anyhow::anyhow!("invalid rgb color \"{s}\": bad red value"))?;
        let g: u8 =
            parts[1].trim().parse().map_err(|_| anyhow::anyhow!("invalid rgb color \"{s}\": bad green value"))?;
        let b: u8 =
            parts[2].trim().parse().map_err(|_| anyhow::anyhow!("invalid rgb color \"{s}\": bad blue value"))?;
        return Ok(Color::Rgb(r, g, b));
    }

    match s.to_lowercase().as_str() {
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Ok(Color::DarkGray),
        "lightred" | "light_red" => Ok(Color::LightRed),
        "lightgreen" | "light_green" => Ok(Color::LightGreen),
        "lightyellow" | "light_yellow" => Ok(Color::LightYellow),
        "lightblue" | "light_blue" => Ok(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Ok(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        _ => anyhow::bail!(
            "unknown color \"{s}\": expected hex (#rrggbb), rgb(r,g,b), a named color (red, blue, ...), or \"default\""
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_color("#89b4fa").unwrap(), Color::Rgb(137, 180, 250));
        assert_eq!(parse_color("#000000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(parse_color("#ffffff").unwrap(), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_parse_rgb() {
        assert_eq!(parse_color("rgb(137,180,250)").unwrap(), Color::Rgb(137, 180, 250));
        assert_eq!(parse_color("rgb( 137 , 180 , 250 )").unwrap(), Color::Rgb(137, 180, 250));
    }

    #[test]
    fn test_parse_named() {
        assert_eq!(parse_color("red").unwrap(), Color::Red);
        assert_eq!(parse_color("Blue").unwrap(), Color::Blue);
        assert_eq!(parse_color("lightgreen").unwrap(), Color::LightGreen);
    }

    #[test]
    fn test_parse_default() {
        assert_eq!(parse_color("default").unwrap(), Color::Reset);
        assert_eq!(parse_color("Default").unwrap(), Color::Reset);
    }

    #[test]
    fn test_parse_invalid() {
        let err = parse_color("not-a-color").unwrap_err();
        assert!(err.to_string().contains("not-a-color"));
        assert!(err.to_string().contains("hex"));
    }

    #[test]
    fn test_parse_bad_hex() {
        assert!(parse_color("#zzzzzz").is_err());
        assert!(parse_color("#fff").is_err());
    }

    #[test]
    fn test_from_config_default_matches_old_consts() {
        let theme = Theme::from_config(&ThemeConfig::default());
        assert_eq!(theme.accent, Color::Rgb(137, 180, 250));
        assert_eq!(theme.bg, Color::Reset);
        assert_eq!(theme.fg, Color::Rgb(205, 214, 244));
        assert_eq!(theme.header, Style::default().fg(Color::Rgb(205, 214, 244)).bg(Color::Rgb(30, 30, 46)));
        assert_eq!(theme.status_running, Style::default().fg(Color::Rgb(166, 227, 161)));
        assert_eq!(theme.status_pending, Style::default().fg(Color::Rgb(249, 226, 175)));
        assert_eq!(theme.status_failed, Style::default().fg(Color::Rgb(243, 139, 168)));
        assert_eq!(theme.insert_mode, Style::default().fg(Color::Rgb(30, 30, 46)).bg(Color::Rgb(166, 227, 161)));
    }
}
