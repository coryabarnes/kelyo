//! Browser callbacks from the engine, shared by every tab. Each one forwards
//! an event to the shell as a [`Command`], keyed by the browser's id.
//!
//! The request handlers are the exception: they run on CEF's IO thread and
//! answer on the spot, blocking ads and stripping tracking parameters.

use cef::*;
use kite_core::{RequestKind, blocker, clean_url};

use crate::command::{Command, dispatch};
use crate::{filters, shell};

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

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(RequestEvents::new())
        }

        fn focus_handler(&self) -> Option<FocusHandler> {
            Some(FocusEvents::new())
        }
    }
}

wrap_focus_handler! {
    struct FocusEvents;

    impl FocusHandler {
        fn on_got_focus(&self, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                dispatch(Command::BrowserFocused(browser.identifier()));
            }
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

        // Unload handlers ran and the browser may close. For a tab, returning 1
        // stops CEF closing the whole window; the shell instead drops the
        // tab's view, which destroys the browser and leads to `on_before_close`.
        // Popups and DevTools have windows of their own, which CEF closes.
        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            let Some(browser) = browser else { return 0 };
            if !shell::is_tab_browser(browser.identifier()) {
                return 0;
            }
            dispatch(Command::BrowserReadyToClose(browser.identifier()));
            1
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                filters::reset_count(browser.identifier());
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

        // Hide ad slots and run the site's scriptlets as each frame starts loading.
        fn on_load_start(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, _transition_type: TransitionType) {
            let (Some(browser), Some(frame)) = (browser, frame) else { return };
            if blocking_off_for(browser) {
                return;
            }
            let url = CefString::from(&frame.url()).to_string();
            let script = filters::blocker().and_then(|blocker| blocker.page_script(&url));
            if let Some(script) = script {
                frame.execute_java_script(Some(&CefString::from(script.as_str())), Some(&CefString::from(url.as_str())), 0);
            }
        }
    }
}

wrap_request_handler! {
    struct RequestEvents;

    impl RequestHandler {
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: i32,
            _is_download: i32,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut i32>,
        ) -> Option<ResourceRequestHandler> {
            Some(ResourceEvents::new())
        }
    }
}

wrap_resource_request_handler! {
    struct ResourceEvents;

    impl ResourceRequestHandler {
        // Runs on the IO thread for every request.
        fn on_before_resource_load(
            &self,
            browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let (Some(browser), Some(request)) = (browser, request) else { return ReturnValue::CONTINUE };
            let url = CefString::from(&request.url()).to_string();
            let resource_type = request.resource_type();

            if resource_type == ResourceType::MAIN_FRAME {
                // A new page: never blocked, but stripped of tracking parameters.
                filters::reset_count(browser.identifier());
                if let Some(clean) = clean_url::clean(&url) {
                    request.set_url(Some(&CefString::from(clean.as_str())));
                }
                return ReturnValue::CONTINUE;
            }

            let Some(kind) = request_kind(resource_type) else { return ReturnValue::CONTINUE };
            let Some(blocker) = filters::blocker() else { return ReturnValue::CONTINUE };
            let page = page_url(browser);
            if blocker::site_of(&page).is_some_and(|site| filters::is_excepted(&site)) {
                return ReturnValue::CONTINUE;
            }
            if blocker.should_block(&url, &page, kind) {
                filters::count_blocked(browser.identifier());
                ReturnValue::CANCEL
            } else {
                ReturnValue::CONTINUE
            }
        }
    }
}

/// The URL of the page a browser is showing (its main frame).
fn page_url(browser: &Browser) -> String {
    browser
        .main_frame()
        .map(|frame| CefString::from(&frame.url()).to_string())
        .unwrap_or_default()
}

fn blocking_off_for(browser: &Browser) -> bool {
    blocker::site_of(&page_url(browser)).is_some_and(|site| filters::is_excepted(&site))
}

/// How filters see a request type. `None` for requests that are never
/// blocked (pages themselves, and Kite's own favicon and prefetch requests).
fn request_kind(resource_type: ResourceType) -> Option<RequestKind> {
    let kind = if resource_type == ResourceType::SUB_FRAME {
        RequestKind::Subframe
    } else if resource_type == ResourceType::STYLESHEET {
        RequestKind::Stylesheet
    } else if resource_type == ResourceType::SCRIPT
        || resource_type == ResourceType::WORKER
        || resource_type == ResourceType::SHARED_WORKER
        || resource_type == ResourceType::SERVICE_WORKER
    {
        RequestKind::Script
    } else if resource_type == ResourceType::IMAGE {
        RequestKind::Image
    } else if resource_type == ResourceType::FONT_RESOURCE {
        RequestKind::Font
    } else if resource_type == ResourceType::MEDIA {
        RequestKind::Media
    } else if resource_type == ResourceType::OBJECT
        || resource_type == ResourceType::PLUGIN_RESOURCE
    {
        RequestKind::Object
    } else if resource_type == ResourceType::XHR {
        RequestKind::Xhr
    } else if resource_type == ResourceType::PING || resource_type == ResourceType::CSP_REPORT {
        RequestKind::Ping
    } else if resource_type == ResourceType::MAIN_FRAME
        || resource_type == ResourceType::NAVIGATION_PRELOAD_MAIN_FRAME
        || resource_type == ResourceType::FAVICON
    {
        return None;
    } else {
        RequestKind::Other
    };
    Some(kind)
}
