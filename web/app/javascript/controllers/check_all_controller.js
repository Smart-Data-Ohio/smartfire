import { Controller } from "@hotwired/stimulus"

// Bulk-toggles a list of checkboxes, e.g. the import plan's per-row
// selection. Two buttons call checkAll and checkNone.
export default class extends Controller {
  static targets = [ "checkbox" ]

  checkAll() {
    this.checkboxTargets.forEach((checkbox) => { checkbox.checked = true })
  }

  checkNone() {
    this.checkboxTargets.forEach((checkbox) => { checkbox.checked = false })
  }
}
