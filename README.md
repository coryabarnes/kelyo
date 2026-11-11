# Kite

A fast, sidebar-first web browser for Linux: Chromium's engine, with the whole
browser shell written in Rust.

Kite embeds Chromium through the [Chromium Embedded Framework](https://github.com/chromiumembedded/cef)
using the [`cef`](https://crates.io/crates/cef) crate, so pages load and render
exactly as fast as in Chrome. Everything around the page (the window, sidebar,
tabs, address bar, split view and shortcuts) is Rust.

## What the MVP does

- Vertical tab sidebar: click to switch, `×` to close, `+ New Tab`
- Address bar that tells URLs from searches (`example.com`, `localhost:3000`, `fedora tips`)
- Back, forward, reload/stop
- Split view: two tabs side by side
- Links that open new tabs or windows land in the sidebar; `window.open()` popups (sign-in flows) get their own window
- A persistent profile in `~/.local/share/kite`, so logins and cookies survive restarts
- Chromium's sandbox enabled by default

### Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab (closing the last tab closes the window) |
| `Ctrl+L` | Focus address bar |
| `Ctrl+Tab` / `Ctrl+PgDn` | Next tab |
| `Ctrl+Shift+Tab` / `Ctrl+PgUp` | Previous tab |
| `Ctrl+Shift+S` | Toggle split view |
| `Ctrl+R` / `F5` | Reload |
| `Alt+Left` / `Alt+Right` | Back / forward |
| `Esc` in address bar | Restore the current URL |

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

CEF uses X11 by default and runs under XWayland on Fedora Workstation. To run
natively on Wayland, try:

```bash
cargo run --release -- --ozone-platform=wayland
```

## Tests

The tab model, address bar logic and settings live in `kite-core`, which has no
CEF dependency. Its tests run anywhere:

```bash
cargo test -p kite-core
```

## Code layout

```text
crates/kite-core   Engine-independent logic, fully unit-tested
  tabs.rs          Tab order, active tab, split view
  omnibox.rs       URL vs. search detection
  settings.rs      New tab page, search engine
  paths.rs         Profile location (XDG)
crates/kite        The browser
  main.rs          CEF process entry point
  app.rs           CEF settings; starts the shell
  command.rs       Every action and engine event, queued onto the UI thread
  shell.rs         Owns all state; applies commands
  ui.rs            Sidebar and content layout (CEF Views)
  views.rs         Window, tab, button and address bar delegates
  client.rs        Engine callbacks: title, URL, loading, popups
  shortcuts.rs     Keyboard shortcuts
  theme.rs         Colours and sizes
xtask              `cargo xtask bundle`
```

One rule keeps the shell simple: **nothing changes state except a `Command`**.
Delegates and engine callbacks only call `dispatch(command)`. That posts the
command to CEF's UI thread, where `Shell::handle` applies it to the `TabStrip`
model and then updates the views. Callbacks never re-enter the shell mid-update,
and the logic that matters is testable without a browser.

## Known gaps

- Not yet run on real Fedora hardware. See "Status" below.
- No Chrome Web Store extensions. CEF supports only part of the extension APIs.
- No history, bookmarks, downloads UI, or settings page yet.
- Tabs aren't restored after a restart.
- The address bar uses the system text field colours rather than the dark theme.

## Status

`kite-core` compiles with Rust 1.99 and passes `cargo clippy -- -D warnings`.
Its 24 unit tests have not been run yet: they were written on a machine whose
security policy blocked running freshly built test binaries. Run
`cargo test -p kite-core` first.

The CEF shell is written against the `cef` 156.4 API, with every call checked
against the crate's generated Linux bindings, but it has not been compiled or
run yet. Expect a round of fixes on the first `cargo run` on Fedora. The things
most likely to need adjusting:

- **Closing a single tab.** This uses `close_browser` on that tab's browser.
  Check that this closes only the tab, not the whole window.
- **Tab list height.** The tab list is sized by hand for its scroll view.
  Check that long tab lists scroll.
