import "server-only";
import { getTableColumns, sql } from "drizzle-orm";
import { db } from "@/src/db/index";
import { registry, ident, type Entry, type SyncRow } from "./registry";
import { visibleTripIds, visibility, tripPermission, type Reader } from "./access";

async function encode(reader: Reader, e: Entry, row: SyncRow, userId: number) {
  const out: SyncRow = { table: e.name };
  const access = e.name === "trip" ? await tripPermission(userId, Number(row.id), reader) : null;
  const columns = Object.values(getTableColumns(e.table));
  for (const [key, value] of Object.entries(row)) {
    if (key === "id" || key === "user_id" || key === "data") continue;
    if (access !== "owner" && e.hidden.has(key)) continue;
    if (e.foreignKeys[key] && value != null) {
      const fk = await reader.execute<{ sync_id: string }>(sql`SELECT sync_id FROM ${sql.raw(ident(e.foreignKeys[key]))} WHERE id=${value}`);
      out[key] = fk.rows[0]?.sync_id ?? null;
    } else out[key] = value instanceof Date ? (columns.find(c => c.name === key)?.columnType === "PgDateString" ? `${value.getFullYear()}-${String(value.getMonth()+1).padStart(2,"0")}-${String(value.getDate()).padStart(2,"0")}` : value.toISOString()) : typeof value === "bigint" ? value.toString() : value;
  }
  if (access) out.access = access;
  return out;
}
export async function pull(userId: number, since: bigint, limit = 1000) {
  limit = Math.min(1000, Math.max(1, Math.floor(limit)));
  return db.transaction(async tx => {
    const trips = await visibleTripIds(userId, tx);
    type Event = { seq: bigint; entry?: Entry; row: SyncRow; kind: "row" | "tombstone" | "grant" };
    const events: Event[] = [];
    for (const e of Object.values(registry)) {
      // Explicit selection keeps permit bytea out of the query result entirely.
      const columns = Object.values(getTableColumns(e.table)).filter(c => c.name !== "data");
      const result = await tx.execute<SyncRow>(sql`SELECT ${sql.raw(columns.map(c => `r.${ident(c.name)}`).join(","))} FROM ${sql.raw(ident(e.name))} r
        WHERE ${visibility(e, userId, trips)} AND r.server_seq > ${since.toString()} ORDER BY r.server_seq LIMIT ${limit + 1}`);
      events.push(...result.rows.map(row => ({ seq: BigInt(String(row.server_seq)), entry: e, row, kind: "row" as const })));
    }
    const tombstones = await tx.execute<SyncRow>(sql`SELECT * FROM sync_tombstone WHERE server_seq > ${since.toString()} AND
      (owner_user_id=${userId} OR (tbl <> 'trip_share_visit' AND trip_id IN (${sql.join((trips.length ? trips : [-1]).map(id => sql`${id}`), sql`,`)}))) ORDER BY server_seq LIMIT ${limit + 1}`);
    events.push(...tombstones.rows.map(row => ({ seq: BigInt(String(row.server_seq)), row, kind: "tombstone" as const })));
    const grants = await tx.execute<SyncRow>(sql`SELECT * FROM trip_share_visit WHERE user_id=${userId} AND server_seq > ${since.toString()}
      AND trip_id IN (${sql.join((trips.length ? trips : [-1]).map(id => sql`${id}`), sql`,`)}) ORDER BY server_seq LIMIT ${limit + 1}`);
    events.push(...grants.rows.map(row => ({ seq: BigInt(String(row.server_seq)), row, kind: "grant" as const })));
    events.sort((a,b) => a.seq < b.seq ? -1 : a.seq > b.seq ? 1 : 0);
    const selected = events.slice(0, limit);
    const cursor = selected.at(-1)?.seq ?? since;
    const rows: SyncRow[] = [];
    const deleted: SyncRow[] = [];
    for (const event of selected) {
      if (event.kind === "row") rows.push(await encode(tx, event.entry!, event.row, userId));
      if (event.kind === "tombstone" && event.row.tbl !== "trip_share_visit") deleted.push({ table: event.row.tbl, sync_id: event.row.sync_id, server_seq: String(event.row.server_seq) });
      if (event.kind === "grant") {
        for (const e of Object.values(registry).filter(e => e.name === "trip" || e.scope === "trip")) {
          const cols = Object.values(getTableColumns(e.table)).filter(c => c.name !== "data");
          const all = await tx.execute<SyncRow>(sql`SELECT ${sql.raw(cols.map(c => ident(c.name)).join(","))} FROM ${sql.raw(ident(e.name))} WHERE ${sql.raw(e.name === "trip" ? "id" : "trip_id")}=${event.row.trip_id}`);
          for (const row of all.rows) rows.push(await encode(tx, e, row, userId));
        }
      }
    }
    const revoked = await tx.execute<{ sync_id: string }>(sql`SELECT DISTINCT COALESCE(t.sync_id, d.sync_id) AS sync_id
      FROM sync_tombstone v LEFT JOIN trip t ON t.id=v.trip_id
      LEFT JOIN sync_tombstone d ON d.tbl='trip' AND d.trip_id=v.trip_id
      WHERE v.tbl='trip_share_visit' AND v.owner_user_id=${userId} AND v.server_seq > ${since.toString()}
      AND v.server_seq <= ${cursor.toString()} AND v.trip_id NOT IN (${sql.join((trips.length ? trips : [-1]).map(id => sql`${id}`), sql`,`)})`);
    return { rows: [...new Map(rows.map(r => [`${r.table}:${r.sync_id}`, r])).values()], tombstones: deleted,
      revoked_trips: revoked.rows.map(r => r.sync_id).filter(Boolean), cursor: cursor.toString(), more: events.length > limit };
  }, { isolationLevel: "repeatable read", accessMode: "read only" });
}
