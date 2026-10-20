import type { Page } from "@playwright/test";
import { gradientPng, twoFrameApng, twoFrameGif } from "./png.ts";
import { expect, matrix, openApp, SHOTS, shot, syncWelcomed, test } from "./support.ts";

const LOGO = {
  name: "logo.png",
  mimeType: "image/png",
  buffer: gradientPng(128, 128, [255, 122, 69], [168, 38, 120]),
};

const BANNER = {
  name: "banner.png",
  mimeType: "image/png",
  buffer: gradientPng(480, 270, [39, 92, 196], [24, 180, 160]),
};

const ANIMATED_LOGO = { name: "logo.gif", mimeType: "image/gif", buffer: twoFrameGif(64, 64) };

const ANIMATED_BANNER = {
  name: "banner.gif",
  mimeType: "image/gif",
  buffer: twoFrameGif(160, 90),
};

/** Opens the workspace page, inside the app on a desktop. */
async function openProfile(page: Page, theme: "light" | "dark" = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  // The workspace section's own page: on phones `/app/admin` is the list of sections.
  await page.goto("/app/admin/workspace");
  await page.getByRole("heading", { level: 2, name: "Workspace profile" }).waitFor();
}

function railTile(page: Page) {
  return page
    .getByRole("navigation", { name: "Destinations" })
    .getByRole("link", { name: "Smart Data" });
}

function sidebarHeader(page: Page) {
  return page.getByRole("complementary", { name: "Conversations" }).locator(".sidebar-header");
}

async function upload(page: Page, noun: "icon" | "banner", file: typeof LOGO): Promise<void> {
  await page.getByLabel(`Choose ${noun} image`).setInputFiles(file);
  await expect(page.getByText(`${noun === "icon" ? "Icon" : "Banner"} updated`)).toBeVisible();
}

test("an uploaded icon and banner reach the rail and sidebar, and removing them goes back", async ({
  page,
}) => {
  await openProfile(page);
  await expect(railTile(page)).toHaveText("SD");
  await expect(sidebarHeader(page)).not.toHaveAttribute("data-banner");

  await upload(page, "icon", LOGO);
  await expect(railTile(page).locator("img")).toHaveAttribute("src", /\/blobs\//);
  await expect(page.getByRole("button", { name: "Replace icon" })).toBeVisible();

  await upload(page, "banner", BANNER);
  await expect(sidebarHeader(page)).toHaveAttribute("data-banner", "");
  await expect(sidebarHeader(page).locator(".sidebar-banner-image")).toHaveAttribute(
    "src",
    /\/blobs\//,
  );

  // The rail and header keep them after a reload: boot carries them.
  await page.reload();
  await expect(railTile(page).locator("img")).toBeVisible();
  await expect(sidebarHeader(page).locator(".sidebar-banner-image")).toBeVisible();

  await page.getByRole("button", { name: "Remove icon" }).click();
  await expect(railTile(page)).toHaveText("SD");
  await page.getByRole("button", { name: "Remove banner" }).click();
  await expect(sidebarHeader(page)).not.toHaveAttribute("data-banner");
  await expect(page.getByRole("button", { name: "Upload banner" })).toBeVisible();
});

test("another open tab picks up the new icon and banner live", async ({ browser, page }) => {
  const other = await browser.newPage();
  const welcomed = syncWelcomed(other);

  await openApp(other, "");
  await welcomed;

  await openProfile(page);
  await upload(page, "icon", LOGO);
  await upload(page, "banner", BANNER);

  await expect(railTile(other).locator("img")).toBeVisible();
  await expect(sidebarHeader(other)).toHaveAttribute("data-banner", "");

  await page.getByRole("button", { name: "Remove banner" }).click();
  await expect(sidebarHeader(other)).not.toHaveAttribute("data-banner");
  await other.close();
});

test("a file dropped on a slot uploads, and a refused one says why in place", async ({ page }) => {
  await openProfile(page);

  const slot = page.locator(".profile-slot", { has: page.getByRole("heading", { name: "Icon" }) });

  const transfer = await page.evaluateHandle(
    (bytes) => {
      const data = new DataTransfer();

      data.items.add(new File([new Uint8Array(bytes)], "dropped.png", { type: "image/png" }));

      return data;
    },
    [...LOGO.buffer],
  );

  await slot.dispatchEvent("dragenter", { dataTransfer: transfer });
  await expect(slot).toHaveAttribute("data-over", "true");
  await slot.dispatchEvent("drop", { dataTransfer: transfer });
  await expect(page.getByText("Icon updated")).toBeVisible();
  await expect(railTile(page).locator("img")).toBeVisible();

  await page.getByLabel("Choose banner image").setInputFiles({
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("not an image"),
  });
  await expect(
    page.getByRole("alert").filter({ hasText: "Choose a PNG, JPEG, GIF or WebP image." }),
  ).toBeVisible();
});

test("an animated icon rests on its first frame and plays while pointed at", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/app/admin");
  await page.getByRole("heading", { level: 2, name: "Workspace profile" }).waitFor();
  await upload(page, "icon", ANIMATED_LOGO);

  const images = railTile(page).locator("img");

  await expect(images).toHaveCount(1);
  await expect(images.first()).toHaveAttribute("src", /\/representations\//);

  await railTile(page).hover();
  await expect(railTile(page).locator("img[data-animated]")).toHaveAttribute("src", /\/blobs\//);

  await page.mouse.move(600, 600);
  await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
});

for (const [label, file] of [
  ["GIF", ANIMATED_LOGO],
  ["APNG", { name: "logo.png", mimeType: "image/png", buffer: twoFrameApng(64, 64) }],
] as const) {
  test(`under reduced motion an animated ${label} icon never plays, even while uploading`, async ({
    page,
  }) => {
    await openProfile(page);

    // Hold the upload's bytes so the preview shows the file being uploaded.
    const { promise: held, resolve: release } = Promise.withResolvers<void>();

    await page.route("**/rails/active_storage/disk/**", async (route) => {
      await held;
      await route.continue();
    });
    await page.getByLabel("Choose icon image").setInputFiles(file);

    // The preview draws the file's first frame: a plain PNG, not the animated file itself.
    const preview = page.locator(".profile-preview-icon img");

    await expect(preview).toHaveAttribute("src", /^blob:/);

    const drawn = await preview.evaluate(async (image: HTMLImageElement) => {
      const bytes = new Uint8Array(await (await fetch(image.src)).arrayBuffer());
      const text = String.fromCharCode(...bytes);

      return { png: text.slice(1, 4) === "PNG", animated: text.includes("acTL") };
    });

    expect(drawn).toEqual({ png: true, animated: false });

    release();
    await expect(page.getByText("Icon updated")).toBeVisible();

    await railTile(page).hover();
    await expect(railTile(page).locator("img")).toHaveCount(1);
    await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
  });
}

test("an animated PNG is taken as a still image", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/app/admin");
  await page.getByRole("heading", { level: 2, name: "Workspace profile" }).waitFor();
  await upload(page, "icon", {
    name: "logo.png",
    mimeType: "image/png",
    buffer: twoFrameApng(64, 64),
  });

  await expect(railTile(page).locator("img")).toHaveAttribute("src", /\/blobs\//);
  await railTile(page).hover();
  await expect(railTile(page).locator("img")).toHaveCount(1);
  await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
});

test("files that aren't real images, or are too big, are refused in place", async ({ page }) => {
  await openProfile(page);

  // Each picker keeps its own refusal until its next file, so look in the picker under test: a
  // page-wide match would find the other picker's alert too, or pass on it before this one renders.
  const refusedWith = async (noun: "icon" | "banner", file: typeof LOGO, message: string) => {
    const input = page.getByLabel(`Choose ${noun} image`);
    const slot = page.locator(".profile-slot").filter({ has: input });

    await input.setInputFiles(file);
    await expect(slot.getByRole("alert")).toContainText(message);
  };

  // Text with an image type, and a GIF that stops before its first frame.
  await refusedWith(
    "icon",
    { name: "logo.png", mimeType: "image/png", buffer: Buffer.from("not really a png") },
    "must be a PNG, JPEG, GIF or WebP image",
  );
  await refusedWith(
    "banner",
    {
      name: "banner.gif",
      mimeType: "image/gif",
      buffer: Buffer.concat([ANIMATED_BANNER.buffer.subarray(0, 40), Buffer.from([0x99, 1, 2])]),
    },
    "must be a PNG, JPEG, GIF or WebP image",
  );

  // Past the pixel caps: 4096 × 4096 for the icon, 4096 × 2304 for the banner.
  await refusedWith(
    "icon",
    { name: "huge.png", mimeType: "image/png", buffer: gradientPng(5000, 4, [0, 0, 0], [9, 9, 9]) },
    "must be at most 4096 × 4096 pixels",
  );
  await refusedWith(
    "banner",
    { name: "wide.png", mimeType: "image/png", buffer: gradientPng(4097, 4, [0, 0, 0], [9, 9, 9]) },
    "must be at most 4096 × 2304 pixels",
  );

  await expect(page.getByRole("button", { name: "Upload icon" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Upload banner" })).toBeVisible();
  await expect(railTile(page)).toHaveText("SD");
  await expect(sidebarHeader(page)).not.toHaveAttribute("data-banner");
});

test("a GIF broken after its first frame is kept as a still banner", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/app/admin");
  await page.getByRole("heading", { level: 2, name: "Workspace profile" }).waitFor();
  await upload(page, "banner", {
    ...ANIMATED_BANNER,
    buffer: ANIMATED_BANNER.buffer.subarray(0, ANIMATED_BANNER.buffer.length - 100),
  });

  await page.goto("/app/");

  const header = sidebarHeader(page);
  const image = header.locator(".sidebar-banner-image");

  await expect(header).toHaveAttribute("data-banner", "");
  await expect(image).toHaveAttribute("src", /\/blobs\//);
  // No first frame was made for it: folded, it keeps the same (still) image.
  await page.locator(".sidebar-scroll").evaluate((element) => {
    element.scrollTop = 200;
  });
  await expect(header).toHaveAttribute("data-folded", "");
  await expect(image).toHaveAttribute("src", /\/blobs\//);
});

test("a banner that fails to load leaves the plain header", async ({ page }) => {
  await openProfile(page);
  await upload(page, "banner", BANNER);
  await expect(sidebarHeader(page)).toHaveAttribute("data-banner", "");

  await page.route(/\/banner\.png(\?|$)/, (route) => route.fulfill({ status: 404 }));
  await page.goto("/app/");
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await expect(sidebarHeader(page)).not.toHaveAttribute("data-banner");
  await expect(sidebarHeader(page).locator(".sidebar-banner")).toHaveCount(0);
});

test("the banner folds into the plain header as the list scrolls", async ({ page }) => {
  await openProfile(page);
  await upload(page, "banner", ANIMATED_BANNER);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/app/");

  const header = sidebarHeader(page);
  const image = header.locator(".sidebar-banner-image");
  const scroller = page.locator(".sidebar-scroll");

  await expect(header).not.toHaveAttribute("data-folded");
  await expect(image).toHaveAttribute("src", /\/blobs\//);
  await scroller.evaluate((element) => {
    element.scrollTop = 200;
  });
  await expect(header).toHaveAttribute("data-folded", "");
  // Folded away, an animated banner rests on its first frame.
  await expect(image).toHaveAttribute("src", /\/representations\//);
  await scroller.evaluate((element) => {
    element.scrollTop = 0;
  });
  await expect(header).not.toHaveAttribute("data-folded");
  await expect(image).toHaveAttribute("src", /\/blobs\//);
});

// Screenshots only: the first test covers the rail and sidebar.
if (SHOTS) {
  matrix("the workspace profile", async ({ page, theme, phone }) => {
    await openProfile(page, theme);
    await upload(page, "icon", LOGO);
    await upload(page, "banner", BANNER);
    await page.mouse.move(0, 0);
    await shot(page, "branding-admin", theme);

    await page.goto("/app/");
    await page.getByRole("complementary", { name: "Conversations" }).waitFor();
    await expect(sidebarHeader(page).locator(".sidebar-banner-image")).toBeVisible();

    if (!phone) {
      await expect(railTile(page).locator("img")).toBeVisible();
    }

    await page.mouse.move(0, 0);
    await shot(page, "branding-sidebar", theme);
  });
}
