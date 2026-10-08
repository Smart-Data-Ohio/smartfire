// Real Rust pages, vendored Turbo/classic overrides and the production SPA entry.
// The Rust test owns the private seed/server. After seeding a legacy registration,
// sign-in and UI switches reconcile it through application code.
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
  // Quiet startup keeps the existing test-environment opt-out on the sign-in page, whose auth.js
  // registers on load. The page still names the signed-out (SPA_DEFAULT=next) worker.
  const disabled = await browser.newContext()
  const disabledPage = await disabled.newPage()
  await disabledPage.goto(`${origin}/session/new`, { waitUntil: "load" })
  assert.equal(await disabledPage.getAttribute("html", "data-service-worker"), "false")
  assert.equal(await disabledPage.locator('meta[name="service-worker-url"]').getAttribute("content"), "/service-worker.js")
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

  // Polls from Node: waitForFunction treats an async predicate's promise as already truthy.
  // A navigation in progress destroys the context, which only means "not yet".
  async function activeWorker(path) {
    const deadline = Date.now() + 30_000
    let state = null
    for (;;) {
      try {
        state = await page.evaluate(async () => {
          const registrations = await navigator.serviceWorker.getRegistrations()
          return {
            count: registrations.length,
            scope: registrations[0]?.scope ?? null,
            active: registrations[0]?.active?.scriptURL ?? null,
            state: registrations[0]?.active?.state ?? null,
            controller: navigator.serviceWorker.controller?.scriptURL ?? null,
            installing: registrations[0]?.installing?.scriptURL ?? null,
            waiting: registrations[0]?.waiting?.scriptURL ?? null,
            page: location.pathname,
            dataset: document.documentElement.dataset.serviceWorker ?? null,
            meta: document.querySelector('meta[name="service-worker-url"]')?.content ?? null,
            ready: document.readyState,
          }
        })
      } catch (error) {
        state = { error: error.message }
      }
      if (state.count === 1
        && state.scope === `${origin}/`
        && state.active === `${origin}${path}`
        && state.state === "activated"
        && state.controller === `${origin}${path}`) return
      if (Date.now() > deadline) assert.fail(`the root registration never activated ${path}: ${JSON.stringify(state)}`)
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
  }

  async function subscription() {
    return page.evaluate(async () => {
      const registration = await navigator.serviceWorker.getRegistration("/")
      const current = await registration.pushManager.getSubscription()
      return current === null ? null : current.toJSON()
    })
  }

  // A legacy SPA bundle used the /app script at the same root scope. Seed that registration
  // before application startup to prove the canonical update preserves its native identity.
  await page.goto(`${origin}/offline.html`)
  await page.evaluate(async () => {
    await navigator.serviceWorker.register("/app/service-worker.js", { scope: "/", updateViaCache: "none" })
  })
  await activeWorker("/app/service-worker.js")
  // This is an explicit test subscription, using the application's real VAPID key and browser API.
  // Record any native failure exactly; an unsuccessful attempt never proves subscription survival.
  const enrollment = await page.evaluate(async (encoded) => {
    const registration = await Promise.race([
      navigator.serviceWorker.ready,
      new Promise((_, reject) => setTimeout(() => reject(new Error("no service worker became ready")), 30_000)),
    ])
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
    if (!encoded) throw new Error("the Rust fixture must supply its valid VAPID key")
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
  }, process.env.PWA_BROWSER_VAPID_KEY)
  assert.equal(enrollment.before.permission, "granted")
  assert.equal(enrollment.before.state, "activated")
  assert.deepEqual(enrollment.after, enrollment.before, "the push request preserves the active root worker")
  console.log(`PWA_PUSH_ATTEMPT ${JSON.stringify(enrollment.subscribed
    ? { subscribed: true, before: enrollment.before, after: enrollment.after }
    : enrollment)}`)
  // Preservation is only proven with a real subscription. An environment whose Chromium has no
  // push service must opt out explicitly, and then the preservation checks are reported skipped.
  const skipPreservation = !enrollment.subscribed
  if (skipPreservation) {
    assert.ok(
      process.env.PWA_ALLOW_PUSH_UNAVAILABLE === "1",
      `Chromium could not create a push subscription, so preservation is unproven: ${JSON.stringify(enrollment)}`,
    )
    await activeWorker("/app/service-worker.js")
    console.log(`PWA_PUSH_PRESERVATION_SKIPPED ${JSON.stringify(enrollment)}`)
  }
  const initialSubscription = await subscription()
  if (!skipPreservation) {
    assert.deepEqual(initialSubscription, enrollment.subscription)
    assert.ok(initialSubscription.endpoint)
    assert.ok(initialSubscription.keys.p256dh)
    assert.ok(initialSubscription.keys.auth)
  }
  const assertPreserved = async (message) => {
    if (skipPreservation) return
    const current = await subscription()
    assert.ok(current?.endpoint, `${message}: a subscription is still present`)
    assert.deepEqual(current, initialSubscription, message)
  }

  await page.goto(`${origin}/session/new`)
  assert.equal(await page.locator('meta[name="service-worker-url"]').getAttribute("content"), "/service-worker.js")
  await activeWorker("/service-worker.js")
  assert.equal(registrationIds.size, 1, "canonical update keeps the legacy root registration identity")
  assert.equal(deletedIds.size, 0, "canonical update never deletes the legacy root registration")
  await assertPreserved("canonical worker update preserves the legacy subscription")
  await page.evaluate(() => {
    window.pwaOriginalDocument = document.documentElement
  })

  // Password sign-in is a plain form submission: the auth pages have no Turbo, so the page it
  // lands on (a classic page, or two-step setup, itself an auth page) loads afresh. Its head
  // keeps the canonical root worker for the stored classic choice, and
  // its load reconciles that script over the same root registration.
  await page.locator('input[name="email_address"]').fill(process.env.PWA_BROWSER_EMAIL)
  await page.locator('input[name="password"]').fill("secret123456")
  await page.getByRole("button", { name: "Sign in", exact: true }).click()
  await page.waitForURL((url) => url.pathname !== "/session/new")
  await page.waitForFunction(() => document.querySelector('meta[name="service-worker-url"]')?.content === "/service-worker.js")
  assert.equal(await page.evaluate(() => window.pwaOriginalDocument), undefined, "sign-in loads a new document")
  await activeWorker("/service-worker.js")
  assert.equal(registrationIds.size, 1, "sign-in keeps Chromium's root registration identity")
  assert.equal(deletedIds.size, 0, "sign-in does not delete the root registration")
  assert.equal(await page.locator('meta[name="current-user-id"]').getAttribute("content"), "127326141")
  console.log(`PWA_SIGN_IN_LANDING ${new URL(page.url()).pathname}`)
  await assertPreserved("sign-in preserves the root subscription")

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
  await activeWorker("/service-worker.js")
  await assertPreserved("classic to SPA preserves endpoint and keys")

  await page.getByRole("button", { name: "Your account", exact: true }).click()
  await page.getByRole("menuitem", { name: "Switch to classic" }).click()
  await page.waitForURL((url) => !url.pathname.startsWith("/app/"))
  await activeWorker("/service-worker.js")
  await assertPreserved("SPA to classic preserves endpoint and keys")
  assert.equal(registrationIds.size, 1, "Chromium's root registration identity survives every script swap")
  assert.equal(deletedIds.size, 0, "application code never deletes the root registration")
  console.log(`PWA_REGISTRATION_RECEIPT ${JSON.stringify({
    scripts: ["/app/service-worker.js", "/service-worker.js", "/service-worker.js", "/service-worker.js"],
    rootRegistrationIds: [...registrationIds],
    subscription: skipPreservation ? "preservation-skipped-push-unavailable" : "endpoint-and-keys-preserved",
    enrollment: enrollment.subscribed ? { subscribed: true, before: enrollment.before, after: enrollment.after } : enrollment,
  })}`)
  await context.close()
} finally {
  await persistentContext?.close()
  await browser.close()
  await rm(profileDirectory, { recursive: true, force: true })
}
