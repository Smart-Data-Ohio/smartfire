// The cutover scenario in one long-lived headless Chrome: signs in on Rails, keeps the tabs open
// while the host swaps the container to Rust, checks session/CSRF/2FA/Action Cable continuity and
// writes on Rust, then (after the host swaps back) checks that Rails reads and keeps working with
// what Rust wrote.
//
// Coordination with the host is by files in /work/signals: this script creates `ready-for-rust`
// and waits for `rust-up`, then creates `ready-for-rails` and waits for `rails-up`. Results go to
// /work/scenario-results.json (pass/fail per check, ids and counts only) and
// /work/rust-writes.json. Run by run-scenario.sh.

import fs from "node:fs";
import crypto from "node:crypto";
import { launch, newContext, signIn, signedInPath, sleep, startFront, users, waitForFile, writeJson, WORK } from "./lib.mjs";

const signals = `${WORK}/signals`;
fs.mkdirSync(signals, { recursive: true });
const { shared_room: room } = JSON.parse(fs.readFileSync(`${WORK}/targets.json`, "utf8"));
const u = users();
const nonce = crypto.randomBytes(4).toString("hex");
const results = [];
const writes = { nonce, room };
let phase = "rails-1";
const serverErrors = { "rails-1": [], rust: [], "rails-2": [], "swap windows (container down or booting)": [] };
let swapping = false;

function check(name, pass, detail = "") {
  results.push({ phase, name, pass: !!pass, detail });
  console.log(`[${phase}] ${pass ? "PASS" : "FAIL"} ${name}${detail ? ` (${detail})` : ""}`);
}

async function attempt(name, fn) {
  try {
    await fn();
  } catch (error) {
    check(name, false, `exception: ${String(error.message).split("\n")[0].slice(0, 200)}`);
  }
}

function watch(page, label) {
  page.on("response", (response) => {
    if (response.status() >= 500) serverErrors[swapping ? "swap windows (container down or booting)" : phase].push(`${label} ${response.request().method()} ${new URL(response.url()).pathname} ${response.status()}`);
  });
  page.on("pageerror", (error) => serverErrors[swapping ? "swap windows (container down or booting)" : phase].push(`${label} pageerror ${String(error.message).slice(0, 120)}`));
}

// Action Cable sockets of a page, with the subscription confirmations each one received.
function trackCable(page) {
  const sockets = [];
  page.on("websocket", (ws) => {
    if (!ws.url().includes("/cable")) return;
    const socket = { opened: Date.now(), closed: null, frames: 0, confirms: 0, welcomes: 0 };
    sockets.push(socket);
    ws.on("framereceived", (frame) => {
      socket.frames++;
      const payload = String(frame.payload);
      if (payload.includes('"confirm_subscription"')) socket.confirms++;
      if (payload.includes('"welcome"')) socket.welcomes++;
    });
    ws.on("close", () => { socket.closed = Date.now(); });
  });
  return sockets;
}

async function waitForCable(sockets, since, timeoutMs = 90000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const live = sockets.find((s) => s.opened >= since && !s.closed && s.confirms > 0);
    if (live) return live.opened - since;
    await sleep(250);
  }
  return null;
}

async function composerPost(page, text) {
  const box = page.locator('textarea[name="message[markdown_source]"]').first();
  await box.fill(text);
  const [response] = await Promise.all([
    page.waitForResponse((r) => r.request().method() === "POST" && new URL(r.url()).pathname === `/rooms/${room}/messages`, { timeout: 20000 }),
    box.press("Enter"),
  ]);
  return response.status();
}

// fetch from inside the page with its own CSRF token, as the app's JavaScript does.
async function pageFetch(page, path, { method = "POST", fields = {}, png = false, accept = "text/vnd.turbo-stream.html, text/html" } = {}) {
  return page.evaluate(async ({ path, method, fields, png, accept }) => {
    const token = document.querySelector('meta[name="csrf-token"]')?.content;
    const body = new FormData();
    for (const [k, v] of Object.entries(fields)) body.append(k, v);
    if (png) {
      const canvas = document.createElement("canvas");
      canvas.width = 96; canvas.height = 64;
      const g = canvas.getContext("2d");
      g.fillStyle = "#3a7"; g.fillRect(0, 0, 96, 64); g.fillStyle = "#fff"; g.fillRect(20, 16, 40, 24);
      const blob = await new Promise((r) => canvas.toBlob(r, "image/png"));
      body.append("message[attachment]", new File([blob], "rehearsal.png", { type: "image/png" }));
    }
    // A 302 after PATCH keeps the method when fetch follows it (Turbo's form submissions don't hit
    // this), so non-POST requests take the redirect as their answer instead of following it.
    const redirect = method === "POST" ? "follow" : "manual";
    const response = await fetch(path, { method, body, headers: { "X-CSRF-Token": token, Accept: accept }, redirect });
    if (response.type === "opaqueredirect") return { status: 302, url: path, text: "" };
    return { status: response.status, url: new URL(response.url).pathname, text: await response.text() };
  }, { path, method, fields, png, accept });
}

// Public ids (the `message_<id>` DOM id suffix, which is also the route parameter) of the room's
// message elements, optionally only those whose text includes `text`.
const messageIds = (page, text = null) => page.evaluate((t) => [...document.querySelectorAll('div.message[id^="message_"]')]
  .filter((el) => t === null || el.textContent.includes(t))
  .map((el) => el.id.slice(8)), text);
// The element's own edit and boost endpoints (data-message-url, data-boost-url).
const messageUrls = (page, id) => page.evaluate((id) => {
  const el = document.getElementById(`message_${id}`);
  const path = (u) => (u ? new URL(u, location.href).pathname : null);
  return { edit: path(el?.dataset.messageUrl), boost: path(el?.dataset.boostUrl) };
}, id);

const roomHasText = (page, text) => page.locator(`#messages, body`).first().evaluate((el, t) => el.textContent.includes(t), text);

const front = await startFront();
const browser = await launch();

// ---- Phase 1: Rails ---------------------------------------------------------------------------
const ctxA = await newContext(browser);
const a1 = await ctxA.newPage(); watch(a1, "A1");
const cableA1 = trackCable(a1);
const ctxB = await newContext(browser);
const b1 = await ctxB.newPage(); watch(b1, "B1");
const cableB1 = trackCable(b1);
let a2;

await attempt("sign in A on Rails (password + TOTP, remember device)", async () => {
  const r = await signIn(a1, "a", u.a, { remember: true });
  check("sign in A on Rails (password + TOTP, remember device)", r.signedIn && r.challenged, `landed ${r.path}`);
});
await attempt("A's room page renders on Rails with a live cable", async () => {
  await a1.goto(`/rooms/${room}`);
  const ms = await waitForCable(cableA1, 0, 30000);
  check("A's room page renders on Rails with a live cable", ms !== null && (await a1.locator('textarea[name="message[markdown_source]"]').count()) > 0);
});
await attempt("A's profile form open on Rails", async () => {
  a2 = await ctxA.newPage(); watch(a2, "A2");
  await a2.goto("/users/me/profile");
  check("A's profile form open on Rails", (await a2.locator('textarea[name="user[bio]"]').count()) === 1);
});
await attempt("sign in B (2FA) on Rails", async () => {
  const r = await signIn(b1, "b", u.b);
  await b1.goto(`/rooms/${room}`);
  await waitForCable(cableB1, 0, 30000);
  check("sign in B (2FA) on Rails", r.signedIn && r.challenged, `landed ${r.path}`);
});
writes.cookie_names_rails = (await ctxA.cookies()).map((c) => `${c.name}${c.secure ? "(secure)" : ""}${c.httpOnly ? "(httponly)" : ""}`).sort();
const rememberCookies = (await ctxA.cookies()).filter((c) => /remember/i.test(c.name));
check("Rails issued a remembered-device cookie", rememberCookies.length > 0, rememberCookies.map((c) => c.name).join(","));

const swappedToRust = Date.now();
swapping = true;
fs.writeFileSync(`${signals}/ready-for-rust`, "");
await waitForFile(`${signals}/rust-up`);
swapping = false;
phase = "rust";

// ---- Phase 2: Rust ----------------------------------------------------------------------------
await attempt("A1 cable reconnects to Rust", async () => {
  const ms = await waitForCable(cableA1, swappedToRust);
  check("A1 cable reconnects to Rust", ms !== null, ms === null ? "no confirmed socket" : `${ms} ms after the swap began, ${cableA1.length} sockets total`);
});
await attempt("Rails-rendered composer posts to Rust (CSRF continuity)", async () => {
  const status = await composerPost(a1, `rehearsal ${nonce} composer-from-rails-page`);
  check("Rails-rendered composer posts to Rust (CSRF continuity)", status >= 200 && status < 400, `status ${status}`);
});
await attempt("Rails-rendered profile form submits to Rust (setting change)", async () => {
  await a2.fill('textarea[name="user[bio]"]', `rehearsal bio ${nonce}`);
  const [response] = await Promise.all([
    a2.waitForResponse((r) => r.request().method() === "POST" && new URL(r.url()).pathname === "/users/me/profile", { timeout: 20000 }),
    a2.locator('textarea[name="user[bio]"]').evaluate((el) => el.form.requestSubmit()),
  ]);
  await a2.waitForLoadState("load");
  await a2.goto("/users/me/profile");
  const saved = await a2.inputValue('textarea[name="user[bio]"]');
  check("Rails-rendered profile form submits to Rust (setting change)", response.status() < 400 && saved === `rehearsal bio ${nonce}`, `status ${response.status()}`);
});
await attempt("live stream: B's post reaches A's open tab on Rust", async () => {
  await waitForCable(cableB1, swappedToRust);
  const text = `rehearsal ${nonce} live-from-b`;
  const status = await composerPost(b1, text);
  let seen = false;
  for (let i = 0; i < 60 && !seen; i++) { seen = await roomHasText(a1, text); if (!seen) await sleep(250); }
  check("live stream: B's post reaches A's open tab on Rust", seen && status < 400, `post status ${status}`);
});
await attempt("A1 stays signed in after reload on Rust", async () => {
  await a1.reload();
  check("A1 stays signed in after reload on Rust", signedInPath(a1) && new URL(a1.url()).pathname === `/rooms/${room}`, new URL(a1.url()).pathname);
});
await attempt("B (2FA session from Rails) stays signed in on Rust", async () => {
  await b1.reload();
  check("B (2FA session from Rails) stays signed in on Rust", signedInPath(b1), new URL(b1.url()).pathname);
});
const ctxB2 = await newContext(browser);
const b2 = await ctxB2.newPage(); watch(b2, "B2");
await attempt("fresh 2FA sign-in for B on Rust", async () => {
  const r = await signIn(b2, "b", u.b);
  check("fresh 2FA sign-in for B on Rust", r.signedIn && r.challenged, `landed ${r.path}`);
});
await attempt("Rails remembered-device cookie skips TOTP on Rust", async () => {
  const ctx = await newContext(browser);
  await ctx.addCookies(rememberCookies);
  const page = await ctx.newPage(); watch(page, "A-remembered");
  const r = await signIn(page, "a", u.a);
  check("Rails remembered-device cookie skips TOTP on Rust", r.signedIn && !r.challenged, `challenged=${r.challenged} landed ${r.path}`);
  await ctx.close();
});
const ctxC = await newContext(browser);
const c1 = await ctxC.newPage(); watch(c1, "C1");
await attempt("enforced 2FA enrollment for C on Rust", async () => {
  const r = await signIn(c1, "c", u.c);
  writes.c_totp_secret = r.enrolled;
  await c1.goto("/");
  check("enforced 2FA enrollment for C on Rust", !!r.enrolled && signedInPath(c1), `landed ${new URL(c1.url()).pathname}`);
});

// Writes on Rust, from a page Rust rendered.
const a3 = await ctxA.newPage(); watch(a3, "A3");
await a3.goto(`/rooms/${room}`);
await attempt("Rust: post two messages", async () => {
  const statuses = [];
  for (const n of [1, 2]) {
    const r = await pageFetch(a3, `/rooms/${room}/messages`, { fields: { "message[markdown_source]": `rehearsal ${nonce} rust-post-${n}`, "message[client_message_id]": crypto.randomUUID() } });
    statuses.push(r.status);
  }
  await a3.reload();
  const ids = [];
  for (const n of [1, 2]) ids.push(...(await messageIds(a3, `rust-post-${n}`)));
  writes.rust_messages = ids;
  check("Rust: post two messages", ids.length === 2 && statuses.every((x) => x < 400), `statuses ${statuses.join(",")}, found ${ids.length}`);
});
await attempt("Rust: edit a message", async () => {
  const r = await pageFetch(a3, (await messageUrls(a3, writes.rust_messages[0])).edit, { method: "PATCH", fields: { "message[markdown_source]": `rehearsal ${nonce} rust-post-1 edited-on-rust` } });
  await a3.reload();
  const applied = await roomHasText(a3, "rust-post-1 edited-on-rust");
  check("Rust: edit a message", r.status < 400 && applied, `status ${r.status} applied=${applied}`);
});
await attempt("Rust: boost a message", async () => {
  const r = await pageFetch(a3, (await messageUrls(a3, writes.rust_messages[1])).boost, { fields: { "boost[content]": "🔥" } });
  check("Rust: boost a message", r.status < 400, `status ${r.status}`);
});
await attempt("Rust: upload a file", async () => {
  const before = new Set(await messageIds(a3));
  const r = await pageFetch(a3, `/rooms/${room}/messages`, { png: true, fields: { "message[client_message_id]": crypto.randomUUID() } });
  await a3.reload();
  writes.rust_upload_message = (await messageIds(a3)).filter((id) => !before.has(id)).pop();
  check("Rust: upload a file", r.status < 400 && !!writes.rust_upload_message, `status ${r.status}`);
});
await attempt("Rust: create a room", async () => {
  const r = await pageFetch(a3, "/rooms/opens", { fields: { "room[name]": `Rehearsal ${nonce}` }, accept: "text/html" });
  writes.rust_room = Number((r.url.match(/^\/rooms\/(\d+)/) || [])[1]);
  check("Rust: create a room", r.status === 200 && writes.rust_room > 0, `status ${r.status} -> ${r.url}`);
});
await attempt("Rust: uploaded image is served", async () => {
  await a3.goto(`/rooms/${room}`);
  await sleep(3000); // variant processing runs as a job
  await a3.reload();
  const src = await a3.locator(`#message_${writes.rust_upload_message} img`).first().getAttribute("src");
  const r = await a3.request.get(src);
  writes.rust_upload_src_kind = src.replace(/\/[^/]{20,}/g, "/…").slice(0, 80);
  check("Rust: uploaded image is served", r.status() === 200 && (r.headers()["content-type"] || "").startsWith("image/"), `status ${r.status()}`);
});
writes.cookie_names_rust = (await ctxA.cookies()).map((c) => `${c.name}${c.secure ? "(secure)" : ""}${c.httpOnly ? "(httponly)" : ""}`).sort();

const swappedToRails = Date.now();
swapping = true;
fs.writeFileSync(`${signals}/ready-for-rails`, "");
await waitForFile(`${signals}/rails-up`);
swapping = false;
phase = "rails-2";

// ---- Phase 3: back on Rails -------------------------------------------------------------------
await attempt("A1 cable reconnects to Rails", async () => {
  const ms = await waitForCable(cableA1, swappedToRails);
  check("A1 cable reconnects to Rails", ms !== null, ms === null ? "no confirmed socket" : `${ms} ms after the swap began`);
});
await attempt("A (cookies last written by Rust) stays signed in on Rails", async () => {
  await a1.reload();
  check("A (cookies last written by Rust) stays signed in on Rails", signedInPath(a1), new URL(a1.url()).pathname);
});
await attempt("Rails renders the room with everything Rust wrote", async () => {
  await a1.goto(`/rooms/${room}`);
  const has = async (t) => roomHasText(a1, t);
  const posts = (await has(`rust-post-1 edited-on-rust`)) && (await has(`rust-post-2`));
  const composer = await has(`composer-from-rails-page`);
  const boost = await a1.locator(`#message_${writes.rust_messages[1]} .boost, #message_${writes.rust_messages[1]} [class*="reaction"]`).count();
  const img = await a1.locator(`#message_${writes.rust_upload_message} img`).count();
  check("Rails renders the room with everything Rust wrote", posts && composer && boost > 0 && img > 0, `posts=${posts} composer=${composer} boost=${boost} img=${img}`);
});
await attempt("Rails serves the file Rust uploaded", async () => {
  const src = await a1.locator(`#message_${writes.rust_upload_message} img`).first().getAttribute("src");
  const r = await a1.request.get(src);
  check("Rails serves the file Rust uploaded", r.status() === 200 && (r.headers()["content-type"] || "").startsWith("image/"), `status ${r.status()}`);
});
await attempt("Rails renders the room Rust created", async () => {
  const r = await a1.goto(`/rooms/${writes.rust_room}`);
  check("Rails renders the room Rust created", r.status() === 200 && new URL(a1.url()).pathname === `/rooms/${writes.rust_room}`, `status ${r.status()}`);
});
await attempt("Rails shows the setting Rust saved", async () => {
  await a2.goto("/users/me/profile");
  check("Rails shows the setting Rust saved", (await a2.inputValue('textarea[name="user[bio]"]')) === `rehearsal bio ${nonce}`);
});
await attempt("Rails: post and edit after rollback", async () => {
  await a1.goto(`/rooms/${room}`);
  const status = await composerPost(a1, `rehearsal ${nonce} rails-after-rollback`);
  await a1.reload();
  const mid = (await messageIds(a1, "rails-after-rollback")).pop();
  const edit = await pageFetch(a1, (await messageUrls(a1, mid)).edit, { method: "PATCH", fields: { "message[markdown_source]": `rehearsal ${nonce} rails-after-rollback edited` } });
  const edit2 = await pageFetch(a1, (await messageUrls(a1, writes.rust_messages[1])).edit, { method: "PATCH", fields: { "message[markdown_source]": `rehearsal ${nonce} rust-post-2 edited-on-rails` } });
  writes.rails_message = mid;
  await a1.reload();
  const applied = (await roomHasText(a1, "rails-after-rollback edited")) && (await roomHasText(a1, "rust-post-2 edited-on-rails"));
  check("Rails: post and edit after rollback", status < 400 && edit.status < 400 && edit2.status < 400 && applied, `post ${status} edit ${edit.status} edit-rust-msg ${edit2.status} applied=${applied}`);
});
await attempt("B2 (session created on Rust) stays signed in on Rails", async () => {
  await b2.reload();
  check("B2 (session created on Rust) stays signed in on Rails", signedInPath(b2), new URL(b2.url()).pathname);
});
await attempt("C (enrolled on Rust) stays signed in on Rails", async () => {
  await c1.reload();
  check("C (enrolled on Rust) stays signed in on Rails", signedInPath(c1), new URL(c1.url()).pathname);
});
await attempt("C signs in on Rails with the TOTP secret Rust encrypted", async () => {
  const ctx = await newContext(browser);
  const page = await ctx.newPage(); watch(page, "C2");
  const r = await signIn(page, "c", { ...u.c, totp_secret: writes.c_totp_secret });
  check("C signs in on Rails with the TOTP secret Rust encrypted", r.signedIn && r.challenged, `landed ${r.path}`);
  await ctx.close();
});

for (const [p, errors] of Object.entries(serverErrors)) {
  phase = p;
  if (p.startsWith("swap")) { console.log(`[${p}] ${errors.length} 5xx/page errors (expected while no app is serving)`); continue; }
  check("no 5xx responses or page errors seen by the browser", errors.length === 0, errors.slice(0, 8).join("; "));
}
writes.swap_window_errors = serverErrors["swap windows (container down or booting)"].length;
delete writes.c_totp_secret;
writeJson("rust-writes.json", writes);
writeJson("scenario-results.json", { results, cable: { a1: cableA1.length, b1: cableB1.length } });
console.log(`scenario: ${results.filter((r) => r.pass).length}/${results.length} passed`);
await browser.close();
front.close();
