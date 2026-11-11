// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef CHROME_BROWSER_UI_KELYO_THEME_PREFS_H_
#define CHROME_BROWSER_UI_KELYO_THEME_PREFS_H_

#include <optional>
#include <string_view>
#include <vector>

#include "base/strings/string_number_conversions.h"
#include "base/values.h"
#include "components/prefs/pref_service.h"
#include "third_party/skia/include/core/SkColor.h"

// Kelyo's gradient theme, per profile. Set from chrome://kelyo-theme.
namespace kelyo::prefs {

// One to three "#rrggbb" colours, top of the sidebar first. Empty means no
// Kelyo theme: Chrome's own colours are used.
inline constexpr char kThemeColors[] = "kelyo.theme.colors";

// Film grain over the gradient, from 0 (none) to 1.
inline constexpr char kThemeGrain[] = "kelyo.theme.grain";

}  // namespace kelyo::prefs

namespace kelyo {

// Parses "#rrggbb".
inline std::optional<SkColor> ParseThemeColor(std::string_view text) {
  uint32_t rgb = 0;
  if (text.size() != 7 || text[0] != '#' ||
      !base::HexStringToUInt(text.substr(1), &rgb)) {
    return std::nullopt;
  }
  return SkColorSetA(rgb, SK_AlphaOPAQUE);
}

// The profile's theme colours (at most three); empty when no Kelyo theme is set.
inline std::vector<SkColor> GetThemeColors(const PrefService* prefs) {
  std::vector<SkColor> colors;
  for (const base::Value& value : prefs->GetList(prefs::kThemeColors)) {
    if (const std::string* text = value.GetIfString()) {
      if (auto color = ParseThemeColor(*text); color && colors.size() < 3) {
        colors.push_back(*color);
      }
    }
  }
  return colors;
}

}  // namespace kelyo

#endif  // CHROME_BROWSER_UI_KELYO_THEME_PREFS_H_
