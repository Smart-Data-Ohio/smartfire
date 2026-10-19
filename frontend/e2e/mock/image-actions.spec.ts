import { readFile } from "node:fs/promises";
import type { Locator, Page } from "@playwright/test";
import { gradientPng, type Rgb } from "./png.ts";
import {
  expect,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  THEMES,
  type Theme,
  test,
} from "./support.ts";

const LIGHT: readonly [Rgb, Rgb] = [
  [250, 248, 240],
  [255, 236, 200],
];

const DARK: readonly [Rgb, Rgb] = [
  [12, 14, 28],
  [40, 30, 70],
];

function composer(page: Page) {
  return page.getByRole("textbox", { name: "Message #general" });
}

function picture(name: string, [from, to]: readonly [Rgb, Rgb]) {
  return { name, mimeType: "image/png", buffer: gradientPng(640, 400, from, to) };
}

/** Sends `files` with `text` from #general's composer and returns the message's row. */
async function send(
  page: Page,
  text: string,
  files: readonly ReturnType<typeof picture>[],
): Promise<Locator> {
  await page.locator('.composer input[type="file"]').setInputFiles([...files]);
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(files.length, {
    timeout: 15_000,
  });
  await composer(page).fill(text);
  await composer(page).press("Enter");

  const row = page
    .getByRole("log", { name: "Messages" })
    .locator("[data-message-id]", { has: page.getByText(text, { exact: true }) });

  await expect(row.locator(".attachment-image[data-loaded]")).toHaveCount(files.length);

  return row;
}

async function openGeneral(page: Page, theme: Theme = "light"): Promise<void> {
  await openApp(page, `r/${ROOM_IDS.general}`, theme);
  await composer(page).waitFor();
}

function actions(image: Locator): Locator {
  return image.locator(".attachment-image-actions");
}

/** Runs `press` and returns the file the browser saved, with the name it suggested. */
async function downloadFrom(page: Page, press: () => Promise<void>) {
  const [download] = await Promise.all([page.waitForEvent("download"), press()]);

  return { name: download.suggestedFilename(), bytes: await readFile(await download.path()) };
}

for (const theme of THEMES) {
  test(`hovering an image offers its download, and clicking the image still opens it (${theme})`, async ({
    page,
  }) => {
    await openGeneral(page, theme);

    const light = picture("daylight.png", LIGHT);
    const row = await send(page, "One light picture", [light]);
    const image = row.locator(".attachment-image");
    const download = image.getByRole("button", { name: "Download daylight.png" });

    await expect(actions(image)).toHaveCSS("opacity", "0");
    await image.hover();
    await expect(actions(image)).toHaveCSS("opacity", "1");
    await shot(page, "image-actions-single-light-image", theme);

    const saved = await downloadFrom(page, () => download.click());

    expect(saved.name).toBe("daylight.png");
    expect(saved.bytes.equals(light.buffer)).toBe(true);
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await image.getByRole("button", { name: "Open daylight.png" }).click();
    await expect(page.getByRole("dialog", { name: "daylight.png" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);

    const dark = await send(page, "One dark picture", [picture("night.png", DARK)]);

    await dark.locator(".attachment-image").hover();
    await expect(actions(dark.locator(".attachment-image"))).toHaveCSS("opacity", "1");
    await shot(page, "image-actions-single-dark-image", theme);
  });

  test(`every gallery tile offers its own download (${theme})`, async ({ page }) => {
    await openGeneral(page, theme);

    const files = [
      picture("first.png", LIGHT),
      picture("second.png", DARK),
      picture("third.png", LIGHT),
    ];

    const row = await send(page, "Three pictures", files);
    const tiles = row.locator(".attachment-tile.attachment-image");
    const second = tiles.nth(1);

    await second.hover();
    await expect(actions(second)).toHaveCSS("opacity", "1");
    await expect(actions(tiles.nth(0))).toHaveCSS("opacity", "0");
    await shot(page, "image-actions-gallery-dark-tile", theme);

    const saved = await downloadFrom(page, () =>
      second.getByRole("button", { name: "Download second.png" }).click(),
    );

    expect(saved.name).toBe("second.png");
    expect(saved.bytes.equals(files[1]?.buffer ?? Buffer.alloc(0))).toBe(true);

    await tiles.nth(2).hover();
    await expect(actions(tiles.nth(2))).toHaveCSS("opacity", "1");
    await shot(page, "image-actions-gallery-light-tile", theme);

    await tiles.nth(2).getByRole("button", { name: "Open third.png" }).click();
    await expect(page.getByRole("dialog", { name: "third.png" })).toContainText("3 of 3");
  });
}

test("the keyboard reaches the download from the image", async ({ page }) => {
  await openGeneral(page);

  const row = await send(page, "A picture for the keyboard", [picture("keys.png", LIGHT)]);
  const image = row.locator(".attachment-image");

  await image.getByRole("button", { name: "Open keys.png" }).focus();
  await page.keyboard.press("Tab");

  const download = image.getByRole("button", { name: "Download keys.png" });

  await expect(download).toBeFocused();
  await expect(actions(image)).toHaveCSS("opacity", "1");

  const saved = await downloadFrom(page, () => page.keyboard.press("Enter"));

  expect(saved.name).toBe("keys.png");

  await page.keyboard.press("Tab");
  await expect(image.getByRole("button", { name: "Open original keys.png" })).toBeFocused();
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("a tap opens the lightbox, which has the download, and leaves no bubble over the image", async ({
    page,
  }) => {
    await openGeneral(page);

    const row = await send(page, "A picture from my phone", [picture("pocket.png", LIGHT)]);
    const image = row.locator(".attachment-image");

    await image.tap();

    const dialog = page.getByRole("dialog", { name: "pocket.png" });

    await expect(dialog).toBeVisible();

    const saved = await downloadFrom(page, () =>
      dialog.getByRole("button", { name: "Download" }).tap(),
    );

    expect(saved.name).toBe("pocket.png");

    await dialog.getByRole("button", { name: "Close" }).tap();
    await expect(dialog).toBeHidden();
    await expect(actions(image)).toHaveCSS("opacity", "0");
    await expect(actions(image)).toHaveCSS("pointer-events", "none");
  });
});
