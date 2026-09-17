import { eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";

export const dynamic = "force-dynamic";

// Serve the stored permit inline so it renders in the print view and browser tab.
export async function GET(_req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db
    .select()
    .from(schema.tripPermit)
    .where(eq(schema.tripPermit.tripId, Number(id)));
  if (!row) return new Response("Not found", { status: 404 });

  return new Response(new Uint8Array(row.data), {
    headers: {
      "Content-Type": row.mimeType,
      "Content-Disposition": `inline; filename="${row.filename.replace(/"/g, "")}"`,
      "Cache-Control": "private, no-store",
    },
  });
}
