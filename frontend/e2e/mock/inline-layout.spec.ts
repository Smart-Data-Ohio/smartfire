import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import { DESKTOP, expect, PHONE, ROOM_IDS, shot, type Theme, test, USER_IDS } from "./support.ts";

/**
 * Icon-and-label rows, inline images and the reaction row's rhythm. Production once shipped the
 * component layer under the reset (a shared chunk's stylesheet named `ui` before the entry's
 * order statement), so every icon stacked above its label. These checks hold the geometry, and
 * CI also runs this file against the production build (`SMARTFIRE_E2E_BUILD=1`, which serves
 * `vite build` through `vite preview` with the same mock).
 */

type Box = { x: number; y: number; width: number; height: number };

async function box(locator: Locator): Promise<Box> {
  const found = await locator.boundingBox();

  expect(found, "the element has a box").not.toBeNull();

  return found ?? { x: 0, y: 0, width: 0, height: 0 };
}

/** `icon` sits on the same line as `label`, to its left (not stacked above it). */
async function expectSameRow(icon: Locator, label: Locator): Promise<void> {
  const a = await box(icon);
  const b = await box(label);

  expect(Math.abs(a.y + a.height / 2 - (b.y + b.height / 2))).toBeLessThanOrEqual(3);
  expect(a.x + a.width).toBeLessThanOrEqual(b.x + 1);
}

function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-row][data-message-id="${messageId}"]`).first();
}

async function openOn(page: Page, messageId: number, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}/m/${messageId}`);
  await expect(row(page, messageId)).toBeVisible();
}

/** Has Jonah post `markdown` in #general and returns the message id. */
async function post(request: APIRequestContext, markdown: string): Promise<number> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post("/__mock/post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { roomId: ROOM_IDS.general, userId: USER_IDS.jonah, markdown },
  });

  return (await response.json()).id;
}

/** The remote image a rich-text body carries (attachables.rs): a big transparent logo. */
const IMAGE_BODY =
  '<p>The logo</p><figure class="attachment attachment--preview">' +
  '<img width="1200" height="800" src="/__test/logo.svg" /></figure>';

/** A page of #general's messages with message `id`'s body swapped for `IMAGE_BODY`. */
function withImage(page: MessagePage, id: number): MessagePage {
  return {
    ...page,
    messages: page.messages.map((message) =>
      message.id === id ? { ...message, bodyHtml: IMAGE_BODY } : message,
    ),
  };
}

for (const theme of ["light", "dark"] as const) {
  test(`icons share a row with their labels (${theme})`, async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await page.context().grantPermissions(["microphone", "camera"]);
    await openOn(page, MESSAGE_IDS.generalReactions, theme);

    const jump = page.locator(".sidebar-jump");

    await expectSameRow(jump.locator("svg").first(), jump.locator(".sidebar-jump-label"));
    await expectSameRow(jump.locator(".sidebar-jump-label"), jump.locator(".sidebar-jump-kbd"));
    expect((await box(jump)).height).toBeLessThanOrEqual(32);

    const workspace = page.locator(".sidebar-workspace");

    await expectSameRow(workspace.locator(".sidebar-workspace-name"), workspace.locator("svg"));

    const huddle = page.locator(".room-header .huddle-launcher");

    await expectSameRow(huddle.locator("svg").first(), huddle.locator(".huddle-launcher-label"));
    expect((await box(huddle)).height).toBeLessThanOrEqual(32);

    const chip = row(page, MESSAGE_IDS.generalReactions).locator(".button.reaction").first();

    await expectSameRow(chip.locator(".reaction-glyph"), chip.locator(".reaction-count"));
    expect((await box(chip)).height).toBeLessThanOrEqual(26);

    await shot(page, "inline-layout", theme);
  });

  test(`the new-messages pill keeps its arrow beside its label (${theme})`, async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    // #general opens at its first unread, far above the present: the pill shows.
    await page.goto(`/app/r/${ROOM_IDS.general}`);

    const jump = page.locator(".timeline-jump");
    const pill = jump.locator(".button");

    await expect(jump).toHaveAttribute("data-open", "true");
    await expect(pill).toBeVisible();

    // The arrow sits inside the label (Button puts the icon there): one line, not stacked.
    const icon = await box(pill.locator("svg").first());
    const label = await box(pill.locator(".button-label"));

    expect(Math.abs(icon.y + icon.height / 2 - (label.y + label.height / 2))).toBeLessThanOrEqual(
      3,
    );
    expect(label.width).toBeGreaterThan(icon.width * 3);
    expect(label.height).toBeLessThanOrEqual(icon.height + 8);
    expect((await box(pill)).height).toBeLessThanOrEqual(32);
  });

  test(`the reaction row keeps the accessory gap and lines up with the body (${theme})`, async ({
    page,
  }) => {
    await page.setViewportSize(DESKTOP);
    await openOn(page, MESSAGE_IDS.generalChart, theme);

    const target = row(page, MESSAGE_IDS.generalChart);
    const body = await box(target.locator(".message-body"));
    const image = await box(target.locator(".attachment-image"));
    const reactions = await box(target.locator(".reactions"));
    const add = await box(target.locator(".reaction-add"));
    const chip = await box(target.locator(".button.reaction").first());

    expect(Math.round(image.y - (body.y + body.height))).toBe(6);
    expect(Math.round(reactions.y - (image.y + image.height))).toBe(6);
    expect(Math.round(reactions.x)).toBe(Math.round(body.x));
    expect(Math.round(add.y)).toBe(Math.round(chip.y));
    expect(Math.round(add.height)).toBe(Math.round(chip.height));
  });

  test(`brand and workspace icons sit in the text (${theme})`, async ({ page, request }) => {
    const id = await post(request, "- :anthropic: Opus 5.5 - 58\n- :shipit: Ship it");

    await page.setViewportSize(DESKTOP);
    await openOn(page, id, theme);

    const brand = row(page, id).locator("img.icon--brand");
    const custom = row(page, id).locator("img.icon--custom");

    await expect(brand).toBeVisible();

    const icon = await box(brand);

    expect(icon.height).toBeGreaterThanOrEqual(14);
    expect(icon.height).toBeLessThanOrEqual(24);
    // In the line, not a block above it: the list item stays one line tall around the icon.
    const item = await box(row(page, id).locator("li").first());

    expect(item.height).toBeLessThanOrEqual(28);
    expect(icon.y).toBeGreaterThanOrEqual(item.y - 1);
    expect(icon.y + icon.height).toBeLessThanOrEqual(item.y + item.height + 1);
    // Brand icons are a black glyph on transparency: inverted in the dark theme only.
    await expect(brand).toHaveCSS("filter", theme === "dark" ? "invert(1)" : "none");
    await expect(custom).toHaveCSS("filter", "none");
  });

  test(`inline images are bounded and sit on a backdrop (${theme})`, async ({ page, request }) => {
    // The newest message, whose body the replies below swap for a remote image.
    const id = await post(request, "The logo");

    await page.route("**/__test/logo.svg", (route) =>
      route.fulfill({
        contentType: "image/svg+xml",
        body: '<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800" viewBox="0 0 12 8"><path d="M1 7 4 1l3 6z"/></svg>',
      }),
    );
    await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages*`, async (route) => {
      const response = await route.fetch();
      const reply: MessagePage = await response.json();

      await route.fulfill({ response, json: withImage(reply, id) });
    });

    for (const viewport of [DESKTOP, PHONE]) {
      await page.setViewportSize(viewport);
      await openOn(page, id, theme);

      const image = row(page, id).locator(".message-body img");

      await expect(image).toBeVisible();
      await expect
        .poll(() => image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth))
        .toBeGreaterThan(0);

      const size = await box(image);

      expect(size.width).toBeLessThanOrEqual(360.5);
      expect(size.height).toBeLessThanOrEqual(320.5);
      // The aspect ratio holds (1200×800).
      expect(Math.abs(size.width / size.height - 1.5)).toBeLessThan(0.02);
      expect(await image.evaluate((img) => getComputedStyle(img).backgroundImage)).toContain(
        "conic-gradient",
      );
    }
  });
}
