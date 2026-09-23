require "application_system_test_case"

class ServiceWorkerTest < ApplicationSystemTestCase
  test "the worker caches static assets and never authenticated responses" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)
    wait_for_service_worker

    # A second navigation through the controlling worker: the room HTML,
    # the sidebar frame, and every API call must pass through uncached.
    join_room rooms(:watercooler)

    urls = cached_request_urls
    assert_includes urls, "/offline.html", "expected the install handler to precache the offline shell"
    assert urls.any? { |url| url.start_with?("/assets/") }, "expected cached static assets, got: #{urls.inspect}"

    uncacheable = urls.reject { |url| url == "/offline.html" || url.start_with?("/assets/") }
    assert_empty uncacheable, "the worker must never cache authenticated HTML or API responses"
  end

  test "the offline shell renders with working retry behavior" do
    # The navigation-fallback branch itself is driven by the Node harness
    # (PwaControllerTest), because CDP-emulated offline swallows
    # navigations before the worker in headless Chrome instead of failing
    # them. Here the shell page renders for real and its retry reloads.
    visit "/offline.html"

    assert_selector "h1", text: "You’re offline — reconnecting…"
    assert_selector "#offline-retry", text: "Try again now"
    assert_selector '[role="status"]', text: "automatically when your connection returns"

    click_button "Try again now"

    assert_selector "h1", text: "You’re offline — reconnecting…"
  end

  private
    def wait_for_service_worker(timeout: 15)
      # evaluate_script does not await promises, so the page records its
      # worker state on window and Ruby polls that flag instead.
      page.document.synchronize(timeout) do
        state = service_worker_state
        raise Capybara::ExpectationNotMet, "service worker state: #{state}" unless state == "ready"
      end
    end

    def service_worker_state
      page.evaluate_script(<<~JS)
        (function () {
          if (!window.__sw_state) {
            window.__sw_state = "installing";
            navigator.serviceWorker.ready.then((registration) => {
              if (!registration.active) {
                window.__sw_state = "installing";
                return;
              }
              window.__sw_state = navigator.serviceWorker.controller ? "ready" : "unclaimed";
              if (window.__sw_state === "unclaimed") {
                // claim() lands a beat after activation; re-check shortly.
                setTimeout(() => {
                  window.__sw_state = navigator.serviceWorker.controller ? "ready" : "unclaimed";
                }, 500);
              }
            });
          }
          return window.__sw_state;
        })()
      JS
    end

    def cached_request_urls
      page.evaluate_script(<<~JS)
        (function () {
          window.__cached_urls = null;
          (async () => {
            const urls = [];
            for (const name of await caches.keys()) {
              const cache = await caches.open(name);
              for (const request of await cache.keys()) {
                urls.push(new URL(request.url).pathname);
              }
            }
            window.__cached_urls = urls;
          })();
          return null;
        })()
      JS

      page.document.synchronize(10) do
        urls = page.evaluate_script("window.__cached_urls")
        raise Capybara::ExpectationNotMet, "waiting for cache inventory" if urls.nil?

        urls
      end
    end
end
