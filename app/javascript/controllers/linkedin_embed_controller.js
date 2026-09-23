import { Controller } from "@hotwired/stimulus"

// Swaps a LinkedIn "Show embedded post" button for the official embed
// player iframe, on click only: the iframe (and its tracking) never loads
// until the reader asks for it.
export default class extends Controller {
  static targets = [ "button" ]
  static values = { src: String }

  show(event) {
    event.preventDefault()
    if (!this.srcValue || this.element.querySelector("iframe")) return

    const frame = document.createElement("iframe")
    frame.src = this.srcValue
    frame.title = "Embedded LinkedIn post"
    frame.loading = "lazy"
    frame.allowFullscreen = true
    frame.classList.add("linkedin-post-card__player")
    this.element.replaceChildren(frame)
    frame.focus({ preventScroll: true })
  }
}
