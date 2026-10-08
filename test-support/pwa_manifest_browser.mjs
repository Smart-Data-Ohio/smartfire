// Chromium's own manifest fetch, after a real password sign-in. Fetching it with an HTTP
// test client (or page.evaluate(fetch)) would miss the link's credentials-mode behavior.
import assert from "node:assert/strict"
import { createHmac } from "node:crypto"
import { readFile } from "node:fs/promises"
import { createRequire } from "node:module"

const { chromium } = process.env.PWA_BROWSER_LOCAL === "1"
  ? createRequire(new URL("../frontend/package.json", import.meta.url))("@playwright/test")
  : await import("playwright")
const origin = process.env.PWA_BROWSER_TARGET
assert.ok(origin, "the Rust test supplies its private server")
const labels = JSON.parse(await readFile(new URL("../parity/.seed/default/labels.json", import.meta.url), "utf8"))

function authenticatorCode(secret) {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
  const bits = [...secret.toUpperCase()].map((character) => alphabet.indexOf(character).toString(2).padStart(5, "0")).join("")
  const key = Buffer.from(bits.match(/.{8}/g).map((byte) => Number.parseInt(byte, 2)))
  const counter = Buffer.alloc(8)
  counter.writeBigUInt64BE(BigInt(Math.floor(new Date(labels["clock.now"]).getTime() / 30_000)))
  const digest = createHmac("sha1", key).update(counter).digest()
  const offset = digest.at(-1) & 15
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, "0")
}

const browser = await chromium.launch({ channel: "chromium", headless: true })
try {
  const context = await browser.newContext()
  const page = await context.newPage()
  const devtools = await context.newCDPSession(page)
  const manifestRequests = []
  const requestHeaders = new Map()
  const manifestResponses = new Map()
  const receipts = []
  devtools.on("Network.requestWillBeSent", (event) => {
    if (event.type === "Manifest") manifestRequests.push(event)
  })
  devtools.on("Network.requestWillBeSentExtraInfo", (event) => {
    requestHeaders.set(event.requestId, event.headers)
  })
  devtools.on("Network.responseReceived", (event) => {
    if (event.type === "Manifest") manifestResponses.set(event.requestId, event.response)
  })
  await devtools.send("Network.enable")

  async function manifestFor(path, layout) {
    manifestRequests.length = 0
    await page.goto(`${origin}${path}`, { waitUntil: "load" })
    if (layout === "spa") {
      const boot = JSON.parse(await page.locator("#boot").textContent())
      assert.equal(boot.user.id, 127326141)
    } else {
      assert.equal(await page.locator('meta[name="current-user-id"]').getAttribute("content"), "127326141")
    }
    // This invokes Chromium's manifest loader with the document link's credentials mode.
    const result = await devtools.send("Page.getAppManifest")
    assert.equal(result.url, `${origin}/webmanifest.json`)
    const manifest = JSON.parse(result.data)
    const deadline = Date.now() + 10_000
    let request
    for (;;) {
      request = manifestRequests.at(-1)
      if (request !== undefined && requestHeaders.has(request.requestId) && manifestResponses.has(request.requestId)) break
      assert.ok(Date.now() < deadline, "Chromium exposes the manifest request and its actual headers")
      await new Promise((resolve) => setTimeout(resolve, 50))
    }
    assert.equal(request.request.url, `${origin}/webmanifest.json`)
    // The SPA registers its worker normally; personalized manifests still come from the server.
    assert.notEqual(manifestResponses.get(request.requestId).fromServiceWorker, true)
    const headers = requestHeaders.get(request.requestId)
    const cookie = Object.entries(headers).find(([name]) => name.toLowerCase() === "cookie")?.[1] ?? ""
    const session = (await context.cookies(origin)).find((item) => item.name === "session_token")
    assert.ok(session, "password sign-in issued the browser's real session cookie")
    receipts.push({
      layout,
      sessionCookie: cookie.split(/;\s*/).includes(`session_token=${session.value}`),
      profileShortcut: manifest.shortcuts[1].url,
    })
  }

  await page.goto(`${origin}/session/new`)
  await page.locator('input[name="email_address"]').fill(process.env.PWA_BROWSER_EMAIL)
  await page.locator('input[name="password"]').fill("secret123456")
  await page.getByRole("button", { name: "Sign in", exact: true }).click()
  await page.waitForURL((url) => url.pathname === "/two_factor_setup")
  await manifestFor("/two_factor_setup", "classic-auth")

  const secret = (await page.locator("#two_factor_manual_key").innerText()).replace(/\s+/g, "")
  await page.getByLabel("Authenticator code", { exact: true }).fill(authenticatorCode(secret))
  await page.getByRole("button", { name: "Verify and continue", exact: true }).click()
  await page.getByRole("heading", { name: "Save your backup codes", exact: true }).waitFor()
  await page.getByRole("link", { name: "Continue", exact: true }).click()
  await page.waitForURL((url) => url.pathname !== "/two_factor_setup")
  await manifestFor("/users/me/profile?classic=1", "classic-application")
  await manifestFor("/app/settings", "spa")

  console.log(`PWA_MANIFEST_FETCHES ${JSON.stringify(receipts)}`)
  for (const receipt of receipts) {
    if (receipt.layout === "spa") {
      assert.equal(receipt.sessionCookie, true, "spa: Chromium's manifest request carries the session cookie")
      assert.equal(receipt.profileShortcut, "/users/me/profile", "spa: saved classic choice overrides SPA_DEFAULT=next")
    } else {
      // Classic layouts keep Rails' exact manifest link, which sends no credentials.
      assert.equal(receipt.sessionCookie, false, `${receipt.layout}: Rails' manifest link sends no session cookie`)
    }
  }
  console.log(`PWA_MANIFEST_RECEIPT ${JSON.stringify({ defaultUi: "next", savedUi: "classic", fetches: receipts })}`)
} finally {
  await browser.close()
}
