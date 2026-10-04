require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger = Rails.logger
travel_to Time.utc(2026,3,2,16) do
  agent = Agent.find(773018776)
  agent.agent_grants.delete_all; agent.agent_credentials.delete_all
  agent.update_columns(owner_id:127326141,daily_message_cap:nil)
  secret = 'ws11api-agent-http-test'
  agent.agent_credentials.create!(name:'Review',created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:'test')
  ActiveRecord::Base.connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,closed_at,last_activity_at,created_at,updated_at) VALUES(1900700030,'Review',486777696,394959859,394959859,'in_progress','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00')")
  seed = ActiveStorage::Blob.find(9)
  source = ActiveStorage::Blob.create_and_upload!(io:StringIO.new(seed.download),filename:seed.filename.to_s,content_type:seed.content_type)
  body = {thread_id:1900700030,message:{attachment:source.signed_id,client_message_id:'pr192-r3-fresh-video'}}.to_json
  session = ActionDispatch::Integration::Session.new(Rails.application); session.host! 'campfire.test'
  headers = {'Accept'=>'application/json','Content-Type'=>'application/json','Authorization'=>['Bearer',secret].join(' ')}
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  tables = %w[messages channel_threads active_storage_blobs active_storage_attachments active_storage_variant_records]
  before = tables.to_h{|t| [t,ActiveRecord::Base.connection.select_value("SELECT COUNT(*) FROM #{t}").to_i]}
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  rails = {status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  rollback = {row_deltas:tables.to_h{|t| [t,ActiveRecord::Base.connection.select_value("SELECT COUNT(*) FROM #{t}").to_i-before[t]]},thread_closed_at:ChannelThread.find(1900700030).closed_at,source_file_exists:source.service.exist?(source.key),jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map{|j|j[:job].name}}
  queued = ActiveJob::Base.queue_adapter.enqueued_jobs.find { |j| j[:job] == Message::AttachmentProcessingJob }
  raise 'processing job missing after commit' unless queued
  ActiveJob::Base.queue_adapter.enqueued_jobs.delete(queued)
  ActiveJob::Base.execute(queued)
  preview = source.reload.preview_image.blob
  image = preview.variant_records.order(:id).last!.image.blob
  session.post('/rooms/486777696/agents/messages',params:body,headers:headers)
  approved = {status:session.response.status,response:session.response.body,headers:session.response.headers.slice('content-type','retry-after')}
  message = Message.find_by!(client_message_id:'pr192-r3-fresh-video')
  thread = ChannelThread.find(1900700030)
  state = {message:message.attributes,thread:thread.attributes,source_metadata:source.reload.metadata,
    blobs:[source,preview.reload,image.reload].map{|b| {attributes:b.attributes.except('key'),file_exists:b.service.exist?(b.key),file_size:File.size(b.service.send(:path_for,b.key))}},
    variant_count:preview.variant_records.count,
    attachments:ActiveStorage::Attachment.where(blob_id:[source.id,preview.id,image.id]).order(:id).map{|a|a.attributes.except('id')},
    jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map{|j| {class:j[:job].name,args:case j[:job].name
      when 'ChannelThread::PushMessageJob' then {thread_id:j[:args][0]['_aj_globalid'].split('/').last.to_i,message_id:j[:args][1]['_aj_globalid'].split('/').last.to_i}
      when 'ActiveStorage::AnalyzeJob' then {blob_id:j[:args][0]['_aj_globalid'].split('/').last.to_i}
      else j[:args] end}}}
  puts JSON.pretty_generate(notes:['Fresh #226 on the pinned harness: HTTP commits before the real processing job generates and uploads JPEG and WebP. Every row and logical remaining job is captured.'],cases:[{name:'fresh_video_closed_thread',method:'POST',path:'/rooms/486777696/agents/messages',body:body,rails:rails,rails_state:rollback,approved:approved,state:state}])
end
