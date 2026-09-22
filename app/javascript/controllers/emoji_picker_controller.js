import { Controller } from "@hotwired/stimulus"
import { debounce } from "helpers/timing_helpers"

const GRID_COLUMNS = 8
const VIEWPORT_PADDING = 8
// Boost content is limited to 16 characters server-side; options that cannot
// be stored are left out of the grid.
const MAX_CONTENT_LENGTH = 16
const MAX_SEARCH_RESULTS = 48
const MAX_RECENT = 24
const RECENT_KEY = "emoji-picker:recent:v1"

// The shared emoji picker, rendered once per page. It opens from a message
// toolbar button with a category tab bar (Recent, the Unicode groups, and
// Custom workspace icons) over a lazily fetched static data asset, plus
// search that merges that data with the icon autocomplete endpoint.
// Selecting an option submits it as a boost.
export default class extends Controller {
  static targets = [ "panel", "search", "tabs", "tab", "grid", "status", "form", "content" ]
  static values = { iconsUrl: String, dataUrl: String }

  #message
  #anchor
  #open = false
  #tab = "smileys"
  #data = null
  #dataPromise = null
  #customIcons = null
  #customPromise = null
  #searchRequest
  #searchToken = 0
  #connected = false

  initialize() {
    this.search = debounce(this.search.bind(this), 200)
  }

  connect() {
    if (!this.hasPanelTarget) return
    this.#connected = true
    this.#syncTabAttributes()

    this.onOpenRequest = this.#onOpenRequest.bind(this)
    this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)
    this.onPanelKeydown = this.#onPanelKeydown.bind(this)
    this.onGridKeydown = this.#onGridKeydown.bind(this)
    this.onPanelFocusOut = this.#onPanelFocusOut.bind(this)

    window.addEventListener("emoji-picker:open", this.onOpenRequest)
    document.addEventListener("pointerdown", this.onDocumentPointerDown)
    this.panelTarget.addEventListener("keydown", this.onPanelKeydown)
    this.gridTarget.addEventListener("keydown", this.onGridKeydown)
    this.panelTarget.addEventListener("focusout", this.onPanelFocusOut)
  }

  disconnect() {
    this.#connected = false
    this.#searchRequest?.abort()

    window.removeEventListener("emoji-picker:open", this.onOpenRequest)
    document.removeEventListener("pointerdown", this.onDocumentPointerDown)
    this.panelTarget?.removeEventListener("keydown", this.onPanelKeydown)
    this.gridTarget?.removeEventListener("keydown", this.onGridKeydown)
    this.panelTarget?.removeEventListener("focusout", this.onPanelFocusOut)
  }

  search() {
    if (!this.#open) return
    const query = this.searchTarget.value.trim()
    if (!query) {
      this.#searchRequest?.abort()
      this.#renderCurrentTab()
      return
    }
    void this.#renderSearch(query)
  }

  choose(event) {
    const option = event.target.closest("button[data-content]")
    if (!option || !this.gridTarget.contains(option)) return
    event.preventDefault()
    this.#setRoving(option)
    this.#submit(option.dataset.content, {
      label: option.dataset.label || option.getAttribute("aria-label"),
      image: option.dataset.image || null,
    })
  }

  chooseTab(event) {
    const tab = event.target.closest("[role='tab']")
    if (!tab || !this.tabsTarget.contains(tab)) return
    event.preventDefault()
    this.selectTab(tab.dataset.tab)
  }

  tabsKeydown(event) {
    if (![ "ArrowRight", "ArrowLeft", "Home", "End" ].includes(event.key)) return
    const tabs = this.tabTargets
    const current = tabs.indexOf(document.activeElement)
    if (current < 0) return

    event.preventDefault()
    let next
    switch (event.key) {
      case "ArrowRight":
        next = (current + 1) % tabs.length
        break
      case "ArrowLeft":
        next = (current - 1 + tabs.length) % tabs.length
        break
      case "Home":
        next = 0
        break
      case "End":
        next = tabs.length - 1
        break
    }
    this.selectTab(tabs[next].dataset.tab, { focus: true })
  }

  selectTab(id, { focus = false } = {}) {
    this.#tab = id
    this.#syncTabAttributes(focus)

    const active = this.tabTargets.find(tab => tab.dataset.tab === id)
    if (active) this.gridTarget.setAttribute("aria-labelledby", active.id)

    this.searchTarget.value = ""
    this.#searchRequest?.abort()
    this.#searchToken++
    this.#setStatus("")
    this.#renderCurrentTab()
    // Renders reset roving themselves; this covers the pre-data fallback.
    this.#resetRoving()
  }

  // Internal event handlers

  #onOpenRequest(event) {
    const { message, anchor } = event.detail || {}
    if (!message?.isConnected || !message.dataset.boostUrl) return

    if (this.#open && this.#message === message && this.#anchor === (anchor || null)) {
      this.#close()
      return
    }

    this.#message = message
    this.#anchor = anchor || null
    this.#open = true
    this.selectTab(this.#tab)
    this.panelTarget.hidden = false
    this.#showPopover()
    this.#position()
    this.searchTarget.focus()

    // The data asset loads on first open, never with the page, and is then
    // cached for the session. Tabs render from it once it arrives.
    void this.#ensureData().then(() => {
      if (!this.#connected || !this.#open || this.searchTarget.value.trim()) return
      this.#renderCurrentTab()
    }).catch(() => {
      if (this.#connected && this.#open) this.#setStatus("Emoji failed to load.")
    })
  }

  #onDocumentPointerDown(event) {
    if (!this.#open) return
    if (this.panelTarget.contains(event.target)) return
    if (this.#anchor?.contains(event.target)) return
    this.#close({ restoreFocus: false })
  }

  #onPanelKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault()
      this.#close()
      return
    }

    if (event.target === this.searchTarget && event.key === "ArrowDown") {
      const first = this.#options()[0]
      if (first) {
        event.preventDefault()
        first.focus()
      }
    }
  }

  #onGridKeydown(event) {
    const options = this.#options()
    if (options.length === 0) return
    const currentIndex = options.indexOf(document.activeElement)
    let nextIndex

    switch (event.key) {
      case "ArrowRight":
        nextIndex = Math.min(options.length - 1, currentIndex + 1)
        break
      case "ArrowLeft":
        nextIndex = Math.max(0, currentIndex - 1)
        break
      case "ArrowDown":
        nextIndex = Math.min(options.length - 1, (currentIndex < 0 ? -GRID_COLUMNS : currentIndex) + GRID_COLUMNS)
        break
      case "ArrowUp":
        if (currentIndex >= 0 && currentIndex < GRID_COLUMNS) {
          event.preventDefault()
          this.searchTarget.focus()
          return
        }
        nextIndex = Math.max(0, currentIndex - GRID_COLUMNS)
        break
      case "Home":
        nextIndex = 0
        break
      case "End":
        nextIndex = options.length - 1
        break
      default:
        return
    }

    // Enter and Space activate the focused option natively as a click.
    event.preventDefault()
    const next = options[nextIndex]
    if (next) {
      this.#setRoving(next)
      next.focus()
    }
  }

  #onPanelFocusOut(event) {
    if (!this.#open) return
    if (event.relatedTarget && this.panelTarget.contains(event.relatedTarget)) return
    this.#close({ restoreFocus: false })
  }

  // Tab rendering

  #renderCurrentTab() {
    if (this.#tab === "recent") this.#renderRecent()
    else if (this.#tab === "custom") void this.#renderCustom()
    else this.#renderGroup(this.#tab)
  }

  #renderGroup(id) {
    // Before the data asset arrives the server-rendered fallback stays in
    // place; the open handler re-renders once the fetch resolves.
    const group = this.#data?.groups?.find(group => group.id === id)
    if (!group) return

    this.#renderOptions(group.emoji.map(([ character, aliases, description ]) => ({
      content: character,
      label: this.#label(description),
      title: `:${aliases.split(" ", 1)[0]}:`,
    })))
  }

  #renderRecent() {
    const entries = this.#recentSorted()
    if (entries.length === 0) {
      this.gridTarget.replaceChildren()
      this.#setStatus("Emoji you react with will show up here.")
      return
    }

    this.#setStatus("")
    this.#renderOptions(entries.map(entry => ({
      content: entry.c,
      label: entry.l || entry.c,
      image: entry.i || null,
    })))
  }

  async #renderCustom() {
    if (!this.#customIcons) this.#setStatus("Loading workspace icons…")

    let icons
    try {
      icons = await this.#ensureCustom()
    } catch {
      if (this.#connected && this.#open && this.#tab === "custom") {
        this.#setStatus("Workspace icons are unavailable right now.")
      }
      return
    }
    if (!this.#connected || !this.#open || this.#tab !== "custom" || this.searchTarget.value.trim()) return

    const options = icons.flatMap(icon => {
      const content = `:${icon.name}:`
      if (!icon.name || content.length > MAX_CONTENT_LENGTH) return []
      return [ { content, label: icon.title || icon.name, image: icon.image || null } ]
    })

    if (options.length === 0) {
      this.gridTarget.replaceChildren()
      this.#setStatus("No workspace icons yet.")
      return
    }

    this.#setStatus("")
    this.#renderOptions(options)
  }

  async #renderSearch(query) {
    const token = ++this.#searchToken
    this.#setStatus("Searching…")

    const [ data, servers ] = await Promise.all([
      this.#ensureData().catch(() => null),
      this.#fetchServerIcons(query),
    ])
    if (!this.#connected || !this.#open || token !== this.#searchToken) return
    if (this.searchTarget.value.trim() !== query) return

    const seen = new Set()
    const options = []
    for (const match of this.#localMatches(query, data)) {
      if (seen.has(match.content)) continue
      seen.add(match.content)
      options.push(match)
    }
    for (const icon of servers || []) {
      const content = icon.character || (icon.name ? `:${icon.name}:` : null)
      if (!content || content.length > MAX_CONTENT_LENGTH || seen.has(content)) continue
      seen.add(content)
      options.push({ content, label: icon.title || icon.name || content, image: icon.image || null })
    }

    this.#renderOptions(options)
    if (options.length === 0) {
      this.#setStatus(servers === null ? "Search is unavailable right now." : "No matches.")
    } else {
      this.#setStatus(`${options.length} ${options.length === 1 ? "result" : "results"}.`)
    }
  }

  #localMatches(query, data = this.#data) {
    const groups = data?.groups
    if (!groups) return []

    const q = query.toLowerCase()
    const matches = []
    for (const group of groups) {
      for (const [ character, aliases, description, tags ] of group.emoji) {
        if (!`${aliases} ${description} ${tags} ${character}`.toLowerCase().includes(q)) continue

        const names = aliases.toLowerCase().split(" ")
        let score = 3
        if (names[0] === q) score = 0
        else if (names[0].startsWith(q)) score = 1
        else if (names.some(name => name.startsWith(q))) score = 2
        matches.push({ score, content: character, label: this.#label(description), title: `:${aliases.split(" ", 1)[0]}:` })
      }
    }
    return matches.sort((a, b) => a.score - b.score).slice(0, MAX_SEARCH_RESULTS)
  }

  // Data loading

  #ensureData() {
    if (this.#data) return Promise.resolve(this.#data)
    this.#dataPromise ??= (async () => {
      const response = await fetch(this.dataUrlValue, { headers: { Accept: "application/json" } })
      if (!response.ok) throw new Error(`emoji data (${response.status})`)
      this.#data = await response.json()
      return this.#data
    })().catch(error => {
      this.#dataPromise = null
      throw error
    })
    return this.#dataPromise
  }

  #ensureCustom() {
    if (this.#customIcons) return Promise.resolve(this.#customIcons)
    this.#customPromise ??= (async () => {
      const response = await fetch(`${this.iconsUrlValue}?custom=1`, { headers: { Accept: "application/json" } })
      if (!response.ok) throw new Error(`custom icons (${response.status})`)
      const icons = await response.json()
      this.#customIcons = Array.isArray(icons) ? icons : []
      return this.#customIcons
    })().catch(error => {
      this.#customPromise = null
      throw error
    })
    return this.#customPromise
  }

  async #fetchServerIcons(query) {
    this.#searchRequest?.abort()
    const request = new AbortController()
    this.#searchRequest = request

    try {
      const response = await fetch(`${this.iconsUrlValue}?q=${encodeURIComponent(query)}`, {
        headers: { Accept: "application/json" },
        signal: request.signal,
      })
      if (!response.ok || this.#searchRequest !== request) return []
      const icons = await response.json()
      return Array.isArray(icons) ? icons : []
    } catch (error) {
      if (error.name === "AbortError") return []
      return null
    } finally {
      if (this.#searchRequest === request) this.#searchRequest = null
    }
  }

  // Options grid

  #renderOptions(options) {
    const nodes = options.map(({ content, label, title, image }) => {
      const option = document.createElement("button")
      option.type = "button"
      option.className = "emoji-picker__option"
      option.dataset.content = content
      option.dataset.label = label
      if (image) option.dataset.image = image
      option.tabIndex = -1
      option.setAttribute("aria-label", label)
      if (title) option.title = title

      if (image) {
        const img = document.createElement("img")
        img.src = image
        img.alt = ""
        img.setAttribute("aria-hidden", "true")
        img.width = 20
        img.height = 20
        option.append(img)
      } else {
        const glyph = document.createElement("span")
        glyph.setAttribute("aria-hidden", "true")
        glyph.textContent = content
        option.append(glyph)
      }
      return option
    })

    this.gridTarget.replaceChildren(...nodes)
    this.#resetRoving()
  }

  #resetRoving() {
    this.#options().forEach((option, index) => {
      option.tabIndex = index === 0 ? 0 : -1
    })
  }

  #setRoving(option) {
    this.#options().forEach(other => {
      other.tabIndex = other === option ? 0 : -1
    })
  }

  #syncTabAttributes(focus = false) {
    this.tabTargets.forEach(tab => {
      const selected = tab.dataset.tab === this.#tab
      tab.setAttribute("aria-selected", String(selected))
      tab.tabIndex = selected ? 0 : -1
      if (selected && focus) tab.focus()
    })
  }

  // Recents (frequent-first, from localStorage)

  #recentSorted() {
    return this.#loadRecent()
      .sort((a, b) => (b.n - a.n) || (b.t - a.t))
      .slice(0, MAX_RECENT)
  }

  #loadRecent() {
    try {
      const entries = JSON.parse(localStorage.getItem(RECENT_KEY) || "[]")
      return Array.isArray(entries) ? entries.filter(entry => entry && entry.c) : []
    } catch {
      return []
    }
  }

  #recordRecent(content, label, image) {
    try {
      const entries = this.#loadRecent()
      const existing = entries.find(entry => entry.c === content)
      if (existing) {
        existing.n = (existing.n || 1) + 1
        existing.t = Date.now()
        existing.l = label || existing.l
        existing.i = image || existing.i
      } else {
        entries.push({ c: content, l: label, i: image, n: 1, t: Date.now() })
      }
      localStorage.setItem(RECENT_KEY, JSON.stringify(entries.slice(-100)))
    } catch {
      // Private browsing: reactions still work, recents just don't persist.
    }
  }

  // Picker lifecycle

  #submit(content, { label, image } = {}) {
    if (!content || !this.#message?.isConnected) {
      this.#close({ restoreFocus: false })
      return
    }
    this.#recordRecent(content, label, image)
    this.formTarget.action = this.#message.dataset.boostUrl
    this.formTarget.setAttribute("data-turbo-frame", `boosting_${this.#message.id}`)
    this.contentTarget.value = content
    this.#close({ restoreFocus: false })
    this.formTarget.requestSubmit()
    this.#anchor?.focus?.({ preventScroll: true })
  }

  #close({ restoreFocus = true } = {}) {
    if (!this.#open && this.panelTarget?.hidden) return

    this.#open = false
    this.#searchRequest?.abort()
    this.#searchRequest = null
    this.#searchToken++
    if (this.panelTarget) {
      this.#hidePopover()
      this.panelTarget.hidden = true
    }

    if (restoreFocus) this.#anchor?.focus?.({ preventScroll: true })
    this.#anchor = null
  }

  #position() {
    if (!this.#open || !this.panelTarget || this.panelTarget.hidden) return
    if (!this.#anchor?.isConnected) {
      this.panelTarget.style.left = ""
      this.panelTarget.style.top = ""
      return
    }

    const anchor = this.#anchor.getBoundingClientRect()
    const rect = this.panelTarget.getBoundingClientRect()
    const viewportHeight = window.visualViewport?.height || window.innerHeight
    const maxX = Math.max(VIEWPORT_PADDING, window.innerWidth - rect.width - VIEWPORT_PADDING)
    const maxY = Math.max(VIEWPORT_PADDING, viewportHeight - rect.height - VIEWPORT_PADDING)

    const x = Math.min(Math.max(VIEWPORT_PADDING, anchor.left), maxX)
    const below = anchor.bottom + 4
    const y = below + rect.height <= viewportHeight - VIEWPORT_PADDING
      ? below
      : Math.min(Math.max(VIEWPORT_PADDING, anchor.top - rect.height - 4), maxY)

    this.panelTarget.style.left = `${x}px`
    this.panelTarget.style.top = `${y}px`
  }

  #showPopover() {
    if (this.panelTarget.getAttribute("popover") === null || typeof this.panelTarget.showPopover !== "function") return
    try {
      this.panelTarget.showPopover()
    } catch {
      // The fixed-position fallback remains usable without popover support.
    }
  }

  #hidePopover() {
    if (typeof this.panelTarget.hidePopover !== "function") return
    try {
      if (this.panelTarget.matches(":popover-open")) this.panelTarget.hidePopover()
    } catch {
      // The hidden attribute still closes the fixed-position fallback.
    }
  }

  #options() {
    return Array.from(this.gridTarget.querySelectorAll("button[data-content]"))
  }

  // Accessible names read like the server icon titles ("Fire"), while the
  // Unicode descriptions in the data asset are lowercase ("fire").
  #label(description) {
    return description ? description.charAt(0).toUpperCase() + description.slice(1) : description
  }

  #setStatus(message) {
    if (this.hasStatusTarget) this.statusTarget.textContent = message
  }
}
