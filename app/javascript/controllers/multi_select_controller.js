import { Controller } from "@hotwired/stimulus"

// Checkbox multi-select shared by the member panel, the people directory,
// and the new-DM picker. One sticky bar posts the selection: Message opens
// the 1:1 or group DM, Start huddle opens it and rings the humans in it.
//
// The directory and picker show their checkboxes always: Shift-click a
// checkbox to extend from the last one touched, and Ctrl/Cmd-click needs
// no handling since each checkbox toggles independently. The member panel
// opts into selection mode (selection-mode-value="true") instead: rows
// render as avatar + identity, and the checkbox column appears only while
// something is selected. There a long-press, Ctrl/Cmd-click, or Space on
// a row selects it; plain clicks toggle while selecting instead of opening
// the profile card; Ctrl/Cmd-Shift-click adds the range from the anchor
// (the last Ctrl/Cmd-clicked or long-pressed row).
const MAX_OTHERS = 9
const LONG_PRESS_MS = 500
// A touch release is followed by compatibility mouse events at the same
// point, which would open the profile card under the finger, so clicks
// near the press point are swallowed briefly. Mirrors the message list.
const SUPPRESS_CLICK_MS = 500
const SUPPRESS_CLICK_RADIUS = 20
let noteIdCounter = 0

export default class extends Controller {
  static targets = [ "checkbox", "bar", "form", "inputs", "huddleInput", "messageButton", "huddleButton", "note", "status" ]
  static values = { selectionMode: Boolean }

  // Survives list re-renders (the member panel refreshes every 12
  // seconds): reconnected checkboxes restore from here, so removal only
  // ever happens through an explicit toggle or Clear. Class fields, not
  // connect(): target callbacks fire before connect for elements already
  // in the DOM. The range anchor is a user id rather than an index for
  // the same reason.
  selectedIds = new Set()
  lastId = null
  // The last row the user touched, for focus on mode exit. Unlike the
  // range anchor above, plain clicks and Space move it too.
  lastInteractedId = null

  connect() {
    if (!this.noteTarget.id) this.noteTarget.id = `multi-select-note-${++noteIdCounter}`
    this.suppressClick = this.#suppressClick.bind(this)
    document.addEventListener("click", this.suppressClick, true)
    if (this.selectionModeValue) {
      this.rowClick = this.#rowClick.bind(this)
      this.rowKeydown = this.#rowKeydown.bind(this)
      // Capture: row clicks are claimed before the profile-card
      // triggers inside them fire.
      this.element.addEventListener("click", this.rowClick, true)
      this.element.addEventListener("keydown", this.rowKeydown)
    }
    this.update()
  }

  disconnect() {
    document.removeEventListener("click", this.suppressClick, true)
    if (this.selectionModeValue) {
      this.element.removeEventListener("click", this.rowClick, true)
      this.element.removeEventListener("keydown", this.rowKeydown)
    }
    this.#cancelPress()
  }

  checkboxTargetConnected(checkbox) {
    checkbox.checked = this.selectedIds.has(checkbox.dataset.userId)
    this.update()
  }

  toggle(event) {
    // A long-press ends in a synthetic click on the row: the row already
    // selected itself, so the click must not toggle back.
    if (this.pressJustFired) return

    const checkbox = event.currentTarget

    // Selection-mode ranges add, as file managers do: Ctrl/Cmd-Shift-click
    // on the checkbox itself takes the same path as on the row, instead of
    // the inbox-style set-to-clicked-state range below (which would uncheck
    // the whole range when the clicked box just toggled off).
    if (this.selectionModeValue && event.shiftKey && (event.ctrlKey || event.metaKey)) {
      this.#selectRange(checkbox)
      return
    }
    const wasSelecting = this.element.classList.contains("is-selecting")
    const boxes = this.checkboxTargets
    const index = boxes.indexOf(checkbox)
    const lastIndex = boxes.findIndex((box) => box.dataset.userId === this.lastId)

    if (event.shiftKey && lastIndex !== -1 && lastIndex !== index) {
      const [ from, to ] = [ lastIndex, index ].sort((a, b) => a - b)
      boxes.slice(from, to + 1).forEach((box) => {
        box.checked = checkbox.checked
        this.#track(box)
      })
    } else {
      this.#track(checkbox)
    }

    if (this.selectionModeValue) {
      // The range anchor is the last Ctrl/Cmd-clicked or long-pressed
      // row: plain and Shift checkbox clicks leave it alone.
      if (event.ctrlKey || event.metaKey) this.lastId = checkbox.dataset.userId
    } else {
      this.lastId = checkbox.dataset.userId
    }
    this.update()
    this.#restoreFocusIfExited(wasSelecting)
  }

  clear() {
    const wasSelecting = this.element.classList.contains("is-selecting")
    this.selectedIds.clear()
    this.lastId = null
    this.checkboxTargets.forEach((box) => { box.checked = false })
    this.update()
    this.#restoreFocusIfExited(wasSelecting)
  }

  submitHuddle() {
    this.huddleInputTarget.value = "1"
  }

  // The bar posted and Turbo leaves for the DM: a success exits the
  // mode, while a failure keeps the selection for a retry.
  submitted(event) {
    if (event.detail?.success === false) return
    this.clear()
  }

  pressStart(event) {
    const row = event.currentTarget
    this.#cancelPress()

    const touch = event.touches[0]
    this.pressTimer = window.setTimeout(() => {
      const checkbox = row.querySelector("[data-multi-select-target='checkbox']")
      if (checkbox && !checkbox.checked) {
        checkbox.checked = true
        this.#track(checkbox)
        this.lastId = checkbox.dataset.userId
        this.update()
      } else if (checkbox && this.selectionModeValue) {
        this.lastId = checkbox.dataset.userId
        this.lastInteractedId = checkbox.dataset.userId
      }
      if (this.pressTouch) {
        this.suppressClickAt = { ...this.pressTouch }
        this.suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS
      }
      this.pressJustFired = true
      window.setTimeout(() => { this.pressJustFired = false }, 100)
    }, LONG_PRESS_MS)

    this.pressTouch = touch ? { x: touch.clientX, y: touch.clientY } : null
  }

  pressMove(event) {
    const touch = event.touches[0]
    if (!touch || !this.pressTouch) return

    const moved = Math.hypot(touch.clientX - this.pressTouch.x, touch.clientY - this.pressTouch.y)
    if (moved > 10) this.#cancelPress()
  }

  pressEnd() {
    this.#cancelPress()
  }

  // Blocked while a touch press is in progress, not just after it
  // fired: Android fires its native contextmenu while the finger is
  // still down, with the long-press timer pending. Mirrors the message
  // list.
  suppressMenu(event) {
    if (this.pressTimer || this.pressJustFired || Date.now() <= this.suppressClickUntil) event.preventDefault()
  }

  update() {
    if (!this.hasBarTarget) return

    const selected = this.checkboxTargets.filter((box) => box.checked)
    const humans = selected.filter((box) => box.dataset.bot !== "true")
    const bots = selected.length - humans.length
    const overCap = selected.length > MAX_OTHERS

    this.barTarget.hidden = selected.length === 0
    this.element.classList.toggle("is-selecting", this.selectionModeValue && selected.length > 0)
    this.#renderInputs(selected)
    this.huddleInputTarget.value = ""

    this.messageButtonTarget.textContent = `Message (${selected.length})`
    this.huddleButtonTarget.textContent = `Start huddle (${humans.length})`

    // The live region announces only the count, and only when it changes:
    // rewriting the same text would repeat the announcement on every
    // presence re-render.
    if (this.hasStatusTarget) {
      const label = `${selected.length} selected`
      if (this.statusTarget.textContent !== label) this.statusTarget.textContent = label
    }

    this.messageButtonTarget.disabled = overCap
    this.huddleButtonTarget.disabled = overCap || humans.length === 0

    const reasons = []
    if (overCap) reasons.push(`Group DMs hold at most ${MAX_OTHERS + 1} people including you.`)
    if (bots > 0) reasons.push(humans.length === 0
      ? "Agents can't join huddles."
      : `${bots} ${bots === 1 ? "agent stays" : "agents stay"} in the DM but won't be rung.`)

    this.noteTarget.hidden = reasons.length === 0
    this.noteTarget.textContent = reasons.join(" ")

    if (reasons.length === 0) {
      this.messageButtonTarget.removeAttribute("aria-describedby")
      this.huddleButtonTarget.removeAttribute("aria-describedby")
    } else {
      this.messageButtonTarget.setAttribute("aria-describedby", this.noteTarget.id)
      this.huddleButtonTarget.setAttribute("aria-describedby", this.noteTarget.id)
    }
    this.messageButtonTarget.title = overCap ? reasons[0] : ""
    this.huddleButtonTarget.title = reasons.join(" ")
  }

  // Selection-mode row clicks, before the profile-card triggers inside
  // the row. Ctrl/Cmd-click toggles (Ctrl/Cmd-Shift-click ranges from
  // the anchor); a plain click toggles while selecting and opens the
  // profile card otherwise. Your own row has no checkbox: Ctrl/Cmd-click
  // is swallowed, plain clicks behave as before.
  #rowClick(event) {
    const row = event.target.closest?.(".member-panel__member")
    if (!row || !this.element.contains(row)) return
    if (event.target.closest?.("input[data-multi-select-target='checkbox']")) return

    const checkbox = row.querySelector("[data-multi-select-target='checkbox']")
    const wasSelecting = this.element.classList.contains("is-selecting")

    if (event.ctrlKey || event.metaKey) {
      event.preventDefault()
      event.stopPropagation()
      if (!checkbox) return
      if (event.shiftKey) {
        this.#selectRange(checkbox)
      } else {
        this.#setChecked(checkbox, !checkbox.checked)
        this.lastId = checkbox.dataset.userId
        this.update()
        this.#restoreFocusIfExited(wasSelecting)
      }
      return
    }

    // Keyed on the connected rows, like update(): a selected member who
    // left the channel lingers in selectedIds until Clear, but the mode is
    // visibly over (bar hidden), so plain clicks must open cards again.
    if (!this.checkboxTargets.some((box) => box.checked) || !checkbox) return
    event.preventDefault()
    event.stopPropagation()
    this.#setChecked(checkbox, !checkbox.checked)
    this.update()
    this.#restoreFocusIfExited(wasSelecting)
  }

  // Space selects the focused row where Enter opens its profile card.
  // Checkboxes keep their native Space handling.
  #rowKeydown(event) {
    if (event.key !== " " && event.key !== "Spacebar") return
    const row = event.target.closest?.(".member-panel__member")
    if (!row || !this.element.contains(row)) return
    if (event.target.closest?.("input[data-multi-select-target='checkbox']")) return
    const checkbox = row.querySelector("[data-multi-select-target='checkbox']")
    if (!checkbox) return

    const wasSelecting = this.element.classList.contains("is-selecting")
    event.preventDefault()
    this.#setChecked(checkbox, !checkbox.checked)
    this.update()
    this.#restoreFocusIfExited(wasSelecting)
  }

  // Ctrl/Cmd-Shift-click adds the visual range between the anchor and
  // the clicked row, in rendered order across the Starred, Online and
  // Offline groups. Only checkbox rows take part, so your own row is
  // skipped; the clicked row becomes the new anchor. Without an anchor
  // the click just selects its own row.
  #selectRange(checkbox) {
    const boxes = this.checkboxTargets
    const index = boxes.indexOf(checkbox)
    const anchor = boxes.findIndex((box) => box.dataset.userId === this.lastId)

    if (anchor === -1) {
      this.#setChecked(checkbox, true)
    } else {
      const [ from, to ] = [ anchor, index ].sort((a, b) => a - b)
      boxes.slice(from, to + 1).forEach((box) => this.#setChecked(box, true))
    }
    this.lastId = checkbox.dataset.userId
    this.update()
  }

  // Swallows the compatibility click from a long-press release: any click
  // within a short window near the press point. Anything later or farther
  // away is a genuine tap and passes through.
  #suppressClick(event) {
    if (!this.suppressClickAt || Date.now() > this.suppressClickUntil) {
      this.suppressClickAt = null
      return
    }
    const distance = Math.hypot(event.clientX - this.suppressClickAt.x, event.clientY - this.suppressClickAt.y)
    if (distance > SUPPRESS_CLICK_RADIUS) return
    event.preventDefault()
    event.stopPropagation()
  }

  #setChecked(checkbox, checked) {
    checkbox.checked = checked
    this.#track(checkbox)
  }

  #track(checkbox) {
    this.lastInteractedId = checkbox.dataset.userId
    if (checkbox.checked) {
      this.selectedIds.add(checkbox.dataset.userId)
    } else {
      this.selectedIds.delete(checkbox.dataset.userId)
    }
  }

  // Leaving selection mode hides the focused control (the ✕ button, the
  // last checkbox), dropping focus to the page and out of the mobile
  // drawer's focus trap. Land on the last-touched row's name button
  // instead, or the first row's when that row is gone.
  #restoreFocusIfExited(wasSelecting) {
    if (!wasSelecting || !this.selectionModeValue) return
    if (this.checkboxTargets.some((box) => box.checked)) return

    const row = (this.lastInteractedId && this.element.querySelector(`[data-member-id="${this.lastInteractedId}"]`)) ||
      this.element.querySelector(".member-panel__member")
    const target = row?.querySelector("button.profile-card-name") || row?.querySelector("[data-action*='profile-card#open']")
    target?.focus({ preventScroll: true })
  }

  #renderInputs(selected) {
    this.inputsTarget.replaceChildren(
      ...selected.map((box) => {
        const input = document.createElement("input")
        input.type = "hidden"
        input.name = "user_ids[]"
        input.value = box.dataset.userId
        return input
      })
    )
  }

  #cancelPress() {
    window.clearTimeout(this.pressTimer)
    this.pressTimer = null
    this.pressTouch = null
  }
}
