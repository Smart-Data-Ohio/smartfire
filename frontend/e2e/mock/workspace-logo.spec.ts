import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { Locator, Page } from "@playwright/test";
import { MOCK_DEAD_INVITES, MOCK_INVITE } from "../../mock/s2/join.ts";
import { ROOM_IDS } from "../../mock/seed.ts";
import { rgbaPng } from "./png.ts";
import { DESKTOP, expect, PHONE, SHOTS, SHOTS_DIR, THEMES, type Theme, test } from "./support.ts";

/**
 * The workspace logo is drawn whole wherever it appears: never cropped to a circle, a rounded
 * corner or its box, whatever its shape. Checked with the Smart Data mark itself, a ring that
 * fills its square to the edges, and a wide transparent wordmark.
 */

/** A green ring touching all four edges of a transparent square. */
function ringPng(size: number): Buffer {
  const centre = size / 2;

  return rgbaPng(size, size, (x, y) => {
    const distance = Math.hypot(x + 0.5 - centre, y + 0.5 - centre);

    return distance <= centre && distance >= centre * 0.55 ? [92, 178, 58, 255] : [0, 0, 0, 0];
  });
}

/** A 400 × 120 wordmark: three dark letter blocks and a green bar, edge to edge, on transparency. */
function wordmarkPng(): Buffer {
  return rgbaPng(400, 120, (x, y) => {
    if (y >= 96) return [92, 178, 58, 255];

    return x % 140 < 110 && y >= 8 ? [31, 42, 38, 255] : [0, 0, 0, 0];
  });
}

const LOGOS = [
  {
    label: "the Smart Data mark",
    file: {
      name: "smart-data-logo.png",
      mimeType: "image/png",
      buffer: readFileSync(new URL("../fixtures/images/smart-data-logo.png", import.meta.url)),
    },
  },
  { label: "a ring logo", file: { name: "ring.png", mimeType: "image/png", buffer: ringPng(256) } },
  {
    label: "a wide wordmark",
    file: { name: "wordmark.png", mimeType: "image/png", buffer: wordmarkPng() },
  },
] as const;

/**
 * How far past the colours around it in the copy a pixel may land. A filter on a box around the
 * image (the rail tile brightens under the pointer) clips at white across the whole box, which a
 * filter on the copy alone can't match exactly.
 */
const TOLERANCE = 24;

/**
 * What hides part of the picture `image` draws: its own box under `object-fit`, a rounded corner
 * of the image or of a box around it that clips, or anything that changes how it looks. Empty
 * when the whole picture shows.
 */
async function hiddenParts(image: Locator): Promise<string[]> {
  // `complete` is true for a failed load too, which would leave nothing to measure.
  await image.evaluate((element: HTMLImageElement) => element.decode());

  const natural = await image.evaluate((element: HTMLImageElement) => [
    element.naturalWidth,
    element.naturalHeight,
  ]);

  expect(Math.min(...natural), "the logo's decoded size").toBeGreaterThan(0);
  await image.scrollIntoViewIfNeeded();

  const { hidden, content, picture } = await image.evaluate((element: HTMLImageElement) => {
    const px = (value: string) => Number.parseFloat(value) || 0;
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    const left = box.left + px(style.borderLeftWidth) + px(style.paddingLeft);
    const top = box.top + px(style.borderTopWidth) + px(style.paddingTop);
    const width = box.right - px(style.borderRightWidth) - px(style.paddingRight) - left;
    const height = box.bottom - px(style.borderBottomWidth) - px(style.paddingBottom) - top;
    const fit = style.objectFit;

    const scale = (fit === "cover" ? Math.max : Math.min)(
      width / element.naturalWidth,
      height / element.naturalHeight,
    );

    const drawnWidth = fit === "fill" ? width : element.naturalWidth * scale;
    const drawnHeight = fit === "fill" ? height : element.naturalHeight * scale;
    // Half a pixel in from each edge, so anti-aliasing at a curve isn't counted.
    const x0 = left + (width - drawnWidth) / 2 + 0.5;
    const y0 = top + (height - drawnHeight) / 2 + 0.5;
    const x1 = x0 + drawnWidth - 1;
    const y1 = y0 + drawnHeight - 1;
    const hidden: string[] = [];

    const content = { left, top, width, height };
    const picture = { x: x0 - 0.5, y: y0 - 0.5, width: drawnWidth, height: drawnHeight };

    if (drawnWidth > width + 0.5 || drawnHeight > height + 0.5) {
      return {
        hidden: [`object-fit: ${fit} crops it to its ${width} × ${height} box`],
        content,
        picture,
      };
    }

    const name = (clipper: Element) =>
      clipper === element
        ? "the image's own"
        : `${clipper.localName}${[...clipper.classList].map((c) => `.${c}`).join("")}'s`;

    // A clip's rounded corner, from its border box less `inset` on each side.
    const clips = (clipper: Element, inset: (side: string) => number) => {
      const clipStyle = getComputedStyle(clipper);
      const rect = clipper.getBoundingClientRect();
      const l = rect.left + inset("Left");
      const t = rect.top + inset("Top");
      const r = rect.right - inset("Right");
      const b = rect.bottom - inset("Bottom");

      for (const [corner, x, y, sx, sy, horizontal, vertical] of [
        ["top-left", x0, y0, 1, 1, "Left", "Top"],
        ["top-right", x1, y0, -1, 1, "Right", "Top"],
        ["bottom-left", x0, y1, 1, -1, "Left", "Bottom"],
        ["bottom-right", x1, y1, -1, -1, "Right", "Bottom"],
      ] as const) {
        const [rx = "0", ry = rx] = clipStyle
          .getPropertyValue(`border-${vertical.toLowerCase()}-${horizontal.toLowerCase()}-radius`)
          .split(" ");

        const radius = (value: string, size: number) =>
          value.endsWith("%") ? (px(value) / 100) * size : px(value);

        const radiusX = Math.max(0, radius(rx, rect.width) - inset(horizontal));
        const radiusY = Math.max(0, radius(ry, rect.height) - inset(vertical));

        const cx = (sx === 1 ? l : r) + sx * radiusX;
        const cy = (sy === 1 ? t : b) + sy * radiusY;
        const beyond = x < l - 0.01 || x > r + 0.01 || y < t - 0.01 || y > b + 0.01;

        const rounded =
          radiusX > 0 &&
          radiusY > 0 &&
          sx * (x - cx) < 0 &&
          sy * (y - cy) < 0 &&
          ((x - cx) / radiusX) ** 2 + ((y - cy) / radiusY) ** 2 > 1;

        if (beyond || rounded) {
          hidden.push(`${name(clipper)} ${beyond ? "edge" : "rounded corner"} hides its ${corner}`);
        }
      }
    };

    // A replaced element's content is trimmed to its content-edge curve.
    const borderWidth = (box: CSSStyleDeclaration, side: string) =>
      px(box.getPropertyValue(`border-${side.toLowerCase()}-width`));

    clips(
      element,
      (side) =>
        borderWidth(style, side) + px(style.getPropertyValue(`padding-${side.toLowerCase()}`)),
    );

    for (let parent = element.parentElement; parent !== null; parent = parent.parentElement) {
      const parentStyle = getComputedStyle(parent);

      if (parentStyle.overflowX !== "visible" || parentStyle.overflowY !== "visible") {
        clips(parent, (side) => borderWidth(parentStyle, side));
      }
    }

    return { hidden, content, picture };
  });

  if (hidden.length > 0) return hidden;

  // Anything that changes a pixel of the picture covers it there, however it paints: a box, a
  // pseudo-element, a shadow, text, a clip. The page is compared with itself under a copy of the
  // image drawn on top of everything at the image's own place and size, the image hidden beneath.
  // The copy is absolutely positioned: a fixed one gets its own layer, which scales images less
  // sharply.
  const page = image.page();
  const shot = () => page.screenshot({ clip: picture, animations: "disabled" });
  const shown = await shot();

  const inlineStyle = await image.evaluate((element: HTMLImageElement, box) => {
    const style = getComputedStyle(element);
    const copy = document.createElement("img");
    const filters: string[] = [];
    let opacity = 1;

    // The boxes around the image fade and filter it; nothing else may change it.
    for (let at: Element | null = element; at !== null; at = at.parentElement) {
      const atStyle = getComputedStyle(at);

      opacity *= Number(atStyle.opacity);

      if (atStyle.filter !== "none") filters.push(atStyle.filter);
    }

    for (const [property, value] of Object.entries({
      all: "initial",
      position: "absolute",
      "z-index": "2147483647",
      display: "block",
      left: `${box.left + scrollX}px`,
      top: `${box.top + scrollY}px`,
      width: `${box.width}px`,
      height: `${box.height}px`,
      "object-fit": style.objectFit,
      "object-position": style.objectPosition,
      "image-rendering": style.imageRendering,
      opacity: String(opacity),
      filter: filters.join(" ") || "none",
    })) {
      copy.style.setProperty(property, value, "important");
    }

    copy.src = element.currentSrc;
    copy.dataset.logoCopy = "";
    document.body.append(copy);

    const before = element.style.cssText;

    element.style.setProperty("visibility", "hidden", "important");

    return copy.decode().then(() => before);
  }, content);

  let bare: Buffer;

  try {
    bare = await shot();
  } finally {
    await image.evaluate((element: HTMLImageElement, before) => {
      element.style.cssText = before;
      document.querySelector("[data-logo-copy]")?.remove();
    }, inlineStyle);
  }

  const changed = await page.evaluate(
    async ([a, b, tolerance]) => {
      const pixels = async (png: string) => {
        const bytes = Uint8Array.from(atob(png), (c) => c.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
        const context = new OffscreenCanvas(bitmap.width, bitmap.height).getContext("2d");

        if (context === null) throw new Error("no 2D canvas");
        context.drawImage(bitmap, 0, 0);

        return context.getImageData(0, 0, bitmap.width, bitmap.height);
      };

      const [shown, bare] = await Promise.all([pixels(a), pixels(b)]);
      const { width, height } = shown;

      // The page and the copy scale the image separately, so an edge can land a little
      // differently in each. A pixel changed only when a channel falls outside the range that
      // channel spans within a pixel of it in the copy.
      const changed: [number, number][] = [];

      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          const outside = [0, 1, 2].some((channel) => {
            const near: number[] = [];

            for (let by = Math.max(0, y - 1); by <= Math.min(height - 1, y + 1); by++) {
              for (let bx = Math.max(0, x - 1); bx <= Math.min(width - 1, x + 1); bx++) {
                near.push(bare.data[(by * width + bx) * 4 + channel] ?? 0);
              }
            }

            const value = shown.data[(y * width + x) * 4 + channel] ?? 0;

            return value < Math.min(...near) - tolerance || value > Math.max(...near) + tolerance;
          });

          if (outside) changed.push([x, y]);
        }
      }

      return { count: changed.length, total: width * height, first: changed[0] ?? [0, 0] };
    },
    [shown.toString("base64"), bare.toString("base64"), TOLERANCE] as const,
  );

  return changed.count === 0
    ? []
    : [
        `something over it changes ${changed.count} of its ${changed.total} pixels, the first ${changed.first.join(", ")} device pixels from its top left`,
      ];
}

/** Every check is soft, so one run names each place that crops the logo. */
async function expectWhole(image: Locator, where: string): Promise<void> {
  await expect(image, where).toBeVisible();
  await expect.soft(image, where).toHaveCSS("object-fit", "contain");
  expect.soft(await hiddenParts(image), where).toEqual([]);
}

/** Saves a close-up of `target` and what's around it when `SMARTFIRE_SHOTS=1`. */
async function closeUp(target: Locator, name: string, theme: Theme): Promise<void> {
  if (!SHOTS) return;

  const box = await target.boundingBox();

  if (box === null) throw new Error(`${name} isn't on screen`);

  mkdirSync(SHOTS_DIR, { recursive: true });
  await target.page().mouse.move(0, 0);
  await target.page().screenshot({
    path: join(SHOTS_DIR, `${name}-${theme}.png`),
    animations: "disabled",
    clip: { x: box.x - 12, y: box.y - 12, width: box.width + 24, height: box.height + 24 },
  });
}

const railTile = (page: Page) =>
  page.getByRole("navigation", { name: "Destinations" }).getByRole("link", { name: "Smart Data" });

async function controlWelcome(page: Page): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.post("/__mock/welcome", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: {
      message: "Say hi in #general.",
      rooms: [{ roomId: ROOM_IDS.general, emoji: null, description: "Everyone's here" }],
      guide: [],
    },
  });

  expect(response.ok()).toBe(true);
}

test.use({ deviceScaleFactor: 2 });

// Each test visits a dozen pages; the first also waits for the dev server's first compile.
test.describe.configure({ timeout: 120_000 });

for (const { label, file } of LOGOS) {
  test(`${label} shows whole wherever the workspace logo is drawn`, async ({ page }) => {
    const slug = file.name.replace(/\.png$/, "");

    await page.setViewportSize(DESKTOP);
    await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
    await page.goto("/app/admin/workspace");
    await page.getByRole("heading", { level: 2, name: "Workspace profile" }).waitFor();
    await page.getByLabel("Choose icon image").setInputFiles(file);
    await expect(page.getByText("Icon updated")).toBeVisible();
    await expect(railTile(page).locator("img")).toHaveJSProperty("complete", true);

    for (const theme of THEMES) {
      await page.emulateMedia({ colorScheme: theme });
      await closeUp(railTile(page), `rail-${slug}`, theme);
      await closeUp(page.locator(".profile-preview"), `admin-preview-${slug}`, theme);
      await closeUp(page.locator(".profile-invite-preview"), `admin-invite-${slug}`, theme);
    }

    await expectWhole(railTile(page).locator("img"), "rail tile");
    await expectWhole(page.locator(".profile-preview-icon img"), "admin preview's rail icon");
    await expectWhole(page.locator(".profile-invite-card img"), "admin invite preview");
    await expectWhole(
      page.locator('.profile-slot[data-kind="logo"] .profile-slot-thumb img'),
      "admin icon slot thumbnail",
    );

    await railTile(page).hover();
    await expectWhole(railTile(page).locator("img"), "rail tile under the pointer");

    await controlWelcome(page);
    await page.goto("/app/");
    await expectWhole(page.locator(".welcome-card-logo"), "welcome card");

    for (const theme of THEMES) {
      await page.emulateMedia({ colorScheme: theme });
      await page.goto("/app/session/new");
      await closeUp(page.locator(".auth-view-logo"), `sign-in-${slug}`, theme);
    }

    await page.emulateMedia({ colorScheme: "light" });
    await expectWhole(page.locator(".auth-view-logo"), "sign-in page");
    await page.goto(`/app/invite/${MOCK_INVITE}`);
    await expectWhole(page.locator(".auth-view-logo"), "invite page");

    for (const [refusal, token] of Object.entries(MOCK_DEAD_INVITES)) {
      await page.goto(`/app/invite/${token}`);
      const mark = page.locator(".auth-view-invite-mark");

      for (const theme of THEMES) {
        await page.emulateMedia({ colorScheme: theme });
        await closeUp(mark, `dead-invite-${refusal}-${slug}`, theme);
      }

      await page.emulateMedia({ colorScheme: "light" });
      await expectWhole(mark.locator(".auth-view-logo"), `${refusal} invite page`);
    }

    await page.setViewportSize(PHONE);
    await page.goto("/app/");

    for (const theme of THEMES) {
      await page.emulateMedia({ colorScheme: theme });
      await closeUp(page.locator(".sidebar-workspace"), `phone-sidebar-${slug}`, theme);
    }

    await expectWhole(page.locator(".sidebar-workspace-logo"), "phone sidebar header");

    for (const [refusal, token] of Object.entries(MOCK_DEAD_INVITES)) {
      await page.goto(`/app/invite/${token}`);
      await expectWhole(
        page.locator(".auth-view-invite-mark .auth-view-logo"),
        `${refusal} invite page on a phone`,
      );
    }
  });
}
