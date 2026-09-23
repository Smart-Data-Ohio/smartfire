// Registers the service worker on every page load so the installed app
// (and any browser tab on a flaky network) gets push notifications and
// the offline shell. Registration is idempotent: the browser no-ops when
// the worker is already installed. A failed registration (an unsupported
// browser, a blocked script) never breaks the page.
// The test environment opts out (data-service-worker="false") so every
// fresh test browser does not install and claim a worker mid-page; the
// service worker tests opt back in.
if ("serviceWorker" in navigator && document.documentElement.dataset.serviceWorker !== "false") {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("/service-worker.js").catch(() => {})
  })
}
