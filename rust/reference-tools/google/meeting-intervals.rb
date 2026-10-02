require "json"
time = ->(s,e,attrs={}) { {"status"=>"confirmed","start"=>{"dateTime"=>s},"end"=>{"dateTime"=>e}}.merge(attrs) }
base=time.call("2026-09-23T10:00:00Z","2026-09-23T11:00:00Z")
cases = [
 ["busy",[base]], ["cancelled",[base.merge("status"=>"cancelled")]], ["focusTime",[base.merge("eventType"=>"focusTime")]],
 ["transparent",[base.merge("transparency"=>"transparent")]], ["ooo",[base.merge("eventType"=>"outOfOffice")]],
 ["self-declined",[base.merge("attendees"=>[{"self"=>true,"responseStatus"=>"declined"}])]],
 ["other-declined",[base.merge("attendees"=>[{"responseStatus"=>"declined"}])]],
 ["truthy-self",[base.merge("attendees"=>[{"self"=>0,"responseStatus"=>"declined"}])]],
 ["tentative",[base.merge("status"=>"tentative")]], ["all-day",[{"start"=>{"date"=>"2026-09-23"},"end"=>{"date"=>"2026-09-24"}}]],
 ["all-day-ooo",[{"eventType"=>"outOfOffice","start"=>{"date"=>"2026-03-07"},"end"=>{"date"=>"2026-03-10"}}]],
 ["all-day-ooo-utc",[{"eventType"=>"outOfOffice","start"=>{"date"=>"2026-03-07"},"end"=>{"date"=>"2026-03-10"}}]],
 ["malformed",[nil,"event",{},time.call("oops","2026-09-23T11:00:00Z"),time.call("2026-09-23T11:00:00Z","2026-09-23T10:00:00Z"),time.call("2026-09-23T10:00:00Z","2026-09-23T10:00:00Z")]],
 ["sort",[time.call("2026-09-23T13:00:00Z","2026-09-23T13:30:00Z"),base]],
 ["offset",[time.call("2026-09-23T06:00:00-04:00","2026-09-23T07:00:00-04:00")]]
]
puts JSON.pretty_generate({reference:"d7c7de9264c63015be398001d7a1094e7695a6db",cases:cases.map{|name,items|zone=(name == "all-day-ooo-utc" ? "UTC" : "America/New_York");{name:,items:,zone:,busy:Calendar::MeetingIntervals.from_items(items).map{|s,e|[s.iso8601,e.iso8601]},ooo:Calendar::OooIntervals.from_items(items,zone:).map{|s,e|[s.iso8601,e.iso8601]}}}})
