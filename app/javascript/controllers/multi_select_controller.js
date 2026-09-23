import { Controller } from "@hotwired/stimulus"

// Checkbox multi-select shared by the member panel, the people directory,
// and the new-DM picker. One sticky bar posts the selection: Message opens
// the 1:1 or group DM, Start huddle opens it and rings the humans in it.
//
// Desktop range selection works inbox-style: Shift-click a checkbox to
// extend from the last one touched. Ctrl/Cmd-click needs no handling —
// each checkbox already toggles independently. On touch, long-pressing a
// row selects it and reveals the bar.
const MAX_OTHERS = 9
const LONG_PRESS_MS = 500
let noteIdCounter = 0

export default class extends Controller {
  static targets = [ "checkbox", "bar", "form", "inputs", "huddleInput", "messageButton", "huddleButton", "note" ]

  // Survives list re-renders (the member panel refreshes every 12
  // seconds): reconnected checkboxes restore from here, so removal only
  // ever happens through an explicit toggle or Clear. Class fields, not
  // connect(): target callbacks fire before connect for elements already
  // in the DOM. The range anchor is a user id rather than an index for
  // the same reason.
  selectedIds = new Set()
  lastId = null

  connect() {
    if (!this.noteTarget.id) this.noteTarget.id = `multi-select-note-${++noteIdCounter}`
    this.update()
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

    this.lastId = checkbox.dataset.userId
    this.update()
  }

  clear() {
    this.selectedIds.clear()
    this.lastId = null
    this.checkboxTargets.forEach((box) => { box.checked = false })
    this.update()
  }

  submitHuddle() {
    this.huddleInputTarget.value = "1"
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

  suppressMenu(event) {
    if (this.pressJustFired) event.preventDefault()
  }

  update() {
    if (!this.hasBarTarget) return

    const selected = this.checkboxTargets.filter((box) => box.checked)
    const humans = selected.filter((box) => box.dataset.bot !== "true")
    const bots = selected.length - humans.length
    const overCap = selected.length > MAX_OTHERS

    this.barTarget.hidden = selected.length === 0
    this.#renderInputs(selected)
    this.huddleInputTarget.value = ""

    this.messageButtonTarget.textContent = `Message (${selected.length})`
    this.huddleButtonTarget.textContent = `Start huddle (${humans.length})`

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

  #track(checkbox) {
    if (checkbox.checked) {
      this.selectedIds.add(checkbox.dataset.userId)
    } else {
      this.selectedIds.delete(checkbox.dataset.userId)
    }
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
