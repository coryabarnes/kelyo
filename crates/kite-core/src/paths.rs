//! Where Kite keeps its data on disk.

use std::path::PathBuf;

const APP_DIR: &str = "kite";

/// The directory for persistent browser data (profile, cookies, cache).
///
/// Follows the XDG base directory spec on Linux: `$XDG_DATA_HOME/kite`,
/// falling back to `~/.local/share/kite`. Returns `None` if no home
/// directory can be found, in which case the browser runs without a
/// persistent profile.
pub fn data_dir() -> Option<PathBuf> {
    data_dir_from(|key| std::env::var_os(key).filter(|value| !value.is_empty()))
}

fn data_dir_from(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    if let Some(xdg) = env("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg).join(APP_DIR));
    }
    if let Some(home) = env("HOME") {
        return Some(PathBuf::from(home).join(".local/share").join(APP_DIR));
    }
    // Lets developers run the shell on Windows too.
    env("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join(APP_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn env_of<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |key| {
            vars.iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn prefers_xdg_data_home() {
        let env = env_of(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/me")]);
        assert_eq!(data_dir_from(env), Some(PathBuf::from("/data/kite")));
    }

    #[test]
    fn falls_back_to_home() {
        let env = env_of(&[("HOME", "/home/me")]);
        assert_eq!(
            data_dir_from(env),
            Some(PathBuf::from("/home/me/.local/share/kite"))
        );
    }

    #[test]
    fn none_without_any_base_directory() {
        assert_eq!(data_dir_from(env_of(&[])), None);
    }
}
