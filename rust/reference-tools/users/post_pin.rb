# Compatibility guard for producers that formerly overlaid #163's status files.
# All ten files now match the pin; use the image's templates, controller and routes.
require 'json'
require 'digest'
work=ENV.fetch('PARITY_WORK')
reference=File.read(File.join(work,'parity/reference.sha')).strip
raise 'status oracle requires the pinned reference image' unless ENV['PARITY_REFERENCE_SHA']==reference
ledger=File.join(work,'test-support/post-pin/source-hashes.json')
JSON.parse(File.read(ledger)).each do |path,hash|
  source=path.start_with?('app/','config/') ? path : "app/views/#{path}"
  raise "status source drift: #{source}" unless Digest::SHA256.file(Rails.root.join(source)).hexdigest==hash
end
