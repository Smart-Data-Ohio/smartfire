require "test_helper"

class DndAllowedUserTest < ActiveSupport::TestCase
  test "allows starring another active person" do
    allowance = DndAllowedUser.create!(user: users(:david), allowed_user: users(:jason))

    assert users(:david).reload.dnd_allows?(users(:jason))
    assert_not users(:david).dnd_allows?(users(:kevin))
    assert_not users(:david).dnd_allows?(nil)
    assert_equal users(:jason), allowance.allowed_user
  end

  test "rejects duplicates, self-stars, and bots" do
    DndAllowedUser.create!(user: users(:david), allowed_user: users(:jason))

    assert_not DndAllowedUser.new(user: users(:david), allowed_user: users(:jason)).valid?
    assert_not DndAllowedUser.new(user: users(:david), allowed_user: users(:david)).valid?
    assert_not DndAllowedUser.new(user: users(:david), allowed_user: users(:bender)).valid?
  end
end
