import type { APIRequestContext, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import { S4_BOARD, S4_BOARD_POST_IDS, S4_WORK_IDS } from "../../mock/s4/seed.ts";
import {
  expect,
  matrix,
  openApp,
  openHeaderTool,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

const GENERAL = ROOM_IDS.general;

const AGENT_OWNED = "Invite-to-first-message conversion dip";

/** Opens the app at `path`, waiting for the conversation (a phone in a thread hides the sidebar). */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

function workSection(page: Page) {
  return pane(page).getByRole("region", { name: "Work" });
}

function rows(page: Page) {
  return page.locator(".page .list-row");
}

/** Has someone else set a thread's status through the mock's `/__mock/work-status` control. */
async function setStatusAs(
  request: APIRequestContext,
  threadId: number,
  status: string,
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post("/__mock/work-status", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { threadId, status },
  });

  expect(response.ok()).toBe(true);
}

/** Waits for menus, dialogs and the accordion to finish moving before a shot. */
async function settle(page: Page): Promise<void> {
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

// --- the thread pane ---

matrix("a work thread's pane", async ({ page, theme, phone }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.agentOwned}`, theme);

  const work = workSection(page);

  await expect(
    work.getByRole("button", { name: "Status: In progress. Change status" }),
  ).toBeVisible();
  await expect(work.getByRole("button", { name: "Owner: Ember. Change owner" })).toBeVisible();
  await expect(work.getByRole("link", { name: "Run, opens in a new tab" })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "work-pane", theme);

  await work.getByRole("button", { name: "Result, steps and history" }).click();
  await expect(work.getByRole("heading", { name: "History", exact: true })).toBeVisible();
  await expect(work.getByRole("heading", { name: /^Steps \(\d+\)$/ })).toBeVisible();
  await settle(page);
  await shot(page, "work-pane-details", theme);

  if (!phone) {
    await work.getByRole("button", { name: /Change status/ }).click();
    await expect(page.getByRole("menuitemradio", { name: "In progress" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await settle(page);
    await shot(page, "work-status-menu", theme);
    await page.keyboard.press("Escape");

    await work.getByRole("button", { name: /Change owner/ }).click();
    await expect(page.getByRole("group", { name: "Agents" })).toBeVisible();
    await settle(page);
    await shot(page, "work-owner-menu", theme);
    await page.keyboard.press("Escape");
  }
});

matrix("an inactive owner and a cancelled event", async ({ page, theme }) => {
  await open(page, `r/${ROOM_IDS.design}/t/${S4_WORK_IDS.inactiveOwner}`, theme);

  const work = workSection(page);

  await expect(work.getByText("owner inactive")).toBeVisible();
  await expect(work.getByRole("link", { name: /Cancelled$/ })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "work-inactive-owner", theme);
});

test("changing the status shows at once and lands in the history", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.agentOwned}`);

  const work = workSection(page);

  await work.getByRole("button", { name: "Result, steps and history" }).click();
  await work.getByRole("button", { name: /Change status/ }).click();
  await page.getByRole("menuitemradio", { name: "Blocked" }).click();

  await expect(work.getByRole("button", { name: "Status: Blocked. Change status" })).toBeFocused();
  await expect(
    work.getByRole("list").filter({ hasText: "status In progress → Blocked" }),
  ).toBeVisible();
});

test("someone else's change refetches the pane's history", async ({ page, request }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.agentOwned}`);

  const work = workSection(page);

  await work.getByRole("button", { name: "Result, steps and history" }).click();
  await expect(work.getByRole("heading", { name: "History", exact: true })).toBeVisible();
  await setStatusAs(request, S4_WORK_IDS.agentOwned, "done");

  await expect(work.getByRole("button", { name: "Status: Done. Change status" })).toBeVisible();
  await expect(work.getByText("status In progress → Done")).toBeVisible();
});

matrix("handing work off to an agent", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.done}`, theme);

  const work = workSection(page);

  await work.getByRole("button", { name: "Result, steps and history" }).click();
  await work.getByRole("button", { name: "Hand off to an agent" }).click();

  const dialog = page.getByRole("dialog", { name: /^Hand off/ });

  await expect(dialog.getByRole("radio", { name: /Ember/ })).toBeFocused();
  await settle(page);
  await shot(page, "work-handoff", theme);

  await dialog.getByRole("textbox", { name: /^Links/ }).fill("not a url");
  await dialog.getByRole("button", { name: "Hand off" }).click();
  await expect(dialog.getByText("Summary can't be blank")).toBeVisible();
  await expect(dialog.getByText("Links must be http(s) URLs")).toBeVisible();
  await shot(page, "work-handoff-errors", theme);

  await dialog.getByRole("textbox", { name: /^Links/ }).fill("https://example.com/spec");
  await dialog.getByRole("textbox", { name: "Summary" }).fill("Results are in; write the summary.");
  await dialog.getByRole("button", { name: "Hand off" }).click();

  await expect(dialog).toHaveCount(0);
  await expect(work.getByRole("button", { name: "Owner: Ember. Change owner" })).toBeVisible();
  await expect(work.getByText(/handed off owner .* → Ember/)).toBeVisible();
});

test("recording a result renders it with who updated it", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.agentOwned}`);

  const work = workSection(page);

  await work.getByRole("button", { name: "Result, steps and history" }).click();
  await work.getByRole("button", { name: /^(Add|Edit) result$/ }).click();
  await work.getByRole("textbox", { name: "Result" }).fill("Conversion is back to **42%**.");
  await work.getByRole("textbox", { name: "Result" }).press("Control+Enter");

  await expect(work.locator(".work-result-body strong")).toHaveText("42%");
  await expect(work.getByText(/^Updated just now by /)).toBeVisible();
  await expect(work.getByRole("button", { name: "Edit result" })).toBeFocused();
});

test("tracking a thread as work, then stopping", async ({ page }) => {
  await open(page, `r/${GENERAL}/t/${S4_WORK_IDS.untracked}`);
  await expect(workSection(page)).toHaveCount(0);

  await pane(page).getByRole("button", { name: "Thread actions" }).click();
  await page.getByRole("menuitem", { name: "Track as work" }).click();

  const work = workSection(page);

  await expect(work.getByRole("button", { name: "Status: Planned. Change status" })).toBeVisible();
  await expect(work.getByRole("button", { name: "Owner: Unassigned. Change owner" })).toBeVisible();

  await work.getByRole("button", { name: /Change status/ }).click();
  await page.getByRole("menuitem", { name: "Stop tracking…" }).click();

  const confirm = page.getByRole("alertdialog", { name: "Stop tracking this work?" });

  await expect(confirm.getByRole("button", { name: "Cancel" })).toBeFocused();
  await confirm.getByRole("button", { name: "Stop tracking" }).click();

  await expect(workSection(page)).toHaveCount(0);
  await expect(pane(page).getByRole("button", { name: "Thread actions" })).toBeFocused();
});

// --- thread rows and indicators ---

matrix("work on the Threads pane's rows", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, "Threads");

  const row = pane(page).getByRole("button", { name: new RegExp(AGENT_OWNED) });

  await expect(row.locator(".work-status")).toHaveAttribute("data-status", "in_progress");
  await expect(row.getByText("Ember")).toBeVisible();
  await expect(pane(page).getByRole("list", { name: `Links for ${AGENT_OWNED}` })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "work-thread-rows", theme);
});

test("a reply indicator carries the thread's work", async ({ page }) => {
  // The root is above the present; its permalink loads and centers it in the virtualized list.
  await open(page, `r/${GENERAL}/m/${MESSAGE_IDS.generalThreadRoot}`);
  await expect(page.locator(".timeline > .t-skel")).toHaveAttribute("aria-busy", "false");

  const work = page
    .locator(`[data-message-id="${MESSAGE_IDS.generalThreadRoot}"]`)
    .locator(".thread-indicator-work");

  // Thread work arrives separately, and list measurements can remount the row meanwhile.
  await expect(async () => {
    await work.scrollIntoViewIfNeeded({ timeout: 1000 });
    await expect(work.locator(".work-status")).toBeVisible({ timeout: 500 });
    await expect(work.locator(".work-status")).toHaveAttribute("data-status", "in_progress", {
      timeout: 500,
    });
    await expect(work).toBeInViewport({ ratio: 1, timeout: 500 });
  }).toPass();
  await page.mouse.move(0, 0);
  await settle(page);
  await shot(page, "work-indicator", "light");
});

// --- the work page ---

matrix("the work page", async ({ page, theme }) => {
  await openApp(page, "work", theme);
  await expect(page.getByRole("heading", { level: 1, name: "Work" })).toBeVisible();
  await expect(rows(page).first()).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "work-page-open", theme);

  await page.getByRole("tab", { name: "Agents" }).click();
  await expect(page).toHaveURL(/state=agents/);
  await expect(rows(page)).toHaveCount(3);
  await expect(rows(page).first()).toContainText(AGENT_OWNED);

  await page.getByRole("tab", { name: "Boards" }).click();
  await expect(page).toHaveURL(/state=boards/);
  await expect(rows(page).first().getByText("Board", { exact: true })).toBeVisible();
  await shot(page, "work-page-boards", theme);

  // An empty tab: the seed fills every tab, so the empty answer is stubbed.
  await page.route("**/api/v1/work?*", (route) =>
    route.fulfill({ json: { threads: [], users: [] } }),
  );
  await openApp(page, "work?state=done", theme);
  await expect(page.getByText(/^No completed work yet/)).toBeVisible();
  await shot(page, "work-page-empty", theme);
  await page.unroute("**/api/v1/work?*");

  // Loading: the reply waits until the shot is taken.
  const held = Promise.withResolvers<void>();

  await page.route("**/api/v1/work?*", async (route) => {
    await held.promise;
    await route.fallback();
  });
  await openApp(page, "work?state=all", theme);
  await expect(page.locator(".page-skeleton")).toBeVisible();
  await shot(page, "work-page-loading", theme);
  held.resolve();
  await expect(rows(page).first()).toBeVisible();
  await page.unroute("**/api/v1/work?*");

  await page.route("**/api/v1/work?*", (route) =>
    route.fulfill({ status: 500, json: { error: "Server error" } }),
  );
  await openApp(page, "work", theme);
  await expect(page.getByText("The work list couldn't be loaded.")).toBeVisible();
  await shot(page, "work-page-error", theme);
});

test("a work row opens its thread in the room", async ({ page }) => {
  await openApp(page, "work?state=agents");
  await rows(page).first().locator(".list-row-open").click();

  await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/t/${S4_WORK_IDS.agentOwned}$`));
  await expect(pane(page).getByRole("heading", { name: AGENT_OWNED })).toBeVisible();
});

test("a work row opens its board post in the SPA", async ({ page }) => {
  await openApp(page, "work?state=boards");
  await rows(page).filter({ hasText: "Public API" }).first().locator(".list-row-open").click();

  await expect(page).toHaveURL(
    new RegExp(`/app/r/${S4_BOARD.roomId}/t/${S4_BOARD_POST_IDS.publicApi}$`),
  );
  await expect(pane(page).getByRole("heading", { name: "Public API" })).toBeVisible();
  await expect(pane(page).getByRole("heading", { name: "Result" })).toBeVisible();
});

test("the sidebar's Work destination opens the page, and it reloads when shown again", async ({
  page,
  request,
}) => {
  await openApp(page, `r/${GENERAL}`);
  await page.getByRole("link", { name: "Work" }).click();
  await expect(page).toHaveURL(/\/app\/work$/);
  await expect(page.getByRole("link", { name: "Work" })).toHaveAttribute("aria-current", "page");
  await expect(rows(page).filter({ hasText: AGENT_OWNED })).toHaveCount(1);

  // Done work leaves the open tab, but only once the page is shown again.
  await setStatusAs(request, S4_WORK_IDS.agentOwned, "done");
  await expect(rows(page).filter({ hasText: AGENT_OWNED })).toHaveCount(1);
  await page.getByRole("link", { name: "Saved" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Saved" })).toBeVisible();
  await page.getByRole("link", { name: "Work" }).click();
  await expect(rows(page).filter({ hasText: AGENT_OWNED })).toHaveCount(0);
});

test("arrow keys move between work rows", async ({ page }) => {
  await openApp(page, "work?state=all");

  const first = rows(page).nth(0).locator(".list-row-open");

  await first.focus();
  await page.keyboard.press("ArrowDown");
  await expect(rows(page).nth(1).locator(".list-row-open")).toBeFocused();
});
