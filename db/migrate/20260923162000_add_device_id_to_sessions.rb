# Adds the stable device identifier sessions are keyed by for new-device
# sign-in alerts and the "Your sessions" page. Strictly additive: one new
# nullable column plus one new index; old rows keep NULL and are treated
# as unknown devices.
class AddDeviceIdToSessions < ActiveRecord::Migration[8.2]
  def change
    add_column :sessions, :device_id, :string
    add_index :sessions, %i[ user_id device_id ]
  end
end
