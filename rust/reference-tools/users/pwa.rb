require "json"
require "digest"
%w[app/controllers/pwa_controller.rb app/views/pwa/manifest.json.erb app/views/pwa/service_worker.js public/offline.html].each do |file|
  hash = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/source-hashes.json"))).fetch(file)
  raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
responses = %w[/webmanifest.json /service-worker.js /offline.html].map do |path|
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! "campfire.test"
  session.get path
  ActiveSupport::IsolatedExecutionState.clear
  { path: path, status: session.response.status, content_type: session.response.headers["Content-Type"], body: session.response.body }
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), responses: responses)
warn "Rails PWA oracle: #{responses.length} endpoint bodies; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
