#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(self) -> i32 {
        self.right - self.left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapAction {
    Rect(Rect),
    Minimize,
}

pub const fn should_restore_before_snap(is_minimized: bool) -> bool {
    is_minimized
}

pub const fn should_process_snap(viewer_focused: bool, window_minimized: bool) -> bool {
    viewer_focused || window_minimized
}

pub fn snap_action(current: Rect, work: Rect, direction: SnapDirection) -> SnapAction {
    if matches!(direction, SnapDirection::Down) && current == work {
        SnapAction::Minimize
    } else {
        SnapAction::Rect(snap_rect(current, work, direction))
    }
}

pub fn snap_rect(current: Rect, work: Rect, direction: SnapDirection) -> Rect {
    let half_width = work.width() / 2;
    let half_height = (work.bottom - work.top) / 2;
    let is_half_width = (current.width() - half_width).abs() <= 2
        && (current.bottom - current.top - (work.bottom - work.top)).abs() <= 2;
    let on_left = current.left < work.left + half_width;

    match direction {
        SnapDirection::Left => Rect::new(work.left, work.top, work.left + half_width, work.bottom),
        SnapDirection::Right => {
            Rect::new(work.left + half_width, work.top, work.right, work.bottom)
        }
        SnapDirection::Up if is_half_width => Rect::new(
            if on_left {
                work.left
            } else {
                work.left + half_width
            },
            work.top,
            if on_left {
                work.left + half_width
            } else {
                work.right
            },
            work.top + half_height,
        ),
        SnapDirection::Down if is_half_width => Rect::new(
            if on_left {
                work.left
            } else {
                work.left + half_width
            },
            work.top + half_height,
            if on_left {
                work.left + half_width
            } else {
                work.right
            },
            work.bottom,
        ),
        SnapDirection::Up => work,
        SnapDirection::Down => work,
    }
}
