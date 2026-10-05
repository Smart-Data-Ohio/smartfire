// Smoke and latency run against whichever runtime is up: signs in as the viewer through the real
// forms, then requests every path in /work/targets.json (no redirects followed) a warm-up plus
// SAMPLES times, recording the status, the latency and counts of key elements. Writes
// /work/smoke-<label>.json; no page content is kept.
//
// Usage: rehearsal.sh driver smoke.mjs LABEL [SAMPLES] [--reuse-session]
//
// --reuse-session signs in only if /work/smoke-state.json holds no working session, and saves the
// session afterwards, so back-to-back runs on the two runtimes see identical data (a sign-in adds
// a session row and a new-sign-in activity item).

import fs from "node:fs";
import { BASE, elementCounts, launch, newContext, signIn, startFront, users, writeJson, WORK } from "./lib.mjs";

const label = process.argv[2] || "run";
const samples = Number(process.argv[3] || 10);
const reuse = process.argv.includes("--reuse-session");
const statePath = `${WORK}/smoke-state.json`;
const { paths } = JSON.parse(fs.readFileSync(`${WORK}/targets.json`, "utf8"));

const front = await startFront();
const browser = await launch();
let context = reuse && fs.existsSync(statePath)
  ? await browser.newContext({ baseURL: BASE, ignoreHTTPSErrors: true, storageState: statePath })
  : await newContext(browser);
const probe = await context.request.get("/users/me/profile", { maxRedirects: 0, failOnStatusCode: false });
let reused = probe.status() === 200;
if (!reused) {
  const page = await context.newPage();
  const login = await signIn(page, "a", users().a);
  if (!login.signedIn) throw new Error(`sign-in failed, landed on ${login.path}`);
  await page.close();
}
if (reuse) await context.storageState({ path: statePath });

const results = [];
for (const path of paths) {
  const timings = [];
  let status, location, counts;
  for (let i = 0; i <= samples; i++) {
    const started = performance.now();
    const response = await context.request.get(path, { maxRedirects: 0, failOnStatusCode: false });
    const body = await response.text();
    const elapsed = performance.now() - started;
    if (i === 0) {
      status = response.status();
      location = response.headers().location ? new URL(response.headers().location, "https://x").pathname : undefined;
      counts = status === 200 && (response.headers()["content-type"] || "").includes("html") ? elementCounts(body) : undefined;
    } else {
      timings.push(elapsed);
      if (response.status() !== status) status = `${status}/${response.status()}`;
    }
  }
  timings.sort((a, b) => a - b);
  const pct = (p) => timings[Math.min(timings.length - 1, Math.floor(p * timings.length))];
  results.push({ path, status, location, p50_ms: +pct(0.5).toFixed(2), p95_ms: +pct(0.95).toFixed(2), counts });
}

writeJson(`smoke-${label}.json`, { label, samples, reused_session: reused, at: new Date().toISOString(), results });
const bad = results.filter((r) => typeof r.status !== "number" || r.status >= 500);
console.log(`${label}: ${reused ? "reused session, " : ""}${results.length} paths, statuses ${JSON.stringify(results.reduce((m, r) => ((m[r.status] = (m[r.status] || 0) + 1), m), {}))}, ${bad.length} >=500`);
await browser.close();
front.close();
