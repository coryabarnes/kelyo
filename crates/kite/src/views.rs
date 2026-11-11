//! CEF Views delegates. They hold no shell state: each one either answers a
//! layout question or turns an event into a [`Command`].

use std::cell::RefCell;

use cef::*;
use kite_core::TabId;

use crate::command::{Command, dispatch};
use crate::shortcuts::{self, key};
use crate::{shell, theme};

wrap_window_delegate! {
    // The main browser window. Holds the root view until the window is created.
    pub struct MainWindow {
        root: RefCell<Option<Panel>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: theme::WINDOW_WIDTH, height: theme::WINDOW_HEIGHT }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            if let Some(root) = self.root.borrow_mut().take() {
                window.add_child_view(Some(&mut View::from(&root)));
            }
            window.set_background_color(theme::WINDOW_BG);
            window.set_title(Some(&CefString::from("Kite")));
            shortcuts::register(window);
            window.show();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            quit_message_loop();
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // First let every tab run its unload handlers; the shell closes the
            // window again once the last browser is gone.
            if shell::ready_to_close() {
                1
            } else {
                dispatch(Command::CloseWindow);
                0
            }
        }

        fn on_accelerator(&self, _window: Option<&mut Window>, command_id: i32) -> i32 {
            match shortcuts::command(command_id) {
                Some(command) => {
                    dispatch(command);
                    1
                }
                None => 0,
            }
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}

wrap_window_delegate! {
    // A window opened by a page with `window.open()` (sign-in flows, etc.).
    pub struct PopupWindow {
        browser_view: RefCell<Option<BrowserView>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 800, height: 600 }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            if let Some(view) = self.browser_view.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(view)));
            }
            window.show();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            // Break the window -> delegate -> view reference cycle.
            self.browser_view.borrow_mut().take();
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let host = self.browser_view.borrow().as_ref().and_then(|view| view.browser()).and_then(|b| b.host());
            match host {
                Some(host) => host.try_close_browser(),
                None => 1,
            }
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}

wrap_browser_view_delegate! {
    // Delegate for the web view of one tab.
    pub struct TabView {
        tab: TabId,
    }

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn on_browser_created(&self, _view: Option<&mut BrowserView>, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                dispatch(Command::BrowserCreated { tab: self.tab, browser: browser.identifier() });
            }
        }

        fn on_popup_browser_view_created(
            &self,
            _view: Option<&mut BrowserView>,
            popup: Option<&mut BrowserView>,
            _is_devtools: i32,
        ) -> i32 {
            let mut window = PopupWindow::new(RefCell::new(popup.cloned()));
            window_create_top_level(Some(&mut window));
            1
        }

        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}

wrap_textfield_delegate! {
    // The address bar: Enter navigates, Escape restores the current URL.
    pub struct AddressBar;

    impl ViewDelegate {}

    impl TextfieldDelegate {
        fn on_key_event(&self, textfield: Option<&mut Textfield>, event: Option<&KeyEvent>) -> i32 {
            let (Some(textfield), Some(event)) = (textfield, event) else { return 0 };
            if event.type_ != KeyEventType::RAWKEYDOWN {
                return 0;
            }
            match event.windows_key_code {
                key::RETURN => {
                    let input = CefString::from(&textfield.text()).to_string();
                    dispatch(Command::Navigate(input));
                    1
                }
                key::ESCAPE => {
                    dispatch(Command::ResetAddressBar);
                    1
                }
                _ => 0,
            }
        }
    }
}

wrap_button_delegate! {
    // A button that dispatches a fixed command when pressed.
    pub struct CommandButton {
        command: Command,
    }

    impl ViewDelegate {}

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            dispatch(self.command.clone());
        }
    }
}

wrap_panel_delegate! {
    // Gives a panel a fixed preferred size (a zero dimension would mean "unset").
    pub struct FixedSize {
        width: i32,
        height: i32,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: self.width, height: self.height }
        }
    }

    impl PanelDelegate {}
}
