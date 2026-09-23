require "test_helper"

class SlashCommands::TimeParserTest < ActiveSupport::TestCase
  setup do
    @zone = "America/New_York"
    travel_to Time.zone.local(2026, 9, 23, 12, 0, 0)
  end

  teardown do
    travel_back
  end

  test "parses relative durations" do
    assert_equal Time.current + 20.minutes, SlashCommands::TimeParser.parse("in 20 minutes", zone: @zone)
    assert_equal Time.current + 1.hour, SlashCommands::TimeParser.parse("in 1 hour", zone: @zone)
    assert_equal Time.current + 3.hours, SlashCommands::TimeParser.parse("in 3 hours", zone: @zone)
    assert_equal Time.current + 2.days, SlashCommands::TimeParser.parse("in 2 days", zone: @zone)
    assert_equal Time.current + 1.week, SlashCommands::TimeParser.parse("in 1 week", zone: @zone)
  end

  test "parses tomorrow and today in the given zone" do
    assert_equal Time.find_zone(@zone).local(2026, 9, 24, 9, 0), SlashCommands::TimeParser.parse("tomorrow 9am", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 24, 21, 30), SlashCommands::TimeParser.parse("tomorrow at 9:30pm", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 23, 15, 0), SlashCommands::TimeParser.parse("today at 3pm", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 23, 15, 0), SlashCommands::TimeParser.parse("at 15:00", zone: @zone)
  end

  test "a passed time today rolls to tomorrow" do
    # Noon UTC is 8am in New York, so 7am already passed there.
    assert_equal Time.find_zone(@zone).local(2026, 9, 24, 7, 0), SlashCommands::TimeParser.parse("at 7am", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 23, 9, 0), SlashCommands::TimeParser.parse("at 9am", zone: @zone)
  end

  test "parses weekdays with a 9am default" do
    # 2026-09-23 is a Wednesday.
    assert_equal Time.find_zone(@zone).local(2026, 9, 25, 9, 0), SlashCommands::TimeParser.parse("friday", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 25, 17, 0), SlashCommands::TimeParser.parse("friday 5pm", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 10, 2, 9, 0), SlashCommands::TimeParser.parse("next friday", zone: @zone)
    assert_equal Time.find_zone(@zone).local(2026, 9, 23, 17, 0), SlashCommands::TimeParser.parse("wednesday 5pm", zone: @zone)
  end

  test "parses explicit datetimes in the given zone" do
    assert_equal Time.find_zone(@zone).local(2026, 10, 1, 15, 0),
      SlashCommands::TimeParser.parse("2026-10-01 15:00", zone: @zone)
  end

  test "returns nil for unparseable text" do
    assert_nil SlashCommands::TimeParser.parse("sometime-ish", zone: @zone)
    assert_nil SlashCommands::TimeParser.parse("", zone: @zone)
    assert_nil SlashCommands::TimeParser.parse("in zero minutes", zone: @zone)
  end

  test "split_leading_time separates when from text" do
    time, text = SlashCommands::TimeParser.split_leading_time("in 20 minutes review the deploy", zone: @zone)

    assert_equal Time.current + 20.minutes, time
    assert_equal "review the deploy", text
  end

  test "split_leading_time returns nil without a leading time" do
    assert_nil SlashCommands::TimeParser.split_leading_time("review the deploy friday", zone: @zone)
  end

  test "split_trailing_time separates title from when" do
    title, time = SlashCommands::TimeParser.split_trailing_time("Launch party friday 5pm", zone: @zone)

    assert_equal "Launch party", title
    assert_equal Time.find_zone(@zone).local(2026, 9, 25, 17, 0), time
  end

  test "split_trailing_time keeps a bare time as the title" do
    assert_equal [ "Friday", nil ], SlashCommands::TimeParser.split_trailing_time("Friday", zone: @zone)
    assert_equal [ "Party", nil ], SlashCommands::TimeParser.split_trailing_time("Party", zone: @zone)
  end
end
