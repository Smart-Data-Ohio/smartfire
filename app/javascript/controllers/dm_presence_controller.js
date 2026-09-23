import { Controller } from "@hotwired/stimulus"

const REFRESH_INTERVAL = 60 * 1000
const PRESENCE_LABELS = {
  online: "Online",
  idle: "Idle",
  dnd: "Do not disturb",
  agent: "Agent",
  offline: "Offline"
}

// Paints presence dots on cached DM rows. The rows render an offline dot
// with the other member's id; this fetches live presence for the visible
// ids in one request and updates each dot without busting row caches.
export default class extends Controller {
  static values = { url: String }

  connect() {
    this.refresh()
    this.refreshTimer = window.setInterval(() => this.refresh(), REFRESH_INTERVAL)
  }

  disconnect() {
    window.clearInterval(this.refreshTimer)
    this.refreshTimer = null
    this.request?.abort()
    this.request = null
  }

  refresh() {
    if (document.visibilityState !== "visible" || !this.hasUrlValue) return

    const dots = Array.from(this.element.querySelectorAll("[data-dm-presence-user-id]"))
    if (dots.length === 0) return

    const ids = [ ...new Set(dots.map((dot) => dot.dataset.dmPresenceUserId)) ]
    this.request?.abort()
    const request = new AbortController()
    this.request = request

    const params = new URLSearchParams()
    ids.forEach((id) => params.append("ids[]", id))

    fetch(`${this.urlValue}?${params}`, { headers: { Accept: "application/json" }, signal: request.signal })
      .then((response) => {
        if (!response.ok) throw new Error(`Presence lookup failed (${response.status})`)
        return response.json()
      })
      .then((payload) => {
        if (request !== this.request) return
        this.#paint(dots, payload.presences || [])
      })
      .catch((error) => {
        if (error.name !== "AbortError") {
          // Dots keep their last (or default offline) state; the next
          // refresh retries.
        }
      })
      .finally(() => {
        if (request === this.request) this.request = null
      })
  }

  #paint(dots, presences) {
    const byId = new Map(presences.map((entry) => [ String(entry.id), entry ]))

    dots.forEach((dot) => {
      const entry = byId.get(dot.dataset.dmPresenceUserId)
      if (!entry || typeof entry.presence !== "string") return

      const label = PRESENCE_LABELS[entry.presence] || entry.presence
      dot.dataset.presence = entry.presence
      dot.setAttribute("aria-label", label)
      if (typeof entry.status === "string" && entry.status.length > 0) {
        dot.title = entry.status
      } else {
        dot.removeAttribute("title")
      }
    })
  }
}
