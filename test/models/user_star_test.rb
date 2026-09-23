require "test_helper"

class UserStarTest < ActiveSupport::TestCase
  test "one user can star another only once" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    duplicate = users(:david).user_stars.build(starred_user: users(:kevin))
    assert_not duplicate.valid?
    assert_includes duplicate.errors[:starred_user_id], "has already been taken"
    assert_raises(ActiveRecord::RecordNotUnique) do
      duplicate.save!(validate: false)
    end
  end

  test "different users can star the same person" do
    users(:david).user_stars.create!(starred_user: users(:kevin))
    users(:jason).user_stars.create!(starred_user: users(:kevin))

    assert_equal 2, UserStar.where(starred_user: users(:kevin)).count
  end

  test "a user cannot star themselves" do
    star = users(:david).user_stars.build(starred_user: users(:david))

    assert_not star.valid?
    assert_includes star.errors[:starred_user], "must be someone else"
  end

  test "a bot cannot star itself either" do
    star = users(:bender).user_stars.build(starred_user: users(:bender))

    assert_not star.valid?
    assert_includes star.errors[:starred_user], "must be someone else"
  end

  test "bots and agents can be starred" do
    star = users(:david).user_stars.create!(starred_user: users(:bender))

    assert star.persisted?
    assert users(:bender).agent.present?
  end

  test "destroying the starrer removes their stars" do
    users(:david).user_stars.create!(starred_user: users(:kevin))
    users(:jason).user_stars.create!(starred_user: users(:kevin))

    users(:david).destroy!

    assert_empty UserStar.where(user_id: users(:david).id)
    assert_equal 1, UserStar.where(starred_user: users(:kevin)).count
  end

  test "destroying the starred user removes stars on them" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    users(:kevin).destroy!

    assert_empty UserStar.where(starred_user_id: users(:kevin).id)
  end

  test "deactivating the starred user keeps the row" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    users(:kevin).deactivate

    assert_equal 1, UserStar.where(user: users(:david), starred_user: users(:kevin)).count
  end

  test "starred? and starred_ids_among read one viewer's stars" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    assert users(:david).starred?(users(:kevin))
    assert_not users(:david).starred?(users(:jason))
    assert_not users(:jason).starred?(users(:kevin))
    assert_equal Set[users(:kevin).id],
      users(:david).starred_ids_among([ users(:kevin).id, users(:jason).id ])
  end
end
