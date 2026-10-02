# Review probe: compare real sidebar avatars and both presence APIs at the pin.
require 'json'
ApplicationController.allow_forgery_protection = false
ActionCable.server.define_singleton_method(:broadcast) { |*_| }
ENV.update('LIVEKIT_URL'=>'wss://public.example.test', 'LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880', 'LIVEKIT_API_KEY'=>'ws13-fixture-api-key', 'LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret', 'LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
viewer = User.find(127326141)
peers = [User.find(149087659), User.find(712064548)]
names = ['ΟΣ', 'οςa']
peers.zip(names).each { |user, name| user.update!(name: name) }
room = Rooms::Closed.create_for({name: 'Sigma review', creator: viewer}, users: [viewer, *peers])
peers.each do |user|
  session = user.sessions.create!(token: "ws13-sigma-#{user.id}", two_factor_verified_at: Time.current, last_active_at: Time.current)
  grant = HuddleGrant.issue!(session: session, membership: room.memberships.find_by!(user: user))
  grant.record_seen!
end
request = ActionDispatch::Integration::Session.new(Rails.application)
request.host! 'campfire.test'
session = viewer.sessions.create!(token: 'ws13-sigma-viewer', two_factor_verified_at: Time.current, last_active_at: Time.current)
jar = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test')).cookie_jar
jar.signed.permanent[:session_token] = {value: session.token, httponly: true, same_site: :lax}
request.cookies['session_token'] = jar[:session_token]
request.get('/users/me/sidebar', headers: {'Turbo-Frame'=>'user_sidebar'})
raise "sidebar status #{request.response.status}" unless request.response.status == 200
stack = Nokogiri::HTML5.fragment(request.response.body).at_css("#sidebar_voice_participants_rooms_closed_#{room.id}")
sidebar_names = stack.css('img.voice-stack__avatar').map { |img| img['title'] }
request.get('/users/huddle_presence')
raise "presence status #{request.response.status}" unless request.response.status == 200
presence = JSON.parse(request.response.body).find { |entry| entry['room_id'] == room.id }
presence_names = presence.fetch('participants').map { |user| user.fetch('name') }
request.get("/rooms/#{room.id}/huddle/participants")
raise "participants status #{request.response.status}" unless request.response.status == 200
participant_names = JSON.parse(request.response.body).map { |user| user.fetch('name') }
puts JSON.pretty_generate(reference_pin: 'd7c7de92', input_names: names, downcase: names.map(&:downcase), sidebar_names: sidebar_names, presence_names: presence_names, participant_names: participant_names)
