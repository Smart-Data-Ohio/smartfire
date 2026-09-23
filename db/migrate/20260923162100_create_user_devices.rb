# Stable per-account device registry for new-device sign-in alerts.
# Strictly additive: one new table, indexes, and foreign key. A row is
# created on the first sign-in from a browser (identified by the
# long-lived signed device cookie); later sign-ins from unknown browsers
# alert while the first-ever sign-in stays silent. Rows outlive sessions
# on purpose: signing out everywhere must not reset "first ever".
class CreateUserDevices < ActiveRecord::Migration[8.2]
  def change
    create_table :user_devices do |t|
      t.references :user, null: false, foreign_key: { on_delete: :cascade }
      t.string :device_id, null: false
      t.string :user_agent
      t.timestamps

      t.index %i[ user_id device_id ], unique: true
    end
  end
end
