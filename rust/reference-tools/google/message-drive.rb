# Real merged room/thread HTTP producers. No Google call is permitted or needed to attach ids.
require 'json'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Net::HTTP.define_singleton_method(:start) { |*|raise 'Drive message fixture attempted Google HTTP' }
A='1AbcDefGhIjKlMnOpQrSt';B='2BcdEfgHiJkLmNoPqRsTu';C='3CdeFghIjKlMnOpQrStUv'
rows=[];frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,payload,**_|frames << {stream:,payload:} }
%w[root thread].each do |scope|
  %w[create_order create_textless create_dedupe create_invalid create_scalar create_overflow update_replace update_omit update_clear update_invalid update_scalar update_overflow update_textless_clear update_noncreator json_shape json_empty].each do |scenario|
    ActiveRecord::Base.transaction(requires_new:true) do
      thread_scope=scope=='thread';create=scenario.start_with?('create_')
      room=Room.find(thread_scope || scenario=='update_noncreator' ? 654632876 : 486777696)
      creator=User.find(thread_scope ? 773523953 : scenario=='update_noncreator' ? 149087659 : 127326141)
      actor=User.find(scenario=='update_noncreator' ? (thread_scope ? 712064548 : 773523953) : creator.id)
      GoogleAccount.where(user:actor).delete_all
      ActiveRecord::Base.connection.execute("INSERT INTO sqlite_sequence(name,seq) SELECT 'channel_threads',9700000000 WHERE NOT EXISTS(SELECT 1 FROM sqlite_sequence WHERE name='channel_threads')")
      ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=9700000000 WHERE name='channel_threads'")
      thread=ChannelThread.create!(room:,creator:,name:'Drive request fixture') if thread_scope
      ThreadMembership.join!(thread,creator) if thread
      ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=9600000000 WHERE name='messages'")
      client_id="drive-fixture-#{scope}-#{scenario}"
      original=scenario=='update_textless_clear' ? '' : 'before'
      unless create
        message=Message.new(room:,thread:,creator:,markdown_source:original,client_message_id:client_id)
        [A,B].each { |id|message.drive_attachments.build(file_id:id) } unless scenario=='json_empty'
        message.save!
      end
      params={markdown_source:create ? 'see these' : 'after',client_message_id:client_id}
      case scenario
      when 'create_order' then params[:drive_file_ids]=[B,A]
      when 'create_textless' then params.merge!(markdown_source:'',drive_file_ids:[A])
      when 'create_dedupe' then params[:drive_file_ids]=[A,'',"  #{A}  ",B]
      when 'create_invalid','update_invalid' then params[:drive_file_ids]=[A,'bad id']
      when 'create_scalar','update_scalar' then params[:drive_file_ids]=B
      when 'create_overflow','update_overflow' then params[:drive_file_ids]=11.times.map { |i|"overflow-file-#{i}" }
      when 'update_replace','update_noncreator' then params[:drive_file_ids]=[C]
      when 'update_clear' then params[:drive_file_ids]=['']
      when 'update_textless_clear' then params.merge!(markdown_source:'',drive_file_ids:[''])
      end
      path=thread_scope ? "/rooms/#{room.id}/threads/#{thread.id}/messages" : "/rooms/#{room.id}/messages"
      path+="/#{message.id}" unless create
      format=thread_scope || scenario.start_with?('json_') ? 'json' : create ? 'turbo_stream' : nil
      path+=".#{format}" if format
      client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
      session=Session.create!(user:actor,user_agent:'Drive message fixture',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
      request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
      jar=ActionDispatch::Cookies::CookieJar.build(request,{});jar.signed[:session_token]={value:session.token};client.cookies['session_token']=jar[:session_token]
      before=[Message.count,DriveAttachment.count];frames.clear
      client.public_send(create ? :post : thread_scope ? :patch : :put,path,params:{message:params},as: :json,headers:{'Accept'=>format=='turbo_stream' ? 'text/vnd.turbo-stream.html' : format=='json' ? 'application/json' : 'text/html'})
      stored=Message.find_by(client_message_id:client_id)
      attachment_frames=frames.select { |f|f[:payload].to_s.include?("target=\"drive_attachments_message_#{client_id}\"") }
      json_drive=format=='json' && client.response.status<400 ? client.response.parsed_body['drive_attachments'] : nil
      rows << {scope:,scenario:,room_id:room.id,creator_id:creator.id,actor_id:actor.id,thread_id:thread&.id,message_id:message&.id,client_id:,original:,params:,path:,result:{status:client.response.status,location:client.response.location,delta:[Message.count-before[0],DriveAttachment.count-before[1]],files:stored&.drive_attachments&.map(&:file_id),source:stored&.markdown_source,json_drive:,frames:attachment_frames}}
      raise ActiveRecord::Rollback
    end
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:})
