//! Colours (ARGB) and sizes for the shell UI.

pub const WINDOW_BG: u32 = 0xFF_16_16_1D;
pub const SIDEBAR_BG: u32 = WINDOW_BG;
pub const TAB_ACTIVE_BG: u32 = 0xFF_2A_2A_36;
/// Tabs shown in split view, other than the active one.
pub const TAB_SPLIT_BG: u32 = 0xFF_21_21_2B;
pub const TEXT: u32 = 0xFF_E6_E6_EE;
pub const TEXT_DIM: u32 = 0xFF_8C_8C_9A;

pub const FONT: &str = "Sans, 13px";

pub const WINDOW_WIDTH: i32 = 1280;
pub const WINDOW_HEIGHT: i32 = 800;

pub const SIDEBAR_WIDTH: i32 = 260;
pub const SIDEBAR_PADDING: i32 = 10;
pub const SIDEBAR_SPACING: i32 = 8;

/// Height of one tab row, and the gap between rows.
pub const TAB_ROW_HEIGHT: i32 = 34;
pub const TAB_ROW_GAP: i32 = 2;
/// Space reserved on the right of the tab list for the scrollbar.
pub const SCROLLBAR_ALLOWANCE: i32 = 12;
pub const TAB_LIST_WIDTH: i32 = SIDEBAR_WIDTH - 2 * SIDEBAR_PADDING - SCROLLBAR_ALLOWANCE;

/// Gap around and between the web views.
pub const CONTENT_PADDING: i32 = 6;
