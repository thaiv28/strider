import "server-only";
import { getTableColumns, sql } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { db } from "@/src/db/index";
import { propagateGearItem, propagateIngredient, propagateMeal, type Tx } from "@/lib/trip-propagation";
import { renumberDays, deriveTripFromDays, syncDaysFromCampsites } from "@/lib/trip-derived";
import { recomputeKitWeight } from "@/lib/gear-kit";
import { VOLUME_ML } from "@/lib/food";
import { registry, ident, type Entry, type SyncRow } from "./registry";
import { canWriteRow, tripPermission } from "./access";
export type Operation = { op: "upsert" | "delete"; table: string; sync_id: string; base_seq?: string; set?: SyncRow; base_text?: Record<string, string | null> };
export type Batch = { batch_id: string; ops: Operation[] };
class InvalidPush extends Error {}
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const texts = new Set(["planning_notes", "trip_report", "packing_list", "notes", "template", "packing_default", "permit_notes", "driving_notes", "water_sources"]);
export class PushRejected extends Error {
  constructor(public index: number, public reason: string) { super(reason); }
}
async function find(tx: Tx, table: string, syncId: string) {
  const row = (await tx.execute<SyncRow>(sql`SELECT * FROM ${sql.raw(ident(table))} WHERE sync_id=${syncId}`)).rows[0];
  // Raw pg queries parse SQL dates as local Dates, unlike Drizzle's date codec.
  // Normalize before validating a merged row, including unchanged date fields.
  if (!row) return row;
  // Drop columns the schema no longer declares (e.g. the legacy trip.companions)
  // so they are never validated, written, or echoed back.
  const known = new Set(Object.values(getTableColumns(registry[table].table)).map((column) => column.name));
  for (const key of Object.keys(row)) if (!known.has(key)) delete row[key];
  for (const column of Object.values(getTableColumns(registry[table].table))) {
    const value = row[column.name];
    if (column.columnType === "PgDateString" && value instanceof Date) {
      row[column.name] = `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
    }
  }
  return row;
}
function validate(e: Entry, values: SyncRow, insert: boolean) {
  const columns = Object.values(getTableColumns(e.table));
  for (const [key, value] of Object.entries(values)) {
    const c = columns.find(c => c.name === key);
    if (!c) throw new InvalidPush(`Invalid column ${key}`);
    if (value === null) { if (c.notNull) throw new InvalidPush(`Required ${key}`); continue; }
    if (key === "name" && (typeof value !== "string" || !value.trim())) throw new InvalidPush("Name is required");
    if (c.enumValues?.length && !c.enumValues.includes(String(value))) throw new InvalidPush(`Invalid ${key}`);
    if (c.dataType === "number" && (!Number.isSafeInteger(value) || Number(value) < -2147483648 || Number(value) > 2147483647)) throw new InvalidPush(`Invalid ${key}`);
    if (c.dataType === "boolean" && typeof value !== "boolean") throw new InvalidPush(`Invalid ${key}`);
    if (c.dataType === "string" && typeof value !== "string") throw new InvalidPush(`Invalid ${key}`);
    if (c.columnType === "PgNumeric" && (typeof value !== "string" || !/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(value) || !Number.isFinite(Number(value)))) throw new InvalidPush(`Invalid ${key}`);
    if ((key.endsWith("_g") || key.endsWith("_cents") || ["grams", "food_g_per_day", "water_g_per_day", "fuel_g_per_day", "entered_amount", "kcal_per_100g", "density_g_ml", "water_ml", "nights", "distance_mi"].includes(key)) && Number(value) < 0) throw new InvalidPush(`Negative ${key}`);
    if (["quantity", "base_servings", "servings", "party_size", "grocery_people", "day_number", "ft_per_energy_mile"].includes(key) && Number(value) < 1) throw new InvalidPush(`Invalid ${key}`);
    if (c.columnType === "PgDateString" && (!/^\d{4}-\d{2}-\d{2}$/.test(String(value)) || Number.isNaN(Date.parse(String(value))) || new Date(String(value)).toISOString().slice(0, 10) !== value)) throw new InvalidPush(`Invalid ${key}`);
    if (["lat", "start_lat"].includes(key) && Math.abs(Number(value)) > 90) throw new InvalidPush(`Invalid ${key}`);
    if (["lon", "start_lon"].includes(key) && Math.abs(Number(value)) > 180) throw new InvalidPush(`Invalid ${key}`);
    if (c.dataType === "json") validateJson(key, value);
  }
  if (insert) for (const c of columns) if (c.notNull && !c.hasDefault && c.name !== "data" && values[c.name] == null) throw new InvalidPush(`Required ${c.name}`);
  if (e.name === "trip_meal" && Boolean(values.meal_id) === Boolean(values.ingredient_id)) throw new InvalidPush("Exactly one food reference required");
  if (values.entered_unit != null && !["g", "oz", ...Object.keys(VOLUME_ML)].includes(String(values.entered_unit))) throw new InvalidPush("Invalid entered_unit");
  if (values.density_source != null && !["usda", "label", "manual"].includes(String(values.density_source))) throw new InvalidPush("Invalid density_source");
  if (values.sex != null && !["male", "female"].includes(String(values.sex))) throw new InvalidPush("Invalid sex");
}
function validateJson(key: string, value: unknown) {
  if (!Array.isArray(value)) throw new InvalidPush(`Invalid ${key}`);
  const finite = (v: unknown) => typeof v === "number" && Number.isFinite(v);
  const point = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
  const valid = value.every(v => {
    if (key === "grocery_removed") return typeof v === "string";
    if (key === "track") return Array.isArray(v) && v.length === 2 && finite(v[0]) && finite(v[1]) && Math.abs(v[0]) <= 90 && Math.abs(v[1]) <= 180;
    if (!point(v) || !finite(v.d) || Number(v.d) < 0 || !finite(v.e)) return false;
    return key === "profile" || (key === "points" && finite(v.lat) && finite(v.lon) && Math.abs(Number(v.lat)) <= 90 && Math.abs(Number(v.lon)) <= 180);
  });
  if (!valid) throw new InvalidPush(`Invalid ${key}`);
}
async function defaults(tx: Tx, e: Entry, values: SyncRow, insert: boolean) {
  if (e.name === "trip_gear" && insert && values.gear_item_id != null) {
    const item = (await tx.execute<SyncRow>(sql`SELECT g.*, c.name AS category FROM gear_item g LEFT JOIN gear_category c ON c.id=g.category_id WHERE g.id=${values.gear_item_id}`)).rows[0];
    values.snapshot_name = item.name; values.snapshot_weight_g = item.weight_g ?? 0; values.snapshot_category = item.category;
  }
  if (e.name === "meal_ingredient" && values.ingredient_id != null) {
    const item = (await tx.execute<SyncRow>(sql`SELECT * FROM ingredient WHERE id=${values.ingredient_id}`)).rows[0];
    values.snapshot_name = item.name; values.snapshot_kcal_per_100g = item.kcal_per_100g == null ? values.snapshot_kcal_per_100g ?? null : String(Math.round(Number(item.kcal_per_100g))); values.snapshot_density_g_ml = item.density_g_ml;
    if (values.entered_amount != null && values.entered_unit != null && item.density_g_ml != null && Object.hasOwn(VOLUME_ML, String(values.entered_unit)))
      values.amount_g = Math.round(Number(values.entered_amount) * VOLUME_ML[String(values.entered_unit) as keyof typeof VOLUME_ML] * Number(item.density_g_ml));
  }
  if (e.name === "trip_meal") {
    if (values.meal_id != null) {
      const item = (await tx.execute<SyncRow>(sql`SELECT m.name,m.base_servings,COALESCE(SUM(mi.amount_g),0) AS g,
        COALESCE(SUM(ROUND(COALESCE(mi.snapshot_kcal_per_100g,0)/100*mi.amount_g)),0) AS kcal FROM meal m
        LEFT JOIN meal_ingredient mi ON mi.meal_id=m.id WHERE m.id=${values.meal_id} GROUP BY m.id`)).rows[0];
      const scale = Number(values.servings ?? 1)/Number(item.base_servings);
      values.snapshot_name=item.name; values.snapshot_weight_g=Math.round(Number(item.g)*scale); values.snapshot_kcal=Math.round(Number(item.kcal)*scale);
    } else if (values.ingredient_id != null) {
      const item = (await tx.execute<SyncRow>(sql`SELECT * FROM ingredient WHERE id=${values.ingredient_id}`)).rows[0];
      values.snapshot_name=item.name; values.snapshot_weight_g ??= 0;
      values.snapshot_kcal=item.kcal_per_100g == null ? values.snapshot_kcal ?? 0 : Math.round(Number(item.kcal_per_100g)/100*Number(values.snapshot_weight_g));
    }
  }
}
const duplicateKeys: Record<string, string[]> = { trip_gear: ["trip_id", "gear_item_id"], loadout_item: ["loadout_id", "gear_item_id"], wishlist_replacement: ["wishlist_item_id", "replaces_item_id"], wishlist_loadout: ["wishlist_item_id", "loadout_id"] };
export async function push(userId: number, batch: Batch) {
  if (!batch || !uuid.test(batch.batch_id) || !Array.isArray(batch.ops) || batch.ops.length > 500) throw new PushRejected(0,"Invalid batch");
  let index = 0;
  try {
    const result = await db.transaction(async tx => {
      await tx.execute(sql`SELECT pg_advisory_xact_lock(7645912384021::bigint)`);
      const replay = await tx.execute<{ result: PushResult }>(sql`SELECT result FROM sync_batch WHERE user_id=${userId} AND batch_id=${batch.batch_id}`);
      if (replay.rows[0]) return replay.rows[0].result;
      const result: PushResult = { applied: 0, cursor_hint: "0", adopt: [], replaced: [] };
      const kits = new Set<number>(); const meals = new Set<number>(); const trips = new Set<number>();
      const dayTrips = new Set<number>(); const campsiteTrips = new Set<number>(); const renumberTrips = new Set<number>();
      for (index=0; index<batch.ops.length; index++) {
        const op = batch.ops[index]; const e = Object.hasOwn(registry,op?.table) ? registry[op.table] : undefined;
        if (!e || !uuid.test(op.sync_id) || !["upsert","delete"].includes(op.op)) throw new InvalidPush("Invalid operation or table");
        if (op.base_seq != null && (typeof op.base_seq !== "string" || !/^\d+$/.test(op.base_seq) || BigInt(op.base_seq) > 9223372036854775807n)) throw new InvalidPush("Invalid base_seq");
        if (op.base_text != null && (typeof op.base_text !== "object" || Array.isArray(op.base_text) || Object.entries(op.base_text).some(([key, value]) => !texts.has(key) || (value !== null && typeof value !== "string")))) throw new InvalidPush("Invalid base_text");
        const old = await find(tx,e.name,op.sync_id);
        if (old && !await canWriteRow(userId,e,old,tx)) throw new InvalidPush("Ownership failure");
        if (op.op === "delete") {
          if (!old) continue;
          if (e.name === "trip" && await tripPermission(userId,Number(old.id),tx) !== "owner") throw new InvalidPush("Owner only");
          await tx.execute(sql`DELETE FROM ${sql.raw(ident(e.name))} WHERE sync_id=${op.sync_id}`);
        } else {
          if (op.set != null && (typeof op.set !== "object" || Array.isArray(op.set))) throw new InvalidPush("Invalid set");
          const patch: SyncRow = { ...op.set };
          for (const key of Object.keys(patch)) {
            if (!e.writable.has(key) || (old && e.insertOnly.has(key)) || (old && e.name === "meal_ingredient" && key === "amount_g" && Object.hasOwn(VOLUME_ML,String(patch.entered_unit ?? old.entered_unit)))) throw new InvalidPush(`Disallowed column ${key}`);
            if (e.foreignKeys[key] && patch[key] != null) {
              if (typeof patch[key] !== "string" || !uuid.test(String(patch[key]))) throw new InvalidPush("Invalid foreign key");
              const ref = await find(tx,e.foreignKeys[key],String(patch[key]));
              if (!ref) throw new InvalidPush("Unresolvable foreign key");
              // Library references must belong to the caller; trip grants never grant access to an owner's library.
              if (!await canWriteRow(userId,e.foreignKeys[key],ref,tx)) throw new InvalidPush("Foreign key ownership failure");
              patch[key]=ref.id;
            }
          }
          const values: SyncRow = { ...old, ...patch, sync_id: op.sync_id };
          if (!old && e.scope === "user") values.user_id=userId;
          if (e.name !== "trip" && !await canWriteRow(userId,e,values,tx)) throw new InvalidPush("Ownership failure");
          if (e.name === "trip" && old && await tripPermission(userId,Number(old.id),tx) !== "owner" && Object.hasOwn(patch,"trip_report")) throw new InvalidPush("Owner only");
          if (e.name === "trip_permit" && !old) throw new InvalidPush("Use permit upload endpoint");
          await defaults(tx,e,values,!old);
          validate(e,values,!old);
          const keys=duplicateKeys[e.name];
          if (!old && keys && keys.every(k => values[k] != null)) {
            const duplicate=await tx.execute<{sync_id:string}>(sql`SELECT sync_id FROM ${sql.raw(ident(e.name))} WHERE ${sql.join(keys.map(k=>sql`${sql.raw(ident(k))}=${values[k]}`),sql` AND `)}`);
            if (duplicate.rows[0]) { result.adopt.push({table:e.name,from:op.sync_id,to:duplicate.rows[0].sync_id}); continue; }
          }
          for (const [column,base] of Object.entries(op.base_text ?? {})) if (old && texts.has(column) && Object.hasOwn(patch,column) && old[column] !== base)
            result.replaced.push({table:e.name,sync_id:op.sync_id,column,value:old[column]});
          const written = old ? Object.fromEntries(Object.keys(values).filter(k=>Object.hasOwn(patch,k) || (k.startsWith("snapshot_") && values[k] !== old[k]) || (k === "amount_g" && values[k] !== old[k])).map(k=>[k,values[k]])) : values;
          if (old && Object.keys(written).length && ["gear_item", "trip"].includes(e.name)) written.updated_at = new Date();
          const entries=Object.entries(written);
          if (old && entries.length) await tx.execute(sql`UPDATE ${sql.raw(ident(e.name))} SET ${sql.join(entries.map(([k,v])=>sql`${sql.raw(ident(k))}=${parameter(e,k,v)}`),sql`,`)} WHERE sync_id=${op.sync_id}`);
          if (!old) await tx.execute(sql`INSERT INTO ${sql.raw(ident(e.name))} (${sql.raw(entries.map(([k])=>ident(k)).join(","))}) VALUES (${sql.join(entries.map(([k,v])=>parameter(e,k,v)),sql`,`)})`);
        }
        const row = op.op === "delete" ? old : await find(tx,e.name,op.sync_id);
        if (!row) continue;
        result.applied++;
        if (e.name === "gear_item" && op.op !== "delete") { await propagateGearItem(tx,Number(row.id)); if(row.is_kit) kits.add(Number(row.id)); }
        if (e.name === "gear_component") { kits.add(Number(row.gear_item_id)); if (old) kits.add(Number(old.gear_item_id)); }
        if (e.name === "ingredient" && op.op !== "delete") await propagateIngredient(tx,Number(row.id));
        if (e.name === "meal" && op.op !== "delete") meals.add(Number(row.id));
        if (e.name === "meal_ingredient") { meals.add(Number(row.meal_id)); if (old) meals.add(Number(old.meal_id)); }
        if (e.name === "trip_day") { dayTrips.add(Number(row.trip_id)); if(old?.trip_id) dayTrips.add(Number(old.trip_id)); if(op.op === "delete") renumberTrips.add(Number(row.trip_id)); }
        if (e.name === "trip_campsite") { campsiteTrips.add(Number(row.trip_id)); if(old?.trip_id) campsiteTrips.add(Number(old.trip_id)); }
        if (row.trip_id != null) trips.add(Number(row.trip_id));
        if (old?.trip_id != null) trips.add(Number(old.trip_id));
        if (e.name === "trip" && op.op !== "delete" && Object.hasOwn(op.set ?? {},"nights")) trips.add(Number(row.id));
      }
      for(const id of renumberTrips) await renumberDays(id,tx);
      for(const id of campsiteTrips) await syncDaysFromCampsites(id,tx);
      for(const id of dayTrips) if(!campsiteTrips.has(id)) await deriveTripFromDays(id,tx);
      for(const id of kits) await recomputeKitWeight(tx,id);
      for(const id of meals) await propagateMeal(tx,id);
      for(const id of trips) await tx.execute(sql`UPDATE trip t SET food_g_per_day=(SELECT ROUND(COALESCE(SUM(snapshot_weight_g),0)::numeric/GREATEST(COALESCE(t.nights,0)+1,1))::integer FROM trip_meal WHERE trip_id=t.id) WHERE t.id=${id}`);
      const seq=await tx.execute<{last_value:string;is_called:boolean}>(sql`SELECT last_value::text,is_called FROM sync_seq`);
      result.cursor_hint=seq.rows[0].is_called ? seq.rows[0].last_value : "0";
      await tx.execute(sql`INSERT INTO sync_batch(user_id,batch_id,result) VALUES (${userId},${batch.batch_id},${JSON.stringify(result)}::jsonb)`);
      return result;
    });
    revalidatePath("/", "layout");
    return result;
  } catch(error) {
    if (error instanceof PushRejected) throw error;
    // Database errors can contain private values; expose only a stable rejection reason.
    const reason = error instanceof InvalidPush ? error.message : "Invalid value or unique-key clash";
    throw new PushRejected(index,reason);
  }
}
function parameter(e: Entry,key:string,value:unknown) {
  const c=Object.values(getTableColumns(e.table)).find(c=>c.name===key);
  return c?.dataType === "json" && value != null ? sql`${JSON.stringify(value)}::jsonb` : sql`${value}`;
}
export type PushResult = { applied:number; cursor_hint:string; adopt:{table:string;from:string;to:string}[]; replaced:{table:string;sync_id:string;column:string;value:unknown}[] };
