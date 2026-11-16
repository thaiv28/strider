import { and, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { changeTripShareLink } from "@/lib/trip-sharing";
import { requireDevice } from "@/lib/sync/device";
import { apiError, apiJson } from "@/lib/sync/http";
export const dynamic = "force-dynamic";
export async function POST(request: Request, context: { params: Promise<{ syncId: string }> }) {
  try {
    const { userId } = await requireDevice(request);
    const { syncId } = await context.params;
    if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(syncId)) return apiJson({ error: "not_found" }, 404);
    const body = await request.json().catch(() => null);
    if (!body || !["view", "edit"].includes(body.kind) || !["create", "rotate", "revoke"].includes(body.action)) return apiJson({ error: "invalid_request" }, 400);
    const [trip] = await db.select({ id: schema.trip.id }).from(schema.trip).where(and(eq(schema.trip.syncId, syncId), eq(schema.trip.userId, userId)));
    if (!trip) return apiJson({ error: "not_found" }, 404);
    const token = await changeTripShareLink(userId, trip.id, body.kind, body.action);
    return apiJson({ token });
  } catch (error) { return apiError(error); }
}
