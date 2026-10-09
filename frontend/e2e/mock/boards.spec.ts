import type { APIRequestContext, Page } from "@playwright/test";
import { BOARD_POST_IDS, BOARD_ROOM_ID, LONG_DISCUSSION } from "../../mock/s6/seed.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import { DESKTOP, expect, matrix, openApp, shot, syncWelcomed, test, USER_IDS } from "./support.ts";

const BOARD = BOARD_ROOM_ID;

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

function posts(page: Page) {
  return page.getByRole("list", { name: "Posts" });
}

function column(page: Page, name: string) {
  return page.getByRole("region", { name });
}

/** Has Maya change a post through the mock's `/__mock/board-update` control. */
async function updateAsMaya(
  request: APIRequestContext,
  body: { threadId: number; status?: string; ownerId?: number | null; tags?: string[] },
): Promise<void> {
  const reply = await request.post("/__mock/board-update", { data: body });

  expect(reply.ok()).toBe(true);
}

test("filters narrow the list and show as chips", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}`);
  await page.getByRole("button", { name: /^Filter/ }).click();
  await page.getByRole("menuitem", { name: /^Tag:/ }).click();
  await page.getByRole("menuitemradio", { name: /^bug/ }).click();

  await expect(page).toHaveURL(/tag=bug/);
  await expect(page.getByRole("button", { name: "Clear tag filter" })).toBeVisible();
  await expect(posts(page).getByRole("link")).toHaveCount(2);
  await expect(posts(page).getByRole("link", { name: /Fix retries/ })).toBeVisible();

  await page.getByRole("button", { name: /^Filter/ }).click();
  await page.getByRole("menuitem", { name: /^Owner:/ }).click();
  await page.getByRole("menuitemradio", { name: "Me", exact: true }).click();
  await expect(posts(page).getByRole("link")).toHaveCount(1);
  await expect(posts(page).getByRole("link", { name: /Migrate background workers/ })).toBeVisible();
  await shot(page, "board-filtered", "light");

  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(page).not.toHaveURL(/tag=|owner=/);
  await expect(posts(page).getByRole("link", { name: /Onboarding checklist/ })).toBeVisible();
});

test("the + button creates a post that opens in the right pane", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}`);
  await page.getByRole("button", { name: "New post" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/posts/new`));

  const dialog = page.getByRole("dialog", { name: "New post" });

  await expect(dialog).toBeVisible();
  await shot(page, "board-new-post", "light");
  await dialog.getByRole("button", { name: "Create post" }).click();
  await expect(dialog.getByText("Give the post a title.")).toBeVisible();

  await dialog.getByLabel("Title").fill("Write the incident runbook");
  await dialog.getByLabel(/^Brief/).fill("Cover paging and the status page.");
  await dialog.getByLabel("Status").selectOption("in_progress");
  await dialog.getByRole("textbox", { name: "Tags" }).fill("infra, Docs");
  await dialog.getByRole("button", { name: "Create post" }).click();

  await expect(dialog).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/t/\\d+$`));
  await expect(
    pane(page).getByRole("heading", { name: "Write the incident runbook" }),
  ).toBeVisible();
  await expect(pane(page).getByText("In progress").first()).toBeVisible();
  await expect(pane(page).getByText("docs", { exact: true })).toBeVisible();
  await expect(posts(page).getByRole("link", { name: /Write the incident runbook/ })).toBeVisible();
});

test("a classic new-thread URL on a board opens the new-post dialog", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/posts/new`);
  await expect(page.getByRole("dialog", { name: "New post" })).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}$`));
});

matrix("a post opens with its work above the discussion", async ({ page, theme, phone }) => {
  await openApp(page, `r/${BOARD}`, theme);
  // Open posts only, by default: done ones stay out of the list.
  await expect(posts(page).getByRole("link", { name: /Pricing page refresh/ })).toHaveCount(0);
  await posts(page)
    .getByRole("link", { name: /Fix retries duplicating sends/ })
    .click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/t/${BOARD_POST_IDS.retryBug}$`));

  const work = pane(page);

  await expect(work.getByRole("heading", { name: "Fix retries duplicating sends" })).toBeVisible();
  await expect(work.getByText("Blocked").first()).toBeVisible();
  await expect(work.getByRole("link", { name: /Open run/ })).toHaveAttribute(
    "href",
    "https://ci.example.com/runs/42",
  );
  await expect(work.getByRole("region", { name: "Linked" })).toBeVisible();
  await expect(work.getByRole("heading", { name: "Result" })).toBeVisible();
  await expect(work.getByText("The original message was deleted.")).toHaveCount(0);
  await work.getByRole("button", { name: "Steps (2)" }).click();
  await expect(work.getByText("Reproduce the duplicate")).toBeVisible();
  await expect(work.getByText("1.2s")).toBeVisible();
  await expect(work.getByText("Two sends share one client id")).toBeVisible();
  await expect(work.getByText("Write the fix")).toBeVisible();
  await expect(work.getByText(/^Discussion · \d+ repl/)).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, phone ? "board-post-phone" : "board-post", theme);
});

for (const key of ["End", "PageDown"] as const) {
  test(`${key} from a post's focused Steps button takes over before an incoming reply`, async ({
    page,
  }) => {
    const threadId = BOARD_POST_IDS.retryBug;

    await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
      const response = await route.fetch();
      const body: MessagePage = await response.json();

      body.messages = body.messages.map((message) => ({
        ...message,
        bodyHtml: `<div style="height:500px">${message.bodyHtml}</div>`,
      }));
      await route.fulfill({ response, json: body });
    });
    const welcomed = syncWelcomed(page);

    await openApp(page, `r/${BOARD}/t/${threadId}`);
    await welcomed;
    const list = pane(page).getByRole("log", { name: "Replies" });
    const steps = list.getByRole("button", { name: "Steps (2)" });

    await expect(list).toHaveAttribute("data-placement-settled", "true");
    await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(0);
    await steps.focus();
    await expect(steps).toBeFocused();
    await list.evaluate((element) => {
      element.setAttribute("data-key-scroll-settled", "false");
      element.addEventListener(
        "scrollend",
        () => element.setAttribute("data-key-scroll-settled", "true"),
        { once: true },
      );
    });
    await steps.press(key);
    await expect(list).toHaveAttribute("data-key-scroll-settled", "true");

    const before = await list.evaluate((element) => ({
      offset: element.scrollTop,
      height: element.scrollHeight,
    }));

    expect(before.offset).toBeGreaterThan(0);
    const state = await (await page.request.get("/__mock/state")).json();

    const posted = await page.request.post("/__mock/thread-post", {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: { threadId, userId: USER_IDS.maya, markdown: "A reply after the Steps scrolling key" },
    });

    expect(posted.ok()).toBe(true);
    await expect
      .poll(() => list.evaluate((element) => element.scrollHeight))
      .toBeGreaterThan(before.height);

    if (key === "End") {
      await expect(
        list.getByText("A reply after the Steps scrolling key", { exact: true }),
      ).toBeInViewport();
      await expect
        .poll(() =>
          list.evaluate(
            (element) => element.scrollHeight - element.clientHeight - element.scrollTop,
          ),
        )
        .toBeLessThanOrEqual(1);
    } else {
      await expect
        .poll(() =>
          list.evaluate((element, offset) => Math.abs(element.scrollTop - offset), before.offset),
        )
        .toBeLessThanOrEqual(3);
    }
  });
}

test("End on a fitting post's sole reply follows an incoming long reply", async ({ page }) => {
  const threadId = BOARD_POST_IDS.onboardingChecklist;

  await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    body.messages = body.messages.slice(-1);
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });
  const welcomed = syncWelcomed(page);

  await page.setViewportSize({ width: 1440, height: 1200 });
  await openApp(page, `r/${BOARD}/t/${threadId}`);
  await welcomed;
  const list = pane(page).getByRole("log", { name: "Replies" });
  const reply = list.locator("[data-message-row]");

  await expect(list).toHaveAttribute("data-placement-settled", "true");
  await expect(reply).toHaveCount(1);
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight - element.clientHeight))
    .toBe(0);
  await reply.focus();
  await expect(reply).toBeFocused();
  await reply.press("End");
  await expect(reply).toBeFocused();

  const state = await (await page.request.get("/__mock/state")).json();

  const posted = await page.request.post("/__mock/thread-post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: {
      threadId,
      userId: USER_IDS.maya,
      markdown: Array.from(
        { length: 100 },
        (_, index) => `Board End reply paragraph ${index}`,
      ).join("\n\n"),
    },
  });

  expect(posted.ok()).toBe(true);
  await expect(list.getByText("Board End reply paragraph 99", { exact: true })).toBeInViewport();
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight - element.clientHeight))
    .toBeGreaterThan(500);
  await expect
    .poll(() =>
      list.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop),
    )
    .toBeLessThanOrEqual(1);
});

test("a post with more replies than one page still opens at its work", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/t/${BOARD_POST_IDS.keyboardNavigation}`);

  const work = pane(page);

  await expect(work.getByRole("button", { name: /^Status:/ })).toBeVisible();
  await expect(work.getByRole("heading", { name: "Result" })).toBeVisible();

  const earlier = work.getByRole("button", { name: /earlier repl/ });

  await expect(earlier).toBeVisible();
  // The newest page is loaded; the brief and the first replies wait under the work.
  await expect(work.getByText(/^Brief: Keyboard navigation/)).toHaveCount(0);
  await earlier.click();
  await expect(work.getByText(/^Brief: Keyboard navigation/)).toBeVisible();
  await expect(earlier).toHaveCount(0);
  await expect(work.getByRole("button", { name: /^Status:/ })).toBeVisible();
  expect(LONG_DISCUSSION).toBeGreaterThan(40);
});

test("changing a post's status in the pane moves its card", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/t/${BOARD_POST_IDS.apiPagination}?view=board`);
  await expect(
    column(page, "Planned").getByRole("link", { name: /Cursor pagination/ }),
  ).toBeVisible();

  await pane(page).getByRole("button", { name: "Status: Planned" }).click();
  await page.getByRole("menuitemradio", { name: "In progress" }).click();

  await expect(
    column(page, "In progress").getByRole("link", { name: /Cursor pagination/ }),
  ).toBeVisible();
  await expect(
    column(page, "Planned").getByRole("link", { name: /Cursor pagination/ }),
  ).toHaveCount(0);
});

test("editing tags and the result from the pane", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/t/${BOARD_POST_IDS.onboardingChecklist}`);

  await pane(page).getByRole("button", { name: "Edit tags" }).click();
  await pane(page).getByRole("textbox", { name: "Tags" }).fill("design, onboarding");
  await pane(page).getByRole("button", { name: "Save tags" }).click();
  await expect(pane(page).getByText("onboarding", { exact: true })).toBeVisible();
  await expect(posts(page).getByRole("link", { name: /Onboarding checklist/ })).toContainText(
    "onboarding",
  );

  await pane(page).getByRole("button", { name: "Edit result" }).click();
  await pane(page).getByLabel("Result in Markdown").fill("Shipped behind the **checklist** flag.");
  await pane(page).getByRole("button", { name: "Save result" }).click();
  await expect(pane(page).locator(".post-result-body strong")).toHaveText("checklist");
});

test("another member's changes move rows live", async ({ page, request }) => {
  await page.setViewportSize(DESKTOP);

  const welcomed = syncWelcomed(page);

  await openApp(page, `r/${BOARD}?view=board`);
  await welcomed;
  await expect(
    column(page, "Planned").getByRole("link", { name: /Launch week plan/ }),
  ).toBeVisible();

  await updateAsMaya(request, { threadId: BOARD_POST_IDS.launchWeek, status: "blocked" });
  await expect(
    column(page, "Blocked").getByRole("link", { name: /Launch week plan/ }),
  ).toBeVisible();
  await expect(column(page, "Planned").getByRole("link", { name: /Launch week plan/ })).toHaveCount(
    0,
  );

  // In the list, a post that's done leaves the open filter.
  await page.getByRole("tab", { name: "List" }).click();
  await expect(posts(page).getByRole("link", { name: /Launch week plan/ })).toBeVisible();
  await updateAsMaya(request, { threadId: BOARD_POST_IDS.launchWeek, status: "done" });
  await expect(posts(page).getByRole("link", { name: /Launch week plan/ })).toHaveCount(0);

  // A post someone else creates arrives at the top.
  await request.post("/__mock/board-create", {
    data: { name: "Live post from Maya", status: "planned", tags: ["api"] },
  });
  await expect(posts(page).getByRole("link").first()).toContainText("Live post from Maya");
});

test("a board post uses the shared work handoff dialog", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/t/${BOARD_POST_IDS.onboardingChecklist}`);
  await pane(page).getByRole("button", { name: "Hand off to an agent" }).click();

  const dialog = page.getByRole("dialog", { name: /^Hand off/ });

  await expect(dialog.getByRole("radio", { name: /Ember/ })).toBeFocused();
  await dialog.getByRole("textbox", { name: "Summary" }).fill("Finish the checklist rollout.");
  await dialog.getByRole("button", { name: "Hand off", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(pane(page).getByRole("button", { name: "Hand off to an agent" })).toHaveCount(0);
  await pane(page)
    .getByRole("button", { name: /^Work history/ })
    .click();
  await expect(pane(page).getByText(/handed off owner .* → Ember/)).toBeVisible();
});
