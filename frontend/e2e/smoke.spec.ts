import { expect, test } from "@playwright/test";

test("the built app renders its shell under /app/", async ({ page }) => {
  await page.goto("/app/");

  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
});
