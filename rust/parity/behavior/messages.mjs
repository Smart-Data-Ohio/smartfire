import assert from "node:assert/strict"

// Test fixture setup through the real create endpoint, using the requested creator's session.
// Do not insert generated rows behind either app's validation/callback chain.
export async function createMessage(page, room, attributes, attachment = null) {
  const result = await page.evaluate(async ({ room, attributes, attachment }) => {
    const headers = { Accept: "text/vnd.turbo-stream.html",
      "X-CSRF-Token": document.querySelector("meta[name='csrf-token']").content }
    let body
    if (attachment) {
      body = new FormData()
      for (const [name, value] of Object.entries(attributes)) body.append(`message[${name}]`, value)
      body.append("message[attachment]", new File([
        Uint8Array.from(atob(attachment.base64), c => c.charCodeAt(0)),
      ], attachment.name, { type: attachment.type }))
    } else {
      headers["Content-Type"] = "application/json"
      body = JSON.stringify({ message: attributes })
    }
    const response = await fetch(`/rooms/${room}/messages`, { method: "POST", headers, body })
    const html = await response.text()
    return { status: response.status, id: html.match(/data-message-id="(\d+)"/)?.[1] }
  }, { room, attributes, attachment })
  assert.equal(result.status, 200, "message fixture create status")
  assert(result.id, "persisted message id in create stream")
  return result.id
}
