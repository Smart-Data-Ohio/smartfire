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
    // The minimum the official player needs, verified against the live
    // player in headless Chrome: scripts run the player, same-origin
    // keeps its storage and requests working, and popups let its links
    // open new tabs. Notably absent: allow-forms, allow-top-navigation
    // (the player can never navigate the Campfire page), and
    // allow-downloads. The referrer policy keeps the room URL out of
    // LinkedIn's logs: it sees only our origin.
    frame.setAttribute("sandbox", "allow-scripts allow-same-origin allow-popups")
    frame.referrerPolicy = "strict-origin-when-cross-origin"
    frame.classList.add("linkedin-post-card__player")
    this.element.replaceChildren(frame)
    frame.focus({ preventScroll: true })
  }
}
