require 'json'
ApplicationController.allow_forgery_protection = false
ActionCable.server.define_singleton_method(:broadcast) { |*_| }
Room::DestroyJob.define_singleton_method(:perform_later) { |*_| }
viewer = User.find(127326141)
members = [viewer, User.find(149087659), User.find(712064548)]
room = Rooms::Stage.create_for({name: 'Review stage', creator: viewer}, users: members)
request = ActionDispatch::Integration::Session.new(Rails.application)
request.host! 'campfire.test'
session = viewer.sessions.create!(token: 'ws13-review-session', two_factor_verified_at: Time.current, last_active_at: Time.current)
jar = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test')).cookie_jar
jar.signed.permanent[:session_token] = {value: session.token, httponly: true, same_site: :lax}
request.cookies['session_token'] = jar[:session_token]
ENV.delete('LIVEKIT_API_SECRET')
request.get("/rooms/#{room.id}")
stage = {status: request.response.status, show_stage: request.response.body.include?('aria-label="Show stage"'), dialog: request.response.body.include?("id=\"stage_panel_dialog_rooms_stage_#{room.id}\""), roster: request.response.body.include?("id=\"stage_roster_rooms_stage_#{room.id}\""), media_launcher: request.response.body.include?('data-controller="huddle-launcher"')}
ActiveRecord::Base.connection.execute("CREATE TRIGGER ws13_reject_room_audit BEFORE INSERT ON audit_logs WHEN NEW.action='room.destroy' BEGIN SELECT RAISE(ABORT,'WS13 rejected audit'); END")
request.delete("/rooms/#{room.id}", headers: {'Accept' => 'application/json'})
room.reload
deletion = {status: request.response.status, deleted: !!room.deleted_at, memberships: room.memberships.count}
ActiveRecord::Base.connection.execute('DROP TRIGGER ws13_reject_room_audit')
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
identities = [Rooms::Open, Rooms::Closed, Rooms::Direct, Rooms::Voice, Rooms::Stage, Rooms::Board].map do |klass|
  item = klass.create_for({name: 'Review identity', creator: viewer}, users: members)
  request.get("/rooms/#{item.id}")
  {kind: klass.name, status: request.response.status, label: request.response.body[/<span class="room-header__kind">([^<]+)<\/span>/, 1], header_id: request.response.body[/class="room-header__identity room--current min-width" id="([^"]+)"/, 1].sub(item.id.to_s, ':id')}
end
100.times { |i| Rooms::Closed.create_for({name: "Quiet #{i}", creator: viewer}, users: [viewer]) }
queries = []
subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') { |*args| queries << args.last[:sql] unless args.last[:cached] }
request.get('/users/me/sidebar')
ActiveSupport::Notifications.unsubscribe(subscriber)
sidebar = {status: request.response.status, participant_queries: queries.count { |q| q.match?(/\bFROM "huddle_grants"/) }, stream_queries: queries.count { |q| q.match?(/\bFROM "streams"/) }}
puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], audit_failure: deletion, unconfigured_stage: stage, identities: identities, quiet_sidebar: sidebar)
