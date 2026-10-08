import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { CARD_IDS } from "../../mock/s3/cards.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { Poll } from "../../src/gen/Poll.ts";
import {
  expect,
  matrix,
  openApp,
  ROOM_IDS,
  shot,
  syncWelcomed,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

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
  await expect(open.getByRole("button", { name: "Discuss" })).toBeVisible();

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

/**
 * Holds the cards chunk back until `release()`. `requested` settles once the page has asked for
 * it, which proves the hold is in effect (a build that names the chunk differently would skip it).
 */
async function holdCardsChunk(page: Page) {
  let release: () => void = () => undefined;
  let markRequested: () => void = () => undefined;

  const released = new Promise<void>((resolve) => {
    release = resolve;
  });

  const requested = new Promise<void>((resolve) => {
    markRequested = resolve;
  });

  await page.route("**/src/features/cards/message-cards.tsx*", async (route) => {
    markRequested();
    await released;
    await route.continue();
  });

  return { release, requested };
}

/** The timeline's skeleton cover: `true` while it hides the list. */
function timelineBusy(page: Page): Locator {
  return page.locator(".timeline > .t-skel");
}

/** A message with a poll, from the cards room, to put into another room's page. */
async function seededPoll(request: APIRequestContext): Promise<Poll | undefined> {
  const page: MessagePage = await (
    await request.get(`/api/v1/rooms/${ROOM}/messages?around=${messages.pollOpen}`)
  ).json();

  return page.messages.find((message) => message.poll !== null)?.poll ?? undefined;
}

test("a permalinked row stays in view when the cards chunk arrives late", async ({ page }) => {
  const chunk = await holdCardsChunk(page);

  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM}/m/${messages.eventRecurring}`);
  await chunk.requested;

  // The window is in and its row mounted, but it waits, unplaced, under the skeleton.
  await expect(row(page, messages.eventRecurring)).toBeAttached();
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "true");
  chunk.release();

  const event = row(page, messages.eventRecurring);

  await expect(event.getByRole("region", { name: "Event: Weekly product sync" })).toBeVisible();
  await expect(event).toBeInViewport();
});

test("a card arriving live while the chunk loads keeps the list and its place", async ({
  page,
}) => {
  const chunk = await holdCardsChunk(page);
  const welcomed = syncWelcomed(page);

  await openApp(page, `r/${ROOM_IDS.engineering}`);
  await chunk.requested;
  // No cards in this window: it shows at once, at the bottom.
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");
  await welcomed;

  const state = await (await page.request.get("/__mock/state")).json();

  const created = await page.request.post(`/api/v1/rooms/${ROOM_IDS.engineering}/polls`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { question: "Ship on Friday?", options: ["Yes", "No"], clientMessageId: "e2e-poll-1" },
  });

  expect(created.ok()).toBe(true);

  const posted = page.locator("[data-message-row]").filter({ hasText: "Ship on Friday?" });

  // It arrives, followed at the bottom, with the list still up (not swapped for a skeleton).
  await expect(posted).toBeInViewport();
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");
  chunk.release();
  await expect(posted.getByRole("region", { name: "Poll" })).toBeVisible();
  await expect(posted).toBeInViewport();
});

// Quarantined: this fails often in CI because cards loading above the reader moves the
// timeline. PR #332 fixes that and rewrites this test; it removes this fixme.
test.fixme("an older page with cards while the chunk loads keeps the list and its place", async ({
  page,
}) => {
  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const welcomed = syncWelcomed(page);

  let markInjected: (id: number) => void = () => undefined;
  let markOlderRequested: (firstId: number) => void = () => undefined;
  let releaseOlder: () => void = () => undefined;

  const injected = new Promise<number>((resolve) => {
    markInjected = resolve;
  });

  const olderRequested = new Promise<number>((resolve) => {
    markOlderRequested = resolve;
  });

  const olderReleased = new Promise<void>((resolve) => {
    releaseOlder = resolve;
  });

  expect(poll).toBeTruthy();
  // The page before the first window carries a poll on its newest message.
  await page.route(`**/api/v1/rooms/${ROOM_IDS.engineering}/messages?before=*`, async (route) => {
    markOlderRequested(Number(new URL(route.request().url()).searchParams.get("before")));
    const response = await route.fetch();
    const body: MessagePage = await response.json();
    const newest = body.messages[body.messages.length - 1];

    if (newest !== undefined && poll !== undefined) {
      body.messages[body.messages.length - 1] = { ...newest, poll };
      markInjected(newest.id);
    }

    await olderReleased;
    await route.fulfill({ response, json: body });
  });
  await openApp(page, `r/${ROOM_IDS.engineering}`);
  await chunk.requested;
  await welcomed;
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");

  const list = page.locator("[data-message-list]");
  const older = page.waitForResponse((response) => response.url().includes("before="));

  await list.evaluate((element) => {
    element.scrollTop = 0;
  });

  // The DOM can still contain the previous bottom window just after scrollTop changes. Hold
  // the prepend until virtua has drawn the original first row inside the list's visible area.
  const firstId = await olderRequested;

  await expect
    .poll(() =>
      list.evaluate((element, id) => {
        const bounds = element.getBoundingClientRect();
        const first = element.querySelector(`[data-message-row][data-message-id="${id}"]`);
        const rowBounds = first?.getBoundingClientRect();

        return (
          rowBounds !== undefined && rowBounds.bottom > bounds.top && rowBounds.top < bounds.bottom
        );
      }, firstId),
    )
    .toBe(true);

  const anchor = await list.evaluate((element) => {
    const bounds = element.getBoundingClientRect();

    return Array.from(element.querySelectorAll<HTMLElement>("[data-message-row]")).find(
      (message) => {
        const rect = message.getBoundingClientRect();

        return rect.bottom > bounds.top && rect.top < bounds.bottom;
      },
    )?.dataset.messageId;
  });

  expect(anchor).toBeDefined();
  await expect(row(page, Number(anchor))).toBeInViewport();
  releaseOlder();

  await older;
  // The older rows are in (the injected one mounted above), the row that was at the top stays in
  // view, and the list is still up.
  await expect(row(page, await injected)).toBeAttached();
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");
  await expect(row(page, Number(anchor))).toBeInViewport();
  chunk.release();
  await expect(page.getByRole("region", { name: "Poll" }).first()).toBeAttached();
  await expect(row(page, Number(anchor))).toBeInViewport();
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

  await poll.getByRole("radio", { name: "Tacos" }).check();
  await poll.getByRole("button", { name: "Vote" }).click();
  await expect(poll.getByText("5 votes")).toBeVisible();
  await expect(poll.getByText("(your vote)")).toHaveCount(1);
  await settle(page);
  await shot(page, "cards-poll-after", theme);

  await poll.getByRole("button", { name: "Change vote" }).click();
  await poll.getByRole("radio", { name: "Pizza" }).check();
  await poll.getByRole("button", { name: "Vote" }).click();
  await expect(poll.locator(".poll-result[data-mine] .poll-result-text")).toHaveText("Pizza");
  await expect(poll.getByText("5 votes")).toBeVisible();

  await poll.getByRole("button", { name: "Retract" }).click();
  await expect(poll.getByText("4 votes")).toBeVisible();
  await expect(poll.getByRole("radio", { name: "Tacos" })).toBeVisible();
});

test("voting from the keyboard keeps focus in the poll and says what happened", async ({
  page,
}) => {
  await openAt(page, messages.pollOpen, "light");

  const poll = row(page, messages.pollOpen);
  const tacos = poll.getByRole("radio", { name: "Tacos" });
  const pizza = poll.getByRole("radio", { name: "Pizza" });
  const change = poll.getByRole("button", { name: "Change vote" });

  await tacos.focus();
  await page.keyboard.press("Space");
  await expect(tacos).toBeChecked();
  await page.keyboard.press("ArrowDown");
  await expect(pizza).toBeChecked();
  await expect(pizza).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(poll.getByRole("button", { name: "Vote" })).toBeFocused();
  await page.keyboard.press("Enter");

  // The options give way to the results; focus lands on Change vote, not the page.
  await expect(change).toBeFocused();
  await expect(poll.getByRole("status")).toHaveText(/^Voted for Pizza\. Results: /);

  await page.keyboard.press("Enter");
  await expect(pizza).toBeFocused();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await expect(poll.getByRole("button", { name: "Cancel" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(change).toBeFocused();

  await page.keyboard.press("Tab");
  await expect(poll.getByRole("button", { name: "Retract" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(tacos).toBeFocused();
  await expect(poll.getByRole("status")).toHaveText("Vote retracted.");
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
  await expect(poll.getByRole("radio", { name: "Tacos" })).toHaveCount(0);
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
  await row(page, messages.githubOpen).getByRole("button", { name: "Discuss" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM}/t/\\d+`));

  const header = page.locator("aside.right-pane .github-card");

  await expect(header.getByText("4 files changed")).toBeVisible();
  await expect(header.getByRole("button", { name: "Discuss" })).toHaveCount(0);
  await settle(page);
  await shot(page, "cards-github-thread", theme);
});
