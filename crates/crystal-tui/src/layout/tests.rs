use super::*;

#[test]
fn render_context_requires_theme() {
    let theme = Theme::default();
    let pane_tree = PaneTree::new(crate::pane::ViewType::Empty);
    let panes = std::collections::HashMap::new();
    let ctx = RenderContext {
        cluster_name: None,
        namespace: None,
        namespace_selector: None,
        context_selector: None,
        resource_switcher: None,
        confirm_dialog: None,
        port_forward_dialog: None,
        toasts: &[],
        pane_tree: &pane_tree,
        focused_pane: None,
        fullscreen_pane: None,
        panes: &panes,
        tab_names: &[],
        active_tab: 0,
        mode_name: "Normal",
        mode_hints: &[],
        theme: &theme,
    };
    assert_eq!(ctx.active_tab, 0);
}
