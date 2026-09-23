require "test_helper"

class Fizzy::CardUrlTest < ActiveSupport::TestCase
  test "extracts a canonical card URL" do
    assert_equal [ Fizzy::CardUrl::Reference.new("897362094", 579) ],
      Fizzy::CardUrl.extract("see https://app.fizzy.do/897362094/cards/579 please")
  end

  test "extracts trailing paths, queries, and fragments" do
    assert_equal [ Fizzy::CardUrl::Reference.new("897362094", 579) ],
      Fizzy::CardUrl.extract("https://app.fizzy.do/897362094/cards/579/comments/03comment1")

    assert_equal [ Fizzy::CardUrl::Reference.new("897362094", 579) ],
      Fizzy::CardUrl.extract("https://app.fizzy.do/897362094/cards/579?x=1#frag")
  end

  test "extracts multiple URLs and deduplicates repeats" do
    text = <<~TEXT
      https://app.fizzy.do/897362094/cards/1 and https://app.fizzy.do/6264925/cards/2
      again: https://app.fizzy.do/897362094/cards/1
    TEXT

    assert_equal [
      Fizzy::CardUrl::Reference.new("897362094", 1),
      Fizzy::CardUrl::Reference.new("6264925", 2)
    ], Fizzy::CardUrl.extract(text)
  end

  test "ignores non-card URLs" do
    assert_empty Fizzy::CardUrl.extract("https://app.fizzy.do/897362094/boards/03board1")
    assert_empty Fizzy::CardUrl.extract("https://app.fizzy.do/897362094/cards")
    assert_empty Fizzy::CardUrl.extract("https://example.com/897362094/cards/123")
    assert_empty Fizzy::CardUrl.extract("http://app.fizzy.do/897362094/cards/123")
    assert_empty Fizzy::CardUrl.extract("just some text")
    assert_empty Fizzy::CardUrl.extract(nil)
  end

  test "extract caps references at the PR cards MAX_PER_MESSAGE" do
    assert_equal Github::PullRequestUrl::MAX_PER_MESSAGE, Fizzy::CardUrl::MAX_PER_MESSAGE

    text = (1..6).map { |number| "https://app.fizzy.do/897362094/cards/#{number}" }.join(" ")

    assert_equal [ 1, 2, 3, 4 ], Fizzy::CardUrl.extract(text).map(&:number)
  end

  test "non_code_text drops code spans and fenced blocks but keeps prose and labeled links" do
    html = <<~HTML
      <p>see <code>https://app.fizzy.do/1/cards/1</code> and https://app.fizzy.do/1/cards/2</p>
      <pre><code class="language-text">https://app.fizzy.do/1/cards/3</code></pre>
      <p><a href="https://app.fizzy.do/1/cards/4">the card</a></p>
    HTML

    text = Fizzy::CardUrl.non_code_text(html)

    assert_not_includes text, "cards/1"
    assert_not_includes text, "cards/3"
    assert_includes text, "cards/2"
    assert_includes text, "cards/4"
  end

  test "card_url? matches card URLs only" do
    assert Fizzy::CardUrl.card_url?("https://app.fizzy.do/897362094/cards/579")
    assert_not Fizzy::CardUrl.card_url?("https://app.fizzy.do/897362094/boards/1")
    assert_not Fizzy::CardUrl.card_url?("https://example.com/x")
  end

  test "extracts only card URLs on the configured host" do
    original = ENV["FIZZY_API_BASE_URL"]
    ENV["FIZZY_API_BASE_URL"] = "https://fizzy.example.com"
    begin
      assert_equal [ Fizzy::CardUrl::Reference.new("897362094", 579) ],
        Fizzy::CardUrl.extract("see https://fizzy.example.com/897362094/cards/579 please")
      assert_empty Fizzy::CardUrl.extract("see https://app.fizzy.do/897362094/cards/579 please")
      assert Fizzy::CardUrl.card_url?("https://fizzy.example.com/897362094/cards/579")
      assert_not Fizzy::CardUrl.card_url?("https://app.fizzy.do/897362094/cards/579")
    ensure
      ENV["FIZZY_API_BASE_URL"] = original
    end
  end
end
