//! Engine-independent core of the Kite browser.
//!
//! Nothing in this crate touches CEF or any UI toolkit, so all of it is
//! unit-testable with plain `cargo test`.

pub mod blocker;
pub mod clean_url;
pub mod history;
pub mod omnibox;
pub mod pages;
pub mod paths;
pub mod session;
pub mod settings;
pub mod store;
pub mod tabs;

pub use blocker::{Blocker, RequestKind};
pub use session::Session;
pub use settings::{SearchEngine, Settings};
pub use store::Store;
pub use tabs::{MAX_PANES, Placement, Split, Tab, TabId, TabStrip};
