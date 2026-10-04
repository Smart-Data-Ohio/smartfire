# Whole HTTP requests and the directory loader, measured after fixture setup.
# SQL uses the same first-FROM classification as the independent review probes.
require 'action_dispatch/testing/integration'
require 'json'
ActiveRecord::Base.logger = nil
ApplicationController.allow_forgery_protection = false
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
admin = User.find(127326141)
bot = User.find(394959859)
room = Room.find(486777696)
tables = %w[agent_approvals agent_events agent_grants agents rooms users sessions messages]
snapshot = lambda do |names|
  names.flat_map do |name|
    ["DELETE FROM #{name}"] + ActiveRecord::Base.connection.select_all("SELECT * FROM #{name} ORDER BY id").map do |row|
      "INSERT INTO #{name} (#{row.keys.map { |key| ActiveRecord::Base.connection.quote_column_name(key) }.join(',')}) VALUES (#{row.values.map { |value| ActiveRecord::Base.connection.quote(value) }.join(',')})"
    end
  end
end
observe = lambda do |path, accept|
  counts = tables.to_h { |table| [table, 0] }
  subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*, payload|
    sql = payload[:sql]
    table = sql[/\bFROM\s+"?([a-z_]+)/i, 1]
    counts[table] += 1 if sql.match?(/\ASELECT/i) && counts.key?(table) && !payload[:cached]
  end
  if path == 'directory_loader'
    records = ActiveRecord::Base.uncached { Agent.for_directory }
    result = {status: 200, size: records.size, selects: counts.values.sum}
  else
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! 'campfire.test'
    browser.get(path, headers: {'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'Accept' => accept, 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0'})
    raise "#{path}: #{browser.response.status}" unless browser.response.status == 200
    result = {status: browser.response.status}
    result[:body] = browser.response.body if accept == 'application/json'
  end
  result.merge(counts:)
ensure
  ActiveSupport::Notifications.unsubscribe(subscriber)
  ActiveSupport::ExecutionContext.clear
end
cases = []
[
  ['approvals_denied', [2,12]], ['approvals_pending', [10,50]],
  ['events', [2,12,10,50]], ['grants', [2,12]],
  ['directory', [2,12,10,100]], ['bots', [2,12]], ['sessions', [10,100]]
].each do |kind, sizes|
  sizes.each do |size|
    ActiveRecord::Base.transaction do
      agent = bot.agent
      agent.update_columns(owner_id: admin.id)
      names = %w[agents]
      path = nil
      accept = 'text/html'
      case kind
      when /^approvals/
        AgentApproval.where(agent:).delete_all
        size.times do |index|
          approval = AgentApproval.create!(agent:, room:, action: 'deploy', summary: "Read approval #{index}", expires_at: Time.current+3600)
          approval.update_columns(status: 'denied', decided_by_id: admin.id, decided_at: Time.current) if kind == 'approvals_denied'
        end
        names += %w[agent_approvals activity_items]
        path = "/agents/#{agent.id}/approvals"
      when 'events'
        AgentEvent.where(agent:).delete_all
        size.times { |index| AgentEvent.create!(agent:, room:, actor: admin, event_type: 'mention', outcome: 'suppressed', detail: "Read event #{index}") }
        names << 'agent_events'
        path = "/agents/#{agent.id}/events"
      when 'grants'
        AgentGrant.where(agent:).delete_all
        size.times do
          grant = AgentGrant.create!(agent:, room:, granted_by: admin, capability: 'read_messages')
          grant.update_columns(revoked_at: Time.current)
        end
        names << 'agent_grants'
        path = "/account/bots/#{bot.id}/grants"
      when 'directory', 'bots'
        User.where(role: :bot).update_all(status: :deactivated)
        size.times do |index|
          user = User.create!(name: "Read bot #{index.to_s.rjust(3,'0')}", role: :bot, status: :active)
          Agent.create!(user:, owner: admin, kind: :workspace)
        end
        names += %w[users memberships]
        path = kind == 'directory' ? '/agents' : '/account/bots'
      when 'sessions'
        ActivityItem.delete_all
        size.times do |index|
          session = Session.create!(id: 9100000000+index, user: admin, token: "page-read-session-#{index}", device_id: "page-read-device-#{index}", ip_address: "198.51.100.#{index+1}", user_agent: 'Mozilla/5.0 Chrome/140.0.0.0', last_active_at: Time.current)
          ActivityItem.create!(id: 9200000000+index, user: admin, source: session, event_type: 'new_sign_in')
        end
        names += %w[sessions activity_items]
        path = '/activity.json'
        accept = 'application/json'
      end
      sql = snapshot.call(names)
      result = observe.call(path, accept)
      entry = {kind:,size:,path:,accept:,sql:,result:}
      entry[:loader] = observe.call('directory_loader', accept) if kind == 'directory'
      cases << entry
      warn "Rails page reads: #{kind} #{size}; #{result[:counts].to_json}"
      raise ActiveRecord::Rollback
    end
    ActiveSupport::ExecutionContext.clear
  end
end
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), cases:)
warn "Rails page-read oracle: #{cases.size} cases; administrator HTTP and directory loader"
