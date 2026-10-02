require 'json'
Current.user=User.find(127326141)
ActionController::Base.allow_forgery_protection=false
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
original=Room.find(699448326)
cases=%w[Rooms::Open Rooms::Closed Rooms::Voice Rooms::Stage Rooms::Board].map do |type|
 original.update_columns(type:)
 room=Room.find(original.id)
 {room_id:room.id,room_name:ApplicationController.helpers.room_display_name(room),room_kind:room.class.model_name.param_key,commands:ApplicationController.helpers.send(:slash_command_names_for,room),html:renderer.render(partial:'rooms/show/composer',locals:{room:,thread:nil,inline:true})}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',cases:)+"\n")
puts "WS8bm2 STI composer Rails oracle: #{cases.size} complete Open/Closed/Voice/Stage/Board Markdown composers"
