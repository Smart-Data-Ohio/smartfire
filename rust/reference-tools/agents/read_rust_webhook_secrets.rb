# A private seeded Rails database reads the actual column ciphertext produced by Rust.
values = JSON.parse(File.read(ENV.fetch("WS11_RUST_SECRETS_PATH")))
bot = User.find(394959859)
conn = ActiveRecord::Base.connection
[[bot.webhook, :signing_secret, values.fetch("webhook")], [bot.agent, :webhook_signing_secret, values.fetch("agent")]].each do |record, column, value|
  conn.execute("UPDATE #{record.class.table_name} SET #{column} = #{conn.quote(value.fetch('ciphertext'))} WHERE id = #{record.id}")
  record.reload
  secret = record.public_send(column)
  raise "Ruby did not decrypt Rust #{column}" unless secret == value.fetch("plaintext") && secret.encoding.to_s == "US-ASCII"
  repeated = record.is_a?(Webhook) ? record.ensure_signing_secret! : record.ensure_webhook_signing_secret!
  raise "Ruby regenerated Rust #{column}" unless repeated == secret
end
puts "WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated"
