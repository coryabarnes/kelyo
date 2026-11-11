//! Commands: the single way anything asks the shell to change.
//!
//! UI delegates and engine callbacks never touch shell state directly. They
//! call [`dispatch`], which posts the command to the CEF UI thread's message
//! loop. Each command then runs on its own, after the current callback has
//! returned, so the shell is never re-entered while it is mid-update.

use std::sync::{Arc, Mutex};

use cef::*;
use kite_core::TabId;

/// CEF's identifier for a browser instance (`Browser::identifier`).
pub type BrowserId = i32;

#[derive(Debug, Clone)]
pub enum Command {
    // User actions.
    NewTab {
        url: Option<String>,
        background: bool,
    },
    CloseTab(TabId),
    CloseActiveTab,
    ReopenClosedTab,
    ActivateTab(TabId),
    CycleTabs {
        forward: bool,
    },
    ToggleSplit,
    /// Add the tab to split view, or take it out if it's already there.
    SplitWith(TabId),
    ToggleSplitOrientation,
    /// Raw address bar input for the active tab.
    Navigate(String),
    /// The user typed in the address bar; this is the text after the edit.
    AddressBarEdited(String),
    FocusAddressBar,
    /// Put the active tab's URL back into the address bar (Escape).
    ResetAddressBar,
    Back,
    Forward,
    Reload,
    ToggleDevTools,
    /// Copy the active tab's URL, without tracking parameters.
    CopyCleanLink,
    OpenHistory,
    /// Turn ad blocking off (or back on) for the active tab's site.
    ToggleBlocking,
    CloseWindow,

    // Engine events.
    BrowserCreated {
        tab: TabId,
        browser: BrowserId,
    },
    /// The page agreed to close (its unload handlers ran); drop its view.
    BrowserReadyToClose(BrowserId),
    BrowserClosed(BrowserId),
    /// A page got focus (e.g. a click into a split view pane).
    BrowserFocused(BrowserId),
    TitleChanged {
        browser: BrowserId,
        title: String,
    },
    AddressChanged {
        browser: BrowserId,
        url: String,
    },
    LoadingChanged {
        browser: BrowserId,
        loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    },
}

/// Queues `command` to run on the UI thread.
pub fn dispatch(command: Command) {
    let mut task = RunCommand::new(Arc::new(Mutex::new(Some(command))));
    post_task(ThreadId::UI, Some(&mut task));
}

/// Queues `command` to run on the UI thread after `delay_ms` milliseconds.
pub fn dispatch_after(command: Command, delay_ms: i64) {
    let mut task = RunCommand::new(Arc::new(Mutex::new(Some(command))));
    post_delayed_task(ThreadId::UI, Some(&mut task), delay_ms);
}

wrap_task! {
    struct RunCommand {
        command: Arc<Mutex<Option<Command>>>,
    }

    impl Task {
        fn execute(&self) {
            let command = self.command.lock().ok().and_then(|mut slot| slot.take());
            if let Some(command) = command {
                crate::shell::handle(command);
            }
        }
    }
}
