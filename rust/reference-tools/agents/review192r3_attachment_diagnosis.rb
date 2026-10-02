# Preserve the failing request; materialize media outside the posting transaction only
# afterward to obtain Rails' normal successful payload and valid file-state oracle.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
exceptions = []
ActiveStorage::Service::DiskService.prepend(Module.new do
  define_method(:download) do |key, &block|
    super(key, &block)
  rescue ActiveStorage::FileNotFoundError => e
    exceptions << {class:e.class.name, message:e.message, open_transactions:ActiveRecord::Base.connection.open_transactions, backtrace:e.backtrace}
    raise
  end
end)
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
  # Normal Active Storage operations in committed transactions avoid the preview's
  # deferred-upload ordering bug. No Rails request or failure path is patched.
  source.reload.analyze
  preview = source.preview({}).processed.image.blob
  preview.analyze
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  source.preview(format: :webp).processed
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
  puts JSON.pretty_generate(notes:[
    'Maintainer-approved difference: Rails fresh-video preview opens an unuploaded file before commit (FileNotFoundError); Rust does not reproduce this accidental crash and returns 201 with usable preview and WebP files.',
    'Rails failure is captured unchanged first, including rollback and transaction depth. The successful body/state oracle uses ordinary Rails preprocessing in committed transactions, analyzes the preview, then posts the same source. This avoids the defect without patching the request or masking its outcome.',
    'All nonrandom message/thread/blob fields, attachment relationships, file sizes and logical queued-job arguments are checked. Random storage keys and attachment surrogate IDs are omitted.'
  ],cases:[{name:'fresh_video_closed_thread',method:'POST',path:'/rooms/486777696/agents/messages',body:body,rails:rails,rails_state:rollback,exception:exceptions.first,approved:approved,state:state}])
end
