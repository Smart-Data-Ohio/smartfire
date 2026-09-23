require "test_helper"

class ContentSecurityPolicyTest < ActionDispatch::IntegrationTest
  setup do
    @livekit_url = ENV["LIVEKIT_URL"]
  end

  teardown do
    ENV["LIVEKIT_URL"] = @livekit_url
  end

  test "pages carry a report-only policy with a script nonce and no unsafe script sources" do
    sign_in :david
    get room_url(rooms(:watercooler))

    assert_response :success
    assert_nil response.headers["Content-Security-Policy"], "report-only until the reports are quiet"
    policy = directives(response.headers["Content-Security-Policy-Report-Only"])

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
    first = response.headers["Content-Security-Policy-Report-Only"][/'nonce-[^']+'/]
    get user_profile_url
    assert_equal first, response.headers["Content-Security-Policy-Report-Only"][/'nonce-[^']+'/]
  end

  test "the LiveKit gateway is allowed in both its WebSocket and HTTPS forms" do
    ENV["LIVEKIT_URL"] = "wss://huddle.example.com"
    sign_in :david
    get room_url(rooms(:watercooler))

    connect_sources = directives(response.headers["Content-Security-Policy-Report-Only"])["connect-src"]
    assert_includes connect_sources, "wss://huddle.example.com"
    assert_includes connect_sources, "https://huddle.example.com"
    assert_includes connect_sources, "https://www.googleapis.com"
  end

  test "the report-only policy allows the LinkedIn embed player in a frame" do
    sign_in :david
    get room_url(rooms(:watercooler))

    assert_response :success
    policy = directives(response.headers["Content-Security-Policy-Report-Only"])

    assert_includes policy["frame-src"], "https://www.linkedin.com"
  end

  test "the event form's inline time-zone script carries the nonce" do
    sign_in :david
    get new_room_event_url(rooms(:watercooler))

    nonce = response.headers["Content-Security-Policy-Report-Only"][/'nonce-([^']+)'/, 1]
    assert_select "script[nonce=?]", nonce, minimum: 2
  end

  private
    def directives(header)
      header.split(";").map(&:strip).reject(&:empty?).to_h do |directive|
        name, *sources = directive.split(/\s+/)
        [ name, sources ]
      end
    end
end
