import type { Page } from "@playwright/test";
import { JOINABLE_OPEN_ROOM } from "../../mock/seed.ts";
import { DESKTOP, expect, openApp, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

test("a public room you haven't joined previews, then joins into the sidebar", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${JOINABLE_OPEN_ROOM.id}`);

  const preview = page.getByRole("region", { name: "Join #campfire" });

  await expect(preview.getByRole("heading", { name: "#campfire" })).toBeVisible();
  await expect(preview.getByText("You're not a member of this channel.")).toBeVisible();
  await expect(preview.getByText("3 members")).toBeVisible();
  await expect(page.getByRole("region", { name: "Conversation" })).toBeHidden();
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ })).toBeHidden();

  await preview.getByRole("button", { name: "Join channel" }).click();

  await expect(page.getByRole("region", { name: "Conversation" })).toBeVisible();
  await expect(page.getByRole("link", { name: "campfire, room settings" })).toBeVisible();
  await expect(sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ })).toBeVisible();
});

test("a missing room stays unavailable", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, "r/999999");

  await expect(page.getByRole("region", { name: "Room unavailable" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Join channel" })).toBeHidden();
});
