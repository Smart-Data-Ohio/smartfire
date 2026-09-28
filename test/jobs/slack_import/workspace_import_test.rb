require "test_helper"

# End-to-end workspace imports against the fake Slack workspace in
# test/fixtures/files/slack, driving the real step jobs across executions.
class SlackImport::WorkspaceImportTest < ActiveSupport::TestCase
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

  def start_run(mode: "import", options: {}, kind: "workspace")
    SlackImport.start!(workspace: @workspace, user: users(:david),
      connection: @connection, kind:, mode:, options:)
  end

  def history_page(messages, cursor: nil)
    { ok: true, messages:, has_more: cursor.present?,
      response_metadata: { next_cursor: cursor.to_s } }
  end

  def slack_message(ts, text = "message #{ts}", user: "U001")
    { "type" => "message", "user" => user, "text" => text, "ts" => ts }
  end

  def table_counts(except: [])
    ActiveRecord::Base.connection.tables.to_h do |table|
      quoted = ActiveRecord::Base.connection.quote_table_name(table)
      [ table, ActiveRecord::Base.connection.select_value("SELECT COUNT(*) FROM #{quoted}") ]
    end.except(*except)
  end

  def cable_broadcasts_count
    ActionCable.server.pubsub.send(:channels_data).values.sum(&:size)
  end

  # Slack ts floats carry error past the microsecond; the database keeps
  # microseconds, so comparisons round-trip through iso8601(6).
  def assert_equal_time(expected_float, actual)
    assert_equal Time.at(expected_float).utc.iso8601(6), actual.utc.iso8601(6)
  end

  test "dry run writes nothing except the run row and its issues" do
    run = start_run(mode: "dry_run")
    before = table_counts(except: %w[ slack_imports slack_import_issues ])

    drive_import_to_completion(run)

    assert_equal before, table_counts(except: %w[ slack_imports slack_import_issues ])
    assert_equal "completed", run.status
    assert_equal "done", run.stats["phase"]
    assert_equal 3, run.stats["conversations"].size
    assert_equal 9, run.stats["users"]["total"]
    assert_operator run.stats["samples"].size, :<=, 20
    assert_not_empty run.stats["samples"]
    assert_equal 0, run.records.count
    # 1 users + 1 list + 3 members + 4 history pages: replies never read.
    assert_equal 9, run.stats["api_calls"]
  end

  test "workspace import creates rooms, memberships, messages, threads, boosts and pins" do
    run = drive_import_to_completion(start_run)

    assert_equal "completed", run.status
    counts = run.stats["counts"]
    assert_equal({ "rooms_created" => 3, "rooms_merged" => 0, "messages" => 11,
      "replies" => 2, "threads" => 1, "reactions" => 3, "pins" => 1,
      "files_linked" => 1, "skipped" => 6 }, counts)

    general = Rooms::Open.find_by!(name: "general")
    assert_equal users(:david), general.creator
    archived = Rooms::Open.find_by!(name: "old-project (archived)")
    secret = Rooms::Closed.find_by!(name: "secret")

    # Slack members keep the default involvement, other active users in a
    # created Open room go invisible, archived rooms are fully invisible.
    jane = User.find_by!(email_address: "jane@example.com")
    assert_equal "mentions", general.memberships.find_by!(user: jane).involvement
    assert_equal "mentions", general.memberships.find_by!(user: users(:david)).involvement
    assert_equal "invisible", general.memberships.find_by!(user: users(:jason)).involvement
    assert_empty archived.memberships.where.not(involvement: "invisible")
    assert_equal [ "David", "Kevin" ], secret.users.order(:name).pluck(:name)

    # Authors, timestamps with microseconds, converted markdown, mentions.
    first = general.messages.find_by!(created_at: Time.at(Rational("1700000001.000001")))
    assert_equal jane, first.creator
    assert_equal "Hello @[Kevin] and @ghost! cc @[Jane Doe]", first.markdown_source
    assert_equal [ users(:kevin) ], first.mentionees.to_a
    assert_includes first.body.body.to_html, "application/vnd.campfire.mention"

    # Duplicate and non-member mentions stay literal tokens.
    assert_includes first.markdown_source, "@[Jane Doe]"
    assert_equal 1, first.mentionees.count

    # Edited message with a file line.
    edited = general.messages.find_by!(created_at: Time.at(Rational("1700000006.000006")))
    assert_equal Time.at(Rational("1700000007.0")), edited.edited_at
    assert_includes edited.markdown_source, "📎 [spec.pdf](https://smartdata.slack.com/files/U002/F123/spec.pdf)"

    # Bot author keyed by bot_id, attachments quoted, me_message italic.
    bot_message = general.messages.find_by!(created_at: Time.at(Rational("1700000004.000004")))
    assert_equal "Build Bot", bot_message.creator.name
    assert bot_message.creator.deactivated?
    assert_includes bot_message.markdown_source, "> All green"
    me = general.messages.find_by!(created_at: Time.at(Rational("1700000007.000007")))
    assert_equal "*waves hello*", me.markdown_source

    # Special syntax: literals, handles, channels, mailto, bullets, code.
    special = general.messages.find_by!(created_at: Time.at(Rational("1700000010.000010")))
    assert_includes special.markdown_source, "@here standup in #secret with @engs"
    assert_includes special.markdown_source, "[email us](mailto:team@example.com)"
    assert_includes special.markdown_source, "- first item"
    assert_includes special.markdown_source, "`code *stays*`"
    assert_empty special.mentionees.to_a

    # Both history pages were read: the newest message (first page) and the
    # oldest (second page, since Slack pages newest-first).
    assert general.messages.exists?(created_at: Time.at(Rational("1700000011.000011")))
    assert general.messages.exists?(created_at: Time.at(Rational("1700000001.000001")))

    # Skipped subtypes left nothing behind.
    assert_empty general.messages.where("markdown_source LIKE ?", "%has joined%")
    assert_empty Message.where("markdown_source LIKE ?", "%huddle happened%")

    # Thread: default name, historical activity stamp, followers.
    parent = general.messages.find_by!(created_at: Time.at(Rational("1700000002.000002")))
    thread = parent.channel_thread
    assert_equal users(:david), thread.creator
    assert_equal 2, thread.messages_count
    assert_equal parent.plain_text_body.lines.first.strip, thread.name
    assert_equal_time 1700000102.000102, thread.last_activity_at
    assert_equal 2, thread.users.count
    assert_equal [ thread.id ], thread.messages.pluck(:thread_id).uniq
    broadcast = thread.messages.find_by!(created_at: Time.at(Rational("1700000102.000102")))
    assert_equal "Broadcast reply @[Jane Doe]", broadcast.markdown_source

    # Reactions: skin tone stripped, custom emoji kept, bad emoji an issue.
    reacted = general.messages.find_by!(created_at: Time.at(Rational("1700000005.000005")))
    assert_equal [ "👍", "👍", ":custom_thing:" ].sort, reacted.boosts.map(&:content).sort
    assert_equal users(:kevin), reacted.boosts.find_by!(content: "👍", booster: users(:kevin)).booster
    assert run.issues.any? { |issue| issue.message.include?("party-blob") }

    # Pin written directly: no system note, no broadcast.
    pinned = general.messages.find_by!(created_at: Time.at(Rational("1700000009.000009")))
    assert MessagePin.exists?(message: pinned, room: general, pinner: users(:david))
    assert_empty general.messages.where(system_note: true)

    # Room timestamps follow the last imported message, memberships point
    # at it with no unread.
    assert_equal_time 1700000102.000102, general.reload.updated_at
    last_imported = general.messages.order(created_at: :desc, id: :desc).first
    general.memberships.each do |membership|
      assert_nil membership.unread_at
      assert_equal last_imported.id, membership.last_read_message_id
    end
  end

  test "tiny step budget spans several steps for one conversation" do
    run = start_run(options: { "conversation_ids" => %w[ CCHAN ] })
    steps = 0
    while run.reload.active?
      SlackImport::StepJob.perform_now(run.id)
      steps += 1
      assert steps < 50, "run did not finish"
    end

    assert_equal "completed", run.status
    assert_operator steps, :>, 4, "expected CCHAN to span several steps, took #{steps}"
  end

  test "a second import creates zero duplicates" do
    drive_import_to_completion(start_run)
    before = table_counts(except: %w[ slack_imports slack_import_issues ])

    run = drive_import_to_completion(start_run)

    assert_equal before, table_counts(except: %w[ slack_imports slack_import_issues ])
    assert_equal 0, run.stats["counts"]["messages"]
    assert_equal 0, run.stats["counts"]["rooms_created"]
  end

  test "catch-up picks up a new message and a late reply" do
    drive_import_to_completion(start_run)

    WebMock.reset!
    parent = JSON.parse(slack_fixture("history_CCHAN_p2.json"))["messages"]
      .find { |message| message["ts"] == "1700000002.000002" }
    stub_slack_workspace!(history_overrides: {
      "CCHAN" => [
        { ok: true, messages: [
          { "type" => "message", "user" => "U001", "text" => "Fresh news",
            "ts" => "1700000099.000099" },
          parent
        ], has_more: false, response_metadata: { next_cursor: "" } }
      ]
    }, replies_overrides: {
      "CCHAN" => { ok: true, messages: [
        { "type" => "message", "user" => "U002", "text" => "parent",
          "ts" => "1700000002.000002" },
        { "type" => "message", "user" => "U001", "text" => "Late reply",
          "ts" => "1700000200.000200", "thread_ts" => "1700000002.000002" }
      ], has_more: false, response_metadata: { next_cursor: "" } }
    })

    run = drive_import_to_completion(start_run)
    counts = run.stats["counts"]

    assert_equal 1, counts["messages"]
    assert_equal 1, counts["replies"]
    thread = Rooms::Open.find_by!(name: "general").channel_threads.sole
    assert_equal 3, thread.reload.messages_count
    assert_equal_time 1700000200.000200, thread.last_activity_at
    assert_equal "done", run.stats["phase"]

    # History re-read from 30 days before the newest imported message.
    expected_oldest = format("%.6f", 1700000102.000102 - 30.days.to_f)
    assert_requested :get, "#{SLACK_API}/conversations.history",
      query: hash_including({ "channel" => "CCHAN", "oldest" => expected_oldest })
  end

  test "catch-up skips a thread whose mapped thread was deleted" do
    drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    general = Rooms::Open.find_by!(name: "general")
    parent = general.messages.find_by!(created_at: Time.at(Rational("1700000002.000002")))
    assert_not_nil parent.channel_thread
    parent.channel_thread.destroy!

    WebMock.reset!
    stub_slack_workspace!(history_overrides: {
      "CCHAN" => [
        { ok: true, messages: [
          { "type" => "message", "user" => "U001", "text" => "Fresh news",
            "ts" => "1700000099.000099" },
          JSON.parse(slack_fixture("history_CCHAN_p2.json"))["messages"]
            .find { |message| message["ts"] == "1700000002.000002" }
        ], has_more: false, response_metadata: { next_cursor: "" } }
      ]
    }, replies_overrides: {
      "CCHAN" => { ok: true, messages: [
        { "type" => "message", "user" => "U002", "text" => "parent",
          "ts" => "1700000002.000002" },
        { "type" => "message", "user" => "U001", "text" => "Late reply",
          "ts" => "1700000200.000200", "thread_ts" => "1700000002.000002" }
      ], has_more: false, response_metadata: { next_cursor: "" } }
    })

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", run.status
    assert_equal 1, run.stats["counts"]["messages"]
    assert_equal 0, run.stats["counts"]["replies"]
    assert_empty general.messages.where(created_at: Time.at(Rational("1700000200.000200")))
    assert_equal 1, run.issues.count { |issue| issue.message.include?("was deleted") }
  end

  test "import skips a thread whose parent message was deleted mid-run" do
    parent_ts = "1700000002.000002"
    reply_ts = "1700000101.000101"
    WebMock.reset!
    stub_slack_workspace!(history_overrides: {
      "CCHAN" => [ { ok: true, messages: [
        { "type" => "message", "user" => "U001", "text" => "parent",
          "ts" => parent_ts, "reply_count" => 1 }
      ], has_more: false, response_metadata: { next_cursor: "" } } ]
    }, replies_overrides: {
      "CCHAN" => { ok: true, messages: [
        { "type" => "message", "user" => "U001", "text" => "parent", "ts" => parent_ts },
        { "type" => "message", "user" => "U002", "text" => "reply",
          "ts" => reply_ts, "thread_ts" => parent_ts }
      ], has_more: false, response_metadata: { next_cursor: "" } }
    })

    run = start_run(options: { "conversation_ids" => %w[ CCHAN ] })
    deleted = false
    50.times do
      break if run.reload.finished?
      SlackImport::StepJob.perform_now(run.id)
      run.reload
      if !deleted && run.running? && Array(run.state.dig("convo", "thread_queue")).any?
        Message.find_by!(created_at: Time.at(Rational(parent_ts))).destroy!
        deleted = true
      end
    end

    assert deleted, "expected to delete the parent while its thread was queued"
    assert_equal "completed", run.status
    assert_equal 1, run.issues.count { |issue| issue.message.include?("was deleted") }
    assert_empty Message.where(created_at: Time.at(Rational(reply_ts)))
  end

  test "a multi-year conversation spanning several steps imports everything" do
    WebMock.reset!
    stub_slack_workspace!(history_overrides: {
      "CCHAN" => [
        history_page([ slack_message("1719792000.000001", "summer 2024"),
          slack_message("1704067200.000002", "new year 2024") ], cursor: "page-2"),
        history_page([ slack_message("1685577600.000003", "summer 2023") ], cursor: "page-3"),
        history_page([ slack_message("1640995200.000004", "new year 2022") ])
      ]
    })

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", run.status
    assert_equal 4, run.stats["counts"]["messages"]
    general = Rooms::Open.find_by!(name: "general")
    assert_equal 4, general.messages.count
    assert_equal 1, general.messages.where("created_at < ?", Time.utc(2023, 1, 1)).count
  end

  test "a full import after a date-bounded test import imports everything older too" do
    pages = [
      history_page([ slack_message("1719792000.000001", "recent news") ], cursor: "page-2"),
      history_page([ slack_message("1640995200.000004", "ancient history") ])
    ]
    WebMock.reset!
    stub_slack_workspace!(history_overrides: { "CCHAN" => pages })

    test_run = drive_import_to_completion(start_run(options: {
      "conversation_ids" => %w[ CCHAN ], "oldest" => "2024-06-17T00:00:00Z" }))
    assert_equal "completed", test_run.status
    assert_equal 1, test_run.stats["counts"]["messages"]

    full_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    assert_equal "completed", full_run.status
    assert_equal 1, full_run.stats["counts"]["messages"]

    general = Rooms::Open.find_by!(name: "general")
    assert_equal 2, general.messages.count
    assert_equal 1, general.messages.where("created_at < ?", Time.utc(2023, 1, 1)).count
  end

  test "catch-up after a kept test import and a full import re-reads only 30 days" do
    pages = [
      history_page([ slack_message("1719792000.000001", "recent news") ], cursor: "page-2"),
      history_page([ slack_message("1640995200.000004", "ancient history") ])
    ]
    WebMock.reset!
    stub_slack_workspace!(history_overrides: { "CCHAN" => pages })

    test_run = drive_import_to_completion(start_run(options: {
      "conversation_ids" => %w[ CCHAN ], "oldest" => "2024-06-17T00:00:00Z" }))
    assert_equal "completed", test_run.status
    assert_equal 1, test_run.stats["counts"]["messages"]

    full_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    assert_equal "completed", full_run.status
    assert_equal 1, full_run.stats["counts"]["messages"]
    # The full import reused the test import's mapping, so the mapping row
    # still belongs to the date-bounded run.
    assert_equal test_run.id, SlackImport::Record.find_by!(
      slack_kind: "conversation", slack_key: "CCHAN").slack_import_id

    catchup_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", catchup_run.status
    assert_equal 0, catchup_run.stats["counts"]["messages"]
    expected_oldest = format("%.6f", 1719792000.000001 - 30.days.to_f)
    assert_requested :get, "#{SLACK_API}/conversations.history",
      query: hash_including({ "channel" => "CCHAN", "oldest" => expected_oldest }), times: 2
  end

  # Three years of CCHAN history, newest first: a test import bounded to
  # the middle window takes only "summer 2023".
  def three_window_pages
    [
      history_page([ slack_message("1719792000.000001", "summer 2024") ], cursor: "page-2"),
      history_page([ slack_message("1685577600.000002", "summer 2023") ], cursor: "page-3"),
      history_page([ slack_message("1640995200.000003", "new year 2022") ])
    ]
  end

  # History stubs hand out their pages once, so every run re-stubs (which
  # also clears the recorded requests).
  def stub_three_windows!
    WebMock.reset!
    stub_slack_workspace!(history_overrides: { "CCHAN" => three_window_pages })
  end

  def middle_window_options
    { "conversation_ids" => %w[ CCHAN ],
      "oldest" => "2023-05-01T00:00:00Z", "latest" => "2023-07-01T00:00:00Z" }
  end

  def cchan_history_requests
    WebMock::RequestRegistry.instance.requested_signatures.hash.keys.select do |signature|
      signature.uri.path == "/api/conversations.history" &&
        URI.decode_www_form(signature.uri.query.to_s).to_h["channel"] == "CCHAN"
    end.map { |signature| URI.decode_www_form(signature.uri.query.to_s).to_h }
  end

  test "undo is last-in, first-out per conversation" do
    stub_three_windows!
    test_run = drive_import_to_completion(start_run(options: middle_window_options))
    assert_equal 1, test_run.stats["counts"]["messages"]
    stub_three_windows!
    full_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    assert_equal 2, full_run.stats["counts"]["messages"]
    # A later run over other conversations never blocks the test import.
    other_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CARCH ] }))
    assert other_run.undoable?

    assert_not test_run.undoable?
    assert_equal "A later import (##{full_run.id}) also imported some of these conversations; undo that one first.",
      test_run.undo_blocked_reason
    assert_not test_run.undo!
    assert_equal "completed", test_run.reload.status
    assert full_run.undoable?

    assert full_run.undo!
    drive_undo_to_completion(full_run)
    assert test_run.reload.undoable?
    assert_nil test_run.undo_blocked_reason
    assert test_run.undo!
    drive_undo_to_completion(test_run)
    assert_nil Rooms::Open.find_by(name: "general")

    stub_three_windows!
    reimport = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", reimport.status
    assert_equal 3, reimport.stats["counts"]["messages"]
    assert_equal [ "new year 2022", "summer 2023", "summer 2024" ],
      Rooms::Open.find_by!(name: "general").messages.order(:created_at).pluck(:markdown_source)
    assert_not_empty cchan_history_requests
    assert cchan_history_requests.none? { |params| params.key?("oldest") },
      "expected the re-import to read CCHAN's whole history"
  end

  test "a full import's coverage ends when an earlier run under it is undone" do
    stub_three_windows!
    test_run = drive_import_to_completion(start_run(options: middle_window_options))
    stub_three_windows!
    full_run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    assert_equal "completed", full_run.status
    # Bypass the last-in, first-out rule, as a direct database change would.
    test_run.update_columns(status: "undoing", state: { "phase" => "undo" }, finished_at: nil)
    travel 1.minute do
      drive_undo_to_completion(test_run)
    end
    general = Rooms::Open.find_by!(name: "general")
    assert_empty general.messages.where(created_at: Time.at(Rational("1685577600.000002")))

    stub_three_windows!
    travel 2.minutes do
      drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    end

    assert_equal 1, general.messages.where(created_at: Time.at(Rational("1685577600.000002"))).count
    assert cchan_history_requests.none? { |params| params.key?("oldest") },
      "expected CCHAN to be re-read in full, not from the catch-up window"
  end

  test "per-conversation record lookups seek the identity index" do
    plan = ActiveRecord::Base.connection.exec_query(
      "EXPLAIN QUERY PLAN #{SlackImport::Record.for_conversation(@workspace.id, "message", "CCHAN").to_sql}"
    ).rows.flatten.join(" ")

    assert_includes plan, "USING INDEX index_slack_import_records_on_slack_identity"
    assert_includes plan, "slack_key"
  end

  test "finishing refreshes the heartbeat while looping over rooms" do
    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    run.update_columns(heartbeat_at: 10.minutes.ago)

    SlackImport::Runner.new(run.reload).send(:finish_rooms)

    assert_operator run.reload.heartbeat_at, :>, 5.minutes.ago
  end

  test "completing a run kicks the next queued run" do
    first = start_run(options: { "conversation_ids" => %w[ CARCH ] })
    second = start_run(options: { "conversation_ids" => %w[ CPRIV ] })
    second.clear_pending_step_job!
    clear_enqueued_jobs

    drive_import_to_completion(first)

    assert_equal "completed", first.status
    assert_enqueued_with(job: SlackImport::StepJob, args: [ second.id ])
  end

  test "undoing a run kicks the next queued run" do
    first = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CARCH ] }))
    assert first.undo!
    second = start_run(options: { "conversation_ids" => %w[ CPRIV ] })
    second.clear_pending_step_job!
    clear_enqueued_jobs

    drive_undo_to_completion(first)

    assert_equal "undone", first.status
    assert_enqueued_with(job: SlackImport::StepJob, args: [ second.id ])
  end

  test "undo is blocked while another run is queued" do
    first = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CARCH ] }))
    second = start_run(options: { "conversation_ids" => %w[ CPRIV ] })

    assert_not first.undo!
    assert_equal "completed", first.reload.status
    assert_equal "Another import is queued or running. Wait for it to finish, then undo.",
      first.undo_blocked_reason

    second.cancel!
    assert first.undo!
  end

  test "message timestamps keep exact microseconds" do
    # 1700000001.015838 cannot round-trip through a double: Time.at of the
    # float lands on .015837.
    WebMock.reset!
    stub_slack_workspace!(history_overrides: {
      "CCHAN" => [ history_page([ slack_message("1700000001.015838", "exact") ]) ]
    })

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", run.status
    message = Rooms::Open.find_by!(name: "general").messages.sole
    assert_equal 15_838, message.reload.created_at.usec
    assert_equal "2023-11-14T22:13:21.015838Z", message.created_at.utc.iso8601(6)
  end

  test "truncated reaction lists import the listed users with one issue per message" do
    history = { ok: true, messages: [ {
      "type" => "message", "user" => "U001", "text" => "popular",
      "ts" => "1700000001.000001",
      "reactions" => [
        { "name" => "thumbsup", "users" => %w[ U002 UADMIN ], "count" => 5 },
        { "name" => "+1", "users" => %w[ U001 ], "count" => 3 }
      ] } ], has_more: false, response_metadata: { next_cursor: "" } }
    WebMock.reset!
    stub_slack_workspace!(history_overrides: { "CCHAN" => [ history ] })

    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    message = Rooms::Open.find_by!(name: "general").messages.sole
    assert_equal 3, message.boosts.count
    assert_equal 3, run.stats["counts"]["reactions"]
    assert_equal 1, run.issues.count { |issue| issue.message.include?("truncated") }
  end

  test "undo removes exactly what the run created and leaves the rest" do
    room = Rooms::Open.create!(name: "Pre-existing", creator: users(:david))
    kept_message = room.messages.create!(creator: users(:david), markdown_source: "keep me")
    dm = Current.set(user: users(:david)) do
      Rooms::Direct.find_or_create_for([ users(:david), users(:jz) ])
    end
    kept_dm_message = dm.messages.create!(creator: users(:jz), markdown_source: "keep dm")
    kevin_memberships = users(:kevin).memberships.pluck(:room_id, :involvement).sort

    run = drive_import_to_completion(start_run)
    assert run.undo!
    created_room_ids = run.records.where(slack_kind: "conversation", created_record: true).pluck(:record_id)
    created_user_ids = run.records.where(slack_kind: "user", created_record: true).pluck(:record_id)
    assert_not_empty created_room_ids
    assert_not_empty created_user_ids

    drive_undo_to_completion(run)

    assert_equal "undone", run.status
    assert_empty Room.where(id: created_room_ids)
    assert_empty User.where(id: created_user_ids)
    assert_empty run.records.reload
    assert_equal "keep me", Message.find(kept_message.id).markdown_source
    assert_equal "keep dm", Message.find(kept_dm_message.id).markdown_source
    assert Room.exists?(room.id)
    assert Room.exists?(dm.id)
    assert_equal kevin_memberships, users(:kevin).memberships.pluck(:room_id, :involvement).sort
    assert users(:kevin).active?
    assert_empty Message.search("standup")
  end

  test "undo keeps mappings for kept placeholders, so re-import creates no duplicates" do
    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    general = Rooms::Open.find_by!(name: "general")
    bot = run.records.find_by!(slack_kind: "user", slack_key: "UBOT1").record
    assert bot.deactivated?
    assert_nil bot.email_address
    general.messages.create!(creator: bot, markdown_source: "bot posted after import")

    assert run.undo!
    drive_undo_to_completion(run)

    assert_equal "undone", run.status
    assert User.exists?(bot.id), "expected the bot placeholder to survive undo"
    assert run.records.exists?(slack_kind: "user", record_id: bot.id),
      "expected undo to keep the surviving placeholder's mapping"

    reimport = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))

    assert_equal "completed", reimport.status
    assert_equal bot.id, SlackImport::Record.find_by!(slack_kind: "user", slack_key: "UBOT1").record_id
    # UBOT1's placeholder reused; only the message bot (whose user undo
    # removed) is recreated.
    assert_equal 2, User.where(name: "Build Bot").count
  end

  test "undo keeps a created room that gained foreign messages and records an issue" do
    run = drive_import_to_completion(start_run)
    assert run.undo!
    general = Rooms::Open.find_by!(name: "general")
    general.messages.create!(creator: users(:david), markdown_source: "posted after import")

    drive_undo_to_completion(run)

    assert Room.exists?(general.id)
    assert_equal [ "posted after import" ], Room.find(general.id).messages.pluck(:markdown_source)
    assert run.issues.reload.any? { |issue| issue.message.include?("kept") }
  end

  test "undo keeps a created room that holds an event and a scheduled message" do
    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN CARCH ] }))
    general = Rooms::Open.find_by!(name: "general")
    archived_id = Rooms::Open.find_by!(name: "old-project (archived)").id
    message_ids_before = general.messages.pluck(:id).to_set
    event = general.events.create!(organizer: users(:david), title: "Kickoff",
      starts_at: 1.week.from_now, time_zone: "UTC")
    scheduled = general.scheduled_messages.create!(user: users(:david),
      markdown_source: "Reminder!", send_at: 1.day.from_now)
    # The event announcement is a foreign message too; remove it so the
    # room is kept for the event and the scheduled message alone.
    general.messages.where.not(id: message_ids_before.to_a).destroy_all
    assert_empty general.messages.where.not(id: run.records.where(slack_kind: "message").select(:record_id))
    memberships_before = general.memberships.pluck(:user_id, :involvement).sort

    assert run.undo!
    drive_undo_to_completion(run)

    assert_equal "undone", run.status
    assert Room.exists?(general.id)
    assert Event.exists?(event.id)
    assert ScheduledMessage.exists?(scheduled.id)
    surviving_ids = User.where(id: memberships_before.map(&:first)).pluck(:id).to_set
    expected = memberships_before.select { |user_id, _| surviving_ids.include?(user_id) }
    assert_equal expected, general.memberships.reload.pluck(:user_id, :involvement).sort
    assert run.records.exists?(slack_kind: "conversation", slack_key: "CCHAN")
    assert run.issues.reload.any? { |issue| issue.message.include?("kept") }
    assert_not Room.exists?(archived_id)
  end

  test "undo keeps threads and rooms with real activity, with members and mappings" do
    run = drive_import_to_completion(start_run)
    general = Rooms::Open.find_by!(name: "general")
    secret = Rooms::Closed.find_by!(name: "secret")
    parent = general.messages.find_by!(created_at: Time.at(Rational("1700000002.000002")))
    thread = parent.channel_thread

    foreign_reply = thread.post_message!(creator: users(:david),
      attributes: { markdown_source: "real reply after import" })
    foreign_message = general.messages.create!(creator: users(:david),
      markdown_source: "real message after import")
    memberships_before = general.memberships.pluck(:user_id, :involvement).sort
    placeholder_ids = run.records.where(slack_kind: "user", created_record: true).pluck(:record_id).to_set

    assert run.undo!
    drive_undo_to_completion(run)

    assert_equal "undone", run.status
    # The thread, its parent and the real reply survive; the run's own
    # replies in the kept thread are gone.
    assert ChannelThread.exists?(thread.id)
    assert Message.exists?(parent.id)
    assert_equal thread.id, Message.find(foreign_reply.id).thread_id
    assert_equal [ foreign_reply.id ], thread.messages.reload.pluck(:id)
    # The room, the real message and the surviving members' memberships
    # survive untouched; only import-created placeholders that author
    # nothing left are removed (with their memberships, per the original
    # placeholder rules).
    assert Room.exists?(general.id)
    assert Message.exists?(foreign_message.id)
    surviving_ids = User.where(id: memberships_before.map(&:first)).pluck(:id).to_set
    expected = memberships_before.select { |user_id, _| surviving_ids.include?(user_id) }
    assert_equal expected, general.memberships.reload.pluck(:user_id, :involvement).sort
    removed_ids = memberships_before.map(&:first).to_set - surviving_ids
    assert_not_empty removed_ids
    assert_empty removed_ids - placeholder_ids
    # Rooms without foreign content are still removed.
    assert_not Room.exists?(secret.id)
    # Mappings for the survivors stay, so a later run reuses them.
    assert run.records.exists?(slack_kind: "conversation", slack_key: "CCHAN")
    assert run.records.exists?(slack_kind: "message", record_id: parent.id)
    assert run.records.exists?(slack_kind: "thread", record_id: thread.id)
    assert run.issues.reload.any? { |issue| issue.message.include?("kept") }
  end

  def imported_thread
    parent = Rooms::Open.find_by!(name: "general").messages
      .find_by!(created_at: Time.at(Rational("1700000002.000002")))
    [ parent, parent.channel_thread ]
  end

  test "undo keeps the parent of a thread holding a saved reply" do
    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    parent, thread = imported_thread
    saved_reply = thread.messages.find_by!(created_at: Time.at(Rational("1700000101.000101")))
    other_reply_ids = thread.messages.where.not(id: saved_reply.id).pluck(:id)
    assert_not_empty other_reply_ids
    SavedItem.create!(user: users(:david), message: saved_reply)

    assert run.undo!
    drive_undo_to_completion(run)

    assert Message.exists?(saved_reply.id)
    assert Message.exists?(parent.id), "expected the kept thread's parent to survive"
    assert_equal parent.id, ChannelThread.find(thread.id).parent_message_id
    assert_equal thread.id, Message.find(saved_reply.id).thread_id
    assert_empty Message.where(id: other_reply_ids)
    assert run.records.exists?(slack_kind: "message", record_id: parent.id)
    assert run.records.exists?(slack_kind: "thread", record_id: thread.id)
    assert run.records.exists?(slack_kind: "message", record_id: saved_reply.id)
  end

  test "undo keeps an imported message someone started a thread on" do
    run = drive_import_to_completion(start_run(options: { "conversation_ids" => %w[ CCHAN ] }))
    general = Rooms::Open.find_by!(name: "general")
    message = general.messages.find_by!(created_at: Time.at(Rational("1700000001.000001")))
    thread = ChannelThread.create!(room: general, creator: users(:david), parent_message: message)
    reply = thread.post_message!(creator: users(:david), attributes: { markdown_source: "real reply" })

    assert run.undo!
    drive_undo_to_completion(run)

    assert Message.exists?(message.id)
    assert_equal message.id, ChannelThread.find(thread.id).parent_message_id
    assert Message.exists?(reply.id)
    assert run.records.exists?(slack_kind: "message", record_id: message.id)
  end

  test "import stays silent: no foreign jobs, broadcasts, unread or inbox items" do
    activity_before = ActivityItem.count
    broadcasts_before = cable_broadcasts_count
    clear_enqueued_jobs

    run = drive_import_to_completion(start_run)

    queues = enqueued_jobs.map { |job| job[:queue] }
    assert_not_empty queues
    assert_empty queues - %w[ slack_import ]
    assert_empty enqueued_jobs.select { |job| job[:job] != SlackImport::StepJob }
    assert_equal broadcasts_before, cable_broadcasts_count
    assert_equal activity_before, ActivityItem.count
    assert_empty Membership.where.not(unread_at: nil)
      .where(room_id: run.records.where(slack_kind: "conversation").select(:record_id))

    # DB-only reference rows are still written for linked messages.
    linked = Message.find_by!(created_at: Time.at(Rational("1700000002.000002")))
    assert_not_empty linked.link_embed_references
  end

  test "imported messages are searchable" do
    drive_import_to_completion(start_run)

    assert_equal 1, Message.search("standup").count
    assert_equal 1, Message.search("kickoff").count
    assert_equal 1, Message.search("Broadcast reply").count
  end
end
