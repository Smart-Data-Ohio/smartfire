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

/** A custom property as the page resolves it on <html>. */
function rootToken(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (token) => getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
    name,
  );
}

test("a colour palette and a font apply at once, and stay on this device", async ({ page }) => {
  await openApp(page, "settings/appearance");

  const before = await rootToken(page, "--accent");
  const palettes = page.getByRole("radiogroup", { name: "Colour palette" });

  await palettes.getByRole("radio", { name: "Ember" }).check();
  await expect(html(page)).toHaveAttribute("data-palette", "ember");
  expect(await rootToken(page, "--accent")).not.toBe(before);

  const fonts = page.getByRole("radiogroup", { name: "Font" });

  await fonts.getByRole("radio", { name: "Atkinson Hyperlegible" }).check();
  await expect(html(page)).toHaveAttribute("data-font", "atkinson");
  await expect
    .poll(() => page.evaluate(() => getComputedStyle(document.body).fontFamily))
    .toMatch(/^"Atkinson Hyperlegible Next"/);

  await page.reload();
  await expect(html(page)).toHaveAttribute("data-palette", "ember");
  await expect(html(page)).toHaveAttribute("data-font", "atkinson");
  await expect(
    page.getByRole("radiogroup", { name: "Colour palette" }).getByRole("radio", { name: "Ember" }),
  ).toBeChecked();

  await page
    .getByRole("radiogroup", { name: "Colour palette" })
    .getByRole("radio", { name: "Smartfire" })
    .check();
  await expect(html(page)).not.toHaveAttribute("data-palette", /./);
  expect(await rootToken(page, "--accent")).toBe(before);
});

matrix("palettes and fonts", async ({ page, theme }) => {
  await openApp(page, "settings/appearance");
  await page.getByRole("radio", { name: theme === "dark" ? "Dark" : "Light", exact: true }).check();
  await page.getByRole("radiogroup", { name: "Colour palette" }).getByRole("radio", { name: "Ocean" }).check();
  await page.getByRole("radiogroup", { name: "Font" }).getByRole("radio", { name: "Atkinson Hyperlegible" }).check();
  await page.mouse.move(0, 0);
  await shot(page, "appearance-presets", theme);

  await page.getByRole("radiogroup", { name: "Colour palette" }).getByRole("radio", { name: "Ember" }).check();
  await page.getByRole("radiogroup", { name: "Font" }).getByRole("radio", { name: "Source Serif" }).check();
  await page.goto("/app/");
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, "appearance-ember-serif", theme);
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
