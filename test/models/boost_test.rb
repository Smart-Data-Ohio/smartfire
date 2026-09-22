require "test_helper"

class BoostTest < ActiveSupport::TestCase
  test "accepts a brand shortcode" do
    boost = Boost.new(message: messages(:first), booster: users(:david), content: ":openai:")

    assert boost.valid?
  end

  test "stores an emoji shortcode as its character" do
    boost = Boost.create!(message: messages(:first), booster: users(:david), content: ":thumbsup:")

    assert_equal "👍", boost.reload.content
    assert_not boost.shortcode_content?
  end

  test "canonicalises a brand alias to its brand name" do
    boost = Boost.create!(message: messages(:first), booster: users(:david), content: ":gpt:")

    assert_equal ":openai:", boost.reload.content
  end

  test "stores an unknown shortcode literally" do
    boost = Boost.create!(message: messages(:first), booster: users(:david), content: ":lol:")

    assert_equal ":lol:", boost.reload.content
    assert boost.shortcode_content?
  end

  test "leaves existing plain-text and emoji content unchanged" do
    assert Boost.new(message: messages(:first), booster: users(:david), content: "Morning!").valid?
    assert Boost.new(message: messages(:first), booster: users(:david), content: "💯").valid?
    assert Boost.new(message: messages(:first), booster: users(:david), content: "great :fire: work").valid?
  end

  test "strips the trailing space icon autocomplete inserts before resolving" do
    boost = Boost.create!(message: messages(:first), booster: users(:david), content: ":fire: ")

    assert_equal "🔥", boost.reload.content
  end

  test "strips the trailing space from a brand shortcode" do
    boost = Boost.create!(message: messages(:first), booster: users(:david), content: ":anthropic: ")

    assert_equal ":anthropic:", boost.reload.content
  end

  test "reaction content is a single emoji or a known shortcode" do
    assert Boost.reaction?("👍")
    assert Boost.reaction?("💯")
    assert Boost.reaction?("❤️")
    assert Boost.reaction?(":thumbsup:")
    assert Boost.reaction?(":anthropic:")

    create_workspace_icon(name: "acme", title: "Acme Corp")
    assert Boost.reaction?(":acme:")

    assert_not Boost.reaction?("Morning!")
    assert_not Boost.reaction?(":lol:")
    assert_not Boost.reaction?("👍👍")
    assert_not Boost.reaction?("great :fire: work")
  end

  test "reaction content counts keycaps, flags, ZWJ sequences, VS16 and modifiers" do
    assert Boost.reaction?("1️⃣")
    assert Boost.reaction?("#️⃣")
    assert Boost.reaction?("🇺🇸")
    assert Boost.reaction?("❤️")
    assert Boost.reaction?("👍🏽")
    assert Boost.reaction?("👨‍👩‍👧")

    assert_not Boost.reaction?("1")
    assert_not Boost.reaction?("a")
  end
end
