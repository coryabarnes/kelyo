// Copyright 2026 Kite. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#include "chrome/browser/ui/webui/kite_welcome/kite_welcome_ui.h"

#include "chrome/grit/kite_welcome_resources.h"
#include "chrome/grit/kite_welcome_resources_map.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_data_source.h"
#include "ui/webui/webui_util.h"

KiteWelcomeUI::KiteWelcomeUI(content::WebUI* web_ui)
    : content::WebUIController(web_ui) {
  content::WebUIDataSource* source = content::WebUIDataSource::CreateAndAdd(
      web_ui->GetWebContents()->GetBrowserContext(), kChromeUIKiteWelcomeHost);
  webui::SetupWebUIDataSource(source, kKiteWelcomeResources,
                              IDR_KITE_WELCOME_WELCOME_HTML);
}

KiteWelcomeUI::~KiteWelcomeUI() = default;
