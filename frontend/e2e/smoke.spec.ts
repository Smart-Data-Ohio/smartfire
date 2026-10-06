import { expect, test } from "@playwright/test";

test("the built app renders under /app/", async ({ page }) => {
  await page.goto("/app/");

  await expect(page.getByRole("heading", { name: "Smartfire" })).toBeVisible();
});
