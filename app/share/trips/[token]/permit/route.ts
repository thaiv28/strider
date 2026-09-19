import { and, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";

export const dynamic = "force-dynamic";

export async function GET(_request: Request, { params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  const [row] = await db
    .select({ filename: schema.tripPermit.filename, mimeType: schema.tripPermit.mimeType, data: schema.tripPermit.data })
    .from(schema.tripPermit)
    .innerJoin(schema.trip, and(eq(schema.trip.id, schema.tripPermit.tripId), eq(schema.trip.shareToken, token)));
  if (!row) return new Response("Not found", { status: 404 });

  return new Response(new Uint8Array(row.data), {
    headers: {
      "Content-Type": row.mimeType,
      "Content-Disposition": `inline; filename="${row.filename.replace(/"/g, "")}"`,
      "Cache-Control": "private, no-store",
      "X-Robots-Tag": "noindex, nofollow, noarchive",
    },
  });
}
