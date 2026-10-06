import type { Page } from "@playwright/test";
import { boardDeckPdf, onboardingMockupPng } from "../../mock/s2/assets.ts";
import { expect, matrix, ROOM_IDS, shot, type Theme, test } from "./support.ts";

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
  await expect(posted(page, "fine by me ¯_(ツ)_/¯")).toBeVisible();

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
  await expect(posted(page, "far back ¯_(ツ)_/¯")).toBeInViewport();
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
