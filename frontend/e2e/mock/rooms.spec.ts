import type { Page } from "@playwright/test";
import { DESKTOP, expect, matrix, openApp, ROOM_IDS, shot, type Theme, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

const rowName = (page: Page, name: string) =>
  page.locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) });

const sidebarRow = (page: Page, name: string) => sidebar(page).locator(rowName(page, name));

/** A conversation's row, by the name it shows. */
const rowFor = (page: Page, name: string) =>
  sidebar(page).locator(".sidebar-row", { has: rowName(page, name) });

/** Takes the shot once every finite animation (the dialog's entrance, a step's slide) is done. */
async function settledShot(page: Page, name: string, theme: Theme) {
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== "running" ||
          animation.effect?.getTiming().iterations === Number.POSITIVE_INFINITY,
      ),
  );
  await shot(page, name, theme);
}

test.describe("creating rooms", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the channels section's + makes a private channel with people and opens it", async ({
    page,
  }) => {
    await openApp(page, "/");
    await sidebar(page).getByRole("button", { name: "Create a channel", exact: true }).click();

    const dialog = page.getByRole("dialog", { name: "Create a channel" });

    await expect(dialog.getByRole("radio", { name: /Text channel/ })).toBeChecked();
    await expect(dialog.getByLabel("Name", { exact: true })).toBeFocused();
    await dialog.getByLabel("Name", { exact: true }).fill("design-crit");

    // An icon from the shared emoji picker.
    await dialog.getByRole("button", { name: "Choose an icon" }).click();
    await page.getByRole("combobox", { name: "Search emoji" }).fill("rocket");
    await page.keyboard.press("Enter");
    await expect(dialog.getByRole("button", { name: "Change icon (:rocket:)" })).toBeVisible();

    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await dialog.getByRole("button", { name: "Next" }).click();

    const people = page.getByRole("dialog", { name: "Add people" });

    await expect(people.getByRole("combobox")).toBeFocused();
    await people.getByRole("combobox").fill("Maya");
    await people.getByRole("option", { name: /Maya/ }).click();
    await people.getByRole("button", { name: "Create channel" }).click();

    await expect(people).toBeHidden();
    await expect(page).toHaveURL(/\/app\/r\/\d+$/);
    await expect(page.getByRole("heading", { level: 1, name: "design-crit" })).toBeVisible();
    await expect(sidebarRow(page, "design-crit")).toBeVisible();
    await expect(rowFor(page, "design-crit").locator(".room-glyph-emoji")).toHaveText("🚀");
  });

  test("a public channel is made in one step, and Back keeps what was typed", async ({ page }) => {
    await openApp(page, "/");
    await page.locator(".sidebar-workspace").first().click();
    await page.getByRole("menuitem", { name: "Create a channel…" }).click();

    // The title follows the kind, so this finds the dialog by role alone.
    const dialog = page.getByRole("dialog");

    await expect(dialog).toHaveAccessibleName("Create a channel");
    await dialog.getByRole("radio", { name: /Voice channel/ }).check();
    await expect(dialog).toHaveAccessibleName("Create a voice channel");
    await dialog.getByLabel("Name", { exact: true }).fill("Standup");
    await dialog.getByRole("button", { name: "Next" }).click();
    await page.getByRole("button", { name: "Back" }).click();
    await expect(dialog.getByLabel("Name", { exact: true })).toHaveValue("Standup");

    await dialog.getByRole("radio", { name: /Text channel/ }).check();
    await dialog.getByLabel("Name", { exact: true }).fill("watercooler");
    await dialog.getByLabel("Name", { exact: true }).press("Enter");

    await expect(dialog).toBeHidden();
    await expect(page.getByRole("heading", { level: 1, name: "watercooler" })).toBeVisible();
  });

  test("the classic new page and the app shortcut open the dialog on their kind", async ({
    page,
  }) => {
    await openApp(page, "rooms/new/voice");

    const dialog = page.getByRole("dialog", { name: "Create a voice channel" });

    await expect(dialog.getByRole("radio", { name: /Voice channel/ })).toBeChecked();
    // The dialog sits over the home screen; the new-room URL doesn't stay in history.
    await expect(page).not.toHaveURL(/rooms\/new/);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
  });
});

test.describe("room settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the header's name opens the settings; a rename saves in place", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}`);
    await page.getByRole("link", { name: /room settings$/ }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.launchPlanning}/settings$`));
    await expect(dialog.getByRole("button", { name: "Save changes" })).toBeDisabled();
    await dialog.getByLabel("Name", { exact: true }).fill("launch-war-room");
    await expect(dialog.getByText("Unsaved changes")).toBeVisible();
    await dialog.getByRole("button", { name: "Save changes" }).click();

    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.launchPlanning}$`));
    await expect(page.getByRole("heading", { level: 1, name: "launch-war-room" })).toBeVisible();
    await expect(sidebarRow(page, "launch-war-room")).toBeVisible();
  });

  test("members are added and removed in the draft, then saved together", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: /Members/ }).click();

    const before = await dialog.getByRole("tab", { name: /Members/ }).textContent();

    await dialog.getByRole("combobox", { name: "Add people" }).fill("Grace");
    await dialog.getByRole("option", { name: /Grace/ }).click();
    await expect(dialog.getByRole("listitem").filter({ hasText: "Grace" })).toContainText("New");
    await dialog.getByRole("button", { name: "Remove Grace" }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toHaveText(before ?? "");
    await expect(dialog.getByRole("button", { name: "Save changes" })).toBeDisabled();

    await dialog.getByRole("combobox", { name: "Add people" }).fill("Grace");
    await page.keyboard.press("Enter");
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await expect(dialog).toBeHidden();

    await page.goto(`/app/r/${ROOM_IDS.launchPlanning}/settings`);
    await page.getByRole("tab", { name: /Members/ }).click();
    await expect(
      page.getByRole("dialog", { name: "Channel settings" }).getByRole("listitem").filter({
        hasText: "Grace",
      }),
    ).toBeVisible();
  });

  test("a channel can go private, and Delete asks first", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.quiet}`);
    await rowFor(page, "quiet").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Channel settings" }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeVisible();
    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeHidden();

    await dialog.getByRole("button", { name: "Delete…" }).click();

    const confirm = page.getByRole("alertdialog", { name: "Delete #quiet?" });

    await confirm.getByRole("button", { name: "Keep it" }).click();
    await expect(confirm).toBeHidden();
    await dialog.getByRole("button", { name: "Delete…" }).click();
    await confirm.getByRole("button", { name: "Delete channel" }).click();

    await expect(dialog).toBeHidden();
    await expect(page).not.toHaveURL(new RegExp(`/r/${ROOM_IDS.quiet}`));
    await expect(sidebarRow(page, "quiet")).toBeHidden();
  });

  test("a direct message's settings URL opens the conversation", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.dmMaya}/settings`);
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.dmMaya}$`));
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });
});

matrix("the create dialog and the settings", async ({ page, theme }) => {
  await openApp(page, "rooms/new/closed", theme);

  const dialog = page.getByRole("dialog", { name: "Create a channel" });

  await dialog.getByLabel("Name", { exact: true }).fill("design-crit");
  await settledShot(page, "rooms-create", theme);
  await dialog.getByRole("button", { name: "Next" }).click();
  await page.getByRole("dialog", { name: "Add people" }).getByRole("combobox").fill("a");
  await settledShot(page, "rooms-create-people", theme);
  await page.keyboard.press("Escape");

  await page.goto(`/app/r/${ROOM_IDS.lounge}/settings`);

  const settings = page.getByRole("dialog", { name: "Voice channel settings" });

  await expect(settings.getByLabel("Name", { exact: true })).toHaveValue("Lounge");
  await settledShot(page, "rooms-settings", theme);
  await settings.getByRole("tab", { name: /Members/ }).click();
  await expect(settings.getByRole("listitem").first()).toBeVisible();
  await settledShot(page, "rooms-settings-members", theme);
});
