require "test_helper"

# Message fragments outlive deploys in Redis, so a fix to legacy presentation
# only reaches cached messages if MessagesHelper::PRESENTATION_CACHE_VERSION
# moves with it.
class Messages::LegacyPresentationCacheTest < ActionDispatch::IntegrationTest
  PAYLOAD = %(<p title="x> http://evil.test/ <img src=x onerror=alert(1)>">hi</p>)

  setup do
    host! "smartfire.test"
    sign_in :david
    @room = rooms(:watercooler)
    @room.messages.create! body: PAYLOAD, client_message_id: "legacy-cached-autolink", creator: users(:david)
  end

  test "fragments cached before the autolink fix aren't served after it" do
    with_caching do
      as_rendered_before_the_fix do
        get room_messages_url(@room)
        assert_select "[onerror]", 1, "the pre-fix render is the vulnerable one"
      end

      get room_messages_url(@room)

      assert_response :success
      assert_select "[onerror]", 0
      assert_select "p[title=?]", "x> http://evil.test/ <img src=x onerror=alert(1)>"
    end
  end

  private
    # Renders as the code did before the fix: without the attribute-value
    # check, and under the cache version that went with it.
    def as_rendered_before_the_fix
      fix = RailsExt::AutoLinkOutsideAttributeValues
      fix.alias_method :fixed_inside_attribute_value?, :inside_attribute_value?
      fix.define_method(:inside_attribute_value?) { |_offset| false }
      with_presentation_cache_version(MessagesHelper::PRESENTATION_CACHE_VERSION - 1) { yield }
    ensure
      fix.alias_method :inside_attribute_value?, :fixed_inside_attribute_value?
      fix.remove_method :fixed_inside_attribute_value?
    end

    def with_presentation_cache_version(version)
      current = MessagesHelper::PRESENTATION_CACHE_VERSION
      replace_presentation_cache_version version
      yield
    ensure
      replace_presentation_cache_version current
    end

    def replace_presentation_cache_version(version)
      MessagesHelper.send :remove_const, :PRESENTATION_CACHE_VERSION
      MessagesHelper.const_set :PRESENTATION_CACHE_VERSION, version
    end

    def with_caching
      original_cache = Rails.cache
      original_collection_cache = ActionView::PartialRenderer.collection_cache
      original_perform_caching = ActionController::Base.perform_caching
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      # The collection renderer snapshots its store at boot, so point it at
      # the memory store too or cached: keeps hitting the null store.
      ActionView::PartialRenderer.collection_cache = Rails.cache
      ActionController::Base.perform_caching = true

      yield
    ensure
      ActionController::Base.perform_caching = original_perform_caching
      ActionView::PartialRenderer.collection_cache = original_collection_cache
      Rails.cache = original_cache
    end
end
