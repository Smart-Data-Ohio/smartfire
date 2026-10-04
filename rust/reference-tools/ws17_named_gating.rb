require "minitest"
require "active_support/test_case"
require "active_support/testing/time_helpers"
require "active_job/test_helper"
require "mocha/minitest"
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
ActionCable.server.config.cable={"adapter"=>"test"}
eval(File.read(File.join(__dir__,"pinned/ws17-mention_test_helper.rb")),TOPLEVEL_BINDING)
eval(File.read(File.join(__dir__,"pinned/ws17-dns_test_helper.rb")),TOPLEVEL_BINDING)
class ActiveSupport::TestCase
 include ActiveSupport::Testing::TimeHelpers
 include ActiveJob::TestHelper
 include MentionTestHelper
 include DnsTestHelper
 def users(label);User.find(ActiveRecord::FixtureSet.identify(label));end
 def rooms(label);Room.find(ActiveRecord::FixtureSet.identify(label));end
 def events(label);Event.find(ActiveRecord::FixtureSet.identify(label));end
 def memberships(label);Membership.find(ActiveRecord::FixtureSet.identify(label));end
 setup do
  Resolv.stubs(:getaddresses).returns(["142.250.185.206"])
 end
end
Minitest.seed=17
rows=[]
[ ["push_gating","test/models/notifications/push_gating_test.rb"],["room_push","test/models/room/push_test.rb"] ].each do |name,file|
 source=File.read(File.join(__dir__,"pinned/ws17-#{name}_test.rb"))
 titles=source.scan(/test "([^"]+)"/).flatten.to_h { |title| ["test_#{title.gsub(/\s+/, '_')}",title] }
 eval(source.sub('require "test_helper"',""),TOPLEVEL_BINDING,"/rails/#{file}")
 klass=name=="push_gating" ? Notifications::PushGatingTest : Room::PushTest
 klass.runnable_methods.sort.each do |method|
  title=titles.fetch(method)
  next if name=="push_gating" && title.include?("huddle")
  next if name=="room_push" && title!="a forwarded note follows the mention push path while its snapshot does not"
  ActiveRecord::Base.transaction(joinable: false) do
   User.update_all(dnd_enabled:false,dnd_until:nil,presence_setting:"auto",quiet_hours_enabled:false,time_zone:nil,meeting_status_enabled:false,ooo_until:nil,ooo_calendar_enabled:false)
   DndAllowedUser.delete_all;Calendar::MeetingCache.delete_all;ActivityItem.delete_all
   ActiveSupport::TestCase.new("host").travel_to(Time.utc(2026,9,23,12))
   Event.where(id:ActiveRecord::FixtureSet.identify(:launch_party)).update_all(starts_at:2.days.from_now,ends_at:2.days.from_now+1.hour)
   result=klass.new(method).run
   raise "#{method}: #{result.failures.map(&:message).join('; ')}" unless result.passed?
   rows << {file:,test:title,assertions:result.assertions}
   raise ActiveRecord::Rollback
  end
 end
end
puts JSON.generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:)
