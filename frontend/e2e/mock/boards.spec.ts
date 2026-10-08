import type { APIRequestContext, Page } from "@playwright/test";
import { BOARD_POST_IDS, BOARD_ROOM_ID, LONG_DISCUSSION } from "../../mock/s6/seed.ts";
import { DESKTOP, expect, matrix, openApp, shot, syncWelcomed, test } from "./support.ts";

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

matrix("a board opens as a list of its posts", async ({ page, theme, phone }) => {
  await openApp(page, `r/${BOARD}`, theme);

  await expect(page.getByRole("toolbar", { name: "Board" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "List" })).toHaveAttribute("aria-selected", "true");
  // Open posts only, by default: done ones stay out of the list.
  await expect(posts(page).getByRole("link", { name: /Onboarding checklist/ })).toBeVisible();
  await expect(posts(page).getByRole("link", { name: /Pricing page refresh/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "New post" })).toBeVisible();
  expect(page.url()).not.toContain("classic");
  await page.mouse.move(0, 0);
  await shot(page, phone ? "board-list-phone" : "board-list", theme);
});

matrix("the board view sorts posts into status columns", async ({ page, theme, phone }) => {
  await openApp(page, `r/${BOARD}?view=board`, theme);

  for (const name of ["Planned", "In progress", "Blocked", "Done"]) {
    await expect(column(page, name)).toBeVisible();
  }

  await expect(
    column(page, "Done").getByRole("link", { name: /Pricing page refresh/ }),
  ).toBeVisible();
  await expect(column(page, "Blocked").getByRole("link", { name: /Fix retries/ })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, phone ? "board-columns-phone" : "board-columns", theme);
});

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
