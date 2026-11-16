import { searchFoods } from "@/lib/food-search";
import { searchReferenceFoods } from "@/lib/usda-reference";
import { requireDevice } from "@/lib/sync/device";
import { apiError, apiJson } from "@/lib/sync/http";
export const dynamic = "force-dynamic";
export async function GET(request: Request) {
  try {
    await requireDevice(request);
    const query = new URL(request.url).searchParams;
    const q = (query.get("q") ?? "").slice(0, 200);
    return apiJson(query.get("packaged") === "1" ? await searchFoods(q) : searchReferenceFoods(q));
  } catch (error) { return apiError(error); }
}
