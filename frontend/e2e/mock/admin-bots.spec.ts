import type { Page } from "@playwright/test";
import { expect, matrix, SHOTS, shot, test } from "./support.ts";

/** Opens a bot page under `/app/admin/bots` with motion reduced. */
async function openBots(page: Page, path = "", theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/admin/bots${path}`);
  await page.locator(".settings-page h1").waitFor();
}

/** Lets entrance animations finish so a shot is settled. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

// Screenshots only: the other tests open each of these pages.
if (SHOTS) {
  matrix("the bot pages", async ({ page, theme }) => {
    await openBots(page, "", theme);

    await expect(page.getByRole("heading", { level: 1, name: "Chat bots" })).toBeVisible();
    await settle(page);
    await shot(page, "admin-bots", theme);

    await page.getByRole("link", { name: "Edit Ember" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Ember" })).toBeVisible();
    await settle(page);
    await shot(page, "admin-bot", theme);

    await page.getByRole("link", { name: "Grants" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Ember's grants" })).toBeVisible();
    await settle(page);
    await shot(page, "admin-bot-grants", theme);
  });
}

test("a new bot shows its key once, then opens its page", async ({ page }) => {
  await openBots(page, "/new");

  await page.getByRole("textbox", { name: "Name" }).fill("Robo");
  await page.getByRole("button", { name: "Create bot" }).click();

  const dialog = page.getByRole("alertdialog", { name: "Robo's key" });

  await expect(dialog.getByText(/^\d+-[0-9a-f]+$/)).toBeVisible();
  await expect(dialog.getByText(/\/rooms\/ROOM_ID\//)).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();

  await expect(page).toHaveURL(/\/app\/admin\/bots\/\d+$/);
  await expect(page.getByRole("heading", { level: 1, name: "Robo" })).toBeVisible();
});

test("a bot's edit saves, and a bad budget names its field", async ({ page }) => {
  await openBots(page);
  await page.getByRole("link", { name: "Edit Ember" }).click();

  await page.getByRole("textbox", { name: "Messages a day" }).fill("lots");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("must be a whole number greater than 0")).toBeVisible();

  await page.getByRole("textbox", { name: "Messages a day" }).fill("25");
  await page.getByRole("textbox", { name: "Name" }).fill("Ember Two");
  await page.getByRole("button", { name: "Save", exact: true }).click();

  await expect(page.getByText("Bot saved")).toBeVisible();
  await expect(page.getByRole("heading", { level: 1, name: "Ember Two" })).toBeVisible();
  await expect(page.getByText(/0\/25 messages/)).toBeVisible();
});

test("the kill switch asks first and leaves the agent suspended", async ({ page }) => {
  await openBots(page);
  await page.getByRole("link", { name: "Edit Ember" }).click();

  await page.getByRole("button", { name: "Kill switch" }).click();

  const dialog = page.getByRole("alertdialog");

  await expect(dialog.getByText(/cancel its pending approvals/)).toBeVisible();
  await dialog.getByRole("button", { name: "Suspend" }).click();

  await expect(page.getByText("This agent is suspended.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Kill switch" })).toHaveCount(0);
});

test("a credential's secret shows once, and revoking keeps the focus in the list", async ({
  page,
}) => {
  await openBots(page);
  await page.getByRole("link", { name: "Edit Ember" }).click();
  await page.getByRole("link", { name: "Credentials" }).click();

  await expect(page.getByText("No credentials yet.")).toBeVisible();
  await page.getByRole("textbox", { name: "Name" }).fill("ci");
  await page.getByRole("button", { name: "Issue credential" }).click();

  const dialog = page.getByRole("alertdialog", { name: "Copy this secret now" });

  await expect(dialog.getByText(/^cfa_/)).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();

  await page.getByRole("button", { name: "Revoke ci" }).click();
  await expect(page.getByText("ci was revoked")).toBeVisible();
  await expect(page.locator("[data-row]", { hasText: "Revoked" })).toBeFocused();
});

test("granting a capability lists it, and a grant revokes", async ({ page }) => {
  await openBots(page);
  await page.getByRole("link", { name: "Edit Ember" }).click();
  await page.getByRole("link", { name: "Grants" }).click();

  await expect(page.getByText(/Legacy access/)).toBeVisible();
  await page.getByLabel("Capability").selectOption("react");
  await page.getByRole("button", { name: "Grant capability" }).click();

  await expect(page.getByText(/Legacy access/)).toHaveCount(0);
  await page.getByRole("button", { name: /Revoke react/ }).click();
  await expect(page.locator("[data-row]", { hasText: "Revoked" })).toBeFocused();
});

test("without a clipboard, a secret is selected for copying by hand", async ({ page }) => {
  // An origin that isn't secure has no `navigator.clipboard` at all.
  await page.addInitScript(() => {
    Object.defineProperty(Navigator.prototype, "clipboard", { get: () => undefined });
  });
  await openBots(page);
  await page.getByRole("link", { name: "Edit Ember" }).click();
  await page.getByRole("link", { name: "Credentials" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("ci");
  await page.getByRole("button", { name: "Issue credential" }).click();

  const dialog = page.getByRole("alertdialog", { name: "Copy this secret now" });
  const secret = await dialog.getByText(/^cfa_/).textContent();

  await dialog.getByRole("button", { name: /^Copy/ }).click();

  await expect(page.getByText("Couldn't copy — select it manually")).toBeVisible();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(secret);
});
