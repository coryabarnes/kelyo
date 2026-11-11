# Kite

A fast, sidebar-first web browser for Linux: Chromium's engine, with the whole
browser shell written in Rust.

Kite embeds Chromium through the [Chromium Embedded Framework](https://github.com/chromiumembedded/cef)
using the [`cef`](https://crates.io/crates/cef) crate, so pages load and render
exactly as fast as in Chrome. Everything around the page (the window, sidebar,
tabs, address bar, split view and shortcuts) is Rust.

## What it does

- Vertical tab sidebar: click to switch, `×` to close, `+ New Tab`
- Address bar that tells URLs from searches (`example.com`, `localhost:3000`, `fedora tips`),
  and completes what you type from history (`gith` → `github.com`)
- Back, forward, reload/stop
- Split view: up to three tabs side by side or stacked. `◫` on a tab row adds it
  to (or takes it out of) the split; click into a pane to make it the active tab
- Ad and tracker blocking with uBlock Origin's filter lists (plus EasyList,
  EasyPrivacy, Peter Lowe's list and the EasyList cookie-banner list), matched
  by Brave's [`adblock`](https://crates.io/crates/adblock) engine. Lists update
  every four days. The `🛡` button shows how many requests were blocked on the
  page; click it to turn blocking off (or back on) for that site
- Clean links: tracking parameters (`utm_*`, `fbclid`, `gclid`, YouTube's `si`, ...)
  are stripped from every page you open, so the address bar, history and copied
  links never carry them
- History at `kite://history`: search it, remove single pages, or clear it all
- Your tabs and split view come back after a restart, or a crash. Restored tabs
  load when you first open them, so a big session starts fast
- Reopen closed tabs, most recent first
- DevTools in their own window
- Links that open new tabs or windows land in the sidebar; `window.open()` popups (sign-in flows) get their own window
- A persistent profile in `~/.local/share/kite`, so logins and cookies survive restarts
- Chromium's sandbox enabled by default

### Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab (closing the last tab closes the window) |
| `Ctrl+Shift+T` | Reopen the last closed tab |
| `Ctrl+L` | Focus address bar |
| `Ctrl+Shift+C` | Copy the page's link, without tracking parameters |
| `Ctrl+H` | History |
| `Ctrl+Tab` / `Ctrl+PgDn` | Next tab |
| `Ctrl+Shift+Tab` / `Ctrl+PgUp` | Previous tab |
| `Ctrl+Shift+S` | Toggle split view |
| `Ctrl+Shift+O` | Split view side by side / stacked |
| `Ctrl+R` / `F5` | Reload |
| `Alt+Left` / `Alt+Right` | Back / forward |
| `F12` / `Ctrl+Shift+I` | DevTools |
| `Esc` in address bar | Restore the current URL |
| `Backspace` in address bar | Drop the completion |

## Building on Fedora

Install Rust and the libraries Chromium needs at runtime:

```bash
sudo dnf install rustup gcc nss atk at-spi2-atk cups-libs libdrm libxkbcommon libXcomposite libXdamage libXrandr mesa-libgbm pango alsa-lib
```

```bash
rustup-init -y
```

Download CEF once into a shared location. This saves re-downloading it on every
clean build. The `export-cef-dir` tool comes from the `cef-rs` repository:

```bash
cargo install --git https://github.com/tauri-apps/cef-rs export-cef-dir
```

```bash
export-cef-dir --force $HOME/.local/share/cef
```

Then point the build at it. Add these lines to `~/.bashrc` to keep them:

```bash
export CEF_PATH=$HOME/.local/share/cef
export LD_LIBRARY_PATH=$CEF_PATH:$LD_LIBRARY_PATH
```

If you skip the shared download, the first build downloads CEF (a few hundred MB)
into `target/` by itself.

Run it:

```bash
cargo run --release
```

Make a self-contained folder you can copy anywhere (`target/bundle`):

```bash
cargo xtask bundle --release
```

### Wayland

Chromium runs natively on Wayland when the session is Wayland, and on X11
otherwise. To force X11 (XWayland), run:

```bash
cargo run --release -- --ozone-platform=x11
```

### Snap

`packaging/snap/pack.sh` bundles Kite and packs it into a classic snap in
`target/`, which you can install locally:

```bash
sudo snap install --dangerous --classic target/kite_*.snap
```

## Tests

The tab model, address bar logic, history, saved sessions, ad blocking rules,
link cleaning and `kite://` pages live in `kite-core`, which has no CEF
dependency. Its tests run anywhere:

```bash
cargo test -p kite-core
```

## Code layout

```text
crates/kite-core   Engine-independent logic, fully unit-tested
  tabs.rs          Tab order, active tab, split view, recently closed tabs
  omnibox.rs       URL vs. search detection
  store.rs         The profile database (SQLite): schema and site settings
  history.rs       History, and address bar completion
  session.rs       Saving and restoring open tabs
  blocker.rs       Ad blocking: filter lists and request matching
  clean_url.rs     Tracking parameter removal
  pages.rs         kite:// pages (history)
  settings.rs      New tab page, search engine
  paths.rs         Profile location (XDG)
crates/kite        The browser
  main.rs          CEF process entry point
  app.rs           CEF settings; starts the shell
  command.rs       Every action and engine event, queued onto the UI thread
  shell.rs         Owns all state; applies commands
  ui.rs            Sidebar and content layout (CEF Views)
  views.rs         Window, tab, button and address bar delegates
  client.rs        Engine callbacks: title, URL, loading, popups, focus,
                   and request blocking (on CEF's IO thread)
  filters.rs       Downloads and caches filter lists; holds the blocker
  scheme.rs        Serves kite:// pages
  shortcuts.rs     Keyboard shortcuts
  theme.rs         Colours and sizes
xtask              `cargo xtask bundle`
packaging/snap     Snap packaging
```

One rule keeps the shell simple: **nothing changes state except a `Command`**.
Delegates and engine callbacks only call `dispatch(command)`. That posts the
command to CEF's UI thread, where `Shell::handle` applies it to the `TabStrip`
model and then updates the views. Callbacks never re-enter the shell mid-update,
and the logic that matters is testable without a browser.

Two exceptions answer the engine on the spot: request blocking runs on CEF's
IO thread against the shared blocker in `filters.rs`, and `kite://` pages are
served from their own database connection.

### CEF runtime styles

Tabs are Alloy style browser views inside Chrome style windows. A Chrome style
window holds only one Chrome style browser view, so tabs and split view need
Alloy style views. The windows stay Chrome style because DevTools crashes when
its parent window has no Chrome profile.

## Known gaps

- **Video ads on YouTube.** Network and element-hiding filters work, but
  uBlock Origin's scriptlets (the `+js(...)` filters that defeat YouTube's
  video ads) don't run yet: uBlock ships them as JavaScript modules that need
  converting into `adblock` resources first.
- **Generic element hiding.** Only site-specific hiding rules apply; rules that
  hide elements on every site by class or id are skipped.
- **No drag and drop.** CEF Views widgets get no mouse or drag events, so tabs
  can't be dragged to reorder or into split view. That needs the sidebar drawn
  as a web page.
- No Chrome Web Store extensions. CEF supports only part of the extension APIs.
- No bookmarks, downloads UI, or settings page yet.
- The address bar uses the system text field colours rather than the dark theme.

## Status

`cargo test -p kite-core` passes (58 tests), and the workspace passes
`cargo clippy --all-targets -- -D warnings`.

The browser runs on Fedora 44 (KDE Plasma, Wayland and X11). Tested by hand:
tabs, closing and reopening tabs, split view with two and three panes, both
orientations, address bar completion, history, session restore (including
after a crash), ad blocking and the per-site toggle, clean links, DevTools,
and `window.open()` popups.
