import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { CARD_IDS } from "../../mock/s3/cards.ts";
import { expect, matrix, shot, type Theme, test, USER_IDS } from "./support.ts";

const ROOM = CARD_IDS.room;

const { messages, polls } = CARD_IDS;

/**
 * Opens #product-updates (the cards room) at a message's permalink, so its row is mounted and
 * centred, with motion reduced; waits for the room, as a phone hides the sidebar there.
 */
async function openAt(page: Page, messageId: number, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM}/m/${messageId}`);
  await page.getByRole("main").waitFor();
  await expect(row(page, messageId)).toBeVisible();
}

function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-row][data-message-id="${messageId}"]`).first();
}

/** Waits for every finite animation to finish, so a shot isn't caught mid-fade. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .flatMap((animation) =>
          animation.effect?.getComputedTiming().iterations === Infinity
            ? []
            : [animation.finished.catch(() => animation)],
        ),
    ),
  );
}

/** Drives the cards mock's `/__mock/cards` control (another member votes, a poll closes...). */
async function control(
  request: APIRequestContext,
  data: Readonly<Record<string, number | string | readonly number[]>>,
) {
  const state = await (await request.get("/__mock/state")).json();

  return request.post("/__mock/cards", { headers: { "X-CSRF-Token": state.csrfToken }, data });
}

matrix("pull request cards: open, merged, draft, loading, failed", async ({ page, theme }) => {
  await openAt(page, messages.githubOpen, theme);

  const open = row(page, messages.githubOpen);

  await expect(
    open.getByRole("link", { name: "Rate limit the sync endpoint with a token bucket" }),
  ).toBeVisible();
  await expect(open.getByText("Open", { exact: true })).toBeVisible();
  await expect(open.getByText("Approved")).toBeVisible();
  await expect(open.getByText("Checks passing")).toBeVisible();
  await expect(open.getByRole("link", { name: "Discuss" })).toBeVisible();

  await expect(
    row(page, messages.drive).getByRole("link", { name: /Google Drive file/ }),
  ).toBeVisible();

  const merged = row(page, messages.githubMerged);

  await expect(merged.getByText("Merged", { exact: true })).toBeVisible();
  await settle(page);
  await shot(page, "cards-github", theme);

  await openAt(page, messages.githubDraftAndLoading, theme);

  const pair = row(page, messages.githubDraftAndLoading);

  await expect(pair.getByText("Draft", { exact: true })).toBeVisible();
  await expect(pair.locator('.github-card[aria-busy="true"]')).toHaveCount(1);

  const failed = row(page, messages.githubFailedAndHidden);

  await expect(failed.getByText(/Couldn't load this pull request/)).toBeVisible();
  await expect(failed.getByRole("button", { name: "Retry" })).toBeVisible();
  // The private repository the viewer can't read shows no card at all.
  await expect(failed.locator(".github-card")).toHaveCount(1);
  await settle(page);
  await shot(page, "cards-github-states", theme);
});

matrix("posts on X, Drive files and events", async ({ page, theme }) => {
  await openAt(page, messages.xPost, theme);

  const post = row(page, messages.xPost);

  await expect(post.getByRole("region", { name: /^Post by / })).toBeVisible();
  await expect(post.locator(".x-media img").first()).toBeVisible();
  await expect(post.locator(".x-quote")).toBeVisible();
  await settle(page);
  await shot(page, "cards-x-drive", theme);

  await openAt(page, messages.eventRecurring, theme);

  const event = row(page, messages.eventRecurring);

  await expect(event.getByRole("region", { name: "Event: Weekly product sync" })).toBeVisible();
  await expect(event.getByRole("button", { name: /^Going/ })).toHaveAttribute(
    "aria-pressed",
    "false",
  );
  await expect(event.getByText("Repeats")).toBeVisible();
  await expect(event.getByRole("link", { name: "Join with Google Meet" })).toBeVisible();

  const cancelled = row(page, messages.eventCancelled);

  await expect(cancelled.getByText("Cancelled", { exact: true })).toBeVisible();
  await expect(cancelled.getByRole("button", { name: /^Going/ })).toHaveCount(0);
  await settle(page);
  await shot(page, "cards-events", theme);
});

matrix("answering an event, for every future occurrence", async ({ page, theme }) => {
  await openAt(page, messages.eventRecurring, theme);

  const event = row(page, messages.eventRecurring);
  const going = event.getByRole("button", { name: /^Going/ });

  await expect(going).toContainText("2");
  await event.getByText("Apply to all future occurrences").click();
  await going.click();
  await expect(going).toHaveAttribute("aria-pressed", "true");
  await expect(going).toContainText("3");

  const maybe = event.getByRole("button", { name: /^Maybe/ });

  await maybe.click();
  await expect(maybe).toHaveAttribute("aria-pressed", "true");
  await expect(going).toContainText("2");
  await settle(page);
  await shot(page, "cards-event-answered", theme);
});

matrix("Fizzy cards, quotes, LinkedIn and link previews", async ({ page, theme }) => {
  await openAt(page, messages.fizzyLoaded, theme);

  const fizzy = row(page, messages.fizzyLoaded);

  await expect(fizzy.getByRole("region", { name: /^Fizzy card: / })).toBeVisible();
  await expect(
    row(page, messages.fizzyNotConnectedAndNotFound).getByText(/Connect Fizzy/),
  ).toBeVisible();
  await expect(
    row(page, messages.fizzyFailedAndLoading).getByRole("button", { name: "Retry" }),
  ).toBeVisible();
  await settle(page);
  await shot(page, "cards-fizzy", theme);

  await openAt(page, messages.quoteFetched, theme);
  await expect(
    row(page, messages.quoteInline).getByRole("link", { name: /^Quoted message from / }),
  ).toBeVisible();
  await expect(
    row(page, messages.quoteFetched).getByRole("link", { name: /^Quoted message from / }),
  ).toHaveCount(1);
  await settle(page);
  await shot(page, "cards-quotes", theme);

  await openAt(page, messages.linkImage, theme);
  await expect(
    row(page, messages.linkedin).getByRole("button", { name: "Show embedded post" }),
  ).toBeVisible();
  await expect(
    row(page, messages.linkedinChip).getByRole("link", { name: /View post on LinkedIn/ }),
  ).toBeVisible();
  await expect(row(page, messages.linkImage).locator(".link-card")).toBeVisible();
  await expect(row(page, messages.linkPlain).locator(".link-card")).toBeVisible();
  await settle(page);
  await shot(page, "cards-links", theme);
});

matrix("polls: closed, multiple choice and anonymous", async ({ page, theme }) => {
  await openAt(page, messages.pollMultiple, theme);
  await expect(
    row(page, messages.pollClosed).getByText("Final results", { exact: false }),
  ).toBeVisible();

  const multiple = row(page, messages.pollMultiple);

  await expect(multiple.getByText("Multiple choice")).toBeVisible();
  await expect(multiple.getByRole("button", { name: "Change vote" })).toBeVisible();

  const anonymous = row(page, messages.pollAnonymous);

  await expect(anonymous.getByText(/Anonymous/)).toBeVisible();
  await expect(anonymous.getByText(/Closes in/)).toBeVisible();
  await expect(anonymous.locator(".poll-voters")).toHaveCount(0);
  await settle(page);
  await shot(page, "cards-polls", theme);
});

matrix("voting in a poll, changing it and taking it back", async ({ page, theme }) => {
  await openAt(page, messages.pollOpen, theme);

  const poll = row(page, messages.pollOpen);

  await expect(poll.getByText("4 votes")).toBeVisible();
  await settle(page);
  await shot(page, "cards-poll-before", theme);

  await poll.getByRole("button", { name: "Tacos" }).click();
  await expect(poll.getByText("5 votes")).toBeVisible();
  await expect(poll.getByText("(your vote)")).toHaveCount(1);
  await settle(page);
  await shot(page, "cards-poll-after", theme);

  await poll.getByRole("button", { name: "Change vote" }).click();
  await poll.getByRole("button", { name: "Pizza" }).click();
  await expect(poll.locator(".poll-result[data-mine] .poll-result-text")).toHaveText("Pizza");
  await expect(poll.getByText("5 votes")).toBeVisible();

  await poll.getByRole("button", { name: "Retract" }).click();
  await expect(poll.getByText("4 votes")).toBeVisible();
  await expect(poll.getByRole("button", { name: "Tacos" })).toBeVisible();
});

matrix("creating a poll from the + menu", async ({ page, theme }) => {
  await openAt(page, messages.pollOpen, theme);
  await page.getByRole("textbox", { name: /^Message #/ }).click();
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "Create a poll" }).click();

  const dialog = page.getByRole("dialog", { name: "Create a poll" });

  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Post poll" }).click();
  await expect(dialog.getByText("Ask a question.")).toBeVisible();
  await dialog.getByLabel("Question").fill("Where should the offsite be?");
  await dialog.getByRole("textbox", { name: "Option 1" }).fill("Columbus");
  await dialog.getByRole("textbox", { name: "Option 2" }).fill("Cleveland");
  await dialog.getByRole("button", { name: "Add option" }).click();
  await dialog.getByRole("textbox", { name: "Option 3" }).fill("Cincinnati");
  await dialog.getByText("Allow multiple choices").click();
  await dialog.getByRole("button", { name: "In 1 day" }).click();
  await settle(page);
  await shot(page, "cards-create-poll", theme);

  await dialog.getByRole("button", { name: "Post poll" }).click();
  await expect(dialog).toBeHidden();

  const created = page.locator(".poll-card").filter({ hasText: "Cincinnati" });

  await expect(created).toBeVisible();
  await expect(created.getByText("Multiple choice")).toBeVisible();
  await expect(created.getByText(/^Closes /)).toBeVisible();
});

test("/poll opens the dialog with the question filled in", async ({ page }) => {
  await openAt(page, messages.pollOpen, "light");

  const composer = page.getByRole("textbox", { name: /^Message #/ });

  await composer.fill("/poll Pizza or tacos?");
  await composer.press("Enter");

  const dialog = page.getByRole("dialog", { name: "Create a poll" });

  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Question")).toHaveValue("Pizza or tacos?");
});

test("another member's vote and a closing poll arrive live", async ({ page, request }) => {
  await openAt(page, messages.pollOpen, "light");

  const poll = row(page, messages.pollOpen);

  await expect(poll.getByText("4 votes")).toBeVisible();
  await control(request, {
    op: "vote",
    pollId: polls.open,
    userId: USER_IDS.theo,
    optionIds: [polls.open * 10 + 1],
  });
  await expect(poll.getByText("5 votes")).toBeVisible();
  await control(request, { op: "close-poll", pollId: polls.open });
  await expect(poll.getByText("Closed", { exact: true })).toBeVisible();
  await expect(poll.getByRole("button", { name: "Tacos" })).toHaveCount(0);
});

test("unknown kinds and suppressed previews render nothing", async ({ page }) => {
  await openAt(page, messages.unknownKind, "light");
  await expect(row(page, messages.unknownKind).locator(".card")).toHaveCount(0);

  const suppressed = row(page, messages.suppressed);

  // The pull request still shows; the link preview the author hid doesn't.
  await expect(suppressed.locator(".github-card")).toHaveCount(1);
  await expect(suppressed.locator(".link-card")).toHaveCount(0);
});

matrix("a pull request's discussion thread lists its files", async ({ page, theme }) => {
  await openAt(page, messages.githubOpen, theme);
  await row(page, messages.githubOpen).getByRole("link", { name: "Discuss" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM}/t/new\\?parent=${messages.githubOpen}`));

  const reply = page.locator("aside.right-pane").getByRole("textbox", { name: "Reply…" });

  await reply.fill("Looking at the limiter now");
  await reply.press("Enter");
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM}/t/\\d+$`));

  const header = page.locator("aside.right-pane .github-card");

  await expect(header.getByText("4 files changed")).toBeVisible();
  await expect(header.getByRole("link", { name: "Discuss" })).toHaveCount(0);
  await settle(page);
  await shot(page, "cards-github-thread", theme);
});
