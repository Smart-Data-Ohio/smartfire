# Capture the original preload assertion at two batch sizes, without live HTTP.
require 'json'
ActiveJob::Base.queue_adapter=:test
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-preload-secret'
rows=[]
[3,30].each do |size|
  ActiveRecord::Base.transaction(requires_new:true) do
    user=User.find(127326141);room=Room.find(486777696)
    EventCalendarEntry.delete_all
    GoogleAccount.where(user:).delete_all
    GoogleAccount.create!(user:,email:'fixture@example.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:now+3600,scopes:Google::Client::CALENDAR_SCOPE)
    events=size.times.map do |i|
      event=room.events.create!(id:9_500_000_000+i,organizer:user,title:"Preload #{i}",starts_at:now+3600,time_zone:'UTC')
      event.respond!(user,'going')
      EventCalendarEntry.create!(event:,user:,google_event_id:Calendar::EntrySync.google_event_id_for(event.id,user.id),synced_at:now)
      event
    end
    calls=[]
    Google::Client.class_eval do
      define_method(:get_event) do |id|
        raise 'unrecorded event' unless events.any? { |e|Calendar::EntrySync.google_event_id_for(e.id,user.id)==id }
        calls << id
        {'status'=>'confirmed'}
      end
    end
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    queries=[]
    callback=->(*,payload) { queries << payload[:sql] if payload[:sql].include?('FROM "events"') }
    ActiveSupport::Notifications.subscribed(callback,'sql.active_record') { Calendar::InboundSyncJob.perform_now(user.id) }
    rows << {size:,event_reads:queries.size,calls:,responses:events.map { |e|e.reload.response_for(user) },jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j|j[:job].name }}
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:})
