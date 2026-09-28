require "test_helper"

# User mapping, room targets, personal runs, date bounds, cancel, errors
# and single-flight for the Slack importer.
class SlackImport::RunLifecycleTest < ActiveSupport::TestCase
  include SlackImportTestHelper

  setup do
    @workspace = create_slack_workspace!
    @connection = create_slack_connection!(workspace: @workspace, user: users(:david))
    stub_slack_workspace!
    use_tiny_step_budget!
  end

  teardown do
    restore_step_budget!
  end

  def start_run(kind: "workspace", mode: "import", options: {})
    SlackImport.start!(workspace: @workspace, user: users(:david),
      connection: @connection, kind:, mode:, options:)
  end

  test "users map by email, placeholders fill in, guests stay deactivated" do
    returning = User.create!(name: "Robin Returner", email_address: "returning@example.com",
      status: :deactivated)
    gone_user = User.create!(name: "Gone Active", email_address: "gone@example.com")

    run = drive_import_to_completion(start_run)
    users = run.stats["users"]

    assert_equal({ "matched" => 3, "placeholders" => 2, "deactivated" => 2,
      "bots" => 2, "total" => 9 }, users)

    # Active placeholder attributes.
    jane = User.find_by!(email_address: "jane@example.com")
    assert jane.active?
    assert_equal "Engineer", jane.bio
    assert_nil jane.time_zone # invalid tz ignored

    # The connection owner maps to their own Slack id without a placeholder.
    owner_record = @workspace.import_records.find_by!(slack_kind: "user", slack_key: "UADMIN")
    assert_equal users(:david).id, owner_record.record_id
    assert_not owner_record.created_record?
    assert_nil User.find_by(email_address: "ada@example.com")

    # Email matches, including a deactivated account.
    assert_equal users(:kevin).id, @workspace.import_records
      .find_by!(slack_kind: "user", slack_key: "U002").record_id
    assert_equal returning.id, @workspace.import_records
      .find_by!(slack_kind: "user", slack_key: "URET").record_id
    assert_not @workspace.import_records.find_by!(slack_kind: "user", slack_key: "URET").created_record?

    # Deleted members never match by email.
    gone_placeholder = @workspace.import_records.find_by!(slack_kind: "user", slack_key: "UDEL1")
    assert gone_placeholder.created_record?
    assert_not_equal gone_user.id, gone_placeholder.record_id
    assert User.find(gone_placeholder.record_id).deactivated?

    # Guests, bots and Slackbot are deactivated with no email and no grants.
    guest = @workspace.import_records.find_by!(slack_kind: "user", slack_key: "UGUEST1").record
    assert guest.deactivated?
    assert_nil guest.email_address
    assert_empty guest.memberships
    assert User.find(@workspace.import_records.find_by!(slack_kind: "user", slack_key: "UBOT1").record_id).deactivated?
    slackbot = @workspace.import_records.find_by!(slack_kind: "user", slack_key: "USLACKBOT").record
    assert_equal "Slackbot", slackbot.name
    assert slackbot.deactivated?
  end

  test "mentions of mapped users outside the channel render as tokens, unknown ids fall back" do
    history = { ok: true,
      messages: [ { "type" => "message", "user" => "U002",
        "text" => "hi <@URET|robin> and <@U999|ghost>", "ts" => "1700000031.000031" } ],
      has_more: false, response_metadata: { next_cursor: "" } }
    WebMock.reset!
    stub_slack_workspace!(history_overrides: { "CPRIV" => [ history ] })

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CPRIV ] }))

    assert_equal "completed", run.status
    message = Rooms::Closed.find_by!(name: "secret").messages.sole
    # URET is mapped (users phase) but not a room member: the token
    # renders, and the renderer leaves it as text.
    assert_equal "hi @[Robin Returner] and @ghost", message.markdown_source
    assert_empty message.mentionees.to_a
  end

  test "placeholder Google-link eligibility follows the allowed domains" do
    run = with_google_domains("example.com") do
      drive_import_to_completion(start_run)
    end

    assert User.find_by!(email_address: "jane@example.com").google_email_link_allowed?
    assert_not User.find_by!(email_address: "jane@other.org").google_email_link_allowed?
    assert_equal "completed", run.status
  end

  test "channels merge into same-name rooms without touching memberships" do
    room = Rooms::Open.create!(name: "General", creator: users(:david))
    memberships_before = room.memberships.pluck(:user_id, :involvement).to_h

    run = drive_import_to_completion(start_run)

    assert_equal 1, Rooms::Open.where("LOWER(name) = 'general'").count
    record = run.records.find_by!(slack_kind: "conversation", slack_key: "CCHAN")
    assert_not record.created_record?
    assert_equal room.id, record.record_id
    assert_equal 1, run.stats["counts"]["rooms_merged"]
    after = room.memberships.pluck(:user_id, :involvement).to_h
    memberships_before.each { |user_id, involvement| assert_equal involvement, after[user_id] }
    recorded = run.records.where(slack_kind: "membership").select(:record_id)
    assert_empty Membership.where(room: room, id: recorded)
    assert_not_empty room.messages.where("markdown_source LIKE '%Hello%'")
    entry = run.stats["conversations"].find { |row| row["id"] == "CCHAN" }
    assert_equal "merge", entry["target"]["action"]
  end

  test "room targets force new, skip and explicit rooms, and reject bad ids" do
    target = Rooms::Closed.create!(name: "Target", creator: users(:david))

    run = drive_import_to_completion(start_run(options: { "room_targets" => {
      "CCHAN" => "new", "CPRIV" => "skip", "CARCH" => target.id, "NOPE" => 999_999
    } }))

    assert_equal 1, Rooms::Open.where(name: "general").count
    assert_nil Room.find_by(name: "secret")
    assert_equal target.id, run.records.find_by!(slack_kind: "conversation", slack_key: "CARCH").record_id
    entries = run.stats["conversations"].index_by { |row| row["id"] }
    assert_equal "skip", entries["CPRIV"]["target"]["action"]
    assert entries["CPRIV"]["done"]
  end

  test "room target id must be an alive Open or Closed room" do
    deleted = Rooms::Open.create!(name: "Gone", creator: users(:david))
    deleted.update!(deleted_at: Time.current)

    run = drive_import_to_completion(start_run(options: { "room_targets" => { "CCHAN" => deleted.id } }))

    assert_nil Room.find_by(name: "general")
    assert run.issues.any? { |issue| issue.level == "error" && issue.message.include?("not an alive Open or Closed room") }
    entry = run.stats["conversations"].find { |row| row["id"] == "CCHAN" }
    assert_equal "skip", entry["target"]["action"]
  end

  test "workspace runs never auto-merge a private channel by name" do
    closed = Rooms::Closed.create!(name: "secret", creator: users(:david))

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CPRIV ] }))

    assert_equal 2, Rooms::Closed.where(name: "secret").count
    record = run.records.find_by!(slack_kind: "conversation", slack_key: "CPRIV")
    assert record.created_record?
    assert_empty closed.reload.messages
    entry = run.stats["conversations"].find { |row| row["id"] == "CPRIV" }
    assert_equal "create", entry["target"]["action"]
  end

  test "workspace runs merge a private channel only into its room target" do
    target = Rooms::Closed.create!(name: "Target", creator: users(:david))

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CPRIV ],
      "room_targets" => { "CPRIV" => target.id } }))

    record = run.records.find_by!(slack_kind: "conversation", slack_key: "CPRIV")
    assert_not record.created_record?
    assert_equal target.id, record.record_id
    assert_equal 1, target.messages.count
  end

  test "personal runs never merge a private channel into an existing room" do
    stub_slack_workspace!(list: :personal)
    closed = Rooms::Closed.create!(name: "secret", creator: users(:david))
    kevin_connection = create_slack_connection!(workspace: @workspace,
      user: users(:kevin), slack_user_id: "U002")
    assert_not closed.users.include?(users(:kevin))

    run = SlackImport.start!(workspace: @workspace, user: users(:kevin),
      connection: kevin_connection, kind: "personal", mode: "import",
      options: { "conversation_ids" => %w[ CPRIV ] })
    drive_import_to_completion(run)

    assert_equal 2, Rooms::Closed.where(name: "secret").count
    record = run.records.find_by!(slack_kind: "conversation", slack_key: "CPRIV")
    assert record.created_record?
    assert_empty closed.reload.messages
  end

  test "a conversation whose mapped room was deleted is skipped with an issue" do
    first = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN CARCH ] }))
    assert_equal "completed", first.status
    Rooms::Open.find_by!(name: "general").update!(deleted_at: Time.current)

    second = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN CARCH ] }))

    assert_equal "completed", second.status
    entries = second.stats["conversations"].index_by { |row| row["id"] }
    assert_equal "skip", entries["CCHAN"]["target"]["action"]
    assert entries["CCHAN"]["done"]
    assert_equal "merge", entries["CARCH"]["target"]["action"]
    assert entries["CARCH"]["done"]
    assert second.issues.any? { |issue| issue.message == "mapped room was deleted; undo the earlier run or remove the mapping to re-import" }
  end

  test "personal runs ignore room target ids" do
    stub_slack_workspace!(list: :personal)
    target = Rooms::Closed.create!(name: "Target", creator: users(:david))
    kevin_connection = create_slack_connection!(workspace: @workspace,
      user: users(:kevin), slack_user_id: "U002")

    run = SlackImport.start!(workspace: @workspace, user: users(:kevin),
      connection: kevin_connection, kind: "personal", mode: "import",
      options: { "conversation_ids" => %w[ CPRIV ], "room_targets" => { "CPRIV" => target.id } })
    drive_import_to_completion(run)

    record = run.records.find_by!(slack_kind: "conversation", slack_key: "CPRIV")
    assert record.created_record?
    assert_not_equal target.id, record.record_id
    assert_empty target.reload.messages
  end

  test "personal run imports DMs, group DMs and private channels" do
    stub_slack_workspace!(list: :personal)

    run = drive_import_to_completion(start_run(kind: "personal"))

    assert_equal "completed", run.status
    jane = User.find_by!(email_address: "jane@example.com")

    dm = Rooms::Direct.find_for([ users(:david), jane ])
    assert_not_nil dm
    assert_equal 2, dm.messages.count
    assert_equal users(:david), dm.creator

    group = Rooms::Direct.find_for([ users(:david), jane, users(:kevin) ])
    assert_not_nil group
    roots = group.messages.where(thread_id: nil).order(:created_at).to_a
    assert_equal 2, roots.size
    assert_equal roots.first.id, roots.second.reply_to_message_id
    assert_empty group.channel_threads

    secret = Rooms::Closed.find_by!(name: "secret")
    assert_equal 1, secret.messages.count
    assert_equal 3, run.stats["conversations"].size
  end

  test "personal run skips self DMs and Slackbot DMs" do
    list = JSON.parse(slack_fixture("conversations_personal.json"))
    list["channels"] += [
      { "id" => "DSELF", "is_im" => true, "is_private" => true, "user" => "UADMIN", "num_members" => 1 },
      { "id" => "DBOT", "is_im" => true, "is_private" => true, "user" => "USLACKBOT", "num_members" => 2 }
    ]
    stub_slack_workspace!(list: :personal, list_body: JSON.generate(list),
      members_overrides: {
        "DSELF" => { ok: true, members: %w[ UADMIN ], response_metadata: { next_cursor: "" } },
        "DBOT" => { ok: true, members: %w[ UADMIN USLACKBOT ], response_metadata: { next_cursor: "" } }
      })

    rooms_before = Room.count
    messages_before = Message.count
    run = drive_import_to_completion(start_run(kind: "personal", options: { "conversation_ids" => %w[ DSELF DBOT ] }))

    assert_equal "completed", run.status
    assert_equal 0, run.stats["counts"]["rooms_created"]
    assert_equal rooms_before, Room.count
    assert_equal messages_before, Message.count
    entries = run.stats["conversations"].index_by { |row| row["id"] }
    assert_equal "skip", entries["DSELF"]["target"]["action"]
    assert_equal "skip", entries["DBOT"]["target"]["action"]
  end

  test "personal run dedupes a DM another member already imported" do
    stub_slack_workspace!(list: :personal)
    drive_import_to_completion(start_run(kind: "personal"))
    rooms_before = Room.count
    messages_before = Message.count

    kevin_connection = create_slack_connection!(workspace: @workspace,
      user: users(:kevin), slack_user_id: "U002")
    run = SlackImport.start!(workspace: @workspace, user: users(:kevin),
      connection: kevin_connection, kind: "personal", mode: "import", options: {})
    drive_import_to_completion(run)

    assert_equal rooms_before, Room.count
    assert_equal messages_before, Message.count
    assert_equal "completed", run.status
  end

  test "date bounds keep every row in range and are sent to Slack" do
    oldest = Time.at(1700000010.000010).utc.iso8601(6)
    latest = Time.at(1700000010.500000).utc.iso8601(6)

    run = drive_import_to_completion(start_run(options: { "oldest" => oldest, "latest" => latest }))

    assert_equal "completed", run.status
    assert_equal 1, run.stats["counts"]["messages"]
    stamp = Message.where(id: run.records.where(slack_kind: "message").select(:record_id)).sole.created_at
    assert_equal Time.at(1700000010.000010).utc.iso8601(6), stamp.utc.iso8601(6)
    assert_requested :get, "#{SLACK_API}/conversations.history",
      query: hash_including({ "channel" => "CCHAN", "oldest" => "1700000010.000010",
        "latest" => "1700000010.500000" }), times: 2
  end

  test "cancel stops the run at the next step boundary" do
    run = start_run
    first_page = slack_fixture("history_CCHAN_p1.json")
    WebMock.reset!
    stub_slack_workspace!(history_first_responses: {
      "CCHAN" => [ ->(_request) { run.cancel! && first_page } ]
    })

    drive_import_to_completion(run)

    assert run.reload.cancelled?
    assert_not_nil run.finished_at
  end

  test "HTTP 429 reschedules the run after Retry-After" do
    WebMock.reset!
    stub_slack_workspace!(history_first_responses: {
      "CCHAN" => [ { status: 429, headers: { "Retry-After" => "5" }, body: "" },
        slack_fixture("history_CCHAN_p1.json") ]
    })

    run = start_run
    clear_enqueued_jobs
    10.times do
      break if enqueued_jobs.any? { |job| job[:job] == SlackImport::StepJob && job[:at] }
      SlackImport::StepJob.perform_now(run.id)
    end

    assert run.reload.running?
    delayed = enqueued_jobs.find { |job| job[:job] == SlackImport::StepJob && job[:at] }
    assert_not_nil delayed, "expected a delayed step job, got #{enqueued_jobs.inspect}"
    assert_in_delta Time.current.to_f + 5, delayed[:at], 60

    drive_import_to_completion(run)
    assert_equal "completed", run.status
  end

  test "auth errors fail the run and flag the connection" do
    WebMock.reset!
    stub_slack_workspace!(auth_error: "invalid_auth")

    run = start_run
    drive_import_to_completion(run)
    run.reload

    assert_equal "failed", run.status
    assert_includes run.error, "invalid_auth"
    assert_includes @connection.reload.disconnected_reason, "invalid_auth"
  end

  test "scope errors fail with the missing scope and leave the connection" do
    WebMock.reset!
    stub_slack_workspace!(history_error_body: { ok: false, error: "missing_scope",
      needed: "channels:history", provided: "channels:read" })

    run = start_run
    drive_import_to_completion(run)
    run.reload

    assert_equal "failed", run.status
    assert_includes run.error, "channels:history"
    assert_nil @connection.reload.disconnected_reason
  end

  test "transient failures past the retry budget fail the run" do
    WebMock.reset!
    stub_slack_workspace!(history_first_responses: {
      "CARCH" => [ { status: 500, body: "boom" } ]
    })

    run = drive_import_to_completion(start_run)

    assert_equal "failed", run.status
    assert_includes run.error, "500"
  end

  test "failed runs stay resumable: a new run continues from the mapping" do
    WebMock.reset!
    stub_slack_workspace!(auth_error: "invalid_auth")
    failed = drive_import_to_completion(start_run)
    assert_equal "failed", failed.status

    WebMock.reset!
    stub_slack_workspace!
    @connection.reload.update!(disconnected_reason: nil)
    run = drive_import_to_completion(start_run)

    assert_equal "completed", run.status
    assert_equal 11, run.stats["counts"]["messages"]
    assert_equal 0, run.stats["users"]["total"]
  end

  test "workspace runs exclude private channels when asked" do
    drive_import_to_completion(start_run(options: { "include_private" => false }))

    assert_requested :get, "#{SLACK_API}/conversations.list",
      query: hash_including({ "types" => "public_channel" })
  end

  test "two queued runs run one at a time" do
    first = start_run
    second = start_run
    first.update!(status: "running", heartbeat_at: Time.current)

    SlackImport::StepJob.perform_now(second.id)
    assert second.reload.queued?

    first.update!(status: "completed", finished_at: Time.current)
    drive_import_to_completion(second)
    assert_equal "completed", second.status
  end
end
