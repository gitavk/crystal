use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(widget: &StatusBarWidget, width: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            widget.render(frame, frame.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
    buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect()
}

fn default_widget(theme: &Theme) -> StatusBarWidget<'_> {
    StatusBarWidget {
        mode: "Normal",
        context: Some("minikube"),
        help_key: Some("F1"),
        namespace_key: Some("Ctrl+N"),
        context_key: Some("Ctrl+K"),
        close_pane_key: Some("Alt+X"),
        new_tab_key: Some("Ctrl+T"),
        quit_key: Some("Ctrl+Q"),
        theme,
    }
}

#[test]
fn shows_mode_and_context() {
    let theme = Theme::default();
    let w = default_widget(&theme);
    let text = buf_text(&render(&w, 120));
    assert!(text.contains("NORMAL"));
    assert!(text.contains("minikube"));
}

#[test]
fn shows_keybindings() {
    let theme = Theme::default();
    let w = default_widget(&theme);
    let text = buf_text(&render(&w, 120));
    assert!(text.contains("F1"));
    assert!(text.contains("Help"));
    assert!(text.contains("Ctrl+N"));
    assert!(text.contains("Namespace"));
    assert!(text.contains("Ctrl+K"));
    assert!(text.contains("Context"));
    assert!(text.contains("Alt+X"));
    assert!(text.contains("Close pane"));
    assert!(text.contains("Ctrl+T"));
    assert!(text.contains("New tab"));
    assert!(text.contains("Ctrl+Q"));
    assert!(text.contains("Quit"));
}

#[test]
fn shows_no_context_when_disconnected() {
    let theme = Theme::default();
    let mut w = default_widget(&theme);
    w.context = None;
    let text = buf_text(&render(&w, 80));
    assert!(text.contains("no-context"));
}

#[test]
fn mode_label_is_uppercased() {
    let theme = Theme::default();
    let w = default_widget(&theme);
    let text = buf_text(&render(&w, 80));
    assert!(text.contains("NORMAL"));
}

#[test]
fn shows_insert_mode_label() {
    let theme = Theme::default();
    let mut w = default_widget(&theme);
    w.mode = "Insert";
    let text = buf_text(&render(&w, 120));
    assert!(text.contains("INSERT"));
}

#[test]
fn insert_mode_has_distinct_style() {
    let theme = Theme::default();
    let normal = default_widget(&theme);
    let mut insert = default_widget(&theme);
    insert.mode = "Insert";

    let buf_normal = render(&normal, 80);
    let buf_insert = render(&insert, 80);

    let normal_bg = buf_normal.cell((1, 0)).unwrap().bg;
    let insert_bg = buf_insert.cell((1, 0)).unwrap().bg;
    assert_ne!(normal_bg, insert_bg, "Insert mode should have a different background color");
}
