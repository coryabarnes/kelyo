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

/// Two tabs shown side by side. `left` comes before `right` in the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Split {
    pub left: TabId,
    pub right: TabId,
}

impl Split {
    pub fn contains(&self, id: TabId) -> bool {
        self.left == id || self.right == id
    }
}

/// Ordered tabs plus selection state.
///
/// Invariants: `active` is `None` only when there are no tabs, and `split`,
/// when set, names two distinct existing tabs, one of which is active.
#[derive(Debug, Default)]
pub struct TabStrip {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    split: Option<Split>,
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

    pub fn split(&self) -> Option<Split> {
        self.split
    }

    /// The tabs currently on screen, in strip order.
    pub fn visible(&self) -> Vec<TabId> {
        match (self.split, self.active) {
            (Some(split), _) => vec![split.left, split.right],
            (None, Some(active)) => vec![active],
            (None, None) => Vec::new(),
        }
    }

    /// Opens a tab right after the active one.
    pub fn open(&mut self, url: impl Into<String>, placement: Placement) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;

        let index = self
            .active
            .and_then(|active| self.index_of(active))
            .map_or(self.tabs.len(), |i| i + 1);
        self.tabs.insert(
            index,
            Tab {
                id,
                title: String::new(),
                url: url.into(),
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
        if self.split.is_some_and(|split| !split.contains(id)) {
            self.split = None;
        }
        self.active = Some(id);
        true
    }

    /// Removes a tab. If it was active, its right neighbour (or else its left) becomes active.
    pub fn close(&mut self, id: TabId) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };
        self.tabs.remove(index);

        if let Some(split) = self.split.filter(|split| split.contains(id)) {
            self.split = None;
            if self.active == Some(id) {
                // Stay on the other half of the split.
                let partner = if split.left == id {
                    split.right
                } else {
                    split.left
                };
                self.active = Some(partner);
            }
        }
        if self.active == Some(id) {
            let neighbour = index.min(self.tabs.len().saturating_sub(1));
            self.active = self.tabs.get(neighbour).map(|tab| tab.id);
        }
        true
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
            left: self.tabs[left].id,
            right: self.tabs[right].id,
        });
        true
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
