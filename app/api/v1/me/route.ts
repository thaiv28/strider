import { eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { requireDevice } from "@/lib/sync/device";
import { apiError, apiJson, MIN_APP_VERSION } from "@/lib/sync/http";
export const dynamic = "force-dynamic";
export async function GET(request: Request) {
  try {
    const { userId } = await requireDevice(request);
    const [user] = await db.select({ email: schema.users.email, name: schema.users.name, image_url: schema.users.imageUrl }).from(schema.users).where(eq(schema.users.id, userId));
    return apiJson({ ...user, min_app_version: MIN_APP_VERSION, waqi_token: process.env.NEXT_PUBLIC_WAQI_TOKEN ?? null });
  } catch (error) { return apiError(error); }
}
