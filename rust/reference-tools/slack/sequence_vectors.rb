# Run the actual Rails jobs, mappers, writer and undoer against recorded HTTP pages.
# Only transport, pacing, the clock (reference --freeze), and UUID inputs are fixed.
require 'json'
require 'net/http'
kind = ARGV.fetch(0, 'workspace')
raise 'unknown scenario' unless kind.in?(%w[workspace personal])
ActiveJob::Base.queue_adapter = :test
ENV['GOOGLE_SIGN_IN_DOMAINS'] = 'example.com'
SlackImport::Runner.step_budget = 0.seconds
Slack::Client.prepend(Module.new do
  def initialize(**args)
    super(**args.merge(pacing: false))
  end
end)
uuid_index = 0
Random.singleton_class.define_method(:uuid) do
  uuid_index += 1
  format('00000000-0000-4000-8000-%012d', uuid_index)
end
fixtures = Rails.root.join('test/fixtures/files/slack')
requests = []
transport = Object.new
transport.define_singleton_method(:request) do |request|
  raise 'wrong fixture token' unless request['Authorization'] == 'Bearer fixture-user-token'
  uri = URI(request.path)
  params = URI.decode_www_form(uri.query.to_s).to_h
  fixture = case uri.path
  when '/api/users.list' then 'users'
  when '/api/conversations.list'
    params.fetch('types').include?('public_channel') ? 'conversations_workspace' : 'conversations_personal'
  when '/api/conversations.members' then "members_#{params.fetch('channel')}"
  when '/api/conversations.history'
    channel = params.fetch('channel')
    channel == 'CCHAN' ? "history_CCHAN_p#{params['cursor'].present? ? 2 : 1}" : "history_#{channel}"
  when '/api/conversations.replies' then "replies_#{params.fetch('channel')}_parent"
  else raise "unrecorded Slack request: #{request.path}"
  end
  requests << {'path'=>uri.path, 'query'=>params}
  response = Net::HTTPOK.new('1.1', '200', 'OK')
  response['Content-Type'] = 'application/json'
  response.instance_variable_set(:@read, true)
  response.body = File.read(fixtures.join("#{fixture}.json"))
  response
end
transport.define_singleton_method(:get) do |path, headers|
  request(Net::HTTP::Get.new(path, headers))
end
Net::HTTP.singleton_class.prepend(Module.new do
  define_method(:start) do |host, *args, **options, &block|
    raise "unexpected network host #{host}" unless host == 'slack.com'
    block.call(transport)
  end
end)

db = ActiveRecord::Base.connection
# Every app table is compared, including empty tables which would reveal unintended
# notifications, agents or activity. Schema bookkeeping and FTS shadow tables are not
# application rows. Job persistence is separately tested on Rust's durable queue.
tables = db.tables.reject { |name| name.in?(%w[ar_internal_metadata schema_migrations]) || name.start_with?('sqlite_', 'message_search_index_') }.sort
tables << 'message_search_index'
tables << 'sqlite_sequence'
json_columns = tables.to_h { |table| [table, db.columns(table).select { |c| c.sql_type.downcase == 'json' }.map(&:name)] }
snapshot = lambda do
  tables.to_h do |table|
    columns = table == 'message_search_index' ? 'rowid AS id, body' : '*'
    rows = db.select_all("SELECT #{columns} FROM #{db.quote_table_name(table)}").to_a
    rows.each { |row| json_columns.fetch(table).each { |column| row[column] = JSON.parse(row[column]) if row[column].is_a?(String) } }
    # Keep every value; ordering is only a row-set presentation convention.
    rows.sort_by! { |row| row.fetch('id', row.fetch('name', '')).is_a?(Numeric) ? [0,row['id']] : [1,row.fetch('name','')] }
    [table, rows]
  end
end
owner = User.create!(name: 'Run owner')
User.create!(name: 'Existing Kevin', email_address: 'kevin@37signals.com', status: :deactivated)
workspace = SlackWorkspace.create!(client_id: 'fixture-client', client_secret: 'fixture-secret', configured_by: owner)
connection = SlackConnection.create!(slack_workspace: workspace, user: owner, slack_user_id: 'UADMIN', access_token: 'fixture-user-token')
initial = snapshot.call
drive = lambda do |run, job, expected|
  100.times do
    break unless run.reload.active?
    job.perform_now(run.id)
  end
  raise "sequence #{expected} stopped at #{run.reload.status}: #{run.error}" unless run.status == expected
end
run = SlackImport.start!(workspace: workspace, user: owner, connection: connection, kind: kind, mode: :import)
drive.call(run, SlackImport::StepJob, 'completed')
imported = snapshot.call
raise 'undo claim refused' unless run.undo!
drive.call(run, SlackImport::UndoJob, 'undone')
undone = snapshot.call
again = SlackImport.start!(workspace: workspace, user: owner, connection: connection, kind: kind, mode: :import)
drive.call(again, SlackImport::StepJob, 'completed')
reimported = snapshot.call
bounds_writer = Slack::MessageWriter.allocate
bounds = %w[1700000000.000001 1700000000.000002 1700000000.000003 1700000000.999999 -1700000000.000001 0.000001].flat_map do |ts|
  [ {'oldest'=>ts.to_f,'latest'=>nil}, {'oldest'=>nil,'latest'=>ts.to_f}, {'oldest'=>ts.to_f,'latest'=>ts.to_f} ].map do |bound|
    {'ts'=>ts, 'bounds'=>bound, 'contains'=>bounds_writer.send(:in_bounds?,ts,{oldest:bound['oldest'],latest:bound['latest']})}
  end
end
output = {'reference'=>'d7c7de9264c63015be398001d7a1094e7695a6db', 'time'=>Time.current.iso8601(6),
  'bounds'=>bounds, 'json_columns'=>json_columns, 'initial'=>initial, 'imported'=>imported, 'undone'=>undone,
  'reimported'=>reimported, 'requests'=>requests}
File.write(File.join(ENV.fetch('PARITY_WORK'), kind == 'workspace' ? 'vectors/slack/sequence.json' : 'vectors/slack/sequence_personal.json'), JSON.pretty_generate(output)+"\n")
puts "Slack Rails sequence (#{kind}): import -> undo -> reimport; #{tables.size} tables per snapshot; #{requests.size} recorded API requests"
