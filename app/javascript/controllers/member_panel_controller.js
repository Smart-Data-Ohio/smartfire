import { Controller } from "@hotwired/stimulus"

const DESKTOP_QUERY = "(min-width: 80rem)"
const REFRESH_INTERVAL = 12 * 1000
const PRESENCE_LABELS = {
  online: "Online",
  idle: "Idle",
  dnd: "Do not disturb",
  agent: "Agent",
  offline: "Offline"
}
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "summary",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])"
].join(",")

export default class extends Controller {
  static targets = [ "panel", "backdrop", "close", "content", "status", "summary", "toggle", "toggleCount", "rowMenu", "rowMenuStar", "rowMenuStatus" ]

  connect() {
    this.desktopQuery = window.matchMedia(DESKTOP_QUERY)
    this.handleViewportChange = this.#handleViewportChange.bind(this)
    this.handleVisibilityChange = this.#handleVisibilityChange.bind(this)
    this.handleRowMenuPointerDown = this.#handleRowMenuPointerDown.bind(this)
    this.handleRowMenuWindowKeydown = this.#handleRowMenuWindowKeydown.bind(this)
    this.desktopQuery.addEventListener("change", this.handleViewportChange)
    document.addEventListener("visibilitychange", this.handleVisibilityChange)
    document.addEventListener("pointerdown", this.handleRowMenuPointerDown)
    window.addEventListener("keydown", this.handleRowMenuWindowKeydown)
    this.desktopClosed = false
    this.isOpen = false

    if (this.hasPanelTarget && this.desktopQuery.matches) {
      this.#open({ focusPanel: false, announce: false })
    } else {
      this.#syncAccessibility()
    }
  }

  disconnect() {
    this.desktopQuery?.removeEventListener("change", this.handleViewportChange)
    document.removeEventListener("visibilitychange", this.handleVisibilityChange)
    document.removeEventListener("pointerdown", this.handleRowMenuPointerDown)
    window.removeEventListener("keydown", this.handleRowMenuWindowKeydown)
    this.#stopRefreshing()
    this.#abortRequest()
    this.#closeRowMenu({ restoreFocus: false })
    this.element.classList.remove("member-panel-open")
  }

  toggle(event) {
    event.preventDefault()

    if (this.isOpen) {
      if (this.desktopQuery.matches) this.desktopClosed = true
      this.#close({ restoreFocus: false })
    } else {
      if (this.desktopQuery.matches) this.desktopClosed = false
      this.#open({ focusPanel: !this.desktopQuery.matches })
    }
  }

  close(event) {
    if (!this.isOpen) return
    // An open profile card owns Esc: it closes the card and returns focus
    // to its trigger inside this panel. This runs before profile-card#close
    // (see the action order in the layout), so the card is still open here.
    if (event?.type === "keydown" && this.#profileCardOpen()) return
    // Selection mode owns Esc next: leaving it keeps the panel open, on
    // desktop (which never closes on Esc) and in the mobile drawer.
    if (event?.type === "keydown" && this.#exitSelectionMode()) {
      event.preventDefault()
      return
    }
    if (event?.type === "keydown" && this.desktopQuery.matches) return

    event?.preventDefault()
    this.#close({ restoreFocus: true })
  }

  prepareForCache() {
    this.#stopRefreshing()
    this.#abortRequest()
    this.#close({ restoreFocus: false })
    this.#clearMembers("Open the member list to load members.")
  }

  roomChanged() {
    if (!this.hasPanelTarget) return

    const nextUrl = this.panelTarget.dataset.membersUrl
    if (nextUrl === this.loadedUrl && this.contentTarget.childElementCount > 0) return

    this.#abortRequest()
    this.#closeRowMenu({ restoreFocus: false })
    this.loadedUrl = null
    this.#clearMembers("Loading members…")
    if (this.isOpen) this.#fetchMembers()
  }

  refreshPresence() {
    if (!this.isOpen) return

    this.#abortRequest()
    this.#fetchMembers()
  }

  trapFocus(event) {
    if (event.key !== "Tab") return
    if (!this.isOpen || this.desktopQuery.matches || !this.hasPanelTarget) return
    // The profile card traps focus itself while open; yielding avoids the
    // two traps fighting over every Tab.
    if (this.#profileCardOpen()) return

    const focusable = this.#focusableElements()
    if (focusable.length === 0) return

    const first = focusable[0]
    const last = focusable[focusable.length - 1]

    if (!this.panelTarget.contains(document.activeElement)) {
      event.preventDefault()
      const focusTarget = event.shiftKey ? last : first
      focusTarget.focus()
      return
    }

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }

  // Right-click on a member row opens the row menu (Star/Unstar). A
  // touch long-press ends in a synthetic contextmenu too, but that one
  // already selected the row: multi-select#suppressMenu ran first and
  // prevented it, so the menu stays shut there.
  openMenu(event) {
    if (event.defaultPrevented || !this.hasRowMenuTarget) return

    const row = event.target.closest?.("[data-member-id]")
    if (!row || !this.element.contains(row) || this.#isSelfRow(row)) return

    event.preventDefault()
    this.#openRowMenu(row, { x: event.clientX, y: event.clientY })
  }

  // Keyboard path to the same menu: the menu key or Shift+F10 on any
  // focused control inside a row, like the sidebar room menu.
  rowKeydown(event) {
    const menuKey = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)
    if (!menuKey || !this.hasRowMenuTarget) return

    const row = event.target.closest?.("[data-member-id]")
    if (!row || !this.element.contains(row) || this.#isSelfRow(row)) return

    event.preventDefault()
    const rect = row.getBoundingClientRect()
    this.#openRowMenu(row, { x: rect.left + Math.min(rect.width / 2, 240), y: rect.bottom })
  }

  async toggleRowMenuStar() {
    const userId = this.menuUserId
    if (!userId || !this.hasRowMenuTarget) return

    const starred = this.#menuRow()?.dataset.starred === "true"
    const url = this.#starUrl(userId)
    if (!url) return

    this.rowMenuStarTarget.disabled = true
    try {
      const response = await fetch(url, {
        method: starred ? "DELETE" : "POST",
        headers: {
          Accept: "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || ""
        }
      }).catch(() => null)

      if (!response?.ok) {
        this.rowMenuStatusTarget.textContent = starred ? "Couldn’t unstar" : "Couldn’t star"
        return
      }

      const payload = await response.json().catch(() => ({}))
      const nowStarred = payload.starred ?? !starred
      // The refetch below replaces the row element, so the menu keeps
      // only the user id: look the row up fresh every time.
      this.#menuRow()?.setAttribute("data-starred", String(nowStarred))
      this.rowMenuStarTarget.textContent = nowStarred ? "★ Unstar" : "☆ Star"
      this.rowMenuStatusTarget.textContent = ""
      this.refreshPresence()
    } finally {
      this.rowMenuStarTarget.disabled = false
    }
  }

  rowMenuKeydown(event) {
    if (!this.hasRowMenuTarget || this.rowMenuTarget.hidden) return

    if (event.key === "Escape" || event.key === "Tab") {
      // Stop here: the window-level Esc handlers would otherwise close
      // the whole panel (and the card trap) behind the menu.
      event.preventDefault()
      event.stopPropagation()
      this.#closeRowMenu({ restoreFocus: true })
    }
  }

  #open({ focusPanel = false, announce = true } = {}) {
    if (!this.hasPanelTarget) return

    if (announce) window.dispatchEvent(new CustomEvent("member-panel:opening"))
    this.previouslyFocusedElement = document.activeElement
    this.isOpen = true
    this.element.classList.add("member-panel-open")
    this.#syncAccessibility()
    this.#fetchMembers()
    this.#startRefreshing()

    if (focusPanel) requestAnimationFrame(() => this.closeTarget?.focus())
  }

  #close({ restoreFocus }) {
    this.isOpen = false
    this.element.classList.remove("member-panel-open")
    this.#stopRefreshing()
    this.#abortRequest()
    this.#closeRowMenu({ restoreFocus: false })
    this.#syncAccessibility()

    if (restoreFocus) this.#restoreFocus()
  }

  #syncAccessibility() {
    const open = this.hasPanelTarget && this.isOpen

    this.toggleTargets.forEach((toggle) => {
      toggle.setAttribute("aria-expanded", String(open))
      toggle.setAttribute("aria-label", open ? "Hide members" : "Show members")
      toggle.title = open ? "Hide members" : "Show members"
    })

    if (!this.hasPanelTarget) return

    this.panelTarget.toggleAttribute("inert", !open)
    this.panelTarget.setAttribute("aria-hidden", String(!open))
    this.panelTarget.setAttribute("aria-busy", String(Boolean(this.loading && open)))

    const modal = open && !this.desktopQuery.matches
    if (modal) {
      this.panelTarget.setAttribute("role", "dialog")
      this.panelTarget.setAttribute("aria-modal", "true")
    } else {
      this.panelTarget.removeAttribute("role")
      this.panelTarget.removeAttribute("aria-modal")
    }

    if (this.hasBackdropTarget) this.backdropTarget.hidden = !modal
  }

  async #fetchMembers() {
    if (!this.isOpen || !this.hasPanelTarget || this.loading || document.visibilityState !== "visible") return

    const url = this.panelTarget.dataset.membersUrl
    if (!url) {
      this.#clearMembers("Members unavailable.")
      return
    }

    if (this.contentTarget.childElementCount === 0) this.#clearMembers("Loading members…")
    this.#abortRequest()
    const request = new AbortController()
    this.request = request
    this.loading = true
    this.#syncAccessibility()

    try {
      const response = await fetch(url, {
        headers: { Accept: "application/json" },
        signal: request.signal
      })

      if (!response.ok) throw new Error(`Unable to load members (${response.status})`)
      const payload = await response.json()
      if (!Array.isArray(payload.members)) throw new Error("Invalid member response")
      if (request !== this.request || url !== this.panelTarget.dataset.membersUrl) return

      const members = payload.members.filter((member) => {
        return (typeof member.id === "number" || typeof member.id === "string") &&
          typeof member.name === "string" && member.name.trim().length > 0
      })

      this.loadedUrl = url
      this.#renderMembers(members)
    } catch (error) {
      if (error.name !== "AbortError" && request === this.request) {
        this.loadedUrl = null
        this.#clearMembers("Members unavailable.")
      }
    } finally {
      if (request === this.request) {
        this.request = null
        this.loading = false
        this.#syncAccessibility()
      }
    }
  }

  #renderMembers(members) {
    const starred = members.filter((member) => member.starred === true)
    const unstarred = members.filter((member) => member.starred !== true)
    const online = unstarred.filter((member) => member.online === true)
    const offline = unstarred.filter((member) => member.online !== true)
    const currentOfflineGroup = this.contentTarget.querySelector(".member-panel__offline")
    const offlineWasOpen = currentOfflineGroup?.open ?? true
    const offlineSummaryHadFocus = currentOfflineGroup?.querySelector("summary") === document.activeElement
    const fragment = document.createDocumentFragment()

    if (starred.length > 0) fragment.append(this.#memberGroup("Starred", starred, false))
    fragment.append(this.#memberGroup("Online", online, false))

    const offlineGroup = this.#memberGroup("Offline", offline, true)
    offlineGroup.open = offlineWasOpen
    fragment.append(offlineGroup)

    this.contentTarget.replaceChildren(fragment)
    if (offlineSummaryHadFocus) offlineGroup.querySelector("summary")?.focus()
    this.statusTarget.textContent = members.length === 0 ? "No members in this channel." : ""
    this.summaryTarget.textContent = `${members.length} ${members.length === 1 ? "member" : "members"}`
    this.toggleCountTargets.forEach((target) => { target.textContent = String(members.length) })
  }

  #memberGroup(label, members, collapsible) {
    const group = document.createElement(collapsible ? "details" : "section")
    group.className = `member-panel__group${collapsible ? " member-panel__offline" : ""}`

    const heading = document.createElement(collapsible ? "summary" : "h3")
    heading.className = "member-panel__group-heading"
    heading.textContent = `${label} — ${members.length}`
    group.append(heading)

    const list = document.createElement("ul")
    list.className = "member-panel__list"
    list.setAttribute("aria-label", `${label} members`)
    members.forEach((member) => list.append(this.#memberRow(member)))
    group.append(list)
    return group
  }

  #memberRow(member) {
    const online = member.online === true
    const presence = typeof member.presence === "string" && member.presence.length > 0
      ? member.presence
      : (online ? "online" : "offline")
    const label = PRESENCE_LABELS[presence] || presence
    const item = document.createElement("li")
    item.className = "member-panel__member multi-select-row"
    item.dataset.memberId = String(member.id)
    item.dataset.online = String(online)
    item.dataset.starred = String(member.starred === true)
    item.dataset.action = "touchstart->multi-select#pressStart touchmove->multi-select#pressMove touchend->multi-select#pressEnd contextmenu->multi-select#suppressMenu contextmenu->member-panel#openMenu keydown->member-panel#rowKeydown"

    const isSelf = String(member.id) === document.querySelector("meta[name='current-user-id']")?.content
    if (!isSelf) {
      const select = document.createElement("input")
      select.type = "checkbox"
      select.id = `select-member-${member.id}`
      select.dataset.multiSelectTarget = "checkbox"
      select.dataset.action = "click->multi-select#toggle"
      select.dataset.userId = String(member.id)
      select.dataset.bot = String(member.bot === true)
      select.setAttribute("aria-label", `Select ${member.name}`)
      item.append(select)
    }

    const cardUrl = this.#cardUrl(member.id)
    const avatar = document.createElement(cardUrl ? "button" : "span")
    if (cardUrl) {
      avatar.type = "button"
      avatar.setAttribute("aria-label", `View profile of ${member.name}`)
      avatar.dataset.action = "click->profile-card#open"
      avatar.dataset.profileCardUrl = cardUrl
    }
    avatar.className = "avatar member-panel__avatar profile-card-avatar"
    const image = document.createElement("img")
    image.src = member.avatar_url || this.panelTarget.dataset.defaultAvatarUrl
    image.alt = ""
    image.setAttribute("aria-hidden", "true")
    avatar.append(image)

    const dot = document.createElement("span")
    dot.className = "member-panel__presence"
    dot.dataset.presence = presence
    dot.setAttribute("aria-label", label)
    avatar.append(dot)

    const identity = document.createElement("span")
    identity.className = "member-panel__identity"
    const name = document.createElement(cardUrl ? "button" : "strong")
    if (cardUrl) {
      name.type = "button"
      name.dataset.action = "click->profile-card#open"
      name.dataset.profileCardUrl = cardUrl
    }
    name.className = cardUrl ? "profile-card-name overflow-ellipsis" : "overflow-ellipsis"
    const nameText = document.createElement("strong")
    nameText.textContent = member.name
    name.append(nameText)
    const status = document.createElement("span")
    status.className = "member-panel__status-label"
    status.textContent = (typeof member.status === "string" && member.status.length > 0) ? member.status : label
    identity.append(name, status)

    item.append(avatar, identity)
    return item
  }

  #cardUrl(memberId) {
    const template = this.panelTarget.dataset.cardUrlTemplate
    if (!template || memberId === undefined || memberId === null) return null

    return template.replace("USER_ID", String(memberId))
  }

  #starUrl(memberId) {
    const template = this.panelTarget.dataset.starUrlTemplate
    if (!template || memberId === undefined || memberId === null) return null

    return template.replace("USER_ID", String(memberId))
  }

  #menuRow() {
    if (!this.menuUserId || !this.hasContentTarget) return null
    return this.contentTarget.querySelector(`[data-member-id="${this.menuUserId}"]`)
  }

  #isSelfRow(row) {
    return row.dataset.memberId === document.querySelector("meta[name='current-user-id']")?.content
  }

  #openRowMenu(row, point) {
    this.menuUserId = row.dataset.memberId
    this.menuReturnFocus = row.querySelector("[data-action*='profile-card#open']")
    this.rowMenuStarTarget.textContent = row.dataset.starred === "true" ? "★ Unstar" : "☆ Star"
    this.rowMenuStarTarget.disabled = false
    this.rowMenuStatusTarget.textContent = ""

    const menu = this.rowMenuTarget
    menu.hidden = false
    menu.style.left = "0px"
    menu.style.top = "0px"
    const rect = menu.getBoundingClientRect()
    const left = Math.min(point.x, window.innerWidth - rect.width - 8)
    const top = Math.min(point.y, window.innerHeight - rect.height - 8)
    menu.style.left = `${Math.max(8, left)}px`
    menu.style.top = `${Math.max(8, top)}px`

    this.rowMenuStarTarget.focus({ preventScroll: true })
  }

  #closeRowMenu({ restoreFocus } = {}) {
    if (!this.hasRowMenuTarget || this.rowMenuTarget.hidden) {
      this.menuUserId = null
      this.menuReturnFocus = null
      return
    }

    this.rowMenuTarget.hidden = true
    const userId = this.menuUserId
    this.menuUserId = null

    if (restoreFocus) {
      const fallback = this.menuReturnFocus?.isConnected
        ? this.menuReturnFocus
        : this.contentTarget.querySelector(`[data-member-id="${userId}"] [data-action*='profile-card#open']`)
      fallback?.focus({ preventScroll: true })
    }
    this.menuReturnFocus = null
  }

  #handleRowMenuPointerDown(event) {
    if (!this.hasRowMenuTarget || this.rowMenuTarget.hidden) return
    if (this.rowMenuTarget.contains(event.target)) return
    if (this.#menuRow()?.contains(event.target)) return
    this.#closeRowMenu({ restoreFocus: false })
  }

  // Escape still reaches the menu after focus moved elsewhere (the
  // profile card opened from the same row, say): the menu's own keydown
  // handler only sees keys pressed inside it. Mirrors the sidebar room
  // menu; the contains check keeps the two handlers from double-closing.
  #handleRowMenuWindowKeydown(event) {
    if (!this.hasRowMenuTarget || this.rowMenuTarget.hidden) return
    if (event.key !== "Escape" || this.rowMenuTarget.contains(event.target)) return

    event.preventDefault()
    this.#closeRowMenu({ restoreFocus: true })
  }

  #clearMembers(message) {
    if (this.hasContentTarget) this.contentTarget.replaceChildren()
    if (this.hasStatusTarget) this.statusTarget.textContent = message
    if (this.hasSummaryTarget) this.summaryTarget.textContent = message === "Loading members…" ? "Loading…" : "No member data"
    this.toggleCountTargets.forEach((target) => { target.textContent = "" })
  }

  #startRefreshing() {
    if (!this.isOpen || document.visibilityState !== "visible") return
    this.refreshTimer ??= window.setInterval(() => this.#fetchMembers(), REFRESH_INTERVAL)
  }

  #stopRefreshing() {
    window.clearInterval(this.refreshTimer)
    this.refreshTimer = null
  }

  #abortRequest() {
    this.request?.abort()
    this.request = null
    this.loading = false
  }

  #handleVisibilityChange() {
    if (document.visibilityState === "visible" && this.isOpen) {
      this.#fetchMembers()
      this.#startRefreshing()
    } else {
      this.#stopRefreshing()
      this.#abortRequest()
    }
  }

  #handleViewportChange() {
    if (!this.hasPanelTarget) return

    if (this.desktopQuery.matches && !this.desktopClosed) {
      this.#open({ focusPanel: false, announce: false })
    } else if (!this.desktopQuery.matches) {
      this.#close({ restoreFocus: false })
    } else {
      this.#syncAccessibility()
    }
  }

  #focusableElements() {
    return Array.from(this.panelTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter((element) => {
      return !element.hidden && element.tabIndex >= 0 && element.getClientRects().length > 0
    })
  }

  #profileCardOpen() {
    const popover = document.getElementById("profile-card-popover")
    return !!popover && !popover.hidden
  }

  // Esc with a live selection clears it instead of closing the panel.
  #exitSelectionMode() {
    const surface = this.panelTarget?.querySelector("[data-controller~='multi-select']")
    const selection = surface && this.application.getControllerForElementAndIdentifier(surface, "multi-select")
    if (!selection || selection.selectedIds.size === 0) return false
    selection.clear()
    return true
  }

  #restoreFocus() {
    const previous = this.previouslyFocusedElement
    const previousIsUsable = previous?.isConnected && previous !== document.body && !this.panelTarget.contains(previous)
    const target = previousIsUsable ? previous : this.toggleTargets[0]
    target?.focus()
  }
}
