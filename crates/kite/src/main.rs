//! Kite: a sidebar-first browser built on Chromium (via CEF) with a Rust shell.
//!
//! Layout of this crate:
//! - `app`      CEF process setup; starts the shell once CEF is ready.
//! - `command`  Every user action and engine event, queued onto the UI thread.
//! - `shell`    Owns the tab model and all views; the only place state changes.
//! - `ui`       Builds the sidebar and content area out of CEF Views widgets.
//! - `client`   CEF browser callbacks (title, address, loading, popups, ad blocking).
//! - `filters`  Downloads filter lists and holds the ad blocker.
//! - `scheme`   Serves `kite://` pages (history).
//! - `views`    CEF Views delegates (windows, tab views, buttons, address bar).
//! - `shortcuts` Keyboard shortcuts.
//! - `theme`    Colours and sizes.

mod app;
mod client;
mod command;
mod filters;
mod scheme;
mod shell;
mod shortcuts;
mod theme;
mod ui;
mod views;

use std::process::ExitCode;

use cef::{args::Args, *};

// libcef must load before libc. Chromium replaces `close()` and finds libc's
// with `dlsym(RTLD_NEXT)`, which only searches libraries loaded after libcef;
// otherwise it aborts with "close symbol missing". Linking it from this crate
// puts it first, ahead of the libraries other crates (like `libc`) pull in.
#[link(name = "cef")]
unsafe extern "C" {}

fn main() -> ExitCode {
    // Must run before any other CEF call.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();
    let mut app = app::KiteApp::new();

    // Chromium re-launches this binary for its renderer, GPU and utility
    // processes. For those, `execute_process` runs the process and returns its
    // exit code; for the main browser process it returns -1.
    let code = execute_process(
        Some(args.as_main_args()),
        Some(&mut app),
        std::ptr::null_mut(),
    );
    if code >= 0 {
        return ExitCode::from(u8::try_from(code).unwrap_or(1));
    }

    let settings = app::cef_settings();
    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    ) != 1
    {
        eprintln!("kite: failed to initialize CEF");
        return ExitCode::FAILURE;
    }

    run_message_loop();
    shutdown();
    ExitCode::SUCCESS
}
