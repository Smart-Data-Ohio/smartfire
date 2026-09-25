import { Controller } from "@hotwired/stimulus"

const FOCUSABLE_SELECTOR = "button:not([disabled])"
const VIEWPORT_PADDING = 8

// The shared sidebar room menu: right-click or Shift+F10 on any room row
// opens it for that row (favourite, reorder, mute, categorize). One menu
// per sidebar, configured per row like the shared message menu.
export default class extends Controller {
  static targets = [
    "menu", "favoriteAction", "favoriteGlyph", "favoriteLabel", "moveUp", "moveDown",
    "muteAction", "muteLabel", "categoryGroup", "categoryList", "removeFromCategory",
    "dangerGroup", "leaveAction", "deleteAction",
    "confirmDialog", "confirmMessage", "confirmButton", "status"
  ]

  static values = { flashKey: { type: String, default: "room-menu-flash" } }

  #row
  #open = false
  #announceTimer
  #pending

  connect() {
    this.onContextMenu = this.#onContextMenu.bind(this)
    this.onKeydown = this.#onKeydown.bind(this)
    this.onMenuKeydown = this.#onMenuKeydown.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onWindowKeydown = this.#onWindowKeydown.bind(this)
    this.onBeforeCache = this.#onBeforeCache.bind(this)
    this.onDialogClose = this.#onDialogClose.bind(this)
    this.onPageLoad = this.#onPageLoad.bind(this)

    this.element.addEventListener("contextmenu", this.onContextMenu)
    this.element.addEventListener("keydown", this.onKeydown)
    this.menuTarget.addEventListener("keydown", this.onMenuKeydown)
    this.confirmDialogTarget.addEventListener("close", this.onDialogClose)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    document.addEventListener("turbo:load", this.onPageLoad)
    window.addEventListener("keydown", this.onWindowKeydown)
    document.addEventListener("turbo:before-cache", this.onBeforeCache)
    this.#showPendingFlash()
  }

  disconnect() {
    clearTimeout(this.#announceTimer)
    this.element.removeEventListener("contextmenu", this.onContextMenu)
    this.element.removeEventListener("keydown", this.onKeydown)
    this.menuTarget?.removeEventListener("keydown", this.onMenuKeydown)
    this.confirmDialogTarget?.removeEventListener("close", this.onDialogClose)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    document.removeEventListener("turbo:load", this.onPageLoad)
    window.removeEventListener("keydown", this.onWindowKeydown)
    document.removeEventListener("turbo:before-cache", this.onBeforeCache)
  }

  async toggleFavorite() {
    const roomId = this.#roomId
    const favorited = this.#row.dataset.menuFavorited === "true"
    const response = await this.#request(`/rooms/${roomId}/favorite`, favorited ? "DELETE" : "POST")
    if (!response?.ok) return this.#announce(favorited ? "Couldn’t remove from favourites" : "Couldn’t add to favourites")
    this.#reloadSidebar()
  }

  async moveUp() {
    await this.#moveFavorite(-1)
  }

  async moveDown() {
    await this.#moveFavorite(1)
  }

  async toggleMute() {
    const roomId = this.#roomId
    const muted = this.#row.dataset.menuMuted === "true"
    const involvement = muted ? this.#row.dataset.menuDefaultInvolvement : "muted"
    const response = await this.#request(`/rooms/${roomId}/involvement?involvement=${involvement}`, "PUT")
    if (!response?.ok) return this.#announce(muted ? "Couldn’t unmute room" : "Couldn’t mute room")
    this.#reloadSidebar()
  }

  async assignCategory(event) {
    const categoryId = event.currentTarget?.dataset.categoryId
    const response = await this.#assignCategoryRequest(categoryId)
    if (!response?.ok) return this.#announce("Couldn’t move to category")
    this.#reloadSidebar()
  }

  async removeFromCategory() {
    const response = await this.#assignCategoryRequest(null)
    if (!response?.ok) return this.#announce("Couldn’t remove from category")
    this.#reloadSidebar()
  }

  askLeave() {
    const label = this.#row.dataset.menuRoomLabel
    let message = `Leave #${label}? You'll stop seeing it in your sidebar.`
    if (this.#row.dataset.menuDirectRoom === "true") {
      message += " You can be added back later."
    } else if (this.#row.dataset.menuOpenRoom !== "true") {
      message += " Someone will need to add you back."
    }
    this.#askConfirm({
      type: "leave",
      url: this.#row.dataset.menuLeaveUrl,
      roomId: this.#roomId,
      message,
      confirmLabel: "Leave",
      destructive: false,
    })
  }

  askDelete() {
    const label = this.#row.dataset.menuRoomLabel
    this.#askConfirm({
      type: "delete",
      url: `/rooms/${this.#roomId}`,
      roomId: this.#roomId,
      message: `Delete #${label} and all its messages? This can't be undone.`,
      confirmLabel: "Delete",
      destructive: true,
      notice: `Deleted #${label}`,
    })
  }

  cancelPending() {
    this.confirmDialogTarget.close()
  }

  async confirmPending() {
    const pending = this.#pending
    this.confirmDialogTarget.close()
    if (!pending) return

    const response = await this.#request(pending.url, "DELETE")
    if (!response?.ok) {
      const verb = pending.type === "delete" ? "delete" : "leave"
      this.#flashError(`Couldn’t ${verb} this room.`)
      return
    }

    // The server broadcast removes the row everywhere for a delete, and for
    // the leaver's other tabs on a leave. Remove it here too: leaving resets
    // this member's cable connections, so the broadcast can land while this
    // tab is reconnecting and never arrive.
    this.#removeRows(pending.roomId)

    if (this.#currentRoomId === pending.roomId) {
      if (pending.notice) this.#savePendingFlash(pending.notice)
      Turbo.visit("/")
    } else if (pending.notice) {
      this.#flashNotice(pending.notice)
    }
  }

  #removeRows(roomId) {
    this.element.querySelectorAll(`a[data-room-id="${CSS.escape(String(roomId))}"]`).forEach((row) => row.remove())
  }

  // Events

  #onContextMenu(event) {
    const row = event.target.closest?.("a[data-room-id]")
    if (!row || !this.element.contains(row)) return
    event.preventDefault()
    this.#openFor(row, { x: event.clientX, y: event.clientY })
  }

  #onKeydown(event) {
    const menuKey = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)
    if (!menuKey) return
    const row = event.target.closest?.("a[data-room-id]")
    if (!row || !this.element.contains(row)) return
    event.preventDefault()
    const rect = row.getBoundingClientRect()
    this.#openFor(row, { x: rect.left + Math.min(rect.width / 2, 240), y: rect.bottom })
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
    items[nextIndex].focus()
  }

  #onDocumentPointerDown(event) {
    if (!this.#open) return
    if (this.menuTarget.contains(event.target)) return
    if (this.#row?.contains(event.target)) return
    this.#closeMenu()
  }

  #onWindowKeydown(event) {
    if (this.#open && event.key === "Escape" && !this.menuTarget.contains(event.target)) {
      event.preventDefault()
      this.#closeMenu()
    }
  }

  #onBeforeCache() {
    this.#pending = null
    if (this.confirmDialogTarget.open) this.confirmDialogTarget.close()
    this.#closeMenu({ restoreFocus: false })
  }

  #onDialogClose() {
    const row = this.#pending?.row
    this.#pending = null
    if (row?.isConnected) row.focus()
  }

  // The sidebar frame is permanent across Turbo visits, so a notice saved
  // before navigating home would never render on connect: every page load
  // picks it up instead.
  #onPageLoad() {
    this.#showPendingFlash()
  }

  #showPendingFlash() {
    let notice
    try {
      notice = sessionStorage.getItem(this.flashKeyValue)
      if (notice) sessionStorage.removeItem(this.flashKeyValue)
    } catch {
      return
    }
    if (notice) this.#flashNotice(notice)
  }

  // Internal

  async #openFor(row, point) {
    this.#row = row
    await this.#configureForRow()
    if (this.#row !== row) return
    this.#openMenu(point)
  }

  async #configureForRow() {
    const favorited = this.#row.dataset.menuFavorited === "true"
    this.favoriteGlyphTarget.textContent = favorited ? "★" : "☆"
    this.favoriteLabelTarget.textContent = favorited ? "Remove from favourites" : "Add to favourites"

    const favoriteRows = Array.from(document.querySelectorAll("#favorite_rooms a[data-room-id]"))
    const favoriteIndex = favoriteRows.indexOf(this.#row)
    this.moveUpTarget.hidden = favoriteIndex < 0
    this.moveDownTarget.hidden = favoriteIndex < 0
    this.moveUpTarget.disabled = favoriteIndex <= 0
    this.moveDownTarget.disabled = favoriteIndex < 0 || favoriteIndex >= favoriteRows.length - 1

    const muted = this.#row.dataset.menuMuted === "true"
    this.muteLabelTarget.textContent = muted ? "Unmute" : "Mute"

    const canLeave = this.#row.dataset.menuCanLeave === "true"
    const canDelete = this.#row.dataset.menuCanDelete === "true"
    this.leaveActionTarget.hidden = !canLeave
    this.deleteActionTarget.hidden = !canDelete
    this.dangerGroupTarget.hidden = !canLeave && !canDelete

    const categorizable = this.#row.dataset.menuCategorizable === "true"
    // Fetched on every open: the sidebar frame reloads around the menu,
    // so only a live read is guaranteed current.
    const categories = categorizable ? await this.#fetchCategories() : []
    this.categoryGroupTarget.hidden = !categorizable || categories.length === 0
    if (!this.categoryGroupTarget.hidden) this.#renderCategories(categories)
    this.removeFromCategoryTarget.hidden = !this.#row.dataset.menuCategoryId
  }

  #renderCategories(categories) {
    const currentId = this.#row.dataset.menuCategoryId
    this.categoryListTarget.innerHTML = ""

    categories.forEach(category => {
      const button = document.createElement("button")
      button.type = "button"
      button.className = "btn room-menu__item"
      button.setAttribute("role", "menuitem")
      button.dataset.action = "room-menu#assignCategory"
      button.dataset.categoryId = category.id
      if (String(category.id) === currentId) button.setAttribute("aria-current", "true")

      const check = document.createElement("span")
      check.setAttribute("aria-hidden", "true")
      check.textContent = String(category.id) === currentId ? "✓ " : ""
      button.append(check, document.createTextNode(category.name))
      this.categoryListTarget.append(button)
    })
  }

  async #fetchCategories() {
    try {
      const response = await fetch("/room_categories.json", { headers: { Accept: "application/json" } })
      if (!response.ok) return []
      return await response.json()
    } catch {
      return []
    }
  }

  async #moveFavorite(direction) {
    const favoriteRows = Array.from(document.querySelectorAll("#favorite_rooms a[data-room-id]"))
    const index = favoriteRows.indexOf(this.#row)
    if (index < 0) return

    const response = await this.#request(`/rooms/${this.#roomId}/favorite`, "PATCH", { position: index + direction })
    if (!response?.ok) return this.#announce("Couldn’t reorder favourites")
    this.#reloadSidebar()
  }

  #assignCategoryRequest(categoryId) {
    const body = categoryId ? { room_category_id: categoryId } : { room_category_id: "" }
    return this.#request(`/rooms/${this.#roomId}/category_assignment`, "PATCH", body)
  }

  #askConfirm(pending) {
    this.#pending = { ...pending, row: this.#row }
    this.#closeMenu({ restoreFocus: false })
    this.confirmMessageTarget.textContent = pending.message
    this.confirmButtonTarget.textContent = pending.confirmLabel
    this.confirmButtonTarget.classList.toggle("btn--negative", pending.destructive)
    this.confirmButtonTarget.classList.toggle("btn--reversed", !pending.destructive)
    this.confirmDialogTarget.showModal()
  }

  #savePendingFlash(notice) {
    try {
      sessionStorage.setItem(this.flashKeyValue, notice)
    } catch {
      // Private browsing: the notice is lost with the navigation.
    }
  }

  #flashNotice(message) {
    this.#flash(message, false)
  }

  #flashError(message) {
    this.#flash(message, true)
  }

  // A client-side flash matching the server flash markup: it removes
  // itself on animationend through element-removal and inherits the
  // reduced-motion persistence. The menu is closed by now, so there is
  // nothing to announce into.
  #flash(message, error) {
    const flash = document.createElement("div")
    flash.className = "flash flash--client"
    flash.dataset.controller = "element-removal"
    flash.dataset.action = "animationend->element-removal#remove"
    flash.setAttribute("role", "alert")

    const inner = document.createElement("div")
    inner.className = "flash__inner flash__inner--text shadow"
    if (error) inner.style.setProperty("--flash-background", "var(--color-negative)")
    inner.textContent = message

    const dismiss = document.createElement("button")
    dismiss.type = "button"
    dismiss.className = "flash__dismiss"
    dismiss.dataset.action = "element-removal#remove"
    dismiss.setAttribute("aria-label", "Dismiss notification")
    dismiss.textContent = "×"

    flash.append(inner, dismiss)
    document.body.append(flash)
  }

  async #request(url, method, body) {
    return await fetch(url, {
      method,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
      body: body ? JSON.stringify(body) : undefined,
    }).catch(() => null)
  }

  // Every menu action ends in a sidebar reload: the row may have moved
  // sections, and the menu element itself is replaced with the frame.
  #reloadSidebar() {
    this.#closeMenu({ restoreFocus: false })
    document.getElementById("user_sidebar")?.reload()
  }

  #openMenu({ x, y }) {
    this.#open = true
    this.#row.setAttribute("aria-expanded", "true")
    if (this.menuTarget.id) this.#row.setAttribute("aria-controls", this.menuTarget.id)

    this.menuTarget.hidden = false
    if (!this.menuTarget.matches(":popover-open")) this.menuTarget.showPopover()
    this.#positionMenu(x, y)

    const focus = () => {
      if (this.#open) this.#menuItems()[0]?.focus({ preventScroll: true })
    }
    if (window.requestAnimationFrame) window.requestAnimationFrame(focus)
    else setTimeout(focus, 0)
  }

  #closeMenu({ restoreFocus = true } = {}) {
    if (!this.#open && this.menuTarget?.hidden) return
    this.#open = false

    if (this.menuTarget) {
      try { this.menuTarget.hidePopover() } catch { /* already hidden */ }
      this.menuTarget.hidden = true
    }
    if (restoreFocus && this.#row?.isConnected) this.#row.focus()
    this.#row?.setAttribute("aria-expanded", "false")
    this.#row = null
  }

  #positionMenu(x, y) {
    const menu = this.menuTarget
    menu.style.left = "0px"
    menu.style.top = "0px"
    // offsetWidth/Height ignore the enter scale: getBoundingClientRect
    // would measure the menu mid-pop at 0.97 and clamp it past the edge.
    const menuWidth = menu.offsetWidth
    const menuHeight = menu.offsetHeight
    const left = Math.min(x, window.innerWidth - menuWidth - VIEWPORT_PADDING)
    const top = Math.min(y, window.innerHeight - menuHeight - VIEWPORT_PADDING)
    menu.style.left = `${Math.max(VIEWPORT_PADDING, left)}px`
    menu.style.top = `${Math.max(VIEWPORT_PADDING, top)}px`
  }

  #menuItems() {
    return Array.from(this.menuTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter(item => {
      if (item.hidden || item.closest("[hidden]")) return false
      // offsetParent is null for fixed-position popovers, so rects decide.
      if (item.getClientRects().length > 0) return true
      return window.getComputedStyle(item).display !== "none"
    })
  }

  #announce(message) {
    if (!this.hasStatusTarget) return
    this.statusTarget.textContent = message
    clearTimeout(this.#announceTimer)
    this.#announceTimer = setTimeout(() => this.statusTarget.textContent = "", 2_000)
  }

  get #roomId() {
    return this.#row.dataset.roomId
  }

  get #currentRoomId() {
    return document.querySelector("meta[name='current-room-id']")?.content
  }
}
