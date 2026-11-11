//! The shell: owns the tab model and every view, and applies commands.
//!
//! All state lives in one [`Shell`] on the CEF UI thread. Commands arrive one
//! at a time from the message loop (see `command.rs`), update the
//! engine-independent [`TabStrip`] first, then bring the views in line with it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use cef::*;
use kite_core::{Placement, Settings, TabId, TabStrip, omnibox};

use crate::client::KiteClient;
use crate::command::{BrowserId, Command, dispatch};
use crate::ui::{TabRow, Ui};
use crate::views::{MainWindow, TabView};

thread_local! {
    static SHELL: RefCell<Option<Shell>> = const { RefCell::new(None) };
}

/// Set once every tab has closed, so the window may close for real.
static READY_TO_CLOSE: AtomicBool = AtomicBool::new(false);

/// Creates the main window and opens the first tab. Call once, on the UI thread.
pub fn start(settings: Settings) {
    let shell = Shell::new(settings);
    SHELL.with(|cell| *cell.borrow_mut() = Some(shell));
    dispatch(Command::NewTab {
        url: None,
        background: false,
    });
}

/// Applies a command. Runs on the UI thread, from the message loop.
pub fn handle(command: Command) {
    SHELL.with(|cell| match cell.try_borrow_mut() {
        Ok(mut shell) => {
            if let Some(shell) = shell.as_mut() {
                shell.handle(command);
            }
        }
        // Only possible if CEF spins a nested message loop while a command is
        // running. Try again once the current command has finished.
        Err(_) => dispatch(command),
    });
}

pub fn ready_to_close() -> bool {
    READY_TO_CLOSE.load(Ordering::SeqCst)
}

/// The views belonging to one tab.
struct TabEntry {
    view: BrowserView,
    row: TabRow,
    /// Known once the engine has created the browser.
    browser: Option<BrowserId>,
}

struct Shell {
    settings: Settings,
    strip: TabStrip,
    entries: HashMap<TabId, TabEntry>,
    browsers: HashMap<BrowserId, TabId>,
    ui: Ui,
    client: Client,
    window: Option<Window>,
    closing: bool,
}

impl Shell {
    fn new(settings: Settings) -> Self {
        let ui = Ui::build();
        let mut delegate = MainWindow::new(RefCell::new(Some(ui.root.clone())));
        let window = window_create_top_level(Some(&mut delegate));
        Self {
            settings,
            strip: TabStrip::new(),
            entries: HashMap::new(),
            browsers: HashMap::new(),
            ui,
            client: KiteClient::new(),
            window,
            closing: false,
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::NewTab { url, background } => self.open_tab(url, background),
            Command::CloseTab(tab) => self.close_tab(tab),
            Command::CloseActiveTab => {
                if let Some(tab) = self.strip.active() {
                    self.close_tab(tab);
                }
            }
            Command::ActivateTab(tab) => {
                self.strip.activate(tab);
                self.sync_selection();
            }
            Command::CycleTabs { forward } => {
                self.strip.cycle(forward);
                self.sync_selection();
            }
            Command::ToggleSplit => {
                self.strip.toggle_split();
                self.sync_selection();
            }
            Command::Navigate(input) => self.navigate(&input),
            Command::FocusAddressBar => self.focus_address_bar(),
            Command::ResetAddressBar => self.sync_address_bar(true),
            Command::Back => self.with_active_browser(|browser| browser.go_back()),
            Command::Forward => self.with_active_browser(|browser| browser.go_forward()),
            Command::Reload => {
                let loading = self
                    .strip
                    .active()
                    .and_then(|id| self.strip.get(id))
                    .is_some_and(|tab| tab.loading);
                self.with_active_browser(|browser| {
                    if loading {
                        browser.stop_load()
                    } else {
                        browser.reload()
                    }
                });
            }
            Command::CloseWindow => self.close_window(),

            Command::BrowserCreated { tab, browser } => {
                if let Some(entry) = self.entries.get_mut(&tab) {
                    entry.browser = Some(browser);
                    self.browsers.insert(browser, tab);
                }
            }
            Command::BrowserClosed(browser) => {
                if let Some(tab) = self.browsers.get(&browser).copied() {
                    self.remove_tab(tab);
                }
            }
            Command::TitleChanged { browser, title } => {
                if let Some(tab) = self.update_tab(browser, |tab| tab.title = title) {
                    self.refresh_row(tab);
                    self.sync_window_title();
                }
            }
            Command::AddressChanged { browser, url } => {
                if let Some(tab) = self.update_tab(browser, |tab| tab.url = url) {
                    self.refresh_row(tab);
                    if self.strip.active() == Some(tab) {
                        self.sync_address_bar(false);
                    }
                }
            }
            Command::LoadingChanged {
                browser,
                loading,
                can_go_back,
                can_go_forward,
            } => {
                let updated = self.update_tab(browser, |tab| {
                    tab.loading = loading;
                    tab.can_go_back = can_go_back;
                    tab.can_go_forward = can_go_forward;
                });
                if updated.is_some_and(|tab| self.strip.active() == Some(tab)) {
                    self.sync_nav_buttons();
                }
            }
        }
    }

    // Tabs.

    fn open_tab(&mut self, url: Option<String>, background: bool) {
        let is_blank = url.is_none();
        let url = url.unwrap_or_else(|| self.settings.new_tab_url.clone());
        let placement = if background {
            Placement::Background
        } else {
            Placement::Foreground
        };
        let tab = self.strip.open(url.clone(), placement);

        let mut client = self.client.clone();
        let mut delegate = TabView::new(tab);
        let view = browser_view_create(
            Some(&mut client),
            Some(&CefString::from(url.as_str())),
            Some(&BrowserSettings::default()),
            None,
            None,
            Some(&mut delegate),
        );
        let Some(view) = view else {
            eprintln!("kite: failed to create a browser view for {url}");
            self.strip.close(tab);
            return;
        };
        view.set_visible(0);
        self.ui.content.add_child_view(Some(&mut View::from(&view)));

        self.entries.insert(
            tab,
            TabEntry {
                view,
                row: TabRow::new(tab),
                browser: None,
            },
        );
        self.rebuild_tab_list();
        self.sync_selection();
        if is_blank && !background {
            self.focus_address_bar();
        }
    }

    /// Starts closing a tab. Its unload handlers run first; the tab is removed
    /// when the engine reports the browser closed.
    fn close_tab(&mut self, tab: TabId) {
        if self.strip.len() == 1 && self.strip.get(tab).is_some() {
            // Closing the last tab closes the window.
            self.close_window();
            return;
        }
        match self.host_for(tab) {
            Some(host) => host.close_browser(0),
            None => self.remove_tab(tab),
        }
    }

    fn remove_tab(&mut self, tab: TabId) {
        self.strip.close(tab);
        if let Some(entry) = self.entries.remove(&tab) {
            if let Some(browser) = entry.browser {
                self.browsers.remove(&browser);
            }
            self.ui
                .content
                .remove_child_view(Some(&mut View::from(&entry.view)));
        }
        self.rebuild_tab_list();
        self.sync_selection();
        self.finish_closing_if_done();
    }

    fn close_window(&mut self) {
        self.closing = true;
        let tabs: Vec<TabId> = self.strip.iter().map(|tab| tab.id).collect();
        for tab in tabs {
            match self.host_for(tab) {
                Some(host) => host.close_browser(0),
                None => self.remove_tab(tab),
            }
        }
        self.finish_closing_if_done();
    }

    fn finish_closing_if_done(&mut self) {
        if self.closing && self.strip.is_empty() {
            READY_TO_CLOSE.store(true, Ordering::SeqCst);
            if let Some(window) = self.window.take() {
                window.close();
            }
        }
    }

    /// Applies `change` to the tab shown by `browser`, returning its id.
    fn update_tab(
        &mut self,
        browser: BrowserId,
        change: impl FnOnce(&mut kite_core::Tab),
    ) -> Option<TabId> {
        let tab = *self.browsers.get(&browser)?;
        change(self.strip.get_mut(tab)?);
        Some(tab)
    }

    fn host_for(&self, tab: TabId) -> Option<BrowserHost> {
        self.entries.get(&tab)?.view.browser()?.host()
    }

    fn with_active_browser(&self, action: impl FnOnce(&Browser)) {
        let browser = self
            .strip
            .active()
            .and_then(|tab| self.entries.get(&tab))
            .and_then(|entry| entry.view.browser());
        if let Some(browser) = browser {
            action(&browser);
        }
    }

    // Navigation.

    fn navigate(&mut self, input: &str) {
        let Some(url) = omnibox::resolve(input, &self.settings.search) else {
            return;
        };
        self.with_active_browser(|browser| {
            if let Some(frame) = browser.main_frame() {
                frame.load_url(Some(&CefString::from(url.as_str())));
            }
        });
        self.focus_active_view();
    }

    // Keeping views in sync with the tab model.

    /// Re-adds the tab rows in strip order.
    fn rebuild_tab_list(&self) {
        self.ui.tab_list.remove_all_child_views();
        for tab in self.strip.iter() {
            if let Some(entry) = self.entries.get(&tab.id) {
                entry.row.set_title(tab.label());
                self.ui
                    .tab_list
                    .add_child_view(Some(&mut View::from(&entry.row.panel)));
            }
        }
        self.ui.resize_tab_list(self.strip.len());
    }

    fn refresh_row(&self, tab: TabId) {
        if let (Some(model), Some(entry)) = (self.strip.get(tab), self.entries.get(&tab)) {
            entry.row.set_title(model.label());
        }
    }

    /// Shows the visible tabs' web views, highlights the active row, and
    /// updates everything that depends on the active tab.
    fn sync_selection(&self) {
        let visible = self.strip.visible();
        let active = self.strip.active();
        for (id, entry) in &self.entries {
            entry.view.set_visible(visible.contains(id).into());
            entry.row.set_active(Some(*id) == active);
        }
        self.ui.content.layout();
        self.ui.split.set_enabled((self.strip.len() >= 2).into());

        self.sync_address_bar(true);
        self.sync_nav_buttons();
        self.sync_window_title();
        self.focus_active_view();
    }

    /// Shows the active tab's URL. Unless `force`, leaves the bar alone while
    /// the user is typing in it.
    fn sync_address_bar(&self, force: bool) {
        if !force && self.ui.address_bar.has_focus() != 0 {
            return;
        }
        let url = self
            .strip
            .active()
            .and_then(|id| self.strip.get(id))
            .map_or("", |tab| tab.url.as_str());
        self.ui.address_bar.set_text(Some(&CefString::from(url)));
    }

    fn sync_nav_buttons(&self) {
        let tab = self.strip.active().and_then(|id| self.strip.get(id));
        let (back, forward, loading) = tab.map_or((false, false, false), |tab| {
            (tab.can_go_back, tab.can_go_forward, tab.loading)
        });
        self.ui.back.set_enabled(back.into());
        self.ui.forward.set_enabled(forward.into());
        self.ui
            .reload
            .set_text(Some(&CefString::from(if loading { "✕" } else { "↻" })));
    }

    fn sync_window_title(&self) {
        let (Some(window), Some(tab)) = (
            &self.window,
            self.strip.active().and_then(|id| self.strip.get(id)),
        ) else {
            return;
        };
        let title = format!("{} — Kite", tab.label());
        window.set_title(Some(&CefString::from(title.as_str())));
    }

    fn focus_address_bar(&self) {
        self.ui.address_bar.request_focus();
        self.ui.address_bar.select_all(0);
    }

    fn focus_active_view(&self) {
        if let Some(entry) = self.strip.active().and_then(|tab| self.entries.get(&tab)) {
            entry.view.request_focus();
        }
    }
}
