import type { Locator, Page } from "@playwright/test";
import { MESSAGE_IDS, seededReplyId, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  DESKTOP,
  expect,
  matrix,
  ROOM_IDS,
  shot,
  syncWelcomed,
  type Theme,
  test,
} from "./support.ts";

const GENERAL = ROOM_IDS.general;

/** Grace's launch note in #general. */
const SOURCE = MESSAGE_IDS.generalReactions;

const SOURCE_TEXT = "Shipped the new onboarding checklist to 100% of new workspaces 🎉";

/** Priya's reply in the active thread. */
const REPLY = seededReplyId(THREAD_IDS.generalActive, 1);

const REPLY_TITLE =
  "The drop is almost all between invite sent and invite accepted. Deliverability again?";

/** Theo's reply in the locked thread. */
const LOCKED_REPLY = seededReplyId(THREAD_IDS.generalLocked, 0);

const DISCONNECTED =
  "Connect your Fizzy account first: card previews and creation use your own Fizzy access.";

type Mode = "connected" | "not-connected" | "read-only" | "refused" | "reply-failed";

function row(scope: Page | Locator, messageId: number): Locator {
  return scope.locator(`[data-message-id="${messageId}"]`);
}

function dialog(page: Page): Locator {
  return page.getByRole("dialog", { name: "Create Fizzy card" });
}

function pane(page: Page): Locator {
  return page.locator("aside.right-pane");
}

/**
 * Opens `path` under /app/ with motion reduced and waits for the sync welcome, so a mock control
 * fired next can't race the reset's dropped connections.
 */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  const welcomed = syncWelcomed(page);

  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
  await welcomed;
}

/** Sets how the mock's Fizzy answers (`/__mock/fizzy`). */
async function fizzyMode(page: Page, mode: Mode): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  await page.request.post("/__mock/fizzy", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { mode },
  });
}

/** Right-clicks `target` and picks "Create Fizzy card" from its menu. */
async function createFrom(page: Page, target: Locator): Promise<Locator> {
  await expect(target).toBeVisible();
  await target.locator(".message-body").first().click({ button: "right" });

  const menu = page.getByRole("menu", { name: "Message actions" });

  await menu.getByRole("menuitem", { name: "Create Fizzy card" }).click();

  const form = dialog(page);

  await expect(form).toBeVisible();

  return form;
}

/** The form opened from the menu on Grace's note, on its permalink. */
async function openForm(page: Page): Promise<Locator> {
  await open(page, `r/${GENERAL}/m/${SOURCE}`);

  return createFrom(page, row(page, SOURCE));
}

/** Holds every create until the returned release is called. */
async function holdCreates(page: Page): Promise<() => void> {
  let release: () => void = () => undefined;

  const held = new Promise<void>((resolve) => {
    release = resolve;
  });

  await page.route("**/fizzy_cards", async (route) => {
    await held;
    await route.continue();
  });

  return release;
}

/** Posts `count` replies to a thread through the API, so its early replies fall off the newest page. */
async function postReplies(page: Page, threadId: number, count: number): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  for (let index = 0; index < count; index += 1) {
    await page.request.post(`/api/v1/threads/${threadId}/messages`, {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: {
        clientMessageId: `fizzy-filler-${index}`,
        markdownSource: `Filler reply ${index}`,
        replyToMessageId: null,
        replyNotifyAuthor: null,
        attachmentSignedId: null,
      },
    });
  }
}

async function submitTo(form: Locator, board: string): Promise<void> {
  await form.getByLabel("Board").selectOption({ label: board });
  await form.getByRole("button", { name: "Create card" }).click();
}

test.describe("Create Fizzy card", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the message menu opens the form prefilled from the message; Back doesn't reopen it", async ({
    page,
  }) => {
    const form = await openForm(page);

    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}/fizzy/new$`));
    await expect(form.getByText("Create Fizzy card in general")).toBeVisible();
    await expect(form.locator("blockquote")).toContainText(SOURCE_TEXT);
    await expect(form.locator("blockquote")).toContainText("— Grace Adeyemi");
    await expect(form.getByLabel("Board")).toBeFocused();
    await expect(form.getByLabel("Board")).toHaveValue("");
    await expect(form.getByLabel("Title")).toHaveValue(SOURCE_TEXT);
    await expect(form.getByLabel("Description")).toHaveValue(
      `${SOURCE_TEXT}\n\nSource: https://smartfire.test/rooms/${GENERAL}/@${SOURCE}`,
    );
    await expect(
      form.getByText(/^Creates the card as .+ in Smart Data, then posts a reply here/),
    ).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(form).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));

    // Closing stepped back: the form is the entry ahead, not one behind.
    await page.goForward();
    await expect(dialog(page)).toBeVisible();
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await expect(dialog(page)).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));
  });

  test("creating the card posts the reply and says so", async ({ page }) => {
    const form = await openForm(page);

    await submitTo(form, "Engineering");

    await expect(form).toBeHidden();
    await expect(page.getByText("Fizzy card #580 created.")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));
    await expect(
      page.getByRole("log", { name: "Messages" }).locator("[data-message-row]", {
        hasText: `Created from https://smartfire.test/rooms/${GENERAL}/@${SOURCE}`,
      }),
    ).toBeVisible();
  });

  test("a card created after Back left the form doesn't step back again", async ({ page }) => {
    // An entry before the room, which a second step back would land on.
    await open(page, `r/${ROOM_IDS.design}`);
    await open(page, `r/${GENERAL}/m/${SOURCE}`);

    const form = await createFrom(page, row(page, SOURCE));
    let release: () => void = () => undefined;

    const held = new Promise<void>((resolve) => {
      release = resolve;
    });

    await page.route("**/fizzy_cards", async (route) => {
      await held;
      await route.continue();
    });
    await submitTo(form, "Engineering");
    await page.goBack();
    await expect(form).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));

    release();
    await expect(page.getByText("Fizzy card #580 created.")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));
  });

  test("a create that completes after the form reopened leaves the new opening alone", async ({
    page,
  }) => {
    const form = await openForm(page);
    const release = await holdCreates(page);

    await submitTo(form, "Engineering");
    await page.goBack();
    await expect(form).toBeHidden();

    const again = await createFrom(page, row(page, SOURCE));

    release();
    await expect(page.getByText("Fizzy card #580 created.")).toBeVisible();
    await expect(again).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}/fizzy/new$`));
  });

  test("a create that completes after leaving the room doesn't step back", async ({ page }) => {
    await open(page, `r/${ROOM_IDS.design}`);
    await page.locator(".sidebar").getByText("general", { exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}$`));

    const last = page.getByRole("log", { name: "Messages" }).locator("[data-message-row]").last();
    const form = await createFrom(page, last);
    const release = await holdCreates(page);

    await submitTo(form, "Engineering");
    // Straight past the room, so the form unmounts while it's still open.
    await page.evaluate(() => window.history.go(-2));
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.design}$`));
    await expect(form).toBeHidden();

    release();
    await expect(page.getByText("Fizzy card #580 created.")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.design}$`));
  });

  test("a board and a title are required", async ({ page }) => {
    const form = await openForm(page);

    await form.getByLabel("Title").fill("   ");
    await form.getByRole("button", { name: "Create card" }).click();

    await expect(
      form.getByText("Choose a board and enter a title.", { exact: true }),
    ).toBeVisible();
    await expect(form.getByText("Choose a board.", { exact: true })).toBeVisible();
    await expect(form.getByText("Enter a title.", { exact: true })).toBeVisible();
    await expect(form.getByLabel("Board")).toBeFocused();

    await form.getByLabel("Board").selectOption({ label: "Support" });
    await expect(form.getByText("Choose a board.", { exact: true })).toHaveCount(0);
  });

  test("without a connection it shows the source and the way to connect", async ({ page }) => {
    await open(page, `r/${GENERAL}/m/${SOURCE}`);
    await fizzyMode(page, "not-connected");

    const form = await createFrom(page, row(page, SOURCE));

    await expect(form.locator("blockquote")).toContainText(SOURCE_TEXT);
    await expect(form.getByText(DISCONNECTED)).toBeVisible();
    await expect(form.getByLabel("Board")).toHaveCount(0);
    await expect(form.getByRole("button", { name: "Create card" })).toHaveCount(0);

    await form.getByRole("link", { name: "Connect Fizzy on your profile" }).click();
    await expect(page).toHaveURL(/\/app\/settings\/integrations#integration-fizzy$/);
  });

  test("a read-only token or a refusal keeps the form with Fizzy's reason", async ({ page }) => {
    await open(page, `r/${GENERAL}/m/${SOURCE}`);
    await fizzyMode(page, "read-only");

    const form = await createFrom(page, row(page, SOURCE));

    await submitTo(form, "Engineering");
    await expect(form.getByRole("alert")).toContainText(
      "That Fizzy token is read-only. Generate a Read + Write token to create cards.",
    );
    await expect(form.getByLabel("Title")).toHaveValue(SOURCE_TEXT);

    await fizzyMode(page, "refused");
    await form.getByRole("button", { name: "Create card" }).click();
    await expect(form.getByRole("alert")).toContainText(
      "Fizzy refused the new card (Fizzy refused: Board is unavailable).",
    );
    await expect(form.getByRole("button", { name: "Create card" })).toBeEnabled();
  });

  test("a card whose reply failed is linked, and can't be created again", async ({ page }) => {
    await open(page, `r/${GENERAL}/m/${SOURCE}`);
    await fizzyMode(page, "reply-failed");

    const form = await createFrom(page, row(page, SOURCE));

    await submitTo(form, "Support");
    await expect(
      form.getByText(
        "Fizzy card #580 created, but the reply could not be posted (Body is too long).",
      ),
    ).toBeVisible();
    await expect(form.getByRole("link", { name: /Open Fizzy card #580/ })).toHaveAttribute(
      "href",
      "https://fizzy.test/897362094/cards/580",
    );
    await expect(form.getByRole("button", { name: "Create card" })).toHaveCount(0);
    await expect(form.getByLabel("Title")).toBeDisabled();

    await form.locator(".dialog-footer").getByRole("button", { name: "Close" }).click();
    await expect(form).toBeHidden();
  });

  test("a thread reply is quoted from the thread, which stays open beneath", async ({ page }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`);

    const form = await createFrom(page, row(pane(page), REPLY));

    await expect(page).toHaveURL(
      new RegExp(`/app/r/${GENERAL}/t/${THREAD_IDS.generalActive}/m/${REPLY}/fizzy/new$`),
    );
    await expect(form.locator("blockquote")).toContainText("— Priya Raman");
    await expect(form.getByLabel("Title")).toHaveValue(REPLY_TITLE);
    await expect(form.getByLabel("Description")).toHaveValue(
      new RegExp(
        `Source: https://smartfire\\.test/rooms/${GENERAL}\\?thread=${THREAD_IDS.generalActive}&message_id=${REPLY}$`,
      ),
    );

    await submitTo(form, "Roadmap");
    await expect(form).toBeHidden();
    await expect(page.getByText("Fizzy card #580 created.")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/t/${THREAD_IDS.generalActive}$`));
    await expect(
      pane(page).locator("[data-message-row]", { hasText: "Created from https://smartfire.test" }),
    ).toBeVisible();
  });

  test("its URL opens the form over the room; closing lands on the message", async ({ page }) => {
    await open(page, `r/${GENERAL}/m/${SOURCE}/fizzy/new`);

    const form = dialog(page);

    await expect(form.getByLabel("Board")).toBeFocused();
    await expect(form.getByLabel("Title")).toHaveValue(SOURCE_TEXT);
    await form.getByRole("button", { name: "Cancel" }).click();

    await expect(form).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${GENERAL}/m/${SOURCE}$`));
    await expect(row(page, SOURCE)).toBeVisible();
    await expect(row(page, SOURCE)).toBeFocused();
  });

  test("a direct entry on an older thread reply closes onto that reply", async ({ page }) => {
    const oldest = seededReplyId(THREAD_IDS.generalActive, 0);

    await open(page, `r/${GENERAL}`);
    await postReplies(page, THREAD_IDS.generalActive, 45);
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}/m/${oldest}/fizzy/new`);

    const form = dialog(page);

    await expect(form.getByLabel("Board")).toBeFocused();
    await form.getByRole("button", { name: "Cancel" }).click();
    await expect(form).toBeHidden();
    await expect(page).toHaveURL(
      new RegExp(`/app/r/${GENERAL}/t/${THREAD_IDS.generalActive}\\?m=${oldest}$`),
    );
    await expect(row(pane(page), oldest)).toBeVisible();
    await expect(row(pane(page), oldest)).toBeFocused();

    // The viewer moves on with the keyboard, and focus stays where they put it.
    const focusedMessage = () =>
      page.evaluate(
        () =>
          document.activeElement?.closest("[data-message-id]")?.getAttribute("data-message-id") ??
          null,
      );

    await page.keyboard.press("ArrowDown");
    await expect.poll(focusedMessage).not.toBe(String(oldest));

    const moved = await focusedMessage();

    expect(moved).not.toBeNull();
    await page.waitForTimeout(1000);
    expect(await focusedMessage()).toBe(moved);
  });

  test("a locked thread's reply opens the form, and creating says the thread is locked", async ({
    page,
  }) => {
    await open(page, `r/${GENERAL}/t/${THREAD_IDS.generalLocked}/m/${LOCKED_REPLY}/fizzy/new`);

    const form = dialog(page);

    await expect(form.getByLabel("Title")).toHaveValue("Can we keep this to the agreed format?");
    await expect(pane(page)).toBeVisible();
    await submitTo(form, "Engineering");
    await expect(form.getByRole("alert")).toContainText("This thread is locked");

    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(
      new RegExp(`/app/r/${GENERAL}/t/${THREAD_IDS.generalLocked}\\?m=${LOCKED_REPLY}$`),
    );
  });
});

matrix("the Create Fizzy card form", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}/m/${SOURCE}/fizzy/new`, theme);
  await expect(dialog(page).getByLabel("Title")).toHaveValue(SOURCE_TEXT);
  await shot(page, "fizzy-card-form", theme);

  await dialog(page).getByRole("button", { name: "Create card" }).click();
  await expect(dialog(page).getByText("Choose a board.", { exact: true })).toBeVisible();
  await shot(page, "fizzy-card-form-invalid", theme);

  await fizzyMode(page, "not-connected");
  await page.goto(`/app/r/${GENERAL}/m/${SOURCE}/fizzy/new`);
  await expect(dialog(page).getByText(DISCONNECTED)).toBeVisible();
  await shot(page, "fizzy-card-disconnected", theme);
});
