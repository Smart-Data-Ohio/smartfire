require "test_helper"

class HuddleNoticeChannelTest < ActionCable::Channel::TestCase
  test "streams only the subscriber's own huddle notices" do
    user = users(:david)
    stub_connection(current_user: user)

    subscribe

    assert subscription.confirmed?
    assert_has_stream HuddleNoticeChannel.stream_name_for(user.id)
    assert_not_includes subscription.streams, HuddleNoticeChannel.stream_name_for(users(:jason).id)
  end

  test "rejects bots" do
    stub_connection(current_user: users(:bender))
    subscribe
    assert subscription.rejected?
  end

  test "rejects inactive users" do
    user = users(:david)
    stub_connection(current_user: user)
    user.update!(status: :deactivated)
    subscribe
    assert subscription.rejected?
  end
end
