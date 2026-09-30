require "json"
require "active_record/fixtures"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
fixtures=Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join("**/*.yml")].map { |p|p.delete_prefix("#{fixtures}/").delete_suffix(".yml") }, {"twitter_posts"=>Twitter::Post,"twitter_post_references"=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
users=%w[david jason jz].to_h {|n|[n,User.find(ActiveRecord::FixtureSet.identify(n))]}
room=Room.find(ActiveRecord::FixtureSet.identify(:designers))
stamp=->(t) {t&.utc&.strftime("%Y-%m-%d %H:%M:%S.%6N")}
change=->(index,attrs,scope="this_and_following") {{kind:"update",index:,attrs:,scope:}}
rsvp=->(index,user,response) {{kind:"rsvp",index:,user:,response:}}
cancel=->(index,scope="this_event") {{kind:"cancel",index:,scope:}}
cases=[]
add=->(name,steps,input={}) {cases << {name:,input:{starts_at:"2026-10-05 09:00:00",ends_at:"2026-10-05 10:00:00",recurrence_rule:"weekly",recurrence_until:"2026-10-26"}.merge(input),steps:}}
add.call("local title",[change.call(1,{title:"Changed"},"this_event")])
add.call("default scope",[change.call(1,{title:"Changed"},"bogus")])
add.call("following title",[change.call(1,{title:"Changed",description:"Details"})])
add.call("following move onto next slot",[change.call(1,{starts_at:"2026-10-19 09:00:00",ends_at:"2026-10-19 10:00:00"})])
add.call("entire series onto next slot",[change.call(0,{starts_at:"2026-10-12 09:00:00",ends_at:"2026-10-12 10:00:00"})])
add.call("earlier following move",[change.call(2,{starts_at:"2026-10-18 09:00:00",ends_at:"2026-10-18 10:00:00"})])
add.call("local between neighbours",[change.call(1,{starts_at:"2026-10-13 09:00:00",ends_at:"2026-10-13 10:00:00"},"this_event")])
add.call("local past neighbour",[change.call(1,{starts_at:"2026-10-20 09:00:00",ends_at:"2026-10-20 10:00:00"},"this_event")])
%w[this_event this_and_following].each {|scope|add.call("previous bound #{scope}",[change.call(1,{starts_at:"2026-10-05 09:00:00"},scope)])}
add.call("head time guard",[change.call(0,{starts_at:"2026-10-05 09:30:00"},"this_event")])
add.call("follower rule guard",[change.call(1,{recurrence_rule:"daily"})])
add.call("head local rule guard",[change.call(0,{recurrence_rule:"daily"},"this_event")])
add.call("head remove rule",[change.call(0,{recurrence_rule:""})])
add.call("only start preserves follower duration",[change.call(1,{starts_at:"2026-10-12 09:30:00"})])
add.call("only end extends follower duration",[change.call(1,{ends_at:"2026-10-12 11:00:00"})])
add.call("remove ends",[change.call(1,{ends_at:nil})])
add.call("add ends",[change.call(1,{ends_at:"2026-10-12 10:00:00"})],{ends_at:nil})
add.call("extend copies responses",[rsvp.call(0,"jason","going"),change.call(0,{recurrence_until:"2026-11-09"})])
add.call("protect distinct RSVP",[rsvp.call(0,"jason","going"),rsvp.call(2,"jason","declined"),change.call(0,{recurrence_rule:"daily",recurrence_until:"2026-10-06"})])
add.call("cancel excess protected",[rsvp.call(0,"jason","going"),rsvp.call(1,"jason","declined"),rsvp.call(2,"jz","going"),change.call(0,{recurrence_rule:"daily",recurrence_until:"2026-10-06"})])
add.call("cancelled keeps slot",[rsvp.call(0,"jason","going"),rsvp.call(2,"jz","going"),cancel.call(1),change.call(0,{recurrence_until:"2026-10-12"})])
add.call("shrink keeps cancelled beyond range",[rsvp.call(0,"jason","going"),rsvp.call(3,"jason","declined"),cancel.call(2),change.call(0,{recurrence_until:"2026-10-12"})])
add.call("monthly parking collision",[rsvp.call(0,"jason","going"),rsvp.call(5,"jason","declined"),change.call(0,{recurrence_rule:"monthly",recurrence_until:"2027-07-31"})],{starts_at:"2027-01-31 10:00:00",ends_at:"2027-01-31 11:00:00",recurrence_until:"2027-03-07"})
add.call("cancel local",[rsvp.call(0,"jason","going"),cancel.call(1)])
add.call("cancel following",[rsvp.call(0,"jason","going"),rsvp.call(2,"jz","maybe"),cancel.call(1,"this_and_following")])
add.call("cancel all",[rsvp.call(0,"jason","going"),cancel.call(0,"this_and_following")])
add.call("cancel twice",[cancel.call(1),cancel.call(1,"this_and_following")])
add.call("cancel unknown defaults local",[cancel.call(1,"all")])
add.call("repeat updates coalesce",[rsvp.call(0,"jason","going"),change.call(1,{starts_at:"2026-10-12 09:10:00",ends_at:"2026-10-12 10:10:00"}),change.call(0,{starts_at:"2026-10-05 09:10:00",ends_at:"2026-10-05 10:10:00"})])
travel_to Time.utc(2026,9,22,12) do
  cases.each do |c|
    Event.destroy_all
    attrs=c[:input].transform_values.with_index {|v,_|v}
    %i[starts_at ends_at].each {|k| attrs[k]=attrs[k]&&Time.parse(attrs[k]).utc}
    attrs[:recurrence_until]=Date.parse(attrs[:recurrence_until])
    head=room.events.create!(**attrs,organizer:users["david"],title:"Planning session",time_zone:"UTC")
    ids=head.series_events.ids
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    c[:results]=c[:steps].map do |step|
      e=Event.find(ids[step[:index]])
      begin
        result=case step[:kind]
        when "rsvp" then e.respond!(users[step[:user]],step[:response]);nil
        when "cancel" then e.cancel_with_scope!(scope:step[:scope],actor:users["david"])
        when "update"
          a=step[:attrs].dup
          %i[starts_at ends_at].each {|k|a[k]=a[k]&&Time.parse(a[k]).utc if a.key?(k)}
          a[:recurrence_until]=Date.parse(a[:recurrence_until]) if a[:recurrence_until]
          e.update_with_scope!(a,scope:step[:scope],actor:users["david"])
        end
        {value:result}
      rescue ActiveRecord::RecordInvalid => ex
        {errors:ex.record.errors.map {|error|[error.attribute,error.message]}}
      end
    end
    ids += Event.where(series_id:head.id).order(:id).ids-ids
    c[:rows]=ids.map do |id|
      e=Event.find_by(id:)
      e && {starts_at:stamp.call(e.starts_at),ends_at:stamp.call(e.ends_at),title:e.title,description:e.description,rule:e.recurrence_rule,until:e.recurrence_until.to_s,cancelled:e.cancelled?,responses: e.attendances.order(:user_id).map {|a|[a.user_id,a.response]}}
    end
    c[:items]=ActivityItem.where(source_type:"Event",source_id:ids).order(:user_id,:source_id).map {|i|[i.user_id,ids.index(i.source_id),i.event_type,i.read_at.nil?,i.handled_at.nil?]}
    c[:jobs]=ActiveJob::Base.queue_adapter.enqueued_jobs.select {|j|j[:job].name.start_with?("Calendar::")}.map {|j|[j[:job].name,j[:args].each_with_index.map {|a,n|n==0 ? ids.index(a) : a}]}
  end
end
File.write("/rails/storage/db/event-scoped.json",JSON.pretty_generate(cases)+"\n")
puts "Rails event scoped vectors: #{cases.length} scenarios, #{cases.sum {|c|c[:steps].size}} operations"
