// Live-region silencing for message lists. Paginated history, edits, and
// replacements must not be announced; only genuinely new messages reach
// the screen reader.
//
// Silencing sets aria-live="off" plus aria-busy="true", and restores after
// a repaint plus a short delay. A microtask restore would run before the
// accessibility tree processes the insert, announcing a whole page of
// history; the delayed restore keeps the quiet state in force until the
// insert has been processed. Overlapping silences nest: the region comes
// back once the last one ends.
const states = new WeakMap()

export function silenceLiveRegion(container) {
  const region = container.hasAttribute("aria-live")
    ? container
    : container.parentElement?.closest("[aria-live]")
  if (!region) return () => {}

  let state = states.get(region)
  if (!state) {
    state = { count: 0, value: "polite" }
    states.set(region, state)
  }
  if (state.count === 0) {
    state.value = region.getAttribute("aria-live") || "polite"
    region.setAttribute("aria-live", "off")
  }
  state.count += 1
  region.setAttribute("aria-busy", "true")

  let released = false
  return () => {
    if (released) return
    released = true

    const restore = () => {
      state.count -= 1
      if (state.count > 0) return
      state.count = 0
      region.setAttribute("aria-live", state.value)
      region.removeAttribute("aria-busy")
    }

    if (window.requestAnimationFrame) window.requestAnimationFrame(() => setTimeout(restore, 50))
    else setTimeout(restore, 50)
  }
}
