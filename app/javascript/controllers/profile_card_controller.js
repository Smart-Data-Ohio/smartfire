import { Controller } from "@hotwired/stimulus"

const SHEET_QUERY = "(max-width: 40rem)"
const PANEL_GAP = 8
const PANEL_WIDTH = 340
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])"
].join(",")

export default class extends Controller {
  static targets = [ "popover", "panel", "frame" ]

  connect() {
    this.sheetQuery = window.matchMedia(SHEET_QUERY)
    this.hideBeforeVisit = () => this.hide()
    document.addEventListener("turbo:before-visit", this.hideBeforeVisit)
  }

  disconnect() {
    document.removeEventListener("turbo:before-visit", this.hideBeforeVisit)
  }

  open(event) {
    if (event.type === "keydown" && event.key !== "Enter" && event.key !== " ") return

    const trigger = event.currentTarget
    const url = trigger?.dataset.profileCardUrl
    if (!url || !this.hasPopoverTarget) return

    event.preventDefault()
    // Avatar triggers sit inside room links and message links: the card
    // opens instead of navigating.
    event.stopPropagation()

    this.trigger = trigger
    this.#position(trigger)
    this.popoverTarget.hidden = false

    if (this.frameTarget.getAttribute("src") !== url) {
      this.frameTarget.innerHTML = `<div class="profile-card__loading" aria-live="polite">Loading…</div>`
      this.frameTarget.setAttribute("src", url)
    }
    requestAnimationFrame(() => this.panelTarget.focus({ preventScroll: true }))
  }

  close(event) {
    event?.preventDefault?.()
    if (!this.hasPopoverTarget || this.popoverTarget.hidden) return

    const trigger = this.trigger
    this.hide()
    if (trigger?.isConnected) trigger.focus({ preventScroll: true })
  }

  hide() {
    if (!this.hasPopoverTarget || this.popoverTarget.hidden) return

    this.popoverTarget.hidden = true
    this.frameTarget.removeAttribute("src")
    this.trigger = null
  }

  trapFocus(event) {
    if (event.key !== "Tab" || !this.hasPopoverTarget || this.popoverTarget.hidden) return

    const focusable = Array.from(this.panelTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter((element) => {
      return !element.hidden && element.tabIndex >= 0 && element.getClientRects().length > 0
    })
    if (focusable.length === 0) {
      event.preventDefault()
      this.panelTarget.focus()
      return
    }

    const first = focusable[0]
    const last = focusable[focusable.length - 1]

    if (!this.panelTarget.contains(document.activeElement)) {
      event.preventDefault()
      ;(event.shiftKey ? last : first).focus()
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }

  #position(trigger) {
    if (this.sheetQuery.matches) {
      this.panelTarget.style.removeProperty("top")
      this.panelTarget.style.removeProperty("left")
      return
    }

    const rect = trigger.getBoundingClientRect()
    const panelHeight = this.panelTarget.offsetHeight || 320

    let left = Math.min(rect.left, window.innerWidth - PANEL_WIDTH - PANEL_GAP)
    left = Math.max(left, PANEL_GAP)

    let top = rect.bottom + PANEL_GAP
    if (top + panelHeight > window.innerHeight - PANEL_GAP) {
      top = Math.max(rect.top - panelHeight - PANEL_GAP, PANEL_GAP)
    }

    this.panelTarget.style.top = `${top}px`
    this.panelTarget.style.left = `${left}px`
  }
}
