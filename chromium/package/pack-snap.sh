#!/bin/sh
# Packs a Kelyo build (Chromium out dir) into a classic snap.
#
#   chromium/package/pack-snap.sh ~/chromium/src/out/Kelyo [version]
#
# Install the result with:
#   pkexec snap install --dangerous --classic kelyo_<version>_amd64.snap
set -eu

out=${1:?usage: pack-snap.sh <chromium out dir> [version]}
here=$(cd "$(dirname "$0")" && pwd)
version=${2:-$(. "$out/../../chrome/VERSION"; echo "$MAJOR.$MINOR.$BUILD.$PATCH")}
root=$(mktemp -d)
chmod 755 "$root"  # mktemp makes it private; snaps must be world-readable.
trap 'rm -rf "$root"' EXIT
app="$root/kelyo"

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

# Kelyo's defaults: first-run welcome, dark theme, DuckDuckGo, uBlock Origin Lite.
cp "$here/initial_preferences" "$app/"
cp -r "$here/extensions" "$app/"

cat > "$root/bin/kelyo" <<'EOF'
#!/bin/sh
# Tells Kelyo which launcher entry is its own, so the window groups with its icon.
export CHROME_DESKTOP=kelyo_kelyo.desktop
# Kelyo stays light: no Gemini assistant, Lens, shopping helpers, AI history
# search, or background downloads of AI models and optimization hints.
exec "$SNAP/kelyo/chrome" --class=kelyo \
  --disable-features=Glic,GlicRollout,ApiGlicPrivate,ApiContextualTasksPrivate,HistoryEmbeddings,LensOverlay,LensStandalone,MerchantTrust,ShoppingList,OptimizationHints,OptimizationTargetPrediction,OptimizationGuideModelExecution,OnDeviceModelBackgroundDownload,ModelQualityLogging \
  "$@"
EOF
chmod 755 "$root/bin/kelyo"

cp "$here/../branding/kelyo-icon.png" "$root/meta/gui/kelyo.png"
cat > "$root/meta/gui/kelyo.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Kelyo
GenericName=Web Browser
Comment=The web. On your terms.
Exec=kelyo %U
Icon=${SNAP}/meta/gui/kelyo.png
Terminal=false
Categories=Network;WebBrowser;
MimeType=text/html;x-scheme-handler/http;x-scheme-handler/https;
StartupNotify=true
StartupWMClass=kelyo
Actions=new-window;new-private-window;

[Desktop Action new-window]
Name=New Window
Exec=kelyo

[Desktop Action new-private-window]
Name=New Incognito Window
Exec=kelyo --incognito
EOF

cat > "$root/meta/snap.yaml" <<EOF
name: kelyo
version: '$version'
summary: Kelyo Browser. The web. On your terms.
description: |
  Kelyo Browser is a Chromium-based browser built on three promises:
  privacy by default, intelligence by choice, and freedom by design.
  Tabs and the address bar live in a sidebar painted with your own
  gradient theme, uBlock Origin Lite blocks ads and trackers, and the
  Chrome Web Store works as usual.
architectures: [amd64]
base: core24
confinement: classic
grade: devel
apps:
  kelyo:
    command: bin/kelyo
EOF

snap pack "$root" "$(pwd)"
