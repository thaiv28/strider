import { getCurrentUserId, getGearView, getLoadoutData } from "@/lib/gear";
import { GearFrame } from "@/components/gear/gear-frame";

export const dynamic = "force-dynamic";
export const metadata = { title: "Gear" };

export default async function GearPage() {
  const userId = await getCurrentUserId();
  const [view, loadoutData] = await Promise.all([getGearView(userId), getLoadoutData(userId)]);
  return (
    <GearFrame
      groups={view.groups}
      wishlist={view.wishlist}
      categoryOptions={view.categoryOptions}
      loadouts={loadoutData.loadouts}
      membership={loadoutData.membership}
      wishlistOverrides={loadoutData.wishlistOverrides}
    />
  );
}
