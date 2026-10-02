# SQL-generated upsert timestamps and explicitly assigned fetched_at have different precision.
require 'json'
now = Time.current
Calendar::MeetingCache.delete_all
attrs = {user_id:127326141, fetched_at:now, busy_intervals:[], ooo_intervals:[], fetch_error:nil, refresh_pending_at:nil}
Calendar::MeetingCache.upsert(attrs, unique_by: :user_id)
row = Calendar::MeetingCache.find_by!(user_id:127326141)
snapshot = -> { row.reload.attributes.slice('fetched_at','created_at','updated_at').transform_values { |v| v.utc.iso8601(6) } }
initial = snapshot.call
row.update_column(:updated_at, now - 300)
Calendar::MeetingCache.upsert(attrs, unique_by: :user_id)
puts JSON.generate(reference:'d7c7de92', now:now.utc.iso8601(6), **initial, after_repeat:snapshot.call)
