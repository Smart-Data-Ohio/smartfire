# The production reference image omits model test sources. Supply the byte-exact
# pin archived by ci-seed prepare, then run WS12's unchanged hash-guarded producer.
require "fileutils"
path = "test/models/channel_thread_agent_assignment_test.rb"
destination = Rails.root.join(path)
unless destination.file?
  source = File.join(ENV.fetch("PARITY_WORK"), "parity/.ci/reference", path)
  FileUtils.mkdir_p(destination.dirname)
  FileUtils.cp(source, destination)
end
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/work/agent_named.rb")
