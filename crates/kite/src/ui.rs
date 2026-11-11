//! Builds the window contents from CEF Views widgets:
//!
//! ```text
//! ┌──────────── root (horizontal) ─────────────────────────────┐
//! │ sidebar (vertical, fixed width) │ content (horizontal)       │
//! │  ‹ › ↻ ◫ 🛡      nav row         │  web view [ | web view ]   │
//! │  [address bar              ]    │  (only the visible tabs    │
//! │  tab rows (scrollable)          │   are shown; split view    │
//! │  + New Tab                      │   shows up to three, side  │
//! │                                 │   by side or stacked)      │
//! └─────────────────────────────────┴────────────────────────────┘
//! ```

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use cef::*;
use kite_core::TabId;

use crate::command::Command;
use crate::theme;
use crate::views::{AddressBar, CommandButton, FixedSize};

pub struct Ui {
    pub root: Panel,
    pub address_bar: Textfield,
    pub tab_list: Panel,
    /// Holds one web view per tab; hidden views take no space.
    pub content: Panel,
    pub back: LabelButton,
    pub forward: LabelButton,
    pub reload: LabelButton,
    pub split: LabelButton,
    /// Ad blocking for the current site: shows the blocked count, click to toggle.
    pub shield: LabelButton,
    content_vertical: Cell<bool>,
}

impl Ui {
    pub fn build() -> Self {
        let root = panel(None);
        root.set_background_color(theme::WINDOW_BG);
        let root_layout = set_box_layout(&root, box_settings(Axis::Horizontal, 0, 0));

        // Sidebar.
        let sidebar = panel(Some(FixedSize::new(theme::SIDEBAR_WIDTH, 1)));
        sidebar.set_background_color(theme::SIDEBAR_BG);
        let sidebar_layout = set_box_layout(
            &sidebar,
            box_settings(
                Axis::Vertical,
                theme::SIDEBAR_PADDING,
                theme::SIDEBAR_SPACING,
            ),
        );

        let nav = panel(None);
        set_box_layout(&nav, box_settings(Axis::Horizontal, 0, 4));
        let back = button("‹", Command::Back, "Back (Alt+Left)");
        let forward = button("›", Command::Forward, "Forward (Alt+Right)");
        let reload = button("↻", Command::Reload, "Reload (Ctrl+R)");
        let split = button("◫", Command::ToggleSplit, "Split view (Ctrl+Shift+S)");
        let shield = button("🛡", Command::ToggleBlocking, "");
        for item in [&back, &forward, &reload, &split, &shield] {
            add(&nav, item);
        }
        add(&sidebar, &nav);

        let address_bar =
            textfield_create(Some(&mut AddressBar::new(Arc::new(AtomicBool::new(false)))))
                .expect("CEF failed to create a textfield");
        address_bar.set_font_list(Some(&CefString::from(theme::FONT)));
        address_bar.set_placeholder_text(Some(&CefString::from("Search or enter address")));
        add(&sidebar, &address_bar);

        let tab_list = panel(None);
        set_box_layout(
            &tab_list,
            box_settings(Axis::Vertical, 0, theme::TAB_ROW_GAP),
        );
        let tab_scroll = scroll_view_create(None).expect("CEF failed to create a scroll view");
        tab_scroll.set_content_view(Some(&mut View::from(&tab_list)));
        add(&sidebar, &tab_scroll);
        sidebar_layout.set_flex_for_view(Some(&mut View::from(&tab_scroll)), 1);

        let new_tab = button(
            "+  New Tab",
            Command::NewTab {
                url: None,
                background: false,
            },
            "New tab (Ctrl+T)",
        );
        new_tab.set_horizontal_alignment(HorizontalAlignment::LEFT);
        add(&sidebar, &new_tab);

        let content = panel(None);
        set_content_layout(&content, Axis::Horizontal);

        add(&root, &sidebar);
        add(&root, &content);
        root_layout.set_flex_for_view(Some(&mut View::from(&content)), 1);

        Self {
            root,
            address_bar,
            tab_list,
            content,
            back,
            forward,
            reload,
            split,
            shield,
            content_vertical: Cell::new(false),
        }
    }

    /// Lays the visible web views out side by side, or stacked if `vertical`.
    pub fn set_content_vertical(&self, vertical: bool) {
        if self.content_vertical.replace(vertical) != vertical {
            let axis = if vertical {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            set_content_layout(&self.content, axis);
        }
    }

    /// Sizes the tab list to fit `rows` rows, so the scroll view knows its extent.
    pub fn resize_tab_list(&self, rows: usize) {
        let rows = i32::try_from(rows).unwrap_or(i32::MAX);
        let height = rows * theme::TAB_ROW_HEIGHT + (rows - 1).max(0) * theme::TAB_ROW_GAP;
        self.tab_list.set_size(Some(&Size {
            width: theme::TAB_LIST_WIDTH,
            height,
        }));
        self.tab_list.layout();
    }
}

/// One row in the sidebar's tab list: the title (click to switch) and a close button.
pub struct TabRow {
    pub panel: Panel,
    title: LabelButton,
}

impl TabRow {
    pub fn new(tab: TabId) -> Self {
        let panel = panel(Some(FixedSize::new(
            theme::TAB_LIST_WIDTH,
            theme::TAB_ROW_HEIGHT,
        )));
        let layout = set_box_layout(&panel, box_settings(Axis::Horizontal, 0, 0));

        let title = button("New Tab", Command::ActivateTab(tab), "");
        title.set_horizontal_alignment(HorizontalAlignment::LEFT);
        let split = button(
            "◫",
            Command::SplitWith(tab),
            "Show beside the current tab (up to 3)",
        );
        let close = button("×", Command::CloseTab(tab), "Close tab (Ctrl+W)");

        add(&panel, &title);
        add(&panel, &split);
        add(&panel, &close);
        layout.set_flex_for_view(Some(&mut View::from(&title)), 1);

        let row = Self { panel, title };
        row.set_state(RowState::Normal);
        row
    }

    pub fn set_title(&self, text: &str) {
        let text = CefString::from(text);
        self.title.set_text(Some(&text));
        self.title.set_tooltip_text(Some(&text));
    }

    pub fn set_state(&self, state: RowState) {
        let color = match state {
            RowState::Active => theme::TAB_ACTIVE_BG,
            RowState::InSplit => theme::TAB_SPLIT_BG,
            RowState::Normal => theme::SIDEBAR_BG,
        };
        self.panel.set_background_color(color);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Active,
    /// On screen in split view, but not the active pane.
    InSplit,
    Normal,
}

// Small constructors that keep the layout code above readable.

enum Axis {
    Horizontal,
    Vertical,
}

fn box_settings(axis: Axis, padding: i32, spacing: i32) -> BoxLayoutSettings {
    BoxLayoutSettings {
        horizontal: matches!(axis, Axis::Horizontal).into(),
        inside_border_insets: Insets {
            top: padding,
            left: padding,
            bottom: padding,
            right: padding,
        },
        between_child_spacing: spacing,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }
}

/// `default_flex` makes every web view share the space equally.
fn set_content_layout(content: &Panel, axis: Axis) {
    set_box_layout(
        content,
        BoxLayoutSettings {
            default_flex: 1,
            ..box_settings(axis, theme::CONTENT_PADDING, theme::CONTENT_PADDING)
        },
    );
}

fn panel(delegate: Option<PanelDelegate>) -> Panel {
    let mut delegate = delegate;
    panel_create(delegate.as_mut()).expect("CEF failed to create a panel")
}

fn set_box_layout(panel: &Panel, settings: BoxLayoutSettings) -> BoxLayout {
    panel
        .set_to_box_layout(Some(&settings))
        .expect("CEF failed to create a box layout")
}

fn add<V>(parent: &Panel, child: &V)
where
    for<'a> View: From<&'a V>,
{
    parent.add_child_view(Some(&mut View::from(child)));
}

fn button(label: &str, command: Command, tooltip: &str) -> LabelButton {
    let mut delegate = CommandButton::new(command);
    let button = label_button_create(Some(&mut delegate), Some(&CefString::from(label)))
        .expect("CEF failed to create a button");
    button.set_font_list(Some(&CefString::from(theme::FONT)));
    button.set_enabled_text_colors(theme::TEXT);
    button.set_text_color(ButtonState::DISABLED, theme::TEXT_DIM);
    if !tooltip.is_empty() {
        button.set_tooltip_text(Some(&CefString::from(tooltip)));
    }
    button
}
