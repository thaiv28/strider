import "server-only";
import { createHash } from "node:crypto";
import { and, eq, isNull, lt, or } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { apiJson, requireAppVersion } from "./http";

export const hashToken = (value: string) => createHash("sha256").update(value).digest("hex");
export async function requireDevice(request: Request): Promise<{ userId: number; sessionId: number }> {
  const match = request.headers.get("Authorization")?.match(/^Bearer ([A-Za-z0-9_-]{43})$/);
  if (!match) throw apiJson({ error: "unauthorized" }, 401);
  const [session] = await db.select().from(schema.deviceSession).where(and(
    eq(schema.deviceSession.tokenHash, hashToken(match[1])), isNull(schema.deviceSession.revokedAt),
  ));
  if (!session) throw apiJson({ error: "unauthorized" }, 401);
  requireAppVersion(request);
  const cutoff = new Date(Date.now() - 60_000);
  if (!session.lastUsedAt || session.lastUsedAt < cutoff) {
    await db.update(schema.deviceSession).set({ lastUsedAt: new Date() }).where(and(
      eq(schema.deviceSession.id, session.id), or(isNull(schema.deviceSession.lastUsedAt), lt(schema.deviceSession.lastUsedAt, cutoff)),
    ));
  }
  return { userId: session.userId, sessionId: session.id };
}
