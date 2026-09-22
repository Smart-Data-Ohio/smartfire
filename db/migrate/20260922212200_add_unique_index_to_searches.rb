class AddUniqueIndexToSearches < ActiveRecord::Migration[8.2]
  def up
    # Collapse duplicates left by find_or_create races onto the newest row so
    # the unique index cannot fail on existing data. Pure SQL, so it runs
    # with networking off during the migration rehearsal.
    execute <<~SQL.squish
      DELETE FROM searches
      WHERE id NOT IN (SELECT MAX(id) FROM searches GROUP BY user_id, query)
    SQL

    add_index :searches, %i[user_id query], unique: true, name: "index_searches_on_user_and_query"
    remove_index :searches, :user_id
  end

  def down
    add_index :searches, :user_id
    remove_index :searches, name: "index_searches_on_user_and_query"
  end
end
