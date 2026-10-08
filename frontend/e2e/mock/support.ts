import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import {
  type APIRequestContext,
  test as base,
  expect,
  type Locator,
  type Page,
} from "@playwright/test";

/** The seeded ids (mock/seed.ts). */
export { ROOM_IDS, USER_IDS } from "../../mock/seed.ts";

export const DESKTOP = { width: 1440, height: 900 } as const;

export const PHONE = { width: 390, height: 844 } as const;

/** Between the phone and desktop layouts: the right pane floats over the room as a sheet. */
export const TABLET = { width: 900, height: 1000 } as const;

export type Theme = "light" | "dark";

/** Wait for a native wheel gesture to move the list and finish scrolling. */
export async function scrollByWheel(page: Page, list: Locator, delta: number): Promise<void> {
  if (delta === 0) return;

  await list.hover();
  await list.evaluate((element) => {
    const offset = element.scrollTop;

    element.setAttribute("data-wheel-settled", "false");
    element.addEventListener(
      "wheel",
      () => {
        // Passive wheel listeners can run after the compositor has already scrolled.
        let moved = element.scrollTop !== offset;

        const scroll = () => {
          moved ||= element.scrollTop !== offset;
        };

        const end = (event: Event) => {
          if (event.target !== element || !moved) return;

          element.setAttribute("data-wheel-settled", "true");
          element.removeEventListener("scroll", scroll);
          element.removeEventListener("scrollend", end);
        };

        element.addEventListener("scroll", scroll);
        element.addEventListener("scrollend", end);
      },
      { capture: true, passive: true, once: true },
    );
  });
  await page.mouse.wheel(0, delta);
  await expect(list).toHaveAttribute("data-wheel-settled", "true");
}

/** Each test starts on a fresh seed: `/__mock/reset` (it also drops sync connections). */
export const test = base.extend<{ resetMock: undefined }>({
  resetMock: [
    async ({ request }, use) => {
      const state = await (await request.get("/__mock/state")).json();

      await request.post("/__mock/reset", { headers: { "X-CSRF-Token": state.csrfToken } });
      await use(undefined);
    },
    { auto: true },
  ],
});

export { expect };

interface HoldOptions {
  /**
   * Drops the frames other than the `welcome` that the server sends while held, as a socket that
   * was down never sees them: the welcome's refetch is all that brings their changes in.
   */
  readonly missed?: boolean;
}

/**
 * Holds every frame the sync socket sends until the returned function is called, so a test can
 * arrange the timeline before the first `welcome` (and the refetch it starts) arrives.
 */
export async function holdSync(
  page: Page,
  { missed = false }: HoldOptions = {},
): Promise<() => void> {
  const held = Promise.withResolvers<void>();
  let holding = true;

  await page.routeWebSocket(/\/api\/v1\/sync/, (socket) => {
    const server = socket.connectToServer();

    server.onMessage(async (message) => {
      if (missed && holding && !String(message).includes('"t":"welcome"')) {
        return;
      }

      await held.promise;
      socket.send(message);
    });
  });

  return () => {
    holding = false;
    held.resolve();
  };
}

interface MockPost {
  readonly roomId: number;
  readonly userId: number;
  readonly markdown: string;
}

/** Has someone post in a room through the mock's `/__mock/post` control. */
export async function postMessage(request: APIRequestContext, body: MockPost): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/post", { headers: { "X-CSRF-Token": state.csrfToken }, data: body });
}

/** Restores an ordinary thread for scenarios that need its full conversation viewport. */
export async function stopTrackingThread(
  request: APIRequestContext,
  threadId: number,
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.patch(`/api/v1/threads/${threadId}/work`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { status: null, ownerId: null },
  });

  expect(response.ok()).toBe(true);
}

/** Opens the app at `path` (under /app/) in `theme`, with motion reduced so shots are settled. */
/** Saves the account's appearance through the API, as another device (or person) would. */
export async function saveAccountAppearance(
  page: Page,
  change: Partial<Record<"theme" | "textSize" | "timeZone", string>>,
): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.patch("/api/v1/settings/appearance", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { theme: null, textSize: null, timeZone: null, ...change },
  });

  if (!response.ok()) {
    throw new Error(`saving the appearance answered ${response.status()}`);
  }
}

export async function openApp(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path.replace(/^\//, "")}`);
  // Phones show one column: the conversation list, or the open conversation.
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.getByRole("main"))
    .first()
    .waitFor();
}

/**
 * Resolves once the page's sync socket is welcomed. Call it before the page opens; await it
 * before a step that publishes an event the page must receive live. Until the welcome, a change
 * made elsewhere arrives only through the catch-up reload, not as an event.
 */
export function syncWelcomed(page: Page): Promise<void> {
  return page
    .waitForEvent("websocket", (socket) => socket.url().includes("/api/v1/sync"))
    .then((socket) =>
      socket.waitForEvent("framereceived", (frame) =>
        String(frame.payload).includes('"t":"welcome"'),
      ),
    )
    .then(() => undefined);
}

const SHOTS = process.env.SMARTFIRE_SHOTS === "1";

const SHOTS_DIR = process.env.SMARTFIRE_SHOTS_DIR ?? join(homedir(), ".cache/frontend-s2/shots");

/**
 * Saves `<name>-<theme>-<desktop|tablet|phone>.png` when `SMARTFIRE_SHOTS=1`; otherwise a no-op, so the
 * specs double as the screenshot script without writing files on every run.
 */
export async function shot(page: Page, name: string, theme: Theme): Promise<void> {
  if (!SHOTS) return;

  mkdirSync(SHOTS_DIR, { recursive: true });

  const width = page.viewportSize()?.width ?? DESKTOP.width;
  const size = width < 720 ? "phone" : width < 1100 ? "tablet" : "desktop";

  await page.screenshot({ path: join(SHOTS_DIR, `${name}-${theme}-${size}.png`) });
}

/** Runs `body` once per theme and viewport: four combinations, each its own test. */
export function matrix(
  title: string,
  body: (context: { page: Page; theme: Theme; phone: boolean }) => Promise<void>,
): void {
  for (const theme of ["light", "dark"] as const) {
    for (const [label, viewport] of [
      ["desktop", DESKTOP],
      ["phone", PHONE],
    ] as const) {
      test(`${title} (${theme}, ${label})`, async ({ page }) => {
        await page.setViewportSize(viewport);
        await body({ page, theme, phone: viewport === PHONE });
      });
    }
  }
}
