# Rails rollback compatibility: run over the snapshot exported by the Rust readback test.
require "json"
require "base64"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
vectors = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/attachment_assignments.json")))
icon = WorkspaceIcon.find_by!(name: "attach_readback")
blob = icon.image.blob
raise "metadata mismatch" unless blob.metadata == vectors.fetch("analysis").fetch("metadata")
raise "file mismatch" unless blob.download == Base64.strict_decode64(vectors.fetch("png_base64"))
attachments = blob.attachments.includes(:record).order(:id).to_a
raise "not four shared attachments" unless attachments.size == 4
attachments.each do |attachment|
  raise "invalid attachment: #{attachment.errors.full_messages}" unless attachment.valid?
  record = attachment.record
  raise "invalid #{record.class}: #{record.errors.full_messages}" unless record.valid?
  raise "signed id lookup failed" unless ActiveStorage::Blob.find_signed!(attachment.signed_id).id == blob.id
end
raise "not the account logo" unless Account.first.logo.blob.id == blob.id
raise "not the user avatar" unless User.find(127326141).avatar.blob.id == blob.id
message = attachments.find { |attachment| attachment.record_type == "Message" }.record
raise "not the message attachment" unless message.attachment.blob.id == blob.id
raise "not valid PNG icon" unless icon.valid?
# The default seed has one unrelated orphaned fixture message. Check the tables written here.
%w[active_storage_blobs active_storage_attachments workspace_icons].each do |table|
  raise "foreign keys in #{table}" unless ActiveRecord::Base.connection.select_rows("PRAGMA foreign_key_check(#{table})").empty?
end
puts "Rails readback: 4 valid records; 4 valid attachments; shared signed blob; PNG bytes and analyzed metadata match; attachment foreign keys clean"
