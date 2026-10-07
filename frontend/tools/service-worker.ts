import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { build, type Plugin } from "vite";

export function contentHash(content: string | Uint8Array): string {
  return createHash("sha256").update(content).digest("hex").slice(0, 16);
}

/** Match Vite's eight-character asset fingerprints and the embedder's immutable-file rule. */
export function assetHash(content: string | Uint8Array): string {
  return createHash("sha256").update(content).digest("base64url").slice(0, 8);
}

export function precacheFiles(names: readonly string[], base: string): string[] {
  const paths: string[] = [];

  for (const name of names) {
    if (/^assets\/.+-[\w-]{8,}\.[^/]+$/.test(name)) {
      paths.push(`${base}${name}`);
    }
  }

  return [...paths, `${base}offline.html`].sort();
}

export function buildVersion(precache: readonly string[]): string {
  return contentHash([...precache].sort().join("\n"));
}

/** Stable bootstrap, content-hashed TS runtime, and a build-derived precache with no hash cycle. */
export function smartfireServiceWorker(): Plugin {
  let root = "";
  let base = "/";
  let dist = "";

  return {
    name: "smartfire-service-worker",
    enforce: "post",
    configResolved(config) {
      root = config.root;
      base = config.base;
      dist = resolve(root, config.build.outDir);
    },
    async generateBundle(_options, bundle) {
      const workerBuild = await build({
        configFile: false,
        root,
        base,
        logLevel: "silent",
        build: {
          write: false,
          minify: true,
          lib: {
            entry: resolve(root, "src/service-worker/worker.ts"),
            formats: ["iife"],
            name: "SmartfireWorker",
          },
        },
      });

      const outputs = Array.isArray(workerBuild)
        ? workerBuild
        : "output" in workerBuild
          ? [workerBuild]
          : [];

      if (outputs.length !== 1) {
        throw new Error("Expected one service worker build output");
      }

      const chunk = outputs[0]?.output.find((output) => output.type === "chunk");
      const offline = bundle["offline.html"];

      if (chunk === undefined || offline?.type !== "asset") {
        throw new Error("The production build must emit a worker and offline.html");
      }

      const runtimeName = `assets/service-worker-${assetHash(chunk.code)}.js`;
      const offlineName = `assets/offline-page-${assetHash(offline.source)}.html`;

      this.emitFile({ type: "asset", fileName: runtimeName, source: chunk.code });
      // offline.html is stable. Its tiny hashed copy makes an HTML-only change alter the list's
      // names and version too, without rewriting the page the user designed.
      this.emitFile({ type: "asset", fileName: offlineName, source: offline.source });

      const precache = precacheFiles([...Object.keys(bundle), runtimeName, offlineName], base);
      const config = { version: buildVersion(precache), precache, offline: `${base}offline.html` };

      this.emitFile({
        type: "asset",
        fileName: "service-worker.js",
        source: `self.smartfireBuild=${JSON.stringify(config)};\nimportScripts(${JSON.stringify(`${base}${runtimeName}`)});\n`,
      });
    },
    configurePreviewServer(server) {
      // Browser tests update real service-worker requests at the HTTP server. Playwright's
      // context.route cannot intercept the browser's update-script requests reliably.
      let revision = "";
      let noStoreReads = 0;
      const testing = process.env.SMARTFIRE_PWA_E2E === "1";

      server.middlewares.use(async (request, response, next) => {
        if (testing && request.url === "/assets/pwa-no-store-0123456789abcdef.txt") {
          noStoreReads += 1;
          response.setHeader("Content-Type", "text/plain; charset=utf-8");
          response.setHeader("Cache-Control", "no-store");
          response.end(String(noStoreReads));

          return;
        }

        if (testing && request.url === "/service-worker.js") {
          response.setHeader("Content-Type", "text/javascript; charset=utf-8");
          response.setHeader("Cache-Control", "no-cache");
          response.end(
            'self.addEventListener("install", e => e.waitUntil(self.skipWaiting())); self.addEventListener("activate", e => e.waitUntil(self.clients.claim()));',
          );

          return;
        }

        if (testing && request.url?.startsWith("/__pwa/version?") && request.method === "POST") {
          revision = new URL(request.url, "http://preview").searchParams.get("value") ?? "";
          response.statusCode = 204;
          response.end();

          return;
        }

        if (request.url?.split("?")[0] !== `${base}service-worker.js`) {
          next();

          return;
        }

        try {
          let script = await readFile(resolve(dist, "service-worker.js"), "utf8");

          if (testing && revision !== "") {
            script = script.replace(/("version":")[^"]+("\s*,)/, `$1${revision}$2`);
          }

          for (const [name, value] of Object.entries(server.config.preview.headers ?? {})) {
            if (value !== undefined) response.setHeader(name, value);
          }

          response.setHeader("Content-Type", "text/javascript; charset=utf-8");
          response.setHeader("Cache-Control", "no-cache");
          response.setHeader("Service-Worker-Allowed", "/");
          response.end(script);
        } catch (error) {
          next(error);
        }
      });
    },
  };
}
