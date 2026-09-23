import { Controller } from "@hotwired/stimulus"

// The room header's pins button and dialog. The list frame stays lazy
// until the dialog opens: Turbo's appearance observer never fires
// inside a closed dialog, so opening flips the frame to eager, which
// loads it immediately. Later opens reuse the loaded list, which pin
// and unpin broadcasts keep fresh.
export default class extends Controller {
  open() {
    const dialog = this.#dialog
    if (!dialog) return
    if (dialog.showModal) {
      if (!dialog.open) dialog.showModal()
    } else {
      dialog.setAttribute("open", "")
    }
    this.#loadList()
  }

  close() {
    const dialog = this.#dialog
    if (!dialog) return
    if (dialog.open && dialog.close) {
      dialog.close()
    } else {
      dialog.removeAttribute("open")
    }
  }

  #loadList() {
    const frame = this.element.querySelector("turbo-frame[loading='lazy']")
    frame?.setAttribute("loading", "eager")
  }

  get #dialog() {
    return this.element.querySelector("dialog")
  }
}
