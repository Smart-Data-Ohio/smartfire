import type { Page } from "@playwright/test";
import { expect, matrix, ROOM_IDS, shot, type Theme, test } from "./support.ts";

/**
 * Opens the app at `path` (under /app/) with motion reduced; unlike `openApp` it waits for the
 * conversation, since a phone in a room or thread hides the sidebar.
 */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

const GENERAL = ROOM_IDS.general;

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

async function openPane(page: Page, name: RegExp): Promise<void> {
  await page.locator(".room-header").getByRole("button", { name }).first().click();
}

matrix("the members pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openPane(page, /^Members/);

  await expect(pane(page).getByRole("heading", { name: "Members" })).toBeVisible();
  await expect(pane(page).getByRole("heading", { name: /^Starred/ })).toBeVisible();
  await expect(pane(page).getByRole("heading", { name: /^Online/ })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "members", theme);

  await pane(page).getByRole("searchbox", { name: "Find a member" }).fill("maya");
  await expect(pane(page).getByRole("button", { name: /^Message Maya/ })).toBeVisible();
  await shot(page, "members-search", theme);
});

test("starring a member moves them to Starred", async ({ page }) => {
  await open(page, `r/${GENERAL}`);
  await openPane(page, /^Members/);

  const starred = pane(page).getByRole("region", { name: /^Starred/ });

  await pane(page)
    .getByRole("button", { name: /^Star Jonah/ })
    .click();
  await expect(starred.getByRole("button", { name: /^Message Jonah/ })).toBeVisible();
  await pane(page)
    .getByRole("button", { name: /^Unstar Jonah/ })
    .click();
  await expect(starred.getByRole("button", { name: /^Message Jonah/ })).toHaveCount(0);
});

test("choosing a member opens the direct message", async ({ page }) => {
  await open(page, `r/${GENERAL}`);
  await openPane(page, /^Members/);
  await pane(page)
    .getByRole("button", { name: /^Message Jonah/ })
    .click();

  await expect(page).not.toHaveURL(new RegExp(`/r/${GENERAL}$`));
  await expect(page.getByRole("textbox", { name: /Jonah/ })).toBeVisible();
});

matrix("the pins pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openPane(page, /^Pinned messages/);

  await expect(pane(page).getByRole("heading", { name: "Pinned messages" })).toBeVisible();

  const cards = pane(page).locator(".pin-card");

  await expect(cards).toHaveCount(3);
  await page.mouse.move(0, 0);
  await shot(page, "pins", theme);

  await cards.first().hover();
  await cards.first().getByRole("button", { name: "Unpin" }).click();
  await expect(cards).toHaveCount(2);
});

matrix("the files pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openPane(page, /^Files/);

  await expect(pane(page).getByRole("heading", { name: "Files" })).toBeVisible();
  await expect(
    pane(page).getByRole("list", { name: "Files" }).getByRole("listitem").first(),
  ).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "files", theme);

  await pane(page).getByRole("tab", { name: "Images" }).click();
  await expect(pane(page).locator(".file-grid .file-tile").first()).toBeVisible();
  await shot(page, "files-images", theme);

  await pane(page).getByRole("tab", { name: "All" }).click();
  await pane(page).getByRole("searchbox", { name: "Search files by name" }).fill("zzz-nothing");
  await expect(pane(page).getByText('No files match "zzz-nothing".')).toBeVisible();
});

matrix("the threads pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openPane(page, /^Threads/);

  await expect(pane(page).getByRole("heading", { name: "Threads" })).toBeVisible();
  await expect(pane(page).locator(".thread-row").first()).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "threads", theme);

  await pane(page).getByRole("tab", { name: "Following" }).click();
  await expect(pane(page).locator(".thread-row")).toHaveCount(1);

  await pane(page).getByRole("tab", { name: "Closed" }).click();
  await expect(pane(page).locator(".thread-row").first()).toBeVisible();
  await shot(page, "threads-closed", theme);
});

test("the header buttons show which pane is open", async ({ page }) => {
  await open(page, `r/${GENERAL}`);

  const files = page.locator(".room-header").getByRole("button", { name: "Files" });

  await files.click();
  await expect(files).toHaveAttribute("aria-pressed", "true");
  await files.click();
  await expect(files).toHaveAttribute("aria-pressed", "false");
  await expect(pane(page)).toHaveCount(0);
});
