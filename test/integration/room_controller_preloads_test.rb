require "test_helper"

class RoomControllerPreloadsTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "room page preloads every controller it renders" do
    get room_url(@room)
    assert_response :success
    assert_preloads_cover_rendered_controllers(response.body)

    Room.voices.first&.then do |voice|
      get room_url(voice)
      assert_response :success
      assert_preloads_cover_rendered_controllers(response.body)
    end

    Room.boards.first&.then do |board|
      get room_url(board)
      assert_response :success
      assert_preloads_cover_rendered_controllers(response.body)
    end
  end

  test "sidebar frame preloads are covered by the room list" do
    get user_sidebar_url
    assert_response :success

    rendered = rendered_controller_names(response.body)
    assert_includes rendered, "rooms_list"

    missing = rendered - RoomsHelper::FIRST_PAINT_CONTROLLERS
    assert_empty missing, "sidebar renders controllers without preloads: #{missing.inspect}"
  end

  test "every listed controller is preloaded exactly once" do
    get room_url(@room)
    assert_response :success

    preloads = module_preloads(response.body)
    assert_not_empty preloads

    RoomsHelper::FIRST_PAINT_CONTROLLERS.each do |name|
      assert_equal 1, preloads.grep(%r{controllers/#{name}_controller-}).size,
        "expected the #{name} controller to be preloaded exactly once"
    end
  end

  test "other pages controllers stay lazy" do
    get room_url(@room)
    assert_response :success

    preloads = module_preloads(response.body)

    %w[ activity_inbox search_results sessions autocomplete auto_submit
        icon_field copy_to_clipboard filter message_format upload_preview
        form scroll_into_view thread_messages ].each do |name|
      assert_empty preloads.grep(%r{controllers/#{name}_controller-}),
        "expected the #{name} controller not to be preloaded"
    end
  end

  private
    def assert_preloads_cover_rendered_controllers(html)
      rendered = rendered_controller_names(html)
      assert_not_empty rendered

      missing = rendered - RoomsHelper::FIRST_PAINT_CONTROLLERS
      assert_empty missing, "room page renders controllers without preloads: #{missing.inspect}"
    end

    def rendered_controller_names(html)
      html.scan(/data-controller="([^"]+)"/).flatten.flat_map { |value| value.split }
        .map { |identifier| identifier.tr("-", "_") }.uniq
    end

    def module_preloads(html)
      html.scan(/<link rel="modulepreload" href="([^"]+)"/).flatten
    end
end
