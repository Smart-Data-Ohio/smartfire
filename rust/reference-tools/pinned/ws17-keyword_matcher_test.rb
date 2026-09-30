require "test_helper"

class Notifications::KeywordMatcherTest < ActiveSupport::TestCase
  test "matches case-insensitively" do
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy" ] }, "Time to DEPLOY")
  end

  test "matches on word boundaries only" do
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy" ] }, "Redeploying now")
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy" ] }, "deploy, then lunch")
  end

  test "matches multi-word phrases" do
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy freeze" ] }, "There is a Deploy Freeze today")
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy freeze" ] }, "Deploy the freeze ray")
  end

  test "treats phrases literally, not as patterns" do
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "v1.2 (rc)" ] }, "Shipping v1.2 (rc) now")
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "v1.2 (rc)" ] }, "Shipping v1X2 rc now")
  end

  test "returns every user with a match" do
    matched = Notifications::KeywordMatcher.matching_user_ids(
      { 1 => [ "deploy" ], 2 => [ "lunch", "deploy" ], 3 => [ "nothing" ] },
      "Deploy after lunch"
    )

    assert_equal [ 1, 2 ], matched.sort
  end

  test "overlapping phrases across users all match" do
    matched = Notifications::KeywordMatcher.matching_user_ids(
      { 1 => [ "deploy failed" ], 2 => [ "deploy" ] },
      "deploy failed again"
    )

    assert_equal [ 1, 2 ], matched.sort
  end

  test "nested phrases match the same user once" do
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids(
      { 1 => [ "deploy", "deploy failed" ] }, "deploy failed again"
    )
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids(
      { 1 => [ "deploy failed again", "deploy" ] }, "deploy failed again"
    )
  end

  test "repeated phrases match every holder once" do
    matched = Notifications::KeywordMatcher.matching_user_ids(
      { 1 => [ "deploy" ], 2 => [ "deploy", "again" ] },
      "deploy, deploy, deploy again"
    )

    assert_equal [ 1, 2 ], matched.sort
  end

  test "ignores blank phrases and blank text" do
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "  " ] }, "Deploy now")
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy" ] }, "   ")
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({}, "Deploy now")
  end
  test "a phrase matches across a line break and not inside a longer Unicode word" do
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "deploy failed" ] }, "the deploy\nfailed again")
    assert_equal [], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "caf" ] }, "meet at the café")
    assert_equal [ 1 ], Notifications::KeywordMatcher.matching_user_ids({ 1 => [ "café" ] }, "meet at the Café today")
  end
end
