import { Controller } from "@hotwired/stimulus"

// Poll builder dialog: dynamic option inputs (2-10), focus handling,
// and form reset on close.
export default class extends Controller {
  static targets = [ "question", "options", "addButton" ]

  connect() {
    this.onClose = this.#reset.bind(this)
    this.element.addEventListener("close", this.onClose)
  }

  disconnect() {
    this.element.removeEventListener("close", this.onClose)
  }

  open() {
    if (!this.element.open) this.element.showModal()
    this.questionTarget.focus()
  }

  close() {
    this.element.close()
  }

  addOption() {
    const count = this.optionsTarget.querySelectorAll(".poll-builder__option").length
    if (count >= 10) return

    const wrapper = document.createElement("div")
    wrapper.className = "poll-builder__option"

    const label = document.createElement("label")
    label.className = "for-screen-reader"
    label.setAttribute("for", `poll_option_${count}`)
    label.textContent = `Option ${count + 1} (optional)`

    const input = document.createElement("input")
    input.type = "text"
    input.name = "poll[options][]"
    input.id = `poll_option_${count}`
    input.className = "input"
    input.maxLength = 200
    input.autocomplete = "off"

    const remove = document.createElement("button")
    remove.type = "button"
    remove.className = "btn btn--borderless btn--small"
    remove.setAttribute("aria-label", `Remove option ${count + 1}`)
    remove.textContent = "✕"
    remove.addEventListener("click", () => {
      wrapper.remove()
      this.#syncAddButton()
      this.#renumberOptions()
    })

    wrapper.append(label, input, remove)
    this.optionsTarget.append(wrapper)
    this.#syncAddButton()
    input.focus()
  }

  #syncAddButton() {
    const count = this.optionsTarget.querySelectorAll(".poll-builder__option").length
    this.addButtonTarget.disabled = count >= 10
  }

  #renumberOptions() {
    this.optionsTarget.querySelectorAll(".poll-builder__option").forEach((wrapper, index) => {
      const label = wrapper.querySelector("label")
      const input = wrapper.querySelector("input")
      label.setAttribute("for", `poll_option_${index}`)
      label.textContent = index < 2 ? `Option ${index + 1}` : `Option ${index + 1} (optional)`
      input.id = `poll_option_${index}`
      input.required = index < 2
    })
  }

  #reset() {
    const form = this.element.querySelector("form")
    form?.reset()
    this.optionsTarget.querySelectorAll(".poll-builder__option").forEach((wrapper, index) => {
      if (index >= 2) wrapper.remove()
    })
    this.#syncAddButton()
  }
}
