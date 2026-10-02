// Hold an actual search response, not application assets or synthetic markup.
import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import { readFile } from "node:fs/promises"
import { runSuite, visible, expect } from "./runtime.mjs"
import { createMessage } from "./messages.mjs"
import { searchUploads } from "./file-search.mjs"

const legacy = process.argv.includes("--legacy-setup")
process.argv = process.argv.filter(arg => arg !== "--legacy-setup")

await runSuite("Files search readiness", ({ room }) => [["filename search renders before the Images click", async page => {
  const query = `file-readiness-${randomUUID()}`
  const filename = `${query}.png`
  const base64 = (await readFile(new URL("../../../test/fixtures/files/earth.png", import.meta.url))).toString("base64")
  await createMessage(page, room, { body: "Files readiness" }, { base64, name: filename, type: "image/png" })
  await page.reload()
  await page.getByRole("link", { name: "Show files", exact: true }).click()
  const upload = page.locator(".room-files__name").filter({ hasText: filename })
  const images = page.getByRole("link", { name: "Images", exact: true })
  await visible(upload, 10_000)
  assert.equal(new URL(await images.getAttribute("href"), page.url()).searchParams.has("filename"), false)

  const received = Promise.withResolvers(), release = Promise.withResolvers()
  const path = `/rooms/${room}/files`
  const isSearch = url => url.pathname === path && url.searchParams.get("filename") === query
    && url.searchParams.get("type") === "all"
  let responseInfo, searchCompleted = false
  await page.route(isSearch, async route => {
    try {
      const response = await route.fetch()
      assert.equal(response.status(), 200)
      responseInfo = await page.evaluate(html => {
        const doc = new DOMParser().parseFromString(html, "text/html")
        const link = [...doc.querySelectorAll(".room-files__type")].find(link => link.textContent.trim() === "Images")
        return { uploads: doc.querySelectorAll(".room-files__name").length, images: link.getAttribute("href") }
      }, await response.text())
      assert.equal(responseInfo.uploads, 1, "the actual app's search response must filter correctly")
      assert.equal(new URL(responseInfo.images, page.url()).searchParams.get("filename"), query)
      console.log(`TRACE search response: status=200 uploads=${responseInfo.uploads} Images carries filename=true`)
      received.resolve()
      await release.promise
      try { await route.fulfill({ response }) } catch (error) {
        if (!legacy || !route.request().failure()) throw error
      }
    } catch (error) { received.reject(error); throw error }
  })

  try {
    const search = (legacy ? (async () => {
      // Preserve exactly the former driver's non-discriminating search wait.
      await page.getByLabel("Search by filename", { exact: true }).fill(query)
      await page.getByRole("button", { name: "Search", exact: true }).click()
      await visible(upload, 10_000)
    })() : searchUploads(page, query, filename)).then(() => { searchCompleted = true })
    await received.promise
    // Both old pre-submit assertions are already satisfied while delivery is held.
    await visible(upload, 10_000)
    console.log(`TRACE search held: old filename visible=true Images carries filename=${new URL(await images.getAttribute("href"), page.url()).searchParams.has("filename")}`)
    if (legacy) {
      await search
    } else {
      assert.equal(searchCompleted, false, "search must not complete against the pre-submit DOM")
      release.resolve()
      await search
    }
    const request = page.waitForRequest(request => {
      const url = new URL(request.url())
      return url.pathname === path && url.searchParams.get("type") === "images"
    })
    const response = page.waitForResponse(response => {
      const url = new URL(response.url())
      return url.pathname === path && url.searchParams.get("type") === "images"
    })
    await images.click()
    const sent = new URL((await request).url())
    console.log(`TRACE Images request: filename present=${sent.searchParams.has("filename")}`)
    if (!legacy) assert.equal(sent.searchParams.get("filename"), query)
    assert.equal((await response).status(), 200)
    release.resolve()
    await expect(page.locator(".room-files__name")).toHaveCount(1, { timeout: 10_000 })
    await visible(upload, 10_000)
  } finally { release.resolve() }
}]])
