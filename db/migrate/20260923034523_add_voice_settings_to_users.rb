class AddVoiceSettingsToUsers < ActiveRecord::Migration[8.2]
  def change
    add_column :users, :voice_mode, :string
    add_column :users, :push_to_talk_key, :string
  end
end
