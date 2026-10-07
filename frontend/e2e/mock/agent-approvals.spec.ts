import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { AGENT_IDS } from "../../mock/s4/agents.ts";
import type { AgentLedgerPage } from "../../src/gen/AgentLedgerPage.ts";
import { expect, matrix, openApp, shot, test, USER_IDS } from "./support.ts";

const { ember: EMBER, scout: SCOUT } = AGENT_IDS;

/** Calls one of the mock's controls with `data`. */
async function control(
  request: APIRequestContext,
  action: string,
  data: Record<string, string | number | boolean> = {},
) {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok()).toBe(true);
}

/** Opens a section of an agent's profile with the simulation paused, so nothing moves. */
async function openSection(page: Page, path: string, theme: "light" | "dark" = "light") {
  await control(page.request, "pause");
  await openApp(page, path, theme);
  await expect(page.getByRole("tablist", { name: "Sections" })).toBeVisible();
}

function card(page: Page, summary: string | RegExp): Locator {
  return page.locator(".approval-card").filter({ hasText: summary });
}

/** Scrolls the list down until `target` shows (rows are virtual), or gives up. */
async function scrollUntil(page: Page, target: Locator): Promise<void> {
  const list = page.locator(".page-list-scroll");

  for (let step = 0; step < 40 && !(await target.isVisible()); step++) {
    await list.evaluate((element) => element.scrollBy(0, 500));
    await page.waitForTimeout(60);
  }

  await expect(target).toBeVisible();
}

// --- approvals ---

matrix("an agent's approval requests", async ({ page, theme }) => {
  await openSection(page, `agents/${EMBER}/approvals`, theme);

  const deploy = card(page, "deploy production");

  await expect(deploy).toBeVisible();
  await expect(deploy.getByRole("button", { name: "Approve" })).toBeVisible();
  await expect(deploy.getByText(/^Asked .* expires in/)).toBeVisible();
  // The newest pending card holds the beam (a still edge under reduced motion).
  await expect(page.locator(".beam-host[data-beam]")).toHaveCount(1);
  await page.mouse.move(0, 0);
  await shot(page, "agents-approvals", theme);

  await page.getByRole("tablist", { name: "Status" }).getByRole("tab", { name: "Denied" }).click();
  await expect(page).toHaveURL(/status=denied/);
  await expect(card(page, "Open 3 issues")).toContainText("Denied by Priya");
  await expect(card(page, "Open 3 issues")).toContainText("Two of the three are duplicates");
  await shot(page, "agents-approvals-denied", theme);
});

test("approving with a note moves the request out of Pending", async ({ page }) => {
  await openSection(page, `agents/${EMBER}/approvals?status=pending`);

  const deploy = card(page, "deploy production");

  await deploy.getByRole("button", { name: "Add a note" }).click();
  await deploy.getByRole("textbox", { name: "Note" }).fill("Ship it after standup");
  await deploy.getByRole("button", { name: "Approve" }).click();
  await expect(deploy).toHaveCount(0);
  await expect(page.getByRole("status").filter({ hasText: "Approved:" })).toBeAttached();

  await page
    .getByRole("tablist", { name: "Status" })
    .getByRole("tab", { name: "Approved" })
    .click();
  await expect(card(page, "deploy production")).toContainText("Approved by Riel St. Amand");
  await expect(card(page, "deploy production")).toContainText("Ship it after standup");

  // The server kept it.
  await page.reload();
  await expect(card(page, "deploy production")).toContainText("Approved by Riel St. Amand");
});

test("a decision made elsewhere and a new request arrive live", async ({ page }) => {
  await openSection(page, `agents/${EMBER}/approvals?status=pending`);
  await expect(card(page, "Merge PR #318")).toBeVisible();

  await control(page.request, "approval-settle", {
    id: 99,
    status: "denied",
    deciderId: USER_IDS.priya,
  });
  await expect(card(page, "Merge PR #318")).toHaveCount(0);
  // Said politely, once the burst settles; the first load said nothing.
  await expect(
    page.getByRole("status").filter({ hasText: "Denied by Priya Raman: Merge PR #318" }),
  ).toBeAttached();

  await control(page.request, "approval-request", { summary: "Merge PR #400 into main" });
  await expect(card(page, "Merge PR #400")).toBeVisible();
  await expect(card(page, "Merge PR #400").getByRole("button", { name: "Approve" })).toBeVisible();
});

test("deciding by keyboard in All keeps focus on the request", async ({ page }) => {
  await openSection(page, `agents/${EMBER}/approvals`);

  const deploy = card(page, "deploy production");

  await deploy.getByRole("button", { name: "Approve" }).focus();
  await page.keyboard.press("Enter");
  await expect(deploy.getByRole("button", { name: "Approve" })).toHaveCount(0);
  await expect(deploy).toContainText("Approved by Riel St. Amand");
  await expect(deploy.locator(".approval-summary")).toBeFocused();
});

test("the approvals list pages past the first fifty", async ({ page }) => {
  const second = page.waitForResponse(
    (response) => response.url().includes("/approvals?") && response.url().includes("before="),
  );

  await openSection(page, `agents/${EMBER}/approvals`);
  await expect(card(page, "deploy production")).toBeVisible();
  await page.locator(".page-list-scroll").evaluate((element) => element.scrollBy(0, 100_000));
  expect((await second).status()).toBe(200);
});

matrix("an owner who isn't an administrator", async ({ page, theme }) => {
  await control(page.request, "viewer-role", { role: "member" });
  await openSection(page, `agents/${EMBER}/approvals?status=pending`, theme);

  const merge = card(page, "Merge PR #318");

  await expect(merge.getByRole("button", { name: "Deny" })).toBeVisible();
  await expect(merge.getByRole("button", { name: "Approve" })).toHaveCount(0);
  await expect(merge).toContainText("Only an administrator can approve GitHub write actions.");
  await expect(merge).toContainText("Acts on GitHub as @ember-bot");
  await expect(card(page, "disk alert")).toContainText("a room you're not in");
  await page.mouse.move(0, 0);
  await shot(page, "agents-approvals-member", theme);
});

// --- activity ledger ---

matrix("an agent's activity ledger", async ({ page, theme }) => {
  await openSection(page, `agents/${EMBER}/events`, theme);

  const rows = page.locator(".ledger-row");

  await expect(rows.first()).toBeVisible();
  await expect(
    page.getByText("GitHub merge_pull_request: succeeded", { exact: false }),
  ).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "agents-ledger", theme);

  await page
    .getByRole("tablist", { name: "Outcome" })
    .getByRole("tab", { name: "Suppressed" })
    .click();
  await expect(page).toHaveURL(/outcome=suppressed/);
  await expect(page.getByText("Not delivered: rate limit").first()).toBeVisible();
  await shot(page, "agents-ledger-suppressed", theme);
});

test("the ledger skips unknown entries, keeps a short page's cursor, and links to messages", async ({
  page,
}) => {
  await openSection(page, `agents/${EMBER}/events`);

  const first: AgentLedgerPage = await (
    await page.request.get(`/api/v1/agents/${EMBER}/events`)
  ).json();

  expect(first.events).toHaveLength(48);
  expect(first.nextCursor).not.toBeNull();
  expect(first.events.map((event) => event.eventType)).not.toContain("budget_threshold_crossed");

  await scrollUntil(page, page.getByText("Handoff:").first());
  await expect(page.getByText("Budget threshold crossed")).toHaveCount(0);
  await scrollUntil(page, page.getByText("Webhook failed · 3 attempts", { exact: false }).first());
  await scrollUntil(page, page.getByText("ops-oncall").first());

  await page.locator(".page-list-scroll").evaluate((element) => element.scrollTo(0, 0));
  await page.getByRole("link", { name: "View message" }).first().click();
  await expect(page).toHaveURL(/\/app\/r\/\d+\/m\/\d+$/);
});

test("the ledger hides rooms an owner isn't in", async ({ page }) => {
  await control(page.request, "viewer-role", { role: "member" });
  await openSection(page, `agents/${EMBER}/events`);
  await scrollUntil(page, page.getByText("a room you're not in").first());
  await expect(page.getByText("ops-oncall")).toHaveCount(0);
});

test("a ledger closed mid-session says so, with no Retry", async ({ page }) => {
  await openSection(page, `agents/${SCOUT}`);
  await control(page.request, "viewer-role", { role: "member" });
  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(page.getByText("Activity is private")).toBeVisible();
  await expect(page.getByRole("button", { name: /try again|retry/i })).toHaveCount(0);
});

test("someone else's agent shows only its overview", async ({ page }) => {
  await control(page.request, "viewer-role", { role: "member" });
  await openApp(page, `agents/${SCOUT}`);
  await expect(page.getByRole("heading", { level: 2, name: "Scout" })).toBeVisible();
  await expect(page.getByRole("tablist", { name: "Sections" })).toHaveCount(0);

  await openApp(page, `agents/${SCOUT}/approvals`);
  await expect(page.getByText("Only for administrators and its owner")).toBeVisible();
});

test("an approval request in the inbox opens the agent's approvals", async ({ page }) => {
  await control(page.request, "pause");
  await openApp(page, "activity?tab=agents");
  await page.getByText("Merge PR #318 into main").first().click();
  await expect(page).toHaveURL(new RegExp(`/app/agents/${EMBER}/approvals$`));
  await expect(card(page, "Merge PR #318")).toBeVisible();
});
