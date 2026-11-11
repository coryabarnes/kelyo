//! The open tabs, saved as they change so the next start (or a restart after
//! a crash) brings them back.

use rusqlite::params;

use crate::store::Store;
use crate::tabs::{Placement, TabId, TabStrip};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTab {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    /// Index into `tabs`.
    pub active: usize,
    /// Indexes into `tabs`, in pane order; empty without split view.
    pub split: Vec<usize>,
    pub split_vertical: bool,
}

impl Session {
    pub fn from_strip(strip: &TabStrip) -> Self {
        let ids: Vec<TabId> = strip.iter().map(|tab| tab.id).collect();
        let index_of = |id: TabId| ids.iter().position(|tab| *tab == id);
        Self {
            tabs: strip
                .iter()
                .map(|tab| SessionTab {
                    url: tab.url.clone(),
                    title: tab.title.clone(),
                })
                .collect(),
            active: strip.active().and_then(index_of).unwrap_or(0),
            split: strip
                .split()
                .map(|split| {
                    split
                        .panes()
                        .iter()
                        .filter_map(|id| index_of(*id))
                        .collect()
                })
                .unwrap_or_default(),
            split_vertical: strip.split().is_some_and(|split| split.vertical),
        }
    }

    /// Builds a tab strip with these tabs, selection and split.
    pub fn to_strip(&self) -> TabStrip {
        let mut strip = TabStrip::new();
        let ids: Vec<TabId> = self
            .tabs
            .iter()
            .map(|tab| {
                let id = strip.open(tab.url.clone(), Placement::Background);
                strip.get_mut(id).expect("just opened").title = tab.title.clone();
                // `open` places tabs after the active one; keep them in order.
                strip.activate(id);
                id
            })
            .collect();
        if let Some(active) = ids.get(self.active).or(ids.first()) {
            strip.activate(*active);
        }
        let panes = self
            .split
            .iter()
            .filter_map(|i| ids.get(*i).copied())
            .collect();
        strip.set_split(panes, self.split_vertical);
        strip
    }
}

impl Store {
    pub fn save_session(&self, session: &Session) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM session_tabs", [])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO session_tabs (position, url, title, active, pane) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (position, tab) in session.tabs.iter().enumerate() {
                let pane = session
                    .split
                    .iter()
                    .position(|index| *index == position)
                    .map(|p| p as i64);
                insert.execute(params![
                    position as i64,
                    tab.url,
                    tab.title,
                    position == session.active,
                    pane
                ])?;
            }
        }
        tx.commit()?;
        self.set_state("split_vertical", session.split_vertical.into())
    }

    /// The last saved session, or `None` if it had no tabs.
    pub fn load_session(&self) -> rusqlite::Result<Option<Session>> {
        let mut statement = self
            .conn
            .prepare("SELECT url, title, active, pane FROM session_tabs ORDER BY position")?;
        let rows: Vec<(String, String, bool, Option<i64>)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        if rows.is_empty() {
            return Ok(None);
        }
        let mut panes: Vec<(i64, usize)> = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.3.map(|pane| (pane, index)))
            .collect();
        panes.sort_unstable();
        Ok(Some(Session {
            active: rows.iter().position(|row| row.2).unwrap_or(0),
            split: panes.into_iter().map(|(_, index)| index).collect(),
            split_vertical: self.state("split_vertical")?.unwrap_or(0) != 0,
            tabs: rows
                .into_iter()
                .map(|(url, title, _, _)| SessionTab { url, title })
                .collect(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(url: &str) -> SessionTab {
        SessionTab {
            url: url.into(),
            title: format!("Title of {url}"),
        }
    }

    #[test]
    fn saves_and_loads_tabs_selection_and_split() {
        let store = Store::in_memory();
        assert_eq!(store.load_session().unwrap(), None);
        let session = Session {
            tabs: vec![
                tab("https://a.example/"),
                tab("https://b.example/"),
                tab("https://c.example/"),
            ],
            active: 2,
            split: vec![2, 0],
            split_vertical: true,
        };
        store.save_session(&session).unwrap();
        assert_eq!(store.load_session().unwrap(), Some(session));

        store.save_session(&Session::default()).unwrap();
        assert_eq!(store.load_session().unwrap(), None);
    }

    #[test]
    fn round_trips_through_a_tab_strip() {
        let session = Session {
            tabs: vec![
                tab("https://a.example/"),
                tab("https://b.example/"),
                tab("https://c.example/"),
            ],
            active: 1,
            split: vec![1, 2],
            split_vertical: false,
        };
        let strip = session.to_strip();
        let urls: Vec<&str> = strip.iter().map(|tab| tab.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://a.example/",
                "https://b.example/",
                "https://c.example/"
            ]
        );
        assert_eq!(
            strip.active_tab().unwrap().title,
            "Title of https://b.example/"
        );
        assert_eq!(Session::from_strip(&strip), session);
    }

    #[test]
    fn invalid_saved_split_is_dropped() {
        let session = Session {
            tabs: vec![tab("https://a.example/"), tab("https://b.example/")],
            active: 0,
            split: vec![1, 7],
            split_vertical: false,
        };
        assert_eq!(session.to_strip().split(), None);
    }
}
