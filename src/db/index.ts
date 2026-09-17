import "dotenv/config";
import { drizzle } from "drizzle-orm/node-postgres";
import pg from "pg";
import * as schema from "./schema";

// Reuse a single pool across HMR reloads in dev so we don't exhaust connections.
const g = globalThis as unknown as { __pgPool?: pg.Pool };
const pool = g.__pgPool ?? new pg.Pool({ connectionString: process.env.DATABASE_URL });
if (process.env.NODE_ENV !== "production") g.__pgPool = pool;

export const db = drizzle(pool, { schema });
export { pool, schema };
