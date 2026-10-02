# Member panel and the shared selection bar from pinned Smartfire Rails.
require_relative "../bots_ui/setup"

result = { "panels" => [], "selection_bars" => {}, "toggle" => nil }
%w[pets designers david_and_kevin].each do |label|
  room = Room.find(ActiveRecord::FixtureSet.identify(label))
  %w[david kevin].each do |viewer|
    result["panels"] << {
      name: "#{label}_#{viewer}", room_id: room.id, viewer: viewer,
      html: render_with(user: user("#{viewer}@37signals.com"), partial: "rooms/show/member_panel", locals: { room: })
    }
  end
end
[true, false].each do |exit_button|
  result["selection_bars"][exit_button.to_s] = render_with(user: user("david@37signals.com"), partial: "shared/multi_select_bar", locals: { exit_button: })
end
# Extract the control directly from our Rails nav, preserving its indentation.
nav = Rails.root.join("app/views/rooms/show/_nav.html.erb").read
button = nav[/    <button type="button"\n.*?class="btn room-header__action member-panel-toggle".*?<\/button>/m]
raise "member panel toggle missing" unless button
result["toggle"] = render_with(user: user("david@37signals.com"), inline: button + "\n")
File.write("/rails/storage/db/member-panel.json", JSON.pretty_generate(result) + "\n")
puts "member panel Rails fixtures: #{result['panels'].size} panels; #{result['selection_bars'].size} selection bars"
