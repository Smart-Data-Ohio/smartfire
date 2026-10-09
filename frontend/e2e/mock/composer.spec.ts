import type { Page } from "@playwright/test";
import { boardDeckPdf, onboardingMockupPng } from "../../mock/s2/assets.ts";
import {
  expect,
  expectTouchTargets,
  matrix,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

const GENERAL = `r/${ROOM_IDS.general}`;

/**
 * Opens #general with motion reduced. Unlike `openApp` it waits for the composer, not the
 * sidebar: on a phone a room's route shows the room alone.
 */
async function openApp(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await composer(page).waitFor();
}

/** Waits for the finite animations and transitions (not spinners), so a shot never catches a fade. */
async function settle(page: Page): Promise<void> {
  await page.evaluate(() => {
    const finishing = document.getAnimations().flatMap((animation) =>
      animation.effect?.getComputedTiming().iterations === Infinity
        ? []
        : // A cancelled animation (its element left) rejects; it's settled all the same.
          [animation.finished.catch(() => animation)],
    );

    return Promise.all(finishing);
  });
}

function composer(page: Page) {
  return page.getByRole("textbox", { name: "Message #general" });
}

function suggestions(page: Page) {
  return page.getByRole("listbox");
}

/** Types into the composer one key at a time, so the caret and the triggers follow along. */
async function typeInto(page: Page, text: string): Promise<void> {
  await composer(page).click();
  await composer(page).pressSequentially(text);
}

async function clearComposer(page: Page): Promise<void> {
  await composer(page).fill("");
}

/** A posted message in the room's log containing `text`. */
function posted(page: Page, text: string | RegExp) {
  return page.getByRole("log", { name: "Messages" }).getByText(text).last();
}

/** A file a few MB long, so a throttled upload shows its progress ring part-way. */
function bigPdf(): Buffer {
  const deck = Buffer.from(boardDeckPdf());

  return Buffer.concat([deck, Buffer.alloc(3 * 1024 * 1024, 32)]);
}

async function throttleUploads(page: Page, bytesPerSecond: number | null): Promise<void> {
  const cdp = await page.context().newCDPSession(page);

  await cdp.send("Network.enable");
  await cdp.send("Network.emulateNetworkConditions", {
    offline: false,
    latency: 0,
    downloadThroughput: -1,
    uploadThroughput: bytesPerSecond ?? -1,
  });
}

matrix("autocomplete: people, emoji, commands and channels", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);

  await typeInto(page, "Thanks @ma");
  await expect(suggestions(page).getByRole("option", { name: /Maya Okafor/ })).toBeVisible();
  await settle(page);
  await shot(page, "autocomplete-mention", theme);
  await composer(page).press("Enter");
  await expect(composer(page)).toHaveValue("Thanks @[Maya Okafor] ");
  await expect(suggestions(page)).toBeHidden();

  await clearComposer(page);
  await typeInto(page, "Ship it :ta");
  await expect(suggestions(page).getByRole("option").first()).toBeVisible();
  await settle(page);
  await shot(page, "autocomplete-emoji", theme);
  await composer(page).press("Tab");
  await expect(composer(page)).toHaveValue(/^Ship it :[a-z0-9_+-]+: $/);

  await clearComposer(page);
  await typeInto(page, "/");
  await expect(suggestions(page).getByRole("option", { name: /\/remind/ })).toBeVisible();
  await settle(page);
  await shot(page, "autocomplete-command", theme);
  await composer(page).pressSequentially("shr");
  await expect(suggestions(page).getByRole("option")).toHaveCount(1);
  await composer(page).press("Enter");
  await expect(composer(page)).toHaveValue("/shrug ");

  await clearComposer(page);
  await typeInto(page, "See #des");
  await expect(suggestions(page).getByRole("option", { name: /design/ })).toBeVisible();
  await settle(page);
  await shot(page, "autocomplete-channel", theme);
  await composer(page).press("Enter");
  await expect(composer(page)).toHaveValue(`See [#design](/rooms/${ROOM_IDS.design}) `);
});

test("autocomplete: arrows move, Escape closes until the trigger changes", async ({ page }) => {
  await openApp(page, GENERAL);
  await typeInto(page, "@");

  const options = suggestions(page).getByRole("option");

  await expect(options.first()).toHaveAttribute("aria-selected", "true");
  await composer(page).press("ArrowDown");
  await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
  await expect(composer(page)).toHaveAttribute(
    "aria-activedescendant",
    (await options.nth(1).getAttribute("id")) ?? "",
  );
  await composer(page).press("Escape");
  await expect(suggestions(page)).toBeHidden();
  await composer(page).pressSequentially("j");
  await expect(suggestions(page)).toBeHidden();
});

matrix("the + menu opens as liquid", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "no-preference" });
  await composer(page).click();

  const trigger = page.getByRole("button", { name: "Attach and more" });

  // Until its lazy chunk arrives the + button is the plain dropdown, and the liquid one replaces
  // it (closed) when the chunk lands; a fresh dev server compiles that chunk on first use.
  await expect(trigger).toHaveClass(/\bgooey-plus-trigger\b/);
  await trigger.click();

  const menu = page.getByRole("menu", { name: "Attach and more" });

  await expect(menu.getByRole("menuitem", { name: /Upload a file/ })).toBeVisible();
  await expect(trigger).toHaveAttribute("aria-expanded", "true");
  await page.waitForTimeout(900);
  await settle(page);
  await shot(page, "plus-menu-open", theme);
  await page.keyboard.press("Escape");
  await expect(trigger).toHaveAttribute("aria-expanded", "false");
});

matrix("uploads a file with progress, then sends it", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);
  await throttleUploads(page, 400 * 1024);
  await page.locator('.composer input[type="file"]').setInputFiles([
    { name: "Q4-board-deck.pdf", mimeType: "application/pdf", buffer: bigPdf() },
    {
      name: "onboarding-mockup.png",
      mimeType: "image/png",
      buffer: Buffer.from(onboardingMockupPng()),
    },
  ]);

  const chip = page.locator(".tray-chip").first();

  await expect(chip).toHaveAttribute("data-phase", "uploading");
  await expect(chip).toContainText(/Uploading [1-9]\d?%/);
  await settle(page);
  await shot(page, "upload-progress", theme);
  await throttleUploads(page, null);
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(2, { timeout: 15_000 });
  await typeInto(page, "Deck and mockup for Thursday");
  await composer(page).press("Enter");
  await expect(posted(page, "Deck and mockup for Thursday")).toBeVisible();
  await expect(page.locator(".tray-chip")).toHaveCount(0);
});

test("sending while a file uploads waits for it", async ({ page }) => {
  await openApp(page, GENERAL);
  await throttleUploads(page, 600 * 1024);
  await page
    .locator('.composer input[type="file"]')
    .setInputFiles({ name: "notes.pdf", mimeType: "application/pdf", buffer: bigPdf() });
  await typeInto(page, "Sending once it lands");
  await composer(page).press("Enter");
  await expect(page.getByRole("button", { name: "Sending when uploads finish" })).toBeVisible();
  await throttleUploads(page, null);
  await expect(posted(page, "Sending once it lands")).toBeVisible({ timeout: 15_000 });
});

test("pasting an image attaches it", async ({ page }) => {
  await openApp(page, GENERAL);
  await composer(page).click();

  const bytes = [...onboardingMockupPng()];

  await composer(page).evaluate((element, data) => {
    const transfer = new DataTransfer();

    transfer.items.add(new File([new Uint8Array(data)], "image.png", { type: "image/png" }));
    element.dispatchEvent(
      new ClipboardEvent("paste", { clipboardData: transfer, bubbles: true, cancelable: true }),
    );
  }, bytes);

  const chip = page.locator(".tray-chip");

  await expect(chip).toHaveAttribute("data-kind", "image");
  await expect(chip.getByRole("button", { name: /Remove Pasted image/ })).toBeVisible();
  await expect(chip).toHaveAttribute("data-phase", "done", { timeout: 10_000 });
});

matrix("the drop overlay covers the pane", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);

  const transfer = await page.evaluateHandle(() => {
    const data = new DataTransfer();

    data.items.add(new File(["hello"], "hello.txt", { type: "text/plain" }));

    return data;
  });

  await page.locator(".room").dispatchEvent("dragenter", { dataTransfer: transfer });
  await expect(page.getByText("Drop files to upload")).toBeVisible();
  await settle(page);
  await shot(page, "drop-overlay", theme);
  await page.locator(".room").dispatchEvent("drop", { dataTransfer: transfer });
  await expect(page.getByText("Drop files to upload")).toBeHidden();
  await expect(page.locator(".tray-chip")).toHaveCount(1);
});

matrix("previews the Markdown as it will post", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);
  await typeInto(page, "**Launch** moves to _Thursday_. See `rollout.md` and @[Maya Okafor]");
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "Preview message" }).click();

  const preview = page.locator(".composer-preview-body");

  await expect(preview.locator("strong")).toHaveText("Launch");
  await expect(preview.locator("code")).toHaveText("rollout.md");
  await settle(page);
  await shot(page, "preview", theme);
  await composer(page).press("Escape");
  await expect(preview).toBeHidden();
});

matrix("schedules a message and manages the scheduled list", async ({ page, theme }) => {
  await openApp(page, GENERAL, theme);

  const list = page.getByRole("button", { name: /scheduled messages$/ });

  await expect(list).toHaveAccessibleName("1 scheduled messages");
  await typeInto(page, "Standup notes are in the doc");
  await page.getByRole("button", { name: "Schedule message" }).click();
  await expect(page.getByRole("menuitem", { name: /Tomorrow at 9:00/ })).toBeVisible();
  await settle(page);
  await shot(page, "schedule-menu", theme);
  await page.getByRole("menuitem", { name: /Tomorrow at 9:00/ }).click();
  await expect(composer(page)).toHaveValue("");
  await expect(list).toHaveAccessibleName("2 scheduled messages");

  await list.click();

  const items = page.locator(".scheduled-item");

  await expect(items).toHaveCount(2);
  await expect(items.filter({ hasText: "Standup notes are in the doc" })).toBeVisible();
  await settle(page);
  await shot(page, "scheduled-list", theme);
  await items
    .filter({ hasText: "Standup notes" })
    .getByRole("button", { name: "Cancel scheduled message" })
    .click();
  await expect(items).toHaveCount(1);
});

test("schedules at a custom time", async ({ page }) => {
  await openApp(page, GENERAL);
  await typeInto(page, "Later, at a time I pick");
  await page.getByRole("button", { name: "Schedule message" }).click();
  await page.getByRole("menuitem", { name: /Custom time/ }).click();

  const dialog = page.getByRole("dialog", { name: "Schedule message" });

  await expect(dialog.getByLabel("Send at")).toBeFocused();
  await dialog.getByRole("button", { name: "Schedule" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("button", { name: "2 scheduled messages" })).toBeVisible();
});

test("slash commands run, // escapes, unknown words post", async ({ page }) => {
  await openApp(page, GENERAL);

  await typeInto(page, "/shrug fine by me");
  await composer(page).press("Enter");
  await expect(posted(page, String.raw`fine by me ¯\_(ツ)_/¯`)).toBeVisible();

  await typeInto(page, "//etc/hosts is the file");
  await composer(page).press("Enter");
  await expect(posted(page, "/etc/hosts is the file")).toBeVisible();

  await typeInto(page, "/nope not a command");
  await composer(page).press("Enter");
  await expect(posted(page, "/nope not a command")).toBeVisible();
});

test("a command's post takes a reader far back in the room to it", async ({ page }) => {
  // #general opens on a window around its first unread, many messages back; with no newer pages
  // to be had, it stays short of the present, where the post lands.
  await page.route(
    (url) => url.searchParams.has("after"),
    (route) => route.abort(),
  );
  await openApp(page, GENERAL);
  await expect(page.locator(".unread-divider")).toBeInViewport();

  await typeInto(page, "/shrug far back");
  await composer(page).press("Enter");
  await expect(posted(page, String.raw`far back ¯\_(ツ)_/¯`)).toBeInViewport();
});

test("a command's usage shows while you type it", async ({ page }) => {
  await openApp(page, GENERAL);
  await typeInto(page, "/remind ");
  await expect(page.locator(".composer-usage")).toContainText("/remind <when> <text>");
});

test("formatting chords wrap the selection; Cmd+K is left to the switcher", async ({ page }) => {
  await openApp(page, GENERAL);
  await typeInto(page, "docs");
  await composer(page).press("ControlOrMeta+a");
  await composer(page).press("ControlOrMeta+b");
  await expect(composer(page)).toHaveValue("**docs**");
  await composer(page).press("ControlOrMeta+a");
  await composer(page).press("ControlOrMeta+Shift+u");
  await expect(composer(page)).toHaveValue("[**docs**](url)");
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  /** A finger resting on `target` for `ms`, then lifting: touch pointer events, then the click. */
  async function longPress(target: ReturnType<Page["getByRole"]>, ms = 650): Promise<void> {
    const box = await target.boundingBox();

    if (box === null) {
      throw new Error("the long-press target has no box");
    }

    const point = { clientX: box.x + box.width / 2, clientY: box.y + box.height / 2 };
    const touch = { ...point, pointerType: "touch", pointerId: 7, isPrimary: true, bubbles: true };

    await target.dispatchEvent("pointerdown", { ...touch, buttons: 1 });
    await target.page().waitForTimeout(ms);
    await target.dispatchEvent("pointerup", { ...touch, buttons: 0 });
    await target.dispatchEvent("click", point);
  }

  test("the composer's buttons are finger-sized", async ({ page }) => {
    await openApp(page, GENERAL);
    await typeInto(page, "Standup notes are in the doc");
    await expectTouchTargets(page, ".composer-toolbar");
  });

  test("a long press on send offers the schedule options instead of sending", async ({ page }) => {
    await openApp(page, GENERAL);
    await typeInto(page, "Standup notes are in the doc");

    // One send button: the schedule chevron folds into it.
    const group = await page.locator(".composer-send-group").boundingBox();
    const send = page.getByRole("button", { name: "Send message" });
    const sendBox = await send.boundingBox();

    expect(group?.width ?? 0).toBeLessThanOrEqual((sendBox?.width ?? 0) + 1);

    await longPress(send);

    const tomorrow = page.getByRole("menuitem", { name: /Tomorrow at 9:00/ });

    await expect(tomorrow).toBeVisible();
    await expect(composer(page)).toHaveValue("Standup notes are in the doc");
    await tomorrow.click();
    await expect(composer(page)).toHaveValue("");
    await expect(page.getByRole("button", { name: "2 scheduled messages" })).toBeVisible();
  });

  /** A finger resting on `target` past the long press, then `end`ing it some other way. */
  async function heldThen(
    target: ReturnType<Page["getByRole"]>,
    end: "cancel" | "drift",
  ): Promise<void> {
    const box = await target.boundingBox();

    if (box === null) {
      throw new Error("the long-press target has no box");
    }

    const point = { clientX: box.x + box.width / 2, clientY: box.y + box.height / 2 };
    const touch = { ...point, pointerType: "touch", pointerId: 7, isPrimary: true, bubbles: true };

    await target.dispatchEvent("pointerdown", { ...touch, buttons: 1 });
    await target.page().waitForTimeout(650);

    if (end === "cancel") {
      // The browser took the touch for a scroll: no pointerup, no click.
      await target.dispatchEvent("pointercancel", { ...touch, buttons: 0 });
    } else {
      await target.dispatchEvent("pointermove", {
        ...touch,
        clientY: point.clientY - 40,
        buttons: 1,
      });
      await target.dispatchEvent("pointerup", {
        ...touch,
        clientY: point.clientY - 40,
        buttons: 0,
      });
      await target.dispatchEvent("click", point);
    }

    // Long enough for a menu that opens after the release to have shown.
    await target.page().waitForTimeout(400);
  }

  for (const end of ["cancel", "drift"] as const) {
    test(`a held press that ends in a ${end === "cancel" ? "pointercancel" : "drift"} neither opens the menu nor sends`, async ({
      page,
    }) => {
      await openApp(page, GENERAL);
      await typeInto(page, "Standup notes are in the doc");
      await heldThen(page.getByRole("button", { name: "Send message" }), end);
      await expect(page.getByRole("menuitem", { name: /Tomorrow at 9:00/ })).toHaveCount(0);
      await expect(composer(page)).toHaveValue("Standup notes are in the doc");
    });
  }

  test("a tap on send still sends", async ({ page }) => {
    await openApp(page, GENERAL);
    await typeInto(page, "Sent with a tap");
    await page.getByRole("button", { name: "Send message" }).tap();
    await expect(posted(page, "Sent with a tap")).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /Tomorrow at 9:00/ })).toHaveCount(0);
  });

  test("(edited) runs on after a wrapped message's last word", async ({ page }) => {
    await openApp(page, GENERAL);

    const first = "First of two, so the next one continues the group";
    const long = "A continuation long enough to wrap onto a second line on a phone screen";

    await typeInto(page, first);
    await composer(page).press("Enter");
    await expect(posted(page, first)).toBeVisible();
    await typeInto(page, long);
    await composer(page).press("Enter");

    const row = page.locator(".message[data-message-id]").filter({ hasText: long });
    const id = await row.getAttribute("data-message-id");
    const state = await (await page.request.get("/__mock/state")).json();

    const edit = await page.request.patch(`/api/v1/messages/${id}`, {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: { markdownSource: `${long}, now edited` },
    });

    expect(edit.ok()).toBe(true);

    const edited = row.locator(".message-body-row > .message-edited");

    await expect(edited).toBeVisible();

    const placement = await row.locator(".message-body-row").evaluate((bodyRow) => {
      const body = bodyRow.querySelector(".message-body");
      const mark = bodyRow.querySelector(".message-edited")?.getBoundingClientRect();
      const range = document.createRange();

      if (body === null || mark === undefined) {
        throw new Error("no body or no (edited)");
      }

      range.selectNodeContents(body);

      const lines = [...range.getClientRects()].filter((rect) => rect.width > 0);
      const last = lines.at(-1);

      if (last === undefined) {
        throw new Error("the body has no text");
      }

      return {
        lines: new Set(lines.map((rect) => Math.round(rect.top))).size,
        sameLine: mark.top < last.bottom && mark.bottom > last.top,
        gap: mark.left - last.right,
      };
    });

    expect(placement.lines, "the message wraps").toBeGreaterThan(1);
    expect(placement.sameLine, "(edited) sits on the body's last line").toBe(true);
    expect(placement.gap, "right after the last word").toBeGreaterThanOrEqual(0);
    expect(placement.gap, "right after the last word").toBeLessThan(12);
  });

  test("the keyboard still reaches the schedule options", async ({ page }) => {
    await openApp(page, GENERAL);
    await typeInto(page, "Standup notes are in the doc");

    const more = page.getByRole("button", { name: "Schedule message" });

    // Folded away until the keyboard lands on it: no box of its own beside send.
    await expect(more).toHaveCSS("clip-path", "inset(50%)");

    await page.getByRole("button", { name: "Send message" }).focus();
    await page.keyboard.press("Tab");
    await expect(more).toBeFocused();
    await expect(more).toHaveCSS("clip-path", "none");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("menuitem", { name: /In 1 hour/ })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(more).toBeFocused();
  });
});
