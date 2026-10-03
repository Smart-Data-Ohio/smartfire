# Real HTTP consumers for relative splits, wide dates and exceptional clock input.
require 'json'
require 'action_dispatch/testing/integration'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter = :test
Random.define_singleton_method(:uuid) { 'fixture-relative-message' }
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
zones = %w[UTC America/New_York Australia/Lord_Howe Pacific/Apia]
commands = ['/remind in 9223372036854775807 minutes Review', '/remind in 2147483648 days Review', '/remind in 3652059 days Review', '/event Launch in 999999999999999999999 weeks', '/remind 2026-03-05 14:30:60 Review', '/event Launch 2026-03-05 14:30']
dates = ['10000-03-05 14:30 +05:30', '2026-03-05 25:00']
scenarios = zones.flat_map do |zone|
  items = commands.each_with_index.map { |text, i| {name: "#{zone}/slash_#{i}", zone:, action: 'slash', params: {text:}} }
  dates.each_with_index do |date, i|
    items << {name: "#{zone}/saved_#{i}", zone:, action: 'saved', params: {saved_item: {remind_at: date}}}
    items << {name: "#{zone}/scheduled_#{i}", zone:, action: 'scheduled', params: {scheduled_message: {markdown_source: 'Exceptional schedule', send_at: date}}}
  end
  items
end
if ARGV[1] == 'overflow'
  # PR #223: event URLs, reminder notices, rows and the Saved page render the same wide
  # timestamps. `relative_consumers.rb OUT overflow` writes relative_overflow_consumers.json.
  scenarios = [[130, 'days'], [140, 'days'], [142, 'hours']].map do |exponent, unit|
    {name: "UTC/event_#{exponent}_#{unit}", zone: 'UTC', action: 'slash', params: {text: "/event Review in 1#{'0' * exponent} #{unit}"}}
  end
  scenarios += [[140, 'days'], [142, 'hours']].map do |exponent, unit|
    {name: "UTC/remind_#{exponent}_#{unit}", zone: 'UTC', action: 'slash', params: {text: "/remind in 1#{'0' * exponent} #{unit} Review"}, saved_page: true}
  end
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
      user.reload.update_columns(time_zone: scenario.fetch(:zone))
      frames.clear
      Rails.cache.clear
      browser = ActionDispatch::Integration::Session.new(Rails.application)
      browser.host! 'campfire.test'
      queries = []
      observer = ->(*args) { p = args.last; queries << p[:sql] if !p[:cached] && p[:name] != 'SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
      ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
        browser.post(path, params: JSON.generate(params), headers:)
      end
      cases << {name: scenario[:name], zone: scenario[:zone], path:, params:, status: browser.response.status, content_type: browser.response.headers['Content-Type'], body: browser.response.body, location: browser.response.headers['Location'], reads: queries.length,
                state: tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }.reject { |table, rows| rows == baseline[table] },
                frames: frames.select { |f| f[:stream] == "#{room.to_gid_param}:messages" }.dup}
      if scenario[:saved_page]
        # app/views/saved_items/_item.html.erb renders each reminder with local_datetime_tag.
        browser.get('/saved', headers: headers.slice('Cookie'))
        cases.last[:saved_page] = {status: browser.response.status, items: browser.response.body.scan(%r{<article id="saved_item_.*?</article>}m)}
      end
    end
  end
  groups << group.slice('size', 'rows').merge(baseline:, cases:)
 end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', groups:) + "\n")
puts "WS8bm2 relative consumer Rails: #{groups.sum { |g| g[:cases].length }} actual requests; status/type/body/location, complete feature rows and scoped broadcasts"
