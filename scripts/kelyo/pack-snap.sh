#!/bin/sh
# usage: scripts/kelyo/pack-snap.sh <package.tar.xz from scripts/linux/package.py> <version>
# Makes kelyo_<version>_amd64.snap in the current folder. Install it with:
#   pkexec snap install --dangerous --classic kelyo_<version>_amd64.snap
set -eu
package=${1:?usage: pack-snap.sh <package.tar.xz> <version>}
version=${2:?usage: pack-snap.sh <package.tar.xz> <version>}
here=$(cd "$(dirname "$0")" && pwd)
root=$(mktemp -d)
chmod 755 "$root"
trap 'rm -rf "$root"' EXIT
tar -xJf "$package" -C "$root"
mv "$root/ardia/lib/ardia" "$root/kelyo"
rm -rf "$root/ardia"
mkdir -p "$root/bin" "$root/meta/gui"
cat > "$root/bin/kelyo" <<'LAUNCHER'
#!/bin/sh
export CHROME_DESKTOP=kelyo_kelyo.desktop KELYO_DESKTOP=kelyo_kelyo.desktop
exec "$SNAP/kelyo/chrome" --class=kelyo --user-data-dir="${XDG_CONFIG_HOME:-$HOME/.config}/kelyo-browser" "$@"
LAUNCHER
chmod 755 "$root/bin/kelyo"
cp "$here/../../apps/linux/kelyo-assets/app-icon.png" "$root/meta/gui/kelyo.png"
cat > "$root/meta/gui/kelyo.desktop" <<'DESKTOP'
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
DESKTOP
cat > "$root/meta/snap.yaml" <<YAML
name: kelyo
version: '$version'
summary: Kelyo Browser. The web. On your terms.
description: |
  Kelyo Browser: privacy by default, intelligence by choice, freedom by
  design. Built from Ardia (github.com/ardia-browser/ardia), GPL-3.0;
  source at https://github.com/coryabarnes/kelyo.
license: GPL-3.0
architectures: [amd64]
base: core24
confinement: classic
grade: devel
apps:
  kelyo:
    command: bin/kelyo
YAML
snap pack "$root" "$(pwd)"
