require "application_system_test_case"
require "fileutils"

class MessageInteractionsTest < ApplicationSystemTestCase
  SCREENSHOT_DIR = Rails.root.join("tmp/screenshots/message-ux")

  setup do
    FileUtils.mkdir_p(SCREENSHOT_DIR)
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "opens message actions from context menu and keyboard, and cancels a moving long press" do
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    assert_selector "#message-actions-menu .message__quick-reaction", count: EmojiHelper::REACTIONS.length, visible: true

    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"

    message = find("##{dom_id(messages(:third))}")
    message.click
    page.execute_script <<~JS, message
      arguments[0].dispatchEvent(new KeyboardEvent("keydown", {
        bubbles: true,
        cancelable: true,
        key: "F10",
        shiftKey: true
      }))
    JS
    assert_message_menu_open
    page.send_keys :escape
    assert_focused "##{dom_id(messages(:third))}"

    page.current_window.resize_to(390, 844)
    message = find("##{dom_id(messages(:third))}")
    long_press(message, move_by: [ 25, 0 ])
    assert_no_selector ".message[data-message-actions-open]"

    long_press(message)
    assert_message_menu_open
    assert_menu_within_viewport
    page.execute_script "window.confirm = () => false"
    click_button "Delete message"
    assert_selector "#message-actions-menu .message__delete-action", visible: true
    assert_selector "##{dom_id(messages(:third))}"
    save_screenshot "mobile-long-press.png"
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "a release click landing on the just-opened menu does not activate it" do
    page.current_window.resize_to(390, 844)
    message = find("##{dom_id(messages(:third))}")

    long_press(message)
    assert_message_menu_open

    # The browser fires compatibility mouse events at the release point after
    # every touch. When the menu opens under the finger first, that click
    # lands on the menu item below it (Reply on phones) and must be
    # swallowed instead of activating it.
    hit = page.evaluate_script(<<~JS, dom_id(messages(:third)))
      ((messageId) => {
        const message = document.getElementById(messageId)
        const rect = message.getBoundingClientRect()
        const x = rect.left + rect.width / 2
        const y = rect.top + rect.height / 2
        const target = document.elementFromPoint(x, y)
        if (!target || !target.closest("#message-actions-menu")) return target ? target.tagName : "none"
        target.dispatchEvent(new MouseEvent("click", {
          bubbles: true, cancelable: true, clientX: x, clientY: y, view: window
        }))
        return "menu"
      })(arguments[0])
    JS
    assert_equal "menu", hit, "expected the press point to hit the open menu"

    assert_message_menu_open
    assert_selector "[data-composer-target='context'][hidden]", visible: false
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "shows the message action menu as a bottom sheet on phones" do
    page.current_window.resize_to(390, 844)
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    assert_bottom_sheet_action_menu

    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"

    page.current_window.resize_to(320, 740)
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    assert_bottom_sheet_action_menu

    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "edits through the normal composer and restores the saved draft on cancel and success" do
    fill_in "Write a message", with: "A draft that must survive editing"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Edit message"

    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
    assert_field "Write a message", with: "Third time's a charm."
    save_screenshot "desktop-edit-context.png"
    click_button "Cancel message context"
    assert_field "Write a message", with: "A draft that must survive editing"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Edit message"
    fill_in "Write a message", with: "Saved through the main composer"
    click_button "Send Message"

    assert_selector ".message__body", text: "Saved through the main composer", wait: 10
    assert_selector "[data-composer-target='context'][hidden]", visible: false
    assert_field "Write a message", with: "A draft that must survive editing"
    save_screenshot "desktop-edit-composer.png"
  end

  test "a duplicate delivery does not replace the message while its actions are open" do
    message = messages(:third)
    within_message(message) do
      right_click_message
    end
    assert_message_menu_open
    assert_button "Edit message", wait: 10

    page.execute_script <<~JS, dom_id(message), dom_id(@room, :messages)
      const [messageId, targetId] = arguments;
      window.originalDeliveredMessage = document.getElementById(messageId);
      const observeDelivery = event => {
        const stream = event.detail.newStream;
        if (stream.getAttribute('action') !== 'append' || stream.getAttribute('target') !== targetId) return;
        document.removeEventListener('turbo:before-stream-render', observeDelivery);
        const render = event.detail.render;
        event.detail.render = async streamElement => {
          await render(streamElement);
          document.documentElement.setAttribute('data-duplicate-delivery-rendered', 'true');
        };
      };
      document.addEventListener('turbo:before-stream-render', observeDelivery);
    JS

    message.broadcast_create
    assert_selector "html[data-duplicate-delivery-rendered]", wait: 10
    assert page.evaluate_script("window.originalDeliveredMessage.isConnected"), "redelivery must preserve the existing message and its active controls"
    click_button "Edit message"
    assert_field "Write a message", with: "Third time's a charm."
  end

  test "keeps newer typing through an asynchronous edit and leaves failures in edit mode" do
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Edit message"
    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
    fill_in "Write a message", with: "First edit request"

    page.execute_script <<~JS
      window.__messageInteractionsOriginalFetch = window.fetch
      window.fetch = (input, options = {}) => {
        if (options.method === "PATCH") {
          return new Promise(resolve => { window.__messageInteractionsResolveEdit = resolve })
        }
        return window.__messageInteractionsOriginalFetch(input, options)
      }
    JS

    click_button "Send Message"
    fill_in "Write a message", with: "A newer draft typed while saving"
    page.execute_script <<~JS
      window.__messageInteractionsResolveEdit(new Response("{}", { status: 200 }))
    JS

    assert_field "Write a message", with: "A newer draft typed while saving", wait: 10
    assert_selector "[data-composer-target='context'][hidden]", visible: false

    page.execute_script <<~JS
      window.fetch = (input, options = {}) => {
        if (options.method === "PATCH") {
          return Promise.resolve(new Response(JSON.stringify({ error: "The message could not be saved" }), {
            status: 422,
            headers: { "Content-Type": "application/json" }
          }))
        }
        return window.__messageInteractionsOriginalFetch(input, options)
      }
    JS

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Edit message"
    fill_in "Write a message", with: "Failed edit"
    click_button "Send Message"
    assert_selector "[data-composer-target='feedback']", text: "The message could not be saved", visible: true, wait: 10
    assert_selector "[data-composer-target='context']", visible: true
    click_button "Cancel message context"
  ensure
    page.execute_script "window.fetch = window.__messageInteractionsOriginalFetch" if page
  end

  test "replies with notify off and renders a tombstone when the target is deleted" do
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Reply"

    assert_selector "[data-composer-target='contextLabel']", text: /Replying to JZ/, wait: 10
    assert_field "Notify author", checked: true
    uncheck "Notify author"
    fill_in "Write a message", with: "A reply without a notification"
    click_button "Send Message"

    assert_selector ".message__reply-preview", text: /Replying to JZ/, wait: 10
    reply = @room.messages.find_by!(markdown_source: "A reply without a notification")
    assert_equal messages(:third).id, reply.reply_to_message_id
    assert_not reply.reply_notify_author?

    messages(:third).destroy!
    visit room_url(@room)
    assert_selector ".message__reply-preview", text: "Replying to a deleted message", wait: 10
  end

  test "copies message text and link and forwards to a server-provided thread destination" do
    destination_thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Forward destination")

    page.execute_script <<~JS
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: { writeText: text => { window.__messageInteractionsCopied = text; return Promise.resolve() } }
      })
    JS

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Copy text"
    assert_equal "Third time's a charm.", page.evaluate_script("window.__messageInteractionsCopied")

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Copy message link"
    assert_includes page.evaluate_script("window.__messageInteractionsCopied"), "/rooms/#{@room.id}/@#{messages(:third).to_param}"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Forward"
    assert_selector "dialog[open]", visible: true, wait: 10
    find(".message-forward-dialog__destination", text: "Forward destination", wait: 10).click
    fill_in "Add a note", with: "Forwarded from the interaction test"
    within "dialog[open]" do
      click_button "Forward"
    end

    assert_selector "[data-message-actions-target='forwardStatus']", text: /Forwarded to 1 destination/, wait: 10
    assert Message.exists?(forward_note: "Forwarded from the interaction test", thread_id: destination_thread.id)
    save_screenshot "forward-dialog.png"
  end

  test "forwarding twice in a row submits only once" do
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Forward"
    assert_selector "dialog[open]", visible: true, wait: 10

    find(".message-forward-dialog__destination input", match: :first, wait: 10).check
    submit = find("[data-message-actions-target='forwardSubmit']")

    assert_difference -> { messages(:third).forwards.count }, 1 do
      submit.click
      assert_selector "[data-message-actions-target='forwardStatus']", text: /Forwarded to 1 destination/, wait: 10
      assert_selector "[data-message-actions-target='forwardSubmit']:disabled"
    end
    assert_no_selector "dialog[open]", wait: 10
  end

  test "groups emoji reactions, updates the live count, and highlights the current user" do
    using_session("David") do
      sign_in "david@37signals.com"
      join_room @room

      within_message(messages(:third)) do
        right_click_message
      end
      assert_message_menu_open
      find(".message__quick-reaction[title='Thumbs up']").click

      assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
      assert_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"
    end

    assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
    assert_no_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    find(".message__quick-reaction[title='Thumbs up']").click

    assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "2", wait: 10
    assert_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"

    using_session("David") do
      assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "2", wait: 10
      assert_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"
    end

    find(".reaction-chip[data-reaction='👍']").click
    assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
    assert_no_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"

    using_session("David") do
      assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
      assert_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"
    end
  end

  private
    def save_screenshot(name)
      page.save_screenshot SCREENSHOT_DIR.join(name)
    end

    def assert_bottom_sheet_action_menu
      # Metadata reveals the edit/delete actions and re-runs placement, so
      # wait for it before measuring the final geometry.
      assert_selector ".message__edit-action", visible: true, wait: 10
      geometry = page.evaluate_script(<<~JS)
        (() => {
          const menu = document.querySelector("#message-actions-menu:not([hidden])")
          const bounds = element => {
            const rect = element.getBoundingClientRect()
            return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height }
          }
          const visibleSizes = selector => Array.from(menu.querySelectorAll(selector))
            .filter(el => el.getClientRects().length > 0)
            .map(el => {
              const rect = el.getBoundingClientRect()
              return { width: rect.width, height: rect.height }
            })
          return {
            menu: menu ? bounds(menu) : null,
            reactions: menu ? visibleSizes(".message__quick-reaction") : [],
            actions: menu ? visibleSizes(".message__menu-action") : [],
            viewport: { width: window.innerWidth, height: window.innerHeight }
          }
        })()
      JS
      refute_nil geometry["menu"], "expected the message action menu to be open"

      menu = geometry["menu"]
      viewport = geometry["viewport"]

      assert_in_delta viewport["width"], menu["width"], 1, "expected the menu to span the viewport width"
      assert_in_delta viewport["height"], menu["bottom"], 1, "expected the menu to sit on the viewport bottom"
      assert_operator menu["height"], :<=, viewport["height"] * 0.7 + 1, "expected the menu to be at most 70vh tall"
      assert_operator menu["top"], :>=, 0
      assert_operator menu["left"], :>=, 0

      assert_not_empty geometry["reactions"], "expected quick reactions to be present"
      geometry["reactions"].each do |reaction|
        assert_operator reaction["width"], :>=, 44, "expected reaction targets at least 44px wide"
        assert_operator reaction["height"], :>=, 44, "expected reaction targets at least 44px tall"
      end
      geometry["actions"].each do |action|
        assert_operator action["height"], :>=, 44, "expected menu actions at least 44px tall"
      end
    end

    def assert_menu_within_viewport
      bounds = page.evaluate_script(<<~JS)
        (() => {
          const menu = document.querySelector("#message-actions-menu:not([hidden])")
          if (!menu) return null
          const rect = menu.getBoundingClientRect()
          return {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
            width: rect.width,
            height: rect.height,
            viewportWidth: window.innerWidth,
            viewportHeight: window.innerHeight
          }
        })()
      JS
      refute_nil bounds
      assert_operator bounds["left"], :>=, 0
      assert_operator bounds["top"], :>=, 0
      assert_operator bounds["right"], :<=, bounds["viewportWidth"]
      assert_operator bounds["bottom"], :<=, bounds["viewportHeight"]
    end
end
