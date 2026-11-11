# Kelyo Browser on Chromium

Kelyo is Chromium with Kelyo's changes applied on top, built the way Google
builds Chrome (official build: profile-guided optimization and ThinLTO). This
folder holds everything that makes it Kelyo; the Chromium source itself lives
outside the repository (about 30 GB).

```text
apply.sh            Applies everything below to a Chromium checkout (safe to rerun)
branding/           App icon (SVG source and rendered PNGs)
welcome/            chrome://kelyo-welcome (first-run tour and its generative tune)
                    and chrome://kelyo-theme (gradient theme picker)
overlay/            New files added to Chromium: those pages' WebUI, the theme
                    prefs and painter, and the sidebar toolbar flag
patches/            Changes to Chromium's own files
  0001  Tabs in the sidebar by default; theme prefs
  0002  Register Kelyo's pages; let the welcome play its music on open
  0003  Memory Saver on by default; background tabs nap after 30 minutes
  0004  Skip Chrome's first-run screens; no "API keys missing" warning
  0005  Address bar (whole toolbar) in the sidebar; window buttons on the left
  0006  Gradient themes in the sidebar; "Theme and colors" in the menu
package/            Snap packaging
  initial_preferences   First-run tab, dark theme, DuckDuckGo search
  extensions/           uBlock Origin Lite, installed on first run
  pack-snap.sh          Builds the snap, with a launcher that turns off
                        Gemini, Lens, shopping and background AI downloads
```

## Building

Needs about 150 GB of disk. The first build takes several hours; after that,
changes rebuild in minutes.

```bash
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git ~/depot_tools
export PATH=$HOME/depot_tools:$PATH
```

Get the Chromium release Kelyo is based on (157.0.8103.50), with the PGO
profile official builds use:

```bash
mkdir ~/chromium && cd ~/chromium
cat > .gclient <<'EOF'
solutions = [{
  "name": "src",
  "url": "https://chromium.googlesource.com/chromium/src.git@refs/tags/157.0.8103.50",
  "managed": False,
  "custom_deps": {},
  "custom_vars": {"checkout_pgo_profiles": True},
}]
target_os = ["linux"]
EOF
gclient sync --nohooks --no-history --shallow
gclient runhooks
```

Apply Kelyo and build:

```bash
~/src/kite/chromium/apply.sh ~/chromium/src
cd ~/chromium/src
mkdir -p out/Kelyo
cat > out/Kelyo/args.gn <<'EOF'
is_official_build = true
chrome_pgo_phase = 2
use_thin_lto = true
is_cfi = false
is_debug = false
is_component_build = false
symbol_level = 0
blink_symbol_level = 0
v8_symbol_level = 0
treat_warnings_as_errors = false
enable_linux_installer = false
proprietary_codecs = true
ffmpeg_branding = "Chrome"
EOF
gn gen out/Kelyo
autoninja -C out/Kelyo chrome
```

Package and install:

```bash
~/src/kite/chromium/package/pack-snap.sh ~/chromium/src/out/Kelyo
pkexec snap install --dangerous --classic kelyo_*.snap
```

Kelyo keeps its profile in `~/.config/kelyo`.
