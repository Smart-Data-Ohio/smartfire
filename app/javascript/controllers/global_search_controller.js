import { Controller } from "@hotwired/stimulus"

// The top-bar search (layouts/_global_search.html.erb): a combobox over
// the viewer's recent searches, following the ARIA combobox + listbox
// pattern. Focus stays in the field; ↑/↓ move aria-activedescendant over
// the recents, Enter opens the highlighted one or submits the typed query
// to searches#create (which records it), and Esc closes the panel.
// Typing filters the recents. When the header is too narrow for a field
// (a container query in global_search.css) it folds into an icon button
// that expands it over the header; Esc or leaving collapses it. "/" and
// Ctrl/⌘+Shift+F focus it from anywhere (keyboard_shortcuts dispatches
// global-search:focus).
export default class extends Controller {
  static targets = [ "toggle", "form", "input", "panel", "option" ]
  static classes = [ "expanded" ]

  connect() {
    this.activeIndex = -1
    this.onResize = this.#onResize.bind(this)
    window.addEventListener("resize", this.onResize)
  }

  disconnect() {
    window.removeEventListener("resize", this.onResize)
  }

  open() {
    if (!this.panelTarget.hidden) return

    this.#applyFilter()
    this.panelTarget.hidden = false
    this.inputTarget.setAttribute("aria-expanded", "true")
  }

  close() {
    this.#setActive(-1)
    if (this.panelTarget.hidden) return

    this.panelTarget.hidden = true
    this.inputTarget.setAttribute("aria-expanded", "false")
  }

  expand() {
    this.element.classList.add(this.expandedClass)
    this.toggleTarget.setAttribute("aria-expanded", "true")
    this.inputTarget.focus()
    this.open()
  }

  collapse({ restoreFocus = true } = {}) {
    this.close()
    if (!this.element.classList.contains(this.expandedClass)) return

    const hadFocus = this.element.contains(document.activeElement)
    this.element.classList.remove(this.expandedClass)
    this.toggleTarget.setAttribute("aria-expanded", "false")
    if (restoreFocus && hadFocus) this.toggleTarget.focus()
  }

  // The keyboard shortcuts land here.
  focusField() {
    if (this.#compact()) {
      this.expand()
    } else {
      this.inputTarget.focus()
      this.open()
    }
    this.inputTarget.select()
  }

  filter() {
    this.#applyFilter()
    this.open()
  }

  keydown(event) {
    if (event.isComposing) return

    if (event.key === "Escape") {
      this.#escape(event)
      return
    }

    if (event.target !== this.inputTarget) return

    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault()
        if (this.panelTarget.hidden) this.open()
        const options = this.#visibleOptions()
        if (options.length === 0) return
        const step = event.key === "ArrowDown" ? 1 : -1
        const start = this.activeIndex === -1 ? (step === 1 ? -1 : options.length) : this.activeIndex
        this.#setActive((start + step + options.length) % options.length)
        break
      }
      case "Enter": {
        const option = this.#visibleOptions()[this.activeIndex]
        if (option && !this.panelTarget.hidden) {
          event.preventDefault()
          this.#visit(option)
        }
        break
      }
    }
  }

  // Options take clicks without stealing focus from the field.
  keepFocus(event) {
    event.preventDefault()
  }

  choose(event) {
    this.#visit(event.currentTarget)
  }

  // Tabbing out closes; a null relatedTarget (a click on nothing
  // focusable, the clear confirm, a node swapped out from under focus)
  // is left to the pointer handler instead.
  focusOut(event) {
    const next = event.relatedTarget
    if (!next || this.element.contains(next)) return

    this.close()
    this.collapse({ restoreFocus: false })
  }

  pointerDownOutside(event) {
    if (this.element.contains(event.target)) return

    this.close()
    this.collapse({ restoreFocus: false })
  }

  // A clear from the panel swaps its recents out in place (searches/
  // clear.turbo_stream.erb). Cached snapshots of other pages still hold
  // the old recents, so drop them, and put focus back in the field.
  recentsCleared(event) {
    if (!event.detail?.success) return
    if (!event.target.closest?.(".global-search__clear-form")) return

    window.Turbo?.cache?.clear()
    this.inputTarget.focus()
  }

  reset() {
    this.collapse({ restoreFocus: false })
  }

  // Internal

  #escape(event) {
    if (!this.panelTarget.hidden) {
      event.preventDefault()
      event.stopPropagation()
      const fromPanel = this.panelTarget.contains(document.activeElement)
      this.close()
      if (this.#compact()) {
        this.collapse()
      } else if (fromPanel) {
        this.inputTarget.focus({ preventScroll: true })
        this.close()
      }
    } else if (this.element.classList.contains(this.expandedClass)) {
      event.preventDefault()
      event.stopPropagation()
      this.collapse()
    }
  }

  // Compact means the header shows the icon button instead of the field.
  #compact() {
    return this.toggleTarget.getClientRects().length > 0
  }

  // Growing out of compact drops the expanded state (the field shows in
  // place); shrinking into it with the panel open but nothing expanded
  // would leave the panel hanging off the header, so it closes.
  #onResize() {
    if (!this.#compact()) {
      if (this.element.classList.contains(this.expandedClass)) {
        this.element.classList.remove(this.expandedClass)
        this.toggleTarget.setAttribute("aria-expanded", "false")
      }
    } else if (!this.element.classList.contains(this.expandedClass)) {
      this.close()
    }
  }

  #visit(option) {
    const url = option.dataset.url
    this.inputTarget.value = option.dataset.query || this.inputTarget.value
    this.close()
    if (window.Turbo) {
      window.Turbo.visit(url)
    } else {
      window.location.href = url
    }
  }

  #applyFilter() {
    const term = this.inputTarget.value.trim().toLowerCase()
    const listbox = this.panelTarget.querySelector("[role='listbox']")
    let visible = 0

    this.optionTargets.forEach(option => {
      const matches = !term || (option.dataset.query || "").toLowerCase().includes(term)
      option.hidden = !matches
      if (matches) visible++
    })

    if (listbox && this.optionTargets.length > 0) listbox.hidden = visible === 0
    this.#setActive(-1)
  }

  #visibleOptions() {
    return this.optionTargets.filter(option => !option.hidden)
  }

  #setActive(index) {
    const options = this.#visibleOptions()
    this.activeIndex = index

    this.optionTargets.forEach(option => option.setAttribute("aria-selected", "false"))

    const active = options[index]
    if (active) {
      active.setAttribute("aria-selected", "true")
      this.inputTarget.setAttribute("aria-activedescendant", active.id)
      active.scrollIntoView({ block: "nearest" })
    } else {
      this.activeIndex = -1
      this.inputTarget.removeAttribute("aria-activedescendant")
    }
  }
}
