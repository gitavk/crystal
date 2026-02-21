use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn normalize_key_event(key: KeyEvent) -> KeyEvent {
    if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT) {
        let mut modifiers = key.modifiers;
        modifiers -= KeyModifiers::SHIFT;
        return KeyEvent::new(KeyCode::BackTab, modifiers);
    }
    if key.code == KeyCode::BackTab && key.modifiers.contains(KeyModifiers::SHIFT) {
        let mut modifiers = key.modifiers;
        modifiers -= KeyModifiers::SHIFT;
        return KeyEvent::new(KeyCode::BackTab, modifiers);
    }
    // Normalize Shift+char: crossterm may report Shift+'g' or just 'G' with SHIFT.
    // Canonicalize to uppercase char + SHIFT modifier.
    if let KeyCode::Char(c) = key.code {
        // In most terminals Ctrl+Shift+<letter> is indistinguishable from Ctrl+<letter>.
        // Canonicalize all Ctrl+letter to lowercase without SHIFT for stable matching.
        if key.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() {
            let mut modifiers = key.modifiers;
            modifiers -= KeyModifiers::SHIFT;
            return KeyEvent::new(KeyCode::Char(c.to_ascii_lowercase()), modifiers);
        }
        if c.is_ascii_lowercase() && key.modifiers.contains(KeyModifiers::SHIFT) {
            return KeyEvent::new(KeyCode::Char(c.to_ascii_uppercase()), key.modifiers);
        }
        if c.is_ascii_uppercase() && !key.modifiers.contains(KeyModifiers::SHIFT) {
            return KeyEvent::new(key.code, key.modifiers | KeyModifiers::SHIFT);
        }
    }
    key
}

pub(super) fn format_key_display(key_str: &str) -> String {
    key_str
        .split('+')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub(super) fn key_to_input_string(key: KeyEvent) -> String {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            return String::from(byte as char);
        }
    }

    match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "\r".into(),
        KeyCode::Tab => "\t".into(),
        KeyCode::Backspace => "\x7f".into(),
        KeyCode::Esc => "\x1b".into(),
        KeyCode::Up => "\x1b[A".into(),
        KeyCode::Down => "\x1b[B".into(),
        KeyCode::Right => "\x1b[C".into(),
        KeyCode::Left => "\x1b[D".into(),
        KeyCode::Home => "\x1b[H".into(),
        KeyCode::End => "\x1b[F".into(),
        KeyCode::PageUp => "\x1b[5~".into(),
        KeyCode::PageDown => "\x1b[6~".into(),
        KeyCode::Delete => "\x1b[3~".into(),
        KeyCode::F(n) => match n {
            1 => "\x1bOP".into(),
            2 => "\x1bOQ".into(),
            3 => "\x1bOR".into(),
            4 => "\x1bOS".into(),
            5 => "\x1b[15~".into(),
            6 => "\x1b[17~".into(),
            7 => "\x1b[18~".into(),
            8 => "\x1b[19~".into(),
            9 => "\x1b[20~".into(),
            10 => "\x1b[21~".into(),
            11 => "\x1b[23~".into(),
            12 => "\x1b[24~".into(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

pub fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;

    let key_part_raw = if parts.len() == 1 {
        parts[0]
    } else {
        for &modifier in &parts[..parts.len() - 1] {
            match modifier.to_ascii_lowercase().as_str() {
                "alt" => modifiers |= KeyModifiers::ALT,
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return None,
            }
        }
        parts[parts.len() - 1]
    };

    let key_lower = key_part_raw.to_ascii_lowercase();
    let code = match key_lower.as_str() {
        "tab" if modifiers.contains(KeyModifiers::SHIFT) => {
            modifiers -= KeyModifiers::SHIFT;
            KeyCode::BackTab
        }
        "tab" => KeyCode::Tab,
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        _ if key_part_raw.len() == 1 => {
            let ch = key_part_raw.chars().next().unwrap();
            if ch.is_ascii_uppercase() {
                modifiers |= KeyModifiers::SHIFT;
                KeyCode::Char(ch)
            } else if modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_lowercase() {
                KeyCode::Char(ch.to_ascii_uppercase())
            } else {
                KeyCode::Char(ch)
            }
        }
        s if s.starts_with('f') => {
            let n: u8 = s[1..].parse().ok()?;
            KeyCode::F(n)
        }
        _ => return None,
    };

    let parsed = KeyEvent::new(code, modifiers);
    Some(normalize_key_event(parsed))
}
