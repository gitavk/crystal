use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_status_bar(
    mode: &str,
    hints: &[(String, String)],
    cluster: Option<&str>,
    namespace: Option<&str>,
    width: u16,
) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            let theme = Theme::default();
            let widget = StatusBarWidget { mode, hints, cluster, namespace, theme: &theme };
            widget.render(frame, area);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
    buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect()
}

#[test]
fn shows_hints() {
    let hints = vec![("Alt+v".into(), "Split V".into()), ("?".into(), "Help".into())];
    let buf = render_status_bar("Normal", &hints, Some("minikube"), Some("default"), 120);
    let text = buf_text(&buf);
    assert!(text.contains("NORMAL"));
    assert!(text.contains("<Alt+v>"));
    assert!(text.contains("Split V"));
    assert!(text.contains("<?>"));
    assert!(text.contains("Help"));
}

#[test]
fn shows_cluster_info() {
    let buf = render_status_bar("Normal", &[], Some("minikube"), Some("default"), 80);
    let text = buf_text(&buf);
    assert!(text.contains("minikube / default"));
}

#[test]
fn shows_no_cluster_when_disconnected() {
    let buf = render_status_bar("Normal", &[], None, None, 80);
    let text = buf_text(&buf);
    assert!(text.contains("No cluster"));
}

#[test]
fn mode_label_is_uppercased() {
    let buf = render_status_bar("Normal", &[], Some("ctx"), Some("ns"), 80);
    let text = buf_text(&buf);
    assert!(text.contains("NORMAL"));
}

#[test]
fn shows_insert_mode_label() {
    let hints = vec![("Esc".into(), "Normal mode".into())];
    let buf = render_status_bar("Insert", &hints, Some("minikube"), Some("default"), 120);
    let text = buf_text(&buf);
    assert!(text.contains("INSERT"));
    assert!(text.contains("<Esc>"));
    assert!(text.contains("Normal mode"));
}

#[test]
fn insert_mode_has_distinct_style() {
    let buf_normal = render_status_bar("Normal", &[], Some("ctx"), Some("ns"), 80);
    let buf_insert = render_status_bar("Insert", &[], Some("ctx"), Some("ns"), 80);

    let normal_bg = buf_normal.cell((1, 0)).unwrap().bg;
    let insert_bg = buf_insert.cell((1, 0)).unwrap().bg;
    assert_ne!(normal_bg, insert_bg, "Insert mode should have a different background color");
}

#[test]
fn shows_normal_mode_label() {
    let buf = render_status_bar("Normal", &[], Some("ctx"), Some("ns"), 80);
    let text = buf_text(&buf);
    assert!(text.contains("NORMAL"));
    assert!(!text.contains("INSERT"));
}
