import "server-only";
import { sql } from "drizzle-orm";
import { propagateGearItem, type Tx } from "@/lib/trip-propagation";
export async function recomputeKitWeight(tx: Tx, id: number) {
  await tx.execute(sql`UPDATE gear_item SET weight_g=(SELECT COALESCE(SUM(weight_g * quantity),0)::integer FROM gear_component WHERE gear_item_id=${id}) WHERE id=${id} AND is_kit=true`);
  await propagateGearItem(tx, id);
}
