# User destruction used by Slack undo must retain the real huddle callbacks.
require 'json'
ActiveJob::Base.queue_adapter = :test
%w[LIVEKIT_API_KEY LIVEKIT_API_SECRET LIVEKIT_INTERNAL_URL LIVEKIT_URL].each { |key| ENV.delete(key) }
db = ActiveRecord::Base.connection
stamp = db.quote(Time.current)
input = {
  'users' => [
    { 'id'=>9000, 'name'=>'Room owner', 'created_at'=>Time.current, 'updated_at'=>Time.current },
    { 'id'=>8800, 'name'=>'Unclaimed Slack placeholder', 'status'=>1, 'created_at'=>Time.current, 'updated_at'=>Time.current }
  ],
  'rooms' => [{ 'id'=>8801, 'name'=>'Stage retained by owner', 'type'=>'Rooms::Stage', 'creator_id'=>9000, 'created_at'=>Time.current, 'updated_at'=>Time.current }],
  'memberships' => [{ 'id'=>8811, 'room_id'=>8801, 'user_id'=>8800, 'stage_role'=>'speaker', 'created_at'=>Time.current, 'updated_at'=>Time.current }],
  # An orphaned old session does not claim the placeholder. Rails associations are optional.
  'huddle_grants' => [{ 'id'=>8821, 'identity'=>'ws16-fixture-participant', 'room_name'=>'ws16-fixture-stage', 'session_id'=>8831, 'user_id'=>8800, 'membership_id'=>8811, 'room_id'=>8801, 'stage_role'=>'speaker', 'last_seen_at'=>Time.current, 'created_at'=>Time.current, 'updated_at'=>Time.current }],
  'streams' => [{ 'id'=>8841, 'room_id'=>8801, 'membership_id'=>8811, 'user_id'=>8800, 'quality'=>'1080p30', 'started_at'=>Time.current, 'created_at'=>Time.current, 'updated_at'=>Time.current }]
}
input.each do |table, rows|
  rows.each do |row|
    db.execute("INSERT INTO #{db.quote_table_name(table)} (#{row.keys.map { |key| db.quote_column_name(key) }.join(',')}) VALUES (#{row.values.map { |value| db.quote(value) }.join(',')})")
  end
end
selectors = {'users'=>'id IN (8800,9000)', 'rooms'=>'id=8801', 'memberships'=>'id=8811', 'huddle_grants'=>'id=8821', 'streams'=>'id=8841', 'huddle_cleanups'=>'huddle_grant_id=8821'}
snapshot = -> { selectors.to_h { |table, where| [table, db.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a] } }
before = snapshot.call
User.find(8800).destroy!
after = snapshot.call
raise 'expected participant cleanup' unless after['huddle_cleanups'].size == 1
raise 'expected ended stream' unless after['streams'].first['ended_at'].present?
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/undo_huddle.json'),JSON.pretty_generate({'selectors'=>selectors,'before'=>before,'after'=>after})+"\n")
puts 'Slack undo huddle oracle: real Rails User.destroy callbacks; all rows and fields in six affected tables generated'
