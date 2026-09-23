require "test_helper"

class Rooms::PollsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:watercooler)
  end

  test "creates a poll on a new message" do
    assert_difference -> { @room.messages.count }, 1 do
      assert_difference -> { Poll.count }, 1 do
        post room_polls_url(@room), params: {
          poll: { question: "Lunch?", options: [ "Tacos", "Pizza" ] }
        }, as: :json
      end
    end

    assert_response :created
    assert_equal "Lunch?", response.parsed_body["question"]
    assert_equal 2, response.parsed_body["options"].size

    poll = Poll.order(:id).last
    assert_equal @room.messages.ordered.last, poll.message
    assert_equal users(:david), poll.message.creator
  end

  test "creates multiple anonymous polls with a close time" do
    post room_polls_url(@room), params: {
      poll: {
        question: "Snacks?", options: [ "Chips", "Fruit", "Cake" ],
        multiple: "1", anonymous: "1", closes_at: 2.hours.from_now.strftime("%Y-%m-%dT%H:%M")
      }
    }, as: :json

    assert_response :created
    poll = Poll.order(:id).last
    assert poll.multiple?
    assert poll.anonymous?
    assert_in_delta 2.hours.from_now.to_f, poll.closes_at.to_f, 61
  end

  test "rejects too few options without posting" do
    assert_no_difference -> { Message.count } do
      post room_polls_url(@room), params: {
        poll: { question: "Lunch?", options: [ "Only" ] }
      }, as: :json
    end

    assert_response :unprocessable_entity
  end

  test "votes replace the ballot and broadcast the card" do
    poll = create_poll

    assert_broadcasts room_messages_stream_name(@room), 1 do
      post vote_room_poll_url(@room, poll), params: { option_ids: [ poll.poll_options.first.id ] }, as: :json
    end

    assert_response :success
    assert_equal 1, response.parsed_body["total_votes"]
    assert_equal [ poll.poll_options.first.id ], poll.poll_votes.where(user: users(:david)).pluck(:poll_option_id)

    post vote_room_poll_url(@room, poll), params: { option_ids: [ poll.poll_options.last.id ] }, as: :json

    assert_equal [ poll.poll_options.last.id ], poll.poll_votes.where(user: users(:david)).pluck(:poll_option_id)
  end

  test "empty ballots retract" do
    poll = create_poll
    poll.cast_vote!(users(:david), [ poll.poll_options.first.id ])

    post vote_room_poll_url(@room, poll), as: :json

    assert_response :success
    assert_empty poll.poll_votes.where(user: users(:david))
  end

  test "closed polls reject votes" do
    poll = create_poll(closes_at: 1.hour.from_now)
    travel 2.hours do
      post vote_room_poll_url(@room, poll), params: { option_ids: [ poll.poll_options.first.id ] }, as: :json

      assert_response :unprocessable_entity
    end
  end

  test "foreign options are rejected" do
    poll = create_poll
    other = create_poll(question: "Dinner?")

    post vote_room_poll_url(@room, poll), params: { option_ids: [ other.poll_options.first.id ] }, as: :json

    assert_response :unprocessable_entity
  end

  test "non-members cannot create or vote" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)
    message = private_room.root_messages.create!(creator: users(:jason), markdown_source: "Secret?")
    poll = Poll.create_for_message!(message: message, labels: [ "Yes", "No" ])

    post room_polls_url(private_room), params: { poll: { question: "Hi?", options: [ "A", "B" ] } }, as: :json
    assert_response :not_found

    post vote_room_poll_url(private_room, poll), params: { option_ids: [ poll.poll_options.first.id ] }, as: :json
    assert_response :not_found
  end

  test "bots are forbidden" do
    delete session_url
    bot = users(:bender)
    bot.update!(email_address: "bender@example.test", password: "secret123456")
    sign_in bot

    post room_polls_url(@room), params: { poll: { question: "Hi?", options: [ "A", "B" ] } }, as: :json
    assert_response :forbidden
  end

  private
    def create_poll(question: "Lunch?", **options)
      message = @room.root_messages.create!(creator: users(:david), markdown_source: question)
      Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ], **options)
    end

    def room_messages_stream_name(room)
      signed = Turbo::StreamsChannel.signed_stream_name([ room, :messages ])
      Turbo::StreamsChannel.verified_stream_name(signed)
    end
end
