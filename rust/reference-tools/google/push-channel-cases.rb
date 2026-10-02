# Real pinned channel model/sweeper. Record random-value properties, relationships and all other fields.
require 'json'
require 'net/http'
require 'fileutils'
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger = Rails.logger
ENV['GOOGLE_CLIENT_ID'] = 'test-client-id'
ENV['GOOGLE_CLIENT_SECRET'] = 'FAKE-google-client-secret'
BASE = Time.utc(2026, 9, 23, 10, 30)
Time.define_singleton_method(:current) { BASE }
callback = 'https://app.test/google/calendar/notifications'
specs = [
  {name:'no_callback', callback:false}, {name:'no_account', account:false},
  {name:'first_watch', response:[200, {resourceId:'resource-1', expiration:((BASE+3.days).to_f*1000).to_i.to_s}]},
  {name:'rewatch', old:true},
  {name:'rewatch_forbidden', old:true, response:[403, {error:{errors:[{reason:'forbidden'}]}}]},
  {name:'renew_soon', renew:true, old:true, expiry_offset:7200},
  {name:'renew_fresh', renew:true, old:true, expiry_offset:172800},
  {name:'renew_missing_account', renew:true, old:true, account:false},
  {name:'heal_missing', renew:true},
  {name:'heal_disconnected', renew:true, disconnected:true},
  {name:'renew_timeout', renew:true, old:true, expiry_offset:7200, response:['timeout', nil]},
  {name:'preload_2', renew:true, old:true, expiry_offset:172800, users:2},
  {name:'preload_12', renew:true, old:true, expiry_offset:172800, users:12},
  {name:'renew_no_expiry', renew:true, old:true},
  {name:'renew_before_boundary', renew:true, old:true, expiry_offset:86399.999999},
  {name:'renew_at_boundary', renew:true, old:true, expiry_offset:86400},
  {name:'renew_after_boundary', renew:true, old:true, expiry_offset:86400.000001},
  {name:'stop_not_found', old:true, stop_status:404},
  {name:'stop_timeout', old:true, stop_status:'timeout'},
  {name:'watch_rate_limit', old:true, response:[429, {}]},
  {name:'watch_server_error', old:true, response:[503, {}]},
  {name:'watch_no_scope', old:true, scopes:'openid email'},
  {name:'watch_unreadable', old:true, unreadable:true},
  {name:'watch_no_secret', old:true, secret:false},
  {name:'renew_disconnected', renew:true, old:true, disconnected:true},
  {name:'renew_no_scope', renew:true, old:true, scopes:'openid email'},
  {name:'renew_unreadable', renew:true, old:true, unreadable:true}
]
calls = []; answers = []
http = Object.new
http.define_singleton_method(:method_missing) do |method, path, *args|
  body = args.first.is_a?(String) ? JSON.parse(args.first) : nil
  headers = args.last.is_a?(Hash) ? args.last : {}
  calls << {method:method.to_s.upcase, path:, body:, content_type:headers['Content-Type'], access_token:headers['Authorization']&.delete_prefix('Bearer ')}
  status, value = answers.shift || raise("Unrecorded Google HTTP prohibited: #{method} #{path}")
  raise Net::OpenTimeout if status == 'timeout'
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1', status.to_s, 'fixture')
  response.define_singleton_method(:body) { value.to_json }
  response
end
Net::HTTP.define_singleton_method(:start) do |host, *args, **kwargs, &block|
  raise "Unexpected Google host #{host}" unless %w[www.googleapis.com oauth2.googleapis.com].include?(host)
  block.call(http)
end
rows = specs.map do |spec|
  database = ActiveRecord::Base.connection_db_config.database
  ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)'); ActiveRecord::Base.connection_pool.disconnect!
  backup = "#{database}.push-channels"; FileUtils.cp(database, backup)
  begin
    Rails.application.executor.run!(reset:true)
    ENV['GOOGLE_CALENDAR_WEBHOOK_URL'] = spec[:callback] == false ? nil : callback
    ENV['GOOGLE_CLIENT_SECRET'] = spec[:secret] == false ? nil : 'FAKE-google-client-secret'
    GoogleAccount.delete_all; Calendar::PushChannel.delete_all
    users = [User.find(127326141)]
    (spec.fetch(:users, 1)-1).times do |i|
      users << User.create!(name:"Push fixture #{i}", email_address:"push-fixture-#{i}@example.test", password:'secret123456')
    end
    users.each do |user|
      account = GoogleAccount.create!(user:, email:'fixture@example.test', access_token:'access-token', refresh_token:'refresh-token', access_token_expires_at:BASE+3600, scopes:spec[:scopes], disconnected_reason:spec[:disconnected] ? 'revoked' : nil) if spec.fetch(:account, true)
      ActiveRecord::Base.connection.execute("UPDATE google_accounts SET refresh_token='broken-AR-ciphertext' WHERE id=#{account.id}") if spec[:unreadable]
      Calendar::PushChannel.create!(user:, channel_id:"old-#{user.id}", token_digest:Calendar::PushChannel.digest("old-token-#{user.id}"), resource_id:"old-resource-#{user.id}", expires_at:spec[:expiry_offset] && BASE+spec[:expiry_offset], last_message_number:42, last_notification_at:BASE-60, last_error:'previous failure', created_at:BASE-300, updated_at:BASE-300) if spec[:old]
    end
    # Raw ciphertext corruption must be observed by a fresh association, as in the helper's reload.
    users.each(&:reload)
    initial = Calendar::PushChannel.order(:id).map { |c| c.attributes.slice('id','user_id','channel_id','token_digest','resource_id','expires_at','last_message_number','last_notification_at','last_error','created_at','updated_at').transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.iso8601(6) : v } }
    answers.replace([spec.fetch(:response, [200, {resourceId:'new-resource'}]), [spec.fetch(:stop_status, 200), {}]])
    calls.clear; error = nil; queries = []
    listener = ->(*, payload) { queries << payload[:sql] if payload[:sql].include?('FROM "users"') }
    ActiveSupport::Notifications.subscribed(listener, 'sql.active_record') do
      begin
        spec[:renew] ? Calendar::PushChannel.renew_expiring! : Calendar::PushChannel.watch_for!(users.first)
      rescue => e
        error = e.class.name
      end
    end
    watches = calls.select { |c| c[:path].end_with?('/events/watch') }
    # These observations assert the random values' exact shape and stored relationship;
    # token/id bytes are intentionally random in both apps, never frozen or replaced.
    requests = calls.map do |c|
      next c.dup unless c[:path].end_with?('/events/watch')
      body = c[:body]
      c.merge(body:body.except('id','token'), uuid_v4:body['id'].match?(/\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/), token_hex_64:body['token'].match?(/\A[0-9a-f]{64}\z/))
    end
    channels = users.map do |user|
      channel = Calendar::PushChannel.find_by(user:)
      next nil unless channel
      old = initial.find { |c| c['user_id'] == user.id }
      watched = watches.find { |c| c[:body]['id'] == channel.channel_id }
      state = channel.attributes.slice('user_id','resource_id','expires_at','last_message_number','last_notification_at','last_error','created_at','updated_at')
        .transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.iso8601(6) : v }
      state.merge(same_row:old ? channel.id == old['id'] : nil, old_identity:channel.channel_id == "old-#{user.id}",
        digest_matches:channel.token_digest == Calendar::PushChannel.digest(watched ? watched[:body]['token'] : "old-token-#{user.id}"))
    end
    {spec:, user_ids:users.map(&:id), initial:, calls:requests, error:, channels:,
      disconnected:users.map { |u| u.reload.google_account&.disconnected_reason }, user_reads:queries.size}
  ensure
    ActiveRecord::Base.connection_pool.disconnect!; FileUtils.rm_f(["#{database}-wal","#{database}-shm"])
    FileUtils.cp(backup, database); FileUtils.rm_f(backup)
  end
end
channel = Calendar::PushChannel.new(token_digest:Calendar::PushChannel.digest('secret-token'))
tokens = ['secret-token', 'wrong-token', '', nil, ' ', "\u00a0"].map { |token| {token:, matches:channel.token_matches?(token)} }
claims = []
ActiveRecord::Base.transaction do
  Calendar::PushChannel.delete_all
  channel = Calendar::PushChannel.create!(user_id:127326141, channel_id:'claim-fixture', token_digest:Calendar::PushChannel.digest('claim-token'))
  %w[7 7 3 8 bogus].each do |number|
    won = channel.claim_notification!(number)
    claims << {number:, won:, last_message_number:channel.reload.last_message_number, last_notification_at:channel.last_notification_at&.utc&.iso8601(6)}
  end
  raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(reference:'d7c7de92', now:BASE.iso8601, rows:, tokens:, claims:)
warn "Pinned Rails PushChannel: #{rows.size} watch/renew scenarios; #{tokens.size} token cases; #{claims.size} notification claims; recorded HTTP only"
