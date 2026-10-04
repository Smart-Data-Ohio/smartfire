require 'json'
require 'digest'
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/layout-preferences-source-hashes.json'))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
load File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/post_pin.rb')
class LayoutPreferencesGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user = User.find(127326141)
defaults = {theme: 'system', text_size: 'default', time_zone: nil, time_zone_explicit: false,
  dnd_enabled: false, dnd_until: nil, presence_setting: 'auto', quiet_hours_enabled: false,
  quiet_hours_start_minute: nil, quiet_hours_end_minute: nil, meeting_status_enabled: false,
  meeting_dnd_enabled: false, ooo_until: nil, ooo_calendar_enabled: false, ooo_notify_enabled: false}
now = Time.current
busy = [[(now - 300).iso8601, (now + 3300).iso8601]]
future = [[(now + 3600).iso8601, (now + 7200).iso8601]]
drive = 'https://www.googleapis.com/auth/drive.file'
calendar = 'https://www.googleapis.com/auth/calendar.events'
inputs = [
  ['default', {}],
  ['manual_dnd', {theme: 'light', text_size: 'large', time_zone: 'Pacific Time (US & Canada)', dnd_enabled: true}],
  ['dnd_presence', {presence_setting: 'dnd'}],
  ['expired_dnd', {dnd_enabled: true, dnd_until: (now - 1).iso8601}],
  ['boundary_dnd', {dnd_enabled: true, dnd_until: now.iso8601}],
  ['running_dnd', {dnd_enabled: true, dnd_until: (now + 1).iso8601}],
  ['quiet_hours', {time_zone: 'UTC', quiet_hours_enabled: true, quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420}],
  ['quiet_hours_off', {quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420}],
  ['quiet_hours_incomplete', {quiet_hours_enabled: true, quiet_hours_start_minute: 0}],
  ['quiet_hours_equal', {quiet_hours_enabled: true, quiet_hours_start_minute: 0, quiet_hours_end_minute: 0}],
  ['quiet_hours_zone_default', {quiet_hours_enabled: true, quiet_hours_start_minute: 60, quiet_hours_end_minute: 120}],
  ['meeting_current', {meeting_status_enabled: true, meeting_dnd_enabled: true}, {busy: busy}],
  ['meeting_future', {meeting_status_enabled: true, meeting_dnd_enabled: true}, {busy: future}],
  ['meeting_empty', {meeting_status_enabled: true, meeting_dnd_enabled: true}, {busy: []}],
  ['meeting_missing', {meeting_status_enabled: true, meeting_dnd_enabled: true}],
  ['meeting_status_off', {meeting_dnd_enabled: true}, {busy: busy}],
  ['meeting_quiet_off', {meeting_status_enabled: true}, {busy: busy}],
  ['manual_ooo', {ooo_until: (now + 86400).iso8601}],
  ['expired_ooo', {ooo_until: (now - 1).iso8601}],
  ['boundary_ooo', {ooo_until: now.iso8601}],
  ['calendar_ooo_future', {ooo_calendar_enabled: true}, {ooo: future}],
  ['calendar_ooo_off', {}, {ooo: future}],
  ['ooo_notifications_kept', {ooo_until: (now + 86400).iso8601, ooo_calendar_enabled: true, ooo_notify_enabled: true}, {ooo: future}],
  ['ooo_manual_and_calendar', {ooo_until: (now + 86400).iso8601, ooo_calendar_enabled: true}, {ooo: busy + future}],
  ['cache_order_and_offsets', {meeting_status_enabled: true, meeting_dnd_enabled: true, ooo_calendar_enabled: true},
    {busy: [['2026-03-02T18:00:00.999999+02:00', '2026-03-02T19:00:00+02:00'], *future, *busy, *busy], ooo: future}],
  ['cache_malformed_pairs', {meeting_status_enabled: true, meeting_dnd_enabled: true, ooo_calendar_enabled: true},
    {busy: [nil, 'wrong', [], ['wrong', now.iso8601], [now.iso8601], *busy, [now.iso8601, (now - 1).iso8601, 'ignored']], ooo: [nil, [], *future]}],
  ['drive_missing', {}],
  ['drive_calendar_only', {}, nil, {scopes: calendar}],
  ['drive_current_scope', {}, nil, {scopes: "#{calendar} #{drive}"}],
  ['drive_retired_scope', {}, nil, {scopes: 'https://www.googleapis.com/auth/drive.metadata.readonly'}],
  ['drive_disconnected', {}, nil, {scopes: drive, reason: 'Rejected by Google'}],
  ['drive_ascii_separators', {}, nil, {scopes: "openid\t#{drive}\nemail"}],
  ['drive_wrong_case', {}, nil, {scopes: drive.upcase}],
  ['drive_unicode_separator', {}, nil, {scopes: "openid\u00a0#{drive}"}]
]
cases = inputs.map do |name, attributes, cache, google|
  user.update_columns(defaults.merge(attributes))
  Calendar::MeetingCache.where(user: user).delete_all
  GoogleAccount.where(user: user).delete_all
  Calendar::MeetingCache.create!(user: user, busy_intervals: cache.fetch(:busy, []), ooo_intervals: cache.fetch(:ooo, [])) if cache
  GoogleAccount.create!(user: user, email: 'layout@example.test', scopes: google[:scopes], disconnected_reason: google[:reason]) if google
  user.reload
  Current.reset
  Current.user = user
  Time.use_zone(user.time_zone.presence || 'UTC') do
    renderer = LayoutPreferencesGoldenController.renderer.new(http_host: 'campfire.test', https: false, 'rack.session' => {})
    sound_meta = renderer.render(inline: '<%= notification_sound_meta_tags %>')
    # These are the exact expressions in the application layout, not a substitute formatter.
    drive_meta = renderer.render(inline: '<%= tag.meta name: "google-drive-previews", content: "enabled" if Current.user&.google_account&.drive? %>')
    time_zone_meta = renderer.render(inline: '<%= tag.meta name: "current-user-time-zone", content: current_user_time_zone_meta_content %>')
    {name: name, attributes: defaults.merge(attributes), cache: cache, google: google, sound_meta: sound_meta, drive_meta: drive_meta, time_zone_meta: time_zone_meta}
  end
end
Current.reset
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), now: now.iso8601, cases: cases)
warn "Rails layout preferences oracle: #{cases.size} persisted settings/cache cases; complete sound and Drive meta bytes; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
