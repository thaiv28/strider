import "server-only";
import { randomBytes } from "node:crypto";
import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { db, schema } from "@/src/db/index";

export async function changeTripShareLink(userId: number, tripId: number, kind: "view" | "edit", action: "create" | "rotate" | "revoke") {
  const token = await db.transaction(async (tx) => {
    const [trip] = await tx.select().from(schema.trip).where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId))).for("update");
    if (!trip) throw new Error("Trip not found");
    const existing = kind === "view" ? trip.shareToken : trip.editToken;
    if (action === "create" && existing) return existing;
    const next = action === "revoke" ? null : randomBytes(24).toString("base64url");
    await tx.update(schema.trip).set(kind === "view" ? { shareToken: next, updatedAt: new Date() } : { editToken: next, updatedAt: new Date() }).where(eq(schema.trip.id, tripId));
    if (action !== "create") await tx.delete(schema.tripShareVisit).where(and(eq(schema.tripShareVisit.tripId, tripId), eq(schema.tripShareVisit.permission, kind)));
    return next;
  });
  revalidatePath(`/trips/${tripId}`);
  if (action !== "create") revalidatePath("/trips");
  return token;
}
