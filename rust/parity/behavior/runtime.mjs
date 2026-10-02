// Shared browser behavior driver. No screenshots, retries or relaxed timing bounds.
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import { parseArgs } from "node:util"
import { chromium } from "playwright"
import { expect as baseExpect } from "playwright/test"
import { startProxy, DEFAULT_ORIGIN } from "../capture/proxy.ts"
import { SessionCache } from "../capture/session.ts"

// The source cases use Capybara's 2-second default, with explicit 10-second message waits.
export const expect = baseExpect.configure({ timeout: 2_000 })
export const visible = (locator, timeout = 2_000) => locator.waitFor({ state: "visible", timeout })
export const scheduled = page => page.goto(new URL("/scheduled_messages", DEFAULT_ORIGIN).href)
export const notice = (page, text) => visible(page.getByText(text, { exact: true }))

export async function waitForComposer(page) {
  // Cable can connect before the lazily imported input controllers. Playwright's
  // atomic fill then emits its only input event before Stimulus can observe it.
  // This is a setup precondition; selector and application waits remain 2s.
  await page.waitForFunction(() => ["composer", "markdown-editor", "markdown-autocomplete"]
    .every(identifier => {
      const element = document.querySelector(`[data-controller~="${identifier}"]`)
      return element && window.Stimulus?.getControllerForElementAndIdentifier(element, identifier)
    }))
}

export async function runSuite(suite, makeCases, setup = {}) {
  const { values } = parseArgs({ options: {
    target: { type: "string" }, name: { type: "string" }, labels: { type: "string" },
  } })
  assert(values.target && values.name && values.labels, "--target, --name and --labels are required")
  const labels = JSON.parse(await readFile(values.labels, "utf8"))
  const room = labels["rooms.designers"]
  assert(Number.isInteger(room), "designers room label required")
  const cases = makeCases({ room, labels })
  const proxy = await startProxy(values.target)
  const browser = await chromium.launch({ headless: true, args: ["--no-sandbox", "--mute-audio"] })
  const target = { name: values.name, url: values.target, origin: DEFAULT_ORIGIN }
  const proxyOptions = { server: proxy.server, bypass: "<-loopback>" }
  const sessions = new SessionCache()
  let passed = 0, failed = 0

  async function pageForCase(user = "jz", roomId = room) {
    const storageState = await sessions.get(browser, target, proxyOptions, user, labels)
    const context = await browser.newContext({ proxy: proxyOptions, storageState,
      timezoneId: "UTC", locale: "en-US", viewport: { width: 1400, height: 1400 } })
    // Freeze Date's wall clock only. Constructed dates and browser timers still work normally.
    await context.addInitScript(() => {
      const RealDate = Date, instant = RealDate.parse("2026-03-02T16:00:00Z")
      function FrozenDate(...args) {
        if (new.target) return args.length ? new RealDate(...args) : new RealDate(instant)
        return new RealDate(instant).toString()
      }
      FrozenDate.prototype = RealDate.prototype
      FrozenDate.now = () => instant
      FrozenDate.parse = RealDate.parse
      FrozenDate.UTC = RealDate.UTC
      window.Date = FrozenDate
    })
    await context.route(url => /^https?:$/.test(url.protocol) && url.origin !== DEFAULT_ORIGIN,
      route => route.abort("blockedbyclient"))
    const page = await context.newPage()
    page.setDefaultTimeout(2_000)
    page.setDefaultNavigationTimeout(30_000)
    page.on("dialog", dialog => dialog.accept())
    try {
      await setup.beforeVisit?.(page)
      const response = await page.goto(new URL(`/rooms/${roomId}`, DEFAULT_ORIGIN).href)
      assert.equal(response?.status(), 200, "room setup HTTP status")
      assert.equal(new URL(page.url()).pathname, `/rooms/${roomId}`, "room setup final URL")
      await visible(page.getByLabel("Write a message", { exact: true }))
      // SystemTestHelper#join_room waits for every stream (at least three) for 15s.
      // The visible textarea alone can precede Stimulus connecting its input handler.
      await page.waitForFunction(() => {
        const total = document.querySelectorAll("turbo-cable-stream-source").length
        const connected = document.querySelectorAll("turbo-cable-stream-source[connected]").length
        return total >= 3 && connected === total
      }, undefined, { timeout: 15_000 })
      const pwa = page.locator("[data-pwa-install-target~='dialog']")
      if (await pwa.isVisible()) await pwa.getByRole("button", { name: "Close", exact: true }).click()
      await setup.afterJoinRoom?.(page)
      if (!setup.legacyComposerSetup) await waitForComposer(page)
      return { page, context }
    } catch (error) { await context.close(); throw error }
  }

  async function withUser(user, roomId, run) {
    const state = await pageForCase(user, roomId)
    try { return await run(state.page) } finally { await state.context.close() }
  }

  try {
    for (const [name, run] of cases) {
      let context
      try {
        const state = await pageForCase(); context = state.context
        await run(state.page, { withUser })
        passed++
        console.log(`PASS ${values.name}: ${name}`)
      } catch (error) {
        failed++
        console.error(`FAIL ${values.name}: ${name}: ${error.stack ?? error}`)
      } finally { await context?.close() }
    }
  } finally { await browser.close(); await proxy.close() }
  console.log(`WS8bm2 browser ${suite} (${values.name}): ${passed}/${cases.length} passed; ${failed} failed`)
  if (failed) process.exitCode = 1
}
