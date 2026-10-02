# Complete authorized root/thread Drive writes, row rollback and actual publisher frames.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
Rails.application.env_config['action_dispatch.show_exceptions']=:all
ActiveJob::Base.queue_adapter=:test
user=User.find(127326141); room=Room.find(486777696)
a='1AbcDefGhIjKlMnOpQrSt'; b='2BcdEfgHiJkLmNoPqRsTu'; c='3CdeFghIjKlMnOpQrStUv'
sessions={}
[user,User.find(149087659)].each do |viewer|
 request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed[:session_token]=viewer.sessions.where.not(two_factor_verified_at:nil).first!.token
 sessions[viewer.id]={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
end
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
frames=[]
ActionCable.server.singleton_class.prepend(Module.new { define_method(:broadcast) { |stream,payload,**options| frames << {stream:stream,payload:payload}; super(stream,payload,**options) } })
cases=[
 ['create_order',nil,nil,{markdown_source:'see these',drive_file_ids:[b,a]}],
 ['create_textless',nil,nil,{markdown_source:'',drive_file_ids:[a]}],
 ['create_dedupe',nil,nil,{markdown_source:'dupes',drive_file_ids:[a,'',"  #{a}  ",b]}],
 ['create_invalid',nil,nil,{markdown_source:'bad',drive_file_ids:[a,'nope']}],
 ['create_scalar',nil,nil,{markdown_source:'bad',drive_file_ids:a}],
 ['create_overflow',nil,nil,{markdown_source:'too many',drive_file_ids:11.times.map{|i|"overflow-file-#{i}"}}],
 ['replace','before',[a,b],{markdown_source:'after',drive_file_ids:[c]}],
 ['untouched','before',[a],{markdown_source:'after'}],
 ['clear','before',[a,b],{markdown_source:'after',drive_file_ids:['']}],
 ['invalid_update','before',[a],{markdown_source:'after',drive_file_ids:['bogus id']}],
 ['scalar_update','before',[a],{markdown_source:'after',drive_file_ids:b}],
 ['clear_textless','',[a],{markdown_source:'',drive_file_ids:['']}],
 ['not_author','before',[a],{markdown_source:'hijacked',drive_file_ids:[b]}],
 ['json_fields','json',[a,b],{markdown_source:'json'}],
 ['json_empty','plain',[],{markdown_source:'plain'}]
]
rows=[]
['root','thread'].each do |mode|
 cases.each do |name,source,drive_ids,input|
  Current.reset; Current.user=user
  thread= mode=='thread' ? ChannelThread.create!(room:room,creator:user,name:"Drive #{name}") : nil
  ThreadMembership.join!(thread,user) if thread
  key="drive-#{mode}-#{name}"
  message= if source
   attrs={markdown_source:source,client_message_id:key}
   if thread
    thread.post_message!(creator:user,attributes:attrs,drive_file_ids:drive_ids)
   else
    value=room.messages.new(attrs.merge(creator:user))
    drive_ids.each { |file_id| value.drive_attachments.build(file_id:file_id) }
    value.save!; value
   end
  end
  base="/rooms/#{room.id}#{thread ? "/threads/#{thread.id}" : ''}/messages"
  method=message ? 'patch' : 'post'
  format=mode=='root' && !message ? 'turbo_stream' : 'json'
  path="#{base}#{message ? "/#{message.id}" : ''}.#{format}"
  input=input.merge(client_message_id:key) unless message
  counts={messages:Message.count,drive:DriveAttachment.count}
  frames.clear
  browser.public_send(method,path,params:{message:input},headers:sessions.fetch(name=='not_author' ? 149087659 : user.id),as: :json)
  saved=Message.find_by(client_message_id:key)
  # Capture primary response before additional read/policy checks.
  primary={status:browser.response.status,body:browser.response.body,location:browser.response.headers['Location'],content_type:browser.response.headers['Content-Type']}
  extra={}
  if name=='not_author'
   other=User.find(149087659); old_role=other.role; other.update_column(:role,0)
   browser.public_send(method,path,params:{message:input},headers:sessions.fetch(other.id),as: :json)
   extra[:nonadmin]={status:browser.response.status,body:browser.response.body}
   other.update_column(:role,old_role)
  end
  if mode=='root' && name=='json_fields'
   Current.reset
   extra[:show]=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(template:'messages/show',layout:false,assigns:{message:saved,room:room})
   account=user.google_account || user.build_google_account(email:'drive-fixture@example.test')
   account.update!(access_token:'drive-fixture-token',scopes:'https://www.googleapis.com/auth/drive.file',disconnected_reason:nil)
   consent=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(template:'messages/show',layout:false,assigns:{message:saved,room:room})
   raise 'Drive consent changed generic attachment markup' unless consent==extra[:show]
   extra[:edit]=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(template:'messages/edit',layout:false,assigns:{message:saved,room:room})
  end
  rows << {extra:extra,mode:mode,name:name,thread_id:thread&.id,setup:source ? {id:message.id,source:source,drive:drive_ids} : nil,client_id:key,viewer:name=='not_author' ? 149087659 : user.id,method:method,path:path,input:input,status:primary[:status],body:primary[:body],
    location:primary[:location],content_type:primary[:content_type],delta:{messages:Message.count-counts[:messages],drive:DriveAttachment.count-counts[:drive]},
    saved:saved && {id:saved.id,source:saved.markdown_source,drive:saved.drive_attachments.map(&:file_id)},frames:frames.dup}
 end
end
Current.reset
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:rows)+"\n")
puts "WS8bm Drive controllers: #{rows.size} actual Rails writes; complete responses, rows and frames; root/thread"
