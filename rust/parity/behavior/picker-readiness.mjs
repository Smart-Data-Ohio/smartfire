// Deterministically reproduce the atomic-fill/lazy-controller setup race on either app.
// --legacy-setup runs the former driver ordering and must fail at the unchanged 2s bound.
import assert from "node:assert/strict"
import { runSuite, visible, expect } from "./runtime.mjs"

const legacy = process.argv.includes("--legacy-setup")
process.argv = process.argv.filter(arg => arg !== "--legacy-setup")
const releases = new WeakMap()
const editor = page => page.getByLabel("Write a message", { exact: true })

await runSuite("picker readiness", () => [["lazy autocomplete connects before the first input", async page => {
  if (!legacy) await editor(page).fill("/poll")
  await visible(page.locator("suggestion-option").filter({ hasText: "/poll" }))
  await editor(page).press("Enter")
  await visible(page.locator("#poll-builder[open]"))
  await expect(editor(page)).toHaveValue("")
}]], {
  legacyComposerSetup: legacy,
  async beforeVisit(page) {
    const gate = new Promise(resolve => releases.set(page, resolve))
    await page.route(/\/assets\/controllers\/markdown_autocomplete_controller-[^/]+\.js$/, async route => {
      await gate
      await route.continue()
    })
  },
  async afterJoinRoom(page) {
    assert.equal(await page.evaluate(() => {
      const element = document.querySelector('[data-controller~="markdown-autocomplete"]')
      return !!window.Stimulus?.getControllerForElementAndIdentifier(element, "markdown-autocomplete")
    }), false, "lazy controller must still be gated at the cable setup barrier")
    if (legacy) await editor(page).fill("/poll")
    releases.get(page)()
  },
})
