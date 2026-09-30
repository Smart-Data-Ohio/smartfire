# Actual pinned Rails row bytes and locals; run over a private default parity seed.
require "json"
require "digest"
raise "reference drift" unless Digest::SHA256.file(Rails.root.join("app/helpers/users/sidebar_helper.rb")).hexdigest == "c240a5ea5af72a9fc1fb075fc137c062368ac1c87c645272d8be27802c94d32d"
raise "reference drift" unless Digest::SHA256.file(Rails.root.join("app/models/rooms/direct.rb")).hexdigest == "4947f936c7607738dd39b6b52f3b5f5adc071bde0d21cf801a05ac801bb6e6f1"
raise "reference drift" unless Digest::SHA256.file(Rails.root.join("app/views/users/sidebars/rooms/_direct.html.erb")).hexdigest == "a88b1b6b2f6a29d15d6e99cdb21646ee3378127204d00298ade9fb13f0c1e8d9"
raise "reference drift" unless Digest::SHA256.file(Rails.root.join("app/views/users/sidebars/rooms/_shared.html.erb")).hexdigest == "c2983b2fe7a620cf8ec6128b9438d40cc823f2bcc109b8bb38c9e46269ae40c5"
rows = []
controller = ApplicationController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.url_scheme" => "http", "SERVER_PORT" => "80", "REQUEST_METHOD" => "GET", "rack.input" => StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
helpers = controller.view_context
render_row = lambda do |name, room, membership, actor|
  Current.reset
  Current.user = actor
  direct = room.direct?
  members = direct ? room.users.to_a.reject { |u| u.id == membership.user_id } : []
  members = [membership.user] if direct && members.empty?
  label = if direct
    members.many? || room.name.present? ? helpers.direct_room_display_name(room, members: members, for_user: membership.user) : members.first.name.split(" ")[0]
  end
  data = {name: name, direct: direct, room_id: room.id, param_key: room.model_name.param_key,
    room_name: room.name, unread: membership&.unread? || false, menu: helpers.room_menu_data(room, membership, label: label),
    viewer_admin: membership&.user&.administrator? || false, icon_name: room.icon_name,
    members: members.map { |u| {id: u.id, name: u.name, avatar_path: helpers.fresh_user_avatar_path(u)} }, label: label,
    membership_id: membership&.id, membership_updated_at: membership&.updated_at&.iso8601(6), updated_at_epoch: room.updated_at.to_fs(:epoch)}
  locals = direct ? {membership: membership, members: members, participants: [], huddleable: false} : {room: room, membership: membership, unread: data[:unread]}
  data[:html] = ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: "users/sidebars/rooms/#{direct ? 'direct' : 'shared'}", locals: locals)
  rows << data
end
david = User.find(127326141)
kevin = User.find(712064548)
jason = User.find(149087659)
jz = User.find(773523953)
shared = Room.find(486777696)
member = shared.memberships.find_by!(user: david)
render_row.call("shared_admin_read", shared, member, david)
member.update!(unread_at: Time.current, involvement: "muted", favorite_position: 3)
category = david.room_categories.create!(name: "Squad")
member.update!(room_category: category)
render_row.call("shared_muted_unread_favorite_category", shared, member, david)
render_row.call("shared_broadcast_defaults", shared, nil, david)
shared.update!(icon_name: "fire")
render_row.call("shared_emoji", shared, member, david)
shared.update!(icon_name: "github")
render_row.call("shared_brand", shared, member, david)
plain = Rooms::Closed.create_for({name: "Owned <&> channel", creator: kevin}, users: [kevin, david, jz])
render_row.call("shared_member_creator", plain, plain.memberships.find_by!(user: kevin), david)
render_row.call("shared_member_not_creator", plain, plain.memberships.find_by!(user: jz), david)
pair = Room.find(186869642)
render_row.call("direct_pair", pair, pair.memberships.find_by!(user: david), david)
other_pair = Room.find(699448325)
render_row.call("direct_pair_member_not_creator", other_pair, other_pair.memberships.find_by!(user: kevin), david)
solo = Current.set(user: david) { Rooms::Direct.find_or_create_for([david]) }
render_row.call("direct_solo", solo, solo.memberships.find_by!(user: david), david)
group = Current.set(user: kevin) { Rooms::Direct.find_or_create_for([david, jason, kevin]) }
render_row.call("group_nonadmin_creator", group, group.memberships.find_by!(user: kevin), david)
render_row.call("group_admin_recipient", group, group.memberships.find_by!(user: david), kevin)
group.update!(name: "Weekend <&>")
group.memberships.find_by!(user: david).update!(unread_at: Time.current, involvement: "muted", favorite_position: 0)
render_row.call("group_named_muted_unread", group, group.memberships.find_by!(user: david), david)
extras = 7.times.map { |i| User.create!(name: "Person #{i}", email_address: "row-person#{i}@example.test") }
large = Current.set(user: david) { Rooms::Direct.find_or_create_for([david,jason,kevin,*extras]) }
render_row.call("group_ten", large, large.memberships.find_by!(user: david), david)
Current.reset
puts JSON.pretty_generate(rows)
warn "Rails sidebar rows: #{rows.size} byte goldens; reference d7c7de92"
