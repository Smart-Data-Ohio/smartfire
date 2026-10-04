# Real JSON root and strong-parameter container requests, complete feature rows and frames.
require 'json'
require 'action_dispatch/testing/integration'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
conn = ActiveRecord::Base.connection
source = JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, html, **| frames << {stream:, html:} }
user = User.find(127326141)
room = Room.find(699448326)
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", 'Accept' => 'application/json', 'Content-Type' => 'application/json'}
roots = [nil, false, true, 17, 'tomorrow', [], [{}], {'unrelated' => 'value'}]
children = [nil, false, 17, [], [{}], {'nested' => ['2026-03-05', nil]}]
scenarios = roots.flat_map { |root| %w[saved scheduled slash].map { |action| {name: "#{action}_root_#{roots.index(root)}", action:, params: root} } }
children.each_with_index do |child, i|
  scenarios << {name: "saved_container_#{i}", action: 'saved', params: {saved_item: child}}
  scenarios << {name: "scheduled_container_#{i}", action: 'scheduled', params: {scheduled_message: child}}
  scenarios << {name: "slash_nested_#{i}", action: 'slash', params: {text: child}}
end
tables = %w[messages action_text_rich_texts saved_items scheduled_messages]
groups = []
outer_reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0) + '.groups.json')
source.fetch('groups').each do |group|
 outer_reset.call do
  conn = ActiveRecord::Base.connection
  group.fetch('rows').each do |table, rows|
    rows.each do |row|
      conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})")
    end
  end
  reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
  baseline = tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }
  cases = []
  scenarios.each do |scenario|
    reset.call do
      conn = ActiveRecord::Base.connection
      params = Marshal.load(Marshal.dump(scenario.fetch(:params)))
      params[:message_id] = group.fetch('old_ids').first if scenario[:action] == 'saved' && params.is_a?(Hash) && params.key?(:saved_item)
      path = {'saved' => '/saved', 'scheduled' => "/rooms/#{room.id}/scheduled_messages", 'slash' => "/rooms/#{room.id}/slash_commands"}.fetch(scenario[:action])
      frames.clear
      Rails.cache.clear
      browser = ActionDispatch::Integration::Session.new(Rails.application)
      browser.host! 'campfire.test'
      queries = []
      observer = ->(*args) { p = args.last; queries << p[:sql] if !p[:cached] && p[:name] != 'SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
      ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
        browser.post(path, params: JSON.generate(params), headers:)
      end
      cases << {name: scenario[:name], path:, params:, status: browser.response.status, content_type: browser.response.headers['Content-Type'], body: browser.response.body, location: browser.response.headers['Location'], reads: queries.length,
                state: tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }.reject { |table, rows| rows == baseline[table] },
                frames: frames.select { |f| f[:stream] == "#{room.to_gid_param}:messages" }.dup}
    end
  end
  groups << group.slice('size', 'rows').merge(baseline:, cases:)
 end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], groups:) + "\n")
puts "WS8bm2 container Rails: #{groups.sum { |g| g[:cases].length }} actual requests; status/type/body/location, complete feature rows and scoped broadcasts"
