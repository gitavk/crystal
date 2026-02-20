use super::*;
use crate::pane::ResourceKind;

#[test]
fn registry_returns_render_fn_for_known_types() {
    let registry = ViewRegistry::new();
    assert!(registry.get(&ViewType::Empty).is_some());
    assert!(registry.get(&ViewType::Help).is_some());
}

#[test]
fn registry_returns_none_for_unregistered_types() {
    let registry = ViewRegistry::new();
    assert!(registry.get(&ViewType::Terminal).is_none());
    assert!(registry.get(&ViewType::ResourceList(ResourceKind::Pods)).is_none());
}

#[test]
fn custom_renderer_can_be_registered() {
    let mut registry = ViewRegistry::new();
    fn custom_render(_frame: &mut Frame, _area: Rect, _focused: bool, _theme: &Theme) {}
    registry.register("Terminal", custom_render);
    assert!(registry.get(&ViewType::Terminal).is_some());
}
