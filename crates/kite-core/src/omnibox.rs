//! Turns what the user typed into the address bar into a URL.

use crate::SearchEngine;

/// What the address bar input means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Go to this URL.
    Url(String),
    /// Search for this text.
    Search(String),
}

/// Schemes that are always treated as URLs, even without a dot in the host.
const KNOWN_SCHEMES: &[&str] = &[
    "http://",
    "https://",
    "file://",
    "ftp://",
    "about:",
    "chrome://",
    "data:",
    "view-source:",
    "kite://",
];

/// Classifies address bar input. Returns `None` for blank input.
pub fn classify(raw: &str) -> Option<Input> {
    let input = raw.trim();
    if input.is_empty() {
        return None;
    }
    // A leading `?` forces a search, like in Chrome.
    if let Some(query) = input.strip_prefix('?') {
        let query = query.trim();
        return (!query.is_empty()).then(|| Input::Search(query.to_owned()));
    }
    if input.chars().any(char::is_whitespace) {
        return Some(Input::Search(input.to_owned()));
    }
    let lower = input.to_ascii_lowercase();
    if KNOWN_SCHEMES.iter().any(|scheme| lower.starts_with(scheme)) {
        return Some(Input::Url(input.to_owned()));
    }
    match host_kind(input) {
        Some(HostKind::Local) => Some(Input::Url(format!("http://{input}"))),
        Some(HostKind::Public) => Some(Input::Url(format!("https://{input}"))),
        None => Some(Input::Search(input.to_owned())),
    }
}

/// Resolves address bar input to the URL to load. Returns `None` for blank input.
pub fn resolve(raw: &str, engine: &SearchEngine) -> Option<String> {
    Some(match classify(raw)? {
        Input::Url(url) => url,
        Input::Search(query) => engine.url_for(&query),
    })
}

enum HostKind {
    /// Loopback, LAN or development hosts, which usually don't serve HTTPS.
    Local,
    Public,
}

/// Decides whether scheme-less input starts with something that looks like a host.
fn host_kind(input: &str) -> Option<HostKind> {
    let authority = input.split(['/', '?', '#']).next().unwrap_or_default();
    let host = strip_port(authority)?;

    if host.starts_with('[') && host.ends_with(']') {
        return Some(HostKind::Local); // IPv6 literal
    }
    let host = host.to_ascii_lowercase();
    if host == "localhost" || is_ipv4(&host) {
        return Some(HostKind::Local);
    }

    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || !labels.iter().all(|label| is_dns_label(label)) {
        return None;
    }
    let tld = labels[labels.len() - 1];
    if matches!(tld, "localhost" | "local" | "test" | "internal" | "lan") {
        return Some(HostKind::Local);
    }
    let tld_ok =
        tld.starts_with("xn--") || (tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()));
    tld_ok.then_some(HostKind::Public)
}

/// Removes a trailing `:port`. Returns `None` if the port is not numeric.
fn strip_port(authority: &str) -> Option<&str> {
    // IPv6 literals contain colons, so only look after the closing bracket.
    let search_from = authority.rfind(']').map_or(0, |i| i + 1);
    match authority[search_from..].rfind(':') {
        Some(offset) => {
            let colon = search_from + offset;
            let port = &authority[colon + 1..];
            (!port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
                .then(|| &authority[..colon])
        }
        None => Some(authority),
    }
}

fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4 && parts.iter().all(|part| part.parse::<u8>().is_ok())
}

fn is_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Option<Input> {
        Some(Input::Url(s.into()))
    }

    fn search(s: &str) -> Option<Input> {
        Some(Input::Search(s.into()))
    }

    #[test]
    fn blank_input_is_ignored() {
        assert_eq!(classify("   "), None);
        assert_eq!(classify("?  "), None);
    }

    #[test]
    fn full_urls_pass_through() {
        assert_eq!(
            classify("https://example.com/a?b=c"),
            url("https://example.com/a?b=c")
        );
        assert_eq!(classify("HTTP://Example.com"), url("HTTP://Example.com"));
        assert_eq!(classify("about:blank"), url("about:blank"));
        assert_eq!(classify("chrome://gpu"), url("chrome://gpu"));
        assert_eq!(classify("kite://history"), url("kite://history"));
        assert_eq!(
            classify("file:///etc/fedora-release"),
            url("file:///etc/fedora-release")
        );
    }

    #[test]
    fn bare_domains_get_https() {
        assert_eq!(classify("example.com"), url("https://example.com"));
        assert_eq!(classify("docs.rs/cef"), url("https://docs.rs/cef"));
        assert_eq!(
            classify("sub.example.co.uk:8443/x"),
            url("https://sub.example.co.uk:8443/x")
        );
        assert_eq!(
            classify("xn--bcher-kva.example"),
            url("https://xn--bcher-kva.example")
        );
    }

    #[test]
    fn local_hosts_get_http() {
        assert_eq!(classify("localhost:3000"), url("http://localhost:3000"));
        assert_eq!(classify("127.0.0.1/admin"), url("http://127.0.0.1/admin"));
        assert_eq!(classify("[::1]:8080"), url("http://[::1]:8080"));
        assert_eq!(classify("printer.local"), url("http://printer.local"));
        assert_eq!(classify("app.test"), url("http://app.test"));
    }

    #[test]
    fn everything_else_is_a_search() {
        assert_eq!(classify("rust cef bindings"), search("rust cef bindings"));
        assert_eq!(classify("fedora"), search("fedora"));
        assert_eq!(classify("v1.2"), search("v1.2"));
        assert_eq!(classify("example.com:abc"), search("example.com:abc"));
        assert_eq!(classify("-bad-.com"), search("-bad-.com"));
        assert_eq!(classify("999.1.1.1"), search("999.1.1.1"));
    }

    #[test]
    fn question_mark_forces_search() {
        assert_eq!(classify("?example.com"), search("example.com"));
    }

    #[test]
    fn resolve_builds_search_urls() {
        let engine = SearchEngine::duckduckgo();
        assert_eq!(
            resolve("fedora workstation", &engine).as_deref(),
            Some("https://duckduckgo.com/?q=fedora+workstation")
        );
        assert_eq!(
            resolve("example.com", &engine).as_deref(),
            Some("https://example.com")
        );
    }
}
