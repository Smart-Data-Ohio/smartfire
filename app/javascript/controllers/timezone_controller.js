import { Controller } from "@hotwired/stimulus"
import { patch } from "@rails/request.js"

// Reports the browser's time zone once, when the member has none saved.
// A hand-picked zone always wins: the server ignores later detections.
export default class extends Controller {
  connect() {
    const url = document.querySelector("meta[name='time-zone-url']")?.getAttribute("content")
    if (!url || this.#savedZone() !== null) return

    const detected = this.#detectedZone()
    if (!detected) return

    patch(url, {
      body: { time_zone: detected },
      responseKind: "json"
    }).then((response) => {
      if (response.ok) this.#rememberZone(detected)
    }).catch(() => {
      // Detection is best-effort; the profile form still offers every zone.
    })
  }

  #savedZone() {
    return document.querySelector("meta[name='current-user-time-zone']")?.getAttribute("content")
  }

  #rememberZone(zone) {
    document.querySelector("meta[name='current-user-time-zone']")?.setAttribute("content", zone)
  }

  #detectedZone() {
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone
    } catch {
      return null
    }
  }
}
