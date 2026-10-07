// Real Rust pages, vendored Turbo/classic overrides and the production SPA entry.
// The Rust test owns the private seed/server; registration always comes from application code.
import assert from "node:assert/strict"
import { createHmac } from "node:crypto"
import { mkdtemp, readFile, rm } from "node:fs/promises"
import { createRequire } from "node:module"
import { tmpdir } from "node:os"
import { join } from "node:path"

const { chromium } = process.env.PWA_BROWSER_LOCAL === "1"
  ? createRequire(new URL("../frontend/package.json", import.meta.url))("@playwright/test")
  : await import("playwright")
const origin = process.env.PWA_BROWSER_TARGET
assert.ok(origin, "the Rust test supplies its private server")
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

const browser = await chromium.launch({ channel: "chromium", headless: true })
const profileDirectory = await mkdtemp(join(tmpdir(), "smartfire-pwa-"))
let persistentContext = null
try {
  // Quiet classic startup keeps the existing test-environment opt-out, on both lifecycle events.
  const disabled = await browser.newContext()
  const disabledPage = await disabled.newPage()
  await disabledPage.goto(`${origin}/session/new`)
  assert.equal(await disabledPage.getAttribute("html", "data-service-worker"), "false")
  await disabledPage.evaluate(() => document.dispatchEvent(new Event("turbo:load")))
  assert.equal(await disabledPage.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length), 0)
  await disabled.close()
  await browser.close()

  // Use a disposable normal profile to exercise the storage mode of an installed app.
  persistentContext = await chromium.launchPersistentContext(profileDirectory, {
    channel: "chromium", headless: true, permissions: ["notifications"],
  })
  const context = persistentContext
  await context.addCookies([{ name: "enable_service_worker", value: "1", url: origin }])
  const page = context.pages()[0] ?? await context.newPage()
  const devtools = await context.newCDPSession(page)
  const registrationIds = new Set()
  const deletedIds = new Set()
  devtools.on("ServiceWorker.workerRegistrationUpdated", ({ registrations }) => {
    for (const registration of registrations) {
      if (registration.scopeURL === `${origin}/`) {
        registrationIds.add(registration.registrationId)
        if (registration.isDeleted) deletedIds.add(registration.registrationId)
      }
    }
  })
  await devtools.send("ServiceWorker.enable")

  async function activeWorker(path) {
    await page.waitForFunction(async ({ origin, path }) => {
      const registrations = await navigator.serviceWorker.getRegistrations()
      return registrations.length === 1
        && registrations[0].scope === `${origin}/`
        && registrations[0].active?.scriptURL === `${origin}${path}`
        && registrations[0].active.state === "activated"
        && navigator.serviceWorker.controller?.scriptURL === `${origin}${path}`
    }, { origin, path }, { timeout: 30_000 })
  }

  async function subscription() {
    return page.evaluate(async () => {
      const registration = await navigator.serviceWorker.getRegistration("/")
      const current = await registration.pushManager.getSubscription()
      return current === null ? null : current.toJSON()
    })
  }

  await page.goto(`${origin}/session/new`)
  assert.equal(await page.locator('meta[name="service-worker-url"]').getAttribute("content"), "/app/service-worker.js")
  await activeWorker("/app/service-worker.js")
  await page.evaluate(() => {
    window.pwaOriginalDocument = document.documentElement
    window.pwaLoadCount = 0
    window.addEventListener("load", () => window.pwaLoadCount++)
  })

  // This is an explicit test subscription, using the page's real VAPID key and browser API.
  // Record any native failure exactly; an unsuccessful attempt never proves subscription survival.
  const enrollment = await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.ready
    // ready can resolve while the activate event is still running. Subscribe only after the
    // browser reports this active worker as activated, including its startup reconciliation.
    await new Promise((resolve, reject) => {
      const worker = registration.active
      const changed = () => {
        if (worker.state === "activated") {
          worker.removeEventListener("statechange", changed)
          resolve()
        } else if (worker.state === "redundant") {
          worker.removeEventListener("statechange", changed)
          reject(new Error("the selected worker became redundant before push enrollment"))
        }
      }
      worker.addEventListener("statechange", changed)
      changed()
    })
    const encoded = document.querySelector('meta[name="vapid-public-key"]').content
    if (!encoded) throw new Error("the Rust fixture must expose its valid VAPID key")
    const raw = atob(encoded.replace(/-/g, "+").replace(/_/g, "/"))
    const key = Uint8Array.from(raw, (character) => character.charCodeAt(0))
    const state = (current) => ({
      permission: Notification.permission,
      scope: current?.scope,
      active: current?.active?.scriptURL,
      state: current?.active?.state,
      controller: navigator.serviceWorker.controller?.scriptURL,
    })
    const before = state(registration)
    let result
    try {
      result = await Promise.race([
        registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: key })
          .then((value) => ({ subscribed: true, subscription: value.toJSON() })),
        new Promise((resolve) => setTimeout(() => resolve({ subscribed: false, name: "Pending", message: "Chromium pushManager.subscribe did not settle within 30 seconds" }), 30_000)),
      ])
    } catch (error) {
      result = { subscribed: false, name: error.name, message: error.message }
    }
    return { ...result, before, after: state(await navigator.serviceWorker.getRegistration("/")) }
  })
  assert.equal(enrollment.before.permission, "granted")
  assert.equal(enrollment.before.state, "activated")
  assert.deepEqual(enrollment.after, enrollment.before, "the push request preserves the active root worker")
  console.log(`PWA_PUSH_ATTEMPT ${JSON.stringify(enrollment.subscribed
    ? { subscribed: true, before: enrollment.before, after: enrollment.after }
    : enrollment)}`)
  if (!enrollment.subscribed) {
    assert.ok(["AbortError", "NotSupportedError", "Pending"].includes(enrollment.name), JSON.stringify(enrollment))
    await activeWorker("/app/service-worker.js")
    console.log(`PWA_PUSH_UNAVAILABLE ${JSON.stringify(enrollment)}`)
  }
  const initialSubscription = await subscription()
  if (enrollment.subscribed) {
    assert.deepEqual(initialSubscription, enrollment.subscription)
    assert.ok(initialSubscription.endpoint)
    assert.ok(initialSubscription.keys.p256dh)
    assert.ok(initialSubscription.keys.auth)
  } else {
    assert.equal(initialSubscription, null, "a failed enrollment never becomes subscription proof")
  }

  // Password sign-in is a real Turbo form submission. The stored classic choice must replace
  // the signed-out next default without a window load or an html-element replacement.
  await page.locator('input[name="email_address"]').fill(process.env.PWA_BROWSER_EMAIL)
  await page.locator('input[name="password"]').fill("secret123456")
  await page.getByRole("button", { name: "Go", exact: true }).click()
  await page.waitForFunction(() => document.querySelector('meta[name="service-worker-url"]')?.content === "/service-worker.js")
  assert.deepEqual(await page.evaluate(() => ({
    sameDocument: document.documentElement === window.pwaOriginalDocument,
    loads: window.pwaLoadCount,
  })), { sameDocument: true, loads: 0 })
  await activeWorker("/service-worker.js")
  assert.equal(registrationIds.size, 1, "Turbo sign-in keeps Chromium's root registration identity")
  assert.equal(deletedIds.size, 0, "Turbo sign-in does not delete the root registration")
  assert.equal(await page.locator('meta[name="current-user-id"]').getAttribute("content"), "127326141")
  assert.deepEqual(await subscription(), initialSubscription, "Turbo sign-in preserves the root subscription")

  await page.goto(`${origin}/users/me/profile?classic=1`)
  // Password authentication grants the session but still enforces authenticator enrollment.
  // Complete the application's existing security flow instead of bypassing that requirement.
  if (new URL(page.url()).pathname === "/two_factor_setup") {
    const secret = (await page.locator("#two_factor_manual_key").innerText()).replace(/\s+/g, "")
    await page.getByLabel("Authenticator code", { exact: true }).fill(authenticatorCode(secret))
    await page.getByRole("button", { name: "Verify and continue", exact: true }).click()
    await page.getByRole("heading", { name: "Save your backup codes", exact: true }).waitFor()
    await page.getByRole("link", { name: "Continue", exact: true }).click()
    await page.waitForURL((url) => url.pathname === "/users/me/profile")
  }
  assert.equal(await page.locator('meta[name="current-user-id"]').getAttribute("content"), "127326141")
  await page.getByRole("button", { name: "Try the new Smartfire", exact: true }).click()
  await page.waitForURL((url) => url.pathname.startsWith("/app/"))
  await page.getByRole("button", { name: "Your account", exact: true }).waitFor()
  await activeWorker("/app/service-worker.js")
  assert.deepEqual(await subscription(), initialSubscription, "classic to SPA preserves endpoint and keys")

  await page.getByRole("button", { name: "Your account", exact: true }).click()
  await page.getByRole("menuitem", { name: "Switch to classic" }).click()
  await page.waitForURL((url) => !url.pathname.startsWith("/app/"))
  await activeWorker("/service-worker.js")
  assert.deepEqual(await subscription(), initialSubscription, "SPA to classic preserves endpoint and keys")
  assert.equal(registrationIds.size, 1, "Chromium's root registration identity survives every script swap")
  assert.equal(deletedIds.size, 0, "application code never deletes the root registration")
  console.log(`PWA_REGISTRATION_RECEIPT ${JSON.stringify({
    scripts: ["/app/service-worker.js", "/service-worker.js", "/app/service-worker.js", "/service-worker.js"],
    rootRegistrationIds: [...registrationIds],
    subscription: enrollment.subscribed ? "endpoint-and-keys-preserved" : "unavailable-real-getSubscription-null",
    enrollment: enrollment.subscribed ? { subscribed: true, before: enrollment.before, after: enrollment.after } : enrollment,
  })}`)
  await context.close()
} finally {
  await persistentContext?.close()
  await browser.close()
  await rm(profileDirectory, { recursive: true, force: true })
}
