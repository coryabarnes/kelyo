import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";

export const NAME = "Kelyo";
export const SOURCE_URL = "https://github.com/coryabarnes/kelyo";
const EDITS: Record<string, [string, string][]> = {
  "state/telemetry/transport.ts": [
    [
      "entry.status = host.send ? await host.send(body, signal) : (await fetch(LOGS_URL, { method: \"POST\", headers: POST_HEADERS, body, signal })).status;",
      "entry.status = 204;",
    ],
  ],
  "platform/killSwitches.ts": [
    ['const SWITCHES_URL = "https://ardia.dev/switches.json";', 'const SWITCHES_URL = "data:application/json,{}";'],
  ],
  "state/report/actions.ts": [
    ['export const FEEDBACK_URL = "https://ardia.dev/api/feedback";', 'export const FEEDBACK_URL = "";'],
  ],
  "reportProblem.ts": [["https://github.com/mantrakp04/ardia/issues/new", `${SOURCE_URL}/issues/new`]],
  "components/settings/panes/Advanced.tsx": [["https://github.com/mantrakp04/ardia", SOURCE_URL]],
  "appUrls.ts": [
    ['const APP_SCHEME = "ardia";', 'const APP_SCHEME = "kelyo";'],
    ["const APP_PREFIX = /^(view-source:)?ardia:(?:\\/\\/)?/i;", "const APP_PREFIX = /^(view-source:)?(?:kelyo|ardia):(?:\\/\\/)?/i;"],
    ["/^ardia:\\/\\/[^/?#]/i.test(url)", "/^(?:kelyo|ardia):\\/\\/[^/?#]/i.test(url)"],
  ],
  "state/pages/urls.ts": [["`ardia://${page}", "`kelyo://${page}"]],
  "state/automation/workspaceOps.ts": [['if (url === "ardia://newtab" || url === "ardia://newtab/") return "";', 'if (/^(?:kelyo|ardia):\\/\\/newtab\\/?$/.test(url)) return "";']],
};
const KEEP_NAME = new Set(["state/telemetry/sanitize.ts", "state/automation/automationMethods.ts"]);

const ASSETS = fileURLToPath(new URL("./kelyo-assets/", import.meta.url));

export function kelyoAsset(source: string): string | null {
  const name = source.split(/[\\/]/).pop()?.split("?")[0] ?? "";
  if (name === "app-icon.png" || /^new-tab-mark(@2x)?\.png$/.test(name)) return ASSETS + name;
  const mark = /^ntp-mark-(?:[a-z]+-)?(back|front|sun)(@2x)?\.png$/.exec(name);
  if (mark) return `${ASSETS}ntp-mark-${mark[1] === "front" ? "front" : "blank"}${mark[2] ?? ""}.png`;
  return null;
}

const ROOTS = ["/apps/browser/src/", "/apps/linux/src/", "/packages/core/src/"];

export function rebrand(id: string, code: string): string | null {
  const path = id.split("?")[0] ?? "";
  const root = ROOTS.find((r) => path.includes(r));
  if (!root || !/\.(ts|tsx)$/.test(path)) return null;
  const file = path.slice(path.indexOf(root) + root.length);
  let out = code;
  for (const [from, to] of EDITS[file] ?? []) {
    if (!out.includes(from)) throw new Error(`${file} no longer contains ${JSON.stringify(from.slice(0, 80))}; update apps/linux/kelyoBrand.ts`);
    out = out.replace(from, () => to);
  }
  if (!KEEP_NAME.has(file)) out = out.replace(/(?<![\^$])\bArdia\b(?![[$])/g, NAME);
  return out === code ? null : out;
}

export function kelyoBrand(): Plugin {
  return {
    name: "kelyo-brand",
    enforce: "pre",
    transformIndexHtml(html) {
      return html.replace(/\bArdia\b/g, NAME);
    },
    resolveId(source) {
      return kelyoAsset(source);
    },
    transform(code, id) {
      const out = rebrand(id, code);
      return out === null ? null : { code: out, map: null };
    },
  };
}
