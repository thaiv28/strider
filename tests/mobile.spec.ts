import { expect, test, type Page } from "@playwright/test";
import { authenticate } from "./auth";

test.beforeEach(async ({ page }) => {
  await authenticate(page);
});

async function expectNoHorizontalOverflow(page: Page) {
  const report = await page.evaluate(() => {
    const viewport = document.documentElement.clientWidth;
    const offenders = [...document.body.querySelectorAll<HTMLElement>("*")]
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          tag: element.tagName.toLowerCase(),
          className: typeof element.className === "string" ? element.className.slice(0, 120) : "",
          left: Math.round(rect.left),
          right: Math.round(rect.right),
        };
      })
      .filter((item) => item.left < -1 || item.right > viewport + 1)
      .slice(0, 10);
    return {
      viewport,
      scrollWidth: document.documentElement.scrollWidth,
      offenders,
    };
  });

  expect(report.scrollWidth, JSON.stringify(report.offenders, null, 2)).toBeLessThanOrEqual(report.viewport + 1);
}

const pages = [
  { path: "/basecamp", heading: "Basecamp" },
  { path: "/gear", heading: "Gear Library" },
  { path: "/food", heading: "Food" },
  { path: "/trips", heading: "Trips" },
  { path: "/calendar", heading: "Calendar" },
  { path: "/settings", heading: "Settings" },
];

for (const destination of pages) {
  test(`${destination.path} fits the phone viewport`, async ({ page }) => {
    await page.goto(destination.path);
    await expect(page.getByRole("heading", { name: destination.heading, exact: true })).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });
}

test("the public landing page fits the phone viewport", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /Plan farther/ })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
});

test("a trip detail page fits the phone viewport", async ({ page }) => {
  await page.goto("/trips");
  const firstTrip = page.locator('a[href^="/trips/"]').first();
  test.skip((await firstTrip.count()) === 0, "No trip exists in the seed data");
  const href = await firstTrip.getAttribute("href");
  expect(href).toBeTruthy();
  await page.goto(href!);
  await expect(page.getByText("Logistics", { exact: true }).first()).toBeVisible();
  const tabStrip = page.getByRole("button", { name: "Logistics", exact: true }).locator("..");
  await expect(tabStrip).toHaveCSS("position", "static");
  await expect(tabStrip).toHaveCSS("overflow-y", "hidden");
  const tabStripHeight = await tabStrip.evaluate((element) => ({
    clientHeight: element.clientHeight,
    scrollHeight: element.scrollHeight,
  }));
  expect(tabStripHeight.scrollHeight).toBeLessThanOrEqual(tabStripHeight.clientHeight + 1);
  const dateInput = page.locator('input[type="date"]').first();
  if (await dateInput.count()) {
    const dateWidths = await dateInput.evaluate((element) => ({
      right: element.getBoundingClientRect().right,
      parentRight: element.parentElement?.getBoundingClientRect().right ?? 0,
      viewport: document.documentElement.clientWidth,
    }));
    expect(dateWidths.right).toBeLessThanOrEqual(dateWidths.parentRight + 1);
    expect(dateWidths.right).toBeLessThanOrEqual(dateWidths.viewport + 1);
  }
  await page.waitForTimeout(1_500);
  await expectNoHorizontalOverflow(page);
});

test("route maps stay below the mobile navigation", async ({ page }) => {
  await page.goto("/basecamp");
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  const bottomNavigationOwnsViewportEdge = await page.evaluate(() =>
    Boolean(document.elementFromPoint(window.innerWidth / 2, window.innerHeight - 20)?.closest('nav[aria-label="Primary navigation"]')),
  );
  expect(bottomNavigationOwnsViewportEdge).toBe(true);
});
