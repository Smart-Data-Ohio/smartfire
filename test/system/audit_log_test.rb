require "application_system_test_case"

class AuditLogTest < ApplicationSystemTestCase
  setup do
    page.current_window.resize_to(1440, 1000)
    sign_in "david@37signals.com"
  end

  teardown do
    page.current_window.resize_to(1400, 1400)
  end

  test "admin browses filters and exports the audit log" do
    AuditLog.record!(action: "user.ban", actor: users(:david), target: users(:kevin),
      changes: { status: %w[ active banned ] })
    AuditLog.record!(action: "room.create", actor: users(:david), target: rooms(:watercooler),
      changes: { name: "All Talk" })

    visit account_audit_log_path

    assert_selector "h1", text: "Audit log"
    assert_selector "tbody tr", count: 2
    assert_selector "tbody td code", text: "user.ban"
    assert_selector "tbody td", text: "Kevin <kevin@37signals.com>"

    select "user.ban", from: "Action"
    click_on "Filter"

    assert_selector "tbody tr", count: 1
    assert_selector "tbody td code", text: "user.ban"

    export = find_link("Export CSV")[:href]
    assert_includes export, "audit_log.csv"
    assert_includes export, "audit_action=user.ban"
  end

  test "audit log stays usable at phone width" do
    AuditLog.record!(action: "user.ban", actor: users(:david), target: users(:kevin))
    page.current_window.resize_to(390, 844)

    visit account_audit_log_path

    assert_selector "h1", text: "Audit log"
    assert_field "Actor"
    assert_selector "tbody td code", text: "user.ban"
    # The table scrolls inside its region instead of overflowing the page.
    assert page.evaluate_script("document.documentElement.scrollWidth <= window.innerWidth + 1")
  end
end
