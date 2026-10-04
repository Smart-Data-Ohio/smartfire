# Pinned Rails form body, including real ActionView form helpers.
require 'json'
room=Room.new(id:42,name:'Engineering <&>')
room.define_singleton_method(:direct?) { false }
message=Message.new(id:99,room:room,creator:User.new(id:1,name:'David <&>'))
message.define_singleton_method(:plain_text_body) { "The deploy <&> is broken\nTrack the fix" }
account=FizzyConnectedAccount.new(fizzy_user_name:'David <&>',fizzy_account_name:'Smart Data <&>')
account.define_singleton_method(:usable?) { true }
cases=[['connect',nil,nil,nil],['linked',account,nil,nil],['selected',account,'03board2',nil],['thread',account,'03board1',ChannelThread.new(id:5,room:room)]]
frames=cases.map do |name,linked,board,thread|
 html=ApplicationController.renderer.render(template:'rooms/fizzy/message_cards/new',layout:false,assigns:{room:room,message:message,thread:thread,account:linked,boards:[{'id'=>'03board1','name'=>'Engineering <&>'},{'id'=>'03board2','name'=>'Support'}],board_id:board,title:'Deploy <&>',description:"Description <&>\nTwo"})
 {name:name,connected:!!linked,board:board,thread:thread&.id,html:html[html.index('<main')..]}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),frames:frames})+"\n")
puts "WS15e Fizzy message form Rails oracle: #{frames.size} form branches"
