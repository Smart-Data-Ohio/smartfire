class AddUniqueIndexToSearches < ActiveRecord::Migration[8.2]
  def change
    # Race-safe Search.record without touching existing rows: new rows carry
    # their query in dedup_key, which stays NULL for every pre-existing row
    # (NULLs are exempt from the unique index), so legacy duplicates — and
    # the old index — are left alone. Runtime trimming keeps only the 10
    # most recent searches per user.
    add_column :searches, :dedup_key, :string
    add_index :searches, %i[user_id dedup_key], unique: true, name: "index_searches_on_user_and_dedup_key"
  end
end
