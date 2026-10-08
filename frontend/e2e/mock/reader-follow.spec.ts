import type { Locator, Page } from "@playwright/test";
import { THREAD_IDS } from "../../mock/s2/seed.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import { expect, holdSync, openApp, ROOM_IDS, test, USER_IDS } from "./support.ts";

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

async function scenario(page: Page, conversation: "room" | "thread") {
  const releaseSync = await holdSync(page);
  const threadId = THREAD_IDS.generalActive;

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
