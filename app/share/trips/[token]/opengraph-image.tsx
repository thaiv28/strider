import { eq } from "drizzle-orm";
import { notFound } from "next/navigation";
import { socialImage, socialImageContentType, socialImageSize } from "@/lib/social-image";
import { db, schema } from "@/src/db/index";

export const size = socialImageSize;
export const contentType = socialImageContentType;
export const alt = "Shared Strider trip plan";
export const dynamic = "force-dynamic";

export default async function OpenGraphImage({ params }: { params: { token: string } }) {
  const { token } = params;
  const [trip] = await db
    .select({ name: schema.trip.name })
    .from(schema.trip)
    .where(eq(schema.trip.shareToken, token));
  if (!trip) notFound();

  const title = trip.name.length > 55 ? `${trip.name.slice(0, 54)}…` : trip.name;
  return socialImage(title, "View this backpacking trip on Strider");
}
