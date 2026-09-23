require "test_helper"

class AuditLogsHelperTest < ActionView::TestCase
  tests AuditLogsHelper

  test "marked pairs render as before arrow after" do
    assert_equal "role: member → administrator",
      audit_changes_summary({ "role" => { "before" => "member", "after" => "administrator" } })
    assert_equal "role: member → administrator",
      audit_changes_summary({ role: AuditLog.pair("member", "administrator") })
    assert_equal "provider: ∅ → openai",
      audit_changes_summary({ "provider" => { "before" => nil, "after" => "openai" } })
  end

  test "plain two-element arrays render as lists, not pairs" do
    assert_equal 'granted: ["Alice","Bob"]',
      audit_changes_summary({ "granted" => [ "Alice", "Bob" ] })
  end

  test "other hashes render as JSON, not pairs" do
    assert_equal 'endpoint: {"origin":"https://example.com","digest":"abc123"}',
      audit_changes_summary({ "endpoint" => { "origin" => "https://example.com", "digest" => "abc123" } })
  end

  test "scalars render as-is and blank payloads render a dash" do
    assert_equal "method: password", audit_changes_summary({ "method" => "password" })
    assert_equal "—", audit_changes_summary({})
    assert_equal "—", audit_changes_summary(nil)
  end
end
