class CreateAgentSlashCommands < ActiveRecord::Migration[8.2]
  def change
    create_table :agent_slash_commands do |t|
      t.references :agent, null: false, foreign_key: true
      t.references :room, null: false, foreign_key: true
      t.string :name, null: false
      t.string :description

      t.timestamps
    end

    # One owner per command name in a room: invocation must never be
    # ambiguous about which agent receives it.
    add_index :agent_slash_commands, %i[ room_id name ], unique: true
  end
end
