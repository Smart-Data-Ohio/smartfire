require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
ApplicationController.allow_forgery_protection = false
room = Room.find(486777696)
viewer = User.find(127326141)
source = room.root_messages.create!(creator: viewer, markdown_source: '**Snapshot**', client_message_id: 'forward-source')
source.drive_attachments.create!(file_id: 'abcdefghij')
open_thread = ChannelThread.create!(room:, creator: viewer, name: 'Forward open')
locked = ChannelThread.create!(room:, creator: viewer, name: 'Forward locked'); locked.lock_conversation!
stale = ChannelThread.create!(room:, creator: viewer, name: 'Forward stale'); stale.update_columns(last_activity_at: 2.hours.ago, auto_archive_after_minutes: 60)
child = open_thread.messages.create!(room:, creator: viewer, markdown_source: 'Nested source', client_message_id: 'forward-child')
board = Rooms::Board.create_for({name: 'Forward board', creator: viewer}, users: [viewer])
copy = Room.find(699448326).root_messages.create!(creator: viewer, body: source.body.body.to_html, forwarded_from_message: source, forwarded_at: Time.current, forwarded_markdown: true, client_message_id: 'forward-copy')
nested_copy = open_thread.messages.create!(room:, creator: viewer, body: child.body.body.to_html, forwarded_from_message: child, forwarded_at: Time.current, forwarded_markdown: true, client_message_id: 'forward-nested-copy')
stale.update_columns(last_activity_at: 2.hours.ago, closed_at: nil)
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
rows=[]
capture = ->(name,method,path,input={}) do
  browser.public_send(method,path,params:input,headers:headers.dup,as: :json)
  rows << {name:,method:,path:,input:,status:browser.response.status,cache_control:browser.response.headers['Cache-Control'],content_type:browser.response.headers['Content-Type'],body:browser.response.body,message_count:Message.count}
end
base = "/rooms/#{room.id}/messages/#{source.id}/forwards"
capture.call('destinations',:get,"#{base}/destinations.json")
capture.call('global_destinations',:get,"/messages/#{source.id}/forwards/destinations.json")
capture.call('nested_destinations',:get,"/rooms/#{room.id}/threads/#{open_thread.id}/messages/#{child.id}/forwards/destinations.json")
[ ['empty',[]], ['too_many',Array.new(6){{room_id: room.id}}], ['missing_room',[{}]], ['unreachable',[{room_id:340026324}]],
  ['locked',[{room_id:room.id,thread_id:locked.id}]], ['wrong_thread',[{room_id:699448326,thread_id:open_thread.id}]], ['board',[{room_id:board.id}]], ['duplicate',[{room_id:room.id},{room_id:room.id}]],
  ['filtered_scalar',['room']], ['null',[nil]] ].each {|name,destinations|capture.call(name,:post,"#{base}.json",{forward:{destinations:}})}
capture.call('source_visible',:get,"/rooms/699448326/messages/#{copy.id}/forward_source.json")
capture.call('source_global',:get,"/messages/#{copy.id}/forward_source.json")
capture.call('nested_source',:get,"/rooms/#{room.id}/threads/#{open_thread.id}/messages/#{nested_copy.id}/forward_source.json")
capture.call('not_forwarded',:get,"/messages/#{source.id}/forward_source.json")
Membership.find_by!(room:,user:viewer).destroy!
capture.call('source_inaccessible',:get,"/messages/#{copy.id}/forward_source.json")
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',source_id:source.id,thread_id:open_thread.id,locked_id:locked.id,stale_id:stale.id,child_id:child.id,board_id:board.id,copy_id:copy.id,nested_copy_id:nested_copy.id,rows:) + "\n")
puts "WS8bm forwards oracle: #{rows.size} real picker/refusal/source-privacy requests; exact JSON bytes"
