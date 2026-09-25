import { Controller } from "@hotwired/stimulus"

const MOBILE_QUERY = "(max-width: 63.999rem)"
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])"
].join(",")

// Room lists that have already been opened once. The sidebar frame is
// turbo-permanent, so its scroller outlives this controller across visits;
// keying on the element (not the controller) keeps "first open" meaning the
// first open of this list, and a frame reload that swaps the list resets it.
const revealedScrollers = new WeakSet()

export default class extends Controller {
  static targets = [ "sidebar", "opener", "backdrop" ]

  connect() {
    this.mobileQuery = window.matchMedia(MOBILE_QUERY)
    this.handleViewportChange = this.#handleViewportChange.bind(this)
    this.mobileQuery.addEventListener("change", this.handleViewportChange)
    this.#updateAccessibility()
    this.syncCurrentRoom()
  }

  disconnect() {
    this.mobileQuery?.removeEventListener("change", this.handleViewportChange)
    this.element.classList.remove("workspace-navigation-open")
  }

  open() {
    if (!this.mobileQuery.matches || !this.hasSidebarTarget) return

    window.dispatchEvent(new CustomEvent("workspace-navigation:opening"))
    this.previouslyFocusedElement = document.activeElement
    this.sidebarTarget.classList.add("open")
    this.element.classList.add("workspace-navigation-open")
    this.#updateAccessibility()
    // The open state does not transition visibility (workspace.css), so
    // the drawer is focusable as soon as the class lands; focus never
    // waits on the slide.
    requestAnimationFrame(() => {
      if (this.sidebarTarget.classList.contains("open")) this.#focusNavigation({ allowReveal: true })
    })
  }

  close(event) {
    if (!this.hasSidebarTarget || !this.sidebarTarget.classList.contains("open")) return

    event?.preventDefault()
    this.sidebarTarget.classList.remove("open")
    this.element.classList.remove("workspace-navigation-open")
    this.#updateAccessibility()

    const clickedBackdrop = this.hasBackdropTarget && event?.currentTarget === this.backdropTarget

    if (event?.type !== "click" || clickedBackdrop) {
      this.previouslyFocusedElement?.focus()
    }
  }

  navigate(event) {
    const link = event.target.closest("a[href]")
    const staysInSidebarFrame = link?.dataset.turboFrame && link.dataset.turboFrame !== "_top"

    if (link && !staysInSidebarFrame && this.mobileQuery.matches) {
      this.sidebarTarget.classList.remove("open")
      this.element.classList.remove("workspace-navigation-open")
      this.#updateAccessibility()
    }
  }

  trapFocus(event) {
    if (event.key !== "Tab") return
    if (!this.mobileQuery.matches || !this.sidebarTarget.classList.contains("open")) return

    const focusable = this.#focusableElements()
    if (focusable.length === 0) return

    const first = focusable[0]
    const last = focusable[focusable.length - 1]

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }

  syncCurrentRoom() {
    if (!this.hasSidebarTarget) return

    const currentRoomId = document.querySelector('meta[name="current-room-id"]')?.content

    this.sidebarTarget.querySelectorAll("[data-workspace-destination]").forEach((link) => {
      const isCurrent = window.location.pathname === link.dataset.workspaceDestination
      if (isCurrent) {
        link.setAttribute("aria-current", "page")
      } else {
        link.removeAttribute("aria-current")
      }
    })

    this.sidebarTarget.querySelectorAll("[data-room-id]").forEach((roomLink) => {
      const isCurrent = Boolean(currentRoomId) && roomLink.dataset.roomId === currentRoomId
      roomLink.classList.toggle("room--active", isCurrent)

      if (isCurrent) {
        roomLink.setAttribute("aria-current", "page")
      } else {
        roomLink.removeAttribute("aria-current")
      }
    })

    if (this.mobileQuery.matches && this.sidebarTarget.classList.contains("open") && !this.sidebarTarget.contains(document.activeElement)) {
      this.#focusNavigation()
    }
  }

  updateUnreadStatus(event) {
    const targetId = String(event.detail.targetId)
    const roomLink = Array.from(this.sidebarTarget.querySelectorAll("[data-room-id]")).find((link) => {
      return link.id === targetId || link.dataset.roomId === targetId
    })

    if (!roomLink) return

    const isUnread = event.type === "rooms-list:unread"
    const currentStatus = roomLink.querySelector(":scope > .sidebar-item__status")

    if (isUnread && !currentStatus) {
      const status = document.createElement("span")
      status.className = "sidebar-item__status"
      status.textContent = "New"
      roomLink.append(status)
    } else if (!isUnread) {
      currentStatus?.remove()
    }
  }

  #handleViewportChange() {
    if (!this.mobileQuery.matches) {
      this.sidebarTarget.classList.remove("open")
      this.element.classList.remove("workspace-navigation-open")
    }

    this.#updateAccessibility()
  }

  #updateAccessibility() {
    if (!this.hasSidebarTarget) return

    const isMobile = this.mobileQuery.matches
    const isOpen = !isMobile || this.sidebarTarget.classList.contains("open")

    this.openerTargets.forEach((opener) => opener.setAttribute("aria-expanded", String(isMobile && isOpen)))
    this.sidebarTarget.toggleAttribute("inert", !isOpen)

    if (isMobile) {
      this.sidebarTarget.setAttribute("aria-hidden", String(!isOpen))
      if (this.hasBackdropTarget) this.backdropTarget.hidden = !isOpen
    } else {
      this.sidebarTarget.removeAttribute("aria-hidden")
      if (this.hasBackdropTarget) this.backdropTarget.hidden = true
    }
  }

  #focusableElements() {
    return Array.from(this.sidebarTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter((element) => {
      return !element.hidden && element.tabIndex >= 0 && element.getClientRects().length > 0
    })
  }

  // The first open of a room list reveals the current room (a normal focus
  // scroll) so the drawer starts where the user is. After that the list's
  // scroll position belongs to the user: focus lands on the current room only
  // when it is already fully in view, otherwise on the first control visible
  // in the drawer, and never scrolls anything.
  #focusNavigation({ allowReveal = false } = {}) {
    const currentRoom = this.sidebarTarget.querySelector("a[aria-current='page']")
    const scroller = this.#scroller()

    if (allowReveal && currentRoom && scroller && !revealedScrollers.has(scroller)) {
      currentRoom.focus()
      // Only a reveal that actually happened spends the list's first open.
      if (document.activeElement === currentRoom) {
        revealedScrollers.add(scroller)
        return
      }
    }

    const focusable = this.#focusableElements()
    const target = currentRoom && this.#isVisibleInDrawer(currentRoom)
      ? currentRoom
      : focusable.find((element) => this.#isVisibleInDrawer(element)) || focusable[0]

    target?.focus({ preventScroll: true })
  }

  #isVisibleInDrawer(element) {
    const rect = element.getBoundingClientRect()
    if (rect.width === 0 || rect.height === 0) return false

    const scroller = this.#scroller()
    let top = 0
    let bottom = window.innerHeight

    if (scroller?.contains(element)) {
      top = scroller.getBoundingClientRect().top + scroller.clientTop
      bottom = top + scroller.clientHeight
    }

    return rect.top >= top - 1 && rect.bottom <= bottom + 1
  }

  #scroller() {
    return this.sidebarTarget.querySelector(".sidebar__scroll")
  }
}
