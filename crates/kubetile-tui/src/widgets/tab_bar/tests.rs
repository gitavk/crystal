use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_tab_bar(tabs: &[String], active: usize, width: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            let theme = Theme::default();
            let widget = TabBarWidget { tabs, active, theme: &theme };
            widget.render(frame, area);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
    buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect()
}

#[test]
fn renders_correct_number_of_tabs() {
    let tabs: Vec<String> = vec!["Pods".into(), "Services".into(), "Terminal".into()];
    let buf = render_tab_bar(&tabs, 0, 60);
    let content = buf_text(&buf);
    assert!(content.contains("[1] Pods"));
    assert!(content.contains("[2] Services"));
    assert!(content.contains("[3] Terminal"));
}

#[test]
fn active_tab_is_visually_distinct() {
    let tabs: Vec<String> = vec!["Pods".into(), "Services".into()];
    let buf = render_tab_bar(&tabs, 1, 40);

    let pods_cell = &buf.cell((0, 0)).unwrap();
    let pods_fg = pods_cell.fg;

    let sep_width = " │ ".len() as u16;
    let first_tab_width = "[1] Pods".len() as u16;
    let services_x = first_tab_width + sep_width;
    let services_cell = &buf.cell((services_x, 0)).unwrap();
    let services_fg = services_cell.fg;

    assert_ne!(pods_fg, services_fg, "active and inactive tabs should have different colors");
}

#[test]
fn single_tab_renders() {
    let tabs: Vec<String> = vec!["Main".into()];
    let buf = render_tab_bar(&tabs, 0, 30);
    let content = buf_text(&buf);
    assert!(content.contains("[1] Main"));
    assert!(!content.contains("│"));
}

#[test]
fn scrolls_to_show_active_tab() {
    let tabs: Vec<String> = (1..=10).map(|i| format!("Tab-{i}")).collect();
    // Each label is "[N] Tab-N" (~11 chars) + separator " │ " (3 chars)
    // 10 tabs won't fit in 40 columns
    let buf = render_tab_bar(&tabs, 8, 40);
    let content = buf_text(&buf);
    assert!(content.contains("[9] Tab-9"), "active tab 9 should be visible");
    assert!(!content.contains("[1] Tab-1"), "first tab should be scrolled away");
}

#[test]
fn no_scroll_when_all_fit() {
    let tabs: Vec<String> = vec!["A".into(), "B".into()];
    let buf = render_tab_bar(&tabs, 1, 40);
    let content = buf_text(&buf);
    assert!(content.contains("[1] A"));
    assert!(content.contains("[2] B"));
}
