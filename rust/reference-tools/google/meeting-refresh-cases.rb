# Full refresh/cache observations from pinned Rails; HTTP is recorded and cannot escape.
require 'json'
require 'net/http'
require 'fileutils'
ActiveJob::Base.queue_adapter = :test
ActiveRecord::Base.logger = nil
ENV['GOOGLE_CLIENT_ID'] = 'test-client-id'
ENV['GOOGLE_CLIENT_SECRET'] = 'FAKE-google-client-secret'
BASE = Time.utc(2026, 9, 23, 10, 30)
Time.define_singleton_method(:current) { BASE }
BUSY = [['2026-09-23T10:00:00Z', '2026-09-23T11:00:00Z']]
OOO = [['2026-09-22T00:00:00Z', '2026-09-25T00:00:00Z']]
timed = ->(start_at, end_at, attrs = {}) { {'status'=>'confirmed', 'start'=>{'dateTime'=>start_at}, 'end'=>{'dateTime'=>end_at}}.merge(attrs) }
busy = timed.call(*BUSY[0])
free = timed.call('2026-09-23T13:00:00Z', '2026-09-23T13:30:00Z', 'transparency'=>'transparent')
ooo = busy.merge('eventType'=>'outOfOffice')
later = timed.call('2026-09-23T13:00:00Z', '2026-09-23T13:30:00Z')
quota = {'error'=>{'errors'=>[{'reason'=>'userRateLimitExceeded'}]}}
specs = [
  {name:'success', cache:{fetch_error:'stale notice'}, items:[busy, free]},
  {name:'privacy_fields'},
  {name:'never_opted_in', meeting:false},
  {name:'inactive', inactive:true},
  {name:'unusable', disconnected:true, cache:{busy_intervals:BUSY, ooo_intervals:OOO}},
  {name:'revoked', expired:true, cache:{busy_intervals:BUSY}, responses:[[400, {error:'invalid_grant'}.to_json]]},
  {name:'rate_limit', cache:{busy_intervals:BUSY}, responses:[[429, '{}']]},
  {name:'quota', cache:{busy_intervals:BUSY}, responses:[[403, quota.to_json]]},
  {name:'server_error', cache:{busy_intervals:BUSY}, responses:[[500, 'boom']]},
  {name:'malformed', cache:{busy_intervals:BUSY}, responses:[[200, '{oops']]},
  {name:'fresh', cache:{fetched_offset_us:-30000000}},
  {name:'followup', cache:{fetched_offset_us:-30000000}},
  {name:'dedupe', cache:{fetched_offset_us:-30000000}, offsets:[0, 0]},
  {name:'completed_claim', cache:{fetched_offset_us:-30000000}, offsets:[0, 61, 61]},
  {name:'ooo_only', meeting:false, ooo:true, items:[ooo, later]},
  {name:'both_opt_ins', ooo:true, items:[ooo, later]},
  {name:'meeting_only_no_ooo', items:[ooo]},
  {name:'meeting_lookahead'},
  {name:'ooo_server_error', ooo:true, cache:{ooo_intervals:OOO}, responses:[[500, 'boom']]},
  {name:'both_revoked', ooo:true, expired:true, cache:{busy_intervals:BUSY, ooo_intervals:OOO}, responses:[[400, {error:'invalid_grant'}.to_json]]},
  # Additional boundaries, shapes and failures go through the same real client.
  {name:'missing_user', missing_user:true}, {name:'missing_account', account:false},
  {name:'calendar_scope_missing', scopes:'openid email'},
  {name:'unreadable_token', unreadable:true},
  {name:'transport', cache:{busy_intervals:BUSY, ooo_intervals:OOO}, responses:[['timeout', '']]},
  {name:'forbidden', cache:{busy_intervals:BUSY}, responses:[[403, '{}']]},
  {name:'null_response', responses:[[200, 'null']]},
  {name:'empty_response', responses:[[200, '{}']]},
  {name:'stale_pending_cleared', cache:{pending_offset_us:-120000000}, items:[busy]},
  {name:'transient_pending_cleared', cache:{pending_offset_us:-120000000, busy_intervals:BUSY}, responses:[[503, '{}']]},
  {name:'fetched_before_boundary', cache:{fetched_offset_us:-60000001}},
  {name:'fetched_at_boundary', cache:{fetched_offset_us:-60000000}},
  {name:'fetched_after_boundary', cache:{fetched_offset_us:-59999999}},
  {name:'pending_before_boundary', cache:{fetched_offset_us:-30000000, pending_offset_us:-60000001}},
  {name:'pending_at_boundary', cache:{fetched_offset_us:-30000000, pending_offset_us:-60000000}},
  {name:'pending_after_boundary', cache:{fetched_offset_us:-30000000, pending_offset_us:-59999999}},
  {name:'all_day_ooo', meeting:false, ooo:true, zone:'Pacific Time (US & Canada)', items:[{'eventType'=>'outOfOffice', 'start'=>{'date'=>'2026-09-28'}, 'end'=>{'date'=>'2026-10-05'}}]},
  {name:'broadcast_preserve_false', cache:{in_meeting_broadcast:false}, items:[busy]},
  {name:'broadcast_preserve_true', cache:{in_meeting_broadcast:true}, items:[busy]},
  {name:'broadcast_unusable_preserves_true', disconnected:true, cache:{busy_intervals:BUSY, in_meeting_broadcast:true}},
  {name:'broadcast_transient_preserves_false', cache:{busy_intervals:BUSY, in_meeting_broadcast:false}, responses:[[503, '{}']]},
  {name:'broadcast_boundary_flips', cache:{in_meeting_broadcast:false}, items:[busy], broadcast_claims:[true,true,false,false,true]},
  {name:'pages', responses:[[200, {items:[busy], nextPageToken:'page-2'}.to_json], [200, {items:[later]}.to_json]]}
]
calls = []; answers = []
http = Object.new
http.define_singleton_method(:method_missing) do |method, path, *args|
  headers = args.last.is_a?(Hash) ? args.last : {}
  calls << {method:method.to_s.upcase, path:, body:%i[get delete].include?(method) ? '' : args.first.to_s,
    content_type:headers['Content-Type'], access_token:headers['Authorization']&.delete_prefix('Bearer ')}
  status, body = answers.shift || raise("Unrecorded Google HTTP prohibited: #{method} #{path}")
  raise Net::OpenTimeout if status == 'timeout'
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1', status.to_s, 'fixture')
  response.define_singleton_method(:body) { body }
  response
end
Net::HTTP.define_singleton_method(:start) do |host, *args, **kwargs, &block|
  raise "Unexpected Google host #{host}" unless %w[www.googleapis.com oauth2.googleapis.com].include?(host)
  block.call(http)
end
cache_state = ->(user_id) {
  cache = Calendar::MeetingCache.find_by(user_id:)
  next nil unless cache
  state = cache.attributes.slice('busy_intervals', 'ooo_intervals', 'fetch_error', 'fetched_at', 'refresh_pending_at', 'created_at', 'updated_at', 'in_meeting_broadcast')
  state.transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.iso8601(6) : v }
}
rows = specs.map do |spec|
  database = ActiveRecord::Base.connection_db_config.database
  ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)')
  ActiveRecord::Base.connection_pool.disconnect!
  backup = "#{database}.meeting-refresh"
  FileUtils.cp(database, backup)
  begin
    Rails.application.executor.run!(reset:true)
    GoogleAccount.delete_all; Calendar::MeetingCache.delete_all
    user = User.find(127326141)
    user.update_columns(status:spec[:inactive] ? 1 : 0, meeting_status_enabled:spec.fetch(:meeting, true),
      ooo_calendar_enabled:spec.fetch(:ooo, false), time_zone:spec[:zone])
    account = GoogleAccount.create!(user:, email:'fixture@example.test', access_token:'access-token', refresh_token:'refresh-token',
      access_token_expires_at:BASE + (spec[:expired] ? -3600 : 3600), scopes:spec[:scopes],
      disconnected_reason:spec[:disconnected] ? 'Google rejected the connection' : nil) if spec.fetch(:account, true)
    if spec[:unreadable]
      ActiveRecord::Base.connection.execute("UPDATE google_accounts SET refresh_token='broken-AR-ciphertext' WHERE id=#{account.id}")
    end
    if spec[:cache]
      attrs = spec[:cache].dup
      attrs[:fetched_at] = BASE + Rational(attrs.delete(:fetched_offset_us), 1_000_000) if attrs.key?(:fetched_offset_us)
      attrs[:refresh_pending_at] = BASE + Rational(attrs.delete(:pending_offset_us), 1_000_000) if attrs.key?(:pending_offset_us)
      Calendar::MeetingCache.create!(user:, created_at:BASE - 300, updated_at:BASE - 300, **attrs)
    end
    initial = cache_state.call(user.id)
    boundary_offsets = {
      'fetched_before_boundary'=>['fetched_at', -60_000_001],
      'fetched_at_boundary'=>['fetched_at', -60_000_000],
      'fetched_after_boundary'=>['fetched_at', -59_999_999],
      'pending_before_boundary'=>['refresh_pending_at', -60_000_001],
      'pending_at_boundary'=>['refresh_pending_at', -60_000_000],
      'pending_after_boundary'=>['refresh_pending_at', -59_999_999]
    }
    if (boundary = boundary_offsets[spec[:name]])
      column, expected = boundary
      actual = (Time.iso8601(initial.fetch(column)).to_r - BASE.to_r) * 1_000_000
      raise "#{spec[:name]}: persisted #{column} offset #{actual} != #{expected} microseconds" unless actual == expected
    end
    initial_offsets_us = {}
    {fetched_at: :fetched_offset_us, refresh_pending_at: :pending_offset_us}.each do |column, key|
      next unless spec.fetch(:cache, {}).key?(key)
      actual = (Time.iso8601(initial.fetch(column.to_s)).to_r - BASE.to_r) * 1_000_000
      expected = spec[:cache].fetch(key)
      raise "#{spec[:name]}: persisted #{column} offset #{actual} != #{expected} microseconds" unless actual == expected
      initial_offsets_us[column] = actual.to_i
    end
    calls.clear; ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    responses = spec.fetch(:responses, [[200, {items:spec.fetch(:items, [])}.to_json]])
    answers.replace(responses.map(&:dup))
    steps = spec.fetch(:offsets, [0]).map do |offset|
      at = BASE + offset
      result = Calendar::MeetingRefresh.refresh(spec[:missing_user] ? 0 : user.id, now:at)
      jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Calendar::MeetingRefreshJob }
        .map { |j| {class:j[:job].name, args:j[:args], at:Time.at(j[:at]).utc.iso8601(6)} }
      stored = cache_state.call(user.id)
      broadcast_claims = spec.fetch(:broadcast_claims, []).map do |active|
        won = Calendar::MeetingCache.find_by!(user_id:user.id).claim_broadcast!(active)
        {active:, won:, cache:cache_state.call(user.id)}
      end
      {at:at.utc.iso8601(6), result:, cache:stored, broadcast_claims:, calls:calls.dup, jobs:,
        disconnected:account&.reload&.disconnected_reason}
    end
    {spec:, initial:, initial_offsets_us:, responses:, steps:}
  ensure
    ActiveRecord::Base.connection_pool.disconnect!
    FileUtils.rm_f(["#{database}-wal", "#{database}-shm"])
    FileUtils.cp(backup, database); FileUtils.rm_f(backup)
  end
end
puts JSON.pretty_generate(reference:'d7c7de92', now:BASE.iso8601, rows:)
warn "Pinned Rails MeetingRefresh: #{rows.length} scenarios; #{rows.sum { |r| r[:steps].size }} ordered refreshes; recorded HTTP only"
