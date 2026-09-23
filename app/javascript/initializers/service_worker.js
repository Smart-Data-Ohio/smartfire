// Registers the service worker on every page load so the installed app
// (and any browser tab on a flaky network) gets push notifications and
// the offline shell. Registration is idempotent: the browser no-ops when
// the worker is already installed. A failed registration (an unsupported
// browser, a blocked script) never breaks the page.
if ("serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("/service-worker.js").catch(() => {})
  })
}
