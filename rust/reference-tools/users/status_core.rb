# Shared-layout #163 change, with the existing WS6 fixture/renderer and the approved status assets.
load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/post_pin.rb')
output=$stdout
begin
  $stdout=$stderr
  load File.join(ENV.fetch('PARITY_WORK'),'reference-tools/views/core/goldens.rb')
ensure
  $stdout=output
end
puts File.read('/rails/storage/db/goldens.json')
