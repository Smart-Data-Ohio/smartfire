require "test_helper"

class KeywordAlertTest < ActiveSupport::TestCase
  test "normalizes whitespace" do
    alert = KeywordAlert.create!(user: users(:david), phrase: "  deploy   freeze  ")

    assert_equal "deploy freeze", alert.phrase
  end

  test "rejects blank and overlong phrases" do
    assert_not KeywordAlert.new(user: users(:david), phrase: "   ").valid?

    long = KeywordAlert.new(user: users(:david), phrase: "x" * (KeywordAlert::PHRASE_LIMIT + 1))
    assert_not long.valid?
  end

  test "rejects case-insensitive duplicates per user but allows them across users" do
    KeywordAlert.create!(user: users(:david), phrase: "Deploy")

    duplicate = KeywordAlert.new(user: users(:david), phrase: "deploy")
    assert_not duplicate.valid?

    other = KeywordAlert.new(user: users(:jason), phrase: "deploy")
    assert other.valid?
  end

  test "caps each user at twenty phrases" do
    KeywordAlert::MAX_PER_USER.times do |index|
      KeywordAlert.create!(user: users(:david), phrase: "phrase #{index}")
    end

    extra = KeywordAlert.new(user: users(:david), phrase: "one too many")
    assert_not extra.valid?
  end
end
