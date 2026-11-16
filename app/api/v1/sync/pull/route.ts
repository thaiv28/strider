import { requireDevice } from "@/lib/sync/device";
import { apiJson, apiError } from "@/lib/sync/http";
import { pull } from "@/lib/sync/pull";
export const dynamic = "force-dynamic";
export async function GET(request: Request) {
  try {
    const { userId } = await requireDevice(request);
    const since = new URL(request.url).searchParams.get("since") ?? "0";
    if (!/^\d+$/.test(since) || BigInt(since) > 9223372036854775807n) return apiJson({ error: "Invalid cursor" },400);
    return apiJson(await pull(userId,BigInt(since)));
  } catch(error) { return apiError(error); }
}
