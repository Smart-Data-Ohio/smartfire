import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = [ "label" ]
  static values = {
    roomId: Number,
    roomName: String,
    joinLabel: String,
    activeLabel: String,
    toggle: Boolean
  }

  connect() {
    this.activeInRoom = false
    this.autojoinAttempted = false
    this.handleChange = this.#handleChange.bind(this)
    window.addEventListener("huddle:changed", this.handleChange)

    queueMicrotask(() => window.dispatchEvent(new CustomEvent("huddle:query")))
  }

  disconnect() {
    window.removeEventListener("huddle:changed", this.handleChange)
  }

  join() {
    // The stage join control carries a non-authoritative publishing hint for
    // the huddle panel; voice and DM controls carry none.
    const detail = { roomId: this.roomIdValue, roomName: this.roomNameValue }
    const hint = this.element.dataset.huddleCanPublishParam
    if (hint !== undefined) detail.canPublishHint = hint === "true"

    window.dispatchEvent(new CustomEvent("huddle:join", { detail }))
  }

  // Voice channels toggle in place: leaving goes through the huddle panel's
  // own Leave control, so the panel needs no new behavior.
  toggle() {
    if (this.toggleValue && this.activeInRoom) {
      const leaveButton = document.querySelector("#channel-huddle [data-action='huddle#leave']")
      if (leaveButton) leaveButton.click()
      else this.join()
    } else {
      this.join()
    }
  }

  #handleChange({ detail: { roomId, state } }) {
    const isCurrentRoom = roomId && Number(roomId) === this.roomIdValue
    const isActive = [ "connecting", "connected", "reconnecting" ].includes(state)

    this.activeInRoom = Boolean(isCurrentRoom && [ "connected", "reconnecting" ].includes(state))

    const label = isCurrentRoom && state === "connecting"
      ? "Joining…"
      : isCurrentRoom && isActive ? this.activeLabelValue : this.joinLabelValue

    this.element.setAttribute("aria-pressed", String(Boolean(isCurrentRoom && isActive)))
    this.element.setAttribute("aria-label", label)
    this.labelTarget.textContent = label

    this.#autojoinIfRequested()
  }

  // Profile-card "Start call" and multi-select "Start huddle" land here
  // with ?huddle=start: join once the panel answers the readiness query,
  // then drop the parameter so a revisit never rejoins on its own. A
  // first-time browser still stops at the device check; only the join
  // itself is automatic.
  #autojoinIfRequested() {
    if (this.autojoinAttempted) return

    const url = new URL(window.location.href)
    if (url.searchParams.get("huddle") !== "start") return
    this.autojoinAttempted = true

    url.searchParams.delete("huddle")
    window.history.replaceState(window.history.state, "", url)
    this.join()
  }
}
