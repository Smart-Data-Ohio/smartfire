require 'json'
require 'base64'
require 'digest'
require 'tempfile'
require_relative 'oracle-database'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = true
ActiveJob::Base.queue_adapter = :test
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
scenario = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
user = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.detect(&:two_factor_verified?).token
cookie = request.cookie_jar[:session_token]
inputs = JSON.parse(File.read('/work/vectors/attachment_filenames.json')).fetch('assignments')
png = JSON.parse(File.read('/work/vectors/attachment_assignments.json')).fetch('png_base64')
inputs << {'kind' => 'real_image', 'filename' => 'icon.png', 'content_type' => 'image/png', 'data_base64' => png}
rows = []
%w[avatar bot logo].each do |kind|
  inputs.each do |input|
    %w[signed multipart].each do |mode|
      scenario.call do
        browser = ActionDispatch::Integration::Session.new(Rails.application)
        browser.host! 'campfire.test'
        browser.cookies['session_token'] = cookie
        browser.get '/account/edit'
        token = Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
        record, name, path, params = case kind
        when 'avatar' then [User.find(user.id), 'avatar', '/users/me/profile', {user: {name: 'Attachment parity'}}]
        when 'bot' then [User.find(394959859), 'avatar', '/account/bots/394959859', {user: {name: 'Attachment parity'}}]
        else [Account.first, 'logo', '/account', {account: {name: 'Attachment parity'}}]
        end
        bytes = Base64.strict_decode64(input.fetch('data_base64'))
        temporary = Tempfile.new('avatar-parity', '/rails/storage')
        temporary.binmode; temporary.write(bytes); temporary.rewind
        source = if mode == 'signed'
          blob = ActiveStorage::Blob.create_before_direct_upload!(filename: input.fetch('filename'), content_type: input.fetch('content_type'), byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes))
          blob.upload_without_unfurling(StringIO.new(bytes)); blob.signed_id
        else
          Rack::Test::UploadedFile.new(temporary.path, input.fetch('content_type'), true, original_filename: input.fetch('filename'))
        end
        params.values.first[name.to_sym] = source
        ActiveJob::Base.queue_adapter.enqueued_jobs.clear
        browser.patch path, params: params, headers: {'X-CSRF-Token' => token}
        blob = record.reload.public_send(name).blob
        rows << {kind:, mode:, input:, path:, response: {status: browser.response.status, body: browser.response.body, content_type: browser.response.headers['Content-Type'], cache_control: browser.response.headers['Cache-Control'], location: browser.response.headers['Location']},
          record_name: record.name, filename: blob.filename.to_s, raw_filename: blob[:filename], content_type: blob.content_type, metadata: blob.metadata,
          analysis_jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job] == ActiveStorage::AnalyzeJob }, data_base64: Base64.strict_encode64(blob.download)}
        temporary.close!
      end
    end
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows:) + "\n")
puts "WS8bm avatar-logo oracle: #{rows.size} authenticated CSRF requests; avatar/bot/logo; signed/multipart; sanitized filenames and inline/durable analyzers"
