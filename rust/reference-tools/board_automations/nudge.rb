# BoardSlaNudge model, recorder and inbox contracts from our Rails reference.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
root = ENV.fetch('PARITY_WORK')
JSON.parse(File.read(File.join(root, 'reference-tools/board_automations/source-hashes.json'))).each do |path, hash|
  raise "Rails source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
conn = ActiveRecord::Base.connection
setup = [
  "UPDATE rooms SET type='Rooms::Board' WHERE id=486777696",
  "DELETE FROM activity_items",
  "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(970000001,486777696,127326141,'SLA <&> post','in_progress','2026-03-02 14:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"
]
base = { room_id: 486777696, channel_thread_id: 970000001, recipient_id: 127326141, work_status: 'in_progress', stage: 'nudge', status_entered_at: '2026-03-02 14:00:00' }
cases = [
  ['valid', {}], ['escalation', { stage: 'escalation' }], ['done-is-valid-claim', { work_status: 'done' }],
  ['missing-room', { room_id: 0 }], ['missing-thread', { channel_thread_id: 0 }], ['missing-recipient', { recipient_id: 0 }],
  ['missing-status', { work_status: nil }], ['blank-status', { work_status: '' }], ['whitespace-status', { work_status: ' ' }], ['unknown-status', { work_status: 'shipped' }],
  ['missing-stage', { stage: nil }], ['blank-stage', { stage: '' }], ['unknown-stage', { stage: 'later' }], ['missing-entry', { status_entered_at: nil }],
  ['duplicate', {}, true], ['new-stage', { stage: 'escalation' }, true], ['new-status', { work_status: 'blocked' }, true], ['new-crossing', { status_entered_at: '2026-03-02 13:59:59.999999' }, true],
  ['future-entry', { status_entered_at: '2026-03-02 16:01:00' }], ['fraction-before-minute', { status_entered_at: '2026-03-02 15:59:00.000001' }], ['at-minute', { status_entered_at: '2026-03-02 15:59:00' }],
  ['before-hour', { status_entered_at: '2026-03-02 15:00:00.000001' }], ['at-hour', { status_entered_at: '2026-03-02 15:00:00' }],
  ['bot-recipient-model-valid', { recipient_id: 394959859 }]
]
models = cases.map do |name, overrides, duplicate|
  row = nil
  ActiveRecord::Base.transaction do
    setup.each { |sql| conn.execute(sql) }
    BoardSlaNudge.create!(base) if duplicate
    input = base.merge(overrides)
    nudge = BoardSlaNudge.new(input)
    valid = nudge.valid?
    before = ActivityItem.count
    nudge.save! if valid
    row = { name:, input:, duplicate: !!duplicate, valid:, errors: nudge.errors.to_hash, full_messages: nudge.errors.full_messages,
      waited: valid ? nudge.waited_minutes : nil, recipients: valid ? nudge.activity_recipient_ids : nil, activity_delta: ActivityItem.count - before }
    raise ActiveRecord::Rollback
  end
  row
end
recorder = %w[recipient wrong-recipient bot inactive repeated handled-repeated].map do |name|
  row = nil
  ActiveRecord::Base.transaction do
    setup.each { |sql| conn.execute(sql) }
    recipient = User.find(name == 'bot' ? 394959859 : 127326141)
    recipient.update_columns(status: User.statuses.fetch('deactivated')) if name == 'inactive'
    nudge = BoardSlaNudge.create!(base.merge(recipient_id: recipient.id))
    caller = name == 'wrong-recipient' ? User.find(149087659) : recipient
    first = ActivityItems::Recorder.record!(recipient: caller, source: nudge, event_type: 'work_sla')
    first.mark_handled! if name == 'handled-repeated' && first
    again = ActivityItems::Recorder.record!(recipient: caller, source: nudge, event_type: 'work_sla') if %w[repeated handled-repeated].include?(name)
    row = { name:, caller: caller.id, recipient: recipient.id, status: recipient.status, result: first&.id, count: ActivityItem.where(source: nudge).count,
      same_item: again && again.id == first.id, state: first&.reload&.then { |item| item.handled_at ? 'handled' : 'unread' } }
    nudge.destroy!
    row[:remaining_items] = ActivityItem.where(source_type: 'BoardSlaNudge', source_id: nudge.id).count
    row[:remaining_nudges] = BoardSlaNudge.where(id: nudge.id).count
    raise ActiveRecord::Rollback
  end
  row
end
# Full JSON responses and token-free detached HTML list fragments; no byte masks.
http = []
%w[nudge escalation fractional revoked wrong-user].each do |kind|
  ActiveRecord::Base.transaction do
    setup.each { |sql| conn.execute(sql) }
    entered = kind == 'fractional' ? '2026-03-02 15:59:00.000001' : base[:status_entered_at]
    nudge = BoardSlaNudge.create!(base.merge(id: 970000002, stage: kind == 'escalation' ? 'escalation' : 'nudge', status_entered_at: entered))
    item = ActivityItems::Recorder.record!(recipient: User.find(127326141), source: nudge, event_type: 'work_sla')
    item.update_columns(id: 970000003)
    extra = []
    if kind == 'revoked'
      extra << 'DELETE FROM memberships WHERE room_id=486777696 AND user_id=127326141'
    elsif kind == 'wrong-user'
      extra << 'UPDATE activity_items SET user_id=149087659 WHERE id=970000003'
    end
    extra.each { |sql| conn.execute(sql) }
    sql = setup + ["INSERT INTO board_sla_nudges(id,room_id,channel_thread_id,recipient_id,work_status,stage,status_entered_at,created_at,updated_at) VALUES(970000002,486777696,970000001,127326141,'in_progress','#{nudge.stage}','#{entered}','2026-03-02 16:00:00','2026-03-02 16:00:00')", "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(970000003,127326141,'BoardSlaNudge',970000002,'work_sla','2026-03-02 16:00:00','2026-03-02 16:00:00')"] + extra
    %w[application/json].each do |accept|
      browser = ActionDispatch::Integration::Session.new(Rails.application)
      browser.host!('campfire.test')
      labels = JSON.parse(File.read(File.join(root, 'parity/.seed/default/labels.json')))
      browser.get('/activity?type=threads', headers: { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'Accept' => accept, 'Turbo-Frame' => 'activity_test', 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' })
      Current.session = User.find(127326141).sessions.where.not(two_factor_verified_at: nil).first!
      visible = ActivityItem.accessible_to(User.find(127326141)).unread.order(updated_at: :desc, id: :desc).to_a
      fragment = ApplicationController.renderer.render(partial: 'activity_items/list', locals: { activity_items: visible, filter: 'unread', type_filter: 'threads' })
      raise 'unexpected session token in detached fragment' if fragment.include?('authenticity_token')
      http << { fragment:, name: "#{kind}-#{accept}", setup: sql, path: '/activity?type=threads', accept:, status: browser.response.status, body: browser.response.body }
    end
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate(reference: 'd7c7de92 plus approved board drift', setup:, models:, recorder:, http:)
warn "Rails BoardSlaNudge oracle: #{models.size} model cases; #{recorder.size} recorder cases; #{http.size} complete JSON responses plus HTML list fragments; 0 masks"
