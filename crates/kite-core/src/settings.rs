//! User-facing browser settings.

/// A search engine, described by a URL template containing `{query}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEngine {
    pub name: String,
    pub template: String,
}

impl SearchEngine {
    pub fn duckduckgo() -> Self {
        Self {
            name: "DuckDuckGo".into(),
            template: "https://duckduckgo.com/?q={query}".into(),
        }
    }

    /// Builds the search URL for `query`, percent-encoding it.
    pub fn url_for(&self, query: &str) -> String {
        self.template.replace("{query}", &encode_query(query))
    }
}

/// Encodes `input` for use as a URL query value (`application/x-www-form-urlencoded`).
fn encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Page loaded in every new tab.
    pub new_tab_url: String,
    pub search: SearchEngine,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_tab_url: "https://duckduckgo.com/".into(),
            search: SearchEngine::duckduckgo(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_spaces_and_reserved_characters() {
        let engine = SearchEngine::duckduckgo();
        assert_eq!(
            engine.url_for("rust & cef?"),
            "https://duckduckgo.com/?q=rust+%26+cef%3F"
        );
    }

    #[test]
    fn encodes_non_ascii_as_utf8_bytes() {
        let engine = SearchEngine::duckduckgo();
        assert_eq!(
            engine.url_for("café"),
            "https://duckduckgo.com/?q=caf%C3%A9"
        );
    }
}
