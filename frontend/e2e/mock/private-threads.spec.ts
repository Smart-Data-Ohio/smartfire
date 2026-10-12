import type { APIRequestContext, APIResponse, Page } from "@playwright/test";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import { DESKTOP, expect, PHONE_TOUCH, ROOM_IDS, syncWelcomed, test, USER_IDS } from "./support.ts";

const GENERAL = ROOM_IDS.general;

/** The bodies of the mock controls this spec uses. */
type ControlBody =
  | { readonly role: "member" }
  | { readonly threadId: number; readonly userId: number }
  | { readonly threadId: number; readonly userId: number; readonly markdown: string }
  | {
      readonly threadId: number;
      readonly userId: number;
      readonly actorId: number;
      readonly added: boolean;
    }
  | {
      readonly roomId: number;
      readonly creatorId: number;
      readonly name: string;
      readonly markdown: string;
      readonly withViewer?: boolean;
      readonly notifyViewer?: boolean;
    };

/** Runs one of the mock's controls, as another person's client would act. */
async function control(
  request: APIRequestContext,
  action: string,
  data: ControlBody,
): Promise<APIResponse> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok(), `${action} answered ${response.status()}`).toBe(true);

  return response;
}

interface Readable {
  readonly readable: boolean;
}

/** Whether `userId` can see the thread, as the server decides it. */
async function readable(
  request: APIRequestContext,
  threadId: number,
  userId: number,
): Promise<boolean> {
  const body: Readable = await (
    await control(request, "thread-readable", { threadId, userId })
  ).json();

  return body.readable;
}

interface PrivateThreadStart {
  readonly name: string;
  readonly markdown: string;
  readonly withViewer?: boolean;
  /** Its first reply also lands in the viewer's Activity inbox. */
  readonly notifyViewer?: boolean;
}

/** Maya gives the viewer access to the thread (`added`) or takes it away. */
async function mayaShares(request: APIRequestContext, threadId: number, added: boolean) {
  await control(request, "thread-access", {
    threadId,
    userId: USER_IDS.riel,
    actorId: USER_IDS.maya,
    added,
  });
}

interface PrivateThreadStarted {
  readonly threadId: number;
  readonly parentMessageId: number;
}

/** Maya starts a private thread in #general, sharing it with the viewer or not. */
async function mayaStartsPrivate(
  request: APIRequestContext,
  start: PrivateThreadStart,
): Promise<number> {
  const body: PrivateThreadStarted = await (
    await control(request, "private-thread", {
      roomId: GENERAL,
      creatorId: USER_IDS.maya,
      ...start,
    })
  ).json();

  return body.threadId;
}

/** Opens the app at `path` as an ordinary member, who sees only the private threads shared. */
async function openAsMember(page: Page, path: string): Promise<void> {
  await control(page.request, "viewer-role", { role: "member" });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

const SIZES = [
  { label: "desktop", use: { viewport: DESKTOP } },
  { label: "phone", use: PHONE_TOUCH },
] as const;

for (const size of SIZES) {
  test.describe(`private threads (${size.label})`, () => {
    test.use(size.use);

    test("start one, add someone, and see its lock", async ({ page }) => {
      await openAsMember(page, `r/${GENERAL}/t/new?parent=${MESSAGE_IDS.generalChart}`);

      const toggle = pane(page).getByRole("switch", { name: /Private thread/ });

      await expect(toggle).toHaveAttribute("aria-checked", "false");
      await toggle.click();
      await expect(toggle).toHaveAttribute("aria-checked", "true");
      await expect(pane(page).getByRole("heading", { name: "New private thread" })).toBeVisible();

      await pane(page).getByLabel("Thread name (optional)").fill("Chart, just us");

      const composer = pane(page).getByRole("textbox", { name: "Reply…" });

      await composer.click();
      await composer.fill("Keeping this one small for now.");
      await composer.press("Enter");

      await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}/t/\\d+$`));
      await expect(pane(page).getByRole("heading", { name: "Chart, just us" })).toBeVisible();
      await expect(pane(page).locator(".thread-status[data-status='private']")).toHaveText(
        "Private",
      );

      const threadId = Number(new URL(page.url()).pathname.split("/").at(-1));

      // Only its creator so far: Maya, in the room, can't see it yet.
      expect(await readable(page.request, threadId, USER_IDS.maya)).toBe(false);

      const members = pane(page).getByRole("button", { name: "Members: 1 person" });

      await members.click();

      const dialog = page.getByRole("dialog", { name: "Private thread members" });

      await expect(dialog.getByRole("listitem")).toHaveCount(1);
      await dialog.getByRole("combobox", { name: "Add people" }).fill("Maya");
      await dialog.getByRole("option", { name: /Maya/ }).click();
      await expect(dialog.getByRole("listitem")).toHaveCount(2);
      await expect(dialog.getByRole("listitem").nth(1)).toContainText("Maya");

      // Shared with Maya now; Jonah, also in the room, still can't see it.
      expect(await readable(page.request, threadId, USER_IDS.maya)).toBe(true);
      expect(await readable(page.request, threadId, USER_IDS.jonah)).toBe(false);

      await dialog.getByRole("button", { name: "Done" }).click();
      await expect(dialog).toHaveCount(0);
      await expect(pane(page).getByRole("button", { name: "Members: 2 people" })).toBeVisible();

      // The Threads pane lists it with its lock.
      await page.goto(`/app/r/${GENERAL}/threads`);
      await pane(page).getByRole("tab", { name: "All" }).click();

      const row = pane(page).locator(".thread-row", { hasText: "Chart, just us" });

      await expect(row.getByRole("img", { name: "Private" })).toBeVisible();
    });

    test("a member sees a thread shared with them, and loses it when removed", async ({ page }) => {
      await control(page.request, "viewer-role", { role: "member" });

      const shared = await mayaStartsPrivate(page.request, {
        name: "Maya and Riel",
        markdown: "Can we talk about the launch date?",
        withViewer: true,
      });

      const hidden = await mayaStartsPrivate(page.request, {
        name: "Maya's own notes",
        markdown: "Notes for me.",
      });

      expect(await readable(page.request, hidden, USER_IDS.riel)).toBe(false);

      await openAsMember(page, `r/${GENERAL}/threads`);
      await pane(page).getByRole("tab", { name: "All" }).click();

      const row = pane(page).locator(".thread-row", { hasText: "Maya and Riel" });

      await expect(row.getByRole("img", { name: "Private" })).toBeVisible();
      await expect(pane(page).locator(".thread-row", { hasText: "Maya's own notes" })).toHaveCount(
        0,
      );

      // The reply summary under its message carries the lock; the hidden thread has none.
      const welcomed = syncWelcomed(page);

      await page.goto(`/app/r/${GENERAL}`);
      await page.getByRole("button", { name: /^Jump to present$|new messages?$/ }).click();

      const summary = page.getByRole("button", { name: /^Private thread\. 1 reply/ });

      await expect(summary).toHaveCount(1);
      await summary.evaluate((element) =>
        element.scrollIntoView({ block: "center", behavior: "instant" }),
      );
      await expect(summary.locator(".thread-private-glyph")).toBeVisible();

      await summary.click();
      await expect(pane(page).getByRole("heading", { name: "Maya and Riel" })).toBeVisible();
      await expect(pane(page).getByText("Can we talk about the launch date?")).toBeVisible();
      await expect(pane(page).locator(".thread-status[data-status='private']")).toBeVisible();

      // Maya takes it back, and the open pane says so.
      await welcomed;
      await control(page.request, "thread-access", {
        threadId: shared,
        userId: USER_IDS.riel,
        actorId: USER_IDS.maya,
        added: false,
      });

      await expect(
        pane(page).getByText("You no longer have access to this private thread."),
      ).toBeVisible();
      expect(await readable(page.request, shared, USER_IDS.riel)).toBe(false);
    });

    test("a closed thread's reply summary shows its lock before opening it", async ({ page }) => {
      await control(page.request, "viewer-role", { role: "member" });
      const started: PrivateThreadStarted = await (
        await control(page.request, "private-thread", {
          roomId: GENERAL,
          creatorId: USER_IDS.riel,
          name: "Closed private discussion",
          markdown: "A reply in a closed private thread.",
        })
      ).json();
      const state = await (await page.request.get("/__mock/state")).json();
      const closed = await page.request.patch(`/api/v1/threads/${started.threadId}`, {
        headers: { "X-CSRF-Token": state.csrfToken },
        data: { status: "closed" },
      });

      expect(closed.ok()).toBe(true);
      await openAsMember(page, `r/${GENERAL}`);
      await page.getByRole("button", { name: /^Jump to present$|new messages?$/ }).click();

      const summary = page.getByRole("button", { name: /^Private thread\. 1 reply/ });

      await expect(summary).toHaveCount(1);
      await summary.evaluate((element) =>
        element.scrollIntoView({ block: "center", behavior: "instant" }),
      );
      await expect(summary.getByRole("img", { name: "Private" })).toBeVisible();
    });

    test("a removed member who reloads gets live replies as soon as re-added", async ({ page }) => {
      await control(page.request, "viewer-role", { role: "member" });
      const threadId = await mayaStartsPrivate(page.request, {
        name: "Re-added after reload",
        markdown: "The original private reply.",
        withViewer: true,
      });
      const welcomed = syncWelcomed(page);

      await openAsMember(page, `r/${GENERAL}/t/${threadId}`);
      await expect(
        pane(page).getByRole("heading", { name: "Re-added after reload" }),
      ).toBeVisible();
      await welcomed;
      await mayaShares(page.request, threadId, false);
      await expect(
        pane(page).getByText("You no longer have access to this private thread."),
      ).toBeVisible();

      const reconnected = syncWelcomed(page);

      await page.reload();
      await expect(pane(page).getByRole("alert")).toBeVisible();
      await reconnected;
      await mayaShares(page.request, threadId, true);
      await expect(
        pane(page).getByRole("heading", { name: "Re-added after reload" }),
      ).toBeVisible();
      await expect(pane(page).getByText("The original private reply.")).toBeVisible();
      await control(page.request, "thread-post", {
        threadId,
        userId: USER_IDS.maya,
        markdown: "A live reply after re-add, without another reload.",
      });
      await expect(
        pane(page).getByText("A live reply after re-add, without another reload."),
      ).toBeVisible();
    });

    test("someone removed and added back gets the open thread back live", async ({ page }) => {
      await control(page.request, "viewer-role", { role: "member" });

      const threadId = await mayaStartsPrivate(page.request, {
        name: "Maya and Riel",
        markdown: "Can we talk about the launch date?",
        withViewer: true,
      });

      const welcomed = syncWelcomed(page);

      await openAsMember(page, `r/${GENERAL}/t/${threadId}`);
      await expect(pane(page).getByRole("heading", { name: "Maya and Riel" })).toBeVisible();
      await welcomed;

      await mayaShares(page.request, threadId, false);
      await expect(
        pane(page).getByText("You no longer have access to this private thread."),
      ).toBeVisible();
      await expect(pane(page).getByText("Can we talk about the launch date?")).toHaveCount(0);

      await mayaShares(page.request, threadId, true);
      await expect(pane(page).getByRole("heading", { name: "Maya and Riel" })).toBeVisible();
      await expect(pane(page).getByText("Can we talk about the launch date?")).toBeVisible();

      // Following the thread again: Maya's next reply arrives without a reload.
      await control(page.request, "thread-post", {
        threadId,
        userId: USER_IDS.maya,
        markdown: "Glad you're back.",
      });
      await expect(pane(page).getByText("Glad you're back.")).toBeVisible();
    });
  });
}

test.describe("private threads, cached elsewhere (desktop)", () => {
  test.use({ viewport: DESKTOP });

  test("nothing of a lost thread stays in Activity or the switcher", async ({ page }) => {
    await control(page.request, "viewer-role", { role: "member" });

    const threadId = await mayaStartsPrivate(page.request, {
      name: "Launch date, quietly",
      markdown: "Can we move the launch to Friday?",
      withViewer: true,
      notifyViewer: true,
    });

    const welcomed = syncWelcomed(page);

    await openAsMember(page, "activity");

    const excerpt = page.getByText("Can we move the launch to Friday?");

    await expect(excerpt).toBeVisible();

    const switcher = page.getByRole("dialog", { name: "Jump to a conversation" });
    const option = switcher.getByRole("option", { name: /Launch date, quietly/ });

    await page.keyboard.press("ControlOrMeta+k");
    await expect(switcher).toBeVisible();
    await page.keyboard.type("Launch date");
    await expect(option).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(switcher).toBeHidden();

    await welcomed;
    await mayaShares(page.request, threadId, false);

    await expect(excerpt).toHaveCount(0);

    // The switcher's next answer never comes, so only its cache could still name the thread.
    await page.route("**/api/v1/switcher", () => undefined);
    await page.keyboard.press("ControlOrMeta+k");
    await expect(switcher).toBeVisible();
    await page.keyboard.type("Launch date");
    await expect(option).toHaveCount(0);
  });
});
