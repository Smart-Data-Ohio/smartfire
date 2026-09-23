class CreateLinkEmbeds < ActiveRecord::Migration[8.2]
  def change
    create_table :link_embeds do |t|
      t.string :normalized_url, null: false
      t.string :site_name
      t.string :title
      t.text :description
      t.string :image_url
      t.datetime :fetched_at
      t.datetime :fetch_requested_at
      t.string :fetch_error
      t.datetime :expires_at

      t.timestamps
    end
    add_index :link_embeds, :normalized_url, unique: true

    create_table :link_embed_references do |t|
      t.references :message, null: false, foreign_key: true
      t.references :link_embed, null: false, foreign_key: true
      t.string :url
      t.integer :position, null: false, default: 0

      t.timestamps
    end
    add_index :link_embed_references, %i[ message_id link_embed_id ], unique: true,
      name: "index_link_embed_references_on_message_and_embed"
  end
end
