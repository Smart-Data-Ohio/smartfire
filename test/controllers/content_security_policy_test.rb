require "test_helper"

class ContentSecurityPolicyTest < ActionDispatch::IntegrationTest
  setup do
    @livekit_url = ENV["LIVEKIT_URL"]
  end

  teardown do
    ENV["LIVEKIT_URL"] = @livekit_url
  end

  test "pages carry an enforced policy with a script nonce and no unsafe script sources" do
    sign_in :david
    get room_url(rooms(:watercooler))

    assert_response :success
    assert_nil response.headers["Content-Security-Policy-Report-Only"], "the policy is enforced, not report-only"
    policy = directives(response.headers["Content-Security-Policy"])

    assert_equal [ "'none'" ], policy["object-src"]
    assert_equal [ "'self'" ], policy["base-uri"]
    assert_includes policy["report-uri"], "/csp_reports"

    script_sources = policy["script-src"]
    assert_includes script_sources, "'self'"
    assert_not_includes script_sources, "'unsafe-inline'"
    assert_not_includes script_sources, "'unsafe-eval'"
    nonce = script_sources.find { |source| source.start_with?("'nonce-") }.delete_prefix("'nonce-").delete_suffix("'")
    assert nonce.present?

    # text/template blocks are inert markup, not script.
    inline_scripts = css_select("script:not([src])").reject { |script| script["type"] == "text/template" }
    assert inline_scripts.any?, "importmap renders inline scripts"
    inline_scripts.each { |script| assert_equal nonce, script["nonce"], "every inline script carries the nonce" }
    assert_select "meta[name=csp-nonce][content=?]", nonce
  end

  test "the nonce stays stable across a session's pages for Turbo navigation" do
    sign_in :david
    get room_url(rooms(:watercooler))
    first = response.headers["Content-Security-Policy"][/'nonce-[^']+'/]
    get user_profile_url
    assert_equal first, response.headers["Content-Security-Policy"][/'nonce-[^']+'/]
  end

  test "the LiveKit gateway is allowed in both its WebSocket and HTTPS forms" do
    ENV["LIVEKIT_URL"] = "wss://huddle.example.com"
    sign_in :david
    get room_url(rooms(:watercooler))

    connect_sources = directives(response.headers["Content-Security-Policy"])["connect-src"]
    assert_includes connect_sources, "wss://huddle.example.com"
    assert_includes connect_sources, "https://huddle.example.com"
    assert_includes connect_sources, "https://www.googleapis.com"
  end

  test "the enforced policy allows the LinkedIn embed player in a frame" do
    sign_in :david
    get room_url(rooms(:watercooler))

    assert_response :success
    policy = directives(response.headers["Content-Security-Policy"])

    assert_includes policy["frame-src"], "https://www.linkedin.com"
  end

  test "form-action allows every host a sudo or form flow redirects to" do
    sign_in :david
    get room_url(rooms(:watercooler))

    form_action = directives(response.headers["Content-Security-Policy"])["form-action"]
    assert_includes form_action, "'self'"

    # Read from the same constants the redirects are built with, so a new
    # OAuth host fails here until the policy allows it: Google sign-in,
    # sudo re-auth, sign-in linking, and Calendar/Drive connect all land
    # on accounts.google.com, while GitHub App connect lands on
    # github.com (reached through a sudo-continued redirect chain, which
    # Chromium checks against form-action end to end).
    oauth_hosts = [
      Google::SignIn::AUTHORIZE_HOST, Google::Client::AUTHORIZE_HOST, Github::App::AUTHORIZE_HOST
    ].uniq
    assert oauth_hosts.many?, "expected both a Google and a GitHub OAuth host, got #{oauth_hosts.inspect}"
    oauth_hosts.each do |host|
      assert_includes form_action, "https://#{host}"
    end
  end

  test "the enforced policy allows everything the Google Picker loads" do
    sign_in :david
    get room_url(rooms(:watercooler))

    assert_response :success
    policy = directives(response.headers["Content-Security-Policy"])

    # Per Google's picker CSP guidance (googleworkspace/drive-picker-element
    # README): the API and Identity Services loaders, the picker and auth
    # frames, thumbnails, injected fonts, and the Drive API calls the
    # composer makes from the browser.
    assert_includes policy["script-src"], "https://apis.google.com"
    assert_includes policy["script-src"], "https://accounts.google.com/gsi/"
    assert_includes policy["frame-src"], "https://docs.google.com"
    assert_includes policy["frame-src"], "https://drive.google.com"
    assert_includes policy["frame-src"], "https://accounts.google.com"
    assert_includes policy["img-src"], "https://*.googleusercontent.com"
    assert_includes policy["font-src"], "https://fonts.gstatic.com"
    assert_includes policy["connect-src"], "https://www.googleapis.com"
  end

  test "the event form fills its time zone with a Stimulus controller, not an inline script" do
    sign_in :david
    get new_room_event_url(rooms(:watercooler))

    assert_response :success
    assert_select "form[data-controller~='event-time-zone']", count: 1
    assert_select "[data-event-time-zone-target='field']", count: 1
    assert_select "[data-event-time-zone-target='label']", count: 1

    # No body inline script may depend on the nonce: after a
    # Turbo-driven sign-out and sign-in the document keeps its old nonce
    # and blocks any inline script carrying the new one. (The importmap
    # tags carry types, so only a typeless inline script can be one.)
    assert_empty css_select("script:not([src]):not([type])")
  end

  private
    def directives(header)
      header.split(";").map(&:strip).reject(&:empty?).to_h do |directive|
        name, *sources = directive.split(/\s+/)
        [ name, sources ]
      end
    end
end
