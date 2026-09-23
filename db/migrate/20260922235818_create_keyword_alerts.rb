class CreateKeywordAlerts < ActiveRecord::Migration[8.1]
  def change
    create_table :keyword_alerts do |t|
      t.references :user, null: false, foreign_key: true
      t.string :phrase, null: false

      t.timestamps
    end

    add_index :keyword_alerts, %i[ user_id phrase ], unique: true
  end
end
