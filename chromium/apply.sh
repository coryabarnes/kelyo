#!/bin/sh
# Applies Kite's changes to a Chromium checkout. Safe to run more than once.
#
#   chromium/apply.sh ~/chromium/src
#
# Branding (name, icons, profile folder) is rewritten in place, new files are
# copied from overlay/ and welcome/, and changes to Chromium's own files live
# in patches/ (applied with `git apply`).
set -eu

src=${1:?usage: apply.sh <chromium/src>}
here=$(cd "$(dirname "$0")" && pwd)
cd "$src"

echo "Branding: name"
sed -i \
  -e 's/^COMPANY_FULLNAME=.*/COMPANY_FULLNAME=Kite/' \
  -e 's/^COMPANY_SHORTNAME=.*/COMPANY_SHORTNAME=Kite/' \
  -e 's/^PRODUCT_FULLNAME=.*/PRODUCT_FULLNAME=Kite/' \
  -e 's/^PRODUCT_SHORTNAME=.*/PRODUCT_SHORTNAME=Kite/' \
  -e 's/^PRODUCT_INSTALLER_FULLNAME=.*/PRODUCT_INSTALLER_FULLNAME=Kite Installer/' \
  -e 's/^PRODUCT_INSTALLER_SHORTNAME=.*/PRODUCT_INSTALLER_SHORTNAME=Kite Installer/' \
  -e 's/^COPYRIGHT=.*/COPYRIGHT=Copyright @LASTCHANGE_YEAR@ Kite and The Chromium Authors./' \
  chrome/app/theme/chromium/BRANDING
# User-visible strings ("Chromium is up to date", "About Chromium", ...), in
# every language. Whole words only, so ids and "ChromiumOS" are untouched.
sed -i 's/\bChromium\b/Kite/g' chrome/app/chromium_strings.grd chrome/app/resources/chromium_strings_*.xtb

echo "Branding: profile folder (~/.config/kite)"
sed -i 's/std::string data_dir_basename = "chromium";/std::string data_dir_basename = "kite";/' \
  chrome/common/chrome_paths_linux.cc

echo "Branding: icons"
logo="$here/branding/product_logo_256.png"
find chrome/app/theme -path '*chromium*' -name 'product_logo_*.png' \
  ! -name '*mono*' ! -name '*name*' | while read -r png; do
  size=$(magick identify -format '%w' "$png")
  magick "$logo" -resize "${size}x${size}" -strip "$png"
done
magick "$logo" -resize 32x32 chrome/app/theme/chromium/linux/product_logo_32.xpm

echo "New files (overlay/, and the welcome page)"
cp -r "$here/overlay/." .
cp "$here"/welcome/welcome.html "$here"/welcome/welcome.css \
   "$here"/welcome/welcome.js "$here"/welcome/music.js \
   chrome/browser/resources/kite_welcome/

echo "Code changes"
for patch in "$here"/patches/*.patch; do
  [ -e "$patch" ] || continue
  if git apply --reverse --check "$patch" 2>/dev/null; then
    echo "  already applied: $(basename "$patch")"
  else
    git apply "$patch"
    echo "  applied: $(basename "$patch")"
  fi
done
