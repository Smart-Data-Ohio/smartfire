// All four test/system/polls_test.rb interactions, with public HTTP creates.
import assert from "node:assert/strict"
import { DEFAULT_ORIGIN } from "../capture/proxy.ts"
import { runSuite, visible, expect, waitForRoom, waitForComposer } from "./runtime.mjs"

async function createPoll(page, room, attributes) {
  const result = await page.evaluate(async ({ room, attributes }) => {
    const response = await fetch(`/rooms/${room}/polls`, { method: "POST",
      headers: { Accept: "application/json", "Content-Type": "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']").content },
      body: JSON.stringify({ poll: attributes }) })
    return { status: response.status, poll: await response.json() }
  }, { room, attributes })
  assert.equal(result.status, 201, "poll fixture create status")
  assert(result.poll.id, "persisted poll ID")
  await page.reload()
  await waitForRoom(page)
  await waitForComposer(page)
  return `#card_poll_${result.poll.id}`
}
const cases = ({ room, labels }) => [
  ["creates a poll from the builder and changes a vote", async page => {
    const old = await page.locator(".poll").evaluateAll(nodes => nodes.map(n => n.id))
    await page.getByLabel("Write a message", { exact: true }).fill("/poll")
    await page.getByRole("button", { name: "Send Message", exact: true }).click()
    const builder = page.locator("#poll-builder[open]")
    await visible(builder)
    await builder.getByLabel("Question", { exact: true }).fill("Lunch?")
    await builder.getByLabel("Option 1", { exact: true }).fill("Tacos")
    await builder.getByLabel("Option 2", { exact: true }).fill("Pizza")
    await builder.getByRole("button", { name: "Add option", exact: true }).click()
    await builder.getByLabel("Option 3 (optional)", { exact: true }).fill("Sushi")
    await builder.getByRole("button", { name: "Post poll", exact: true }).click()
    const card = page.locator(`.poll${old.map(id => `:not([id=${JSON.stringify(id)}])`).join("")}`)
    await visible(card, 10_000)
    const id = await card.getAttribute("id")
    // Every vote replaces the card, so the locator resolves the fresh node.
    const live = page.locator(`[id=${JSON.stringify(id)}]`)
    await expect(live.locator(".poll__label")).toContainText(["Tacos", "Pizza", "Sushi"])
    await expect(live.locator(".poll__retract-form .btn")).not.toBeVisible()
    await live.getByLabel("Tacos", { exact: true }).check()
    await live.getByRole("button", { name: "Vote", exact: true }).click()
    await visible(live.locator(".poll__option--voted .poll__label").filter({ hasText: "Tacos" }))
    await visible(live.locator(".poll__count").filter({ hasText: "1 · 100%" }))
    await visible(live.locator(".poll__voters").filter({ hasText: "JZ" }))
    await live.getByLabel("Pizza", { exact: true }).check()
    await live.getByRole("button", { name: "Change vote", exact: true }).click()
    await visible(live.locator(".poll__option--voted .poll__label").filter({ hasText: "Pizza" }))
    await live.getByRole("button", { name: "Retract vote", exact: true }).click()
    await expect(live.locator(".poll__option--voted")).toHaveCount(0)
    await expect(live.locator(".poll__meta")).toContainText("0 votes")
  }],
  ["multiple-choice anonymous polls hide voters", async page => {
    const selector = await createPoll(page, room, { question: "Snacks?", options: ["Chips", "Fruit"], multiple: true, anonymous: true })
    const card = page.locator(selector)
    await card.getByLabel("Chips", { exact: true }).check()
    await card.getByLabel("Fruit", { exact: true }).check()
    await card.getByRole("button", { name: "Vote", exact: true }).click()
    await expect(card).toContainText("2 votes")
    await expect(card).not.toContainText("JZ")
    await expect(card.locator(".poll__meta")).toContainText("Anonymous")
    await page.reload()
    await waitForRoom(page)
    await expect(card.locator(".poll__option--voted .poll__label")).toContainText(["Chips", "Fruit"])
  }],
  ["results update live in another session", async (page, { withUser }) => {
    const selector = await createPoll(page, room, { question: "Lunch?", options: ["Tacos", "Pizza"] })
    await withUser("jason", room, async other => {
      await visible(other.locator(selector))
      const card = page.locator(selector)
      await card.getByLabel("Tacos", { exact: true }).check()
      await card.getByRole("button", { name: "Vote", exact: true }).click()
      await visible(card.locator(".poll__count").filter({ hasText: "1 · 100%" }))
      await visible(other.locator(`${selector} .poll__count`).filter({ hasText: "1 · 100%" }), 15_000)
      await visible(other.locator(`${selector} .poll__voters`).filter({ hasText: "JZ" }))
    })
  }],
  ["closed polls show results without the form", async page => {
    const id = labels["polls.closed"]
    const poll = await page.evaluate(async ({ room, id }) => {
      const response = await fetch(`/rooms/${room}/polls/${id}`, { headers: { Accept: "application/json" } })
      return response.json()
    }, { room, id })
    await page.goto(new URL(`/rooms/${room}/@${poll.message_id}`, DEFAULT_ORIGIN).href)
    await waitForRoom(page)
    const card = page.locator(`#card_poll_${id}`)
    await visible(card.locator(".poll__meta").filter({ hasText: "Closed" }))
    await expect(card.locator("form")).toHaveCount(0)
  }],
]
await runSuite("polls", cases)
