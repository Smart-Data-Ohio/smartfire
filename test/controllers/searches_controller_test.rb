require "test_helper"

class SearchesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @message = rooms(:designers).messages.create! body: "Hello world!", client_message_id: "search", creator: users(:david)
  end

  test "index initial view" do
    get searches_url

    assert_response :success
    assert_select ".message", count: 0
  end

  test "finding reachable messages" do
    get searches_url, params: { q: "hello" }

    assert_response :success
    assert_select ".message", text: /Hello world!/
  end

  test "unreachable messages are not found" do
    memberships(:david_designers).destroy!

    get searches_url, params: { q: "hello" }

    assert_response :success
    assert_select ".message", count: 0
  end

  test "operator words are searched literally instead of raising" do
    rooms(:designers).messages.create! body: "cats and dogs", client_message_id: "operators", creator: users(:david)

    get searches_url, params: { q: "NOT" }
    assert_response :success

    get searches_url, params: { q: "cats AND" }
    assert_response :success
    assert_select ".message", text: /cats and dogs/
  end

  test "a leading operator returns 200" do
    get searches_url, params: { q: "OR bar" }
    assert_response :success
  end

  test "a boolean-looking query does not exclude terms" do
    rooms(:designers).messages.create! body: "foo bar", client_message_id: "no-not", creator: users(:david)
    rooms(:designers).messages.create! body: "foo not bar", client_message_id: "with-not", creator: users(:david)

    get searches_url, params: { q: "foo NOT bar" }

    assert_response :success
    assert_select ".message", text: /foo not bar/
    assert_select ".message", text: /foo bar/, count: 0
  end

  test "a quote character returns 200 with sensible results" do
    get searches_url, params: { q: '"hello"' }

    assert_response :success
    assert_select ".message", text: /Hello world!/
  end

  test "create does not run the search" do
    SearchesController.any_instance.expects(:set_messages).never

    post searches_url, params: { q: "NOT" }

    assert_redirected_to searches_url(q: "NOT")
    assert users(:david).searches.exists?(query: "NOT")
  end

  test "clear does not run the search" do
    SearchesController.any_instance.expects(:set_messages).never

    delete clear_searches_url, params: { q: "NOT" }

    assert_redirected_to searches_url
  end

  test "a query with no results shows an empty state" do
    get searches_url, params: { q: "zebra stripes tuxedo" }

    assert_response :success
    assert_select "p.searches__empty", text: /No messages match/
  end

  test "results page through Load older results" do
    base = Time.zone.local(2026, 1, 1, 12, 0, 0)
    created = 45.times.map do |index|
      rooms(:designers).messages.create!(
        body: "paging message number #{index}",
        client_message_id: "paging-#{index}",
        creator: users(:david),
        created_at: base + index.seconds
      )
    end

    get searches_url, params: { q: "paging message number" }

    assert_response :success
    assert_select "#search-results .message", count: 40
    assert_select "#search-results .message", text: /paging message number 44/
    assert_select "#search-results .message", text: /paging message number 0/, count: 0
    assert_select "a", text: "Load older results"

    get searches_url(q: "paging message number", before: created[5].id), as: :turbo_stream

    assert_response :success
    assert_includes response.body, "paging message number 0"
    assert_includes response.body, "paging message number 4"
    assert_not_includes response.body, "paging message number 44"
    assert_includes response.body, 'action="remove" target="load_older_results"'
  end

  test "an older window renders as a page without JavaScript" do
    base = Time.zone.local(2026, 1, 1, 12, 0, 0)
    created = 45.times.map do |index|
      rooms(:designers).messages.create!(
        body: "fallback paging message #{index}",
        client_message_id: "fallback-paging-#{index}",
        creator: users(:david),
        created_at: base + index.seconds
      )
    end

    get searches_url(q: "fallback paging message", before: created[5].id)

    assert_response :success
    assert_select "#search-results .message", count: 5
    assert_select "a", text: "Load older results", count: 0
  end

  test "create saves the search term" do
    assert_difference -> { users(:david).searches.count }, +1 do
      post searches_url, params: { q: "hello" }
    end

    assert_redirected_to searches_url(q: "hello")
    assert users(:david).searches.exists?(query: "hello")
  end

  test "clear search history" do
    assert users(:david).searches.any?

    delete clear_searches_url

    assert users(:david).searches.none?
  end
end
