import { Controller } from "@hotwired/stimulus"

const FOCUSABLE_SELECTOR = "button:not([disabled]), a[href]:not([aria-disabled='true'])"
const VIEWPORT_PADDING = 8

// The shared message menu and forward dialog, rendered once per page. A
// message-list controller (or a message toolbar button) opens it for one
// message at a time through the message-actions:open window event; the
// per-message URLs come from the message element's own data attributes.
export default class extends Controller {
  static targets = [
    "menu", "item", "downloadLink", "threadLabel", "pinLabel", "saveLabel", "status",
    "forwardDialog", "forwardPreview", "forwardNote",
    "forwardDestinations", "forwardStatus", "forwardSubmit",
    "saveDialog", "savePreview", "saveOption", "saveCustomWrap", "saveCustom", "saveStatus", "saveSubmit"
  ]

  #message
  #messageUrl
  #metadataUrl
  #boostUrl
  #metadata
  #metadataMessage
  #metadataRequest
  #metadataPromise
  #forwardDestinationsRequest
  #forwardDestinationsController
  #previouslyFocusedElement
  #menuPoint
  #mobileSheetQuery
  #announceTimer
  #forwardPreviouslyFocusedElement
  #savePreviouslyFocusedElement
  #open = false
  #connected = false

  connect() {
    if (!this.hasMenuTarget) return
    this.#connected = true

    this.onMenuKeydown = this.#onMenuKeydown.bind(this)
    this.onMenuClick = this.#onMenuClick.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onWindowKeydown = this.#onWindowKeydown.bind(this)
    this.onOpenRequest = this.#onOpenRequest.bind(this)
    this.onThreadRequest = this.#onThreadRequest.bind(this)
    this.onReactRequest = this.#onReactRequest.bind(this)
    this.onEditLast = this.#onEditLast.bind(this)
    this.onReposition = this.#reposition.bind(this)
    this.onForwardClose = this.#onForwardClose.bind(this)
    this.onSaveClose = this.#onSaveClose.bind(this)
    this.onBeforeCache = this.#onBeforeCache.bind(this)

    this.menuTarget.addEventListener("keydown", this.onMenuKeydown)
    this.menuTarget.addEventListener("click", this.onMenuClick)
    this.forwardDialogTarget?.addEventListener("close", this.onForwardClose)
    this.saveDialogTarget?.addEventListener("close", this.onSaveClose)
    document.addEventListener("turbo:before-cache", this.onBeforeCache)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    window.addEventListener("keydown", this.onWindowKeydown)
    window.addEventListener("resize", this.onReposition)
    window.visualViewport?.addEventListener("resize", this.onReposition)
    window.visualViewport?.addEventListener("scroll", this.onReposition)
    window.addEventListener("message-actions:open", this.onOpenRequest)
    window.addEventListener("message-actions:thread", this.onThreadRequest)
    window.addEventListener("message-actions:react", this.onReactRequest)
    window.addEventListener("message-actions:edit-last", this.onEditLast)
  }

  disconnect() {
    this.#connected = false
    this.#metadataRequest?.abort()
    this.#metadataPromise = null
    this.#forwardDestinationsController?.abort()

    this.menuTarget?.removeEventListener("keydown", this.onMenuKeydown)
    this.menuTarget?.removeEventListener("click", this.onMenuClick)
    this.forwardDialogTarget?.removeEventListener("close", this.onForwardClose)
    this.saveDialogTarget?.removeEventListener("close", this.onSaveClose)
    document.removeEventListener("turbo:before-cache", this.onBeforeCache)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    window.removeEventListener("keydown", this.onWindowKeydown)
    window.removeEventListener("resize", this.onReposition)
    window.visualViewport?.removeEventListener("resize", this.onReposition)
    window.visualViewport?.removeEventListener("scroll", this.onReposition)
    window.removeEventListener("message-actions:open", this.onOpenRequest)
    window.removeEventListener("message-actions:thread", this.onThreadRequest)
    window.removeEventListener("message-actions:react", this.onReactRequest)
    window.removeEventListener("message-actions:edit-last", this.onEditLast)
  }

  openFor(message, point) {
    if (!message?.isConnected || !message.dataset.actionsUrl) return
    this.#setMessage(message)
    this.#configureForMessage()
    this.#openMenu(point)
  }

  async requestThread(message) {
    if (!message?.isConnected || !message.dataset.actionsUrl) return
    this.#setMessage(message)
    await this.#ensureMetadata()
    if (this.#message !== message || !message.isConnected) return
    this.#dispatchThread()
  }

  close(event) {
    this.#closeMenu()
  }

  reply(event) {
    event.preventDefault()
    this.#dispatchMessageEvent("message:reply")
    this.#closeMenu({ restoreFocus: false })
  }

  async edit(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    // Another menu opening mid-request must not redirect this action to
    // its message; the newer menu stays open untouched.
    if (this.#message !== message || !message?.isConnected) return
    if (this.#boolean(this.#metadata || {}, "can_edit", "canEdit", "editable") !== true) {
      this.#closeMenu({ restoreFocus: false })
      return
    }
    this.#dispatchMessageEvent("message:edit")
    this.#closeMenu({ restoreFocus: false })
  }

  copyText(event) {
    event.preventDefault()
    const content = this.#copyTextContent()
    this.#copy(content, "Message copied")
  }

  copyLink(event) {
    event.preventDefault()
    this.#copy(this.#permalinkUrl() || this.#messageUrl, "Message link copied")
  }

  async forward(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    if (this.#message !== message || !message?.isConnected) return
    const detail = {
      forwardUrl: this.#stringFromMetadata("forward_url", "forwardUrl"),
      sourceUrl: this.#permalinkUrl() || this.#messageUrl,
    }
    this.#dispatchMessageEvent("message:forward", detail)
    this.#openForwardDialog(detail)
    this.#closeMenu({ restoreFocus: false })
  }

  closeForward(event) {
    event?.preventDefault()
    this.#closeForwardDialog()
  }

  async submitForward(event) {
    event.preventDefault()
    if (!this.hasForwardDialogTarget || this.forwardSubmitTarget.disabled) return

    const selected = this.forwardDestinationsTarget.querySelectorAll("input[type='checkbox']:checked")
    if (selected.length === 0) {
      this.#setForwardStatus("Choose at least one destination.")
      return
    }
    if (selected.length > 5) {
      this.#setForwardStatus("Choose up to 5 destinations.")
      return
    }

    const url = this.#stringFromMetadata("forward_url", "forwardUrl") || `${this.#messageUrl}/forwards`
    const destinations = Array.from(selected, input => {
      const destination = { room_id: input.dataset.roomId || input.value }
      if (input.dataset.threadId) destination.thread_id = input.dataset.threadId
      return destination
    })
    const note = this.hasForwardNoteTarget ? this.forwardNoteTarget.value.trim() : ""
    this.forwardSubmitTarget.disabled = true
    this.#setForwardStatus("Forwarding…")

    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
        },
        body: JSON.stringify({ forward: { note: note || null, destinations } }),
      })

      if (!response.ok) throw new Error(await this.#responseError(response))

      this.#setForwardStatus(`Forwarded to ${destinations.length} ${destinations.length === 1 ? "destination" : "destinations"}.`)
      setTimeout(() => this.#closeForwardDialog(), 650)
    } catch (error) {
      this.#setForwardStatus(error.message || "Couldn’t forward message.")
      this.forwardSubmitTarget.disabled = false
    }
  }

  async thread(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    if (this.#message !== message || !message?.isConnected) return
    this.#dispatchThread()
    this.#closeMenu({ restoreFocus: false })
  }

  async pin(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    if (this.#message !== message || !message?.isConnected) return

    const url = this.#stringFromMetadata("pin_url", "pinUrl")
    if (!url) {
      this.#announce("Pinning is temporarily unavailable")
      return
    }

    const pinned = this.#boolean(this.#metadata || {}, "pinned") === true
    const response = await fetch(url, {
      method: pinned ? "DELETE" : "POST",
      headers: {
        Accept: "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
    }).catch(() => null)

    if (!response?.ok) {
      this.#announce(response ? await this.#responseError(response, "pin") : "Couldn't reach the server")
      return
    }

    if (this.#metadata) this.#metadata.pinned = !pinned
    this.#refreshPinSaveLabels()
    this.#announce(pinned ? "Message unpinned" : "Message pinned")
    if (this.#message === message) this.#closeMenu({ restoreFocus: false })
  }

  async save(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    if (this.#message !== message || !message?.isConnected) return

    if (this.#boolean(this.#metadata || {}, "saved") === true) {
      await this.#unsave(message)
      return
    }

    this.#openSaveDialog()
    this.#closeMenu({ restoreFocus: false })
  }

  closeSave(event) {
    event?.preventDefault()
    this.#closeSaveDialog()
  }

  revealSaveCustom() {
    if (!this.hasSaveCustomWrapTarget) return
    const custom = this.saveOptionTargets.find(input => input.checked)?.value === "custom"
    this.saveCustomWrapTarget.hidden = !custom
    if (custom) this.saveCustomTarget?.focus({ preventScroll: true })
  }

  async submitSave(event) {
    event.preventDefault()
    if (!this.hasSaveDialogTarget || this.saveSubmitTarget.disabled) return

    let remindAt
    try {
      remindAt = this.#saveRemindAt()
    } catch (error) {
      this.#setSaveStatus(error.message)
      return
    }

    const url = this.#stringFromMetadata("save_url", "saveUrl")
    if (!url) {
      this.#setSaveStatus("Saving is temporarily unavailable.")
      return
    }

    this.saveSubmitTarget.disabled = true
    this.#setSaveStatus("Saving…")

    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
        },
        body: JSON.stringify({ message_id: this.#messageDetails().messageId, saved_item: { remind_at: remindAt } }),
      })

      if (!response.ok) throw new Error(await this.#responseError(response, "save"))

      const payload = await response.json().catch(() => ({}))
      if (this.#metadata) {
        this.#metadata.saved = true
        if (payload.url) this.#metadata.saved_item_url = payload.url
      }
      this.#refreshPinSaveLabels()
      this.#announce(remindAt ? "Saved with a reminder" : "Saved for later")
      this.#closeSaveDialog()
      this.#closeMenu({ restoreFocus: false })
    } catch (error) {
      this.#setSaveStatus(error.message || "Couldn't save message.")
      this.saveSubmitTarget.disabled = false
    }
  }

  async delete(event) {
    event.preventDefault()
    if (!window.confirm("Are you sure you want to delete this message?")) return

    const message = this.#message
    const messageUrl = this.#messageUrl
    const response = await fetch(messageUrl, {
      method: "DELETE",
      headers: {
        Accept: "text/vnd.turbo-stream.html, application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
    }).catch(() => null)

    if (!response?.ok) {
      this.#announce("Couldn’t delete message")
      return
    }

    const contentType = response.headers.get("content-type") || ""
    if (contentType.includes("turbo-stream")) {
      Turbo.renderStreamMessage(await response.text())
    } else if (message?.isConnected) {
      message.remove()
    }

    // The confirmed delete still lands, but a menu opened mid-request
    // keeps its own message and state.
    if (this.#message === message) this.#closeMenu({ restoreFocus: false })
  }

  async removeEmbeds(event) {
    event.preventDefault()
    const message = this.#message
    await this.#ensureMetadata()
    // Another menu opening mid-request must not redirect this action to
    // its message; the newer menu stays open untouched.
    if (this.#message !== message || !message?.isConnected) return
    if (this.#boolean(this.#metadata || {}, "can_remove_embeds", "canRemoveEmbeds") !== true) {
      this.#closeMenu({ restoreFocus: false })
      return
    }

    const url = this.#stringFromMetadata("suppress_embeds_url", "suppressEmbedsUrl") || `${this.#messageUrl}/embed_suppression`
    const response = await fetch(url, {
      method: "POST",
      headers: {
        Accept: "text/vnd.turbo-stream.html, application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
    }).catch(() => null)

    if (!response?.ok) {
      this.#announce("Couldn’t remove embeds")
      return
    }

    const contentType = response.headers.get("content-type") || ""
    if (contentType.includes("turbo-stream")) {
      Turbo.renderStreamMessage(await response.text())
    } else {
      message.querySelectorAll(".link-embed-cards, .linkedin-post-cards").forEach(container => container.replaceChildren())
    }
    this.#announce("Embeds removed")

    if (this.#message === message) this.#closeMenu({ restoreFocus: false })
  }

  // Internal event handlers

  #onOpenRequest(event) {
    if (!event.detail?.message) return
    this.openFor(event.detail.message, { x: event.detail.x, y: event.detail.y })
  }

  #onThreadRequest(event) {
    if (event.detail?.message) void this.requestThread(event.detail.message)
  }

  // The toolbar quick-react has no form of its own (forms carry per-session
  // tokens, which the cached message HTML must not include), so it submits
  // through the matching shared-menu form.
  #onReactRequest(event) {
    const { message, content } = event.detail || {}
    if (!message?.isConnected || !message.dataset.boostUrl || !content) return
    const input = this.menuTarget.querySelector(`input[name="boost[content]"][value="${CSS.escape(content)}"]`)
    const form = input?.closest("form")
    if (!form) return
    form.action = message.dataset.boostUrl
    form.setAttribute("data-turbo-frame", `boosting_${message.id}`)
    form.requestSubmit()
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
    this.#focusMenuItem(items[nextIndex])
  }

  #onMenuClick(event) {
    const item = event.target.closest(FOCUSABLE_SELECTOR)
    if (!item || !this.menuTarget.contains(item)) return

    // Actions that open another UI or submit a form close themselves. The delayed
    // close also lets the browser finish a normal link/form activation first.
    if (!item.matches("[data-action*='message-actions#delete']")) {
      setTimeout(() => this.#closeMenu({ restoreFocus: false }), 0)
    }
  }

  #onDocumentPointerDown(event) {
    if (!this.#open) return
    if (this.menuTarget.contains(event.target)) return
    if (this.hasForwardDialogTarget && this.forwardDialogTarget.contains(event.target)) return
    if (this.hasSaveDialogTarget && this.saveDialogTarget.contains(event.target)) return
    if (this.#message?.contains(event.target)) return
    this.#closeMenu()
  }

  #onWindowKeydown(event) {
    if (this.#open && event.key === "Escape" && !this.menuTarget.contains(event.target)) {
      event.preventDefault()
      this.#closeMenu()
    }
  }

  #onBeforeCache() {
    // Snapshots must not keep an open menu: the restored page would show
    // aria-expanded="true" on a message whose menu is gone.
    this.#closeMenu({ restoreFocus: false })
    this.#closeSaveDialog()
  }

  #onForwardClose() {
    this.#forwardDestinationsController?.abort()
    this.#forwardDestinationsController = null
    this.#forwardDestinationsRequest = null
    this.forwardDialogTarget?.querySelectorAll("input[type='checkbox']").forEach(input => {
      input.checked = false
      input.disabled = false
    })
    // The submit stays disabled from a successful forward until the dialog
    // closes, so the 650ms close delay cannot double-submit.
    if (this.hasForwardSubmitTarget) this.forwardSubmitTarget.disabled = false
    if (this.hasForwardNoteTarget) this.forwardNoteTarget.value = ""
    this.#setForwardStatus("")
    const focusTarget = this.#forwardPreviouslyFocusedElement?.isConnected ? this.#forwardPreviouslyFocusedElement : this.#message
    this.#forwardPreviouslyFocusedElement = null
    focusTarget?.focus?.({ preventScroll: true })
  }

  async #onEditLast(event) {
    const message = event.detail?.message
    if (!message?.isConnected || !message.dataset.actionsUrl) return
    this.#setMessage(message)
    await this.#ensureMetadata()
    if (this.#message !== message || !message.isConnected) return
    if (this.#metadataMessage !== message || !this.#metadata) {
      this.#flashError("Message actions are temporarily unavailable")
      return
    }
    if (this.#boolean(this.#metadata, "can_edit", "canEdit", "editable") !== true) return
    this.#dispatchMessageEvent("message:edit")
  }

  // Menu lifecycle

  #setMessage(message) {
    if (this.#message && this.#message !== message) this.#clearMessageState()
    this.#message = message
    this.#messageUrl = message.dataset.messageUrl
    this.#metadataUrl = message.dataset.actionsUrl
    this.#boostUrl = message.dataset.boostUrl
    if (this.#metadataMessage !== message) {
      this.#metadataRequest?.abort()
      this.#metadataRequest = null
      this.#metadataPromise = null
      this.#metadata = null
      this.#metadataMessage = message
    }
  }

  #configureForMessage() {
    const frame = `boosting_${this.#message.id}`
    this.menuTarget.querySelectorAll("form").forEach(form => {
      form.action = this.#boostUrl
      form.setAttribute("data-turbo-frame", frame)
    })

    if (this.hasDownloadLinkTarget) {
      const source = this.#message.querySelector(".message__body-content a.message__action-btn[href], .message__body-content a[data-lightbox-target='image'][href]")
      if (source) {
        this.downloadLinkTarget.href = source.href
        this.downloadLinkTarget.hidden = false
        this.downloadLinkTarget.setAttribute("aria-hidden", "false")
      } else {
        this.downloadLinkTarget.hidden = true
        this.downloadLinkTarget.setAttribute("aria-hidden", "true")
      }
    }

    this.#setActionAvailability(".message__edit-action", false)
    this.#setActionAvailability(".message__delete-action", false)
    this.#setActionAvailability(".message__remove-embeds-action", false)
    if (this.hasThreadLabelTarget) this.threadLabelTarget.textContent = "Create thread"
    if (this.hasPinLabelTarget) this.pinLabelTarget.textContent = "Pin message"
    if (this.hasSaveLabelTarget) this.saveLabelTarget.textContent = "Save for later"
    this.itemTargets.filter(item => item.dataset.reaction).forEach(item => {
      item.removeAttribute("aria-pressed")
      item.removeAttribute("data-reaction-active")
    })

    this.#message.setAttribute("aria-haspopup", "menu")
    if (this.menuTarget.id) this.#message.setAttribute("aria-controls", this.menuTarget.id)

    if (this.#metadataMessage === this.#message && this.#metadata) {
      this.#applyMetadata({ actions: this.#metadata })
    }
  }

  #clearMessageState() {
    this.#message?.removeAttribute("data-message-actions-open")
    this.#message?.setAttribute("aria-expanded", "false")
    this.#moreButton()?.setAttribute("aria-expanded", "false")
  }

  #moreButton() {
    return this.#message?.querySelector("[data-action~='message-toolbar#more']")
  }

  #openMenu(point) {
    if (!this.#message?.isConnected) return

    this.#previouslyFocusedElement = document.activeElement
    this.#menuPoint = point
    this.#open = true
    this.#message.setAttribute("data-message-actions-open", "")
    this.#message.setAttribute("aria-expanded", "true")
    this.#moreButton()?.setAttribute("aria-expanded", "true")
    this.menuTarget.hidden = false
    this.menuTarget.setAttribute("aria-hidden", "false")
    this.#showMenuPopover()
    this.#positionMenu()
    void this.#ensureMetadata()

    const focus = () => {
      this.#positionMenu()
      this.#focusMenuItem(this.#menuItems()[0])
    }
    if (window.requestAnimationFrame) window.requestAnimationFrame(focus)
    else setTimeout(focus, 0)
  }

  #closeMenu({ restoreFocus = true } = {}) {
    if (!this.#open && this.menuTarget?.hidden) return

    this.#open = false
    this.#clearMessageState()
    if (this.menuTarget) {
      this.#hideMenuPopover()
      this.menuTarget.hidden = true
      this.menuTarget.setAttribute("aria-hidden", "true")
    }

    if (restoreFocus) {
      const focusTarget = this.#previouslyFocusedElement?.isConnected ? this.#previouslyFocusedElement : this.#message
      focusTarget?.focus?.({ preventScroll: true })
    }
    this.#previouslyFocusedElement = null
  }

  #openForwardDialog(detail) {
    if (!this.hasForwardDialogTarget) return

    this.#forwardPreviouslyFocusedElement = this.#previouslyFocusedElement?.isConnected
      ? this.#previouslyFocusedElement
      : this.#message
    this.#showForwardDestinationsLoading()
    if (this.hasForwardPreviewTarget) this.forwardPreviewTarget.textContent = this.#messageDetails().previewText
    if (this.forwardDialogTarget.showModal) {
      if (!this.forwardDialogTarget.open) this.forwardDialogTarget.showModal()
    } else {
      this.forwardDialogTarget.setAttribute("open", "")
    }
    void this.#loadForwardDestinations()
  }

  #closeForwardDialog() {
    if (!this.hasForwardDialogTarget) return
    if (this.forwardDialogTarget.open && this.forwardDialogTarget.close) {
      this.forwardDialogTarget.close()
    } else {
      this.forwardDialogTarget.removeAttribute("open")
      this.#onForwardClose()
    }
  }

  #showForwardDestinationsLoading() {
    if (!this.hasForwardDestinationsTarget) return

    const loading = document.createElement("p")
    loading.className = "message-forward-dialog__empty"
    loading.textContent = "Loading destinations…"
    this.forwardDestinationsTarget.replaceChildren(loading)
  }

  async #loadForwardDestinations() {
    if (!this.hasForwardDestinationsTarget || !this.#connected) return

    this.#forwardDestinationsController?.abort()
    const controller = new AbortController()
    this.#forwardDestinationsController = controller
    const request = this.#forwardDestinationsRequest = (async () => {
      const url = this.#stringFromMetadata("forward_destinations_url", "forwardDestinationsUrl") || this.#fallbackForwardDestinationsUrl()
      if (!url) {
        this.#populateForwardDestinations([])
        return
      }

      try {
        const response = await fetch(url, {
          headers: { Accept: "application/json" },
          cache: "no-store",
          signal: controller.signal,
        })

        if (!response.ok) throw new Error(await this.#responseError(response))

        const payload = await response.json()
        if (!this.#connected || controller.signal.aborted) return
        this.#populateForwardDestinations(payload.destinations || [])
      } catch (error) {
        if (error.name === "AbortError") return
        this.#populateForwardDestinations([])
        this.#setForwardStatus(error.message || "Couldn’t load destinations.")
      }
    })()

    try {
      await request
    } finally {
      if (this.#forwardDestinationsRequest === request) {
        this.#forwardDestinationsRequest = null
        this.#forwardDestinationsController = null
      }
    }
  }

  #fallbackForwardDestinationsUrl() {
    if (!this.#messageUrl) return null
    return `${this.#messageUrl.replace(/\/$/, "")}/forwards/destinations`
  }

  #populateForwardDestinations(destinations) {
    if (!this.hasForwardDestinationsTarget) return

    const nodes = []
    for (const destination of Array.isArray(destinations) ? destinations : []) {
      const roomId = destination.room_id ?? destination.id
      if (!roomId) continue

      const room = document.createElement("div")
      room.className = "message-forward-dialog__room"

      const heading = document.createElement("p")
      heading.className = "message-forward-dialog__room-name"
      heading.textContent = destination.name || `Room ${roomId}`
      room.append(heading)

      room.append(this.#forwardDestinationOption({
        roomId,
        label: destination.name || `Room ${roomId}`,
      }))

      for (const thread of Array.isArray(destination.threads) ? destination.threads : []) {
        if (!thread?.id) continue
        room.append(this.#forwardDestinationOption({
          roomId,
          threadId: thread.id,
          label: thread.name || `Thread ${thread.id}`,
          status: thread.status,
        }))
      }

      nodes.push(room)
    }

    this.forwardDestinationsTarget.replaceChildren(...nodes)
    if (nodes.length === 0) {
      const empty = document.createElement("p")
      empty.className = "message-forward-dialog__empty"
      empty.textContent = "No available destinations."
      this.forwardDestinationsTarget.append(empty)
      return
    }

    this.#enforceForwardLimit()
    this.forwardDestinationsTarget.querySelector("input")?.focus({ preventScroll: true })
  }

  #forwardDestinationOption({ roomId, threadId, label, status }) {
    const wrapper = document.createElement("label")
    wrapper.className = [
      "message-forward-dialog__destination",
      threadId ? "message-forward-dialog__destination--thread" : null,
    ].filter(Boolean).join(" ")

    const input = document.createElement("input")
    input.type = "checkbox"
    input.value = roomId
    input.dataset.roomId = roomId
    if (threadId) input.dataset.threadId = threadId
    input.addEventListener("change", () => this.#enforceForwardLimit())

    const text = document.createElement("span")
    text.textContent = threadId ? `↳ ${label}` : label
    if (status) text.title = status
    wrapper.append(input, text)
    return wrapper
  }

  #enforceForwardLimit() {
    if (!this.hasForwardDestinationsTarget) return
    const checkboxes = Array.from(this.forwardDestinationsTarget.querySelectorAll("input[type='checkbox']"))
    const selected = checkboxes.filter(input => input.checked)
    checkboxes.forEach(input => {
      input.disabled = !input.checked && selected.length >= 5
    })
    this.#setForwardStatus(selected.length >= 5 ? "Up to 5 destinations selected." : "")
  }

  #setForwardStatus(message) {
    if (this.hasForwardStatusTarget) this.forwardStatusTarget.textContent = message
  }

  async #unsave(message) {
    const url = this.#stringFromMetadata("saved_item_url", "savedItemUrl")
    if (!url) {
      this.#announce("Removing is temporarily unavailable")
      return
    }

    const response = await fetch(url, {
      method: "DELETE",
      headers: {
        Accept: "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
    }).catch(() => null)

    if (!response?.ok) {
      this.#announce("Couldn't remove saved message")
      return
    }

    if (this.#metadata) {
      this.#metadata.saved = false
      this.#metadata.saved_item_url = null
    }
    this.#refreshPinSaveLabels()
    this.#announce("Removed from Saved")
    if (this.#message === message) this.#closeMenu({ restoreFocus: false })
  }

  #openSaveDialog() {
    if (!this.hasSaveDialogTarget) return

    this.#savePreviouslyFocusedElement = this.#previouslyFocusedElement?.isConnected
      ? this.#previouslyFocusedElement
      : this.#message
    if (this.hasSavePreviewTarget) this.savePreviewTarget.textContent = this.#messageDetails().previewText
    if (this.saveDialogTarget.showModal) {
      if (!this.saveDialogTarget.open) this.saveDialogTarget.showModal()
    } else {
      this.saveDialogTarget.setAttribute("open", "")
    }
  }

  #closeSaveDialog() {
    if (!this.hasSaveDialogTarget) return
    if (this.saveDialogTarget.open && this.saveDialogTarget.close) {
      this.saveDialogTarget.close()
    } else {
      this.saveDialogTarget.removeAttribute("open")
      this.#onSaveClose()
    }
  }

  #onSaveClose() {
    if (this.hasSaveCustomTarget) this.saveCustomTarget.value = ""
    if (this.hasSaveCustomWrapTarget) this.saveCustomWrapTarget.hidden = true
    const first = this.saveOptionTargets[0]
    if (first) first.checked = true
    if (this.hasSaveSubmitTarget) this.saveSubmitTarget.disabled = false
    this.#setSaveStatus("")
    const focusTarget = this.#savePreviouslyFocusedElement?.isConnected ? this.#savePreviouslyFocusedElement : this.#message
    this.#savePreviouslyFocusedElement = null
    focusTarget?.focus?.({ preventScroll: true })
  }

  // Reminder presets resolve in the viewer's own time zone and submit
  // as UTC ISO8601, so "tomorrow at 9am" means the viewer's tomorrow.
  #saveRemindAt() {
    const selected = this.saveOptionTargets.find(input => input.checked)?.value || "none"
    const now = new Date()

    switch (selected) {
      case "minutes_20":
        return new Date(now.getTime() + 20 * 60 * 1000).toISOString()
      case "hour_1":
        return new Date(now.getTime() + 60 * 60 * 1000).toISOString()
      case "hours_3":
        return new Date(now.getTime() + 3 * 60 * 60 * 1000).toISOString()
      case "tomorrow_9am": {
        const next = new Date(now)
        next.setDate(next.getDate() + 1)
        next.setHours(9, 0, 0, 0)
        return next.toISOString()
      }
      case "custom": {
        const value = this.hasSaveCustomTarget ? this.saveCustomTarget.value : ""
        if (!value) throw new Error("Choose a custom time.")
        const at = new Date(value)
        if (Number.isNaN(at.getTime())) throw new Error("That time isn't valid.")
        if (at <= now) throw new Error("Choose a time in the future.")
        return at.toISOString()
      }
      default:
        return null
    }
  }

  #setSaveStatus(message) {
    if (this.hasSaveStatusTarget) this.saveStatusTarget.textContent = message
  }

  #refreshPinSaveLabels() {
    const metadata = this.#metadata || {}
    if (this.hasPinLabelTarget) {
      this.pinLabelTarget.textContent = this.#boolean(metadata, "pinned") === true ? "Unpin message" : "Pin message"
    }
    if (this.hasSaveLabelTarget) {
      this.saveLabelTarget.textContent = this.#boolean(metadata, "saved") === true ? "Remove from Saved" : "Save for later"
    }
  }

  async #responseError(response, verb = "forward") {
    const fallback = `Couldn’t ${verb} message (${response.status})`
    try {
      const payload = await response.clone().json()
      const value = payload.error || payload.errors
      if (typeof value === "string") return value
      if (value && typeof value === "object") {
        const message = Object.values(value).flat().find(Boolean)
        if (message) return String(message)
      }
    } catch {}
    return fallback
  }

  #positionMenu() {
    if (!this.#open || !this.menuTarget || this.menuTarget.hidden) return

    if (this.#isMobileSheet()) {
      // The mobile bottom sheet is placed entirely by CSS. Clear any
      // pointer-anchored coordinates so scrolling, resizing, or metadata
      // arriving late can't fight the fixed placement.
      this.menuTarget.style.left = ""
      this.menuTarget.style.top = ""
      return
    }

    const point = this.#menuPoint || { x: 0, y: 0 }
    const rect = this.menuTarget.getBoundingClientRect()
    const viewportHeight = window.visualViewport?.height || window.innerHeight
    const maxX = Math.max(VIEWPORT_PADDING, window.innerWidth - rect.width - VIEWPORT_PADDING)
    const maxY = Math.max(VIEWPORT_PADDING, viewportHeight - rect.height - VIEWPORT_PADDING - this.#safeAreaBottom())

    this.menuTarget.style.left = `${Math.min(Math.max(VIEWPORT_PADDING, point.x), maxX)}px`
    this.menuTarget.style.top = `${Math.min(Math.max(VIEWPORT_PADDING, point.y), maxY)}px`
  }

  // Keep the menu above the home indicator on phones with a bottom inset.
  #safeAreaBottom() {
    return parseFloat(getComputedStyle(this.menuTarget).getPropertyValue("--safe-area-bottom")) || 0
  }

  // Matches the bottom-sheet breakpoint in messages.css so JS and CSS agree.
  #isMobileSheet() {
    this.#mobileSheetQuery ??= window.matchMedia("(max-width: 100ch), (pointer: coarse)")
    return this.#mobileSheetQuery.matches
  }

  #showMenuPopover() {
    if (this.menuTarget.getAttribute("popover") === null || typeof this.menuTarget.showPopover !== "function") return

    try {
      this.menuTarget.showPopover()
    } catch {
      // The fixed-position fallback remains usable in browsers without a
      // working manual popover implementation.
    }
  }

  #hideMenuPopover() {
    if (typeof this.menuTarget.hidePopover !== "function") return

    try {
      if (this.menuTarget.matches(":popover-open")) this.menuTarget.hidePopover()
    } catch {
      // The hidden attribute below still closes the fixed-position fallback.
    }
  }

  #reposition() {
    this.#positionMenu()
  }

  #focusMenuItem(item) {
    if (!item) return
    this.#menuItems().forEach(menuItem => menuItem.tabIndex = menuItem === item ? 0 : -1)
    item.focus({ preventScroll: true })
  }

  #menuItems() {
    return Array.from(this.menuTarget.querySelectorAll(FOCUSABLE_SELECTOR)).filter(item => {
      return !item.hidden && item.getAttribute("aria-disabled") !== "true" && this.#isVisible(item)
    })
  }

  #isVisible(item) {
    if (item.getClientRects().length > 0) return true
    return window.getComputedStyle(item).display !== "none"
  }

  // Metadata and actions

  // The menu-open fetch and a follow-up edit/forward share one in-flight
  // request instead of aborting and refetching the same metadata.
  #ensureMetadata() {
    if (this.#metadata && this.#metadataMessage === this.#message) return Promise.resolve(this.#metadata)
    if (!this.#metadataUrl) return Promise.resolve(null)
    if (this.#metadataPromise && this.#metadataMessage === this.#message) return this.#metadataPromise

    this.#metadataRequest?.abort()
    const request = new AbortController()
    this.#metadataRequest = request
    const message = this.#message
    const url = this.#metadataUrl

    const promise = this.#metadataPromise = (async () => {
      try {
        const response = await fetch(url, {
          headers: { Accept: "application/json" },
          cache: "no-store",
          signal: request.signal,
        })
        if (!response.ok || !this.#connected || this.#message !== message) return null

        const payload = await response.json()
        if (this.#connected && this.#message === message) this.#applyMetadata(payload)
        return this.#metadata
      } catch (error) {
        if (error.name !== "AbortError") this.#announce("Message actions are temporarily unavailable")
        return null
      } finally {
        if (this.#metadataRequest === request) this.#metadataRequest = null
        if (this.#metadataPromise === promise) this.#metadataPromise = null
      }
    })()
    return promise
  }

  #applyMetadata(payload) {
    this.#metadata = payload.actions || payload.message || payload
    const metadata = this.#metadata || {}

    this.#setActionAvailability(".message__edit-action", this.#boolean(metadata, "can_edit", "canEdit", "editable"))
    this.#setActionAvailability(".message__delete-action", this.#boolean(metadata, "can_delete", "canDelete", "deletable"))
    this.#setActionAvailability(".message__remove-embeds-action", this.#boolean(metadata, "can_remove_embeds", "canRemoveEmbeds"))
    this.#refreshPinSaveLabels()

    const threadSummary = metadata.thread_summary || metadata.threadSummary
    if (threadSummary && this.hasThreadLabelTarget) {
      this.threadLabelTarget.textContent = this.#threadCount(threadSummary) > 0 ? "View thread" : "Create thread"
    }

    const reactions = metadata.reactions
    if (reactions && typeof reactions === "object") {
      this.itemTargets.filter(item => item.dataset.reaction).forEach(item => {
        const state = reactions[item.dataset.reaction]
        const active = typeof state === "object" ? state.active : state
        if (active === undefined) return
        item.setAttribute("aria-pressed", String(Boolean(active)))
        item.toggleAttribute("data-reaction-active", Boolean(active))
      })
    }

    this.#positionMenu()
  }

  #setActionAvailability(selector, allowed) {
    const action = this.menuTarget.querySelector(selector)
    if (!action || allowed === undefined) return
    action.hidden = !allowed
    action.setAttribute("aria-hidden", String(!allowed))
  }

  #boolean(metadata, ...keys) {
    for (const key of keys) {
      if (metadata[key] !== undefined && metadata[key] !== null) return Boolean(metadata[key])
    }
    return undefined
  }

  #stringFromMetadata(...keys) {
    const metadata = this.#metadata || {}
    for (const key of keys) {
      if (metadata[key] !== undefined && metadata[key] !== null) return String(metadata[key])
    }
    return null
  }

  #threadCount(summary) {
    if (typeof summary === "number") return summary
    return Number(summary.count || summary.message_count || summary.messageCount || 0)
  }

  #dispatchThread() {
    this.#dispatchMessageEvent("message:thread", {
      threadUrl: this.#stringFromMetadata("thread_url", "threadUrl"),
      threadSummary: this.#metadata?.thread_summary || this.#metadata?.threadSummary || null,
    })
  }

  #dispatchMessageEvent(name, extra = {}) {
    window.dispatchEvent(new CustomEvent(name, {
      detail: {
        ...this.#messageDetails(),
        ...extra,
      },
    }))
  }

  #messageDetails() {
    const body = this.#message.querySelector("[data-reply-target='body']")
    const author = this.#message.querySelector("[data-reply-target='author']")?.textContent.trim() || ""
    const source = this.#stringFromMetadata("edit_source", "editable_markdown_source", "editableMarkdownSource", "markdown_source")

    return {
      message: this.#message,
      messageId: this.#message.dataset.messageId,
      roomId: this.#message.dataset.roomId || document.querySelector("meta[name='current-room-id']")?.content,
      threadId: this.#message.dataset.threadId || "",
      author,
      url: this.#permalinkUrl() || this.#messageUrl,
      messageUrl: this.#messageUrl,
      source: source || null,
      sourceFormat: this.#stringFromMetadata("edit_format", "editable_format", "editableFormat") || "markdown",
      previewText: this.#fallbackText(body),
      driveAttachments: this.#driveAttachments(),
    }
  }

  #permalinkUrl() {
    const href = this.#message?.querySelector("a.message__permalink")?.getAttribute("href")
    if (!href) return null
    try {
      return new URL(href, window.location.origin).href
    } catch {
      return href
    }
  }

  // The message's current Drive attachments for the composer edit flow: the
  // file id from each block anchor's open?id= link, plus the display name
  // and kind the drive-link controller already resolved for this viewer
  // (generic when it has not). Names shown here are already on screen, so
  // nothing new is revealed.
  #driveAttachments() {
    return Array.from(this.#message.querySelectorAll(".drive-attachments a.drive-attachment")).map((anchor) => {
      const id = anchor.getAttribute("href")?.match(/[?&]id=([A-Za-z0-9_-]{10,})/)?.[1]
      const chip = anchor.querySelector(".drive-chip")
      return {
        id,
        name: anchor.querySelector(".drive-chip__name")?.textContent?.trim() || "Google Drive file",
        kind: chip?.className.match(/drive-chip--([\w-]+)/)?.[1] || "file",
      }
    }).filter((attachment) => attachment.id)
  }

  #fallbackText(body) {
    if (!body) return ""
    const clone = body.cloneNode(true)
    clone.querySelectorAll(".markdown-code-copy, .message__reply-preview, .boosts").forEach(node => node.remove())
    return (clone.innerText || clone.textContent || "").trim()
  }

  #copyTextContent() {
    const metadataText = this.#stringFromMetadata("copy_text", "copyText")
    return metadataText ?? this.#messageDetails().previewText
  }

  async #copy(text, successMessage) {
    try {
      await navigator.clipboard.writeText(text)
      this.#announce(successMessage)
    } catch {
      this.#announce("Copy failed")
    }
    this.#closeMenu({ restoreFocus: false })
  }

  #announce(message) {
    if (!this.hasStatusTarget) return
    this.statusTarget.textContent = message
    clearTimeout(this.#announceTimer)
    this.#announceTimer = setTimeout(() => this.statusTarget.textContent = "", 2_000)
  }

  // A client-side error flash matching the server flash markup: it removes
  // itself on animationend through element-removal and inherits the
  // reduced-motion persistence. Used where no menu is open to announce
  // into, like a failed up-arrow-to-edit.
  #flashError(message) {
    const flash = document.createElement("div")
    flash.className = "flash flash--client"
    flash.dataset.controller = "element-removal"
    flash.dataset.action = "animationend->element-removal#remove"
    flash.setAttribute("role", "alert")

    const inner = document.createElement("div")
    inner.className = "flash__inner flash__inner--text shadow"
    inner.style.setProperty("--flash-background", "var(--color-negative)")
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
}
