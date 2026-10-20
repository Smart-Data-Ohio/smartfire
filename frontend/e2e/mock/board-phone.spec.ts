import type { Locator, Page } from "@playwright/test";
import { BOARD_POST_IDS, BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  longPress,
  openApp,
  PHONE_TOUCH,
  shot,
  TABLET,
  test,
} from "./support.ts";

/**
 * A board on a touch phone: one status at a time under a switcher with counts, swiped or tapped
 * between, full-width cards that open the post, a "⋯" or long press that moves a post through an
 * action sheet, filters in a sheet, and nothing wider than the screen.
 */

const BOARD = `r/${BOARD_ROOM_ID}`;

function tab(page: Page, status: RegExp): Locator {
  return page.getByRole("tablist", { name: "Status" }).getByRole("tab", { name: status });
}

function panel(page: Page, status: string): Locator {
  return page.getByRole("tabpanel", { name: status });
}

function card(page: Page, title: RegExp): Locator {
  return page.locator(".phone-card").filter({ hasText: title });
}

async function openBoard(page: Page): Promise<void> {
  await openApp(page, `${BOARD}?view=board`);
  await expect(page.getByRole("tablist", { name: "Status" })).toBeVisible();
}

/**
 * Scrolls the pager sideways by `pages` widths and lets it snap, as a swipe does. Headless Chromium
 * can't synthesize a touch scroll, so a wheel stands in: the same scroll and the same snap.
 */
async function swipe(page: Page, pages: number): Promise<void> {
  const pager = page.locator(".phone-board-pager");
  const box = await pager.boundingBox();

  if (box === null) {
    throw new Error("the pager isn't on screen");
  }

  await page.mouse.move(box.x + box.width / 2, box.y + 120);
  await page.mouse.wheel(Math.round(box.width * pages), 0);
}

/**
 * Routes the sync socket so the test can drop it. The page reconnects to a welcome that says
 * nothing replays (as after a restart), so it re-reads the rooms it shows. Resolves on that welcome.
 */
async function droppableSync(page: Page): Promise<() => Promise<void>> {
  let close: (() => Promise<void>) | null = null;
  let again: PromiseWithResolvers<void> | null = null;

  await page.routeWebSocket(/\/api\/v1\/sync/, (socket) => {
    const server = socket.connectToServer();

    close = () => socket.close({ code: 4000, reason: "dropped by the test" });
    server.onMessage((message) => {
      const frame: { readonly t: string } = JSON.parse(String(message));

      if (frame.t === "welcome" && again !== null) {
        socket.send(JSON.stringify({ ...frame, resumed: false }));
        again.resolve();

        return;
      }

      socket.send(message);
    });
  });

  return async () => {
    again = Promise.withResolvers<void>();
    await close?.();
    await again.promise;
  };
}

test.describe("phone", () => {
  test.use(PHONE_TOUCH);

  test("New post starts in the status the board is showing", async ({ page }) => {
    await openBoard(page);

    const fab = page.getByRole("button", { name: "New post" });
    const dialog = page.getByRole("dialog", { name: "New post" });

    await fab.tap();
    await expect(dialog.getByLabel("Status")).toHaveValue("planned");
    await dialog.getByRole("button", { name: "Cancel" }).tap();
    await expect(dialog).toBeHidden();

    await tab(page, /Blocked/).tap();
    await expect(tab(page, /Blocked/)).toHaveAttribute("aria-selected", "true");
    await fab.tap();
    await expect(dialog.getByLabel("Status")).toHaveValue("blocked");
  });

  test("the board shows one status at a time; the switcher and a swipe page between them", async ({
    page,
  }) => {
    await openBoard(page);

    await expect(tab(page, /Planned/)).toHaveAttribute("aria-selected", "true");
    await expect(tab(page, /Planned/)).toContainText("3");
    await expect(tab(page, /In progress/)).toContainText("3");
    await expect(card(page, /Cursor pagination/)).toBeInViewport();
    await expect(card(page, /Onboarding checklist/)).not.toBeInViewport();
    // Only the showing column takes focus and reaches assistive tech, as one tab panel does.
    await expect(panel(page, "In progress")).toHaveAttribute("inert");
    await expect(panel(page, "Planned")).not.toHaveAttribute("inert");

    const width = page.viewportSize()?.width ?? 0;
    const planned = await panel(page, "Planned").boundingBox();

    expect(planned?.width).toBe(width);

    await tab(page, /In progress/).tap();
    await expect(tab(page, /In progress/)).toHaveAttribute("aria-selected", "true");
    await expect(card(page, /Onboarding checklist/)).toBeInViewport();
    await expect(card(page, /Cursor pagination/)).not.toBeInViewport();

    await swipe(page, 0.7);
    await expect(tab(page, /Blocked/)).toHaveAttribute("aria-selected", "true");
    await expect(card(page, /Migrate background workers/)).toBeInViewport();
    await expect(tab(page, /In progress/)).toHaveAttribute("aria-selected", "false");

    await swipe(page, -0.7);
    await expect(tab(page, /In progress/)).toHaveAttribute("aria-selected", "true");

    await expectNoHorizontalOverflow(page, { allowScroll: ".phone-board-pager" });
    await expectTouchTargets(page, ".board");
    await shot(page, "board-phone", "light");
  });

  test("a card opens its post, and coming back lands on the same status", async ({ page }) => {
    await openBoard(page);
    await tab(page, /Done/).tap();
    await card(page, /Publish rate limit guidance/).tap();

    await expect(page).toHaveURL(
      new RegExp(`/r/${BOARD_ROOM_ID}/t/${BOARD_POST_IDS.rateLimits}\\?view=board`),
    );
    await expect(
      page
        .locator("aside.right-pane")
        .getByRole("heading", { name: "Publish rate limit guidance" }),
    ).toBeVisible();

    await page.goBack();
    await expect(tab(page, /Done/)).toHaveAttribute("aria-selected", "true");
    await expect(card(page, /Publish rate limit guidance/)).toBeInViewport();
  });

  test("the ⋯ sheet moves a post to another status, and Undo puts it back", async ({ page }) => {
    await openBoard(page);
    await page.getByRole("button", { name: "Cursor pagination for the API actions" }).tap();

    const sheet = page.getByRole("menu", { name: "Cursor pagination for the API actions" });

    await expect(sheet).toHaveClass(/\baction-sheet\b/);
    await expect(sheet.getByRole("menuitemradio", { name: "Planned" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await expectTouchTargets(page, ".action-sheet");
    await sheet.getByRole("menuitemradio", { name: "In progress" }).tap();

    await expect(sheet).toBeHidden();
    await expect(tab(page, /Planned/)).toContainText("2");
    await expect(tab(page, /In progress/)).toContainText("4");
    await expect(panel(page, "In progress").getByText("Cursor pagination for the API")).toHaveCount(
      1,
    );
    await expect(page.getByText("Moved to In progress")).toBeVisible();

    const toast = await page.locator(".toast").filter({ hasText: "Moved to" }).boundingBox();
    const fab = await page.getByRole("button", { name: "New post" }).boundingBox();

    expect(toast === null ? Infinity : toast.y + toast.height).toBeLessThanOrEqual(fab?.y ?? 0);

    await page.getByRole("button", { name: "Undo" }).tap();
    await expect(tab(page, /Planned/)).toContainText("3");
    await expect(panel(page, "Planned").getByText("Cursor pagination for the API")).toHaveCount(1);
  });

  test("a long press on a card opens the same sheet, not the post or the browser's menu", async ({
    page,
  }) => {
    await openBoard(page);

    const pressed = card(page, /Rebuild the search index/);

    await longPress(pressed);

    const sheet = page.getByRole("menu", { name: "Rebuild the search index actions" });

    await expect(sheet).toBeVisible();

    // The context menu and the click a lifted finger brings are both refused.
    const refused = await pressed.evaluate((element) =>
      ["contextmenu", "click"].map(
        (type) =>
          !element.dispatchEvent(
            new MouseEvent(type, { bubbles: true, cancelable: true, detail: 1 }),
          ),
      ),
    );

    expect(refused).toEqual([true, true]);
    await expect(page).toHaveURL(new RegExp(`/r/${BOARD_ROOM_ID}\\?view=board$`));
    await expect(sheet).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(sheet).toBeHidden();
    await pressed.focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(
      new RegExp(`/r/${BOARD_ROOM_ID}/t/${BOARD_POST_IDS.searchIndex}\\?view=board`),
    );
  });

  test("filters open as a sheet; a column they empty says so", async ({ page }) => {
    await openBoard(page);
    await page.getByRole("button", { name: /^Filter/ }).tap();

    const sheet = page.locator(".action-sheet");

    await expect(sheet).toBeVisible();
    await sheet.getByRole("menuitem", { name: /^Tag:/ }).tap();
    await sheet.getByRole("menuitemradio", { name: /^bug/ }).tap();

    await expect(page).toHaveURL(/tag=bug/);
    await expect(tab(page, /Planned/)).toContainText("0");
    await expect(tab(page, /Blocked/)).toContainText("2");
    await expect(panel(page, "Planned")).toContainText("No planned posts match these filters.");

    await panel(page, "Planned").getByRole("button", { name: "Clear filters" }).tap();
    await expect(page).not.toHaveURL(/tag=/);
    await expect(tab(page, /Planned/)).toContainText("3");
  });

  test("a reconnect re-reads the board without blanking it", async ({ page }) => {
    const reconnect = await droppableSync(page);

    await openApp(page, BOARD);

    const post = page.getByRole("list", { name: "Posts" }).getByRole("listitem").first();

    await expect(post).toBeVisible();
    await post.evaluate((element) => element.setAttribute("data-kept", ""));

    const reread = page.waitForResponse((response) =>
      new URL(response.url()).pathname.endsWith(`/rooms/${BOARD_ROOM_ID}/board`),
    );

    await reconnect();
    await reread;

    await expect(page.locator(".board .t-skel")).toHaveAttribute("aria-busy", "false");
    await expect(post).toHaveAttribute("data-kept");
  });

  test("the list scrolls to its last post, clear of New post in thumb reach", async ({ page }) => {
    await openApp(page, BOARD);

    const cards = page.getByRole("list", { name: "Posts" }).locator(".phone-card");

    await expect(cards).toHaveCount(9);
    await expect(cards.first().locator(".board-status")).toBeVisible();
    await expect(cards.last()).not.toBeInViewport();

    const first = await cards.first().boundingBox();

    await page.mouse.move(180, (first?.y ?? 0) + 40);
    await page.mouse.wheel(0, 3000);
    await expect(cards.last()).toBeInViewport({ ratio: 1 });

    const fab = page.getByRole("button", { name: "New post" });
    const button = await fab.boundingBox();
    const last = await cards.last().boundingBox();
    const height = page.viewportSize()?.height ?? 0;

    expect(button === null ? 0 : button.y + button.height).toBeGreaterThan(height - 80);
    expect(last === null || button === null ? 1 : last.y + last.height).toBeLessThanOrEqual(
      button?.y ?? 0,
    );
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".board");

    await fab.tap();
    await expect(page.getByRole("dialog", { name: "New post" })).toBeVisible();
    await expectTouchTargets(page, ".board-form");
  });
});

test("a tablet's board scrolls sideways to its last column, inside the room", async ({ page }) => {
  await page.setViewportSize(TABLET);
  await openApp(page, `${BOARD}?view=board`);

  const columns = page.locator(".board-columns");
  const done = page.getByRole("region", { name: "Done" });

  await expect(done).toBeAttached();
  await columns.evaluate((element) => {
    element.scrollLeft = element.scrollWidth;
  });
  await expect(done).toBeInViewport({ ratio: 1 });
  await expectNoHorizontalOverflow(page, { allowScroll: ".board-columns" });
});
