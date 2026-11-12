import { expect, test, type Page } from "@playwright/test";
import { authenticate, e2ePassword } from "./auth";
import { densityFromServing, parseVolumeUnit } from "../lib/food";

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
      const more = page.getByRole("button", { name: "More trip options", exact: true });
      if (await more.isVisible().catch(() => false)) await more.click();
      const button = page.getByRole("menuitem", { name: "Delete trip", exact: true });
      page.once("dialog", (dialog) => dialog.accept());
      await Promise.all([page.waitForURL((url) => url.pathname === "/trips"), button.click()]);
    }
  } finally {
    await context.close();
  }
});

test("authentication endpoints use the configured public origin", async ({ request }) => {
  const response = await request.get(`${baseURL}/api/auth/providers`);
  expect(response.ok()).toBeTruthy();

  const providers = (await response.json()) as {
    google: { signinUrl: string; callbackUrl: string };
  };
  const expectedOrigin = new URL(process.env.AUTH_URL ?? baseURL).origin;

  expect(new URL(providers.google.signinUrl).origin).toBe(expectedOrigin);
  expect(new URL(providers.google.callbackUrl).origin).toBe(expectedOrigin);
  expect(new URL(providers.google.callbackUrl).pathname).toBe("/api/auth/callback/google");
});

test("protected pages redirect to sign in", async ({ browser }) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto(`${baseURL}/`);
  await expect(page.getByRole("heading", { name: /Plan your backpacking trip/ })).toBeVisible();
  await expect(page).toHaveURL(`${baseURL}/`);
  await page.goto(`${baseURL}/privacy`);
  await expect(page.getByRole("heading", { name: "Privacy Policy", exact: true })).toBeVisible();
  await page.goto(`${baseURL}/trips`);
  await expect.poll(() => new URL(page.url()).pathname).toBe("/");
  await expect(page.getByRole("button", { name: "Sign in with Google", exact: true })).toBeVisible();
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
      ownerPage.getByRole("button", { name: "New trip" }).click(),
    ]);
    tripUrl = ownerPage.url();

    await authenticate(outsiderPage, "e2e-outsider@strider.invalid");
    const response = await outsiderPage.goto(tripUrl);
    expect(response?.status()).toBe(404);
    await expect(outsiderPage.getByText(name, { exact: true })).toHaveCount(0);
  } finally {
    if (tripUrl) {
      await ownerPage.goto(tripUrl);
      const more = ownerPage.getByRole("button", { name: "More trip options", exact: true });
      if (await more.isVisible().catch(() => false)) await more.click();
      const button = ownerPage.getByRole("menuitem", { name: "Delete trip", exact: true });
      if (await button.isVisible().catch(() => false)) {
        ownerPage.once("dialog", (dialog) => dialog.accept());
        await Promise.all([ownerPage.waitForURL((url) => url.pathname === "/trips"), button.click()]);
      }
    }
    await owner.close();
    await outsider.close();
  }
});

test("view and edit links grant distinct, revocable access", async ({ browser }) => {
  test.setTimeout(180_000);
  const owner = await browser.newContext();
  const visitor = await browser.newContext();
  const guest = await browser.newContext();
  const ownerPage = await owner.newPage();
  const visitorPage = await visitor.newPage();
  const guestPage = await guest.newPage();
  const name = `E2E trip sharing ${Date.now()}`;
  let tripUrl: string | null = null;

  try {
    await authenticate(ownerPage);
    await ownerPage.goto("/trips");
    ownerPage.once("dialog", (dialog) => dialog.accept(name));
    await Promise.all([ownerPage.waitForURL(/\/trips\/\d+$/), ownerPage.getByRole("button", { name: "New trip" }).click()]);
    tripUrl = ownerPage.url();
    await ownerPage.getByRole("button", { name: "Share", exact: true }).click();
    await ownerPage.getByRole("menuitem", { name: /Create view-only link/ }).click();
    const viewLink = await ownerPage.getByRole("menuitem", { name: "Open view-only page" }).getAttribute("href");
    await ownerPage.getByRole("menuitem", { name: /Create edit link/ }).click();
    const editLink = await ownerPage.getByRole("menuitem", { name: "Open edit link" }).getAttribute("href");
    expect(editLink).not.toBe(viewLink);

    await guestPage.goto(editLink!);
    await expect(guestPage).toHaveURL(/\/\?callbackUrl=/);
    await guestPage.goto(viewLink!);
    await expect(guestPage.getByTestId("shared-trip")).toBeVisible();
    await expect(guestPage.getByRole("link", { name: "Strider", exact: true })).toBeVisible();

    await authenticate(visitorPage, "e2e-collaborator@strider.invalid");
    await visitorPage.goto(viewLink!);
    await expect(visitorPage.getByRole("link", { name: "Strider", exact: true })).toBeVisible();
    await visitorPage.getByRole("link", { name: "Gear", exact: true }).click();
    await expect(visitorPage).toHaveURL(/\/gear$/);
    await visitorPage.goto(viewLink!);
    await visitorPage.getByRole("link", { name: "Trips", exact: true }).click();
    await expect(visitorPage).toHaveURL(/\/trips\?section=shared$/);
    await expect(visitorPage.getByRole("button", { name: /Shared with me/ })).toHaveAttribute("aria-pressed", "true");
    await expect(visitorPage.getByRole("link", { name: /View only/ })).toContainText(name);
    expect((await visitorPage.getByRole("link", { name: /View only/ }).getAttribute("href"))).toBe(viewLink);
    expect((await visitorPage.goto(tripUrl!))?.status()).toBe(404);

    await visitorPage.goto(editLink!);
    await expect(visitorPage).toHaveURL(tripUrl!);
    await expect(visitorPage.getByRole("button", { name: "Delete trip" })).toHaveCount(0);
    await expect(visitorPage.getByRole("menuitem", { name: "Delete trip" })).toHaveCount(0);
    await expect(visitorPage.getByRole("button", { name: "Share", exact: true })).toHaveCount(0);
    await visitorPage.getByRole("button", { name: "Report", exact: true }).click();
    await expect(visitorPage.getByText("Trip report (private notes)")).toHaveCount(0);
    await fillAndSave(visitorPage, "Trip name", `${name} edited`);
    await ownerPage.reload();
    await expect(ownerPage.getByLabel("Trip name")).toHaveValue(`${name} edited`);
    await visitorPage.goto("/trips");
    await visitorPage.getByRole("button", { name: /Shared with me/ }).click();
    await expect(visitorPage.getByRole("link", { name: /Can edit/ })).toContainText(`${name} edited`);

    await ownerPage.getByRole("button", { name: "Share", exact: true }).click();
    await ownerPage.getByRole("menuitem", { name: "Revoke edit link" }).click();
    await ownerPage.getByRole("menuitem", { name: "Confirm revoke edit link" }).click();
    await expect(ownerPage.getByRole("status")).toContainText("Edit access revoked.");
    expect((await visitorPage.goto(tripUrl!))?.status()).toBe(404);
    await visitorPage.goto("/trips");
    await visitorPage.getByRole("button", { name: /Shared with me/ }).click();
    await expect(visitorPage.getByRole("link", { name: /View only/ })).toBeVisible();
    await ownerPage.getByRole("menuitem", { name: "Revoke view-only link" }).click();
    await ownerPage.getByRole("menuitem", { name: "Confirm revoke" }).click();
    await expect(ownerPage.getByRole("status")).toContainText("View-only link revoked.");
    await visitorPage.reload();
    await visitorPage.getByRole("button", { name: /Shared with me/ }).click();
    await expect(visitorPage.getByText("Open a trip link while signed in")).toBeVisible();
  } finally {
    if (tripUrl) {
      await ownerPage.goto(tripUrl);
      const more = ownerPage.getByRole("button", { name: "More trip options", exact: true });
      if (await more.isVisible().catch(() => false)) await more.click();
      const button = ownerPage.getByRole("menuitem", { name: "Delete trip", exact: true });
      if (await button.isVisible().catch(() => false)) {
        ownerPage.once("dialog", (dialog) => dialog.accept());
        await Promise.all([ownerPage.waitForURL((url) => url.pathname === "/trips"), button.click()]);
      }
    }
    await owner.close();
    await visitor.close();
    await guest.close();
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
    const more = page.getByRole("button", { name: "More trip options", exact: true });
    if (await more.isVisible().catch(() => false)) await more.click();
    const button = page.getByRole("menuitem", { name: "Delete trip", exact: true });
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
      page.getByRole("button", { name: "New trip" }).click(),
    ]);
    tripUrl = page.url();

    await expect(page.getByLabel("Trip name")).toHaveValue(initialName);
    await expect(page.getByRole("combobox").filter({ has: page.locator('option[value="idea"]') })).toHaveValue("idea");
    const dateInput = page.getByLabel("Start date", { exact: true });
    await Promise.all([
      page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
      dateInput.fill("2026-10-15"),
    ]);
    await expect(dateInput).toHaveValue("2026-10-15");
    await Promise.all([
      page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
      page.getByRole("button", { name: "Clear start date" }).click(),
    ]);
    await page.reload();
    await expect(dateInput).toHaveValue("");

    await fillAndSave(page, "Trip name", updatedName);
    await fillAndSave(page, "Region", "E2E Test Range");
    await fillAndSave(page, "Area type", "Test wilderness");

    await page.getByRole("button", { name: "Add day", exact: true }).click();
    await expect(page.getByLabel("Day 1 distance in miles")).toBeVisible();
    await fillAndSave(page, "Day 1 distance in miles", "4.5");
    await fillAndSave(page, "Day 1 elevation gain in feet", "1200");

    // Basecamp must reflect the trip's own stats, not only linked-trail defaults.
    // Recent trips lists completed trips newest first, so complete this one with a
    // far-future date to guarantee it leads the list whatever else exists.
    await Promise.all([
      page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
      page.getByLabel("Trip status", { exact: true }).selectOption("completed"),
    ]);
    await Promise.all([
      page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
      page.getByLabel("Start date", { exact: true }).fill("2099-06-01"),
    ]);
    await page.goto("/basecamp");
    const basecampCard = page.locator('a[href^="/trips/"]').filter({ hasText: updatedName });
    await expect(basecampCard).toContainText("4.5 mi");
    await expect(basecampCard).toContainText("1.2k ft");
    await page.goto(tripUrl);

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
    await page.getByRole("button", { name: "Route options", exact: true }).click();
    await expect(page.getByRole("menuitem", { name: "Use route distance and gain", exact: true })).toBeVisible();
    await page.keyboard.press("Escape");

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
    await expect(page.getByTestId("elevation-profile")).toBeVisible();

    // Use a real touch context: a tap may not deliver a pointermove before click.
    const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
    try {
      const phonePage = await phone.newPage();
      await authenticate(phonePage);
      await phonePage.goto(tripUrl);
      await phonePage.getByRole("button", { name: "Map", exact: true }).click();
      const profile = phonePage.getByTestId("elevation-profile");
      await expect(profile).toBeVisible();
      await profile.tap({ position: { x: 90, y: 44 } });
      const campsite = profile.getByRole("button", { name: "Night 1 campsite" });
      await expect(campsite).toHaveCount(1);
      await campsite.tap();
      await expect(phonePage.getByRole("button", { name: "Remove campsite" })).toBeVisible();
      await expect(profile.getByRole("button", { name: "Night 2 campsite" })).toHaveCount(0);
      await phonePage.getByRole("button", { name: "Remove campsite" }).click();
      await expect(campsite).toHaveCount(0);
      await phonePage.reload();
      await phonePage.getByRole("button", { name: "Map", exact: true }).click();
      await expect(phonePage.getByTestId("elevation-profile").getByRole("button", { name: "Night 1 campsite" })).toHaveCount(0);
    } finally {
      await phone.close();
    }

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
    await expect(sharedPage.locator('meta[property="og:title"]')).toHaveAttribute("content", `${updatedName} · Strider`);
    const previewImage = await sharedPage.locator('meta[property="og:image"]').getAttribute("content");
    expect(previewImage).toContain(`${sharedHref}/opengraph-image`);
    // The metadata uses the production canonical origin; fetch this CI trip on
    // the isolated test server instead of asking production for its token.
    const imageUrl = new URL(previewImage!);
    const imageResponse = await sharedPage.request.get(`${baseURL}${imageUrl.pathname}${imageUrl.search}`);
    expect(imageResponse.status()).toBe(200);
    expect(imageResponse.headers()["content-type"]).toContain("image/png");
    await expect(sharedPage.getByText("Trip report (private notes)")).toHaveCount(0);
    await sharedPage.getByRole("button", { name: "Planning", exact: true }).click();
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

test("volume labels and labeled serving densities are parsed without false matches", () => {
  for (const label of ["slice", "large", "undetermined"]) {
    expect(parseVolumeUnit(label)).toBeNull();
  }
  expect(parseVolumeUnit("tbsp")).toBe("tbsp");
  expect(parseVolumeUnit("Tablespoons")).toBe("tbsp");
  expect(parseVolumeUnit("fl oz")).toBe("floz");
  expect(densityFromServing("2 Tbsp (32 g)", null)).toBeCloseTo(1.082, 3);
  expect(densityFromServing("1 1/2 cup", 240)).toBeCloseTo(0.676, 3);
  expect(densityFromServing("1tbsp (15g)", null)).toBeCloseTo(1.014, 3);
});

test("recipe volume amounts persist and ingredient edits update a completed trip", async ({ browser, page }, testInfo) => {
  test.skip(!e2ePassword, "E2E_TEST_PASSWORD is required");
  test.setTimeout(180_000);
  const suffix = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
  const ingredientName = `E2E verified food ${suffix}`;
  const mealName = `E2E verified recipe ${suffix}`;
  const tripName = `E2E verified volume ${suffix}`;
  let tripUrl: string | null = null;
  let createdReference = false;

  try {
    await authenticate(page);
    await page.goto("/food");
    await page.getByRole("tab", { name: "Ingredients", exact: true }).click();
    await page.getByRole("button", { name: "Add ingredient", exact: true }).click();
    await page.getByLabel("Name", { exact: true }).fill(ingredientName);
    await page.getByLabel("kcal / 100g", { exact: true }).fill("800");
    // Leave volume weight blank; the recipe asks for a measured weight.
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Add ingredient", exact: true })).toHaveCount(0);

    await page.getByRole("tab", { name: "Meals", exact: true }).click();
    await page.getByRole("button", { name: "Add meal", exact: true }).click();
    await page.getByLabel("Meal name", { exact: true }).fill(mealName);
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
    await page.getByPlaceholder("Search foods, e.g. olive oil, rolled oats").fill(ingredientName);
    // Saved foods list first; the packaged-search row also repeats the query.
    await page.getByRole("button").filter({ hasText: ingredientName }).first().click();
    const ingredientLine = page.getByText(ingredientName, { exact: true }).locator("..");
    await ingredientLine.getByRole("combobox").selectOption("tbsp");
    await expect(ingredientLine.getByLabel("Measure weight grams")).toBeVisible();
    await ingredientLine.getByLabel("Measure weight grams").fill("13.5");
    await ingredientLine.getByRole("button", { name: "Set", exact: true }).click();
    await expect(ingredientLine.getByLabel("Measure weight grams")).toHaveCount(0);
    const amount = ingredientLine.getByLabel("Ingredient amount", { exact: true });
    await amount.fill("2");
    await ingredientLine.getByRole("combobox").selectOption("oz");
    await expect(amount).toHaveValue("2");
    await ingredientLine.getByRole("combobox").selectOption("g");
    await expect(amount).toHaveValue("57");
    await amount.fill("2");
    await ingredientLine.getByRole("combobox").selectOption("tbsp");
    await expect(amount).toHaveValue("2");
    await expect(ingredientLine).toContainText("216 kcal");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
    await page.reload();
    await page.getByRole("button").filter({ hasText: mealName }).click();
    await expect(ingredientLine.getByRole("spinbutton")).toHaveValue("2");
    await expect(ingredientLine.getByRole("combobox")).toHaveValue("tbsp");

    await page.goto("/trips");
    page.once("dialog", (dialog) => dialog.accept(tripName));
    await Promise.all([
      page.waitForURL(/\/trips\/\d+$/),
      page.getByRole("button", { name: "New trip", exact: true }).click(),
    ]);
    tripUrl = page.url();
    await page.getByRole("button", { name: "Food", exact: true }).locator("visible=true").first().click();
    await page.getByPlaceholder("Search foods, e.g. olive oil, rolled oats").first().fill(mealName);
    await page.getByRole("button").filter({ hasText: mealName }).first().click();
    const plannedMeal = page.getByRole("link", { name: mealName, exact: true }).locator("..");
    await expect(plannedMeal).toContainText("216 kcal");
    // Completed trips follow the same source edits as trips still being planned.
    const status = page.getByRole("combobox").filter({ has: page.locator('option[value="idea"]') });
    await Promise.all([
      page.waitForResponse((response) => response.request().method() === "POST" && response.ok()),
      status.selectOption("completed"),
    ]);

    await page.goto("/food");
    await page.getByRole("tab", { name: "Ingredients", exact: true }).click();
    await page.getByRole("button", { name: ingredientName, exact: true }).locator("visible=true").click();
    await page.getByLabel("kcal / 100g", { exact: true }).fill("400");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("heading", { name: ingredientName, exact: true })).toHaveCount(0);
    await page.goto(tripUrl);
    await page.getByRole("button", { name: "Food", exact: true }).locator("visible=true").first().click();
    await expect(plannedMeal).toContainText("108 kcal");

    // Preserve a preexisting pantry reference; remove only a food this test creates.
    await page.goto("/food");
    await page.getByRole("tab", { name: "Ingredients", exact: true }).click();
    const existingReference = await page.locator('[data-food-source="usda"][data-food-source-id="171413"]').count() > 0;
    await page.getByRole("tab", { name: "Meals", exact: true }).click();
    await page.getByRole("button").filter({ hasText: mealName }).click();
    await page.getByPlaceholder("Search foods, e.g. olive oil, rolled oats").fill("olive oil");
    const reference = page.locator('[data-reference-id="171413"]');
    await expect(reference).toContainText("1 tbsp = 13.5 g");
    createdReference = !existingReference;
    await reference.click();
    await expect(page.locator("[data-meal-line]")).toHaveCount(2);
    await expect(page.locator("[data-meal-line]").last().getByLabel("Ingredient amount", { exact: true })).toBeFocused();
  } finally {
    test.setTimeout(testInfo.timeout + 90_000);
    // A fresh context keeps cleanup usable if an assertion closed the test page.
    // Delete in foreign-key order: trip, then recipe, then its ingredient.
    const cleanup = await browser.newContext();
    const cleanupPage = await cleanup.newPage();
    cleanupPage.setDefaultTimeout(15_000);
    cleanupPage.setDefaultNavigationTimeout(20_000);
    try {
      await authenticate(cleanupPage);
      await cleanupPage.goto(tripUrl ?? "/trips");
      if (!tripUrl) {
        const trip = cleanupPage.locator('a[href^="/trips/"]').filter({ hasText: tripName });
        if (await trip.count()) await trip.click();
      }
      const more = cleanupPage.getByRole("button", { name: "More trip options", exact: true });
      if (await more.isVisible().catch(() => false)) await more.click();
      const deleteTrip = cleanupPage.getByRole("menuitem", { name: "Delete trip", exact: true });
      if (await deleteTrip.isVisible().catch(() => false)) {
        cleanupPage.once("dialog", (dialog) => dialog.accept());
        await Promise.all([
          cleanupPage.waitForURL((url) => url.pathname === "/trips"),
          deleteTrip.click(),
        ]);
      }
      await cleanupPage.goto("/food");
      const meal = cleanupPage.getByRole("button").filter({ hasText: mealName });
      if (await meal.count()) {
        await meal.click();
        cleanupPage.once("dialog", (dialog) => dialog.accept());
        await cleanupPage.getByRole("button", { name: "Delete", exact: true }).click();
        await expect(cleanupPage.getByLabel("Meal name", { exact: true })).toHaveCount(0);
      }
      await cleanupPage.getByRole("tab", { name: "Ingredients", exact: true }).click();
      const ingredient = cleanupPage.getByRole("button", { name: ingredientName, exact: true }).locator("visible=true");
      if (await ingredient.count()) {
        const deleteIngredient = cleanupPage.getByRole("button", { name: `Delete ${ingredientName}`, exact: true });
        cleanupPage.once("dialog", (dialog) => dialog.accept());
        await deleteIngredient.click({ timeout: 15_000 });
        await expect(ingredient).toHaveCount(0);
      }
      if (createdReference) {
        const reference = cleanupPage.locator('[data-food-source="usda"][data-food-source-id="171413"]').locator("visible=true");
        if (await reference.count()) {
          cleanupPage.once("dialog", (dialog) => dialog.accept());
          await reference.locator("..").getByRole("button", { name: /^Delete / }).click({ timeout: 15_000 });
          await expect(reference).toHaveCount(0);
        }
      }
    } finally {
      await cleanup.close();
    }
  }
});
