// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_THEME_PAINTER_H_
#define CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_THEME_PAINTER_H_

// Paints Kelyo's gradient theme (chrome://kelyo-theme) wherever Chrome would
// paint the frame colour: the sidebar, the window-button band, and the
// toolbar while it sits in the sidebar. Header-only so the views that paint
// it need no new build target.

#include <algorithm>
#include <vector>

#include "base/no_destructor.h"
#include "cc/paint/paint_flags.h"
#include "cc/paint/paint_image.h"
#include "cc/paint/paint_shader.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/kelyo_theme_prefs.h"
#include "chrome/browser/ui/views/frame/browser_view.h"
#include "third_party/skia/include/core/SkBitmap.h"
#include "third_party/skia/include/core/SkColor.h"
#include "ui/gfx/canvas.h"
#include "ui/gfx/color_utils.h"
#include "ui/views/view.h"

namespace kelyo {

// A tile of monochrome noise for the grain, made once.
inline const cc::PaintImage& GrainTile() {
  static const base::NoDestructor<cc::PaintImage> tile([] {
    constexpr int kSize = 256;
    SkBitmap bitmap;
    bitmap.allocN32Pixels(kSize, kSize);
    uint32_t seed = 0x6b697465;  // "kelyo"
    for (int y = 0; y < kSize; ++y) {
      for (int x = 0; x < kSize; ++x) {
        seed = seed * 1664525u + 1013904223u;
        const uint8_t v = seed >> 24;
        // Opaque grey, so the pixel byte order doesn't matter.
        *bitmap.getAddr32(x, y) = SkColorSetARGB(SK_AlphaOPAQUE, v, v, v);
      }
    }
    bitmap.setImmutable();
    return cc::PaintImage::CreateFromBitmap(std::move(bitmap));
  }());
  return *tile;
}

// Fills `view`'s bounds (within `canvas`'s clip) with the theme gradient and
// grain.
// Both are positioned relative to the whole browser view, so neighbouring
// views painted this way join seamlessly. Returns false, painting nothing,
// when no Kelyo theme is set.
inline bool PaintThemeGradient(gfx::Canvas* canvas,
                               const views::View* view,
                               const BrowserView* browser_view) {
  if (!browser_view || !browser_view->GetProfile()) {
    return false;
  }
  const PrefService* prefs = browser_view->GetProfile()->GetPrefs();
  std::vector<SkColor> colors = GetThemeColors(prefs);
  if (colors.empty()) {
    return false;
  }
  if (colors.size() == 1) {
    // One colour still gets some depth: it deepens toward the bottom.
    colors.push_back(color_utils::AlphaBlend(SK_ColorBLACK, colors[0], 0.3f));
  }

  gfx::Point origin;
  views::View::ConvertPointToTarget(view, browser_view, &origin);
  const float height = std::max(1, browser_view->height());
  // Mostly top to bottom, leaning slightly across.
  const SkPoint points[2] = {
      SkPoint::Make(-origin.x(), -origin.y()),
      SkPoint::Make(height * 0.3f - origin.x(), height - origin.y())};
  std::vector<SkColor4f> stops;
  for (SkColor color : colors) {
    stops.push_back(SkColor4f::FromColor(color));
  }
  const gfx::RectF bounds(view->GetLocalBounds());
  cc::PaintFlags gradient;
  gradient.setShader(cc::PaintShader::MakeLinearGradient(
      points, stops.data(), nullptr, stops.size(), SkTileMode::kClamp));
  canvas->DrawRect(bounds, gradient);

  const double grain = std::clamp(prefs->GetDouble(prefs::kThemeGrain), 0.0, 1.0);
  if (grain > 0.0) {
    const SkMatrix matrix = SkMatrix::Translate(-origin.x(), -origin.y());
    cc::PaintFlags noise;
    noise.setShader(cc::PaintShader::MakeImage(
        GrainTile(), SkTileMode::kRepeat, SkTileMode::kRepeat, &matrix));
    noise.setAlphaf(static_cast<float>(0.28 * grain));
    noise.setBlendMode(SkBlendMode::kOverlay);
    canvas->DrawRect(bounds, noise);
  }
  return true;
}

}  // namespace kelyo

#endif  // CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_THEME_PAINTER_H_
