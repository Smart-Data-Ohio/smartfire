require 'json'
require 'digest'
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/status-panels-source-hashes.json'))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
load File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/post_pin.rb')
class EffectiveOooGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ENV['GOOGLE_CLIENT_ID'] = 'parity-client'
ENV['GOOGLE_CLIENT_SECRET'] = 'parity-secret'
user = User.find(127326141)
now = Time.current
active = [[(now - 3600).iso8601, (now + 172800).iso8601]]
inputs = [
  ['calendar_current', {ooo_calendar_enabled: true}, active],
  ['calendar_future', {ooo_calendar_enabled: true}, [[(now + 3600).iso8601, (now + 7200).iso8601]]],
  ['calendar_start_boundary', {ooo_calendar_enabled: true}, [[now.iso8601, (now + 3600).iso8601]]],
  ['calendar_end_boundary', {ooo_calendar_enabled: true}, [[(now - 3600).iso8601, now.iso8601]]],
  ['calendar_disabled', {}, active],
  ['calendar_later_end', {ooo_calendar_enabled: true, ooo_until: (now + 86400).iso8601, ooo_note: 'Back <&> soon'}, active],
  ['manual_later_end', {ooo_calendar_enabled: true, ooo_until: (now + 259200).iso8601}, active],
  ['expired_manual_with_calendar', {ooo_calendar_enabled: true, ooo_until: (now - 1).iso8601}, active],
  ['member_zone_return_date', {ooo_calendar_enabled: true, time_zone: 'Pacific Time (US & Canada)'}, [[(now - 3600).iso8601, '2026-03-03T01:00:00Z']]],
  ['overlapping_calendar_latest_end', {ooo_calendar_enabled: true}, [*active, [(now - 120).iso8601, (now + 259200).iso8601]]],
  ['fractional_calendar_end', {ooo_calendar_enabled: true}, [[(now - 1).iso8601, (now + 0.5).iso8601(6)]]],
  ['calendar_note_without_manual', {ooo_calendar_enabled: true, ooo_note: 'Old manual note <&>'}, active]
]
cases = inputs.map do |name, attrs, intervals|
  attributes = {presence_setting: 'auto', custom_status_emoji: '🚀', custom_status_text: 'Shipping Rust',
    ooo_note: nil, ooo_until: nil, meeting_status_enabled: false, ooo_calendar_enabled: false, time_zone: nil}.merge(attrs)
  user.update_columns(attributes)
  Calendar::MeetingCache.where(user: user).delete_all
  GoogleAccount.where(user: user).delete_all
  GoogleAccount.create!(user: user, email: 'fixture<&>@example.test')
  Calendar::MeetingCache.create!(user: user, fetched_at: now, ooo_intervals: intervals)
  user.reload
  Current.reset
  Current.user = user
  Time.use_zone(user.time_zone.presence || 'UTC') do
    renderer = EffectiveOooGoldenController.renderer.new(http_host: 'campfire.test', https: false, 'rack.session' => {})
    {name: name, attributes: attributes, intervals: intervals,
      manual_ooo: user.manual_ooo_active?, return_date: user.out_of_office? ? user.ooo_until_date : nil,
      html: renderer.render(partial: 'users/profiles/status', assigns: {user: user})}
  end
end
Current.reset
puts JSON.pretty_generate(reference: 'd7c7de92', status_reference: '2e20b24c', now: now.iso8601, cases: cases)
warn "Rails effective OOO profile oracle: #{cases.size} persisted Calendar/manual/boundary/zone cases; complete status panel bytes; status template 2e20b24c, model files d7c7de92"
