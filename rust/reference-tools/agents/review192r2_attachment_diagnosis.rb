require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger=Rails.logger
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
  tables=%w[messages channel_threads active_storage_blobs active_storage_attachments active_storage_variant_records]
  before=tables.to_h { |table| [table,ActiveRecord::Base.connection.select_value("SELECT COUNT(*) FROM #{table}").to_i] }
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  first={status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  committed={row_deltas:tables.to_h { |table| [table,ActiveRecord::Base.connection.select_value("SELECT COUNT(*) FROM #{table}").to_i-before[table]] },thread_closed_at:ChannelThread.find(1900700020).closed_at,source_file_exists:ActiveStorage::Blob.find(1).service.exist?(ActiveStorage::Blob.find(1).key),open_transactions:ActiveRecord::Base.connection.open_transactions,jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map { |job| job[:job].name }}
  queued=ActiveJob::Base.queue_adapter.enqueued_jobs.find { |j| j[:job]==Message::AttachmentProcessingJob }
  raise 'processing job missing after commit' unless queued
  ActiveJob::Base.queue_adapter.enqueued_jobs.delete(queued)
  ActiveJob::Base.execute(queued)
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
    jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map{|j|{class:j[:job].name,args:j[:job].name=='ChannelThread::PushMessageJob' ? {thread_id:j[:args].fetch(0).fetch('_aj_globalid').delete_prefix('gid://campfire/ChannelThread/').to_i,message_id:j[:args].fetch(1).fetch('_aj_globalid').delete_prefix('gid://campfire/Message/').to_i} : j[:job].name=='ActiveStorage::AnalyzeJob' ? {blob_id:j[:args][0]['_aj_globalid'].split('/').last.to_i} : j[:args]}}}
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  approved={status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],notes:['Fresh #226 on the pinned harness: the post commits, then its actual processing job generates the thumbnail in committed transactions.'],cases:[{name:'fresh_jpeg_closed_thread',method:'POST',path:'/rooms/486777696/agents/messages',body:body,rails:first,rails_state:committed,approved:approved,state:state}])
end
