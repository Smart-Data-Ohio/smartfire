require "test_helper"

class AuditLog::SignInAuditTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper

  test "password sign-in success is recorded with actor and method" do
    assert_difference -> { AuditLog.where(action: "session.sign_in.success").count }, +1 do
      post session_url, params: { email_address: "david@37signals.com", password: "secret123456" }
    end

    entry = AuditLog.where(action: "session.sign_in.success").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal "David <david@37signals.com>", entry.actor_label
    assert_equal({ "method" => "password" }, entry.details)
    assert entry.ip_address.present?
  end

  test "password sign-in failure is recorded without an actor" do
    assert_difference -> { AuditLog.where(action: "session.sign_in.failure").count }, +1 do
      post session_url, params: { email_address: "david@37signals.com", password: "wrong" }
    end

    assert_response :unauthorized
    entry = AuditLog.where(action: "session.sign_in.failure").last
    assert_nil entry.actor_id
    assert_equal "david@37signals.com", entry.actor_label
    assert_equal({ "method" => "password" }, entry.details)
  end

  test "a password typed into the email field is not stored" do
    post session_url, params: { email_address: "correct horse battery staple", password: "wrong" }

    assert_response :unauthorized
    entry = AuditLog.where(action: "session.sign_in.failure").last
    assert_equal "[unrecognized]", entry.actor_label
    assert_no_match "correct horse", entry.attributes.values.join(" ")
  end

  test "sign-in failures from one IP collapse to a single row" do
    3.times do
      post session_url, params: { email_address: "david@37signals.com", password: "wrong" }
      assert_response :unauthorized
    end

    assert_equal 1, AuditLog.where(action: "session.sign_in.failure").count

    travel 6.minutes do
      post session_url, params: { email_address: "david@37signals.com", password: "wrong" }
    end

    assert_equal 2, AuditLog.where(action: "session.sign_in.failure").count
  end

  test "transfer sign-in success and failure are recorded" do
    put session_transfer_url(users(:david).transfer_id)
    assert_redirected_to root_url

    success = AuditLog.where(action: "session.sign_in.success").last
    assert_equal users(:david).id, success.actor_id
    assert_equal({ "method" => "transfer" }, success.details)

    put session_transfer_url("bogus-transfer-id")
    assert_response :bad_request

    failure = AuditLog.where(action: "session.sign_in.failure").last
    assert_equal "[unrecognized]", failure.actor_label
    assert_equal({ "method" => "transfer" }, failure.details)
    assert_no_match "bogus-transfer-id", failure.details.to_json
  end

  test "Google sign-in success is recorded" do
    state = start_google_sign_in

    assert_difference -> { AuditLog.where(action: "session.sign_in.success").count }, +1 do
      complete_google_sign_in(state:, email: "audited@smartdata.net", hd: "smartdata.net", sub: "google-sub-audited")
    end

    assert_redirected_to root_url
    entry = AuditLog.where(action: "session.sign_in.success").last
    assert_equal User.find_by!(email_address: "audited@smartdata.net").id, entry.actor_id
    assert_equal({ "method" => "google" }, entry.details)
  end

  test "rejected Google sign-in is recorded as a failure" do
    state = start_google_sign_in

    assert_difference -> { AuditLog.where(action: "session.sign_in.failure").count }, +1 do
      complete_google_sign_in(state:, email: "mallory@evil.example", hd: "evil.example", sub: "google-sub-mallory")
    end

    entry = AuditLog.where(action: "session.sign_in.failure").last
    assert_equal "[unrecognized]", entry.actor_label
    assert_equal({ "method" => "google" }, entry.details)
  end
end
