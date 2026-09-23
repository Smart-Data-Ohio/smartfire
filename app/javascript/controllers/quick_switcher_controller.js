import { Controller } from "@hotwired/stimulus"

const RECENT_LIMIT = 8

const KIND_LABELS = {
  channel: "Channel", dm: "DM", group: "Group DM",
  voice: "Voice", stage: "Stage", board: "Board"
}

// The Ctrl/⌘+K quick switcher: a modal combobox over the user's rooms,
// the people they can open a DM with, and recent threads. One JSON
// endpoint feeds it; filtering is local and fuzzy, and recents (this
// browser's visited rooms) lead when the query is empty.
export default class extends Controller {
  static targets = [ "input", "list", "status" ]
  static values = { url: String, userId: Number }

  #data
  #request
  #options = []
  #activeIndex = -1

  connect() {
    this.#recordVisit()
  }

  disconnect() {
    this.#request?.abort()
    this.#request = null
  }

  open() {
    if (this.element.open) {
      this.inputTarget.focus()
      return
    }
    this.element.showModal()
    this.inputTarget.value = ""
    void this.#load()
  }

  toggle() {
    if (this.element.open) this.element.close()
    else this.open()
  }

  wasClosed() {
    this.inputTarget.value = ""
    this.inputTarget.removeAttribute("aria-activedescendant")
    this.#options = []
    this.#activeIndex = -1
  }

  dismissBackdrop(event) {
    if (event.target === this.element) this.element.close()
  }

  filter() {
    this.#render(this.inputTarget.value)
  }

  key(event) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault()
        this.#moveActive(1)
        break
      case "ArrowUp":
        event.preventDefault()
        this.#moveActive(-1)
        break
      case "Home":
        event.preventDefault()
        this.#setActive(0)
        break
      case "End":
        event.preventDefault()
        this.#setActive(this.#options.length - 1)
        break
      case "Enter":
        event.preventDefault()
        this.#activate(this.#options[this.#activeIndex])
        break
    }
  }

  // Internal

  async #load() {
    this.#renderLoading()
    this.inputTarget.focus()

    try {
      this.#data ??= await this.#fetch()
    } catch (error) {
      if (error.name === "AbortError") return
      this.#renderError()
      return
    }
    if (!this.element.open) return
    this.#render(this.inputTarget.value)
  }

  async #fetch() {
    this.#request?.abort()
    const controller = new AbortController()
    this.#request = controller

    try {
      const response = await fetch(this.urlValue, {
        headers: { Accept: "application/json" },
        signal: controller.signal,
      })
      if (!response.ok) throw new Error("request failed")
      return await response.json()
    } finally {
      if (this.#request === controller) this.#request = null
    }
  }

  #render(query) {
    const data = this.#data
    if (!data) return

    const sections = this.#sections(data, query.trim())
    this.listTarget.innerHTML = ""
    this.#options = []
    this.#activeIndex = -1
    this.inputTarget.removeAttribute("aria-activedescendant")

    if (sections.every(section => section.options.length === 0)) {
      this.listTarget.append(this.#emptyState(query))
      this.#announce("No matches")
      return
    }

    sections.forEach(section => {
      if (section.options.length === 0) return
      this.listTarget.append(this.#groupLabel(section.label))

      section.options.forEach(option => {
        const element = this.#optionElement(option, this.#options.length)
        this.#options.push({ ...option, element })
        this.listTarget.append(element)
      })
    })

    this.#setActive(0)
    this.#announce(`${this.#options.length} result${this.#options.length === 1 ? "" : "s"}`)
  }

  #sections(data, query) {
    if (query === "") {
      const currentId = Number(document.querySelector("meta[name='current-room-id']")?.content)
      const recentIds = this.#recentRoomIds()
      const recents = recentIds
        .map(id => data.rooms.find(room => room.id === id))
        .filter(room => room && room.id !== currentId)
      const rest = data.rooms.filter(room => !recentIds.includes(room.id))
      return [
        { label: "Recent", options: recents.map(room => this.#roomOption(room)) },
        { label: "Rooms", options: rest.map(room => this.#roomOption(room)) },
        { label: "People", options: data.people.map(person => this.#personOption(person)) },
        { label: "Threads", options: data.threads.map(thread => this.#threadOption(thread)) },
      ]
    }

    const rooms = this.#ranked(data.rooms, room => [ room.name ], query).map(room => this.#roomOption(room))
    const people = this.#ranked(data.people, person => [ person.name ], query).map(person => this.#personOption(person))
    const threads = this.#ranked(data.threads, thread => [ thread.name, thread.room_name ], query).map(thread => this.#threadOption(thread))
    return [
      { label: "Rooms", options: rooms },
      { label: "People", options: people },
      { label: "Threads", options: threads },
    ]
  }

  #roomOption(room) {
    return {
      kind: "room", id: room.id, url: room.url,
      title: room.name, detail: KIND_LABELS[room.kind] || "Room",
      icon: room.icon_name, unread: room.unread, muted: room.muted,
    }
  }

  #personOption(person) {
    return {
      kind: "person", id: person.id, url: person.dm_url, personId: person.id,
      title: person.name, detail: "Open DM", avatarUrl: person.avatar_url,
    }
  }

  #threadOption(thread) {
    return {
      kind: "thread", id: thread.id, url: thread.url,
      title: thread.name, detail: thread.room_name,
    }
  }

  // Subsequence match with contiguous and word-start bonuses; items
  // whose every haystack misses are dropped. (Math.max over only nulls
  // is 0, so the nulls filter out before the max.)
  #ranked(items, haystacksFor, query) {
    return items
      .map(item => {
        const scores = haystacksFor(item)
          .map(haystack => this.#fuzzyScore(haystack, query))
          .filter(score => score !== null)
        return { item, score: scores.length > 0 ? Math.max(...scores) : null }
      })
      .filter(match => match.score !== null)
      .sort((a, b) => b.score - a.score)
      .map(match => match.item)
  }

  #fuzzyScore(haystack, needle) {
    const hay = haystack.toLowerCase()
    let position = 0
    let score = 0
    let lastMatch = -1

    for (const character of needle.toLowerCase()) {
      const found = hay.indexOf(character, position)
      if (found === -1) return null
      if (found === lastMatch + 1) score += 2
      else score += 1
      if (found === 0 || /[\s_\-#]/.test(hay[found - 1])) score += 2
      lastMatch = found
      position = found + 1
    }
    return score
  }

  #moveActive(direction) {
    if (this.#options.length === 0) return
    this.#setActive((this.#activeIndex + direction + this.#options.length) % this.#options.length)
  }

  #setActive(index) {
    if (index < 0 || index >= this.#options.length) return
    this.#options.forEach((option, i) => option.element.setAttribute("aria-selected", String(i === index)))
    this.#activeIndex = index
    const element = this.#options[index].element
    this.inputTarget.setAttribute("aria-activedescendant", element.id)
    element.scrollIntoView({ block: "nearest" })
  }

  async #activate(option) {
    if (!option) return

    if (option.kind === "person" && !option.url) {
      const url = await this.#createDirectRoom(option.personId)
      if (!url) {
        this.#announce("Couldn’t open that conversation")
        return
      }
      option.url = url
    }

    this.element.close()
    Turbo.visit(option.url)
  }

  async #createDirectRoom(userId) {
    const response = await fetch("/rooms/directs", {
      method: "POST",
      headers: {
        Accept: "text/html",
        "Content-Type": "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
      body: JSON.stringify({ user_ids: [ userId ] }),
    }).catch(() => null)

    if (!response?.ok) return null
    return response.url
  }

  #optionElement(option, index) {
    const element = document.createElement("div")
    element.className = "quick-switcher__option"
    element.id = `quick-switcher-option-${index}`
    element.setAttribute("role", "option")
    element.setAttribute("aria-selected", "false")
    element.dataset.index = index

    if (option.avatarUrl) {
      const avatar = document.createElement("img")
      avatar.className = "quick-switcher__avatar"
      avatar.src = option.avatarUrl
      avatar.alt = ""
      avatar.width = 24
      avatar.height = 24
      element.append(avatar)
    }

    const title = document.createElement("span")
    title.className = "quick-switcher__title"
    title.textContent = option.title
    element.append(title)

    if (option.unread) {
      const badge = document.createElement("span")
      badge.className = "quick-switcher__badge"
      badge.textContent = "New"
      element.append(badge)
    }

    const detail = document.createElement("span")
    detail.className = "quick-switcher__detail"
    detail.textContent = option.muted ? `Muted ${option.detail}` : option.detail
    element.append(detail)

    if (option.muted) element.classList.add("quick-switcher__option--muted")

    element.addEventListener("click", () => this.#activate(option))
    element.addEventListener("mousemove", () => this.#setActive(index))
    return element
  }

  #groupLabel(label) {
    const element = document.createElement("div")
    element.className = "quick-switcher__group"
    element.setAttribute("role", "presentation")
    element.textContent = label
    return element
  }

  #emptyState(query) {
    const element = document.createElement("div")
    element.className = "quick-switcher__empty"
    element.textContent = query ? `No matches for “${query}”` : "Nothing to show yet"
    return element
  }

  #renderLoading() {
    this.listTarget.innerHTML = ""
    const element = document.createElement("div")
    element.className = "quick-switcher__empty"
    element.textContent = "Loading…"
    this.listTarget.append(element)
  }

  #renderError() {
    this.listTarget.innerHTML = ""
    const element = document.createElement("div")
    element.className = "quick-switcher__empty"
    element.textContent = "Couldn’t load the switcher. Close and try again."
    this.listTarget.append(element)
    this.#announce("Couldn’t load the switcher")
  }

  #announce(message) {
    this.statusTarget.textContent = message
  }

  // Recents

  #recordVisit() {
    const roomId = document.querySelector("meta[name='current-room-id']")?.content
    if (!roomId || !this.hasUserIdValue) return

    const ids = this.#recentRoomIds().filter(id => id !== Number(roomId))
    ids.unshift(Number(roomId))
    try {
      window.localStorage.setItem(this.#recentsKey, JSON.stringify(ids.slice(0, RECENT_LIMIT)))
    } catch { /* private browsing: recents simply don't persist */ }
  }

  #recentRoomIds() {
    try {
      const ids = JSON.parse(window.localStorage.getItem(this.#recentsKey) || "[]")
      return Array.isArray(ids) ? ids.filter(id => typeof id === "number") : []
    } catch {
      return []
    }
  }

  get #recentsKey() {
    return `quick-switcher-recents-${this.userIdValue}`
  }
}
