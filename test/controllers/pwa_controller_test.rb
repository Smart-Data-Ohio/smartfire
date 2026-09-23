require "test_helper"

class PwaControllerTest < ActionDispatch::IntegrationTest
  test "service worker serves as JavaScript with the fetch and notification handlers" do
    get "/service-worker.js"

    assert_response :success
    assert_equal "text/javascript", response.media_type

    body = response.body
    assert_includes body, 'addEventListener("fetch"'
    assert_includes body, 'addEventListener("push"'
    assert_includes body, 'addEventListener("notificationclick"'
  end

  test "service worker caches static assets only" do
    get "/service-worker.js"

    assert_response :success

    body = response.body
    # The single cache-write gate: fingerprinted assets and the offline
    # shell. Navigations fall back to the shell without caching the
    # response itself.
    assert_includes body, 'url.pathname === OFFLINE_URL || url.pathname.startsWith("/assets/")'
    assert_includes body, "networkThenOffline"
    assert_equal 1, body.scan("cache.put").size
  end

  test "notification clicks focus an existing window before opening a new one" do
    get "/service-worker.js"

    assert_response :success

    body = response.body
    assert_includes body, "clients.matchAll"
    assert_includes body, "existing.focus()"
    assert_includes body, "clients.openWindow(url)"
  end

  test "offline shell renders signed-out with reconnect behavior" do
    get "/offline.html"

    assert_response :success
    assert_includes response.body, "You&rsquo;re offline &mdash; reconnecting&hellip;"
    assert_includes response.body, "offline-retry"
    assert_includes response.body, 'addEventListener("online"'
    assert_no_match(/session_token/, response.headers["Set-Cookie"].to_s)
  end

  test "service worker fetch and notification logic" do
    skip "node is required to drive the service worker harness" unless node_available?

    output = IO.popen([ "node", Rails.root.join("test/scripts/service_worker_harness.mjs").to_s ], err: %i[ child out ]) do |io|
      io.read
    end

    assert $?.success?, output
    assert_includes output, "all checks passed"
  end

  private
    def node_available?
      system("node --version", out: File::NULL, err: File::NULL)
    end
end
