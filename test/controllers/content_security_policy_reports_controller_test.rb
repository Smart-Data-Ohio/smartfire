require "test_helper"

class ContentSecurityPolicyReportsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @log = StringIO.new
    @logger = Rails.logger
    Rails.logger = ActiveSupport::Logger.new(@log)
    ContentSecurityPolicyReportsController::RATE_LIMIT_STORE.clear
  end

  teardown do
    Rails.logger = @logger
  end

  test "logs a report-uri violation without its query string, unauthenticated and without CSRF" do
    post_report({ "csp-report" => {
      "document-uri" => "https://chat.example.com/rooms/1?token=secret-value",
      "effective-directive" => "script-src-elem",
      "blocked-uri" => "https://evil.example.com/x.js?session=abc",
      "line-number" => 12
    } }, content_type: "application/csp-report")

    assert_response :no_content
    assert_match "CSP violation: directive=script-src-elem blocked=https://evil.example.com document=/rooms/1 line=12", @log.string
    assert_no_match "secret-value", @log.string
    assert_no_match "session=abc", @log.string
  end

  test "logs Reporting API batches" do
    post_report([ { "type" => "csp-violation", "body" => {
      "documentURL" => "https://chat.example.com/account/edit", "effectiveDirective" => "img-src", "blockedURL" => "inline"
    } } ], content_type: "application/reports+json")

    assert_response :no_content
    assert_match "directive=img-src blocked=inline document=/account/edit", @log.string
  end

  test "ignores malformed and oversized bodies" do
    post content_security_policy_reports_url, params: "not json", headers: { "Content-Type" => "application/csp-report" }
    assert_response :no_content

    post_report({ "csp-report" => { "effective-directive" => "x" * 20.kilobytes } }, content_type: "application/csp-report")
    assert_response :no_content
    assert_no_match "CSP violation", @log.string
  end

  test "rate-limits reports per client" do
    statuses = 25.times.map do
      post_report({ "csp-report" => { "effective-directive" => "img-src" } }, content_type: "application/csp-report")
      response.status
    end

    assert_equal 20, statuses.count(204)
    assert_equal 429, statuses.last
  end

  private
    def post_report(payload, content_type:)
      post content_security_policy_reports_url, params: payload.to_json, headers: { "Content-Type" => content_type }
    end
end
