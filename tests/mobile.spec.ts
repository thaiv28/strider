import { expect, test, type Page } from "@playwright/test";

const sessionSecret = process.env.MOBILE_TEST_SESSION_SECRET;
const baseURL = process.env.MOBILE_TEST_BASE_URL ?? "http://127.0.0.1:3000";

test.beforeEach(async ({ context }) => {
  test.skip(!sessionSecret, "MOBILE_TEST_SESSION_SECRET is required");
  await context.addCookies([
    {
      name: "strider_session",
      value: sessionSecret!,
      url: baseURL,
      httpOnly: true,
      secure: baseURL.startsWith("https://"),
      sameSite: "Strict",
    },
  ]);
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
  { path: "/", heading: "Basecamp" },
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

test("a trip detail page fits the phone viewport", async ({ page }) => {
  await page.goto("/trips");
  const firstTrip = page.locator('a[href^="/trips/"]').first();
  test.skip((await firstTrip.count()) === 0, "No trip exists in the seed data");
  const href = await firstTrip.getAttribute("href");
  expect(href).toBeTruthy();
  await page.goto(href!);
  await expect(page.getByText("Logistics", { exact: true }).first()).toBeVisible();
  await expectNoHorizontalOverflow(page);
});
