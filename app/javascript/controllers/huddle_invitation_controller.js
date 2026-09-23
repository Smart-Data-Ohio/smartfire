import { Controller } from "@hotwired/stimulus"
import { cable } from "@hotwired/turbo-rails"

const ACTIVE_HUDDLE_STATES = [ "connecting", "connected", "reconnecting" ]

export default class extends Controller {
  static targets = [ "title", "description" ]
  static values = {
    activityItemId: Number,
    roomId: Number,
    roomName: String,
    roomPath: String,
    readPath: String,
    handledPath: String,
    // How long a ring may run before the banner gives up on its own. Matches
    // the server's missed-call resolution wait, so a banner-only ring (which
    // has no item to resolve) stops on the same schedule as an inbox one.
    ringTimeout: { type: Number, default: 45000 },
    // How long the "caller left" state stays up before dismissing itself.
    endedTimeout: { type: Number, default: 5000 }
  }

  async connect() {
    this.handleHuddleJoin = this.#huddleJoined.bind(this)
    this.handleHuddleChange = this.#huddleChanged.bind(this)
    window.addEventListener("huddle:join", this.handleHuddleJoin)
    window.addEventListener("huddle:changed", this.handleHuddleChange)

    // Learn the panel's current room so an invitation for a call the user
    // already joined never rings; the panel answers with huddle:changed.
    queueMicrotask(() => window.dispatchEvent(new CustomEvent("huddle:query")))

    const generation = this.generation = Symbol()
    try {
      const subscription = await cable.subscribeTo({ channel: "ActivityChannel" }, {
        received: (payload) => { if (this.generation === generation) this.#activityReceived(payload) }
      })
      if (this.generation === generation) {
        this.subscription = subscription
      } else {
        subscription.unsubscribe()
      }
    } catch {
      // The invitation also lands in the activity inbox, so a missed
      // subscription only loses the real-time banner, not the call itself.
    }
  }

  disconnect() {
    this.generation = undefined
    this.subscription?.unsubscribe()
    this.subscription = undefined
    clearTimeout(this.ringTimer)
    clearTimeout(this.endedTimer)
    this.#stopRinging()
    this.tabNotification?.close()
    this.tabNotification = null
    window.removeEventListener("huddle:join", this.handleHuddleJoin)
    window.removeEventListener("huddle:changed", this.handleHuddleChange)
  }

  async join(event) {
    event?.preventDefault()

    const roomId = this.roomIdValue
    const roomName = this.roomNameValue
    const handledPath = this.handledPathValue
    this.#hide()

    // Joining answers the call, so the invitation is handled immediately
    // rather than waiting for the missed-huddle resolution. A banner-only
    // ring carries an empty handled path, which reads as "no item": never
    // fetch, so Dismiss/Join can never issue a request to a null path.
    if (handledPath) await this.#patch(handledPath)
    if (!roomId) return

    if (window.location.pathname === this.roomPathValue) {
      this.#dispatchJoin(roomId, roomName)
    } else {
      // The listener is on the window, so it survives the navigation that
      // replaces this controller's element.
      window.addEventListener("turbo:load", () => this.#dispatchJoin(roomId, roomName), { once: true })
      Turbo.visit(this.roomPathValue)
    }
  }

  async dismiss(event) {
    event?.preventDefault()

    const readPath = this.readPathValue
    this.#hide()
    // A banner-only ring carries an empty read path ("no item"): hiding
    // the banner is the whole dismissal, so return without fetching.
    if (!readPath) return

    await this.#patch(readPath)
  }

  #activityReceived(payload) {
    const invitation = payload?.huddleInvitation
    if (!invitation) return

    if (invitation.eventType === "huddle_started" && invitation.state === "unread") {
      this.#show(invitation)
    } else if (invitation.eventType === "huddle_ended" && this.#matchesCurrentRing(invitation)) {
      this.#showCallEnded(invitation)
    } else if (invitation.activityItemId === this.activityItemIdValue) {
      this.#hide()
    }
  }

  #huddleJoined({ detail }) {
    if (detail && Number(detail.roomId) === this.roomIdValue) this.#hide()
  }

  #huddleChanged({ detail }) {
    if (!detail) return
    this.huddleRoomId = Number(detail.roomId)
    this.huddleState = detail.state
    if (Number(detail.roomId) !== this.roomIdValue) return
    if (ACTIVE_HUDDLE_STATES.includes(detail.state)) this.#hide()
  }

  async #patch(path) {
    try {
      await fetch(path, {
        method: "PATCH",
        headers: {
          Accept: "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content
        },
        credentials: "same-origin"
      })
    } catch {
      // The banner is already gone; the inbox still offers Mark read.
    }
  }

  #dispatchJoin(roomId, roomName) {
    window.dispatchEvent(new CustomEvent("huddle:join", {
      detail: { roomId, roomName }
    }))
  }

  #show(invitation) {
    // Never ring for a call the user is already in.
    if (Number(invitation.roomId) === this.huddleRoomId && ACTIVE_HUDDLE_STATES.includes(this.huddleState)) return

    // The banner-only payload sends 0 and empty strings for the missing
    // item; coerce here so a null would still read as "no item" instead
    // of the literal "null" path and a NaN id in these typed values.
    this.activityItemIdValue = invitation.activityItemId || 0
    this.roomIdValue = invitation.roomId
    this.roomNameValue = invitation.roomName
    this.roomPathValue = invitation.roomPath
    this.readPathValue = invitation.readPath || ""
    this.handledPathValue = invitation.handledPath || ""
    this.titleTarget.textContent = `${invitation.callerName} started a huddle`
    this.descriptionTarget.textContent = `Join the huddle in ${invitation.roomName}`
    this.element.hidden = false

    // A silent invitation still shows the banner but never makes a sound or
    // a system notification: the recipient is in do-not-disturb or quiet
    // hours. The banner itself is the ring.
    if (invitation.silent) {
      this.#stopRinging()
    } else {
      this.#startRinging()
      this.#notifyHiddenTab(invitation)
    }

    // A ring that nothing ever stops — a lost "call ended" event, a missed
    // inbox resolution — stops itself here.
    clearTimeout(this.ringTimer)
    clearTimeout(this.endedTimer)
    this.ringTimer = setTimeout(() => this.#hide(), this.ringTimeoutValue)
  }

  // The starter hung up (or was removed) while this ring was live: say so,
  // then get out of the way. An ended event for any other ring is ignored,
  // so a stale event can never cut off a newer call.
  #showCallEnded(invitation) {
    clearTimeout(this.ringTimer)
    this.#stopRinging()
    this.tabNotification?.close()
    this.tabNotification = null
    this.titleTarget.textContent = `${invitation.callerName} left the huddle`
    this.descriptionTarget.textContent = `Missed call in ${invitation.roomName}`
    this.element.hidden = false

    clearTimeout(this.endedTimer)
    this.endedTimer = setTimeout(() => this.#hide(), this.endedTimeoutValue)
  }

  // The current ring matches by item when the payload carries one, and by
  // room for banner-only rings (and their ended events), which have none.
  #matchesCurrentRing(invitation) {
    if (this.element.hidden || this.roomIdValue === 0) return false
    if (invitation.activityItemId) return invitation.activityItemId === this.activityItemIdValue
    return Number(invitation.roomId) === this.roomIdValue
  }

  // The audible ring: a two-tone pattern on a shared AudioContext, no
  // audio asset needed. Autoplay policies suspend audio before the first
  // gesture, so a suspended context waits for one instead of failing: the
  // banner still shows, and the ring starts on the next click or keypress.
  #startRinging() {
    this.shouldRing = true

    if (!this.ringUnlock) {
      this.ringUnlock = () => this.#ensureRinging()
      window.addEventListener("pointerdown", this.ringUnlock)
      window.addEventListener("keydown", this.ringUnlock)
    }

    this.#ensureRinging()
  }

  #ensureRinging(retried = false) {
    if (!this.shouldRing || this.ringing) return

    const AudioContextClass = window.AudioContext || window.webkitAudioContext
    if (!AudioContextClass) return

    try {
      this.ringContext ||= new AudioContextClass()
    } catch {
      return
    }

    if (this.ringContext.state === "suspended") {
      // One chained retry per gesture: the unlock listeners below keep
      // retrying on later gestures either way.
      if (!retried) this.ringContext.resume().then(() => this.#ensureRinging(true)).catch(() => {})
      return
    }

    const context = this.ringContext
    const gain = context.createGain()
    gain.gain.value = 0.12
    gain.connect(context.destination)

    const playTone = () => {
      if (!this.shouldRing) return

      const startedAt = context.currentTime
      for (const frequency of [ 440, 480 ]) {
        const oscillator = context.createOscillator()
        oscillator.type = "sine"
        oscillator.frequency.value = frequency

        const envelope = context.createGain()
        envelope.gain.setValueAtTime(0, startedAt)
        envelope.gain.linearRampToValueAtTime(1, startedAt + 0.05)
        envelope.gain.setValueAtTime(1, startedAt + 0.9)
        envelope.gain.linearRampToValueAtTime(0, startedAt + 1)

        oscillator.connect(envelope)
        envelope.connect(gain)
        oscillator.start(startedAt)
        oscillator.stop(startedAt + 1.05)
      }
    }

    playTone()
    this.ringInterval = setInterval(playTone, 3000)
    this.ringing = true
    this.ringGain = gain

    if (this.ringUnlock) {
      window.removeEventListener("pointerdown", this.ringUnlock)
      window.removeEventListener("keydown", this.ringUnlock)
      this.ringUnlock = null
    }
  }

  #stopRinging() {
    this.shouldRing = false
    this.ringing = false
    clearInterval(this.ringInterval)
    this.ringInterval = null

    try {
      this.ringGain?.disconnect()
    } catch {
      // Already gone; the interval stopping is what matters.
    }
    this.ringGain = null

    if (this.ringUnlock) {
      window.removeEventListener("pointerdown", this.ringUnlock)
      window.removeEventListener("keydown", this.ringUnlock)
      this.ringUnlock = null
    }
  }

  // A hidden tab cannot show the banner, so the call raises a system
  // notification instead — only with an already-granted permission, since
  // asking needs a gesture the ring does not have.
  #notifyHiddenTab(invitation) {
    if (document.visibilityState !== "hidden") return
    if (typeof Notification === "undefined" || Notification.permission !== "granted") return

    try {
      this.tabNotification?.close()
      this.tabNotification = new Notification(`${invitation.callerName} started a huddle`, {
        body: `Join the huddle in ${invitation.roomName}`,
        tag: `huddle-invitation-${invitation.roomId}`
      })
      this.tabNotification.onclick = () => {
        window.focus()
        this.tabNotification?.close()
      }
    } catch {
      // The banner still shows when the tab becomes visible.
    }
  }

  #hide() {
    clearTimeout(this.ringTimer)
    clearTimeout(this.endedTimer)
    this.#stopRinging()
    this.tabNotification?.close()
    this.tabNotification = null
    this.activityItemIdValue = 0
    this.roomIdValue = 0
    this.element.hidden = true
  }
}
