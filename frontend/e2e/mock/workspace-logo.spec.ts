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
 * What hides part of the picture `image` draws: its own box under `object-fit`, a rounded corner
 * of the image or of a box around it that clips, or anything painted over it. Empty when the
 * whole picture shows.
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

  return image.evaluate((element: HTMLImageElement) => {
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

    if (drawnWidth > width + 0.5 || drawnHeight > height + 0.5) {
      return [`object-fit: ${fit} crops it to its ${width} × ${height} box`];
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

    // Whether a box, or its ::before or ::after, paints anything: a transparent wrapper doesn't.
    const clear = (colour: string) => colour === "transparent" || /[,/]\s*0\)$/.test(colour);

    const paints = (box: CSSStyleDeclaration) =>
      box.opacity !== "0" &&
      box.visibility !== "hidden" &&
      (!clear(box.backgroundColor) ||
        box.backgroundImage !== "none" ||
        ["Top", "Right", "Bottom", "Left"].some(
          (side) =>
            borderWidth(box, side) > 0 &&
            !clear(box.getPropertyValue(`border-${side.toLowerCase()}-color`)),
        ));

    const pseudoPaints = (box: Element) =>
      ["::before", "::after"].some((pseudo) => {
        const pseudoStyle = getComputedStyle(box, pseudo);

        return (
          pseudoStyle.content !== "none" &&
          (paints(pseudoStyle) || /^".+"$/.test(pseudoStyle.content))
        );
      });

    const ownPaints = (box: Element) =>
      paints(getComputedStyle(box)) ||
      ["img", "svg", "canvas", "video"].includes(box.localName) ||
      [...box.childNodes].some((node) => node.nodeType === 3 && node.textContent?.trim());

    // Anything hit before the image at a point of its picture is painted over it there. Every
    // box is made hittable for the check, so `pointer-events: none` can't hide an overlay.
    const hittable = document.createElement("style");
    hittable.textContent = "* { pointer-events: auto !important; }";
    document.head.append(hittable);

    try {
      for (let i = 0; i <= 8; i++) {
        for (let j = 0; j <= 8; j++) {
          const x = x0 + ((x1 - x0) * i) / 8;
          const y = y0 + ((y1 - y0) * j) / 8;
          const stack = document.elementsFromPoint(x, y);
          const at = stack.indexOf(element);

          if (at === -1) {
            hidden.push("part of it is off screen or clipped away");
            continue;
          }

          for (const over of stack.slice(0, at)) {
            const covers = over.contains(element)
              ? pseudoPaints(over)
              : ownPaints(over) || pseudoPaints(over);

            if (covers) hidden.push(`${name(over).replace(/'s$/, "")} covers part of it`);
          }
        }
      }
    } finally {
      hittable.remove();
    }

    return [...new Set(hidden)];
  });
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
  });
}
