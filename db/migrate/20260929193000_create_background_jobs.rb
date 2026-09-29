# The durable job queue of the Rust port (rust/crates/jobs). The Rails app
# never reads or writes this table: Rails jobs stay on Resque. It lives in
# the main database so that the Rust app enqueues a job in the same
# transaction as the write that triggers it. New table only: no models,
# jobs, Redis, or network.
class CreateBackgroundJobs < ActiveRecord::Migration[8.2]
  def change
    create_table :background_jobs do |t|
      t.string :queue_name, null: false
      t.string :job_class, null: false
      t.json :arguments, null: false
      t.integer :payload_version, null: false, default: 1
      t.string :status, null: false, default: "ready"
      t.integer :attempts, null: false, default: 0
      t.datetime :run_at, null: false
      t.string :claimed_by
      t.datetime :lease_expires_at
      t.text :last_error
      t.datetime :failed_at
      t.timestamps
      t.index %i[ status queue_name run_at ], name: "index_background_jobs_for_claiming"
    end
  end
end
