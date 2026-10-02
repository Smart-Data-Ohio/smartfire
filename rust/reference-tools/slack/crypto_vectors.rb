require 'json'
require 'digest'
# Fake fixture values deliberately avoid provider token prefixes.
workspace = SlackWorkspace.create!(client_id: 'fixture-client', client_secret: 'fixture-slack-secret')
user = User.create!(name: 'Slack Crypto Fixture', email_address: 'slack-crypto@example.invalid')
connection = SlackConnection.create!(slack_workspace: workspace, user: user, slack_user_id: 'UCRYPTO', access_token: 'fixture-slack-user-token')
result = { 'reference' => 'd7c7de9264c63015be398001d7a1094e7695a6db',
  'secret_key_base' => ENV.fetch('SECRET_KEY_BASE'),
  'workspace' => { 'plaintext' => workspace.client_secret, 'ciphertext' => workspace.client_secret_before_type_cast },
  'connection' => { 'plaintext' => connection.access_token, 'ciphertext' => connection.access_token_before_type_cast } }
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/crypto.json'), JSON.pretty_generate(result) + "\n")
puts 'Slack encryption vectors: 2 Rails-written encrypted columns generated'
