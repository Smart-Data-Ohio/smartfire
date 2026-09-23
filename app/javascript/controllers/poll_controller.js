import { Controller } from "@hotwired/stimulus"

// Marks the viewer's own poll state on the viewer-agnostic card HTML:
// voted options get a marker, the matching inputs check, and the
// retract control appears. The card lives inside the shared message
// fragment cache, so none of this can render server-side.
export default class extends Controller {
  static targets = [ "option", "input", "submit", "retract" ]

  connect() {
    this.#syncState()
  }

  #syncState() {
    const currentUserId = document.querySelector("meta[name='current-user-id']")?.content
    let voted = false

    this.optionTargets.forEach(option => {
      const voterIds = (option.dataset.voterIds || "").split(",").map(value => value.trim()).filter(Boolean)
      const mine = Boolean(currentUserId) && voterIds.includes(String(currentUserId))
      option.classList.toggle("poll__option--voted", mine)
      if (mine) voted = true
    })

    this.inputTargets.forEach(input => {
      const option = this.optionTargets.find(option => option.dataset.pollOptionId === input.value)
      const voterIds = (option?.dataset.voterIds || "").split(",").map(value => value.trim()).filter(Boolean)
      input.checked = Boolean(currentUserId) && voterIds.includes(String(currentUserId))
    })

    if (this.hasRetractTarget) this.retractTarget.hidden = !voted
    if (this.hasSubmitTarget && voted) this.submitTarget.value = "Change vote"
  }
}
