# Run the original pinned harness against the real Rails HTTP response.
require 'fileutils'
Rails.application.config.hosts.clear
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! 'campfire.test'
client.get '/service-worker.js'
raise "service worker HTTP status #{client.response.status}" unless client.response.status == 200
root = File.join(ENV.fetch('PARITY_WORK'), '.scratch/review244-r3/rails-worker-source')
FileUtils.mkdir_p(File.join(root, 'app/views/pwa'))
FileUtils.mkdir_p(File.join(root, 'test/scripts'))
File.write(File.join(root, 'app/views/pwa/service_worker.js'), client.response.body)
FileUtils.cp(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/service_worker_original_harness.mjs'), File.join(root, 'test/scripts/service_worker_harness.mjs'))
puts "Rails service worker source receipt: HTTP #{client.response.status}; real response staged for the original Node harness"
