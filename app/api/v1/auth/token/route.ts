import { createHash, randomBytes, timingSafeEqual } from "node:crypto";
import { and, eq, gt, isNull } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { db, schema } from "@/src/db/index";
import { hashToken, requireDevice } from "@/lib/sync/device";
import { apiError, apiJson, requireAppVersion } from "@/lib/sync/http";
export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  try {
    requireAppVersion(request);
    const body = await request.json().catch(() => null);
    if (!body || typeof body.code !== "string" || !/^[A-Za-z0-9_-]{43}$/.test(body.code) || typeof body.code_verifier !== "string" || !/^[A-Za-z0-9._~-]{43,128}$/.test(body.code_verifier) || typeof body.device_name !== "string" || !body.device_name.trim() || body.device_name.length > 60) return apiJson({ error: "invalid_grant" }, 400);
    const result = await db.transaction(async (tx) => {
      const [code] = await tx.select().from(schema.deviceAuthCode).where(and(eq(schema.deviceAuthCode.codeHash, hashToken(body.code)), isNull(schema.deviceAuthCode.usedAt), gt(schema.deviceAuthCode.expiresAt, new Date()))).for("update");
      const challenge = createHash("sha256").update(body.code_verifier).digest("base64url");
      if (!code || code.deviceName !== body.device_name || Buffer.byteLength(challenge) !== Buffer.byteLength(code.codeChallenge) || !timingSafeEqual(Buffer.from(challenge), Buffer.from(code.codeChallenge))) return null;
      await tx.update(schema.deviceAuthCode).set({ usedAt: new Date() }).where(eq(schema.deviceAuthCode.codeHash, code.codeHash));
      const token = randomBytes(32).toString("base64url");
      await tx.insert(schema.deviceSession).values({ userId: code.userId, deviceName: code.deviceName, tokenHash: hashToken(token) });
      const [user] = await tx.select({ email: schema.users.email, name: schema.users.name }).from(schema.users).where(eq(schema.users.id, code.userId));
      return { token, user };
    });
    if (result) revalidatePath("/settings");
    return result ? apiJson(result) : apiJson({ error: "invalid_grant" }, 400);
  } catch (error) { return apiError(error); }
}
export async function DELETE(request: Request) {
  try {
    const { sessionId } = await requireDevice(request);
    await db.update(schema.deviceSession).set({ revokedAt: new Date() }).where(eq(schema.deviceSession.id, sessionId));
    revalidatePath("/settings");
    return apiJson({ ok: true });
  } catch (error) { return apiError(error); }
}
