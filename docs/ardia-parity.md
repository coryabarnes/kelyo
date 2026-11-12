# Matching Ardia: command bar, bookmarks, settings, search engines

Notes on what Ardia (github.com/mantrakp04/ardia) does, so Kelyo can offer the
same features. Behaviour and layout only: none of Ardia's code, artwork or
sounds are reused. Ardia is a macOS app, so ⌘ maps to Ctrl and ⌥ to Alt on
Linux.

## Command bar
- Ctrl+L opens it. Ctrl+T opens a new tab page by default; the setting
  Tabs › "New tab opens: Command Bar" makes Ctrl+T open the bar instead (the
  tab is only created when you pick something; Esc changes nothing).
- Placement (Appearance › "Ctrl+L opens the command bar"): "At the address"
  (default; a dropdown growing from the address pill, min 350 wide, radius 12)
  or "In the centre" (floating card 652 wide, top at 22% of window height).
- Panel: radius 20, solid #2D2D2D dark / #FEFFFF light, hairline border, soft
  shadow (24 blur, 10 down). Field row 55 tall, 15 px text, placeholder
  "Search or enter address" ("Search privately or enter address" incognito).
- One flat list (no headers), about 8 rows: best match first; search row;
  calculator ("=" icon, "Copy"); 2–4 commands with shortcut hints; extensions;
  quick-create (New Google Doc, Sheet, Slides, …); up to 3 pages; search
  suggestions; more pages. Open tabs show "Switch to Tab". Rows 36 tall,
  radius 8; selected row is a soft white wash.
- Empty input lists open tabs (up to 8).
- Keys: ↑/↓ move (wrap); Tab enters a site-search scope or accepts the inline
  completion; Enter = here, Ctrl+Enter = new tab, Alt+Enter = split view,
  Shift+Enter = new window; Shift+Delete removes a history row; Esc undoes in
  layers (preview → scope → close).
- Scopes: type an engine or site name ("youtube", "github.com") then Tab or
  Space: a chip appears and the placeholder becomes "Search YouTube".
  DuckDuckGo-style !bangs also work.
- Commands include New Tab/Window/Incognito, Close Tab, Reopen Closed Tab,
  Duplicate, Pin/Unpin, Reload, Copy URL (and as Markdown), Find, Bookmark
  This Page, Print, Zoom, Developer Tools, Toggle sidebar, Toggle Bookmarks
  Bar, Downloads, History, Bookmarks, Settings…, Clear Browsing Data…, Import
  Browser Data, Keyboard Shortcuts, Dark/Light/System Appearance.

## Bookmarks
- A bookmarks bar under the toolbar, separate from the sidebar's pinned tiles.
  Show bookmarks bar: Always / On New Tab Only / Never (default Never);
  Ctrl+Shift+B toggles. Size: Default (30 tall, 12 px text, radius 4) or
  Large (38 tall, 13 px text, radius 8).
- Items: favicon + title (icon only if untitled); folders open cascading
  menus; "»" overflow; "Other Bookmarks"; drag to reorder; context menus to
  open/edit/delete, New Folder, Bookmarks Bar Size.
- Ctrl+D: "Bookmark Added" popover with name, folder picker, Remove / Done.

## Settings
- Its own window titled "Settings" (Ctrl+,). Left sidebar 214 wide with a
  search field, pane rows with icons, and a "Show advanced settings" switch
  at the bottom; right side a centred column (max 600) of grouped cards with
  switches and pop-ups on the right and descriptions below.
- Panes: General, Profiles, Sync, Tabs, Appearance, Privacy & Security,
  Passwords, Autofill, Extensions, Search Engine, Keyboard Shortcuts,
  Advanced (Ardia also has Engines, Live Folders and Calendar, which are
  macOS/WebKit or service integrations).
- Appearance: Theme (Automatic/Light/Dark pictures); Interface style
  (Glass/Classic) with tint amount; Address bar (toolbar/sidebar); command bar
  placement; Compact toolbar; bookmarks options; Frame around the page slider
  (default 9; 0 = None); glow around selected tab.
- Tabs: tab layout (sidebar/top); auto-hide tabs; sidebar Left/Right; new tab
  opens (New Tab Page/Command Bar); add new tabs (bottom/top/below current);
  clean up tabs unused for N minutes/hours/days.
- General: default browser, windows and tabs on start (preserve/start fresh),
  website-first vs search-first, downloads folder + ask where to save,
  import/export.
- Privacy: block ads / trackers / cookie banners, site permissions, zoom with
  Ctrl+wheel. Advanced: show full URL, Memory Saver (Moderate/Balanced/
  Maximum), Battery Saver.

## Search engines (in this order; Ardia's default is Google)
| Name | Search URL | Suggestions |
|---|---|---|
| Google | https://www.google.com/search?q=%s | https://suggestqueries.google.com/complete/search?client=firefox&ie=utf-8&oe=utf-8&q=%s |
| Bing | https://www.bing.com/search?q=%s | https://api.bing.com/osjson.aspx?query=%s |
| DuckDuckGo | https://duckduckgo.com/?q=%s | https://duckduckgo.com/ac/?q=%s&type=list |
| Perplexity | https://www.perplexity.ai/search?q=%s | — |
| ChatGPT | https://chatgpt.com/?q=%s | — |
| Ecosia | https://www.ecosia.org/search?q=%s | https://ac.ecosia.org/autocomplete?q=%s&type=list |
| Brave Search | https://search.brave.com/search?q=%s | https://search.brave.com/api/suggest?q=%s |
| Kagi | https://kagi.com/search?q=%s | — |
| Startpage | https://www.startpage.com/do/search?q=%s | https://www.startpage.com/osuggestions?q=%s |
| Yahoo | https://search.yahoo.com/search?p=%s | https://search.yahoo.com/sugg/ff?output=fxjson&command=%s |
| Baidu | https://www.baidu.com/s?wd=%s | https://www.baidu.com/su?action=opensearch&ie=utf-8&wd=%s |
| Yandex | https://yandex.com/search/?text=%s | https://suggest.yandex.com/suggest-ff.cgi?part=%s |

Each engine's shortcut is its host. Picked in Settings › Search Engine from a
list of rows ("✓ Default" / "Make Default"), plus "Add…" for custom engines
(name, shortcut, URL with %s) and a "Show search suggestions" switch.
