require 'json'
client = Slack::Client.new(token: 'fixture-slack-user-token', pacing: false)
cases = []
[ {}, { 'ok' => true, 'result' => 1 }, { 'ok' => 0 },
  *%w[invalid_auth token_revoked account_inactive not_authed ratelimited channel_not_found].map { |e| { 'ok' => false, 'error' => e } },
  { 'ok' => false, 'error' => 'missing_scope', 'needed' => 'channels:history', 'provided' => 'channels:read' },
  { 'ok' => false, 'error' => 'missing_scope', 'needed' => ['a', 'b'], 'provided' => [] },
  { 'ok' => false, 'error' => 'missing_scope' },
  *[0, 42, false, true, [], [1, 'x'], {}, {'a'=>1}].map { |error| {'ok'=>false, 'error'=>error} },
  *[0, false, true, [1, false, nil], [['a','b'],'c'], {'a'=>1,'b'=>false}, [{'a'=>1}]].map { |needed| {'ok'=>false, 'error'=>'missing_scope', 'needed'=>needed, 'provided'=>{'raw'=>true}} },
  *['', [], {}].map { |ok| {'ok'=>ok} },
  {'ok'=>false,'error'=>"\u00a0"}, {'ok'=>false,'error'=>"\u0000"}
].each do |payload|
  entry = { 'payload' => payload }
  begin
    entry['expected'] = { 'payload' => client.send(:check_ok!, payload, 'users.list') }
  rescue Slack::Client::Error => error
    entry['expected'] = { 'kind' => error.class.name.demodulize, 'message' => error.message }
    entry['expected']['retry_after'] = error.retry_after if error.respond_to?(:retry_after)
    entry['expected']['needed'] = error.needed if error.respond_to?(:needed)
    entry['expected']['provided'] = error.provided if error.respond_to?(:provided)
  end
  cases << entry
end
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/client.json'), JSON.pretty_generate({ 'reference' => ENV.fetch("PARITY_REFERENCE_SHA"), 'cases' => cases }) + "\n")
puts "Slack client vectors: #{cases.size} Rails error and success mappings generated"
