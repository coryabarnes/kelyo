//! Keyboard shortcuts, registered as window accelerators so they work even
//! while a web page has focus.

use cef::*;

use crate::command::Command;

/// Windows virtual-key codes, which CEF uses on every platform.
pub mod key {
    pub const TAB: i32 = 0x09;
    pub const RETURN: i32 = 0x0D;
    pub const ESCAPE: i32 = 0x1B;
    pub const PAGE_UP: i32 = 0x21;
    pub const PAGE_DOWN: i32 = 0x22;
    pub const LEFT: i32 = 0x25;
    pub const RIGHT: i32 = 0x27;
    pub const L: i32 = 0x4C;
    pub const R: i32 = 0x52;
    pub const S: i32 = 0x53;
    pub const T: i32 = 0x54;
    pub const W: i32 = 0x57;
    pub const F5: i32 = 0x74;
}

#[derive(Debug, Clone, Copy)]
enum Action {
    NewTab,
    CloseTab,
    FocusAddressBar,
    NextTab,
    PreviousTab,
    ToggleSplit,
    Reload,
    Back,
    Forward,
}

impl Action {
    fn command(self) -> Command {
        match self {
            Action::NewTab => Command::NewTab {
                url: None,
                background: false,
            },
            Action::CloseTab => Command::CloseActiveTab,
            Action::FocusAddressBar => Command::FocusAddressBar,
            Action::NextTab => Command::CycleTabs { forward: true },
            Action::PreviousTab => Command::CycleTabs { forward: false },
            Action::ToggleSplit => Command::ToggleSplit,
            Action::Reload => Command::Reload,
            Action::Back => Command::Back,
            Action::Forward => Command::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Keys {
    code: i32,
    ctrl: bool,
    shift: bool,
    alt: bool,
}

const fn keys(code: i32, ctrl: bool, shift: bool, alt: bool) -> Keys {
    Keys {
        code,
        ctrl,
        shift,
        alt,
    }
}

/// Every binding. Its index is the accelerator command id CEF hands back.
const BINDINGS: &[(Action, Keys)] = &[
    (Action::NewTab, keys(key::T, true, false, false)),
    (Action::CloseTab, keys(key::W, true, false, false)),
    (Action::FocusAddressBar, keys(key::L, true, false, false)),
    (Action::NextTab, keys(key::TAB, true, false, false)),
    (Action::NextTab, keys(key::PAGE_DOWN, true, false, false)),
    (Action::PreviousTab, keys(key::TAB, true, true, false)),
    (Action::PreviousTab, keys(key::PAGE_UP, true, false, false)),
    (Action::ToggleSplit, keys(key::S, true, true, false)),
    (Action::Reload, keys(key::R, true, false, false)),
    (Action::Reload, keys(key::F5, false, false, false)),
    (Action::Back, keys(key::LEFT, false, false, true)),
    (Action::Forward, keys(key::RIGHT, false, false, true)),
];

pub fn register(window: &Window) {
    for (id, (_, keys)) in BINDINGS.iter().enumerate() {
        let high_priority = 1; // Take the key before the web page does.
        window.set_accelerator(
            id as i32,
            keys.code,
            keys.shift.into(),
            keys.ctrl.into(),
            keys.alt.into(),
            high_priority,
        );
    }
}

/// The command for an accelerator id received in `on_accelerator`.
pub fn command(id: i32) -> Option<Command> {
    let index = usize::try_from(id).ok()?;
    BINDINGS.get(index).map(|(action, _)| action.command())
}
