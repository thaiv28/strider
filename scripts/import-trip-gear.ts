import { readFileSync } from "node:fs";
import { eq, and } from "drizzle-orm";
import { db, pool, schema } from "../src/db/index";

const CSV = "/home/tvillalu/gearlistweights.csv";

// CSV trip column header → app trip name. North Cascades has no matching trip.
const COLUMN_TRIP: Record<string, string> = {
  "Big Sur 5/26": "Vicente Flat",
  "Robin Lake 6/26": "Tuck and Robin Lake",
  "Olympic 8/26": "Enchanted Valley",
  "Ptarmidge Ridge 8/26": "Ptarmigan Ridge",
};

// Minimal RFC-4180 CSV parser (handles quotes, escaped quotes, newlines in fields).
function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let i = 0;
  let inQ = false;
  while (i < text.length) {
    const c = text[i];
    if (inQ) {
      if (c === '"') {
        if (text[i + 1] === '"') { field += '"'; i += 2; continue; }
        inQ = false; i++; continue;
      }
      field += c; i++; continue;
    }
    if (c === '"') { inQ = true; i++; continue; }
    if (c === ",") { row.push(field); field = ""; i++; continue; }
    if (c === "\r") { i++; continue; }
    if (c === "\n") { row.push(field); rows.push(row); row = []; field = ""; i++; continue; }
    field += c; i++;
  }
  if (field !== "" || row.length) { row.push(field); rows.push(row); }
  return rows;
}

const norm = (s: string) => s.trim().toLowerCase();

// CSV item names → current library names (renamed/split/quantity-cleaned).
const ALIAS: Record<string, string[]> = {
  x10: ["Matches"],
  "gallon (x1), sandwich (x1)": ["Ziploc Gallon Bag", "Ziploc Sandwich Bag"],
  chair: ["REI Flexlite Air"],
  "soto windmaster": ["Soto Windmaster Stove"],
  "toaks 750ml": ["Toaks 750mL Pot"],
  "snow peak titanium": ["Snow Peak Titanium Spork"],
  "inreach mini 3": ["Garmin Inreach Mini"],
};

function resolveNames(raw: string): string[] {
  const a = ALIAS[norm(raw)];
  if (a) return a;
  return [raw.replace(/\s*\(x\d+\)\s*$/i, "").trim()]; // strip trailing "(xN)"
}

async function main() {
  const rows = parseCsv(readFileSync(CSV, "utf8"));
  const header = rows[0];
  const catCol = header.indexOf("Category");
  const itemCol = header.indexOf("Current Item");
  const tripCols = Object.keys(COLUMN_TRIP)
    .map((h) => ({ header: h, idx: header.indexOf(h), tripName: COLUMN_TRIP[h] }))
    .filter((c) => c.idx >= 0);

  // Library lookup by normalized name (any status, so retired gear matches too).
  const items = await db
    .select({
      id: schema.gearItem.id,
      name: schema.gearItem.name,
      weightG: schema.gearItem.weightG,
      quantity: schema.gearItem.quantity,
      cls: schema.gearItem.defaultWeightClass,
      category: schema.gearCategory.name,
    })
    .from(schema.gearItem)
    .leftJoin(schema.gearCategory, eq(schema.gearItem.categoryId, schema.gearCategory.id));
  const byName = new Map<string, (typeof items)[number]>();
  for (const it of items) if (!byName.has(norm(it.name))) byName.set(norm(it.name), it);

  const trips = await db.select({ id: schema.trip.id, name: schema.trip.name }).from(schema.trip);
  const tripByName = new Map(trips.map((t) => [t.name, t.id]));

  for (const col of tripCols) {
    const tripId = tripByName.get(col.tripName);
    if (!tripId) { console.log(`skip ${col.tripName}: no such trip`); continue; }
    const existing = await db
      .select({ id: schema.tripGear.id })
      .from(schema.tripGear)
      .where(eq(schema.tripGear.tripId, tripId))
      .limit(1);
    if (existing.length) { console.log(`skip ${col.tripName}: already has gear`); continue; }

    const added = new Set<number>();
    const values: (typeof schema.tripGear.$inferInsert)[] = [];
    let order = 0;
    let unmatched = 0;
    for (const r of rows.slice(1)) {
      if (norm(r[col.idx] ?? "") !== "true") continue;
      const raw = (r[itemCol] ?? "")
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      if (raw.length === 0 && r[catCol]) raw.push(r[catCol].trim());
      const candidates = raw.flatMap(resolveNames);
      for (const name of candidates) {
        const it = byName.get(norm(name));
        if (!it) { unmatched++; continue; }
        if (added.has(it.id)) continue;
        added.add(it.id);
        values.push({
          tripId,
          gearItemId: it.id,
          snapshotName: it.name,
          snapshotWeightG: it.weightG ?? 0,
          snapshotCategory: it.category,
          weightClass: it.cls === "worn" ? "worn" : "base",
          quantity: it.quantity,
          sortOrder: order++,
        });
      }
    }
    if (values.length) await db.insert(schema.tripGear).values(values);
    console.log(`${col.tripName}: added ${values.length} items (${unmatched} unmatched names)`);
  }
  await pool.end();
}

main().catch(async (e) => {
  console.error(e);
  await pool.end();
  process.exit(1);
});
