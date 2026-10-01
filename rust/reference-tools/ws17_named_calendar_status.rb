require "minitest"
require "active_support/test_case"
require "active_support/testing/time_helpers"
ActiveRecord::Base.logger = nil
ActiveJob::Base.queue_adapter = :test
ActionCable.server.config.cable = { "adapter" => "test" }
class ActiveSupport::TestCase
  include ActiveSupport::Testing::TimeHelpers
  def users(label); User.find(ActiveRecord::FixtureSet.identify(label)); end
end

module Ws17StatusCapture
  KEYS = %w[status presence_setting custom_status_emoji custom_status_text custom_status_expires_at
    dnd_enabled dnd_until quiet_hours_enabled quiet_hours_start_minute quiet_hours_end_minute
    time_zone meeting_status_enabled meeting_dnd_enabled ooo_until ooo_note ooo_calendar_enabled
    ooo_notify_enabled ooo_broadcast].freeze
  def self.encode(value)
    case value
    when Time, ActiveSupport::TimeWithZone then value.utc.iso8601(6)
    when User then { user_id: value.id }
    when Hash then value.transform_values { |v| encode(v) }
    when Array then value.map { |v| encode(v) }
    else value
    end
  end
  def self.snapshot(user)
    encode(user.attributes.slice(*KEYS))
  end
  METHODS = %i[update update! update_columns reload deactivate claim_ooo_broadcast! ooo_preset_until
    manual_ooo_active? calendar_ooo_active? out_of_office? ooo_until_effective ooo_until_date
    ooo_status_visible? ooo_status_text status_text_display in_meeting? meeting_status_visible?
    meeting_dnd_active? ooo_dnd_active? quiet_hours_active? ooo_until ooo_note ooo_broadcast
    ooo_calendar_enabled? ooo_notify_enabled? meeting_status_enabled? meeting_dnd_enabled?].freeze
  METHODS.each do |method|
    define_method(method) do |*args, **kwargs, &block|
      return super(*args, **kwargs, &block) unless $ws17_recording && $ws17_depth.zero?
      $ws17_depth += 1
      row = { operation: method, now: Time.current.utc.iso8601(6), args: Ws17StatusCapture.encode(args), kwargs: Ws17StatusCapture.encode(kwargs),
        loaded_before: Ws17StatusCapture.snapshot(self), stored_before: Ws17StatusCapture.snapshot(User.find(id)) }
      begin
        value = super(*args, **kwargs, &block)
        row[:result] = %i[update update! update_columns].include?(method) ? value :
          (%i[reload deactivate].include?(method) ? nil : Ws17StatusCapture.encode(value))
        row[:errors] = errors.map { |e| [e.attribute, e.message] } if %i[update update!].include?(method)
        value
      rescue ArgumentError => error
        row[:error] = error.message
        raise
      ensure
        row[:loaded_after] = Ws17StatusCapture.snapshot(self)
        row[:stored_after] = Ws17StatusCapture.snapshot(User.find(id))
        $ws17_steps << row
        $ws17_depth -= 1
      end
    end
  end
end
User.prepend(Ws17StatusCapture)
module Ws17CaptureCacheCreate
  def create!(*args, **kwargs, &block)
    return super unless $ws17_recording && $ws17_depth.zero?
    $ws17_depth += 1
    cache = super
    $ws17_steps << { operation: "cache_create", now: Time.current.utc.iso8601(6),
      attrs: Ws17StatusCapture.encode(cache.attributes.slice("user_id", "busy_intervals", "ooo_intervals", "fetched_at")) }
    cache
  ensure
    $ws17_depth -= 1 if defined?(cache) && cache
  end
end
Calendar::MeetingCache.singleton_class.prepend(Ws17CaptureCacheCreate)

Minitest.seed = 17
$ws17_depth = 0
$ws17_recording = false
rows = []
%w[out_of_office meeting_status].each do |name|
  source = File.read(File.join(__dir__, "pinned/ws17-#{name}_test.rb"))
  titles = source.scan(/test "([^"]+)"/).flatten.to_h { |title| ["test_#{title.gsub(/\s+/, '_')}", title] }
  eval(source.sub('require "test_helper"', ""), TOPLEVEL_BINDING, "/rails/test/models/user/#{name}_test.rb")
  klass = name == "out_of_office" ? User::OutOfOfficeTest : User::MeetingStatusTest
  klass.runnable_methods.sort.each do |method|
    ActiveRecord::Base.transaction do
      User.where(id: ActiveRecord::FixtureSet.identify(:david)).update_all(status: 0, presence_setting: "auto",
        custom_status_emoji: nil, custom_status_text: nil, custom_status_expires_at: nil, dnd_enabled: false,
        dnd_until: nil, quiet_hours_enabled: false, quiet_hours_start_minute: nil, quiet_hours_end_minute: nil,
        time_zone: nil, meeting_status_enabled: false, meeting_dnd_enabled: false, ooo_until: nil,
        ooo_note: nil, ooo_calendar_enabled: false, ooo_notify_enabled: false, ooo_broadcast: nil)
      Calendar::MeetingCache.delete_all
      ActiveSupport::TestCase.new("host").travel_to(Time.utc(2026, 9, 23, 12))
      $ws17_steps = []
      $ws17_recording = true
      result = klass.new(method).run
      $ws17_recording = false
      raise "#{method}: #{result.failures.map(&:message).join('; ')}" unless result.passed?
      rows << { file: "test/models/user/#{name}_test.rb", test: titles.fetch(method), assertions: result.assertions, steps: $ws17_steps }
      raise ActiveRecord::Rollback
    end
  end
end
puts JSON.generate(reference: "d7c7de92", rows:)
