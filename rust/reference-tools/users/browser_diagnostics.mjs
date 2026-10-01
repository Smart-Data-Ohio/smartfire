// Preserve the first failure's runtime evidence; never retry a timed-out startup.
export function diagnostics(page) {
  const events = []
  const record = (kind, text) => { events.push({kind, text}); if (events.length > 80) events.shift() }
  page.on("pageerror", e => record("pageerror", e.stack || String(e)))
  page.on("console", m => { if (["error", "warning"].includes(m.type())) record(m.type(), m.text()) })
  page.on("requestfailed", r => record("requestfailed", `${r.method()} ${r.url()}: ${r.failure()?.errorText}`))
  page.on("response", r => { if (r.status() >= 400) record("http", `${r.status()} ${r.url()}`) })
  return async error => {
    let state
    try {
      state = await page.evaluate(() => ({
        url: location.href, ready: document.readyState,
        stimulus: Boolean(window.Stimulus),
        registered: window.Stimulus?.router?.modules?.map(m => m.identifier),
        connected: window.Stimulus?.controllers?.map(c => c.identifier),
        bodyControllers: document.body?.dataset.controller,
        modules: [...document.querySelectorAll('script[type="module"]')].map(s => s.src || s.textContent),
        importmap: document.querySelector('script[type="importmap"]')?.textContent,
      }))
    } catch (e) { state = {inspectionError: String(e)} }
    console.error("WS8br2 browser failure diagnostics: " + JSON.stringify({error: String(error), events, state}))
  }
}
