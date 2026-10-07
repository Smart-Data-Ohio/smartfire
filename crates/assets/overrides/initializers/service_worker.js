// Registers the service worker on page loads and Turbo visits so the installed app
// (and any browser tab on a flaky network) gets push notifications and
// the offline shell. Registration is idempotent: the browser no-ops when
// the worker is already installed. A failed registration (an unsupported
// browser, a blocked script) never breaks the page.
// The test environment opts out (data-service-worker="false") so every
// fresh test browser does not install and claim a worker mid-page; the
// service worker tests opt back in. Turbo refreshes provisional head elements
// before turbo:load, including the worker selected for the newly signed-in user.
function reconcileServiceWorker() {
  if (document.documentElement.dataset.serviceWorker === "false" || document.documentElement.hasAttribute("data-turbo-preview")) return

  const url = document.querySelector('meta[name="service-worker-url"]')?.content
  if (url) navigator.serviceWorker.register(url, { scope: "/", updateViaCache: "none" }).catch(() => {})
}

if ("serviceWorker" in navigator) {
  window.addEventListener("load", reconcileServiceWorker)
  document.addEventListener("turbo:load", reconcileServiceWorker)
}
