import { Controller } from "@hotwired/stimulus"

// "Schedule send" for the composer: presets and a custom time, resolved
// in the author's browser time zone and submitted as UTC. Schedules the
// composer's current draft (including reply context) through the
// scheduled-messages endpoint, then clears the composer.
export default class extends Controller {
  static targets = [ "status", "custom" ]
  static values = { url: String, threadId: String, dialogId: String }

  open() {
    this.#clearStatus()
    this.#dialog().showModal()
  }

  close() {
    this.#dialog().close()
  }

  schedulePreset(event) {
    const sendAt = this.#presetTime(event.currentTarget.dataset.preset)
    if (sendAt) this.#schedule(sendAt)
  }

  // The dialog lives inside the message form: Enter in the datetime
  // input must schedule, never send the message.
  customEnter(event) {
    if (event.key === "Enter") {
      event.preventDefault()
      this.scheduleCustom()
    }
  }

  scheduleCustom() {
    const raw = this.customTarget.value
    if (!raw) {
      this.#showStatus("Pick a date and time first.")
      return
    }

    const sendAt = new Date(raw)
    if (Number.isNaN(sendAt.getTime())) {
      this.#showStatus("That date and time could not be understood.")
      return
    }

    this.#schedule(sendAt)
  }

  async #schedule(sendAt) {
    const form = this.element.closest("form")
    const textarea = form?.querySelector("textarea[name='message[markdown_source]'], textarea[name='message[body]']")
    const text = textarea?.value.trim() || ""

    if (!text) {
      this.#showStatus("Write a message first.")
      return
    }
    if (sendAt.getTime() <= Date.now()) {
      this.#showStatus("That time is in the past.")
      return
    }

    this.#setBusy(true)
    this.#clearStatus()

    try {
      const replyTo = form?.querySelector("input[name='message[reply_to_message_id]']")?.value || null
      const response = await fetch(this.urlValue, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
        },
        body: JSON.stringify({
          scheduled_message: {
            markdown_source: text,
            send_at: sendAt.toISOString(),
            thread_id: this.threadIdValue || null,
            reply_to_message_id: replyTo,
          },
        }),
      })

      if (!response.ok) {
        const payload = await response.json().catch(() => ({}))
        throw new Error(payload.error || this.#errorsSentence(payload.errors) || `Scheduling failed (${response.status})`)
      }

      this.close()
      this.#notifyComposer(`Scheduled for ${this.#formatLocal(sendAt)}.`)
    } catch (error) {
      this.#showStatus(error.message || "Couldn't schedule that message.")
    } finally {
      this.#setBusy(false)
    }
  }

  #presetTime(preset) {
    const now = new Date()

    if (preset === "hour") return new Date(now.getTime() + 60 * 60 * 1000)

    if (preset === "tomorrow") {
      const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 9, 0, 0)
      return tomorrow
    }

    if (preset === "monday") {
      // Next Monday 9 AM local time (at least a day out: on a Monday
      // this means a week ahead).
      const daysUntilMonday = ((8 - now.getDay()) % 7) || 7
      return new Date(now.getFullYear(), now.getMonth(), now.getDate() + daysUntilMonday, 9, 0, 0)
    }

    return null
  }

  #notifyComposer(notice) {
    const form = this.element.closest("form")
    const composer = form && this.application.getControllerForElementAndIdentifier(form, "composer")
    if (composer) composer.scheduled(notice)
  }

  #formatLocal(date) {
    return date.toLocaleString(undefined, {
      weekday: "short", month: "short", day: "numeric",
      hour: "numeric", minute: "2-digit",
    })
  }

  #errorsSentence(errors) {
    if (!errors || typeof errors !== "object") return null
    return Object.values(errors).flat().filter(Boolean).join(". ") || null
  }

  #dialog() {
    return document.getElementById(this.dialogIdValue)
  }

  #showStatus(message) {
    this.statusTarget.textContent = message
    this.statusTarget.hidden = false
  }

  #clearStatus() {
    this.statusTarget.textContent = ""
    this.statusTarget.hidden = true
  }

  #setBusy(busy) {
    this.element.querySelectorAll("button").forEach(button => { button.disabled = busy })
  }
}
