import { Controller } from "@hotwired/stimulus"

// Polls its turbo-frame's status endpoint until the refreshed content
// reports the run finished (data-frame-poll-finished). The server
// renders this controller only while the run is active, so an
// already-finished run never polls at all.
export default class extends Controller {
  static values = { interval: { type: Number, default: 5000 } }

  connect() {
    this.checkFinished = this.checkFinished.bind(this)
    this.element.addEventListener("turbo:frame-load", this.checkFinished)
    this.timer = setInterval(() => this.element.reload(), this.intervalValue)
  }

  disconnect() {
    this.stop()
  }

  checkFinished() {
    if (this.element.querySelector("[data-frame-poll-finished]")) {
      this.stop()
    }
  }

  stop() {
    clearInterval(this.timer)
    this.element.removeEventListener("turbo:frame-load", this.checkFinished)
  }
}
