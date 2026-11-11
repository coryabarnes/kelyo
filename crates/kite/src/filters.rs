//! Keeps the ad blocker loaded and its filter lists fresh.
//!
//! At startup a background thread loads the compiled blocker cached on disk
//! (fast), then, if the lists are older than [`MAX_AGE`], downloads them,
//! recompiles and swaps the new blocker in. Until the first blocker is ready
//! nothing is blocked. If a download fails, that list's cached copy is used.
//!
//! The blocker is read on CEF's IO thread for every request, so it lives in
//! a global behind an `RwLock`, along with the sites the user turned
//! blocking off for and a count of blocked requests per browser.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use kite_core::Blocker;
use kite_core::blocker::{FILTER_LISTS, RESOURCES_URL};

use crate::command::BrowserId;

/// How old the filter lists may get before they're downloaded again.
const MAX_AGE: Duration = Duration::from_secs(4 * 24 * 60 * 60);
const DOWNLOAD_LIMIT: u64 = 32 * 1024 * 1024;

static BLOCKER: RwLock<Option<Arc<Blocker>>> = RwLock::new(None);
static EXCEPTIONS: RwLock<Option<HashSet<String>>> = RwLock::new(None);
static BLOCKED: RwLock<Option<HashMap<BrowserId, u32>>> = RwLock::new(None);

/// The current blocker, once one has loaded.
pub fn blocker() -> Option<Arc<Blocker>> {
    BLOCKER.read().ok()?.clone()
}

/// Whether the user turned blocking off for `site` (see `blocker::site_of`).
pub fn is_excepted(site: &str) -> bool {
    EXCEPTIONS
        .read()
        .is_ok_and(|sites| sites.as_ref().is_some_and(|sites| sites.contains(site)))
}

pub fn set_exceptions(sites: HashSet<String>) {
    if let Ok(mut slot) = EXCEPTIONS.write() {
        *slot = Some(sites);
    }
}

/// Requests blocked since the browser's current page started loading.
pub fn blocked_count(browser: BrowserId) -> u32 {
    BLOCKED
        .read()
        .ok()
        .and_then(|counts| counts.as_ref()?.get(&browser).copied())
        .unwrap_or(0)
}

pub fn count_blocked(browser: BrowserId) {
    if let Ok(mut counts) = BLOCKED.write() {
        *counts
            .get_or_insert_with(HashMap::new)
            .entry(browser)
            .or_insert(0) += 1;
    }
}

/// Starts counting afresh for a new page, or forgets a closed browser.
pub fn reset_count(browser: BrowserId) {
    if let Ok(mut counts) = BLOCKED.write()
        && let Some(counts) = counts.as_mut()
    {
        counts.remove(&browser);
    }
}

/// Loads (and if needed refreshes) the blocker on a background thread.
/// `dir` holds the cached lists; without one, lists are downloaded every start.
pub fn start(dir: Option<PathBuf>) {
    let spawned = std::thread::Builder::new()
        .name("kite-filters".into())
        .spawn(move || {
            let dir = dir.unwrap_or_else(|| std::env::temp_dir().join("kite-filters"));
            if let Err(error) = std::fs::create_dir_all(&dir) {
                eprintln!("kite: can't create {}: {error}", dir.display());
            }
            load_and_refresh(&dir);
        });
    if let Err(error) = spawned {
        eprintln!("kite: couldn't start the filter thread; ads won't be blocked: {error}");
    }
}

fn load_and_refresh(dir: &Path) {
    let engine_path = dir.join("engine.dat");
    let resources_path = dir.join("resources.json");

    let cached = std::fs::read(&engine_path)
        .ok()
        .and_then(|data| Blocker::deserialize(&data));
    let have_cache = cached.is_some();
    if let Some(blocker) = cached {
        install(blocker, &resources_path);
    }

    let fresh = modified_age(&engine_path).is_some_and(|age| age < MAX_AGE);
    if have_cache && fresh {
        return;
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into();
    let mut lists = Vec::new();
    for list in FILTER_LISTS {
        let path = dir.join(format!("{}.txt", list.name));
        match download(&agent, list.url) {
            Ok(text) => {
                let _ = std::fs::write(&path, &text);
                lists.push(text);
            }
            Err(error) => {
                eprintln!("kite: couldn't update filter list {}: {error}", list.name);
                if let Ok(text) = std::fs::read_to_string(&path) {
                    lists.push(text);
                }
            }
        }
    }
    match download(&agent, RESOURCES_URL) {
        Ok(json) => {
            let _ = std::fs::write(&resources_path, json);
        }
        Err(error) => eprintln!("kite: couldn't update scriptlet resources: {error}"),
    }
    if lists.is_empty() {
        eprintln!("kite: no filter lists available; ads won't be blocked");
        return;
    }

    let blocker = Blocker::from_lists(lists.iter().map(String::as_str));
    if let Err(error) = std::fs::write(&engine_path, blocker.serialize()) {
        eprintln!("kite: couldn't cache the compiled filters: {error}");
    }
    install(blocker, &resources_path);
}

fn install(mut blocker: Blocker, resources_path: &Path) {
    if let Ok(json) = std::fs::read_to_string(resources_path)
        && let Err(error) = blocker.use_resources_json(&json)
    {
        eprintln!("kite: ignoring unreadable scriptlet resources: {error}");
    }
    if let Ok(mut slot) = BLOCKER.write() {
        *slot = Some(Arc::new(blocker));
    }
}

fn download(agent: &ureq::Agent, url: &str) -> Result<String, ureq::Error> {
    agent
        .get(url)
        .call()?
        .body_mut()
        .with_config()
        .limit(DOWNLOAD_LIMIT)
        .read_to_string()
}

fn modified_age(path: &Path) -> Option<Duration> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}
