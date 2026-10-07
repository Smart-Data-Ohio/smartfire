import type { Page } from "@playwright/test";
import { expect, matrix, openApp, shot, test } from "./support.ts";

/** Saves the account's appearance through the API, as another device would. */
async function saveAccount(page: Page, change: Record<string, string>): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.patch("/api/v1/settings/appearance", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { theme: null, textSize: null, timeZone: null, ...change },
  });

  expect(response.ok()).toBe(true);
}

/** The computed font size of `selector`, in px. */
function fontSize(page: Page, selector: string): Promise<number> {
  return page
    .locator(selector)
    .first()
    .evaluate((element) => Number.parseFloat(getComputedStyle(element).fontSize));
}

const html = (page: Page) => page.locator("html");

test("the account's theme applies on start, over what this device last showed", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  // A device that last showed the light theme, before the account switched to dark elsewhere.
  await page.addInitScript(() => {
    if (localStorage.getItem("smartfire.appearance") === null) {
      localStorage.setItem("smartfire.appearance", JSON.stringify({ theme: "light" }));
    }
  });
  await openApp(page, "");
  await saveAccount(page, { theme: "dark" });

  await page.reload();
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await expect(html(page)).toHaveAttribute("data-theme", "dark");
});

test("a theme pinned on this device wins, until it's set back to the account's", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openApp(page, "settings/appearance");
  await saveAccount(page, { theme: "dark" });
  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", "dark");

  const device = page.getByLabel("Theme on this device");

  await device.selectOption({ label: "Light" });
  await expect(html(page)).toHaveAttribute("data-theme", "light");

  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", "light");
  await expect(page.getByRole("radio", { name: "Dark" })).toBeChecked();

  await page.getByLabel("Theme on this device").selectOption({ label: "Use my account's theme" });
  await expect(html(page)).toHaveAttribute("data-theme", "dark");
});

test("the five text sizes change the text, and the choice comes back on start", async ({
  page,
}) => {
  await openApp(page, "settings/appearance");

  const sizes: Record<string, number> = {};

  for (const label of ["Smaller", "Small", "Default", "Large", "Larger"]) {
    await page.getByRole("radio", { name: label, exact: true }).check();
    await expect(page.getByRole("radio", { name: label, exact: true })).toBeChecked();
    sizes[label] = await fontSize(page, ".settings-page h1, h1");
  }

  expect(sizes.Smaller).toBeLessThan(sizes.Small ?? 0);
  expect(sizes.Small).toBeLessThan(sizes.Default ?? 0);
  expect(sizes.Default).toBeLessThan(sizes.Large ?? 0);
  expect(sizes.Large).toBeLessThan(sizes.Larger ?? 0);

  // The account keeps "Larger": a fresh start shows it, messages included.
  await openApp(page, "");
  await expect(html(page)).toHaveAttribute("data-text-size", "larger");
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize)).toBe(
    "18px",
  );
});

test("the sidebar's theme button saves to the account when nothing is pinned", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openApp(page, "");

  const button = page.getByRole("button", { name: /^Theme: / });

  await button.click();
  await expect(html(page)).toHaveAttribute("data-theme", /light|dark/);

  const shown = await html(page).getAttribute("data-theme");

  await page.goto("/app/settings/appearance");
  await expect(
    page.getByRole("radio", { name: shown === "dark" ? "Dark" : "Light" }),
  ).toBeChecked();
});

matrix("appearance from the account", async ({ page, theme }) => {
  await openApp(page, "settings/appearance");
  await saveAccount(page, { theme, textSize: "large" });
  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", theme);
  await page.mouse.move(0, 0);
  await shot(page, "appearance-settings", theme);

  await page.goto("/app/");
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, "appearance-large-text", theme);
});
