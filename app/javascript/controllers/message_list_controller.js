import { Controller } from "@hotwired/stimulus"

const MESSAGE_SELECTOR = ".message"
const LONG_PRESS_DELAY = 550
const MOVE_THRESHOLD = 10

// One controller per message list. It owns the roving tabindex (the list is
// a single Tab stop; Up/Down move between messages) and one delegated set of
// input listeners that open the shared message menu. This replaces the
// per-message tabindex and the per-message window/document listeners.
export default class extends Controller {
  #tabbable
  #menuId
  #longPressTimer
  #longPressPointer
  #longPressTriggered = false
  #observer

  connect() {
    this.onContextMenu = this.#onContextMenu.bind(this)
    this.onKeydown = this.#onKeydown.bind(this)
    this.onFocusIn = this.#onFocusIn.bind(this)
    this.onPointerDown = this.#onPointerDown.bind(this)
    this.onPointerMove = this.#onPointerMove.bind(this)
    this.onPointerUp = this.#onPointerUp.bind(this)
    this.onCancelLongPress = this.#cancelLongPress.bind(this)

    this.element.addEventListener("contextmenu", this.onContextMenu)
    this.element.addEventListener("keydown", this.onKeydown)
    this.element.addEventListener("focusin", this.onFocusIn)
    this.element.addEventListener("pointerdown", this.onPointerDown)
    this.element.addEventListener("pointermove", this.onPointerMove)
    this.element.addEventListener("pointerup", this.onPointerUp)
    this.element.addEventListener("pointercancel", this.onPointerUp)
    window.addEventListener("scroll", this.onCancelLongPress, true)

    this.#initTabindex()
    this.#observer = new MutationObserver(this.#onMutations.bind(this))
    this.#observer.observe(this.element, { childList: true })
  }

  disconnect() {
    this.#observer?.disconnect()
    this.#observer = null
    this.#cancelLongPress()

    this.element.removeEventListener("contextmenu", this.onContextMenu)
    this.element.removeEventListener("keydown", this.onKeydown)
    this.element.removeEventListener("focusin", this.onFocusIn)
    this.element.removeEventListener("pointerdown", this.onPointerDown)
    this.element.removeEventListener("pointermove", this.onPointerMove)
    this.element.removeEventListener("pointerup", this.onPointerUp)
    this.element.removeEventListener("pointercancel", this.onPointerUp)
    window.removeEventListener("scroll", this.onCancelLongPress, true)
  }

  // Roving tabindex: exactly one message is a Tab stop. Focus (Tab or arrow
  // keys) moves the stop; streamed and paginated messages join as -1.

  #initTabindex() {
    const messages = this.#messages()
    messages.forEach((message, index) => {
      message.tabIndex = index === 0 ? 0 : -1
      this.#annotate(message)
    })
    this.#tabbable = messages[0] || null
  }

  #onMutations(records) {
    let changed = false

    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType !== Node.ELEMENT_NODE) continue
        const added = node.matches(MESSAGE_SELECTOR) ? [ node ] : Array.from(node.querySelectorAll(MESSAGE_SELECTOR))
        for (const message of added) {
          message.tabIndex = -1
          this.#annotate(message)
          changed = true
        }
      }
      if (record.removedNodes.length > 0) changed = true
    }

    if (changed) this.#ensureTabbable()
  }

  #ensureTabbable() {
    if (this.#tabbable?.isConnected && this.element.contains(this.#tabbable)) return

    const first = this.#messages()[0]
    if (first) {
      first.tabIndex = 0
      this.#tabbable = first
    } else {
      this.#tabbable = null
    }
  }

  #onFocusIn(event) {
    const message = event.target.closest?.(MESSAGE_SELECTOR)
    if (!message || !this.element.contains(message) || message === this.#tabbable) return

    if (this.#tabbable?.isConnected) this.#tabbable.tabIndex = -1
    message.tabIndex = 0
    this.#tabbable = message
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
      if (message.isConnected) setTimeout(() => this.#openMenuFor(message, { x, y }), 0)
      return
    }
    if (!this.#longPressPointer || event.pointerId === this.#longPressPointer.id) this.#cancelLongPress()
  }

  #openMenuFor(message, point) {
    window.dispatchEvent(new CustomEvent("message-actions:open", {
      detail: { message, ...point },
    }))
  }

  #messages() {
    return Array.from(this.element.children).filter(element => element.matches(MESSAGE_SELECTOR))
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
