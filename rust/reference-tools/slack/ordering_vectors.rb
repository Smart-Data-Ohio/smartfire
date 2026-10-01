require 'json'
ActiveJob::Base.queue_adapter = :test
ActiveRecord::Schema.verbose = false
Net::HTTP.singleton_class.prepend(Module.new { define_method(:start) { |*| raise 'No Slack network allowed in ordering oracle' } })

def reset_ordering
  ActiveRecord::Base.clear_query_caches_for_current_thread
  load Rails.root.join('db/schema.rb')
  Account.create!(name: 'Ordering oracle')
  owner = User.create!(id: 811, name: 'Owner')
  workspace = SlackWorkspace.create!(id: 851, client_id: 'fixture-client', client_secret: 'fixture-secret')
  run = SlackImport.create!(id: 853, slack_workspace: workspace, user: owner, kind: :workspace, mode: :import)
  [owner, workspace, run]
end

_, workspace, run = reset_ordering
names = ['Alice', 'alice', 'ALICE'] + (0..7).map { |i| "Z#{i}" }
keys = names.each_index.map { |i| 'U%02d' % i }
users = names.each_with_index.to_h { |name, i| [keys[i], User.create!(id: 812 + i, name:)] }
mapper = Slack::ConversationMapper.new(workspace:, run:)
direct = mapper.resolve({'id' => 'GDIRECT', 'is_mpim' => true}, member_ids: keys, users:).room.name
keys.reverse.each do |key|
  run.records.create!(slack_workspace: workspace, slack_kind: 'user', slack_key: key,
    record: users.fetch(key), created_record: true)
end
member_ids = keys.reverse + [keys.first, '', nil]
user_mapper = Slack::UserMapper.new(workspace:, run:)
mapped = user_mapper.users_for(member_ids)
duplicate_count = 40_000
duplicate_keys = user_mapper.users_for([keys.first] * duplicate_count).keys
mapped_name = mapper.resolve({'id' => 'GMAPPED', 'is_mpim' => true}, member_ids:, users: mapped).room.name
runner = SlackImport::Runner.new(run, client: Object.new)
missing_ids = ['UMISSING-Z', *keys.reverse, 'UMISSING-A']
ensured = runner.send(:ensured_users, missing_ids)

mentioned = {'U10' => users.fetch('U10')}
messages = [{'text' => '<@U02> <@U00> <@U02>', 'attachments' => [{'text' => '<@U01>'}]}]
writer = Slack::MessageWriter.new(workspace:, run:, user_mapper: Slack::UserMapper.new(workspace:, run:))
writer.send(:enrich_users_with_mentions!, mentioned, messages)

aliases = {'UZ' => users.fetch('U00'), 'UA' => users.fetch('U00'), 'UB' => users.fetch('U01')}
room = mapper.resolve({'id' => 'CALIAS', 'name' => 'alias', 'is_private' => true}, member_ids: aliases.keys, users: aliases).room
membership_keys = run.records.where(slack_kind: 'membership', record_id: room.memberships.pluck(:id)).order(:slack_key).pluck(:slack_key)

run.update!(status: :running, state: {'written_conversation_ids' => ['GMAPPED', 'GDIRECT', 'GMAPPED']})
runner = SlackImport::Runner.new(run, client: Object.new)
finished = []
runner.define_singleton_method(:finish_room) { |id| finished << id; super(id) }
runner.send(:finish_rooms)

result = {reference: 'd7c7de9264c63015be398001d7a1094e7695a6db', names:, keys:, member_ids:,
  direct_name: direct, mapped_keys: mapped.keys, mapped_name:, duplicate_count:, duplicate_keys:, missing_ids:, ensured_keys: ensured.keys,
  messages:, mention_keys: mentioned.keys, alias_keys: aliases.keys, membership_keys:,
  written_ids: run.state['written_conversation_ids'], finished_ids: finished}
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/ordering.json'), JSON.pretty_generate(result) + "\n")
puts "Slack ordering oracle: 11-member tied-name group; mapped, missing-author, mention, alias and finishing orders recorded"
