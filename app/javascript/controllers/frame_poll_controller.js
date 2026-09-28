import { Controller } from "@hotwired/stimulus"

// Reloads its turbo-frame every few seconds. The server renders this
// controller only while the import run is active, so polling stops by
// itself once the run finishes.
export default class extends Controller {
  static values = { interval: { type: Number, default: 5000 } }

  connect() {
    this.timer = setInterval(() => this.element.reload(), this.intervalValue)
  }

  disconnect() {
    clearInterval(this.timer)
  }
}
