require "json"
load File.join(__dir__, "providers.rb")
base=JSON.parse(File.read(ARGV.fetch(0)))
user=User.find(127326141); Current.user=user; room=Room.find(699448326)
message=Message.find(base.fetch("cases").find{|c| c["label"]=="open"}.fetch("message_id"))
ActionController::Base.allow_forgery_protection=false; Rails.application.config.hosts.clear
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={"Cookie"=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! "campfire.test"
frames=[]; ActionCable.server.define_singleton_method(:broadcast){|stream,html,**| frames << {stream:,html:}}
steps=[]
["https://github.com/provider-owner/repo-1/pull/2 https://page.example.test/post#new-fragment", "https://github.com/provider-owner/repo-9/pull/10", "no more provider references"].each do |text|
 frames.clear; input={message:{markdown_source:text}}
 browser.patch("/rooms/#{room.id}/messages/#{message.id}",params:input,headers:headers.dup,as: :json)
 steps << {input:,status:browser.response.status,frames:frames.select{|f|f[:stream]=="#{room.to_gid_param}:messages"}.dup,
  github:Github::PullRequestReference.where(message:).order(:id).pluck(:github_pull_request_id),
  embeds:LinkEmbedReference.where(message:).order(:position).pluck(:url)}
end
private_case=base["cases"].find{|c| c["label"]=="private"}
private_id=base["rows"]["github_pull_requests"].find{|r|r["repo"]=="repo-9"}["id"]
public_case=base["cases"].find{|c|c["label"]=="merged"}
public_id=base["rows"]["github_pull_requests"].find{|r|r["repo"]=="repo-2"}["id"]
reads=[]
[[private_id,private_case["message_id"]],[public_id,public_case["message_id"]],[private_id,public_case["message_id"]]].each do |pr,m|
 path="/rooms/#{room.id}/github/pull_requests/#{pr}/card?message_id=#{m}"
 browser.get(path,headers:headers.dup)
 reads << {path:,status:browser.response.status,body:browser.response.body}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:"d7c7de92",rows:base["rows"],room_id:room.id,message_id:message.id,steps:,reads:)+"\n")
puts "WS8bm2 provider callbacks: #{steps.size} changed-URL HTTP edits, #{steps.sum{|s|s[:frames].size}} socket frames; #{reads.size} actual provider card reads"
