import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  DESKTOP,
  expect,
  openApp,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  simulateKeyboard,
  simulateSafeAreas,
  type Theme,
  test,
} from "./support.ts";

/**
 * Dialogs on a 360 px touch phone are sheets: full height (or bottom-anchored at size="sm"), the
 * header and footer pinned, the footer above the keyboard and the home indicator, a 44 px close
 * button, no keyboard raised on open, and a swipe down from the header to dismiss.
 */

const GENERAL = `r/${ROOM_IDS.general}`;

/** The space Dialog's full-height sheet leaves above itself, short of the status bar. */
const SHEET_GAP = 8;

/** Right-clicks a message (the long-press menu's twin) and picks `item`. */
async function fromMessageMenu(page: Page, item: RegExp, theme: Theme = "light"): Promise<void> {
  await openApp(page, `${GENERAL}/m/${MESSAGE_IDS.generalReactions}`, theme);

  const row = page.locator(`[data-message-id="${MESSAGE_IDS.generalReactions}"]`).first();

  await expect(row).toBeVisible();
  await row.locator(".message-body").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: item }).click();
}

/** Waits out the sheet's own entrance (it fades in even with motion reduced), for a settled shot. */
async function settled(dialog: Locator): Promise<void> {
  await dialog.evaluate((element) =>
    Promise.all(element.getAnimations().map((animation) => animation.finished.catch(() => null))),
  );
}

interface Sheet {
  readonly title: string;
  /** Opens the dialog; resolves once its content (not a loading state) is showing. */
  readonly open: (page: Page, dialog: Locator, theme?: Theme) => Promise<void>;
}

const SHEETS: readonly Sheet[] = [
  {
    title: "Create a channel",
    open: async (page, dialog, theme = "light") => {
      await openApp(page, "", theme);
      await page.getByRole("button", { name: "Create a channel", exact: true }).first().click();
      await expect(dialog.getByLabel("Name", { exact: true })).toBeVisible();
    },
  },
  {
    title: "Channel settings",
    open: async (page, dialog, theme = "light") => {
      await openApp(page, `${GENERAL}/settings`, theme);
      await expect(dialog.getByLabel("Name", { exact: true })).toBeVisible();
    },
  },
  {
    title: "Schedule an event",
    open: async (page, dialog, theme = "light") => {
      await openApp(page, `${GENERAL}/events/new`, theme);
      await expect(dialog.getByLabel("Title")).toBeVisible();
    },
  },
  {
    title: "Create a poll",
    open: async (page, dialog, theme = "light") => {
      await openApp(page, GENERAL, theme);
      await page.getByRole("button", { name: "Attach and more" }).click();
      await page.getByRole("menuitem", { name: "Create a poll" }).click();
      await expect(dialog.getByRole("button", { name: "Post poll" })).toBeVisible();
    },
  },
  {
    title: "Forward message",
    open: async (page, dialog, theme = "light") => {
      await fromMessageMenu(page, /Forward/, theme);
      await expect(dialog.getByRole("checkbox").first()).toBeVisible();
    },
  },
  {
    title: "Create Fizzy card",
    open: async (page, dialog, theme = "light") => {
      await fromMessageMenu(page, /Create Fizzy card/, theme);
      await expect(dialog.getByLabel("Board")).toBeVisible();
    },
  },
  {
    title: "New message",
    open: async (page, dialog, theme = "light") => {
      await openApp(page, "", theme);
      await page.getByRole("button", { name: "New message" }).first().click();
      await expect(dialog.getByRole("option").first()).toBeVisible();
    },
  },
];

function sheet(title: string): Sheet {
  const found = SHEETS.find((candidate) => candidate.title === title);

  if (found === undefined) {
    throw new Error(`no sheet ${title}`);
  }

  return found;
}

function slug(title: string): string {
  return title.toLowerCase().replaceAll(" ", "-");
}

/** A field that raises the on-screen keyboard when it takes focus. */
const TEXT_FIELD =
  "textarea, [contenteditable='true'], input:not([type='button'], [type='checkbox'], [type='radio'], [type='range'], [type='submit'])";

/** The focused element when it is a field that raises the keyboard, else null. */
function focusedField(page: Page): Promise<string | null> {
  return page.evaluate(() => {
    const active = document.activeElement;

    const field =
      active instanceof HTMLTextAreaElement ||
      (active instanceof HTMLElement && active.isContentEditable) ||
      (active instanceof HTMLInputElement &&
        !["button", "checkbox", "radio", "range", "submit"].includes(active.type));

    return field ? active.outerHTML.slice(0, 120) : null;
  });
}

async function box(locator: Locator) {
  const rect = await locator.boundingBox();

  if (rect === null) {
    throw new Error("not rendered");
  }

  return { ...rect, bottom: rect.y + rect.height };
}

/**
 * Drags a real touch (CDP touch events, so the pointer is active) from `from` down by `distance`
 * (and across by `sideways`) in `steps` moves. Each move lands about a frame after the last, so a
 * few big steps make a flick and many small ones a slow drag.
 */
async function swipeDown(
  page: Page,
  from: Locator,
  distance: number,
  steps: number,
  sideways = 0,
): Promise<void> {
  const start = await box(from);
  const x = start.x + start.width / 2;
  const y = start.y + start.height / 2;
  const cdp = await page.context().newCDPSession(page);

  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y }] });

  for (let step = 1; step <= steps; step += 1) {
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchMove",
      touchPoints: [{ x: x + (sideways * step) / steps, y: y + (distance * step) / steps }],
    });
  }

  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await cdp.detach();
}

test.describe("on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  for (const { title, open } of SHEETS) {
    test(`${title} is a full-height sheet that keeps its footer above the keyboard`, async ({
      page,
    }) => {
      const dialog = page.getByRole("dialog", { name: title });

      await open(page, dialog);

      // Edge to edge, from just under the status bar to the bottom.
      const frame = await box(dialog);

      expect(frame.x).toBe(0);
      expect(frame.width).toBe(PHONE_SMALL.width);
      expect(frame.y).toBeLessThanOrEqual(SHEET_GAP);
      expect(frame.bottom).toBe(PHONE_SMALL.height);

      // Nothing raised the keyboard before the sheet could be read.
      expect(await focusedField(page)).toBeNull();

      const close = await box(dialog.getByRole("button", { name: "Close" }));

      expect(close.width).toBeGreaterThanOrEqual(44);
      expect(close.height).toBeGreaterThanOrEqual(44);

      // Tapping a field raises an overlaid keyboard (iOS) over the bottom 320 px: the footer sits
      // right above it. The shell only reads a keyboard while a text field has focus.
      await dialog.locator(TEXT_FIELD).first().focus();
      await simulateKeyboard(page, PHONE_SMALL.height - 420);
      await expect(page.locator("html")).toHaveAttribute("data-keyboard", "open");

      const footer = dialog.locator(".dialog-footer, [data-dialog-actions]").first();
      const header = await box(dialog.locator(".dialog-header"));

      await expect.poll(async () => (await box(dialog)).bottom).toBe(420);

      const pinned = await box(footer);

      expect(pinned.bottom).toBeLessThanOrEqual(420);
      expect(pinned.y).toBeGreaterThan(header.bottom);
      await expect(footer.getByRole("button").last()).toBeInViewport();
      await settled(dialog);
      await shot(page, `sheet-${slug(title)}-keyboard`, "light");
    });
  }

  test("a resizing keyboard (Android) shrinks the sheet, keeping the footer", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Schedule an event" });

    await sheet("Schedule an event").open(page, dialog);
    await page.setViewportSize({ width: PHONE_SMALL.width, height: 420 });

    await expect.poll(async () => (await box(dialog)).bottom).toBe(420);
    await expect(dialog.getByRole("button", { name: "Schedule event" })).toBeInViewport();
  });

  test("a keyboard that pans the page up keeps the sheet in the visible area", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Schedule an event" });

    await sheet("Schedule an event").open(page, dialog);
    await dialog.getByLabel("Title").focus();
    // iOS pans the layout viewport 100 px up to the field: the visible area is 100 to 420 + 100.
    await simulateKeyboard(page, PHONE_SMALL.height - 420, { offsetTop: 100 });

    await expect.poll(async () => (await box(dialog)).bottom).toBe(520);
    expect((await box(dialog)).y).toBe(100 + SHEET_GAP);
    await expect(dialog.getByRole("button", { name: "Close" })).toBeInViewport();
    await expect(dialog.getByRole("button", { name: "Schedule event" })).toBeInViewport();
  });

  test("the footer keeps clear of the home indicator, and the header of the notch", async ({
    page,
  }) => {
    const dialog = page.getByRole("dialog", { name: "Create a poll" });

    await sheet("Create a poll").open(page, dialog);
    await simulateSafeAreas(page, { top: 47, bottom: 34 });

    expect((await box(dialog)).y).toBe(47 + SHEET_GAP);

    const post = await box(dialog.getByRole("button", { name: "Post poll" }));

    expect(post.bottom).toBeLessThanOrEqual(PHONE_SMALL.height - 34);
  });

  test("a confirmation is a short sheet anchored to the bottom", async ({ page }) => {
    await fromMessageMenu(page, /Delete message/);

    const dialog = page.getByRole("alertdialog", { name: "Delete message?" });
    const frame = await box(dialog);

    expect(frame.x).toBe(0);
    expect(frame.width).toBe(PHONE_SMALL.width);
    expect(frame.bottom).toBe(PHONE_SMALL.height);
    expect(frame.height).toBeLessThan(PHONE_SMALL.height / 2);
    // No grabber: a confirmation doesn't swipe away.
    await expect(dialog.locator(".dialog-grabber")).toHaveCount(0);
    await expect(dialog.getByRole("button", { name: "Cancel" })).toBeFocused();
    await settled(dialog);
    await shot(page, "sheet-confirmation", "light");
  });

  test("a swipe down the header dismisses the sheet; a short one springs back", async ({
    page,
  }) => {
    const dialog = page.getByRole("dialog", { name: "New message" });

    await sheet("New message").open(page, dialog);

    const title = dialog.locator(".dialog-title");

    // A slow drag short of the distance springs back.
    await swipeDown(page, title, 60, 20);
    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);

    // As short, but a flick: it goes.
    await swipeDown(page, title, 60, 2);
    await expect(dialog).toBeHidden();

    // A slow drag past the distance closes it too.
    await sheet("New message").open(page, dialog);
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);
    await expect(dialog).toBeHidden();
  });

  test("a swipe while the form is scrolled is the form's, not the sheet's", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Schedule an event" });

    await sheet("Schedule an event").open(page, dialog);
    await dialog.locator(".dialog-body").evaluate((body) => {
      body.scrollTop = 200;
    });
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeVisible();
    expect((await box(dialog)).bottom).toBe(PHONE_SMALL.height);
  });

  test("a sideways drag on the header doesn't close the sheet", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "New message" });

    await sheet("New message").open(page, dialog);
    // Far enough down to close, were it a swipe down; but mostly across.
    await swipeDown(page, dialog.locator(".dialog-title"), 120, 40, -200);

    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);
  });

  test("a poll with a question typed springs back from a swipe", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Create a poll" });

    await sheet("Create a poll").open(page, dialog);
    await dialog.getByLabel("Question").fill("Lunch?");
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);
    await expect(dialog.getByLabel("Question")).toHaveValue("Lunch?");

    // The close button still closes it.
    await dialog.getByRole("button", { name: "Close" }).click();
    await expect(dialog).toBeHidden();
  });

  test("a poll with only a toggle flipped springs back from a swipe", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Create a poll" });
    const anonymous = dialog.getByRole("switch", { name: "Anonymous" });

    await sheet("Create a poll").open(page, dialog);
    await anonymous.click();
    await expect(anonymous).toHaveAttribute("aria-checked", "true");
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);
    await expect(anonymous).toHaveAttribute("aria-checked", "true");
  });

  test("a new message with a recipient tapped, nothing typed, springs back", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "New message" });

    await sheet("New message").open(page, dialog);
    await dialog.getByRole("option").first().click();
    await expect(dialog.locator(".picker-chip")).toHaveCount(1);
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);
    await expect(dialog.locator(".picker-chip")).toHaveCount(1);
  });

  test("a forward with a search typed and cleared still swipes away", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Forward message" });
    const search = dialog.getByRole("searchbox", { name: "Search conversations" });

    await sheet("Forward message").open(page, dialog);
    await search.fill("gen");
    await search.fill("");
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeHidden();
  });

  test("a poll with a toggle flipped and flipped back swipes away", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Create a poll" });
    const anonymous = dialog.getByRole("switch", { name: "Anonymous" });

    await sheet("Create a poll").open(page, dialog);
    await anonymous.click();
    await anonymous.click();
    await expect(anonymous).toHaveAttribute("aria-checked", "false");
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeHidden();
  });

  test("a new message with a recipient tapped then removed swipes away", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "New message" });

    await sheet("New message").open(page, dialog);
    await dialog.getByRole("option").first().click();
    await dialog.getByRole("button", { name: /^Remove / }).click();
    await expect(dialog.locator(".picker-chip")).toHaveCount(0);
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeHidden();
  });

  test("a rename renamed elsewhere, with the field as it opened, swipes away", async ({
    page,
    context,
  }) => {
    const path = `r/${ROOM_IDS.general}/t/${THREAD_IDS.generalActive}`;
    const dialog = page.getByRole("dialog", { name: "Rename thread" });
    const field = dialog.getByRole("textbox", { name: "Name" });

    await openApp(page, path);
    await page.getByRole("button", { name: "Thread actions" }).click();
    await page.getByRole("menuitem", { name: "Rename thread…" }).click();

    const opened = await field.inputValue();

    // Another tab renames the thread while this one has the dialog open.
    const other = await context.newPage();

    await openApp(other, path);
    await other.getByRole("button", { name: "Thread actions" }).click();
    await other.getByRole("menuitem", { name: "Rename thread…" }).click();

    const elsewhere = other.getByRole("dialog", { name: "Rename thread" });

    await elsewhere.getByRole("textbox", { name: "Name" }).fill("Renamed elsewhere");
    await elsewhere.getByRole("button", { name: "Save" }).click();
    await expect(elsewhere).toBeHidden();
    await other.close();
    await expect(page.getByText("Renamed elsewhere", { exact: true }).first()).toBeAttached();

    // Typed, then put back as the dialog found it: nothing to lose.
    await field.fill("Something else");
    await field.fill(opened);
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeHidden();
  });

  test("a sheet that won't close while sending springs back with its footer", async ({ page }) => {
    let release: () => void = () => undefined;

    const held = new Promise<void>((resolve) => {
      release = resolve;
    });

    await page.route("**/fizzy_cards", async (route) => {
      await held;
      await route.continue();
    });

    const dialog = page.getByRole("dialog", { name: "Create Fizzy card" });

    await sheet("Create Fizzy card").open(page, dialog);
    await dialog.getByLabel("Board").selectOption({ index: 1 });
    await dialog.getByRole("button", { name: "Create card" }).click();
    await swipeDown(page, dialog.locator(".dialog-title"), 200, 60);

    await expect(dialog).toBeVisible();
    await expect.poll(async () => (await box(dialog)).bottom).toBe(PHONE_SMALL.height);
    await expect(dialog.locator(".dialog-footer")).toBeInViewport({ ratio: 1 });
    release();
  });

  test("the switcher still opens ready to type", async ({ page }) => {
    await openApp(page, "");
    await page
      .getByRole("button", { name: /Jump to/ })
      .first()
      .click();

    const dialog = page.getByRole("dialog", { name: "Jump to a conversation" });

    await expect(dialog.getByRole("combobox")).toBeFocused();
    expect((await box(dialog)).bottom).toBe(PHONE_SMALL.height);
    await expect(dialog.getByRole("button", { name: "Close" })).toBeVisible();
  });

  for (const theme of ["light", "dark"] as const satisfies readonly Theme[]) {
    test(`the sheets in ${theme}`, async ({ page }) => {
      for (const { title, open } of SHEETS) {
        const dialog = page.getByRole("dialog", { name: title });

        await open(page, dialog, theme);
        await settled(dialog);
        await shot(page, `sheet-${slug(title)}`, theme);
      }
    });
  }
});

test.describe("on a desktop", () => {
  test.use({ viewport: DESKTOP });

  test("a dialog is still a centred card with its first field focused", async ({ page }) => {
    const dialog = page.getByRole("dialog", { name: "Schedule an event" });

    await sheet("Schedule an event").open(page, dialog);

    const frame = await box(dialog);

    // Centred, give or take the scrollbar gutter the open dialog keeps.
    expect(frame.width).toBe(480);
    expect(Math.abs(frame.x + frame.width / 2 - DESKTOP.width / 2)).toBeLessThanOrEqual(8);
    expect(frame.y).toBeGreaterThan(0);
    expect(frame.bottom).toBeLessThan(DESKTOP.height);
    await expect(dialog.getByLabel("Title")).toBeFocused();
    await expect(dialog.locator(".dialog-grabber")).toBeHidden();
  });
});
