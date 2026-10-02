require 'json'
Current.user=User.find(127326141)
ActionController::Base.allow_forgery_protection=false
room=Room.find(699448326)
thread=ChannelThread.create!(room:,creator:Current.user,name:'Thread <&>')
AgentSlashCommand.create!(room:,agent:Agent.first!,name:'deploy',description:'Deploy',takes_arguments:true)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
cases=[]
[[room,nil,false],[Room.find(186869642),nil,false],[room,thread,false],[room,nil,true]].each do |r,t,picker|
 ENV['GOOGLE_CLIENT_ID']=picker ? 'fixture-composer-client' : nil
 ENV['GOOGLE_PICKER_API_KEY']=picker ? 'fixture-composer-key' : nil
 ENV['GOOGLE_CLOUD_PROJECT_NUMBER']=picker ? '1234' : nil
 cases<<{room_id:r.id,room_name:ApplicationController.helpers.room_display_name(r),room_kind:r.class.model_name.param_key,thread:t&.attributes&.slice('id','name'),picker:,commands:ApplicationController.helpers.send(:slash_command_names_for,r),html:renderer.render(partial:'rooms/show/composer',locals:{room:r,thread:t,inline:true})}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',cases:)+"\n")
puts "WS8bm2 composer Rails oracle: #{cases.size} complete Markdown composers including thread and Drive-share controls"
