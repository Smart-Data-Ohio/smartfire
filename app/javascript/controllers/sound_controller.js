import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { "url": String }

  play() {
    if (document.querySelector("meta[name='notification-sounds'][content='muted']")) return

    const sound = new Audio(this.urlValue)
    sound.play()
  }
}
