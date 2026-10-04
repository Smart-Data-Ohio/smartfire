require 'json'
require 'nokogiri'
require "digest"
{
  "app/controllers/rooms_controller.rb" => "53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb",
  "app/models/membership.rb" => "7942db424021486f7046742095b26141031dab9da05e599e22f382d120e4ed9b",
}.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
Current.user=User.find(127326141)
room=Room.find(486777696)
messages=room.root_messages.ordered.to_a
membership=room.memberships.find_by!(user:Current.user)
user=Current.user
session=user.sessions.detect(&:two_factor_verified?) || raise("verified seed session missing")
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed[:session_token]=session.token
client=ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.cookies[:session_token]=request.cookie_jar[:session_token]
headers={"User-Agent"=>"Mozilla/5.0 Chrome/120.0.0.0"}
controller=RoomsController.new
rows=[]
ActiveRecord::Base.transaction do
  [
    ['read_stale_pointer',nil,messages.first.id],
    ['six_unread',messages[-6].created_at,messages[-7].id],
    ['five_unread',messages[-5].created_at,messages[-6].id],
    ['three_unread',messages[-3].created_at,messages[-4].id],
    ['off_page',messages[1].created_at,messages.first.id],
    ['legacy_stamp',messages[-3].created_at,nil],
    ['deleted_pointer',messages[1].created_at,9999999999],
    ['no_boundary',Time.current+10.days,nil]
  ].each do |name,stamp,pointer|
    Current.user=user
    membership.update_columns(unread_at:stamp,last_read_message_id:pointer)
    probe=RoomsController.new
    probe.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new)))
    probe.set_response!(ActionDispatch::Response.new)
    probe.instance_variable_set(:@room,room)
    probe.instance_variable_set(:@messages,room.root_messages.last_page)
    probe.send(:set_unread_divider)
    client.get "/rooms/#{room.id}",headers:headers
    raise "room GET: #{client.response.status}" unless client.response.status==200
    html=client.response.body
    divider_html=html[/<div id="unread-divider".*?<\/div>
/m]
    start=divider_html && html.index(divider_html)
    prefix_whitespace=start && html[...start][/\s*\z/]
    suffix_whitespace=start && html[(start+divider_html.length)..][/\A\s*/]
    first_unread=probe.instance_variable_get(:@unread_divider_message_id)
    unread_dom_id=first_unread && "message_#{Message.find(first_unread).client_message_id}"
    nodes=Nokogiri::HTML(html)
    divider_node=nodes.at_css("#unread-divider")
    following=divider_node&.next_element&.[]("id")
    preceding=divider_node&.previous_element&.[]("id")
    rows << {prefix_whitespace:prefix_whitespace,suffix_whitespace:suffix_whitespace,status:client.response.status,divider_html:divider_html,following:following,preceding:preceding,unread_dom_id:unread_dom_id,
      scroll_flag:html.include?('data-messages-scroll-to-divider-value="true"'),
      jump_button:html.include?('id="jump-to-unread"'),jump_html:html[/<(?:button|a) [^>]*id="jump-to-unread".*?<\/(?:button|a)>/m],name:name,room_id:room.id,user_id:Current.user.id,unread_at:stamp&.iso8601(6),last_read_message_id:pointer,
      divider:probe.instance_variable_get(:@unread_divider_message_id),scroll:probe.instance_variable_get(:@scroll_to_unread_divider),count:probe.instance_variable_get(:@unread_count)||0,jump_url:probe.instance_variable_get(:@jump_to_unread_url)}
  end
  raise ActiveRecord::Rollback
end
Current.reset
puts JSON.pretty_generate(rows)
warn "Rails unread shell oracle: #{rows.size} pointer and divider cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
