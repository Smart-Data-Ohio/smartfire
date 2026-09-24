class AddTakesArgumentsToAgentSlashCommands < ActiveRecord::Migration[8.2]
  # Whether picking the command in the composer inserts "/name " and waits
  # for arguments (true) or runs it immediately (false). Existing
  # registrations keep today's insert behavior.
  def change
    add_column :agent_slash_commands, :takes_arguments, :boolean, default: true, null: false
  end
end
