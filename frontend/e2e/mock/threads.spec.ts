import type { APIRequestContext, Page } from "@playwright/test";
import { MESSAGE_IDS, seededReplyId, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  expect,
  matrix,
  PHONE_TOUCH,
  ROOM_IDS,
  SHOTS,
  shot,
  simulateKeyboard,
  syncWelcomed,
  type Theme,
  test,
} from "./support.ts";

/**
 * Opens the app at `path` (under /app/) with motion reduced; unlike `openApp` it waits for the
 * conversation, since a phone in a room or thread hides the sidebar.
 */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

const GENERAL = ROOM_IDS.general;

const ACTIVE_NAME = "Invite-to-first-message conversion dip";

interface ThreadPost {
  readonly threadId: number;
  readonly userId: number;
  readonly markdown: string;
}

/** Posts `count` replies so an early one falls off the newest page. */
async function postReplies(page: Page, threadId: number, count: number): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  for (let index = 0; index < count; index += 1) {
    await page.request.post(`/api/v1/threads/${threadId}/messages`, {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: {
        clientMessageId: `paging-filler-${index}`,
        markdownSource: `Filler reply ${index}`,
        replyToMessageId: null,
        replyNotifyAuthor: null,
        attachmentSignedId: null,
      },
    });
  }
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
  // The thread's root is among the room's latest messages, 44 below the first unread the room
  // opens at (and the sync welcome leaves the reader there): jump down to them.
  await open(page, `r/${GENERAL}`, theme);
  await page.getByRole("button", { name: /^Jump to present$|new messages?$/ }).click();

  const indicator = page.getByRole("button", { name: /^\d+ replies, unread\./ }).first();

  await expect(indicator).toHaveCSS("pointer-events", "auto");
  await indicator.evaluate((element) =>
    element.scrollIntoView({ block: "center", behavior: "instant" }),
  );
  await expect(indicator).toBeInViewport({ ratio: 1 });
  await expect(indicator).toHaveCSS("pointer-events", "auto");
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

// Screenshots only: the locked state is checked by the next test.
if (SHOTS) {
  matrix("locked and closed threads", async ({ page, theme }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalLocked}`, theme);
    await expect(
      pane(page).getByRole("heading", { name: "Meeting format decision" }),
    ).toBeVisible();
    await expect(pane(page).getByText("Locked", { exact: true }).first()).toBeVisible();
    await shot(page, "thread-locked", theme);

    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalClosed}`, theme);
    await expect(pane(page).getByText("This thread is closed. Replying reopens it.")).toBeVisible();
    await shot(page, "thread-closed", theme);
  });
}

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

test("an older reply's permalink pages forward when scrolled to the bottom", async ({ page }) => {
  const oldest = seededReplyId(THREAD_IDS.generalActive, 0);

  // The room page's own socket is welcomed first, so it can't connect late and answer the
  // waiter below in place of the permalink page's socket.
  const roomWelcomed = syncWelcomed(page);

  await open(page, `r/${GENERAL}`);
  await roomWelcomed;
  // More replies than a permalink window plus the welcome's re-read around its middle, so the
  // tail stays past the loaded window until the reader scrolls down to it.
  await postReplies(page, THREAD_IDS.generalActive, 80);

  const welcomed = syncWelcomed(page);

  const newer = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/threads/${THREAD_IDS.generalActive}/messages?after=`),
  );

  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}?m=${oldest}`);
  await welcomed;

  const log = pane(page).getByRole("log", { name: "Replies" });
  const oldestRow = log.locator(`[data-message-id="${oldest}"]`);

  await expect(oldestRow).toBeVisible();
  // End asks the virtual list to bring its last loaded reply to the bottom edge, which pages on.
  await oldestRow.focus();
  await page.keyboard.press("End");
  await (await newer).finished();
  await page.keyboard.press("End");
  await expect(log.getByText("Filler reply 79", { exact: true })).toBeVisible();
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

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the work facts fold to one line, and give way to the replies while typing", async ({
    page,
  }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);

    const work = pane(page).getByRole("region", { name: "Work" });

    await expect(work.getByRole("button", { name: /^Status: / })).toBeVisible();

    // One line: the status, the owner and the details toggle share a row.
    const middle = async (name: string | RegExp) => {
      const box = await work.getByRole("button", { name }).boundingBox();

      return (box?.y ?? 0) + (box?.height ?? 0) / 2;
    };

    const status = await middle(/^Status: /);
    const toggle = await middle("Result, steps and history");

    expect(Math.abs(status - toggle), "the toggle beside the status").toBeLessThan(4);

    const section = await work.boundingBox();

    expect(section?.height ?? 999, "one line").toBeLessThanOrEqual(52);

    const keyboard = 320;
    const visible = PHONE_TOUCH.viewport.height - keyboard;

    await pane(page).getByRole("textbox", { name: "Reply…" }).click();
    await simulateKeyboard(page, keyboard);
    await expect(work).toBeHidden();

    const log = await pane(page).getByRole("log", { name: "Replies" }).boundingBox();

    expect(log?.height ?? 0, "the replies keep half the visible screen").toBeGreaterThanOrEqual(
      visible * 0.5,
    );
  });

  test("editing the work result keeps the editor in view while typing", async ({ page }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);

    const work = pane(page).getByRole("region", { name: "Work" });

    await work.getByRole("button", { name: "Result, steps and history" }).click();
    await work.getByRole("button", { name: /^(Add|Edit) result$/ }).click();

    const editor = work.getByRole("textbox", { name: "Result" });

    await editor.click();
    await simulateKeyboard(page, 320);
    await expect(editor).toBeVisible();
    await expect(editor).toBeFocused();
  });
});
