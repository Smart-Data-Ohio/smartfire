import type { Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  openApp,
  PHONE,
  ROOM_IDS,
  shot,
  THEMES,
  test,
} from "./support.ts";

/**
 * The note-to-self, Slack's "message yourself": first in Direct messages as "Your Name (you)"
 * before it exists, created on first click, offered in the new-DM picker, and its room says it
 * is your space with no call, notifications or add-people.
 */

const ME = "Riel St. Amand";

const LINE = "This is your space. Draft messages, keep links, or jot notes for later.";

const MOD = process.platform === "darwin" ? "Meta" : "Control";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

const offer = (page: Page) => sidebar(page).locator("[data-note-to-self-offer]");

/** The room's header and intro say it is yours, and its call and people tools are gone. */
async function expectNoteToSelfRoom(page: Page) {
  const header = page.locator(".room-header");

  await expect(header.getByText(`${ME} (you)`)).toBeVisible();
  await expect(header.getByText(LINE)).toBeVisible();
  await expect(page.locator(".room-intro[data-note-to-self]")).toContainText(`${ME} (you)`);
  await expect(page.locator(".room-intro[data-note-to-self]")).toContainText(LINE);
  await expect(page.getByPlaceholder("Jot something down")).toBeVisible();
  await expect(header.getByRole("button", { name: /huddle|call/i })).toHaveCount(0);
  await expect(header.getByRole("button", { name: "Add people" })).toHaveCount(0);
}

for (const theme of THEMES) {
  test(`the sidebar offers it first and opens it on first use (${theme})`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openApp(page, `r/${ROOM_IDS.general}`, theme);

    // Offered before the room exists, ahead of every other conversation.
    await expect(offer(page)).toHaveCount(1);
    await expect(offer(page).getByRole("button")).toHaveAccessibleName(`${ME} (you)`);

    const directs = offer(page).locator("xpath=..");

    await expect(directs.locator(":scope > li").first()).toHaveAttribute(
      "data-note-to-self-offer",
      "true",
    );

    await shot(page, "note-to-self-offer", theme);

    await offer(page).getByRole("button").click();
    await expect(page).toHaveURL(/\/app\/r\/\d+/);
    await expectNoteToSelfRoom(page);

    // Now a row like any DM's: active, first, and named for you.
    await expect(offer(page)).toHaveCount(0);

    const row = sidebar(page).getByRole("link", { name: new RegExp(`^${ME} \\(you\\)`) });

    await expect(row).toHaveCount(1);
    await expect(row).toHaveAttribute("aria-current", "page");

    const composer = page.getByPlaceholder("Jot something down");

    await composer.fill("Buy milk");
    await composer.press("Enter");
    await expect(page.getByRole("main").getByText("Buy milk").first()).toBeVisible();

    // Your own message never makes it unread, even after you leave it.
    await sidebar(page)
      .getByRole("link", { name: /general/ })
      .first()
      .click();
    await expect(row).not.toHaveAttribute("data-state", "unread");
    await row.click();
    await shot(page, "note-to-self-room", theme);
  });

  test(`the new-DM picker offers you (${theme})`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openApp(page, `r/${ROOM_IDS.general}`, theme);
    await page.keyboard.press(`${MOD}+Shift+k`);

    const dialog = page.getByRole("dialog", { name: "New message" });
    const input = dialog.getByRole("combobox");

    await expect(dialog.getByRole("option").first()).toHaveText(new RegExp(`${ME} \\(you\\)`));
    await input.fill("riel");
    await input.press("Enter");
    await expect(dialog.getByRole("button", { name: "Open your notes" })).toBeEnabled();
    await shot(page, "note-to-self-picker", theme);

    // You beside someone else is just the DM with them.
    await input.fill("maya");
    await input.press("Enter");
    await dialog.getByRole("button", { name: "Start conversation" }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.dmMaya}$`));

    // Only you opens the note-to-self.
    await expect(dialog).toBeHidden();
    await expect(async () => {
      await page.keyboard.press(`${MOD}+Shift+k`);
      await expect(dialog).toBeVisible({ timeout: 1000 });
    }).toPass();
    await input.fill("riel");
    await input.press("Enter");
    await dialog.getByRole("button", { name: "Open your notes" }).click();
    await expectNoteToSelfRoom(page);
  });
}

test.describe("on a 390 px phone", () => {
  test.use({ viewport: PHONE, hasTouch: true, isMobile: true });

  for (const theme of THEMES) {
    test(`it leads the list and opens a room of your own (${theme})`, async ({ page }) => {
      await openApp(page, "", theme);
      await page
        .getByRole("navigation", { name: "Destinations" })
        .getByRole("button", { name: "DMs" })
        .tap();
      await expect(offer(page)).toBeVisible();
      await expect(offer(page)).toContainText("Jot something down");
      await expect(offer(page)).toContainText(`${ME} (you)`);
      await shot(page, "note-to-self-list", theme);

      await offer(page).getByRole("button").click();
      await expectNoteToSelfRoom(page);
      await expectNoHorizontalOverflow(page);
      await shot(page, "note-to-self-room", theme);

      // The ⋯ menu keeps the panes, events and search, but nobody to add or leave and no levels.
      const menu = page.getByRole("menu", { name: "More" });

      await page.locator(".room-header").getByRole("button", { name: "More" }).tap();
      await expect(menu.getByRole("menuitem", { name: "Search" })).toBeVisible();
      await expect(
        menu.getByRole("menuitem", { name: /Add people|Leave|Notifications/ }),
      ).toHaveCount(0);
      await page.keyboard.press("Escape");
      await expect(menu).toBeHidden();

      // The title opens its details: you, your space, and no levels or people to add.
      const pane = page.locator("aside.right-pane");

      await page.locator(".room-header .room-title-button").tap();
      await expect(pane.getByRole("heading", { name: "Details" })).toBeVisible();
      await expect(pane.locator(".details-name")).toHaveText(`${ME} (you)`);
      await expect(pane.getByText(LINE)).toBeVisible();
      await expect(pane.getByRole("button", { name: /^Pinned messages/ })).toBeVisible();
      await expect(pane.getByRole("button", { name: /^Notifications/ })).toHaveCount(0);
      await expect(pane.getByRole("button", { name: /Add people|Rename|Leave/ })).toHaveCount(0);
      await expect(pane.getByRole("link", { name: /settings/i })).toHaveCount(0);
      await shot(page, "note-to-self-details", theme);
    });
  }
});
