// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#include "chrome/browser/ui/webui/kelyo_welcome/kelyo_welcome_ui.h"

#include <memory>
#include <vector>

#include "base/functional/bind.h"
#include "base/values.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/themes/theme_service.h"
#include "chrome/browser/themes/theme_service_factory.h"
#include "chrome/browser/ui/kelyo_theme_prefs.h"
#include "chrome/grit/kelyo_welcome_resources.h"
#include "chrome/grit/kelyo_welcome_resources_map.h"
#include "components/prefs/pref_service.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_data_source.h"
#include "content/public/browser/web_ui_message_handler.h"
#include "ui/base/mojom/themes.mojom.h"
#include "ui/gfx/color_utils.h"
#include "ui/webui/webui_util.h"

namespace {

// Lets Kelyo's pages read and set the gradient theme:
//   getKelyoTheme(callbackId) -> {colors: ["#rrggbb", ...], grain: 0..1}
//   setKelyoTheme(colors, grain)  (empty colors: back to Kelyo's default look)
class KelyoThemeHandler : public content::WebUIMessageHandler {
 public:
  KelyoThemeHandler() = default;
  KelyoThemeHandler(const KelyoThemeHandler&) = delete;
  KelyoThemeHandler& operator=(const KelyoThemeHandler&) = delete;
  ~KelyoThemeHandler() override = default;

  void RegisterMessages() override {
    web_ui()->RegisterMessageCallback(
        "getKelyoTheme", base::BindRepeating(&KelyoThemeHandler::HandleGetTheme,
                                            base::Unretained(this)));
    web_ui()->RegisterMessageCallback(
        "setKelyoTheme", base::BindRepeating(&KelyoThemeHandler::HandleSetTheme,
                                            base::Unretained(this)));
  }

 private:
  Profile* profile() { return Profile::FromWebUI(web_ui()); }

  void HandleGetTheme(const base::ListValue& args) {
    AllowJavascript();
    PrefService* prefs = profile()->GetPrefs();
    base::DictValue theme;
    theme.Set("colors", prefs->GetList(kelyo::prefs::kThemeColors).Clone());
    theme.Set("grain", prefs->GetDouble(kelyo::prefs::kThemeGrain));
    ResolveJavascriptCallback(args[0], theme);
  }

  void HandleSetTheme(const base::ListValue& args) {
    if (args.size() != 2 || !args[0].is_list() || !args[1].is_double()) {
      return;
    }
    base::ListValue colors;
    std::vector<SkColor> parsed;
    for (const base::Value& value : args[0].GetList()) {
      const std::string* text = value.GetIfString();
      if (!text || parsed.size() == 3) {
        continue;
      }
      if (auto color = kelyo::ParseThemeColor(*text)) {
        parsed.push_back(*color);
        colors.Append(*text);
      }
    }
    PrefService* prefs = profile()->GetPrefs();
    prefs->SetList(kelyo::prefs::kThemeColors, std::move(colors));
    prefs->SetDouble(kelyo::prefs::kThemeGrain,
                     std::clamp(args[1].GetDouble(), 0.0, 1.0));

    // Chrome derives its text, icon and menu colours from one colour: give it
    // the gradient's average, and pick light or dark mode by its brightness so
    // text on the sidebar stays readable. This also repaints every window.
    ThemeService* theme_service = ThemeServiceFactory::GetForProfile(profile());
    if (parsed.empty()) {
      theme_service->SetUserColor(std::nullopt);
      theme_service->SetBrowserColorScheme(
          ThemeService::BrowserColorScheme::kDark);
      return;
    }
    int r = 0, g = 0, b = 0;
    for (SkColor color : parsed) {
      r += SkColorGetR(color);
      g += SkColorGetG(color);
      b += SkColorGetB(color);
    }
    const int n = static_cast<int>(parsed.size());
    const SkColor average = SkColorSetRGB(r / n, g / n, b / n);
    theme_service->SetBrowserColorScheme(
        color_utils::IsDark(average) ? ThemeService::BrowserColorScheme::kDark
                                     : ThemeService::BrowserColorScheme::kLight);
    theme_service->SetUserColorAndBrowserColorVariant(
        average, ui::mojom::BrowserColorVariant::kTonalSpot);
  }
};

void SetUpPage(content::WebUI* web_ui, const char* host, int default_resource) {
  content::WebUIDataSource* source = content::WebUIDataSource::CreateAndAdd(
      web_ui->GetWebContents()->GetBrowserContext(), host);
  webui::SetupWebUIDataSource(source, kKelyoWelcomeResources, default_resource);
  web_ui->AddMessageHandler(std::make_unique<KelyoThemeHandler>());
}

}  // namespace

KelyoWelcomeUI::KelyoWelcomeUI(content::WebUI* web_ui)
    : content::WebUIController(web_ui) {
  SetUpPage(web_ui, kChromeUIKelyoWelcomeHost, IDR_KELYO_WELCOME_WELCOME_HTML);
}

KelyoWelcomeUI::~KelyoWelcomeUI() = default;

KelyoThemeUI::KelyoThemeUI(content::WebUI* web_ui)
    : content::WebUIController(web_ui) {
  SetUpPage(web_ui, kChromeUIKelyoThemeHost, IDR_KELYO_WELCOME_THEME_HTML);
}

KelyoThemeUI::~KelyoThemeUI() = default;
