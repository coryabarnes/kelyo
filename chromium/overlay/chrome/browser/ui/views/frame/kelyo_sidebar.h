// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_SIDEBAR_H_
#define CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_SIDEBAR_H_

#include "ui/base/class_property.h"

// Kelyo moves the toolbar into the top of the expanded vertical tab strip, so
// pages run to the top of the window. The browser layout sets this property on
// the toolbar while it's there; the toolbar then lays itself out in two rows
// (buttons, then the address bar) to fit the sidebar's width.
//
// Defined inline here (rather than with DEFINE_UI_CLASS_PROPERTY_KEY in a .cc)
// so the layout and the toolbar can share it without a new build target.
inline constexpr ui::ClassProperty<bool> kKelyoToolbarInSidebarValue = {
    false, "kKelyoToolbarInSidebarKey", false, nullptr};
inline constexpr const ui::ClassProperty<bool>* kKelyoToolbarInSidebarKey =
    &kKelyoToolbarInSidebarValue;

#endif  // CHROME_BROWSER_UI_VIEWS_FRAME_KELYO_SIDEBAR_H_
