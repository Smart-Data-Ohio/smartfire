require "test_helper"

class Accounts::AuditLogsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @admin = users(:david)
    @member = users(:kevin)
    @ban = AuditLog.record!(action: "user.ban", actor: @admin, target: @member,
      changes: { status: %w[ active banned ] }, ip_address: "203.0.113.7")
    @role = AuditLog.record!(action: "user.role.change", actor: @admin, target: @member,
      changes: { role: %w[ member administrator ] })
  end

  test "admins can browse the log" do
    sign_in :david
    get account_audit_log_url

    assert_response :success
    assert_select "h1", text: "Audit log"
    assert_select "tbody td code", text: "user.ban"
    assert_select "tbody td code", text: "user.role.change"
    assert_select "tbody td", text: "Kevin <kevin@37signals.com>"
    assert_select "a[href='#{account_audit_log_path(format: :csv)}']", text: "Export CSV"
  end

  test "members are forbidden" do
    sign_in :kevin
    get account_audit_log_url

    assert_response :forbidden
  end

  test "visitors are sent to sign in" do
    get account_audit_log_url

    assert_redirected_to new_session_url
  end

  test "visitors cannot export CSV" do
    get account_audit_log_url(format: :csv)

    assert_redirected_to new_session_url
  end

  test "members cannot export CSV" do
    sign_in :kevin
    get account_audit_log_url(format: :csv)

    assert_response :forbidden
  end

  test "filtering by actor matches names and emails in labels" do
    sign_in :david
    AuditLog.record!(action: "user.ban", actor: users(:jason), target: @member)

    get account_audit_log_url(actor: "jason@37signals.com")

    assert_response :success
    assert_select "tbody tr", count: 1
    assert_select "tbody td", text: "Jason <jason@37signals.com>"
  end

  test "filtering by action and target type" do
    sign_in :david
    AuditLog.record!(action: "room.create", actor: @admin, target: rooms(:watercooler))

    get account_audit_log_url(audit_action: "room.create", target_type: "Room")

    assert_response :success
    assert_select "tbody tr", count: 1
    assert_select "tbody td code", text: "room.create"

    get account_audit_log_url(audit_action: "room.create", target_type: "User")

    assert_response :success
    assert_select "tbody tr", count: 0
  end

  test "unknown filter values are ignored" do
    sign_in :david

    get account_audit_log_url(audit_action: "room.nuke", target_type: "Spaceship")

    assert_response :success
    assert_select "tbody td code", text: "user.ban"
    assert_select "tbody td code", text: "user.role.change"
  end

  test "filtering by date range" do
    sign_in :david
    AuditLog.where(id: @ban.id).update_all(created_at: 10.days.ago)

    get account_audit_log_url(from: 5.days.ago.to_date.to_s)

    assert_response :success
    assert_select "tbody td code", text: "user.ban", count: 0
    assert_select "tbody td code", text: "user.role.change"

    get account_audit_log_url(to: 5.days.ago.to_date.to_s)

    assert_select "tbody td code", text: "user.ban"
    assert_select "tbody td code", text: "user.role.change", count: 0
  end

  test "paging walks older entries" do
    sign_in :david
    60.times do |index|
      AuditLog.record!(action: "user.ban", actor: @admin, target: @member,
        changes: { n: index }, ip_address: "10.0.0.#{index % 250 + 1}")
    end

    get account_audit_log_url

    assert_response :success
    assert_match "Older entries", response.body
    assert_select "tbody tr", count: Accounts::AuditLogsController::PAGE_SIZE

    get account_audit_log_url(page: 2)

    assert_response :success
    assert_match "Newest entries", response.body
  end

  test "CSV export carries headers and the filtered rows" do
    sign_in :david

    get account_audit_log_url(format: :csv, audit_action: "user.ban")

    assert_response :success
    assert_equal "text/csv", response.media_type
    rows = CSV.parse(response.body, headers: true)
    assert_equal %w[ time action actor target_type target changes ip_address user_agent ], rows.headers
    assert_equal 1, rows.length
    assert_equal "user.ban", rows[0]["action"]
    assert_equal "David <david@37signals.com>", rows[0]["actor"]
    assert_equal "User", rows[0]["target_type"]
    assert_equal "Kevin <kevin@37signals.com>", rows[0]["target"]
    assert_equal "203.0.113.7", rows[0]["ip_address"]
    assert_equal({ "status" => %w[ active banned ] }, JSON.parse(rows[0]["changes"]))
  end

  test "CSV export neutralizes formula injection" do
    sign_in :david
    # A hostile display name lands in the label snapshot at record time.
    @member.update!(name: "=cmd|'/c calc'!A0")
    AuditLog.record!(action: "user.ban", actor: @admin, target: @member)

    get account_audit_log_url(format: :csv)

    rows = CSV.parse(response.body, headers: true)
    targets = rows.map { |row| row["target"] }
    assert targets.any? { |target| target.start_with?("'=") }, "expected a quoted formula cell in #{targets.inspect}"
  end

  test "past the export cap the page warns and the CSV filename says truncated" do
    sign_in :david
    with_csv_export_limit(2) do
      AuditLog.record!(action: "user.ban", actor: @admin, target: @member)

      get account_audit_log_url
      assert_response :success
      assert_match "newest 2", response.body

      get account_audit_log_url(format: :csv)
      assert_response :success
      assert_match "truncated-to-2", response.headers["Content-Disposition"]
      assert_equal 2, CSV.parse(response.body, headers: true).length
    end
  end

  test "within the export cap there is no truncation notice" do
    sign_in :david
    with_csv_export_limit(5) do
      get account_audit_log_url
      assert_response :success
      assert_no_match "newest 5", response.body

      get account_audit_log_url(format: :csv)
      assert_response :success
      assert_no_match "truncated", response.headers["Content-Disposition"]
      # The two setup rows plus the sign-in success row, all exported.
      assert_equal 3, CSV.parse(response.body, headers: true).length
    end
  end

  test "CSV export neutralizes formula injection in request columns" do
    sign_in :david
    # The user agent is fully attacker-controlled (any sign-in attempt
    # stores it); it must not reach the export as a live formula.
    AuditLog.record!(action: "session.sign_in.failure", actor_label: "mallory@evil.example",
      ip_address: "198.51.100.9", user_agent: "=cmd|'/c calc'!A0")

    get account_audit_log_url(format: :csv, audit_action: "session.sign_in.failure")

    rows = CSV.parse(response.body, headers: true)
    assert_equal 1, rows.length
    assert_equal "'=cmd|'/c calc'!A0", rows[0]["user_agent"]
  end

  private
    def with_csv_export_limit(limit)
      controller = Accounts::AuditLogsController
      original = controller::CSV_EXPORT_LIMIT
      controller.send(:remove_const, :CSV_EXPORT_LIMIT)
      controller.const_set(:CSV_EXPORT_LIMIT, limit)
      yield
    ensure
      controller.send(:remove_const, :CSV_EXPORT_LIMIT)
      controller.const_set(:CSV_EXPORT_LIMIT, original)
    end
end
