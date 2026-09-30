# Actual owned Rails partials, fixed CSRF values lent to both renderers. No HTML normalization.
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
class Ws17SettingsGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
env = {http_host: "campfire.test", https: false, "rack.session" => {},
  "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new}
user = User.find_by!(email_address: "david@37signals.com")
base = {presence_setting: "auto", custom_status_emoji: nil, custom_status_text: nil,
  custom_status_expires_at: nil, dnd_enabled: false, dnd_until: nil, quiet_hours_enabled: false,
  quiet_hours_start_minute: nil, quiet_hours_end_minute: nil, meeting_status_enabled: false,
  meeting_dnd_enabled: false, ooo_until: nil, ooo_note: nil, ooo_calendar_enabled: false,
  ooo_notify_enabled: false, time_zone: "Pacific Time (US & Canada)"}
states = [
  {name: "default"},
  {name: "configured", google: true},
  {name: "disconnected_on", google: true, attrs: {meeting_status_enabled: true, ooo_calendar_enabled: true}},
  {name: "connected", google: true, connected: true},
  {name: "connected_error", google: true, connected: true, fetch_error: "Fetch <failed> & retry", attrs: {meeting_status_enabled: true, ooo_calendar_enabled: true}},
  {name: "custom_expired", attrs: {presence_setting: "invisible", custom_status_emoji: "🚂", custom_status_text: "On a <train> & slow", custom_status_expires_at: "2026-09-29T12:00:00Z"}},
  {name: "manual_ooo", attrs: {ooo_until: "2026-10-02T06:59:59Z", ooo_note: "Away <&> on a train"}},
  {name: "calendar_ooo", google: true, connected: true, ooo_intervals: [["2026-09-29T12:00:00Z", "2026-10-03T06:59:59Z"]], attrs: {ooo_calendar_enabled: true}},
  {name: "notifications_on", attrs: {dnd_enabled: true, quiet_hours_enabled: true, quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420, meeting_dnd_enabled: true, ooo_notify_enabled: true}, allowances: true, keywords: true},
  {name: "dnd_expired", attrs: {dnd_enabled: true, dnd_until: "2026-09-29T12:00:00Z"}},
  {name: "errors", attrs: {presence_setting: "away", custom_status_emoji: "😀"*9, custom_status_text: "é"*101, ooo_note: "x"*141, quiet_hours_enabled: true}, errors: true},
  {name: "ooo_error", errors: [["ooo_until", "needs a future date and time"], ["base", "Phrase is too long (maximum is 80 characters)"]]},
]
rows = []
travel_to(Time.utc(2026, 9, 30, 12)) do
  states.each do |state|
    GoogleAccount.where(user:).delete_all
    Calendar::MeetingCache.where(user:).delete_all
    user.keyword_alerts.delete_all
    user.dnd_allowed_users.delete_all
    user.reload
    user.assign_attributes(base.merge(state.fetch(:attrs, {})))
    ENV["GOOGLE_CLIENT_ID"] = state[:google] ? "client-id" : nil
    ENV["GOOGLE_CLIENT_SECRET"] = state[:google] ? "client-secret" : nil
    if state[:connected]
      GoogleAccount.create!(user:, email: "calendar<&>@example.test")
    end
    if state[:fetch_error] || state[:ooo_intervals]
      Calendar::MeetingCache.create!(user:, busy_intervals: [], ooo_intervals: state.fetch(:ooo_intervals, []), fetch_error: state[:fetch_error])
    end
    if state[:allowances]
      %w[jason kevin].each { |name| DndAllowedUser.create!(user:, allowed_user: User.find_by!(email_address: "#{name}@37signals.com")) }
    end
    if state[:keywords]
      ["production", "deploy <freeze> & ready"].each { |phrase| KeywordAlert.create!(user:, phrase:) }
    end
    user.association(:google_account).reset
    user.association(:meeting_cache).reset
    if state[:errors] == true
      user.valid?
    else
      Array(state[:errors]).each { |attribute, message| user.errors.add(attribute, message) }
    end
    Current.reset
    Current.user = user
    data = {
      presence_setting: user.presence_setting, custom_status_emoji: user.custom_status_emoji,
      custom_status_text: user.custom_status_text, ooo_note: user.ooo_note,
      dnd_active: user.manual_dnd_active?, quiet_hours_enabled: user.quiet_hours_enabled?,
      quiet_hours_start: user.quiet_hours_start, quiet_hours_end: user.quiet_hours_end,
      meeting_status_enabled: user.meeting_status_enabled?, meeting_dnd_enabled: user.meeting_dnd_enabled?,
      ooo_calendar_enabled: user.ooo_calendar_enabled?, ooo_notify_enabled: user.ooo_notify_enabled?,
      out_of_office: user.out_of_office?, manual_ooo_active: user.manual_ooo_active?, ooo_until_date: user.ooo_until_date,
      google_configured: Google::Client.configured?, calendar_connected: !!(user.google_account&.connected? && user.google_account.calendar?),
      google_email: user.google_account&.email, fetch_error: user.meeting_cache&.fetch_error,
      allowed_people: user.dnd_allowed_people.ordered.map { |person| {id: person.id, name: person.name} },
      keyword_alerts: user.keyword_alerts.order(:phrase).pluck(:phrase).join("\n"),
      errors: user.errors.map { |error| [error.attribute.to_s, error.message] }
    }
    html = %w[status notifications].to_h do |partial|
      [partial, Ws17SettingsGoldenController.renderer.new(env).render(partial: "users/profiles/#{partial}", assigns: {user:})]
    end
    rows << {name: state[:name], data:, html:}
  end
end
puts JSON.generate({reference: "d7c7de92", check_asset: ApplicationController.helpers.asset_path("check.svg"), rows:})
Current.reset
