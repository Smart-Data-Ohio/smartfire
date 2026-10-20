import type { Page } from "@playwright/test";
import { gradientPng } from "./png.ts";
import { expect, openApp, test, USER_IDS } from "./support.ts";

/** Where the injected banner of someone else is served from. */
const SERVED = "/e2e/maya-banner.png";

const BANNER = {
  name: "banner.png",
  mimeType: "image/png",
  buffer: gradientPng(600, 200, [39, 92, 196], [24, 180, 160]),
};

async function openSettings(page: Page): Promise<void> {
  await openApp(page, "settings/profile");
  await page.getByRole("heading", { name: "Profile banner" }).waitFor();
}

function settingsBanner(page: Page) {
  return page.getByRole("figure", { name: "Profile preview" }).locator(".profile-banner");
}

function personBanner(page: Page) {
  return page.locator(".person .profile-banner");
}

/** Maya's person page, her profile carrying `bannerUrl` as the server would send it. */
async function openMaya(page: Page, bannerUrl: string | null): Promise<void> {
  await page.route(`**${SERVED}`, (route) =>
    route.fulfill({ contentType: "image/png", body: BANNER.buffer }),
  );
  await page.route(`**/api/v1/people/${USER_IDS.maya}`, async (route) => {
    const response = await route.fetch();
    const profile = await response.json();

    profile.user = {
      ...profile.user,
      bannerUrl,
      bannerStillUrl: null,
      updatedAt: new Date(Date.parse(profile.user.updatedAt) + 1000)
        .toISOString()
        .replace("Z", "000Z"),
    };
    await route.fulfill({ response, json: profile });
  });
  await openApp(page, `people/${USER_IDS.maya}`);
  await page.locator(".person .profile-hero").waitFor();
}

test("a profile banner uploads in settings, previews over the avatar and is removed", async ({
  page,
}) => {
  await openSettings(page);
  await expect(settingsBanner(page)).toHaveAttribute("data-empty", "true");
  await page.getByLabel("Choose banner image").setInputFiles(BANNER);
  await expect(page.getByText("Profile banner updated")).toBeVisible();

  const preview = settingsBanner(page).locator("img");

  await expect(preview).toHaveAttribute("src", /\/blobs\//);
  await expect(preview).toHaveAttribute("alt", /'s profile banner$/);
  await expect
    .poll(() => preview.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);
  await expect(page.getByRole("button", { name: "Replace banner" })).toBeVisible();

  await page.reload();
  await expect(settingsBanner(page).locator("img")).toHaveAttribute("src", /\/blobs\//);
  await page.getByRole("button", { name: "Remove banner" }).click();
  await expect(settingsBanner(page)).toHaveAttribute("data-empty", "true");
  await expect(page.getByRole("button", { name: "Upload banner" })).toBeVisible();
  await page.reload();
  await expect(settingsBanner(page)).toHaveAttribute("data-empty", "true");
});

test("files that aren't real images, or are too wide, are refused in place", async ({ page }) => {
  await openSettings(page);
  await page.getByLabel("Choose banner image").setInputFiles({
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("plain words"),
  });
  await expect(page.getByRole("alert")).toContainText("Choose a PNG, JPEG, GIF or WebP image.");
  await page.getByLabel("Choose banner image").setInputFiles({
    name: "wide.png",
    mimeType: "image/png",
    buffer: gradientPng(4097, 4, [0, 0, 0], [9, 9, 9]),
  });
  await expect(page.getByRole("alert")).toContainText("4096");
  await expect(settingsBanner(page)).toHaveAttribute("data-empty", "true");
});

test("someone's banner runs above their avatar on their page", async ({ page }) => {
  await openMaya(page, SERVED);

  const banner = personBanner(page).locator("img");

  await expect(banner).toHaveAttribute("alt", "Maya Okafor's profile banner");
  await expect
    .poll(() => banner.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);
});

test("without a banner, or with one that won't load, the strip takes the avatar's tint", async ({
  page,
}) => {
  await openMaya(page, "/e2e/missing-banner.png");
  await expect(personBanner(page)).toHaveAttribute("data-empty", "true");
  await expect(personBanner(page).locator("img")).toHaveCount(0);
});

test("on a phone the banner spans the column with the avatar over its edge", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openMaya(page, SERVED);

  const strip = personBanner(page);

  await expect(strip.locator("img")).toBeVisible();

  const banner = await strip.boundingBox();
  const avatar = await page.locator(".person .profile-hero-avatar").boundingBox();

  expect(banner).not.toBeNull();
  expect(avatar).not.toBeNull();

  if (banner === null || avatar === null) {
    return;
  }

  expect(banner.x).toBeGreaterThanOrEqual(0);
  expect(banner.x + banner.width).toBeLessThanOrEqual(390);
  expect(banner.width).toBeGreaterThan(300);
  // The avatar straddles the banner's lower edge.
  expect(avatar.y).toBeLessThan(banner.y + banner.height);
  expect(avatar.y + avatar.height).toBeGreaterThan(banner.y + banner.height);
});
