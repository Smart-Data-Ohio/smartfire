require 'json'
require 'digest'
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/sign-in-google-source-hashes.json'))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
load File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/post_pin.rb')
class SignInGoogleGoldenController < SessionsController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
credentials = {'GOOGLE_CLIENT_ID' => 'parity-client', 'GOOGLE_CLIENT_SECRET' => 'parity-secret'}
inputs = [
  ['unconfigured', {}],
  ['client_only', {'GOOGLE_CLIENT_ID' => 'parity-client', 'GOOGLE_SIGN_IN_DOMAINS' => 'example.test'}],
  ['missing_domains', credentials],
  ['invalid_domains', credentials.merge('GOOGLE_SIGN_IN_DOMAINS' => 'invalid,-bad.test,https://example.test')],
  ['one_domain', credentials.merge('GOOGLE_SIGN_IN_DOMAINS' => 'example.test')],
  ['two_domains', credentials.merge('GOOGLE_SIGN_IN_DOMAINS' => ' Example.TEST , second.test, example.test')],
  ['three_domains', credentials.merge('GOOGLE_SIGN_IN_DOMAINS' => 'example.test,second.test,sub.third.test')],
  ['blank_client', credentials.merge('GOOGLE_CLIENT_ID' => "\t ", 'GOOGLE_SIGN_IN_DOMAINS' => 'example.test')]
]
Current.reset
cases = inputs.map do |name, environment|
  %w[GOOGLE_CLIENT_ID GOOGLE_CLIENT_SECRET GOOGLE_SIGN_IN_DOMAINS].each { |key| ENV[key] = environment[key] }
  renderer = SignInGoogleGoldenController.renderer.new(http_host: 'campfire.test', https: false, 'rack.session' => {})
  {name: name, environment: environment, configured: Google::SignIn.configured?, domains: Google::SignIn.allowed_domains,
    body: renderer.render(template: 'sessions/new', layout: false)}
end
puts JSON.pretty_generate(reference: 'd7c7de92', cases: cases)
warn "Rails Google sign-in display oracle: #{cases.size} complete sign-in bodies and configuration cases; reference d7c7de92"
