require 'json'
# Layout adapters only: no Notifications::Policy or outbound integration calls.
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
actor=User.find(127326141)
base={dnd_enabled:false,dnd_until:nil,presence_setting:'auto',quiet_hours_enabled:false,quiet_hours_start_minute:nil,quiet_hours_end_minute:nil,meeting_dnd_enabled:false,meeting_status_enabled:false,ooo_notify_enabled:false,ooo_calendar_enabled:false,ooo_until:nil,time_zone:nil}
now=Time.current
windows=[['2026-03-02T15:00:00Z','2026-03-02T16:30:00Z'],['2026-03-03T12:00:00+02:00','2026-03-03T12:30:00+02:00'],['bad','2026-03-02T16:00:00Z'],[],nil,['2026-03-02T18:00:00Z','2026-03-02T17:00:00Z']]
variants=[
 ['defaults',{},nil,nil],
 ['dnd_indefinite',{dnd_enabled:true},nil,nil],
 ['dnd_expiry_boundary',{dnd_enabled:true,dnd_until:now},nil,nil],
 ['dnd_future',{dnd_enabled:true,dnd_until:now+1.second},nil,nil],
 ['dnd_presence',{presence_setting:'dnd'},nil,nil],
 ['invisible_is_not_muted',{presence_setting:'invisible'},nil,nil],
 ['overnight',{quiet_hours_enabled:true,quiet_hours_start_minute:1320,quiet_hours_end_minute:420,time_zone:'Eastern Time (US & Canada)'},nil,nil],
 ['zero_start',{quiet_hours_enabled:true,quiet_hours_start_minute:0,quiet_hours_end_minute:60},nil,nil],
 ['equal_endpoints',{quiet_hours_enabled:true,quiet_hours_start_minute:60,quiet_hours_end_minute:60},nil,nil],
 ['incomplete_hours',{quiet_hours_enabled:true,quiet_hours_start_minute:60},nil,nil],
 ['meeting_windows',{meeting_dnd_enabled:true,meeting_status_enabled:true},windows,nil],
 ['meeting_without_status',{meeting_dnd_enabled:true},windows,nil],
 ['meeting_without_dnd',{meeting_status_enabled:true},windows,nil],
 ['manual_ooo',{ooo_until:now+1.hour},nil,nil],
 ['expired_ooo',{ooo_until:now},nil,nil],
 ['calendar_ooo',{ooo_calendar_enabled:true},nil,windows],
 ['combined_ooo',{ooo_until:now+1.hour,ooo_calendar_enabled:true},nil,windows],
 ['ooo_notifications_enabled',{ooo_until:now+1.hour,ooo_calendar_enabled:true,ooo_notify_enabled:true},nil,windows],
 ['drive',{},nil,nil,'https://www.googleapis.com/auth/drive.file'],
 ['legacy_drive',{},nil,nil,'https://www.googleapis.com/auth/drive.metadata.readonly'],
 ['drive_disconnected',{},nil,nil,"https://www.googleapis.com/auth/calendar.events\thttps://www.googleapis.com/auth/drive.file"],
 ['scopes_substring',{},nil,nil,'xhttps://www.googleapis.com/auth/drive.filex'],
 ['recent_searches',{},nil,nil]
]
cases=variants.map do |name,changes,busy,ooo,scopes|
 actor.update_columns(base.merge(changes));actor.reload
 Calendar::MeetingCache.where(user:actor).delete_all
 Calendar::MeetingCache.create!(user:actor,busy_intervals:busy||[],ooo_intervals:ooo||[]) if busy || ooo
 GoogleAccount.where(user:actor).delete_all
 GoogleAccount.create!(user:actor,email:'fixture@example.test',scopes:scopes,disconnected_reason:(name=='drive_disconnected' ? 'fixture' : nil)) if scopes
 Search.where(user:actor).delete_all
 if name=='recent_searches'
   12.times {|i|Search.insert_all!([{id:9000+i,user_id:actor.id,query:"query #{i} <&>",created_at:now-i.seconds,updated_at:now-i.seconds}])}
 end
 Current.reset;Current.user=actor
 cache=actor.meeting_cache
 muted=actor.manual_dnd_active? || actor.presence_setting=='dnd'
 quiet=actor.quiet_hours_enabled? && actor.quiet_hours_start_minute && actor.quiet_hours_end_minute ? [actor.quiet_hours_start_minute,actor.quiet_hours_end_minute] : nil
 meetings=actor.meeting_dnd_enabled? && actor.meeting_status_enabled? ? cache&.quiet_window_epochs.to_a : []
 ooo_epochs=[]
 unless actor.ooo_notify_enabled?
   ooo_epochs << [0,actor.ooo_until.to_i] if actor.manual_ooo_active?
   ooo_epochs.concat(cache&.ooo_window_epochs.to_a) if actor.ooo_calendar_enabled?
 end
 {name:name,input:{user:base.merge(changes),busy:busy||[],ooo:ooo||[],scopes:scopes,disconnected:name=='drive_disconnected',searches:actor.searches.ordered.map{|s|{id:s.id,query:s.query,updated_at:s.updated_at.iso8601(6)}}},expected:{muted:muted,quiet_hours:quiet,meeting_quiet:meetings,ooo_quiet:ooo_epochs,google_drive:!!actor.google_account&.drive?,recent_searches:actor.searches.ordered.limit(10).map{|s|{id:s.id,query:s.query}}},meta:ApplicationController.helpers.notification_sound_meta_tags.to_s}
end
picker_cases=(0...8).map do |bits|
  config={ 'GOOGLE_CLIENT_ID'=>'public-client<&>', 'GOOGLE_PICKER_API_KEY'=>'public-picker-key', 'GOOGLE_CLOUD_PROJECT_NUMBER'=>'12345' }
  config.keys.each_with_index {|key,index| bits[index]==1 ? ENV[key]=config[key] : ENV.delete(key)}
  {input:config.select{|key,_|ENV.key?(key)},configured:Google::Picker.configured?}
end
puts JSON.pretty_generate({reference_pin:'d7c7de92',now:now.to_i,cases:cases,picker_cases:picker_cases})
