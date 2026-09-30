load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
# Full, unnormalized sidebar bytes from the pinned app. Tokens are lent to both renderers.
require 'json'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
ActionController::Base.logger=Rails.logger
require "digest"
{
  "app/controllers/users/sidebars_controller.rb" => "a5816989380845cdd31e9abb2b75106629ccbfde1ee1c4edea1ab51a90381341",
  "app/views/users/sidebars/_room_categories.html.erb" => "14d8fbd173e1a19c8b9279ff1f514ab4026f5536d233b6baea897b857bc3af10",
}.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
require 'digest'
raise 'reference drift' unless Digest::SHA256.file(Rails.root.join('app/views/users/sidebars/show.html.erb')).hexdigest == 'ac37bdb0e0fbdefd1a8ba583885d9d88f78955e741fefcc5233efafdfea165ff'
class SidebarGoldenController < Users::SidebarsController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
controller = SidebarGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test', 'rack.url_scheme'=>'http', 'SERVER_PORT'=>'80', 'REQUEST_METHOD'=>'GET', 'rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
h = controller.view_context
rows = []
user_data = ->(u) { {id: u.id, name: u.name, avatar_path: h.fresh_user_avatar_path(u), administrator: u.administrator?} }
row_data = lambda do |m|
  room = m.room
  users = room.direct? ? room.memberships.map(&:user).reject { |u| u.id == m.user_id } : []
  users = [m.user] if room.direct? && users.empty?
  label = room.direct? ? (users.many? || room.name.present? ? h.direct_room_display_name(room, members: users, for_user: m.user) : users.first.name.split(' ')[0]) : nil
  {room_id: room.id, param_key: room.model_name.param_key, room_name: room.name,
    unread: m.unread?, menu: h.room_menu_data(room,m,label: label), viewer_admin: m.user.administrator?,
    icon_name: room.icon_name, members: users.map(&user_data), label: label,
    membership_id: m.id, membership_updated_at: m.updated_at.iso8601(6), updated_at_epoch: room.updated_at.to_fs(:epoch)}
end
render_state = lambda do |name,user|
  Current.reset
  Current.user = user
  controller.show
  assigns = controller.view_assigns
  data = {name: name, user: user_data.call(user), account_name: Current.account.name,
    account_logo: h.fresh_account_logo_path, has_logo: Current.account.logo.attached?,
    can_create_rooms: user.administrator? || !Current.account.settings.restrict_room_creation_to_administrators?}
  %w[favorite_memberships direct_memberships voice_memberships categorized_memberships other_memberships].each { |key| data[key] = assigns[key].map(&row_data) }
  data[:room_categories] = assigns['room_categories'].map { |c| {id: c.id, name: c.name, collapsed: c.collapsed?} }
  data[:direct_placeholder_users] = assigns['direct_placeholder_users'].map(&user_data)
  data[:html] = SidebarGoldenController.renderer.new(http_host: 'campfire.test', https: false, 'rack.session'=>{}).render(template: 'users/sidebars/show', layout: false, assigns: assigns)
  rows << data
end
ActiveRecord::Base.transaction do
  david=User.find(127326141); kevin=User.find(712064548); jz=User.find(773523953)
  render_state.call('seed_admin',david)
  render_state.call('seed_member',kevin)
  render_state.call('seed_outsider',jz)
  first=david.room_categories.create!(name: 'Alpha <&>', position: 1)
  collapsed=david.room_categories.create!(name: 'Zeta', collapsed: true, position: 1)
  david.room_categories.create!(name: 'Empty', position: 0)
  david.memberships.find_by!(room_id:486777696).update!(room_category:collapsed, unread_at: Time.current, involvement:'muted')
  david.memberships.find_by!(room_id:699448329).update!(favorite_position:2,unread_at:Time.current)
  david.memberships.find_by!(room_id:186869642).update!(favorite_position:1,involvement:'muted')
  room=Rooms::Closed.create_for({name:'Channel <&>',creator:david},users:[david,kevin])
  room.memberships.find_by!(user:david).update!(room_category:first)
  shared=Rooms::Open.create_for({name:'Starred',creator:david},users:[david])
  shared.memberships.find_by!(user:david).update!(favorite_position:1)
  david.memberships.joins(:room).where(rooms: {type: ['Rooms::Board','Rooms::Voice','Rooms::Stage']}).order(:id).each.with_index do |m,i|
    m.update!(favorite_position:i+3,unread_at:Time.current,involvement:'muted')
  end
  render_state.call('organized_admin',david)
  Current.account.settings.restrict_room_creation_to_administrators=true
  Current.account.save!
  render_state.call('restricted_member',kevin)
  raise ActiveRecord::Rollback
end
Current.reset
puts JSON.pretty_generate(rows)
warn "Rails sidebar page: #{rows.size} complete frame goldens; sidebar template 2e20b24c, other files d7c7de92"
