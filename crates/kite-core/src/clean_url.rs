//! Strips tracking parameters (`utm_source`, `fbclid`, ...) from URLs.
//!
//! Applied to every page navigation, so the address bar, history and copied
//! links never carry them.

use url::Url;

/// Query parameters that only exist to track clicks, on any site.
const TRACKING_PARAMS: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "msclkid",
    "yclid",
    "twclid",
    "ttclid",
    "li_fat_id",
    "igshid",
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "mkt_tok",
    "oly_anon_id",
    "oly_enc_id",
    "vero_id",
    "wickedid",
    "rb_clickid",
    "s_cid",
    "_openstat",
    "srsltid",
];

/// Parameter prefixes that only exist to track clicks, on any site.
const TRACKING_PREFIXES: &[&str] = &["utm_", "pk_", "mtm_", "hsa_", "pd_rd_", "pf_rd_"];

/// Parameters that track only on particular sites; elsewhere they may mean something.
const SITE_PARAMS: &[(&str, &[&str])] = &[
    ("youtube.com", &["si", "pp", "feature"]),
    ("youtu.be", &["si", "feature"]),
    ("spotify.com", &["si", "context"]),
    ("twitter.com", &["s", "t", "ref_src", "ref_url"]),
    ("x.com", &["s", "t", "ref_src", "ref_url"]),
    ("instagram.com", &["igsh", "img_index"]),
    (
        "amazon.com",
        &["ref", "ref_", "_encoding", "content-id", "psc"],
    ),
    ("reddit.com", &["share_id", "rdt"]),
    ("linkedin.com", &["trk", "trackingId", "lipi"]),
];

/// Returns `url` without tracking parameters, or `None` if it has none (or
/// isn't an http(s) URL).
pub fn clean(url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.query().is_none() {
        return None;
    }
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let site_params: Vec<&str> = SITE_PARAMS
        .iter()
        .filter(|(site, _)| host == *site || host.ends_with(&format!(".{site}")))
        .flat_map(|(_, params)| params.iter().copied())
        .collect();
    let is_tracking = |name: &str| {
        let lower = name.to_ascii_lowercase();
        TRACKING_PARAMS.contains(&lower.as_str())
            || TRACKING_PREFIXES
                .iter()
                .any(|prefix| lower.starts_with(prefix))
            || site_params.contains(&name)
    };

    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    let kept: Vec<&(String, String)> = pairs
        .iter()
        .filter(|(name, _)| !is_tracking(name))
        .collect();
    if kept.len() == pairs.len() {
        return None;
    }
    if kept.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.query_pairs_mut().clear().extend_pairs(kept);
    }
    Some(parsed.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_generic_trackers_and_keeps_the_rest() {
        assert_eq!(
            clean("https://example.com/a?id=7&utm_source=x&utm_medium=y&fbclid=abc#top").as_deref(),
            Some("https://example.com/a?id=7#top")
        );
    }

    #[test]
    fn drops_the_question_mark_when_nothing_is_left() {
        assert_eq!(
            clean("https://example.com/?gclid=1&UTM_Campaign=2").as_deref(),
            Some("https://example.com/")
        );
    }

    #[test]
    fn site_params_only_apply_on_their_site() {
        assert_eq!(
            clean("https://www.youtube.com/watch?v=dQw4w9WgXcQ&si=abc").as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
        assert_eq!(clean("https://example.com/search?si=1&s=rust"), None);
        assert_eq!(
            clean("https://x.com/user/status/1?s=20&t=abc").as_deref(),
            Some("https://x.com/user/status/1")
        );
    }

    #[test]
    fn leaves_clean_and_non_web_urls_alone() {
        assert_eq!(clean("https://example.com/a?q=rust"), None);
        assert_eq!(clean("https://example.com/"), None);
        assert_eq!(clean("kite://history?utm_source=x"), None);
        assert_eq!(clean("not a url"), None);
    }
}
