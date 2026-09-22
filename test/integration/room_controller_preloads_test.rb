require "test_helper"

class RoomControllerPreloadsTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "room page preloads its first-paint controllers but not lazy ones" do
    get room_url(@room)
    assert_response :success

    preloads = response.body.scan(/<link rel="modulepreload" href="([^"]+)"/).flatten
    assert_not_empty preloads

    # Lazy controllers load on demand; they must not ride along as preloads.
    %w[ huddle drive_share thread_panel ].each do |name|
      assert_empty preloads.grep(%r{controllers/#{name}_controller-}),
        "expected the #{name} controller not to be preloaded"
    end

    RoomsHelper::FIRST_PAINT_CONTROLLERS.each do |name|
      assert_equal 1, preloads.grep(%r{controllers/#{name}_controller-}).size,
        "expected the #{name} controller to be preloaded exactly once"
    end
  end
end
