# Run in our pinned Rails image; Event::Recurrence is the oracle, including
# ActiveSupport's gap/fold resolution and loss of fractional seconds in zone.local.
require "json"

cases = []
add = ->(name, zone, local_start, rule, until_date, duration = 3600) do
  starts = ActiveSupport::TimeZone[zone].parse(local_start)
  ends = duration && starts + duration
  finish = Date.iso8601(until_date)
  cases << {
    name:, time_zone: zone, starts_at: starts.utc.strftime("%Y-%m-%d %H:%M:%S.%6N"),
    ends_at: ends&.utc&.strftime("%Y-%m-%d %H:%M:%S.%6N"), rule:, until_date:,
    slots: Event::Recurrence.slots(starts_at: starts, ends_at: ends, time_zone: zone,
      rule:, until_date: finish).map { |s, e| [s.utc.strftime("%Y-%m-%d %H:%M:%S.%6N"), e&.utc&.strftime("%Y-%m-%d %H:%M:%S.%6N")] }
  }
end
%w[daily weekly biweekly monthly].each do |rule|
  add.call("New York spring #{rule}", "Eastern Time (US & Canada)", "2026-03-01 09:15:42", rule, "2026-04-15")
  add.call("Berlin autumn #{rule}", "Europe/Berlin", "2026-10-01 09:15:42", rule, "2026-11-15")
end
add.call("NY gap", "America/New_York", "2026-03-07 02:30:00", "daily", "2026-03-09")
add.call("NY fold", "America/New_York", "2026-10-31 01:30:00", "daily", "2026-11-02")
add.call("Lord Howe half-hour gap", "Australia/Lord_Howe", "2026-10-03 02:15:00", "daily", "2026-10-05")
add.call("Lord Howe fold", "Australia/Lord_Howe", "2026-04-04 01:45:00", "daily", "2026-04-06")
add.call("Apia skipped date", "Pacific/Apia", "2011-12-29 09:00:00", "daily", "2011-12-31")
add.call("fractional head", "UTC", "2026-01-01 09:00:00.123456", "daily", "2026-01-03", 3600.654321)
add.call("no end", "Asia/Kolkata", "2026-01-01 09:00:00", "weekly", "2026-02-01", nil)
add.call("inclusive end", "UTC", "2026-01-01 09:00:00", "weekly", "2026-01-15")
add.call("unknown rule defaults weekly", "UTC", "2026-01-01 09:00:00", "unknown", "2026-01-15")
add.call("52 boundary", "UTC", "2026-01-01 09:00:00", "daily", "2026-02-21")
add.call("53 count is not truncated", "UTC", "2026-01-01 09:00:00", "daily", "2026-02-22")
[2027, 2028].each do |year|
  [29, 30, 31].each do |day|
    add.call("month end #{year}-#{day}", "America/New_York", "#{year}-01-#{day} 09:00:00", "monthly", "#{year}-05-31")
  end
end
add.call("leap head", "UTC", "2028-02-29 09:00:00", "monthly", "2029-02-28")
File.write("/rails/storage/db/event-recurrence.json", JSON.pretty_generate(cases) + "\n")
puts "Rails event recurrence vectors: #{cases.length} cases, #{cases.sum { |c| c[:slots].length }} slots"

now = Time.utc(2026, 9, 22, 12)
windows = [-3601, -3600, -301, -300, 900, 901].map do |offset|
  event = Event.new(starts_at: now + offset)
  due = ((now - Event::ReminderDispatcher::REMIND_AFTER_GRACE)..(now + Event::ReminderDispatcher::REMIND_BEFORE)).cover?(event.starts_at)
  { offset:, due:, pushed: due && !Event::ReminderPusher.stale?(event, now:) }
end
File.write("/rails/storage/db/event-reminder-window.json", JSON.pretty_generate(windows) + "\n")
puts "Rails event reminder windows: #{windows.length} cases"
