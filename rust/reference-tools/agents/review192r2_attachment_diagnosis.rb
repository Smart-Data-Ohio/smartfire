# The caller authorized a status difference for a post-commit Rails defect, not for state.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
Rails.logger=ActiveSupport::Logger.new($stderr)
exceptions=[]
ActionDispatch::ExceptionWrapper.prepend(Module.new do
  define_method(:initialize) do |*args|
    e=args.last
    exceptions << {class:e.class.name,message:e.message,backtrace:e.backtrace}
    super(*args)
  end
end)
ActiveJob::Base.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776)
  agent.agent_grants.delete_all;agent.agent_credentials.delete_all
  secret='ws11api-agent-http-test'
  agent.agent_credentials.create!(name:'Review',created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:'test')
  agent.update_columns(owner_id:127326141,daily_message_cap:nil)
  ActiveRecord::Base.connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,closed_at,last_activity_at,created_at,updated_at) VALUES(1900700020,'Review',486777696,394959859,394959859,'in_progress','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00')")
  session=ActionDispatch::Integration::Session.new(Rails.application);session.host! 'campfire.test'
  body={thread_id:1900700020,message:{attachment:ActiveStorage::Blob.find(1).signed_id,client_message_id:'pr192-r2-fresh-jpeg'}}.to_json
  headers={'Accept'=>'application/json','Content-Type'=>'application/json','Authorization'=>['Bearer',secret].join(' ')}
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  first={status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  message=Message.find_by!(client_message_id:'pr192-r2-fresh-jpeg')
  thread=ChannelThread.find(1900700020)
  variant=ActiveStorage::VariantRecord.where(blob_id:1).order(:id).last!
  image=variant.image.blob
  path=image.service.send(:path_for,image.key)
  state={message:message.attributes,thread:thread.attributes,
    source_metadata:ActiveStorage::Blob.find(1).metadata,
    variant:variant.attributes.except('id'),
    image:image.attributes.except('key'),
    attachments:ActiveStorage::Attachment.where(record_type:'Message',record_id:message.id).or(ActiveStorage::Attachment.where(record_type:'ActiveStorage::VariantRecord',record_id:variant.id)).order(:id).map{|a|a.attributes.except('id','record_id')},
    variant_file_exists:File.exist?(path),variant_file_size:File.exist?(path) ? File.size(path) : nil,
    variant_count:ActiveStorage::VariantRecord.where(blob_id:1).count,
    attachment_count:ActiveStorage::Attachment.where(record_type:'Message',record_id:message.id).or(ActiveStorage::Attachment.where(record_type:'ActiveStorage::VariantRecord',record_id:variant.id)).count,
    jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map{|j|{class:j[:job].name,args:j[:job].name=='ChannelThread::PushMessageJob' ? {thread_id:1900700020,message_id:message.id} : j[:args]}}}
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  approved={status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  puts JSON.pretty_generate({notes:['Approved by the maintainer: keep Rust 201 for this post-commit Rails IOError, while asserting the same persisted state and jobs.','The successful response is the Rails idempotent replay of the committed first request. Random storage keys are omitted; all nonrandom blob attributes and file presence are asserted. Jobs use logical named arguments rather than runtime-specific serialization envelopes.'],cases:[{name:'fresh_jpeg_closed_thread',method:'POST',path:'/rooms/486777696/agents/messages',body:body,rails:first,approved:approved,state:state,exception:exceptions.first}]})
end
