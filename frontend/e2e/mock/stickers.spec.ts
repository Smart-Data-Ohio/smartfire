import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
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

/** Hovers `row` until its hover bar shows, as a resting pointer sees no hover. */
async function hoverBar(row: Locator): Promise<Locator> {
  const bar = row.getByRole("toolbar", { name: "Message actions" });

  await expect(async () => {
    await row.page().mouse.move(0, 0);
    await row.hover();
    await expect(bar).toBeVisible({ timeout: 1000 });
  }).toPass();

  return bar;
}

async function messageAction(row: Locator, name: "Edit" | "Save for later", phone: boolean) {
  if (phone) {
    await row.press(name === "Edit" ? "e" : "s");
  } else {
    await (await hoverBar(row)).getByRole("button", { name, exact: true }).click();
  }
  if (name === "Save for later") {
    await expect(row.locator('[data-flag="saved"]')).toHaveText("Saved for later");
  }
}

/** Sends the seeded Wave sticker from the room composer and answers its row. */
async function sendWave(page: Page): Promise<Locator> {
  const picker = await openPicker(page);

  await picker.getByRole("option", { name: "Wave" }).click();
  await expect(picker).toBeHidden();

  const row = timeline(page).last();

  await expect(row.getByRole("img", { name: "Sticker: Wave" })).toBeVisible();

  return row;
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

matrix("clearing a sticker's caption keeps the sticker", async ({ page, phone }) => {
  await openApp(page, `r/${GENERAL}`);

  const row = await sendWave(page);

  // Give it a caption, then take the caption away again: no deletion is offered,
  // and the sticker stays.
  await messageAction(row, "Edit", phone);
  await page.getByRole("textbox", { name: "Edit message" }).fill("hello sticker");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(row).toContainText("hello sticker");

  await messageAction(row, "Edit", phone);
  await page.getByRole("textbox", { name: "Edit message" }).fill("");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("alertdialog", { name: "Delete message?" })).toHaveCount(0);
  await expect(row.getByRole("img", { name: "Sticker: Wave" })).toBeVisible();
  await expect(row).not.toContainText("hello sticker");
  if (phone) await expectNoHorizontalOverflow(page);
});

/** The new-thread pane for the seeded chart message, its composer ready. */
async function openNewThread(page: Page): Promise<Locator> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${GENERAL}/t/new?parent=${MESSAGE_IDS.generalChart}`);

  const pane = page.locator("aside.right-pane");

  await pane.getByRole("textbox", { name: "Reply…" }).waitFor();

  return pane;
}

/** Picks `name` in the pane composer's sticker picker; the thread is not started yet. */
async function pickInPane(pane: Locator, name: string): Promise<void> {
  await pane.locator(".composer").getByRole("button", { name: "Sticker" }).click();

  const picker = pane.page().getByRole("dialog", { name: "Send a sticker" });

  await picker.getByRole("option", { name }).click();
  await expect(picker).toBeHidden();
}

matrix("a thread starts with a sticker alone", async ({ page, phone }) => {
  const pane = await openNewThread(page);

  await pickInPane(pane, "Wave");

  // The pick waits with the draft: still on the new-thread pane, sticker held.
  await expect(page).toHaveURL(/\/t\/new\?/);
  await expect(pane.locator(".composer").getByRole("img", { name: "Sticker: Wave" })).toBeVisible();

  await expect(pane.getByRole("button", { name: "Send message" })).toBeEnabled();
  await pane.getByRole("textbox", { name: "Reply…" }).press("Enter");
  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
  await expect(pane.getByRole("img", { name: "Sticker: Wave" })).toBeVisible();
  if (phone) await expectNoHorizontalOverflow(page);
});

matrix("a thread starts with a sticker under its text", async ({ page, phone }) => {
  const pane = await openNewThread(page);
  const composer = pane.getByRole("textbox", { name: "Reply…" });

  await composer.fill("hi all");
  await pickInPane(pane, "Dance");
  await composer.press("Enter");

  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
  await expect(pane.getByText("hi all")).toBeVisible();
  // Reduced motion: the animated sticker rests on its first frame.
  await expect(pane.getByRole("img", { name: "Sticker: Dance" })).toHaveAttribute(
    "src",
    "/stickers/2?still=1",
  );
  if (phone) await expectNoHorizontalOverflow(page);
});

for (const caption of ["", "A sticker caption"]) {
  matrix(
    `search and Saved show a ${caption === "" ? "sticker-only" : "captioned sticker"} message`,
    async ({ page, phone }) => {
      await openApp(page, `r/${GENERAL}`);
      const picker = await openPicker(page);
      await picker.getByRole("option", { name: "Dance" }).click();
      const row = timeline(page).last();
      await expect(row.getByRole("img", { name: "Sticker: Dance" })).toBeVisible();
      if (caption !== "") {
        await messageAction(row, "Edit", phone);
        await page.getByRole("textbox", { name: "Edit message" }).fill(caption);
        await page.getByRole("button", { name: "Save", exact: true }).click();
        await expect(row).toContainText(caption);
      }
      await messageAction(row, "Save for later", phone);
      for (const destination of ["/app/search?q=Dance", "/app/saved"]) {
        await page.goto(destination);
        const image = page.getByRole("img", { name: "Sticker: Dance" });
        await expect(image).toBeVisible();
        await expect(image).toHaveAttribute("src", "/stickers/2?still=1");
        const box = await image.boundingBox();
        expect(box?.width).toBe(160);
        expect(box?.height).toBe(160);
        if (caption !== "") await expect(page.getByText(caption, { exact: true })).toBeVisible();
        await page.emulateMedia({ reducedMotion: "no-preference" });
        await expect(image).toHaveAttribute("src", "/stickers/2");
        await page.emulateMedia({ reducedMotion: "reduce" });
        if (phone) await expectNoHorizontalOverflow(page);
      }
      await page.goto("/app/admin/stickers");
      await page.getByRole("button", { name: "Delete Dance" }).click();
      await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();
      await expect(page.getByRole("button", { name: "Delete Dance" })).toHaveCount(0);
      for (const destination of ["/app/search?q=Dance", "/app/saved"]) {
        await page.goto(destination);
        await expect(page.getByText("Dance · Sticker deleted")).toBeVisible();
        await expect(page.getByRole("img", { name: "Sticker: Dance" })).toHaveCount(0);
        if (phone) await expectNoHorizontalOverflow(page);
      }
    },
  );
}

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("a new thread picker and its held sticker work by touch", async ({ page }) => {
    const pane = await openNewThread(page);
    await pane.locator(".composer").getByRole("button", { name: "Sticker" }).tap();
    const picker = page.getByRole("dialog", { name: "Send a sticker" });
    await expect(picker).toHaveClass(/action-sheet/);
    await expectTouchTargets(page, ".popover.action-sheet");
    await picker.getByRole("option", { name: "Dance" }).tap();
    await expect(picker).toBeHidden();
    const remove = pane.getByRole("button", { name: "Remove Dance sticker" });
    const box = await remove.boundingBox();
    expect(box?.width).toBeGreaterThanOrEqual(44);
    expect(box?.height).toBeGreaterThanOrEqual(44);
    await expectNoHorizontalOverflow(page);
    await remove.tap();
    await expect(pane.getByRole("button", { name: "Send message" })).toBeDisabled();
    await pickInPane(pane, "Wave");
    await pane.getByRole("button", { name: "Send message" }).tap();
    await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
    await expect(pane.getByRole("img", { name: "Sticker: Wave" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

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
