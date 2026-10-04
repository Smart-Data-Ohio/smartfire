require 'json'
ActiveJob::Base.queue_adapter = :test
ActiveRecord::Schema.verbose = false
Net::HTTP.singleton_class.prepend(Module.new { define_method(:start) { |*| raise 'No Slack network allowed in review oracle' } })

def reset_review
  ActiveRecord::Base.clear_query_caches_for_current_thread
  load Rails.root.join('db/schema.rb')
  Account.create!(name: 'Review oracle')
  owner = User.create!(id: 811, name: 'Owner')
  workspace = SlackWorkspace.create!(id: 851, client_id: 'fixture-client', client_secret: 'fixture-secret')
  run = SlackImport.create!(id: 853, slack_workspace: workspace, user: owner, kind: :workspace, mode: :import)
  [owner, workspace, run]
end

rooms = [
  ['équipe', 'Équipe'], ['Équipe', 'équipe'], ['Équipe', 'Équipe'],
  ['GENERAL', 'general'], ['general', 'GENERAL'], ['Σ', 'σ'],
  ['ΟΣ', 'οσ'], ['οσ', 'ΟΣ'], ['İ', "i\u0307"],
  ['éQUIPE', 'équipe'], ['ß', 'SS']
].map do |incoming, existing|
  owner, workspace, run = reset_review
  Rooms::Open.create!(id: 861, name: existing, creator: owner)
  mapper = Slack::ConversationMapper.new(workspace: workspace, run: run)
  conversation = {'id' => 'CREVIEW', 'name' => incoming}
  preview = mapper.preview(conversation, member_ids: [], users: {})
  result = mapper.resolve(conversation, member_ids: [], users: {})
  record = run.records.find_by!(slack_kind: 'conversation')
  {incoming:, existing:, preview: {action: preview.action, room_id: preview.room&.id},
    result: {action: result.action, room_id: result.room.id, name: result.room.name,
      created: record.created_record?, memberships: result.room.memberships.count}}
end

emails = [
  ['équipe@example.com', 'Équipe@example.com'], ['Équipe@example.com', 'équipe@example.com'],
  ['Équipe@example.com', 'Équipe@example.com'], ['GENERAL@EXAMPLE.COM', 'general@example.com'],
  ['ΟΣ@example.com', 'οσ@example.com'], ['οσ@example.com', 'ΟΣ@example.com'],
  ['İ@example.com', "i\u0307@example.com"]
].map do |incoming, existing|
  _, workspace, run = reset_review
  User.create!(id: 812, name: 'Existing', email_address: existing)
  mapper = Slack::UserMapper.new(workspace: workspace, run: run)
  members = [{'id' => 'UREVIEW', 'name' => 'Incoming', 'profile' => {'email' => incoming}}]
  preview = mapper.preview_page(members)
  begin
    stats = mapper.map_page(members)
  rescue ActiveRecord::RecordNotUnique => error
    next({incoming:, existing:, preview:, error: error.cause.message,
      records: run.records.count, users: User.count})
  end
  record = run.records.find_by!(slack_kind: 'user')
  user = record.record
  {incoming:, existing:, preview:, stats:, user: {id: user.id, name: user.name, email: user.email_address, created: record.created_record?}}
end

owner, workspace, run = reset_review
mapper = Slack::UserMapper.new(workspace: workspace, run: run)
handles = ['Équipe', 'équipe', 'Équipe'].map do |name|
  user = mapper.bot_user_for({'username' => name})
  {name:, id: user.id}
end

owner, workspace, run = reset_review
names = ['οτ', 'ΟΣ', 'οςA'] + (0..7).map { |i| "ω#{i}" }
users = names.each_with_index.to_h { |name, i| ["U#{i}", User.create!(id: 812 + i, name:)] }
group = Slack::ConversationMapper.new(workspace: workspace, run: run).resolve(
  {'id' => 'GREVIEW', 'is_mpim' => true}, member_ids: users.keys, users:)

result = {reference: ENV.fetch("PARITY_REFERENCE_SHA"), rooms:, emails:, handles:,
  group: {names:, result: group.room.name}, downcase: ['ΟΣ', 'AΣ', 'İ', 'Équipe'].map { |input| {input:, output: input.downcase} }}
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/review_regressions.json'), JSON.pretty_generate(result) + "\n")
downcase = (0..0x10ffff).filter_map do |point|
  next if (0xd800..0xdfff).cover?(point)
  char = [point].pack('U')
  lowered = char.downcase
  [char, lowered] if char != lowered
end
File.write(File.join(ENV.fetch('PARITY_WORK'), 'crates/campfire/data/slack-ruby-downcase.json'), JSON.pretty_generate(downcase) + "\n")
puts "Slack review comparison oracle: #{rooms.size} room cases; #{emails.size} email cases; #{handles.size} handle cases; Unicode group name and 4 Ruby downcase values recorded"
puts "Slack Ruby downcase table: #{downcase.size} mappings recorded from the pinned Ruby runtime"
