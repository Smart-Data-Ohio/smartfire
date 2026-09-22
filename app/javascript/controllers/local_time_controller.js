import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = [ "time", "date", "datetime", "title" ]

  initialize() {
    this.timeFormatter = new Intl.DateTimeFormat(undefined, { timeStyle: "short" })
    this.dateFormatter = new Intl.DateTimeFormat(undefined, { dateStyle: "long" })
    this.dateTimeFormatter = new Intl.DateTimeFormat(undefined, { timeStyle: "short", dateStyle: "short" })
  }

  timeTargetConnected(target) {
    this.#formatTime(this.timeFormatter, target)
  }

  dateTargetConnected(target) {
    this.#formatTime(this.dateFormatter, target)
  }

  datetimeTargetConnected(target) {
    this.#formatTime(this.dateTimeFormatter, target)
  }

  // Localizes only the tooltip, leaving the visible label (e.g. "(edited)")
  // untouched. The server-rendered title stays as the no-JS fallback.
  titleTargetConnected(target) {
    const dt = new Date(target.getAttribute("datetime"))
    target.title = `Edited ${this.dateTimeFormatter.format(dt)}`
  }

  #formatTime(formatter, target) {
    const dt = new Date(target.getAttribute("datetime"))
    target.textContent = formatter.format(dt)
    target.title = this.dateTimeFormatter.format(dt)
  }
}
