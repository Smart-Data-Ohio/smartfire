import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import { seededMessageId } from "../../mock/seed.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import {
  expect,
  holdSync,
  matrix,
  postMessage,
  ROOM_IDS,
  scrollByWheel,
  shot,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/** A message row by id. */
function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-id="${messageId}"]`);
}

/**
 * Opens `path` under /app/ and waits for the message list. Unlike `openApp` it doesn't wait for
 * the sidebar, which a phone doesn't show beside a room.
 */
async function openRoom(page: Page, path: string, theme: Theme = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("log", { name: "Messages" }).waitFor();
}

/** Takes the shot once every finite animation (a popup's entrance, a fade) has played out. */
async function settledShot(page: Page, name: string, theme: Theme) {
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== "running" ||
          animation.effect?.getTiming().iterations === Number.POSITIVE_INFINITY,
      ),
  );
  await shot(page, name, theme);
}

/** Opens #general on the permalink, so the row is mounted and centred. */
async function openOn(page: Page, messageId: number, theme: Theme = "light") {
  await openRoom(page, `r/${ROOM_IDS.general}/m/${messageId}`, theme);
  await expect(row(page, messageId)).toBeVisible();
}

/**
 * Pages toward the present until the jump pill closes: the window reaches the present and the
 * reader is at its end. Scrolling is what fetches each newer page. The wheel is the reader, so a
 * permalink does not keep the place they just left. It follows the jump: the listener settles the
 * position it sees, and a wheel before the jump anchors the permalink.
 */
async function scrollToPresent(page: Page) {
  const log = page.getByRole("log", { name: "Messages" });
  const jump = page.locator(".timeline-jump");

  await expect(async () => {
    await log.evaluate((element) => {
      element.scrollTop = element.scrollHeight;
      element.dispatchEvent(
        new WheelEvent("wheel", { deltaY: 1, bubbles: true, cancelable: true }),
      );
    });
    await expect(log.getByRole("status", { name: "Loading messages" })).toHaveCount(0);
    await expect(jump).toHaveAttribute("data-open", "false");
  }).toPass({ timeout: 20_000 });
}

/**
 * Hovers `target` until its hover bar shows. The permalinked row can be swapped for a fresh one
 * as the timeline settles, and a row mounted under a resting pointer sees no hover until it moves.
 */
async function hoverBar(target: Locator): Promise<Locator> {
  const bar = target.getByRole("toolbar", { name: "Message actions" });

  await expect(async () => {
    await target.page().mouse.move(0, 0);
    await target.hover();
    await expect(bar).toBeVisible({ timeout: 1000 });
  }).toPass();

  return bar;
}

/** Sends `text` from the room's composer and answers the new row. */
async function send(page: Page, text: string): Promise<Locator> {
  const input = page.locator(".composer-input").first();

  await input.fill(text);
  await input.press("Enter");

  const pending = page.locator("[data-message-row]", { hasText: text }).last();

  await expect(pending).toHaveAttribute("data-message-id", /^\d+$/);

  // By id from here on: an open editor replaces the body text the row was found by.
  return row(page, Number(await pending.getAttribute("data-message-id")));
}

async function openMenu(page: Page, target: Locator) {
  await target.locator(".message-body").first().click({ button: "right" });
  await expect(page.getByRole("menu", { name: "Message actions" })).toBeVisible();
}

test.describe("message actions", () => {
  test("the hover bar appears on hover and offers the row's actions", async ({ page }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);

    const target = row(page, MESSAGE_IDS.generalReactions);

    const bar = await hoverBar(target);

    await expect(bar.getByRole("button", { name: "Add reaction" })).toBeVisible();
    await expect(bar.getByRole("button", { name: "Reply in thread" })).toBeVisible();
    await expect(bar.getByRole("button", { name: "More actions" })).toBeVisible();

    await page.mouse.move(5, 5);
    await expect(bar).toBeHidden();
  });

  test("right click opens the message menu", async ({ page }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);
    await openMenu(page, row(page, MESSAGE_IDS.generalReactions));

    const menu = page.getByRole("menu", { name: "Message actions" });

    await expect(menu.getByRole("menuitem", { name: /Reply in thread/ })).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: /Copy link/ })).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: /Delete message/ })).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();
  });

  test("a reaction from the picker lands on the row, and clicking it takes it back", async ({
    page,
  }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);

    const target = row(page, MESSAGE_IDS.generalReactions);

    await (await hoverBar(target)).getByRole("button", { name: "Add reaction" }).click();

    const search = page.getByRole("combobox", { name: "Search emoji" });

    await expect(search).toBeFocused();
    await search.fill("rocket");
    await page.keyboard.press("Enter");

    const pill = target.locator(".reactions").getByRole("button", { name: /^rocket:/i });

    await expect(pill).toHaveAttribute("aria-pressed", "true");
    await pill.click();
    await expect(pill).toBeHidden();
  });

  test("editing in place saves with Enter and marks the message edited", async ({ page }) => {
    await openRoom(page, `r/${ROOM_IDS.general}`);

    const sent = await send(page, "Draft for the release notes");

    await openMenu(page, sent);
    await page.getByRole("menuitem", { name: /Edit message/ }).click();

    const editor = sent.getByRole("textbox", { name: "Edit message" });

    await expect(editor).toBeFocused();
    await editor.fill("Final release notes");
    await editor.press("Enter");

    await expect(sent.locator(".message-body")).toHaveText("Final release notes");
    await expect(sent.getByText("(edited)")).toBeVisible();
  });

  test("Esc cancels an edit without saving", async ({ page }) => {
    await openRoom(page, `r/${ROOM_IDS.general}`);

    const sent = await send(page, "Keep this wording");

    await openMenu(page, sent);
    await page.getByRole("menuitem", { name: /Edit message/ }).click();

    const editor = sent.getByRole("textbox", { name: "Edit message" });

    await editor.fill("Something else");
    await editor.press("Escape");

    await expect(sent.locator(".message-body")).toHaveText("Keep this wording");
    await expect(sent.getByText("(edited)")).toHaveCount(0);
  });

  test("deleting asks first, then removes the row", async ({ page }) => {
    await openRoom(page, `r/${ROOM_IDS.general}`);

    const sent = await send(page, "Typo, ignore me");

    await openMenu(page, sent);
    await page.getByRole("menuitem", { name: /Delete message/ }).click();

    const dialog = page.getByRole("alertdialog", { name: "Delete message?" });

    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Cancel" })).toBeFocused();
    await dialog.getByRole("button", { name: "Delete" }).click();

    await expect(page.locator("[data-message-row]", { hasText: "Typo, ignore me" })).toHaveCount(0);
  });

  test("pin and save show on the row", async ({ page }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);

    const target = row(page, MESSAGE_IDS.generalReactions);

    await openMenu(page, target);
    await page.getByRole("menuitem", { name: /Pin to conversation/ }).click();
    await expect(target.locator(".message-flag", { hasText: "Pinned" })).toBeVisible();

    await openMenu(page, target);
    await page.getByRole("menuitem", { name: /Save for later/ }).click();
    await expect(target.locator(".message-flag", { hasText: "Saved for later" })).toBeVisible();

    await openMenu(page, target);
    await page.getByRole("menuitem", { name: /Unpin from conversation/ }).click();
    await expect(target.locator(".message-flag", { hasText: "Pinned" })).toHaveCount(0);
  });

  test("forwarding sends to the chosen conversation", async ({ page }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);
    await openMenu(page, row(page, MESSAGE_IDS.generalReactions));
    await page.getByRole("menuitem", { name: /Forward/ }).click();

    const dialog = page.getByRole("dialog", { name: "Forward message" });

    await expect(dialog).toBeVisible();
    await dialog.getByRole("searchbox", { name: "Search conversations" }).fill("random");
    await dialog.getByRole("checkbox", { name: "random" }).check();
    await dialog.getByRole("textbox", { name: /Add a note/ }).fill("For the weekly roundup");
    await dialog.getByRole("button", { name: "Forward", exact: true }).click();

    await expect(dialog.getByText("Forwarded")).toBeVisible();
    await expect(dialog).toBeHidden();
  });

  test("the keyboard walks the rows and acts on the focused one", async ({ page }) => {
    await openOn(page, MESSAGE_IDS.generalReactions);

    const target = row(page, MESSAGE_IDS.generalReactions);

    await target.focus();
    await page.keyboard.press("ArrowUp");

    const previous = page.locator("[data-message-row]:focus");

    await expect(previous).toHaveCount(1);
    await expect(previous).not.toHaveAttribute(
      "data-message-id",
      `${MESSAGE_IDS.generalReactions}`,
    );

    await page.keyboard.press("ArrowDown");
    await expect(target).toBeFocused();
    await expect(target.getByRole("toolbar", { name: "Message actions" })).toBeVisible();

    await page.keyboard.press("Shift+F10");
    await expect(page.getByRole("menu", { name: "Message actions" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(target).toBeFocused();

    await page.keyboard.press("r");
    await expect(page.getByRole("combobox", { name: "Search emoji" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(target).toBeFocused();
  });

  test("Home and End reach the ends of the window, past the rows drawn", async ({ page }) => {
    await openRoom(page, `r/${ROOM_IDS.general}`);
    // The socket's first welcome refetches the window; let that land before reading the rows.
    await page.waitForLoadState("networkidle");

    const drawn = () =>
      page.evaluate(() =>
        [
          ...document.querySelectorAll('[role="log"][aria-label="Messages"] [data-message-row]'),
        ].map((element) => Number(element.getAttribute("data-message-id"))),
      );

    /** The focused row's id, once a row has focus. */
    const focusedId = async () =>
      Number(
        await (
          await page.waitForFunction(() => {
            const active = document.activeElement;

            return active?.matches("[data-message-row]")
              ? active.getAttribute("data-message-id")
              : null;
          })
        ).jsonValue(),
      );

    const ids = await drawn();
    const newest = Math.max(...ids);

    await row(page, newest).focus();
    await page.keyboard.press("Home");

    // The first message wasn't among the rows drawn at the bottom.
    expect(await focusedId()).toBeLessThan(Math.min(...ids));

    await page.keyboard.press("End");
    expect(await focusedId()).toBeGreaterThanOrEqual(newest);
  });
});

test("the sync welcome's refetch leaves a reader on the unread divider", async ({
  page,
  request,
}) => {
  const release = await holdSync(page);
  const messages = `/api/v1/rooms/${ROOM_IDS.general}/messages`;
  // The newer page can load on its own as the room opens, so listen from the start.
  const newer = page.waitForResponse((response) => response.url().includes(`${messages}?after=`));

  // #general opens on a window around its first unread, many messages back.
  await openRoom(page, `r/${ROOM_IDS.general}`);

  const log = page.getByRole("log", { name: "Messages" });
  const divider = log.locator(".unread-divider");

  await expect(divider).toBeInViewport();
  await expect(log).toHaveAttribute("data-placement-settled", "true");
  await expect(log).toHaveAttribute("data-scroll-settled", "true");

  // Page down to the present and back up to the divider before the welcome arrives.
  const top = await log.evaluate((element) => element.scrollTop);

  await scrollByWheel(page, log, 10_000);

  await (await newer).finished();
  await expect(log.getByRole("status", { name: "Loading messages" })).toHaveCount(0);

  const remaining = await log.evaluate(
    (element) => element.scrollHeight - element.clientHeight - element.scrollTop,
  );

  if (remaining > 1) await scrollByWheel(page, log, remaining);

  // Late row measurements can move the bottom after its offset is read but before the wheel
  // arrives. Finish returning to the divider before testing what the welcome does to its place.
  await expect(async () => {
    await scrollByWheel(page, log, top - (await log.evaluate((element) => element.scrollTop)));
    await expect(divider).toBeInViewport({ timeout: 1000 });
  }).toPass();

  // Someone posts while the socket is held. The welcome refetches the newest page, which holds
  // their message: it joins the window below the reader, counted, without moving them.
  await postMessage(request, {
    roomId: ROOM_IDS.general,
    userId: USER_IDS.maya,
    markdown: "posted while away",
  });

  const newest = page.waitForResponse((response) => response.url().endsWith(messages));

  release();
  await (await newest).finished();
  await expect(page.getByRole("button", { name: "1 new message" })).toBeVisible();
  await expect(divider).toBeInViewport();
});

test("a reader at the bottom through a long absence is paged on to the present", async ({
  page,
  request,
}) => {
  const release = await holdSync(page, { missed: true });

  // #engineering is read: it opens at the present, the reader at the bottom.
  await openRoom(page, `r/${ROOM_IDS.engineering}`);

  const log = page.getByRole("log", { name: "Messages" });

  await expect(log.getByRole("status", { name: "Loading messages" })).toHaveCount(0);

  // More is posted while the socket is down than the welcome's newest page holds, so that page no
  // longer meets the window: it stops short of the present until the reader is paged on.
  for (let at = 1; at <= 45; at++) {
    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.maya,
      markdown: `while away ${at}`,
    });
  }

  // The welcome also re-reads the window around its middle, which adds older rows and so scrolls
  // the list; fail that re-read, so nothing the reader does or sees moves the list.
  await page.route(
    (url) => url.searchParams.has("around"),
    (route) => route.abort(),
  );

  const paged = page.waitForResponse(async (response) => {
    const url = new URL(response.url());

    if (
      url.pathname !== `/api/v1/rooms/${ROOM_IDS.engineering}/messages` ||
      !url.searchParams.has("after")
    )
      return false;

    const body: MessagePage = await response.json();

    return body.after === null;
  });

  release();
  await (await paged).finished();
  await expect(log.getByText("while away 45", { exact: true })).toBeInViewport();
});

test("a permalink scrolled to the present pages on after a long absence", async ({
  page,
  request,
}) => {
  const release = await holdSync(page, { missed: true });

  // Early in #engineering, so the page around it stops short of the present. Scrolling there
  // keeps the message focused: the URL still names it.
  const permalink = seededMessageId(ROOM_IDS.engineering, 20);

  await openRoom(page, `r/${ROOM_IDS.engineering}/m/${permalink}`);
  await expect(row(page, permalink)).toBeVisible();
  await scrollToPresent(page);
  await expect(page).toHaveURL(new RegExp(`/m/${permalink}$`));

  // More is posted while the socket is down than the welcome's newest page holds, so that page
  // no longer meets the window the reader scrolled to.
  for (let at = 1; at <= 45; at++) {
    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.maya,
      markdown: `missed on permalink ${at}`,
    });
  }

  // The welcome also re-reads the window around its middle, which adds older rows and so scrolls
  // the list; fail that re-read, so nothing the reader does or sees moves the list.
  await page.route(
    (url) => url.searchParams.has("around"),
    (route) => route.abort(),
  );
  release();

  const log = page.getByRole("log", { name: "Messages" });

  await expect(log.getByText("missed on permalink 45", { exact: true })).toBeInViewport();
});

matrix("message hover bar", async ({ page, theme, phone }) => {
  await openOn(page, MESSAGE_IDS.generalReactions, theme);

  const target = row(page, MESSAGE_IDS.generalReactions);

  if (phone) {
    await target.focus();
  } else {
    await hoverBar(target);
  }

  await settledShot(page, "hover-bar", theme);
});

matrix("message context menu", async ({ page, theme }) => {
  await openOn(page, MESSAGE_IDS.generalReactions, theme);
  await openMenu(page, row(page, MESSAGE_IDS.generalReactions));
  await settledShot(page, "context-menu", theme);
});

matrix("emoji picker", async ({ page, theme, phone }) => {
  await openOn(page, MESSAGE_IDS.generalReactions, theme);

  const target = row(page, MESSAGE_IDS.generalReactions);

  // A phone has no hover bar; the reactions row's add pill opens the picker there.
  if (phone) {
    await target.locator(".reaction-add").click();
  } else {
    await (await hoverBar(target)).getByRole("button", { name: "Add reaction" }).click();
  }

  await expect(page.getByRole("combobox", { name: "Search emoji" })).toBeFocused();
  await expect(page.getByRole("option").first()).toBeVisible();
  await settledShot(page, "emoji-picker", theme);
});

matrix("message edit", async ({ page, theme }) => {
  await openRoom(page, `r/${ROOM_IDS.general}`, theme);

  const sent = await send(page, "Shipping the beta on Thursday");

  await openMenu(page, sent);
  await page.getByRole("menuitem", { name: /Edit message/ }).click();
  await expect(sent.getByRole("textbox", { name: "Edit message" })).toBeFocused();
  await settledShot(page, "edit", theme);
});

matrix("forward dialog", async ({ page, theme }) => {
  await openOn(page, MESSAGE_IDS.generalReactions, theme);
  await openMenu(page, row(page, MESSAGE_IDS.generalReactions));
  await page.getByRole("menuitem", { name: /Forward/ }).click();

  const dialog = page.getByRole("dialog", { name: "Forward message" });

  await expect(dialog.getByRole("checkbox").first()).toBeVisible();
  await dialog.getByRole("checkbox").nth(1).check();
  await settledShot(page, "forward-dialog", theme);
});

matrix("image lightbox", async ({ page, theme }) => {
  await openOn(page, MESSAGE_IDS.generalChart, theme);
  await row(page, MESSAGE_IDS.generalChart).locator(".attachment-image").click();

  const dialog = page.getByRole("dialog");

  await expect(dialog.locator(".lightbox-image")).toBeVisible();
  await settledShot(page, "lightbox", theme);
});

matrix("reactions and boosts", async ({ page, theme }) => {
  await openOn(page, MESSAGE_IDS.generalBoosts, theme);
  await expect(row(page, MESSAGE_IDS.generalBoosts).locator(".boost").first()).toBeVisible();
  await settledShot(page, "reactions", theme);
});

matrix("forwarded, pinned and file rows", async ({ page, theme }) => {
  await openOn(page, MESSAGE_IDS.generalForward, theme);
  await settledShot(page, "forward-and-files", theme);
});
