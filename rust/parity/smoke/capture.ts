import fs from "node:fs"
import path from "node:path"
import { chromium } from "playwright"
import type { Browser, Page } from "playwright"
import { DEFAULT_ORIGIN, startProxy } from "../capture/proxy.ts"
import { pages, interpolate } from "./pages.ts"
import type { SmokePage } from "./pages.ts"

const args = process.argv.slice(2)
function option(name: string): string {
  const i = args.indexOf(name)
  if (i < 0 || !args[i + 1]) throw new Error(`missing ${name}`)
  return args[i + 1]
}
const out = path.resolve(option("--out"))
const labels = JSON.parse(fs.readFileSync(option("--labels"), "utf8")) as Record<string, unknown>
const time = String(labels["clock.now"])
if (!labels["session_cookies.david"] || !Number.isFinite(Date.parse(time))) throw new Error("seed needs David's verified session and clock.now")
const referenceRoot = path.resolve(option("--reference-root"))
const targets = [{ app: "rails", url: option("--rails") }, { app: "rust", url: option("--rust") }]
for (const target of targets) {
  if (!/^http:\/\/127\.0\.0\.1:\d+$/.test(target.url)) throw new Error(`only local parity targets are permitted: ${target.url}`)
}
const widths = [{ name: "desktop", width: 1440, height: 900 }, { name: "phone", width: 390, height: 844 }]
const only = args.includes("--only") ? new Set(option("--only").split(",")) : undefined
const inventory = pages.filter(page => !only || only.has(page.id))
if (!inventory.length || (only && [...only].some(id => !pages.some(page => page.id === id)))) throw new Error("--only has an unknown page id")

interface Diagnostic {
  app: string
  page: string
  title: string
  fullPage: boolean
  blockMedia: boolean
  viewport: { name: string; width: number; height: number }
  requestedPath?: string
  finalUrl?: string
  initialStatus?: number
  finalDocumentStatus?: number
  png: string
  consoleErrors: { text: string; location: unknown }[]
  pageErrors: string[]
  httpErrors: { url: string; method: string; status: number; resourceType: string }[]
  requestFailures: { url: string; method: string; resourceType: string; error: string | null }[]
  blockedExternalRequests: string[]
  fixtureRequests: string[]
  captureErrors: string[]
  evidence?: unknown
  durationMs?: number
}

const manifest: Diagnostic[] = []
let browser: Browser | undefined
let stopping = false
async function stop(signal: string) {
  if (stopping) return
  stopping = true
  await browser?.close().catch(() => {})
  fs.writeFileSync(path.join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n")
  process.exit(signal === "SIGINT" ? 130 : 143)
}
process.once("SIGINT", () => { void stop("SIGINT") })
process.once("SIGTERM", () => { void stop("SIGTERM") })
fs.mkdirSync(out, { recursive: true })

async function settle(page: Page, result: Diagnostic) {
  await page.waitForLoadState("load", { timeout: 15_000 }).catch(error => result.captureErrors.push(`load: ${String(error)}`))
  await page.waitForLoadState("networkidle", { timeout: 5000 }).catch(() => {})
  await page.evaluate(async () => {
    await Promise.race([document.fonts.ready, new Promise(resolve => setTimeout(resolve, 3000))])
    const visible = [...document.images].filter(image => {
      const rect = image.getBoundingClientRect()
      return rect.bottom > 0 && rect.right > 0 && rect.top < innerHeight && rect.left < innerWidth
    })
    await Promise.race([
      Promise.all(visible.map(image => image.complete ? Promise.resolve() : new Promise(resolve => {
        image.addEventListener("load", resolve, { once: true })
        image.addEventListener("error", resolve, { once: true })
      }))),
      new Promise(resolve => setTimeout(resolve, 3000)),
    ])
  }).catch(error => result.captureErrors.push(`fonts/images: ${String(error)}`))
  // Leave real timers running for Stimulus, Action Cable and lazy images.
  await page.waitForTimeout(750)
}

async function capture(browser: Browser, target: typeof targets[number], viewport: typeof widths[number], spec: SmokePage) {
  const started = performance.now()
  const base = path.join(out, target.app, spec.id, viewport.name)
  fs.mkdirSync(path.dirname(base), { recursive: true })
  // A failed rerun must never appear to have kept its previous successful image.
  for (const extension of [".png", ".json", ".html"]) fs.rmSync(base + extension, { force: true })
  const result: Diagnostic = {
    app: target.app, page: spec.id, title: spec.title, fullPage: spec.fullPage ?? false, blockMedia: spec.blockMedia ?? false, viewport, png: base + ".png",
    consoleErrors: [], pageErrors: [], httpErrors: [], requestFailures: [],
    blockedExternalRequests: [], fixtureRequests: [], captureErrors: [],
  }
  const proxy = await startProxy(target.url)
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height }, deviceScaleFactor: 1,
    isMobile: viewport.name === "phone", hasTouch: viewport.name === "phone",
    colorScheme: "light", timezoneId: "UTC", locale: "en-US", serviceWorkers: "block",
    proxy: { server: proxy.server, bypass: "<-loopback>" },
  })
  try {
    if (spec.blockMedia) await context.addInitScript(() => {
      const state = globalThis as typeof globalThis & { __smokeBlockedMediaRequests: string[] }
      state.__smokeBlockedMediaRequests = []
      const deny = (method: string) => () => {
        state.__smokeBlockedMediaRequests.push(method)
        return Promise.reject(new DOMException("Cutover smoke capture blocks real media", "NotAllowedError"))
      }
      // Replace the capture APIs before application code loads. Never call them.
      if (navigator.mediaDevices) for (const method of ["getUserMedia", "getDisplayMedia"]) {
        Object.defineProperty(navigator.mediaDevices, method, { value: deny(method), configurable: true })
      }
    })
    if (!spec.anonymous) await context.addCookies([{
      name: "session_token", value: String(labels["session_cookies.david"]),
      url: DEFAULT_ORIGIN, httpOnly: true, secure: false, sameSite: "Lax",
    }])
    const origin = new URL(DEFAULT_ORIGIN).origin
    await context.route(url => /^https?:$/.test(url.protocol) && url.origin !== origin, async route => {
      result.blockedExternalRequests.push(route.request().url())
      await route.abort("blockedbyclient")
    })
    for (const glob of ["https://example.com/og/**", "https://pbs.twimg.com/profile_images/**"]) {
      await context.route(glob, async route => {
        result.fixtureRequests.push(route.request().url())
        await route.fulfill({ path: path.join(referenceRoot, "test/fixtures/files/moon.jpg"), contentType: "image/jpeg" })
      })
    }
    const page = await context.newPage()
    page.setDefaultTimeout(8000)
    // Date stays at the seed instant; native timer scheduling continues normally.
    await page.clock.setFixedTime(new Date(time))
    page.on("console", message => {
      if (message.type() === "error") result.consoleErrors.push({ text: message.text(), location: message.location() })
    })
    page.on("pageerror", error => result.pageErrors.push(String(error)))
    page.on("requestfailed", request => result.requestFailures.push({
      url: request.url(), method: request.method(), resourceType: request.resourceType(), error: request.failure()?.errorText ?? null,
    }))
    page.on("response", response => {
      const request = response.request()
      if (response.status() >= 400) result.httpErrors.push({ url: response.url(), method: request.method(), status: response.status(), resourceType: request.resourceType() })
      if (request.isNavigationRequest() && request.frame() === page.mainFrame()) result.finalDocumentStatus = response.status()
    })
    page.on("dialog", dialog => { void dialog.dismiss() })
    try {
      result.requestedPath = interpolate(spec.path, labels)
      const response = await page.goto(new URL(result.requestedPath, DEFAULT_ORIGIN).href, { waitUntil: "domcontentloaded", timeout: 25_000 })
      result.initialStatus = response?.status()
      await settle(page, result)
      for (const step of spec.steps ?? []) {
        if (step.viewport && step.viewport !== viewport.name) continue
        try {
          if ("click" in step) await page.locator(interpolate(step.click, labels)).click()
          else await page.locator(interpolate(step.wait_for, labels)).waitFor({ state: "visible" })
        } catch (error) { throw new Error(`step ${JSON.stringify(step)}: ${String(error)}`) }
      }
      if (spec.scroll === "bottom") {
        await page.locator(".messages").first().evaluate(element => { element.scrollTop = element.scrollHeight }).catch(error => result.captureErrors.push(`scroll: ${String(error)}`))
      } else if (spec.scroll) {
        const selector = interpolate(spec.scroll, labels)
        await page.locator(selector).scrollIntoViewIfNeeded().catch(error => result.captureErrors.push(`scroll ${selector}: ${String(error)}`))
      }
      if (spec.scrollContainerBottom) {
        const selector = interpolate(spec.scrollContainerBottom, labels)
        await page.locator(selector).evaluate(element => { element.scrollTop = element.scrollHeight }).catch(error => result.captureErrors.push(`scroll container ${selector}: ${String(error)}`))
      }
      await settle(page, result)
    } catch (error) {
      result.captureErrors.push(String(error))
    }
    result.finalUrl = page.url()
    result.evidence = await page.evaluate(() => ({
      title: document.title,
      bodyClass: document.body?.className ?? "",
      window: { innerWidth, innerHeight, scrollX, scrollY, devicePixelRatio },
      workspaceOpeners: [...document.querySelectorAll('[aria-label="Open workspace navigation"]')].map(element => {
        const style = getComputedStyle(element)
        const rect = element.getBoundingClientRect()
        return {
          className: element.className,
          display: style.display, visibility: style.visibility, opacity: style.opacity, position: style.position, zIndex: style.zIndex,
          rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height, top: rect.top, right: rect.right, bottom: rect.bottom, left: rect.left },
        }
      }),
      bodyText: document.body?.innerText ?? "",
      horizontalOverflow: Math.max(document.body?.scrollWidth ?? 0, document.documentElement.scrollWidth) > innerWidth,
      scrollContainers: [...document.querySelectorAll("#main-content,.messages")].map(element => ({
        id: element.id, className: element.className, scrollTop: element.scrollTop,
        scrollHeight: element.scrollHeight, clientHeight: element.clientHeight,
      })),
      visibleMainContent: [...document.querySelectorAll("#main-content h1,#main-content h2,#main-content h3,#main-content legend,#main-content a,#main-content button,#main-content p")].filter(element => {
        const rect = element.getBoundingClientRect()
        return rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth
      }).map(element => ({ tag: element.tagName, id: element.id, text: element.textContent?.trim() })).filter(element => element.text).slice(-40),
      headings: [...document.querySelectorAll("h1,h2,h3")].map(element => element.textContent?.trim()),
      messageCount: document.querySelectorAll(".message[data-message-id]").length,
      attachmentCount: document.querySelectorAll(".message .attachment, .message [data-controller~=attachment]").length,
      reactionGroups: [...document.querySelectorAll('[aria-label="Message reactions"]')].map(element => element.textContent?.trim()),
      threadCounts: [...document.querySelectorAll(".message__thread-indicator:not([hidden])")].map(element => element.textContent?.trim()),
      huddleState: document.querySelector("#channel-huddle")?.getAttribute("data-state") ?? null,
      blockedMediaRequests: (globalThis as typeof globalThis & { __smokeBlockedMediaRequests?: string[] }).__smokeBlockedMediaRequests ?? [],
      brokenImages: [...document.images].filter(image => {
        // Hidden lightbox templates and lazy images have not attempted delivery.
        // Empty src resolves to the document URL; it is not a broken asset.
        const explicitSource = image.getAttribute("src")?.trim() || image.getAttribute("srcset")?.trim()
        const rect = image.getBoundingClientRect()
        return !!explicitSource && image.complete && image.naturalWidth === 0 && rect.width > 0 && rect.height > 0 &&
          rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth
      }).map(image => ({ src: image.currentSrc || image.src, alt: image.alt })),
      visibleImages: [...document.images].filter(image => {
        const rect = image.getBoundingClientRect()
        return rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth
      }).map(image => ({ src: image.currentSrc || image.src, alt: image.alt, loaded: image.complete && image.naturalWidth > 0 })),
      buttons: [...document.querySelectorAll("button")].map(button => ({ text: button.textContent?.trim(), label: button.getAttribute("aria-label") })),
    })).catch(error => { result.captureErrors.push(`evidence: ${String(error)}`); return undefined })
    await page.screenshot({ path: result.png, fullPage: spec.fullPage ?? false, timeout: 20_000 }).catch(error => result.captureErrors.push(`screenshot: ${String(error)}`))
    fs.writeFileSync(base + ".html", await page.content().catch(() => ""))
  } finally {
    await context.close().catch(() => {})
    await proxy.close()
    result.durationMs = Math.round(performance.now() - started)
    fs.writeFileSync(base + ".json", JSON.stringify(result, null, 2) + "\n")
    manifest.push(result)
    fs.writeFileSync(path.join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n")
    console.log(`${target.app} ${spec.id} ${viewport.name}: HTTP ${result.finalDocumentStatus ?? "?"}; ${result.consoleErrors.length} console errors; ${result.httpErrors.length} HTTP errors; ${result.captureErrors.length} capture issues`)
  }
}

try {
  browser = await chromium.launch({ headless: true })
  for (const spec of inventory) for (const viewport of widths) for (const target of targets) await capture(browser, target, viewport, spec)
} finally {
  await browser?.close()
}

const rows = inventory.flatMap(spec => widths.map(viewport => {
  const cells = targets.map(target => {
    const result = manifest.find(result => result.page === spec.id && result.viewport.name === viewport.name && result.app === target.app)
    return result ? `![${target.app}](${result.png})<br>[diagnostics](${result.png.replace(/\.png$/, ".json")})` : "Capture missing"
  })
  return `| ${spec.title} (${viewport.width}×${viewport.height}) | ${cells.join(" | ")} | Reviewer assessment pending |`
}))
fs.writeFileSync(path.join(out, "CONTACT.md"), [
  "# Cutover capture contact sheet", "", `Seed clock: ${time}. Screenshots use the same seeded David session; sign-in is anonymous.`,
  "No pixel comparisons or gates. Fill in the separate committed REPORT.md after visual review.", "",
  "| Page × width | Rails | Rust | Assessment |", "| --- | --- | --- | --- |", ...rows, "",
].join("\n"))
// This reports capture infrastructure/step failure; application diagnostics remain
// review evidence and never become a pixel or appearance gate.
if (manifest.some(result => result.captureErrors.length || !fs.existsSync(result.png))) process.exitCode = 1
