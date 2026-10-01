# Header partials and actual per-recipient Turbo callback bytes from our pinned Rails.
require "json"
require "digest"
{
  "app/helpers/rooms_helper.rb" => "63bd8556318ebfcb8cb4048b0a921ab1072b88e078878c95eb2d6dd89477fab7",
  "app/views/rooms/show/_header_identity.html.erb" => "0796fffd979c1223b628e021f05da4853ff300a97d311be5787b3733426d2e83",
  "app/models/rooms/direct.rb" => "4947f936c7607738dd39b6b52f3b5f5adc071bde0d21cf801a05ac801bb6e6f1",
  "app/helpers/users/sidebar_helper.rb" => "c240a5ea5af72a9fc1fb075fc137c062368ac1c87c645272d8be27802c94d32d"
}.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
david, jason, kevin, jz = [127326141,149087659,712064548,773523953].map { |id| User.find(id) }
headers = []
header = lambda do |name, room, viewer|
  Current.reset
  Current.user = david # different actor proves the explicit for_user local wins
  html = ApplicationController.renderer.render(partial: "rooms/show/header_identity", locals: {room: room, for_user: viewer})
  controller = ApplicationController.new
  controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new)))
  controller.set_response!(ActionDispatch::Response.new)
  headers << {name: name, id: room.id, param_key: room.model_name.param_key, direct: room.direct?,
    kind_label: room.stage? ? "Stage channel" : controller.view_context.room_kind_label(room),
    icon_name: room.icon_name, room_name: room.name, viewer_name: viewer.name,
    display_name: controller.view_context.room_display_name(room, for_user: viewer),
    other_names: room.direct? ? room.users.ordered.where.not(id: viewer.id).pluck(:name) : [], html: html}
end
ActiveRecord::Base.transaction do
shared = Room.find(486777696)
header.call("channel", shared, david)
shared.update_columns(icon_name: "fire")
header.call("emoji", shared, david)
shared.update_columns(icon_name: "github")
header.call("brand", shared, david)
shared.update_columns(icon_name: "missing-icon")
header.call("unknown_icon", shared, david)
%w[Voice Stage Board].each do |kind|
  room = Room.create!(type: "Rooms::#{kind}", name: "#{kind} <&>", creator: david)
  header.call(kind.downcase, room, david)
end
pair = Room.find(186869642)
header.call("pair_for_david", pair, david)
header.call("pair_for_jason", pair, jason)
solo = Current.set(user: david) { Rooms::Direct.find_or_create_for([david]) }
header.call("solo", solo, david)
group = Current.set(user: kevin) { Rooms::Direct.find_or_create_for([david,jason,kevin]) }
header.call("group_for_david", group, david)
header.call("group_for_kevin", group, kevin)
extras = 7.times.map { |i| User.create!(name: "Person #{i}", email_address: "directory-person#{i}@example.test") }
large = Current.set(user: david) { Rooms::Direct.find_or_create_for([david,jason,kevin,*extras]) }
header.call("group_ten", large, david)
group.update_columns(name: "Weekend <&>")
header.call("group_named", group, kevin)
Current.reset
raise ActiveRecord::Rollback
end
# Operations use a fresh default database in the Rust regression so IDs/timestamps are fixed.
# Keep a separate room after the header fixture mutations above; record all setup facts.
group = Current.set(user: kevin) { Rooms::Direct.find_or_create_for([david,jason,kevin,jz]) }
frames = []
ActionCable.server.define_singleton_method(:broadcast) do |stream, message, **_options|
  frames << {stream: stream, html: message} if stream.end_with?(":rooms")
end
operations = []
capture = lambda do |name, &block|
  frames.clear
  Current.set(user: kevin, &block)
  operations << {name: name, frames: frames.map(&:dup)}
end
capture.call("rename") { group.rename("Friday <&>", renamed_by: kevin) }
capture.call("clear_name") { group.rename("", renamed_by: kevin) }
capture.call("leave") { group.leave(jz) }
capture.call("add") { group.add_members([jz], added_by: kevin) }
Current.reset
guards = {}
path = "/rooms/directs/#{group.id}"
anonymous = ActionDispatch::Integration::Session.new(Rails.application)
anonymous.host! "campfire.test"
anonymous.patch(path)
guards[:anonymous] = anonymous.response.status
ActiveSupport::IsolatedExecutionState.clear
anonymous.patch("#{path}?bot_key=394959859-BenderToken1")
guards[:bot] = anonymous.response.status
ActiveSupport::IsolatedExecutionState.clear
outsider = User.create!(name: "Directory Outsider", email_address: "directory-outsider@example.test")
session = outsider.sessions.create!(two_factor_verified_at: Time.current)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = {value: session.token, httponly: true, same_site: :lax}
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.cookies["session_token"] = request.cookie_jar[:session_token]
client.get("/users/me/profile")
ActiveSupport::IsolatedExecutionState.clear
token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')&.[]("content")
raise "guard requires valid CSRF token: #{client.response.status}" unless token
client.patch(path, params: {room: {name: "Stolen"}}, headers: {"X-CSRF-Token" => token})
guards[:outsider] = client.response.status
guards[:outsider_location] = client.response.headers["Location"]
ActiveSupport::IsolatedExecutionState.clear
labels = [["One\u00a0Two"], ["Line\vBreak"], ["One\u00a0Two", "Line\vBreak"], ["\tOne Two", "Three\fFour"]].map do |names|
  members = names.map { |name| User.new(name: name) }
  label = members.many? ? Rooms::Direct.new.direct_display_name(members: members) : members.first.name.split(" ")[0]
  {names: names, label: label}
end
puts JSON.pretty_generate(reference: "d7c7de92", headers: headers,
  setup: {group_id: group.id}, operations: operations, guards: guards, labels: labels)
warn "Rails room directory: #{headers.size} header goldens, #{operations.sum { |op| op[:frames].size }} recipient frames; reference d7c7de92"
