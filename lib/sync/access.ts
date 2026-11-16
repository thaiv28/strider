import "server-only";
import { sql } from "drizzle-orm";
import { db } from "@/src/db/index";
import type { Tx } from "@/lib/trip-propagation";
import { registry, ident, type Entry, type SyncRow } from "./registry";
export type Reader = Pick<Tx, "execute">;
export async function visibleTripIds(userId: number, reader: Reader = db): Promise<number[]> {
  const result = await reader.execute<{ id: number }>(sql`SELECT t.id FROM trip t WHERE t.user_id = ${userId} OR EXISTS
    (SELECT 1 FROM trip_share_visit v WHERE v.trip_id=t.id AND v.user_id=${userId}
      AND ((v.permission='edit' AND t.edit_token IS NOT NULL) OR (v.permission='view' AND t.share_token IS NOT NULL)))`);
  return result.rows.map(r => r.id);
}
export async function tripPermission(userId: number, tripId: number, reader: Reader = db) {
  const result = await reader.execute<{ access: "owner" | "edit" | "view" }>(sql`SELECT CASE WHEN t.user_id=${userId} THEN 'owner'
    WHEN t.edit_token IS NOT NULL AND EXISTS(SELECT 1 FROM trip_share_visit v WHERE v.trip_id=t.id AND v.user_id=${userId} AND v.permission='edit') THEN 'edit'
    ELSE 'view' END AS access FROM trip t WHERE t.id=${tripId} AND (t.user_id=${userId} OR EXISTS
    (SELECT 1 FROM trip_share_visit v WHERE v.trip_id=t.id AND v.user_id=${userId} AND
    ((v.permission='edit' AND t.edit_token IS NOT NULL) OR (v.permission='view' AND t.share_token IS NOT NULL))))`);
  return result.rows[0]?.access ?? null;
}
export async function canWriteRow(userId: number, table: string | Entry, row: SyncRow, reader: Reader = db): Promise<boolean> {
  const e = typeof table === "string" ? registry[table] : table;
  if (!e) return false;
  if (e.name === "trip" || e.scope === "trip") {
    const access = await tripPermission(userId, Number(e.name === "trip" ? row.id : row.trip_id), reader);
    return access === "owner" || access === "edit";
  }
  if (e.scope === "user") return row.user_id === userId;
  const parent = e.scope.slice(7);
  const key = Object.keys(e.foreignKeys).find(k => e.foreignKeys[k] === parent)!;
  const result = await reader.execute(sql`SELECT 1 FROM ${sql.raw(ident(parent))} WHERE id=${row[key]} AND user_id=${userId}`);
  return result.rows.length > 0;
}
export function visibility(e: Entry, userId: number, trips: number[]) {
  if (e.name === "trip") return sql`r.id IN (${sql.join((trips.length ? trips : [-1]).map(id => sql`${id}`), sql`,`)})`;
  if (e.scope === "trip") return sql`r.trip_id IN (${sql.join((trips.length ? trips : [-1]).map(id => sql`${id}`), sql`,`)})`;
  if (e.scope === "user") return sql`r.user_id=${userId}`;
  const parent = e.scope.slice(7);
  const key = Object.keys(e.foreignKeys).find(k => e.foreignKeys[k] === parent)!;
  return sql`EXISTS (SELECT 1 FROM ${sql.raw(ident(parent))} p WHERE p.id=r.${sql.raw(ident(key))} AND p.user_id=${userId})`;
}
