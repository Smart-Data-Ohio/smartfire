import { defineConfig, devices } from "@playwright/test";

// `SMARTFIRE_E2E_PORT` lets parallel worktrees run their own servers.
const port = Number(process.env.SMARTFIRE_E2E_PORT ?? 4180);

/**
 * The S2 specs: the SPA on Vite's dev server with the in-memory mock backend (mock/vite-plugin.ts),
 * the simulation off so every run sees the same seeded workspace. `pnpm test:e2e:mock`.
 * `SMARTFIRE_SHOTS=1` also writes screenshots to `SMARTFIRE_SHOTS_DIR` (default
 * ~/.cache/frontend-s2/shots).
 */
export default defineConfig({
  testDir: "e2e/mock",
  forbidOnly: Boolean(process.env.CI),
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
    command: `pnpm exec vite --mode mock --host 127.0.0.1 --port ${port} --strictPort`,
    env: { SMARTFIRE_MOCK_SIMULATE: "0" },
    url: `http://127.0.0.1:${port}/app/`,
    reuseExistingServer: !process.env.CI,
  },
});
