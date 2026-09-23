require "test_helper"

class AuditLog::FizzyAuditTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    sign_in :david
  end

  test "Fizzy connect and disconnect are recorded without the token" do
    stub_fizzy_identity("fizzy_pat_pasted")

    assert_difference -> { AuditLog.where(action: "fizzy.account.connect").count }, +1 do
      post fizzy_connection_url, params: { access_token: "fizzy_pat_pasted" }
    end

    connect = AuditLog.where(action: "fizzy.account.connect").last
    assert_equal users(:david).id, connect.actor_id
    assert_equal users(:david).id, connect.target_id
    assert_equal "David", connect.details["fizzy_user_name"]
    assert_equal "Smart Data", connect.details["fizzy_account_name"]
    assert_no_match "fizzy_pat_pasted", connect.details.to_json

    assert_difference -> { AuditLog.where(action: "fizzy.account.disconnect").count }, +1 do
      delete fizzy_connection_url
    end

    disconnect = AuditLog.where(action: "fizzy.account.disconnect").last
    assert_equal users(:david).id, disconnect.actor_id
    assert_equal "David", disconnect.details["fizzy_user_name"]
  end

  test "rejected Fizzy token and linkless disconnect write no rows" do
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 401, body: {}.to_json)

    assert_no_difference -> { AuditLog.where(action: %w[ fizzy.account.connect fizzy.account.disconnect ]).count } do
      post fizzy_connection_url, params: { access_token: "bad-token" }
      delete fizzy_connection_url
    end
  end
end
