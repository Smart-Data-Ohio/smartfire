require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
travel_to Time.utc(2026,2,10,12)
fixtures=Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join('**/*.yml')].map{|p|p.delete_prefix("#{fixtures}/").delete_suffix('.yml')},{'twitter_posts'=>Twitter::Post,'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
class EventPageGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action&&method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=EventPageGoldenController.renderer.new(http_host:'campfire.test',https:false,'HTTP_USER_AGENT'=>'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36','rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'})
room=Room.find(ActiveRecord::FixtureSet.identify(:designers));david=User.find(ActiveRecord::FixtureSet.identify(:david));jason=User.find(ActiveRecord::FixtureSet.identify(:jz))
def event_fact(e,u,remaining:nil)
  h=ApplicationController.helpers;venue=e.venue
  {card:{id:e.id,room_id:e.room_id,title:e.title,organizer_name:e.organizer.name,starts_at:e.starts_at.utc.iso8601,ends_at:e.ends_at&.utc&.iso8601,time_zone:e.time_zone,series:e.series?,cancelled:e.cancelled?,venue_name:venue&.name,meet_link:h.safe_meet_link(e)},venue:venue && {id:venue.id,name:venue.name,stage:venue.stage?,member:u.rooms.exists?(id:venue.id),live_user:venue.stage? ? venue.live_stream&.user&.name : nil},going:e.attendances.where(response:'going').count,maybe:e.attendances.where(response:'maybe').count,declined:e.attendances.where(response:'declined').count,recurrence_label:e.recurrence_rule && h.recurrence_label(e.recurrence_rule),recurrence_phrase:e.recurrence_rule && h.recurrence_phrase(e.recurrence_rule),recurrence_until:e.recurrence_until && h.recurrence_until_text(e.recurrence_until),remaining:,manageable:e.manageable_by?(u),respondable:e.respondable_by?(u),current_response:e.response_for(u),previous:e.previous_occurrence&.id,next:e.next_occurrence&.id,head:e.series_head?,description_html:e.description.present? ? h.simple_format(e.description) : nil,calendar_copy:e.calendar_entries.exists?(user_id:u.id),attendances:e.attendances.order(:response,:id).map{|a|{name:a.user.name,response:a.response}}}
end
out=[]
travel_to Time.utc(2026,9,22,12) do
  voice=Rooms::Voice.create!(creator:david,name:'Calls & chat');voice.memberships.create!(user:david)
  stage=Rooms::Stage.create!(creator:david,name:'Presentation <&>');membership=stage.memberships.create!(user:david,stage_role:'host')
  Stream.create!(room:stage,membership:,user:david,quality:'720p15')
  events=[room.events.create!(organizer:david,title:'Planning <&>',starts_at:Time.utc(2026,10,5,9),ends_at:Time.utc(2026,10,5,10),time_zone:'Eastern Time (US & Canada)',description:"One line\nsecond line\n\n<script>bad</script><b>safe</b>"),room.events.create!(organizer:david,title:'Repeating',starts_at:Time.utc(2026,10,6,9),time_zone:'UTC',recurrence_rule:'weekly',recurrence_until:Date.new(2026,10,20),venue:stage),room.events.create!(organizer:david,title:'Past',starts_at:Time.utc(2026,9,20,9),time_zone:'UTC',venue:voice)]
  events[0].respond!(jason,'maybe');events[0].update_column(:meet_link,'https://meet.google.com/a?x=1&y=2')
  events << room.events.create!(organizer:david,title:'Cancelled',starts_at:Time.utc(2026,10,7,9),time_zone:'UTC');events.last.cancel!(actor:david)
  [david,jason].each do |u|
    ['UTC','Hawaii'].each do |zone|
      Time.use_zone(zone) do
        Current.user=u
        events.map{|e|Event.find(e.id)}.flat_map{|e| e.series? ? e.series_events.to_a : [e]}.each do |e|
          html=renderer.render(template:'rooms/events/show',assigns:{room:,event:e,attendances:e.attendances.includes(:user).order(:response,:id),current_response:e.response_for(u)})
          out << {kind:'show',user:u.name,zone:,view:{room_name:room.name,event:event_fact(e,u)},html:}
        end
        upcoming=room.events.upcoming.soonest_first.to_a;counts=upcoming.select(&:series?).group_by(&:series_id).transform_values(&:size)
        seen=[];upcoming=upcoming.select{|e| !e.series? || !seen.include?(e.series_id) && seen.push(e.series_id)}
        count_by_id=upcoming.select(&:series?).to_h{|e|[e.id,counts[e.series_id]]};past=room.events.past.ordered.to_a;cancelled=room.events.cancelled.ordered.to_a
        html=renderer.render(template:'rooms/events/index',assigns:{room:,upcoming_events:upcoming,past_events:past,cancelled_events:cancelled,series_counts:count_by_id})
        out << {kind:'index',user:u.name,zone:,view:{room_id:room.id,room_name:room.name,upcoming:upcoming.map{|e|event_fact(e,u,remaining:count_by_id[e.id])},past:past.map{|e|event_fact(e,u)},cancelled:cancelled.map{|e|event_fact(e,u)}},html:}
        Current.reset
      end
    end
  end
end
out.each do |state|
  Time.use_zone(state[:zone]) do
    Current.user=User.find_by!(name:state[:user])
    state[:logo]=renderer.render(inline:'<%= fresh_account_logo_path %>',layout:false)
    state[:avatar]=renderer.render(inline:'<%= fresh_user_avatar_path(Current.user) %>',layout:false)
    Current.reset
  end
end
File.write('/rails/storage/db/event-pages.json',JSON.pretty_generate(out)+"\n")
puts "Rails event full pages: #{out.size} states"
