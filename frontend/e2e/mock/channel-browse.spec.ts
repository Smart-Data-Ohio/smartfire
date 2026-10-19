import type { APIRequestContext, Page } from "@playwright/test";
import { JOINABLE_OPEN_ROOM } from "../../mock/seed.ts";
import { DESKTOP, expect, matrix, openApp, shot, test } from "./support.ts";

/** A promise the test resolves by hand, to hold a response until a chosen moment. */
function gate() {
  let open = () => {};

  const opened = new Promise<void>((resolve) => {
    open = resolve;
  });

  return { open, opened };
}

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

const sidebarRoom = (page: Page, name: string) =>
  sidebar(page).locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) });

async function seedChannels(
  request: APIRequestContext,
  data: { readonly count: number; readonly prefix: string; readonly hidden: boolean },
) {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/open-channels", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });
}

test("the switcher finds a channel you aren't in and choosing it joins it into the sidebar", async ({
  page,
}) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, "/");
  await expect(sidebarRoom(page, "general")).toBeVisible();
  await expect(sidebarRoom(page, JOINABLE_OPEN_ROOM.name)).toBeHidden();

  await page.keyboard.press("ControlOrMeta+k");

  const switcher = page.getByRole("dialog", { name: "Jump to a conversation" });

  await switcher.getByRole("combobox").fill("camp");

  const joinable = switcher.getByRole("group", { name: "Channels you can join" });

  await expect(joinable.getByRole("option", { name: /campfire/ })).toBeVisible();
  await expect(joinable.getByText("3 members")).toBeVisible();
  await expect(joinable.getByText("Join", { exact: true })).toBeVisible();

  await switcher.getByRole("combobox").fill("general");
  await expect(switcher.getByRole("group", { name: "Channels" })).toBeVisible();
  await expect(switcher.getByRole("group", { name: "Channels you can join" })).toBeHidden();

  await switcher.getByRole("combobox").fill("campfire");
  await expect(joinable.getByRole("option", { name: /campfire/ })).toBeVisible();
  await shot(page, "switcher-joinable", "light");
  await page.keyboard.press("Enter");

  await expect(switcher).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`/r/${JOINABLE_OPEN_ROOM.id}$`));
  await expect(page.getByRole("region", { name: "Conversation" })).toBeVisible();
  await expect(sidebarRoom(page, JOINABLE_OPEN_ROOM.name)).toBeVisible();

  await page.keyboard.press("ControlOrMeta+k");
  await switcher.getByRole("combobox").fill("campfire");
  await expect(switcher.getByRole("group", { name: "Channels" })).toContainText("campfire");
  await expect(switcher.getByRole("group", { name: "Channels you can join" })).toBeHidden();
});

test("the switcher's footer opens the channel browser", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, "/");
  await page.keyboard.press("ControlOrMeta+k");
  await page
    .getByRole("dialog", { name: "Jump to a conversation" })
    .getByRole("button", { name: "Browse channels" })
    .click();

  await expect(page.getByRole("dialog", { name: "Browse channels" })).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Jump to a conversation" })).toBeHidden();
});

matrix(
  "the channel browser searches, joins with Joined at once, and never lists a closed room",
  async ({ page, theme }) => {
    await openApp(page, "/", theme);
    await sidebar(page).getByRole("button", { name: "Add channels" }).click();
    await page.getByRole("menuitem", { name: "Browse channels" }).click();

    const browser = page.getByRole("dialog", { name: "Browse channels" });
    const list = browser.getByRole("list", { name: "Channels" });

    const row = (name: string) =>
      list.getByRole("listitem").filter({ has: page.getByText(name, { exact: true }) });

    await expect(browser.getByRole("searchbox", { name: "Search channels" })).toBeFocused();
    await expect(row("general")).toContainText("Joined");
    await expect(row(JOINABLE_OPEN_ROOM.name)).toContainText("3 members");
    await expect(list).not.toContainText("launch-planning");
    await expect(list).not.toContainText("Lounge");
    await shot(page, "browse-channels", theme);

    await browser.getByRole("searchbox", { name: "Search channels" }).fill("launch");
    await expect(browser.getByText("No channels match “launch”")).toBeVisible();
    await expect(list.getByRole("listitem")).toHaveCount(0);

    await browser.getByRole("searchbox", { name: "Search channels" }).fill("CAMP");
    await expect(list.getByRole("listitem")).toHaveCount(1);
    await expect(sidebarRoom(page, JOINABLE_OPEN_ROOM.name)).toHaveCount(0);

    await browser.getByRole("button", { name: `Join #${JOINABLE_OPEN_ROOM.name}` }).click();
    await expect(row(JOINABLE_OPEN_ROOM.name)).toContainText("Joined");
    await expect(
      browser.getByRole("button", { name: `Join #${JOINABLE_OPEN_ROOM.name}` }),
    ).toBeHidden();
    await shot(page, "browse-channels-joined", theme);

    await browser.getByRole("button", { name: "Close" }).click();
    await expect(sidebarRoom(page, JOINABLE_OPEN_ROOM.name)).toHaveCount(1);
  },
);

test("the switcher offers a channel you can join behind a full page of joined matches", async ({
  page,
  request,
}) => {
  await seedChannels(request, { count: 55, prefix: "camp", hidden: false });
  await page.setViewportSize(DESKTOP);
  await openApp(page, "/");
  await expect(sidebarRoom(page, "camp-55")).toBeAttached();
  await page.keyboard.press("ControlOrMeta+k");

  const switcher = page.getByRole("dialog", { name: "Jump to a conversation" });

  await switcher.getByRole("combobox").fill("camp");
  await expect(
    switcher
      .getByRole("group", { name: "Channels you can join" })
      .getByRole("option", { name: /campfire/ }),
  ).toBeVisible();
  await shot(page, "switcher-joinable-behind-joined", "light");
});

test("the channel browser loads the next page after it is closed and opened again", async ({
  page,
  request,
}) => {
  await seedChannels(request, { count: 60, prefix: "lot", hidden: true });
  await page.setViewportSize(DESKTOP);
  await openApp(page, "/");

  const browser = page.getByRole("dialog", { name: "Browse channels" });
  const rows = browser.getByRole("list", { name: "Channels" }).getByRole("listitem");

  const openBrowser = async () => {
    await sidebar(page).getByRole("button", { name: "Add channels" }).click();
    await page.getByRole("menuitem", { name: "Browse channels" }).click();
    await expect(rows).toHaveCount(50);
  };

  await openBrowser();
  await browser.getByRole("button", { name: "Close" }).click();
  await expect(browser).toBeHidden();
  await openBrowser();
  await rows.last().scrollIntoViewIfNeeded();
  await expect(rows.filter({ has: page.getByText("lot-1", { exact: true }) })).toHaveCount(1);
  await expect(rows).not.toHaveCount(50);
  await shot(page, "browse-channels-reopened-page-two", "light");
});

test("the channel browser loads page two when its first page lands after you reach the end", async ({
  page,
  request,
}) => {
  await seedChannels(request, { count: 60, prefix: "lot", hidden: true });
  await page.setViewportSize(DESKTOP);

  const pageTwoAsked = gate();
  const pageTwo = gate();
  const refresh = gate();
  let firstPages = 0;

  await page.route("**/api/v1/rooms/browse?**", async (route) => {
    if (new URL(route.request().url()).searchParams.get("cursor")) {
      pageTwoAsked.open();
      await pageTwo.opened;
    } else if (++firstPages === 2) {
      await refresh.opened;
    }

    await route.continue();
  });

  await openApp(page, "/");

  const browser = page.getByRole("dialog", { name: "Browse channels" });
  const rows = browser.getByRole("list", { name: "Channels" }).getByRole("listitem");

  const openBrowser = async () => {
    await sidebar(page).getByRole("button", { name: "Add channels" }).click();
    await page.getByRole("menuitem", { name: "Browse channels" }).click();
    await expect(rows).toHaveCount(50);
  };

  await openBrowser();
  await rows.last().scrollIntoViewIfNeeded();
  await pageTwoAsked.opened;
  await browser.getByRole("button", { name: "Close" }).click();
  await expect(browser).toBeHidden();
  await openBrowser();
  await rows.last().scrollIntoViewIfNeeded();
  // The sentinel's observer reports on a frame, and it must report while the refresh is held.
  await page.evaluate(
    () => new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled))),
  );
  pageTwo.open();
  await expect.poll(() => firstPages).toBe(2);
  refresh.open();
  await expect(rows.filter({ has: page.getByText("lot-1", { exact: true }) })).toHaveCount(1);
  await expect(rows).not.toHaveCount(50);
});
