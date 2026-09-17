import { sql, eq, and } from "drizzle-orm";
import { db, pool, schema } from "../src/db/index";
import { parseSheet, headerMap, toGrams, gramsFromGrams, num, OZ_TO_G } from "./parse";

const DIR = "/home/tvillalu/backpack";
const P = {
  gear: `${DIR}/Gear List (weights).html`,
  firstAid: `${DIR}/First Aid.html`,
  repair: `${DIR}/Repair Kit.html`,
  meals: `${DIR}/Meals.html`,
  hikes: `${DIR}/Hikes.html`,
  calories: `${DIR}/Calories.html`,
};

type WClass = "base" | "worn" | "consumable";

// Functional categories, in display order.
const CATEGORIES = [
  "Shelter",
  "Sleep",
  "Storage",
  "Kitchen",
  "Clothing",
  "Electronics",
  "Safety",
  "Tools & Repair",
  "Health & Hygiene",
];

// Sheet slot (col A) → [category, default weight class]. Worn/consumable are
// excluded from base weight and become trip-specific.
const SLOT_MAP: Record<string, [string, WClass]> = {
  shelter: ["Shelter", "base"],
  stakes: ["Shelter", "base"],
  "shock cords": ["Shelter", "base"],
  "sleeping pad": ["Sleep", "base"],
  "sleeping bag": ["Sleep", "base"],
  pillow: ["Sleep", "base"],
  backpack: ["Storage", "base"],
  "pack liner": ["Storage", "base"],
  "food storage": ["Storage", "base"],
  "stuff sack": ["Storage", "base"],
  ziplocs: ["Storage", "base"],
  stove: ["Kitchen", "base"],
  pot: ["Kitchen", "base"],
  "stove + pot": ["Kitchen", "base"],
  spork: ["Kitchen", "base"],
  "water filter": ["Kitchen", "base"],
  "water bottles": ["Kitchen", "base"],
  "rain jacket": ["Clothing", "base"],
  windbreaker: ["Clothing", "base"],
  "mid layer": ["Clothing", "base"],
  underwear: ["Clothing", "base"],
  socks: ["Clothing", "base"],
  "running cap": ["Clothing", "base"],
  beanie: ["Clothing", "base"],
  "head net": ["Clothing", "base"],
  phone: ["Electronics", "base"],
  digicam: ["Electronics", "base"],
  book: ["Electronics", "base"],
  headlamp: ["Electronics", "base"],
  "sat messenger": ["Electronics", "base"],
  whistle: ["Safety", "base"],
  "fire starter": ["Safety", "base"],
  matches: ["Safety", "base"],
  "swiss army knife": ["Tools & Repair", "base"],
  trowel: ["Tools & Repair", "base"],
  "bamboo toothbrush": ["Health & Hygiene", "base"],
  toothpaste: ["Health & Hygiene", "base"],
  fuel: ["Kitchen", "consumable"],
  water: ["Kitchen", "consumable"],
  food: ["Kitchen", "consumable"],
  "lip balm": ["Health & Hygiene", "consumable"],
  "bug repellant": ["Health & Hygiene", "consumable"],
  "hand sanitizer": ["Health & Hygiene", "consumable"],
  sunhoodie: ["Clothing", "worn"],
  bandana: ["Clothing", "worn"],
  "hiking poles": ["Clothing", "worn"],
};

// Rows in col A that are group/rollup headers, not gear.
const SKIP = new Set([
  "Category",
  "Base",
  "Pack",
  "Skin-out",
  "Essentials",
  "Extras",
  "Clothes",
  "Toiletries",
  "Consumables",
  "Worn",
]);
const WISH_LEGEND = new Set(["Purchased", "High Priority", "Would Be Nice", "Wish List Item"]);
const KIT_SLOTS = new Set(["first aid", "repair kit"]);

// One sheet cell held two products with no separator. Split into real items
// (weights looked up online; to be re-weighed).
const SPLIT: Record<string, { name: string; weightG: number }[]> = {
  "Gossamer Gear TwinnKatabatic Gear Piñon Bivy": [
    { name: "Gossamer Gear Twinn", weightG: 332 },
    { name: "Katabatic Gear Piñon Bivy", weightG: 221 },
  ],
};

async function wipe() {
  const tables = [
    "trip_food",
    "trip_consumable",
    "trip_gear",
    "trip_day",
    "file",
    "trip",
    "trail",
    "food_item",
    "loadout_item",
    "loadout",
    "gear_component",
    "gear_item",
    "gear_category",
    "energy_params",
    "users",
  ];
  await db.execute(sql.raw(`TRUNCATE ${tables.join(", ")} RESTART IDENTITY CASCADE`));
}

async function makeCategories(userId: number): Promise<Map<string, number>> {
  const map = new Map<string, number>();
  for (let i = 0; i < CATEGORIES.length; i++) {
    const [c] = await db
      .insert(schema.gearCategory)
      .values({ userId, name: CATEGORIES[i], sortOrder: i })
      .returning({ id: schema.gearCategory.id });
    map.set(CATEGORIES[i], c.id);
  }
  return map;
}

async function importKit(
  path: string,
  name: string,
  categoryId: number | null,
  userId: number,
): Promise<number> {
  const rows = parseSheet(path);
  const { headerIdx } = headerMap(rows, "Item");
  const components: { name: string; weightG: number }[] = [];
  for (const r of rows.slice(headerIdx + 1)) {
    const itemName = r[1];
    if (!itemName || itemName === "Total Weight:") continue;
    const g = gramsFromGrams(r[2]);
    if (g == null) continue;
    components.push({ name: itemName, weightG: g });
  }
  const total = components.reduce((s, c) => s + c.weightG, 0);
  const [kit] = await db
    .insert(schema.gearItem)
    .values({ userId, categoryId, name, weightG: total, isKit: true, defaultWeightClass: "base" })
    .returning({ id: schema.gearItem.id });
  if (components.length)
    await db.insert(schema.gearComponent).values(components.map((c) => ({ gearItemId: kit.id, ...c })));
  return components.length;
}

async function importGear(userId: number, cats: Map<string, number>) {
  const rows = parseSheet(P.gear);
  const { headerIdx, col } = headerMap(rows, "Category");
  const nameCol = col("Category");
  const weightOzCol = col("Weight (oz)");
  const currentItemCol = col("Current Item");
  const wishCol = col("Wish List Item");
  const wishWeightCol = wishCol + 1;

  let retired = false;
  let items = 0;
  let wishItems = 0;

  for (const r of rows.slice(headerIdx + 1)) {
    const name = r[nameCol]?.trim();

    const wish = r[wishCol]?.trim();
    if (wish && !WISH_LEGEND.has(wish) && !SKIP.has(name ?? "") && name !== "Removed Gear") {
      await db.insert(schema.gearItem).values({
        userId,
        name: wish,
        weightG: toGrams(r[wishWeightCol]),
        status: "wishlist",
        defaultWeightClass: "base",
      });
      wishItems++;
    }

    if (!name || SKIP.has(name)) continue;
    if (name === "Removed Gear") {
      retired = true;
      continue;
    }

    const weightG = toGrams(r[weightOzCol]);
    if (weightG == null) continue;
    const key = name.toLowerCase();
    if (KIT_SLOTS.has(key)) continue; // kits imported separately

    const mapped = SLOT_MAP[key];
    const cls: WClass = mapped ? mapped[1] : "base";
    if (cls === "consumable") continue; // consumables are trip-specific, not library gear
    const categoryId = mapped ? (cats.get(mapped[0]) ?? null) : null;
    const product = r[currentItemCol]?.trim();
    const status = retired ? "retired" : "current";

    const split = product ? SPLIT[product] : undefined;
    if (split) {
      for (const s of split) {
        await db.insert(schema.gearItem).values({
          userId,
          categoryId,
          name: s.name,
          weightG: s.weightG,
          defaultWeightClass: cls,
          status,
        });
        items++;
      }
      continue;
    }

    await db.insert(schema.gearItem).values({
      userId,
      categoryId,
      name: product && product !== "" ? product : name,
      weightG,
      defaultWeightClass: cls,
      status,
    });
    items++;
  }
  return { items, wishItems };
}

// A default loadout of all current base/worn gear, using each item's default class.
async function seedDefaultLoadout(userId: number): Promise<number> {
  const [lo] = await db
    .insert(schema.loadout)
    .values({ userId, name: "Standard Kit", isDefault: true })
    .returning({ id: schema.loadout.id });
  const items = await db
    .select({ id: schema.gearItem.id, cls: schema.gearItem.defaultWeightClass })
    .from(schema.gearItem)
    .where(and(eq(schema.gearItem.userId, userId), eq(schema.gearItem.status, "current")));
  const members = items.filter((i) => i.cls === "base" || i.cls === "worn");
  if (members.length)
    await db
      .insert(schema.loadoutItem)
      .values(members.map((i) => ({ loadoutId: lo.id, gearItemId: i.id, weightClass: i.cls as "base" | "worn" })));
  return members.length;
}

async function importFood(userId: number) {
  const rows = parseSheet(P.meals);
  const { headerIdx } = headerMap(rows, "Food");
  let n = 0;
  for (const r of rows.slice(headerIdx + 1)) {
    const name = r[1]?.trim();
    const calPerOz = num(r[2]);
    if (!name || calPerOz == null || name === "Total:") continue;
    await db.insert(schema.ingredient).values({
      userId,
      name,
      kcalPer100g: String((calPerOz / OZ_TO_G) * 100),
      defaultServingG: toGrams(r[3]),
      source: "manual",
    });
    n++;
  }
  return n;
}

function parseCompleted(s: string | undefined): string | null {
  if (!s) return null;
  const m = s.match(/(\d{1,2})\/(\d{2})/);
  if (!m) return null;
  return `20${m[2]}-${m[1].padStart(2, "0")}-01`;
}

async function importHikes(userId: number) {
  const rows = parseSheet(P.hikes);
  const { headerIdx } = headerMap(rows, "Trail");
  const trips: { name: string; id: number }[] = [];
  let trails = 0;
  for (const r of rows.slice(headerIdx + 1)) {
    const name = r[1]?.trim();
    if (!name) continue;
    const [tr] = await db
      .insert(schema.trail)
      .values({
        userId,
        name,
        region: r[2] || null,
        areaType: r[3] || null,
        typicalDistanceMi: num(r[5]) != null ? String(num(r[5])) : null,
        typicalElevationFt: num(r[6]) != null ? Math.round(num(r[6])!) : null,
        notes: r[9] || null,
      })
      .returning({ id: schema.trail.id });
    trails++;

    const raw = r[8]?.trim() ?? "";
    const date = parseCompleted(raw);
    const status: "idea" | "planned" | "completed" = /planned/i.test(raw)
      ? "planned"
      : date
        ? "completed"
        : "idea";
    const [t] = await db
      .insert(schema.trip)
      .values({
        userId,
        trailId: tr.id,
        name,
        status,
        startDate: date,
        nights: num(r[4]) != null ? Math.round(num(r[4])!) : null,
      })
      .returning({ id: schema.trip.id });
    trips.push({ name, id: t.id });
  }
  return { trails, trips };
}

async function importCalories(trips: { name: string; id: number }[]) {
  const rows = parseSheet(P.calories);
  const { headerIdx } = headerMap(rows, "Miles (mi)");
  let matched = 0;
  let days = 0;
  let current: number | null = null;
  let dayNum = 0;
  for (const r of rows.slice(headerIdx + 1)) {
    const label = r[1]?.trim();
    if (!label) continue;
    if (/^Day\s*\d+/i.test(label)) {
      if (current == null) continue;
      dayNum++;
      const dist = num(r[2]);
      const elev = num(r[3]);
      if (dist == null && elev == null) continue;
      await db.insert(schema.tripDay).values({
        tripId: current,
        dayNumber: dayNum,
        distanceMi: dist != null ? String(dist) : null,
        elevationGainFt: elev != null ? Math.round(elev) : null,
      });
      days++;
      continue;
    }
    const base = label.replace(/\s*\(.*\)\s*$/, "").trim();
    const hit = trips.find(
      (t) =>
        t.name.toLowerCase() === base.toLowerCase() ||
        t.name.toLowerCase().includes(base.toLowerCase()),
    );
    current = hit?.id ?? null;
    dayNum = 0;
    if (hit) matched++;
  }
  return { matched, days };
}

async function main() {
  await wipe();
  const [u] = await db
    .insert(schema.users)
    .values({ email: "me@backpack.local", name: "Me" })
    .returning({ id: schema.users.id });
  await db.insert(schema.energyParams).values({ userId: u.id });

  const cats = await makeCategories(u.id);
  const fa = await importKit(P.firstAid, "First Aid", cats.get("Health & Hygiene")!, u.id);
  const rk = await importKit(P.repair, "Repair Kit", cats.get("Tools & Repair")!, u.id);
  const gear = await importGear(u.id, cats);
  await db.execute(
    sql.raw(
      `UPDATE gear_item SET sort_order = sub.rn FROM
       (SELECT id, row_number() OVER (PARTITION BY category_id ORDER BY id) rn FROM gear_item) sub
       WHERE gear_item.id = sub.id`,
    ),
  );
  const loadoutItems = await seedDefaultLoadout(u.id);
  const food = await importFood(u.id);
  const hikes = await importHikes(u.id);
  const cal = await importCalories(hikes.trips);

  console.log("Import complete:");
  console.log(`  categories:  ${cats.size}`);
  console.log(`  loadout:     Standard Kit (${loadoutItems} items)`);
  console.log(`  kits:        First Aid (${fa}), Repair Kit (${rk})`);
  console.log(`  gear items:  ${gear.items} current/retired, ${gear.wishItems} wishlist`);
  console.log(`  food items:  ${food}`);
  console.log(`  trails:      ${hikes.trails}`);
  console.log(`  trips:       ${hikes.trips.length}`);
  console.log(`  trip days:   ${cal.days} (${cal.matched} matched)`);
  await pool.end();
}

main().catch(async (e) => {
  console.error(e);
  await pool.end();
  process.exit(1);
});
