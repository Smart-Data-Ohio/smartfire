import type { Page } from "@playwright/test";
import { STICKER_GIF } from "../../mock/s2/stickers.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
} from "./support.ts";

/**
 * Stickers against the mock backend: an administrator uploads one (a wrong file is refused
 * before it uploads), it shows in the composer's picker, a pick sends it at once, and the message
 * shows it, on its still under reduced motion. Deleting it leaves the message a placeholder.
 */

const GENERAL = ROOM_IDS.general;

/** The conversation's messages, without the pending rows. */
function timeline(page: Page) {
  return page.locator("article.message:not([data-pending])");
}

/** Opens the composer's sticker picker. */
async function openPicker(page: Page) {
  await page.locator(".composer").getByRole("button", { name: "Sticker" }).click();

  const picker = page.getByRole("dialog", { name: "Send a sticker" });

  await expect(picker.getByRole("listbox")).toBeVisible();

  return picker;
}

matrix("an administrator's new sticker is picked, sent and shown", async ({ page, phone }) => {
  await openApp(page, "admin/stickers");
  await expect(page.getByRole("heading", { level: 1, name: "Stickers" })).toBeVisible();
  await expect(page.getByText("Stickers: 3 of 60 used.")).toBeVisible();

  // The wrong kind of file is refused here, before it uploads.
  await page.getByLabel("Image").setInputFiles({
    name: "logo.svg",
    mimeType: "image/svg+xml",
    buffer: Buffer.from("<svg xmlns='http://www.w3.org/2000/svg'/>"),
  });
  await expect(page.getByText("Image must be a PNG, APNG or GIF image.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Add sticker" })).toBeDisabled();

  await page.getByRole("textbox", { name: /Name/ }).fill("Party");
  await page.getByRole("textbox", { name: /Tags/ }).fill("🎉 celebrate");
  await page.getByLabel("Image").setInputFiles({
    name: "party.gif",
    mimeType: "image/gif",
    buffer: Buffer.from(STICKER_GIF),
  });
  await page.getByRole("button", { name: "Add sticker" }).click();

  await expect(page.getByRole("button", { name: "Delete Party" })).toBeAttached();
  await expect(page.getByText("Stickers: 4 of 60 used.")).toBeVisible();

  if (phone) await expectNoHorizontalOverflow(page);

  await page.goto(`/app/r/${GENERAL}`);

  const picker = await openPicker(page);

  await picker.getByRole("combobox", { name: "Search stickers" }).fill("celebr");
  await expect(picker.getByRole("option")).toHaveCount(1);
  await picker.getByRole("option", { name: "Party" }).click();
  await expect(picker).toBeHidden();

  // The sent message shows the sticker, on its first frame (openApp reduces motion).
  const sent = timeline(page).last().getByRole("img", { name: "Sticker: Party" });

  await expect(sent).toBeVisible();
  await expect(sent).toHaveAttribute("src", /^\/stickers\/\d+\?still=1$/);

  const box = await sent.boundingBox();

  expect(box?.width).toBe(160);
  expect(box?.height).toBe(160);

  // With motion allowed it plays.
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await expect(sent).toHaveAttribute("src", /^\/stickers\/\d+$/);

  if (phone) await expectNoHorizontalOverflow(page);

  // Deleted from the catalog, the message keeps a placeholder with its name.
  await page.goto("/app/admin/stickers");
  await page.getByRole("button", { name: "Delete Party" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("Stickers: 3 of 60 used.")).toBeVisible();

  await page.goto(`/app/r/${GENERAL}`);
  await expect(timeline(page).last()).toContainText("Party · Sticker deleted");
  await expect(timeline(page).last().getByRole("img", { name: "Sticker: Party" })).toHaveCount(0);
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the picker is a sheet of finger-sized stickers, and a tap sends one", async ({ page }) => {
    await openApp(page, `r/${GENERAL}`);

    const picker = await openPicker(page);

    await expect(picker).toHaveClass(/action-sheet/);
    await expectTouchTargets(page, ".popover.action-sheet");
    await expectNoHorizontalOverflow(page);

    await picker.getByRole("option", { name: "Wave" }).tap();
    await expect(picker).toBeHidden();
    await expect(timeline(page).last().getByRole("img", { name: "Sticker: Wave" })).toBeVisible();
  });
});
