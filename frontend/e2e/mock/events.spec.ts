import type { Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  matrix,
  openApp,
  ROOM_IDS,
  SHOTS,
  shot,
  syncWelcomed,
  test,
} from "./support.ts";

const UPCOMING = 8001;

const WEEKLY_HEAD = 8100;

const WEEKLY_SECOND = 8101;

const WEEKLY_LAST = 8103;

declare global {
  interface Window {
    /** History calls recorded by `watchHistory`. */
    smartfireHistoryCalls?: string[];
  }
}

interface HeldRead {
  /** The server has answered: the answer is a snapshot of that moment. */
  readonly answered: Promise<void>;
  /** Hands the answer to the page. */
  readonly release: () => void;
  /** The page has the answer. */
  readonly delivered: Promise<void>;
}

/**
 * Holds the page's next `method` request to `path` (under /api/v1) after the server has answered
 * it, so the page gets an old snapshot (a read's, or a save's reply) late, after newer ones.
 */
async function holdNextRead(
  page: Page,
  path: string,
  method: "GET" | "PATCH" = "GET",
): Promise<HeldRead> {
  const answered = Promise.withResolvers<void>();
  const released = Promise.withResolvers<void>();
  const delivered = Promise.withResolvers<void>();
  let holding = true;

  await page.route(`**/api/v1${path}`, async (route) => {
    if (!holding || route.request().method() !== method) {
      return route.fallback();
    }

    holding = false;

    const response = await route.fetch();

    answered.resolve();
    await released.promise;
    await route.fulfill({ response });
    delivered.resolve();
  });

  return {
    answered: answered.promise,
    release: () => released.resolve(),
    delivered: delivered.promise,
  };
}

/** Holds `method` requests to `path` (under /api/v1) until the returned release is called. */
async function holdWrite(page: Page, method: "POST" | "PATCH", path: string): Promise<() => void> {
  const held = Promise.withResolvers<void>();

  await page.route(`**/api/v1${path}`, async (route) => {
    if (route.request().method() !== method) {
      return route.fallback();
    }

    await held.promise;

    return route.fallback();
  });

  return () => held.resolve();
}

/**
 * Records the history calls the app makes from now on (back, forward, go, pushState,
 * replaceState). A completion that navigates has made its call by the time it settles, even
 * when the traversal it asked for hasn't landed yet, so an empty list proves it stayed put.
 */
async function watchHistory(page: Page): Promise<() => Promise<readonly string[]>> {
  await page.evaluate(() => {
    const calls: string[] = [];
    const history = window.history;
    const back = history.back.bind(history);
    const forward = history.forward.bind(history);
    const go = history.go.bind(history);
    const pushState = history.pushState.bind(history);
    const replaceState = history.replaceState.bind(history);

    window.smartfireHistoryCalls = calls;

    history.back = () => {
      calls.push("back");
      back();
    };

    history.forward = () => {
      calls.push("forward");
      forward();
    };

    history.go = (delta) => {
      calls.push("go");
      go(delta);
    };

    history.pushState = (data, unused, url) => {
      calls.push("pushState");
      pushState(data, unused, url);
    };

    history.replaceState = (data, unused, url) => {
      calls.push("replaceState");
      replaceState(data, unused, url);
    };
  });

  return () => page.evaluate(() => [...(window.smartfireHistoryCalls ?? [])]);
}

/** Lets a reply the page just took in (and any navigation it starts) reach the screen. */
interface LostNews {
  /** The page's sync socket is welcomed for the first time. */
  readonly welcomed: Promise<void>;
  /** From now on the page hears nothing on its socket: what's published meanwhile is lost. */
  readonly lose: () => void;
  /**
   * Drops the socket. The page reconnects to a server that says it can't replay what was lost
   * (its welcome `resumed: false`, as after an expired replay or a restart), then hears again.
   * Resolves once that welcome reached the page.
   */
  readonly reconnectUnresumed: () => Promise<void>;
}

interface Sequenced {
  readonly seq: number;
}

/** What `interceptSync` reads of a server frame. */
interface ServerFrameSeen {
  readonly t: string;
  readonly seq?: number;
  readonly events?: readonly Sequenced[];
}

/** Stands between the page and its sync socket, to lose news and reconnect without a replay. */
async function interceptSync(page: Page): Promise<LostNews> {
  const first = Promise.withResolvers<void>();
  let again: PromiseWithResolvers<void> | null = null;
  let losing = false;
  let lostThrough = 0;
  let close: (() => Promise<void>) | null = null;

  await page.routeWebSocket(/\/api\/v1\/sync/, (socket) => {
    const server = socket.connectToServer();

    close = () => socket.close({ code: 4000, reason: "dropped by the test" });
    server.onMessage((message) => {
      const frame: ServerFrameSeen = JSON.parse(String(message));

      if (frame.t !== "welcome") {
        if (!losing) {
          socket.send(message);
        } else {
          for (const event of frame.events ?? []) lostThrough = Math.max(lostThrough, event.seq);
        }

        return;
      }

      if (again === null) {
        socket.send(message);
        first.resolve();

        return;
      }

      // The mock can replay what was lost, so it welcomes at the page's own point and replays.
      // A server that can't says so, at its newest sequence: the page then skips the replay that
      // follows, as it would never have come.
      losing = false;
      socket.send(
        JSON.stringify({ ...frame, seq: Math.max(frame.seq ?? 0, lostThrough), resumed: false }),
      );
      again.resolve();
    });
  });

  return {
    welcomed: first.promise,
    lose: () => {
      losing = true;
    },
    reconnectUnresumed: async () => {
      const welcome = Promise.withResolvers<void>();

      again = welcome;
      await close?.();
      await welcome.promise;
    },
  };
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(resolve, 250)));
      }),
  );
}

test.describe("a room's events", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the header's calendar opens the events; a row opens its page, and Going counts", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}`);
    await page.getByRole("link", { name: "Events", exact: true }).click();

    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
    await expect(page.getByRole("heading", { level: 1, name: "Events" })).toBeVisible();

    const tabs = page.getByRole("tablist", { name: "Events" });

    await expect(tabs.getByRole("tab", { name: /Upcoming/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(page.getByRole("link", { name: "Cancelled coffee chat" })).toHaveCount(0);
    await tabs.getByRole("tab", { name: /Cancelled/ }).click();
    await expect(page.getByRole("link", { name: "Cancelled coffee chat" })).toBeVisible();
    await tabs.getByRole("tab", { name: /Upcoming/ }).click();

    await page.getByRole("link", { name: "Team check-in" }).click();
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}$`));
    await expect(page.getByRole("heading", { level: 2, name: "Team check-in" })).toBeVisible();
    await expect(page.getByText("You haven't answered yet.")).toBeVisible();

    const attendees = page.getByRole("region", { name: /Attendees/ });

    await expect(attendees).toContainText("1 going");
    await page.getByRole("button", { name: "Going" }).click();
    await expect(page.getByRole("button", { name: "Going" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(page.getByText("Currently: Going")).toBeVisible();
    await expect(attendees).toContainText("2 going");

    await page.getByRole("link", { name: /All events in/ }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
  });

  test("a new event asks for a title, then opens its page; Back doesn't reopen the form", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events`);
    await page.getByRole("link", { name: "New event" }).click();

    const dialog = page.getByRole("dialog", { name: "Schedule an event" });
    const title = dialog.getByLabel("Title");

    await expect(title).toBeFocused();
    await expect(dialog.getByText(/Times are in your time zone/)).toBeVisible();
    await dialog.getByRole("button", { name: "Schedule event" }).click();
    await expect(dialog.getByText("Give the event a title.")).toBeVisible();

    await title.fill("Launch retro");
    await dialog.getByLabel("Where").selectOption({ label: "Lounge" });
    await dialog.getByLabel("Repeats").selectOption({ label: "Weekly" });
    await expect(dialog.getByLabel("Until")).toBeVisible();
    await dialog.getByLabel("Repeats").selectOption({ label: "Does not repeat" });
    await dialog.getByRole("button", { name: "Schedule event" }).click();

    await expect(dialog).toBeHidden();
    await expect(page.getByText("Event scheduled.")).toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "Launch retro" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Lounge", exact: true })).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("editing a series' later event asks how far the change goes", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.engineering}/events/${WEEKLY_SECOND}`);
    await expect(page.getByText(/Repeats weekly/i)).toBeVisible();
    await expect(page.getByRole("link", { name: "Previous" })).toBeVisible();
    await expect(page.getByLabel("Apply to all future occurrences")).toBeVisible();

    await page.getByRole("link", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog", { name: "Edit event" });

    await expect(dialog.getByRole("radio", { name: "This event" })).toBeChecked();
    await expect(dialog.getByLabel("Repeats")).toHaveCount(0);
    await dialog.getByLabel("Title").fill("Engineering weekly (moved)");
    await expect(dialog.getByText("Unsaved changes")).toBeVisible();
    await dialog.getByRole("button", { name: "Save changes" }).click();

    await expect(dialog).toBeHidden();
    await expect(page.getByText("Event updated.")).toBeVisible();
    await expect(
      page.getByRole("heading", { level: 2, name: "Engineering weekly (moved)" }),
    ).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/events/${WEEKLY_SECOND}$`));

    await page.goBack();
    await expect(page).not.toHaveURL(/\/edit$/);
  });

  test("cancelling asks first, then closes responses", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await page.getByRole("button", { name: "Cancel event" }).click();

    const confirm = page.getByRole("alertdialog", { name: "Cancel this event?" });

    await expect(confirm.getByRole("button", { name: "Keep event" })).toBeFocused();
    await confirm.getByRole("button", { name: "Cancel event" }).click();

    await expect(confirm).toBeHidden();
    await expect(page.getByText("Event cancelled.")).toBeVisible();
    await expect(page.getByText("This event was cancelled.", { exact: true })).toBeVisible();
    await expect(
      page.getByText("Responses are closed because this event was cancelled."),
    ).toBeVisible();
    await expect(page.getByRole("link", { name: "Edit" })).toHaveCount(0);
  });

  test("the attendance URL opens the event on the viewer's response", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}/attendance`);

    await expect(page.getByRole("button", { name: "Going" })).toBeFocused();
  });

  test("answering twice keeps the last answer when the first one's read lands last", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await expect(page.getByText("You haven't answered yet.")).toBeVisible();

    const attendees = page.getByRole("region", { name: /Attendees/ });
    const first = await holdNextRead(page, `/rooms/${ROOM_IDS.general}/events/${UPCOMING}`);

    await page.getByRole("button", { name: "Going" }).click();
    // The server's snapshot after Going is taken; the page gets it only after Maybe's.
    await first.answered;
    await page.getByRole("button", { name: "Maybe" }).click();
    await expect(attendees).toContainText("1 going · 1 maybe");

    first.release();
    await first.delivered;
    await settle(page);

    await expect(page.getByText("Currently: Maybe")).toBeVisible();
    await expect(page.getByRole("button", { name: "Maybe" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(attendees).toContainText("1 going · 1 maybe");
  });

  test("an edit that saves after Back closed the form stays where Back went", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await page.getByRole("link", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog", { name: "Edit event" });

    await dialog.getByLabel("Title").fill("Team check-in (moved)");

    const release = await holdWrite(page, "PATCH", `/rooms/${ROOM_IDS.general}/events/${UPCOMING}`);

    await dialog.getByRole("button", { name: "Save changes" }).click();
    await page.goBack();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}$`));

    const historyCalls = await watchHistory(page);

    release();
    await expect(page.getByText("Event updated.")).toBeVisible();
    await settle(page);
    expect(await historyCalls()).toEqual([]);
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}$`));
    await expect(
      page.getByRole("heading", { level: 2, name: "Team check-in (moved)" }),
    ).toBeVisible();
  });

  test("an edit that saves after its form was closed and opened again leaves the new opening be", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await page.getByRole("link", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog", { name: "Edit event" });

    await dialog.getByLabel("Title").fill("Team check-in (moved)");

    const release = await holdWrite(page, "PATCH", `/rooms/${ROOM_IDS.general}/events/${UPCOMING}`);

    await dialog.getByRole("button", { name: "Save changes" }).click();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    // The same occurrence's form again, while the first save is still on its way.
    await page.getByRole("link", { name: "Edit" }).click();
    await expect(dialog.getByLabel("Title")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}/edit$`));

    const historyCalls = await watchHistory(page);

    release();
    await expect(page.getByText("Event updated.")).toBeVisible();
    await settle(page);

    // The earlier save neither closes this opening nor navigates away from it.
    expect(await historyCalls()).toEqual([]);
    await expect(dialog).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}/edit$`));
    await expect(dialog.getByRole("button", { name: "Save changes" })).toBeEnabled();
  });

  for (const dismissal of ["Escape", "Back"] as const) {
    test(`a new event that saves after ${dismissal} closed the form stays on the list`, async ({
      page,
    }) => {
      await openApp(page, `r/${ROOM_IDS.general}/events`);
      await page.getByRole("link", { name: "New event" }).click();

      const dialog = page.getByRole("dialog", { name: "Schedule an event" });

      await dialog.getByLabel("Title").fill("Launch retro");

      const release = await holdWrite(page, "POST", `/rooms/${ROOM_IDS.general}/events`);

      await dialog.getByRole("button", { name: "Schedule event" }).click();

      if (dismissal === "Escape") {
        await page.keyboard.press("Escape");
      } else {
        await page.goBack();
      }

      await expect(dialog).toBeHidden();
      await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));

      const historyCalls = await watchHistory(page);

      release();
      await expect(page.getByText("Event scheduled.")).toBeVisible();
      await settle(page);
      expect(await historyCalls()).toEqual([]);
      await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
      await expect(page.getByRole("link", { name: "Launch retro" })).toBeVisible();
    });
  }

  test("a save whose reply lands after a newer answer doesn't take the answer back", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await expect(page.getByText("You haven't answered yet.")).toBeVisible();
    await page.getByRole("link", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog", { name: "Edit event" });

    await dialog.getByLabel("Title").fill("Team check-in (moved)");

    const save = await holdNextRead(page, `/rooms/${ROOM_IDS.general}/events/${UPCOMING}`, "PATCH");

    await dialog.getByRole("button", { name: "Save changes" }).click();
    // The save's reply (moved, not answered) is taken; the page gets it only after Going's.
    await save.answered;
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await page.getByRole("button", { name: "Going" }).click();
    await expect(page.getByText("Currently: Going")).toBeVisible();
    // Let any read the answer's news set off finish first, so nothing newer than the save's reply
    // can come along after it and paper over a reply that wrongly landed.
    await settle(page);

    save.release();
    await save.delivered;
    await expect(page.getByText("Event updated.")).toBeVisible();
    await settle(page);

    await expect(page.getByText("Currently: Going")).toBeVisible();
    await expect(page.getByRole("button", { name: "Going" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(
      page.getByRole("heading", { level: 2, name: "Team check-in (moved)" }),
    ).toBeVisible();
  });

  test("another tab's edit and cancel reach an open list and event page", async ({
    page,
    context,
  }) => {
    const welcomed = syncWelcomed(page);

    await openApp(page, `r/${ROOM_IDS.general}/events`);
    await welcomed;

    // Scheduling posts the event's announcement, whose card later changes are told through.
    await page.getByRole("link", { name: "New event" }).click();

    const create = page.getByRole("dialog", { name: "Schedule an event" });

    await create.getByLabel("Title").fill("Sync check");
    await create.getByRole("button", { name: "Schedule event" }).click();
    await expect(page.getByRole("heading", { level: 2, name: "Sync check" })).toBeVisible();

    const eventId = Number(/\/events\/(\d+)$/.exec(page.url())?.[1]);

    await page.getByRole("link", { name: /All events in/ }).click();
    await expect(page.getByRole("link", { name: "Sync check" })).toBeVisible();

    const other = await context.newPage();

    await openApp(other, `r/${ROOM_IDS.general}/events/${eventId}`);
    await other.getByRole("link", { name: "Edit" }).click();

    const edit = other.getByRole("dialog", { name: "Edit event" });

    await edit.getByLabel("Title").fill("Sync check (renamed)");
    await edit.getByRole("button", { name: "Save changes" }).click();
    await expect(edit).toBeHidden();

    await expect(page.getByRole("link", { name: "Sync check (renamed)" })).toBeVisible();
    await page.getByRole("link", { name: "Sync check (renamed)" }).click();
    await expect(
      page.getByRole("heading", { level: 2, name: "Sync check (renamed)" }),
    ).toBeVisible();

    await other.getByRole("button", { name: "Cancel event" }).click();
    await other
      .getByRole("alertdialog", { name: "Cancel this event?" })
      .getByRole("button", { name: "Cancel event" })
      .click();
    await expect(other.getByText("This event was cancelled.", { exact: true })).toBeVisible();

    await expect(page.getByText("This event was cancelled.", { exact: true })).toBeVisible();
    await other.close();
  });

  test("another tab shortening the series takes a removed occurrence off an open page", async ({
    page,
    context,
  }) => {
    const welcomed = syncWelcomed(page);

    await openApp(page, `r/${ROOM_IDS.engineering}/events/${WEEKLY_LAST}`);
    await welcomed;
    await expect(page.getByRole("heading", { level: 2, name: "Engineering weekly" })).toBeVisible();

    const other = await context.newPage();

    await openApp(other, `r/${ROOM_IDS.engineering}/events/${WEEKLY_HEAD}`);
    await other.getByRole("link", { name: "Edit" }).click();

    const edit = other.getByRole("dialog", { name: "Edit event" });
    const until = edit.getByLabel("Until");

    // A week earlier, the series has no slot left for its last occurrence.
    const end = new Date(`${await until.inputValue()}T00:00:00Z`);

    end.setUTCDate(end.getUTCDate() - 7);
    await edit.getByRole("radio", { name: "This and following" }).check();
    await until.fill(end.toISOString().slice(0, 10));
    await edit.getByRole("button", { name: "Save changes" }).click();
    await expect(edit).toBeHidden();

    await expect(page.getByText("This event isn't here")).toBeVisible();
    await other.close();
  });
  test("an event page that missed its event's cancel while offline reads it on reconnect", async ({
    page,
    context,
  }) => {
    const sync = await interceptSync(page);

    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await sync.welcomed;
    await expect(page.getByRole("button", { name: "Cancel event" })).toBeVisible();
    sync.lose();

    const other = await context.newPage();

    await openApp(other, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await other.getByRole("button", { name: "Cancel event" }).click();
    await other
      .getByRole("alertdialog", { name: "Cancel this event?" })
      .getByRole("button", { name: "Cancel event" })
      .click();
    await expect(other.getByText("This event was cancelled.", { exact: true })).toBeVisible();
    await settle(page);
    // The news was lost: the page doesn't know yet.
    await expect(page.getByText("This event was cancelled.", { exact: true })).toHaveCount(0);

    await sync.reconnectUnresumed();

    await expect(page.getByText("This event was cancelled.", { exact: true })).toBeVisible();
    await other.close();
  });
});

test.describe("a prefilled new-event link", () => {
  test.use({ timezoneId: "Europe/Berlin" });

  test("a link shaped like the /event command's opens the form with its title and start", async ({
    page,
  }) => {
    await page.setViewportSize(DESKTOP);

    // The mock has no classic pages: this is the URL the classic /event link redirects to, Rails'
    // nested keys, sorted and form-encoded. The server test
    // `the_event_commands_link_opens_the_spa_form_with_its_prefill` pins that shape.
    const query = [
      "event%5Bstarts_at%5D=2030-03-08T22%3A00%3A00Z",
      "event%5Btime_zone%5D=Eastern+Time+%28US+%26+Canada%29",
      "event%5Btitle%5D=Launch+party",
    ].join("&");

    await openApp(page, `r/${ROOM_IDS.general}/events/new?${query}`);

    const dialog = page.getByRole("dialog", { name: "Schedule an event" });

    await expect(dialog.getByLabel("Title")).toHaveValue("Launch party");
    // 22:00 UTC is 23:00 in Berlin, the zone the form schedules it in.
    await expect(dialog.getByLabel("Starts")).toHaveValue("2030-03-08T23:00");
    await expect(dialog.getByText(/Times are in your time zone/)).toContainText("Europe/Berlin");
  });
});

// Screenshots only: the room's events tests above open the list, a page and the form.
if (SHOTS) {
  matrix("the events list and an event's page", async ({ page, theme }) => {
    await openApp(page, `r/${ROOM_IDS.engineering}/events`, theme);
    await expect(page.getByRole("link", { name: "Engineering weekly" }).first()).toBeVisible();
    await shot(page, "events-list", theme);

    await page.goto(`/app/r/${ROOM_IDS.engineering}/events/${WEEKLY_SECOND}`);
    await expect(page.getByRole("heading", { level: 2, name: "Engineering weekly" })).toBeVisible();
    await shot(page, "event-page", theme);

    await page.goto(`/app/r/${ROOM_IDS.engineering}/events/new`);
    await expect(
      page.getByRole("dialog", { name: "Schedule an event" }).getByLabel("Title"),
    ).toBeVisible();
    await shot(page, "event-form", theme);
  });
}
