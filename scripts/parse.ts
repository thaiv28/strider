import { readFileSync } from "node:fs";
import * as cheerio from "cheerio";

// Google Sheets HTML export → rows of trimmed cell strings. The first column is
// Sheets' row-number gutter; callers locate real columns by header label.
export function parseSheet(path: string): string[][] {
  const $ = cheerio.load(readFileSync(path, "utf8"));
  const rows: string[][] = [];
  $("tr").each((_, tr) => {
    const cells: string[] = [];
    $(tr)
      .find("td,th")
      .each((__, td) => {
        cells.push($(td).text().trim());
      });
    if (cells.some((c) => c !== "")) rows.push(cells);
  });
  return rows;
}

// Find the header row (first row containing `label`) and return it with a
// label→index map. Duplicate labels keep their first index; use `indexOfFrom`
// for the second occurrence.
export function headerMap(
  rows: string[][],
  label: string,
): { headerIdx: number; col: (name: string) => number; header: string[] } {
  const headerIdx = rows.findIndex((r) => r.includes(label));
  if (headerIdx < 0) throw new Error(`header row with "${label}" not found`);
  const header = rows[headerIdx];
  const col = (name: string) => header.indexOf(name);
  return { headerIdx, col, header };
}

export const OZ_TO_G = 28.3495;

export function toGrams(oz: string | undefined): number | null {
  const n = num(oz);
  return n == null ? null : Math.round(n * OZ_TO_G);
}

export function gramsFromGrams(g: string | undefined): number | null {
  const n = num(g);
  return n == null ? null : Math.round(n);
}

export function num(s: string | undefined): number | null {
  if (s == null) return null;
  const cleaned = s.replace(/[$,]/g, "").trim();
  if (cleaned === "" || /#DIV|#REF|#N\/A/i.test(cleaned)) return null;
  const n = Number(cleaned);
  return Number.isFinite(n) ? n : null;
}
