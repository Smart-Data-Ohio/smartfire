import type { Locator, Page, Request } from "@playwright/test";
import { forbidden } from "../../mock/http.ts";
import { BOARD_POST_IDS, BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import type { BoardListing } from "../../src/gen/BoardListing.ts";
import { DESKTOP, expect, holdSync, openApp, test } from "./support.ts";

const BOARD = BOARD_ROOM_ID;

const LISTING = new RegExp(`/api/v1/rooms/${BOARD}/board(\\?|$)`);

function column(page: Page, name: string): Locator {
  return page.getByRole("region", { name });
}

function card(page: Page, columnName: string, title: RegExp): Locator {
  return column(page, columnName).getByRole("link", { name: title });
}

function workWrites(page: Page): Request[] {
  const writes: Request[] = [];

  page.on("request", (request) => {
    if (request.method() === "PATCH" && /\/api\/v1\/threads\/\d+\/work$/.test(request.url())) {
      writes.push(request);
    }
  });

  return writes;
}

async function center(target: Locator): Promise<{ x: number; y: number }> {
  await expect.poll(() => target.boundingBox()).not.toBeNull();

  const box = await target.boundingBox();

  if (box === null) {
    throw new Error("the target has no box");
  }

  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

async function holdOver(page: Page, from: Locator, to: { x: number; y: number }): Promise<void> {
  const start = await center(from);

  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 3, start.y + 2);
  await page.mouse.move(to.x, to.y, { steps: 12 });
}

interface SyncEvent {
  readonly type: string;
  readonly data: { readonly id?: number };
}

interface GatheredSync {
  readonly hold: (page: Page) => Promise<() => void>;
  readonly gather: () => void;
  readonly holds: (threadId: number) => boolean;
  readonly deliver: () => void;
}

function gatherSync(): GatheredSync {
  let gathering = false;
  let events: SyncEvent[] = [];
  let send: (message: string) => void = () => {};

  return {
    async hold(page) {
      const held = Promise.withResolvers<void>();

      await page.routeWebSocket(/\/api\/v1\/sync/, (socket) => {
        const server = socket.connectToServer();

        send = (message) => socket.send(message);
        server.onMessage(async (message) => {
          await held.promise;

          const frame: { readonly t: string; readonly events?: SyncEvent[] } = JSON.parse(
            String(message),
          );

          if (gathering && frame.t === "batch") {
            events.push(...(frame.events ?? []));
          } else {
            socket.send(message);
          }
        });
      });

      return () => held.resolve();
    },
    gather() {
      gathering = true;
    },
    holds: (threadId) =>
      events.some((event) => event.type === "thread.updated" && event.data.id === threadId),
    deliver() {
      gathering = false;
      send(JSON.stringify({ t: "batch", events }));
      events = [];
    },
  };
}

async function openSyncedBoard(
  page: Page,
  hold: (page: Page) => Promise<() => void> = holdSync,
): Promise<void> {
  await page.setViewportSize(DESKTOP);

  const release = await hold(page);

  await openApp(page, `r/${BOARD}?view=board`);
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible({ timeout: 15_000 });

  const reloaded = page.waitForResponse((response) => LISTING.test(response.url()));

  release();
  await reloaded;
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
}

async function viewAsPlainMember(page: Page): Promise<void> {
  await page.route(LISTING, async (route) => {
    const response = await route.fetch();
    const listing: BoardListing = await response.json();

    await route.fulfill({ response, json: { ...listing, canAdminister: false } });
  });
}

test.describe("on a touch screen", () => {
  test.use({ hasTouch: true });

  test("a tap opens the card and no card offers a drag", async ({ page }) => {
    await openSyncedBoard(page);

    const target = card(page, "Planned", /Launch week plan/);

    await expect(target).not.toHaveAttribute("aria-describedby", /.+/);
    await expect(page.locator(".board-card[data-movable]")).toHaveCount(0);
    await target.tap();
    await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/t/${BOARD_POST_IDS.launchWeek}`));
  });
});

test("dragging a card to another column moves it and saves the status", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "In progress")),
  );

  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();
  await page.mouse.up();

  await expect(card(page, "In progress", /Launch week plan/)).toBeVisible();
  await expect(card(page, "Planned", /Launch week plan/)).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.url()).toMatch(new RegExp(`/threads/${BOARD_POST_IDS.launchWeek}/work$`));
  expect(writes[0]?.postDataJSON()).toEqual({ status: "in_progress" });

  await page.reload();
  await expect(card(page, "In progress", /Launch week plan/)).toBeVisible();
});

test("a short press still opens the card", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);

  const target = card(page, "Planned", /Launch week plan/);
  const start = await center(target);

  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 2, start.y + 1);
  await page.mouse.up();

  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/t/${BOARD_POST_IDS.launchWeek}`));
  expect(writes).toHaveLength(0);
});

test("Escape and a drop outside every column put the card back", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "Blocked")),
  );
  await expect(column(page, "Blocked").getByText("Moves here")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(column(page, "Blocked").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();

  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));

  const toolbar = await center(page.getByRole("button", { name: "New post", exact: true }));

  await holdOver(page, card(page, "Planned", /Launch week plan/), toolbar);
  await page.mouse.up();

  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));
  await page.waitForTimeout(300);
  expect(writes).toHaveLength(0);
});

test("a card the viewer can't move has no drag", async ({ page }) => {
  const writes = workWrites(page);

  await viewAsPlainMember(page);
  await openSyncedBoard(page);

  const someoneElsesPost = card(page, "Planned", /Launch week plan/);
  const agentOwnedPostTheViewerCreated = card(page, "Blocked", /Fix retries/);

  await expect(someoneElsesPost).not.toHaveAttribute("aria-describedby", /.+/);
  await someoneElsesPost.focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up/)).toHaveCount(0);

  await holdOver(page, someoneElsesPost, await center(column(page, "In progress")));
  await expect(column(page, "In progress").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();

  await holdOver(page, agentOwnedPostTheViewerCreated, await center(column(page, "Done")));
  await page.mouse.up();
  await expect(card(page, "Done", /Fix retries/)).toBeVisible();
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.url()).toMatch(new RegExp(`/threads/${BOARD_POST_IDS.retryBug}/work$`));
});

test("the keyboard picks a card up and moves it between columns", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);

  const target = card(page, "Planned", /Cursor pagination/);

  await target.focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up Cursor pagination for the API/)).toBeAttached();
  await page.keyboard.press("ArrowRight");
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(column(page, "Blocked").getByText("Moves here")).toBeVisible();
  await page.keyboard.press("Enter");

  await expect(card(page, "Blocked", /Cursor pagination/)).toBeFocused();
  await expect(page.getByText(/^Moved Cursor pagination for the API to Blocked/)).toBeAttached();
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.postDataJSON()).toEqual({ status: "blocked" });

  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("Escape");
  await expect(card(page, "Blocked", /Cursor pagination/)).toBeFocused();
  await expect(page.getByText(/stays in Blocked/)).toBeAttached();
  expect(writes).toHaveLength(1);
});

test("a refused move goes back with a toast", async ({ page }) => {
  await page.route(`**/api/v1/threads/${BOARD_POST_IDS.launchWeek}/work`, (route) =>
    route.fulfill({
      status: 403,
      contentType: "application/json",
      body: JSON.stringify({ error: forbidden().error }),
    }),
  );
  await openSyncedBoard(page);
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "Done")),
  );
  await page.mouse.up();

  await expect(page.getByText("Couldn't move Launch week plan")).toBeVisible();
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(card(page, "Done", /Launch week plan/)).toHaveCount(0);
});

test("another member's moves still land while a card is held", async ({ page, request }) => {
  await openSyncedBoard(page);
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "In progress")),
  );

  const reply = await request.post("/__mock/board-update", {
    data: { threadId: BOARD_POST_IDS.searchIndex, status: "done" },
  });

  expect(reply.ok()).toBe(true);
  await expect(card(page, "Done", /Rebuild the search index/)).toBeVisible();
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();
  await page.mouse.up();

  await expect(card(page, "In progress", /Launch week plan/)).toBeVisible();
});

test("a release over the card after Escape doesn't open it", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);

  const target = card(page, "Planned", /Launch week plan/);
  const start = await center(target);

  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 12, start.y + 4, { steps: 4 });
  await expect(page.locator(".board-drag-ghost")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(page.locator(".board-drag-ghost")).toHaveCount(0);
  await page.mouse.up();

  await page.waitForTimeout(300);
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));
  expect(writes).toHaveLength(0);
});

test("a card held on the keyboard stays held when another member moves it", async ({
  page,
  request,
}) => {
  await openSyncedBoard(page);
  await card(page, "Planned", /Cursor pagination/).focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up Cursor pagination for the API/)).toBeAttached();

  const reply = await request.post("/__mock/board-update", {
    data: { threadId: BOARD_POST_IDS.apiPagination, status: "done" },
  });

  expect(reply.ok()).toBe(true);
  await expect(card(page, "Done", /Cursor pagination/)).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByText(/Cursor pagination for the API stays in Done/)).toBeAttached();

  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "Blocked")),
  );
  await expect(column(page, "Blocked").getByText("Moves here")).toBeVisible();
  await page.mouse.up();
  await expect(card(page, "Blocked", /Launch week plan/)).toBeVisible();
});

test("a second move queued behind a slow save still lands focused", async ({ page }) => {
  const writes = workWrites(page);

  await page.route(`**/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work`, async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    await route.continue();
  });
  await openSyncedBoard(page);
  await card(page, "Planned", /Cursor pagination/).focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(card(page, "In progress", /Cursor pagination/)).toBeFocused();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");

  await expect.poll(() => writes.length, { timeout: 10_000 }).toBe(2);
  await expect(card(page, "Blocked", /Cursor pagination/)).toBeFocused();
  expect(writes.map((write) => write.postDataJSON())).toEqual([
    { status: "in_progress" },
    { status: "blocked" },
  ]);
});

test("a queued move doesn't pull focus back after the viewer tabs away", async ({ page }) => {
  const writes = workWrites(page);

  await page.route(`**/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work`, async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    await route.continue();
  });
  await openSyncedBoard(page);
  await card(page, "Planned", /Cursor pagination/).focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(card(page, "In progress", /Cursor pagination/)).toBeFocused();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Tab");

  await expect.poll(() => writes.length, { timeout: 10_000 }).toBe(2);
  await expect(card(page, "Blocked", /Cursor pagination/)).toBeVisible();
  await page.waitForTimeout(300);
  await expect(card(page, "Blocked", /Cursor pagination/)).not.toBeFocused();
});

test("a queued move doesn't take focus from a card picked up after another member moved it", async ({
  page,
  request,
}) => {
  const writes = workWrites(page);

  await page.route(`**/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work`, async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    await route.continue();
  });
  await openSyncedBoard(page);
  await card(page, "Planned", /Cursor pagination/).focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(card(page, "In progress", /Cursor pagination/)).toBeFocused();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");

  const reply = await request.post("/__mock/board-update", {
    data: { threadId: BOARD_POST_IDS.apiPagination, status: "done" },
  });

  expect(reply.ok()).toBe(true);
  await expect(card(page, "Done", /Cursor pagination/)).toBeVisible();
  await card(page, "Planned", /Launch week plan/).focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up Launch week plan/)).toBeAttached();

  await expect.poll(() => writes.length, { timeout: 10_000 }).toBe(2);
  await expect(card(page, "Blocked", /Cursor pagination/)).toBeVisible();
  await page.waitForTimeout(300);
  await expect(card(page, "Planned", /Launch week plan/)).toBeFocused();
  await expect(page.getByText(/^Picked up Launch week plan/)).toBeAttached();
});

test("a queued move landing in the same sync batch as a held card's move leaves it held", async ({
  page,
  request,
}) => {
  const writes = workWrites(page);
  const sync = gatherSync();

  await page.route(`**/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work`, async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 4000));
    await route.continue();
  });
  await openSyncedBoard(page, sync.hold);
  await card(page, "Planned", /Cursor pagination/).focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(card(page, "In progress", /Cursor pagination/)).toBeFocused();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await card(page, "Planned", /Launch week plan/).focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up Launch week plan/)).toBeAttached();

  const moveAway = async (threadId: number, status: string) => {
    const reply = await request.post("/__mock/board-update", { data: { threadId, status } });

    expect(reply.ok()).toBe(true);
    await expect.poll(() => sync.holds(threadId)).toBe(true);
  };

  sync.gather();
  await moveAway(BOARD_POST_IDS.apiPagination, "blocked");
  await moveAway(BOARD_POST_IDS.launchWeek, "done");
  sync.deliver();
  await expect(card(page, "Blocked", /Cursor pagination/)).toBeVisible();
  await expect(card(page, "Done", /Launch week plan/)).toBeFocused();
  await page.waitForTimeout(300);
  await expect(card(page, "Done", /Launch week plan/)).toBeFocused();
  await expect(column(page, "Done").locator('[data-dragging="keyboard"]')).toHaveCount(1);
  expect(writes).toHaveLength(1);
});

test("a release after the board scrolls under a held card saves the column under the pointer", async ({
  page,
}) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);
  await page.setViewportSize({ width: 1024, height: 768 });

  const pointer = await center(column(page, "In progress"));

  await holdOver(page, card(page, "Planned", /Launch week plan/), pointer);
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();

  const blocked = await center(column(page, "Blocked"));

  await page.locator(".board-columns").evaluate((board, by) => {
    board.scrollLeft += by;
  }, blocked.x - pointer.x);
  await expect
    .poll(async () => (await center(column(page, "Blocked"))).x)
    .toBeCloseTo(pointer.x, -1);

  await expect(column(page, "Blocked").getByText("Moves here")).toBeVisible();
  await expect(column(page, "In progress").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();

  await expect(card(page, "Blocked", /Launch week plan/)).toBeVisible();
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.postDataJSON()).toEqual({ status: "blocked" });
});

test("another pointer lifting or cancelling elsewhere doesn't end a held card's drag", async ({
  page,
}) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "In progress")),
  );
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();

  const elsewhere = await center(column(page, "Done"));

  await page.evaluate(({ x, y }) => {
    const target = document.elementFromPoint(x, y) ?? document.body;

    for (const type of ["pointerdown", "pointerup", "pointercancel"]) {
      target.dispatchEvent(
        new PointerEvent(type, {
          pointerId: 41,
          pointerType: "touch",
          isPrimary: false,
          clientX: x,
          clientY: y,
          bubbles: true,
        }),
      );
    }
  }, elsewhere);
  await page.waitForTimeout(300);

  expect(writes).toHaveLength(0);
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();

  await page.mouse.up();

  await expect(card(page, "In progress", /Launch week plan/)).toBeVisible();
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.postDataJSON()).toEqual({ status: "in_progress" });
});

test("a held card goes back when the page loses its pointer", async ({ page }) => {
  const writes = workWrites(page);

  await openSyncedBoard(page);
  await page.evaluate(() => {
    window.addEventListener(
      "pointerdown",
      (event) => {
        document.documentElement.dataset.testPointer = String(event.pointerId);
      },
      { capture: true },
    );
  });
  await holdOver(
    page,
    card(page, "Planned", /Launch week plan/),
    await center(column(page, "In progress")),
  );
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();

  const released = await page.evaluate(() => {
    const pointerId = Number(document.documentElement.dataset.testPointer);
    const holder = [...document.querySelectorAll("*")].find((element) =>
      element.hasPointerCapture(pointerId),
    );

    holder?.releasePointerCapture(pointerId);

    return holder !== undefined;
  });

  expect(released).toBe(true);

  const current = await center(column(page, "In progress"));

  await page.mouse.move(current.x + 4, current.y + 4);
  await expect(column(page, "In progress").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();

  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));
  await page.waitForTimeout(300);
  expect(writes).toHaveLength(0);
});
