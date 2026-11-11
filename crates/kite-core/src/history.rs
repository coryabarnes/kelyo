//! Browsing history: one row per URL, with its title, visit count and last
//! visit. Also answers the address bar's inline completion.

use rusqlite::params;
use url::Url;

use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub url: String,
    pub title: String,
    pub visit_count: u32,
    /// Unix time, in seconds.
    pub last_visit: i64,
}

/// Only web pages go into history, not `kite://` pages, `data:` URLs or files.
pub fn is_recordable(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// `url` without its scheme or a leading `www.`, plus its host (also without `www.`).
fn strip(url: &str) -> Option<(String, String)> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    let rest = url.split_once("://")?.1;
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    Some((rest.to_owned(), host))
}

/// Escapes `%`, `_` and `\` for a `LIKE ... ESCAPE '\'` pattern.
fn like_escape(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_")
}

impl Store {
    /// Counts a visit to `url` at `now` (Unix seconds).
    pub fn record_visit(&self, url: &str, now: i64) -> rusqlite::Result<()> {
        let Some((stripped, host)) = strip(url).filter(|_| is_recordable(url)) else {
            return Ok(());
        };
        self.conn.execute(
            "INSERT INTO visits (url, stripped, host, visit_count, last_visit)
             VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT (url) DO UPDATE
             SET visit_count = visit_count + 1, last_visit = excluded.last_visit",
            params![url, stripped, host, now],
        )?;
        Ok(())
    }

    /// Records the title of a page already in history.
    pub fn set_title(&self, url: &str, title: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE visits SET title = ?2 WHERE url = ?1",
            params![url, title],
        )?;
        Ok(())
    }

    /// History entries, most recent first. A non-empty `query` keeps only
    /// entries whose URL or title contains every word of it.
    pub fn search_history(&self, query: &str, limit: usize) -> rusqlite::Result<Vec<Visit>> {
        let words: Vec<String> = query
            .split_whitespace()
            .map(|word| format!("%{}%", like_escape(word)))
            .collect();
        let mut sql =
            String::from("SELECT url, title, visit_count, last_visit FROM visits WHERE 1");
        for i in 1..=words.len() {
            sql.push_str(&format!(
                " AND (url LIKE ?{i} ESCAPE '\\' OR title LIKE ?{i} ESCAPE '\\')"
            ));
        }
        sql.push_str(&format!(" ORDER BY last_visit DESC LIMIT {limit}"));
        let mut statement = self.conn.prepare(&sql)?;
        statement
            .query_map(rusqlite::params_from_iter(&words), |row| {
                Ok(Visit {
                    url: row.get(0)?,
                    title: row.get(1)?,
                    visit_count: row.get(2)?,
                    last_visit: row.get(3)?,
                })
            })?
            .collect()
    }

    pub fn delete_from_history(&self, url: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM visits WHERE url = ?1", [url])?;
        Ok(())
    }

    pub fn clear_history(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM visits", [])?;
        Ok(())
    }

    /// Inline completion for address bar input: the most visited site (or,
    /// once the input has a `/`, the most visited page) that starts with
    /// `typed`, returned as `typed` plus the rest. `None` if nothing matches
    /// or the input already is a whole match.
    pub fn complete(&self, typed: &str) -> rusqlite::Result<Option<String>> {
        if typed.is_empty() || typed.chars().any(char::is_whitespace) {
            return Ok(None);
        }
        let lower = typed.to_ascii_lowercase();
        let (scheme, rest) = match lower.split_once("://") {
            Some((scheme, rest)) => (Some(scheme), rest),
            None => (None, lower.as_str()),
        };
        let needle = rest.strip_prefix("www.").unwrap_or(rest);
        if needle.is_empty() {
            return Ok(None);
        }
        let pattern = format!("{}%", like_escape(needle));

        let candidate: Option<String> = if needle.contains('/') || scheme.is_some() {
            let scheme_filter = match scheme {
                Some(_) => "AND url LIKE ?2 ESCAPE '\\'",
                None => "",
            };
            let sql = format!(
                "SELECT stripped FROM visits WHERE stripped LIKE ?1 ESCAPE '\\' {scheme_filter}
                 ORDER BY visit_count DESC, last_visit DESC LIMIT 1"
            );
            let scheme_pattern = format!("{}://%", like_escape(scheme.unwrap_or_default()));
            let mut statement = self.conn.prepare(&sql)?;
            let mut rows = match scheme {
                Some(_) => statement.query(params![pattern, scheme_pattern])?,
                None => statement.query(params![pattern])?,
            };
            rows.next()?.map(|row| row.get(0)).transpose()?
        } else {
            let mut statement = self.conn.prepare(
                "SELECT host FROM visits WHERE host LIKE ?1 ESCAPE '\\'
                 GROUP BY host ORDER BY SUM(visit_count) DESC, MAX(last_visit) DESC LIMIT 1",
            )?;
            let mut rows = statement.query([pattern])?;
            rows.next()?.map(|row| row.get(0)).transpose()?
        };

        Ok(candidate
            .filter(|candidate: &String| {
                candidate.len() > needle.len() && candidate.is_char_boundary(needle.len())
            })
            .map(|candidate| format!("{typed}{}", &candidate[needle.len()..])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(visits: &[(&str, u32)]) -> Store {
        let store = Store::in_memory();
        for (time, (url, count)) in visits.iter().enumerate() {
            for _ in 0..*count {
                store.record_visit(url, time as i64).unwrap();
            }
        }
        store
    }

    #[test]
    fn records_and_counts_visits_with_titles() {
        let store = store_with(&[("https://a.example/", 2), ("https://b.example/", 1)]);
        store.set_title("https://a.example/", "A").unwrap();
        let visits = store.search_history("", 10).unwrap();
        assert_eq!(visits.len(), 2);
        assert_eq!(visits[0].url, "https://b.example/", "most recent first");
        assert_eq!((visits[1].title.as_str(), visits[1].visit_count), ("A", 2));
    }

    #[test]
    fn skips_non_web_urls() {
        let store = store_with(&[
            ("kite://history", 1),
            ("data:text/html,hi", 1),
            ("about:blank", 1),
        ]);
        assert!(store.search_history("", 10).unwrap().is_empty());
    }

    #[test]
    fn search_matches_every_word_in_url_or_title() {
        let store = store_with(&[("https://docs.rs/cef", 1), ("https://crates.io/", 1)]);
        store
            .set_title("https://docs.rs/cef", "cef - Rust")
            .unwrap();
        let urls = |q| -> Vec<String> {
            store
                .search_history(q, 10)
                .unwrap()
                .into_iter()
                .map(|v| v.url)
                .collect()
        };
        assert_eq!(urls("rust docs"), vec!["https://docs.rs/cef"]);
        assert!(urls("rust crates").is_empty());
        assert!(urls("100%").is_empty(), "LIKE wildcards are literal");
    }

    #[test]
    fn delete_and_clear() {
        let store = store_with(&[("https://a.example/", 1), ("https://b.example/", 1)]);
        store.delete_from_history("https://a.example/").unwrap();
        assert_eq!(store.search_history("", 10).unwrap().len(), 1);
        store.clear_history().unwrap();
        assert!(store.search_history("", 10).unwrap().is_empty());
    }

    #[test]
    fn completes_hosts_by_total_visits() {
        let store = store_with(&[
            ("https://github.com/a", 2),
            ("https://github.com/b", 2),
            ("https://www.gitlab.com/", 3),
        ]);
        assert_eq!(
            store.complete("git").unwrap().as_deref(),
            Some("github.com")
        );
        assert_eq!(
            store.complete("gitl").unwrap().as_deref(),
            Some("gitlab.com")
        );
        assert_eq!(
            store.complete("www.gitl").unwrap().as_deref(),
            Some("www.gitlab.com")
        );
        assert_eq!(
            store.complete("GitH").unwrap().as_deref(),
            Some("GitHub.com")
        );
        assert_eq!(
            store.complete("github.com").unwrap(),
            None,
            "already complete"
        );
    }

    #[test]
    fn completes_paths_once_there_is_a_slash() {
        let store = store_with(&[
            ("https://github.com/rust-lang/rust", 3),
            ("https://github.com/tauri-apps", 1),
        ]);
        assert_eq!(
            store.complete("github.com/r").unwrap().as_deref(),
            Some("github.com/rust-lang/rust")
        );
        assert_eq!(
            store.complete("https://github.com/t").unwrap().as_deref(),
            Some("https://github.com/tauri-apps")
        );
        assert_eq!(
            store.complete("http://github.com/t").unwrap(),
            None,
            "scheme must match"
        );
    }

    #[test]
    fn does_not_complete_searches() {
        let store = store_with(&[("https://rust-lang.org/", 1)]);
        assert_eq!(store.complete("rust lang").unwrap(), None);
        assert_eq!(store.complete("").unwrap(), None);
        assert_eq!(
            store.complete("_").unwrap(),
            None,
            "LIKE wildcards are literal"
        );
    }
}
