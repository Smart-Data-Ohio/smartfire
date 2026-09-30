# Actual Rails setters/validations and legal zone names at d7c7de92.
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
user = User.find_by!(email_address: "david@37signals.com")
user.assign_attributes(presence_setting:"auto",theme:"system",text_size:"default",time_zone:nil,
  custom_status_emoji:nil,custom_status_text:nil,ooo_until:nil,ooo_note:nil,
  quiet_hours_enabled:false,quiet_hours_start_minute:nil,quiet_hours_end_minute:nil)
clock = Time.utc(2026, 9, 30, 12)
rows = []
%w[UTC America/New_York Pacific\ Time\ (US\ &\ Canada) Australia/Lord_Howe Pacific/Apia].each do |zone|
  %w[2026-09-30T12:00:00Z 2026-03-08T06:30:00Z 2026-11-01T05:30:00Z 2026-09-28T23:00:00Z].each do |instant|
    travel_to(Time.iso8601(instant)) do
      u=user.dup
      u.time_zone=zone
      User::StatusSettings::CUSTOM_STATUS_EXPIRIES.each do |preset|
        u.custom_status_expires_in=preset
        rows << { kind: "custom", zone:, now: instant, preset:, value: u.custom_status_expires_at&.iso8601(6) }
      end
      User::StatusSettings::OOO_PRESETS.each do |preset|
        custom="2026-11-01T01:30"
        rows << { kind: "ooo", zone:, now: instant, preset:, custom:, value: u.ooo_preset_until(preset,custom)&.iso8601(6) }
      end
    end
  end
end
clocks = [nil,"","late","22:00","7:09","00:00","23:59","24:00","22:60","22:00:99","22:00:00"," 22:00 ","22:00\n","١٢:٠٠"].map do |value|
  u=user.dup
  u.quiet_hours_start=value
  { input: value, minute: u.quiet_hours_start_minute, display: u.quiet_hours_start }
end
validations=[]
travel_to(clock) do
  [ {presence_setting:"away"},{theme:"neon"},{text_size:"huge"},{time_zone:"Narnia"},{time_zone:"america/new_york"},{time_zone:"UTC"},{time_zone:" "},
    {custom_status_emoji:"😀"*9},{custom_status_text:"é"*101},{ooo_note:"x"*141},{ooo_until:clock.iso8601},
    {quiet_hours_start_minute:-1},{quiet_hours_end_minute:1440},{quiet_hours_enabled:true},
    {quiet_hours_enabled:true,quiet_hours_start:"22:00:00",quiet_hours_end:"7:09"} ].each do |attrs|
    u=user.dup
    u.assign_attributes(attrs)
    u.valid?
    validations << { attrs:, errors: u.errors.map { |e|[e.attribute.to_s,e.message] } }
  end
end
user.save!
first = User.find(user.id)
second = User.find(user.id)
first.update!(dnd_enabled: true, presence_setting: "invisible")
second.update!(custom_status_text: "Concurrent edit")
dirty_write = second.reload.attributes.slice("dnd_enabled", "presence_setting", "custom_status_text")
puts JSON.generate({reference:"d7c7de92",now:clock.iso8601,rows:,clocks:,validations:,dirty_write:,
  zones:{ names:(ActiveSupport::TimeZone::MAPPING.keys+TZInfo::Timezone.all_identifiers).uniq.sort }})
