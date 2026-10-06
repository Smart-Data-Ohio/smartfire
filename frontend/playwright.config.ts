import { defineConfig, devices } from "@playwright/test";

const port = 4173;

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
    command: `pnpm build && pnpm preview --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}/app/`,
    reuseExistingServer: !process.env.CI,
  },
});
