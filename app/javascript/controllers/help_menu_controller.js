import { Controller } from "@hotwired/stimulus"

// Help menu in the top bar: keyboard shortcuts and the tour restart. The
// shortcuts item only shows once the navigation branch's #keyboard-shortcuts
// dialog exists in the page; until then the menu holds the tour restart
// alone.
export default class extends Controller {
  static targets = [ "shortcuts" ]

  connect() {
    if (!document.getElementById("keyboard-shortcuts")) {
      this.shortcutsTarget.hidden = true
    }
  }

  restartTour() {
    this.element.open = false
    window.dispatchEvent(new CustomEvent("tour:start"))
  }

  openShortcuts() {
    this.element.open = false
    document.getElementById("keyboard-shortcuts")?.showModal()
  }
}
