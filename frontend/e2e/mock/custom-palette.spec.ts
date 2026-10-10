import type { Page } from "@playwright/test";
import { expect, openApp, PHONE_SMALL, test } from "./support.ts";

/** A custom property as set on <html>. */
const rootToken = (page: Page, name: string) =>
  page.evaluate((token) => document.documentElement.style.getPropertyValue(token), name);

test("an edited accent colour shows at once, saves to the account and survives reload", async ({
  page,
}) => {
  await openApp(page, "settings/appearance");
  const editor = page.getByRole("region", { name: "Custom palette" });
  const hex = editor.getByLabel("Accent hex value", { exact: true });
  const sample = editor.locator(".palette-preview-button");

  await hex.fill("#c2410c");
  // Live: on <html> before any save, so the whole app (and the preview card) repaints.
  await expect.poll(() => rootToken(page, "--accent-solid")).toBe("#c2410c");
  await expect(sample).toHaveCSS("background-color", "rgb(194, 65, 12)");
  await expect(editor.getByText("Unsaved changes")).toBeVisible();
  await expect(editor.getByText(/pairs? (is|are) below WCAG AA/)).toHaveCount(0);

  const saved = page.waitForResponse(
    (response) =>
      response.request().method() === "PATCH" && response.url().endsWith("/settings/appearance"),
  );

  await editor.getByRole("button", { name: "Save colours" }).click();
  const response = await saved;
  expect(response.ok()).toBe(true);
  expect(response.request().postDataJSON()).toMatchObject({
    appearancePreferences: { tokens: { "--accent-solid": "#c2410c" } },
  });
  await expect(editor.getByText("Unsaved changes")).toHaveCount(0);

  await page.reload();
  await expect.poll(() => rootToken(page, "--accent-solid")).toBe("#c2410c");
  await expect(
    page
      .getByRole("region", { name: "Custom palette" })
      .getByLabel("Accent hex value", { exact: true }),
  ).toHaveValue("#c2410c");

  // A colour too light for white labels is called out, and still saves.
  await page
    .getByRole("region", { name: "Custom palette" })
    .getByLabel("Accent hex value", { exact: true })
    .fill("#fde68a");
  await expect(page.getByText(/2 of 13 pairs are below WCAG AA/)).toBeVisible();
  await expect(
    page.getByRole("listitem").filter({ hasText: "Text on accent buttons" }),
  ).toContainText("Below 4.5:1");

  // Reset to the preset clears every custom colour.
  await page.getByRole("button", { name: "Reset to Smartfire" }).click();
  await expect.poll(() => rootToken(page, "--accent-solid")).toBe("");

  const cleared = page.waitForResponse(
    (response) =>
      response.request().method() === "PATCH" && response.url().endsWith("/settings/appearance"),
  );

  await page.getByRole("button", { name: "Save colours" }).click();
  expect((await cleared).request().postDataJSON()).toMatchObject({
    appearancePreferences: { tokens: null },
  });
  await page.reload();
  await expect(page.getByRole("region", { name: "Custom palette" })).toBeVisible();
  expect(await rootToken(page, "--accent-solid")).toBe("");
});

test("the palette editor stacks with finger-sized controls on a phone", async ({ page }) => {
  await page.setViewportSize(PHONE_SMALL);
  await openApp(page, "settings/appearance");
  const editor = page.getByRole("region", { name: "Custom palette" });
  const swatch = editor.getByLabel("Accent", { exact: true });

  await swatch.scrollIntoViewIfNeeded();
  const box = await swatch.boundingBox();
  expect(box?.width).toBeGreaterThanOrEqual(44);
  expect(box?.height).toBeGreaterThanOrEqual(44);

  const reset = await editor
    .getByRole("button", { name: "Reset Accent to the preset" })
    .boundingBox();

  expect(reset?.height).toBeGreaterThanOrEqual(44);

  const width = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(width).toBeLessThanOrEqual(PHONE_SMALL.width);
});

test("contrast reads the colours the page computes, workspace CSS included, in both themes", async ({
  page,
}) => {
  await openApp(page, "settings/profile");
  // Workspace custom CSS, unlayered after the stylesheets: a black messages pane.
  await page.addStyleTag({ content: ":root { --bg-pane: #000000; }" });
  await page.getByRole("link", { name: "Appearance" }).click();
  const editor = page.getByRole("region", { name: "Custom palette" });

  await expect(editor.getByLabel("Messages hex value", { exact: true })).toHaveValue("#000000");
  await expect(editor.getByLabel("Accent hex value", { exact: true })).toHaveValue("#3b5bd4");

  await editor.getByLabel("Primary text hex value", { exact: true }).fill("#000000");
  await expect(
    editor.getByRole("listitem").filter({ hasText: "Primary text on messages" }),
  ).toContainText("1.0:1");
  // Near-black text holds on the light sidebar but not on the dark one.
  await expect(
    editor.getByRole("listitem").filter({ hasText: "Primary text on sidebar" }),
  ).toContainText("Below 4.5:1 in dark");
});
