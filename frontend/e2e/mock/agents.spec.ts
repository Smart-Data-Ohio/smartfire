import type { APIRequestContext, Page } from "@playwright/test";
import { AGENT_IDS } from "../../mock/s4/agents.ts";
import { expect, matrix, openApp, openHeaderTool, ROOM_IDS, SHOTS, shot, test } from "./support.ts";

const { ember: EMBER, scout: SCOUT, quill: QUILL } = AGENT_IDS;

/** The fields the agent controls read. */
interface AgentControl {
  readonly agentId?: number;
  readonly status?: string;
  readonly suspended?: boolean;
  readonly presence?: string;
  readonly roomId?: number;
  readonly stage?: number;
}

/** Calls one of the mock's agent controls with `data`. */
async function control(request: APIRequestContext, action: string, data: AgentControl) {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok()).toBe(true);
}

function rows(page: Page) {
  return page.locator(".agent-row");
}

/** Waits for the directory's rows, then moves the pointer out of the way. */
async function directoryReady(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1, name: "Agents" })).toBeVisible();
  await expect(rows(page).first()).toBeVisible();
  await page.mouse.move(0, 0);
}

/**
 * Opens a room and waits for its conversation (a phone in a room hides the sidebar) to come to
 * rest at the present. While the opening scroll settles, virtua turns off pointer events on its
 * rows, so a click then lands on the list and Playwright's retry scrolls the target into view: on
 * a phone that carries the timeline to the top and unmounts the latest messages.
 */
async function openRoom(page: Page, roomId: number, theme: "light" | "dark") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${roomId}`);
  await page.getByRole("main").waitFor();

  const log = page.getByRole("log", { name: "Messages" });

  await expect(log).toHaveAttribute("data-placement-settled", "true");
  await expect(log).toHaveAttribute("data-scroll-settled", "true");
}

// --- directory ---

// Screenshots only: agents.test.tsx covers the filters, and the tests below the rows.
if (SHOTS) {
  matrix("the agent directory", async ({ page, theme }) => {
    await openApp(page, "agents", theme);
    await directoryReady(page);
    await expect(rows(page)).toHaveCount(5);
    // Active agents first; the suspended one sorts last and says so.
    await expect(rows(page).last()).toContainText("Quill");
    await expect(rows(page).last()).toContainText("Suspended");
    await shot(page, "agents-directory", theme);

    await page.getByRole("tab", { name: "Personal" }).click();
    await expect(rows(page)).toHaveCount(2);
    await shot(page, "agents-directory-personal", theme);

    await page.getByRole("tab", { name: "All" }).click();
    await page.getByRole("searchbox", { name: "Find an agent or owner" }).fill("zzz");
    await expect(page.getByText(/No agents match/)).toBeVisible();
    await shot(page, "agents-directory-nomatch", theme);
  });
}

test("the directory finds an agent by its owner, and opens its profile", async ({ page }) => {
  await openApp(page, "agents");
  await directoryReady(page);
  await page.getByRole("tab", { name: "Personal" }).click();
  await expect(rows(page)).toHaveCount(2);
  await page.getByRole("tab", { name: "All" }).click();
  await page.getByRole("searchbox", { name: "Find an agent or owner" }).fill("zzz");
  await expect(page.getByText(/No agents match/)).toBeVisible();
  await page.getByRole("searchbox", { name: "Find an agent or owner" }).fill("theo");
  await expect(rows(page)).toHaveCount(1);
  await expect(rows(page).first()).toContainText("Scout");
  await rows(page).first().getByRole("link").click();
  await expect(page).toHaveURL(new RegExp(`/app/agents/${SCOUT}$`));
  await expect(page.getByRole("heading", { level: 2, name: "Scout" })).toBeVisible();
});

test("the sidebar's Agents destination opens the directory", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await page
    .getByRole("list", { name: "Destinations" })
    .getByRole("link", { name: "Agents" })
    .click();
  await expect(page).toHaveURL(/\/app\/agents$/);
  await directoryReady(page);
});

test("the directory keeps up with agent.status", async ({ page, request }) => {
  await openApp(page, "agents");
  await directoryReady(page);

  const quill = rows(page).filter({ hasText: "Quill" });

  await expect(quill).toHaveAttribute("data-suspended", "true");
  await control(request, "agent-status", { agentId: QUILL, suspended: false, status: "idle" });
  await expect(quill).not.toHaveAttribute("data-suspended", "true");
  await expect(quill).not.toContainText("Suspended");
});

// --- profile ---

// Screenshots only: agents.test.tsx covers the profile and the suspended badge.
if (SHOTS) {
  matrix("an agent's profile", async ({ page, theme }) => {
    await openApp(page, `agents/${SCOUT}`, theme);
    await expect(page.getByRole("heading", { level: 2, name: "Scout" })).toBeVisible();
    await expect(page.getByText("Personal agent of Theo", { exact: false })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Capabilities" })).toBeVisible();
    await expect(page.getByRole("meter").first()).toBeVisible();
    await page.mouse.move(0, 0);
    await shot(page, "agents-profile", theme);

    await openApp(page, `agents/${QUILL}`, theme);
    await expect(page.getByRole("heading", { level: 2, name: "Quill" })).toBeVisible();
    await expect(page.locator(".agent-hero")).toContainText("Suspended");
    await shot(page, "agents-profile-suspended", theme);
  });
}

test("an unknown agent says so and links back", async ({ page }) => {
  await openApp(page, "agents/999");
  await expect(page.getByText("No such agent")).toBeVisible();
  await page
    .getByRole("link", { name: /agents/i })
    .last()
    .click();
  await expect(page).toHaveURL(/\/app\/agents$/);
});

// --- in the conversation ---

matrix("steps and working presence in the Ember DM", async ({ page, theme, phone }) => {
  await openRoom(page, ROOM_IDS.dmEmber, theme);

  const toggle = page.locator(".steps-toggle").first();

  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await shot(page, "agents-steps", theme);

  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator(".steps-list").first().locator(".step").first()).toBeVisible();
  await shot(page, "agents-steps-open", theme);

  await control(request(page), "agent-status", {
    agentId: EMBER,
    status: "working",
    presence: "Summarising #engineering",
  });
  await control(request(page), "agent-steps", { roomId: ROOM_IDS.dmEmber, stage: 1 });

  await expect(page.getByText("Ember is working")).toBeVisible();
  await expect(page.locator(".beam-host[data-beam]").first()).toHaveAttribute(
    "data-beam",
    "static",
  );
  await expect(page.locator(".steps-current").last()).toContainText("Run the test suite");

  await expect(page.locator(".typing-line[data-working]")).toContainText(
    "Summarising #engineering",
  );

  if (!phone) {
    // The header's subtitle carries the live status too.
    await expect(page.locator(".room-header")).toContainText("Summarising #engineering");
  }

  await shot(page, "agents-working", theme);

  await control(request(page), "agent-status", { agentId: EMBER, status: "idle", presence: "" });
  await expect(page.getByText("Ember is working")).toBeHidden();
  await expect(page.locator(".beam-host[data-beam]")).toHaveCount(0);
});

test("clicking an agent's name in a message opens its profile", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.dmEmber}`);
  await page.getByRole("main").getByRole("link", { name: "Ember" }).first().click();
  await expect(page).toHaveURL(new RegExp(`/app/agents/${EMBER}$`));
  await expect(page.getByRole("heading", { level: 2, name: "Ember" })).toBeVisible();
});

// Screenshots only: agents.test.tsx covers the badge.
if (SHOTS) {
  matrix("the agent badge in the members pane", async ({ page, theme }) => {
    await openRoom(page, ROOM_IDS.general, theme);
    await openHeaderTool(page, /^Members/);

    const pane = page.locator("aside.right-pane");
    const ember = pane.locator("li").filter({ hasText: "Ember" }).first();

    await expect(ember.locator(".agent-badge")).toBeVisible();
    await page.mouse.move(0, 0);
    await shot(page, "agents-members", theme);
  });
}

/** The page's request context, so a matrix body can call the controls. */
function request(page: Page): APIRequestContext {
  return page.request;
}
