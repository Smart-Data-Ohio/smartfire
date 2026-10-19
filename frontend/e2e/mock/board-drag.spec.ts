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

/** Every status write the page sends, in order. */
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
  let box: Awaited<ReturnType<Locator["boundingBox"]>> = null;

  await expect.poll(async () => (box = await target.boundingBox()) !== null).toBe(true);

  return box === null ? { x: 0, y: 0 } : { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

/** Presses on `from` with the mouse and moves in steps to `to`, leaving the button down. */
async function dragTo(page: Page, from: Locator, to: { x: number; y: number }): Promise<void> {
  const start = await center(from);

  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 3, start.y + 2);
  await page.mouse.move(to.x, to.y, { steps: 12 });
}

/** Opens the columns once the sync welcome's reload has landed, so no drag starts on a board about to reload. */
async function openBoard(page: Page): Promise<void> {
  await page.setViewportSize(DESKTOP);

  const release = await holdSync(page);

  await openApp(page, `r/${BOARD}?view=board`);
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();

  const reloaded = page.waitForResponse((response) => LISTING.test(response.url()));

  release();
  await reloaded;
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
}

test("dragging a card to another column moves it and saves the status", async ({ page }) => {
  const writes = workWrites(page);

  await openBoard(page);
  await dragTo(page, card(page, "Planned", /Launch week plan/), await center(column(page, "In progress")));

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

  await openBoard(page);

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

  await openBoard(page);
  await dragTo(page, card(page, "Planned", /Launch week plan/), await center(column(page, "Blocked")));
  await expect(column(page, "Blocked").getByText("Moves here")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(column(page, "Blocked").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();

  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));

  const toolbar = await center(page.getByRole("button", { name: "New post" }));

  await dragTo(page, card(page, "Planned", /Launch week plan/), toolbar);
  await page.mouse.up();

  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}\\?view=board$`));
  await page.waitForTimeout(300);
  expect(writes).toHaveLength(0);
});

test("a card the viewer can't move has no drag", async ({ page }) => {
  const writes = workWrites(page);

  // Not the board's creator or an administrator: only posts they created or own move.
  await page.route(LISTING, async (route) => {
    const response = await route.fetch();
    const listing: BoardListing = await response.json();

    await route.fulfill({ response, json: { ...listing, canAdminister: false } });
  });
  await openBoard(page);

  // Maya created and owns Launch week plan; the viewer created Fix retries, which an agent owns.
  const locked = card(page, "Planned", /Launch week plan/);

  await expect(locked).not.toHaveAttribute("aria-describedby", /.+/);
  await locked.focus();
  await page.keyboard.press("Space");
  await expect(page.getByText(/^Picked up/)).toHaveCount(0);

  await dragTo(page, locked, await center(column(page, "In progress")));
  await expect(column(page, "In progress").getByText("Moves here")).toHaveCount(0);
  await page.mouse.up();
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();

  await dragTo(page, card(page, "Blocked", /Fix retries/), await center(column(page, "Done")));
  await page.mouse.up();
  await expect(card(page, "Done", /Fix retries/)).toBeVisible();
  await expect.poll(() => writes.length).toBe(1);
  expect(writes[0]?.url()).toMatch(new RegExp(`/threads/${BOARD_POST_IDS.retryBug}/work$`));
});

test("the keyboard picks a card up and moves it between columns", async ({ page }) => {
  const writes = workWrites(page);

  await openBoard(page);

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
  await openBoard(page);
  await dragTo(page, card(page, "Planned", /Launch week plan/), await center(column(page, "Done")));
  await page.mouse.up();

  await expect(page.getByText("Couldn't move Launch week plan")).toBeVisible();
  await expect(card(page, "Planned", /Launch week plan/)).toBeVisible();
  await expect(card(page, "Done", /Launch week plan/)).toHaveCount(0);
});

test("another member's moves still land while a card is held", async ({ page, request }) => {
  await openBoard(page);
  await dragTo(page, card(page, "Planned", /Launch week plan/), await center(column(page, "In progress")));

  const reply = await request.post("/__mock/board-update", {
    data: { threadId: BOARD_POST_IDS.searchIndex, status: "done" },
  });

  expect(reply.ok()).toBe(true);
  await expect(card(page, "Done", /Rebuild the search index/)).toBeVisible();
  await expect(column(page, "In progress").getByText("Moves here")).toBeVisible();
  await page.mouse.up();

  await expect(card(page, "In progress", /Launch week plan/)).toBeVisible();
});
