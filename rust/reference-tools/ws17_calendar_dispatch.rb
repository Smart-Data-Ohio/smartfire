require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
ActionCable.server.config.cable={"adapter"=>"test"}
user=User.find_by!(email_address:"david@37signals.com")
base={status:0,meeting_status_enabled:false,meeting_dnd_enabled:false,ooo_calendar_enabled:false,ooo_until:nil,ooo_note:nil,ooo_broadcast:nil,ooo_notify_enabled:false,presence_setting:"auto",custom_status_text:nil,custom_status_emoji:nil,time_zone:"UTC",dnd_enabled:false}
rows=[]
travel_to(Time.utc(2026,3,2,16)) do
 User.update_all(meeting_status_enabled:false,ooo_calendar_enabled:false,ooo_until:nil)
 WorkspacePresenceLease.delete_all
 scenarios=[
  {name:"meeting_start",kind:"meeting",attrs:{meeting_status_enabled:true},busy:[["2026-03-02T15:55:00Z","2026-03-02T16:55:00Z"]]},
  {name:"meeting_steady",kind:"meeting",attrs:{meeting_status_enabled:true},busy:[["2026-03-02T15:55:00Z","2026-03-02T16:55:00Z"]],claimed:true},
  {name:"meeting_end",kind:"meeting",attrs:{meeting_status_enabled:true},busy:[["2026-03-02T15:05:00Z","2026-03-02T15:55:00Z"]],claimed:true},
  {name:"meeting_stale",kind:"meeting",attrs:{meeting_status_enabled:true},fetched_at:"2026-03-02T15:44:00Z"},
  {name:"meeting_stale_boundary",kind:"meeting",attrs:{meeting_status_enabled:true},fetched_at:"2026-03-02T15:45:00Z"},
  {name:"meeting_missing",kind:"meeting",attrs:{meeting_status_enabled:true},missing:true},
  {name:"meeting_unfetched",kind:"meeting",attrs:{meeting_status_enabled:true},fetched_at:nil},
  {name:"meeting_fresh",kind:"meeting",attrs:{meeting_status_enabled:true}},
  {name:"meeting_off",kind:"meeting"},
  {name:"meeting_inactive",kind:"meeting",attrs:{meeting_status_enabled:true,status:1}},
  {name:"meeting_malformed",kind:"meeting",attrs:{meeting_status_enabled:true},busy:[nil,"nope",["bad","2026-03-02T16:55:00Z"]]},
  {name:"ooo_manual",kind:"ooo",attrs:{ooo_until:"2026-03-03T16:00:00Z",ooo_note:"Back <soon> & safe"}},
  {name:"ooo_steady",kind:"ooo",attrs:{ooo_until:"2026-03-03T16:00:00Z",ooo_broadcast:true}},
  {name:"ooo_end",kind:"ooo",attrs:{ooo_until:"2026-03-02T15:00:00Z",ooo_note:"Expired",ooo_broadcast:true}},
  {name:"ooo_expired_already_false",kind:"ooo",attrs:{ooo_until:"2026-03-02T15:00:00Z",ooo_note:"Expired",ooo_broadcast:false}},
  {name:"ooo_calendar",kind:"ooo",attrs:{ooo_calendar_enabled:true},ooo:[["2026-03-02T15:55:00Z","2026-03-04T16:00:00Z"]]},
  {name:"ooo_stale",kind:"ooo",attrs:{ooo_calendar_enabled:true},fetched_at:"2026-03-02T15:44:00Z"},
  {name:"ooo_both",kind:"both",attrs:{meeting_status_enabled:true,ooo_calendar_enabled:true},fetched_at:"2026-03-02T15:44:00Z"},
  {name:"ooo_missing",kind:"ooo",attrs:{ooo_calendar_enabled:true},missing:true},
  {name:"ooo_unfetched",kind:"ooo",attrs:{ooo_calendar_enabled:true},fetched_at:nil},
  {name:"ooo_missing_previous_true",kind:"ooo",attrs:{ooo_calendar_enabled:true,ooo_broadcast:true},missing:true},
  {name:"ooo_off",kind:"ooo",missing:true},
  {name:"ooo_inactive",kind:"ooo",attrs:{ooo_until:"2026-03-03T16:00:00Z",status:1}},
  {name:"ooo_invisible",kind:"ooo",attrs:{ooo_until:"2026-03-03T16:00:00Z",presence_setting:"invisible"}},
  {name:"ooo_calendar_malformed",kind:"ooo",attrs:{ooo_calendar_enabled:true},ooo:[nil,"nope",["bad","2026-03-02T16:55:00Z"]]},
 ]
 scenarios.each do |row|
  user.update_columns(**base.merge(row.fetch(:attrs,{})))
  Calendar::MeetingCache.where(user:).delete_all
  unless row[:missing]
   Calendar::MeetingCache.create!(user:,fetched_at:row.fetch(:fetched_at,Time.current),busy_intervals:row.fetch(:busy,[]),ooo_intervals:row.fetch(:ooo,[]),in_meeting_broadcast:row[:claimed])
  end
  runs=[]
  2.times do
   ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   frames=[];updates=[]
   broadcast=ActiveSupport::Notifications.subscribe("broadcast.action_cable") { |*args| p=args.last;frames << {stream:p[:broadcasting],html:p[:message]} }
   sql=ActiveSupport::Notifications.subscribe("sql.active_record") { |*args| text=args.last[:sql];updates << text if text.start_with?("UPDATE") }
   Calendar::MeetingDispatcher.dispatch_due! if %w[meeting both].include?(row[:kind])
   Calendar::OooDispatcher.dispatch_due! if %w[ooo both].include?(row[:kind])
   ActiveSupport::Notifications.unsubscribe(broadcast);ActiveSupport::Notifications.unsubscribe(sql)
   user.reload
   runs << {frames:,updates:updates.size,jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map { |job| {class:job[:job].name,args:job[:args]} },stored:user.attributes.slice("ooo_broadcast","ooo_until","ooo_note").transform_values { |v|v.is_a?(Time) ? v.iso8601(6) : v },meeting_claim:user.meeting_cache&.in_meeting_broadcast}
  end
  rows << row.merge(runs:)
 end
end
tasks=Periodic::Runner.new.instance_variable_get(:@tasks).map { |task| {name:task.name,seconds:task.interval.to_i} }
puts JSON.generate(tasks:,reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:"2026-03-02T16:00:00Z",rows:)
