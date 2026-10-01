# Run the actual Rails jobs, mappers, writer and undoer against recorded HTTP pages.
# Only transport, pacing, the clock (reference --freeze), and UUID inputs are fixed.
require 'json'
require 'net/http'
require 'digest'
kind = ARGV.fetch(0, 'workspace')
retained = ARGV[1]
raise 'unknown scenario' unless kind.in?(%w[workspace personal])
ActiveJob::Base.queue_adapter = :test
ENV['GOOGLE_SIGN_IN_DOMAINS'] = 'example.com'
SlackImport::Runner.step_budget = 0.seconds
Slack::Client.prepend(Module.new do
  def initialize(**args)
    super(**args.merge(pacing: false))
  end
end)
# Encryption randomness is an input just like the clock and message UUIDs. This
# fixture-only IV provider keeps the initial opaque credentials reproducible; the
# real Rails encryptor still derives keys, encrypts, authenticates and serializes.
# These inputs are loaded unchanged by Rust and every encrypted field is compared.
iv_index = 0
ActiveRecord::Encryption::Cipher::Aes256Gcm.prepend(Module.new do
  define_method(:generate_iv) do |cipher, clear_text|
    if @deterministic
      super(cipher, clear_text)
    else
      iv_index += 1
      Digest::SHA256.digest("ws16-fixture-iv:#{iv_index}")[0, cipher.iv_len]
    end
  end
  private :generate_iv
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
  raise 'wrong fixture token' unless request['Authorization'] == ['Bearer', 'fixture-user-token'].join(' ')
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
  db.clear_query_cache
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
mutations = []
if retained
  mapped = ->(kind, key) { SlackImport::Record.find_by!(slack_workspace: workspace, slack_kind: kind, slack_key: key).record_id }
  room = mapped.call('conversation', 'CCHAN')
  parent = mapped.call('message', 'CCHAN:1700000002.000002')
  thread = mapped.call('thread', 'CCHAN:1700000002.000002')
  reply = mapped.call('message', 'CCHAN:1700000101.000101')
  later_reply = mapped.call('message', 'CCHAN:1700000102.000102')
  root = mapped.call('message', 'CCHAN:1700000001.000001')
  human = mapped.call('user', 'U003')
  bot = mapped.call('user', 'UBOT1')
  stamp = Time.current.utc.strftime('%Y-%m-%d %H:%M:%S')
  change = ->(sql, *params) { mutations << {'sql'=>sql, 'params'=>params}; db.raw_connection.execute(sql, params) }
  case retained
  when 'saved_reply'
    change.call('INSERT INTO saved_items(message_id,user_id,created_at,updated_at) VALUES(?,?,?,?)',reply,owner.id,stamp,stamp)
  when 'poll_reply'
    change.call('INSERT INTO polls(message_id,created_at,updated_at) VALUES(?,?,?)',later_reply,stamp,stamp)
  when 'pending_quoted_reply', 'sent_reply'
    change.call('INSERT INTO scheduled_messages(markdown_source,reply_to_message_id,thread_id,room_id,user_id,send_at,sent_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)','Later!',retained == 'sent_reply' ? nil : reply,thread,room,owner.id,stamp,retained == 'sent_reply' ? stamp : nil,stamp,stamp)
  when 'foreign_thread'
    change.call('INSERT INTO channel_threads(id,room_id,creator_id,parent_message_id,name,last_activity_at,created_at,updated_at) VALUES(3000,?,?,?,?,?,?,?)',room,owner.id,root,'Foreign thread',stamp,stamp,stamp)
    change.call('INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,markdown_source,created_at,updated_at) VALUES(4000,?,?,3000,?,?,?,?)',room,owner.id,'foreign-reply','real reply',stamp,stamp)
  when 'room_event_schedule'
    change.call('INSERT INTO events(room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,?,?,?,?,?,?)',room,owner.id,'Kickoff',stamp,'UTC',stamp,stamp)
    change.call('INSERT INTO scheduled_messages(markdown_source,room_id,user_id,send_at,created_at,updated_at) VALUES(?,?,?,?,?,?)','Reminder!',room,owner.id,stamp,stamp,stamp)
  when 'claimed_session'
    change.call('INSERT INTO sessions(user_id,token,last_active_at,created_at,updated_at) VALUES(?,?,?,?,?)',human,'fixture-session-token',stamp,stamp,stamp)
  when 'claimed_google_account'
    change.call('INSERT INTO google_accounts(user_id,email,created_at,updated_at) VALUES(?,?,?,?)',human,'claimed@example.test',stamp,stamp)
  when 'claimed_google_identity'
    change.call('INSERT INTO google_identities(user_id,email,subject,created_at,updated_at) VALUES(?,?,?,?,?)',human,'claimed@example.test','fixture-subject',stamp,stamp)
  when 'claimed_password'
    change.call('UPDATE users SET password_digest=? WHERE id=?','claimed',human)
  when 'placeholder_authorship'
    change.call('INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,created_at,updated_at) VALUES(4000,?,?,?,?,?,?)',room,bot,'foreign-root','bot posted after import',stamp,stamp)
  else raise "unknown retained scenario #{retained}"
  end
end
changed = snapshot.call
raise 'undo claim refused'  unless run.undo!
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
output.merge!('retained'=>retained, 'mutations'=>mutations, 'changed'=>changed) if retained
File.write(File.join(ENV.fetch('PARITY_WORK'), retained ? "vectors/slack/sequence_keep_#{retained}.json" : kind == 'workspace' ? 'vectors/slack/sequence.json' : 'vectors/slack/sequence_personal.json'), JSON.pretty_generate(output)+"\n")
puts "Slack Rails sequence (#{retained || kind}): import -> undo -> reimport; #{tables.size} tables per snapshot; #{requests.size} recorded API requests"
