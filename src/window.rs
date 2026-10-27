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
    Noop,
}

pub const fn should_restore_before_snap(is_minimized: bool) -> bool {
    is_minimized
}

pub const fn should_process_snap(viewer_focused: bool, window_minimized: bool) -> bool {
    viewer_focused || window_minimized
}

pub const fn is_fullscreen(current: Rect, work: Rect) -> bool {
    current.left == work.left
        && current.top == work.top
        && current.right == work.right
        && current.bottom == work.bottom
}

pub const WS_CAPTION: u32 = 0x00C00000;
pub const WS_SYSMENU: u32 = 0x00080000;
pub const WS_THICKFRAME: u32 = 0x00040000;
pub const WS_MINIMIZEBOX: u32 = 0x00020000;
pub const WS_MAXIMIZEBOX: u32 = 0x00010000;
pub const WS_MAXIMIZE: u32 = 0x01000000;

pub const fn native_snap_style(style: u32) -> u32 {
    (style | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX) & !(WS_CAPTION | WS_SYSMENU)
}

pub const fn mark_native_maximized(style: u32) -> u32 {
    style | WS_MAXIMIZE
}

pub fn snap_action(current: Rect, work: Rect, direction: SnapDirection) -> SnapAction {
    let target = snap_rect(current, work, direction);
    if target == current {
        SnapAction::Noop
    } else {
        SnapAction::Rect(target)
    }
}

pub fn snap_rect(current: Rect, work: Rect, direction: SnapDirection) -> Rect {
    let half_width = work.width() / 2;
    let half_height = (work.bottom - work.top) / 2;
    let current_height = current.bottom - current.top;
    let is_half_width = (current.width() - half_width).abs() <= 2
        && (current_height - (work.bottom - work.top)).abs() <= 2;
    let is_quarter =
        (current.width() - half_width).abs() <= 2 && (current_height - half_height).abs() <= 2;
    let is_top_half = current == Rect::new(work.left, work.top, work.right, work.top + half_height);
    let is_bottom_half =
        current == Rect::new(work.left, work.top + half_height, work.right, work.bottom);
    let on_left = current.left < work.left + half_width;
    let on_top = current.top < work.top + half_height;

    match direction {
        SnapDirection::Left if is_top_half => Rect::new(
            work.left,
            work.top,
            work.left + half_width,
            work.top + half_height,
        ),
        SnapDirection::Right if is_top_half => Rect::new(
            work.left + half_width,
            work.top,
            work.right,
            work.top + half_height,
        ),
        SnapDirection::Left if is_quarter && on_top && !on_left => {
            Rect::new(work.left, work.top, work.right, work.top + half_height)
        }
        SnapDirection::Right if is_quarter && on_top && on_left => {
            Rect::new(work.left, work.top, work.right, work.top + half_height)
        }
        SnapDirection::Left if is_bottom_half => Rect::new(
            work.left,
            work.top + half_height,
            work.left + half_width,
            work.bottom,
        ),
        SnapDirection::Right if is_bottom_half => Rect::new(
            work.left + half_width,
            work.top + half_height,
            work.right,
            work.bottom,
        ),
        SnapDirection::Left if is_quarter && !on_top && !on_left => {
            Rect::new(work.left, work.top + half_height, work.right, work.bottom)
        }
        SnapDirection::Right if is_quarter && !on_top && on_left => {
            Rect::new(work.left, work.top + half_height, work.right, work.bottom)
        }
        SnapDirection::Left if is_quarter && on_left => current,
        SnapDirection::Left if is_quarter => Rect::new(
            work.left,
            if on_top {
                work.top
            } else {
                work.top + half_height
            },
            work.left + half_width,
            if on_top {
                work.top + half_height
            } else {
                work.bottom
            },
        ),
        SnapDirection::Right if is_quarter && !on_left => current,
        SnapDirection::Right if is_quarter => Rect::new(
            work.left + half_width,
            if on_top {
                work.top
            } else {
                work.top + half_height
            },
            work.right,
            if on_top {
                work.top + half_height
            } else {
                work.bottom
            },
        ),
        SnapDirection::Left => Rect::new(work.left, work.top, work.left + half_width, work.bottom),
        SnapDirection::Right => {
            Rect::new(work.left + half_width, work.top, work.right, work.bottom)
        }
        SnapDirection::Up if is_quarter && on_top => work,
        SnapDirection::Up if is_quarter => Rect::new(
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
            work.bottom,
        ),
        SnapDirection::Up if is_bottom_half => {
            Rect::new(work.left, work.top, work.right, work.top + half_height)
        }
        SnapDirection::Down if is_quarter && on_top => Rect::new(
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
            work.bottom,
        ),
        SnapDirection::Down if is_quarter => current,
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
        SnapDirection::Down if current == work => {
            Rect::new(work.left, work.top + half_height, work.right, work.bottom)
        }
        SnapDirection::Down => work,
    }
}
