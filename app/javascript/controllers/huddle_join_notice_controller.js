import { Controller } from "@hotwired/stimulus"
import { cable } from "@hotwired/turbo-rails"

const ACTIVE_HUDDLE_STATES = [ "connecting", "connected", "reconnecting" ]
const PARTICIPANTS_URL_ROOM_ID = /\/rooms\/(\d+)\/huddle\/participants/
const MAX_LEAVE_TOASTS = 3
const LEAVE_TOAST_TIMEOUT = 4000

export default class extends Controller {
  static targets = [ "sound", "toasts" ]
  static values = {
    toastTimeout: { type: Number, default: 6000 },
    batchWindow: { type: Number, default: 4000 },
    leaveDelay: { type: Number, default: 5000 }
  }

  async connect() {
    // The host is permanent, so this state survives Turbo navigations:
    // a banner stored here re-renders into the next page's slots.
    this.notices ||= new Map()

    this.handleHuddleJoin = ({ detail }) => this.#dismissNotice(Number(detail?.roomId))
    this.handleHuddleChange = this.#huddleChanged.bind(this)
    this.handleNoticeJoin = this.#noticeJoin.bind(this)
    this.handleNoticeDismiss = ({ detail }) => this.#dismissNotice(Number(detail?.roomId))
    this.handleParticipantsUpdated = this.#participantsUpdated.bind(this)
    this.handleParticipantsRemoved = this.#participantsRemoved.bind(this)
    this.handleNavigation = () => {
      this.#renderRoomBanner()
      this.#syncSidebarPills()
    }
    window.addEventListener("huddle:join", this.handleHuddleJoin)
    window.addEventListener("huddle-join-notice:join", this.handleNoticeJoin)
    window.addEventListener("huddle-join-notice:dismiss", this.handleNoticeDismiss)
    window.addEventListener("huddle:changed", this.handleHuddleChange)
    window.addEventListener("huddle-participants:updated", this.handleParticipantsUpdated)
    window.addEventListener("huddle-participants:removed", this.handleParticipantsRemoved)
    document.addEventListener("turbo:load", this.handleNavigation)
    document.addEventListener("turbo:frame-load", this.handleNavigation)

    // Learn the panel's current room so a notice for a call the viewer is
    // already in toasts instead of bannering; the panel answers with
    // huddle:changed. Notices arriving before the answer trust the
    // server's inCall flag instead.
    queueMicrotask(() => window.dispatchEvent(new CustomEvent("huddle:query")))

    const generation = this.generation = Symbol()
    try {
      const subscription = await cable.subscribeTo({ channel: "HuddleNoticeChannel" }, {
        received: (payload) => { if (this.generation === generation) this.#noticeReceived(payload) }
      })
      if (this.generation === generation) {
        this.subscription = subscription
      } else {
        subscription.unsubscribe()
      }
    } catch {
      // Join notices are ephemeral. A missed subscription only loses the
      // toast and banner; the presence stacks still show who is in.
    }

    this.#renderRoomBanner()
    this.#syncSidebarPills()
  }

  disconnect() {
    this.generation = undefined
    this.subscription?.unsubscribe()
    this.subscription = undefined
    window.removeEventListener("huddle:join", this.handleHuddleJoin)
    window.removeEventListener("huddle-join-notice:join", this.handleNoticeJoin)
    window.removeEventListener("huddle-join-notice:dismiss", this.handleNoticeDismiss)
    window.removeEventListener("huddle:changed", this.handleHuddleChange)
    window.removeEventListener("huddle-participants:updated", this.handleParticipantsUpdated)
    window.removeEventListener("huddle-participants:removed", this.handleParticipantsRemoved)
    document.removeEventListener("turbo:load", this.handleNavigation)
    document.removeEventListener("turbo:frame-load", this.handleNavigation)
    clearTimeout(this.joinToastTimer)
    for (const timer of this.leaveToastTimers || []) clearTimeout(timer)
    this.leaveToastTimers = []
    this.joinBatch = null
    this.joinToastNode = null
    for (const timer of this.pendingLeaves?.values() || []) clearTimeout(timer)
    this.pendingLeaves = new Map()
    // Toasts are transient; banners re-render from the stored notices.
    if (this.hasToastsTarget) this.toastsTarget.replaceChildren()
  }

  // Join buttons rendered into banners and sidebar pills dispatch the
  // same huddle:join event the invitation banner uses, navigating to
  // the DM first when the viewer is elsewhere.
  #noticeJoin({ detail }) {
    const roomId = Number(detail?.roomId)
    const roomName = detail?.roomName || "Huddle"
    const roomPath = detail?.roomPath
    if (!roomId) return
    this.#dismissNotice(roomId)

    if (window.location.pathname === roomPath) {
      this.#dispatchJoin(roomId, roomName)
    } else if (roomPath) {
      window.addEventListener("turbo:load", () => this.#dispatchJoin(roomId, roomName), { once: true })
      Turbo.visit(roomPath)
    }
  }

  #noticeReceived(payload) {
    const notice = payload?.huddleJoinNotice
    if (!notice) return

    const roomId = Number(notice.roomId)
    if (!roomId) return

    if (notice.eventType === "huddle_joined") {
      this.#joined(notice, roomId)
    } else if (notice.eventType === "huddle_left") {
      if (this.#inCall(roomId)) this.#scheduleLeaveToast(roomId, notice.joinerId, notice.joinerName)
      this.#removeBannerJoiner(roomId, notice.joinerId, notice.joinerName)
    } else if (notice.eventType === "huddle_ended") {
      this.#dismissNotice(roomId)
    }
  }

  #joined(notice, roomId) {
    // A join inside the leave delay answers a pending leave from the same
    // person and stays silent itself: a server-mute revoke plus its
    // automatic rejoin reads as one quiet cycle instead of "left" + "joined".
    if (this.#cancelPendingLeave(roomId, notice.joinerId)) return

    if (this.#inCall(roomId)) {
      this.#toastJoin(roomId, notice.joinerName)
    } else if (this.huddleKnown) {
      this.#addBannerNotice(notice, roomId)
    } else if (notice.inCall) {
      this.#toastJoin(roomId, notice.joinerName)
    } else {
      this.#addBannerNotice(notice, roomId)
    }
  }

  #huddleChanged({ detail }) {
    if (!detail) return
    this.huddleKnown = true
    this.huddleRoomId = Number(detail.roomId)
    this.huddleState = detail.state
    // Joining answers the banner, so it clears as soon as the call goes
    // active — the same moment the invitation banner hides.
    if (ACTIVE_HUDDLE_STATES.includes(detail.state)) this.#dismissNotice(Number(detail.roomId))
  }

  // The presence poll is the backstop for quiet endings: a call that
  // expires without a leave report still empties the room, and the
  // banner clears with it. A partial roster prunes joiners who are no
  // longer in the call, so a missed leave notice still drops from the
  // banner on the next refresh. A 404 removal clears too — the
  // membership is gone, so there is nothing left to join.
  #participantsUpdated({ detail }) {
    const roomId = roomIdFromParticipantsUrl(detail?.url)
    const participants = detail?.participants
    if (!roomId || !Array.isArray(participants)) return

    if (participants.length === 0) {
      this.#dismissNotice(roomId)
    } else {
      this.#reconcileBannerRoster(roomId, participants)
    }
  }

  #participantsRemoved({ detail }) {
    this.#dismissNotice(roomIdFromParticipantsUrl(detail?.url))
  }

  #addBannerNotice(notice, roomId) {
    const stored = this.notices.get(roomId) || {
      roomName: notice.roomName, roomPath: notice.roomPath, joiners: []
    }
    stored.roomName = notice.roomName || stored.roomName
    stored.roomPath = notice.roomPath || stored.roomPath
    const joinerId = Number(notice.joinerId) || null
    if (notice.joinerName && !stored.joiners.some((joiner) => joinerMatches(joiner, joinerId, notice.joinerName))) {
      stored.joiners.push({ id: joinerId, name: notice.joinerName })
    }
    this.notices.set(roomId, stored)
    this.#renderRoomBanner()
    this.#syncSidebarPills()
  }

  // A leave drops its joiner from the banner roster while the others
  // remain; the last name out hides the banner and its sidebar pill.
  #removeBannerJoiner(roomId, joinerId, joinerName) {
    const stored = this.notices.get(roomId)
    if (!stored) return
    const id = Number(joinerId) || null

    stored.joiners = stored.joiners.filter((joiner) => !joinerMatches(joiner, id, joinerName))
    if (stored.joiners.length === 0) {
      this.#dismissNotice(roomId)
    } else {
      this.#renderRoomBanner()
      this.#syncSidebarPills()
    }
  }

  // Presence is authoritative: keep only roster names still listed in
  // the call, and hide the banner when none remain.
  #reconcileBannerRoster(roomId, participants) {
    const stored = this.notices.get(roomId)
    if (!stored) return
    const presentIds = new Set(participants.map((participant) => Number(participant.id)).filter(Boolean))
    const presentNames = new Set(participants.map((participant) => participant.name))

    stored.joiners = stored.joiners.filter((joiner) =>
      joiner.id ? presentIds.has(joiner.id) : presentNames.has(joiner.name))
    if (stored.joiners.length === 0) {
      this.#dismissNotice(roomId)
    } else {
      this.#renderRoomBanner()
      this.#syncSidebarPills()
    }
  }

  #dismissNotice(roomId) {
    if (!roomId || !this.notices.delete(roomId)) return
    this.#renderRoomBanner()
    this.#syncSidebarPills()
  }

  #inCall(roomId) {
    return this.huddleKnown === true &&
      this.huddleRoomId === roomId &&
      ACTIVE_HUDDLE_STATES.includes(this.huddleState)
  }

  #renderRoomBanner() {
    const slot = document.getElementById("huddle-join-banner-slot")
    if (!slot) return

    const roomId = Number(document.querySelector('meta[name="current-room-id"]')?.content)
    const notice = roomId ? this.notices.get(roomId) : undefined
    slot.replaceChildren()
    if (!notice || this.#inCall(roomId)) return
    slot.append(this.#buildBanner(roomId, notice))
  }

  // Sidebar rows render from a shared fragment cache, so pills are never
  // rendered server-side: they are injected here, per viewer, and
  // re-injected after every sidebar reload from the stored notices. Rows
  // match on their room id rather than their DOM id, which prefixes the
  // room's STI name and differs per room kind.
  #syncSidebarPills() {
    document.querySelectorAll(".huddle-join-pill").forEach((pill) => pill.remove())

    for (const [ roomId, notice ] of this.notices) {
      if (this.#inCall(roomId)) continue
      const rows = document.querySelectorAll(`#user_sidebar .sidebar-item[data-room-id="${roomId}"]`)
      for (const row of rows) row.append(this.#buildPill(roomId, notice))
    }
  }

  #buildBanner(roomId, notice) {
    const banner = document.createElement("div")
    banner.className = "huddle-join-banner"
    banner.setAttribute("role", "status")

    const text = document.createElement("span")
    text.className = "huddle-join-banner__text overflow-ellipsis"
    text.textContent = bannerText(joinerNames(notice.joiners))
    banner.append(text)

    banner.append(this.#buildJoinButton(roomId, notice))

    const dismiss = document.createElement("button")
    dismiss.type = "button"
    dismiss.className = "btn btn--plain"
    dismiss.setAttribute("aria-label", "Dismiss")
    dismiss.textContent = "×"
    dismiss.addEventListener("click", (event) => {
      event.preventDefault()
      window.dispatchEvent(new CustomEvent("huddle-join-notice:dismiss", { detail: { roomId } }))
    })
    banner.append(dismiss)

    return banner
  }

  #buildPill(roomId, notice) {
    const pill = document.createElement("div")
    pill.className = "huddle-join-pill"

    const text = document.createElement("span")
    text.className = "huddle-join-pill__text overflow-ellipsis"
    text.textContent = bannerText(joinerNames(notice.joiners))
    pill.append(text)

    pill.append(this.#buildJoinButton(roomId, notice))

    return pill
  }

  // Banners and pills render outside this controller's element, where
  // Stimulus actions cannot reach, so their buttons talk back over
  // window events instead.
  #buildJoinButton(roomId, notice) {
    const join = document.createElement("button")
    join.type = "button"
    join.className = "btn btn--primary"
    join.textContent = "Join"
    join.addEventListener("click", (event) => {
      event.preventDefault()
      window.dispatchEvent(new CustomEvent("huddle-join-notice:join", {
        detail: { roomId, roomName: notice.roomName || "Huddle", roomPath: notice.roomPath || "" }
      }))
    })
    return join
  }

  // Rapid joins into one room batch into a single toast ("Chris and Dean
  // joined") with one sound; a join anywhere else starts a fresh toast.
  #toastJoin(roomId, name) {
    if (!name) return

    const batch = this.joinBatch
    if (batch && batch.roomId === roomId && Date.now() - batch.startedAt < this.batchWindowValue) {
      if (!batch.names.includes(name)) batch.names.push(name)
    } else {
      this.joinBatch = { roomId, names: [ name ], startedAt: Date.now() }
      this.#playJoinSound()
    }

    this.joinToastNode ||= document.createElement("div")
    this.joinToastNode.className = "huddle-join-toast shadow"
    this.joinToastNode.textContent = `${sentence(this.joinBatch.names)} joined`
    this.toastsTarget.append(this.joinToastNode)

    clearTimeout(this.joinToastTimer)
    this.joinToastTimer = setTimeout(() => this.#hideJoinToast(), this.toastTimeoutValue)
  }

  #hideJoinToast() {
    clearTimeout(this.joinToastTimer)
    this.joinBatch = null
    this.joinToastNode?.remove()
    this.joinToastNode = null
  }

  // A leave waits out the mute-cycle delay before toasting: a join from
  // the same person inside the window cancels it (see #joined), so a
  // server mute never flashes "left" + "joined". A leave that stands
  // alone toasts once the delay passes, if the viewer is still in.
  #scheduleLeaveToast(roomId, joinerId, name) {
    if (!name) return
    this.#cancelPendingLeave(roomId, joinerId)

    const key = leaveKey(roomId, joinerId)
    const timer = setTimeout(() => {
      this.pendingLeaves?.delete(key)
      if (this.#inCall(roomId)) this.#showLeaveToast(name)
    }, this.leaveDelayValue)
    this.pendingLeaves ||= new Map()
    this.pendingLeaves.set(key, timer)
  }

  #cancelPendingLeave(roomId, joinerId) {
    const timer = this.pendingLeaves?.get(leaveKey(roomId, joinerId))
    if (timer === undefined) return false

    clearTimeout(timer)
    this.pendingLeaves.delete(leaveKey(roomId, joinerId))
    return true
  }

  #showLeaveToast(name) {
    const toast = document.createElement("div")
    toast.className = "huddle-join-toast huddle-join-toast--leave shadow"
    toast.textContent = `${name} left`
    this.toastsTarget.append(toast)

    while (this.toastsTarget.querySelectorAll(".huddle-join-toast--leave").length > MAX_LEAVE_TOASTS) {
      this.toastsTarget.querySelector(".huddle-join-toast--leave")?.remove()
    }

    const timer = setTimeout(() => toast.remove(), LEAVE_TOAST_TIMEOUT)
    this.leaveToastTimers ||= []
    this.leaveToastTimers.push(timer)
  }

  // The nested sound controller gates on the app's DND, quiet-hours,
  // meeting, and out-of-office settings, so the join blip follows the
  // same rules as every other chat sound with no gates of its own.
  #playJoinSound() {
    if (!this.hasSoundTarget) return
    this.application.getControllerForElementAndIdentifier(this.soundTarget, "sound")?.play()
  }

  #dispatchJoin(roomId, roomName) {
    window.dispatchEvent(new CustomEvent("huddle:join", {
      detail: { roomId, roomName }
    }))
  }
}

function leaveKey(roomId, joinerId) {
  return `${roomId}:${joinerId}`
}

function roomIdFromParticipantsUrl(url) {
  const match = String(url || "").match(PARTICIPANTS_URL_ROOM_ID)
  return match ? Number(match[1]) : 0
}

function joinerNames(joiners) {
  return joiners.map((joiner) => joiner.name)
}

function joinerMatches(joiner, id, name) {
  if (id) return joiner.id === id
  return joiner.name === name
}

function bannerText(joiners) {
  if (joiners.length > 1) return `${sentence(joiners)} are in your huddle`
  if (joiners.length === 1) return `${joiners[0]} is in your huddle`
  return "Someone is in your huddle"
}

function sentence(names) {
  if (names.length > 2) return `${names.slice(0, -1).join(", ")}, and ${names[names.length - 1]}`
  return names.join(" and ")
}
