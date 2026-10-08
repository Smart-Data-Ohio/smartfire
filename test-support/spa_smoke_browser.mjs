// Real-server SPA smoke: the actual campfire binary with the production SPA embedded,
// driven like a person. The Rust test owns the private seed/server; every step here runs
// against real pages, forms, cookies and APIs — no mocks, no stubbed network.
import assert from "node:assert/strict"
import { createHmac } from "node:crypto"
import { readFile } from "node:fs/promises"
import { createRequire } from "node:module"

const { chromium } = process.env.SPA_SMOKE_LOCAL === "1"
  ? createRequire(new URL("../frontend/package.json", import.meta.url))("@playwright/test")
  : await import("playwright")
const origin = process.env.SPA_SMOKE_TARGET
assert.ok(origin, "the Rust test supplies its private server")
const email = process.env.SPA_SMOKE_EMAIL
assert.ok(email, "the Rust test supplies the seed user's email")
const room = process.env.SPA_SMOKE_ROOM
assert.ok(room, "the Rust test supplies the room id")
const labels = JSON.parse(await readFile(new URL("../parity/.seed/default/labels.json", import.meta.url), "utf8"))

// The real setup page supplies its fresh secret; the Rust fixture uses the seed's frozen clock.
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

const step = (name, detail = "") => console.log(`SPA_SMOKE_STEP ${name}${detail ? ` ${detail}` : ""}`)

const browser = await chromium.launch({ channel: "chromium", headless: true })
try {
  const context = await browser.newContext()
  const page = await context.newPage()
  const pageErrors = []
  page.on("pageerror", (error) => pageErrors.push(`pageerror: ${error.message}`))
  page.on("response", (response) => {
    if (response.status() >= 400) step("http-error", `${response.status()} ${new URL(response.url()).pathname}`)
  })
  page.on("console", (message) => {
    if (message.type() !== "error") return
    const url = message.location().url
    // This seed has no LiveKit configuration; the browser also probes the absent favicon.
    // Allow only those exact HTTP diagnostics, keeping script errors and broken assets fatal.
    const expected = (url === `${origin}/api/v1/huddles` && message.text() === "Failed to load resource: the server responded with a status of 503 (Service Unavailable)")
      || (url === `${origin}/favicon.ico` && message.text() === "Failed to load resource: the server responded with a status of 404 (Not Found)")
    if (expected) step("expected-resource-error", url)
    else pageErrors.push(`console.error: ${message.text()} (${url})`)
  })

  // Password sign-in can land on two-factor enrollment (the seed user is unenrolled), either
  // directly or after the SPA shell redirects there. Complete the application's existing
  // security flow instead of bypassing that requirement.
  async function enrollIfNeeded() {
    if (new URL(page.url()).pathname !== "/two_factor_setup") return false
    const secret = (await page.locator("#two_factor_manual_key").innerText()).replace(/\s+/g, "")
    await page.getByLabel("Authenticator code", { exact: true }).fill(authenticatorCode(secret))
    await page.getByRole("button", { name: "Verify and continue", exact: true }).click()
    await page.getByRole("heading", { name: "Save your backup codes", exact: true }).waitFor()
    await page.getByRole("link", { name: "Continue", exact: true }).click()
    await page.waitForURL((url) => !url.pathname.startsWith("/two_factor"))
    step("two-factor-enrolled", new URL(page.url()).pathname)
    return true
  }

  try {
    await page.goto(`${origin}/session/new`, { waitUntil: "load" })
    step("sign-in-page", new URL(page.url()).pathname)
    await page.locator('input[name="email_address"]').fill(email)
    await page.locator('input[name="password"]').fill("secret123456")
    await page.getByRole("button", { name: "Sign in", exact: true }).click()
    await page.waitForURL((url) => url.pathname !== "/session/new")
    const landing = new URL(page.url()).pathname
    step("sign-in-landing", landing)
    await enrollIfNeeded()

    // The SPA shell, then the room a person would open.
    await page.goto(`${origin}/app/r/${room}`)
    if (await enrollIfNeeded() && new URL(page.url()).pathname !== `/app/r/${room}`) {
      await page.goto(`${origin}/app/r/${room}`)
    }
    await page.getByRole("button", { name: "Your account", exact: true }).waitFor()
    const timeline = page.getByRole("log", { name: "Messages" })
    await timeline.waitFor()
    const composer = page.getByRole("textbox", { name: /Message/ })
    await composer.waitFor()
    step("room-opened", `/app/r/${room}`)

    const body = `smoke ${Date.now().toString(36)} ${Math.floor(Math.random() * 1e6).toString(36)}`
    await composer.fill(body)
    await page.getByRole("button", { name: "Send message", exact: true }).click()
    await timeline.locator("article[data-message-row][data-message-id]").filter({ hasText: body }).waitFor()
    step("message-sent", body)

    await page.reload({ waitUntil: "load" })
    await page.getByRole("log", { name: "Messages" }).getByText(body).first().waitFor()
    step("message-persisted", body)

    console.log(`SPA_SMOKE_RECEIPT ${JSON.stringify({ room: Number(room), landing, persisted: true })}`)
    assert.deepEqual(pageErrors, [], `the smoke ran without page errors: ${JSON.stringify(pageErrors)}`)
    await context.close()
  } catch (error) {
    let snippet = "<unreadable>"
    try {
      snippet = ((await page.evaluate(() => document.body?.innerText?.slice(0, 2000) ?? "")) || "<empty>").replace(/\n/g, " | ")
    } catch {
      // The page is gone; the URL below is the whole story.
    }
    console.log(`SPA_SMOKE_FAILURE ${JSON.stringify({
      url: page.url(),
      message: error.message?.slice(0, 500),
      body: snippet,
      pageErrors,
    })}`)
    throw error
  }
} finally {
  await browser.close()
}
