import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * Menus and popovers on a touch phone open as bottom action sheets, as in Slack and Discord: the
 * full width over a scrim, 48 px rows, no keyboard hints, submenus that push in place, and the
 * message sheet's quick reactions. A desktop, and a phone-width window with a mouse, keep the
 * anchored dropdowns.
 */

const GENERAL = `r/${ROOM_IDS.general}`;

const TARGET = MESSAGE_IDS.generalReactions;

/** Opens `path` and waits for the message list (a phone shows no sidebar beside a room). */
async function openRoom(page: Page, path: string, theme: Theme = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("log", { name: "Messages" }).waitFor();
}

function messageBody(page: Page): Locator {
  return page.locator(`[data-message-id="${TARGET}"] .message-body`).first();
}

/**
 * Holds a finger on `target` for 600 ms. Synthetic touch PointerEvents, because a press held
 * through CDP's touch input never reaches the row's long-press timer in Chromium.
 */
async function longPress(target: Locator) {
  const box = await target.boundingBox();

  if (box === null) {
    throw new Error("the long-press target isn't on screen");
  }

  await target.evaluate(
    async (element, point) => {
      const init = {
        bubbles: true,
        cancelable: true,
        composed: true,
        pointerId: 11,
        pointerType: "touch",
        isPrimary: true,
        clientX: point.x,
        clientY: point.y,
        button: 0,
        buttons: 1,
      };

      element.dispatchEvent(new PointerEvent("pointerdown", init));
      await new Promise((resolve) => setTimeout(resolve, 600));
      element.dispatchEvent(new PointerEvent("pointerup", { ...init, buttons: 0 }));
    },
    { x: box.x + Math.min(24, box.width / 2), y: box.y + box.height / 2 },
  );
}

/** Asserts `sheet` is an action sheet: on the bottom edge, the viewport's full width. */
async function expectBottomSheet(page: Page, sheet: Locator) {
  await expect(sheet).toBeVisible();
  await expect(sheet).toHaveClass(/\baction-sheet\b/);

  const viewport = page.viewportSize() ?? PHONE_SMALL;

  await expect
    .poll(async () => {
      const box = await sheet.boundingBox();

      return box === null
        ? null
        : {
            left: Math.round(box.x),
            width: Math.round(box.width),
            bottom: Math.round(box.y + box.height),
          };
    })
    .toEqual({ left: 0, width: viewport.width, bottom: viewport.height });
}

/** Asserts every row of `menu` is at least 48 px tall and every tappable thing at least 44 px. */
async function expectSheetRows(page: Page, menu: Locator) {
  const heights = await menu
    .locator(".menu-item")
    .evaluateAll((rows) => rows.map((row) => row.getBoundingClientRect().height));

  expect(heights.length).toBeGreaterThan(0);

  for (const height of heights) {
    expect(height).toBeGreaterThanOrEqual(47.5);
  }

  await expectTouchTargets(page, `#${await menu.getAttribute("id")}`);
}

/** Asserts the menu has shortcut hints in its markup and that none of them shows. */
async function expectNoShortcutHints(menu: Locator) {
  const hints = menu.locator(".menu-shortcut");

  expect(await hints.count()).toBeGreaterThan(0);

  for (const hint of await hints.all()) {
    await expect(hint).toBeHidden();
  }
}

/** The sheet's scrim: a tap near the top of the screen, over the header, well above any sheet. */
async function tapScrim(page: Page) {
  await page.touchscreen.tap(PHONE_SMALL.width / 2, 20);
}

test.describe("on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  for (const theme of ["light", "dark"] as const) {
    test(`a long press opens the message sheet with quick reactions (${theme})`, async ({
      page,
    }) => {
      await openRoom(page, `${GENERAL}/m/${TARGET}`, theme);
      await expect(messageBody(page)).toBeVisible();
      await longPress(messageBody(page));

      const sheet = page.getByRole("menu", { name: "Message actions" });

      await expectBottomSheet(page, sheet);
      await expectSheetRows(page, sheet);
      await expectNoShortcutHints(sheet);
      await expectNoHorizontalOverflow(page);

      const quick = sheet.getByRole("group", { name: "Quick reactions" });

      await expect(quick.getByRole("menuitem")).toHaveCount(7);
      await expect(quick.getByRole("menuitem").first()).toHaveAccessibleName(
        "React with Thumbs up",
      );
      await expect(quick.getByRole("menuitem").last()).toHaveAccessibleName("More reactions");
      await shot(page, "phone-sheet-message", theme);

      if (theme === "dark") {
        return;
      }

      await quick.getByRole("menuitem", { name: "React with Thumbs up" }).click();
      await expect(sheet).toBeHidden();
      await expect(
        page
          .locator(`[data-message-id="${TARGET}"] .reactions`)
          .getByRole("button", { name: /thumbs up/i }),
      ).toHaveAttribute("aria-pressed", "true");
    });
  }

  test("“More reactions” opens the picker as a sheet, its search left for a tap", async ({
    page,
  }) => {
    await openRoom(page, `${GENERAL}/m/${TARGET}`);
    await longPress(messageBody(page));
    await page
      .getByRole("menu", { name: "Message actions" })
      .getByRole("menuitem", { name: "More reactions" })
      .click();

    const picker = page.getByRole("dialog", { name: "Add a reaction" });

    await expectBottomSheet(page, picker);
    await expect(picker.getByRole("option").first()).toBeVisible();
    await expect(picker.getByRole("combobox", { name: "Search emoji" })).not.toBeFocused();
  });

  test("a tap on the scrim only dismisses the sheet, as does Esc", async ({ page }) => {
    await openRoom(page, `${GENERAL}/m/${TARGET}`);

    const sheet = page.getByRole("menu", { name: "Message actions" });
    const url = page.url();

    await longPress(messageBody(page));
    await expectBottomSheet(page, sheet);
    await tapScrim(page);
    await expect(sheet).toBeHidden();
    // The header's tools under the scrim never saw the tap: no pane opened.
    await page.waitForTimeout(300);
    expect(page.url()).toBe(url);
    await expect(page.getByRole("complementary").filter({ visible: true })).toHaveCount(0);

    await longPress(messageBody(page));
    await expect(sheet).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(sheet).toBeHidden();
  });

  test("a press on the scrim that is cancelled leaves the next key press alone", async ({
    page,
  }) => {
    await openRoom(page, GENERAL);

    const trigger = page.getByRole("button", { name: "Attach and more" });
    const sheet = page.getByRole("menu", { name: "Attach and more" });

    await trigger.click();
    await expectBottomSheet(page, sheet);
    // A press that turns into a scroll: the browser cancels it, and no click follows.
    await page.evaluate(
      ({ x, y }) => {
        const target = document.elementFromPoint(x, y) ?? document.body;

        const init = {
          pointerId: 21,
          pointerType: "touch",
          isPrimary: true,
          clientX: x,
          clientY: y,
        };

        target.dispatchEvent(
          new PointerEvent("pointerdown", { ...init, bubbles: true, cancelable: true }),
        );
        target.dispatchEvent(new PointerEvent("pointercancel", { ...init, bubbles: true }));
      },
      { x: PHONE_SMALL.width / 2, y: 20 },
    );
    await expect(sheet).toBeHidden();
    await expect(trigger).toBeFocused();
    await page.keyboard.press("Enter");
    await expectBottomSheet(page, sheet);
  });

  test("an open dropdown turned upright becomes a sheet, its fallback placement dropped", async ({
    page,
  }) => {
    // Deny anchor positioning, as older engines do, so the measured fallback places the dropdown.
    await page.addInitScript(() => {
      const supports = CSS.supports.bind(CSS);

      CSS.supports = (...query: [string]) =>
        !/anchor|position-area/.test(query.join(" ")) && supports(...query);
    });
    await page.setViewportSize({ width: PHONE_SMALL.height, height: PHONE_SMALL.width });
    await openRoom(page, GENERAL);
    await page.getByRole("button", { name: "Attach and more" }).click();

    const sheet = page.getByRole("menu", { name: "Attach and more" });

    await expect(sheet).toHaveClass(/\bfloating\b/);
    expect(await sheet.evaluate((menu) => menu.style.position)).toBe("fixed");
    await page.setViewportSize(PHONE_SMALL);
    await expectBottomSheet(page, sheet);
    expect(
      await sheet.evaluate((menu) => [menu.style.position, menu.style.left, menu.style.top]),
    ).toEqual(["", "", ""]);
  });

  for (const theme of ["light", "dark"] as const) {
    test(`the + menu is a sheet without shortcut hints (${theme})`, async ({ page }) => {
      await openRoom(page, GENERAL, theme);
      await page.getByRole("button", { name: "Attach and more" }).click();

      const sheet = page.getByRole("menu", { name: "Attach and more" });

      await expectBottomSheet(page, sheet);
      await expect(sheet.getByRole("menuitem", { name: /Upload a file/ })).toBeVisible();
      await expectSheetRows(page, sheet);
      await expectNoShortcutHints(sheet);
      await shot(page, "phone-sheet-plus", theme);
    });
  }

  test("the schedule menu is a sheet that keeps the preset's time", async ({ page }) => {
    await openRoom(page, GENERAL);

    const input = page.locator(".composer-input").first();

    await input.fill("Standup notes are in the doc");
    await page.getByRole("button", { name: "Schedule message" }).click();

    const sheet = page.getByRole("menu", { name: "Schedule message" });

    await expectBottomSheet(page, sheet);
    await expectSheetRows(page, sheet);
    await expect(sheet.locator(".menu-item-detail")).toBeVisible();
    await expect(sheet.locator(".menu-shortcut")).toHaveCount(0);
    await shot(page, "phone-sheet-schedule", "light");
    await sheet.getByRole("menuitem", { name: /Tomorrow at/ }).click();
    await expect(sheet).toBeHidden();
    await expect(input).toHaveValue("");
  });

  test("a sidebar row's long press opens its sheet, and a submenu pushes in place", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/app/");

    const row = page.locator(".sidebar-row", { hasText: "general" }).first();

    await expect(row).toBeVisible();
    await longPress(row);

    const sheet = page.getByRole("menu", { name: /options$/ });

    await expectBottomSheet(page, sheet);
    await expectSheetRows(page, sheet);
    await sheet.getByRole("menuitem", { name: "Notifications" }).click();

    const submenu = page.getByRole("menu", { name: "Notifications" });

    await expectBottomSheet(page, submenu);
    await expect(submenu.getByRole("menuitem", { name: "Back" })).toBeVisible();
    await expect(submenu.getByRole("menuitemradio").first()).toBeVisible();
    await expectSheetRows(page, submenu);
    // The parent steps out of the way while its submenu covers it.
    await expect(sheet).toHaveCSS("opacity", "0");
    await shot(page, "phone-sheet-submenu", "light");

    await submenu.getByRole("menuitem", { name: "Back" }).click();
    await expect(submenu).toBeHidden();
    await expect(sheet).toHaveCSS("opacity", "1");
    await tapScrim(page);
    await expect(sheet).toBeHidden();
  });

  for (const theme of ["light", "dark"] as const) {
    test(`the composer's emoji picker is a sheet of finger-sized cells (${theme})`, async ({
      page,
    }) => {
      await openRoom(page, GENERAL, theme);
      await page.getByRole("button", { name: "Emoji", exact: true }).click();

      const picker = page.getByRole("dialog", { name: "Insert emoji" });

      await expectBottomSheet(page, picker);

      const cell = picker.getByRole("option").first();

      await expect(cell).toBeVisible();
      await expect(picker.getByRole("combobox", { name: "Search emoji" })).not.toBeFocused();
      await expect(picker.locator(".emoji-picker-preview")).toHaveCount(0);

      const box = await cell.boundingBox();

      expect(box?.width ?? 0).toBeGreaterThanOrEqual(43.5);
      expect(box?.height ?? 0).toBeGreaterThanOrEqual(43.5);
      await expectNoHorizontalOverflow(page);
      await shot(page, "phone-sheet-emoji", theme);

      if (theme === "dark") {
        return;
      }

      await cell.click();
      await expect(picker).toBeHidden();
      await expect(page.locator(".composer-input").first()).not.toHaveValue("");
    });
  }
});

test.describe("with a mouse", () => {
  test("a desktop's message menu stays an anchored dropdown with its shortcut hints", async ({
    page,
  }) => {
    await page.setViewportSize(DESKTOP);
    await openRoom(page, `${GENERAL}/m/${TARGET}`);
    await messageBody(page).click({ button: "right" });

    const menu = page.getByRole("menu", { name: "Message actions" });

    await expect(menu).toHaveClass(/\bfloating\b/);
    await expect(menu).not.toHaveClass(/\baction-sheet\b/);
    await expect(menu.getByRole("group", { name: "Quick reactions" })).toHaveCount(0);
    await expect(menu.locator(".menu-shortcut").first()).toBeVisible();
    expect((await menu.boundingBox())?.width ?? 0).toBeLessThan(400);
  });

  test("a phone-width window without touch keeps the dropdowns", async ({ page }) => {
    await page.setViewportSize(PHONE_SMALL);
    await openRoom(page, GENERAL);
    await page.getByRole("button", { name: "Attach and more" }).click();

    const menu = page.getByRole("menu", { name: "Attach and more" });

    await expect(menu).toBeVisible();
    await expect(menu).not.toHaveClass(/\baction-sheet\b/);
    await expect(menu.locator(".menu-shortcut").first()).toBeVisible();
  });
});
