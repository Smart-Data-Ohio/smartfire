import { Controller } from "@hotwired/stimulus"

// Marks the viewer's own poll state on the viewer-agnostic card HTML:
// voted options get a marker, the matching inputs check, and the
// retract control appears. The card lives inside the shared message
// fragment cache, so none of this can render server-side. Regular polls
// mark from the card's voter-id data; anonymous polls carry no voter
// ids (they would deanonymize every ballot), so the controller fetches
// the viewer's own ballot from the poll endpoint instead.
export default class extends Controller {
  static targets = [ "option", "input", "submit", "retract" ]
  static values = { anonymous: Boolean, resultsUrl: String }

  connect() {
    if (this.anonymousValue) {
      this.#syncAnonymous()
    } else {
      this.#syncState()
    }
  }

  async #syncAnonymous() {
    if (!this.hasResultsUrlValue) return

    try {
      const response = await fetch(this.resultsUrlValue, { headers: { Accept: "application/json" } })
      if (!response.ok) return
      const results = await response.json()
      const votedIds = new Set(
        (results.options || []).filter(option => option.voted).map(option => String(option.id))
      )
      this.#applyVotedIds(votedIds)
    } catch {
      // Own-vote markers are progressive enhancement; the card stays usable.
    }
  }

  #syncState() {
    const currentUserId = document.querySelector("meta[name='current-user-id']")?.content
    const votedIds = new Set()

    this.optionTargets.forEach(option => {
      const voterIds = (option.dataset.voterIds || "").split(",").map(value => value.trim()).filter(Boolean)
      if (currentUserId && voterIds.includes(String(currentUserId))) votedIds.add(option.dataset.pollOptionId)
    })

    this.#applyVotedIds(votedIds)
  }

  #applyVotedIds(votedIds) {
    const voted = votedIds.size > 0

    this.optionTargets.forEach(option => {
      option.classList.toggle("poll__option--voted", votedIds.has(option.dataset.pollOptionId))
    })

    this.inputTargets.forEach(input => {
      input.checked = votedIds.has(input.value)
    })

    if (this.hasRetractTarget) this.retractTarget.hidden = !voted
    if (this.hasSubmitTarget && voted) this.submitTarget.value = "Change vote"
  }
}
