# Sidebar read and warm-cache regressions: production controllers, real model fixtures,
# per-workspace caches, and a real GET/PATCH/GET time-zone change.
require 'action_dispatch/testing/integration'
load File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/post_pin.rb')
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
ActionView::Base.logger = Rails.logger
ActionController::Base.logger = Rails.logger
ApplicationController.allow_forgery_protection = true
ApplicationController.prepend(Module.new do
  def verify_authenticity_token = nil
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end)
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator'] = ->(_request) { 'NONCE' }
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
connection = ActiveRecord::Base.connection
uid = labels.fetch('users.david')
peer_id = labels.fetch('users.kevin')
fixture_tables = %w[accounts users rooms memberships room_categories dnd_allowed_users]
snapshot = -> {
  fixture_tables.flat_map do |table|
    ["DELETE FROM #{table}"] + connection.select_all("SELECT * FROM #{table} ORDER BY id").map do |row|
      "INSERT INTO #{table} (#{row.keys.map { |k| connection.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| connection.quote(v) }.join(',')})"
    end
  end
}
fixture = ->(&block) {
  result = nil
  ActiveRecord::Base.transaction do
    Current.reset
    Current.user = User.find(uid)
    block.call(Current.user)
    result = snapshot.call
    raise ActiveRecord::Rollback
  end
  result
}
step = ->(path, frame, label = 'cold') { {method: 'GET', path:, frame:, label:} }
cases = []
add_both = ->(name, sql, paths, viewer = 'david') {
  paths.each do |path|
    [true, false].each do |frame|
      cases << {name: "#{name} #{path} #{frame ? 'frame' : 'page'}", sql:, viewer:, steps: [step.call(path, frame)]}
    end
  end
}

[2, 12].each do |size|
  sql = fixture.call do |user|
    category = user.room_categories.create!(id: 9410000000 + size, name: 'Mixed <&>', collapsed: false, position: 0)
    size.times do |i|
      room = Rooms::Closed.create_for({id: 9420000000 + size*1000 + i*10, name: "Mixed <channel #{i} & room>", icon_name: 'smile', creator: user}, users: [user])
      membership = user.memberships.find_by!(room:)
      membership.update!(involvement: i.even? ? 'mentions' : 'muted', unread_at: i.even? ? Time.current : nil, favorite_position: i % 3 == 0 ? i + 10 : nil, room_category_id: i % 3 == 1 ? category.id : nil)
      peer = User.create!(id: 9430000000 + size*100 + i, name: "Peer <#{i} & human>", email_address: "review212-#{size}-#{i}@example.test", password_digest: user.password_digest, skip_open_room_grant: true)
      direct = Rooms::Direct.find_or_create_for([user, peer])
      direct.update_columns(updated_at: Time.current + i)
      user.memberships.find_by!(room: direct).update!(involvement: 'everything', unread_at: Time.current, favorite_position: i % 4 == 0 ? i + 100 : nil)
    end
  end
  ['/users/me/sidebar', '/users/me/profile'].each do |path|
    [true, false].each do |frame|
      kind = path.end_with?('sidebar') ? 'sidebar' : 'profile'
      kind += frame ? '_frame' : '_page'
      cases << {name: "mixed #{size} #{kind}", sql:, viewer: 'david', kind:, size:, steps: [step.call(path, frame, 'cold'), step.call(path, frame, 'warm')]}
    end
  end
end

zone_sql = fixture.call { |user| user.update_columns(time_zone: 'UTC', time_zone_explicit: true) }
cases << {name: 'real warm cache zone update', sql: zone_sql, viewer: 'david', steps: [
  step.call('/users/me/sidebar', true, 'before'),
  {method: 'PATCH', path: '/users/me/profile', frame: false, label: 'zone patch', params: {'user[time_zone]' => 'Asia/Tokyo'}},
  step.call('/users/me/sidebar', true, 'after')
]}

caches = {}
cases.each do |entry|
  ActiveRecord::Base.transaction do
    connection.execute('PRAGMA defer_foreign_keys=ON')
    entry[:sql].each { |sql| connection.execute(sql) }
    Current.reset
    Rails.cache = entry[:workspace] ? (caches[entry[:workspace]] ||= ActiveSupport::Cache::MemoryStore.new) : ActiveSupport::Cache::MemoryStore.new
    ActionController::Base.cache_store = Rails.cache
    ApplicationController.cache_store = Rails.cache
    ActionView::PartialRenderer.collection_cache = Rails.cache
    ActionController::Base.perform_caching = true
    ApplicationController.perform_caching = true
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! 'campfire.test'
    browser.cookies['session_token'] = CGI.unescape(labels.fetch("session_cookies.#{entry[:viewer]}"))
    before_versions = connection.select_rows("SELECT id,updated_at FROM memberships WHERE user_id=#{uid} ORDER BY id")
    entry[:steps].each_with_index do |request, index|
      headers = {'Accept' => 'text/html', 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0'}
      headers['Turbo-Frame'] = 'ui_matrix' if request[:frame]
      Icons.expire_custom_cache! if entry[:kind]
      counts = Hash.new(0)
      queries = []
      callback = ->(*args) {
        payload = args.last; sql = payload[:sql]
        # Schema-cache discovery is process startup, not controller data reads.
        next if payload[:name] == 'SCHEMA'
        table = sql[/\bFROM\s+"?([a-z_]+)/i,1]
        queries << sql if sql.lstrip.start_with?('SELECT') && !payload[:cached]
        counts[table] += 1 if table && sql.lstrip.start_with?('SELECT') && !payload[:cached]
      }
      ActiveRecord::Base.uncached do
        ActiveSupport::Notifications.subscribed(callback, 'sql.active_record') do
          browser.public_send(request[:method].downcase, request[:path], params: request[:params] || {}, headers:)
        end
      end
      request[:result] = {status: browser.response.status, body: browser.response.body, location: browser.response.headers['Location'], counts: counts.sort.to_h, queries:}
      if entry[:name] == 'real warm cache zone update' && index.zero?
        entry[:warm_cache_keys] = Rails.cache.instance_variable_get(:@data).keys.sort
        raise "direct room fragments were not cached: #{entry[:warm_cache_keys].inspect}" unless entry[:warm_cache_keys].any? { |key| key.include?('sidebars/rooms/_direct') || key.include?('sidebars/rooms/direct') }
      end
    end
    if entry[:name] == 'real warm cache zone update'
      entry[:membership_versions_unchanged] = before_versions == connection.select_rows("SELECT id,updated_at FROM memberships WHERE user_id=#{uid} ORDER BY id")
      entry[:final_zone] = User.find(uid).time_zone
      raise 'zone PATCH did not save' unless entry[:final_zone] == 'Asia/Tokyo'
      raise 'zone PATCH touched membership cache versions' unless entry[:membership_versions_unchanged]
    end
    raise ActiveRecord::Rollback
  end
  ActiveSupport::ExecutionContext.clear
end
puts JSON.pretty_generate(reference: 'd7c7de92', status_reference: '2e20b24c', caching: true, cases:)
warn "Rails sidebar review differential: #{cases.size} fixtures; #{cases.sum { |entry| entry[:steps].size }} controller responses"
