import { Controller } from "@hotwired/stimulus"

// First-run tour: five short steps (sidebar, composer, huddles, the Ctrl+K
// switcher, the ? sheet). Steps resolve their anchor at render time;
// anything absent — a non-room page, an unconfigured feature, or UI from
// the not-yet-merged navigation branch — renders the same copy as a
// centered card instead of failing. Skipping or finishing persists
// users.tour_completed_at through users/tours#update so the tour never
// auto-starts again; the help menu restarts it on demand with a tour:start
// window event, without clearing the stamp.
//
// Keyboard: the card is a labelled dialog. Escape skips, Left/Right move
// between steps, and Tab cycles within the card. Focus returns to the
// invoking element on close.
const STEPS = [
  {
    id: "sidebar",
    title: "Your rooms live here",
    body: "The sidebar lists every room you belong to, with unread badges. Star the ones you live in to pin them near the top.",
    selector: "#sidebar"
  },
  {
    id: "composer",
    title: "Write below the messages",
    body: "The composer posts to the open room. Type @ to mention someone, and use the toolbar for files, emoji, and Drive attachments.",
    selector: "#composer"
  },
  {
    id: "huddles",
    title: "Talk it out in a huddle",
    body: "Voice and video huddles start right from the room header, with screen sharing when you need it. The button below joins when a call is live.",
    selector: ".huddle-launcher"
  },
  {
    id: "switcher",
    title: "Jump anywhere with Ctrl+K",
    body: "Press Ctrl+K (or Cmd+K on a Mac) to open the room switcher and hop between conversations without touching the mouse.",
    selector: '[aria-label^="Quick switcher"]',
    fallbackSelector: "#header-overflow-button"
  },
  {
    id: "shortcuts",
    title: "Shortcuts live under ?",
    body: "Press ? anywhere to see every keyboard shortcut. This Help menu holds them too, and restarts this tour whenever you like.",
    selector: "#help-menu-button",
    fallbackSelector: "#header-overflow-button"
  }
]

const CARD_MARGIN = 12

export default class extends Controller {
  static targets = [ "card", "title", "body", "progress", "back", "next", "skip" ]
  static values = { autoStart: Boolean, completeUrl: String }

  connect() {
    if (this.autoStartValue) this.start()
  }

  disconnect() {
    this.#clearHighlight()
  }

  start() {
    this.index = 0
    this.invoker = document.activeElement instanceof HTMLElement ? document.activeElement : null
    this.element.hidden = false
    this.#render()
  }

  next() {
    if (this.index >= STEPS.length - 1) {
      this.finish()
    } else {
      this.index += 1
      this.#render()
    }
  }

  back() {
    if (this.index > 0) {
      this.index -= 1
      this.#render()
    }
  }

  skip() {
    this.#complete()
  }

  finish() {
    this.#complete()
  }

  onKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault()
      this.skip()
    } else if (event.key === "ArrowRight") {
      event.preventDefault()
      this.next()
    } else if (event.key === "ArrowLeft") {
      event.preventDefault()
      this.back()
    } else if (event.key === "Tab") {
      this.#trapTab(event)
    }
  }

  #render() {
    const step = STEPS[this.index]
    const target = this.#visibleTarget(step)
    const anchored = Boolean(target)

    this.#clearHighlight()
    this.titleTarget.textContent = step.title
    this.bodyTarget.textContent = step.body
    this.progressTarget.textContent = `Step ${this.index + 1} of ${STEPS.length}`
    this.backTarget.hidden = this.index === 0
    this.nextTarget.textContent = this.index === STEPS.length - 1 ? "Finish" : "Next"

    if (anchored) {
      target.scrollIntoView({ behavior: this.#scrollBehavior, block: "nearest", inline: "nearest" })
      target.classList.add("tour__target")
      this.highlighted = target
      this.#placeBeside(target.getBoundingClientRect())
    } else {
      this.cardTarget.classList.add("tour__card--center")
      this.cardTarget.style.removeProperty("top")
      this.cardTarget.style.removeProperty("left")
    }

    this.nextTarget.focus({ preventScroll: true })
  }

  #placeBeside(rect) {
    if (window.innerWidth < 640) {
      // Narrow screens use the bottom sheet; skip measuring entirely.
      this.cardTarget.classList.add("tour__card--center")
      this.cardTarget.style.removeProperty("top")
      this.cardTarget.style.removeProperty("left")
      return
    }

    this.cardTarget.classList.remove("tour__card--center")
    const width = Math.min(this.cardTarget.offsetWidth, window.innerWidth - CARD_MARGIN * 2)
    const height = this.cardTarget.offsetHeight

    let left, top
    if (rect.height > window.innerHeight * 0.6) {
      // Tall targets (the sidebar) get the card beside them rather than
      // overlapping their top edge: right when it fits, else left.
      const right = rect.right + CARD_MARGIN
      left = right + width <= window.innerWidth - CARD_MARGIN
        ? right
        : Math.max(rect.left - width - CARD_MARGIN, CARD_MARGIN)
      top = Math.min(Math.max(rect.top, CARD_MARGIN), window.innerHeight - height - CARD_MARGIN)
    } else {
      left = Math.min(Math.max(rect.left, CARD_MARGIN), window.innerWidth - width - CARD_MARGIN)
      const below = rect.bottom + CARD_MARGIN
      top = below + height <= window.innerHeight - CARD_MARGIN
        ? below
        : Math.max(rect.top - height - CARD_MARGIN, CARD_MARGIN)
    }

    this.cardTarget.style.left = `${left}px`
    this.cardTarget.style.top = `${top}px`
  }

  #visibleTarget(step) {
    // Below 80rem the switcher and help buttons live in the header
    // overflow menu, so the tour highlights the overflow button itself.
    for (const selector of [ step.selector, step.fallbackSelector ]) {
      if (!selector) continue
      const target = document.querySelector(selector)
      if (target && this.#isVisible(target)) return target
    }
    return null
  }

  #isVisible(element) {
    const rect = element.getBoundingClientRect()
    return rect.width > 0 && rect.height > 0
  }

  get #scrollBehavior() {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth"
  }

  #clearHighlight() {
    this.highlighted?.classList.remove("tour__target")
    this.highlighted = null
  }

  #trapTab(event) {
    const focusable = [ this.backTarget, this.nextTarget, this.skipTarget ].filter((button) => !button.hidden)
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

  #complete() {
    const url = this.completeUrlValue
    if (url) {
      fetch(url, {
        method: "PATCH",
        headers: {
          Accept: "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || ""
        }
      }).catch(() => {})
    }

    this.element.hidden = true
    this.#clearHighlight()
    if (this.invoker?.isConnected) this.invoker.focus({ preventScroll: true })
  }
}
