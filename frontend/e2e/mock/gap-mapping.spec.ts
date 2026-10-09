import type { Page } from "@playwright/test";
import { forbidden, notFound } from "../../mock/http.ts";
import { MESSAGE_IDS, THREAD_IDS } from "../../mock/s2/seed.ts";
import { seededMessageId } from "../../mock/seed.ts";
import { expect, openHeaderTool, ROOM_IDS, test } from "./support.ts";

const ROOM = ROOM_IDS.general;

const ACTIVE_THREAD = "Invite-to-first-message conversion dip";

async function open(page: Page, path: string): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

test("a bare message lands at its room permalink and keeps the query and hash", async ({
  page,
}) => {
  await open(page, `m/${MESSAGE_IDS.generalSaved}?source=classic&source=link#top`);

  await expect(page).toHaveURL(
    new RegExp(`/app/r/${ROOM}/m/${MESSAGE_IDS.generalSaved}\\?source=classic&source=link#top$`),
  );
  await expect(page.locator(`[data-message-id="${MESSAGE_IDS.generalSaved}"]`)).toBeVisible();
});

test("a bare reply opens its existing thread and focuses that reply", async ({ page }) => {
  await open(
    page,
    `m/${MESSAGE_IDS.generalThreadViewerReply}?source=classic&m=999&source=link#reply`,
  );

  await expect(page).toHaveURL(
    new RegExp(
      `/app/r/${ROOM}/t/${THREAD_IDS.generalActive}\\?source=classic&m=${MESSAGE_IDS.generalThreadViewerReply}&source=link#reply$`,
    ),
  );
  await expect(pane(page).getByRole("heading", { name: ACTIVE_THREAD })).toBeVisible();
  await expect(
    pane(page).locator(`[data-message-id="${MESSAGE_IDS.generalThreadViewerReply}"]`),
  ).toBeVisible();
});

test("a classic bare-message link resolves in place with repeated query keys intact", async ({
  page,
}) => {
  await open(page, `r/${ROOM_IDS.quiet}`);
  await page.evaluate((messageId) => {
    const link = document.createElement("a");

    link.href = `/messages/${messageId}/boosts?source=classic&source=link#top`;
    link.textContent = "open a classic message";
    document.querySelector("main")?.append(link);
    Object.assign(window, { resolverDocument: true });
  }, MESSAGE_IDS.generalSaved);
  await page.getByText("open a classic message").click();
  await expect(page).toHaveURL(
    new RegExp(`/app/r/${ROOM}/m/${MESSAGE_IDS.generalSaved}\\?source=classic&source=link#top$`),
  );
  expect(await page.evaluate(() => "resolverDocument" in window)).toBe(true);
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.quiet}$`));
});

for (const { path, title } of [
  { path: "threads", title: "Threads" },
  { path: "files", title: "Files" },
  { path: "pins", title: "Pinned messages" },
]) {
  test(`the mapped ${path} URL opens the existing pane and close clears its URL`, async ({
    page,
  }) => {
    await open(page, `r/${ROOM}/${path}`);

    await expect(pane(page).getByRole("heading", { name: title })).toBeVisible();
    await pane(page).getByRole("button", { name: "Close", exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
    await expect(pane(page)).toHaveCount(0);
    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/${path}$`));
    await expect(pane(page).getByRole("heading", { name: title })).toBeVisible();
  });
}

test("a thread returns to the routed Threads list and header toggles keep the URL in sync", async ({
  page,
}) => {
  await open(page, `r/${ROOM}/threads`);
  await pane(page)
    .getByRole("button", { name: new RegExp(ACTIVE_THREAD) })
    .click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/t/${THREAD_IDS.generalActive}$`));
  await pane(page).getByRole("button", { name: "Back to threads" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/threads$`));
  await expect(pane(page).getByRole("heading", { name: "Threads" })).toBeVisible();

  await page.locator(".room-header").getByRole("button", { name: "Files" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/files$`));
  await expect(pane(page).getByRole("heading", { name: "Files" })).toBeVisible();
  await page.locator(".room-header").getByRole("button", { name: "Files" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
  await expect(pane(page)).toHaveCount(0);
});

test("the involvement URL opens the existing notification menu and selection leaves that URL", async ({
  page,
}) => {
  await open(page, `r/${ROOM}/notifications`);

  const menu = page.getByRole("menu", { name: "Notifications", exact: true });

  await expect(menu).toBeVisible();
  await menu.getByRole("menuitemradio", { name: "No notifications" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
  await expect(menu).toHaveCount(0);
  await expect(
    page.locator(".room-header").getByRole("button", { name: "Notifications: No notifications" }),
  ).toBeVisible();
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/notifications$`));
  await expect(menu).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
  await expect(menu).toHaveCount(0);
});

test("browser Back closes a routed notification menu after keyboard use of its trigger", async ({
  page,
}) => {
  await open(page, `r/${ROOM}`);
  await page.evaluate((roomId) => {
    const link = document.createElement("a");

    link.href = `/rooms/${roomId}/involvement`;
    link.textContent = "open room notifications";
    document.querySelector("main")?.append(link);
  }, ROOM);
  await page.getByText("open room notifications").click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/notifications$`));

  const menu = page.getByRole("menu", { name: "Notifications", exact: true });

  await expect(menu).toBeVisible();
  await page
    .locator(".room-header")
    .getByRole("button", { name: /^Notifications:/ })
    .focus();
  await page.keyboard.press("ArrowDown");
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
  await expect(menu).toHaveCount(0);
});

test.describe("phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("Back to the notification URL closes a Threads pane opened after Escape", async ({
    page,
  }) => {
    await open(page, `r/${ROOM}/notifications`);

    const menu = page.getByRole("menu", { name: "Notifications", exact: true });
    const conversation = page.getByRole("region", { name: "Conversation" });

    await expect(menu).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
    await expect(menu).toHaveCount(0);

    // The ⋯ menu opens Threads as a local pane, a full page over the conversation.
    await openHeaderTool(page, "Threads");
    await expect(pane(page)).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
    await expect(conversation).toHaveAttribute("inert", "");

    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}/notifications$`));
    await expect(menu).toBeVisible();
    await expect(pane(page)).toHaveCount(0);
    await expect(conversation).not.toHaveAttribute("inert");

    // Focus lands in the menu, and the keyboard alone chooses a level, which leaves the URL.
    await expect(menu.getByRole("menuitemradio", { name: "All messages" })).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(menu.getByRole("menuitemradio", { name: "No notifications" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM}$`));
    await expect(menu).toHaveCount(0);

    // The ⋯ menu's notification levels show the new one.
    await page.locator(".room-header").getByRole("button", { name: "More" }).click();
    await page.getByRole("menuitem", { name: "Notifications" }).click();
    await expect(page.getByRole("menuitemradio", { name: /^No notifications/ })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });
});

test("an unknown bare message stays on the existing not-found state", async ({ page }) => {
  await open(page, "m/999999999");

  await expect(page.getByRole("region", { name: "Page not found" })).toBeVisible();
  await expect(page).toHaveURL(/\/app\/m\/999999999$/);
  await expect(page.getByRole("region", { name: "Conversation", exact: true })).toHaveCount(0);
});

test("a forbidden bare message never opens its conversation or redirects to classic", async ({
  page,
}) => {
  await page.route(`**/api/v1/messages/${MESSAGE_IDS.generalSaved}`, (route) =>
    route.fulfill({
      status: 403,
      contentType: "application/json",
      body: JSON.stringify({ error: forbidden("Access denied").error }),
    }),
  );
  await open(page, `m/${MESSAGE_IDS.generalSaved}`);

  await expect(page.getByRole("region", { name: "Page not found" })).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/app/m/${MESSAGE_IDS.generalSaved}$`));
  await expect(page.getByRole("region", { name: "Conversation", exact: true })).toHaveCount(0);
});

for (const status of [403, 404]) {
  for (const path of [
    "threads",
    "files",
    "pins",
    "notifications",
    `m/${seededMessageId(ROOM_IDS.launchPlanning, 0)}`,
  ]) {
    test(`a denied room ${path} route (${status}) exposes no conversation content`, async ({
      page,
    }) => {
      const roomId = ROOM_IDS.launchPlanning;

      await page.route(new RegExp(`/api/v1/rooms/${roomId}(?:/|\\?|$)`), (route) =>
        route.fulfill({
          status,
          contentType: "application/json",
          body: JSON.stringify({
            error: (status === 403 ? forbidden() : notFound()).error,
          }),
        }),
      );
      await open(page, `r/${roomId}/${path}`);

      await expect(page.getByRole("region", { name: "Room unavailable" })).toBeVisible();
      await expect(page).toHaveURL(new RegExp(`/app/r/${roomId}/${path}$`));
      await expect(page.getByRole("region", { name: "Conversation", exact: true })).toHaveCount(0);
      await expect(page.locator("[data-message-id]")).toHaveCount(0);
      await expect(pane(page)).toHaveCount(0);
      await expect(page.getByRole("menu", { name: "Notifications", exact: true })).toHaveCount(0);
    });
  }
}

test("a stale resolver response cannot replace a newer navigation", async ({ page }) => {
  const held = Promise.withResolvers<void>();
  const requested = Promise.withResolvers<void>();

  await page.route(`**/api/v1/messages/${MESSAGE_IDS.generalSaved}`, async (route) => {
    const response = await route.fetch();

    requested.resolve();
    await held.promise;
    await route.fulfill({ response });
  });
  await open(page, `m/${MESSAGE_IDS.generalSaved}`);
  await requested.promise;
  // A classic link exercises in-place navigation while the resolver remains in flight.
  await page.evaluate((roomId) => {
    const link = document.createElement("a");

    link.href = `/rooms/${roomId}`;
    link.textContent = "open another conversation";
    document.querySelector("main")?.append(link);
  }, ROOM_IDS.quiet);
  await page.getByText("open another conversation").click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.quiet}$`));
  const finished = page.waitForResponse(`**/api/v1/messages/${MESSAGE_IDS.generalSaved}`);

  held.resolve();
  await finished;
  await page.evaluate(() => new Promise(requestAnimationFrame));
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.quiet}$`));
});
