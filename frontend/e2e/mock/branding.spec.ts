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

type Rgb = readonly [number, number, number];

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** An RGB PNG's IHDR chunk. */
function pngHeader(width: number, height: number): Buffer {
  const header = Buffer.alloc(13);

  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 2, 0, 0, 0], 8);

  return chunk("IHDR", header);
}

/** A PNG's compressed scanlines, shading diagonally from one colour to another. */
function gradientRows(width: number, height: number, from: Rgb, to: Rgb): Buffer {
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

  return deflateSync(rows);
}

/** A `width` × `height` PNG shading diagonally from one colour to another, for a logo or banner. */
function gradientPng(width: number, height: number, from: Rgb, to: Rgb): Buffer {
  return Buffer.concat([
    PNG_SIGNATURE,
    pngHeader(width, height),
    chunk("IDAT", gradientRows(width, height, from, to)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** An APNG frame control chunk: the whole canvas, shown for a tenth of a second. */
function frameControl(sequence: number, width: number, height: number): Buffer {
  const data = Buffer.alloc(26);

  data.writeUInt32BE(sequence, 0);
  data.writeUInt32BE(width, 4);
  data.writeUInt32BE(height, 8);
  data.writeUInt16BE(1, 20);
  data.writeUInt16BE(10, 22);

  return chunk("fcTL", data);
}

/** A two-frame animated PNG, which the server treats as a still PNG. */
function twoFrameApng(width: number, height: number): Buffer {
  const control = Buffer.alloc(8);

  control.writeUInt32BE(2, 0);

  const sequence = Buffer.alloc(4);

  sequence.writeUInt32BE(2, 0);

  return Buffer.concat([
    PNG_SIGNATURE,
    pngHeader(width, height),
    chunk("acTL", control),
    frameControl(0, width, height),
    chunk("IDAT", gradientRows(width, height, [255, 122, 69], [168, 38, 120])),
    frameControl(1, width, height),
    chunk(
      "fdAT",
      Buffer.concat([sequence, gradientRows(width, height, [39, 92, 196], [24, 180, 160])]),
    ),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * A real two-frame looping GIF, each frame one solid colour. Its LZW stream clears before every
 * pixel, so codes stay 3 bits wide and need no table.
 */
function twoFrameGif(width: number, height: number): Buffer {
  const screen = Buffer.alloc(7);

  screen.writeUInt16LE(width, 0);
  screen.writeUInt16LE(height, 2);
  screen[4] = 0xf0;

  const frame = (index: number): Buffer => {
    const codes: number[] = [];

    for (let pixel = 0; pixel < width * height; pixel += 1) codes.push(4, index);

    codes.push(5);

    const packed: number[] = [];
    let bits = 0;
    let filled = 0;

    for (const code of codes) {
      bits |= code << filled;
      filled += 3;

      while (filled >= 8) {
        packed.push(bits & 0xff);
        bits >>= 8;
        filled -= 8;
      }
    }

    if (filled > 0) packed.push(bits & 0xff);

    const blocks: number[] = [];

    for (let at = 0; at < packed.length; at += 255) {
      const block = packed.slice(at, at + 255);

      blocks.push(block.length, ...block);
    }

    const descriptor = Buffer.alloc(10);

    descriptor[0] = 0x2c;
    descriptor.writeUInt16LE(width, 5);
    descriptor.writeUInt16LE(height, 7);

    return Buffer.concat([
      Buffer.from([0x21, 0xf9, 0x04, 0x00, 10, 0, 0, 0]),
      descriptor,
      Buffer.from([2, ...blocks, 0]),
    ]);
  };

  return Buffer.concat([
    Buffer.from("GIF89a", "latin1"),
    screen,
    Buffer.from([255, 122, 69, 39, 92, 196]),
    Buffer.from([0x21, 0xff, 0x0b, ...Buffer.from("NETSCAPE2.0", "latin1"), 3, 1, 0, 0, 0]),
    frame(0),
    frame(1),
    Buffer.from([0x3b]),
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

const ANIMATED_LOGO = { name: "logo.gif", mimeType: "image/gif", buffer: twoFrameGif(64, 64) };

const ANIMATED_BANNER = {
  name: "banner.gif",
  mimeType: "image/gif",
  buffer: twoFrameGif(160, 90),
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
  await upload(page, "icon", ANIMATED_LOGO);

  const images = railTile(page).locator("img");

  await expect(images).toHaveCount(1);
  await expect(images.first()).toHaveAttribute("src", /\/representations\//);

  await railTile(page).hover();
  await expect(railTile(page).locator("img[data-animated]")).toHaveAttribute("src", /\/blobs\//);

  await page.mouse.move(600, 600);
  await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
});

test("under reduced motion an animated icon never plays, even while uploading", async ({
  page,
}) => {
  await openProfile(page);

  // Hold the upload's bytes so the preview shows the file being uploaded.
  const { promise: held, resolve: release } = Promise.withResolvers<void>();

  await page.route("**/rails/active_storage/disk/**", async (route) => {
    await held;
    await route.continue();
  });
  await page.getByLabel("Choose icon image").setInputFiles(ANIMATED_LOGO);

  // The preview draws the file's first frame: a PNG, not the GIF itself.
  const preview = page.locator(".profile-preview-icon img");

  await expect(preview).toHaveAttribute("src", /^blob:/);
  expect(
    await preview.evaluate(async (image: HTMLImageElement) => {
      const bytes = new Uint8Array(await (await fetch(image.src)).arrayBuffer());

      return String.fromCharCode(...bytes.subarray(1, 4));
    }),
  ).toBe("PNG");

  release();
  await expect(page.getByText("Icon updated")).toBeVisible();
  await expect(preview).toHaveAttribute("src", /\/representations\//);

  await railTile(page).hover();
  await expect(railTile(page).locator("img")).toHaveCount(1);
  await expect(railTile(page).locator("img[data-animated]")).toHaveCount(0);
});

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

  const refusedWith = async (noun: "icon" | "banner", file: typeof LOGO, message: string) => {
    await page.getByLabel(`Choose ${noun} image`).setInputFiles(file);
    await expect(page.getByRole("alert").filter({ hasText: message })).toBeVisible();
  };

  // Text with an image type, and a GIF whose blocks are broken.
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
