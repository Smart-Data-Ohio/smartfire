import { defineConfig, devices } from "@playwright/test";

// The frozen-seed Rust test owns the server and the embedded production SPA.
export default defineConfig({
  testDir: "e2e/oauth",
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  use: {
    baseURL: "https://localhost:4398",
    ignoreHTTPSErrors: true,
    // TestApp disables the production front server. Supply its TLS proxy header.
    extraHTTPHeaders: { "X-Forwarded-Proto": "https" },
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
