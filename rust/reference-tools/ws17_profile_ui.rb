# Owned appearance form and subscription content, actual pinned Rails output without masks.
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
class Ws17ProfileUiController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
env = {http_host: "campfire.test", https: false, "rack.session" => {},
 "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new}
user = User.find_by!(email_address: "david@37signals.com")
choices = ApplicationController.helpers.profile_time_zone_choices
names = JSON.parse(File.read("/rails/rust/crates/db/src/tests/ws17_settings_zones.json"))["names"] rescue (ActiveSupport::TimeZone::MAPPING.keys + TZInfo::Timezone.all_identifiers).uniq
choice_zones = ActiveSupport::TimeZone.all.uniq { |zone| zone.tzinfo.identifier }.map do |zone|
  changes = zone.tzinfo.transitions_up_to(Time.utc(2101)).map { |tr| [tr.at.to_i,tr.offset.base_utc_offset] }.uniq { |row| row[0] }
  {name:zone.name,id:zone.tzinfo.identifier,initial:zone.tzinfo.period_for_utc(Time.utc(1800)).base_utc_offset,changes:}
end
mapping = names.to_h { |name| [name, ActiveSupport::TimeZone[name]&.tzinfo&.identifier] }
user.update_columns(time_zone:nil,time_zone_explicit:false,dnd_enabled:false,quiet_hours_enabled:false,meeting_status_enabled:false,ooo_calendar_enabled:false,ooo_until:nil)
layout=File.read(Rails.root.join("app/views/layouts/application.html.erb"))
metadata_source={root:layout.lines.find { |line| line.start_with?("<html ") },color_scheme:layout.lines.find { |line| line.include?('name="color-scheme"') },time_zone:layout.lines.find { |line| line.include?('name: "current-user-time-zone"') }}
rows = []
travel_to(Time.utc(2026,3,2,16)) do
  [ ["system","default",nil], ["dark","smaller","America/New_York"], ["light","larger","Pacific Time (US & Canada)"], ["system","small","UTC"], ["neon","huge","Narnia"], ["system","large",""] ].each do |theme,size,zone|
    user.assign_attributes(theme:,text_size:size,time_zone:zone)
    user.valid?
    Current.user = user
    renderer=Ws17ProfileUiController.renderer.new(env)
    metadata=metadata_source.transform_values { |source| renderer.render(inline:source) }.merge(sounds:ApplicationController.helpers.notification_sound_meta_tags.to_s)
    rows << {data: {theme:,text_size:size,time_zone:zone,time_zone_choices:ApplicationController.helpers.profile_time_zone_choices,errors:user.errors.map { |error| [error.attribute.to_s,error.message] }}, metadata:, html: renderer.render(partial: "users/profiles/appearance", assigns: {user:})}
  end
  user.assign_attributes(dnd_enabled:true,dnd_until:nil,quiet_hours_enabled:true,quiet_hours_start_minute:nil,quiet_hours_end_minute:nil)
  user.valid?
  Current.user=user
  notification_error_sounds=ApplicationController.helpers.notification_sound_meta_tags.to_s
  user.push_subscriptions.delete_all
  Current.user=user.reload
  subscriptions=[]
  [nil,"Mozilla/5.0","Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36", "<unknown & browser>"].each_with_index do |agent,index|
    sub=user.push_subscriptions.new(endpoint: "https://fcm.googleapis.com/fcm/send/#{index}?x=<a>&y=b",p256dh_key:"123",auth_key:"456",user_agent:agent)
    sub.save!(validate:false)
    parsed=UserAgent.parse(agent)
    subscriptions << {id:sub.id,endpoint:sub.endpoint,user_agent:agent,browser:parsed.browser.to_s,version:parsed.version.to_s,platform:parsed.platform.to_s}
  end
  subscription_content=Ws17ProfileUiController.renderer.new(env).render(template:"users/push_subscriptions/index",layout:false,assigns:{push_subscriptions:user.push_subscriptions.to_a})
  assets=%w[check.svg notification-bell-everything.svg minus.svg].to_h { |name| [name,ApplicationController.helpers.asset_path(name)] }
  puts JSON.generate(reference:"d7c7de92",choices:,choice_zones:,mapping:,rows:,notification_error_sounds:,subscriptions:,subscription_content:,assets:)
end
Current.reset
