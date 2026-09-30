require "minitest"
require "active_support/test_case"
require "active_support/testing/time_helpers"
require "stringio"
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
ActionCable.server.config.cable={"adapter"=>"test"}
# The actual pinned test declarations, assertions, setup and private policy helpers run below.
# Only the shared test environment is supplied locally; each case uses an isolated transaction.
class ActiveSupport::TestCase
 include ActiveSupport::Testing::TimeHelpers
 def users(label);User.find(ActiveRecord::FixtureSet.identify(label));end
 def rooms(label);Room.find(ActiveRecord::FixtureSet.identify(label));end
 def assert_queries_count(count)
  queries=[]
  callback=ActiveSupport::Notifications.subscribe("sql.active_record") { |*args|p=args.last;queries << p[:sql] unless %w[SCHEMA TRANSACTION].include?(p[:name]) }
  yield
  assert_equal count,queries.size
 ensure
  ActiveSupport::Notifications.unsubscribe(callback)
 end
end
$ws17_calls=[]
$ws17_title=nil
module Ws17CapturePolicy
 def initialize(**args)
  super
  user=args[:recipient]
  keys=%w[presence_setting custom_status_emoji custom_status_text custom_status_expires_at dnd_enabled dnd_until quiet_hours_enabled quiet_hours_start_minute quiet_hours_end_minute time_zone meeting_status_enabled meeting_dnd_enabled ooo_until ooo_note ooo_calendar_enabled ooo_notify_enabled status role]
  attrs=user&.attributes&.slice(*keys)&.transform_values { |v|v.respond_to?(:iso8601) ? v.iso8601(6) : v }
  cache=user&.meeting_cache
  $ws17_calls << {args:args.slice(:kind,:mentioned,:reply_to_recipient,:keyword_matched,:dnd_exception).transform_values { |v|v.is_a?(Symbol) ? v.to_s : v },allowed_sender_ids:user ? DndAllowedUser.where(user_id:user.id).pluck(:allowed_user_id) : [],recipient_id:user&.id,sender_id:args[:sender]&.id,room:args[:room_membership]&.involvement,room_present:!args[:room_membership].nil?,thread:args[:thread_membership]&.involvement,attrs:,busy:cache&.busy_intervals||[],ooo:cache&.ooo_intervals||[],now:Time.current.iso8601(6),expected:{inbox:inbox_event_type,push:push?,sound:sound?}}
 rescue ArgumentError => error
  $ws17_calls << {args:args.slice(:kind),error:error.class.name,message:error.message}
  raise
 end
end
Notifications::Policy.prepend(Ws17CapturePolicy)
source=File.read(File.join(__dir__,"pinned/ws17-policy_test.rb")).sub('require "test_helper"',"")
eval(source,TOPLEVEL_BINDING,"/rails/test/models/notifications/policy_test.rb")
Minitest.seed=17
rows=[]
Notifications::PolicyTest.runnable_methods.sort.each do |method|
 $ws17_calls=[]
 result=nil
 ActiveRecord::Base.transaction do
  User.where(id:ActiveRecord::FixtureSet.identify(:david)).update_all(status:0,dnd_enabled:false,dnd_until:nil,quiet_hours_enabled:false,time_zone:"UTC",meeting_status_enabled:false,meeting_dnd_enabled:false,ooo_until:nil,ooo_note:nil,ooo_calendar_enabled:false,ooo_notify_enabled:false)
  Calendar::MeetingCache.delete_all;DndAllowedUser.delete_all
  result=Notifications::PolicyTest.new(method).run
  raise "#{method}: #{result.failures.map(&:message).join('; ')}" unless result.passed?
  rows << {test:method.delete_prefix("test_").tr('_',' '),method:,assertions:result.assertions,calls:$ws17_calls.dup}
  raise ActiveRecord::Rollback
 end
end
puts JSON.generate(reference:"d7c7de92",rows:)
