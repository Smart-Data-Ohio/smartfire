import type { APIRequestContext, Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  longPress,
  matrix,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  SHOTS,
  shot,
  swipeLeft,
  test,
} from "./support.ts";

/** Calls one of the mock's `/__mock/*` controls. */
async function control(request: APIRequestContext, action: string): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post(`/__mock/${action}`, { headers: { "X-CSRF-Token": state.csrfToken } });
}

function rows(page: Page) {
  return page.locator(".page .list-row:not([data-motion='leave'])");
}

/** Waits for a destination's first page: its heading and at least one row. */
async function ready(page: Page, title: string): Promise<void> {
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
  await expect(rows(page).first()).toBeVisible();
  await page.mouse.move(0, 0);
}

const EMPTY_SAVED = { items: [], messages: [], users: [], conversations: [], nextCursor: null };

// --- activity ---

matrix("the activity inbox", async ({ page, theme, phone }) => {
  await openApp(page, "activity", theme);
  await ready(page, "Activity");
  await shot(page, "activity-all", theme);

  if (!phone) {
    await rows(page).first().hover();
    await expect(rows(page).first().locator(".list-row-bar")).toHaveCSS("opacity", "1");
    await expect(rows(page).first().locator(".activity-time")).toHaveCSS("opacity", "0");
    await shot(page, "activity-hover", theme);
    await page.mouse.move(0, 0);
  }

  await page.getByRole("tab", { name: "Mentions" }).click();
  await expect(page).toHaveURL(/tab=mentions/);
  await expect(rows(page).first()).toContainText(/Mention|Reply|Keyword/);
  await shot(page, "activity-mentions", theme);

  // A tab with nothing in it: the seed fills every tab, so the empty answer is stubbed.
  await page.route("**/api/v1/activity?*", async (route) => {
    const response = await route.fetch();
    const snapshot = await response.json();

    await route.fulfill({ json: { ...snapshot, items: [], users: [], nextCursor: null } });
  });
  await openApp(page, "activity?tab=github&status=handled", theme);
  await expect(page.getByText("No handled review requests")).toBeVisible();
  await shot(page, "activity-empty", theme);
});

test("the rail's Activity badge counts unread items and opens the inbox", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const activity = page.getByRole("button", { name: "Activity" });

  await expect(activity.locator(".badge")).toHaveText(/\d+/);
  await activity.click();
  await expect(page).toHaveURL(/\/app\/activity$/);
  await expect(activity).toHaveAttribute("aria-pressed", "true");
});

/** The header's unread count. */
async function unreadCount(page: Page): Promise<number> {
  return Number(await page.locator(".page-count [aria-hidden='true']").textContent());
}

function body(page: Page, index = 0) {
  return rows(page).nth(index).locator(".activity-body");
}

/** The page's live region, which says what a row action did. */
function said(page: Page) {
  return page.locator(".page [role='status'][aria-live='polite']");
}

/** The open button of the `index`th row still in the list. */
function openButton(page: Page, index = 0) {
  return rows(page).nth(index).locator(".list-row-open");
}

test("marking an item handled moves it from Unread to Handled", async ({ page }) => {
  await openApp(page, "activity");
  await ready(page, "Activity");

  const text = (await body(page).textContent()) ?? "";
  const before = await unreadCount(page);

  await rows(page).first().locator(".list-row-open").focus();
  await page.keyboard.press("e");
  await expect(page.locator(".page-count [aria-hidden='true']")).toHaveText(`${before - 1}`);
  await expect(body(page)).not.toHaveText(text);
  // Focus moves on to the row that took its place, and the change is announced.
  await expect(openButton(page)).toBeFocused();
  await expect(said(page)).toHaveText("Marked handled");

  await page.getByRole("tab", { name: "Handled" }).click();
  await expect(page).toHaveURL(/status=handled/);
  await expect(body(page)).toHaveText(text);
  await expect(rows(page).first()).toContainText("Handled");
});

test("marking an item read from its context menu", async ({ page }) => {
  await openApp(page, "activity");
  await ready(page, "Activity");

  const text = (await body(page).textContent()) ?? "";

  await rows(page).first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Mark as read" }).click();
  await expect(body(page)).not.toHaveText(text);
  // Once the menu has gone, focus is on the next row rather than lost.
  await expect(openButton(page)).toBeFocused();

  await page.getByRole("tab", { name: "Read", exact: true }).click();
  await expect(body(page)).toHaveText(text);
});

test("Home and End reach the first and last loaded rows", async ({ page }) => {
  await openApp(page, "activity");
  await ready(page, "Activity");

  const first = (await body(page).textContent()) ?? "";

  await openButton(page).focus();
  await page.keyboard.press("End");
  await expect(page.locator(".page .list-row-open:focus")).toHaveCount(1);
  await expect(page.locator(".page .list-row-open:focus .activity-body")).not.toHaveText(first);

  await page.keyboard.press("Home");
  await expect(openButton(page)).toBeFocused();
  await expect(body(page)).toHaveText(first);
});

test("opening a mention goes to the message", async ({ page }) => {
  await openApp(page, "activity?tab=mentions");
  await ready(page, "Activity");
  await rows(page).first().locator(".list-row-open").click();

  await expect(page).toHaveURL(/\/app\/r\/\d+(\/(m|t)\/\d+)?/);
});

test("a live item slides in at the top", async ({ page, request }) => {
  await openApp(page, "activity");
  await ready(page, "Activity");

  const before = await unreadCount(page);
  const text = (await body(page).textContent()) ?? "";

  await control(request, "activity-arrival");
  await expect(page.locator(".page-count [aria-hidden='true']")).toHaveText(`${before + 1}`);
  await expect(body(page, 1)).toHaveText(text);
});

// --- saved ---

matrix("saved messages", async ({ page, theme, phone }) => {
  await openApp(page, "saved", theme);
  await ready(page, "Saved");
  await shot(page, "saved", theme);

  if (!phone) {
    await rows(page).nth(1).hover();
    await expect(rows(page).nth(1).locator(".list-row-bar")).toHaveCSS("opacity", "1");
    await expect(rows(page).nth(1).locator(".saved-time")).toHaveCSS("opacity", "0");
    await shot(page, "saved-hover", theme);
    await page.mouse.move(0, 0);
  }

  await page.route("**/api/v1/saved?*", (route) => route.fulfill({ json: EMPTY_SAVED }));
  await openApp(page, "saved?status=done", theme);
  await expect(page.getByText("Nothing done yet")).toBeVisible();
  await shot(page, "saved-empty", theme);
});

test("removing a saved message offers Undo", async ({ page }) => {
  await openApp(page, "saved");
  await ready(page, "Saved");

  const text = (await rows(page).first().locator(".saved-body").textContent()) ?? "";

  await rows(page).first().locator(".list-row-open").focus();
  await page.keyboard.press("Delete");
  await expect(rows(page).first().locator(".saved-body")).not.toHaveText(text);
  await expect(openButton(page)).toBeFocused();
  // The toast is the one announcement: the live region stays quiet.
  await expect(page.getByRole("status").filter({ hasText: "Removed from saved" })).toBeVisible();
  await expect(said(page)).toHaveText("");
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(rows(page).first().locator(".saved-body")).toHaveText(text);
});

test("the sidebar leads to Saved and Scheduled", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await page.getByRole("link", { name: "Saved" }).click();
  await expect(page).toHaveURL(/\/app\/saved$/);
  await expect(page.getByRole("link", { name: "Saved" })).toHaveAttribute("aria-current", "page");
  await page.getByRole("link", { name: "Scheduled" }).click();
  await expect(page).toHaveURL(/\/app\/scheduled$/);
});

// --- scheduled ---

matrix("scheduled messages", async ({ page, theme, phone }) => {
  await openApp(page, "scheduled", theme);
  await ready(page, "Scheduled");
  await expect(page.getByRole("heading", { name: /^Upcoming/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /^Can't be sent/ })).toBeVisible();
  await shot(page, "scheduled", theme);

  if (!phone) {
    await rows(page).first().hover();
    await expect(rows(page).first().locator(".list-row-bar")).toHaveCSS("opacity", "1");
    await expect(rows(page).first().locator(".scheduled-when")).toHaveCSS("opacity", "0");
    await shot(page, "scheduled-hover", theme);
    await page.mouse.move(0, 0);
  }

  const past = page.getByRole("heading", { name: /^Past/ });

  await past.scrollIntoViewIfNeeded();
  await expect(past).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "scheduled-past", theme);
});

test("cancelling asks first, then removes the message", async ({ page }) => {
  await openApp(page, "scheduled");
  await ready(page, "Scheduled");

  const first = rows(page).first();
  const text = (await first.locator(".scheduled-body").textContent()) ?? "";

  await first.hover();
  await first.getByRole("button", { name: "Cancel scheduled message" }).click();
  await expect(
    page.getByRole("alertdialog", { name: /Cancel this scheduled message/ }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Cancel message" }).click();
  await expect(rows(page).first().locator(".scheduled-body")).not.toHaveText(text);
  // The dialog came from a row that's gone: focus lands on the row that took its place.
  await expect(openButton(page)).toBeFocused();
  await expect(
    page.getByRole("status").filter({ hasText: "Scheduled message cancelled" }),
  ).toBeVisible();
});

test("editing a scheduled message saves its new text", async ({ page }) => {
  await openApp(page, "scheduled");
  await ready(page, "Scheduled");
  await rows(page).first().locator(".list-row-open").click();

  const dialog = page.getByRole("dialog");

  await dialog.getByRole("textbox", { name: "Message" }).fill("Moved to the new launch doc");
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();
  await expect(rows(page).first()).toContainText("Moved to the new launch doc");
});

// --- touch phones ---

test.describe("the lists on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  test("activity is thumb-sized, with its filter in a menu and no icon bar", async ({ page }) => {
    await openApp(page, "activity");
    await ready(page, "Activity");
    await expectTouchTargets(page, ".page");
    await expectNoHorizontalOverflow(page, { allowScroll: ".page-toolbar .tabs" });
    await expect(rows(page).first().locator(".list-row-bar")).toHaveCSS("opacity", "0");

    const filter = page.getByRole("button", { name: "Show: Unread" });

    await expect(page.getByRole("tab", { name: "Handled" })).toBeHidden();
    await filter.click();
    await page.getByRole("menuitemradio", { name: "Handled" }).click();
    await expect(page).toHaveURL(/status=handled/);
    await expect(page.getByRole("button", { name: "Show: Handled" })).toBeVisible();
  });

  test("a long press on an activity row opens its action sheet", async ({ page }) => {
    await openApp(page, "activity");
    await ready(page, "Activity");
    await longPress(rows(page).first().locator(".list-row-inner"));

    const sheet = page.getByRole("menu", { name: "Activity actions" });

    await expect(sheet).toBeVisible();
    await expect(sheet).toHaveClass(/\baction-sheet\b/);
    // The swipe's action is in the sheet too, for anyone who can't swipe.
    await expect(sheet.getByRole("menuitem", { name: "Mark as handled" })).toBeVisible();
    await expectTouchTargets(page, `#${await sheet.getAttribute("id")}`);
    // The press doesn't also open the item.
    await expect(page).toHaveURL(/\/app\/activity/);
  });

  test("swiping an activity row left marks it handled", async ({ page }) => {
    await openApp(page, "activity");
    await ready(page, "Activity");

    const text = (await body(page).textContent()) ?? "";
    const before = await unreadCount(page);

    await swipeLeft(rows(page).first().locator(".list-row-inner"));
    await expect(page.locator(".page-count [aria-hidden='true']")).toHaveText(`${before - 1}`);
    await expect(body(page)).not.toHaveText(text);
    await expect(said(page)).toHaveText("Marked handled");
    await expect(page).toHaveURL(/\/app\/activity/);
  });

  test("saved and scheduled are thumb-sized and open a sheet on a long press", async ({ page }) => {
    await openApp(page, "saved");
    await ready(page, "Saved");
    await expectTouchTargets(page, ".page");
    await expectNoHorizontalOverflow(page, { allowScroll: ".page-toolbar .tabs" });
    await longPress(rows(page).first().locator(".list-row-inner"));
    await expect(page.getByRole("menu", { name: "Saved message actions" })).toHaveClass(
      /\baction-sheet\b/,
    );
    await page.keyboard.press("Escape");

    await openApp(page, "scheduled");
    await ready(page, "Scheduled");
    await expectTouchTargets(page, ".page");
    await longPress(rows(page).first().locator(".list-row-inner"));
    await expect(page.getByRole("menu", { name: "Scheduled message actions" })).toHaveClass(
      /\baction-sheet\b/,
    );
  });
});

// --- loading and errors ---

matrix(
  "a list shows a skeleton while it loads, and Retry when it fails",
  async ({ page, theme }) => {
    const held = Promise.withResolvers<void>();
    let fail = true;

    await page.route("**/api/v1/saved?*", async (route) => {
      await held.promise;

      if (fail) {
        await route.fulfill({ status: 500, body: "" });
      } else {
        await route.continue();
      }
    });
    await openApp(page, "saved", theme);
    await expect(page.locator(".page-list .t-skel")).toHaveAttribute("aria-busy", "true");
    await page.mouse.move(0, 0);
    await shot(page, "saved-loading", theme);

    held.resolve();
    await expect(page.getByText("Your saved messages couldn't be loaded.")).toBeVisible();
    await shot(page, "saved-error", theme);

    fail = false;
    await page.getByRole("button", { name: "Try again" }).click();
    await expect(rows(page).first()).toBeVisible();
  },
);

// --- shell ---

// Screenshot only: the entries' behaviour is checked by the rail badge and sidebar tests above.
if (SHOTS) {
  matrix("the rail and sidebar entries", async ({ page, theme, phone }) => {
    await openApp(page, phone ? "" : `r/${ROOM_IDS.general}`, theme);
    await expect(page.getByRole("link", { name: "Saved" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Activity" })).toBeVisible();
    await page.mouse.move(0, 0);
    await shot(page, "shell-entries", theme);
  });
}
