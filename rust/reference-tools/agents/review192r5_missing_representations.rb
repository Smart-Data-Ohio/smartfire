# Handled missing-file responses in pinned Rails, with real committed processing.
require "active_support/testing/time_helpers"
require "base64"
require "digest"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
kind=ARGV.fetch(0)
raise "unknown case" unless %w[video_preview video_variant jpeg_variant].include?(kind)
def response(session,path)
  session.get(path,headers:{"Accept"=>"*/*"})
  reply=session.response
  {path:path,status:reply.status,body_base64:Base64.strict_encode64(reply.body.b),body_bytes:reply.body.bytesize,
   headers:%w[Content-Type Cache-Control Content-Disposition Content-Length Location Last-Modified ETag Accept-Ranges Content-Transfer-Encoding].to_h{|header|[header,reply.headers[header]]}}
end
travel_to Time.utc(2026,3,2,16) do
  original=ActiveStorage::Blob.find(kind=="jpeg_variant" ? 1 : 9)
  source=ActiveStorage::Blob.create_and_upload!(key:"ws11apir5source",io:StringIO.new(original.download),filename:original.filename.to_s,content_type:original.content_type)
  changes=kind=="jpeg_variant" ? {resize_to_limit:[1200,800]} : {format: :webp}
  key=source.representation(changes).variation.key
  image=ActiveStorage::Blob.find_by!(key:source.representation(key).processed.key)
  preview=source.preview_image.blob if source.preview_image.attached?
  [ [preview,"ws11apir5preview"], [image,"ws11apir5variant"] ].each do |blob,fixed_key|
    next unless blob
    blob.service.upload(fixed_key,StringIO.new(blob.download),checksum:blob.checksum)
    blob.service.delete(blob.key)
    blob.update_columns(key:fixed_key)
    File.utime(Time.current.to_time,Time.current.to_time,blob.service.send(:path_for,fixed_key))
  end
  source.reload
  routes=Rails.application.routes.url_helpers
  redirect=routes.rails_blob_representation_path(signed_blob_id:source.signed_id,variation_key:key,filename:source.filename.to_s)
  proxy=routes.rails_blob_representation_proxy_path(signed_blob_id:source.signed_id,variation_key:key,filename:source.filename.to_s)
  session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
  baseline_redirect=response(session,redirect)
  baseline_disk=response(session,URI.parse(baseline_redirect[:headers]["Location"]).request_uri)
  baseline_proxy=response(session,proxy)
  raise "positive control failed" unless [baseline_redirect[:status],baseline_disk[:status],baseline_proxy[:status]]==[302,200,200]
  missing=kind=="video_preview" ? preview : image
  missing.service.delete(missing.key)
  before=%w[active_storage_blobs active_storage_attachments active_storage_variant_records].to_h{|table|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} ORDER BY id").to_a]}
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  reply=response(session,redirect)
  disk=response(session,URI.parse(reply[:headers]["Location"]).request_uri)
  proxied=response(session,proxy)
  repeated=response(session,redirect)
  after=before.keys.to_h{|table|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} ORDER BY id").to_a]}
  raise "representation mutated stored rows" unless before==after
  raise "representation regenerated file" if missing.service.exist?(missing.key)
  raise "representation enqueued jobs" unless ApplicationJob.queue_adapter.enqueued_jobs.empty?
  puts JSON.pretty_generate({name:kind,source_id:source.id,preview_id:preview&.id,variant_id:image.id,
    baseline_redirect:baseline_redirect,baseline_disk:baseline_disk,baseline_proxy:baseline_proxy,
    redirect:reply,disk:disk,proxy:proxied,repeat_redirect:repeated,
    rows_unchanged:true,missing_file_regenerated:false,jobs:[]})
end
