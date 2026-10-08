import type { Locator, Page } from "@playwright/test";
import { THREAD_IDS } from "../../mock/s2/seed.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import {
  expect,
  holdSync,
  openApp,
  postMessage,
  ROOM_IDS,
  stopTrackingThread,
  test,
  USER_IDS,
} from "./support.ts";

async function position(row: Locator) {
  return row.evaluate(async (element) => {
    const list = element.closest('[role="log"]');

    if (!list) throw new Error("The reader row is outside the list");

    const measure = () => element.getBoundingClientRect().top - list.getBoundingClientRect().top;

    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    const before = measure();

    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

    return { before, after: measure() };
  });
}

async function stableTop(row: Locator) {
  let top = 0;

  await expect
    .poll(async () => {
      const measured = await position(row);

      top = measured.after;

      return Math.abs(measured.after - measured.before);
    })
    .toBeLessThanOrEqual(1);

  return top;
}

async function atEnd(list: Locator) {
  await expect
    .poll(() =>
      list.evaluate((element) => element.scrollHeight - element.clientHeight - element.scrollTop),
    )
    .toBeLessThanOrEqual(1);
}

async function readerScroll(list: Locator, target: Locator) {
  const id = await target.getAttribute("data-message-id");

  await list.evaluate(
    (element, id) =>
      new Promise<void>((resolve) => {
        const row = element.querySelector(`[data-message-id="${id}"]`);

        if (!row) throw new Error("The older row is not rendered");

        element.addEventListener("scrollend", () => resolve(), { once: true });
        // An external scroll writer stands in for browser find-in-page or autoscroll.
        element.scrollTop +=
          row.getBoundingClientRect().top - element.getBoundingClientRect().top - 40;
      }),
    id,
  );
}

async function bottomGrowth(page: Page, mixed = false) {
  const releaseSync = await holdSync(page);
  const requested = Promise.withResolvers<void>();
  const released = Promise.withResolvers<void>();
  const messagesPath = `/api/v1/rooms/${ROOM_IDS.engineering}/messages`;
  let hiddenId = 0;
  let newestId = 0;

  await page.route("**/__bottom-growth.svg", async (route) => {
    requested.resolve();
    await released.promise;
    await route.fulfill({
      contentType: "image/svg+xml",
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="180"><rect width="100" height="180" fill="teal"/></svg>',
    });
  });
  await page.route(`**${messagesPath}**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    if (newestId === 0) {
      hiddenId = body.messages.at(-3)?.id ?? 0;
      newestId = body.messages.at(-1)?.id ?? 0;
    }

    body.messages = body.messages.map((message) => ({
      ...message,
      bodyHtml:
        message.id > newestId
          ? message.bodyHtml
          : message.id === newestId
            ? '<div style="height:560px">Newest tall row</div><img src="/__bottom-growth.svg" alt="Newest growth"><p>Newest row ends here</p>'
            : mixed && message.id === hiddenId
              ? '<p>Hidden overscan row</p><img src="/__bottom-growth.svg" alt="Hidden growth">'
              : '<div style="height:100px">A preceding row</div>',
      attachment: null,
      cards: [],
      poll: null,
    }));
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });
  await openApp(page, `r/${ROOM_IDS.engineering}`);
  await requested.promise;
  const refreshed = page.waitForResponse((response) => response.url().endsWith(messagesPath));

  releaseSync();
  await (await refreshed).finished();
  const list = page.getByRole("log", { name: "Messages" });
  const newest = list.locator(`[data-message-id="${newestId}"]`);
  const hidden = list.locator(`[data-message-id="${hiddenId}"]`);

  await atEnd(list);
  await stableTop(newest);

  const post = async () => {
    const text = "Another member posts after bottom growth";

    await postMessage(page.request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.jonah,
      markdown: text,
    });

    return list.getByText(text, { exact: true });
  };

  return { list, newest, hidden, grow: () => released.resolve(), post };
}

test("mixed hidden and visible growth preserves follow for the next member's message", async ({
  page,
}) => {
  const { list, newest, hidden, grow, post } = await bottomGrowth(page, true);
  const hiddenHeight = await hidden.evaluate((element) => element.getBoundingClientRect().height);
  const visibleHeight = await newest.evaluate((element) => element.getBoundingClientRect().height);

  await expect(hidden).not.toBeInViewport();
  await expect(newest).toBeInViewport();
  grow();
  await expect
    .poll(() => hidden.evaluate((element) => element.getBoundingClientRect().height))
    .toBeGreaterThan(hiddenHeight + 100);
  await expect
    .poll(() => newest.evaluate((element) => element.getBoundingClientRect().height))
    .toBeGreaterThan(visibleHeight + 100);
  await atEnd(list);
  await expect(await post()).toBeInViewport();
  await atEnd(list);
});

for (const close of ["Esc", "outside click"] as const) {
  test(`newest-row menu defers image growth and replays follow after ${close}`, async ({
    page,
  }) => {
    const { list, newest, grow, post } = await bottomGrowth(page);
    const height = await newest.evaluate((element) => element.getBoundingClientRect().height);

    await newest.getByText("Newest row ends here", { exact: true }).click({ button: "right" });
    await expect(page.getByRole("menu", { name: "Message actions" })).toBeVisible();
    const offset = await list.evaluate((element) => element.scrollTop);

    grow();
    await expect
      .poll(() => newest.evaluate((element) => element.getBoundingClientRect().height))
      .toBeGreaterThan(height + 100);
    await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(offset);

    if (close === "Esc") await page.keyboard.press("Escape");
    else await page.locator(".composer-input").first().click();

    await expect(page.getByRole("menu", { name: "Message actions" })).not.toBeVisible();
    await atEnd(list);
    await expect(await post()).toBeInViewport();
    await atEnd(list);
  });
}

async function scenario(page: Page, conversation: "room" | "thread") {
  const releaseSync = await holdSync(page);
  const threadId = THREAD_IDS.generalActive;

  if (conversation === "thread") {
    // The work seed now tracks this thread. Keep the original ordinary-thread viewport so
    // the older control is offscreen but still mounted in Virtua's buffer for native focus.
    await stopTrackingThread(page.request, threadId);
  }

  const messagesPath =
    conversation === "room"
      ? `/api/v1/rooms/${ROOM_IDS.engineering}/messages`
      : `/api/v1/threads/${threadId}/messages`;

  const imageRequested = Promise.withResolvers<void>();
  const imageReleased = Promise.withResolvers<void>();
  let olderId = 0;

  await page.route("**/__reader-growth.svg", async (route) => {
    imageRequested.resolve();
    await imageReleased.promise;
    await route.fulfill({
      contentType: "image/svg+xml",
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="120"><rect width="100" height="120" fill="teal"/></svg>',
    });
  });
  await page.route(`**${messagesPath}**`, async (route) => {
    const response = await route.fetch();
    const body: MessagePage = await response.json();

    olderId = body.messages.at(-2)?.id ?? 0;
    body.messages = body.messages.map((message, index) => ({
      ...message,
      bodyHtml:
        index === body.messages.length - 1
          ? '<p><a href="/" data-reader-control>Newest reader link</a></p><div style="height:760px">The newest tall message</div>'
          : index === body.messages.length - 2
            ? '<p><a href="/" data-reader-control>Older reader link</a></p><img src="/__reader-growth.svg" alt="Reader growth"><p>The older row ends here</p>'
            : "<p>A preceding message</p>",
      attachment: null,
      cards: [],
      poll: null,
    }));
    body.before = null;
    body.after = null;
    await route.fulfill({ response, json: body });
  });

  await openApp(
    page,
    conversation === "room" ? `r/${ROOM_IDS.engineering}` : `r/${ROOM_IDS.general}/t/${threadId}`,
  );
  await imageRequested.promise;

  const refreshed = page.waitForResponse((response) => response.url().endsWith(messagesPath));

  releaseSync();
  await (await refreshed).finished();

  const list =
    conversation === "room"
      ? page.getByRole("log", { name: "Messages" })
      : page.getByRole("log", { name: "Replies" });

  const older = list.locator(`[data-message-row][data-message-id="${olderId}"]`);

  await atEnd(list);
  await stableTop(older);
  await expect(older).not.toBeInViewport();

  const post = async () => {
    const state = await (await page.request.get("/__mock/state")).json();
    const text = "Another member posts after reader movement";

    const response = await page.request.post(
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

    expect(response.ok()).toBe(true);
    const message: MessagePage["messages"][number] = await response.json();

    // An older reader may leave the new row outside Virtua's mounted window. Confirm
    // delivery in the live timeline before testing that it did not move the viewport.
    await expect
      .poll(() =>
        page.evaluate(
          async ({ conversation, threadId, roomId, id }) => {
            const modulePath = "/app/src/store/store.ts";

            // SAFETY: This fixed Vite URL serves the store.ts module named by the type import.
            const { store } = (await import(
              modulePath
            )) as typeof import("../../src/store/store.ts");

            const state = store.getState();

            const timeline =
              conversation === "room" ? state.timelines[roomId] : state.threadTimelines[threadId];

            return timeline?.ids.includes(id) ?? false;
          },
          { conversation, threadId, roomId: ROOM_IDS.engineering, id: message.id },
        ),
      )
      .toBe(true);

    return list.getByText(text, { exact: true });
  };

  const stays = async (top: number) => {
    await expect(older).toBeInViewport();
    await expect
      .poll(async () => {
        const measured = await position(older);

        return Math.max(Math.abs(measured.before - top), Math.abs(measured.after - top));
      })
      .toBeLessThanOrEqual(3);
  };

  return { list, older, post, stays, grow: () => imageReleased.resolve() };
}

for (const conversation of ["room", "thread"] as const) {
  if (conversation === "room") {
    test("a forward external scroll interrupts Jump to present short of the end", async ({
      page,
    }) => {
      const { list, older, post } = await scenario(page, conversation);

      await readerScroll(list, older);
      await list.evaluate((element) => {
        element.scrollTop = 0;
      });
      await expect(list).toHaveAttribute("data-scroll-settled", "true");
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await expect(page.getByRole("button", { name: "Jump to present" })).toBeVisible();

      const interrupted = list.evaluate(
        (element) =>
          new Promise<number>((resolve) => {
            const start = element.scrollTop;

            const interrupt = () => {
              const end = element.scrollHeight - element.clientHeight;

              if (element.scrollTop <= start + 20 || element.scrollTop >= end - 120) return;

              element.removeEventListener("scroll", interrupt);
              // A forward external writer stands in for find-in-page during native smooth scroll.
              const target = Math.min(end - 120, element.scrollTop + 80);

              element.addEventListener("scrollend", () => resolve(element.scrollTop), {
                once: true,
              });
              element.scrollTop = target;
            };

            element.addEventListener("scroll", interrupt);
          }),
      );

      await page.getByRole("button", { name: "Jump to present" }).click();
      const stopped = await interrupted;

      await expect
        .poll(() =>
          list.evaluate(
            (element) => element.scrollHeight - element.clientHeight - element.scrollTop,
          ),
        )
        .toBeGreaterThan(80);
      const incoming = await post();

      await expect.poll(() => list.evaluate((element) => element.scrollTop)).toBe(stopped);
      await expect(incoming).not.toBeInViewport();
    });
  }

  for (const focus of ["programmatic focus", "Shift+Tab"] as const) {
    test(`a ${conversation} keeps an older control in view after ${focus} and another member's post`, async ({
      page,
    }) => {
      const { list, older, post, stays } = await scenario(page, conversation);
      const link = older.getByRole("link", { name: "Older reader link" });

      if (focus === "programmatic focus") await link.evaluate((element) => element.focus());
      else {
        await list.evaluate((element) => {
          for (const control of element.querySelectorAll<HTMLElement>("a, button, [tabindex]"))
            control.tabIndex = control.hasAttribute("data-reader-control") ? 0 : -1;

          const newest = Array.from(
            element.querySelectorAll<HTMLElement>("[data-reader-control]"),
          ).at(-1);

          newest?.focus({ preventScroll: true });
        });
        await page.keyboard.press("Shift+Tab");
      }

      await expect(link).toBeFocused();
      await expect(link).toBeInViewport();
      const top = await stableTop(older);
      const incoming = await post();

      await stays(top);
      await expect(link).toBeFocused();
      await expect(incoming).not.toBeInViewport();
    });
  }

  for (const input of ["find-in-page", "middle-button autoscroll"] as const) {
    test(`a ${conversation} keeps its reader position after ${input} and visible row growth`, async ({
      page,
    }) => {
      const { list, older, grow, stays, post } = await scenario(page, conversation);

      if (input === "middle-button autoscroll") {
        const bounds = await list.boundingBox();

        if (!bounds) throw new Error("The list is not visible");

        await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
        await page.mouse.down({ button: "middle" });
        await page.mouse.up({ button: "middle" });
      }

      await readerScroll(list, older);
      await expect(older).toBeInViewport();
      const top = await stableTop(older);
      const height = await older.evaluate((element) => element.getBoundingClientRect().height);

      grow();
      await expect
        .poll(() => older.evaluate((element) => element.getBoundingClientRect().height))
        .toBeGreaterThan(height + 40);
      await stays(top);
      const incoming = await post();

      await stays(top);
      await expect(incoming).not.toBeInViewport();
    });
  }

  test(`a ${conversation} follows another member after End returns the reader to the bottom`, async ({
    page,
  }) => {
    const { list, older, post } = await scenario(page, conversation);

    await readerScroll(list, older);
    await list.focus();
    await list.press("End");
    await atEnd(list);
    const incoming = await post();

    await expect(incoming).toBeInViewport();
    await atEnd(list);
  });
}
