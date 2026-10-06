import { Controller } from "@hotwired/stimulus"

// Client-side filter for the new-DM picker. Typing narrows the rendered
// rows to people whose name contains the query (case-insensitive, accents
// folded, matched anywhere); an empty query shows everyone, and a query
// with no matches shows the empty state. No server round-trip.
//
// Selection lives in the sibling multi-select controller, so filtering
// never clears it: checked rows stay checked while hidden, and the
// Message/Start huddle bar count stays correct.
//
// Keyboard: Enter with exactly one visible row and nothing selected
// selects that row; otherwise Enter does nothing and starting the DM
// always happens from the multi-select bar. Esc leaves the picker.
export default class extends Controller {
  static targets = [ "input", "row", "empty", "cancel" ]

  connect() {
    this.inputTarget.focus()
  }

  filter() {
    const query = normalize(this.inputTarget.value.trim())
    let visible = 0

    this.rowTargets.forEach((row) => {
      const match = query === "" || normalize(row.dataset.name).includes(query)
      row.hidden = !match
      if (match) visible += 1
    })

    this.emptyTarget.hidden = visible > 0
  }

  selectSingle(event) {
    const visible = this.rowTargets.filter((row) => !row.hidden)
    if (visible.length !== 1) return
    if (this.element.querySelector("[data-multi-select-target='checkbox']:checked")) return

    event.preventDefault()
    visible[0].querySelector("[data-multi-select-target='checkbox']")?.click()
  }

  // The whole row is the tap target: clicks outside the interactive
  // controls (checkbox, avatar link, profile-card name button) toggle the
  // row's checkbox through a real click, so multi-select stays in charge
  // of tracking, ranges, and the bar.
  toggleRow(event) {
    if (event.target.closest("a, button, input, select, textarea")) return
    event.currentTarget.querySelector("[data-multi-select-target='checkbox']")?.click()
  }

  cancel() {
    this.cancelTarget?.click()
  }
}

function normalize(value) {
  return value.toLowerCase().normalize("NFD").replace(/\p{Diacritic}/gu, "")
}
