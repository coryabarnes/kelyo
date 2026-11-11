//! Engine-independent core of the Kite browser.
//!
//! Nothing in this crate touches CEF or any UI toolkit, so all of it is
//! unit-testable with plain `cargo test`.

pub mod omnibox;
pub mod paths;
pub mod settings;
pub mod tabs;

pub use settings::{SearchEngine, Settings};
pub use tabs::{Placement, Split, Tab, TabId, TabStrip};
