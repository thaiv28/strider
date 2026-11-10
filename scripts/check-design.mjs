#!/usr/bin/env node
// Design guard: keeps UI code on the Topo tokens described in DESIGN.md.
//
// It fails when app code introduces a color, font, or stylesheet outside the
// design system. Existing, deliberate exceptions are listed in BASELINE with a
// reason and a maximum count per file; a file may go below its count but never
// above it. Raising a count or adding a file to BASELINE is a design change
// and needs the owner's explicit approval (see AGENTS.md).
//
//   npm run check:design            check, exit 1 on violations
//   npm run check:design -- --counts print current counts (to tighten BASELINE)

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;
const SCAN_DIRS = ["app", "components", "lib"];
const TOKEN_SHEET = "app/globals.css";
const FONT_ENTRY = "app/layout.tsx";

const PALETTE =
  "slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black";
const UTILITY =
  "bg|text|border(?:-[trblxy])?|ring|ring-offset|fill|stroke|outline|decoration|divide|from|via|to|shadow|accent|caret|placeholder";

const RULES = {
  "color-literal": {
    help: "Raw color value. Use a Topo token utility (bg-panel, text-muted, ...) or a var(--token) from app/globals.css.",
    re: /(?<![&\w])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b|(?<![A-Za-z0-9-])(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb|color-mix)\(/g,
  },
  "palette-class": {
    help: "Tailwind default palette color. Use a Topo token utility instead (e.g. text-ink, bg-panel, text-signal).",
    re: new RegExp(`(?<![\\w-])(?:${UTILITY})-(?:${PALETTE})(?:-\\d{2,3})?(?![\\w-])`, "g"),
  },
  "arbitrary-var": {
    help: "Arbitrary value wrapping a CSS variable. Use the matching token utility (bg-[var(--panel)] -> bg-panel).",
    re: /-\[var\(--[\w-]+\)\]/g,
  },
  font: {
    help: "Ad-hoc font. Use font-sans (Instrument Sans) or font-display (Bricolage Grotesque); fonts load only in app/layout.tsx.",
    re: /\bfontFamily\b|(?<![\w-])font-\[[^\]]*\]/g,
  },
  "font-import": {
    help: "Font import outside app/layout.tsx. New fonts are a design change.",
    re: /@fontsource|next\/font/g,
  },
};

// Deliberate exceptions: file -> { rule: max count }, with the reason.
const BASELINE = {
  "app/apple-icon.tsx": { why: "Generated PNG icon; image renderer has no CSS variables.", "color-literal": 3 },
  "lib/social-image.tsx": { why: "Generated Open Graph image; image renderer has no CSS variables.", "color-literal": 7 },
  "lib/categories.ts": { why: "Category swatch palette, the single source for categoryColor().", "color-literal": 9 },
  "lib/map-style.ts": { why: "Leaflet route and campsite styling drawn on map tiles.", "color-literal": 9 },
  "lib/topo-map.ts": { why: "Server-rendered static topo map image.", "color-literal": 5 },
  "components/trip/route-map.tsx": {
    why: "NOAA/AQI legend scales must match the source tiles; map-overlay controls sit on light tiles in both themes.",
    "color-literal": 34,
    "palette-class": 1,
  },
  "components/trip/trip-view.tsx": {
    why: "Route card shadow and the light 'Hide map' control over map tiles.",
    "color-literal": 2,
    "palette-class": 1,
  },
  "components/trip/shared-trip-view.tsx": {
    why: "Route card shadow and the light map control over map tiles.",
    "color-literal": 2,
    "palette-class": 1,
  },
  "components/trip/options-menu.tsx": { why: "Light map control over map tiles.", "color-literal": 1, "palette-class": 1 },
  "components/settings/calendar-feeds.tsx": { why: "Default color for a user-chosen calendar feed color.", "color-literal": 2 },
  "app/login/page.tsx": {
    why: "Google's required sign-in button styling and logo; error text predates tokens.",
    "color-literal": 7,
    "palette-class": 3,
    font: 1,
  },
  "components/shell.tsx": { why: "Active desktop nav label predates the headerink token.", "palette-class": 1 },
};

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path, out);
    else if (/\.(tsx?|css)$/.test(name)) out.push(path);
  }
  return out;
}

const files = SCAN_DIRS.flatMap((d) => walk(join(ROOT, d))).map((p) => relative(ROOT, p).split(sep).join("/"));
const violations = [];
const counts = {};

for (const file of files) {
  if (file === TOKEN_SHEET) continue;
  if (file.endsWith(".css")) {
    violations.push({ file, rule: "stylesheet", help: `New stylesheet. Shared styles live in ${TOKEN_SHEET}.`, lines: [] });
    continue;
  }
  const lines = readFileSync(join(ROOT, file), "utf8").split("\n");
  for (const [rule, { re, help }] of Object.entries(RULES)) {
    if (rule === "font-import" && file === FONT_ENTRY) continue;
    const hits = [];
    lines.forEach((line, i) => {
      for (const m of line.matchAll(re)) hits.push(`${i + 1}: ${m[0]}`);
    });
    if (!hits.length) continue;
    (counts[file] ??= {})[rule] = hits.length;
    const allowed = BASELINE[file]?.[rule] ?? 0;
    if (hits.length > allowed) violations.push({ file, rule, help, allowed, lines: hits });
  }
}

if (process.argv.includes("--counts")) {
  console.log(JSON.stringify(counts, null, 2));
  process.exit(0);
}

for (const [file, rules] of Object.entries(BASELINE)) {
  for (const [rule, max] of Object.entries(rules)) {
    if (rule === "why") continue;
    const now = counts[file]?.[rule] ?? 0;
    if (now < max) console.log(`note: ${file} ${rule} is down to ${now} (baseline ${max}); lower BASELINE in scripts/check-design.mjs.`);
  }
}

if (violations.length) {
  console.error(`Design check failed: ${violations.length} problem(s). See DESIGN.md.\n`);
  for (const v of violations) {
    const allowance = v.allowed ? ` (${v.lines.length} found, ${v.allowed} allowed)` : "";
    console.error(`${v.file} [${v.rule}]${allowance}\n  ${v.help}`);
    for (const l of v.lines) console.error(`    ${l}`);
  }
  console.error("\nIf this is an intended design change, get explicit approval and update DESIGN.md and BASELINE together.");
  process.exit(1);
}
console.log(`Design check passed (${files.length} files).`);
