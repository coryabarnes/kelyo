//! Kite's own pages, served from the `kite://` scheme.
//!
//! - `kite://history` (optionally `?q=words`): search and browse history.
//! - `kite://history/delete?url=...`: forget one page, then back to history.
//! - `kite://history/clear`: forget everything, then back to history.
//!
//! The scheme is registered as display-isolated, so web pages can't link to
//! or embed these pages: only the user (via the address bar) can open them.

use url::Url;

use crate::history::Visit;
use crate::store::Store;

/// How many entries the history page shows at most.
const HISTORY_LIMIT: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub status: i32,
    pub mime_type: &'static str,
    pub body: String,
    /// Set for redirects; the body is then empty.
    pub redirect: Option<String>,
}

impl Page {
    fn html(status: i32, body: String) -> Self {
        Self {
            status,
            mime_type: "text/html",
            body,
            redirect: None,
        }
    }

    fn redirect(to: &str) -> Self {
        Self {
            status: 302,
            mime_type: "text/html",
            body: String::new(),
            redirect: Some(to.to_owned()),
        }
    }
}

/// Answers a request for a `kite://` URL.
pub fn respond(url: &str, store: &Store) -> Page {
    let Ok(parsed) = Url::parse(url) else {
        return not_found();
    };
    let param = |name: &str| {
        parsed
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default()
    };
    let path = parsed.path().trim_end_matches('/');
    match (parsed.host_str(), path) {
        (Some("history"), "") => {
            let query = param("q");
            match store.search_history(&query, HISTORY_LIMIT) {
                Ok(visits) => Page::html(200, history_page(&query, &visits)),
                Err(error) => {
                    Page::html(500, error_page(&format!("Couldn't read history: {error}")))
                }
            }
        }
        (Some("history"), "/delete") => {
            let _ = store.delete_from_history(&param("url"));
            let query = param("q");
            if query.is_empty() {
                Page::redirect("kite://history")
            } else {
                let mut back = Url::parse("kite://history").expect("valid URL");
                back.query_pairs_mut().append_pair("q", &query);
                Page::redirect(back.as_str())
            }
        }
        (Some("history"), "/clear") => {
            let _ = store.clear_history();
            Page::redirect("kite://history")
        }
        _ => not_found(),
    }
}

fn not_found() -> Page {
    Page::html(404, error_page("There's no Kite page at this address."))
}

/// Escapes text for HTML element content and double-quoted attributes.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn encode_component(text: &str) -> String {
    url::form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

const STYLE: &str = r#"
:root { color-scheme: dark; --bg: #16161d; --panel: #1f1f28; --hover: #2a2a36; --text: #e6e6ee; --dim: #8c8c9a; --accent: #4f9cf9; }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--text); font: 14px/1.45 system-ui, sans-serif; }
main { max-width: 820px; margin: 0 auto; padding: 32px 16px 64px; }
h1 { font-size: 22px; font-weight: 600; margin: 0 0 16px; }
h2 { font-size: 13px; font-weight: 600; color: var(--dim); margin: 24px 0 6px; text-transform: uppercase; letter-spacing: .04em; }
form { display: flex; gap: 8px; margin-bottom: 8px; }
input[type=search] { flex: 1; background: var(--panel); color: var(--text); border: 1px solid var(--hover); border-radius: 8px; padding: 9px 12px; font: inherit; }
input[type=search]:focus { outline: 2px solid var(--accent); border-color: transparent; }
a.button { color: var(--dim); text-decoration: none; padding: 9px 12px; border-radius: 8px; white-space: nowrap; }
a.button:hover { background: var(--hover); color: var(--text); }
ol { list-style: none; margin: 0; padding: 0; }
li { display: flex; align-items: center; gap: 12px; padding: 6px 8px; border-radius: 8px; }
li:hover { background: var(--panel); }
time { color: var(--dim); font-variant-numeric: tabular-nums; width: 64px; flex: none; }
.page { flex: 1; min-width: 0; text-decoration: none; color: inherit; }
.page span { display: block; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.page .url { color: var(--dim); font-size: 12px; }
.delete { visibility: hidden; color: var(--dim); text-decoration: none; padding: 2px 8px; border-radius: 6px; }
li:hover .delete { visibility: visible; }
.delete:hover { background: var(--hover); color: var(--text); }
.empty { color: var(--dim); padding: 24px 8px; }
"#;

/// Groups rows under day headings and formats times in the viewer's time zone.
const SCRIPT: &str = r#"
const today = new Date().toDateString();
const yesterday = new Date(Date.now() - 864e5).toDateString();
let lastDay = null;
for (const li of document.querySelectorAll('li[data-time]')) {
  const date = new Date(Number(li.dataset.time) * 1000);
  li.querySelector('time').textContent = date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const day = date.toDateString();
  if (day !== lastDay) {
    const h = document.createElement('h2');
    h.textContent = day === today ? 'Today' : day === yesterday ? 'Yesterday'
      : date.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' });
    li.closest('ol').before(h);
    const list = document.createElement('ol');
    h.after(list);
    lastDay = day;
  }
  document.querySelector('h2:last-of-type + ol').append(li);
}
document.querySelectorAll('ol:empty').forEach(ol => ol.remove());
document.querySelector('#clear')?.addEventListener('click', event => {
  if (!confirm('Clear all browsing history?')) event.preventDefault();
});
"#;

fn history_page(query: &str, visits: &[Visit]) -> String {
    let mut rows = String::new();
    for visit in visits {
        let title = if visit.title.is_empty() {
            &visit.url
        } else {
            &visit.title
        };
        let mut delete = format!("kite://history/delete?url={}", encode_component(&visit.url));
        if !query.is_empty() {
            delete.push_str(&format!("&q={}", encode_component(query)));
        }
        rows.push_str(&format!(
            "<li data-time=\"{time}\"><time></time>\
             <a class=\"page\" href=\"{url}\"><span>{title}</span><span class=\"url\">{url}</span></a>\
             <a class=\"delete\" href=\"{delete}\" title=\"Remove from history\">✕</a></li>\n",
            time = visit.last_visit,
            url = escape(&visit.url),
            title = escape(title),
            delete = escape(&delete),
        ));
    }
    let list = if visits.is_empty() {
        let message = if query.is_empty() {
            "Pages you visit will show up here.".to_owned()
        } else {
            format!("Nothing in history matches “{}”.", escape(query))
        };
        format!("<p class=\"empty\">{message}</p>")
    } else {
        format!("<ol>\n{rows}</ol>")
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>History</title><style>{STYLE}</style></head><body><main>\
         <h1>History</h1>\
         <form action=\"kite://history\"><input type=\"search\" name=\"q\" value=\"{query}\" \
         placeholder=\"Search history\" autofocus>\
         <a class=\"button\" id=\"clear\" href=\"kite://history/clear\">Clear history</a></form>\
         {list}</main><script>{SCRIPT}</script></body></html>",
        query = escape(query),
    )
}

fn error_page(message: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Kite</title>\
         <style>{STYLE}</style></head><body><main><h1>Kite</h1><p class=\"empty\">{}</p></main></body></html>",
        escape(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(visits: &[(&str, &str)]) -> Store {
        let store = Store::in_memory();
        for (time, (url, title)) in visits.iter().enumerate() {
            store.record_visit(url, time as i64).unwrap();
            store.set_title(url, title).unwrap();
        }
        store
    }

    #[test]
    fn history_page_lists_and_escapes_entries() {
        let store = store_with(&[("https://a.example/?x=1&y=2", "<script>alert(1)</script>")]);
        let page = respond("kite://history", &store);
        assert_eq!((page.status, page.redirect.as_deref()), (200, None));
        assert!(page.body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!page.body.contains("<script>alert(1)"));
        assert!(
            page.body
                .contains("href=\"https://a.example/?x=1&amp;y=2\"")
        );
    }

    #[test]
    fn history_page_searches_and_echoes_the_query_safely() {
        let store = store_with(&[
            ("https://a.example/", "Alpha"),
            ("https://b.example/", "Beta"),
        ]);
        let page = respond("kite://history?q=alpha", &store);
        assert!(page.body.contains("Alpha") && !page.body.contains("Beta"));
        let page = respond("kite://history/?q=%22%3E%3Cb%3E", &store);
        assert!(page.body.contains("value=\"&quot;&gt;&lt;b&gt;\""));
        assert!(page.body.contains("Nothing in history matches"));
    }

    #[test]
    fn delete_removes_one_entry_and_returns_to_the_search() {
        let store = store_with(&[("https://a.example/?k=v", "A"), ("https://b.example/", "B")]);
        let page = respond(
            "kite://history/delete?url=https%3A%2F%2Fa.example%2F%3Fk%3Dv&q=a+b",
            &store,
        );
        assert_eq!(page.redirect.as_deref(), Some("kite://history?q=a+b"));
        let left: Vec<String> = store
            .search_history("", 10)
            .unwrap()
            .into_iter()
            .map(|v| v.url)
            .collect();
        assert_eq!(left, vec!["https://b.example/"]);
    }

    #[test]
    fn clear_empties_history() {
        let store = store_with(&[("https://a.example/", "A")]);
        assert_eq!(
            respond("kite://history/clear", &store).redirect.as_deref(),
            Some("kite://history")
        );
        assert!(store.search_history("", 10).unwrap().is_empty());
    }

    #[test]
    fn unknown_pages_are_404s() {
        let store = Store::in_memory();
        assert_eq!(respond("kite://nope", &store).status, 404);
        assert_eq!(respond("kite://history/other", &store).status, 404);
    }
}
