# Record relay ingress and the exact production Procfile worker path, not a caller-local worker.
require 'json'
require 'timeout'
require 'shellwords'
require_relative 'review_fixtures'

$stdout.sync = true
abort 'production Rails required' unless Rails.env.production?
abort 'unexpected production VM stack' unless RubyVM::DEFAULT_PARAMS.fetch(:thread_vm_stack_size) == 1_048_576
ENV['INBOUND_EMAIL_DOMAIN'] = 'mail.test'
ENV['RAILS_INBOUND_EMAIL_PASSWORD'] = 'ws10-fixture-password'
Rails.logger = ActiveJob::Base.logger = Logger.new(File::NULL)
ActiveRecord::Base.logger = Rails.logger
command = File.readlines('/rails/Procfile').find { |line| line.start_with?('workers: ') }.split(': ', 2).last.strip
abort 'unexpected worker command' unless command == 'FORK_PER_JOB=false INTERVAL=0.1 bundle exec resque-pool'
runtime = {ruby: RUBY_DESCRIPTION, rails_env: Rails.env, mail: Gem.loaded_specs.fetch('mail').version.to_s,
           stacks: RubyVM::DEFAULT_PARAMS, native_stack_limit: Process.getrlimit(:STACK),
           vm_stack_env: ENV['RUBY_THREAD_VM_STACK_SIZE'], machine_stack_env: ENV['RUBY_THREAD_MACHINE_STACK_SIZE'],
           yjit: RubyVM::YJIT.enabled?, worker_command: command, ingress: 'POST /rails/action_mailbox/relay/inbound_emails'}
puts JSON.generate(runtime.merge(type: 'runtime'))
room = Room.alive.without_directs.detect(&:emailable?) || abort('emailable seed room required')
token = room.regenerate_inbound_email_token!
results = []
if ARGV.include?('--resume')
  prior = JSON.parse(File.read('/out/mime-depth.json'))
  abort 'resume profile differs' unless prior.fetch('runtime') == JSON.parse(JSON.generate(runtime)) &&
    prior.fetch('reference_pin') == 'fec615be' && prior.fetch('reference_image') == ENV.fetch('WS10_MIME_DEPTH_IMAGE')
  results = prior.fetch('fixtures').map { |sample| sample.transform_keys(&:to_sym) }
end
measure = lambda do |depth|
  existing = results.find { |sample| sample[:depth] == depth }
  next existing if existing
  fixture_raw = Ws10ReviewFixtures.nested_mail(depth, fixed_width: true).sub('nobody@mail.test', 'room-token@mail.test')
  raw = fixture_raw.sub('room-token@mail.test', "room-#{token}@mail.test")
  before = room.messages.count
  failures = Resque::Failure.count
  request = Rack::MockRequest.env_for('https://example.com/rails/action_mailbox/relay/inbound_emails',
    method: 'POST', input: raw, 'CONTENT_TYPE' => 'message/rfc822',
    'HTTP_AUTHORIZATION' => ActionController::HttpAuthentication::Basic.encode_credentials('actionmailbox', ENV.fetch('RAILS_INBOUND_EMAIL_PASSWORD')))
  status, _, response = Rails.application.call(request)
  response.close if response.respond_to?(:close)
  abort "relay ingress returned #{status}" unless status == 204
  inbound = ActionMailbox::InboundEmail.order(:id).last
  queue = ActionMailbox::RoutingJob.new(inbound).queue_name
  abort "routing job not queued on #{queue}" unless Resque.size(queue) == 1
  pid = Process.spawn('env', *Shellwords.split(command), out: '/out/pool.log', err: [:child, :out])
  begin
    # A timeout aborts the measurement; only actual recorded Rails exceptions classify failure.
    Timeout.timeout(900) do
      sleep 0.1 until inbound.reload.delivered? || inbound.failed? || Resque::Failure.count > failures
    end
    failure = Resque::Failure.count > failures ? Resque::Failure.all(failures) : nil
    result = {depth: depth, bytes: raw.bytesize, ingress_status: status, status: inbound.reload.status,
              posts: room.messages.count - before, error: failure && failure['exception'],
              error_location: failure && failure.fetch('backtrace').first,
              source: inbound.delivered? ? room.messages.order(:id).last.markdown_source : nil}
    abort "unexpected routing result: #{result.inspect}" unless
      (result[:status] == 'delivered' && result[:posts] == 1 && failure.nil?) ||
      (result[:status] == 'processing' && result[:posts].zero? && result[:error] == 'SystemStackError')
    results << result.merge(raw: fixture_raw)
    puts JSON.generate(result)
    # Keep every completed measurement even if a later probe is interrupted.
    File.write('/out/mime-depth.json', JSON.pretty_generate(reference_pin: 'fec615be',
      reference_image: ENV.fetch('WS10_MIME_DEPTH_IMAGE'), runtime: runtime, fixtures: results) + "\n")
    result
  ensure
    Process.kill('TERM', pid)
    Process.wait(pid)
  end
end

if ARGV == ['--calibrate'] || ARGV == ['--calibrate', '--resume']
  low, high, step = 0, 1751, 16
  abort 'old cutoff unexpectedly posts' unless measure.call(high)[:error] == 'SystemStackError'
  loop do
    low = [high - step, 0].max
    break if measure.call(low)[:status] == 'delivered'
    abort 'plain mail failed' if low.zero?
    high = low
    step *= 2
  end
  while high - low > 1
    middle = (low + high) / 2
    if measure.call(middle)[:status] == 'delivered'
      low = middle
    else
      high = middle
    end
  end
  cutoff = low * 9 / 10 # At least 10% below the last production-path success, per decisions.md.
  [cutoff, cutoff + 1].each { |depth| measure.call(depth) }
  profile = {reference_pin: 'fec615be', reference_image: ENV.fetch('WS10_MIME_DEPTH_IMAGE'), runtime: runtime,
             production_maximum_mime_depth: low, first_overflow_depth: high, margin_percent: 10,
             maximum_mime_depth: cutoff, first_bounce_depth: cutoff + 1,
             fixtures: results.select { |sample| [cutoff, cutoff + 1, low, high, 1751].include?(sample[:depth]) },
             probes: results.map { |sample| sample.reject { |key, _| key == :raw } }}
  File.write('/out/mime-depth.json', JSON.pretty_generate(profile) + "\n")
  puts JSON.generate(profile.reject { |key, _| [:runtime, :fixtures, :probes].include?(key) })
else
  abort 'pass --calibrate or explicit depths' if ARGV.empty?
  ARGV.each { |depth| measure.call(Integer(depth)) }
end
