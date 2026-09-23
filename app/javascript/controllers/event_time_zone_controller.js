import { Controller } from "@hotwired/stimulus"

// Fills the new-event form's hidden time-zone field (and its label) from
// the browser's time zone. A controller rather than an inline script so
// Turbo visits run it without the page's CSP nonce: after a
// Turbo-driven sign-out and sign-in the document keeps enforcing the old
// nonce, which would block an inline script carrying the new one.
export default class extends Controller {
  static targets = [ "field", "label" ]

  connect() {
    if (!this.hasFieldTarget) return

    try {
      const zone = Intl.DateTimeFormat().resolvedOptions().timeZone
      if (!zone) return

      this.fieldTarget.value = zone
      if (this.hasLabelTarget) this.labelTarget.textContent = zone
    } catch {
      // Keep the UTC fallback.
    }
  }
}
