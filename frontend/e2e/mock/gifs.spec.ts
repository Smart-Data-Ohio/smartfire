import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * GIFs against the mock backend (two fixture GIFs, no key needed): the picker opens on trending
 * GIFs and categories, a search narrows it, a pick sends at once — with the draft as its text —
 * into a room and into a thread, where it shows as a file. With GIFs off, there is no button.
 */

const GENERAL = ROOM_IDS.general;

/** Opens the app at `path` (under /app/) with motion reduced, waiting for the conversation. */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

/** The conversation's messages, without the pending rows. */
function timeline(scope: Page | Locator) {
  return scope.locator("article.message:not([data-pending])");
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

/** Opens the GIF picker from a composer's GIF button and waits for its grid. */
async function openPicker(page: Page, composer: Locator) {
  await composer.getByRole("button", { name: "GIF" }).click();

  const picker = page.getByRole("dialog", { name: "Send a GIF" });

  await expect(picker.getByRole("listbox")).toBeVisible();

  return picker;
}

matrix(
  "trending shows, search narrows, and a pick sends with the draft",
  async ({ page, theme, phone }) => {
    await open(page, `r/${GENERAL}`, theme);

    const composer = page.locator(".composer");

    await composer.getByRole("textbox", { name: "Message #general" }).fill("for the launch");

    const picker = await openPicker(page, composer);

    // Trending to start: both fixture GIFs, their category tiles, and the KLIPY credit.
    await expect(picker.getByRole("option")).toHaveCount(2);
    await expect(picker.getByRole("option", { name: "Hello Wave" })).toBeVisible();
    await expect(picker.getByRole("button", { name: "Hello Wave" })).toBeVisible();
    await expect(picker.getByText("Powered by KLIPY")).toBeVisible();
    await shot(page, "gif-picker", theme);

    if (phone) await expectNoHorizontalOverflow(page);

    // A search narrows the grid to what it matches.
    await picker.getByRole("combobox", { name: "Search GIFs" }).fill("celebration");
    await expect(picker.getByRole("option")).toHaveCount(1);
    await picker.getByRole("combobox", { name: "Search GIFs" }).fill("");
    await expect(picker.getByRole("option")).toHaveCount(2);

    await picker.getByRole("option", { name: "Hello Wave" }).click();
    await expect(picker).toBeHidden();

    // The GIF lands as a file on a message carrying the draft, and the draft is spent.
    const sent = timeline(page).last();

    await expect(sent).toContainText("for the launch");
    await expect(sent.getByRole("img", { name: "Hello-Wave.gif" })).toBeVisible();
    await expect(composer.getByRole("textbox", { name: "Message #general" })).toHaveValue("");

    if (phone) await expectNoHorizontalOverflow(page);
  },
);

matrix("a GIF sends into a thread", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`, theme);

  const picker = await openPicker(page, pane(page).locator(".composer"));

  await picker.getByRole("option", { name: "Celebration Cheer" }).click();
  await expect(picker).toBeHidden();

  const sent = timeline(pane(page)).last();

  await expect(sent.getByRole("img", { name: "Celebration-Cheer.gif" })).toBeVisible();
  await shot(page, "gif-picker-thread", theme);
});

matrix("a GIF starts a new thread", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/t/new?parent=${MESSAGE_IDS.generalChart}`, theme);

  await expect(pane(page).getByRole("heading", { name: "New thread" })).toBeVisible();

  const picker = await openPicker(page, pane(page).locator(".composer"));

  await picker.getByRole("option", { name: "Hello Wave" }).click();
  await expect(picker).toBeHidden();

  // The pick is the first reply: the pane moves to the thread it started, GIF on show.
  await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
  await expect(
    timeline(pane(page)).last().getByRole("img", { name: "Hello-Wave.gif" }),
  ).toBeVisible();
  await shot(page, "gif-picker-new-thread", theme);
});

test("with GIFs off, the composer has no GIF button", async ({ page }) => {
  await page.route("**/api/v1/gifs/config", (route) => route.fulfill({ json: { enabled: false } }));

  const config = page.waitForResponse((response) => response.url().includes("/gifs/config"));

  await open(page, `r/${GENERAL}`);
  await config;

  const composer = page.locator(".composer");

  await expect(composer.getByRole("button", { name: "Emoji" })).toBeVisible();
  await expect(composer.getByRole("button", { name: "GIF" })).toHaveCount(0);
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the picker is a sheet, and a tap sends a GIF", async ({ page }) => {
    await open(page, `r/${GENERAL}`);

    const picker = await openPicker(page, page.locator(".composer"));

    await expect(picker).toHaveClass(/action-sheet/);
    await expectTouchTargets(page, ".popover.action-sheet");
    await expectNoHorizontalOverflow(page);
    await shot(page, "gif-picker-sheet", "light");

    await picker.getByRole("option", { name: "Hello Wave" }).tap();
    await expect(picker).toBeHidden();
    await expect(timeline(page).last().getByRole("img", { name: "Hello-Wave.gif" })).toBeVisible();
  });
});
