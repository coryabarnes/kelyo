//! Serves Kite's own pages (`kite://history`) to the engine.
//!
//! CEF asks the factory for a handler on its IO thread, once per request.
//! Each request opens its own connection to the profile database, since the
//! shell's connection belongs to the UI thread.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cef::*;
use kite_core::{Store, pages};

pub const SCHEME: &str = "kite";

/// Registers `kite://` with the engine. Called in every process, before CEF starts.
pub fn register_scheme(registrar: &mut SchemeRegistrar) {
    // Display-isolated: web pages can't link to, frame or redirect to Kite's
    // pages (which can delete history). Only the user can open them.
    let options = sys::cef_scheme_options_t::CEF_SCHEME_OPTION_STANDARD as i32
        | sys::cef_scheme_options_t::CEF_SCHEME_OPTION_SECURE as i32
        | sys::cef_scheme_options_t::CEF_SCHEME_OPTION_DISPLAY_ISOLATED as i32;
    registrar.add_custom_scheme(Some(&CefString::from(SCHEME)), options);
}

/// Starts answering `kite://` requests. Call once CEF's context is ready.
pub fn register_handler(database: Option<PathBuf>) {
    let mut factory = PageFactory::new(database);
    register_scheme_handler_factory(Some(&CefString::from(SCHEME)), None, Some(&mut factory));
}

wrap_scheme_handler_factory! {
    struct PageFactory {
        database: Option<PathBuf>,
    }

    impl SchemeHandlerFactory {
        fn create(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _scheme_name: Option<&CefString>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let url = CefString::from(&request?.url()).to_string();
            let store = match &self.database {
                Some(path) => Store::open(path).unwrap_or_else(|error| {
                    eprintln!("kite: can't open {}: {error}", path.display());
                    Store::in_memory()
                }),
                None => Store::in_memory(),
            };
            let page = pages::respond(&url, &store);
            Some(PageResponse::new(Arc::new(Mutex::new(PageState {
                page,
                offset: 0,
            }))))
        }
    }
}

struct PageState {
    page: pages::Page,
    /// How much of the body has been read.
    offset: usize,
}

wrap_resource_handler! {
    struct PageResponse {
        response: Arc<Mutex<PageState>>,
    }

    impl ResourceHandler {
        fn open(&self, _request: Option<&mut Request>, handle_request: Option<&mut i32>, _callback: Option<&mut Callback>) -> i32 {
            // The page is ready: respond right away.
            if let Some(handle_request) = handle_request {
                *handle_request = 1;
            }
            1
        }

        fn response_headers(&self, response: Option<&mut Response>, response_length: Option<&mut i64>, redirect_url: Option<&mut CefString>) {
            let Ok(state) = self.response.lock() else { return };
            let page = &state.page;
            if let Some(response) = response {
                response.set_status(page.status);
                response.set_mime_type(Some(&CefString::from(page.mime_type)));
                response.set_charset(Some(&CefString::from("utf-8")));
            }
            if let (Some(target), Some(redirect)) = (redirect_url, &page.redirect) {
                *target = CefString::from(redirect.as_str());
            }
            if let Some(length) = response_length {
                *length = page.body.len() as i64;
            }
        }

        fn read(&self, data_out: *mut u8, bytes_to_read: i32, bytes_read: Option<&mut i32>, _callback: Option<&mut ResourceReadCallback>) -> i32 {
            let (Ok(mut state), Some(bytes_read)) = (self.response.lock(), bytes_read) else { return 0 };
            let body = state.page.body.as_bytes();
            let remaining = &body[state.offset.min(body.len())..];
            let count = remaining.len().min(usize::try_from(bytes_to_read).unwrap_or(0));
            if count == 0 {
                *bytes_read = 0;
                return 0; // Done.
            }
            // SAFETY: CEF hands us a buffer of at least `bytes_to_read` bytes,
            // and `count` is no larger.
            unsafe { std::ptr::copy_nonoverlapping(remaining.as_ptr(), data_out, count) };
            *bytes_read = count as i32;
            state.offset += count;
            1
        }

        fn cancel(&self) {}
    }
}
