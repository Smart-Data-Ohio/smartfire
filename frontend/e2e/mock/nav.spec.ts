import type { Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openApp,
  openHeaderTool,
  PHONE_TOUCH,
  ROOM_IDS,
  SHOTS,
  shot,
  test,
} from "./support.ts";

/** The modifier the app reads on this platform (the CI browsers aren't Apple). */
const MOD = process.platform === "darwin" ? "Meta" : "Control";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

async function openSwitcher(page: Page, phone: boolean) {
  if (phone) {
    await sidebar(page)
      .getByRole("button", { name: /Jump to/ })
      .click();
  } else {
    await page.keyboard.press(`${MOD}+k`);
  }

  const dialog = page.getByRole("dialog", { name: "Jump to a conversation" });

  await expect(dialog).toBeVisible();

  return dialog;
}

async function openNewDirect(page: Page, phone: boolean) {
  if (phone) {
    await sidebar(page).getByRole("button", { name: "New message" }).first().click();
  } else {
    await page.keyboard.press(`${MOD}+Shift+k`);
  }

  const dialog = page.getByRole("dialog", { name: "New message" });

  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("option").first()).toBeVisible();

  return dialog;
}

/** The unread pill (the row's ::before) against the box that would clip it. */
async function unreadPill(page: Page) {
  return page
    .locator('.sidebar-row[data-state="unread"]')
    .first()
    .evaluate((row) => {
      const style = getComputedStyle(row, "::before");
      const rect = row.getBoundingClientRect();
      const left = rect.left + Number.parseFloat(style.left);
      let clip = row.parentElement;

      while (clip !== null && getComputedStyle(clip).overflowX === "visible") {
        clip = clip.parentElement;
      }

      const bounds = (clip ?? document.body).getBoundingClientRect();

      return {
        left,
        width: Number.parseFloat(style.width),
        height: Number.parseFloat(style.height),
        radius: style.borderRadius,
        opacity: Number(style.opacity),
        clipLeft: bounds.left,
        rowLeft: rect.left,
      };
    });
}

test("the sidebar's unread pill is whole, inside the list's gutter", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.engineering}`);

  const pill = await unreadPill(page);

  expect(pill.opacity).toBe(1);
  expect(pill.width).toBe(4);
  expect(pill.height).toBe(8);
  expect(pill.left).toBeGreaterThanOrEqual(pill.clipLeft + 1);
  expect(pill.left + pill.width).toBeLessThanOrEqual(pill.rowLeft);
  expect(pill.radius).not.toMatch(/^0px/);
});

test("the switcher finds a room by name and opens it", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const dialog = await openSwitcher(page, false);

  await expect(dialog.getByRole("group", { name: "Recent" })).toBeVisible();
  await page.keyboard.type("desi");
  await expect(dialog.getByRole("option").first()).toContainText("design");
  await page.keyboard.press("Enter");

  await expect(dialog).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.design}$`));
  await expect(page.getByRole("heading", { level: 1, name: "design" })).toBeVisible();

  // The pick comes back first next time.
  await openSwitcher(page, false);
  await expect(
    page.getByRole("group", { name: "Recent" }).getByRole("option").first(),
  ).toContainText("design");
});

test("the switcher moves with the arrow keys and closes on Escape", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const dialog = await openSwitcher(page, false);
  const input = dialog.getByRole("combobox");
  const first = await input.getAttribute("aria-activedescendant");

  await page.keyboard.press("ArrowDown");
  await expect(input).not.toHaveAttribute("aria-activedescendant", first ?? "");
  await page.keyboard.press("ArrowUp");
  await expect(input).toHaveAttribute("aria-activedescendant", first ?? "");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

test("the shortcuts dialog lists the catalogue and filters it", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await page.keyboard.press(`${MOD}+/`);

  const dialog = page.getByRole("dialog", { name: "Keyboard shortcuts" });

  await expect(dialog).toBeVisible();

  for (const group of ["Navigation", "Messages", "Composer", "Formatting"]) {
    await expect(dialog.getByRole("heading", { name: group })).toBeVisible();
  }

  await page.keyboard.type("bold");
  await expect(dialog.getByRole("term")).toHaveText(["Bold"]);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();

  // The workspace menu reaches it too.
  await sidebar(page).getByRole("button", { name: "Smart Data" }).click();
  await page.getByRole("menuitem", { name: /Keyboard shortcuts/ }).click();
  await expect(dialog).toBeVisible();
});

test("Alt+↓ and Alt+↑ step through the sidebar's order", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);
  await expect(page.getByRole("heading", { level: 1, name: "general" })).toBeVisible();

  const hrefs = await sidebar(page)
    .locator(".sidebar-row")
    .evaluateAll((rows) => rows.map((row) => row.getAttribute("href") ?? ""));

  const index = hrefs.findIndex((href) => href.endsWith(`/r/${ROOM_IDS.general}`));
  const next = hrefs[(index + 1) % hrefs.length] ?? "";
  const previous = hrefs[(index - 1 + hrefs.length) % hrefs.length] ?? "";

  await page.keyboard.press("Alt+ArrowDown");
  await expect(page).toHaveURL(new RegExp(`${next}$`));
  await page.keyboard.press("Alt+ArrowUp");
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
  await page.keyboard.press("Alt+ArrowUp");
  await expect(page).toHaveURL(new RegExp(`${previous}$`));
});

test("a new DM with one person opens it", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  const dialog = await openNewDirect(page, false);

  await page.keyboard.type("sam");
  await page.keyboard.press("Enter");
  await expect(dialog.getByText("Sam", { exact: false }).first()).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Start conversation" })).toBeEnabled();
  await page.keyboard.press("Enter");

  await expect(dialog).toBeHidden();
  await expect(page.getByRole("heading", { level: 1, name: /Sam/ })).toBeVisible();
  await expect(sidebar(page).getByRole("link", { name: /Sam/ })).toBeVisible();
});

test("adding people to a one-to-one starts a new group", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.dmMaya}`);
  await page.getByRole("button", { name: "Add people" }).click();

  const dialog = page.getByRole("dialog", { name: /Add people/ });

  await expect(dialog.getByText(/new group conversation/)).toBeVisible();
  // The other person is already in: a chip that can't be taken off.
  await expect(dialog.getByText("Maya Okafor", { exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Remove Maya Okafor" })).toHaveCount(0);
  await page.keyboard.type("sam");
  await page.keyboard.press("Enter");
  await dialog.getByRole("button", { name: "Start group conversation" }).click();

  await expect(dialog).toBeHidden();
  await expect(page).not.toHaveURL(new RegExp(`/r/${ROOM_IDS.dmMaya}$`));
  await expect(page.getByRole("heading", { level: 1, name: /Maya/ })).toBeVisible();
});

// --- Screenshots: every surface in light and dark, desktop and phone. What they check is
// covered by the tests above and by phone-sheets/phone-header, so they only run for shots. ---

if (SHOTS) {
  matrix("sidebar with unread", async ({ page, theme, phone }) => {
    await openApp(page, phone ? "" : `r/${ROOM_IDS.engineering}`, theme);
    await expect(sidebar(page).locator('.sidebar-row[data-state="unread"]').first()).toBeVisible();
    await shot(page, "sidebar", theme);
  });

  matrix("switcher", async ({ page, theme, phone }) => {
    await openApp(page, phone ? "" : `r/${ROOM_IDS.general}`, theme);

    const dialog = await openSwitcher(page, phone);

    await expect(dialog.getByRole("option").first()).toBeVisible();
    await shot(page, "switcher-empty", theme);
    await dialog.getByRole("combobox").fill("an");
    await expect(dialog.getByRole("option").first()).toBeVisible();
    await shot(page, "switcher-query", theme);
  });

  matrix("shortcuts dialog", async ({ page, theme, phone }) => {
    await openApp(page, phone ? "" : `r/${ROOM_IDS.general}`, theme);

    if (phone) {
      await sidebar(page).getByRole("button", { name: "Smart Data" }).click();
      await page.getByRole("menuitem", { name: /Keyboard shortcuts/ }).click();
    } else {
      await page.keyboard.press(`${MOD}+/`);
    }

    await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
    await shot(page, "shortcuts", theme);
  });

  matrix("new DM picker", async ({ page, theme, phone }) => {
    await openApp(page, phone ? "" : `r/${ROOM_IDS.general}`, theme);

    const dialog = await openNewDirect(page, phone);
    const input = dialog.getByRole("combobox");

    await input.fill("lu");
    await input.press("Enter");
    await input.fill("sam");
    await input.press("Enter");
    await input.fill("");
    await shot(page, "new-direct", theme);
  });

  /** Opens the group DM; a phone opens on the sidebar, so it taps through like a person would. */
  async function openGroupDirect(page: Page, theme: "light" | "dark", phone: boolean) {
    if (phone) {
      await openApp(page, "", theme);
      await sidebar(page)
        .getByRole("link", { name: /Jonah Lindqvist, Priya Raman/ })
        .click();
    } else {
      await openApp(page, `r/${ROOM_IDS.groupDm}`, theme);
    }
  }

  matrix("add people", async ({ page, theme, phone }) => {
    await openGroupDirect(page, theme, phone);
    await openHeaderTool(page, "Add people");

    const dialog = page.getByRole("dialog", { name: /Add people/ });

    await expect(dialog.getByRole("option").first()).toBeVisible();
    await dialog.getByRole("combobox").fill("gr");
    await dialog.getByRole("combobox").press("Enter");
    await shot(page, "add-people", theme);
  });

  matrix("rename", async ({ page, theme, phone }) => {
    await openGroupDirect(page, theme, phone);
    await openHeaderTool(page, "Rename conversation");
    await expect(page.getByRole("dialog", { name: "Rename conversation" })).toBeVisible();
    await page.getByRole("textbox", { name: "Name" }).fill("Launch crew");
    await shot(page, "rename", theme);
  });
}

test.describe("the tab bar and the list on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  const tabBar = (page: Page) => page.getByRole("navigation", { name: "Destinations" });

  test("every tab and every control in the list is thumb-sized", async ({ page }) => {
    await openApp(page, "");
    await expect(sidebar(page).locator(".sidebar-row").first()).toBeVisible();
    await expectTouchTargets(page, ".rail");
    await expectTouchTargets(page, ".sidebar");
    await expectNoHorizontalOverflow(page);
  });

  test("a tap on a tab's caption switches to it", async ({ page }) => {
    await openApp(page, "");

    const dms = tabBar(page).getByRole("button", { name: "DMs" });

    await dms.locator(".rail-caption").tap();
    await expect(dms).toHaveAttribute("aria-pressed", "true");
    await expect(sidebar(page).getByText("Direct messages").first()).toBeVisible();
  });

  test("You opens the account menu as a sheet", async ({ page }) => {
    await openApp(page, "");
    await expect(sidebar(page).locator(".sidebar-you")).toBeHidden();
    await tabBar(page).getByRole("button", { name: "You" }).tap();

    const menu = page.getByRole("menu", { name: "Your account" });

    await expect(menu).toBeVisible();
    await expect(menu).toHaveClass(/\baction-sheet\b/);
  });

  test("the DMs tab previews each conversation's newest message and its time", async ({ page }) => {
    await openApp(page, "");
    await tabBar(page).getByRole("button", { name: "DMs" }).tap();

    const row = sidebar(page).locator(".sidebar-row[data-preview]").first();

    await expect(row.locator(".sidebar-row-preview")).toHaveText(/\S/);
    await expect(row.locator(".sidebar-row-time")).toBeVisible();
    await expect(row.locator(".sidebar-row-time")).toHaveAttribute("datetime", /^\d{4}-/);
    await expectTouchTargets(page, ".sidebar");
    await expectNoHorizontalOverflow(page);

    // The preview follows a new message.
    await row.tap();
    await page.getByRole("textbox", { name: /^Message/ }).fill("Previewed on the DMs tab");
    await page.keyboard.press("Enter");
    await page.goBack();
    await tabBar(page).getByRole("button", { name: "DMs" }).tap();
    await expect(sidebar(page).locator(".sidebar-row-preview").first()).toContainText(
      "Previewed on the DMs tab",
    );
  });
});
