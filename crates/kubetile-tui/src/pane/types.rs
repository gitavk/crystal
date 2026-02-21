pub type PaneId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneCommand {
    ScrollUp,
    ScrollDown,
    SelectNext,
    SelectPrev,
    Select,
    Back,
    GoToTop,
    GoToBottom,
    PageUp,
    PageDown,
    ToggleFollow,
    ToggleWrap,
    ScrollLeft,
    ScrollRight,
    SendInput(String),
    SearchInput(char),
    SearchConfirm,
    SearchClear,

    Filter(String),
    ClearFilter,
    SortByColumn(usize),
    ToggleSortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // top/bottom
    Vertical,   // left/right
}
