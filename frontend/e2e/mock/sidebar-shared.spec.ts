import type { APIRequestContext, Locator, Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  longPress,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
} from "./support.ts";

/**
 * The shared workspace layout (C3, C14): the categories and channel order administrators set for
 * everyone, with each person's favourites and own categories drawn over it.
 */

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

/** A section of the sidebar by its key ("shared-1", "category-2", "channels", …). */
const section = (page: Page, key: string) => page.locator(`[data-drop-section="${key}"]`);

const heading = (page: Page, key: string) => section(page, key).locator(".sidebar-section-trigger");

function row(scope: Page | Locator, name: string): Locator {
  const page = "page" in scope ? scope.page() : scope;

  return scope
    .locator(".sidebar-row")
    .filter({ has: page.locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) }) });
}

const names = (page: Page, key: string) =>
  section(page, key).locator('[data-list="main"] .sidebar-row-name').allTextContents();

/** The shared sections' keys, top to bottom. */
const sharedOrder = (page: Page) =>
  sidebar(page)
    .locator('[data-drop-section^="shared-"]')
    .evaluateAll((all) => all.map((element) => element.getAttribute("data-drop-section")));

const announcer = (page: Page) => page.locator('[aria-live="assertive"]');

/** `/__mock/workspace-layout`'s body: category names in order, rooms by category index. */
interface SharedLayoutControl {
  readonly categories: readonly string[];
  readonly rooms: readonly { readonly roomId: number; readonly category: number | null }[];
}

async function control(
  request: APIRequestContext,
  action: "viewer-role" | "workspace-layout",
  data: SharedLayoutControl | { readonly role: "member" },
) {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok()).toBe(true);
}

/** Product holding #general and #quiet, and an empty Later, as another administrator left them. */
const PRODUCT_LAYOUT: SharedLayoutControl = {
  categories: ["Product", "Later"],
  rooms: [
    { roomId: ROOM_IDS.general, category: 0 },
    { roomId: ROOM_IDS.quiet, category: 0 },
  ],
};

async function drag(page: Page, from: Locator, to: Locator, { drop = true } = {}) {
  const start = await from.boundingBox();
  const end = await to.boundingBox();

  if (start === null || end === null) {
    throw new Error("Nothing to drag");
  }

  await page.mouse.move(start.x + 24, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x + 30, start.y + start.height / 2 + 6, { steps: 3 });
  await page.mouse.move(end.x + 40, end.y + end.height / 2, { steps: 8 });

  if (drop) {
    await page.mouse.up();
  }
}

/** Steps a keyboard drag (up, or down) until the announcer names `label`. */
async function arrowUpTo(page: Page, label: string, key = "ArrowUp") {
  for (let step = 0; step < 40; step += 1) {
    await page.keyboard.press(key);

    if ((await announcer(page).textContent()) === label) {
      return;
    }
  }

  throw new Error(`No drop target named ${label}`);
}

async function newSharedCategory(page: Page, name: string) {
  await page.locator(".sidebar-workspace").click();
  await page.getByRole("menuitem", { name: "New shared category" }).click();

  const field = sidebar(page).getByRole("textbox", { name: "New shared category name" });

  await expect(field).toBeFocused();
  await field.fill(name);
  await field.press("Enter");
}

test("an administrator arranges the shared layout by keyboard, drag and menu", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const list = sidebar(page);

  await newSharedCategory(page, "Product");
  await expect(heading(page, "shared-1")).toHaveText("Product");
  await expect(heading(page, "shared-1")).toBeFocused();
  // An empty shared category stays in view for administrators, to drop into.
  await expect(section(page, "shared-1")).toContainText("Drag channels here");
  await newSharedCategory(page, "Ops");
  await expect(heading(page, "shared-2")).toHaveText("Ops");
  expect(await sharedOrder(page)).toEqual(["shared-1", "shared-2"]);

  // Keyboard: Space picks up, the arrows name each place, for everyone where it is shared.
  await row(list, "quiet").focus();
  await page.keyboard.press("Space");
  await arrowUpTo(page, "Top of Product, for everyone");
  await expect(section(page, "shared-1")).toHaveAttribute("data-drop-active", "true");
  await page.keyboard.press("Enter");
  await expect.poll(() => names(page, "shared-1")).toEqual(["quiet"]);
  await expect(row(section(page, "shared-1"), "quiet")).toBeFocused();

  // Pointer: the floating copy says the drop changes the sidebar for everyone.
  await drag(page, row(list, "general"), section(page, "shared-2"), { drop: false });
  await expect(page.locator(".sidebar-drag-ghost-shared")).toHaveText("Everyone");
  await page.mouse.up();
  await expect.poll(() => names(page, "shared-2")).toEqual(["general"]);

  // The room menu moves for everyone without a drag.
  await row(list, "random").click({ button: "right" });
  await page
    .getByRole("menu", { name: "random options" })
    .getByRole("menuitem", { name: "Move for everyone" })
    .click();

  const everyone = page.getByRole("menu", { name: "Move for everyone" });

  await everyone.getByRole("menuitemradio", { name: "Product" }).click();
  await expect.poll(() => names(page, "shared-1")).toEqual(["quiet", "random"]);
  // The menus play out their exit first; a right click on the way out would land on them.
  await expect(page.getByRole("menu")).toHaveCount(0);

  await row(list, "random").click({ button: "right" });
  await page
    .getByRole("menu", { name: "random options" })
    .getByRole("menuitem", { name: "Move for everyone" })
    .click();
  await expect(everyone.getByRole("menuitemradio", { name: "Product" })).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await everyone.getByRole("menuitem", { name: "Move up" }).click();
  await expect.poll(() => names(page, "shared-1")).toEqual(["random", "quiet"]);

  // Category order, from the heading's menu: focus stays on the moved heading.
  await section(page, "shared-2").hover();
  await list.getByRole("button", { name: "Ops options" }).click();
  await expect(page.getByRole("group", { name: "Changes for everyone" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Move up" }).click();
  await expect.poll(() => sharedOrder(page)).toEqual(["shared-2", "shared-1"]);
  await expect(heading(page, "shared-2")).toBeFocused();

  // The server has it all.
  await page.reload();
  await expect(heading(page, "shared-2")).toHaveText("Ops");
  expect(await sharedOrder(page)).toEqual(["shared-2", "shared-1"]);
  expect(await names(page, "shared-1")).toEqual(["random", "quiet"]);

  await section(page, "shared-2").hover();
  await list.getByRole("button", { name: "Ops options" }).click();
  await page.getByRole("menuitem", { name: "Rename" }).click();

  const rename = list.getByRole("textbox", { name: "Rename Ops" });

  await rename.fill("Operations");
  await rename.press("Enter");
  await expect(heading(page, "shared-2")).toHaveText("Operations");

  await section(page, "shared-1").hover();
  await list.getByRole("button", { name: "Product options" }).click();
  await page.getByRole("menuitem", { name: "Delete shared category" }).click();

  const confirm = page.getByRole("alertdialog", { name: "Delete Product for everyone?" });

  await expect(confirm).toContainText("Its 2 channels go back to Channels for everyone");
  await confirm.getByRole("button", { name: "Delete shared category" }).click();
  await expect(section(page, "shared-1")).toHaveCount(0);
  expect(await names(page, "channels")).toEqual(expect.arrayContaining(["random", "quiet"]));
});

test("focus stays on a starred room an administrator drops for everyone", async ({ page }) => {
  await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
  await openApp(page, `r/${ROOM_IDS.general}`);

  // A slow server: the room leaves Favourites only once the shared move has landed.
  let release = () => {};

  const answered = new Promise<void>((resolve) => {
    release = resolve;
  });

  await page.route("**/rooms/*/workspace_category", async (route) => {
    await answered;
    await route.continue();
  });

  await row(section(page, "favorites"), "engineering").focus();
  await page.keyboard.press("Space");
  await arrowUpTo(page, "Product, after general, for everyone", "ArrowDown");
  await page.keyboard.press("Enter");
  await expect(row(section(page, "favorites"), "engineering")).toBeFocused();

  release();
  await expect.poll(() => names(page, "shared-1")).toEqual(["general", "engineering", "quiet"]);
  expect(await names(page, "favorites")).toEqual(["Maya Okafor"]);
  await expect(row(section(page, "shared-1"), "engineering")).toBeFocused();
});

test("focus goes to the heading of a folded category a starred room is dropped into", async ({
  page,
}) => {
  await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
  await openApp(page, `r/${ROOM_IDS.general}`);

  await heading(page, "shared-1").click();
  await expect(heading(page, "shared-1")).toHaveAttribute("aria-expanded", "false");

  await row(section(page, "favorites"), "engineering").focus();
  await page.keyboard.press("Space");
  await arrowUpTo(page, "Product, after general, for everyone", "ArrowDown");
  await page.keyboard.press("Enter");

  await expect.poll(() => names(page, "favorites")).toEqual(["Maya Okafor"]);
  await expect(heading(page, "shared-1")).toBeFocused();
});

test("focus goes to the heading of a folded category a channel is dropped into", async ({
  page,
}) => {
  await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
  await openApp(page, `r/${ROOM_IDS.general}`);

  // A channel that is read and not selected shows nowhere in a folded category.
  await heading(page, "shared-1").click();
  await expect(heading(page, "shared-1")).toHaveAttribute("aria-expanded", "false");

  await row(section(page, "channels"), "product-updates").focus();
  await page.keyboard.press("Space");
  await arrowUpTo(page, "Product, after quiet, for everyone");
  await page.keyboard.press("Enter");

  await expect(
    section(page, "channels").locator("[data-drag-handle]", { hasText: /^product-updates$/ }),
  ).toHaveCount(0);
  await expect(heading(page, "shared-1")).toBeFocused();
});

test("a shared category's heading is dragged into place", async ({ page }) => {
  await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
  await openApp(page, `r/${ROOM_IDS.general}`);

  await drag(page, heading(page, "shared-2"), heading(page, "shared-1"));
  await expect.poll(() => sharedOrder(page)).toEqual(["shared-2", "shared-1"]);
});

test("anyone else sees the shared layout live, and their drags stay their own", async ({
  page,
}) => {
  await control(page.request, "viewer-role", { role: "member" });
  await openApp(page, `r/${ROOM_IDS.general}`);

  const list = sidebar(page);

  await expect(section(page, "shared-1")).toHaveCount(0);

  // Another administrator's change arrives over sync.
  await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
  await expect(heading(page, "shared-1")).toHaveText("Product");
  expect(await names(page, "shared-1")).toEqual(["general", "quiet"]);
  // An empty shared category is only an administrator's.
  await expect(section(page, "shared-2")).toHaveCount(0);

  // No way to change it for everyone.
  await section(page, "shared-1").hover();
  await expect(list.getByRole("button", { name: "Product options" })).toHaveCount(0);
  await expect(list.getByRole("button", { name: "Move Product" })).toHaveCount(0);
  await page.locator(".sidebar-workspace").click();
  await expect(page.getByRole("menuitem", { name: "New message" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "New shared category" })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await row(list, "quiet").click({ button: "right" });
  await expect(page.getByRole("menuitem", { name: "Move for everyone" })).toHaveCount(0);
  await page.keyboard.press("Escape");

  // A drag into one of their own categories is theirs alone, and dropping back on Product
  // returns the room to the shared layout.
  await drag(page, row(list, "general"), section(page, "category-2"));
  await expect.poll(() => names(page, "category-2")).toEqual(["announcements", "general"]);
  expect(await names(page, "shared-1")).toEqual(["quiet"]);

  await drag(page, row(section(page, "category-2"), "general"), section(page, "shared-1"));
  await expect.poll(() => names(page, "shared-1")).toEqual(["general", "quiet"]);
  expect(await names(page, "category-2")).toEqual(["announcements"]);

  // A room whose shared home is elsewhere can't be dropped on Product.
  // Out from under the account bar at the foot of the list first.
  await row(list, "random").evaluate((element) => element.scrollIntoView({ block: "center" }));
  await drag(page, row(list, "random"), section(page, "shared-1"), { drop: false });
  await expect(section(page, "shared-1")).toHaveAttribute("data-drop-disabled");
  await page.keyboard.press("Escape");
  await page.mouse.up();
  expect(await names(page, "shared-1")).toEqual(["general", "quiet"]);

  // With both its rooms in Team, Product is hidden; a drag of either brings it back to drop on.
  await drag(page, row(section(page, "shared-1"), "general"), section(page, "category-2"));
  await drag(page, row(section(page, "shared-1"), "quiet"), section(page, "category-2"));
  await expect.poll(() => names(page, "category-2")).toEqual(["announcements", "general", "quiet"]);
  await expect(section(page, "shared-1")).toHaveCount(0);

  const start = await row(section(page, "category-2"), "general").boundingBox();

  if (start === null) {
    throw new Error("No #general in Team");
  }

  await page.mouse.move(start.x + 24, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x + 30, start.y + start.height / 2 + 6, { steps: 3 });
  await expect(section(page, "shared-1")).toBeVisible();

  const home = await section(page, "shared-1").boundingBox();

  if (home === null) {
    throw new Error("Product didn't show for the drag");
  }

  await page.mouse.move(home.x + 40, home.y + home.height / 2, { steps: 8 });
  await expect(section(page, "shared-1")).not.toHaveAttribute("data-drop-disabled");
  await page.mouse.up();
  await expect.poll(() => names(page, "shared-1")).toEqual(["general"]);

  // And for a keyboard drag: Product is a stop below Team.
  await drag(page, row(section(page, "shared-1"), "general"), section(page, "category-2"));
  await expect(section(page, "shared-1")).toHaveCount(0);
  await row(section(page, "category-2"), "quiet").focus();
  await page.keyboard.press("Space");
  await arrowUpTo(page, "Product", "ArrowDown");
  await page.keyboard.press("Enter");
  await expect.poll(() => names(page, "shared-1")).toEqual(["quiet"]);
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("an administrator moves rooms and categories from their sheets", async ({ page }) => {
    await control(page.request, "workspace-layout", PRODUCT_LAYOUT);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/app/");

    await expect(heading(page, "shared-1")).toHaveText("Product");
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, '[data-drop-section="shared-1"]');

    await longPress(row(section(page, "shared-1"), "quiet"));
    await page
      .getByRole("menu", { name: "quiet options" })
      .getByRole("menuitem", { name: "Move for everyone" })
      .click();

    const everyone = page.getByRole("menu", { name: "Move for everyone" });

    await expect(everyone.getByRole("menuitem", { name: "Back" })).toBeVisible();
    await expectTouchTargets(page, '[role="menu"][aria-label="Move for everyone"]');
    await everyone.getByRole("menuitemradio", { name: "Later" }).click();
    await expect.poll(() => names(page, "shared-2")).toEqual(["quiet"]);

    await section(page, "shared-2").locator(".sidebar-section-trigger").scrollIntoViewIfNeeded();
    await sidebar(page).getByRole("button", { name: "Later options" }).click();
    await page.getByRole("menuitem", { name: "Move up" }).click();
    await expect.poll(() => sharedOrder(page)).toEqual(["shared-2", "shared-1"]);
    await expectNoHorizontalOverflow(page);
  });
});
