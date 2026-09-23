require "test_helper"

class WorkspaceIconsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @icon = create_workspace_icon(name: "acme", title: "Acme Corp")
  end

  test "serves an SVG with the documented headers" do
    sign_in :jz

    get workspace_icon_url("acme")

    assert_response :success
    assert_equal "image/svg+xml", response.media_type
    assert_equal "inline", response.headers["Content-Disposition"].split(";").first
    assert_equal "nosniff", response.headers["X-Content-Type-Options"]
    assert_equal "default-src 'none'; style-src 'unsafe-inline'", response.headers["Content-Security-Policy"]
    assert_equal "max-age=3600, private", response.headers["Cache-Control"]
    assert_equal %("#{@icon.image.blob.checksum}"), response.headers["ETag"]
    assert_equal @icon.image.blob.download, response.body
  end

  test "serves a PNG without the SVG-only headers" do
    create_workspace_icon(name: "pixel", file: "square_64.png")
    sign_in :jz

    get workspace_icon_url("pixel")

    assert_response :success
    assert_equal "image/png", response.media_type
    assert_equal "max-age=3600, private", response.headers["Cache-Control"]
    assert_equal "nosniff", response.headers["X-Content-Type-Options"]
    assert_nil response.headers["Content-Security-Policy"]
  end

  test "supports conditional GETs with the blob checksum" do
    sign_in :jz

    get workspace_icon_url("acme"), headers: { "If-None-Match" => %("#{@icon.image.blob.checksum}") }

    assert_response :not_modified
    assert_empty response.body
  end

  test "returns not found for unknown names" do
    sign_in :jz

    get workspace_icon_url("nope_not_real")

    assert_response :not_found
  end

  test "returns not found for signed-out users" do
    get workspace_icon_url("acme")

    assert_response :not_found
  end

  test "unenrolled sessions are sent to setup instead of served the icon" do
    post session_url, params: { email_address: users(:jz).email_address, password: "secret123456" }

    get workspace_icon_url("acme")

    assert_redirected_to two_factor_setup_url
  end

  test "stale enrolled sessions are signed out instead of served the icon" do
    post session_url, params: { email_address: users(:jz).email_address, password: "secret123456" }
    token = parsed_cookies.signed[:session_token]
    enroll_two_factor!(users(:jz))

    get workspace_icon_url("acme")

    assert_redirected_to new_session_url
    assert_nil Session.find_by(token: token)
  end
end
