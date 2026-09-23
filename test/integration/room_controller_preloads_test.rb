require "test_helper"

class RoomControllerPreloadsTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "room page preloads exactly its first-paint controllers" do
    get room_url(@room)
    assert_response :success

    preloads = controller_preloads(response.body)

    RoomsHelper::FIRST_PAINT_CONTROLLERS.each do |name|
      assert_equal 1, preloads.grep(%r{controllers/#{name}_controller-}).size,
        "expected the #{name} controller to be preloaded exactly once"
    end
    assert_equal RoomsHelper::FIRST_PAINT_CONTROLLERS, preloads.map { |href| href[%r{controllers/(.+)_controller-}, 1] },
      "expected no preloads beyond the first-paint list"
  end

  test "rendered controllers beyond first paint stay lazy" do
    get room_url(@room)
    assert_response :success

    preloads = controller_preloads(response.body)
    lazy = rendered_controller_names(response.body) - RoomsHelper::FIRST_PAINT_CONTROLLERS

    assert_not_empty lazy, "expected the room to render lazily-loaded controllers"
    lazy.each do |name|
      assert_empty preloads.grep(%r{controllers/#{name}_controller-}),
        "expected the #{name} controller to lazy-load instead of preloading"
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
    def rendered_controller_names(html)
      html.scan(/data-controller="([^"]+)"/).flatten.flat_map { |value| value.split }
        .map { |identifier| identifier.tr("-", "_") }.uniq
    end

    def module_preloads(html)
      html.scan(/<link rel="modulepreload" href="([^"]+)"/).flatten
    end

    def controller_preloads(html)
      module_preloads(html).grep(%r{controllers/})
    end
end
