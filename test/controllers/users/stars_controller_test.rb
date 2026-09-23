require "test_helper"

class Users::StarsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "starring creates a row and answers the card's turbo-stream in place" do
    post user_star_url(users(:kevin)), headers: { "Accept" => "text/vnd.turbo-stream.html" }

    assert_response :success
    assert users(:david).starred?(users(:kevin))
    assert_select "turbo-stream[action='replace'][target='star_user_#{users(:kevin).id}']", 1
    assert_select "turbo-stream button", text: "★ Unstar"
  end

  test "starring answers JSON for the member-row menu" do
    post user_star_url(users(:kevin)), as: :json

    assert_response :success
    assert_equal({ "starred" => true }, response.parsed_body)
    assert users(:david).starred?(users(:kevin))
  end

  test "starring twice keeps one row" do
    post user_star_url(users(:kevin)), as: :json
    post user_star_url(users(:kevin)), as: :json

    assert_response :success
    assert_equal 1, UserStar.where(user: users(:david), starred_user: users(:kevin)).count
  end

  test "unstarring removes the row and answers the card's turbo-stream in place" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    delete user_star_url(users(:kevin)), headers: { "Accept" => "text/vnd.turbo-stream.html" }

    assert_response :success
    assert_not users(:david).starred?(users(:kevin))
    assert_select "turbo-stream[action='replace'][target='star_user_#{users(:kevin).id}']", 1
    assert_select "turbo-stream button", text: "☆ Star"
  end

  test "unstarring answers JSON for the member-row menu" do
    users(:david).user_stars.create!(starred_user: users(:kevin))

    delete user_star_url(users(:kevin)), as: :json

    assert_response :success
    assert_equal({ "starred" => false }, response.parsed_body)
  end

  test "unstarring someone never starred still succeeds" do
    delete user_star_url(users(:kevin)), as: :json

    assert_response :success
    assert_equal({ "starred" => false }, response.parsed_body)
  end

  test "unstarring touches only your own row" do
    users(:david).user_stars.create!(starred_user: users(:kevin))
    users(:jason).user_stars.create!(starred_user: users(:kevin))

    delete user_star_url(users(:kevin)), as: :json

    assert_response :success
    assert_not users(:david).starred?(users(:kevin))
    assert users(:jason).starred?(users(:kevin))
  end

  test "starring yourself is refused" do
    post user_star_url(users(:david)), as: :json

    assert_response :unprocessable_entity
    assert_not users(:david).starred?(users(:david))
  end

  test "starring a bot or agent is allowed" do
    post user_star_url(users(:bender)), as: :json

    assert_response :success
    assert users(:david).starred?(users(:bender))
  end

  test "bots acting as themselves are forbidden" do
    delete session_url
    bot = users(:bender)
    bot.update!(email_address: "bender@example.test", password: "secret123456")
    sign_in bot

    post user_star_url(users(:kevin)), as: :json
    assert_response :forbidden

    delete user_star_url(users(:kevin)), as: :json
    assert_response :forbidden
  end

  test "starring requires sign-in" do
    delete session_url

    post user_star_url(users(:kevin)), as: :json
    assert_response :unauthorized

    delete user_star_url(users(:kevin)), as: :json
    assert_response :unauthorized
  end

  test "bot keys and agent tokens cannot star" do
    delete session_url

    post user_star_url(users(:kevin), bot_key: bot_key_for(users(:bender))), as: :json
    assert_response :forbidden

    secret = "bender-test-secret-1234"
    post user_star_url(users(:kevin)), as: :json,
      headers: { "Authorization" => "Bearer #{secret}" }
    assert_response :forbidden
  end

  test "starring an unknown user is not found" do
    post user_star_url(User.maximum(:id).succ), as: :json

    assert_response :not_found
  end

  test "the profile card shows Star and Unstar states" do
    get user_card_url(users(:kevin))
    assert_response :success
    assert_select "#star_user_#{users(:kevin).id} button", text: "☆ Star"

    users(:david).user_stars.create!(starred_user: users(:kevin))

    get user_card_url(users(:kevin))
    assert_response :success
    assert_select "#star_user_#{users(:kevin).id} button", text: "★ Unstar"
  end

  test "your own card and inactive cards show no star toggle" do
    get user_card_url(users(:david))
    assert_response :success
    assert_select "#star_user_#{users(:david).id}", count: 0

    users(:kevin).deactivate

    get user_card_url(users(:kevin))
    assert_response :success
    assert_select "#star_user_#{users(:kevin).id}", count: 0
  end
end
