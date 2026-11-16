import { createHash, randomBytes, randomUUID } from "node:crypto";
import { createServer } from "node:http";
import { once } from "node:events";
import type { AddressInfo } from "node:net";
import { expect, test as base, type APIRequestContext, type BrowserContext, type Page } from "@playwright/test";
import { eq, sql } from "drizzle-orm";
import { db, schema } from "../src/db/index";
import { authenticate, e2ePassword } from "./auth";

const version = { "X-Strider-App-Version": "0.1.0" };
type Row = { table: string; sync_id: string; server_seq: string; [column: string]: unknown };
type Pull = { rows: Row[]; tombstones: Row[]; revoked_trips: string[]; cursor: string; more: boolean };
type Op = { op: "upsert" | "delete"; table: string; sync_id: string; set?: Record<string, unknown>; base_text?: Record<string, string | null> };
type Device = { token: string; page: Page; context: BrowserContext; email: string };
const upsert = (table: string, sync_id: string, set: Record<string, unknown>): Op => ({ op: "upsert", table, sync_id, set });
const headers = (token: string) => ({ ...version, Authorization: `Bearer ${token}` });

async function authorizationCode(page: Page) {
  const verifier = randomBytes(32).toString("base64url");
  const challenge = createHash("sha256").update(verifier).digest("base64url");
  const state = randomUUID();
  let resolveCode!: (code: string) => void;
  let captureTimer: ReturnType<typeof setTimeout>;
  const captured = new Promise<string>((resolve, reject) => {
    resolveCode = resolve;
    captureTimer = setTimeout(() => reject(new Error("Desktop authorization did not reach the loopback callback")), 20_000);
  });
  // Attach immediately so a slow authorize render cannot create an unhandled rejection.
  void captured.catch(() => undefined);
  const listener = createServer((request, response) => {
    const url = new URL(request.url!, "http://127.0.0.1");
    response.end("You can close this window.");
    if (url.pathname === "/callback" && url.searchParams.get("state") === state) {
      resolveCode(url.searchParams.get("code") ?? "");
    }
  });
  listener.listen(0, "127.0.0.1");
  await once(listener, "listening");
  const port = (listener.address() as AddressInfo).port;
  try {
    const query = new URLSearchParams({
      code_challenge: challenge, code_challenge_method: "S256", state,
      redirect_uri: `http://127.0.0.1:${port}/callback`, device_name: "API integration test",
    });
    await page.goto(`/desktop/authorize?${query}`);
    await expect(page.getByRole("heading", { name: "Allow Strider Desktop" })).toBeVisible();
    const [code] = await Promise.all([captured, page.getByRole("button", { name: "Allow", exact: true }).click()]);
    expect(code).not.toBe("");
    return { code, code_verifier: verifier, device_name: "API integration test" };
  } finally {
    clearTimeout(captureTimer!);
    listener.closeAllConnections();
    await new Promise<void>((resolve, reject) => listener.close((error) => error ? reject(error) : resolve()));
  }
}

async function exchange(request: APIRequestContext, input: Awaited<ReturnType<typeof authorizationCode>>) {
  return request.post("/api/v1/auth/token", { headers: version, data: input });
}
async function push(request: APIRequestContext, device: Device, ops: Op[], batch_id: string = randomUUID()) {
  return request.post("/api/v1/sync/push", { headers: headers(device.token), data: { batch_id, ops } });
}
async function applied(request: APIRequestContext, device: Device, ops: Op[], batchId?: string) {
  const response = await push(request, device, ops, batchId);
  expect(response.status(), await response.text()).toBe(200);
  return response.json();
}
async function pull(request: APIRequestContext, device: Device, since = "0"): Promise<Pull> {
  const response = await request.get(`/api/v1/sync/pull?since=${since}`, { headers: headers(device.token) });
  expect(response.status(), await response.text()).toBe(200);
  expect(response.headers()["cache-control"]).toBe("no-store");
  return response.json();
}

// Use fresh accounts on each attempt so retries cannot inherit grants or batches.
// Delete roots in FK order; their dependent rows cascade. Tombstones have no FK.
async function cleanUser(email: string) {
  await db.transaction(async (tx) => {
    const [user] = await tx.select().from(schema.users).where(eq(schema.users.email, email));
    if (!user) return;
    for (const table of ["file", "trip", "meal", "ingredient", "loadout", "gear_item", "calendar_feed", "report_settings", "energy_params", "trail"]) {
      await tx.execute(sql`DELETE FROM ${sql.identifier(table)} WHERE user_id = ${user.id}`);
    }
    await tx.execute(sql`UPDATE gear_category SET parent_id = NULL WHERE user_id = ${user.id}`);
    await tx.execute(sql`DELETE FROM gear_category WHERE user_id = ${user.id}`);
    await tx.delete(schema.tripShareVisit).where(eq(schema.tripShareVisit.userId, user.id));
    await tx.execute(sql`DELETE FROM sync_tombstone WHERE owner_user_id = ${user.id}`);
    await tx.delete(schema.users).where(eq(schema.users.id, user.id));
  });
}

const test = base.extend<{ owner: Device; other: Device }>({
  owner: async ({ browser, request, baseURL }, use) => {
    base.skip(!e2ePassword, "E2E_TEST_PASSWORD is required");
    const email = `api-a-${randomUUID()}@strider.invalid`;
    const context = await browser.newContext({ baseURL });
    const page = await context.newPage();
    try {
      await authenticate(page, email);
      const response = await exchange(request, await authorizationCode(page));
      expect(response.status()).toBe(200);
      await use({ email, context, page, token: (await response.json()).token });
    } finally {
      try { await cleanUser(email); } finally { await context.close(); }
    }
  },
  other: async ({ browser, request, baseURL }, use) => {
    base.skip(!e2ePassword, "E2E_TEST_PASSWORD is required");
    const email = `api-b-${randomUUID()}@strider.invalid`;
    const context = await browser.newContext({ baseURL });
    const page = await context.newPage();
    try {
      await authenticate(page, email);
      const response = await exchange(request, await authorizationCode(page));
      expect(response.status()).toBe(200);
      await use({ email, context, page, token: (await response.json()).token });
    } finally {
      try { await cleanUser(email); } finally { await context.close(); }
    }
  },
});
test.setTimeout(120_000);

test("every protected v1 route rejects a missing device token", async ({ request }) => {
  const trip = randomUUID();
  const routes = [
    ["DELETE", "/auth/token"], ["GET", "/me"], ["GET", "/sync/pull?since=0"],
    ["POST", "/sync/push"], ["GET", `/trips/${trip}/permit`], ["PUT", `/trips/${trip}/permit`],
    ["POST", `/trips/${trip}/share`], ["GET", "/food/search?q=oats"], ["GET", "/geocode?q=Seattle"],
  ];
  for (const [method, path] of routes) {
    const response = await request.fetch(`/api/v1${path}`, { method, headers: version });
    expect(response.status(), `${method} ${path}`).toBe(401);
    expect(response.headers()["cache-control"]).toBe("no-store");
  }
});

test("PKCE rejects wrong verifiers, reused codes, and expired codes", async ({ request, owner }) => {
  const input = await authorizationCode(owner.page);
  const wrong = await exchange(request, { ...input, code_verifier: randomBytes(32).toString("base64url") });
  expect(wrong.status()).toBe(400);
  const valid = await exchange(request, input);
  expect(valid.status()).toBe(200);
  const missingVersion = await request.get("/api/v1/me", { headers: { Authorization: `Bearer ${owner.token}` } });
  expect(missingVersion.status()).toBe(426);
  const oldVersion = await request.get("/api/v1/me", { headers: { Authorization: `Bearer ${owner.token}`, "X-Strider-App-Version": "0.0.9" } });
  expect(oldVersion.status()).toBe(426);
  expect((await exchange(request, input)).status()).toBe(400);
  const expired = await authorizationCode(owner.page);
  await db.execute(sql`UPDATE device_auth_code SET expires_at = now() - interval '1 second'
    WHERE code_hash = ${createHash("sha256").update(expired.code).digest("hex")}`);
  expect((await exchange(request, expired)).status()).toBe(400);
});

test("push resolves same-batch foreign keys and derives kit weight", async ({ request, owner }) => {
  const category = randomUUID(), kit = randomUUID(), a = randomUUID(), b = randomUUID();
  await applied(request, owner, [
    upsert("gear_category", category, { name: "API equipment" }),
    upsert("gear_item", kit, { name: "API repair kit", category_id: category, is_kit: true }),
    upsert("gear_component", a, { gear_item_id: kit, name: "Tape", weight_g: 12, quantity: 2 }),
    upsert("gear_component", b, { gear_item_id: kit, name: "Needle", weight_g: 3, quantity: 1 }),
  ]);
  const result = await pull(request, owner);
  expect(result.rows.find((row) => row.sync_id === kit)).toMatchObject({ table: "gear_item", category_id: category, weight_g: 27 });
  expect(result.rows.find((row) => row.sync_id === a)).toMatchObject({ table: "gear_component", gear_item_id: kit });
  for (const row of result.rows) expect(row).not.toHaveProperty("user_id");
});

test("private rows never leak across users and an unauthorized batch rolls back", async ({ request, owner, other }) => {
  const item = randomUUID(), rolledBack = randomUUID();
  await applied(request, owner, [upsert("gear_item", item, { name: "Owner private item", weight_g: 10 })]);
  const rejected = await push(request, other, [upsert("gear_item", rolledBack, { name: "Must roll back" }), upsert("gear_item", item, { name: "Stolen" })]);
  expect(rejected.status()).toBe(409);
  expect(await rejected.json()).toMatchObject({ rejected: { index: 1 } });
  const result = await pull(request, other);
  expect(result.rows.some((row) => row.sync_id === item || row.sync_id === rolledBack)).toBe(false);
});

test("a batch replay returns the stored result and writes once", async ({ request, owner }) => {
  const item = randomUUID(), batch = randomUUID();
  const ops = [upsert("gear_item", item, { name: "Idempotent item", weight_g: 5 })];
  const first = await applied(request, owner, ops, batch);
  const before = await pull(request, owner);
  expect(await applied(request, owner, ops, batch)).toEqual(first);
  const after = await pull(request, owner, before.cursor);
  expect(after.rows).toEqual([]);
  const count = await db.execute<{ count: string }>(sql`SELECT count(*) FROM gear_item WHERE sync_id = ${item}::uuid`);
  expect(Number(count.rows[0].count)).toBe(1);
});

test("web gear edits advance the cursor and propagate trip snapshots", async ({ request, owner }) => {
  const gear = randomUUID(), trip = randomUUID(), tripGear = randomUUID();
  const name = `API web edit ${randomUUID()}`;
  await applied(request, owner, [
    upsert("gear_item", gear, { name, weight_g: 28 }),
    upsert("trip", trip, { name: "API snapshot trip" }),
    upsert("trip_gear", tripGear, { trip_id: trip, gear_item_id: gear }),
  ]);
  const before = await pull(request, owner);
  await owner.page.goto("/gear");
  await owner.page.getByRole("button", { name: `More options for ${name}`, exact: true }).click();
  await owner.page.getByRole("button", { name: "Edit", exact: true }).click();
  await owner.page.getByLabel("Name", { exact: true }).fill(`${name} changed`);
  await owner.page.getByLabel("Unit (oz)", { exact: true }).fill("2");
  await Promise.all([
    owner.page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
    owner.page.getByRole("button", { name: "Save", exact: true }).click(),
  ]);
  const after = await pull(request, owner, before.cursor);
  expect(after.rows.find((row) => row.sync_id === gear)).toMatchObject({ name: `${name} changed`, weight_g: 57 });
  expect(after.rows.find((row) => row.sync_id === tripGear)).toMatchObject({ snapshot_name: `${name} changed`, snapshot_weight_g: 57 });
});

test("delete emits a root tombstone without tombstones for cascaded children", async ({ request, owner }) => {
  const gear = randomUUID(), component = randomUUID();
  await applied(request, owner, [
    upsert("gear_item", gear, { name: "Deleted kit", is_kit: true }),
    upsert("gear_component", component, { gear_item_id: gear, name: "Deleted component", weight_g: 1 }),
  ]);
  const before = await pull(request, owner);
  await applied(request, owner, [{ op: "delete", table: "gear_item", sync_id: gear }]);
  const after = await pull(request, owner, before.cursor);
  expect(after.tombstones).toEqual(expect.arrayContaining([expect.objectContaining({ table: "gear_item", sync_id: gear })]));
  expect(after.tombstones.some((row) => row.sync_id === component)).toBe(false);
});

test("edit grants hydrate old trip rows, hide reports, and revoke writes", async ({ request, owner, other }) => {
  const trip = randomUUID(), oldDay = randomUUID(), newDay = randomUUID();
  await applied(request, owner, [
    upsert("trip", trip, { name: "Shared API trip", trip_report: "Owner-only report" }),
    upsert("trip_day", oldDay, { trip_id: trip, day_number: 1, distance_mi: "4.5" }),
  ]);
  // The cursor is newer than the trip and its children: only grant hydration
  // can make the existing trip appear in the next pull.
  await applied(request, other, [upsert("gear_item", randomUUID(), { name: "Cursor marker" })]);
  const before = await pull(request, other);
  const share = await request.post(`/api/v1/trips/${trip}/share`, { headers: headers(owner.token), data: { kind: "edit", action: "create" } });
  expect(share.status()).toBe(200);
  const tripRow = (await db.execute<{ edit_token: string }>(sql`SELECT edit_token FROM trip WHERE sync_id = ${trip}::uuid`)).rows[0];
  await other.page.goto(`/share/trips/${tripRow.edit_token}`);
  const shared = await pull(request, other, before.cursor);
  expect(shared.rows.find((row) => row.sync_id === trip)).toMatchObject({ access: "edit", name: "Shared API trip" });
  expect(shared.rows.find((row) => row.sync_id === trip)).not.toHaveProperty("trip_report");
  expect(shared.rows.find((row) => row.sync_id === trip)).not.toHaveProperty("edit_token");
  expect(shared.rows.find((row) => row.sync_id === oldDay)).toMatchObject({ trip_id: trip, distance_mi: "4.5" });
  await applied(request, other, [upsert("trip_day", newDay, { trip_id: trip, day_number: 2 })]);
  const revoke = await request.post(`/api/v1/trips/${trip}/share`, { headers: headers(owner.token), data: { kind: "edit", action: "revoke" } });
  expect(revoke.status()).toBe(200);
  expect((await pull(request, other, shared.cursor)).revoked_trips).toContain(trip);
  expect((await push(request, other, [upsert("trip_day", newDay, { distance_mi: "7" })])).status()).toBe(409);
});

test("long text conflicts return the replaced server text", async ({ request, owner }) => {
  const trip = randomUUID();
  await applied(request, owner, [upsert("trip", trip, { name: "Conflict trip", planning_notes: "Original" })]);
  await applied(request, owner, [upsert("trip", trip, { planning_notes: "Server edit" })]);
  const result = await applied(request, owner, [{ ...upsert("trip", trip, { planning_notes: "Device edit" }), base_text: { planning_notes: "Original" } }]);
  expect(result.replaced).toContainEqual({ table: "trip", sync_id: trip, column: "planning_notes", value: "Server edit" });
  expect((await pull(request, owner)).rows.find((row) => row.sync_id === trip)).toMatchObject({ planning_notes: "Device edit" });
});

test("duplicate trip gear adopts the existing server sync id", async ({ request, owner }) => {
  const gear = randomUUID(), trip = randomUUID(), existing = randomUUID(), duplicate = randomUUID();
  await applied(request, owner, [
    upsert("gear_item", gear, { name: "Duplicate gear", weight_g: 42 }),
    upsert("trip", trip, { name: "Duplicate trip" }),
    upsert("trip_gear", existing, { trip_id: trip, gear_item_id: gear }),
  ]);
  const result = await applied(request, owner, [upsert("trip_gear", duplicate, { trip_id: trip, gear_item_id: gear })]);
  expect(result.adopt).toContainEqual({ table: "trip_gear", from: duplicate, to: existing });
  const rows = (await pull(request, owner)).rows.filter((row) => row.table === "trip_gear" && row.trip_id === trip);
  expect(rows).toHaveLength(1);
  expect(rows[0].sync_id).toBe(existing);
});

test("dated rows remain editable and invalid dates and route points roll back", async ({ request, owner }) => {
  const trip = randomUUID(), gear = randomUUID();
  await applied(request, owner, [
    upsert("trip", trip, { name: "Dated trip", start_date: "2028-02-29" }),
    upsert("gear_item", gear, { name: "Dated gear", acquired_at: "2026-11-16" }),
  ]);
  await applied(request, owner, [
    upsert("trip", trip, { planning_notes: "Date unchanged" }),
    upsert("gear_item", gear, { notes: "Date unchanged" }),
  ]);
  const result = await pull(request, owner);
  expect(result.rows.find(row => row.sync_id === trip)).toMatchObject({ start_date: "2028-02-29", planning_notes: "Date unchanged" });
  expect(result.rows.find(row => row.sync_id === gear)).toMatchObject({ acquired_at: "2026-11-16", notes: "Date unchanged" });
  for (const op of [
    upsert("trip", trip, { start_date: "2026-02-30" }),
    upsert("trip_gpx", randomUUID(), { trip_id: trip, points: [{ d: 0, lat: 999, lon: 0, e: 100 }] }),
    upsert("gear_item", gear, { server_seq: "999999" }),
    upsert("trip", trip, { share_token: "device-controlled" }),
  ]) expect((await push(request, owner, [op])).status()).toBe(409);
  expect((await pull(request, owner, result.cursor)).rows).toEqual([]);
});

test("permit bytes stay outside pull and only editors can upload", async ({ request, owner, other }) => {
  const trip = randomUUID();
  await applied(request, owner, [upsert("trip", trip, { name: "Permit trip" })]);
  const data = Buffer.from("%PDF-1.4\nAPI permit fixture\n%%EOF");
  const upload = (device: Device) => request.put(`/api/v1/trips/${trip}/permit`, {
    headers: { ...headers(device.token), "Content-Type": "application/pdf", "X-Filename": "permit.pdf" }, data,
  });
  expect((await upload(other)).status()).toBe(404);
  const response = await upload(owner);
  expect(response.status()).toBe(200);
  const sha256 = createHash("sha256").update(data).digest("hex");
  expect(await response.json()).toEqual({ sha256 });
  const permit = (await pull(request, owner)).rows.find(row => row.table === "trip_permit");
  expect(permit).toMatchObject({ trip_id: trip, sha256, filename: "permit.pdf" });
  expect(permit).not.toHaveProperty("data");
  const downloaded = await request.get(`/api/v1/trips/${trip}/permit`, { headers: headers(owner.token) });
  expect(downloaded.status()).toBe(200);
  expect(await downloaded.body()).toEqual(data);
  expect(downloaded.headers()["cache-control"]).toBe("no-store");
});

test("device sessions store hashes and revocation rejects subsequent requests", async ({ request, owner }) => {
  const sessions = await db.select().from(schema.deviceSession)
    .where(eq(schema.deviceSession.tokenHash, createHash("sha256").update(owner.token).digest("hex")));
  expect(sessions).toHaveLength(1);
  expect(sessions[0].tokenHash).not.toBe(owner.token);
  await owner.page.goto("/settings");
  await expect(owner.page.getByText("API integration test", { exact: true })).toBeVisible();
  await Promise.all([
    owner.page.waitForResponse(response => response.request().method() === "POST" && response.ok()),
    owner.page.getByRole("button", { name: "Revoke", exact: true }).click(),
  ]);
  expect((await request.get("/api/v1/me", { headers: headers(owner.token) })).status()).toBe(401);
});
