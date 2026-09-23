require "test_helper"

class PollTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @david = users(:david)
    @jason = users(:jason)
  end

  test "creates a poll with options on a message" do
    message = @room.root_messages.create!(creator: @david, markdown_source: "Lunch?")

    poll = Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ])

    assert_equal message, poll.message
    assert_equal [ "Tacos", "Pizza" ], poll.poll_options.map(&:label)
    assert poll.single?
    assert_not poll.anonymous?
    assert poll.open?
  end

  test "requires between two and ten options" do
    message = @room.root_messages.create!(creator: @david, markdown_source: "Lunch?")

    error = assert_raises(ActiveRecord::RecordInvalid) do
      Poll.create_for_message!(message: message, labels: [ "Only" ])
    end
    assert_match "between 2 and 10", error.record.errors.full_messages.to_sentence
    assert_nil message.reload.poll

    error = assert_raises(ActiveRecord::RecordInvalid) do
      Poll.create_for_message!(message: message, labels: (1..11).map { |n| "Option #{n}" })
    end
    assert_match "between 2 and 10", error.record.errors.full_messages.to_sentence
  end

  test "blank labels are dropped before counting" do
    message = @room.root_messages.create!(creator: @david, markdown_source: "Lunch?")

    poll = Poll.create_for_message!(message: message, labels: [ "Tacos", "", "  ", "Pizza" ])

    assert_equal [ "Tacos", "Pizza" ], poll.poll_options.map(&:label)
  end

  test "one message carries at most one poll" do
    message = @room.root_messages.create!(creator: @david, markdown_source: "Lunch?")
    Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ])

    assert_raises(ActiveRecord::RecordInvalid) do
      Poll.create_for_message!(message: message, labels: [ "Sushi", "Ramen" ])
    end
  end

  test "single-choice voters replace their ballot" do
    poll = create_poll

    poll.cast_vote!(@david, [ poll.poll_options.first.id ])
    assert_equal [ poll.poll_options.first.id ], david_votes(poll)

    poll.cast_vote!(@david, [ poll.poll_options.last.id ])
    assert_equal [ poll.poll_options.last.id ], david_votes(poll)
    assert_equal 1, poll.poll_votes.count
  end

  test "single-choice rejects several options" do
    poll = create_poll

    error = assert_raises(ActiveRecord::RecordInvalid) do
      poll.cast_vote!(@david, poll.poll_options.map(&:id))
    end
    assert_match "only one option", error.record.errors.full_messages.to_sentence
  end

  test "multiple-choice voters hold several options" do
    poll = create_poll(multiple: true)

    poll.cast_vote!(@david, poll.poll_options.map(&:id))

    assert_equal 2, david_votes(poll).size
  end

  test "empty ballots retract the vote and touch the poll" do
    poll = create_poll
    poll.cast_vote!(@david, [ poll.poll_options.first.id ])
    before = poll.reload.updated_at

    travel 1.second do
      poll.cast_vote!(@david, [])
    end

    assert_empty david_votes(poll)
    assert poll.reload.updated_at > before
  end

  test "votes reject foreign options" do
    poll = create_poll
    other = create_poll(question: "Dinner?")

    error = assert_raises(ActiveRecord::RecordInvalid) do
      poll.cast_vote!(@david, [ other.poll_options.first.id ])
    end
    assert_match "not part of this poll", error.record.errors.full_messages.to_sentence
  end

  test "closed polls reject votes" do
    poll = create_poll(closes_at: 1.hour.from_now)
    travel 2.hours do
      assert poll.closed?

      error = assert_raises(ActiveRecord::RecordInvalid) do
        poll.cast_vote!(@david, [ poll.poll_options.first.id ])
      end
      assert_match "closed", error.record.errors.full_messages.to_sentence
    end
  end

  test "close_due stamps and broadcasts due polls once" do
    open_poll = create_poll(closes_at: 2.hours.from_now)
    due_poll = create_poll(question: "Due?", closes_at: 1.hour.from_now)

    travel 90.minutes do
      Poll.close_due!

      assert_not open_poll.reload.closed?
      assert due_poll.reload.closed?
      assert_not_nil due_poll.closed_at
    end
  end

  test "results payload carries counts and voters unless anonymous" do
    poll = create_poll
    poll.cast_vote!(@david, [ poll.poll_options.first.id ])
    poll.cast_vote!(@jason, [ poll.poll_options.first.id, poll.poll_options.last.id ].first(1))

    payload = poll.results_payload(viewer: @david)

    assert_equal "Lunch?", payload[:question]
    assert_equal 2, payload[:total_votes]
    assert_equal [ "David", "Jason" ].sort, payload[:options].first[:voters].sort
    assert payload[:options].first[:voted]
    assert_not payload[:options].last[:voted]
  end

  test "anonymous payloads carry counts only" do
    poll = create_poll(anonymous: true)
    poll.cast_vote!(@david, [ poll.poll_options.first.id ])

    payload = poll.results_payload(viewer: @jason)

    assert_equal 1, payload[:options].first[:votes]
    assert_nil payload[:options].first[:voters]
    assert_not payload[:options].first[:voted]
  end

  test "question follows the message text" do
    poll = create_poll
    poll.message.update!(markdown_source: "Dinner?")

    assert_equal "Dinner?", poll.results_payload[:question]
  end

  private
    def create_poll(question: "Lunch?", **options)
      message = @room.root_messages.create!(creator: @david, markdown_source: question)
      Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ], **options)
    end

    def david_votes(poll)
      poll.poll_votes.where(user: @david).pluck(:poll_option_id)
    end
end
