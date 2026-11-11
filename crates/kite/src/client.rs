//! Browser callbacks from the engine, shared by every tab. Each one forwards
//! an event to the shell as a [`Command`], keyed by the browser's id.

use cef::*;

use crate::command::{Command, dispatch};

wrap_client! {
    pub struct KiteClient;

    impl Client {
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(DisplayEvents::new())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(LifeSpanEvents::new())
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(LoadEvents::new())
        }
    }
}

wrap_display_handler! {
    struct DisplayEvents;

    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            let Some(browser) = browser else { return };
            dispatch(Command::TitleChanged {
                browser: browser.identifier(),
                title: title.map(CefString::to_string).unwrap_or_default(),
            });
        }

        fn on_address_change(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, url: Option<&CefString>) {
            let (Some(browser), Some(frame)) = (browser, frame) else { return };
            if frame.is_main() == 0 {
                return; // Ignore iframes.
            }
            dispatch(Command::AddressChanged {
                browser: browser.identifier(),
                url: url.map(CefString::to_string).unwrap_or_default(),
            });
        }
    }
}

wrap_life_span_handler! {
    struct LifeSpanEvents;

    impl LifeSpanHandler {
        // Links that ask for a new tab or window open as a tab instead.
        // Real popups (`window.open` with features) get their own window; see `TabView`.
        fn on_before_popup(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: i32,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            target_disposition: WindowOpenDisposition,
            _user_gesture: i32,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut i32>,
        ) -> i32 {
            let background = if target_disposition == WindowOpenDisposition::NEW_BACKGROUND_TAB {
                true
            } else if target_disposition == WindowOpenDisposition::NEW_FOREGROUND_TAB
                || target_disposition == WindowOpenDisposition::NEW_WINDOW
            {
                false
            } else {
                return 0; // Let CEF create the popup.
            };
            let url = target_url.map(CefString::to_string).filter(|url| !url.is_empty());
            dispatch(Command::NewTab { url, background });
            1 // Cancel the popup; the shell opens a tab.
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                dispatch(Command::BrowserClosed(browser.identifier()));
            }
        }
    }
}

wrap_load_handler! {
    struct LoadEvents;

    impl LoadHandler {
        fn on_loading_state_change(
            &self,
            browser: Option<&mut Browser>,
            is_loading: i32,
            can_go_back: i32,
            can_go_forward: i32,
        ) {
            let Some(browser) = browser else { return };
            dispatch(Command::LoadingChanged {
                browser: browser.identifier(),
                loading: is_loading != 0,
                can_go_back: can_go_back != 0,
                can_go_forward: can_go_forward != 0,
            });
        }
    }
}
