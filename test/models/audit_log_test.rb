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

  test "record_sign_in_failure! throttles repeat failures from one IP" do
    request = ActionDispatch::TestRequest.create
    request.remote_addr = "198.51.100.9"

    first = AuditLog.record_sign_in_failure!(email: "victim@example.com", method: "password", request: request)
    second = AuditLog.record_sign_in_failure!(email: "other@example.com", method: "password", request: request)

    assert first.persisted?
    assert_equal "victim@example.com", first.actor_label
    assert_nil first.actor_id
    assert_equal({ "method" => "password" }, first.details)
    assert_nil second
    assert_equal 1, AuditLog.where(action: "session.sign_in.failure").count
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
