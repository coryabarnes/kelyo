#!/bin/sh
# Builds target/kite_<version>_amd64.snap from a fresh `cargo xtask bundle`.
# Install it with: snap install --dangerous --classic target/kite_*.snap
set -eu
cd "$(dirname "$0")/../.."
here=packaging/snap
root=target/snap

cargo xtask bundle --release

rm -rf "$root"
mkdir -p "$root/meta/gui" "$root/bin"
cp -r target/bundle "$root/kite"
# CEF ships libcef.so with debug info (~1.4 GB); stripping it saves ~1.2 GB.
strip --strip-unneeded "$root/kite/libcef.so"
cp "$here/snap.yaml" "$root/meta/snap.yaml"
cp "$here/kite.desktop" "$here/kite.svg" "$root/meta/gui/"
install -m 755 "$here/kite-launcher" "$root/bin/kite-launcher"

snap pack "$root" target
