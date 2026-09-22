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

  test "user and query are unique so concurrent records cannot duplicate" do
    assert Search.connection.index_exists?(:searches, %i[user_id query], unique: true)
  end
end
