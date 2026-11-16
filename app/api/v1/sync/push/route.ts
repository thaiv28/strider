import { requireDevice } from "@/lib/sync/device";
import { apiJson, apiError } from "@/lib/sync/http";
import { push, PushRejected } from "@/lib/sync/push";
export const dynamic = "force-dynamic";
export async function POST(request: Request) {
  try {
    const {userId} = await requireDevice(request);
    let batch;
    try { batch = await request.json(); } catch { return apiJson({error:"Invalid JSON"},400); }
    return apiJson(await push(userId,batch));
  } catch(error) {
    if(error instanceof PushRejected) return apiJson({rejected:{index:error.index,reason:error.reason}},409);
    return apiError(error);
  }
}
