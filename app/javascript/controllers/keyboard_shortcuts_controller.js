import { Controller } from "@hotwired/stimulus"

// Global keyboard shortcuts, owned by the page body. Unchorded shortcuts
// never fire while typing; chorded ones (Ctrl/Alt) fire anywhere except
// the huddle device menus, which keep Ctrl+K for the browser. Escape is
// last in line behind every dialog, menu and panel: it marks the current
// room read only when nothing else is open.
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
  // Ctrl+K (insert link): the switcher wins everywhere by design.
  #onCaptureKeydown(event) {
    if (!event.ctrlKey && !event.metaKey) return
    if (event.altKey || event.key.toLowerCase() !== "k") return
    if (event.target.closest?.(".huddle__devices")) return

    event.preventDefault()
    event.stopPropagation()
    if (this.hasHelpDialogTarget && this.helpDialogTarget.open) this.helpDialogTarget.close()
    window.dispatchEvent(new CustomEvent("switcher:toggle"))
  }

  handle(event) {
    if (event.defaultPrevented) return

    if (this.#typing(event)) {
      if (event.altKey && !event.ctrlKey && !event.metaKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        this.#moveRoom(event)
      }
      return
    }

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
    }
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
    if (document.querySelector("dialog[open], :popover-open")) return true
    return Boolean(document.querySelector("#channel-members[aria-hidden='false'], #thread-panel[aria-hidden='false']"))
  }
}
