import { expect, test, type Page } from "@playwright/test";

export const e2ePassword = process.env.E2E_TEST_PASSWORD ?? process.env.WORKFLOW_TEST_PASSWORD;

export async function authenticate(page: Page, email = "e2e@strider.invalid") {
  test.skip(!e2ePassword, "E2E_TEST_PASSWORD is required");
  await page.goto("/login?test=1");
  await page.getByLabel("Test account").fill(email);
  await page.getByLabel("Test password").fill(e2ePassword!);
  await Promise.all([
    page.waitForURL((url) => url.pathname === "/basecamp"),
    page.getByRole("button", { name: "Test sign in" }).click(),
  ]);
  await expect(page).toHaveURL(/\/basecamp$/);
}
