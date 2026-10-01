# Independent production-request and transaction probes for PR #174, Rails d7c7de92.
require 'json'
require 'nokogiri'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
travel_to Time.utc(2026, 2, 10, 12)
fixtures = Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures, Dir[fixtures.join('**/*.yml')].map { |p| p.delete_prefix("#{fixtures}/").delete_suffix('.yml') }, {'twitter_posts' => Twitter::Post, 'twitter_post_references' => Twitter::PostReference})
ActiveJob::Base.queue_adapter = :test
Rails.application.routes.default_url_options.merge!(host: 'example.com', protocol: 'http')
ActionController::Base.allow_forgery_protection = false
room = Room.find(ActiveRecord::FixtureSet.identify(:watercooler))
david = User.find(ActiveRecord::FixtureSet.identify(:david))
production_500 = Rails.root.join('public/500.html').read
out = {production_500:, attendance: [], event_params: [], invitations: []}
travel_to Time.utc(2026, 3, 2, 16) do
  Current.user = david
  session = Session.create!(user: david, user_agent: 'ws14e-pr174', ip_address: '127.0.0.1', two_factor_verified_at: Time.current)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'example.com', 'rack.url_scheme' => 'http', 'REQUEST_METHOD' => 'GET'))
  jar = ActionDispatch::Cookies::CookieJar.build(request, {})
  jar.signed[:session_token] = {value: session.token}
  headers = {'HTTP_COOKIE' => "session_token=#{URI.encode_www_form_component(jar[:session_token])}", 'HTTP_ACCEPT' => 'text/html'}
  snapshot = -> { %w[events event_attendances event_references activity_items messages].map { |table| Event.connection.select_all("SELECT * FROM #{table} ORDER BY id").to_a } }
  shapes = {'string' => 'oops', 'empty_string' => '', 'number' => 7, 'true' => true, 'false' => false, 'array' => ['oops'], 'empty_array' => [], 'null' => nil, 'hash' => {}, 'nonempty_hash' => {'value' => 'oops'}}
  cases = [{'name' => 'review_form', 'form' => 'response=declined&message_id=601&attendance=oops'}]
  shapes.each do |label, shape|
    cases << {name: "response_lookup_#{label}", params: {message_id: '601', attendance: shape}}
    cases << {name: "message_lookup_#{label}", params: {response: 'declined', attendance: shape}}
    cases << {name: "apply_lookup_#{label}", params: {response: 'declined', message_id: '601', attendance: shape}}
    cases << {name: "lazy_bypass_#{label}", params: {response: 'declined', message_id: '601', apply_to_future: '1', attendance: shape}}
    cases << {name: "invalid_bypass_#{label}", params: {response: 'invalid', message_id: '601', attendance: shape}}
    cases << {name: "response_shape_#{label}", params: {response: shape, message_id: '601', attendance: {response: 'declined'}}}
    cases << {name: "message_shape_#{label}", params: {response: 'declined', message_id: shape, attendance: {message_id: '602'}}}
    cases << {name: "nested_response_shape_#{label}", params: {message_id: '601', attendance: {response: shape}}}
    cases << {name: "nested_message_shape_#{label}", params: {response: 'declined', attendance: {message_id: shape}}}
    cases << {name: "show_message_shape_#{label}", show: true, params: {message_id: shape}}
    cases << {name: "apply_shape_#{label}", params: {response: 'declined', message_id: '601', apply_to_future: shape, attendance: {apply_to_future: '1'}}}
  end
  Event.connection.execute("UPDATE sqlite_sequence SET seq=8000000000 WHERE name='events'")
  cases.each do |spec|
    [false, true].each do |frame|
      Rails.application.executor.run!(reset: true)
      Current.user = david
      event = room.events.create!(organizer: david, title: 'Shape probe', starts_at: Time.utc(2026, 3, 3, 9), time_zone: 'UTC', recurrence_rule: 'weekly', recurrence_until: Date.new(2026, 3, 17))
      target = event.series_events.order(:starts_at).second
      before = snapshot.call
      client = ActionDispatch::Integration::Session.new(Rails.application)
      h = headers.merge(frame ? {'HTTP_TURBO_FRAME' => 'response_for_message_601'} : {})
      if spec['form']
        client.patch("/rooms/#{room.id}/events/#{target.id}/attendance", params: spec['form'], headers: h.merge('CONTENT_TYPE' => 'application/x-www-form-urlencoded'))
      else
        client.public_send(spec[:show] ? :get : :patch, "/rooms/#{room.id}/events/#{target.id}/attendance", params: spec[:params], headers: h, as: :json)
      end
      response = client.response
      raise 'wrong production error body' if response.status == 500 && response.body != production_500
      out[:attendance] << spec.merge(frame:, status: response.status, unchanged: before == snapshot.call, responses: event.series_events.order(:starts_at).map { |e| e.response_for(david) }, frame_id: Nokogiri::HTML.fragment(response.body).at_css('turbo-frame.event-card__attendance')&.[]('id'), message_value: Nokogiri::HTML.fragment(response.body).at_css('input[name=message_id]')&.[]('value'), invalid: response.body.include?('Choose going, maybe, or declined.'), closed: response.body.include?('This event is no longer open for responses.'))
    end
  end
  # Sibling event actions: require/permit and prefill must retain their shape rules.
  shapes.each do |label, shape|
    %w[create update new].each do |action|
      Rails.application.executor.run!(reset: true)
      Current.user = david
      event = room.events.create!(organizer: david, title: 'Sibling shape', starts_at: Time.utc(2026, 3, 3, 9), time_zone: 'UTC')
      before = snapshot.call
      client = ActionDispatch::Integration::Session.new(Rails.application)
      path = "/rooms/#{room.id}/events"
      method = { 'create' => :post, 'update' => :patch, 'new' => :get }.fetch(action)
      path += "/#{event.id}" if action == 'update'
      path += '/new' if action == 'new'
      client.public_send(method, path, params: {event: shape}, headers: headers.dup, as: :json)
      response = client.response
      raise 'wrong sibling production error body' if response.status == 500 && response.body != production_500
      out[:event_params] << {name: "#{action}_#{label}", action:, params: {event: shape}, status: response.status, unchanged: before == snapshot.call}
    end
  end
  # Reject each position independently; earlier recipients have committed by then.
  invitation_room = Room.find(ActiveRecord::FixtureSet.identify(:designers))
  recipients = invitation_room.memberships.where(involvement: %w[mentions everything]).joins(:user).merge(User.active.where.not(id: david.id).where.not(role: :bot)).order('users.id').pluck('users.id')
  raise 'need at least two invitation recipients' unless recipients.size >= 2
  recipients.each_with_index do |rejected, position|
    Rails.application.executor.run!(reset: true)
    Current.user = david
    Event.connection.execute("CREATE TEMP TRIGGER ws14e_reject_invitation BEFORE INSERT ON activity_items WHEN NEW.source_type='Event' AND NEW.user_id=#{rejected} BEGIN SELECT RAISE(ABORT, 'ws14e rejected invitation'); END")
    event = invitation_room.events.build(organizer: david, title: "Invitation position #{position}", starts_at: Time.utc(2026, 3, 3, 9), time_zone: 'UTC')
    error = nil
    begin
      event.save!
    rescue => e
      error = e.class.name
    ensure
      Event.connection.execute('DROP TRIGGER ws14e_reject_invitation')
    end
    invited = ActivityItem.where(source: event, event_type: :event_invitation).order(:user_id).pluck(:user_id)
    out[:invitations] << {position:, recipients:, rejected:, error:, persisted: Event.exists?(event.id), invited:, organizer_response: event.response_for(david), announcements: event.referencing_messages.count}
  end
  Current.reset
end
File.write('/rails/storage/db/event-review-regressions.json', JSON.pretty_generate(out) + "\n")
puts "Rails PR174: #{out[:attendance].size} RSVP shapes, #{out[:event_params].size} sibling event shapes, #{out[:invitations].size} invitation failure positions"
out[:invitations].each { |s| puts s.to_json }
