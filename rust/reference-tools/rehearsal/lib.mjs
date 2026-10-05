// Shared helpers for the cutover rehearsal drivers. Runs inside the Playwright image on the
// rehearsal's internal Docker network (see rehearsal.sh `driver`).
//
// The browser reaches the app through a local TLS front (https://chat.rehearsal.test:8443 ->
// http://$APP_HOST:80), the way production terminates TLS in front of the app, so both runtimes run
// with their production SSL settings (assume_ssl/force_ssl, Secure cookies) rather than
// DISABLE_SSL. Plain TCP is piped, so WebSockets pass through unchanged.

import crypto from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import tls from "node:tls";
import { chromium } from "playwright";

export const HOST = "chat.rehearsal.test";
export const PORT = 8443;
export const BASE = `https://${HOST}:${PORT}`;
export const WORK = "/work";
const APP_HOST = process.env.APP_HOST || "smartfire";

export function startFront() {
  const server = tls.createServer(
    { key: fs.readFileSync(`${WORK}/tls.key`), cert: fs.readFileSync(`${WORK}/tls.crt`) },
    (client) => {
      const upstream = net.connect(80, APP_HOST);
      client.pipe(upstream).pipe(client);
      const close = () => { client.destroy(); upstream.destroy(); };
      client.on("error", close);
      upstream.on("error", close);
    },
  );
  return new Promise((resolve) => server.listen(PORT, "127.0.0.1", () => resolve(server)));
}

export async function launch() {
  return chromium.launch({ args: [`--host-resolver-rules=MAP ${HOST} 127.0.0.1`] });
}

export async function newContext(browser) {
  return browser.newContext({ baseURL: BASE, ignoreHTTPSErrors: true });
}

export function users() {
  return JSON.parse(fs.readFileSync(`${WORK}/users.json`, "utf8"));
}

// RFC 6238 TOTP (SHA-1, 6 digits, 30 s), as ROTP computes it.
function base32(secret) {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  let bits = "";
  for (const ch of secret.replace(/[\s=]/g, "").toUpperCase()) bits += alphabet.indexOf(ch).toString(2).padStart(5, "0");
  const bytes = [];
  for (let i = 0; i + 8 <= bits.length; i += 8) bytes.push(parseInt(bits.slice(i, i + 8), 2));
  return Buffer.from(bytes);
}

export function totp(secret, step) {
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(step));
  const mac = crypto.createHmac("sha1", base32(secret)).update(counter).digest();
  const offset = mac[mac.length - 1] & 0xf;
  const code = (mac.readUInt32BE(offset) & 0x7fffffff) % 1_000_000;
  return code.toString().padStart(6, "0");
}

// Each code step can be spent once per user (replay protection), across every driver run.
const stepsFile = `${WORK}/totp-steps.json`;
export async function freshCode(key, secret) {
  const used = fs.existsSync(stepsFile) ? JSON.parse(fs.readFileSync(stepsFile, "utf8")) : {};
  let step = Math.floor(Date.now() / 30000);
  while (used[key] !== undefined && step <= used[key]) {
    await sleep(1000);
    step = Math.floor(Date.now() / 30000);
  }
  used[key] = step;
  fs.writeFileSync(stepsFile, JSON.stringify(used));
  return totp(secret, step);
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Signs in through the real forms. Returns where it landed and whether a TOTP was asked for.
// remember: tick "remember this device" on the challenge. enroll: for a user without 2FA, complete
// the enforced setup and return the new secret.
export async function signIn(page, key, user, { remember = false } = {}) {
  await page.goto("/session/new");
  await page.fill('input[name="email_address"]', user.email);
  await page.fill('input[name="password"]', user.password);
  await Promise.all([page.waitForURL((u) => u.pathname !== "/session/new", { timeout: 15000 }).catch(() => {}),
    page.click('button[name="log_in"]')]);
  await page.waitForLoadState("load");
  let challenged = false, enrolled = null;
  if (page.url().includes("/two_factor_challenge")) {
    challenged = true;
    await page.fill('input[name="code"]', await freshCode(key, user.totp_secret));
    if (remember) await page.check('input[name="remember_device"][type="checkbox"]');
    await Promise.all([page.waitForURL((u) => !u.pathname.startsWith("/two_factor_challenge"), { timeout: 15000 }),
      page.click('form[action="/two_factor_challenge"] button[type="submit"]')]);
  } else if (page.url().includes("/two_factor_setup")) {
    const secret = (await page.textContent("#two_factor_manual_key")).replace(/\s+/g, "");
    await page.fill('form[action="/two_factor_setup"] input[name="code"]', await freshCode(key, secret));
    await Promise.all([page.waitForURL((u) => u.pathname !== "/two_factor_setup", { timeout: 15000 }).catch(() => {}),
      page.click('form[action="/two_factor_setup"] button[type="submit"]')]);
    enrolled = secret;
  }
  await page.waitForLoadState("load");
  const path = new URL(page.url()).pathname;
  return { path, challenged, enrolled, signedIn: !/^\/(session|two_factor_challenge)/.test(path) };
}

export function signedInPath(page) {
  const path = new URL(page.url()).pathname;
  return !/^\/(session|two_factor_challenge|two_factor_setup)/.test(path);
}

export async function waitForFile(path, timeoutMs = 15 * 60 * 1000) {
  const start = Date.now();
  while (!fs.existsSync(path)) {
    if (Date.now() - start > timeoutMs) throw new Error(`timed out waiting for ${path}`);
    await sleep(250);
  }
}

// Counts of key elements in a page, for comparing runtimes on the same URL (no content kept).
export function elementCounts(html) {
  const count = (re) => (html.match(re) || []).length;
  return {
    bytes: Buffer.byteLength(html),
    a: count(/<a\b/g), form: count(/<form\b/g), input: count(/<input\b/g), button: count(/<button\b/g),
    img: count(/<img\b/g), turbo_frame: count(/<turbo-frame\b/g),
    cable_stream: count(/<turbo-cable-stream-source\b/g), message: count(/\bdata-message-id="/g),
    template: count(/<template\b/g), script: count(/<script\b/g), controller: count(/\bdata-controller=/g),
    csrf_meta: count(/<meta name="csrf-token"/g), authenticity_token: count(/name="authenticity_token"/g),
    missing_attachment: count(/☒/g),
  };
}

export function writeJson(name, data) {
  fs.writeFileSync(`${WORK}/${name}`, JSON.stringify(data, null, 1));
}
