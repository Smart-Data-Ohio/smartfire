class CreatePolls < ActiveRecord::Migration[8.2]
  def change
    create_table :polls do |t|
      t.references :message, null: false, foreign_key: true, index: { unique: true }
      t.boolean :multiple, null: false, default: false
      t.boolean :anonymous, null: false, default: false
      t.datetime :closes_at
      t.datetime :closed_at

      t.timestamps
    end

    create_table :poll_options do |t|
      t.references :poll, null: false, foreign_key: true
      t.string :label, null: false
      t.integer :position, null: false, default: 0

      t.timestamps
    end
    add_index :poll_options, %i[ poll_id position ]

    create_table :poll_votes do |t|
      t.references :poll, null: false, foreign_key: true
      t.references :poll_option, null: false, foreign_key: true
      t.references :user, null: false, foreign_key: true

      t.timestamps
    end
    add_index :poll_votes, %i[ poll_option_id user_id ], unique: true
    add_index :poll_votes, %i[ poll_id user_id ]
  end
end
