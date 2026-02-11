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
            let widget = StatusBarWidget { mode, hints, cluster, namespace };
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
