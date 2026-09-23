import { Controller } from "@hotwired/stimulus"

const FOCUSABLE_SELECTOR = "button:not([disabled]), a[href]:not([aria-disabled='true'])"
const VIEWPORT_PADDING = 8

// The room header's overflow menu: below 80rem the less-used actions move
// here instead of being silently hidden (see workspace.css). Phones keep
// only members, search, call and this button; tablets keep notifications
// and settings as well. Stage rooms keep their stage toggle visible on
// tablets and move it here on phones, where the header has no room for an
// extra action.
//
// Items trigger exactly the same actions as the header buttons they stand
// in for. Links navigate to the same paths; buttons that own a
// body-level controller (threads, the quick switcher) share its action;
// everything else scoped to a header wrapper (pins, stage, the help
// items) forwards a click to the real, hidden control; notifications
// names its level in an explicit chooser over the involvement endpoint.
export default class extends Controller {
  static targets = [ "button", "menu", "dot", "pinsCount", "notificationsLabel", "notificationsPanel", "notificationsStatus", "notificationsIcon" ]

  #open = false
  #badgeObserver

  connect() {
    this.onMenuKeydown = this.#onMenuKeydown.bind(this)
    this.onMenuClick = this.#onMenuClick.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onWindowKeydown = this.#onWindowKeydown.bind(this)
    this.onBeforeCache = this.#onBeforeCache.bind(this)
    this.onResize = this.#onResize.bind(this)

    this.menuTarget.addEventListener("keydown", this.onMenuKeydown)
    this.menuTarget.addEventListener("click", this.onMenuClick)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    window.addEventListener("keydown", this.onWindowKeydown)
    document.addEventListener("turbo:before-cache", this.onBeforeCache)
    window.addEventListener("resize", this.onResize)

    this.#syncPinsCount()
    this.#updateDot()
    this.#observeBadges()
  }

  disconnect() {
    this.#badgeObserver?.disconnect()
    this.#badgeObserver = null

    this.menuTarget?.removeEventListener("keydown", this.onMenuKeydown)
    this.menuTarget?.removeEventListener("click", this.onMenuClick)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    window.removeEventListener("keydown", this.onWindowKeydown)
    document.removeEventListener("turbo:before-cache", this.onBeforeCache)
    window.removeEventListener("resize", this.onResize)
  }

  toggle(event) {
    event?.preventDefault()
    if (this.#open) {
      this.#closeMenu()
    } else {
      this.#openMenu()
    }
  }

  buttonKeydown(event) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault()
      this.#openMenu(event.key === "ArrowUp" ? "last" : "first")
    }
  }

  // Shared-action items (threads, the switcher) run their own controller
  // first; this closes the menu without stealing the panel or dialog focus
  // that action just set.
  closeAfterActivate() {
    this.#closeMenu({ restoreFocus: false })
  }

  // Scoped-control items carry data-header-overflow-forward-value with a
  // selector for the real header control. The menu closes back onto its
  // button first, then the real control runs, so dialogs and panels take
  // focus from the button and plain toggles land there.
  forward(event) {
    event.preventDefault()
    const selector = event.currentTarget?.dataset.headerOverflowForwardValue
    this.#closeMenu()
    if (selector) document.querySelector(selector)?.click()
  }

  // The notifications toggle names the current level and expands an
  // explicit chooser. Choosing submits exactly that level through the
  // involvement endpoint (the same one the bell uses) instead of cycling
  // blind through mentions → everything → muted → nothing → invisible.
  toggleNotifications(event) {
    event.preventDefault()
    const panel = this.notificationsPanelTarget
    const button = event.currentTarget
    const open = panel.hidden
    panel.hidden = !open
    button.setAttribute("aria-expanded", String(open))
    this.#positionMenu()
    if (open) panel.querySelector("[role='menuitemradio'][aria-checked='true']")?.focus({ preventScroll: true })
  }

  async chooseInvolvement(event) {
    event.preventDefault()
    const option = event.currentTarget
    const level = option.dataset.headerOverflowLevelValue
    const url = this.element.dataset.headerOverflowInvolvementUrlValue
    const response = await fetch(`${url}?involvement=${encodeURIComponent(level)}`, {
      method: "PUT",
      headers: {
        Accept: "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || ""
      }
    }).catch(() => null)

    if (!response?.ok) {
      this.notificationsStatusTarget.textContent = "Couldn't change the notification level"
      return
    }

    this.notificationsStatusTarget.textContent = ""
    this.notificationsLabelTarget.textContent = `Notifications: ${option.dataset.headerOverflowLabelValue}`
    this.notificationsPanelTarget.querySelectorAll("[role='menuitemradio']").forEach(candidate => {
      const current = candidate === option
      candidate.setAttribute("aria-checked", String(current))
      candidate.querySelector("[data-header-overflow-check]").textContent = current ? "✓" : ""
    })
    if (this.hasNotificationsIconTarget) {
      const icon = this.notificationsIconTarget
      icon.src = icon.src.replace(/notification-bell-[a-z]+/, `notification-bell-${level}`)
    }
    this.#reloadInvolvementFrame()
  }

  #onMenuKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault()
      this.#closeMenu()
      return
    }

    const items = this.#menuItems()
    if (items.length === 0) return

    const currentIndex = Math.max(0, items.indexOf(document.activeElement))
    let nextIndex

    switch (event.key) {
      case "ArrowDown":
        nextIndex = (currentIndex + 1) % items.length
        break
      case "ArrowUp":
        nextIndex = (currentIndex - 1 + items.length) % items.length
        break
      case "Home":
        nextIndex = 0
        break
      case "End":
        nextIndex = items.length - 1
        break
      case "Tab":
        event.preventDefault()
        this.#closeMenu()
        return
      default:
        return
    }

    event.preventDefault()
    items[nextIndex].focus({ preventScroll: true })
    // The menu scrolls on short viewports: bring the focused item into
    // view inside the menu without moving the page behind it.
    items[nextIndex].scrollIntoView({ block: "nearest" })
  }

  #onMenuClick(event) {
    const item = event.target.closest(FOCUSABLE_SELECTOR)
    if (!item || !this.menuTarget.contains(item)) return
    if (item.dataset.action?.includes("header-overflow#")) return

    // Plain links navigate on their own; the delayed close lets the
    // browser finish the activation first.
    setTimeout(() => this.#closeMenu({ restoreFocus: false }), 0)
  }

  #onDocumentPointerDown(event) {
    if (!this.#open) return
    if (this.menuTarget.contains(event.target)) return
    if (this.buttonTarget.contains(event.target)) return
    this.#closeMenu({ restoreFocus: false })
  }

  #onWindowKeydown(event) {
    if (this.#open && event.key === "Escape" && !this.menuTarget.contains(event.target)) {
      event.preventDefault()
      this.#closeMenu()
    }
  }

  #onBeforeCache() {
    this.#closeMenu({ restoreFocus: false })
  }

  #onResize() {
    if (!this.#open) return
    if (!this.#isVisible(this.buttonTarget)) {
      this.#closeMenu({ restoreFocus: false })
    } else {
      this.#positionMenu()
    }
  }

  #openMenu(focus = "first") {
    this.#open = true
    this.buttonTarget.setAttribute("aria-expanded", "true")
    this.menuTarget.hidden = false
    this.#showMenuPopover()
    this.#positionMenu()
    this.#syncPinsCount()

    const items = this.#menuItems()
    const target = focus === "last" ? items[items.length - 1] : items[0]
    const focusTarget = () => {
      if (this.#open) {
        this.#positionMenu()
        target?.focus({ preventScroll: true })
      }
    }
    if (window.requestAnimationFrame) window.requestAnimationFrame(focusTarget)
    else setTimeout(focusTarget, 0)
  }

  #closeMenu({ restoreFocus = true } = {}) {
    if (!this.#open && this.menuTarget?.hidden) return
    this.#open = false

    if (this.menuTarget) {
      this.#hideMenuPopover()
      this.menuTarget.hidden = true
    }
    this.buttonTarget?.setAttribute("aria-expanded", "false")
    if (restoreFocus && this.buttonTarget?.isConnected) {
      this.buttonTarget.focus({ preventScroll: true })
    }
  }

  #positionMenu() {
    if (!this.#open || !this.menuTarget || this.menuTarget.hidden) return

    const buttonRect = this.buttonTarget.getBoundingClientRect()
    const menuRect = this.menuTarget.getBoundingClientRect()
    const rtl = document.dir === "rtl" || getComputedStyle(document.documentElement).direction === "rtl"

    const top = buttonRect.bottom + VIEWPORT_PADDING
    const maxTop = Math.max(VIEWPORT_PADDING, window.innerHeight - menuRect.height - VIEWPORT_PADDING)
    const endAligned = rtl ? buttonRect.left : buttonRect.right - menuRect.width
    const maxLeft = Math.max(VIEWPORT_PADDING, window.innerWidth - menuRect.width - VIEWPORT_PADDING)

    this.menuTarget.style.top = `${Math.min(Math.max(VIEWPORT_PADDING, top), maxTop)}px`
    this.menuTarget.style.left = `${Math.min(Math.max(VIEWPORT_PADDING, endAligned), maxLeft)}px`
  }

  #showMenuPopover() {
    if (this.menuTarget.getAttribute("popover") === null || typeof this.menuTarget.showPopover !== "function") return
    try {
      this.menuTarget.showPopover()
    } catch {
      // The fixed-position fallback remains usable without popover.
    }
  }

  #hideMenuPopover() {
    if (typeof this.menuTarget.hidePopover !== "function") return
    try {
      if (this.menuTarget.matches(":popover-open")) this.menuTarget.hidePopover()
    } catch {
      // The hidden attribute below still closes the fallback.
    }
  }

  #menuItems() {
    return Array.from(this.menuTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter(item => {
      if (item.hidden || item.closest("[hidden]")) return false
      return this.#isVisible(item)
    })
  }

  #isVisible(item) {
    if (item.getClientRects().length > 0) return true
    return window.getComputedStyle(item).display !== "none"
  }

  // Badges travel with their items: the pins menu badge mirrors the header
  // count (hidden below 80rem with its button), while the threads menu item
  // is a second thread-panel browser toggle so its unread badge updates in
  // place. The button dot lights when any of them carries state.
  #observeBadges() {
    const headerActions = this.element.closest(".room-header__actions") || document.body
    this.#badgeObserver = new MutationObserver(() => {
      this.#syncPinsCount()
      this.#updateDot()
    })
    this.#badgeObserver.observe(headerActions, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: [ "hidden" ] })
    this.#badgeObserver.observe(this.menuTarget, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: [ "hidden" ] })
  }

  // After an explicit choice the header bell still offers the next level
  // from the old state, so refresh its frame. A bell that never loaded
  // (push never became ready) has no src to reload, so set it instead.
  #reloadInvolvementFrame() {
    const frame = document.getElementById(this.element.dataset.headerOverflowInvolvementFrameValue || "")
    if (!frame) return
    if (frame.getAttribute("src")) frame.reload()
    else frame.src = this.element.dataset.headerOverflowInvolvementUrlValue
  }

  #syncPinsCount() {
    if (!this.hasPinsCountTarget) return
    const source = document.querySelector(".room-header__actions .room-header__pins-count")
    // Guarded: the badge observer watches this target, and an unconditional
    // write would queue another mutation on every callback forever.
    const text = source?.textContent?.trim() || ""
    if (this.pinsCountTarget.textContent !== text) this.pinsCountTarget.textContent = text
  }

  #updateDot() {
    if (!this.hasDotTarget) return

    const pins = document.querySelector(".room-header__actions .room-header__pins-count")?.textContent?.trim()
    const pinsUnread = pins !== undefined && pins !== "" && pins !== "0"

    const threadsUnread = Array.from(document.querySelectorAll(".room-header__actions [data-thread-panel-unread], #header-overflow-menu [data-thread-panel-unread]"))
      .some(badge => !badge.hidden && badge.textContent?.trim() !== "")

    const menuBadgesUnread = Array.from(this.menuTarget.querySelectorAll("[data-header-overflow-badge]"))
      .some(badge => {
        if (badge.hidden || badge.closest("[hidden]")) return false
        if (!this.#isVisible(badge)) return false
        const text = badge.textContent?.trim() || ""
        return text !== "" && text !== "0"
      })

    // Guarded like the pins mirror above: setting hidden unconditionally
    // re-triggers the badge observer on every callback.
    const hidden = !(pinsUnread || threadsUnread || menuBadgesUnread)
    if (this.dotTarget.hidden !== hidden) this.dotTarget.hidden = hidden
  }
}
