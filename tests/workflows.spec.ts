import { expect, test, type Page } from "@playwright/test";
import { authenticate, e2ePassword } from "./auth";

const baseURL =
  process.env.PLAYWRIGHT_BASE_URL ??
  process.env.MOBILE_TEST_BASE_URL ??
  "http://127.0.0.1:3000";
async function fillAndSave(page: Page, label: string, value: string) {
  const input = page.getByLabel(label, { exact: true });
  await input.fill(value);
  await Promise.all([
    page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
    input.press("Tab"),
  ]);
}

// Production verification must never leave its uniquely-prefixed records
// behind, even when the main test times out and its page fixture is closed.
test.afterAll(async ({ browser }) => {
  if (!e2ePassword) return;
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await authenticate(page);
    for (let i = 0; i < 25; i++) {
      await page.goto("/trips");
      const trip = page.locator('a[href^="/trips/"]').filter({ hasText: /^E2E (?:trip|verified) / }).first();
      if (!(await trip.count())) break;
      await trip.click();
      const button = page.getByRole("button", { name: "Delete trip" });
      page.once("dialog", (dialog) => dialog.accept());
      await Promise.all([page.waitForURL((url) => url.pathname === "/trips"), button.click()]);
    }
  } finally {
    await context.close();
  }
});

test("protected pages redirect to sign in", async ({ browser }) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto(`${baseURL}/`);
  await expect(page.getByRole("heading", { name: /Plan farther/ })).toBeVisible();
  await expect(page).toHaveURL(`${baseURL}/`);
  await page.goto(`${baseURL}/privacy`);
  await expect(page.getByRole("heading", { name: "Privacy Policy", exact: true })).toBeVisible();
  await page.goto(`${baseURL}/trips`);
  await expect.poll(() => new URL(page.url()).pathname).toBe("/login");
  await expect(page.getByRole("heading", { name: "STRIDER" })).toBeVisible();
  await context.close();
});

test("private trips are isolated between accounts", async ({ browser }) => {
  const owner = await browser.newContext();
  const ownerPage = await owner.newPage();
  const outsider = await browser.newContext();
  const outsiderPage = await outsider.newPage();
  const name = `E2E trip isolation ${Date.now()}`;
  let tripUrl: string | null = null;

  try {
    await authenticate(ownerPage);
    await ownerPage.goto("/trips");
    ownerPage.once("dialog", (dialog) => dialog.accept(name));
    await Promise.all([
      ownerPage.waitForURL(/\/trips\/\d+$/),
      ownerPage.getByRole("button", { name: "+ New trip" }).click(),
    ]);
    tripUrl = ownerPage.url();

    await authenticate(outsiderPage, "e2e-outsider@strider.invalid");
    const response = await outsiderPage.goto(tripUrl);
    expect(response?.status()).toBe(404);
    await expect(outsiderPage.getByText(name, { exact: true })).toHaveCount(0);
  } finally {
    if (tripUrl) {
      await ownerPage.goto(tripUrl);
      const button = ownerPage.getByRole("button", { name: "Delete trip" });
      if (await button.isVisible().catch(() => false)) {
        ownerPage.once("dialog", (dialog) => dialog.accept());
        await Promise.all([ownerPage.waitForURL((url) => url.pathname === "/trips"), button.click()]);
      }
    }
    await owner.close();
    await outsider.close();
  }
});

test("a trip can be planned, shared, exported, and deleted", async ({ browser, context, page }) => {
  // Production route-map processing and the unauthenticated share round-trip can
  // be slow on the small ARM instance, especially immediately after deployment.
  test.setTimeout(180_000);
  await authenticate(page);

  const suffix = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
  const initialName = `E2E trip ${suffix}`;
  const updatedName = `E2E verified ${suffix}`;
  let tripUrl: string | null = null;

  const deleteCreatedTrip = async () => {
    if (!tripUrl) return;
    await page.goto(tripUrl);
    const button = page.getByRole("button", { name: "Delete trip" });
    if (!(await button.isVisible().catch(() => false))) return;
    page.once("dialog", (dialog) => dialog.accept());
    await Promise.all([
      page.waitForURL((url) => url.pathname === "/trips"),
      button.click(),
    ]);
    tripUrl = null;
  };

  try {
    await page.goto("/trips");
    page.once("dialog", (dialog) => dialog.accept(initialName));
    await Promise.all([
      page.waitForURL(/\/trips\/\d+$/),
      page.getByRole("button", { name: "+ New trip" }).click(),
    ]);
    tripUrl = page.url();

    await fillAndSave(page, "Trip name", updatedName);
    await fillAndSave(page, "Region", "E2E Test Range");
    await fillAndSave(page, "Area type", "Test wilderness");

    await page.getByRole("button", { name: "+ Add day" }).click();
    await expect(page.getByLabel("Day 1 distance in miles")).toBeVisible();
    await fillAndSave(page, "Day 1 distance in miles", "4.5");
    await fillAndSave(page, "Day 1 elevation gain in feet", "1200");

    await page.getByRole("button", { name: "Share", exact: true }).click();
    const printHref = await page
      .getByRole("menuitem", { name: /Print trip/ })
      .getAttribute("href");
    expect(printHref).toMatch(/^\/trips\/\d+\/print$/);
    await page.goto(printHref!);
    await expect(page.getByRole("heading", { name: updatedName })).toBeVisible();
    await page.goto(tripUrl);

    const gpx = `<?xml version="1.0" encoding="UTF-8"?>
      <gpx version="1.1"><trk><name>E2E route</name><trkseg>
        <trkpt lat="47.0000" lon="-121.0000"><ele>1000</ele></trkpt>
        <trkpt lat="47.0100" lon="-121.0100"><ele>1010</ele></trkpt>
        <trkpt lat="47.0200" lon="-121.0200"><ele>1020</ele></trkpt>
      </trkseg></trk></gpx>`;
    await page.locator('input[name="gpx"]').setInputFiles({
      name: "workflow.gpx",
      mimeType: "application/gpx+xml",
      buffer: Buffer.from(gpx),
    });
    await page.getByRole("button", { name: "Upload", exact: true }).click();
    await expect(page.getByRole("button", { name: "Use these for the trip" })).toBeVisible();

    await page.getByRole("button", { name: "Planning", exact: true }).click();
    const permitInput = page.locator('input[type="file"][accept^="application/pdf"]');
    await permitInput.setInputFiles({
      name: "permit.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("%PDF-1.4\n% Strider workflow test\n%%EOF\n"),
    });
    await expect(page.getByRole("link", { name: "permit.pdf" })).toBeVisible();

    await page.reload();
    await expect(page.getByLabel("Trip name")).toHaveValue(updatedName);
    await page.getByRole("button", { name: "Logistics", exact: true }).click();
    await expect(page.getByLabel("Region", { exact: true })).toHaveValue("E2E Test Range");
    await expect(page.getByLabel("Day 1 distance in miles")).toHaveValue(/4\.5/);
    await expect(page.getByRole("button", { name: "Use these for the trip" })).toBeVisible();

    await page.getByRole("button", { name: "Share", exact: true }).click();
    await page.getByRole("menuitem", { name: /Create view-only link/ }).click();
    const sharedHref = await page.getByRole("menuitem", { name: "Open view-only page" }).getAttribute("href");
    expect(sharedHref).toMatch(/^\/share\/trips\/[A-Za-z0-9_-]+$/);

    const guest = await browser.newContext();
    const sharedPage = await guest.newPage();
    const sharedResponse = await sharedPage.goto(`${baseURL}${sharedHref}`);
    expect(sharedResponse?.status()).toBe(200);
    await expect(sharedPage.getByTestId("shared-trip")).toBeVisible();
    await expect(sharedPage.getByRole("heading", { name: updatedName })).toBeVisible();
    await expect(sharedPage.getByText("Trip report (private notes)")).toHaveCount(0);
    await expect(sharedPage.getByRole("link", { name: "permit.pdf" })).toBeVisible();
    await guest.close();

    await page.getByRole("menuitem", { name: "Revoke view-only link" }).click();
    await page.getByRole("menuitem", { name: "Confirm revoke" }).click();
    await expect(page.getByRole("menuitem", { name: /Create view-only link/ })).toBeVisible();
    const revoked = await page.request.get(sharedHref!);
    expect(revoked.status()).toBe(404);

    await deleteCreatedTrip();
    await expect(page.getByText(updatedName, { exact: true })).toHaveCount(0);
  } finally {
    await deleteCreatedTrip();
  }
});
