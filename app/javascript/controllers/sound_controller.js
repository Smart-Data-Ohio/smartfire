import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { "url": String }

  play() {
    if (mutedByDnd()) return
    if (quietHoursActive()) return

    const sound = new Audio(this.urlValue)
    sound.play()
  }
}

// Manual DND and the DND presence come from the server-rendered tag;
// only a settings change flips them, which always re-renders the page.
function mutedByDnd() {
  return document.querySelector("meta[name='notification-dnd'][content='muted']") !== null
}

// Re-evaluated on every play from the server-rendered window, so a
// quiet-hours boundary crossing silences (or unsilences) sounds without
// a reload. Mirrors User#quiet_hours_active?: an empty window (start
// equals end) never mutes, and overnight windows wrap past midnight.
function quietHoursActive(now = new Date()) {
  const range = document.querySelector("meta[name='quiet-hours']")?.getAttribute("content")
  const zone = document.querySelector("meta[name='quiet-hours-zone']")?.getAttribute("content")
  if (!range || !zone) return false

  const match = range.match(/^(\d+)-(\d+)$/)
  if (!match) return false

  const start = Number(match[1])
  const end = Number(match[2])
  if (start === end) return false

  const minute = minutesSinceMidnight(now, zone)
  if (minute === null) return false

  return start < end
    ? minute >= start && minute < end
    : minute >= start || minute < end
}

function minutesSinceMidnight(now, zone) {
  try {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: zone, hour: "numeric", minute: "numeric", hourCycle: "h23"
    }).formatToParts(now)
    const hour = Number(parts.find((part) => part.type === "hour")?.value)
    const minute = Number(parts.find((part) => part.type === "minute")?.value)
    if (Number.isNaN(hour) || Number.isNaN(minute)) return null

    return hour * 60 + minute
  } catch {
    return null
  }
}
