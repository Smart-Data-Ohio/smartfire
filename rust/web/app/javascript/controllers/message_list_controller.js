import { Controller } from "@hotwired/stimulus"

const MESSAGE_SELECTOR = ".message"
const LONG_PRESS_DELAY = 550
const MOVE_THRESHOLD = 10
// A touch release is followed by compatibility mouse events at the same
// point. When the release opens the menu under the finger (the phone
// bottom sheet covers the message), that click would activate the menu
// item below it, so clicks near the press point are swallowed briefly.
const SUPPRESS_CLICK_MS = 500
const SUPPRESS_CLICK_RADIUS = MOVE_THRESHOLD * 2

// One controller per message list. It owns the roving tabindex (the list is
// a single Tab stop; Up/Down move between messages) and one delegated set of
// input listeners that open the shared message menu. This replaces the
// per-message tabindex and the per-message window/document listeners.
export default class extends Controller {
  #tabbable
  #focusInside = false
  #menuId
  #longPressTimer
  #longPressPointer
  #longPressTriggered = false
  #suppressClickAt
  #suppressClickUntil = 0
  #observer

  connect() {
    this.onContextMenu = this.#onContextMenu.bind(this)
    this.onKeydown = this.#onKeydown.bind(this)
    this.onFocusIn = this.#onFocusIn.bind(this)
    this.onFocusOut = this.#onFocusOut.bind(this)
    this.onPointerDown = this.#onPointerDown.bind(this)
    this.onPointerMove = this.#onPointerMove.bind(this)
    this.onPointerUp = this.#onPointerUp.bind(this)
    this.onSuppressClick = this.#onSuppressClick.bind(this)
    this.onCancelLongPress = this.#cancelLongPress.bind(this)

    this.element.addEventListener("contextmenu", this.onContextMenu)
    this.element.addEventListener("keydown", this.onKeydown)
    this.element.addEventListener("focusin", this.onFocusIn)
    this.element.addEventListener("focusout", this.onFocusOut)
    this.element.addEventListener("pointerdown", this.onPointerDown)
    this.element.addEventListener("pointermove", this.onPointerMove)
    this.element.addEventListener("pointerup", this.onPointerUp)
    this.element.addEventListener("pointercancel", this.onPointerUp)
    // Capture phase, before the menu's own click handlers: the swallowed
    // click must never reach the menu item below the finger.
    document.addEventListener("click", this.onSuppressClick, true)
    window.addEventListener("scroll", this.onCancelLongPress, true)

    this.#initTabindex()
    this.#observer = new MutationObserver(this.#onMutations.bind(this))
    this.#observer.observe(this.element, { childList: true, subtree: true })
  }

  disconnect() {
    this.#observer?.disconnect()
    this.#observer = null
    this.#cancelLongPress()

    this.element.removeEventListener("contextmenu", this.onContextMenu)
    this.element.removeEventListener("keydown", this.onKeydown)
    this.element.removeEventListener("focusin", this.onFocusIn)
    this.element.removeEventListener("focusout", this.onFocusOut)
    this.element.removeEventListener("pointerdown", this.onPointerDown)
    this.element.removeEventListener("pointermove", this.onPointerMove)
    this.element.removeEventListener("pointerup", this.onPointerUp)
    this.element.removeEventListener("pointercancel", this.onPointerUp)
    document.removeEventListener("click", this.onSuppressClick, true)
    window.removeEventListener("scroll", this.onCancelLongPress, true)
  }

  // Roving tabindex: exactly one message is a Tab stop, starting on the
  // newest message nearest the composer. Focus (Tab or arrow keys) moves
  // the stop; streamed and paginated messages join as -1.

  #initTabindex() {
    const messages = this.#messages()
    messages.forEach((message, index) => {
      message.tabIndex = index === messages.length - 1 ? 0 : -1
      this.#annotate(message)
    })
    this.#tabbable = messages[messages.length - 1] || null
  }

  #onMutations(records) {
    let changed = false
    const addedMessages = []
    const removedIds = new Set()
    let tabbableRemoved = false
    let tabbableNeighbour = null

    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType !== Node.ELEMENT_NODE) continue
        const added = node.matches(MESSAGE_SELECTOR) ? [ node ] : Array.from(node.querySelectorAll(MESSAGE_SELECTOR))
        for (const message of added) {
          message.tabIndex = -1
          this.#annotate(message)
          addedMessages.push(message)
          changed = true
        }
      }
      for (const node of record.removedNodes) {
        if (node.nodeType !== Node.ELEMENT_NODE) continue
        const removed = node.matches(MESSAGE_SELECTOR) ? [ node ] : Array.from(node.querySelectorAll(MESSAGE_SELECTOR))
        for (const message of removed) {
          removedIds.add(message.id)
          if (message === this.#tabbable) {
            tabbableRemoved = true
            tabbableNeighbour = this.#neighbourMessage(record.nextSibling, "next") || this.#neighbourMessage(record.previousSibling, "previous")
          }
          changed = true
        }
      }
    }

    // A stream replace swaps the node out from under focus: move the tab
    // stop to the replacement with the same id, and focus it when removal
    // dropped focus back to the page. (Turbo streams usually preserve focus
    // themselves; this also covers direct DOM swaps.)
    let refocused = false
    let replaced = false
    if (tabbableRemoved) {
      const replacement = addedMessages.find(message => removedIds.has(message.id))
      if (replacement) {
        replaced = true
        replacement.tabIndex = 0
        this.#tabbable = replacement
        if (this.#focusInside && document.activeElement === document.body) {
          replacement.focus()
          refocused = true
        }
      }
    }

    // A deleted tab stop hands off to the message beside it, not the newest,
    // so someone reading history isn't yanked to the bottom.
    if (tabbableRemoved && !replaced && tabbableNeighbour?.isConnected && this.element.contains(tabbableNeighbour)) {
      tabbableNeighbour.tabIndex = 0
      this.#tabbable = tabbableNeighbour
    }

    if (changed) this.#ensureTabbable()

    // A removal without a same-id replacement is a delete: focus fell to
    // the page with the tab stop, so the next arrow key would be lost.
    // Follow the surviving tab stop when the deleted message had focus.
    if (!refocused && this.#focusInside && tabbableRemoved &&
        document.activeElement === document.body && this.#tabbable?.isConnected) {
      this.#tabbable.focus({ preventScroll: true })
    }
  }

  #neighbourMessage(node, direction) {
    const step = direction === "next" ? "nextElementSibling" : "previousElementSibling"
    let current = node?.nodeType === Node.ELEMENT_NODE ? node : node?.[direction === "next" ? "nextSibling" : "previousSibling"]
    while (current && current.nodeType !== Node.ELEMENT_NODE) current = current[direction === "next" ? "nextSibling" : "previousSibling"]
    while (current) {
      const message = current.matches(MESSAGE_SELECTOR) ? current : current.querySelector?.(MESSAGE_SELECTOR)
      if (message) return message
      current = current[step]
    }
    return null
  }

  #ensureTabbable() {
    if (this.#tabbable?.isConnected && this.element.contains(this.#tabbable)) return

    const messages = this.#messages()
    const newest = messages[messages.length - 1]
    if (newest) {
      newest.tabIndex = 0
      this.#tabbable = newest
    } else {
      this.#tabbable = null
    }
  }

  #onFocusIn(event) {
    const message = event.target.closest?.(MESSAGE_SELECTOR)
    if (!message || !this.element.contains(message)) return
    this.#focusInside = true
    if (message === this.#tabbable) return

    if (this.#tabbable?.isConnected) this.#tabbable.tabIndex = -1
    message.tabIndex = 0
    this.#tabbable = message
  }

  #onFocusOut(event) {
    // A focusout to nowhere (relatedTarget null) is removal fallout when a
    // message is deleted, so it must not clear the flag the observer reads.
    // Clicking empty space shares the signature and leaves the flag stale,
    // but then a later delete only moves the already-moving tab stop.
    if (!event.relatedTarget) return
    if (this.element.contains(event.relatedTarget)) return
    this.#focusInside = false
  }

  #annotate(message) {
    if (!message.dataset.actionsUrl) return
    message.setAttribute("aria-haspopup", "menu")
    this.#menuId ??= document.querySelector("[data-message-actions-target='menu']")?.id
    if (this.#menuId) message.setAttribute("aria-controls", this.#menuId)
    if (message.getAttribute("aria-expanded") !== "true") message.setAttribute("aria-expanded", "false")
  }

  // Delegated input. Right-click, the ContextMenu key / Shift+F10, and a
  // touch long-press open the shared menu; Up/Down move between messages.

  #onContextMenu(event) {
    const message = event.target.closest?.(MESSAGE_SELECTOR)
    if (!message || !this.element.contains(message) || this.#isInteractive(event.target)) return
    event.preventDefault()
    if (this.#longPressPointer || !message.dataset.actionsUrl) return
    this.#openMenuFor(message, { x: event.clientX, y: event.clientY })
  }

  #onKeydown(event) {
    const message = event.target.closest?.(MESSAGE_SELECTOR)
    if (!message || !this.element.contains(message)) return

    if (event.target.matches(MESSAGE_SELECTOR)) {
      const messages = this.#messages()
      const index = messages.indexOf(message)
      let next

      switch (event.key) {
        case "ArrowDown":
          next = messages[index + 1]
          break
        case "ArrowUp":
          next = messages[index - 1]
          break
        case "Home":
          next = messages[0]
          break
        case "End":
          next = messages[messages.length - 1]
          break
      }

      if (next) {
        event.preventDefault()
        next.focus()
        return
      }
      if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Home" || event.key === "End") {
        event.preventDefault()
        return
      }
    }

    const contextMenuKey = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)
    if (!contextMenuKey || this.#isInteractive(event.target) || !message.dataset.actionsUrl) return

    event.preventDefault()
    const rect = message.getBoundingClientRect()
    this.#openMenuFor(message, { x: rect.left + Math.min(rect.width / 2, 240), y: rect.bottom })
  }

  #onPointerDown(event) {
    if (event.pointerType !== "touch" || event.button !== 0 || this.#isInteractive(event.target)) return
    const message = event.target.closest?.(MESSAGE_SELECTOR)
    if (!message || !this.element.contains(message) || !message.dataset.actionsUrl) return

    this.#cancelLongPress()
    this.#longPressPointer = { id: event.pointerId, x: event.clientX, y: event.clientY, message }
    this.#longPressTimer = setTimeout(() => {
      if (!this.#longPressPointer) return
      this.#longPressTriggered = true
    }, LONG_PRESS_DELAY)
  }

  #onPointerMove(event) {
    if (!this.#longPressPointer || event.pointerId !== this.#longPressPointer.id) return

    const distance = Math.hypot(event.clientX - this.#longPressPointer.x, event.clientY - this.#longPressPointer.y)
    if (distance > MOVE_THRESHOLD) this.#cancelLongPress()
  }

  #onPointerUp(event) {
    if (event.type === "pointercancel") {
      this.#cancelLongPress()
      return
    }
    if (this.#longPressPointer && event.pointerId === this.#longPressPointer.id && this.#longPressTriggered) {
      const { x, y, message } = this.#longPressPointer
      this.#cancelLongPress()
      if (!message.isConnected) return
      this.#suppressClickAt = { x, y }
      this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS
      setTimeout(() => this.#openMenuFor(message, { x, y }), 0)
      return
    }
    if (!this.#longPressPointer || event.pointerId === this.#longPressPointer.id) this.#cancelLongPress()
  }

  // Swallows the compatibility click from a long-press release: any click
  // within a short window near the press point. Anything later or farther
  // away is a genuine tap and passes through.
  #onSuppressClick(event) {
    if (!this.#suppressClickAt || Date.now() > this.#suppressClickUntil) {
      this.#suppressClickAt = null
      return
    }
    const distance = Math.hypot(event.clientX - this.#suppressClickAt.x, event.clientY - this.#suppressClickAt.y)
    if (distance > SUPPRESS_CLICK_RADIUS) return
    event.preventDefault()
    event.stopPropagation()
  }

  #openMenuFor(message, point) {
    window.dispatchEvent(new CustomEvent("message-actions:open", {
      detail: { message, ...point },
    }))
  }

  #messages() {
    // Descendants, not just children: the standalone thread page nests its
    // starter and messages in article/section wrappers. The closest guard
    // keeps nested lists (if any) from managing each other's messages.
    return Array.from(this.element.querySelectorAll(MESSAGE_SELECTOR))
      .filter(element => element.closest("[data-controller~='message-list']") === this.element)
  }

  #isInteractive(target) {
    return Boolean(target?.closest?.("a, button, input, textarea, select, option, [contenteditable='true'], [data-no-message-menu]"))
  }

  #cancelLongPress() {
    clearTimeout(this.#longPressTimer)
    this.#longPressTimer = null
    this.#longPressPointer = null
    this.#longPressTriggered = false
  }
}
