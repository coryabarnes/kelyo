// Copyright 2026 Kite. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef CHROME_BROWSER_UI_WEBUI_KITE_WELCOME_KITE_WELCOME_UI_H_
#define CHROME_BROWSER_UI_WEBUI_KITE_WELCOME_KITE_WELCOME_UI_H_

#include "content/public/browser/web_ui_controller.h"
#include "content/public/browser/webui_config.h"
#include "content/public/common/url_constants.h"

// Kept here rather than in webui_url_constants.h, which much of Chrome
// includes, so adding Kite's page doesn't rebuild the world.
inline constexpr char kChromeUIKiteWelcomeHost[] = "kite-welcome";
inline constexpr char kChromeUIKiteWelcomeURL[] = "chrome://kite-welcome/";

class KiteWelcomeUI;

class KiteWelcomeUIConfig : public content::DefaultWebUIConfig<KiteWelcomeUI> {
 public:
  KiteWelcomeUIConfig()
      : DefaultWebUIConfig(content::kChromeUIScheme, kChromeUIKiteWelcomeHost) {
  }
};

// chrome://kite-welcome/: Kite's first-run welcome, with its welcome tune.
class KiteWelcomeUI : public content::WebUIController {
 public:
  explicit KiteWelcomeUI(content::WebUI* web_ui);
  ~KiteWelcomeUI() override;
};

#endif  // CHROME_BROWSER_UI_WEBUI_KITE_WELCOME_KITE_WELCOME_UI_H_
