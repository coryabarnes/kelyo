//! The shell: owns the tab model and every view, and applies commands.
//!
//! All state lives in one [`Shell`] on the CEF UI thread. Commands arrive one
//! at a time from the message loop (see `command.rs`), update the
//! engine-independent [`TabStrip`] first, then bring the views in line with it.
//!
//! Tabs restored from the last session get their web view (and start
//! loading) only when first shown, so a big session starts quickly.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use cef::*;
use kite_core::{
    Placement, Session, Settings, Store, TabId, TabStrip, blocker, clean_url, history, omnibox,
    paths,
};

use crate::client::KiteClient;
use crate::command::{BrowserId, Command, dispatch, dispatch_after};
use crate::filters;
use crate::ui::{RowState, TabRow, Ui};
use crate::views::{MainWindow, TabView};

thread_local! {
    static SHELL: RefCell<Option<Shell>> = const { RefCell::new(None) };
}

/// Set once every tab has closed, so the window may close for real.
static READY_TO_CLOSE: AtomicBool = AtomicBool::new(false);

/// Browsers showing tabs (not popups or DevTools). Kept outside the shell so
/// engine callbacks can check it while a command is running.
static TAB_BROWSERS: Mutex<Option<HashSet<BrowserId>>> = Mutex::new(None);

pub fn is_tab_browser(browser: BrowserId) -> bool {
    TAB_BROWSERS
        .lock()
        .is_ok_and(|ids| ids.as_ref().is_some_and(|ids| ids.contains(&browser)))
}

pub fn set_tab_browser(browser: BrowserId, is_tab: bool) {
    if let Ok(mut ids) = TAB_BROWSERS.lock() {
        let ids = ids.get_or_insert_with(HashSet::new);
        if is_tab {
            ids.insert(browser);
        } else {
            ids.remove(&browser);
        }
    }
}

const HISTORY_URL: &str = "kite://history";

/// Creates the main window and restores the last session, or opens a first tab.
/// Call once, on the UI thread.
pub fn start(settings: Settings) {
    let store = open_store();
    match store.blocking_exceptions() {
        Ok(sites) => filters::set_exceptions(sites),
        Err(error) => eprintln!("kite: couldn't read site settings: {error}"),
    }
    let session = store.load_session().unwrap_or_else(|error| {
        eprintln!("kite: couldn't read the last session: {error}");
        None
    });

    let mut shell = Shell::new(settings, store);
    match session {
        Some(session) => shell.restore(&session),
        None => dispatch(Command::NewTab {
            url: None,
            background: false,
        }),
    }
    SHELL.with(|cell| *cell.borrow_mut() = Some(shell));
}

fn open_store() -> Store {
    let Some(path) = paths::database() else {
        eprintln!("kite: no home directory found; history won't be saved");
        return Store::in_memory();
    };
    Store::open(&path).unwrap_or_else(|error| {
        eprintln!(
            "kite: can't open {}: {error}; history won't be saved",
            path.display()
        );
        Store::in_memory()
    })
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

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// The views belonging to one tab.
struct TabEntry {
    /// Created when the tab is first shown.
    view: Option<BrowserView>,
    row: TabRow,
    /// Known once the engine has created the browser.
    browser: Option<BrowserId>,
}

struct Shell {
    settings: Settings,
    strip: TabStrip,
    entries: HashMap<TabId, TabEntry>,
    browsers: HashMap<BrowserId, TabId>,
    store: Store,
    /// What was last written to the store, to skip writes when nothing changed.
    saved_session: Option<Session>,
    ui: Ui,
    client: Client,
    window: Option<Window>,
    closing: bool,
}

impl Shell {
    fn new(settings: Settings, store: Store) -> Self {
        let ui = Ui::build();
        let mut delegate = MainWindow::new(RefCell::new(Some(ui.root.clone())));
        let window = window_create_top_level(Some(&mut delegate));
        Self {
            settings,
            strip: TabStrip::new(),
            entries: HashMap::new(),
            browsers: HashMap::new(),
            store,
            saved_session: None,
            ui,
            client: KiteClient::new(),
            window,
            closing: false,
        }
    }

    fn restore(&mut self, session: &Session) {
        self.strip = session.to_strip();
        self.saved_session = Some(session.clone());
        let tabs: Vec<TabId> = self.strip.iter().map(|tab| tab.id).collect();
        for tab in tabs {
            self.entries.insert(
                tab,
                TabEntry {
                    view: None,
                    row: TabRow::new(tab),
                    browser: None,
                },
            );
        }
        self.rebuild_tab_list();
        self.sync_selection();
    }

    fn handle(&mut self, command: Command) {
        let saves_session = !matches!(
            command,
            Command::LoadingChanged { .. }
                | Command::AddressBarEdited(_)
                | Command::BrowserCreated { .. }
        );
        match command {
            Command::NewTab { url, background } => self.open_tab(url, background),
            Command::CloseTab(tab) => self.close_tab(tab),
            Command::CloseActiveTab => {
                if let Some(tab) = self.strip.active() {
                    self.close_tab(tab);
                }
            }
            Command::ReopenClosedTab => {
                if let Some(tab) = self.strip.reopen_closed() {
                    self.add_entry(tab);
                    self.rebuild_tab_list();
                    self.sync_selection();
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
            Command::SplitWith(tab) => {
                self.strip.split_with(tab);
                self.sync_selection();
            }
            Command::ToggleSplitOrientation => {
                self.strip.toggle_split_orientation();
                self.sync_selection();
            }
            Command::Navigate(input) => self.navigate(&input),
            Command::AddressBarEdited(text) => self.complete_address(&text),
            Command::FocusAddressBar => self.focus_address_bar(),
            Command::ResetAddressBar => self.sync_address_bar(true),
            Command::Back => self.with_active_browser(|browser| browser.go_back()),
            Command::Forward => self.with_active_browser(|browser| browser.go_forward()),
            Command::Reload => {
                let loading = self.strip.active_tab().is_some_and(|tab| tab.loading);
                self.with_active_browser(|browser| {
                    if loading {
                        browser.stop_load()
                    } else {
                        browser.reload()
                    }
                });
            }
            Command::ToggleDevTools => self.toggle_dev_tools(),
            Command::CopyCleanLink => self.copy_clean_link(),
            Command::OpenHistory => self.open_tab(Some(HISTORY_URL.to_owned()), false),
            Command::ToggleBlocking => self.toggle_blocking(),
            Command::CloseWindow => self.close_window(),

            Command::BrowserCreated { tab, browser } => {
                if let Some(entry) = self.entries.get_mut(&tab) {
                    entry.browser = Some(browser);
                    self.browsers.insert(browser, tab);
                }
            }
            Command::BrowserReadyToClose(browser) => {
                let entry = self
                    .browsers
                    .get(&browser)
                    .and_then(|tab| self.entries.get_mut(tab));
                if let Some(view) = entry.and_then(|entry| entry.view.take()) {
                    self.ui
                        .content
                        .remove_child_view(Some(&mut View::from(&view)));
                }
            }
            Command::BrowserClosed(browser) => {
                set_tab_browser(browser, false);
                if let Some(tab) = self.browsers.get(&browser).copied() {
                    self.remove_tab(tab);
                }
            }
            Command::BrowserFocused(browser) => {
                // Only on a change: re-syncing moves focus, which would fire this again.
                let tab = self.browsers.get(&browser).copied();
                if let Some(tab) = tab.filter(|tab| self.strip.active() != Some(*tab))
                    && self.strip.activate(tab)
                {
                    self.sync_selection();
                }
            }
            Command::TitleChanged { browser, title } => {
                if let Some(tab) = self.update_tab(browser, |tab| tab.title = title) {
                    if let Some(model) = self.strip.get(tab) {
                        let _ = self.store.set_title(&model.url, &model.title);
                    }
                    self.refresh_row(tab);
                    self.sync_window_title();
                }
            }
            Command::AddressChanged { browser, url } => {
                let changed = self
                    .browsers
                    .get(&browser)
                    .and_then(|tab| self.strip.get(*tab))
                    .is_some_and(|tab| tab.url != url);
                if changed
                    && history::is_recordable(&url)
                    && let Err(error) = self.store.record_visit(&url, now())
                {
                    eprintln!("kite: couldn't save history: {error}");
                }
                if let Some(tab) = self.update_tab(browser, |tab| tab.url = url) {
                    self.refresh_row(tab);
                    if self.strip.active() == Some(tab) {
                        self.sync_address_bar(false);
                        self.sync_shield();
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
                    self.sync_shield();
                }
            }
        }
        if saves_session {
            self.save_session();
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
        let tab = self.strip.open(url, placement);
        self.add_entry(tab);
        // Background tabs start loading now, like in any browser.
        if !self.ensure_view(tab) {
            self.strip.close(tab);
            self.entries.remove(&tab);
            return;
        }
        self.rebuild_tab_list();
        self.sync_selection();
        if is_blank && !background {
            self.focus_address_bar();
        }
    }

    fn add_entry(&mut self, tab: TabId) {
        self.entries.insert(
            tab,
            TabEntry {
                view: None,
                row: TabRow::new(tab),
                browser: None,
            },
        );
    }

    /// Creates the tab's web view if it doesn't have one yet, which starts
    /// loading its URL. Returns `false` if the engine couldn't create it.
    fn ensure_view(&mut self, tab: TabId) -> bool {
        let Some(url) = self.strip.get(tab).map(|tab| tab.url.clone()) else {
            return false;
        };
        let Some(entry) = self.entries.get_mut(&tab) else {
            return false;
        };
        if entry.view.is_some() {
            return true;
        }
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
            return false;
        };
        view.set_visible(0);
        self.ui.content.add_child_view(Some(&mut View::from(&view)));
        entry.view = Some(view);
        true
    }

    /// Starts closing a tab. Its unload handlers run first; the tab is removed
    /// when the engine reports the browser closed.
    fn close_tab(&mut self, tab: TabId) {
        if self.strip.len() == 1 && self.strip.get(tab).is_some() {
            // Closing the last tab closes the window, and the next start
            // begins afresh instead of restoring it.
            if let Err(error) = self.store.save_session(&Session::default()) {
                eprintln!("kite: couldn't save the session: {error}");
            }
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
            if let Some(view) = &entry.view {
                self.ui
                    .content
                    .remove_child_view(Some(&mut View::from(view)));
            }
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

    /// Saves the open tabs if they changed. Not while the window is closing:
    /// tabs close one by one then, and the session to keep is the one before.
    fn save_session(&mut self) {
        if self.closing {
            return;
        }
        let session = Session::from_strip(&self.strip);
        if self.saved_session.as_ref() == Some(&session) {
            return;
        }
        match self.store.save_session(&session) {
            Ok(()) => self.saved_session = Some(session),
            Err(error) => eprintln!("kite: couldn't save the session: {error}"),
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

    fn browser_for(&self, tab: TabId) -> Option<Browser> {
        self.entries.get(&tab)?.view.as_ref()?.browser()
    }

    fn host_for(&self, tab: TabId) -> Option<BrowserHost> {
        self.browser_for(tab)?.host()
    }

    fn with_active_browser(&self, action: impl FnOnce(&Browser)) {
        if let Some(browser) = self.strip.active().and_then(|tab| self.browser_for(tab)) {
            action(&browser);
        }
    }

    // Navigation and tools.

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

    /// Inline completion: after the user types, fill in the rest of the most
    /// visited matching site and select the filled-in part, so typing on
    /// replaces it.
    fn complete_address(&self, typed: &str) {
        let bar = &self.ui.address_bar;
        let current = CefString::from(&bar.text()).to_string();
        let cursor_at_end = bar.cursor_position() == current.encode_utf16().count();
        // Stale (the user kept typing) or not a plain append at the end.
        if current != typed || bar.has_focus() == 0 || bar.has_selection() != 0 || !cursor_at_end {
            return;
        }
        let Ok(Some(completion)) = self.store.complete(typed) else {
            return;
        };
        bar.set_text(Some(&CefString::from(completion.as_str())));
        bar.select_range(Some(&Range {
            from: typed.encode_utf16().count() as u32,
            to: completion.encode_utf16().count() as u32,
        }));
    }

    fn toggle_dev_tools(&self) {
        let Some(host) = self.strip.active().and_then(|tab| self.host_for(tab)) else {
            return;
        };
        if host.has_dev_tools() != 0 {
            host.close_dev_tools();
        } else {
            // Opens in its own window, via `TabView::on_popup_browser_view_created`.
            // It must be Alloy style like the tab; Chrome style crashes here.
            let window_info = WindowInfo {
                runtime_style: RuntimeStyle::ALLOY,
                ..Default::default()
            };
            host.show_dev_tools(
                Some(&window_info),
                None,
                Some(&BrowserSettings::default()),
                None,
            );
        }
    }

    /// Copies the active tab's URL through the address bar (which uses the
    /// system clipboard), then briefly confirms in the bar.
    fn copy_clean_link(&self) {
        let Some(url) = self.strip.active_tab().map(|tab| tab.url.clone()) else {
            return;
        };
        let link = clean_url::clean(&url).unwrap_or(url);
        let bar = &self.ui.address_bar;
        bar.set_text(Some(&CefString::from(link.as_str())));
        bar.select_all(0);
        bar.execute_command(TextFieldCommands::COPY);
        bar.set_text(Some(&CefString::from("✓ Link copied")));
        dispatch_after(Command::ResetAddressBar, 1200);
    }

    fn toggle_blocking(&mut self) {
        let Some(site) = self
            .strip
            .active_tab()
            .and_then(|tab| blocker::site_of(&tab.url))
        else {
            return;
        };
        let excepted = !filters::is_excepted(&site);
        if let Err(error) = self.store.set_blocking_exception(&site, excepted) {
            eprintln!("kite: couldn't save the setting for {site}: {error}");
        }
        match self.store.blocking_exceptions() {
            Ok(sites) => filters::set_exceptions(sites),
            Err(error) => eprintln!("kite: couldn't read site settings: {error}"),
        }
        self.with_active_browser(|browser| browser.reload());
        self.sync_shield();
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

    /// Shows the visible tabs' web views in pane order, highlights their rows,
    /// and updates everything that depends on the active tab.
    fn sync_selection(&mut self) {
        let visible = self.strip.visible();
        for tab in &visible {
            self.ensure_view(*tab);
        }
        let active = self.strip.active();
        for (id, entry) in &self.entries {
            let state = if Some(*id) == active {
                RowState::Active
            } else if visible.contains(id) {
                RowState::InSplit
            } else {
                RowState::Normal
            };
            entry.row.set_state(state);
            if let Some(view) = &entry.view {
                view.set_visible(visible.contains(id).into());
            }
        }
        for (index, tab) in visible.iter().enumerate() {
            if let Some(view) = self.entries.get(tab).and_then(|entry| entry.view.as_ref()) {
                self.ui
                    .content
                    .reorder_child_view(Some(&mut View::from(view)), index as i32);
            }
        }
        self.ui
            .set_content_vertical(self.strip.split().is_some_and(|split| split.vertical));
        self.ui.content.layout();
        self.ui.split.set_enabled((self.strip.len() >= 2).into());

        self.sync_address_bar(true);
        self.sync_nav_buttons();
        self.sync_shield();
        self.sync_window_title();
        self.focus_active_view();
    }

    /// Shows the active tab's URL. Unless `force`, leaves the bar alone while
    /// the user is typing in it.
    fn sync_address_bar(&self, force: bool) {
        if !force && self.ui.address_bar.has_focus() != 0 {
            return;
        }
        let url = self.strip.active_tab().map_or("", |tab| tab.url.as_str());
        self.ui.address_bar.set_text(Some(&CefString::from(url)));
    }

    fn sync_nav_buttons(&self) {
        let tab = self.strip.active_tab();
        let (back, forward, loading) = tab.map_or((false, false, false), |tab| {
            (tab.can_go_back, tab.can_go_forward, tab.loading)
        });
        self.ui.back.set_enabled(back.into());
        self.ui.forward.set_enabled(forward.into());
        self.ui
            .reload
            .set_text(Some(&CefString::from(if loading { "✕" } else { "↻" })));
    }

    /// Shows how many requests were blocked on the active page, or that
    /// blocking is off for its site.
    fn sync_shield(&self) {
        let site = self
            .strip
            .active_tab()
            .and_then(|tab| blocker::site_of(&tab.url));
        let (label, tooltip) = match &site {
            None => ("🛡".to_owned(), "Ad blocking works on web pages".to_owned()),
            Some(site) if filters::is_excepted(site) => (
                "🛡 off".to_owned(),
                format!("Ad blocking is off for {site}. Click to turn it back on."),
            ),
            Some(site) => {
                let blocked = self
                    .strip
                    .active()
                    .and_then(|tab| self.entries.get(&tab)?.browser)
                    .map_or(0, filters::blocked_count);
                let label = if blocked > 0 {
                    format!("🛡 {blocked}")
                } else {
                    "🛡".to_owned()
                };
                let tooltip = format!(
                    "{blocked} ads and trackers blocked. Click to turn blocking off for {site}."
                );
                (label, tooltip)
            }
        };
        self.ui
            .shield
            .set_text(Some(&CefString::from(label.as_str())));
        self.ui
            .shield
            .set_tooltip_text(Some(&CefString::from(tooltip.as_str())));
        self.ui.shield.set_enabled(site.is_some().into());
    }

    fn sync_window_title(&self) {
        let (Some(window), Some(tab)) = (&self.window, self.strip.active_tab()) else {
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
        let view = self
            .strip
            .active()
            .and_then(|tab| self.entries.get(&tab))
            .and_then(|entry| entry.view.as_ref());
        if let Some(view) = view {
            view.request_focus();
        }
    }
}
