require 'json'
input = JSON.parse(File.read(Rails.root.join('storage/db/rust-slack-crypto.json')))
workspace = SlackWorkspace.create!(client_id: 'fixture-client', client_secret: 'initial')
user = User.create!(name: 'Rust Slack Crypto Fixture', email_address: 'rust-slack-crypto@example.invalid')
connection = SlackConnection.create!(slack_workspace: workspace, user: user, slack_user_id: 'URUST')
# update_all encrypts string attributes too. Raw SQL is needed for a Rust-produced envelope.
conn = ActiveRecord::Base.connection
conn.execute("UPDATE slack_workspaces SET client_secret = #{conn.quote(input['workspace']['ciphertext'])} WHERE id = #{workspace.id}")
conn.execute("UPDATE slack_connections SET access_token = #{conn.quote(input['connection']['ciphertext'])} WHERE id = #{connection.id}")
raise 'secret mismatch' unless workspace.reload.client_secret == input['workspace']['plaintext']
raise 'token mismatch' unless connection.reload.access_token == input['connection']['plaintext']
raise 'workspace invalid' unless workspace.valid?
raise 'connection invalid' unless connection.valid?
puts 'Slack encryption readback: Rails decrypted and validated 2 Rust-written columns'
