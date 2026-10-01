require 'json'
require 'digest'
raise 'inbound section drift' unless Digest::SHA256.file(Rails.root.join('app/views/rooms/inbound_email_addresses/_section.html.erb')).hexdigest=='2c487640aa9c8f9c6e79de89cfc4c83c014734f32a9a9a9b94772e4f7b01ea65'
class InboundSectionGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=InboundSectionGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
rows=[]
[[false,nil,127326141,201306877],[true,nil,127326141,201306877],[true,'a'*32,127326141,201306877],[true,'a<&>',127326141,201306877],[true,'a'*32,712064548,201306877],[true,'a'*32,127326141,186869642]].each_with_index do |(enabled,token,user_id,id),index|
  ENV['INBOUND_EMAIL_DOMAIN']=enabled ? 'mail.campfire.test' : nil
  Current.reset;Current.user=User.find(user_id)
  room=Room.find(id);room.inbound_email_token=token
  html=renderer.render(partial:'rooms/inbound_email_addresses/section',layout:false,locals:{room:room})
  rows << {name:"state_#{index}",id:id,can_administer:Current.user.can_administer?(room),emailable:room.emailable?,enabled:enabled,address:room.inbound_email_address,html:html}
end
Current.reset
puts JSON.pretty_generate({reference:'d7c7de92',states:rows})
warn "Rails inbound-email section: #{rows.size} complete partial goldens; reference d7c7de92"
