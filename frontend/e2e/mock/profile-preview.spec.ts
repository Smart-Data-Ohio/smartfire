import type { Locator, Page } from "@playwright/test";
import { gradientPng, twoFrameApng, twoFrameGif } from "./png.ts";
import {
  DESKTOP,
  expect,
  openApp,
  PHONE,
  ROOM_IDS,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

const PORTRAIT = {
  name: "portrait.png",
  mimeType: "image/png",
  buffer: gradientPng(200, 250, [120, 70, 40], [30, 60, 110]),
};

const BANNER = {
  name: "banner.png",
  mimeType: "image/png",
  buffer: gradientPng(600, 200, [39, 92, 196], [24, 180, 160]),
};

const LONG_NAME = "Christopher Maximilian Alexander Montgomery-St. Amand";

type Box = { x: number; y: number; width: number; height: number };

async function boxOf(locator: Locator): Promise<Box> {
  const box = await locator.boundingBox();

  expect(box, "the element is laid out").not.toBeNull();

  return box ?? { x: 0, y: 0, width: 0, height: 0 };
}

function overlaps(a: Box, b: Box): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

async function expectSquareAvatars(page: Page): Promise<void> {
  const measure = () =>
    page.locator(".avatar > img.avatar-image").evaluateAll((images: HTMLImageElement[]) =>
      images.flatMap((image) => {
        if (!(image.checkVisibility() && image.complete && image.naturalWidth > 0)) {
          return [];
        }

        const photo = image.getBoundingClientRect();
        const tile = image.parentElement?.getBoundingClientRect();

        return [{ photo: [photo.width, photo.height], tile: [tile?.width, tile?.height] }];
      }),
    );

  await expect
    .poll(async () => (await measure()).length, { message: "a photo shows" })
    .toBeGreaterThan(0);

  for (const { photo, tile } of await measure()) {
    expect(photo, "the photo is cropped to its square tile").toEqual(tile);
    expect(tile[0]).toBe(tile[1]);
  }
}

async function servePortraitFor(page: Page, userId: number): Promise<void> {
  await page.route(`**/users/${userId}/avatar*`, (route) =>
    route.fulfill({ contentType: "image/png", body: PORTRAIT.buffer }),
  );
}

async function openProfileSettings(page: Page, theme: Theme = "light"): Promise<void> {
  await openApp(page, "settings/profile", theme);
  await page.getByRole("button", { name: "Upload avatar" }).waitFor();
}

async function uploadPortrait(page: Page): Promise<void> {
  await page.getByLabel("Choose avatar image").setInputFiles(PORTRAIT);
  await expect(page.getByText("Avatar updated")).toBeVisible();
  await expect(preview(page).locator(".avatar img")).toHaveJSProperty("complete", true);
}

async function typeName(page: Page, name: string): Promise<void> {
  await page.getByRole("textbox", { name: "Name", exact: true }).fill(name);
}

/** Holds every direct upload's bytes until the returned release is called. */
async function holdUploads(page: Page): Promise<() => void> {
  const { promise: held, resolve: release } = Promise.withResolvers<void>();

  await page.route("**/rails/active_storage/disk/**", async (route) => {
    await held;
    await route.continue();
  });

  return release;
}

async function blobLoads(image: Locator, src: string): Promise<boolean> {
  return image.evaluate(
    (_: HTMLImageElement, url: string) =>
      fetch(url).then(
        () => true,
        () => false,
      ),
    src,
  );
}

function preview(page: Page): Locator {
  return page.locator("figure", { has: page.locator(".profile-hero") });
}

function previewAvatarPhoto(page: Page): Locator {
  return preview(page).locator(".profile-hero .avatar img");
}

for (const [label, viewport] of [
  ["desktop", { width: 1280, height: DESKTOP.height }],
  ["phone", PHONE],
] as const) {
  test.describe(`on a ${label}`, () => {
    test.use({ viewport });

    test("a portrait photo is cropped square and never covers the name in the preview", async ({
      page,
    }) => {
      await openProfileSettings(page);
      await uploadPortrait(page);
      await expectSquareAvatars(page);

      const name = preview(page).getByText("Riel St. Amand", { exact: true });

      expect(overlaps(await boxOf(previewAvatarPhoto(page)), await boxOf(name))).toBe(false);
    });

    test("a long name wraps inside the preview card as it is typed, clear of the avatar and banner", async ({
      page,
    }) => {
      await openProfileSettings(page);
      await uploadPortrait(page);
      await page.getByLabel("Choose banner image").setInputFiles(BANNER);
      await expect(page.getByText("Profile banner updated")).toBeVisible();
      await typeName(page, LONG_NAME);

      const card = await boxOf(preview(page));
      const name = await boxOf(preview(page).getByText(LONG_NAME, { exact: true }));
      const banner = await boxOf(preview(page).locator(".profile-banner"));

      expect(overlaps(await boxOf(previewAvatarPhoto(page)), name)).toBe(false);
      expect(overlaps(banner, name)).toBe(false);
      expect(name.x).toBeGreaterThanOrEqual(card.x);
      expect(name.x + name.width).toBeLessThanOrEqual(card.x + card.width);
    });

    test("someone's portrait photo stays square on their page and in their messages", async ({
      page,
    }) => {
      await servePortraitFor(page, USER_IDS.maya);
      await openApp(page, `people/${USER_IDS.maya}`);
      await page.locator(".person .profile-hero .avatar img").waitFor();
      await expectSquareAvatars(page);

      await openApp(page, `r/${ROOM_IDS.dmMaya}`);
      await expectSquareAvatars(page);
    });
  });
}

test.describe("a DM's details on a phone", () => {
  test.use({ viewport: PHONE });

  test("the other side's portrait photo stays square, clear of their name", async ({ page }) => {
    await servePortraitFor(page, USER_IDS.maya);
    await openApp(page, `r/${ROOM_IDS.dmMaya}`);
    await page.locator(".room-header .room-title-button").click();

    const details = page.locator(".details");
    const avatar = details.locator(".profile-hero .avatar img");

    await expect(avatar).toHaveJSProperty("complete", true);
    await expectSquareAvatars(page);
    const name = await boxOf(details.locator(".details-name"));

    expect(overlaps(await boxOf(avatar), name)).toBe(false);
  });
});

for (const [label, file] of [
  ["GIF", { name: "avatar.gif", mimeType: "image/gif", buffer: twoFrameGif(64, 64) }],
  ["APNG", { name: "avatar.png", mimeType: "image/png", buffer: twoFrameApng(64, 64) }],
] as const) {
  test(`under reduced motion an animated ${label} avatar shows a still frame while uploading`, async ({
    page,
  }) => {
    await openProfileSettings(page);
    await page.emulateMedia({ reducedMotion: "reduce" });

    const release = await holdUploads(page);

    await page.getByLabel("Choose avatar image").setInputFiles(file);

    const photo = previewAvatarPhoto(page);

    await expect(photo).toHaveAttribute("src", /^blob:/);

    const still = (await photo.getAttribute("src")) ?? "";

    const drawn = await photo.evaluate(async (image: HTMLImageElement) => {
      const text = String.fromCharCode(
        ...new Uint8Array(await (await fetch(image.src)).arrayBuffer()),
      );

      return { png: text.slice(1, 4) === "PNG", animated: text.includes("acTL") };
    });

    expect(drawn).toEqual({ png: true, animated: false });

    release();
    await expect(page.getByText("Avatar updated")).toBeVisible();
    await expect(photo).not.toHaveAttribute("src", still);
    expect(await blobLoads(page.locator("body"), still), "the drawn still is released").toBe(false);
  });
}

test("the uploading file's preview is released once the avatar is saved", async ({ page }) => {
  await openProfileSettings(page);
  await page.emulateMedia({ reducedMotion: "no-preference" });

  const release = await holdUploads(page);

  await page.getByLabel("Choose avatar image").setInputFiles(PORTRAIT);

  const photo = previewAvatarPhoto(page);

  await expect(photo).toHaveAttribute("src", /^blob:/);

  const preview = (await photo.getAttribute("src")) ?? "";

  expect(await blobLoads(photo, preview), "the preview loads while uploading").toBe(true);
  release();
  await expect(page.getByText("Avatar updated")).toBeVisible();
  await expect(photo).not.toHaveAttribute("src", preview);
  expect(await blobLoads(photo, preview), "the preview is released").toBe(false);
});

test("removing the avatar puts the initials back in the preview card", async ({ page }) => {
  await openProfileSettings(page);
  await uploadPortrait(page);
  await page.getByRole("button", { name: "Remove avatar" }).click();
  await expect(page.getByRole("button", { name: "Upload avatar" })).toBeVisible();
  await expect(previewAvatarPhoto(page)).toHaveCount(0);
  await expect(preview(page).locator(".profile-hero .avatar-initials")).toHaveText("RA");
});

for (const theme of ["light", "dark"] as const) {
  test(`the avatar's ring is the preview card's own colour in ${theme} mode`, async ({ page }) => {
    await openProfileSettings(page, theme);

    const colours = await preview(page).evaluate((card) => ({
      card: getComputedStyle(card).backgroundColor,
      ring: getComputedStyle(card.querySelector(".profile-hero-avatar") ?? card).boxShadow,
    }));

    expect(colours.ring.startsWith(colours.card)).toBe(true);
  });
}
