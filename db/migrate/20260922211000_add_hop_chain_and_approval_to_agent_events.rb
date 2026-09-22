class AddHopChainAndApprovalToAgentEvents < ActiveRecord::Migration[8.2]
  def change
    add_column :agent_events, :hop, :integer, null: false, default: 0
    add_column :agent_events, :chain_id, :string
    add_column :agent_events, :agent_approval_id, :integer

    reversible do |direction|
      direction.up do
        execute <<~SQL.squish
          UPDATE agent_events
          SET hop = COALESCE(CAST(json_extract(metadata, '$.hop') AS INTEGER), 0)
        SQL
        execute <<~SQL.squish
          UPDATE agent_events
          SET agent_approval_id = CAST(json_extract(metadata, '$.approval_id') AS INTEGER)
          WHERE event_type = 'approval_decided'
        SQL

        # Concurrent writers could record the same completion twice before
        # the unique index below existed. The backfill claims the new column
        # only for the lowest id per (agent, approval) group and leaves later
        # duplicates NULL, so the index builds cleanly without touching
        # pre-existing rows or columns; SQLite allows multiple NULLs.
        # Runtime lookups fall back to the metadata payload for those rows.
        execute <<~SQL.squish
          UPDATE agent_events
          SET agent_approval_id = CAST(json_extract(metadata, '$.approval_id') AS INTEGER)
          WHERE event_type = 'github_action_completed'
            AND id IN (
              SELECT MIN(id) FROM agent_events
              WHERE event_type = 'github_action_completed'
                AND json_extract(metadata, '$.approval_id') IS NOT NULL
              GROUP BY agent_id, CAST(json_extract(metadata, '$.approval_id') AS INTEGER)
            )
        SQL
      end
    end

    add_index :agent_events, [ :agent_id, :agent_approval_id ],
      unique: true,
      where: "event_type = 'github_action_completed' AND agent_approval_id IS NOT NULL",
      name: "index_agent_events_on_agent_github_approval"
  end
end
