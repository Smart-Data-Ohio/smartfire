require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
room = Room.find(486777696)
viewer = User.find(127326141)
message = room.root_messages.create!(creator: viewer, markdown_source: 'Reaction source', client_message_id: 'modern-boosts')
WorkspaceIcon.create!(name: 'ws8bm_icon', title: 'Workspace <&>', creator: viewer,
  image: {io: File.open(Rails.root.join('test/fixtures/files/workspace_icons/clean.svg')), filename: 'ws8bm.svg', content_type: 'image/svg+xml'})
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
frames = []
ActionCable.server.singleton_class.prepend(Module.new do
  define_method(:broadcast) do |stream, payload, **options|
    frames << {stream:, payload:}
    super(stream, payload, **options)
  end
end)
rows = []
capture = ->(name, method, path, input, actor = viewer) do
  frames.clear
  request.cookie_jar.signed[:session_token] = actor.sessions.where.not(two_factor_verified_at: nil).first!.token
  actor_headers = headers.merge('Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}")
  browser.public_send(method, path, params: input, headers: actor_headers, as: :json)
  rows << {name:, user_id: actor.id, method:, path:, input:, status: browser.response.status, body: browser.response.body,
    location: browser.response.headers['Location'], cache_control: browser.response.headers['Cache-Control'], content_type: browser.response.headers['Content-Type'],
    boosts: message.reload.boosts.order(:id).map { |b| {id: b.id, content: b.content, booster_id: b.booster_id} }, frames: frames.dup}
end
base = "/messages/#{message.id}/boosts"
[['emoji','👍'], ['emoji_shortcode_toggle',':thumbsup: '], ['flag','🇺🇸'], ['keycap','1️⃣'], ['zwj','👨‍👩‍👧‍👦'],
 ['skin','👍🏽'], ['brand_alias',' :gpt: '], ['brand_canonical_toggle',':openai:'], ['custom',':ws8bm_icon:'],
 ['unknown',':does_not_exist:'], ['unknown_repeat',':does_not_exist:'], ['legacy','great <&>'], ['legacy_repeat','great <&>'],
 ['blank','   '], ['nil',nil], ['boolean',false], ['number',12], ['nbsp',"\u00a0text\u00a0"]].each do |name, content|
  capture.call(name, :post, base, {boost: {content:}})
end
duplicates = 2.times.map {Boost.insert_all!([{message_id: message.id, booster_id: viewer.id, content: '👍', created_at: Time.current, updated_at: Time.current}]).rows.first.first}
capture.call('duplicate_toggle', :post, base, {boost: {content: ':thumbsup:'}})
removed = message.boosts.find_by!(content: '🇺🇸')
capture.call('destroy', :delete, "#{base}/#{removed.id}", {})
capture.call('destroy_missing', :delete, "#{base}/#{removed.id}", {})
capture.call('missing_boost', :post, base, {})
[['flag_pair','🇺🇸'],['keycap_pair','1️⃣'],['family_pair','👨‍👩‍👧'],['vs16_pair','❤️'],['skin_pair','👍🏽'],
 ['nonquick_pair','💯'],['autocompleted_fire',':fire: '],['autocompleted_brand',':anthropic: '],
 ['plain_digit','1'],['plain_letter','a']].each do |name, content|
  2.times { |index| capture.call("#{name}_#{index}", :post, base, {boost: {content:}}) }
end
jason = User.find(149087659)
capture.call('custom_second_reactor', :post, base, {boost: {content: ':ws8bm_icon: '}}, jason)
capture.call('custom_first_reactor_off', :post, base, {boost: {content: ':ws8bm_icon:'}})
capture.call('custom_first_reactor_on', :post, base, {boost: {content: ':ws8bm_icon:'}})
capture.call('custom_second_reactor_off', :post, base, {boost: {content: ':ws8bm_icon:'}}, jason)
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', message_id: message.id, duplicates:, rows:) + "\n")
puts "WS8bm modern-boosts oracle: #{rows.size} actual toggle/alias/legacy/duplicate/delete/coercion requests; #{rows.sum { |r| r[:frames].size }} rendered reaction replacements"
