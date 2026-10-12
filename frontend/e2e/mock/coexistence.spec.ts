import { forbidden } from "../../mock/http.ts";
import { expect, matrix, openApp, ROOM_IDS, SHOTS, shot, test } from "./support.ts";

matrix("the user menu has no way back to classic", async ({ page, theme, phone }) => {
  await openApp(page, "", theme);

  // A phone has it under the tab bar's You, the sidebar's foot on wider screens.
  await page.getByRole("button", { name: phone ? "You" : "Your account" }).click();

  const menu = page.getByRole("menu", { name: "Your account" });

  await expect(menu.getByRole("menuitem", { name: "Profile and settings" })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /classic/i })).toHaveCount(0);

  if (SHOTS) {
    // The menu's entrance finishes before the shot.
    await page.evaluate(() =>
      Promise.all(
        document
          .getAnimations()
          .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
          .map((animation) => animation.finished.catch(() => animation)),
      ),
    );
    await shot(page, "user-menu", theme);
  }
});

test("a link to a ported classic page opens in place", async ({ page }) => {
  await openApp(page, "");

  await page.evaluate((roomId) => {
    const link = document.createElement("a");

    link.href = `/rooms/${roomId}`;
    link.textContent = "classic link";
    document.querySelector("main")?.append(link);
    Object.assign(window, { stillHere: true });
  }, ROOM_IDS.general);
  await page.getByText("classic link").click();

  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));
  expect(await page.evaluate(() => "stillHere" in window)).toBe(true);
});

test("the classic new direct message page opens the New message picker", async ({ page }) => {
  await openApp(page, "rooms/new/direct");

  const dialog = page.getByRole("dialog", { name: "New message" });

  await expect(dialog).toBeVisible();
  // The picker sits over the home screen; the URL doesn't stay in history.
  await expect(page).not.toHaveURL(/rooms\/new/);
});

test("a link to the classic new direct message page opens the picker in place", async ({
  page,
}) => {
  await openApp(page, "");
  await page.evaluate(() => {
    const link = document.createElement("a");

    link.href = "/rooms/directs/new";
    link.textContent = "classic new message";
    document.querySelector("main")?.append(link);
    Object.assign(window, { stillHere: true });
  });
  await page.getByText("classic new message").click();

  await expect(page.getByRole("dialog", { name: "New message" })).toBeVisible();
  expect(await page.evaluate(() => "stillHere" in window)).toBe(true);
});

test("an unknown route stays in the SPA and Go home recovers", async ({ page }) => {
  await page.goto("/app/missing-page?classic=1");
  await expect(page.getByRole("region", { name: "Page not found" })).toBeVisible();
  await expect(page.getByRole("link", { name: /classic/i })).toHaveCount(0);
  await page.getByRole("link", { name: "Go home" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));
  await expect(page.getByRole("region", { name: "Conversation", exact: true })).toBeVisible();
});

test("an unavailable room retries without leaving the SPA", async ({ page }) => {
  const path = `**/api/v1/rooms/${ROOM_IDS.general}`;

  await page.route(path, (route) =>
    route.fulfill({
      status: 403,
      contentType: "application/json",
      body: JSON.stringify({ error: forbidden().error }),
    }),
  );
  await openApp(page, `r/${ROOM_IDS.general}`);
  const error = page.getByRole("region", { name: "Room unavailable" });

  await expect(error).toBeVisible();
  await expect(error.getByRole("link")).toHaveCount(0);
  await page.evaluate(() => Object.assign(window, { retryDocument: true }));
  await page.unroute(path);
  await error.getByRole("button", { name: "Try again" }).click();
  await expect(page.getByRole("region", { name: "Conversation", exact: true })).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));
  expect(await page.evaluate(() => "retryDocument" in window)).toBe(true);
});

test("an activity item with an unknown old path reaches the SPA not-found page in place", async ({
  page,
}) => {
  await page.route("**/api/v1/activity?*", async (route) => {
    const response = await route.fetch();
    const list = await response.json();
    const first = list.items[0];

    await route.fulfill({
      json: {
        ...list,
        items: [
          {
            ...first,
            source: {
              ...first.source,
              sourceType: "agent_approval",
              roomId: null,
              threadId: null,
              messageId: null,
              title: "Retired activity destination",
              path: "/retired-page?source=mail#top",
            },
          },
        ],
      },
    });
  });
  await openApp(page, "activity");
  await page.evaluate(() => Object.assign(window, { activityDocument: true }));
  await page.getByText("Retired activity destination").click();
  await expect(page).toHaveURL(/\/app\/retired-page\?source=mail#top$/);
  await expect(page.getByRole("region", { name: "Page not found" })).toBeVisible();
  expect(await page.evaluate(() => "activityDocument" in window)).toBe(true);
});
