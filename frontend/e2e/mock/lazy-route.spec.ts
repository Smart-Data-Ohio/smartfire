import type { Page } from "@playwright/test";
import { expect, openApp, ROOM_IDS, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

/** Long enough that the placeholder is on screen before the chunk is allowed through. */
const CHUNK_DELAY_MS = 600;

test("a slow lazy route keeps the sidebar up until its screen arrives", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();

  const eventsChunk = page.waitForRequest((request) =>
    new URL(request.url()).pathname.endsWith("/events-page.tsx"),
  );

  await page.getByRole("link", { name: "Events" }).hover();
  await eventsChunk;
  await expect(sidebar(page)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

  await page.route(
    (url) => url.pathname.endsWith("/activity-route.tsx"),
    async (route) => {
      await new Promise((resolve) => setTimeout(resolve, CHUNK_DELAY_MS));
      await route.continue();
    },
  );

  await page.getByRole("button", { name: "Activity" }).click();

  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
  await expect(page.getByRole("status", { name: "Loading page" })).toBeVisible();

  await expect(page.getByRole("heading", { name: "Activity" })).toBeVisible();
  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
});
