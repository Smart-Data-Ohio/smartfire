import type { Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openHeaderTool,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

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

matrix("the members pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Members/);

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
  await openHeaderTool(page, /^Members/);

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
  await openHeaderTool(page, /^Members/);
  await pane(page)
    .getByRole("button", { name: /^Message Jonah/ })
    .click();

  await expect(page).not.toHaveURL(new RegExp(`/r/${GENERAL}$`));
  await expect(page.getByRole("textbox", { name: /Jonah/ })).toBeVisible();
});

matrix("the pins pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Pinned messages/);

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
  await openHeaderTool(page, /^Files/);

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
  await openHeaderTool(page, /^Threads/);

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

test.describe("pane headers on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  for (const [name, heading] of [
    [/^Members/, "Members"],
    [/^Pinned messages/, "Pinned messages"],
    [/^Files/, "Files"],
    [/^Threads/, "Threads"],
  ] as const) {
    test(`the ${heading} pane is a pushed page with a touch-size back button`, async ({ page }) => {
      await open(page, `r/${GENERAL}`);
      await openHeaderTool(page, name);

      const header = pane(page).locator(".pane-header");
      const back = header.getByRole("button", { name: "Back to #general" });

      await expect(header.getByRole("heading", { name: heading, exact: true })).toBeVisible();
      await expect(back).toBeVisible();
      // No close button: Back is the way out of a pushed page.
      await expect(header.getByRole("button", { name: "Close" })).toHaveCount(0);
      await expectTouchTargets(page, "aside.right-pane .pane-header");
      await expectNoHorizontalOverflow(page);

      // The back chevron leads at the screen's edge, the title right after it.
      const backBox = await back.boundingBox();
      const titleBox = await header.locator(".pane-title").boundingBox();

      expect(backBox?.x).toBeLessThanOrEqual(8);
      expect((titleBox?.x ?? 0) - ((backBox?.x ?? 0) + (backBox?.width ?? 0))).toBeLessThanOrEqual(
        12,
      );

      await back.click();
      await expect(pane(page)).toHaveCount(0);
      await expect(page.locator(".room-header .room-title-name")).toBeInViewport();
    });
  }

  for (const [name, heading, rows] of [
    [/^Threads/, "Threads", ".thread-list > li"],
    [/^Files/, "Files", ".file-row"],
  ] as const) {
    test(`the ${heading} pane's rows fit its body without scrolling sideways`, async ({ page }) => {
      await open(page, `r/${GENERAL}`);
      await openHeaderTool(page, name);
      await expect(pane(page).locator(rows).first()).toBeVisible();

      const body = pane(page).locator(".pane-body");

      const { scrollWidth, clientWidth } = await body.evaluate((element) => ({
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
      }));

      expect(scrollWidth, "the pane body's content width").toBeLessThanOrEqual(clientWidth);
      await expectNoHorizontalOverflow(page);
    });
  }
});
