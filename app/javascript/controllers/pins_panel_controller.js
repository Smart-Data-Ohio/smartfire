import { Controller } from "@hotwired/stimulus"

// The room header's pins button and dialog. The list itself lazy-loads
// from the room pins endpoint and refreshes live over the room stream.
export default class extends Controller {
  open() {
    const dialog = this.#dialog
    if (!dialog) return
    if (dialog.showModal) {
      if (!dialog.open) dialog.showModal()
    } else {
      dialog.setAttribute("open", "")
    }
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

  get #dialog() {
    return this.element.querySelector("dialog")
  }
}
