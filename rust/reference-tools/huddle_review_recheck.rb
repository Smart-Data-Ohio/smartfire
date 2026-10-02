require 'json'
ApplicationController.allow_forgery_protection = false
broadcasts = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| broadcasts << [stream, payload] }
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
viewer = User.find(127326141)
request = ActionDispatch::Integration::Session.new(Rails.application)
request.host! 'campfire.test'
session = viewer.sessions.create!(token: 'ws13-recheck-session', two_factor_verified_at: Time.current, last_active_at: Time.current)
jar = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test')).cookie_jar
jar.signed.permanent[:session_token] = {value: session.token, httponly: true, same_site: :lax}
request.cookies['session_token'] = jar[:session_token]
room = Room.find(699448332)
Current.reset
Current.user = viewer
renderer = ApplicationController.renderer.new(http_host:'campfire.test', https:false, 'rack.session'=>{})
nav = renderer.render(inline:'<%= render "rooms/boards/nav", room: @room %><%= content_for(:nav) %>', layout:false, assigns:{room:room})
membership = room.memberships.find_by!(user:viewer)
grant = HuddleGrant.issue!(session:session, membership:membership)
broadcasts.clear
grant.broadcast_voice_presence
header = broadcasts.filter_map { |_,payload| payload if payload.is_a?(String) && payload.include?('target="header_voice_participants_') }.fetch(0)
selects = -> {
  request.get('/users/me/sidebar', headers: {'Turbo-Frame'=>'user_sidebar'})
  queries = []
  subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') { |*args| queries << args.last[:sql] if args.last[:sql].lstrip.start_with?('SELECT') }
  request.get('/users/me/sidebar', headers: {'Turbo-Frame'=>'user_sidebar'})
  ActiveSupport::Notifications.unsubscribe(subscriber)
  raise "sidebar status #{request.response.status}" unless request.response.status == 200
  queries.size
}
# Warm the same signed session and rendering path before both measurements.
request.get('/users/me/sidebar', headers: {'Turbo-Frame'=>'user_sidebar'})
before = selects.call
5.times do |i|
  peer = User.create!(name: "Review peer #{i}", email_address: "ws13-review-peer-#{i}@example.test", password: 'fixture-password')
  Rooms::Direct.create_for({name: "Review group #{i}", creator: viewer}, users: [viewer, User.find(712064548), peer])
end
after = selects.call
request.get("/rooms/#{room.id}")
body = request.response.body
raise 'Board nav not present in real response' unless body.include?(nav)
puts JSON.pretty_generate(reference_pin:'d7c7de92', direct_sidebar:{peers_added:5,before_selects:before,after_selects:after}, board:{room_id:room.id,status:request.response.status,nav_html:nav,media_launchers:body.scan('data-controller="huddle-launcher"').size,new_posts:body.scan('aria-label="New post"').size,participant_ids:body.scan(/id="(header_voice_participants_[^"]+)"/).flatten,presence_replace:header})
