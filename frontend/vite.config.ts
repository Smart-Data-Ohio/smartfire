/// <reference types="vitest/config" />
import { readdirSync, readFileSync } from "node:fs";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig, type Plugin, type ProxyOptions } from "vite";
import { isMockEnabled, smartfireMock } from "./mock/vite-plugin.ts";
import { inlinePaletteTable } from "./src/lib/palette-table.ts";

// The Rust app serves the built SPA under /app/ (crates/spa). In development Vite serves it and
// forwards everything the SPA asks the Rust app for to `cargo run` on :3000, keeping the request's
// Origin so the session cookie and the CSRF origin check line up.
const rust = { target: "http://127.0.0.1:3000", changeOrigin: false };

/** Every palette's tokens, built into index.html's blocking script (src/lib/palette-table.ts). */
const paletteTable: Plugin = {
  name: "smartfire-palette-table",
  transformIndexHtml: { order: "pre", handler: inlinePaletteTable },
};

const fonts = new URL("./src/styles/fonts/", import.meta.url);

/**
 * Each self-hosted font's licence, beside the fonts in the build's assets/: the SIL Open Font
 * License travels with the fonts (crates/spa's tests check every shipped font has its licence).
 */
const fontLicences: Plugin = {
  name: "smartfire-font-licences",
  apply: "build",
  generateBundle() {
    for (const name of readdirSync(fonts).filter((file) => file.startsWith("LICENSE-"))) {
      this.emitFile({
        type: "asset",
        fileName: `assets/${name}`,
        source: readFileSync(new URL(name, fonts)),
      });
    }
  },
};

export default defineConfig(({ mode }) => {
  // With the mock (`SMARTFIRE_MOCK=1` or `--mode mock`) the dev server answers /api itself
  // (mock/vite-plugin.ts), so /api must not be proxied to Rust.
  const proxy = new Map<string, ProxyOptions>([
    ["/cable", { ...rust, ws: true }],
    ["/rails", rust],
    ["/session", rust],
  ]);

  if (!isMockEnabled(mode)) proxy.set("/api", { ...rust, ws: true });

  return {
    base: "/app/",
    plugins: [
      react(),
      babel({ presets: [reactCompilerPreset()] }),
      smartfireMock(),
      paletteTable,
      fontLicences,
    ],
    build: {
      // The bundle-size report in CI reads the entry chunks from here.
      manifest: true,
      rolldownOptions: {
        output: {
          // Vendor code changes far less often than the app, so it ships in its own chunks and
          // stays cached across deploys. All three load with the entry (see the CI size report).
          codeSplitting: {
            groups: [
              { name: "react", test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/ },
              { name: "effect", test: /node_modules[\\/]effect[\\/]/ },
              { name: "router", test: /node_modules[\\/]@tanstack[\\/]/ },
            ],
          },
        },
      },
    },
    server: { proxy: Object.fromEntries(proxy) },
    test: {
      include: ["src/**/*.test.{ts,tsx}", "mock/**/*.test.ts"],
      // Component tests need a DOM. jsdom has no Popover API, showModal() or anchor positioning,
      // so these tests exercise the components' fallbacks (their own focus, Esc and outside-click
      // handling); the native paths are covered by the Playwright pass.
      environment: "jsdom",
      setupFiles: ["src/test/setup.ts"],
    },
  };
});
