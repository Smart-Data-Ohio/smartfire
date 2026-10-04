require 'json'
require 'digest'
require File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/replay_encryption_entropy')
# Fake fixture values deliberately avoid provider token prefixes.
user = User.create!(name: 'Slack Crypto Fixture', email_address: 'slack-crypto@example.invalid')
# The recorded input list is connection/workspace; persistence encrypts workspace first.
workspace, connection = ReplayEncryptionEntropy.with('slack', ivs: ReplayEncryptionEntropy::INPUTS.fetch('slack').reverse) do
  workspace = SlackWorkspace.create!(client_id: 'fixture-client', client_secret: 'fixture-slack-secret')
  connection = SlackConnection.create!(slack_workspace: workspace, user: user, slack_user_id: 'UCRYPTO', access_token: 'fixture-slack-user-token')
  [workspace, connection]
end
result = { 'reference' => ENV.fetch("PARITY_REFERENCE_SHA"),
  'secret_key_base' => ENV.fetch('SECRET_KEY_BASE'),
  'workspace' => { 'plaintext' => workspace.client_secret, 'ciphertext' => workspace.client_secret_before_type_cast },
  'connection' => { 'plaintext' => connection.access_token, 'ciphertext' => connection.access_token_before_type_cast } }
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/crypto.json'), JSON.pretty_generate(result) + "\n")
puts 'Slack encryption vectors: 2 Rails-written encrypted columns generated'
