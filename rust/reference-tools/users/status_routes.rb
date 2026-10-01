load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools',ARGV.first=='campfire' ? 'campfire/routes.rb' : 'routes/routes.rb')
