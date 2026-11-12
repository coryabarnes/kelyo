// Copyright 2026 Kelyo. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#ifndef COMPONENTS_SEARCH_ENGINES_KELYO_SEARCH_ENGINES_H_
#define COMPONENTS_SEARCH_ENGINES_KELYO_SEARCH_ENGINES_H_

#include "base/values.h"

namespace kelyo {

// Kelyo's built-in search engines, in the order they're offered. This is the
// default value of the "search_provider_overrides" pref, which replaces
// Chromium's per-country list. Ids match Chromium's prepopulated engines
// (ChatGPT, which Chromium doesn't have, gets its own).
inline base::ListValue SearchEngines() {
  struct Engine {
    const char* name;
    const char* keyword;
    const char* search_url;
    const char* suggest_url;
    const char* favicon_url;
    int id;
  };
  static constexpr Engine kEngines[] = {
      {"Google", "google.com", "https://www.google.com/search?q={searchTerms}",
       "https://suggestqueries.google.com/complete/"
       "search?client=firefox&ie=utf-8&oe=utf-8&q={searchTerms}",
       "https://www.google.com/images/branding/product/ico/googleg_alldp.ico",
       1},
      {"Bing", "bing.com", "https://www.bing.com/search?q={searchTerms}",
       "https://api.bing.com/osjson.aspx?query={searchTerms}",
       "https://www.bing.com/favicon.ico", 3},
      {"DuckDuckGo", "duckduckgo.com", "https://duckduckgo.com/?q={searchTerms}",
       "https://duckduckgo.com/ac/?q={searchTerms}&type=list",
       "https://duckduckgo.com/favicon.ico", 92},
      {"Perplexity", "perplexity.ai",
       "https://www.perplexity.ai/search?q={searchTerms}", "",
       "https://www.perplexity.ai/favicon.ico", 117},
      {"ChatGPT", "chatgpt.com", "https://chatgpt.com/?q={searchTerms}", "",
       "https://chatgpt.com/favicon.ico", 200},
      {"Ecosia", "ecosia.org", "https://www.ecosia.org/search?q={searchTerms}",
       "https://ac.ecosia.org/autocomplete?q={searchTerms}&type=list",
       "https://cdn.ecosia.org/assets/images/ico/favicon.ico", 101},
      {"Brave Search", "search.brave.com",
       "https://search.brave.com/search?q={searchTerms}",
       "https://search.brave.com/api/suggest?q={searchTerms}",
       "https://cdn.search.brave.com/serp/favicon.ico", 109},
      {"Kagi", "kagi.com", "https://kagi.com/search?q={searchTerms}", "",
       "https://kagi.com/favicon.ico", 115},
      {"Startpage", "startpage.com",
       "https://www.startpage.com/do/search?q={searchTerms}",
       "https://www.startpage.com/osuggestions?q={searchTerms}",
       "https://www.startpage.com/favicon.ico", 113},
      {"Yahoo", "yahoo.com", "https://search.yahoo.com/search?p={searchTerms}",
       "https://search.yahoo.com/sugg/ff?output=fxjson&command={searchTerms}",
       "https://search.yahoo.com/favicon.ico", 2},
      {"Baidu", "baidu.com", "https://www.baidu.com/s?wd={searchTerms}",
       "https://www.baidu.com/su?action=opensearch&ie=utf-8&wd={searchTerms}",
       "https://www.baidu.com/favicon.ico", 21},
      {"Yandex", "yandex.com", "https://yandex.com/search/?text={searchTerms}",
       "https://suggest.yandex.com/suggest-ff.cgi?part={searchTerms}",
       "https://yandex.com/favicon.ico", 15},
  };
  base::ListValue list;
  for (const Engine& engine : kEngines) {
    base::DictValue dict;
    dict.Set("name", engine.name);
    dict.Set("keyword", engine.keyword);
    dict.Set("search_url", engine.search_url);
    if (*engine.suggest_url) {
      dict.Set("suggest_url", engine.suggest_url);
    }
    dict.Set("favicon_url", engine.favicon_url);
    dict.Set("encoding", "UTF-8");
    dict.Set("id", engine.id);
    list.Append(std::move(dict));
  }
  return list;
}

}  // namespace kelyo

#endif  // COMPONENTS_SEARCH_ENGINES_KELYO_SEARCH_ENGINES_H_
