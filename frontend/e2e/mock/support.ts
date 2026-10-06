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
