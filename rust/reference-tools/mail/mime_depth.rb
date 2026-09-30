# Run in the pinned Rails production image; see measure-depth.sh.
require 'json'
require_relative 'review_fixtures'

$stdout.sync = true
abort 'production Rails required' unless Rails.env.production?
abort 'unexpected production VM stack' unless RubyVM::DEFAULT_PARAMS.fetch(:thread_vm_stack_size) == 1_048_576
ENV['INBOUND_EMAIL_DOMAIN'] = 'mail.test'
Rails.logger = ActiveJob::Base.logger = Logger.new(File::NULL)
ActiveRecord::Base.logger = Rails.logger

runtime = {ruby: RUBY_DESCRIPTION, rails_env: Rails.env,
           mail: Gem.loaded_specs.fetch('mail').version.to_s,
           stacks: RubyVM::DEFAULT_PARAMS, native_stack_limit: Process.getrlimit(:STACK),
           vm_stack_env: ENV['RUBY_THREAD_VM_STACK_SIZE'],
           machine_stack_env: ENV['RUBY_THREAD_MACHINE_STACK_SIZE'],
           yjit: RubyVM::YJIT.enabled?}
puts JSON.generate(runtime.merge(type: 'runtime'))

room = Room.alive.without_directs.detect(&:emailable?) || abort('emailable seed room required')
token = room.regenerate_inbound_email_token!
depths = ARGV.empty? ? [1751, 1752] : ARGV.map { |value| Integer(value) }
results = []
depths.each do |depth|
  fixture_raw = Ws10ReviewFixtures.nested_mail(depth, fixed_width: true)
    .sub('nobody@mail.test', 'room-token@mail.test')
  raw = fixture_raw.sub('room-token@mail.test', "room-#{token}@mail.test")
  inbound = ActionMailbox::InboundEmail.create_and_extract_message_id!(raw)
  queue = ActionMailbox::RoutingJob.new(inbound).queue_name
  abort "routing job not queued on #{queue}" unless Resque.size(queue) == 1
  before = room.messages.count
  failures = Resque::Failure.count
  worker = Resque::Worker.new(queue)
  # The actual production adapter and worker perform the queued routing job in its
  # forked child, with the image's unchanged production Ruby stack settings.
  worker.work(0)
  failed = Resque::Failure.count > failures
  failure = failed ? Resque::Failure.all(failures) : nil
  result = {depth: depth, bytes: raw.bytesize, status: inbound.reload.status,
            posts: room.messages.count - before, error: failure && failure['exception'],
            error_location: failure && failure.fetch('backtrace').first,
            source: failed ? nil : room.messages.order(:id).last.markdown_source}
  results << result.merge(raw: fixture_raw)
  puts JSON.generate(result)
  abort "unexpected routing result: #{result.inspect}" unless
    (result[:status] == 'delivered' && result[:posts] == 1 && !failed) ||
    (result[:status] == 'processing' && result[:posts].zero? && result[:error] == 'SystemStackError')
end
profile = {reference_pin: 'fec615be', reference_image: ENV.fetch('WS10_MIME_DEPTH_IMAGE'),
           runtime: runtime, fixtures: results}
last_success = results.select { |sample| sample[:status] == 'delivered' }.map { |sample| sample[:depth] }.max
first_overflow = results.select { |sample| sample[:error] == 'SystemStackError' }.map { |sample| sample[:depth] }.min
if last_success && first_overflow == last_success + 1
  profile.merge!(maximum_mime_depth: last_success, first_overflow_depth: first_overflow)
end
File.write('/out/mime-depth.json', JSON.pretty_generate(profile) + "\n")
