# Complete Rails room pages, with deterministic inputs before rendering; never normalize HTML.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
class WS8brFullPageController < RoomsController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
  def last_room_visited
    Current.user.rooms.original
  end
  def protect_against_forgery?; true; end
  def content_security_policy_nonce; 'NONCE'; end
end
renderer=WS8brFullPageController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{},'HTTP_USER_AGENT'=>'Mozilla','action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'})
rows=[]
[[654632876,127326141],[186869642,127326141],[699448329,712064548],[201306877,773523953]].each do |room_id,user_id|
 Current.reset;Current.user=User.find(user_id);room=Room.find(room_id)
 Time.zone=Current.user.time_zone.presence || 'UTC'
 controller=WS8brFullPageController.new
 controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new)))
 controller.set_response!(ActionDispatch::Response.new)
 controller.instance_variable_set(:@room,room)
 controller.show
 assigns=controller.view_assigns
 messages=assigns['messages'].to_a
 html=renderer.render(template:'rooms/show',layout:'layouts/application',assigns:assigns)
 rows << {room_id:room_id,user_id:user_id,root_ids:messages.map(&:id),html:html}
end
Current.reset
puts JSON.pretty_generate(reference:'d7c7de92',layout_reference:'2e20b24c',rows:rows)
warn "Rails full room pages: #{rows.size} complete native-page goldens; no HTML normalization"
