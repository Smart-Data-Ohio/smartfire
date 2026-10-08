import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { Plugin } from "vite";

// The policy the Rust app sends with the embedded files. The `/app` page it renders also gives
// each of its scripts a per-request nonce (crates/spa/src/shell.rs). `vite preview` has no shell to
// add one, so the browser suite allows the built pages' inline scripts by their hashes instead:
// still only those exact scripts, never `'unsafe-inline'`.
const SCRIPT_SRC = "script-src 'self' 'wasm-unsafe-eval'";

export const PREVIEW_POLICY = `default-src 'self'; ${SCRIPT_SRC}; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; worker-src 'self' blob:`;

const INLINE_SCRIPT = /<script(?![^>]*\ssrc=)[^>]*>([\s\S]*?)<\/script>/g;

/** The preview policy with a `'sha256-…'` source for each inline script in `pages`. */
export function previewPolicy(pages: readonly string[]): string {
  const hashes = new Set<string>();

  for (const html of pages) {
    for (const [, body = ""] of html.matchAll(INLINE_SCRIPT)) {
      hashes.add(`'sha256-${createHash("sha256").update(body).digest("base64")}'`);
    }
  }

  return PREVIEW_POLICY.replace(SCRIPT_SRC, [SCRIPT_SRC, ...hashes].join(" "));
}

/** Sends the preview policy, hashed from the built pages, with every `vite preview` response. */
export function smartfirePreviewPolicy(): Plugin {
  return {
    name: "smartfire-preview-policy",
    configurePreviewServer(server) {
      const dist = resolve(server.config.root, server.config.build.outDir);

      const pages = ["index.html", "offline.html"].map((page) =>
        readFileSync(resolve(dist, page), "utf8"),
      );

      const policy = previewPolicy(pages);

      server.middlewares.use((_request, response, next) => {
        response.setHeader("Content-Security-Policy", policy);
        next();
      });
    },
  };
}
