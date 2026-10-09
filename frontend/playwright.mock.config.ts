import { defineConfig, devices } from "@playwright/test";

// `SMARTFIRE_E2E_PORT` lets parallel worktrees run their own servers.
const port = Number(process.env.SMARTFIRE_E2E_PORT ?? 4180);

// `SMARTFIRE_E2E_BUILD=1` serves the production build (`vite build`, then `vite preview` with the
// same mock) instead of the dev server, for what only the build can get wrong: its CSS chunk
// order, its chunking. CI runs e2e/mock/inline-layout.spec.ts this way.
const build = process.env.SMARTFIRE_E2E_BUILD === "1";

const server = build
  ? `pnpm exec vite build && pnpm exec vite preview --mode mock --host 127.0.0.1 --port ${port} --strictPort`
  : `pnpm exec vite --mode mock --host 127.0.0.1 --port ${port} --strictPort`;

/**
 * The S2 specs: the SPA on Vite's dev server with the in-memory mock backend (mock/vite-plugin.ts),
 * the simulation off so every run sees the same seeded workspace. `pnpm test:e2e:mock`.
 * `SMARTFIRE_SHOTS=1` also writes screenshots to `SMARTFIRE_SHOTS_DIR` (default
 * ~/.cache/frontend-s2/shots).
 */
export default defineConfig({
  testDir: "e2e/mock",
  forbidOnly: Boolean(process.env.CI),
  // One retry in CI: a test that only passes on its retry is reported as flaky rather than failing
  // the pull request, since one transient failure among hundreds of browser tests would block a merge.
  retries: process.env.CI ? 1 : 0,
  fullyParallel: false,
  workers: 1,
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: "retain-on-failure",
    // The huddle specs' device check needs a microphone and camera; Chromium fakes both.
    launchOptions: {
      args: ["--use-fake-ui-for-media-stream", "--use-fake-device-for-media-stream"],
    },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: server,
    env: { SMARTFIRE_MOCK_SIMULATE: "0" },
    url: `http://127.0.0.1:${port}/app/`,
    // A production-build run must serve the build it just made, never a dev server left running.
    reuseExistingServer: !build && !process.env.CI,
    timeout: build ? 180_000 : 60_000,
  },
});
