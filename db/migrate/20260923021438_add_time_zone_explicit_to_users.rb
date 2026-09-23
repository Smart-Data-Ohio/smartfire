class AddTimeZoneExplicitToUsers < ActiveRecord::Migration[8.2]
  # True once the member picks a zone (or "Not set") on the profile page,
  # so browser auto-detect never overwrites a choice they made. Strictly
  # additive: one new column, no backfill, no existing data touched.
  def change
    add_column :users, :time_zone_explicit, :boolean, null: false, default: false
  end
end
