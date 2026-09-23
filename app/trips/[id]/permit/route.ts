import { and, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { tripAccess } from "@/lib/authorization";

export const dynamic = "force-dynamic";

// Serve the stored permit inline so it renders in the print view and browser tab.
export async function GET(_req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const userId = await getCurrentUserId();
  const access = await tripAccess(Number(id), userId);
  if (!access) return new Response("Not found", { status: 404 });
  const [row] = await db
    .select({
      filename: schema.tripPermit.filename,
      mimeType: schema.tripPermit.mimeType,
      data: schema.tripPermit.data,
    })
    .from(schema.tripPermit)
    .innerJoin(schema.trip, eq(schema.tripPermit.tripId, schema.trip.id))
    .where(and(eq(schema.tripPermit.tripId, Number(id)), eq(schema.trip.userId, access.ownerId)));
  if (!row) return new Response("Not found", { status: 404 });

  return new Response(new Uint8Array(row.data), {
    headers: {
      "Content-Type": row.mimeType,
      "Content-Disposition": `inline; filename="${row.filename.replace(/"/g, "")}"`,
      "Cache-Control": "private, no-store",
    },
  });
}
