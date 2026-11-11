//! The tab model: which tabs exist, their order, which one is active, and split view.

/// Stable identifier for a tab. Never reused within a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

impl Tab {
    /// Text to show for the tab: its title, else its URL, else "New Tab".
    pub fn label(&self) -> &str {
        [self.title.as_str(), self.url.as_str()]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or("New Tab")
    }
}

/// Where a newly opened tab goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Open next to the active tab and switch to it.
    Foreground,
    /// Open next to the active tab but stay on the current one.
    Background,
}

/// The most panes split view shows at once.
pub const MAX_PANES: usize = 3;

/// How many closed tabs [`TabStrip::reopen_closed`] remembers.
const CLOSED_HISTORY: usize = 25;

/// Two or three tabs shown together, in pane order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    panes: Vec<TabId>,
    /// Stacked top to bottom instead of side by side.
    pub vertical: bool,
}

impl Split {
    pub fn contains(&self, id: TabId) -> bool {
        self.panes.contains(&id)
    }

    pub fn panes(&self) -> &[TabId] {
        &self.panes
    }
}

/// A closed tab, kept so it can be reopened.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ClosedTab {
    url: String,
    title: String,
    index: usize,
}

/// Ordered tabs plus selection state.
///
/// Invariants: `active` is `None` only when there are no tabs, and `split`,
/// when set, names 2..=[`MAX_PANES`] distinct existing tabs, one of which is active.
#[derive(Debug, Default)]
pub struct TabStrip {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    split: Option<Split>,
    closed: Vec<ClosedTab>,
    next_id: u64,
}

impl TabStrip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter()
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    pub fn active(&self) -> Option<TabId> {
        self.active
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.get(id))
    }

    pub fn split(&self) -> Option<&Split> {
        self.split.as_ref()
    }

    /// The tabs currently on screen, in pane order.
    pub fn visible(&self) -> Vec<TabId> {
        match (&self.split, self.active) {
            (Some(split), _) => split.panes.clone(),
            (None, Some(active)) => vec![active],
            (None, None) => Vec::new(),
        }
    }

    /// Opens a tab right after the active one.
    pub fn open(&mut self, url: impl Into<String>, placement: Placement) -> TabId {
        let index = self
            .active
            .and_then(|active| self.index_of(active))
            .map_or(self.tabs.len(), |i| i + 1);
        self.open_at(index, url.into(), String::new(), placement)
    }

    fn open_at(&mut self, index: usize, url: String, title: String, placement: Placement) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        self.tabs.insert(
            index.min(self.tabs.len()),
            Tab {
                id,
                title,
                url,
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
        );
        if placement == Placement::Foreground || self.active.is_none() {
            self.activate(id);
        }
        id
    }

    /// Makes `id` the active tab. Activating a tab outside the split ends split view.
    pub fn activate(&mut self, id: TabId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        if self.split.as_ref().is_some_and(|split| !split.contains(id)) {
            self.split = None;
        }
        self.active = Some(id);
        true
    }

    /// Removes a tab. If it was active, another pane of the split becomes
    /// active, or else its right neighbour (or else its left).
    pub fn close(&mut self, id: TabId) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };
        let tab = self.tabs.remove(index);
        self.closed.push(ClosedTab {
            url: tab.url,
            title: tab.title,
            index,
        });
        if self.closed.len() > CLOSED_HISTORY {
            self.closed.remove(0);
        }

        if self.split.as_ref().is_some_and(|split| split.contains(id)) {
            let remaining = self.remove_from_split(id);
            if self.active == Some(id) {
                // Stay in what's left of the split.
                self.active = remaining;
            }
        }
        if self.active == Some(id) {
            let neighbour = index.min(self.tabs.len().saturating_sub(1));
            self.active = self.tabs.get(neighbour).map(|tab| tab.id);
        }
        true
    }

    /// Reopens the most recently closed tab where it was, and activates it.
    pub fn reopen_closed(&mut self) -> Option<TabId> {
        let ClosedTab { url, title, index } = self.closed.pop()?;
        Some(self.open_at(index, url, title, Placement::Foreground))
    }

    /// Activates the next (or previous) tab, wrapping around.
    pub fn cycle(&mut self, forward: bool) {
        let Some(index) = self.active.and_then(|id| self.index_of(id)) else {
            return;
        };
        let len = self.tabs.len();
        let next = if forward {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        };
        self.activate(self.tabs[next].id);
    }

    /// Toggles split view, pairing the active tab with its right neighbour
    /// (or its left one, if it is the last tab). Returns whether split view is now on.
    pub fn toggle_split(&mut self) -> bool {
        if self.split.take().is_some() {
            return false;
        }
        let Some(index) = self.active.and_then(|id| self.index_of(id)) else {
            return false;
        };
        let partner = if index + 1 < self.tabs.len() {
            index + 1
        } else if index > 0 {
            index - 1
        } else {
            return false;
        };
        let (left, right) = (index.min(partner), index.max(partner));
        self.split = Some(Split {
            panes: vec![self.tabs[left].id, self.tabs[right].id],
            vertical: false,
        });
        true
    }

    /// Adds `id` to split view as the last pane, starting split view with the
    /// active tab if it's off. If `id` is already a pane, takes it out instead.
    /// Returns whether anything changed.
    pub fn split_with(&mut self, id: TabId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        let Some(active) = self.active else {
            return false;
        };
        match &mut self.split {
            Some(split) if split.contains(id) => {
                let remaining = self.remove_from_split(id);
                if self.active == Some(id) {
                    self.active = remaining;
                }
                true
            }
            Some(split) if split.panes.len() < MAX_PANES => {
                split.panes.push(id);
                true
            }
            Some(_) => false,
            None if id == active => false,
            None => {
                self.split = Some(Split {
                    panes: vec![active, id],
                    vertical: false,
                });
                true
            }
        }
    }

    /// Switches split view between side by side and stacked.
    pub fn toggle_split_orientation(&mut self) {
        if let Some(split) = &mut self.split {
            split.vertical = !split.vertical;
        }
    }

    /// Restores a saved split. Ignored unless it names 2..=[`MAX_PANES`]
    /// distinct tabs including the active one.
    pub fn set_split(&mut self, panes: Vec<TabId>, vertical: bool) {
        let mut distinct = panes.clone();
        distinct.sort();
        distinct.dedup();
        let valid = (2..=MAX_PANES).contains(&panes.len())
            && distinct.len() == panes.len()
            && panes.iter().all(|id| self.index_of(*id).is_some())
            && self.active.is_some_and(|active| panes.contains(&active));
        if valid {
            self.split = Some(Split { panes, vertical });
        }
    }

    /// Takes `id` out of the split, ending split view if fewer than two panes
    /// remain. Returns the first remaining pane.
    fn remove_from_split(&mut self, id: TabId) -> Option<TabId> {
        let split = self.split.as_mut()?;
        split.panes.retain(|pane| *pane != id);
        let first = split.panes.first().copied();
        if split.panes.len() < 2 {
            self.split = None;
        }
        first
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(strip: &TabStrip) -> Vec<TabId> {
        strip.iter().map(|tab| tab.id).collect()
    }

    fn strip_with(n: usize) -> (TabStrip, Vec<TabId>) {
        let mut strip = TabStrip::new();
        for i in 0..n {
            let id = strip.open(format!("https://{i}.example"), Placement::Background);
            // Keep appending at the end: activate the newest tab each time.
            strip.activate(id);
        }
        let tabs = ids(&strip);
        (strip, tabs)
    }

    #[test]
    fn first_tab_becomes_active_even_in_background() {
        let mut strip = TabStrip::new();
        let id = strip.open("a", Placement::Background);
        assert_eq!(strip.active(), Some(id));
        assert_eq!(strip.visible(), vec![id]);
    }

    #[test]
    fn opens_next_to_active_tab() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        let new = strip.open("x", Placement::Foreground);
        assert_eq!(ids(&strip), vec![tabs[0], new, tabs[1], tabs[2]]);
        assert_eq!(strip.active(), Some(new));
    }

    #[test]
    fn background_open_keeps_active_tab() {
        let (mut strip, tabs) = strip_with(2);
        strip.open("x", Placement::Background);
        assert_eq!(strip.active(), Some(tabs[1]));
    }

    #[test]
    fn closing_active_tab_selects_right_then_left_neighbour() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[1]);
        assert!(strip.close(tabs[1]));
        assert_eq!(strip.active(), Some(tabs[2]));
        assert!(strip.close(tabs[2]));
        assert_eq!(strip.active(), Some(tabs[0]));
        assert!(strip.close(tabs[0]));
        assert_eq!(strip.active(), None);
        assert!(strip.visible().is_empty());
    }

    #[test]
    fn closing_inactive_tab_keeps_selection() {
        let (mut strip, tabs) = strip_with(3);
        strip.close(tabs[0]);
        assert_eq!(strip.active(), Some(tabs[2]));
    }

    #[test]
    fn unknown_ids_are_rejected() {
        let (mut strip, tabs) = strip_with(1);
        strip.close(tabs[0]);
        assert!(!strip.close(tabs[0]));
        assert!(!strip.activate(tabs[0]));
    }

    #[test]
    fn cycle_wraps_in_both_directions() {
        let (mut strip, tabs) = strip_with(3);
        strip.cycle(true);
        assert_eq!(strip.active(), Some(tabs[0]));
        strip.cycle(false);
        assert_eq!(strip.active(), Some(tabs[2]));
    }

    #[test]
    fn split_pairs_with_right_neighbour_or_last_with_left() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        assert!(strip.toggle_split());
        assert_eq!(strip.visible(), vec![tabs[0], tabs[1]]);

        assert!(!strip.toggle_split());
        strip.activate(tabs[2]);
        assert!(strip.toggle_split());
        assert_eq!(strip.visible(), vec![tabs[1], tabs[2]]);
    }

    #[test]
    fn split_needs_two_tabs() {
        let (mut strip, _) = strip_with(1);
        assert!(!strip.toggle_split());
        assert_eq!(strip.split(), None);
    }

    #[test]
    fn activating_within_split_keeps_it_and_outside_ends_it() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        strip.toggle_split();
        strip.activate(tabs[1]);
        assert!(strip.split().is_some());
        strip.activate(tabs[2]);
        assert_eq!(strip.split(), None);
        assert_eq!(strip.visible(), vec![tabs[2]]);
    }

    #[test]
    fn split_with_adds_up_to_three_panes_and_toggles_membership() {
        let (mut strip, tabs) = strip_with(4);
        strip.activate(tabs[0]);
        assert!(strip.split_with(tabs[2]));
        assert_eq!(strip.visible(), vec![tabs[0], tabs[2]]);
        assert!(strip.split_with(tabs[3]));
        assert_eq!(strip.visible(), vec![tabs[0], tabs[2], tabs[3]]);
        assert!(!strip.split_with(tabs[1]), "a fourth pane is refused");

        assert!(strip.split_with(tabs[2]));
        assert_eq!(strip.visible(), vec![tabs[0], tabs[3]]);
        assert!(strip.split_with(tabs[3]));
        assert_eq!(strip.split(), None, "one pane left ends split view");
        assert_eq!(strip.active(), Some(tabs[0]));
    }

    #[test]
    fn split_with_the_active_tab_alone_does_nothing() {
        let (mut strip, tabs) = strip_with(2);
        assert!(!strip.split_with(tabs[1]));
        assert_eq!(strip.split(), None);
    }

    #[test]
    fn removing_the_active_pane_activates_another_pane() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        strip.split_with(tabs[1]);
        strip.split_with(tabs[2]);
        strip.split_with(tabs[0]);
        assert_eq!(strip.active(), Some(tabs[1]));
        assert_eq!(strip.visible(), vec![tabs[1], tabs[2]]);
    }

    #[test]
    fn orientation_toggles_only_while_split() {
        let (mut strip, tabs) = strip_with(2);
        strip.toggle_split_orientation();
        assert_eq!(strip.split(), None);
        strip.split_with(tabs[0]);
        strip.toggle_split_orientation();
        assert!(strip.split().unwrap().vertical);
    }

    #[test]
    fn set_split_rejects_invalid_layouts() {
        let (mut strip, tabs) = strip_with(4);
        strip.set_split(vec![tabs[0], tabs[0]], false);
        assert_eq!(strip.split(), None);
        strip.set_split(vec![tabs[0], tabs[1]], false);
        assert_eq!(strip.split(), None, "must include the active tab");
        strip.set_split(tabs.clone(), false);
        assert_eq!(strip.split(), None, "too many panes");
        strip.set_split(vec![tabs[3], tabs[1]], true);
        assert_eq!(strip.visible(), vec![tabs[3], tabs[1]]);
    }

    #[test]
    fn closing_a_pane_of_three_keeps_the_split() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        strip.split_with(tabs[1]);
        strip.split_with(tabs[2]);
        strip.close(tabs[0]);
        assert_eq!(strip.visible(), vec![tabs[1], tabs[2]]);
        assert_eq!(strip.active(), Some(tabs[1]));
    }

    #[test]
    fn reopens_closed_tabs_in_place_most_recent_first() {
        let (mut strip, tabs) = strip_with(3);
        strip.get_mut(tabs[1]).unwrap().title = "B".into();
        strip.close(tabs[1]);
        strip.close(tabs[2]);
        let c = strip.reopen_closed().unwrap();
        assert_eq!(strip.get(c).unwrap().url, "https://2.example");
        let b = strip.reopen_closed().unwrap();
        assert_eq!(strip.get(b).unwrap().title, "B");
        assert_eq!(ids(&strip), vec![tabs[0], b, c]);
        assert_eq!(strip.active(), Some(b));
        assert_eq!(strip.reopen_closed(), None);
    }

    #[test]
    fn closing_half_of_split_stays_on_other_half() {
        let (mut strip, tabs) = strip_with(3);
        strip.activate(tabs[0]);
        strip.toggle_split();
        strip.close(tabs[0]);
        assert_eq!(strip.split(), None);
        assert_eq!(strip.active(), Some(tabs[1]));
    }

    #[test]
    fn label_falls_back_from_title_to_url() {
        let mut strip = TabStrip::new();
        let id = strip.open("", Placement::Foreground);
        assert_eq!(strip.get(id).unwrap().label(), "New Tab");
        strip.get_mut(id).unwrap().url = "https://a.example".into();
        assert_eq!(strip.get(id).unwrap().label(), "https://a.example");
        strip.get_mut(id).unwrap().title = "A".into();
        assert_eq!(strip.get(id).unwrap().label(), "A");
    }
}
