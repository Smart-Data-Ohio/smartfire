/// <reference types="vitest/config" />
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The Rust app serves the built SPA under /app/ (crates/spa). In development Vite serves it and
// forwards everything the SPA asks the Rust app for to `cargo run` on :3000, keeping the request's
// Origin so the session cookie and the CSRF origin check line up.
const rust = { target: "http://127.0.0.1:3000", changeOrigin: false };

export default defineConfig({
  base: "/app/",
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
  build: {
    // The bundle-size report in CI reads the entry chunks from here.
    manifest: true,
  },
  server: {
    proxy: {
      "/api": { ...rust, ws: true },
      "/cable": { ...rust, ws: true },
      "/rails": rust,
      "/session": rust,
    },
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    // Component tests need a DOM. jsdom has no Popover API, showModal() or anchor positioning,
    // so these tests exercise the components' fallbacks (their own focus, Esc and outside-click
    // handling); the native paths are covered by the Playwright pass.
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
  },
});
