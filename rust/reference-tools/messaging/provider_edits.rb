require 'json'
load File.join(__dir__,'providers.rb')
base=JSON.parse(File.read(ARGV.fetch(0)))
user=User.find(127326141);Current.user=user
room=Room.find(699448326)
empty=room.messages.create!(creator:user,markdown_source:'Before edit',client_message_id:'provider-edit-empty')
base['rows']['messages']+=ActiveRecord::Base.connection.select_all("SELECT * FROM messages WHERE id=#{empty.id}").to_a
base['rows']['action_text_rich_texts']+=ActiveRecord::Base.connection.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{empty.id}").to_a
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames<<{stream:,html:} }
cases=base['cases'].select{|c|%w[open private generic_title linkedin_player].include?(c['label'])}.map{|c|c.merge('input'=>{'message'=>{'markdown_source'=>'provider reference'}})}
cases<<{'label'=>'empty_edited','message_id'=>empty.id,'input'=>{'message'=>{'markdown_source'=>'Edited <&> body'}}}
steps=cases.map do |c|
 frames.clear
 browser.patch("/rooms/#{room.id}/messages/#{c['message_id']}",params:c['input'],headers:headers.dup,as: :json)
 c.slice('label','message_id','input').merge(status:browser.response.status,frames:frames.select{|f|f[:stream]=="#{room.to_gid_param}:messages"}.dup)
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:base['rows'],room_id:room.id,steps:)+"\n")
puts "WS8bm2 provider edit Rails oracle: #{steps.size} HTTP edits; #{steps.sum{|s|s[:frames].size}} exact replacement frames"
