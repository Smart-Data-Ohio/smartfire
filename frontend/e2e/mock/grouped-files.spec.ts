import type { Page } from "@playwright/test";
import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { expect, expectTouchTargets, PHONE_TOUCH, ROOM_IDS, test } from "./support.ts";

const GENERAL = `/app/r/${ROOM_IDS.general}`;

function composer(page: Page) {
  return page.getByRole("textbox", { name: "Message #general" });
}

async function openGeneral(page: Page): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(GENERAL);
  await composer(page).waitFor();
}

/** Three images picked at once, uploaded in the tray. */
async function attachThree(page: Page): Promise<void> {
  const png = Buffer.from(onboardingMockupPng());

  await page.locator('.composer input[type="file"]').setInputFiles(
    ["one.png", "two.png", "three.png"].map((name) => ({
      name,
      mimeType: "image/png",
      buffer: png,
    })),
  );
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(3, { timeout: 15_000 });
}

/** The confirmed message rows that hold `text`. */
function rowsWith(page: Page, text: string) {
  return page
    .getByRole("log", { name: "Messages" })
    .locator("[data-message-id]", { has: page.getByText(text, { exact: true }) });
}

test("three files go as one message with a three-image gallery", async ({ page }) => {
  await openGeneral(page);
  await attachThree(page);
  await composer(page).fill("Mockups for review");
  await composer(page).press("Enter");

  const row = rowsWith(page, "Mockups for review");

  await expect(row).toHaveCount(1);
  await expect(page.locator(".tray-chip")).toHaveCount(0);

  const grid = row.getByRole("list", { name: "3 images and videos" });

  await expect(grid.getByRole("listitem")).toHaveCount(3);
  await expect(
    page.getByRole("log", { name: "Messages" }).getByRole("button", { name: "Open two.png" }),
  ).toHaveCount(1);

  await grid.getByRole("button", { name: "Open two.png" }).click();

  await expect(page.getByRole("dialog", { name: "two.png" })).toContainText("2 of 3");
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("dialog", { name: "three.png" })).toBeVisible();
  await page.getByRole("button", { name: "Next image" }).click();
  await expect(page.getByRole("dialog", { name: "one.png" })).toBeVisible();
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the tray's buttons are finger-sized and the gallery fits the screen", async ({ page }) => {
    await openGeneral(page);
    await attachThree(page);
    await expectTouchTargets(page, ".tray");
    await composer(page).fill("From my phone");
    await composer(page).press("Enter");

    const grid = rowsWith(page, "From my phone").getByRole("list", {
      name: "3 images and videos",
    });

    await expect(grid.getByRole("listitem")).toHaveCount(3);

    const box = await grid.boundingBox();
    const width = page.viewportSize()?.width ?? 0;

    expect(box).not.toBeNull();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(width);
  });
});
