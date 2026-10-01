# Run pinned Rails on the per-MIME snapshot exported by the Rust differential test.
require "json"
require "digest"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
vectors = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/attachment_analyzers.json")))
kind = ARGV.fetch(0)
expected = vectors.fetch("cases").find { |entry| entry.fetch("kind") == kind } || raise("unknown kind")
account = Account.first
attachment = account.logo.attachment
blob = attachment.blob
raise "invalid account" unless account.valid?
raise "invalid attachment" unless attachment.valid?
raise "invalid blob" unless blob.valid?
raise "content type mismatch" unless blob.content_type == expected.fetch("identified_content_type")
raise "metadata mismatch" unless blob.metadata == expected.fetch("metadata")
raise "file mismatch" unless Digest::SHA256.hexdigest(blob.download) == expected.fetch("sha256")
raise "analyzer mismatch" unless blob.send(:analyzer_class).name == expected.fetch("analyzer")
raise "signed lookup mismatch" unless ActiveStorage::Blob.find_signed!(blob.signed_id).id == blob.id
jobs = ActiveRecord::Base.connection.select_value("SELECT count(*) FROM background_jobs WHERE job_class = 'ActiveStorage::AnalyzeJob'")
raise "analysis jobs mismatch" unless jobs == expected.fetch("analysis_jobs")
raise "attachment foreign keys" unless ActiveRecord::Base.connection.select_rows("PRAGMA foreign_key_check(active_storage_attachments)").empty?
puts "Rails #{kind} readback: valid account/blob/attachment; bytes, MIME, analyzer, metadata and #{jobs} analysis jobs match"
