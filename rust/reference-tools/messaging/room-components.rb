# Exact native integration slots (no HTML normalization and no owner partial substitutions).
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
raise 'room shell source drift' unless Digest::SHA256.file(Rails.root.join('app/views/rooms/show.html.erb')).hexdigest=='2218e078bc846e09238104d226f4f6674ef32aa4363002d438675fe48eb7e777'
class NativeRoomComponentGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=NativeRoomComponentGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
rows=[]
[[654632876,127326141],[186869642,127326141],[699448329,712064548]].each do |room_id,user_id|
 Current.reset;Current.user=User.find(user_id);room=Room.find(room_id)
 # These are the real root collection and existing shell controller's unread inputs.
 messages=room.root_messages.ordered.last_page.to_a
 parts=JSON.parse(renderer.render(inline:'<% body = render template: "rooms/show" %><%= {body: body, footer: content_for(:footer)}.to_json.html_safe %>',layout:false,assigns:{room:room,messages:messages,ooo_notice_members:[]}))
 rows << {room_id:room_id,user_id:user_id,root_ids:messages.map(&:id),message_list:parts['body'].match(/<div id="#{Regexp.escape(ActionView::RecordIdentifier.dom_id(room,:messages))}"[^>]*>(.*?)<\/div>\n\s*<turbo-cable-stream-source/m)[1],composer:parts['footer'],pending_template:renderer.render(partial:'messages/template')}
end
Current.reset
File.write(ARGV.fetch(0), JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:rows) + "\n")
puts "WS8bm room components: #{rows.size} complete list/composer/template goldens; reference d7c7de92"
