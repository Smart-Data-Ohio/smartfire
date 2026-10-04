require "json"
ActiveJob::Base.queue_adapter = :test
class GoogleProfileHtmlController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
renderer = GoogleProfileHtmlController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
AccountFacts=Struct.new(:email,:connected,:calendar,:drive) do
  alias connected? connected
  alias calendar? calendar
  alias drive? drive
end
UserFacts=Struct.new(:google_account,:google_identity)
email="david<&>@smartdata.net"
cases=[
 ["unconfigured",false,nil], ["new",true,nil], ["calendar",true,AccountFacts.new(email,true,true,false)],
 ["drive",true,AccountFacts.new(email,true,true,true)], ["missing-calendar",true,AccountFacts.new(email,true,false,true)],
 ["disconnected",true,AccountFacts.new(email,false,true,false)], ["disconnected-drive",true,AccountFacts.new(email,false,true,true)]
]
calendar=cases.map do |name,configured,account|
 ENV["GOOGLE_CLIENT_ID"]=configured ? "test-client-id" : ""
 ENV["GOOGLE_CLIENT_SECRET"]=configured ? "FAKE-google-client-secret" : ""
 {name:,configured:,account:account&.to_h,html:renderer.render(partial:"users/profiles/google_calendar",locals:{user:UserFacts.new(account,nil)})}
end
signin=[["unconfigured",false,nil],["not-linked",true,nil],["linked",true,Struct.new(:email).new(email)]].map do |name,configured,identity|
 ENV["GOOGLE_CLIENT_ID"]="test-client-id"
 ENV["GOOGLE_CLIENT_SECRET"]="FAKE-google-client-secret"
 ENV["GOOGLE_SIGN_IN_DOMAINS"]=configured ? "smartdata.net" : ""
 {name:,configured:,email:identity&.email,html:renderer.render(partial:"users/profiles/google_sign_in",locals:{user:UserFacts.new(nil,identity)})}
end
message=Message.order(:id).first!
drive=[[],["1AbcDefGhIjKlMnOpQrSt","2BcDefGhIjKlMnOpQrStU"]].map do |ids|
 message.drive_attachments.destroy_all
 ids.each { |id| message.drive_attachments.create!(file_id:id) }
 message.reload
 {id:message.id,client_message_id:message.client_message_id,urls:message.drive_attachments.map(&:url),html:renderer.render(partial:"messages/drive_attachments",locals:{message:})}
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),calendar:,signin:,drive:})
