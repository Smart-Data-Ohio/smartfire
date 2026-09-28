# Tables behind the Slack importer (docs/slack-import.md): the workspace's
# internal Slack app, each member's Slack connection, import runs, the
# Slack-id-to-record mapping that makes runs idempotent and undoable, and
# per-run issues. New tables only: no models, jobs, Redis, or network.
class CreateSlackImportTables < ActiveRecord::Migration[8.2]
  def change
    create_table :slack_workspaces do |t|
      t.string :client_id
      t.text :client_secret
      t.string :team_id
      t.string :team_name
      t.string :team_domain
      t.references :configured_by, foreign_key: { to_table: :users }
      t.timestamps
      t.index :team_id, unique: true
    end

    create_table :slack_connections do |t|
      t.references :slack_workspace, null: false, foreign_key: true
      t.references :user, null: false, foreign_key: true, index: { unique: true }
      t.string :slack_user_id, null: false
      t.text :access_token
      t.string :scopes
      t.string :disconnected_reason
      t.timestamps
      t.index %i[ slack_workspace_id slack_user_id ], unique: true
    end

    create_table :slack_imports do |t|
      t.references :slack_workspace, null: false, foreign_key: true
      t.references :slack_connection, foreign_key: { on_delete: :nullify }
      t.references :user, null: false, foreign_key: true
      t.string :kind, null: false
      t.string :mode, null: false
      t.string :status, null: false, default: "queued"
      t.json :options, null: false, default: {}
      t.json :state, null: false, default: {}
      t.json :stats, null: false, default: {}
      t.text :error
      t.datetime :heartbeat_at
      t.datetime :started_at
      t.datetime :finished_at
      t.timestamps
      t.index :status
    end

    create_table :slack_import_records do |t|
      t.references :slack_workspace, null: false, foreign_key: true
      t.references :slack_import, null: false, foreign_key: true
      t.string :slack_kind, null: false
      t.string :slack_key, null: false
      t.string :record_type, null: false
      t.bigint :record_id, null: false
      t.boolean :created_record, null: false, default: true
      t.timestamps
      t.index %i[ slack_workspace_id slack_kind slack_key ], unique: true, name: "index_slack_import_records_on_slack_identity"
      t.index %i[ record_type record_id ]
    end

    create_table :slack_import_issues do |t|
      t.references :slack_import, null: false, foreign_key: true
      t.string :level, null: false
      t.string :slack_ref
      t.text :message, null: false
      t.datetime :created_at, null: false
    end
  end
end
