import { crc32, deflateSync } from "node:zlib";
import type { Page } from "@playwright/test";
import { expect, matrix, openApp, shot, syncWelcomed, test } from "./support.ts";

/** One PNG chunk: length, type, data and the CRC over type and data. */
function chunk(type: string, data: Buffer): Buffer {
  const length = Buffer.alloc(4);
  const crc = Buffer.alloc(4);
  const body = Buffer.concat([Buffer.from(type, "latin1"), data]);

  length.writeUInt32BE(data.length);
  crc.writeUInt32BE(crc32(body));

  return Buffer.concat([length, body, crc]);
}

/** A `width` × `height` PNG shading diagonally from one colour to another, for a logo or banner. */
function gradientPng(
  width: number,
  height: number,
  from: readonly [number, number, number],
  to: readonly [number, number, number],
): Buffer {
  const header = Buffer.alloc(13);

  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 2, 0, 0, 0], 8);

  const rows = Buffer.alloc((width * 3 + 1) * height);

  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 3 + 1);

    for (let x = 0; x < width; x += 1) {
      const t = (x / width + y / height) / 2;

      for (let channel = 0; channel < 3; channel += 1) {
        const start = from[channel] ?? 0;
        const end = to[channel] ?? 0;

        rows[row + 1 + x * 3 + channel] = Math.round(start + (end - start) * t);
      }
    }
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

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

/** Opens the workspace page, inside the app on a desktop. */
async function openProfile(page: Page, theme: "light" | "dark" = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/admin");
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
  await upload(page, "icon", { name: "logo.gif", mimeType: "image/gif", buffer: LOGO.buffer });

  const images = railTile(page).locator("img");

  await expect(images).toHaveCount(1);
  await expect(images.first()).toHaveAttribute("src", /\/representations\//);

  await railTile(page).hover();
  await expect(railTile(page).locator("img[data-animated]")).toHaveAttribute("src", /\/blobs\//);

  await page.mouse.move(600, 600);
  await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
});

test("the banner folds into the plain header as the list scrolls", async ({ page }) => {
  await openProfile(page);
  await upload(page, "banner", BANNER);
  await page.goto("/app/");

  const header = sidebarHeader(page);
  const scroller = page.locator(".sidebar-scroll");

  await expect(header).not.toHaveAttribute("data-folded");
  await scroller.evaluate((element) => {
    element.scrollTop = 200;
  });
  await expect(header).toHaveAttribute("data-folded", "");
  await scroller.evaluate((element) => {
    element.scrollTop = 0;
  });
  await expect(header).not.toHaveAttribute("data-folded");
});

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
