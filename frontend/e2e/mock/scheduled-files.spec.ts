import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { expect, ROOM_IDS, test } from "./support.ts";

test("schedules two files and sends one gallery message", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await page.getByRole("textbox", { name: "Message #general" }).waitFor();
  await page.locator('.composer input[type="file"]').setInputFiles([
    {
      name: "scheduled-one.png",
      mimeType: "image/png",
      buffer: Buffer.from(onboardingMockupPng()),
    },
    {
      name: "scheduled-two.png",
      mimeType: "image/png",
      buffer: Buffer.from(onboardingMockupPng()),
    },
  ]);
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(2);
  await page.getByRole("button", { name: "Schedule message", exact: true }).click();
  await page.getByRole("menuitem", { name: /In 1 hour/ }).click();
  await expect(page.locator(".tray-chip")).toHaveCount(0);
  await page.goto("/app/scheduled");
  const row = page.locator(".list-row").filter({ hasText: "scheduled-one.png" });
  await expect(row.getByText("scheduled-one.png", { exact: true })).toBeVisible();
  await expect(row.getByText("scheduled-two.png", { exact: true })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(row.getByAltText("scheduled-one.png")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: "../.scratch/scheduled-mobile.png" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await row.hover();
  await row.getByRole("button", { name: "Send now", exact: true }).click();
  await expect(row.getByText(/Sent /)).toBeVisible();
  await row.getByRole("button", { name: "View message", exact: true }).click();

  const gallery = page
    .locator(".attachment-gallery")
    .filter({ has: page.getByRole("button", { name: /scheduled-one.png/ }) });

  await expect(gallery).toBeVisible();
  await expect(gallery.locator(".attachment-image")).toHaveCount(2);
});
