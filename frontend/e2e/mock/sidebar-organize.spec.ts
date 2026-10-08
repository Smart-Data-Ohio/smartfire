import type { Locator, Page } from "@playwright/test";
import { expect, matrix, openApp, ROOM_IDS, shot, type Theme, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

/** A section of the sidebar by its key ("favorites", "category-1", "channels", …). */
const section = (page: Page, key: string) => page.locator(`[data-drop-section="${key}"]`);

/** A conversation's row link, by the name it shows. */
function row(scope: Page | Locator, name: string): Locator {
  const page = "page" in scope ? scope.page() : scope;

  return scope
    .locator(".sidebar-row")
    .filter({ has: page.locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) }) });
}

/** The names listed in a section's own list, top to bottom. */
const names = (page: Page, key: string) =>
  section(page, key).locator('[data-list="main"] .sidebar-row-name').allTextContents();

/** A category's heading toggle. */
const heading = (page: Page, key: string) => section(page, key).locator(".sidebar-section-trigger");

/** Takes the shot once every finite animation (a menu's entrance, a hover fade) has played out. */
async function settledShot(page: Page, name: string, theme: Theme) {
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== "running" ||
          animation.effect?.getTiming().iterations === Number.POSITIVE_INFINITY,
      ),
  );
  await shot(page, name, theme);
}

/** Drags `from` to the middle of `to` with a real pointer, in steps (so the drag starts). */
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

matrix("sidebar organisation", async ({ page, theme, phone }) => {
  await openApp(page, phone ? "" : `r/${ROOM_IDS.general}`, theme);

  const list = sidebar(page);

  await expect(heading(page, "category-1")).toHaveText("Launch");
  await expect(heading(page, "category-2")).toHaveText("Team");
  expect(await names(page, "category-1")).toEqual(["design", "launch-planning"]);
  expect(await names(page, "category-2")).toEqual(["announcements"]);
  await settledShot(page, "categories", theme);

  // Rename, in place of the heading.
  await section(page, "category-2").hover();
  await list.getByRole("button", { name: "Team options" }).click();
  await page.getByRole("menuitem", { name: "Rename" }).click();

  const field = list.getByRole("textbox", { name: "Rename Team" });

  await expect(field).toBeFocused();
  await field.fill("People & ops");
  await settledShot(page, "rename", theme);
  await field.press("Enter");
  await expect(heading(page, "category-2")).toHaveText("People & ops");
  // The field goes; focus lands on the heading it stood in for.
  await expect(heading(page, "category-2")).toBeFocused();

  // A row's menu, with Move to open.
  await row(list, "quiet").click({ button: "right" });

  const menu = page.getByRole("menu", { name: "quiet options" });

  await expect(menu).toBeVisible();
  await menu.getByRole("menuitem", { name: "Move to" }).click();
  await expect(page.getByRole("menu", { name: "Move to" })).toBeVisible();
  await settledShot(page, "context-menu", theme);
  await page.getByRole("menuitemradio", { name: "Launch" }).click();
  await expect
    .poll(() => names(page, "category-1"))
    .toEqual(["design", "launch-planning", "quiet"]);
  // Focus follows the row to its new section rather than dropping to the page.
  await expect(row(section(page, "category-1"), "quiet")).toBeFocused();

  // A key-opened menu hands focus back to its row when it closes.
  await page.keyboard.press("Shift+F10");
  await expect(page.getByRole("menu", { name: "quiet options" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(row(section(page, "category-1"), "quiet")).toBeFocused();

  // Muted: dimmed, with the bell.
  await row(list, "design").click({ button: "right" });
  await page
    .getByRole("menu", { name: "design options" })
    .getByRole("menuitem", { name: "Mute" })
    .click();
  await expect(row(list, "design")).toHaveAttribute(
    "data-state",
    phone ? "muted" : /muted|selected/,
  );
  await page.mouse.move(0, 0);
  await settledShot(page, "muted", theme);

  // A drag in progress (shot before the drop), then dropped.
  if (!phone) {
    await drag(page, row(list, "general"), section(page, "category-2"), { drop: false });
    await expect(section(page, "category-2")).toHaveAttribute("data-drop-active", "true");
    await settledShot(page, "drag", theme);
    await page.mouse.up();
    await expect.poll(() => names(page, "category-2")).toEqual(["announcements", "general"]);
  } else {
    await drag(page, row(list, "random"), section(page, "category-2"), { drop: false });
    await settledShot(page, "drag", theme);
    await page.keyboard.press("Escape");
    await page.mouse.up();
    expect(await names(page, "category-2")).toEqual(["announcements"]);
  }
});

test("categories are created, folded, reordered and deleted, and it all survives a reload", async ({
  page,
}) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const list = sidebar(page);

  // New category from the Channels heading; Escape hands focus back to the button.
  const newCategory = list.getByRole("button", { name: "New category" });
  const field = list.getByRole("textbox", { name: "New category name" });

  await section(page, "channels").hover();
  await newCategory.click();
  await field.press("Escape");
  await expect(field).toHaveCount(0);
  await expect(newCategory).toBeFocused();

  await newCategory.click();
  await field.press("Enter");
  await expect(list.getByText("Give the category a name")).toBeVisible();
  await field.fill("Ops");
  await field.press("Enter");
  await expect(heading(page, "category-3")).toHaveText("Ops");
  await expect(section(page, "category-3")).toContainText("Drag channels here");
  // Focus follows the new category from its draft to the one the server made.
  await expect(heading(page, "category-3")).toBeFocused();

  // Folding is remembered by the server.
  await heading(page, "category-1").click();
  await expect(section(page, "category-1")).toHaveAttribute("data-open", "false");

  // Drag Ops above Launch by its heading.
  await drag(page, heading(page, "category-3"), heading(page, "category-1"));
  await expect
    .poll(() =>
      list
        .locator('[data-drop-section^="category-"]')
        .evaluateAll((all) => all.map((el) => el.getAttribute("data-drop-section"))),
    )
    .toEqual(["category-3", "category-1", "category-2"]);

  await page.reload();
  await expect(heading(page, "category-3")).toHaveText("Ops");
  await expect(section(page, "category-1")).toHaveAttribute("data-open", "false");
  expect(
    await list
      .locator('[data-drop-section^="category-"]')
      .evaluateAll((all) => all.map((element) => element.getAttribute("data-drop-section"))),
  ).toEqual(["category-3", "category-1", "category-2"]);

  // Move up from the menu: the moved category's heading keeps focus.
  await section(page, "category-2").hover();
  await list.getByRole("button", { name: "Team options" }).click();
  await page.getByRole("menuitem", { name: "Move up" }).click();
  await expect
    .poll(() =>
      list
        .locator('[data-drop-section^="category-"]')
        .evaluateAll((all) => all.map((element) => element.getAttribute("data-drop-section"))),
    )
    .toEqual(["category-3", "category-2", "category-1"]);
  await expect(heading(page, "category-2")).toBeFocused();

  await section(page, "category-3").hover();
  await list.getByRole("button", { name: "Ops options" }).click();
  await page.getByRole("menuitem", { name: "Move down" }).click();
  await expect
    .poll(() =>
      list
        .locator('[data-drop-section^="category-"]')
        .evaluateAll((all) => all.map((element) => element.getAttribute("data-drop-section"))),
    )
    .toEqual(["category-2", "category-3", "category-1"]);
  await expect(heading(page, "category-3")).toBeFocused();

  // Delete asks first; its channels go back to Channels.
  await section(page, "category-1").hover();
  await list.getByRole("button", { name: "Launch options" }).click();
  await page.getByRole("menuitem", { name: "Delete category" }).click();

  const confirm = page.getByRole("alertdialog", { name: "Delete Launch?" });

  await expect(confirm).toContainText("Its 2 channels go back to Channels");
  await confirm.getByRole("button", { name: "Delete category" }).click();
  await expect(section(page, "category-1")).toHaveCount(0);
  await expect(heading(page, "channels")).toBeFocused();
  expect(await names(page, "channels")).toEqual(
    expect.arrayContaining(["design", "launch-planning"]),
  );
});

test("favourites: star from the menu, reorder by drag, drop a channel back", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const list = sidebar(page);

  expect(await names(page, "favorites")).toEqual(["engineering", "Maya Okafor"]);

  await row(list, "random").click({ button: "right" });
  await page.getByRole("menuitem", { name: "Add to Favourites" }).click();
  await expect
    .poll(() => names(page, "favorites"))
    .toEqual(["engineering", "Maya Okafor", "random"]);

  // To the top: drop on the first favourite's upper half.
  const first = row(section(page, "favorites"), "engineering");
  const box = await first.boundingBox();
  const start = await row(section(page, "favorites"), "random").boundingBox();

  if (box === null || start === null) {
    throw new Error("No favourites");
  }

  await page.mouse.move(start.x + 24, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x + 24, start.y + 4, { steps: 3 });
  await page.mouse.move(box.x + 40, box.y + 3, { steps: 8 });
  await expect(section(page, "favorites").locator('[data-drop-edge="before"]')).toHaveCount(1);
  await page.mouse.up();
  await expect
    .poll(() => names(page, "favorites"))
    .toEqual(["random", "engineering", "Maya Okafor"]);

  // A favourite channel dropped on Channels leaves the favourites.
  await drag(page, row(section(page, "favorites"), "engineering"), section(page, "channels"));
  await expect.poll(() => names(page, "favorites")).toEqual(["random", "Maya Okafor"]);
  expect(await names(page, "channels")).toContain("engineering");

  // A direct message can't go in a category: those sections step back while it is dragged.
  await drag(page, row(section(page, "favorites"), "Maya Okafor"), section(page, "category-2"), {
    drop: false,
  });
  await expect(section(page, "category-2")).toHaveAttribute("data-drop-disabled");
  await page.keyboard.press("Escape");
  await page.mouse.up();
  expect(await names(page, "favorites")).toEqual(["random", "Maya Okafor"]);

  // Without a drag (touch has none): Move up from the row's menu.
  await row(section(page, "favorites"), "Maya Okafor").click({ button: "right" });

  const menu = page.getByRole("menu", { name: "Maya Okafor options" });

  await expect(menu.getByRole("menuitem", { name: "Move down" })).toHaveAttribute(
    "aria-disabled",
    "true",
  );
  await menu.getByRole("menuitem", { name: "Move up" }).click();
  await expect.poll(() => names(page, "favorites")).toEqual(["Maya Okafor", "random"]);

  // A drag's click doesn't navigate.
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
});

test("the drag's floating copy only fades in when motion is reduced", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "reduce";
  });

  const list = sidebar(page);

  await drag(page, row(list, "random"), section(page, "category-2"), { drop: false });

  const ghost = page.locator(".sidebar-drag-ghost");

  await expect(ghost).toHaveCSS("rotate", "none");
  await expect(ghost).toHaveCSS("animation-name", "sidebar-ghost-fade");
  await page.keyboard.press("Escape");
  await page.mouse.up();
});

test("a keyboard drag: Space picks up, arrows move, Enter drops", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const list = sidebar(page);
  const quiet = row(list, "quiet");

  await quiet.focus();
  await page.keyboard.press("Space");
  await expect(page.locator('[aria-live="assertive"]')).toContainText("Picked up quiet");

  // Up from Channels: Team, Launch, then the favourites' gaps.
  await page.keyboard.press("ArrowUp");
  await expect(page.locator('[aria-live="assertive"]')).toHaveText("Team");
  await expect(section(page, "category-2")).toHaveAttribute("data-drop-active", "true");
  await page.keyboard.press("Enter");

  await expect.poll(() => names(page, "category-2")).toEqual(["announcements", "quiet"]);
  await expect(row(section(page, "category-2"), "quiet")).toBeFocused();

  // Escape puts it back.
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("Escape");
  await expect(page.locator('[aria-live="assertive"]')).toContainText("Cancelled");
  expect(await names(page, "category-2")).toEqual(["announcements", "quiet"]);
});

test("notifications from the room header: mute dims the row, hide takes it out with an Undo", async ({
  page,
}) => {
  await openApp(page, `r/${ROOM_IDS.quiet}`);

  const list = sidebar(page);
  const bell = page.getByRole("button", { name: /^Notifications: / });

  await expect(bell).toHaveAccessibleName("Notifications: Mentions");
  await bell.click();
  await page.getByRole("menuitemradio", { name: /Muted/ }).click();
  await expect(bell).toHaveAccessibleName("Notifications: Muted");
  await expect(page.getByRole("menu", { name: "Notifications", includeHidden: true })).toHaveCount(
    0,
  );

  await bell.click();
  await page.getByRole("menuitemradio", { name: /Hidden/ }).click();
  await expect(row(list, "quiet")).toHaveCount(0);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(row(list, "quiet")).toHaveCount(1);
  await expect(bell).toHaveAccessibleName("Notifications: Muted");
});
