# Persisted-source contract of ActivityItems::Recorder, without response masks.
require "json"
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
hashes = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/generic-recorder-source-hashes.json")))
hashes.each { |file, hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
identify = ->(name) { ActiveRecord::FixtureSet.identify(name) }
rows = []
travel_to Time.utc(2026, 3, 2, 16) do
  users = %w[david jz kevin bender].map { |name| User.find(identify.call(name)) }
  room = Rooms::Direct.create_for({ id: 901840001, creator: users[0] }, users: users.first(3))
  member = room.memberships.find_by!(user: users[1])
  grant = HuddleGrant.create!(id: 901840002, room: room, user: users[1], membership: member,
    session: users[1].sessions.first || users[1].sessions.create!, identity: "ws12-recorder", room_name: "ws12", last_issued_at: Time.current)
  notice = AgentBudgetNotice.create!(id: 901840003, agent: Agent.find(identify.call("bender_agent")), cap: "board_posts", day: Date.current)
  thread = ChannelThread.create!(id: 901840004, room: Room.find(identify.call("designers")), creator: users[1], name: "Caller-authorized opener")
  ThreadMembership.join!(thread, users[1])
  opener = thread.post_message!(creator: users[1], attributes: {id: 901840005, body: "Opening", client_message_id: "ws12-generic-opener"})
  sources = [opener, SavedItem.first, Event.first, grant, AgentApproval.first, notice, ScheduledMessage.first, Session.first, TwoFactorCredential.first, room]
  raise "missing seed source" if sources.any?(&:nil?)
  # Transfer persisted source rows, not Ruby mocks, into the Rust fixture database.
  tables = %w[users rooms memberships agents sessions channel_threads messages saved_items events huddle_grants agent_approvals agent_budget_notices scheduled_messages two_factor_credentials]
  setup = tables.to_h do |table|
    filter = case table
      when "messages" then " WHERE id IN (SELECT message_id FROM saved_items UNION SELECT sent_message_id FROM scheduled_messages UNION SELECT reply_to_message_id FROM scheduled_messages) OR id=#{opener.id}"
      when "channel_threads" then " WHERE id=#{thread.id}"
      else ""
    end
    [table, ActiveRecord::Base.connection.select_all("SELECT * FROM #{table}#{filter} ORDER BY id").to_a]
  end
  snapshot = -> do
    ActivityItem.order(:user_id, :source_type, :source_id).map { |item| {
      user: item.user_id, source_type: item.source_type, source_id: item.source_id, event: item.event_type,
      read_at: item.read_at&.to_i, handled_at: item.handled_at&.to_i, created_at: item.created_at.to_i, updated_at: item.updated_at.to_i
    } }
  end
  call = ->(source, recipient, event, skip) do
    item = ActivityItems::Recorder.record!(recipient: recipient, source: source, event_type: event, skip_source_check: skip)
    { recorded: item.present?, items: snapshot.call }
  rescue ArgumentError => error
    { error: error.message, items: snapshot.call }
  end
  sources.each do |source|
    [false, true].each do |skip|
      users.each do |user|
        ActivityItem.delete_all
        rows << { name: "#{source.class.base_class.name}_#{user.id}_#{skip}", source_type: source.class.base_class.name,
          source_id: source.id, recipient: user.id, event: "work_update", skip: skip,
          expected: call.call(source, user, "work_update", skip) }
      end
    end
    ActivityItem.delete_all
    rows << { name: "#{source.class.base_class.name}_unpersisted", source_type: source.class.base_class.name,
      source_id: -1, recipient: users[0].id, event: "work_update", skip: true,
      expected: call.call(source.class.new, users[0], "work_update", true) }
  end
  sources.each do |source|
    recipient = source.is_a?(Room) ? users[1] : users[0]
    ActivityItem.delete_all
    recipient.update_columns(status: User.statuses.fetch("deactivated"))
    rows << { name: "#{source.class.base_class.name}_inactive_recipient", source_type: source.class.base_class.name,
      source_id: source.id, recipient: recipient.id, event: "work_update", skip: true,
      sql: ["UPDATE users SET status=1 WHERE id=#{recipient.id}"], expected: call.call(source, recipient, "work_update", true) }
    recipient.update_columns(status: User.statuses.fetch("active"))
    ActivityItem.delete_all
    rows << { name: "#{source.class.base_class.name}_missing_recipient", source_type: source.class.base_class.name,
      source_id: source.id, recipient: -1, event: "work_update", skip: true, expected: call.call(source, nil, "work_update", true) }
  end
  %w[nothing invisible mentions].each do |involvement|
    membership = room.memberships.find_by!(user: users[0])
    membership.update_columns(involvement: involvement)
    [false, true].each do |skip|
      ActivityItem.delete_all
      rows << { name: "HuddleGrant_#{involvement}_#{skip}", source_type: "HuddleGrant", source_id: grant.id,
        recipient: users[0].id, event: "huddle_started", skip: skip,
        sql: ["UPDATE memberships SET involvement='#{involvement}' WHERE room_id=#{room.id} AND user_id=#{users[0].id}"],
        expected: call.call(grant, users[0], "huddle_started", skip) }
    end
  end
  [nil, users[2].id].each do |owner_id|
    agent = notice.agent
    agent.update_columns(owner_id: owner_id)
    notice.reload
    users.each do |recipient|
      [false, true].each do |skip|
        ActivityItem.delete_all
        rows << { name: "AgentBudgetNotice_owner_#{owner_id}_#{recipient.id}_#{skip}", source_type: "AgentBudgetNotice", source_id: notice.id,
          recipient: recipient.id, event: "agent_budget_exceeded", skip: skip,
          sql: ["UPDATE agents SET owner_id=#{owner_id || 'NULL'} WHERE id=#{agent.id}"],
          expected: call.call(notice, recipient, "agent_budget_exceeded", skip) }
      end
    end
  end
  ActivityItem.delete_all
  rows << { name: "invalid_event_before_missing_source", source_type: "SavedItem", source_id: -1,
    recipient: -1, event: "invalid", skip: true, expected: call.call(SavedItem.new, nil, "invalid", true) }
  ActivityItem.delete_all
  denied = ActivityItems::Recorder.record!(recipient: users[0], source: opener, event_type: "thread_activity")
  first = ActivityItems::Recorder.record!(recipient: users[0], source: opener, event_type: "thread_activity", skip_source_check: true)
  second = ActivityItems::Recorder.record!(recipient: users[0], source: opener, event_type: "thread_activity", skip_source_check: true)
  caller_authorized = { source_id: opener.id, recipient: users[0].id, expected: {
    denied: denied.nil?, same_id: first.id == second.id, items: snapshot.call } }
  ActivityItem.delete_all
  item = ActivityItems::Recorder.record!(recipient: users[0], source: sources[1], event_type: "message_reminder", skip_source_check: true)
  item.mark_handled!
  travel_to Time.utc(2026, 3, 2, 16, 0, 1)
  idempotent = call.call(sources[1], users[0], "mention", true)
  puts JSON.pretty_generate(reference: "d7c7de92 plus approved board drift", setup: setup, rows: rows, caller_authorized: caller_authorized,
    idempotent: { source_id: sources[1].id, recipient: users[0].id, expected: idempotent })
end
warn "Rails generic recorder oracle: #{rows.size} source/recipient/authorization cases; preserved handled-source idempotency; 0 masks"
