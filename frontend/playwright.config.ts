import { defineConfig, devices } from "@playwright/test";

// SMARTFIRE_E2E_PORT keeps parallel worktrees off one another's preview server.
const port = Number(process.env.SMARTFIRE_E2E_PORT ?? 4173);

// Until the SPA has an API to talk to, the specs run against `vite preview` of the production
// build. Later slices point them at the Rust binary with the SPA embedded and a frozen seed.
export default defineConfig({
  testDir: "e2e",
  forbidOnly: Boolean(process.env.CI),
  use: {
    baseURL: `http://127.0.0.1:${port}`,
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    // SMARTFIRE_E2E_BUILT: the dist is already built (the Frontend CI job's, which crates/spa
    // embedded), so it is previewed as it is.
    command: `${process.env.SMARTFIRE_E2E_BUILT ? "" : "pnpm build && "}pnpm preview --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}/app/`,
    reuseExistingServer: !process.env.CI,
  },
});
