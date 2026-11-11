// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef CHROME_BROWSER_UI_WEBUI_KELYO_WELCOME_KELYO_WELCOME_UI_H_
#define CHROME_BROWSER_UI_WEBUI_KELYO_WELCOME_KELYO_WELCOME_UI_H_

#include "content/public/browser/web_ui_controller.h"
#include "content/public/browser/webui_config.h"
#include "content/public/common/url_constants.h"

// Kept here rather than in webui_url_constants.h, which much of Chrome
// includes, so adding Kelyo's pages doesn't rebuild the world.
inline constexpr char kChromeUIKelyoWelcomeHost[] = "kelyo-welcome";
inline constexpr char kChromeUIKelyoWelcomeURL[] = "chrome://kelyo-welcome/";
inline constexpr char kChromeUIKelyoThemeHost[] = "kelyo-theme";
inline constexpr char kChromeUIKelyoThemeURL[] = "chrome://kelyo-theme/";

class KelyoWelcomeUI;
class KelyoThemeUI;

class KelyoWelcomeUIConfig : public content::DefaultWebUIConfig<KelyoWelcomeUI> {
 public:
  KelyoWelcomeUIConfig()
      : DefaultWebUIConfig(content::kChromeUIScheme, kChromeUIKelyoWelcomeHost) {
  }
};

class KelyoThemeUIConfig : public content::DefaultWebUIConfig<KelyoThemeUI> {
 public:
  KelyoThemeUIConfig()
      : DefaultWebUIConfig(content::kChromeUIScheme, kChromeUIKelyoThemeHost) {}
};

// chrome://kelyo-welcome/: Kelyo's first-run welcome, with its welcome tune.
class KelyoWelcomeUI : public content::WebUIController {
 public:
  explicit KelyoWelcomeUI(content::WebUI* web_ui);
  ~KelyoWelcomeUI() override;
};

// chrome://kelyo-theme/: picks Kelyo's gradient theme.
class KelyoThemeUI : public content::WebUIController {
 public:
  explicit KelyoThemeUI(content::WebUI* web_ui);
  ~KelyoThemeUI() override;
};

#endif  // CHROME_BROWSER_UI_WEBUI_KELYO_WELCOME_KELYO_WELCOME_UI_H_
