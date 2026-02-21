use super::*;

fn make_lines(n: usize) -> Vec<LogLineRef> {
    (0..n)
        .map(|i| LogLineRef {
            timestamp: Some(format!("2024-01-15T10:30:{:02}.000Z", i % 60)),
            content: format!("log line {i}"),
            container: "main".into(),
            is_stderr: false,
        })
        .collect()
}

fn make_multi_container_lines() -> Vec<LogLineRef> {
    vec![
        LogLineRef {
            timestamp: Some("2024-01-15T10:30:00.000Z".into()),
            content: "from main".into(),
            container: "main".into(),
            is_stderr: false,
        },
        LogLineRef {
            timestamp: Some("2024-01-15T10:30:01.000Z".into()),
            content: "from sidecar".into(),
            container: "sidecar".into(),
            is_stderr: false,
        },
    ]
}

#[test]
fn title_format() {
    let view = LogsView::new(1, "my-pod".into(), "default".into());
    assert_eq!(view.title(), "[logs:my-pod @ default]");
}

#[test]
fn toggle_follow() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    assert!(view.auto_scroll());
    view.toggle_follow();
    assert!(!view.auto_scroll());
    view.toggle_follow();
    assert!(view.auto_scroll());
}

#[test]
fn toggle_timestamps() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    assert!(view.show_timestamps());
    view.toggle_timestamps();
    assert!(!view.show_timestamps());
}

#[test]
fn toggle_wrap() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    assert!(!view.wrap_lines());
    view.toggle_wrap();
    assert!(view.wrap_lines());
}

#[test]
fn filter_hides_non_matching_lines() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = vec![
        LogLineRef {
            timestamp: None,
            content: "error: something failed".into(),
            container: "main".into(),
            is_stderr: false,
        },
        LogLineRef { timestamp: None, content: "info: all good".into(), container: "main".into(), is_stderr: false },
        LogLineRef {
            timestamp: None,
            content: "error: another failure".into(),
            container: "main".into(),
            is_stderr: false,
        },
    ];

    view.set_filter(Some("error".into()));
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 25);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 25), true, None, &theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let status_row = 24u16;
    let mut status_text = String::new();
    for x in 0..80u16 {
        status_text.push_str(buf[(x, status_row)].symbol());
    }
    assert!(status_text.contains("2/3 lines"), "status should show filtered count: {status_text}");
}

#[test]
fn auto_scroll_follows_new_lines_at_bottom() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    assert!(view.auto_scroll());
    assert_eq!(view.scroll_offset(), 0);

    let lines = make_lines(100);
    let theme = Theme::default();
    let backend = ratatui::backend::TestBackend::new(80, 25);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 25), true, None, &theme);
        })
        .unwrap();

    assert_eq!(view.scroll_offset(), 0);
    assert!(view.auto_scroll());
}

#[test]
fn scroll_up_pauses_auto_scroll() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    assert!(view.auto_scroll());
    view.scroll_up(5);
    assert!(!view.auto_scroll());
    assert_eq!(view.scroll_offset(), 5);
}

#[test]
fn scroll_down_to_bottom_resumes_auto_scroll() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_up(10);
    assert!(!view.auto_scroll());
    view.scroll_down(10);
    assert!(view.auto_scroll());
    assert_eq!(view.scroll_offset(), 0);
}

#[test]
fn scroll_to_top_sets_max_offset() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_to_top();
    assert!(!view.auto_scroll());
    assert_eq!(view.scroll_offset(), usize::MAX);
}

#[test]
fn scroll_to_bottom_resets() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_up(42);
    view.scroll_to_bottom();
    assert!(view.auto_scroll());
    assert_eq!(view.scroll_offset(), 0);
}

#[test]
fn scrolling_clamps_to_bounds() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_up(1000);

    let lines = make_lines(10);
    let theme = Theme::default();
    let backend = ratatui::backend::TestBackend::new(80, 25);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 12), true, None, &theme);
        })
        .unwrap();

    assert_eq!(view.scroll_offset(), 0);
}

#[test]
fn timestamp_toggle_affects_rendering() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = make_lines(1);
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 5);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    view.show_timestamps = true;
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 5), true, None, &theme);
        })
        .unwrap();
    let buf_with_ts = terminal.backend().buffer().clone();

    view.show_timestamps = false;
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 5), true, None, &theme);
        })
        .unwrap();
    let buf_without_ts = terminal.backend().buffer().clone();

    let content_start_with_ts = find_text_position(&buf_with_ts, 1, "log line 0");
    let content_start_without_ts = find_text_position(&buf_without_ts, 1, "log line 0");
    assert!(content_start_with_ts.unwrap_or(0) > content_start_without_ts.unwrap_or(0));
}

#[test]
fn container_filter_shows_only_selected() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = make_multi_container_lines();

    view.set_container_filter(Some("sidecar".into()));
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 10), true, None, &theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let mut content = String::new();
    for y in 1..9u16 {
        for x in 0..80u16 {
            content.push_str(buf[(x, y)].symbol());
        }
    }
    assert!(content.contains("from sidecar"));
    assert!(!content.contains("from main"));
}

#[test]
fn tiny_area_does_not_panic() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = make_lines(5);
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 25);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 5, 2), true, None, &theme);
        })
        .unwrap();
}

#[test]
fn unfocused_dims_content() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = make_lines(3);
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 10), false, None, &theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let cell = &buf[(0, 1)];
    let text_dim_color = theme.text_dim.fg.unwrap_or(Color::Reset);
    assert_eq!(cell.fg, text_dim_color);
}

#[test]
fn accessors_return_correct_values() {
    let view = LogsView::new(42, "my-pod".into(), "kube-system".into());
    assert_eq!(view.stream_id(), 42);
    assert_eq!(view.pod_name(), "my-pod");
    assert_eq!(view.namespace(), "kube-system");
}

#[test]
fn has_multiple_containers_detects_multi() {
    let single = make_lines(3);
    assert!(!has_multiple_containers(&single));

    let multi = make_multi_container_lines();
    assert!(has_multiple_containers(&multi));

    let empty: Vec<LogLineRef> = Vec::new();
    assert!(!has_multiple_containers(&empty));
}

#[test]
fn highlight_matches_finds_occurrences() {
    let style = Style::new();
    let theme = Theme::default();
    let spans = highlight_matches("hello error world error", "error", style, &theme);
    assert!(spans.len() >= 4);
}

#[test]
fn container_color_is_deterministic() {
    let c1 = container_color("main");
    let c2 = container_color("main");
    assert_eq!(c1, c2);
}

#[test]
fn container_color_varies_by_name() {
    let c1 = container_color("main");
    let c2 = container_color("sidecar");
    assert_ne!(c1, c2);
}

#[test]
fn set_filter_resets_scroll() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_up(10);
    view.set_filter(Some("test".into()));
    assert_eq!(view.scroll_offset(), 0);
}

#[test]
fn set_container_filter_resets_scroll() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    view.scroll_up(10);
    view.set_container_filter(Some("main".into()));
    assert_eq!(view.scroll_offset(), 0);
}

#[test]
fn status_bar_shows_reconnecting() {
    let mut view = LogsView::new(1, "pod".into(), "ns".into());
    let lines = make_lines(5);
    let theme = Theme::default();

    let backend = ratatui::backend::TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            view.render(&lines, frame, Rect::new(0, 0, 80, 10), true, Some("Reconnecting..."), &theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let status_row = 9u16;
    let mut status = String::new();
    for x in 0..80u16 {
        status.push_str(buf[(x, status_row)].symbol());
    }
    assert!(status.contains("Reconnecting..."));
}

fn find_text_position(buf: &Buffer, row: u16, text: &str) -> Option<u16> {
    let mut row_text = String::new();
    for x in 0..buf.area.width {
        row_text.push_str(buf[(x, row)].symbol());
    }
    row_text.find(text).map(|p| p as u16)
}
