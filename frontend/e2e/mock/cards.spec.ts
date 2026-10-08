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
  scrollByWheel,
  shot,
  stopTrackingThread,
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

type ReaderInput = "wheel" | "PageUp" | "ArrowUp" | "Home" | "touch" | "scrollbar";

async function scrollToStart(page: Page, list: Locator, input: ReaderInput, anchor: number) {
  const atStart = () =>
    list.evaluate((element, anchor) => {
      const row = element.querySelector<HTMLElement>(`[data-message-id="${anchor}"]`);

      if (row === null || getComputedStyle(row).visibility === "hidden") return false;

      const bounds = element.getBoundingClientRect();
      const top = row.getBoundingClientRect().top;

      return top >= bounds.top && top < bounds.bottom;
    }, anchor);

  const startGesture = () =>
    list.evaluate((element, input) => {
      let offset = element.scrollTop;

      if (offset === 0) return false;

      element.setAttribute("data-input-settled", "false");

      const event = {
        wheel: "wheel",
        PageUp: "keydown",
        ArrowUp: "keydown",
        Home: "keydown",
        touch: "touchstart",
        scrollbar: "pointerdown",
      }[input];

      // A pending placement scroll end must not complete the next reader gesture.
      element.addEventListener(
        event,
        () => {
          const keyboard = input === "PageUp" || input === "ArrowUp" || input === "Home";

          // Placement can move between setup and keydown. Keyboard default scrolling
          // starts after this capture callback; wheel scrolling can precede its callback.
          if (keyboard) offset = element.scrollTop;

          if (input === "ArrowUp") {
            element.removeAttribute("data-input-row");
            element.removeAttribute("data-input-row-top");

            const bounds = element.getBoundingClientRect();

            const visible = Array.from(
              element.querySelectorAll<HTMLElement>("[data-message-row][data-message-id]"),
            ).filter((row) => {
              const rect = row.getBoundingClientRect();

              return (
                getComputedStyle(row).visibility !== "hidden" &&
                rect.bottom > bounds.top &&
                rect.top < bounds.bottom
              );
            });

            const witness =
              visible.find((row) => row.getBoundingClientRect().top >= bounds.top) ?? visible[0];

            if (witness) {
              element.setAttribute("data-input-row", witness.dataset.messageId ?? "");
              element.setAttribute(
                "data-input-row-top",
                String(witness.getBoundingClientRect().top - bounds.top),
              );
            }
          }

          // A preceding gesture can reach the edge while its final measurements land.
          if (element.scrollTop === 0) {
            element.setAttribute("data-input-settled", "true");

            return;
          }

          let released = !keyboard;
          let ended = false;
          let moved = element.scrollTop !== offset;

          const repeat = () => {
            released = false;
            ended = false;
          };

          const scroll = () => {
            moved ||= element.scrollTop !== offset;
          };

          const complete = () => {
            if (!released || (!moved && element.scrollTop === offset)) return;

            element.setAttribute("data-input-settled", "true");
            element.removeEventListener("scroll", scroll);
            element.removeEventListener("scrollend", end);
            element.removeEventListener("keydown", repeat, true);
            element.removeEventListener("keyup", release, true);
          };

          const end = (event: Event) => {
            if (event.target !== element) return;

            ended = true;
            complete();
          };

          const release = () => {
            released = true;

            if (ended || element.scrollTop === 0) complete();
          };

          element.addEventListener("scroll", scroll);
          element.addEventListener("scrollend", end);

          if (keyboard) {
            element.addEventListener("keydown", repeat, true);
            element.addEventListener("keyup", release, true);
          }
        },
        { once: true, capture: true, passive: true },
      );

      return true;
    }, input);

  const finishGesture = () =>
    list.evaluate(
      (element) =>
        new Promise<void>((resolve) => {
          // Native scroll end waits for keyboard animation and touch momentum at an edge.
          const finished = () => {
            if (element.getAttribute("data-input-settled") !== "true") return;

            element.removeEventListener("scrollend", finished);
            resolve();
          };

          element.addEventListener("scrollend", finished);
          finished();
        }),
    );

  if (input === "wheel") {
    await list.hover();

    while (!(await atStart())) {
      if (!(await startGesture())) break;
      await page.mouse.wheel(0, -10_000);
      await finishGesture();
    }
  } else if (input === "PageUp" || input === "ArrowUp" || input === "Home") {
    await list.focus();

    let repeated = false;

    while (!(await atStart())) {
      if (!(await startGesture())) break;

      if (input === "ArrowUp" && repeated) {
        // A held key repeats natively; wait for the whole gesture after releasing it.
        try {
          for (let step = 0; step < 10; step++) await page.keyboard.down(input);
        } finally {
          await page.keyboard.up(input);
        }
      } else {
        await list.press(input);
      }

      await finishGesture();

      if (input === "ArrowUp" && !repeated) {
        const id = await list.getAttribute("data-input-row");
        const top = await list.getAttribute("data-input-row-top");

        expect(id).not.toBeNull();
        expect(top).not.toBeNull();
        expect(Number(id)).toBeGreaterThan(0);
        expect(Number.isFinite(Number(top))).toBe(true);
        // Late measurements can increase scrollTop while preserving this row. Its
        // downward displacement proves ArrowUp moved the reader backward.
        await expect
          .poll(() =>
            row(page, Number(id)).evaluate((element) => {
              const list = element.closest("[data-message-list]");

              if (!list) throw new Error("The input witness is outside the message list");

              return element.getBoundingClientRect().top - list.getBoundingClientRect().top;
            }),
          )
          .toBeGreaterThan(Number(top) + 1);
      }

      repeated = true;
    }
  } else if (input === "touch") {
    const session = await page.context().newCDPSession(page);

    await session.send("Emulation.setTouchEmulationEnabled", { enabled: true });

    try {
      while (!(await atStart())) {
        const bounds = await list.boundingBox();

        if (bounds === null) throw new Error("The touch scroller is not visible");

        const x = bounds.x + bounds.width / 2;
        const start = bounds.y + 60;
        const distance = bounds.height - 120;

        if (!(await startGesture())) break;
        await session.send("Input.dispatchTouchEvent", {
          type: "touchStart",
          touchPoints: [{ x, y: start }],
        });

        if ((await list.getAttribute("data-input-settled")) === "true") {
          await session.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });

          break;
        }

        for (let step = 1; step <= 10; step++) {
          await session.send("Input.dispatchTouchEvent", {
            type: "touchMove",
            touchPoints: [{ x, y: start + (distance * step) / 10 }],
          });
          await page.evaluate(
            () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())),
          );
        }

        await session.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
        await finishGesture();
      }
    } finally {
      await session.detach();
    }
  } else {
    // Give headless Chromium a native track wide enough to drag precisely.
    await page.addStyleTag({
      content: "[data-message-list] { scrollbar-width: auto; scrollbar-gutter: stable; }",
    });

    while (!(await atStart())) {
      const bounds = await list.boundingBox();

      if (bounds === null) throw new Error("The scrollbar is not visible");

      const { width, height, total, offset } = await list.evaluate((element) => {
        if (!(element instanceof HTMLElement)) throw new Error("The scrollbar needs an HTML list");

        return {
          width: element.offsetWidth - element.clientWidth,
          height: element.clientHeight,
          total: element.scrollHeight,
          offset: element.scrollTop,
        };
      });

      expect(width).toBeGreaterThan(0);

      const thumb = Math.max(20, (height * height) / total);
      const x = bounds.x + bounds.width - 2;
      const y = bounds.y + thumb / 2;

      if (!(await startGesture())) break;
      await page.mouse.move(x, y + (offset / (total - height)) * (height - thumb));
      await page.mouse.down();
      await page.mouse.move(x, bounds.y + 1, { steps: 10 });
      await page.mouse.up();
      await finishGesture();
    }
  }
}

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
    await expect(anchorRow).not.toBeInViewport();

    await scrollToStart(page, list, input, Number(anchor));
    await expect(list).toHaveAttribute("data-placement-settled", "true");
  } else {
    await expect(list).toHaveAttribute("data-placement-settled", "true");
    await expect(list).toHaveAttribute("data-scroll-settled", "true");
    await scrollToStart(page, list, "wheel", Number(anchor));
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

for (const input of ["PageUp", "ArrowUp", "Home", "touch"] as const) {
  test(`${input} cancels placement and keeps the reader's place when cards arrive`, async ({
    page,
  }) => {
    await olderCardsScenario(page, input);
  });
}

const scrollbarTest = test.extend({
  launchOptions: { ignoreDefaultArgs: ["--hide-scrollbars"] },
});

scrollbarTest(
  "scrollbar cancels placement and keeps the reader's place when cards arrive",
  async ({ page }) => {
    await olderCardsScenario(page, "scrollbar");
  },
);

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
  // Move from the placed end entirely inside the tall parent with reader input.
  await scrollByWheel(page, list, 250 - (await list.evaluate((element) => element.scrollTop)));
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
  await scrollByWheel(page, list, 500 - (await list.evaluate((element) => element.scrollTop)));
  const continuedTop = await parentTop();

  await page.setViewportSize({ width: 1280, height: 740 });
  await expect
    .poll(async () => Math.abs((await parentTop()) - continuedTop))
    .toBeLessThanOrEqual(3);
});

async function shortThreadScenario(page: Page, permalink: boolean, pressEnd = false) {
  await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;

  // The work toolbar would make this conversation overflow before its cards arrive.
  await stopTrackingThread(page.request, threadId);
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

  if (pressEnd) {
    await expect(list).toHaveAttribute("data-placement-settled", "true");
    await list.press("End");
    await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(0);
  }

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

test("pressing End in a fitting thread keeps the end when a card makes it overflow", async ({
  page,
}) => {
  await shortThreadScenario(page, false, true);
});

test("a short thread permalink stays in view when the cards chunk makes it overflow", async ({
  page,
}) => {
  await shortThreadScenario(page, true);
});

test("an earlier reply permalink keeps its placed offset when a later card reveals", async ({
  page,
}) => {
  const releaseSync = await holdSync(page);

  const poll = await seededPoll(page.request);
  const chunk = await holdCardsChunk(page);
  const threadId = THREAD_IDS.generalActive;
  const messageId = MESSAGE_IDS.generalThreadFunnel;

  expect(poll).toBeTruthy();
  await mockThreadParent(page, { threadId, bodyHtml: "<p>A short parent</p>", poll: undefined });
  await page.route(`**/api/v1/threads/${threadId}/messages**`, async (route) => {
    if (route.request().method() !== "GET") {
      await route.continue();

      return;
    }

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

  const refreshed = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/threads/${threadId}/messages`),
  );

  releaseSync();
  await (await refreshed).finished();

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
  await expect(list).not.toHaveAttribute("data-placement-settled", "false");

  const height = await list.evaluate((element) => element.scrollHeight);
  const state = await (await page.request.get("/__mock/state")).json();

  const posted = await page.request.post("/__mock/thread-post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { threadId, userId: USER_IDS.jonah, markdown: "Another member replies after the reveal" },
  });

  expect(posted.ok()).toBe(true);
  // An offscreen reply can stay unmounted; its append still grows the list.
  await expect.poll(() => list.evaluate((element) => element.scrollHeight)).toBeGreaterThan(height);
  await expect(target).toBeInViewport();
  await expect
    .poll(async () => {
      const measured = await position();

      return Math.max(Math.abs(measured.before - top), Math.abs(measured.after - top));
    })
    .toBeLessThanOrEqual(3);
  await expect(list).toHaveAttribute("data-placement-settled", "true");

  // Sending explicitly leaves the permalink and resumes following subsequent replies.
  const composer = page.getByRole("textbox", { name: "Reply…" });

  await composer.fill("My reply leaves the permalink");
  await composer.press("Enter");
  await expect(list.getByText("My reply leaves the permalink", { exact: true })).toBeInViewport();

  const next = await page.request.post("/__mock/thread-post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { threadId, userId: USER_IDS.jonah, markdown: "Another reply follows my own" },
  });

  expect(next.ok()).toBe(true);
  await expect(list.getByText("Another reply follows my own", { exact: true })).toBeInViewport();
});

test("a short room permalink stays visible when another member posts", async ({ page }) => {
  const releaseSync = await holdSync(page);
  const chunk = await holdCardsChunk(page);
  const messageId = MESSAGE_IDS.engineeringCode - 1;

  await page.route(`**/api/v1/rooms/${ROOM_IDS.engineering}/messages**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    body.messages = body.messages
      .filter((message) => message.id >= messageId)
      .slice(0, 3)
      .map((message) => ({ ...message, bodyHtml: "<p>A short message</p>", attachment: null }));
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });
  await openApp(page, `r/${ROOM_IDS.engineering}/m/${messageId}`);
  await chunk.requested;

  const list = page.locator("[data-message-list]");
  const target = row(page, messageId);

  await expect(target).toBeInViewport();
  await expect
    .poll(() => list.evaluate((element) => element.scrollHeight <= element.clientHeight))
    .toBe(true);

  const refreshed = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/rooms/${ROOM_IDS.engineering}/messages`),
  );

  releaseSync();
  await (await refreshed).finished();

  await expect
    .poll(() =>
      target.evaluate(async (element) => {
        const before = element.getBoundingClientRect().top;

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

        return Math.abs(element.getBoundingClientRect().top - before);
      }),
    )
    .toBeLessThanOrEqual(1);

  const state = await (await page.request.get("/__mock/state")).json();

  const posted = await page.request.post("/__mock/post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.jonah,
      markdown: `${Array.from({ length: 30 }, () => "Another member posts a long message").join("\n\n")}\n\nThe incoming room message ends here`,
    },
  });

  expect(posted.ok()).toBe(true);
  await expect(list.getByText("The incoming room message ends here")).toBeAttached();
  await expect(target).toBeInViewport();
  chunk.release();
  await expect(target).toBeInViewport();
});

for (const timing of ["during", "after"] as const) {
  test(`deleting a permalink ${timing} placement restores following new replies at the bottom`, async ({
    page,
  }) => {
    const releaseSync = await holdSync(page);
    const chunk = await holdCardsChunk(page);
    const threadId = THREAD_IDS.generalActive;
    const messageId = MESSAGE_IDS.generalThreadViewerReply;

    if (timing === "during") await keepPlacementMeasuring(page, messageId);
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

    if (timing === "during") await expect(list).toHaveAttribute("data-measurement-pending", "true");

    // Welcome must see the thread subscription, and its refetch must finish before deletion.
    const refreshed = page.waitForResponse((response) =>
      response.url().endsWith(`/api/v1/threads/${threadId}/messages`),
    );

    releaseSync();
    await (await refreshed).finished();

    if (timing === "during") {
      await expect(list).toHaveAttribute("data-placement-settled", "false");
    } else {
      chunk.release();
      await expect(list).toHaveAttribute("data-placement-settled", "true");
      await expect(target).toBeInViewport();
    }

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
}

test("editing an older message from the composer keeps it visible when another member posts", async ({
  page,
}) => {
  const releaseSync = await holdSync(page);
  const chunk = await holdCardsChunk(page);

  await openApp(page, `r/${ROOM_IDS.general}`);
  await chunk.requested;
  chunk.release();

  const refreshed = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/rooms/${ROOM_IDS.general}/messages`),
  );

  releaseSync();
  await (await refreshed).finished();

  const list = page.locator("[data-message-list]");
  const composer = page.locator(".composer-input").first();

  await expect(list).toHaveAttribute("data-placement-settled", "true");
  await composer.fill("My message to edit from the composer");
  await composer.press("Enter");

  const sent = list.locator("[data-message-row]", {
    hasText: "My message to edit from the composer",
  });

  await expect(sent).toHaveAttribute("data-message-id", /^\d+$/);

  const messageId = Number(await sent.getAttribute("data-message-id"));
  const state = await (await page.request.get("/__mock/state")).json();

  const post = (ending: string) =>
    page.request.post("/__mock/post", {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: {
        roomId: ROOM_IDS.general,
        userId: USER_IDS.jonah,
        markdown: `${Array.from({ length: 30 }, () => "Another member's long message").join("\n\n")}\n\n${ending}`,
      },
    });

  expect((await post("The message before editing ends here")).ok()).toBe(true);
  await expect(list.getByText("The message before editing ends here")).toBeInViewport();
  await expect(row(page, messageId)).not.toBeInViewport();
  await composer.press("ArrowUp");

  const editor = row(page, messageId).getByRole("textbox", { name: "Edit message" });

  await expect(editor).toBeFocused();
  await expect(editor).toBeInViewport();
  await expect(list).toHaveAttribute("data-scroll-settled", "true");

  const height = await list.evaluate((element) => element.scrollHeight);

  expect((await post("The message during editing ends here")).ok()).toBe(true);
  await expect.poll(() => list.evaluate((element) => element.scrollHeight)).toBeGreaterThan(height);
  await expect(editor).toBeInViewport();
});

for (const { conversation, fromLink } of [
  { conversation: "room", fromLink: false },
  { conversation: "thread", fromLink: false },
  { conversation: "room", fromLink: true },
] as const) {
  const name = fromLink
    ? "ArrowUp from a message link keeps the reader's place through delayed growth"
    : `a ${conversation} follows another member after an image grows beyond the settled cards`;

  test(name, async ({ page }) => {
    const releaseSync = await holdSync(page);
    const poll = await seededPoll(page.request);
    const chunk = await holdCardsChunk(page);
    const threadId = THREAD_IDS.generalActive;

    const messagesPath =
      conversation === "room"
        ? `/api/v1/rooms/${ROOM_IDS.engineering}/messages`
        : `/api/v1/threads/${threadId}/messages`;

    const imageRequested = Promise.withResolvers<void>();
    const imageReleased = Promise.withResolvers<void>();

    expect(poll).toBeTruthy();
    await page.route("**/__delayed-growth.svg", async (route) => {
      imageRequested.resolve();
      await imageReleased.promise;
      await route.fulfill({
        contentType: "image/svg+xml",
        body: '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="500"><rect width="120" height="500" fill="teal"/></svg>',
      });
    });
    await page.route(`**${messagesPath}**`, async (route) => {
      const response = await route.fetch();
      const body: MessagePage = await response.json();

      body.messages = body.messages.map((message, index) => ({
        ...message,
        bodyHtml:
          index === body.messages.length - 1
            ? `<p>The last message has a delayed image</p><img src="/__delayed-growth.svg" alt="Delayed layout growth">${fromLink ? '<p><a href="/" data-reader-link>Read this message link</a></p>' : ""}`
            : "<p>A message before the image</p>",
        attachment: null,
        cards: [],
        poll: index === body.messages.length - 1 ? (poll ?? null) : null,
      }));
      body.before = null;
      body.after = null;
      await route.fulfill({ response, json: body });
    });

    if (conversation === "thread")
      await mockThreadParent(page, {
        threadId,
        bodyHtml: "<p>A short parent</p>",
        poll: undefined,
      });

    await openApp(
      page,
      conversation === "room" ? `r/${ROOM_IDS.engineering}` : `r/${ROOM_IDS.general}/t/${threadId}`,
    );
    await chunk.requested;
    chunk.release();

    const list =
      conversation === "room"
        ? page.locator("[data-message-list]")
        : page.getByRole("log", { name: "Replies" });

    const growing = list.locator("[data-message-row]").filter({
      has: page.getByRole("img", { name: "Delayed layout growth" }),
    });

    const distanceFromEnd = () =>
      list.evaluate(async (element) => {
        const measure = () => element.scrollHeight - element.clientHeight - element.scrollTop;

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        const before = measure();

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        const after = measure();

        return Math.max(Math.abs(before), Math.abs(after), Math.abs(after - before));
      });

    await expect(growing.getByRole("region", { name: "Poll" })).toBeVisible();
    await imageRequested.promise;

    const refreshed = page.waitForResponse((response) => response.url().endsWith(messagesPath));

    releaseSync();
    await (await refreshed).finished();
    await expect(list).toHaveAttribute("data-placement-settled", "true");
    await expect.poll(distanceFromEnd).toBeLessThanOrEqual(3);

    let readingId: number | null = null;
    let readingTop = 0;

    const readingPosition = () => {
      if (readingId === null) throw new Error("No reader row was captured");

      return row(page, readingId).evaluate(async (element) => {
        const list = element.closest("[data-message-list]");

        if (list === null) throw new Error("The reader row is outside the list");

        const measure = () =>
          element.getBoundingClientRect().top - list.getBoundingClientRect().top;

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        const before = measure();

        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        const after = measure();

        return { before, after };
      });
    };

    const readingDelta = async () => {
      const { before, after } = await readingPosition();

      return Math.max(
        Math.abs(before - readingTop),
        Math.abs(after - readingTop),
        Math.abs(after - before),
      );
    };

    if (fromLink) {
      await expect(list).toHaveAttribute("data-scroll-settled", "true");
      await list.evaluate((element) => {
        element.setAttribute("data-link-key-seen", "false");
        element.setAttribute("data-link-key-settled", "false");
        element.addEventListener(
          "keydown",
          (event) => {
            if (
              !(event instanceof KeyboardEvent) ||
              event.key !== "ArrowUp" ||
              !(event.target instanceof Element) ||
              event.target.closest("[data-reader-link]") === null
            )
              return;

            const offset = element.scrollTop;

            element.setAttribute("data-link-key-seen", "true");
            element.setAttribute("data-link-key-offset", String(offset));

            const end = (event: Event) => {
              if (event.target !== element || element.scrollTop === offset) return;

              element.setAttribute("data-link-key-settled", "true");
              element.removeEventListener("scrollend", end);
            };

            element.addEventListener("scrollend", end);
          },
          { capture: true, once: true },
        );
      });
      await growing.getByRole("link", { name: "Read this message link" }).press("ArrowUp");
      await expect(list).toHaveAttribute("data-link-key-seen", "true");

      const offset = await list.getAttribute("data-link-key-offset");

      expect(offset).not.toBeNull();
      await expect
        .poll(() => list.evaluate((element) => element.scrollTop))
        .toBeLessThan(Number(offset));
      await expect(list).toHaveAttribute("data-link-key-settled", "true");

      readingId = await list.evaluate((element) => {
        const bounds = element.getBoundingClientRect();

        const visible = Array.from(element.querySelectorAll("[data-message-row]")).find((row) => {
          const rect = row.getBoundingClientRect();

          return (
            rect.top >= bounds.top &&
            rect.bottom <= bounds.bottom &&
            row.parentElement?.style.visibility !== "hidden"
          );
        });

        if (visible === undefined) throw new Error("No complete message is visible");

        const id = visible.getAttribute("data-message-id");

        if (id === null) throw new Error("The visible message has no id");

        return Number(id);
      });

      await expect
        .poll(async () => {
          const position = await readingPosition();

          readingTop = position.after;

          return Math.abs(position.after - position.before);
        })
        .toBeLessThanOrEqual(1);
    }

    await expect(growing).toBeInViewport();
    const height = await growing.evaluate((element) => element.getBoundingClientRect().height);

    // The poll has mounted and placement has settled before this dimensionless image resolves.
    imageReleased.resolve();
    await expect
      .poll(() => growing.evaluate((element) => element.getBoundingClientRect().height))
      .toBeGreaterThan(height + 40);

    if (fromLink) await expect.poll(readingDelta).toBeLessThanOrEqual(3);
    else await expect.poll(distanceFromEnd).toBeLessThanOrEqual(3);

    const listHeight = await list.evaluate((element) => element.scrollHeight);
    const state = await (await page.request.get("/__mock/state")).json();
    const text = `Another member follows the delayed ${conversation} image`;

    const posted = await page.request.post(
      conversation === "room" ? "/__mock/post" : "/__mock/thread-post",
      {
        headers: { "X-CSRF-Token": state.csrfToken },
        data: {
          ...(conversation === "room" ? { roomId: ROOM_IDS.engineering } : { threadId }),
          userId: USER_IDS.jonah,
          markdown: text,
        },
      },
    );

    expect(posted.ok()).toBe(true);

    if (fromLink) {
      await expect
        .poll(() => list.evaluate((element) => element.scrollHeight))
        .toBeGreaterThan(listHeight);
      await expect.poll(readingDelta).toBeLessThanOrEqual(3);
      await expect(list.getByText(text, { exact: true })).not.toBeInViewport();
    } else {
      await expect(list.getByText(text, { exact: true })).toBeInViewport();
      await expect.poll(distanceFromEnd).toBeLessThanOrEqual(3);
    }
  });
}

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
