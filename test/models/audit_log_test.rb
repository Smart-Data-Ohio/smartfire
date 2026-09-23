require "test_helper"

class AuditLogTest < ActiveSupport::TestCase
  test "record! stores actor target labels changes and request context" do
    admin = users(:david)
    member = users(:kevin)

    entry = AuditLog.record!(
      action: "user.role.change", actor: admin, target: member,
      changes: { role: %w[ member administrator ] },
      ip_address: "203.0.113.7", user_agent: "TestAgent/1.0"
    )

    assert_equal "user.role.change", entry.action
    assert_equal admin.id, entry.actor_id
    assert_equal "David <david@37signals.com>", entry.actor_label
    assert_equal "User", entry.target_type
    assert_equal member.id, entry.target_id
    assert_equal "Kevin <kevin@37signals.com>", entry.target_label
    assert_equal({ "role" => %w[ member administrator ] }, entry.details)
    assert_equal "203.0.113.7", entry.ip_address
    assert_equal "TestAgent/1.0", entry.user_agent
    assert entry.created_at.present?
  end

  test "record! defaults actor and request context to Current" do
    Current.user = users(:david)

    entry = AuditLog.record!(action: "account.join_code.reset", target: Account.first)

    assert_equal users(:david).id, entry.actor_id
    assert_equal "David <david@37signals.com>", entry.actor_label
  ensure
    Current.user = nil
  end

  test "record! snapshots labels so later renames do not rewrite history" do
    member = users(:kevin)

    entry = AuditLog.record!(action: "user.ban", actor: users(:david), target: member)

    member.update!(name: "Renamed")
    assert_equal "Kevin <kevin@37signals.com>", entry.reload.target_label
  end

  test "record! stores STI targets under their base class" do
    room = rooms(:watercooler)

    entry = AuditLog.record!(action: "room.destroy", target: room)

    assert_equal "Room", entry.target_type
    assert_equal room, entry.target_record
  end

  test "target_record is nil for deleted targets" do
    icon = WorkspaceIcon.new(name: "auditdoomed", title: "Doomed", creator: users(:david))
    icon.image.attach(io: File.open(Rails.root.join("test/fixtures/files/workspace_icons/clean.svg")),
      filename: "clean.svg", content_type: "image/svg+xml")
    icon.save!

    entry = AuditLog.record!(action: "workspace_icon.destroy", target: icon)
    icon.destroy!

    assert_nil entry.target_record
    assert_equal ":auditdoomed:", entry.target_label
  end

  test "record! filters passwords tokens secrets and keys out of changes" do
    entry = AuditLog.record!(
      action: "agent.update", target: users(:bender),
      changes: {
        name: [ "Old", "New" ],
        password: "hunter2",
        current_password: "older",
        access_token: "tok_123",
        webhook_secret: "shh",
        api_key: "key-1",
        bot_key: "5-abcdef",
        session: { id: 1 },
        nested: { credentials: { token: "deep" }, keep: "yes" },
        list: [ { secret: "s" }, "plain" ]
      }
    )

    assert_equal [ "Old", "New" ], entry.details["name"]
    assert_equal "[FILTERED]", entry.details["password"]
    assert_equal "[FILTERED]", entry.details["current_password"]
    assert_equal "[FILTERED]", entry.details["access_token"]
    assert_equal "[FILTERED]", entry.details["webhook_secret"]
    assert_equal "[FILTERED]", entry.details["api_key"]
    assert_equal "[FILTERED]", entry.details["bot_key"]
    assert_equal "[FILTERED]", entry.details["session"]
    assert_equal "[FILTERED]", entry.details["nested"]["credentials"]
    assert_equal "yes", entry.details["nested"]["keep"]
    assert_equal "[FILTERED]", entry.details["list"][0]["secret"]
    assert_equal "plain", entry.details["list"][1]
  end

  test "record! filters join codes and transfer ids out of changes" do
    entry = AuditLog.record!(
      action: "account.join_code.reset", target: Account.first,
      changes: { join_code: "SECRET-CODE", transfer_id: "single-use-id", name: "Fine" }
    )

    assert_equal "[FILTERED]", entry.details["join_code"]
    assert_equal "[FILTERED]", entry.details["transfer_id"]
    assert_equal "Fine", entry.details["name"]
  end

  test "record! keeps emails readable" do
    entry = AuditLog.record!(
      action: "user.email.change", target: users(:kevin),
      changes: { email_address: [ "kevin@37signals.com", "new@example.com" ] }
    )

    assert_equal [ "kevin@37signals.com", "new@example.com" ], entry.details["email_address"]
  end

  test "record! truncates long user agents" do
    entry = AuditLog.record!(action: "user.ban", target: users(:kevin), user_agent: "a" * 600)

    assert_equal 512, entry.user_agent.length
    assert_equal "#{"a" * 509}...", entry.user_agent
  end

  test "persisted rows are readonly" do
    entry = AuditLog.record!(action: "user.ban", target: users(:kevin))

    assert_raises(ActiveRecord::ReadOnlyRecord) { entry.update!(action: "user.unban") }
    assert_raises(ActiveRecord::ReadOnlyRecord) { entry.save! }
    assert_equal "user.ban", entry.reload.action
  end

  test "destroy and delete are refused" do
    entry = AuditLog.record!(action: "user.ban", target: users(:kevin))

    assert_raises(ActiveRecord::ReadOnlyRecord) { entry.destroy }
    assert_raises(ActiveRecord::ReadOnlyRecord) { entry.delete }
    assert AuditLog.exists?(entry.id)
  end

  test "record_sign_in_failure! collapses on IP and email label" do
    request = ActionDispatch::TestRequest.create
    request.remote_addr = "198.51.100.9"

    first = AuditLog.record_sign_in_failure!(email: "victim@example.com", method: "password", request: request)
    repeat = AuditLog.record_sign_in_failure!(email: "Victim@Example.com", method: "password", request: request)
    other = AuditLog.record_sign_in_failure!(email: "other@example.com", method: "password", request: request)

    assert first.persisted?
    assert_equal "victim@example.com", first.actor_label
    assert_nil first.actor_id
    assert_equal({ "method" => "password" }, first.details)
    assert_nil repeat
    assert other.persisted?
    assert_equal 2, AuditLog.where(action: "session.sign_in.failure").count
  end

  test "record_sign_in_failure! caps rows per IP and counts suppressed failures" do
    request = ActionDispatch::TestRequest.create
    request.remote_addr = "198.51.100.9"

    20.times do |index|
      AuditLog.record_sign_in_failure!(email: "target#{index}@example.com", method: "password", request: request)
    end
    assert_equal 20, AuditLog.where(action: "session.sign_in.failure").count

    capped = AuditLog.record_sign_in_failure!(email: "target20@example.com", method: "password", request: request)
    assert_equal 20, AuditLog.where(action: "session.sign_in.failure").count
    assert_equal "target19@example.com", capped.actor_label
    assert_equal "password", capped.details["method"]
    assert_equal 1, capped.details["suppressed_count"]

    capped = AuditLog.record_sign_in_failure!(email: "target21@example.com", method: "password", request: request)
    assert_equal 20, AuditLog.where(action: "session.sign_in.failure").count
    assert_equal 2, capped.details["suppressed_count"]

    travel 6.minutes do
      fresh = AuditLog.record_sign_in_failure!(email: "target22@example.com", method: "password", request: request)
      assert_equal 21, AuditLog.where(action: "session.sign_in.failure").count
      assert_nil fresh.details["suppressed_count"]
    end
  end

  test "record_sign_in_failure! stores emails but never a mistyped password" do
    assert_equal "victim@example.com",
      AuditLog.record_sign_in_failure!(email: "victim@example.com", method: "password").actor_label
    assert_equal "DAVID@37signals.com",
      AuditLog.record_sign_in_failure!(email: "DAVID@37signals.com", method: "password").actor_label
    assert_equal "[unrecognized]",
      AuditLog.record_sign_in_failure!(email: "hunter2", method: "password").actor_label
    assert_equal "[unrecognized]",
      AuditLog.record_sign_in_failure!(email: "", method: "google").actor_label

    User.create!(name: "Dotless", email_address: "dotless@intranet")
    assert_equal "dotless@intranet",
      AuditLog.record_sign_in_failure!(email: "dotless@intranet", method: "password").actor_label

    long = "#{"a" * 300}@example.com"
    assert_equal 254, AuditLog.record_sign_in_failure!(email: long, method: "password").actor_label.length
  end

  test "webhook_origin_summary keeps the origin and a digest, not the URL" do
    assert_nil AuditLog.webhook_origin_summary(nil)

    summary = AuditLog.webhook_origin_summary("https://example.com/hook?token=s3cret")
    assert_equal "https://example.com", summary[:origin]
    assert_equal Digest::SHA256.hexdigest("https://example.com/hook?token=s3cret")[0, 12], summary[:digest]

    assert_equal "http://example.com:3000",
      AuditLog.webhook_origin_summary("http://example.com:3000/hook")[:origin]
    assert_equal "https://example.com",
      AuditLog.webhook_origin_summary("https://example.com:443/hook")[:origin]
    assert_equal "[invalid]", AuditLog.webhook_origin_summary("::::")[:origin]
    assert_not_equal AuditLog.webhook_origin_summary("https://example.com/a")[:digest],
      AuditLog.webhook_origin_summary("https://example.com/b")[:digest]
  end

  test "record_sign_in_failure! records again from another IP or after the window" do
    first_request = ActionDispatch::TestRequest.create
    first_request.remote_addr = "198.51.100.9"
    second_request = ActionDispatch::TestRequest.create
    second_request.remote_addr = "198.51.100.10"

    AuditLog.record_sign_in_failure!(email: "a@example.com", method: "password", request: first_request)
    AuditLog.record_sign_in_failure!(email: "a@example.com", method: "password", request: second_request)

    travel 6.minutes do
      AuditLog.record_sign_in_failure!(email: "a@example.com", method: "password", request: first_request)
    end

    assert_equal 3, AuditLog.where(action: "session.sign_in.failure").count
  end
end
