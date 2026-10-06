// A Turbo Stream render snapshots document.activeElement and restores it
// after the next animation frame. A focus move inside that window (a click,
// Tab, or script focus) is clobbered back to the pre-render element, which
// drops keyboard input: arrow keys sent to a message land in the composer
// instead. Let the in-render move win by re-applying it once the restore
// has run. Genuine focus loss (the render removed the focused element, so
// nothing moved) still restores as before.
const focusinLog = []
let focusinCount = 0

document.addEventListener("focusin", event => {
  focusinCount += 1
  focusinLog.push({ target: event.target, index: focusinCount })
  if (focusinLog.length > 10) focusinLog.shift()
})

document.addEventListener("turbo:before-stream-render", () => {
  const beforeId = document.activeElement?.id
  const startIndex = focusinCount
  if (!beforeId) return

  // Turbo restores after one animation frame; check after two, so the
  // restore (and its own focusin) has already happened.
  requestAnimationFrame(() => requestAnimationFrame(() => {
    if (document.activeElement?.id !== beforeId) return

    const move = focusinLog
      .filter(entry => entry.index > startIndex && entry.target !== document.activeElement)
      .at(-1)
    if (move?.target.isConnected) move.target.focus()
  }))
})
