# Execute the original declarations against real models and isolated pinned fixtures.
require "minitest"
require "active_support/test_case"
require "active_support/testing/time_helpers"
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
class ActiveSupport::TestCase
  include ActiveSupport::Testing::TimeHelpers
  def users(label); User.find(ActiveRecord::FixtureSet.identify(label)); end
end
Minitest.seed=17
rows=[]
selected=["DND is manual-only outside quiet hours", "quiet hours cover an overnight window in the user's time zone", "an expired custom status reads as blank", "effective presence folds the manual setting over the lease state"]
[ ["status_settings",User,"test/models/user/status_settings_test.rb"], ["keyword_matcher",Notifications,"test/models/notifications/keyword_matcher_test.rb"] ].each do |name,namespace,file|
 source=File.read(File.join(__dir__,"pinned/ws17-#{name}_test.rb"))
 titles=source.scan(/test "([^"]+)"/).flatten.to_h { |title| ["test_#{title.gsub(/\s+/, '_')}",title] }
 eval(source.sub('require "test_helper"',""),TOPLEVEL_BINDING,"/rails/#{file}")
 klass=name=="status_settings" ? User::StatusSettingsTest : Notifications::KeywordMatcherTest
 klass.runnable_methods.sort.each do |method|
  next if name=="status_settings" && !selected.include?(titles.fetch(method))
  ActiveRecord::Base.transaction do
   User.where(id:ActiveRecord::FixtureSet.identify(:david)).update_all(presence_setting:"auto",custom_status_emoji:nil,custom_status_text:nil,custom_status_expires_at:nil,dnd_enabled:false,dnd_until:nil,quiet_hours_enabled:false,quiet_hours_start_minute:nil,quiet_hours_end_minute:nil,time_zone:nil,meeting_status_enabled:false,meeting_dnd_enabled:false,ooo_until:nil,ooo_calendar_enabled:false)
   Calendar::MeetingCache.delete_all
   ActiveSupport::TestCase.new("host").travel_to(Time.utc(2026,9,23,12))
   result=klass.new(method).run
   raise "#{method}: #{result.failures.map(&:message).join('; ')}" unless result.passed?
   rows << {file:,test:titles.fetch(method),assertions:result.assertions}
   raise ActiveRecord::Rollback
  end
 end
end
puts JSON.generate(reference:"d7c7de92",rows:)
