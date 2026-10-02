# Execute the pin's original pure-model declarations and capture each complete result.
require 'minitest'
require 'active_support/test_case'
require 'active_support/testing/time_helpers'
require 'digest'
require 'json'
Minitest.seed = 17
rows = []
$ws14g_interval_calls = []
module Ws14gMeetingIntervals
  def from_items(items)
    result = super
    $ws14g_interval_calls << {kind:'meeting', items:, zone:'UTC', result:result.map { |s,e| [s.iso8601,e.iso8601] }}
    result
  end
end
module Ws14gOooIntervals
  def from_items(items, zone:)
    result = super
    $ws14g_interval_calls << {kind:'ooo', items:, zone:, result:result.map { |s,e| [s.iso8601,e.iso8601] }}
    result
  end
end
Calendar::MeetingIntervals.singleton_class.prepend(Ws14gMeetingIntervals)
Calendar::OooIntervals.singleton_class.prepend(Ws14gOooIntervals)
class ActiveSupport::TestCase
  include ActiveSupport::Testing::TimeHelpers
  def users(label); User.find(ActiveRecord::FixtureSet.identify(label)); end
end
%w[meeting_intervals ooo_intervals].each do |name|
  file = "test/models/calendar/#{name}_test.rb"
  source = File.read(Rails.root.join(file))
  titles = source.scan(/test "([^"]+)"/).flatten.to_h { |title| ["test_#{title.gsub(/\s+/, '_')}", title] }
  eval(source.sub('require "test_helper"', ''), TOPLEVEL_BINDING, Rails.root.join(file).to_s)
  klass = name == 'meeting_intervals' ? Calendar::MeetingIntervalsTest : Calendar::OooIntervalsTest
  klass.runnable_methods.sort.each do |method|
    $ws14g_interval_calls.clear
    result = klass.new(method).run
    raise "#{method}: #{result.failures.map(&:message).join('; ')}" unless result.passed?
    rows << {file:, source_sha256:Digest::SHA256.hexdigest(source), test:titles.fetch(method), assertions:result.assertions, calls:$ws14g_interval_calls.dup}
  end
end
file = 'test/models/calendar/meeting_cache_test.rb'
source = File.read(Rails.root.join(file))
eval(source.sub('require "test_helper"', ''), TOPLEVEL_BINDING, Rails.root.join(file).to_s)
cache_uniqueness = nil
ActiveRecord::Base.transaction do
  Calendar::MeetingCache.delete_all
  result = Calendar::MeetingCacheTest.new('test_one_cache_per_user').run
  raise result.failures.map(&:message).join('; ') unless result.passed?
  cache_uniqueness = {file:, source_sha256:Digest::SHA256.hexdigest(source), test:'one cache per user', assertions:result.assertions, rows:Calendar::MeetingCache.count}
  raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(reference:'d7c7de92', rows:, cache_uniqueness:)
warn "Pinned Rails named intervals: #{rows.size} declarations; #{rows.sum { |r| r[:assertions] }} original assertions; 0 skipped"
warn "Pinned Rails cache uniqueness: 1 declaration; #{cache_uniqueness[:assertions]} original assertions; 0 skipped"
