import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { MESSAGE_IDS, THREAD_IDS } from "../../mock/s2/seed.ts";
import { CARD_IDS } from "../../mock/s3/cards.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { Poll } from "../../src/gen/Poll.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import {
  expect,
  holdSync,
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

/** Both room pages and thread detail put the parent into the shared message store. */
async function mockThreadParent(
  page: Page,
  { threadId, bodyHtml, poll }: { threadId: number; bodyHtml: string; poll: Poll | undefined },
) {
  const withParent = (message: MessagePage["messages"][number]) => ({
    ...message,
    bodyHtml,
    poll: poll ?? message.poll,
  });

  await page.route(`**/api/v1/threads/${threadId}`, async (route) => {
    const response = await route.fetch();
    const body: ThreadDetail = await response.json();

    expect(body.parentMessage).not.toBeNull();

    if (body.parentMessage !== null) body.parentMessage = withParent(body.parentMessage);

    await route.fulfill({ response, json: body });
  });
  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    body.messages = body.messages.map((message) =>
      message.thread?.threadId === threadId ? withParent(message) : message,
    );
    await route.fulfill({ response, json: body });
  });
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

for (const placement of ["permalink", "unread divider"] as const) {
  test(`a near-end ${placement} in a card-free window stays in view while the chunk loads`, async ({
    page,
  }) => {
    await holdSync(page);

    const chunk = await holdCardsChunk(page);
    const messageId = MESSAGE_IDS.engineeringCode - 1;

    // The estimated heights initially clamp placement to the end. Measuring the tall row
    // below the target must let Virtua finish centring it (or placing the unread divider).
    await page.route(`**/api/v1/rooms/${ROOM_IDS.engineering}/messages**`, async (route) => {
      const response = await route.fetch();
      const body: MessagePage = await response.json();

      expect(
        body.messages.every((message) => message.poll === null && message.cards.length === 0),
      ).toBe(true);
      body.messages = body.messages.map((message) =>
        message.id === MESSAGE_IDS.engineeringCode
          ? {
              ...message,
              bodyHtml: Array.from({ length: 40 }, () => "<p>A tall card-free message</p>").join(
                "",
              ),
              attachment: null,
            }
          : message,
      );
      await route.fulfill({ response, json: body });
    });

    if (placement === "unread divider") {
      await page.route(`**/api/v1/rooms/${ROOM_IDS.engineering}`, async (route) => {
        const response = await route.fetch();
        const body: RoomDetail = await response.json();

        body.unread = { firstUnreadMessageId: messageId, count: 6 };
        await route.fulfill({ response, json: body });
      });
    }

    await openApp(
      page,
      `r/${ROOM_IDS.engineering}${placement === "permalink" ? `/m/${messageId}` : ""}`,
    );
    await chunk.requested;
    await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");

    const list = page.locator("[data-message-list]");

    const target =
      placement === "permalink" ? row(page, messageId) : list.locator(".unread-divider");

    const targetTop = () =>
      target.evaluate(async (element) => {
        const list = element.closest("[data-message-list]");

        if (list === null) throw new Error("The placement target is outside the message list");

        const measure = () =>
          element.getBoundingClientRect().top - list.getBoundingClientRect().top;

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        const before = measure();

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

        return {
          before,
          after: measure(),
          settled: list.getAttribute("data-scroll-settled") === "true",
        };
      });

    await expect(list).toHaveAttribute("data-scroll-settled", "true");
    await expect(target).toBeInViewport();

    let top = 0;

    await expect
      .poll(async () => {
        const position = await targetTop();

        top = position.after;

        return position.settled ? Math.abs(position.after - position.before) : Infinity;
      })
      .toBeLessThanOrEqual(1);

    const loaded = page.waitForResponse((response) =>
      response.url().includes("/src/features/cards/message-cards.tsx"),
    );

    chunk.release();

    const response = await loaded;

    await response.finished();
    await page.evaluate(async (path) => {
      await import(path);
    }, response.url());
    await expect(target).toBeInViewport();
    await expect
      .poll(async () => {
        const position = await targetTop();

        return position.settled
          ? Math.max(
              Math.abs(position.before - top),
              Math.abs(position.after - top),
              Math.abs(position.after - position.before),
            )
          : Infinity;
      })
      .toBeLessThanOrEqual(3);
  });
}

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

/** Keep real row measurements changing until input, or until that row is removed. */
async function keepPlacementMeasuring(page: Page, messageId: number | null = null) {
  await page.addInitScript((messageId) => {
    let body: HTMLElement | null = null;
    let frame = 0;
    let count = 0;

    const stop = (event: Event) => {
      if (!(event.target instanceof Element) || !event.target.closest('[role="log"]')) return;

      cancelAnimationFrame(frame);

      if (body) body.style.paddingBottom = "";
    };

    for (const input of ["wheel", "touchstart", "keydown", "pointerdown"])
      document.addEventListener(input, stop, { capture: true, passive: true });

    const measure = () => {
      if (body && !body.isConnected) {
        if (messageId !== null) return;

        body = null;
      }

      if (body === null && messageId === null) {
        const rows = document.querySelectorAll("[data-message-list] [data-message-row]");

        body = rows.item(rows.length - 1)?.querySelector<HTMLElement>(".message-body") ?? null;
      } else {
        body ??= document.querySelector<HTMLElement>(
          `[data-message-row][data-message-id="${messageId}"] .message-body`,
        );
      }

      if (body) {
        body.style.paddingBottom = `${(count++ % 2) * 4}px`;
        const list = body.closest<HTMLElement>('[role="log"]');

        if (list) list.dataset.measurementPending = "true";
      }

      frame = requestAnimationFrame(measure);
    };

    frame = requestAnimationFrame(measure);
  }, messageId);
}

type ReaderInput = "wheel" | "PageUp" | "ArrowUp" | "touch" | "scrollbar";

async function olderCardsScenario(page: Page, input: ReaderInput | null) {
  // Keep the welcome's refetch out of this pagination scenario.
  await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);

  if (input !== null) await keepPlacementMeasuring(page);

  let markInjected: (id: number) => void = () => undefined;

  const injected = new Promise<number>((resolve) => {
    markInjected = resolve;
  });

  const firstPage = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/rooms/${ROOM_IDS.engineering}/messages`),
  );

  let releaseOlder: () => void = () => undefined;

  const releasedOlder = new Promise<void>((resolve) => {
    releaseOlder = resolve;
  });

  expect(poll).toBeTruthy();
  // The page before the first window carries a poll on its newest message.
  await page.route(`**/api/v1/rooms/${ROOM_IDS.engineering}/messages?before=*`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();
    const newest = body.messages[body.messages.length - 1];

    if (newest !== undefined && poll !== undefined) {
      body.messages[body.messages.length - 1] = { ...newest, poll };
      markInjected(newest.id);
    }

    await releasedOlder;
    await route.fulfill({ response, json: body });
  });
  await openApp(page, `r/${ROOM_IDS.engineering}`);
  await chunk.requested;
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");

  const body: MessagePage = await (await firstPage).json();
  // The first mounted row changes as the virtual list scrolls; anchor the loaded window's start.
  const anchor = body.before;

  expect(anchor).not.toBeNull();

  const list = page.locator("[data-message-list]");
  const anchorRow = row(page, Number(anchor));

  const anchorTop = () =>
    anchorRow.evaluate(async (element) => {
      const list = element.closest("[data-message-list]");

      if (list === null) throw new Error("The anchor is outside the message list");

      const measure = () => element.getBoundingClientRect().top - list.getBoundingClientRect().top;

      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const before = measure();

      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

      return {
        before,
        after: measure(),
        settled: list.getAttribute("data-scroll-settled") === "true",
      };
    });

  // Virtua's scroll end follows the placement measurements and their scroll event.
  if (input !== null) {
    await expect(list).toHaveAttribute("data-measurement-pending", "true");
    await expect.poll(() => list.evaluate((element) => element.scrollTop > 0)).toBe(true);
    await expect(list).toHaveAttribute("data-placement-settled", "false");

    if (input === "wheel") {
      await list.hover();
      await page.mouse.wheel(0, -10_000);
    } else {
      if (input === "PageUp" || input === "ArrowUp") await list.press(input);
      else if (input === "touch") await list.dispatchEvent("touchstart");
      else await list.dispatchEvent("pointerdown", { pointerType: "mouse" });

      await list.evaluate(async (element) => {
        if (element.scrollTop === 0) return;

        await new Promise<void>((resolve) => {
          element.addEventListener("scroll", () => resolve(), { once: true });
          element.scrollTop = 0;
        });
      });

      if (input === "touch") await list.dispatchEvent("touchend");
    }
  } else {
    await expect(list).toHaveAttribute("data-scroll-settled", "true");
    await list.evaluate(
      (element) =>
        new Promise<void>((resolve) => {
          element.addEventListener("scroll", () => resolve(), { once: true });
          element.scrollTop = 0;
        }),
    );
  }

  await expect(anchorRow).toBeInViewport();

  let top = 0;

  await expect
    .poll(async () => {
      const position = await anchorTop();

      top = position.after;

      return position.settled ? Math.abs(position.after - position.before) : Infinity;
    })
    .toBeLessThanOrEqual(1);

  const anchorDelta = async () => {
    const position = await anchorTop();

    return position.settled
      ? Math.max(
          Math.abs(position.before - top),
          Math.abs(position.after - top),
          Math.abs(position.after - position.before),
        )
      : Infinity;
  };

  releaseOlder();
  // The older rows are in (the injected one mounted above), the row that was at the top stays in
  // view, and the list is still up.
  await expect(row(page, await injected)).toBeAttached();
  await expect(timelineBusy(page)).toHaveAttribute("aria-busy", "false");
  await expect(anchorRow).toBeInViewport();
  await expect.poll(anchorDelta).toBeLessThanOrEqual(3);
  chunk.release();
  await expect(row(page, await injected).getByRole("region", { name: "Poll" })).toBeAttached();
  await expect(anchorRow).toBeInViewport();
  await expect.poll(anchorDelta).toBeLessThanOrEqual(3);
}

test("an older page with cards while the chunk loads keeps the list and its place", async ({
  page,
}) => {
  await olderCardsScenario(page, null);
});

test("scrolling up before the first scroll end keeps its place when cards arrive", async ({
  page,
}) => {
  await olderCardsScenario(page, "wheel");
});

for (const input of ["PageUp", "ArrowUp", "touch", "scrollbar"] as const) {
  test(`${input} cancels placement and keeps the reader's place when cards arrive`, async ({
    page,
  }) => {
    await olderCardsScenario(page, input);
  });
}

test("reading a tall thread parent while the cards chunk loads keeps its place", async ({
  page,
}) => {
  await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;

  expect(poll).toBeTruthy();
  await mockThreadParent(page, {
    threadId,
    bodyHtml: Array.from({ length: 40 }, (_, index) => `<p>Parent paragraph ${index + 1}</p>`).join(
      "",
    ),
    poll,
  });
  await openApp(page, `r/${ROOM_IDS.general}/t/${threadId}`);
  await chunk.requested;

  const list = page.getByRole("log", { name: "Replies" });
  const parent = list.locator(".thread-parent [data-message-row]");

  await expect(list.locator("[data-message-row]").last()).toBeInViewport();
  await expect(list).toHaveAttribute("data-scroll-settled", "true");
  // Establish an end anchor, then move entirely inside the tall parent.
  await list.evaluate(
    (element) =>
      new Promise<void>((resolve) => {
        element.addEventListener("scroll", () => resolve(), { once: true });
        element.scrollTop -= 100;
      }),
  );
  await list.evaluate(
    (element) =>
      new Promise<void>((resolve) => {
        element.addEventListener("scroll", () => resolve(), { once: true });
        element.scrollTop = element.scrollHeight;
      }),
  );
  await list.evaluate(
    (element) =>
      new Promise<void>((resolve) => {
        element.addEventListener("scroll", () => resolve(), { once: true });
        element.scrollTop = 250;
      }),
  );
  await expect(parent).toBeInViewport();
  await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(250);

  const parentTop = () =>
    parent.evaluate((element) => {
      const list = element.closest('[role="log"]');

      if (list === null) throw new Error("The parent is outside the replies list");

      return element.getBoundingClientRect().top - list.getBoundingClientRect().top;
    });

  const top = await parentTop();

  chunk.release();
  await expect(parent.getByRole("region", { name: "Poll" })).toBeAttached();
  await expect.poll(async () => Math.abs((await parentTop()) - top)).toBeLessThanOrEqual(3);

  // Continue reading while another measurement arrives, rather than retaining the old target.
  await list.evaluate(
    (element) =>
      new Promise<void>((resolve) => {
        element.addEventListener("scroll", () => resolve(), { once: true });
        element.scrollTop = 500;
      }),
  );
  const continuedTop = await parentTop();

  await page.setViewportSize({ width: 1280, height: 740 });
  await expect
    .poll(async () => Math.abs((await parentTop()) - continuedTop))
    .toBeLessThanOrEqual(3);
});

async function shortThreadScenario(page: Page, permalink: boolean) {
  await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;

  expect(poll).toBeTruthy();
  await mockThreadParent(page, {
    threadId,
    bodyHtml: Array.from({ length: 6 }, () => "<p>A short parent paragraph</p>").join(""),
    poll:
      permalink && poll !== undefined
        ? {
            ...poll,
            options: poll.options.flatMap((option) =>
              Array.from({ length: 4 }, (_, index) => ({
                ...option,
                id: option.id * 4 + index,
                label: `${option.label} ${index + 1}`,
              })),
            ),
          }
        : poll,
  });
  await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();
    const reply = body.messages[0];

    expect(reply).toBeTruthy();
    body.messages = reply === undefined ? [] : [{ ...reply, bodyHtml: "<p>A short reply</p>" }];
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });
  await openApp(
    page,
    `r/${ROOM_IDS.general}/t/${threadId}${permalink ? `?m=${MESSAGE_IDS.generalThreadFunnel}` : ""}`,
  );
  await chunk.requested;

  const list = page.getByRole("log", { name: "Replies" });
  const reply = list.locator("[data-message-row]").filter({ hasText: "A short reply" });

  await expect(reply).toBeInViewport();
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight <= element.clientHeight))
    .toBe(true);
  await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(0);

  await expect
    .poll(() =>
      reply.evaluate(async (element) => {
        const before = element.getBoundingClientRect().top;

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

        return Math.abs(element.getBoundingClientRect().top - before);
      }),
    )
    .toBeLessThanOrEqual(1);

  chunk.release();
  await expect(list.locator(".thread-parent").getByRole("region", { name: "Poll" })).toBeAttached();
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight > element.clientHeight))
    .toBe(true);

  if (!permalink) {
    await expect
      .poll(() =>
        list.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop),
      )
      .toBeLessThanOrEqual(3);
  }

  await expect(reply).toBeInViewport();

  if (permalink) await expect(list).toHaveAttribute("data-placement-settled", "true");
}

test("a short thread stays at the end when the cards chunk makes it overflow", async ({ page }) => {
  await shortThreadScenario(page, false);
});

test("a short thread permalink stays in view when the cards chunk makes it overflow", async ({
  page,
}) => {
  await shortThreadScenario(page, true);
});

test("an earlier reply permalink keeps its placed offset when a later card reveals", async ({
  page,
}) => {
  await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;
  const messageId = MESSAGE_IDS.generalThreadFunnel;

  expect(poll).toBeTruthy();
  await mockThreadParent(page, { threadId, bodyHtml: "<p>A short parent</p>", poll: undefined });
  await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    expect(body.messages.length).toBeGreaterThan(1);
    body.messages = body.messages.slice(0, 3);
    body.messages = body.messages.map((message, index) => ({
      ...message,
      bodyHtml: `<p>Short reply ${index + 1}</p>`,
      attachment: null,
      poll:
        index === body.messages.length - 1 && poll !== undefined
          ? {
              ...poll,
              options: poll.options.flatMap((option) =>
                Array.from({ length: 8 }, (_, index) => ({
                  ...option,
                  id: option.id * 8 + index,
                  label: `${option.label} ${index + 1}`,
                })),
              ),
            }
          : null,
    }));
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });
  await openApp(page, `r/${ROOM_IDS.general}/t/${threadId}?m=${messageId}`);
  await chunk.requested;

  const list = page.getByRole("log", { name: "Replies" });
  const target = list.locator(`[data-message-id="${messageId}"]`);

  const position = () =>
    target.evaluate(async (element) => {
      const list = element.closest('[role="log"]');

      if (list === null) throw new Error("The target is outside the replies list");

      const measure = () => element.getBoundingClientRect().top - list.getBoundingClientRect().top;

      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const before = measure();

      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

      return { before, after: measure() };
    });

  await expect(target).toBeInViewport();
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight <= element.clientHeight))
    .toBe(true);

  let top = 0;

  await expect
    .poll(async () => {
      const measured = await position();

      top = measured.after;

      return Math.abs(measured.after - measured.before);
    })
    .toBeLessThanOrEqual(1);

  chunk.release();
  await expect(list.getByRole("region", { name: "Poll" })).toBeAttached();
  await expect(target).toBeInViewport();
  await expect
    .poll(async () => {
      const measured = await position();

      return Math.max(Math.abs(measured.before - top), Math.abs(measured.after - top));
    })
    .toBeLessThanOrEqual(3);
  await expect(list).toHaveAttribute("data-placement-settled", "true");
});

test("deleting a permalink during placement restores following new replies at the bottom", async ({
  page,
}) => {
  const releaseSync = await holdSync(page);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;
  const messageId = MESSAGE_IDS.generalThreadViewerReply;

  await keepPlacementMeasuring(page, messageId);
  await mockThreadParent(page, { threadId, bodyHtml: "<p>A short parent</p>", poll: undefined });
  await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    body.messages = body.messages.slice(0, 3).map((message) => ({
      ...message,
      bodyHtml: "<p>A short reply</p>",
      attachment: null,
    }));
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });

  await openApp(page, `r/${ROOM_IDS.general}/t/${threadId}?m=${messageId}`);
  await chunk.requested;

  const list = page.getByRole("log", { name: "Replies" });
  const target = list.locator(`[data-message-id="${messageId}"]`);

  await expect(target).toBeAttached();
  await expect(list).toHaveAttribute("data-measurement-pending", "true");

  // Welcome must see the thread subscription, and its refetch must finish before deletion.
  const refreshed = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/threads/${threadId}/messages`),
  );

  releaseSync();
  await (await refreshed).finished();
  await expect(list).toHaveAttribute("data-placement-settled", "false");

  const state = await (await page.request.get("/__mock/state")).json();
  const headers = { "X-CSRF-Token": state.csrfToken };
  const deleted = await page.request.delete(`/api/v1/messages/${messageId}`, { headers });

  expect(deleted.ok()).toBe(true);
  await expect(target).toHaveCount(0);
  await expect
    .poll(() =>
      list.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop),
    )
    .toBeLessThanOrEqual(3);
  await page.request.post("/__mock/thread-post", {
    headers,
    data: {
      threadId,
      userId: USER_IDS.jonah,
      markdown: `${Array.from({ length: 30 }, () => "A new reply paragraph").join("\n\n")}\n\nThe new reply ends here`,
    },
  });
  await expect(list.getByText("The new reply ends here")).toBeInViewport();
  await expect
    .poll(() =>
      list.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop),
    )
    .toBeLessThanOrEqual(3);
  await expect(list).toHaveAttribute("data-placement-settled", "true");
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
