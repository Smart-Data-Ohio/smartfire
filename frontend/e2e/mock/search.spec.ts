import type { Page } from "@playwright/test";
import { SEARCH_SEED } from "../../mock/s3/search.ts";
import { expect, matrix, openApp, ROOM_IDS, shot, type Theme, test } from "./support.ts";

/** Opens the app at `path` (under /app/) with motion reduced, waiting for the main column. */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

function field(page: Page) {
  return page.getByRole("combobox", { name: "Search messages" }).locator("visible=true");
}

function suggestions(page: Page) {
  return page.getByRole("listbox", { name: "Suggestions" });
}

function hits(page: Page) {
  return page.locator("[data-search-hit]");
}

async function search(page: Page, query: string, theme: Theme): Promise<void> {
  await open(page, `search?q=${encodeURIComponent(query)}`, theme);
  await expect(
    page
      .getByRole("heading", { name: /^Messages/ })
      .or(page.getByRole("heading", { name: /^No results/ })),
  ).toBeVisible();
}

matrix("the search page before a search", async ({ page, theme }) => {
  await open(page, "search", theme);
  await expect(
    page.getByRole("heading", { name: "Search messages, files and conversations" }),
  ).toBeVisible();

  const recents = page.getByRole("region", { name: "Recent searches" });

  for (const query of SEARCH_SEED.recents) {
    await expect(recents.getByRole("button", { name: query })).toBeVisible();
  }

  await expect(field(page)).toBeFocused();
  await page.mouse.move(0, 0);
  await shot(page, "search-home", theme);
});

matrix(
  "the field suggests people, channels and filters as you type",
  async ({ page, theme, phone }) => {
    if (phone) {
      await openApp(page, "", theme);
      await page.getByRole("button", { name: "Search messages" }).click();
      await expect(page).toHaveURL(/\/app\/search$/);
      await expect(field(page)).toBeFocused();
    } else {
      await open(page, `r/${ROOM_IDS.general}`, theme);
      await field(page).click();
    }

    await expect(
      suggestions(page)
        .getByRole("group", { name: "Recent searches" })
        .or(page.getByRole("region", { name: "Recent searches" })),
    ).toBeVisible();
    await field(page).fill("ma");
    await expect(suggestions(page).getByRole("group", { name: "People" })).toBeVisible();
    await expect(suggestions(page).getByRole("option", { name: /Maya Okafor/ })).toBeVisible();
    await shot(page, "search-typeahead", theme);

    await field(page).fill("from:");
    await expect(suggestions(page).getByRole("option", { name: /from:@maya/ })).toBeVisible();
    await shot(page, "search-typeahead-operator", theme);

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(field(page)).toHaveValue(/^from:@\w+ $/);
  },
);

matrix("results show their filters as chips", async ({ page, theme }) => {
  await search(page, "onboarding in:#general has:link", theme);

  const filters = page.getByRole("toolbar", { name: "Filters" });

  await expect(filters.getByText("in: general")).toBeVisible();
  await expect(hits(page).first()).toBeVisible();
  await expect(hits(page).first().locator("mark.search-mark").first()).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "search-results-chips", theme);

  await filters.getByRole("button", { name: "Remove in: general" }).click();
  await expect(page).toHaveURL(/q=onboarding(\+|%20)has(%3A|:)link$/);
  await expect(filters.getByText("in: general")).toHaveCount(0);
});

matrix("sections, people and channels above the messages", async ({ page, theme }) => {
  await search(page, "onboarding", theme);
  await expect(page.getByRole("region", { name: /Board posts/ })).toBeVisible();
  await expect(page.getByRole("region", { name: /Events/ })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "search-sections", theme);
});

matrix("file results", async ({ page, theme }) => {
  await search(page, "has:file", theme);
  await expect(hits(page).first()).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "search-files", theme);
});

matrix("a search with no results says what to try", async ({ page, theme }) => {
  await search(page, "zebracorn has:pin", theme);
  await expect(
    page.getByRole("heading", { name: "No results for “zebracorn has:pin”" }),
  ).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "search-empty", theme);

  await page.getByRole("button", { name: "has: pin" }).last().click();
  await expect(page).toHaveURL(/q=zebracorn$/);
});

matrix("a result opens its message", async ({ page, theme, phone }) => {
  await search(page, "launch checklist in:#launch-planning", theme);
  await page.locator("[data-search-hit-link]").first().click();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.launchPlanning}/m/\\d+$`));

  if (phone) {
    await expect(page.getByRole("main")).toBeVisible();
  }
});

test("older results load as the list scrolls", async ({ page }) => {
  await search(page, "the", "light");
  await expect(hits(page)).toHaveCount(40);
  await page.locator(".search-more").scrollIntoViewIfNeeded();
  await expect.poll(async () => hits(page).count()).toBeGreaterThan(40);
});

test("the shortcut focuses the field, and / does too outside a text field", async ({ page }) => {
  await open(page, `r/${ROOM_IDS.general}`);
  await page.locator("body").press("ControlOrMeta+Shift+F");
  await expect(field(page)).toBeFocused();
  await page.keyboard.press("Escape");
  await page.locator(":focus").blur();
  await page.keyboard.press("/");
  await expect(field(page)).toBeFocused();
});

test("arrow keys move through the results", async ({ page }) => {
  await search(page, "onboarding", "light");
  await field(page).focus();
  // Esc closes the suggestions; ↓ then leaves the field for the results.
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowDown");

  const first = page.locator("[data-search-nav], [data-search-hit-link]").first();

  await expect(first).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(first).not.toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(first).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(field(page)).toBeFocused();
});
