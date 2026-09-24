import { Controller } from "@hotwired/stimulus"

const FOCUSABLE_SELECTOR = "button:not([disabled])"
const VIEWPORT_PADDING = 8

// The composer's + button. When Google Drive is available to this user
// the button opens a small menu — From this device / From Google
// Drive — and without Drive it opens the device file picker directly,
// with no menu of one item. "From this device" clicks the hidden file
// input so the existing composer#filePicked preview path runs
// unchanged; "From Google Drive" forwards to whichever Drive
// controller the composer rendered (drive-share or the legacy
// drive-picker), exactly as the old standalone Drive button did.
// Follows the header-overflow menu pattern: popover top layer,
// arrow-key navigation, Escape and outside tap to close.
export default class extends Controller {
  static targets = [ "button", "menu", "fileInput" ]

  #open = false

  connect() {
    this.onMenuKeydown = this.#onMenuKeydown.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onWindowKeydown = this.#onWindowKeydown.bind(this)
    this.onBeforeCache = this.#onBeforeCache.bind(this)
    this.onResize = this.#onResize.bind(this)

    if (this.hasMenuTarget) this.menuTarget.addEventListener("keydown", this.onMenuKeydown)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    window.addEventListener("keydown", this.onWindowKeydown)
    document.addEventListener("turbo:before-cache", this.onBeforeCache)
    window.addEventListener("resize", this.onResize)
  }

  disconnect() {
    if (this.hasMenuTarget) this.menuTarget.removeEventListener("keydown", this.onMenuKeydown)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    window.removeEventListener("keydown", this.onWindowKeydown)
    document.removeEventListener("turbo:before-cache", this.onBeforeCache)
    window.removeEventListener("resize", this.onResize)
  }

  toggle(event) {
    event?.preventDefault()
    // No menu rendered: one tap opens the device picker directly.
    if (!this.hasMenuTarget) {
      this.fileInputTarget.click()
      return
    }
    if (this.#open) {
      this.#closeMenu()
    } else {
      this.#openMenu()
    }
  }

  buttonKeydown(event) {
    if (!this.hasMenuTarget) return
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault()
      this.#openMenu(event.key === "ArrowUp" ? "last" : "first")
    }
  }

  chooseDevice(event) {
    event.preventDefault()
    this.#closeMenu()
    // Still inside the menu item's user gesture, so the dialog opens.
    this.fileInputTarget.click()
  }

  chooseDrive(event) {
    event.preventDefault()
    // The Drive flows arm document-level outside-click dismissal as
    // they open; this click is still bubbling there, and its target
    // (this menu item) sits outside their elements, so stop it here
    // or it instantly closes what it just opened.
    event.stopPropagation()
    this.#closeMenu()
    // The composer renders exactly one Drive controller element; its
    // lazy module connects on page load, so it is ready by click time.
    const form = this.element.closest("form")
    const shareElement = form?.querySelector('[data-controller="drive-share"]')
    if (shareElement) {
      this.application.getControllerForElementAndIdentifier(shareElement, "drive-share")?.open(event)
      return
    }
    const pickerElement = form?.querySelector('[data-controller="drive-picker"]')
    if (pickerElement) {
      this.application.getControllerForElementAndIdentifier(pickerElement, "drive-picker")?.toggle(event)
    }
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
  }

  #onDocumentPointerDown(event) {
    if (!this.#open || !this.hasMenuTarget) return
    if (this.menuTarget.contains(event.target)) return
    if (this.buttonTarget.contains(event.target)) return
    this.#closeMenu({ restoreFocus: false })
  }

  #onWindowKeydown(event) {
    if (this.#open && event.key === "Escape" && this.hasMenuTarget && !this.menuTarget.contains(event.target)) {
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
    if (!this.hasMenuTarget) return
    this.#open = true
    this.buttonTarget.setAttribute("aria-expanded", "true")
    this.menuTarget.hidden = false
    this.#showMenuPopover()
    this.#positionMenu()

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
    if (!this.hasMenuTarget) return
    if (!this.#open && this.menuTarget.hidden) return
    this.#open = false

    this.#hideMenuPopover()
    this.menuTarget.hidden = true
    this.buttonTarget?.setAttribute("aria-expanded", "false")
    if (restoreFocus && this.buttonTarget?.isConnected) {
      this.buttonTarget.focus({ preventScroll: true })
    }
  }

  // Above the + button, since the keyboard may be open below it;
  // clamped into the viewport on narrow phones.
  #positionMenu() {
    if (!this.#open || !this.hasMenuTarget || this.menuTarget.hidden) return

    const buttonRect = this.buttonTarget.getBoundingClientRect()
    // offsetWidth/Height ignore the enter scale: getBoundingClientRect
    // would measure the menu mid-pop at 0.97 and clamp it past the edge.
    const menuWidth = this.menuTarget.offsetWidth
    const menuHeight = this.menuTarget.offsetHeight
    const rtl = document.dir === "rtl" || getComputedStyle(document.documentElement).direction === "rtl"

    const top = buttonRect.top - menuHeight - VIEWPORT_PADDING
    const maxTop = Math.max(VIEWPORT_PADDING, window.innerHeight - menuHeight - VIEWPORT_PADDING)
    const startAligned = rtl ? buttonRect.right - menuWidth : buttonRect.left
    const maxLeft = Math.max(VIEWPORT_PADDING, window.innerWidth - menuWidth - VIEWPORT_PADDING)

    this.menuTarget.style.top = `${Math.min(Math.max(VIEWPORT_PADDING, top), maxTop)}px`
    this.menuTarget.style.left = `${Math.min(Math.max(VIEWPORT_PADDING, startAligned), maxLeft)}px`
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
    if (!this.hasMenuTarget) return []
    return Array.from(this.menuTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter(item => {
      if (item.hidden || item.closest("[hidden]")) return false
      return this.#isVisible(item)
    })
  }

  #isVisible(item) {
    if (item.getClientRects().length > 0) return true
    return window.getComputedStyle(item).display !== "none"
  }
}
