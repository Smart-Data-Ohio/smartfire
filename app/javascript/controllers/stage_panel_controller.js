import { Controller } from "@hotwired/stimulus"

// The stage drawer: a Hosts/Speakers/Listeners roster that opens from the
// room header. It is a modal dialog, so it renders in the top layer above
// the member panel and the rest of the workspace. Its contents stay live
// over Turbo Streams; this controller only opens and closes the dialog
// itself. Escape closes natively through the `close` event below.
export default class extends Controller {
  static targets = [ "panel", "toggle", "handAnnouncement" ]

  connect() {
    this.huddleRoomId = null
    this.huddleState = "idle"
    this.handCount = this.#raisedHandCount()
    this.handleHuddleChange = this.#handleHuddleChange.bind(this)
    window.addEventListener("huddle:changed", this.handleHuddleChange)

    // Turbo replaces the panel body — and with it the Go live form — on
    // every stream start and end, so a swapped-in form is evaluated against
    // the last known huddle state as soon as it lands. The same observer
    // notices raised hands arriving over the roster stream.
    this.goLiveObserver = new MutationObserver(() => {
      this.#updateGoLiveControl()
      this.#handsMaybeChanged()
    })
    this.goLiveObserver.observe(this.element, { childList: true, subtree: true })
    this.#updateGoLiveControl()

    queueMicrotask(() => window.dispatchEvent(new CustomEvent("huddle:query")))
  }

  disconnect() {
    window.removeEventListener("huddle:changed", this.handleHuddleChange)
    this.goLiveObserver?.disconnect()
    this.goLiveObserver = null
  }

  toggle(event) {
    event?.preventDefault()

    if (this.panelTarget.open) {
      this.close()
    } else {
      this.panelTarget.showModal()
      this.toggleTarget.setAttribute("aria-expanded", "true")
    }
  }

  close(event) {
    event?.preventDefault()
    this.panelTarget.close()
  }

  // A click on the backdrop — the dialog itself rather than its surface —
  // closes, like the member panel's backdrop button.
  backdropClicked(event) {
    if (event.target === this.panelTarget) this.close()
  }

  wasClosed() {
    this.toggleTarget.setAttribute("aria-expanded", "false")
  }

  // The Go-live click hands the whole sequence to the huddle panel
  // synchronously — the capture must start inside this gesture, before any
  // POST round-trip, or Safari denies it. The huddle panel captures first,
  // posts the stream, then publishes; without the panel the form submits
  // on its own and the stream goes live without a share, as before.
  goLive(event) {
    const form = event.currentTarget.closest("form")
    const panel = document.getElementById("channel-huddle")
    const huddle = panel && this.application.getControllerForElementAndIdentifier(panel, "huddle")
    if (!form || !huddle) return

    const roomId = Number(form.dataset.roomId)
    if (!Number.isInteger(roomId) || roomId <= 0) return

    event.preventDefault()
    window.dispatchEvent(new CustomEvent("huddle:go-live", {
      detail: {
        roomId,
        quality: form.querySelector("select[name='quality']")?.value,
        streamUrl: form.action
      }
    }))
  }

  // Stop stream ends the server state through the form's own DELETE; once it
  // lands, the presenting browser stops sharing too. Ordering it after the
  // DELETE keeps a failed stop consistent: the share keeps going while live.
  streamStopSubmitted(event) {
    const form = event.target
    if (!(form instanceof HTMLFormElement) || event.detail?.success !== true) return

    const roomId = Number(form.dataset.roomId)
    if (!Number.isInteger(roomId) || roomId <= 0) return

    window.dispatchEvent(new CustomEvent("huddle:stream-stop", { detail: { roomId } }))
  }

  #handleHuddleChange({ detail }) {
    this.huddleRoomId = Number(detail?.roomId) || null
    this.huddleState = detail?.state || "idle"
    this.#updateGoLiveControl()
  }

  // A newly raised hand notifies viewers who can act on it — hosts and
  // administrators, who are exactly the viewers with action forms — with an
  // announcement and a short chime. Everyone watching gets the queue itself
  // through the roster stream either way.
  #handsMaybeChanged() {
    const count = this.#raisedHandCount()
    if (count === this.handCount) return

    const grew = count > this.handCount
    this.handCount = count
    window.dispatchEvent(new CustomEvent("stage:hands-changed", { detail: { count } }))

    if (!grew || !this.#viewerCanManage() || !this.hasHandAnnouncementTarget) return

    this.handAnnouncementTarget.textContent = count === 1
      ? "A listener raised their hand."
      : `${count} listeners have their hands raised.`
    this.#chime()
  }

  #raisedHandCount() {
    return this.element.querySelectorAll(".stage-panel__hand-badge").length
  }

  #viewerCanManage() {
    return this.element.querySelector(".stage-panel__member-actions") != null
  }

  #chime(retried = false) {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext
    if (!AudioContextClass) return

    try {
      this.handChimeContext ||= new AudioContextClass()
      const context = this.handChimeContext
      if (context.state === "suspended") {
        if (!retried) context.resume().then(() => this.#chime(true)).catch(() => {})
        return
      }

      const startedAt = context.currentTime
      for (const [ index, frequency ] of [ 660, 880 ].entries()) {
        const oscillator = context.createOscillator()
        oscillator.type = "sine"
        oscillator.frequency.value = frequency

        const gain = context.createGain()
        const at = startedAt + index * 0.12
        gain.gain.setValueAtTime(0, at)
        gain.gain.linearRampToValueAtTime(0.12, at + 0.02)
        gain.gain.linearRampToValueAtTime(0, at + 0.12)

        oscillator.connect(gain)
        gain.connect(context.destination)
        oscillator.start(at)
        oscillator.stop(at + 0.15)
      }
    } catch {
      // The badge and the announcement remain; the chime is a nicety.
    }
  }

  // Going live needs the call: the share starts on top of the huddle
  // connection, so the control stays disabled until the huddle panel reports
  // itself connected to this room. Only "connected" counts — a reconnecting
  // huddle would start a stream with nothing to publish over. The server's
  // grant check stays as the backstop.
  #updateGoLiveControl() {
    const form = this.element.querySelector(".stage-panel__stream-form")
    if (!form) return

    const roomId = Number(form.dataset.roomId)
    const connected = Number.isInteger(roomId) && roomId > 0 &&
      roomId === this.huddleRoomId && this.huddleState === "connected"

    const submit = form.querySelector("[type='submit']")
    if (submit) submit.disabled = !connected
  }
}
