//! CEF application and browser-process setup.

use cef::*;
use kite_core::{Settings as KiteSettings, paths};

/// Global CEF settings: sandbox and on-disk profile location.
pub fn cef_settings() -> Settings {
    let mut settings = Settings {
        no_sandbox: (!cfg!(feature = "sandbox")).into(),
        persist_session_cookies: 1,
        ..Default::default()
    };

    // Without a cache path CEF runs every browser in incognito mode, so
    // logins, cookies and history would vanish on exit.
    match paths::data_dir() {
        Some(root) => {
            let profile = root.join("Default");
            // Kite restores its own tabs. Chromium's saved sessions would make
            // it reopen them too, after a crash, in a bare Chromium window.
            for sessions in ["Sessions", "Sessions_Encrypted"] {
                let _ = std::fs::remove_dir_all(profile.join(sessions));
            }
            settings.root_cache_path = CefString::from(root.to_string_lossy().as_ref());
            settings.cache_path = CefString::from(profile.to_string_lossy().as_ref());
        }
        None => eprintln!("kite: no home directory found; using a temporary profile"),
    }
    settings
}

wrap_app! {
    pub struct KiteApp;

    impl App {
        // Runs in every process, so the renderer knows `kite://` too.
        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) {
            if let Some(registrar) = registrar {
                crate::scheme::register_scheme(registrar);
            }
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(KiteBrowserProcessHandler::new())
        }
    }
}

wrap_browser_process_handler! {
    struct KiteBrowserProcessHandler;

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);
            crate::filters::start(paths::filters_dir());
            crate::scheme::register_handler(paths::database());
            crate::shell::start(KiteSettings::default());
        }
    }
}
