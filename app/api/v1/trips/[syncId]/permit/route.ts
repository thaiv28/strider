import { createHash } from "node:crypto";
import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { db, schema } from "@/src/db/index";
import { tripAccess } from "@/lib/authorization";
import { requireDevice } from "@/lib/sync/device";
import { apiError, apiJson } from "@/lib/sync/http";
export const dynamic = "force-dynamic";
type Context = { params: Promise<{ syncId: string }> };
const MAX_BYTES = 15 * 1024 * 1024;
const TYPES = new Set(["application/pdf", "image/png", "image/jpeg", "image/webp", "image/heic"]);
async function resolveTrip(syncId: string) {
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(syncId)) return null;
  return (await db.select().from(schema.trip).where(eq(schema.trip.syncId, syncId)))[0] ?? null;
}
export async function GET(request: Request, context: Context) {
  try {
    const { userId } = await requireDevice(request);
    const trip = await resolveTrip((await context.params).syncId);
    if (!trip) return apiJson({ error: "not_found" }, 404);
    let visible = trip.userId === userId;
    if (!visible) {
      const visits = await db.select().from(schema.tripShareVisit).where(and(eq(schema.tripShareVisit.tripId, trip.id), eq(schema.tripShareVisit.userId, userId)));
      visible = visits.some((v) => v.permission === "edit" ? !!trip.editToken : !!trip.shareToken);
    }
    if (!visible) return apiJson({ error: "not_found" }, 404);
    const [permit] = await db.select().from(schema.tripPermit).where(eq(schema.tripPermit.tripId, trip.id));
    if (!permit) return apiJson({ error: "not_found" }, 404);
    return new Response(new Uint8Array(permit.data), { headers: { "Cache-Control": "no-store", "Content-Type": permit.mimeType, "Content-Disposition": `attachment; filename*=UTF-8''${encodeURIComponent(permit.filename)}`, "X-Filename": encodeURIComponent(permit.filename) } });
  } catch (error) { return apiError(error); }
}
export async function PUT(request: Request, context: Context) {
  try {
    const { userId } = await requireDevice(request);
    const trip = await resolveTrip((await context.params).syncId);
    if (!trip || !await tripAccess(trip.id, userId)) return apiJson({ error: "not_found" }, 404);
    const mimeType = request.headers.get("Content-Type") ?? "";
    const filename = request.headers.get("X-Filename")?.trim();
    if (!TYPES.has(mimeType) || !filename || filename.length > 255 || /[\r\n]/.test(filename)) return apiJson({ error: "invalid_file" }, 400);
    if (Number(request.headers.get("Content-Length")) > MAX_BYTES) return apiJson({ error: "file_too_large" }, 413);
    const reader = request.body?.getReader();
    if (!reader) return apiJson({ error: "invalid_file" }, 400);
    const chunks: Uint8Array[] = []; let size = 0;
    while (true) {
      const chunk = await reader.read(); if (chunk.done) break;
      size += chunk.value.byteLength;
      if (size > MAX_BYTES) { await reader.cancel(); return apiJson({ error: "file_too_large" }, 413); }
      chunks.push(chunk.value);
    }
    if (!size) return apiJson({ error: "invalid_file" }, 400);
    const data = Buffer.concat(chunks);
    const sha256 = createHash("sha256").update(data).digest("hex");
    const row = { tripId: trip.id, filename, mimeType, sizeBytes: size, data, sha256, uploadedAt: new Date() };
    await db.insert(schema.tripPermit).values(row).onConflictDoUpdate({ target: schema.tripPermit.tripId, set: row });
    for (const path of [`/trips/${trip.id}`, "/trips", "/calendar", "/basecamp"]) revalidatePath(path);
    return apiJson({ sha256 });
  } catch (error) { return apiError(error); }
}
