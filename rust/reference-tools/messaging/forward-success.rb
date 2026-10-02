# Positive wire bodies/rows/frames. Random.uuid is a deterministic fixture input to both ports,
# just like the frozen clock and the view oracle's fixed request secrets; no output is masked.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
ApplicationController.allow_forgery_protection = false
room = Room.find(486777696)
viewer = User.find(127326141)
source = room.root_messages.create!(creator: viewer, markdown_source: '**Forward snapshot**', client_message_id: 'success-source')
source.drive_attachments.create!(file_id: 'abcdefghij')
legacy = room.root_messages.create!(creator: viewer, body: '<div>Legacy &amp; <strong>safe</strong></div>', client_message_id: 'success-legacy')
file = room.root_messages.create!(creator: viewer, body: 'File snapshot', client_message_id: 'success-file', attachment: ActiveStorage::Blob.find(13))
thread = ChannelThread.create!(room:, creator: User.find(149087659), name: 'Success closed')
thread.update_columns(closed_at: Time.current, last_activity_at: 2.hours.ago)
targets = [room, Room.find(699448326), Room.find(186869642), thread]
recipients = [127326141, 149087659, 712064548, 394959859].map do |user_id|
  user = User.find(user_id)
  {user_id:, channels: targets.map do |target|
    stream = Turbo::StreamsChannel.send(:stream_name_from, [target, :messages])
    {stream:, allowed: RoomMessagesChannel.subscribable_target(user, stream).present?}
  end}
end
child = thread.messages.create!(room:, creator: viewer, markdown_source: 'Nested snapshot', client_message_id: 'success-child')
ThreadMembership.where(thread:, user: viewer).delete_all
thread.update_columns(closed_at: Time.current, last_activity_at: 2.hours.ago)
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
ids = (1..8).map { |i| "00000000-0000-4000-8000-#{i.to_s.rjust(12, '0')}" }
Random.singleton_class.prepend(Module.new {define_method(:uuid) { ids.shift || raise('fixture UUIDs exhausted') }})
rows = []
capture = ->(name, message, input, format = :json) do
  before = Message.maximum(:id)
  frames.clear
  path = "/messages/#{message.id}/forwards.#{format}"
  browser.post(path, params: input, headers: headers.dup, as: format == :json ? :json : nil)
  messages = Message.where('id > ?', before).order(:id).to_a
  rows << {name:, path:, input:, status: browser.response.status, body: browser.response.body,
    location: browser.response.headers['Location'], cache_control: browser.response.headers['Cache-Control'], content_type: browser.response.headers['Content-Type'],
    client_ids: messages.map(&:client_message_id), messages: messages.map { |m| {id: m.id, room_id: m.room_id, thread_id: m.thread_id, body: m.body.body.to_html,
      forwarded_from_message_id: m.forwarded_from_message_id, forwarded_markdown: m.forwarded_markdown, forward_note: m.forward_note, drive_ids: m.drive_attachments.pluck(:file_id)} },
    frames: frames.dup}
  messages
end
copies = capture.call('multiple_and_reopened', source, {forward: {note: "Note <&>\n@[Jason]", destinations: [{room_id: 699448326}, {room_id: room.id, thread_id: thread.id}, {room_id: 186869642}]}})
capture.call('legacy', legacy, {destinations: [{room_id: 699448326}], note: 'Legacy note'})
capture.call('forward_of_forward', copies.first, {destinations: [{room_id: room.id}]})
capture.call('file', file, {destinations: [{room_id: 699448326}]})
capture.call('nested_html', child, {destinations: [{room_id: 699448326}]}, :html)
raise 'forward fixture UUID count' unless ids.length == 1
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', source_id: source.id, legacy_id: legacy.id, file_id: file.id, thread_id: thread.id, child_id: child.id, recipients:, rows:) + "\n")
puts "WS8bm forward-success oracle: #{rows.size} positive actual requests; #{rows.sum { |r| r[:messages].size }} forwards; complete bodies/rows and #{rows.sum { |r| r[:frames].size }} rendered frames"
