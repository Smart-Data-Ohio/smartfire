import assert from "node:assert/strict"
import { readFileSync } from "node:fs"
import { test } from "node:test"

// Resolve the browser importmap's helper without adding a JS build or DOM dependency.
const source = readFileSync(new URL("../../app/javascript/models/client_message.js", import.meta.url), "utf8")
const helper = new URL("../../app/javascript/helpers/string_helpers.js", import.meta.url).href
const { default: ClientMessage } = await import(`data:text/javascript,${encodeURIComponent(source.replace('"helpers/string_helpers"', JSON.stringify(helper)))}`)

function message({ delivered = false } = {}) {
  const body = { innerHTML: "server attachment and reply preview", children: [], append(node) { this.children.push(node) } }
  const classes = new Set()
  return {
    body,
    classList: { add(name) { classes.add(name) }, contains(name) { return classes.has(name) } },
    hasAttribute(name) { return name === "data-message-id" && delivered },
    querySelector(selector) {
      if (selector === ".message__body-content") return body
      if (selector === ".message__recover-draft") return body.children[0]
    }
  }
}

function client(element) {
  globalThis.document = {
    querySelector(selector) { assert.equal(selector, "#message_upload"); return element },
    createElement() { return { dataset: {} } }
  }
  return new ClientMessage(null)
}

test("upload progress updates a pending message", () => {
  const pending = message()
  client(pending).update("upload", "50%")
  assert.equal(pending.body.innerHTML, "50%")
})

test("late progress cannot overwrite a delivered message", () => {
  const delivered = message({ delivered: true })
  client(delivered).update("upload", "100%")
  assert.equal(delivered.body.innerHTML, "server attachment and reply preview")
})

test("late failure cannot mark a delivered message as pending or recoverable", () => {
  const delivered = message({ delivered: true })
  client(delivered).failed("upload")
  assert.equal(delivered.classList.contains("message--failed"), false)
  assert.equal(delivered.body.children.length, 0)
})

test("a pending failure still offers draft recovery once", () => {
  const pending = message()
  const model = client(pending)
  model.failed("upload")
  model.failed("upload")
  assert.equal(pending.classList.contains("message--failed"), true)
  assert.equal(pending.body.children.length, 1)
  assert.equal(pending.body.children[0].dataset.clientMessageId, "upload")
})

test("progress and failure after a message leaves the page are harmless", () => {
  const model = client(null)
  assert.doesNotThrow(() => model.update("upload", "100%"))
  assert.doesNotThrow(() => model.failed("upload"))
})
