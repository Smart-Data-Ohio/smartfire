import type { Page } from "@playwright/test";
import { expect, matrix, openApp, shot, test } from "./support.ts";

/** Opens a settings section (`""` for the profile) with motion reduced. */
async function openSettings(page: Page, section: string, theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/settings${section === "" ? "" : `/${section}`}`);
  await page.locator(".settings-page h1").waitFor();
}

function nav(page: Page) {
  return page.getByRole("navigation", { name: "Settings sections" });
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

matrix("the settings sections", async ({ page, theme }) => {
  await openSettings(page, "", theme);

  await expect(page.getByRole("heading", { level: 1, name: "Profile" })).toBeVisible();
  await expect(nav(page).getByRole("link", { name: "Profile" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await settle(page);
  await shot(page, "settings-profile", theme);

  for (const [link, heading, name] of [
    ["Status", "Status", "settings-status"],
    ["Notifications", "Notifications", "settings-notifications"],
    ["Appearance", "Appearance", "settings-appearance"],
    ["Sessions", "Sessions", "settings-sessions"],
    ["Push devices", "Push devices", "settings-devices"],
    ["Integrations", "Integrations", "settings-integrations"],
  ] as const) {
    await nav(page).getByRole("link", { name: link }).click();
    await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
    await settle(page);
    await shot(page, name, theme);
  }
});

test("the user menu opens settings in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "Profile and settings" }).click();

  await expect(page).toHaveURL(/\/app\/settings$/);
  await expect(page.getByRole("heading", { level: 1, name: "Profile" })).toBeVisible();
  // The shell stayed: no full page load.
  await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
});

test("an email change asks for the current password", async ({ page }) => {
  await openSettings(page, "");

  await page.getByRole("textbox", { name: "Email address" }).fill("riel@new.example");

  const current = page.getByLabel("Current password");

  await expect(current).toBeVisible();
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(
    page.getByText("Current password is required to change your email address."),
  ).toBeVisible();

  await current.fill("secret123456");
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(page.getByText("Profile saved")).toBeVisible();
  await expect(current).toBeHidden();
});

test("a notification switch saves as it flips", async ({ page }) => {
  await openSettings(page, "notifications");

  const dnd = page.getByRole("switch", { name: /^Do not disturb Silence/ });

  await expect(dnd).toHaveAttribute("aria-checked", "false");
  await dnd.click();
  await expect(dnd).toHaveAttribute("aria-checked", "true");

  await page.reload();
  await expect(page.getByRole("switch", { name: /^Do not disturb Silence/ })).toHaveAttribute(
    "aria-checked",
    "true",
  );
});

test("someone can be let through DND", async ({ page }) => {
  await openSettings(page, "notifications");

  const exceptions = page.locator(".settings-exceptions");

  await exceptions.getByRole("button", { name: "Add someone" }).click();
  await exceptions.getByRole("searchbox", { name: "Add someone" }).fill("Jonah");
  await exceptions.getByRole("button", { name: /Jonah/ }).click();

  await expect(
    exceptions.getByRole("button", { name: /^Remove Jonah .* from DND exceptions$/ }),
  ).toBeVisible();
});

test("a theme choice shows at once", async ({ page }) => {
  await openSettings(page, "appearance");

  await page.getByRole("radio", { name: "Dark" }).check();

  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.getByRole("radio", { name: "Dark" })).toBeChecked();
});

test("signing out every other session", async ({ page }) => {
  await openSettings(page, "sessions");

  const rows = page.locator(".settings-session");

  await expect(rows).toHaveCount(3);
  await page.getByRole("button", { name: "Sign out of all other sessions" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Sign out" }).click();

  await expect(page.getByText("Signed out 2 other sessions.")).toBeVisible();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("This device");
});

test("an out-of-office end in the past is refused in place", async ({ page }) => {
  await openSettings(page, "status");

  await page.getByLabel("Out until").selectOption("custom");
  await page.getByLabel("Custom end").fill("2020-01-01T09:00");
  await page.getByRole("button", { name: "Save out of office" }).click();

  await expect(page.getByText("Out of office needs a future date and time.")).toBeVisible();
});

test("integrations link to the classic page, which stays classic", async ({ page }) => {
  await openSettings(page, "integrations");

  await expect(page.getByRole("link", { name: "Connect Fizzy" })).toHaveAttribute(
    "href",
    "/users/me/profile?classic=1#fizzy-connection-title",
  );
});
