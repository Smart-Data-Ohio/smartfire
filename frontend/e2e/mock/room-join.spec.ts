import type { Page } from "@playwright/test";
import { JOINABLE_OLDEST_MESSAGE_ID, JOINABLE_OPEN_ROOM } from "../../mock/seed.ts";
import { DESKTOP, expect, openApp, postMessage, syncWelcomed, test, USER_IDS } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

/** `/__mock/state`, including `pendingJoins` and `presentRoomIds`. */
function mockState(page: Page) {
  return page.request.get("/__mock/state").then((response) => response.json());
}

async function control(page: Page, action: string, data: { readonly on?: boolean } = {}) {
  const state = await mockState(page);

  const response = await page.request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  if (!response.ok()) {
    throw new Error(`${action} answered ${response.status()}`);
  }
}

test("a public room you haven't joined previews, then joins into the sidebar", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}`);

  const preview = page.getByRole("region", { name: "Join #campfire" });

  await expect(preview.getByRole("heading", { name: "#campfire" })).toBeVisible();
  await expect(preview.getByText("You're not a member of this channel.")).toBeVisible();
  await expect(preview.getByText(/\d+ members/)).toBeHidden();
  await expect(page.getByRole("region", { name: "Conversation" })).toBeHidden();
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ })).toBeHidden();

  await preview.getByRole("button", { name: "Join channel" }).click();

  await expect(page.getByRole("region", { name: "Conversation" })).toBeVisible();
  await expect(page.getByRole("link", { name: "campfire, room settings" })).toBeVisible();
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ })).toBeVisible();
});

test("after joining, a live message arrives and the room is present", async ({ page }) => {
  await page.setViewportSize(DESKTOP);

  const welcomed = syncWelcomed(page);

  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}`);
  await welcomed;
  await page.getByRole("button", { name: "Join channel" }).click();

  // The newest page is loaded after the membership's subscription and presence.
  await expect(page.getByText("campfire-newest")).toBeVisible();

  const joined = await mockState(page);

  expect(joined.presentRoomIds).toContain(JOINABLE_OPEN_ROOM.id);

  await postMessage(page.request, {
    roomId: JOINABLE_OPEN_ROOM.id,
    userId: USER_IDS.maya,
    markdown: "welcome in",
  });

  await expect(page.getByText("welcome in")).toBeVisible();
  await expect(
    sidebar(page).locator(`[data-room-id="${JOINABLE_OPEN_ROOM.id}"] .sidebar-row`),
  ).not.toHaveAttribute("data-state", "unread");
});

test("leaving during a join does not stay present in that room", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}`);

  const preview = page.getByRole("region", { name: "Join #campfire" });

  await expect(preview).toBeVisible();
  await control(page, "hold-join");
  await preview.getByRole("button", { name: "Join channel" }).click();
  await expect.poll(async () => (await mockState(page)).pendingJoins).toBe(1);

  await sidebar(page)
    .locator(".sidebar-row-name", { hasText: /^general$/ })
    .click();
  await expect(page).toHaveURL(/\/r\/1$/);

  const joined = page.waitForResponse(
    (response) =>
      response.url().includes(`/rooms/${JOINABLE_OPEN_ROOM.id}/join`) &&
      response.request().method() === "POST",
  );

  await control(page, "hold-join", { on: false });
  await joined;
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ })).toBeVisible();
  await page.waitForLoadState("networkidle");

  const state = await mockState(page);

  expect(state.presentRoomIds).not.toContain(JOINABLE_OPEN_ROOM.id);
  expect(state.presentRoomIds).toContain(1);
});

test("restarting the server while previewing keeps the join page", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}`);

  const preview = page.getByRole("region", { name: "Join #campfire" });

  await expect(preview).toBeVisible();

  const refetched = page.waitForResponse(
    (response) =>
      response.url().includes(`/rooms/${JOINABLE_OPEN_ROOM.id}/preview`) &&
      response.request().method() === "GET",
  );

  const welcomed = syncWelcomed(page);

  await control(page, "restart");
  await welcomed;
  await refetched;
  await expect(preview).toBeVisible();
  await expect(page.getByRole("region", { name: "Room unavailable" })).toBeHidden();
});

test("joining from a permalink stays on that message", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}/m/${JOINABLE_OLDEST_MESSAGE_ID}`);
  await expect(page.getByRole("region", { name: "Join #campfire" })).toBeVisible();
  await page.getByRole("button", { name: "Join channel" }).click();
  await expect(page.locator("[data-focused]", { hasText: "campfire-oldest" })).toBeVisible();
  await expect(page.getByText("campfire-newest")).toHaveCount(0);
});

test("a held join still opens the room you returned to after a rename", async ({ page }) => {
  await page.setViewportSize(DESKTOP);

  const welcomed = syncWelcomed(page);

  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}/m/${JOINABLE_OLDEST_MESSAGE_ID}`);
  await welcomed;
  await page.evaluate(() => {
    document.documentElement.dataset.roomJoin = "1";
  });

  const preview = page.getByRole("region", { name: "Join #campfire" });

  await expect(preview).toBeVisible();
  await control(page, "hold-join");
  await preview.getByRole("button", { name: "Join channel" }).click();
  await expect.poll(async () => (await mockState(page)).pendingJoins).toBe(1);

  const state = await mockState(page);

  const renamed = await page.request.patch(`/api/v1/rooms/${JOINABLE_OPEN_ROOM.id}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { type: "open", name: "bonfire" },
  });

  expect(renamed.ok()).toBeTruthy();
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^bonfire$/ })).toBeVisible();

  await sidebar(page)
    .locator(".sidebar-row-name", { hasText: /^general$/ })
    .click();
  await expect(page).toHaveURL(/\/r\/1$/);

  await sidebar(page)
    .locator(".sidebar-row-name", { hasText: /^bonfire$/ })
    .click();
  await expect(page).toHaveURL(new RegExp(`/r/${JOINABLE_OPEN_ROOM.id}$`));
  await expect(page.getByText("campfire-newest")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.dataset.roomJoin)).toBe("1");

  const joined = page.waitForResponse(
    (response) =>
      response.url().includes(`/rooms/${JOINABLE_OPEN_ROOM.id}/join`) &&
      response.request().method() === "POST",
  );

  await control(page, "hold-join", { on: false });
  await joined;
  await expect(page.getByRole("link", { name: "bonfire, room settings" })).toBeVisible();
  await expect(page.getByText("campfire-newest")).toBeVisible();
  await expect(page.getByText("campfire-oldest")).toHaveCount(0);

  const after = await mockState(page);

  expect(after.presentRoomIds).toContain(JOINABLE_OPEN_ROOM.id);

  await postMessage(page.request, {
    roomId: JOINABLE_OPEN_ROOM.id,
    userId: USER_IDS.maya,
    markdown: "still here",
  });
  await expect(page.getByText("still here")).toBeVisible();
});

test("a missing room stays unavailable", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, "r/999999");

  await expect(page.getByRole("region", { name: "Room unavailable" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Join channel" })).toBeHidden();
});
