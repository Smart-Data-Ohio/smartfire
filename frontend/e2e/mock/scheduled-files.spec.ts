import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import { expect, ROOM_IDS, test } from "./support.ts";

test("schedules files, edits the group, and sends one gallery reply", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}/m/${MESSAGE_IDS.generalReactions}`);
  await page.getByRole("textbox", { name: "Message #general" }).waitFor();
  const target = page.locator(`[data-message-id="${MESSAGE_IDS.generalReactions}"]`);
  await target.hover();
  await target.getByRole("button", { name: "Reply", exact: true }).click();
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
  await expect(row.getByText(/Replying to Grace Adeyemi/)).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(row.getByAltText("scheduled-one.png")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: "../.scratch/scheduled-mobile.png" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await row.hover();
  await row.getByRole("button", { name: "Edit message", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "Edit scheduled message" });
  await editor.getByRole("button", { name: "Remove scheduled-one.png" }).click();
  await editor.getByLabel("Add files").setInputFiles({
    name: "scheduled-three.png",
    mimeType: "image/png",
    buffer: Buffer.from(onboardingMockupPng()),
  });
  await expect(editor.locator('.tray-chip[data-phase="done"]')).toHaveCount(1);
  await editor.getByRole("button", { name: "Save changes" }).click();
  await expect(editor).not.toBeVisible();
  const edited = page.locator(".list-row").filter({ hasText: "scheduled-three.png" });
  await expect(edited.getByText("scheduled-one.png", { exact: true })).toHaveCount(0);
  await edited.hover();
  await edited.getByRole("button", { name: "Send now", exact: true }).click();
  await expect(edited.getByText(/Sent /)).toBeVisible();
  await edited.getByRole("button", { name: "View message", exact: true }).click();

  const gallery = page
    .locator(".attachment-gallery")
    .filter({ has: page.getByRole("button", { name: /scheduled-three.png/ }) });

  await expect(gallery).toBeVisible();
  await expect(gallery.locator(".attachment-image")).toHaveCount(2);
  const sent = page.locator("[data-message-row]").filter({ has: gallery });
  await expect(sent).toHaveCount(1);
  await expect(sent.locator(".message-reply")).toContainText(
    "Shipped the new onboarding checklist",
  );
});
