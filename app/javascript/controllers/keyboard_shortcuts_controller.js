import { Controller } from "@hotwired/stimulus"

// Global keyboard shortcuts, owned by the page body. Unchorded shortcuts
// never fire while typing; Alt+arrows don't either (macOS Option+arrows
// move by paragraph in text). Ctrl/⌘+K fires from inputs so it works
// from the composer, except while composing (IME), in the huddle
// device menus (which keep Ctrl+K for the browser), or over another
// open modal. Escape is last in line behind every dialog, menu and
// panel: it marks the current room read only when nothing else is open.
// "/" (unchorded) and Ctrl/⌘+Shift+F (from anywhere, inputs included)
// focus the top-bar search field.
export default class extends Controller {
  static targets = [ "helpDialog" ]

  connect() {
    this.onCaptureKeydown = this.#onCaptureKeydown.bind(this)
    window.addEventListener("keydown", this.onCaptureKeydown, true)
  }

  disconnect() {
    window.removeEventListener("keydown", this.onCaptureKeydown, true)
  }

  // Ctrl/⌘+K runs in the capture phase so it preempts the composer's own
  // Ctrl+K (insert link): the switcher wins everywhere by design, except
  // while composing, in huddle device menus, or over another modal.
  // Toggling the switcher itself closed still works over a modal.
  #onCaptureKeydown(event) {
    if (!event.ctrlKey && !event.metaKey) return
    if (event.altKey) return
    if (event.shiftKey && event.key.toLowerCase() === "f") {
      this.#focusSearchFromChord(event)
      return
    }
    if (event.key.toLowerCase() !== "k") return
    if (event.isComposing) return
    if (event.target.closest?.(".huddle__devices")) return
    if (!document.getElementById("quick-switcher")?.open && document.querySelector("dialog[open]")) return

    event.preventDefault()
    event.stopPropagation()
    window.dispatchEvent(new CustomEvent("switcher:toggle"))
  }

  handle(event) {
    if (event.defaultPrevented) return

    if (this.#typing(event)) return

    if (this.#overlayOpen()) return

    if (event.altKey && !event.ctrlKey && !event.metaKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
      this.#moveRoom(event)
      return
    }

    if (event.key === "Escape" && !event.ctrlKey && !event.metaKey && !event.altKey) {
      this.#markCurrentRoomRead(event)
      return
    }

    if (event.key === "?" && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault()
      this.openHelp()
      return
    }

    if (event.key === "/" && !event.ctrlKey && !event.metaKey && !event.altKey && document.getElementById("global-search")) {
      event.preventDefault()
      this.focusSearch()
    }
  }

  focusSearch() {
    window.dispatchEvent(new CustomEvent("global-search:focus"))
  }

  openSwitcher() {
    window.dispatchEvent(new CustomEvent("switcher:open"))
  }

  openHelp() {
    if (this.hasHelpDialogTarget && !this.helpDialogTarget.open) this.helpDialogTarget.showModal()
  }

  dismissBackdrop(event) {
    if (event.target === this.helpDialogTarget) this.helpDialogTarget.close()
  }

  // Internal

  // Ctrl/⌘+Shift+F works from inputs and the composer, like Ctrl/⌘+K,
  // but never over a modal or while composing.
  #focusSearchFromChord(event) {
    if (event.isComposing) return
    if (!document.getElementById("global-search")) return
    if (this.#overlayOpen()) return

    event.preventDefault()
    event.stopPropagation()
    this.focusSearch()
  }

  #moveRoom(event) {
    const links = Array.from(document.querySelectorAll("#sidebar a[data-room-id]"))
    if (links.length === 0) return

    let pool = links
    if (event.shiftKey) {
      pool = links.filter(link => link.classList.contains("unread"))
      if (pool.length === 0) return
    }

    const currentId = document.querySelector("meta[name='current-room-id']")?.content
    const backward = event.key === "ArrowUp"
    let index = pool.findIndex(link => link.dataset.roomId === currentId)
    if (index === -1) index = backward ? 0 : pool.length - 1

    const next = pool[(index + (backward ? -1 : 1) + pool.length) % pool.length]
    if (!next || next.dataset.roomId === currentId) return
    event.preventDefault()
    next.click()
  }

  async #markCurrentRoomRead(event) {
    const roomId = document.querySelector("meta[name='current-room-id']")?.content
    if (!roomId) return
    event.preventDefault()

    // The reads broadcast updates every session, including this one.
    await fetch(`/rooms/${roomId}/read`, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
    }).catch(() => null)
  }

  #typing(event) {
    if (event.isComposing) return true
    return Boolean(event.target.closest?.("input, textarea, select, trix-editor, [contenteditable='true']"))
  }

  #overlayOpen() {
    // Panels mark themselves aria-modal only when modal (the member
    // panel is a persistent side panel on desktop, where Esc must
    // still mark the room read). Theater mode and real full screen
    // are element state rather than overlays, so they need their own
    // checks: Esc there belongs to the theater collapse and to the
    // browser, and this handler runs before the huddle's own Escape
    // listener, so marking read here would swallow the collapse.
    if (document.fullscreenElement || document.webkitFullscreenElement) return true
    if (document.querySelector("#channel-huddle.huddle--theater")) return true
    if (document.querySelector("dialog[open], :popover-open")) return true
    // A hidden or inert container keeps its dialog markup (the profile
    // card's panel is always aria-modal) without covering the page.
    return Array.from(document.querySelectorAll("[aria-modal='true']"))
      .some(element => !element.closest("[hidden], [inert]"))
  }
}
