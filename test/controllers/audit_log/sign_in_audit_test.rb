require "test_helper"
require "minitest/mock"

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

  test "rate-limited sign-in attempts render 429 and collapse into the failure row" do
    counting = ActiveSupport::Cache::MemoryStore.new
    # The limiter captured Rails.cache (the null store in tests) at boot,
    # so swapping Rails.cache cannot reach it; forward increments instead.
    forwarder = proc { |*args, **kwargs| counting.increment(*args, **kwargs) }

    Rails.cache.stub(:increment, forwarder) do
      10.times do
        post session_url, params: { email_address: "david@37signals.com", password: "wrong" }
        assert_response :unauthorized
      end

      post session_url, params: { email_address: "david@37signals.com", password: "wrong" }
      assert_response :too_many_requests
    end

    assert_equal 1, AuditLog.where(action: "session.sign_in.failure").count
    assert_equal({ "method" => "password" }, AuditLog.where(action: "session.sign_in.failure").last.details)
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

  test "Google sign-in that provisions a user records the creation" do
    state = start_google_sign_in

    assert_difference -> { AuditLog.where(action: "user.create").count }, +1 do
      complete_google_sign_in(state:, email: "provisioned@smartdata.net", hd: "smartdata.net", sub: "google-sub-provisioned")
    end

    assert_redirected_to root_url
    user = User.find_by!(email_address: "provisioned@smartdata.net")
    entry = AuditLog.where(action: "user.create").last
    assert_equal user.id, entry.actor_id
    assert_equal user.id, entry.target_id
    assert_equal({ "method" => "google" }, entry.details)
  end

  test "Google sign-in that auto-links an allowed address records the link" do
    member = User.create!(name: "Linkable", email_address: "linkable@smartdata.net", google_email_link_allowed: true)
    state = start_google_sign_in

    assert_difference -> { AuditLog.where(action: "google.sign_in.link").count }, +1 do
      assert_difference -> { AuditLog.where(action: "session.sign_in.success").count }, +1 do
        complete_google_sign_in(state:, email: "linkable@smartdata.net", hd: "smartdata.net", sub: "google-sub-linkable")
      end
    end

    assert_redirected_to root_url
    entry = AuditLog.where(action: "google.sign_in.link").last
    assert_equal member.id, entry.actor_id
    assert_equal member.id, entry.target_id
    assert_equal({ "email" => "linkable@smartdata.net" }, entry.details)
  end

  test "repeat Google sign-in records no link or creation row" do
    member = User.create!(name: "Linked", email_address: "linked@smartdata.net", google_email_link_allowed: true)
    first_state = start_google_sign_in
    complete_google_sign_in(state: first_state, email: "linked@smartdata.net", hd: "smartdata.net", sub: "google-sub-linked")

    delete session_url
    second_state = start_google_sign_in

    assert_no_difference -> { AuditLog.where(action: [ "google.sign_in.link", "user.create" ]).count } do
      complete_google_sign_in(state: second_state, email: "linked@smartdata.net", hd: "smartdata.net", sub: "google-sub-linked")
    end

    assert_redirected_to root_url
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
