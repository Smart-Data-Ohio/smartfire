// Registers the service worker on page loads and Turbo visits so the installed app
// (and any browser tab on a flaky network) gets push notifications and
// the offline shell. Registration is idempotent: the browser no-ops when
// the worker is already installed. A failed registration (an unsupported
// browser, a blocked script) never breaks the page.
// The test environment opts out (data-service-worker="false") so every
// fresh test browser does not install and claim a worker mid-page; the
// service worker tests opt back in. Turbo refreshes provisional head elements
// before turbo:load, including the worker selected for the newly signed-in user.
// Without the SPA the layout names no worker, and the classic one is registered as in Rails.
function selectedServiceWorkerUrl() {
  return document.querySelector('meta[name="service-worker-url"]')?.content || "/service-worker.js"
}

function serviceWorkerEnabled() {
  return document.documentElement.dataset.serviceWorker !== "false" && !document.documentElement.hasAttribute("data-turbo-preview")
}

function waitForStaticImages(body) {
  const decoding = []

  for (const image of body.querySelectorAll("img")) {
    // Lazy or responsive images can select deferred/private resources instead of this source.
    if (image.loading === "lazy" || image.srcset || image.closest("picture")) continue

    let url
    try {
      url = new URL(image.currentSrc || image.src, document.baseURI)
    } catch {
      continue
    }
    if (url.origin !== window.location.origin || !/^\/assets\/.+-[a-f0-9]{8,}\.[^/]+$/.test(url.pathname)) continue

    // Wait for hidden bell images before startup can replace the worker.
    decoding.push(Promise.resolve().then(() => image.decode()))
  }

  return Promise.allSettled(decoding)
}

function waitForNotificationStartup(body) {
  return Promise.allSettled(Array.from(body.querySelectorAll('[data-controller~="notifications"]'), element => {
    if (element.notificationsStartup) return element.notificationsStartup

    // The controller can still be importing when this page's load event fires.
    return new Promise(resolve => element.addEventListener("notifications:startup", event => resolve(event.detail.completion), { once: true }))
  }))
}

let reconciliation = 0

async function reconcileServiceWorker() {
  const generation = ++reconciliation
  const body = document.body
  const workerUrl = selectedServiceWorkerUrl()
  if (!body || !serviceWorkerEnabled()) return

  const controller = navigator.serviceWorker.controller
  if (controller && controller.scriptURL !== new URL(workerUrl, document.baseURI).href) {
    await waitForNotificationStartup(body)
  }
  await waitForStaticImages(body)

  if (generation !== reconciliation || document.body !== body || workerUrl !== selectedServiceWorkerUrl() || !serviceWorkerEnabled()) return

  navigator.serviceWorker.register(workerUrl, { scope: "/", updateViaCache: "none" }).catch(() => {})
}

if ("serviceWorker" in navigator) {
  window.addEventListener("load", reconcileServiceWorker)
  document.addEventListener("turbo:load", reconcileServiceWorker)
}
