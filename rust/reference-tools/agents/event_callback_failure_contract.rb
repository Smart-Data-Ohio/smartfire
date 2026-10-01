# PR #176 merge interaction: observe Calendar job enqueueing around a failing Event commit callback.
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
require 'digest'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
fixtures = Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures, Dir[fixtures.join('**/*.yml')].map { |p| p.delete_prefix("#{fixtures}/").delete_suffix('.yml') }, {'twitter_posts' => Twitter::Post, 'twitter_post_references' => Twitter::PostReference})
ActiveJob::Base.queue_adapter = :test
travel_to Time.utc(2026,9,22,12)
room=Room.find(ActiveRecord::FixtureSet.identify(:designers))
david=User.find(ActiveRecord::FixtureSet.identify(:david))
recipients=room.memberships.where(involvement:%w[mentions everything]).joins(:user).merge(User.active.where.not(id:david.id).where.not(role: :bot)).order('users.id').pluck('users.id')
results=[]
[false,true].each do |meet|
  recipients.each_with_index do |rejected,position|
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    Event.connection.execute("CREATE TEMP TRIGGER ws11_reject_event_invite BEFORE INSERT ON activity_items WHEN NEW.source_type='Event' AND NEW.user_id=#{rejected} BEGIN SELECT RAISE(ABORT,'WS11 rejected event invitation'); END")
    event=room.events.build(organizer:david,title:'Merge callback',starts_at:10.minutes.from_now,time_zone:'UTC',meet_link_requested:meet)
    error=nil
    begin
      event.save!
    rescue ActiveRecord::StatementInvalid
      error='statement_invalid'
    ensure
      Event.connection.execute('DROP TRIGGER ws11_reject_event_invite')
    end
    jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select {|j|j[:job].name.start_with?('Calendar::')}.map {|j|j[:job].name}
    results << {meet:,position:,error:,persisted:Event.exists?(event.id),invited:ActivityItem.where(source:event,event_type: :event_invitation).order(:user_id).pluck(:user_id),calendar_jobs:jobs,announcements:event.referencing_messages.count}
  end
end
sources=%w[app/models/event.rb app/models/event_attendance.rb].to_h {|path|[path,Digest::SHA256.hexdigest(Rails.root.join(path).read)]}
puts JSON.pretty_generate({reference:'d7c7de92',source_sha256:sources,results:results})
