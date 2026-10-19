import type { Locator, Page } from "@playwright/test";
import { gradientPng } from "./png.ts";
import { DESKTOP, expect, openApp, PHONE, ROOM_IDS, test, USER_IDS } from "./support.ts";

/** A portrait photo, taller than it is wide, as most phone selfies are. */
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

type Box = NonNullable<Awaited<ReturnType<Locator["boundingBox"]>>>;

async function boxOf(locator: Locator): Promise<Box> {
  const box = await locator.boundingBox();

  expect(box, "the element is laid out").not.toBeNull();

  return box as Box;
}

function overlaps(a: Box, b: Box): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

/** Every photo avatar on the page fills its tile exactly, whatever the photo's own shape. */
async function expectSquareAvatars(page: Page): Promise<void> {
  const measure = () =>
    page.locator(".avatar > img.avatar-image").evaluateAll((images) =>
      images
        .filter((image) => {
          const photo = image as HTMLImageElement;

          return photo.checkVisibility() && photo.complete && photo.naturalWidth > 0;
        })
        .map((image) => {
          const photo = image.getBoundingClientRect();
          const tile = (image.parentElement as HTMLElement).getBoundingClientRect();

          return { photo: [photo.width, photo.height], tile: [tile.width, tile.height] };
        }),
    );

  await expect.poll(async () => (await measure()).length, { message: "a photo shows" }).toBeGreaterThan(0);

  for (const { photo, tile } of await measure()) {
    expect(photo, "the photo is cropped to its square tile").toEqual(tile);
    expect(tile[0]).toBe(tile[1]);
  }
}

async function openProfileSettings(page: Page): Promise<void> {
  await openApp(page, "settings/profile");
  await page.getByRole("button", { name: "Upload photo" }).waitFor();
}

async function uploadPortrait(page: Page): Promise<void> {
  await page.getByLabel("Upload avatar").setInputFiles(PORTRAIT);
  await expect(page.getByText("Avatar updated")).toBeVisible();
  await expect(preview(page).locator(".avatar img")).toHaveJSProperty("complete", true);
}

async function saveName(page: Page, name: string): Promise<void> {
  await page.getByRole("textbox", { name: "Name", exact: true }).fill(name);
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(page.getByText("Profile saved")).toBeVisible();
}

/** The settings' preview of your profile card: the figure holding the banner and avatar. */
function preview(page: Page): Locator {
  return page.locator("figure", { has: page.locator(".profile-hero") });
}

/** The preview's avatar photo, as drawn (a photo that overflows its tile covers what's below). */
function previewAvatar(page: Page): Locator {
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

      expect(overlaps(await boxOf(previewAvatar(page)), await boxOf(name))).toBe(false);
    });

    test("a long name wraps inside the preview card, clear of the avatar and banner", async ({
      page,
    }) => {
      await openProfileSettings(page);
      await uploadPortrait(page);
      await page.getByLabel("Choose banner image").setInputFiles(BANNER);
      await expect(page.getByText("Profile banner updated")).toBeVisible();
      await saveName(page, LONG_NAME);

      const card = await boxOf(preview(page));
      const name = await boxOf(preview(page).getByText(LONG_NAME, { exact: true }));
      const banner = await boxOf(preview(page).locator(".profile-banner"));

      expect(overlaps(await boxOf(previewAvatar(page)), name)).toBe(false);
      expect(overlaps(banner, name)).toBe(false);
      expect(name.x).toBeGreaterThanOrEqual(card.x);
      expect(name.x + name.width).toBeLessThanOrEqual(card.x + card.width);
    });

    test("someone's portrait photo stays square on their page, in their DM and its details", async ({
      page,
    }) => {
      await page.route(`**/users/${USER_IDS.maya}/avatar*`, (route) =>
        route.fulfill({ contentType: "image/png", body: PORTRAIT.buffer }),
      );

      await openApp(page, `people/${USER_IDS.maya}`);
      await page.locator(".person .profile-hero .avatar img").waitFor();
      await expectSquareAvatars(page);

      await openApp(page, `r/${ROOM_IDS.dmMaya}`);
      await page.locator(".room-header .room-title-button").click();

      const details = page.locator(".details");
      const avatar = details.locator(".profile-hero .avatar img");

      await expect(avatar).toHaveJSProperty("complete", true);
      await expectSquareAvatars(page);
      expect(overlaps(await boxOf(avatar), await boxOf(details.locator(".details-name")))).toBe(
        false,
      );
    });
  });
}
