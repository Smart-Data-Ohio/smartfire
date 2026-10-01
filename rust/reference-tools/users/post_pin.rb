# Exact post-pin files are read-only inputs mounted under /work, not changes to /rails.
require 'json'
require 'digest'
WS8BR2_POST_PIN=File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post-pin')
JSON.parse(File.read(File.join(WS8BR2_POST_PIN,'source-hashes.json'))).each {|path,hash|raise "post-pin source drift: #{path}" unless Digest::SHA256.file(File.join(WS8BR2_POST_PIN,path)).hexdigest==hash}
ApplicationController.prepend_view_path(WS8BR2_POST_PIN)
load File.join(WS8BR2_POST_PIN,'app/controllers/users/statuses_controller.rb')
reloader=Rails.application.routes_reloader
pinned_route=Rails.root.join('config/routes.rb').to_s
raise 'pinned route path missing' unless reloader.paths.include?(pinned_route)
reloader.paths.map! {|path|path==pinned_route ? File.join(WS8BR2_POST_PIN,'config/routes.rb') : path}
reloader.reload!
