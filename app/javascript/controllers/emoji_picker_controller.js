import { Controller } from "@hotwired/stimulus"
import { debounce } from "helpers/timing_helpers"

const GRID_COLUMNS = 8
const VIEWPORT_PADDING = 8
// Boost content is limited to 16 characters server-side; options that cannot
// be stored are left out of the grid.
const MAX_CONTENT_LENGTH = 16

// The shared emoji picker, rendered once per page. It opens from a message
// toolbar button, searches the icon autocomplete endpoint, and submits the
// chosen option as a boost.
export default class extends Controller {
  static targets = [ "panel", "search", "grid", "status", "form", "content" ]
  static values = { iconsUrl: String }

  #message
  #anchor
  #open = false
  #initialGrid
  #searchRequest
  #connected = false

  initialize() {
    this.search = debounce(this.search.bind(this), 200)
  }

  connect() {
    if (!this.hasPanelTarget) return
    this.#connected = true
    this.#initialGrid = this.gridTarget.innerHTML

    this.onOpenRequest = this.#onOpenRequest.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onPanelKeydown = this.#onPanelKeydown.bind(this)
    this.onGridKeydown = this.#onGridKeydown.bind(this)
    this.onPanelFocusOut = this.#onPanelFocusOut.bind(this)

    window.addEventListener("emoji-picker:open", this.onOpenRequest)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    this.panelTarget.addEventListener("keydown", this.onPanelKeydown)
    this.gridTarget.addEventListener("keydown", this.onGridKeydown)
    this.panelTarget.addEventListener("focusout", this.onPanelFocusOut)
  }

  disconnect() {
    this.#connected = false
    this.#searchRequest?.abort()

    window.removeEventListener("emoji-picker:open", this.onOpenRequest)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    this.panelTarget?.removeEventListener("keydown", this.onPanelKeydown)
    this.gridTarget?.removeEventListener("keydown", this.onGridKeydown)
    this.panelTarget?.removeEventListener("focusout", this.onPanelFocusOut)
  }

  search() {
    if (!this.#open) return
    const query = this.searchTarget.value.trim()
    if (!query) {
      this.#searchRequest?.abort()
      this.gridTarget.innerHTML = this.#initialGrid
      this.#setStatus("")
      return
    }
    void this.#loadResults(query)
  }

  choose(event) {
    const option = event.target.closest("button[data-content]")
    if (!option || !this.gridTarget.contains(option)) return
    event.preventDefault()
    this.#submit(option.dataset.content)
  }

  // Internal event handlers

  #onOpenRequest(event) {
    const { message, anchor } = event.detail || {}
    if (!message?.isConnected || !message.dataset.boostUrl) return

    if (this.#open && this.#message === message && this.#anchor === (anchor || null)) {
      this.#close()
      return
    }

    this.#message = message
    this.#anchor = anchor || null
    this.#open = true
    this.searchTarget.value = ""
    this.gridTarget.innerHTML = this.#initialGrid
    this.#setStatus("")
    this.panelTarget.hidden = false
    this.#showPopover()
    this.#position()
    this.searchTarget.focus()
  }

  #onDocumentPointerDown(event) {
    if (!this.#open) return
    if (this.panelTarget.contains(event.target)) return
    if (this.#anchor?.contains(event.target)) return
    this.#close({ restoreFocus: false })
  }

  #onPanelKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault()
      this.#close()
      return
    }

    if (event.target === this.searchTarget && event.key === "ArrowDown") {
      const first = this.#options()[0]
      if (first) {
        event.preventDefault()
        first.focus()
      }
    }
  }

  #onGridKeydown(event) {
    const options = this.#options()
    if (options.length === 0) return
    const currentIndex = options.indexOf(document.activeElement)
    let nextIndex

    switch (event.key) {
      case "ArrowRight":
        nextIndex = Math.min(options.length - 1, currentIndex + 1)
        break
      case "ArrowLeft":
        nextIndex = Math.max(0, currentIndex - 1)
        break
      case "ArrowDown":
        nextIndex = Math.min(options.length - 1, (currentIndex < 0 ? -GRID_COLUMNS : currentIndex) + GRID_COLUMNS)
        break
      case "ArrowUp":
        if (currentIndex >= 0 && currentIndex < GRID_COLUMNS) {
          event.preventDefault()
          this.searchTarget.focus()
          return
        }
        nextIndex = Math.max(0, currentIndex - GRID_COLUMNS)
        break
      case "Home":
        nextIndex = 0
        break
      case "End":
        nextIndex = options.length - 1
        break
      default:
        return
    }

    event.preventDefault()
    options[nextIndex]?.focus()
  }

  #onPanelFocusOut(event) {
    if (!this.#open) return
    if (event.relatedTarget && this.panelTarget.contains(event.relatedTarget)) return
    this.#close({ restoreFocus: false })
  }

  // Picker lifecycle

  async #loadResults(query) {
    this.#searchRequest?.abort()
    const request = new AbortController()
    this.#searchRequest = request

    try {
      const response = await fetch(`${this.iconsUrlValue}?q=${encodeURIComponent(query)}`, {
        headers: { Accept: "application/json" },
        signal: request.signal,
      })
      if (!response.ok || !this.#connected || !this.#open) return

      const icons = await response.json()
      if (!this.#connected || !this.#open || this.#searchRequest !== request) return
      this.#renderResults(Array.isArray(icons) ? icons : [])
    } catch (error) {
      if (error.name === "AbortError") return
      if (this.#connected && this.#open) this.#setStatus("Search is unavailable right now.")
    } finally {
      if (this.#searchRequest === request) this.#searchRequest = null
    }
  }

  #renderResults(icons) {
    const nodes = []
    for (const icon of icons) {
      const content = icon.character || (icon.name ? `:${icon.name}:` : null)
      if (!content || content.length > MAX_CONTENT_LENGTH) continue

      const option = document.createElement("button")
      option.type = "button"
      option.className = "emoji-picker__option"
      option.dataset.content = content
      option.setAttribute("aria-label", icon.title || icon.name || content)

      if (icon.image) {
        const image = document.createElement("img")
        image.src = icon.image
        image.alt = ""
        image.setAttribute("aria-hidden", "true")
        image.width = 20
        image.height = 20
        option.append(image)
      } else {
        const glyph = document.createElement("span")
        glyph.setAttribute("aria-hidden", "true")
        glyph.textContent = icon.character
        option.append(glyph)
      }
      nodes.push(option)
    }

    this.gridTarget.replaceChildren(...nodes)
    this.#setStatus(nodes.length === 0 ? "No matches." : "")
  }

  #submit(content) {
    if (!content || !this.#message?.isConnected) {
      this.#close({ restoreFocus: false })
      return
    }
    this.formTarget.action = this.#message.dataset.boostUrl
    this.formTarget.setAttribute("data-turbo-frame", `boosting_${this.#message.id}`)
    this.contentTarget.value = content
    this.#close({ restoreFocus: false })
    this.formTarget.requestSubmit()
    this.#anchor?.focus?.({ preventScroll: true })
  }

  #close({ restoreFocus = true } = {}) {
    if (!this.#open && this.panelTarget?.hidden) return

    this.#open = false
    this.#searchRequest?.abort()
    this.#searchRequest = null
    if (this.panelTarget) {
      this.#hidePopover()
      this.panelTarget.hidden = true
    }

    if (restoreFocus) this.#anchor?.focus?.({ preventScroll: true })
    this.#anchor = null
  }

  #position() {
    if (!this.#open || !this.panelTarget || this.panelTarget.hidden) return
    if (!this.#anchor?.isConnected) {
      this.panelTarget.style.left = ""
      this.panelTarget.style.top = ""
      return
    }

    const anchor = this.#anchor.getBoundingClientRect()
    const rect = this.panelTarget.getBoundingClientRect()
    const viewportHeight = window.visualViewport?.height || window.innerHeight
    const maxX = Math.max(VIEWPORT_PADDING, window.innerWidth - rect.width - VIEWPORT_PADDING)
    const maxY = Math.max(VIEWPORT_PADDING, viewportHeight - rect.height - VIEWPORT_PADDING)

    const x = Math.min(Math.max(VIEWPORT_PADDING, anchor.left), maxX)
    const below = anchor.bottom + 4
    const y = below + rect.height <= viewportHeight - VIEWPORT_PADDING
      ? below
      : Math.min(Math.max(VIEWPORT_PADDING, anchor.top - rect.height - 4), maxY)

    this.panelTarget.style.left = `${x}px`
    this.panelTarget.style.top = `${y}px`
  }

  #showPopover() {
    if (this.panelTarget.getAttribute("popover") === null || typeof this.panelTarget.showPopover !== "function") return
    try {
      this.panelTarget.showPopover()
    } catch {
      // The fixed-position fallback remains usable without popover support.
    }
  }

  #hidePopover() {
    if (typeof this.panelTarget.hidePopover !== "function") return
    try {
      if (this.panelTarget.matches(":popover-open")) this.panelTarget.hidePopover()
    } catch {
      // The hidden attribute still closes the fixed-position fallback.
    }
  }

  #options() {
    return Array.from(this.gridTarget.querySelectorAll("button[data-content]"))
  }

  #setStatus(message) {
    if (this.hasStatusTarget) this.statusTarget.textContent = message
  }
}
