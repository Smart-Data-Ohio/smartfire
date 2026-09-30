# Astra's fa404c9f reproductions, with nearby Unicode/ordering cases. Expected
# values come from our models, not a second implementation of the algorithms.
require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter = :test
user = User.find(ActiveRecord::FixtureSet.identify("david"))
room = Room.find(ActiveRecord::FixtureSet.identify("designers"))

keys = %w[paßword paẞword paſſword ſecret seßion ßession ßecret api_Key authorİzation authorızation authoﬃzation paßw seß ſession harmless]
filters = [{ "paßword" => "fixture-value" }, keys.to_h { |key| [key, "fixture-value"] }, { "nested" => [{ "paẞword" => "fixture-value" }, { "harmless" => "keep" }] }]
filters = filters.map { |input| { input: input, output: AuditLog.filter_secrets(input) } }

[["adjacent", "a b"], ["separated", "a x b"]].each do |client, body|
  room.messages.create!(creator: user, body: body, client_message_id: client)
end
queries = ["a＿b", "a‿b", "a⁀b", "a⁔b", "a︳b", "a︴b", "a﹍b", "a﹎b", "a﹏b", "a_b", "a b", "a-b", "a‍b", "a‌b", "a²b", "a①b", "aⒶb"]
search = queries.map do |raw|
  query = SearchQuery.parse(raw)
  { raw: raw, tokens: query.text_tokens, expression: query.match_expression, clients: query.apply_to_messages(user.reachable_messages).ordered.pluck(:client_message_id) }
end

direct = [["Ø Upper", "á Lower"], ["é Lower", "É Upper"], ["ΟΣ Test", "Οσ Test"]].map do |names|
  members = names.map { |name| User.create!(name: name) }
  dm = Rooms::Direct.create_for({ creator: user }, users: members)
  { names: names, default: dm.direct_display_name, preloaded: dm.direct_display_name(members: dm.users.to_a), viewer_default: dm.direct_display_name(for_user: members.first), viewer_preloaded: dm.direct_display_name(for_user: members.first, members: dm.users.to_a) }
end

source = "# hi"
message = room.messages.create!(creator: user, markdown_source: source)
saved = { source: source, body: message.reload.body.body.to_html }
edit_source = "# edited"
message.update!(markdown_source: edit_source)
edited = { source: edit_source, body: message.reload.body.body.to_html }
File.write(ARGV.fetch(0), JSON.pretty_generate({ filters: filters, search: search, direct: direct, saved: saved, edited: edited }) + "\n")
puts "WS8 review vectors: #{filters.size} redaction, #{search.size} phrase queries, #{direct.size} direct ordering, 2 canonicalized writes"
