import { requireDevice } from "@/lib/sync/device";
import { apiError, apiJson } from "@/lib/sync/http";
export const dynamic = "force-dynamic";
export async function GET(request: Request) {
  try { await requireDevice(request); return apiJson({ error: "not_implemented" }, 501); }
  catch (error) { return apiError(error); }
}
