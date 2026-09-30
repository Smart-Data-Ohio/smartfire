# Read Rust-written signed-assignment rows with our pinned Rails app.
require "json"
require "digest"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
vectors = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/attachment_filenames.json")))
kind = ARGV.fetch(0)
expected = vectors.fetch("assignments").find { |entry| entry.fetch("kind") == kind } || raise("unknown kind")
account = Account.first
attachment = account.logo.attachment
blob = attachment.blob
raise "invalid account/blob/attachment" unless [account, blob, attachment].all?(&:valid?)
raise "raw filename mismatch" unless blob[:filename] == expected.fetch("stored_filename")
raise "sanitized filename mismatch" unless blob.filename.to_s == expected.fetch("sanitized")
raise "MIME mismatch" unless blob.content_type == expected.fetch("identified_content_type")
raise "metadata mismatch" unless blob.metadata == expected.fetch("metadata")
raise "bytes mismatch" unless Digest::SHA256.hexdigest(blob.download) == expected.fetch("sha256")
raise "analyzer mismatch" unless blob.send(:analyzer_class).name == expected.fetch("analyzer")
raise "signed lookup mismatch" unless ActiveStorage::Blob.find_signed!(blob.signed_id).id == blob.id
copied = ActiveStorage::Blob.order(:id).last
raise "invalid copied blob" unless copied.id != blob.id && copied.valid?
raise "copied filename mismatch" unless copied[:filename] == expected.fetch("sanitized")
raise "copied MIME/metadata mismatch" unless copied.content_type == blob.content_type && copied.metadata == blob.metadata
raise "copied bytes mismatch" unless Digest::SHA256.hexdigest(copied.download) == expected.fetch("sha256")
jobs = ActiveRecord::Base.connection.select_value("SELECT count(*) FROM background_jobs WHERE job_class = 'ActiveStorage::AnalyzeJob'")
raise "analysis jobs mismatch" unless jobs == expected.fetch("analysis_jobs")
raise "attachment foreign keys" unless ActiveRecord::Base.connection.select_rows("PRAGMA foreign_key_check(active_storage_attachments)").empty?
puts "Rails #{kind} readback: valid account/blob/attachment and copied blob; raw and sanitized names, bytes, MIME, analyzer, metadata and #{jobs} analysis jobs match"
