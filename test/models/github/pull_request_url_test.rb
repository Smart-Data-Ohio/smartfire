require "test_helper"

class Github::PullRequestUrlTest < ActiveSupport::TestCase
  test "extracts a canonical pull request URL" do
    assert_equal [ Github::PullRequestUrl::Reference.new("rails", "rails", 123) ],
      Github::PullRequestUrl.extract("see https://github.com/rails/rails/pull/123 please")
  end

  test "extracts /pulls/ variants and trailing paths, queries, and fragments" do
    assert_equal [ Github::PullRequestUrl::Reference.new("o", "r", 45) ],
      Github::PullRequestUrl.extract("https://github.com/o/r/pulls/45")

    assert_equal [ Github::PullRequestUrl::Reference.new("o", "r", 45) ],
      Github::PullRequestUrl.extract("https://github.com/o/r/pull/45/files?diff=split#diff-1")
  end

  test "extracts multiple URLs and deduplicates repeats" do
    text = <<~TEXT
      https://github.com/rails/rails/pull/1 and https://github.com/rails/rails/pull/2
      again: https://github.com/rails/rails/pull/1
    TEXT

    assert_equal [
      Github::PullRequestUrl::Reference.new("rails", "rails", 1),
      Github::PullRequestUrl::Reference.new("rails", "rails", 2)
    ], Github::PullRequestUrl.extract(text)
  end

  test "ignores non-PR URLs" do
    assert_empty Github::PullRequestUrl.extract("https://github.com/rails/rails/issues/123")
    assert_empty Github::PullRequestUrl.extract("https://github.com/rails/rails/pulls")
    assert_empty Github::PullRequestUrl.extract("https://github.com/rails/rails/pull")
    assert_empty Github::PullRequestUrl.extract("https://example.com/rails/rails/pull/123")
    assert_empty Github::PullRequestUrl.extract("http://github.com/rails/rails/pull/123")
    assert_empty Github::PullRequestUrl.extract("just some text")
    assert_empty Github::PullRequestUrl.extract(nil)
  end

  test "extract caps references at Twitter's MAX_PER_MESSAGE" do
    assert_equal Twitter::PostUrl::MAX_PER_MESSAGE, Github::PullRequestUrl::MAX_PER_MESSAGE

    text = (1..6).map { |number| "https://github.com/rails/rails/pull/#{number}" }.join(" ")

    assert_equal [ 1, 2, 3, 4 ], Github::PullRequestUrl.extract(text).map(&:number)
  end

  test "non_code_text drops code spans and fenced blocks but keeps prose and labeled links" do
    html = <<~HTML
      <p>see <code>https://github.com/o/r/pull/1</code> and https://github.com/o/r/pull/2</p>
      <pre><code class="language-text">https://github.com/o/r/pull/3</code></pre>
      <p><a href="https://github.com/o/r/pull/4">the PR</a></p>
    HTML

    text = Github::PullRequestUrl.non_code_text(html)

    assert_not_includes text, "pull/1"
    assert_not_includes text, "pull/3"
    assert_includes text, "pull/2"
    assert_includes text, "pull/4"
  end

  test "pull_request_url? matches only PR URLs" do
    assert Github::PullRequestUrl.pull_request_url?("https://github.com/rails/rails/pull/123")
    assert Github::PullRequestUrl.pull_request_url?("https://github.com/rails/rails/pulls/123/files")
    assert_not Github::PullRequestUrl.pull_request_url?("https://github.com/rails/rails/issues/123")
    assert_not Github::PullRequestUrl.pull_request_url?("https://github.com/rails/rails/pulls")
    assert_not Github::PullRequestUrl.pull_request_url?(nil)
  end

  test "ignores dot-only owner and repo segments" do
    assert_empty Github::PullRequestUrl.extract("https://github.com/../../pull/1")
    assert_empty Github::PullRequestUrl.extract("https://github.com/./rails/pull/1")
    assert_empty Github::PullRequestUrl.extract("https://github.com/rails/../pull/1")
    assert_empty Github::PullRequestUrl.extract("https://github.com/../pull/1")
    assert_not Github::PullRequestUrl.pull_request_url?("https://github.com/../../pull/1")
  end

  test "still matches names containing dots and dashes" do
    assert_equal [ Github::PullRequestUrl::Reference.new("foo.bar", "baz-qux", 7) ],
      Github::PullRequestUrl.extract("https://github.com/foo.bar/baz-qux/pull/7")
  end
end
