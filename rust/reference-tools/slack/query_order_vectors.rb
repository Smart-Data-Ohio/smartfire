require 'json'
require 'digest'
ActiveJob::Base.queue_adapter = :test
ActiveRecord::Schema.verbose = false
Net::HTTP.singleton_class.prepend(Module.new { define_method(:start) { |*| raise 'No Slack network allowed in query oracle' } })
load Rails.root.join('db/schema.rb')
Account.create!(name: 'Query ordering oracle')
owner = User.create!(id: 811, name: 'Owner')
workspace = SlackWorkspace.create!(id: 851, client_id: 'fixture-client', client_secret: 'fixture-secret')
run = SlackImport.create!(id: 853, slack_workspace: workspace, user: owner, kind: :workspace, mode: :import)
mapper = Slack::UserMapper.new(workspace:, run:)
conn = ActiveRecord::Base.connection
now = Time.current
keys = (0...1600).map { |i| ['U', 'u', 'A', 'a'][i % 4] + '%04d' % (i / 4) }
users = keys.each_with_index.map { |key, i| {id: 10000 + i, name: ['Alice', 'alice', 'ALICE'][i % 3], created_at: now, updated_at: now} }
User.insert_all!(users)
records = keys.each_with_index.map { |key, i| {id: 20000 + i, slack_workspace_id: 851, slack_import_id: 853,
  slack_kind: 'user', slack_key: key, record_type: 'User', record_id: 10000 + i, created_record: true, created_at: now, updated_at: now} }
# Explicit rowids differ from both insertion order and the identity index's order.
SlackImport::Record.insert_all!(records.reverse)
escaping = ["U'quoted", "U\\backslash", "U\nnewline", 'équipe', "U') OR 1=1 --"]
inputs = {
  'unsorted_mixed_case' => [keys[7], keys[2], keys[5], keys[0], keys[3], keys[6], keys[1], keys[4]],
  'duplicates_blanks' => [keys[7], nil, keys[2], keys[7], '', keys[0], keys[2], '   ', keys[4]],
  'single' => [keys[120]],
  'bound_limit_997' => keys.reverse.take(997),
  'literal_limit_998' => keys.reverse.take(998),
  'over_999_real_mappings' => keys.reverse.take(1200),
  'all_1600_real_mappings' => keys.rotate(671).reverse,
  'over_32766_ids' => keys.reverse,
  'bound_escaping' => [keys.first, *escaping],
  'literal_escaping' => [*keys.reverse.take(1200), *escaping]
}
result = {reference: ENV.fetch("PARITY_REFERENCE_SHA"), sqlite: conn.select_value('SELECT sqlite_version()'),
  prepared_statements: conn.prepared_statements?, bind_limit: conn.send(:bind_params_length),
  records: keys.each_with_index.map { |key, i| {key:, user_id: 10000 + i, record_id: 20000 + i, name: users[i][:name]} }, cases: []}
['fresh', 'analyzed'].each do |state|
  conn.execute('ANALYZE') if state == 'analyzed'
  inputs.each do |name, mapped_ids|
    missing_count = name == 'over_32766_ids' ? 40_000 : 0
    ids = mapped_ids + (0...missing_count).map { |i| 'MISSING%05d' % i }
    executed = []
    subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
      payload = args.last
      if payload[:sql].start_with?('SELECT "slack_import_records"."slack_key"')
        executed << {sql: payload[:sql], binds: payload[:binds].map(&:value_for_database)}
      end
    end
    begin
      mapped = mapper.users_for(ids)
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end
    raise 'Expected one real mapping query' unless executed.size == 1
    query = executed.fetch(0)
    plan = conn.raw_connection.execute('EXPLAIN QUERY PLAN ' + query[:sql], query[:binds]).map { |row| row.is_a?(Hash) ? row.fetch('detail') : row[3] }
    group_name = Slack::ConversationMapper.new(workspace:, run:).send(:mpim_closed_name, {'name' => 'Plan group'}, mapped)
    result[:cases] << {name: state + '_' + name, analyzed: state == 'analyzed', mapped_ids:, missing_count:,
      expected_keys: mapped.keys, group_name:, sql_sha256: Digest::SHA256.hexdigest(query[:sql]), binds: query[:binds], plan:}
    puts "Slack query #{state}_#{name}: #{ids.compact_blank.uniq.size} distinct ids; #{query[:binds].size} binds; #{mapped.size} matches; #{plan.join('; ')}"
  end
end
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/query_order.json'), JSON.pretty_generate(result) + "\n")
puts "Slack query ordering oracle: #{result[:cases].size} executed SQL cases; 1600 mapped users; before/after ANALYZE; SQLite #{result[:sqlite]}"
