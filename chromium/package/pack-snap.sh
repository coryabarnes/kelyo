#!/bin/sh
# Packs a Kite build (Chromium out dir) into a classic snap.
#
#   chromium/package/pack-snap.sh ~/chromium/src/out/Kite [version]
#
# Install the result with:
#   pkexec snap install --dangerous --classic kite_<version>_amd64.snap
set -eu

out=${1:?usage: pack-snap.sh <chromium out dir> [version]}
here=$(cd "$(dirname "$0")" && pwd)
version=${2:-$(. "$out/../../chrome/VERSION"; echo "$MAJOR.$MINOR.$BUILD.$PATCH")}
root=$(mktemp -d)
chmod 755 "$root"  # mktemp makes it private; snaps must be world-readable.
trap 'rm -rf "$root"' EXIT
app="$root/kite"

mkdir -p "$app" "$root/bin" "$root/meta/gui"

# The same files Chrome's own Linux installer ships.
for f in chrome chrome_crashpad_handler icudtl.dat resources.pak \
         chrome_100_percent.pak chrome_200_percent.pak \
         v8_context_snapshot.bin snapshot_blob.bin \
         libEGL.so libGLESv2.so libvk_swiftshader.so libvulkan.so.1 \
         vk_swiftshader_icd.json libLiteRtWebGpuAccelerator.so \
         liboptimization_guide_internal.so; do
  [ -e "$out/$f" ] && cp -a "$out/$f" "$app/"
done
for d in locales MEIPreload PrivacySandboxAttestationsPreloaded; do
  [ -d "$out/$d" ] && cp -a "$out/$d" "$app/"
done
find "$app" -maxdepth 1 -type f \( -name chrome -o -name chrome_crashpad_handler -o -name '*.so*' \) \
  -exec strip --strip-unneeded {} +

# Kite's defaults: first-run welcome, dark theme, DuckDuckGo, uBlock Origin Lite.
cp "$here/initial_preferences" "$app/"
cp -r "$here/extensions" "$app/"

cat > "$root/bin/kite" <<'EOF'
#!/bin/sh
# Tells Kite which launcher entry is its own, so the window groups with its icon.
export CHROME_DESKTOP=kite_kite.desktop
# Kite stays light: no Gemini assistant, Lens, shopping helpers, AI history
# search, or background downloads of AI models and optimization hints.
exec "$SNAP/kite/chrome" --class=kite \
  --disable-features=Glic,GlicRollout,ApiGlicPrivate,ApiContextualTasksPrivate,HistoryEmbeddings,LensOverlay,LensStandalone,MerchantTrust,ShoppingList,OptimizationHints,OptimizationTargetPrediction,OptimizationGuideModelExecution,OnDeviceModelBackgroundDownload,ModelQualityLogging \
  "$@"
EOF
chmod 755 "$root/bin/kite"

cp "$here/../branding/kite-icon.svg" "$root/meta/gui/kite.svg"
cat > "$root/meta/gui/kite.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Kite
GenericName=Web Browser
Comment=A calmer way through the web
Exec=kite %U
Icon=${SNAP}/meta/gui/kite.svg
Terminal=false
Categories=Network;WebBrowser;
MimeType=text/html;x-scheme-handler/http;x-scheme-handler/https;
StartupNotify=true
StartupWMClass=kite
Actions=new-window;new-private-window;

[Desktop Action new-window]
Name=New Window
Exec=kite

[Desktop Action new-private-window]
Name=New Incognito Window
Exec=kite --incognito
EOF

cat > "$root/meta/snap.yaml" <<EOF
name: kite
version: '$version'
summary: A calmer way through the web
description: |
  Kite is a Chromium-based browser with tabs in a sidebar, split view,
  built-in ad blocking, and the Chrome Web Store.
architectures: [amd64]
base: core24
confinement: classic
grade: devel
apps:
  kite:
    command: bin/kite
EOF

snap pack "$root" "$(pwd)"
