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
