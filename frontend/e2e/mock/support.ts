import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { test as base, type Page } from "@playwright/test";

/** The seeded ids (mock/seed.ts). */
export { ROOM_IDS, USER_IDS } from "../../mock/seed.ts";

export const DESKTOP = { width: 1440, height: 900 } as const;

export const PHONE = { width: 390, height: 844 } as const;

/** Between the phone and desktop layouts: the right pane floats over the room as a sheet. */
export const TABLET = { width: 900, height: 1000 } as const;

export type Theme = "light" | "dark";

/** What `watchSync` keeps on `window`, per document. */
interface SyncWatch {
  welcomes: number;
  inflight: number;
  quietSince: number;
}

declare global {
  interface Window {
    __smartfireSync?: SyncWatch;
  }
}

/**
 * Runs in every document before the app: counts the sync socket's `welcome` frames and the
 * `fetch`es in flight, for `synced`.
 */
function watchSync(): void {
  const watch: SyncWatch = { welcomes: 0, inflight: 0, quietSince: performance.now() };
  const NativeWebSocket = window.WebSocket;
  const nativeFetch = window.fetch.bind(window);

  window.__smartfireSync = watch;

  window.WebSocket = class extends NativeWebSocket {
    constructor(url: string | URL, protocols?: string | string[]) {
      super(url, protocols);
      this.addEventListener("message", (event: MessageEvent) => {
        // The refetches it starts begin a moment later: the quiet period restarts here.
        if (String(event.data).includes('"t":"welcome"')) {
          watch.welcomes += 1;
          watch.quietSince = performance.now();
        }
      });
    }
  };

  window.fetch = async (...args: Parameters<typeof fetch>) => {
    watch.inflight += 1;

    try {
      return await nativeFetch(...args);
    } finally {
      watch.inflight -= 1;
      watch.quietSince = performance.now();
    }
  };
}

/**
 * Each test starts on a fresh seed: `/__mock/reset` (it also drops sync connections). Every
 * document also gets `watchSync`, for `synced`.
 */
export const test = base.extend<{ resetMock: undefined }>({
  resetMock: [
    async ({ request, page }, use) => {
      const state = await (await request.get("/__mock/state")).json();

      await request.post("/__mock/reset", { headers: { "X-CSRF-Token": state.csrfToken } });
      await page.addInitScript(watchSync);
      await use(undefined);
    },
    { auto: true },
  ],
});

/** No request in flight for this long after the welcome counts as settled. */
const SETTLED_MS = 300;

/**
 * Waits until the sync socket's first `welcome` has been handled: a fresh connection refetches
 * the sidebar and every open room's newest page. In #general, whose 52 unread open it on a window
 * around the first unread, that refetch replaces the window (and the view jumps to the bottom,
 * closing menus and unmounting rows) whenever a page of newer messages reached the present first;
 * otherwise it re-reads the window in place. Either way, act once it has landed.
 */
export async function synced(page: Page): Promise<void> {
  await page.waitForFunction((settledMs) => {
    const watch = window.__smartfireSync;

    return (
      watch !== undefined &&
      watch.welcomes > 0 &&
      watch.inflight === 0 &&
      performance.now() - watch.quietSince >= settledMs
    );
  }, SETTLED_MS);
}

export { expect } from "@playwright/test";

/** Opens the app at `path` (under /app/) in `theme`, with motion reduced so shots are settled. */
export async function openApp(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path.replace(/^\//, "")}`);
  // Phones show one column: the conversation list, or the open conversation.
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.getByRole("main"))
    .first()
    .waitFor();
  await synced(page);
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
