//! Ad and tracker blocking with uBlock Origin's filter lists, matched by
//! Brave's `adblock` engine.
//!
//! The browser feeds every sub-resource request through
//! [`Blocker::should_block`], and runs [`Blocker::page_script`] in each frame
//! as it starts loading, to hide ad slots and neutralise ad scripts
//! (scriptlets). Downloading and caching the lists lives in the browser crate.

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::resources::Resource;

/// A filter list Kite subscribes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterList {
    /// Also the file name of the cached copy.
    pub name: &'static str,
    pub url: &'static str,
}

/// uBlock Origin's default lists, plus its cookie-banner list.
pub const FILTER_LISTS: &[FilterList] = &[
    FilterList {
        name: "ublock-filters",
        url: "https://ublockorigin.github.io/uAssets/filters/filters.txt",
    },
    FilterList {
        name: "ublock-badware",
        url: "https://ublockorigin.github.io/uAssets/filters/badware.txt",
    },
    FilterList {
        name: "ublock-privacy",
        url: "https://ublockorigin.github.io/uAssets/filters/privacy.txt",
    },
    FilterList {
        name: "ublock-quick-fixes",
        url: "https://ublockorigin.github.io/uAssets/filters/quick-fixes.txt",
    },
    FilterList {
        name: "ublock-unbreak",
        url: "https://ublockorigin.github.io/uAssets/filters/unbreak.txt",
    },
    FilterList {
        name: "easylist",
        url: "https://easylist.to/easylist/easylist.txt",
    },
    FilterList {
        name: "easyprivacy",
        url: "https://easylist.to/easylist/easyprivacy.txt",
    },
    FilterList {
        name: "peter-lowe",
        url: "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext",
    },
    FilterList {
        name: "easylist-cookie",
        url: "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt",
    },
];

/// Brave's own scriptlets, which some `+js(...)` filters use.
///
/// uBlock Origin's scriptlet library (which most `+js(...)` filters use,
/// including the ones against YouTube video ads) isn't in it: uBlock ships it
/// as JavaScript modules that must be converted first. Until Kite does that,
/// those filters do nothing.
pub const RESOURCES_URL: &str =
    "https://raw.githubusercontent.com/brave/adblock-resources/master/dist/resources.json";

/// What kind of resource a request fetches, as far as filters care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    Document,
    Subframe,
    Stylesheet,
    Script,
    Image,
    Font,
    Media,
    Object,
    Xhr,
    Ping,
    Websocket,
    Other,
}

impl RequestKind {
    /// The name filter options (`$script`, `$image`, ...) use for this kind.
    fn filter_name(self) -> &'static str {
        match self {
            RequestKind::Document => "document",
            RequestKind::Subframe => "sub_frame",
            RequestKind::Stylesheet => "stylesheet",
            RequestKind::Script => "script",
            RequestKind::Image => "image",
            RequestKind::Font => "font",
            RequestKind::Media => "media",
            RequestKind::Object => "object",
            RequestKind::Xhr => "xmlhttprequest",
            RequestKind::Ping => "ping",
            RequestKind::Websocket => "websocket",
            RequestKind::Other => "other",
        }
    }
}

/// The key per-site settings use for the page at `url`: its host, without
/// `www.`. `None` for pages that aren't on the web.
pub fn site_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
}

pub struct Blocker {
    engine: Engine,
}

impl Blocker {
    /// Compiles filter list texts into a blocker. This takes a second or two
    /// for the full set of lists, so the browser does it off the UI thread.
    pub fn from_lists<'a>(lists: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = FilterSet::new(false);
        for list in lists {
            set.add_filter_list(list.to_owned(), ParseOptions::default());
        }
        Self {
            engine: Engine::new_with_filter_set(set),
        }
    }

    /// Loads a blocker saved with [`Blocker::serialize`]. Returns `None` if the
    /// data is from an incompatible engine version or corrupt.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let mut engine = Engine::default();
        engine.deserialize(data).ok()?;
        Some(Self { engine })
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.engine.serialize()
    }

    /// Loads scriptlets from a `resources.json` (see [`RESOURCES_URL`]).
    /// `+js(...)` filters naming a scriptlet that isn't loaded do nothing.
    pub fn use_resources_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let resources: Vec<Resource> = serde_json::from_str(json)?;
        self.engine.use_resources(resources);
        Ok(())
    }

    /// Whether a request for `url`, made by a page at `source_url`, should be cancelled.
    pub fn should_block(&self, url: &str, source_url: &str, kind: RequestKind) -> bool {
        Request::new(url, source_url, kind.filter_name(), "GET")
            .is_ok_and(|request| self.engine.check_network_request(&request).should_block())
    }

    /// JavaScript to run in a frame showing `url` as it starts loading: hides
    /// matching elements and runs the site's scriptlets. `None` if there's
    /// nothing to do.
    pub fn page_script(&self, url: &str) -> Option<String> {
        let resources = self.engine.url_cosmetic_resources(url);
        let mut selectors: Vec<&str> = resources
            .hide_selectors
            .iter()
            .map(String::as_str)
            .collect();
        if selectors.is_empty() && resources.injected_script.is_empty() {
            return None;
        }
        selectors.sort_unstable();

        let mut script = String::new();
        if !selectors.is_empty() {
            let css = format!("{}{{display:none!important}}", selectors.join(",\n"));
            let css = serde_json::to_string(&css).expect("strings always serialize");
            script.push_str(&format!(
                "(()=>{{const s=document.createElement('style');s.textContent={css};\
                 (document.head||document.documentElement).appendChild(s);}})();\n"
            ));
        }
        if !resources.injected_script.is_empty() {
            // One failing scriptlet must not stop the style above or the page.
            script.push_str("try{\n");
            script.push_str(&resources.injected_script);
            script.push_str("\n}catch(e){}\n");
        }
        Some(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocker(rules: &str) -> Blocker {
        Blocker::from_lists([rules])
    }

    #[test]
    fn blocks_matching_requests_only() {
        let blocker = blocker("||ads.example^\n||tracker.example^$script");
        let page = "https://news.example/story";
        assert!(blocker.should_block("https://ads.example/banner.png", page, RequestKind::Image));
        assert!(blocker.should_block("https://tracker.example/t.js", page, RequestKind::Script));
        assert!(!blocker.should_block(
            "https://tracker.example/pixel.gif",
            page,
            RequestKind::Image
        ));
        assert!(!blocker.should_block("https://news.example/app.js", page, RequestKind::Script));
    }

    #[test]
    fn exceptions_win() {
        let blocker = blocker("||cdn.example^\n@@||cdn.example/player.js");
        let page = "https://site.example/";
        assert!(blocker.should_block("https://cdn.example/ad.js", page, RequestKind::Script));
        assert!(!blocker.should_block("https://cdn.example/player.js", page, RequestKind::Script));
    }

    #[test]
    fn third_party_option_uses_the_source_page() {
        let blocker = blocker("||widgets.example^$third-party");
        let url = "https://widgets.example/w.js";
        assert!(blocker.should_block(url, "https://other.example/", RequestKind::Script));
        assert!(!blocker.should_block(url, "https://widgets.example/", RequestKind::Script));
    }

    #[test]
    fn page_script_hides_site_specific_elements() {
        let blocker = blocker("news.example##.sponsored\nnews.example##div[id^=\"ad-\"]");
        let script = blocker.page_script("https://news.example/story").unwrap();
        assert!(script.contains(".sponsored"));
        assert!(script.contains("display:none!important"));
        assert!(blocker.page_script("https://elsewhere.example/").is_none());
    }

    #[test]
    fn site_is_the_host_without_www() {
        assert_eq!(
            site_of("https://www.Example.com/a?b").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            site_of("http://sub.example.com:8080/").as_deref(),
            Some("sub.example.com")
        );
        assert_eq!(site_of("kite://history"), None);
        assert_eq!(site_of("about:blank"), None);
    }

    #[test]
    fn survives_a_serialization_round_trip() {
        let original = blocker("||ads.example^");
        let restored = Blocker::deserialize(&original.serialize()).unwrap();
        assert!(restored.should_block(
            "https://ads.example/x",
            "https://a.example/",
            RequestKind::Image
        ));
        assert!(Blocker::deserialize(b"garbage").is_none());
    }
}
