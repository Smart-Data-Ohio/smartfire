import { Controller } from "@hotwired/stimulus"

// Per-message toolbar actions. This controller attaches no listeners of its
// own; it only handles button actions (delegated by Stimulus) and forwards
// them to the shared menu, the reply flow, or the emoji picker.
export default class extends Controller {
  react() {
    const message = this.#message
    if (!message) return
    window.dispatchEvent(new CustomEvent("message-actions:react", {
      detail: { message, content: "👍" },
    }))
  }

  reply() {
    const message = this.#message
    if (!message) return
    window.dispatchEvent(new CustomEvent("message:reply", { detail: { message } }))
  }

  thread() {
    const message = this.#message
    if (!message) return
    window.dispatchEvent(new CustomEvent("message-actions:thread", { detail: { message } }))
  }

  more(event) {
    const message = this.#message
    if (!message) return
    const rect = event.currentTarget.getBoundingClientRect()
    window.dispatchEvent(new CustomEvent("message-actions:open", {
      detail: { message, x: rect.left + rect.width / 2, y: rect.bottom },
    }))
  }

  pickEmoji(event) {
    const message = this.#message
    if (!message) return
    window.dispatchEvent(new CustomEvent("emoji-picker:open", {
      detail: { message, anchor: event.currentTarget },
    }))
  }

  get #message() {
    return this.element.closest(".message")
  }
}
