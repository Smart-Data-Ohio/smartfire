import type { APIRequestContext, Page } from "@playwright/test";
import { MESSAGE_IDS, THREAD_IDS } from "../../mock/s2/seed.ts";
import { expect, matrix, ROOM_IDS, shot, synced, type Theme, test } from "./support.ts";

/**
 * Opens the app at `path` (under /app/) with motion reduced; unlike `openApp` it waits for the
 * conversation, since a phone in a room or thread hides the sidebar.
 */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
  await synced(page);
}

const GENERAL = ROOM_IDS.general;

const ACTIVE_NAME = "Invite-to-first-message conversion dip";

interface ThreadPost {
  readonly threadId: number;
  readonly userId: number;
  readonly markdown: string;
}

/** Has someone reply in a thread through the mock's `/__mock/thread-post` control. */
async function postReply(request: APIRequestContext, body: ThreadPost): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/thread-post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: body,
  });
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

matrix("a reply indicator opens its thread", async ({ page, theme }) => {
  // The thread's root is among the room's latest messages, 44 below #general's first unread. The
  // room opens on a window around that first unread, which the sync welcome's refetch may or may
  // not swap for the newest page, so open on the root's permalink: it's mounted either way.
  await open(page, `r/${GENERAL}/m/${MESSAGE_IDS.generalThreadRoot}`, theme);

  const indicator = page.getByRole("button", { name: /^\d+ replies, unread\./ }).first();

  await indicator.click();
  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/${THREAD_IDS.generalActive}$`));
  await expect(pane(page).getByRole("heading", { name: ACTIVE_NAME })).toBeVisible();
  await expect(pane(page).getByRole("log", { name: "Replies" })).toBeVisible();
  await expect(pane(page).getByText(/^\d+ replies$/)).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "thread-pane", theme);
});

matrix("replying in a thread", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`, theme);

  const composer = pane(page).getByRole("textbox", { name: "Reply…" });

  await composer.click();
  await composer.fill("Looks like the dip lines up with the new invite email.");
  await composer.press("Enter");

  await expect(
    pane(page).getByText("Looks like the dip lines up with the new invite email."),
  ).toBeVisible();

  await shot(page, "thread-replied", theme);
});

matrix("live replies arrive in the open thread", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`, theme);
  await expect(pane(page).getByRole("log", { name: "Replies" })).toBeVisible();

  await postReply(page.request, {
    threadId: THREAD_IDS.generalActive,
    userId: 2,
    markdown: "Pulled the cohort numbers, posting them in a sec.",
  });

  await expect(
    pane(page).getByText("Pulled the cohort numbers, posting them in a sec."),
  ).toBeVisible();
});

matrix("starting a new thread", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/new?parent=${MESSAGE_IDS.generalChart}`, theme);

  await expect(pane(page).getByRole("heading", { name: "New thread" })).toBeVisible();
  await expect(pane(page).getByText("Start a thread")).toBeVisible();
  await pane(page).getByLabel("Thread name (optional)").fill("Signup chart follow-ups");
  await page.mouse.move(0, 0);
  await shot(page, "thread-new", theme);

  const composer = pane(page).getByRole("textbox", { name: "Reply…" });

  await composer.click();
  await composer.fill("Can we split this by acquisition channel?");
  await composer.press("Enter");

  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
  await expect(pane(page).getByRole("heading", { name: "Signup chart follow-ups" })).toBeVisible();
  await expect(pane(page).getByText("Can we split this by acquisition channel?")).toBeVisible();
});

test("the follow toggle", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);

  const following = pane(page).getByRole("button", { name: "Following" });

  await expect(following).toHaveAttribute("aria-pressed", "true");
  await following.click();

  const follow = pane(page).getByRole("button", { name: "Follow", exact: true });

  await expect(follow).toHaveAttribute("aria-pressed", "false");
  await follow.click();
  await expect(pane(page).getByRole("button", { name: "Following" })).toBeVisible();
});

test("Esc closes the thread and focus goes back", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);
  await expect(pane(page).getByRole("heading", { name: ACTIVE_NAME })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}$`));
  await expect(pane(page)).toHaveCount(0);
});

test("Esc in a composer with a draft keeps the thread open", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);

  const input = pane(page).locator(".composer-input");

  await input.fill("half a thought");
  await input.press("Escape");
  await expect(pane(page).getByRole("heading", { name: ACTIVE_NAME })).toBeVisible();
  await expect(input).toHaveValue("half a thought");
});

test("a thread opened from the Threads pane goes back to the list", async ({ page }) => {
  await open(page, `r/${GENERAL}`);
  await page.locator(".room-header").getByRole("button", { name: "Threads" }).click();
  await pane(page)
    .getByRole("button", { name: new RegExp(ACTIVE_NAME) })
    .click();

  await expect(pane(page).getByRole("heading", { name: ACTIVE_NAME })).toBeVisible();
  await pane(page).getByRole("button", { name: "Back to threads" }).click();
  await expect(pane(page).getByRole("heading", { name: "Threads" })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(pane(page)).toHaveCount(0);
});

matrix("locked and closed threads", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalLocked}`, theme);
  await expect(pane(page).getByRole("heading", { name: "Meeting format decision" })).toBeVisible();
  await expect(pane(page).getByText("Locked", { exact: true }).first()).toBeVisible();
  await shot(page, "thread-locked", theme);

  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalClosed}`, theme);
  await expect(pane(page).getByText("This thread is closed. Replying reopens it.")).toBeVisible();
  await shot(page, "thread-closed", theme);
});

test("a locked thread refuses replies until a moderator unlocks it", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalLocked}`);

  await expect(pane(page).getByText("This thread is locked.")).toBeVisible();
  await expect(pane(page).getByRole("textbox", { name: "Reply…" })).toHaveCount(0);

  await pane(page).getByRole("button", { name: "Unlock" }).click();
  await expect(pane(page).getByRole("textbox", { name: "Reply…" })).toBeVisible();
});

test("the thread menu offers what the viewer may do", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);
  await pane(page).getByRole("button", { name: "Thread actions" }).click();

  await expect(page.getByRole("menuitem", { name: "Copy link" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Rename thread…" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Rename thread…" }).click();

  const field = page.getByRole("dialog").getByLabel("Name");

  await field.fill("Conversion dip, week 40");
  await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
  await expect(pane(page).getByRole("heading", { name: "Conversion dip, week 40" })).toBeVisible();
});

test.describe("phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("the thread is a page with a back button", async ({ page }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);
    await expect(pane(page).getByRole("heading", { name: ACTIVE_NAME })).toBeVisible();

    await pane(page).getByRole("button", { name: "Back to #general" }).click();
    await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}$`));
    await expect(page.getByRole("log", { name: "Messages" })).toBeVisible();
  });
});
