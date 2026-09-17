import { sql, eq, and } from "drizzle-orm";
import { db, pool, schema } from "../src/db/index";
import { OZ_TO_G } from "./parse";

const g2oz = (g: number) => (g / OZ_TO_G).toFixed(2);

async function count(table: string): Promise<number> {
  const r = await db.execute(sql.raw(`SELECT count(*)::int AS n FROM ${table}`));
  return (r.rows[0] as { n: number }).n;
}

async function classTotal(cls: "base" | "worn" | "consumable"): Promise<number> {
  const r = await db
    .select({ total: sql<number>`coalesce(sum(${schema.gearItem.weightG}), 0)::int` })
    .from(schema.gearItem)
    .where(and(eq(schema.gearItem.status, "current"), eq(schema.gearItem.defaultWeightClass, cls)));
  return r[0].total;
}

async function main() {
  console.log("Row counts:");
  for (const t of [
    "users",
    "gear_category",
    "gear_item",
    "gear_component",
    "food_item",
    "trail",
    "trip",
    "trip_day",
  ]) {
    console.log(`  ${t.padEnd(16)} ${await count(t)}`);
  }

  const base = await classTotal("base");
  const worn = await classTotal("worn");
  const cons = await classTotal("consumable");
  console.log("\nCurrent gear-library totals (defaults, no per-trip overrides):");
  console.log(`  base        ${g2oz(base)} oz`);
  console.log(`  worn        ${g2oz(worn)} oz`);
  console.log(`  consumable  ${g2oz(cons)} oz`);
  console.log(`  skin-out    ${g2oz(base + worn + cons)} oz   (sheet Base=194.76, Worn=15.8, Cons=94)`);

  console.log("\nTrips:");
  const trips = await db.select().from(schema.trip);
  for (const t of trips) {
    const days = await count(`trip_day WHERE trip_id = ${t.id}`);
    console.log(
      `  ${t.name.padEnd(24)} ${t.status.padEnd(10)} nights=${t.nights ?? "-"} days=${days}`,
    );
  }
  await pool.end();
}

main().catch(async (e) => {
  console.error(e);
  await pool.end();
  process.exit(1);
});
