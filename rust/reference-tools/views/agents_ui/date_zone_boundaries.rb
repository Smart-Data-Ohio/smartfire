# TZInfo uses its explicit transition table. Unlike Jiff's tzdb reader, this
# pinned reader does not extrapolate future POSIX DST rules indefinitely.
result = {}
TZInfo::Timezone.all_identifiers.each do |name|
 tz = TZInfo::Timezone.get(name)
 transitions = tz.transitions_up_to(Time.utc(10000))
 first = transitions.first
 last = transitions.last
 result[name] = if last
  {first_at:first.at.to_i,first_offset:first.previous_offset.utc_total_offset,first_next_offset:first.offset.utc_total_offset,last_at:last.at.to_i,last_offset:last.offset.utc_total_offset,last_previous_offset:last.previous_offset.utc_total_offset}
 else
  {offset:tz.period_for_utc(Time.current).utc_total_offset}
 end
end
puts JSON.pretty_generate(result)
warn "Rails date zone boundaries: #{result.size} time zones"
