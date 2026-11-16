import "server-only";

export const MIN_APP_VERSION = "0.1.0";
export function apiJson(value: unknown, status = 200) {
  return Response.json(value, { status, headers: { "Cache-Control": "no-store" } });
}
export function requireAppVersion(request: Request) {
  const version = request.headers.get("X-Strider-App-Version");
  const match = version?.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match || (Number(match[1]) === 0 && Number(match[2]) < 1)) {
    throw apiJson({ error: "app_update_required", min_app_version: MIN_APP_VERSION }, 426);
  }
}
export function apiError(error: unknown) {
  if (error instanceof Response) {
    error.headers.set("Cache-Control", "no-store");
    return error;
  }
  console.error("Desktop API request failed", error instanceof Error ? error.name : "Unknown error");
  return apiJson({ error: "internal_error" }, 500);
}
