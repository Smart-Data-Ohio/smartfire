require 'json'
# Real Rails partial composition, deterministic form tokens, no HTML normalization.
class WS13RoomCompositionController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
actor=User.find(127326141)
Current.reset;Current.user=actor
renderer=WS13RoomCompositionController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=[]
[[Rooms::Closed,'closed','Call <&> room'],[Rooms::Direct,'direct','Call <&> room'],[Rooms::Voice,'voice','Call <&> room'],[Rooms::Stage,'stage','Call <&> room'],[Rooms::Direct,'direct',nil]].each_with_index do |(klass,kind,room_name),index|
  room=klass.create_for({id:9101+index,name:room_name,creator:actor},users:[actor,User.find(712064548)])
  ["none","legacy","picker"].each do |drive|
    GoogleAccount.where(user:actor).delete_all
    actor.reload
    GoogleAccount.create!(user:actor,email:'fixture@example.test',scopes:'https://www.googleapis.com/auth/drive.file') if drive=='legacy'
    ENV.delete('GOOGLE_CLIENT_ID');ENV.delete('GOOGLE_PICKER_API_KEY');ENV.delete('GOOGLE_CLOUD_PROJECT_NUMBER')
    ENV.update('GOOGLE_CLIENT_ID'=>'public-client','GOOGLE_PICKER_API_KEY'=>'public-picker-key','GOOGLE_CLOUD_PROJECT_NUMBER'=>'12345') if drive=='picker'
    Current.reset;Current.user=actor.reload
    ["composer","member_panel","thread_panel"].each do |partial|
      next if partial!='composer' && drive!='none'
      input={room:{id:room.id,kind:kind,name:room.name,display_name:(room.direct? ? room.direct_display_name(for_user:actor) : room.name)},neutral_name:(room.direct? ? room.direct_display_name(for_user:nil) : room.name),drive:drive}
      html=renderer.render(partial:"rooms/show/#{partial}",locals:{room:room,inline:true})
      cases << {name:"#{kind}_#{partial}_#{drive}",partial:partial,input:input,html:html}
    end
  end
  cases << {name:"#{kind}_poll_builder",partial:'poll_builder',input:{room:{id:room.id,kind:kind,name:room.name,display_name:(room.direct? ? room.direct_display_name(for_user:actor) : room.name)},drive:'none'},html:renderer.render(partial:'polls/builder',locals:{room:room})}
  room.destroy!
end
puts JSON.pretty_generate({reference_pin: ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases})
