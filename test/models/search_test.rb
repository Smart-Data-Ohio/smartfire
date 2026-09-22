require "test_helper"

class SearchTest < ActiveSupport::TestCase
  test "record creates once and touches on repeat" do
    user = users(:david)

    assert_difference -> { user.searches.count }, 1 do
      user.searches.record("pizza rolls")
    end

    search = user.searches.find_by!(query: "pizza rolls")
    travel 1.minute do
      assert_no_difference -> { user.searches.count } do
        user.searches.record("pizza rolls")
      end

      assert_operator search.reload.updated_at, :>, search.created_at
    end
  end

  test "record is scoped per user" do
    users(:david).searches.record("shared query")
    users(:jason).searches.record("shared query")

    assert_equal 2, Search.where(query: "shared query").count
  end

  test "user and dedup key are unique so concurrent records cannot duplicate" do
    assert Search.connection.index_exists?(:searches, %i[user_id dedup_key], unique: true)
    # The pre-existing index stays: migrations never drop indexes.
    assert Search.connection.index_exists?(:searches, :user_id)
  end

  test "record survives a concurrent insert between its find and create" do
    user = users(:david)
    winner = user.searches.create!(query: "race winner")

    # The loser's find misses, its insert hits the winner's row, and the
    # retry finds the winner: without the unique index the insert would
    # succeed and duplicate the row.
    Search.stubs(:find_by).returns(nil)
    Search.stubs(:find_by!).returns(winner)

    assert_no_difference -> { user.searches.count } do
      user.searches.record("race winner")
    end
  end

  test "record touches a legacy row without a dedup key instead of duplicating it" do
    user = users(:david)
    legacy = user.searches.create!(query: "legacy query")
    legacy.update_columns(dedup_key: nil)

    assert_no_difference -> { user.searches.where(query: "legacy query").count } do
      user.searches.record("legacy query")
    end

    assert_nil legacy.reload.dedup_key
  end
end
