import { readFile } from "node:fs/promises";
import type { Locator, Page } from "@playwright/test";
import { gradientPng, type Rgb } from "./png.ts";
import {
  DESKTOP,
  expect,
  openApp,
  openHeaderTool,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  TABLET,
  THEMES,
  type Theme,
  test,
} from "./support.ts";

const NARROW = { width: 760, height: 1000 } as const;

const WITH_PANE = { width: 1100, height: 900 } as const;

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

function picture(name: string, [from, to]: readonly [Rgb, Rgb], width = 640, height = 400) {
  return { name, mimeType: "image/png", buffer: gradientPng(width, height, from, to) };
}

async function send(
  page: Page,
  text: string | null,
  files: readonly ReturnType<typeof picture>[],
): Promise<Locator> {
  await page.locator('.composer input[type="file"]').setInputFiles([...files]);
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(files.length, {
    timeout: 15_000,
  });
  await composer(page).fill(text ?? "");
  await composer(page).press("Enter");

  const row = page.getByRole("log", { name: "Messages" }).locator("[data-message-id]", {
    has: page.getByRole("button", { name: `Open ${files[0]?.name}`, exact: true }),
  });

  await expect(row.locator(".attachment-image[data-loaded]")).toHaveCount(files.length);

  return row;
}

async function sendImagesUnderMessageBar(page: Page, files: readonly ReturnType<typeof picture>[]) {
  await composer(page).fill("Pictures follow");
  await composer(page).press("Enter");

  return send(page, null, files);
}

async function openGeneral(page: Page, theme: Theme = "light"): Promise<void> {
  await openApp(page, `r/${ROOM_IDS.general}`, theme);
  await composer(page).waitFor();
}

function actions(image: Locator): Locator {
  return image.locator(".attachment-image-actions");
}

async function downloadFrom(page: Page, press: () => Promise<void>) {
  const [download] = await Promise.all([
    page.waitForEvent("download", { timeout: 5_000 }),
    press(),
  ]);

  return { name: download.suggestedFilename(), bytes: await readFile(await download.path()) };
}

/**
 * Clicks by coordinates, skipping Playwright's hit-target check, so whatever covers the button takes
 * the click.
 */
async function clickCentre(page: Page, button: Locator): Promise<void> {
  const box = await button.boundingBox();

  expect(box).not.toBeNull();
  await page.mouse.click(
    (box?.x ?? 0) + (box?.width ?? 0) / 2,
    (box?.y ?? 0) + (box?.height ?? 0) / 2,
  );
}

async function expectClearOfBar(row: Locator, image: Locator): Promise<void> {
  const bar = await row.getByRole("toolbar", { name: "Message actions" }).boundingBox();
  const bubble = await actions(image).boundingBox();

  expect(bar).not.toBeNull();
  expect(bubble).not.toBeNull();

  if (bar === null || bubble === null) {
    return;
  }

  const apart =
    bubble.y >= bar.y + bar.height ||
    bubble.y + bubble.height <= bar.y ||
    bubble.x >= bar.x + bar.width ||
    bubble.x + bubble.width <= bar.x;

  expect(apart, `bubble ${JSON.stringify(bubble)} overlaps bar ${JSON.stringify(bar)}`).toBe(true);
}

async function expectInside(inner: Locator, outer: Locator): Promise<void> {
  const box = await inner.boundingBox();
  const frame = await outer.boundingBox();

  expect(box).not.toBeNull();
  expect(frame).not.toBeNull();

  if (box === null || frame === null) {
    return;
  }

  expect(box.x, "left edge").toBeGreaterThanOrEqual(frame.x);
  expect(box.y, "top edge").toBeGreaterThanOrEqual(frame.y);
  expect(box.x + box.width, "right edge").toBeLessThanOrEqual(frame.x + frame.width);
  expect(box.y + box.height, "bottom edge").toBeLessThanOrEqual(frame.y + frame.height);
}

async function widenSidePane(page: Page, width: number): Promise<void> {
  const handle = page.getByRole("separator", { name: "Resize the side pane" });
  const box = await handle.boundingBox();
  const start = Number(await handle.getAttribute("aria-valuenow"));

  expect(box).not.toBeNull();

  const x = (box?.x ?? 0) + (box?.width ?? 0) / 2;
  const y = (box?.y ?? 0) + (box?.height ?? 0) / 2;

  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x - (width - start), y, { steps: 4 });
  await page.mouse.up();
  await expect(handle).toHaveAttribute("aria-valuenow", String(width));
}

/** Holds the room's message posts until the returned function is called. */
async function holdSend(page: Page): Promise<() => void> {
  const held = Promise.withResolvers<void>();

  await page.route(`**/rooms/${ROOM_IDS.general}/messages`, async (route) => {
    if (route.request().method() !== "POST") {
      await route.fallback();

      return;
    }

    await held.promise;
    await route.fallback();
  });

  return held.resolve;
}

async function geometry(row: Locator): Promise<number[]> {
  return row.evaluate((element) => [
    element.getBoundingClientRect().height,
    ...[...element.querySelectorAll(".attachment-image")].flatMap((frame) => {
      const box = frame.getBoundingClientRect();

      return [box.width, box.height];
    }),
  ]);
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

for (const viewport of [DESKTOP, TABLET, NARROW]) {
  test(`over a picture that opens a follow-up message, the bubble clears the message bar at ${viewport.width} px`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await openGeneral(page);

    const row = await sendImagesUnderMessageBar(page, [picture("follow.png", LIGHT)]);
    const image = row.locator(".attachment-image");

    await image.hover();
    await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();
    await expect(actions(image)).toHaveCSS("opacity", "1");
    await expectClearOfBar(row, image);
    await shot(page, `image-actions-follow-up-${viewport.width}`, "light");

    const saved = await downloadFrom(page, () =>
      clickCentre(page, image.getByRole("button", { name: "Download follow.png" })),
    );

    expect(saved.name).toBe("follow.png");
  });
}

test("the bubble stays clear of a message bar that was already up when the window narrowed", async ({
  page,
}) => {
  await page.setViewportSize(DESKTOP);
  await openGeneral(page);

  const row = await sendImagesUnderMessageBar(page, [picture("narrowed.png", LIGHT)]);
  const image = row.locator(".attachment-image");

  await image.getByRole("button", { name: "Open narrowed.png" }).focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();

  await page.setViewportSize(TABLET);
  await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();
  await expect(actions(image)).toHaveCSS("opacity", "1");
  await expectClearOfBar(row, image);

  const saved = await downloadFrom(page, () =>
    clickCentre(page, image.getByRole("button", { name: "Download narrowed.png" })),
  );

  expect(saved.name).toBe("narrowed.png");
});

test("gallery tiles squeezed by a widened side pane keep their downloads clear of the message bar", async ({
  page,
}) => {
  await page.setViewportSize(WITH_PANE);
  await openGeneral(page);
  await openHeaderTool(page, /^Files/);
  await widenSidePane(page, 552);

  const files = [
    picture("north.png", LIGHT),
    picture("centre.png", DARK),
    picture("south.png", LIGHT),
  ];

  const row = await sendImagesUnderMessageBar(page, files);
  const tiles = row.locator(".attachment-tile.attachment-image");

  for (const [index, file] of files.entries()) {
    const tile = tiles.nth(index);

    await tile.hover();
    await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();
    await expect(actions(tile)).toHaveCSS("opacity", "1");
    await expectClearOfBar(row, tile);
    await expectInside(actions(tile), tile);

    if (index === 0) {
      await shot(page, "image-actions-gallery-narrow-pane", "light");
    }

    const saved = await downloadFrom(page, () =>
      clickCentre(page, tile.getByRole("button", { name: `Download ${file.name}` })),
    );

    expect(saved.name).toBe(file.name);
    await expect(page.getByRole("dialog")).toHaveCount(0);
  }
});

test("a gallery tile revealed under the message bar keeps its download clear of the bar", async ({
  page,
}) => {
  await page.setViewportSize(TABLET);
  await openGeneral(page);
  await composer(page).fill("Pictures follow");
  await composer(page).press("Enter");

  const files = [
    picture("left.png", LIGHT),
    picture("middle.png", DARK),
    picture("right.png", LIGHT),
  ];

  await page.locator('.composer input[type="file"]').setInputFiles(files);
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(3, { timeout: 15_000 });
  await page.getByRole("button", { name: "Mark middle.png as spoiler" }).click();
  await composer(page).press("Enter");

  const row = page
    .getByRole("log", { name: "Messages" })
    .locator("[data-message-id]", { has: page.getByRole("button", { name: "Open left.png" }) });

  const cover = row.getByRole("button", { name: "Spoiler, activate to reveal" });

  await expect(row.locator(".attachment-image[data-loaded]")).toHaveCount(2);
  await page.mouse.move(0, 0);
  await cover.hover();
  await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();
  await cover.click();

  const middle = row.locator(".attachment-image", {
    has: page.getByRole("button", { name: "Open middle.png" }),
  });

  await expect(middle).toHaveAttribute("data-loaded", "true");
  await middle.hover();
  await expect(actions(middle)).toHaveCSS("opacity", "1");
  await expectClearOfBar(row, middle);
  await shot(page, "image-actions-revealed-tile", "light");

  const saved = await downloadFrom(page, () =>
    clickCentre(page, middle.getByRole("button", { name: "Download middle.png" })),
  );

  expect(saved.name).toBe("middle.png");
});

for (const viewport of [DESKTOP, TABLET, NARROW]) {
  test(`an image smaller than the bubble still offers its download at ${viewport.width} px`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await openGeneral(page);

    const tiny = picture("tiny.png", DARK, 32, 32);
    const row = await sendImagesUnderMessageBar(page, [tiny]);
    const image = row.locator(".attachment-image");

    await image.hover();
    await expect(actions(image)).toHaveCSS("opacity", "1");
    await expectClearOfBar(row, image);
    await shot(page, `image-actions-tiny-${viewport.width}`, "light");

    const saved = await downloadFrom(page, () =>
      clickCentre(page, image.getByRole("button", { name: "Download tiny.png" })),
    );

    expect(saved.name).toBe("tiny.png");
    expect(saved.bytes.equals(tiny.buffer)).toBe(true);
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await image.getByRole("button", { name: "Open tiny.png" }).click();
    await expect(page.getByRole("dialog", { name: "tiny.png" })).toBeVisible();
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

function slug(label: string): string {
  return label.replaceAll(" ", "-");
}

const ACKNOWLEDGED = [
  { label: "a picture smaller than the bubble", files: [picture("speck.png", DARK, 32, 32)] },
  { label: "a sliver", files: [picture("sliver.png", LIGHT, 24, 300)] },
  { label: "a strip", files: [picture("strip.png", LIGHT, 240, 40)] },
];

for (const pointer of ["mouse", "touch"] as const) {
  test.describe(`with a ${pointer}`, () => {
    test.use(pointer === "touch" ? PHONE_TOUCH : {});

    for (const { label, files } of ACKNOWLEDGED) {
      test(`${label} keeps its size when the server acknowledges it`, async ({ page }) => {
        await openGeneral(page);
        await composer(page).fill("Pictures follow");
        await composer(page).press("Enter");
        await expect(
          page.getByRole("log", { name: "Messages" }).locator("[data-message-id]", {
            hasText: "Pictures follow",
          }),
        ).toHaveCount(1);

        await page.locator('.composer input[type="file"]').setInputFiles(files);
        await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(files.length, {
          timeout: 15_000,
        });

        const release = await holdSend(page);

        await composer(page).press("Enter");

        const row = page
          .getByRole("log", { name: "Messages" })
          .locator(".message", { has: page.getByAltText(files[0]?.name ?? "", { exact: true }) });

        await expect(row).toHaveAttribute("data-pending", "sending");
        await expect
          .poll(() =>
            row
              .locator("img")
              .evaluateAll((images) =>
                images.every(
                  (image) => image instanceof HTMLImageElement && image.naturalWidth > 0,
                ),
              ),
          )
          .toBe(true);

        const pending = await geometry(row);

        await shot(page, `image-actions-pending-${slug(label)}-${pointer}`, "light");
        release();
        await expect(row).toHaveAttribute("data-message-id", /\d+/);
        await expect(row.locator(".attachment-image[data-loaded]")).toHaveCount(files.length);

        const sent = await geometry(row);

        await shot(page, `image-actions-sent-${slug(label)}-${pointer}`, "light");
        expect(sent).toHaveLength(pending.length);

        for (const [index, value] of sent.entries()) {
          expect(
            Math.abs(value - (pending[index] ?? 0)),
            `pending ${JSON.stringify(pending)} became ${JSON.stringify(sent)}`,
          ).toBeLessThanOrEqual(1);
        }
      });
    }
  });
}

for (const { viewport, bar } of [
  { viewport: TABLET, bar: true },
  { viewport: PHONE_SMALL, bar: false },
]) {
  test.describe(`on a ${viewport.width} px touch screen`, () => {
    test.use({ viewport, hasTouch: true, isMobile: true });

    test("an image keeps its own size, and the keyboard still reaches a download clear of the message bar", async ({
      page,
    }) => {
      await openGeneral(page);
      expect(await page.evaluate(() => matchMedia("(hover: none)").matches)).toBe(true);

      const tiny = picture("pebble.png", DARK, 32, 32);
      const row = await sendImagesUnderMessageBar(page, [tiny]);
      const image = row.locator(".attachment-image");
      const box = await image.boundingBox();

      expect([box?.width, box?.height]).toEqual([32, 32]);

      await image.getByRole("button", { name: "Open pebble.png" }).focus();
      await page.keyboard.press("Tab");
      await page.keyboard.press("Shift+Tab");
      await expect(actions(image)).toHaveCSS("opacity", "1");

      if (bar) {
        await expect(row.getByRole("toolbar", { name: "Message actions" })).toBeVisible();
        await expectClearOfBar(row, image);
      } else {
        await expect(row.getByRole("toolbar", { name: "Message actions" })).toHaveCount(0);
      }

      await shot(page, `image-actions-touch-keyboard-${viewport.width}`, "light");

      await page.keyboard.press("Tab");
      await expect(image.getByRole("button", { name: "Download pebble.png" })).toBeFocused();

      const saved = await downloadFrom(page, () => page.keyboard.press("Enter"));

      expect(saved.name).toBe("pebble.png");
    });
  });
}

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
