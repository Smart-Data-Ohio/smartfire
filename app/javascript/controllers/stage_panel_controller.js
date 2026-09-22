import { Controller } from "@hotwired/stimulus"

// The stage drawer: a Hosts/Speakers/Listeners roster that opens from the
// room header. It is a modal dialog, so it renders in the top layer above
// the member panel and the rest of the workspace. Its contents stay live
// over Turbo Streams; this controller only opens and closes the dialog
// itself. Escape closes natively through the `close` event below.
export default class extends Controller {
  static targets = [ "panel", "toggle" ]

  connect() {
    this.huddleRoomId = null
    this.huddleState = "idle"
    this.handleHuddleChange = this.#handleHuddleChange.bind(this)
    window.addEventListener("huddle:changed", this.handleHuddleChange)

    // Turbo replaces the panel body — and with it the Go live form — on
    // every stream start and end, so a swapped-in form is evaluated against
    // the last known huddle state as soon as it lands.
    this.goLiveObserver = new MutationObserver(() => this.#updateGoLiveControl())
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
