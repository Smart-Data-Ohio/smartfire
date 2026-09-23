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
    "status", "categoriesData"
  ]

  #row
  #open = false
  #announceTimer

  connect() {
    this.onContextMenu = this.#onContextMenu.bind(this)
    this.onKeydown = this.#onKeydown.bind(this)
    this.onMenuKeydown = this.#onMenuKeydown.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onWindowKeydown = this.#onWindowKeydown.bind(this)
    this.onBeforeCache = this.#onBeforeCache.bind(this)

    this.element.addEventListener("contextmenu", this.onContextMenu)
    this.element.addEventListener("keydown", this.onKeydown)
    this.menuTarget.addEventListener("keydown", this.onMenuKeydown)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    window.addEventListener("keydown", this.onWindowKeydown)
    document.addEventListener("turbo:before-cache", this.onBeforeCache)
  }

  disconnect() {
    clearTimeout(this.#announceTimer)
    this.element.removeEventListener("contextmenu", this.onContextMenu)
    this.element.removeEventListener("keydown", this.onKeydown)
    this.menuTarget?.removeEventListener("keydown", this.onMenuKeydown)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
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
    this.#closeMenu({ restoreFocus: false })
  }

  // Internal

  #openFor(row, point) {
    this.#row = row
    this.#configureForRow()
    this.#openMenu(point)
  }

  #configureForRow() {
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

    const categorizable = this.#row.dataset.menuCategorizable === "true"
    const categories = this.#categories()
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

  #categories() {
    try {
      return JSON.parse(this.categoriesDataTarget.textContent || "[]")
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
    const rect = menu.getBoundingClientRect()
    const left = Math.min(x, window.innerWidth - rect.width - VIEWPORT_PADDING)
    const top = Math.min(y, window.innerHeight - rect.height - VIEWPORT_PADDING)
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
}
