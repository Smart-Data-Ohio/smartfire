import type { Locator, Page, Request } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  longPress,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * Inline replies (classic's Reply): pick Reply on a message, the composer quotes it with Notify
 * author, Esc or × drops it, and the sent message carries the quote, which jumps to the original.
 */

const TARGET = MESSAGE_IDS.generalReactions;

const TARGET_TEXT = "Shipped the new onboarding checklist";

function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-id="${messageId}"]`);
}

function composer(page: Page): Locator {
  return page.getByRole("textbox", { name: "Message #general" });
}

function chip(page: Page): Locator {
  return page.locator(".composer").getByRole("region", { name: "Replying to Grace Adeyemi" });
}

async function openOn(page: Page, messageId: number, theme: Theme = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}/m/${messageId}`);
  await expect(row(page, messageId)).toBeVisible();
}

/** Hovers the row until its bar shows (a permalinked row can be swapped as the window settles). */
async function hoverBar(target: Locator): Promise<Locator> {
  const bar = target.getByRole("toolbar", { name: "Message actions" });

  await expect(async () => {
    await target.page().mouse.move(0, 0);
    await target.hover();
    await expect(bar).toBeVisible({ timeout: 1000 });
  }).toPass();

  return bar;
}

/** The next message create the page posts. */
function nextCreate(page: Page): Promise<Request> {
  return page.waitForRequest(
    (request) =>
      request.method() === "POST" && new URL(request.url()).pathname.endsWith("/messages"),
  );
}

/** The confirmed row holding `text`. */
async function sentRow(page: Page, text: string): Promise<Locator> {
  const sent = page.locator("[data-message-row]", { hasText: text }).last();

  await expect(sent).toHaveAttribute("data-message-id", /^\d+$/);

  return row(page, Number(await sent.getAttribute("data-message-id")));
}

for (const theme of ["light", "dark"] as const) {
  test(`reply from the hover bar, send, and jump back from the quote (${theme})`, async ({
    page,
  }) => {
    await openOn(page, TARGET, theme);

    const bar = await hoverBar(row(page, TARGET));

    await bar.getByRole("button", { name: "Reply", exact: true }).click();
    await expect(chip(page)).toBeVisible();
    await expect(chip(page)).toContainText(TARGET_TEXT);
    await expect(chip(page).getByRole("checkbox", { name: "Notify author" })).toBeChecked();
    await expect(composer(page)).toBeFocused();
    await page.mouse.move(0, 0);
    await shot(page, "composer-reply", theme);

    await composer(page).pressSequentially("Count me in");

    const create = nextCreate(page);

    await composer(page).press("Enter");
    expect((await create).postDataJSON()).toMatchObject({
      markdownSource: "Count me in",
      replyToMessageId: TARGET,
      replyNotifyAuthor: true,
    });
    await expect(chip(page)).toBeHidden();

    const reply = await sentRow(page, "Count me in");
    const quote = reply.locator(".message-reply");

    await expect(quote).toContainText("Grace Adeyemi");
    await expect(quote).toContainText(TARGET_TEXT);

    await quote.getByRole("link").click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}/m/${TARGET}$`));
    await expect(row(page, TARGET)).toBeInViewport();
  });
}

test("Esc and × drop the reply; the menu and Q pick it again", async ({ page }) => {
  await openOn(page, TARGET);

  await row(page, TARGET).locator(".message-body").first().click({ button: "right" });
  await page
    .getByRole("menu", { name: "Message actions" })
    .getByRole("menuitem", { name: "Reply", exact: true })
    .click();
  await expect(chip(page)).toBeVisible();
  await expect(composer(page)).toBeFocused();

  await composer(page).pressSequentially("draft");
  await composer(page).press("Escape");
  await expect(chip(page)).toBeHidden();
  await expect(composer(page)).toHaveValue("draft");

  await row(page, TARGET).focus();
  await page.keyboard.press("q");
  await expect(chip(page)).toBeVisible();

  await chip(page).getByRole("button", { name: "Cancel reply" }).click();
  await expect(chip(page)).toBeHidden();
  await expect(composer(page)).toBeFocused();
});

test("Q typed in the composer or a message's edit box doesn't start a reply; a focused row's does", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await expect(page.locator("[data-message-id]").first()).toBeVisible();
  await composer(page).fill("Mine to edit");
  await expect(composer(page)).toHaveValue("Mine to edit");
  await composer(page).press("Enter");

  const own = await sentRow(page, "Mine to edit");
  const anyChip = page.locator(".composer").getByRole("region", { name: /^Replying to / });

  await composer(page).pressSequentially("q Q");
  await expect(composer(page)).toHaveValue("q Q");
  await expect(anyChip).toHaveCount(0);

  await composer(page).fill("");
  await composer(page).press("ArrowUp");

  const editor = own.getByRole("textbox");

  await expect(editor).toBeFocused();
  await editor.press("End");
  await editor.pressSequentially(" q");
  await expect(editor).toHaveValue("Mine to edit q");
  await expect(anyChip).toHaveCount(0);

  await editor.press("Escape");
  await expect(editor).toBeHidden();
  await own.focus();
  await page.keyboard.press("q");
  await expect(anyChip).toBeVisible();
  await expect(composer(page)).toBeFocused();
});

/** Refuses the message creates `refuse` picks with a 422 until the returned undo runs. */
async function refuseCreates(
  page: Page,
  refuse: (body: { attachmentSignedId: string | null }) => boolean,
): Promise<() => Promise<void>> {
  const pattern = `**/api/v1/rooms/${ROOM_IDS.general}/messages`;

  await page.route(pattern, async (route) => {
    const request = route.request();

    if (request.method() === "POST" && refuse(request.postDataJSON())) {
      await route.fulfill({ status: 422, json: { error: "Not today" } });
    } else {
      await route.fallback();
    }
  });

  return () => page.unroute(pattern);
}

test("a text send that fails brings the reply back with its notify choice; Retry lands it", async ({
  page,
}) => {
  await openOn(page, TARGET);

  const bar = await hoverBar(row(page, TARGET));

  await bar.getByRole("button", { name: "Reply", exact: true }).click();
  await chip(page).getByRole("checkbox", { name: "Notify author" }).click();

  const allow = await refuseCreates(page, () => true);

  await composer(page).fill("Try this");
  await composer(page).press("Enter");

  const failed = page.locator(".message-failed");

  await expect(failed).toContainText("Couldn't send");
  await expect(chip(page)).toBeVisible();
  await expect(chip(page).getByRole("checkbox", { name: "Notify author" })).not.toBeChecked();

  await allow();

  const create = nextCreate(page);

  await failed.getByRole("button", { name: "Retry" }).click();
  expect((await create).postDataJSON()).toMatchObject({
    markdownSource: "Try this",
    replyToMessageId: TARGET,
    replyNotifyAuthor: false,
  });
  await sentRow(page, "Try this");
  await expect(chip(page)).toBeHidden();
});

test("a file send that fails brings the reply back", async ({ page }) => {
  await openOn(page, TARGET);

  const bar = await hoverBar(row(page, TARGET));

  await bar.getByRole("button", { name: "Reply", exact: true }).click();
  await page
    .locator('.composer input[type="file"]')
    .setInputFiles({ name: "notes.txt", mimeType: "text/plain", buffer: Buffer.from("notes") });
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(1);

  await refuseCreates(page, (body) => body.attachmentSignedId !== null);

  const create = nextCreate(page);

  await composer(page).press("Enter");
  expect((await create).postDataJSON()).toMatchObject({ replyToMessageId: TARGET });
  await expect(page.locator(".message-failed")).toContainText("Couldn't send");
  await expect(chip(page)).toBeVisible();
  await expect(composer(page)).toBeFocused();
});

test.describe("on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  test("a long unbroken author name ends in an ellipsis inside the chip", async ({ page }) => {
    const LONG = "Graceadeyemiwhosenamerunsonandonwithoutasinglebreakanywhere";

    // Every API body that names Grace names her the long way.
    await page.route("**/api/v1/**", async (route) => {
      const response = await route.fetch();
      const type = response.headers()["content-type"] ?? "";

      if (!type.includes("json")) {
        await route.fulfill({ response });

        return;
      }

      const body = (await response.text()).replaceAll("Grace Adeyemi", LONG);

      await route.fulfill({ response, body });
    });
    await openOn(page, TARGET);
    await longPress(row(page, TARGET).locator(".message-body").first());
    await page
      .getByRole("menu", { name: "Message actions" })
      .getByRole("menuitem", { name: "Reply", exact: true })
      .click();

    const longChip = page.locator(".composer").getByRole("region", { name: `Replying to ${LONG}` });
    const label = longChip.locator(".composer-reply-label");

    await expect(longChip).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".composer-reply");

    const fit = await label.evaluate((element) => {
      const chipBox = element.closest(".composer-reply")?.getBoundingClientRect();
      const box = element.getBoundingClientRect();

      return {
        clipped: element.scrollWidth > element.clientWidth,
        inside: chipBox !== undefined && box.right <= chipBox.right,
      };
    });

    expect(fit).toEqual({ clipped: true, inside: true });
    await expect(longChip.getByRole("button", { name: "Cancel reply" })).toBeInViewport();
    await shot(page, "composer-reply-long-name", "light");
  });

  for (const theme of ["light", "dark"] as const) {
    test(`a long press offers Reply; the quote fits and sends without notifying (${theme})`, async ({
      page,
    }) => {
      await openOn(page, TARGET, theme);
      await longPress(row(page, TARGET).locator(".message-body").first());

      const sheet = page.getByRole("menu", { name: "Message actions" });

      await sheet.getByRole("menuitem", { name: "Reply", exact: true }).click();
      await expect(sheet).toBeHidden();
      await expect(chip(page)).toBeVisible();
      await expectTouchTargets(page, ".composer-reply");
      await expectNoHorizontalOverflow(page);
      await shot(page, "composer-reply", theme);

      await chip(page).getByRole("checkbox", { name: "Notify author" }).tap();
      await expect(chip(page).getByRole("checkbox", { name: "Notify author" })).not.toBeChecked();

      await composer(page).fill("Nice one");

      const create = nextCreate(page);

      await page.getByRole("button", { name: "Send message" }).tap();
      expect((await create).postDataJSON()).toMatchObject({
        replyToMessageId: TARGET,
        replyNotifyAuthor: false,
      });

      const reply = await sentRow(page, "Nice one");

      await expect(reply.locator(".message-reply")).toContainText("Grace Adeyemi");
      await expect(chip(page)).toBeHidden();
    });
  }
});
