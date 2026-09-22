class AddWebhookSigningSecrets < ActiveRecord::Migration[8.2]
  def change
    add_column :agents, :webhook_signing_secret, :string
    add_column :webhooks, :signing_secret, :string
  end
end
